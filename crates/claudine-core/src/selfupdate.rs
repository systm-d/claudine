//! Auto-mise à jour du binaire `claudine` depuis les releases GitHub.
//!
//! `claudine update` interroge l'API GitHub pour la dernière release, compare la
//! version au binaire courant, télécharge l'asset correspondant à la plateforme
//! puis remplace l'exécutable en cours d'exécution (via `self-replace`, qui gère
//! les subtilités multi-plateformes — notamment le verrou Windows).
//!
//! Les assets suivent la convention de `release.yml` :
//!   * `claudine-linux-x86_64.tar.gz`  → binaire dans `claudine-linux-x86_64/claudine`
//!   * `claudine-macos-aarch64.tar.gz` → binaire dans `claudine-macos-aarch64/claudine`
//!   * `claudine-windows-x86_64.exe`   → binaire brut (pas d'archive)
//!
//! Les fonctions de décision (choix d'asset, comparaison de versions, parsing de
//! la réponse) sont pures et testées ; seules les fonctions réseau / système ne
//! le sont pas.

use std::io::Read;
use std::path::Path;

use serde_json::Value;

/// Dépôt GitHub source des releases.
const REPO: &str = "systm-d/claudine";

/// En-tête `User-Agent` (exigé par l'API GitHub).
const USER_AGENT: &str = concat!("claudine/", env!("CARGO_PKG_VERSION"));

/// URL de l'API « dernière release ».
fn api_latest_url() -> String {
    format!("https://api.github.com/repos/{REPO}/releases/latest")
}

/// Une release GitHub, réduite à ce dont on a besoin.
#[derive(Debug, Clone)]
pub struct Release {
    /// Tag de la release (ex. `v0.1.3`).
    pub tag: String,
    /// Version sans le `v` de tête (ex. `0.1.3`).
    pub version: String,
    pub assets: Vec<Asset>,
}

/// Un artefact attaché à une release.
#[derive(Debug, Clone)]
pub struct Asset {
    pub name: String,
    pub url: String,
}

/// L'asset attendu pour la plateforme courante.
#[derive(Debug, Clone)]
pub struct TargetInfo {
    /// Nom exact de l'asset dans la release.
    pub asset: String,
    /// Vrai si l'asset est une archive `.tar.gz` (sinon binaire brut, ex. `.exe`).
    pub archived: bool,
    /// Chemin du binaire dans l'archive (vide pour un binaire brut).
    pub bin_in_archive: String,
}

/// Détermine l'asset attendu pour un couple (os, arch). `None` si aucune release
/// n'est publiée pour cette plateforme (ex. macOS Intel, Linux ARM).
pub fn target_for(os: &str, arch: &str) -> Option<TargetInfo> {
    let label = match (os, arch) {
        ("linux", "x86_64") => "linux-x86_64",
        ("macos", "aarch64") => "macos-aarch64",
        ("windows", "x86_64") => "windows-x86_64",
        _ => return None,
    };
    let archived = os != "windows";
    let (asset, bin_in_archive) = if archived {
        (
            format!("claudine-{label}.tar.gz"),
            format!("claudine-{label}/claudine"),
        )
    } else {
        (format!("claudine-{label}.exe"), String::new())
    };
    Some(TargetInfo {
        asset,
        archived,
        bin_in_archive,
    })
}

/// Asset attendu pour la plateforme d'exécution courante.
pub fn current_target() -> Option<TargetInfo> {
    target_for(std::env::consts::OS, std::env::consts::ARCH)
}

/// Analyse la réponse JSON de l'API GitHub « dernière release ».
pub fn parse_release(json: &str) -> Result<Release, String> {
    let v: Value =
        serde_json::from_str(json).map_err(|e| format!("réponse GitHub illisible : {e}"))?;
    let tag = v
        .get("tag_name")
        .and_then(Value::as_str)
        .ok_or("release sans `tag_name`")?
        .to_string();
    let version = tag.trim_start_matches('v').to_string();
    let assets = v
        .get("assets")
        .and_then(Value::as_array)
        .map(|arr| {
            arr.iter()
                .filter_map(|a| {
                    let name = a.get("name")?.as_str()?.to_string();
                    let url = a.get("browser_download_url")?.as_str()?.to_string();
                    Some(Asset { name, url })
                })
                .collect()
        })
        .unwrap_or_default();
    Ok(Release {
        tag,
        version,
        assets,
    })
}

/// Choisit dans une release l'asset correspondant exactement à la cible.
pub fn pick_asset<'a>(assets: &'a [Asset], target: &TargetInfo) -> Option<&'a Asset> {
    assets.iter().find(|a| a.name == target.asset)
}

/// Découpe une version `major.minor.patch` (les suffixes pré-release/`+build`
/// sont ignorés). Les composants absents valent 0.
fn parse_version(s: &str) -> Option<(u64, u64, u64)> {
    let core = s.trim_start_matches('v');
    let core = core.split(['-', '+']).next().unwrap_or(core);
    let mut it = core.split('.');
    let major = it.next()?.parse().ok()?;
    let minor = it.next().unwrap_or("0").parse().ok()?;
    let patch = it.next().unwrap_or("0").parse().ok()?;
    Some((major, minor, patch))
}

/// Vrai si `latest` est strictement plus récent que `current` (comparaison
/// semver sur major/minor/patch). Faux si l'une des versions est illisible.
pub fn is_newer(current: &str, latest: &str) -> bool {
    match (parse_version(current), parse_version(latest)) {
        (Some(c), Some(l)) => l > c,
        _ => false,
    }
}

// --- Réseau / système (non testés unitairement) ---

/// Agent HTTP partagé. Honore `HTTPS_PROXY` / `https_proxy` (proxy HTTP CONNECT)
/// pour fonctionner derrière un proxy d'entreprise ; sinon connexion directe.
fn agent() -> ureq::Agent {
    let mut builder = ureq::AgentBuilder::new().user_agent(USER_AGENT);
    let proxy = std::env::var("HTTPS_PROXY")
        .or_else(|_| std::env::var("https_proxy"))
        .ok()
        .filter(|p| !p.trim().is_empty());
    if let Some(p) = proxy {
        if let Ok(proxy) = ureq::Proxy::new(p.trim()) {
            builder = builder.proxy(proxy);
        }
    }
    builder.build()
}

/// GET renvoyant le corps texte (petites réponses : API JSON).
fn http_get_string(url: &str) -> Result<String, String> {
    agent()
        .get(url)
        .set("Accept", "application/vnd.github+json")
        .call()
        .map_err(|e| format!("requête échouée : {e}"))?
        .into_string()
        .map_err(|e| format!("lecture de la réponse : {e}"))
}

/// Télécharge un asset binaire (suit les redirections vers le stockage GitHub).
pub fn download(url: &str) -> Result<Vec<u8>, String> {
    let resp = agent()
        .get(url)
        .call()
        .map_err(|e| format!("téléchargement échoué : {e}"))?;
    let mut buf = Vec::new();
    resp.into_reader()
        .read_to_end(&mut buf)
        .map_err(|e| format!("lecture du binaire : {e}"))?;
    Ok(buf)
}

/// Récupère la dernière release publiée sur GitHub.
pub fn fetch_latest() -> Result<Release, String> {
    parse_release(&http_get_string(&api_latest_url())?)
}

/// Installe `bytes` (archive ou binaire brut selon `target`) en remplaçant
/// l'exécutable en cours d'exécution.
pub fn install(bytes: &[u8], target: &TargetInfo) -> Result<(), String> {
    let tmp = std::env::temp_dir().join(format!(
        "claudine-update-{}{}",
        std::process::id(),
        if target.archived { "" } else { ".exe" }
    ));
    if target.archived {
        extract_binary_from_targz(bytes, &target.bin_in_archive, &tmp)?;
    } else {
        std::fs::write(&tmp, bytes).map_err(|e| format!("écriture temporaire : {e}"))?;
    }
    make_executable(&tmp)?;
    let result =
        self_replace::self_replace(&tmp).map_err(|e| format!("remplacement du binaire : {e}"));
    // Nettoyage best-effort (self_replace peut déjà avoir consommé le fichier).
    let _ = std::fs::remove_file(&tmp);
    result
}

/// Extrait le binaire `claudine` d'une archive `.tar.gz` vers `out`.
fn extract_binary_from_targz(bytes: &[u8], bin_in_archive: &str, out: &Path) -> Result<(), String> {
    use flate2::read::GzDecoder;
    use tar::Archive;

    let mut archive = Archive::new(GzDecoder::new(bytes));
    let entries = archive
        .entries()
        .map_err(|e| format!("archive illisible : {e}"))?;
    for entry in entries {
        let mut entry = entry.map_err(|e| format!("entrée d'archive : {e}"))?;
        let is_file = entry.header().entry_type().is_file();
        let path = entry
            .path()
            .map_err(|e| format!("chemin d'archive : {e}"))?;
        let by_full = path.to_str() == Some(bin_in_archive);
        let by_name = is_file && path.file_name().and_then(|n| n.to_str()) == Some("claudine");
        if by_full || by_name {
            let mut f =
                std::fs::File::create(out).map_err(|e| format!("création temporaire : {e}"))?;
            std::io::copy(&mut entry, &mut f).map_err(|e| format!("extraction : {e}"))?;
            return Ok(());
        }
    }
    Err("binaire « claudine » introuvable dans l'archive téléchargée".to_string())
}

#[cfg(unix)]
fn make_executable(p: &Path) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;
    let mut perm = std::fs::metadata(p)
        .map_err(|e| format!("lecture des permissions : {e}"))?
        .permissions();
    perm.set_mode(0o755);
    std::fs::set_permissions(p, perm).map_err(|e| format!("chmod : {e}"))
}

#[cfg(not(unix))]
fn make_executable(_p: &Path) -> Result<(), String> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn target_mapping_covers_released_platforms() {
        let linux = target_for("linux", "x86_64").unwrap();
        assert_eq!(linux.asset, "claudine-linux-x86_64.tar.gz");
        assert!(linux.archived);
        assert_eq!(linux.bin_in_archive, "claudine-linux-x86_64/claudine");

        let mac = target_for("macos", "aarch64").unwrap();
        assert_eq!(mac.asset, "claudine-macos-aarch64.tar.gz");
        assert!(mac.archived);

        let win = target_for("windows", "x86_64").unwrap();
        assert_eq!(win.asset, "claudine-windows-x86_64.exe");
        assert!(!win.archived);
        assert!(win.bin_in_archive.is_empty());
    }

    #[test]
    fn target_none_for_unreleased_platforms() {
        assert!(target_for("macos", "x86_64").is_none());
        assert!(target_for("linux", "aarch64").is_none());
        assert!(target_for("freebsd", "x86_64").is_none());
    }

    #[test]
    fn parses_release_and_assets() {
        let json = r#"{
            "tag_name": "v0.2.0",
            "assets": [
                {"name": "claudine-linux-x86_64.tar.gz", "browser_download_url": "https://x/lin.tgz"},
                {"name": "claudine-windows-x86_64.exe", "browser_download_url": "https://x/win.exe"},
                {"name": "SHA256SUMS", "browser_download_url": "https://x/sums"}
            ]
        }"#;
        let r = parse_release(json).unwrap();
        assert_eq!(r.tag, "v0.2.0");
        assert_eq!(r.version, "0.2.0");
        assert_eq!(r.assets.len(), 3);

        let linux = target_for("linux", "x86_64").unwrap();
        let a = pick_asset(&r.assets, &linux).unwrap();
        assert_eq!(a.url, "https://x/lin.tgz");
    }

    #[test]
    fn parse_release_rejects_bad_json() {
        assert!(parse_release("pas du json").is_err());
        assert!(parse_release(r#"{"no_tag": true}"#).is_err());
    }

    #[test]
    fn pick_asset_missing_returns_none() {
        let assets = vec![Asset {
            name: "claudine-macos-aarch64.tar.gz".to_string(),
            url: "u".to_string(),
        }];
        let linux = target_for("linux", "x86_64").unwrap();
        assert!(pick_asset(&assets, &linux).is_none());
    }

    #[test]
    fn version_comparison() {
        assert!(is_newer("0.1.2", "0.1.3"));
        assert!(is_newer("0.1.2", "0.2.0"));
        assert!(is_newer("0.1.2", "1.0.0"));
        assert!(is_newer("0.1.2", "v0.1.3")); // tolère le `v` de tête
        assert!(!is_newer("0.1.2", "0.1.2"));
        assert!(!is_newer("0.1.3", "0.1.2"));
        // Suffixes pré-release ignorés (comparaison sur le socle numérique).
        assert!(!is_newer("0.1.2", "0.1.2-rc1"));
        // Versions illisibles → pas de mise à jour proposée.
        assert!(!is_newer("0.1.2", "latest"));
    }
}
