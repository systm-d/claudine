//! Gestion des marketplaces de plugins Claude : registre `known_marketplaces.json`,
//! manifeste `.claude-plugin/marketplace.json`, et clonage délégué au binaire `git`.
//!
//! Le clonage est délégué à `git` (aucune dépendance de build ; MSRV 1.74). La
//! clé du registre est le `name` du manifeste, connu seulement après clonage.

use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::{Map, Value};

use crate::error::{CoreError, Result};
use crate::home::ClaudeHome;
use crate::settings::SettingsDoc;

/// Provenance d'une marketplace (clé `source` du registre).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MarketplaceSource {
    Github { repo: String },
    Git { url: String },
    Local { path: PathBuf },
}

impl MarketplaceSource {
    /// Analyse une saisie utilisateur : URL git, `owner/repo`, ou chemin local.
    pub fn parse(input: &str) -> Option<Self> {
        let s = input.trim();
        if s.is_empty() {
            return None;
        }
        if s.contains("://") || s.starts_with("git@") || s.ends_with(".git") {
            return Some(MarketplaceSource::Git { url: s.to_string() });
        }
        if Path::new(s).exists() {
            return Some(MarketplaceSource::Local { path: PathBuf::from(s) });
        }
        if looks_like_owner_repo(s) {
            return Some(MarketplaceSource::Github { repo: s.to_string() });
        }
        None
    }

    /// URL passée à `git clone`.
    pub fn clone_url(&self) -> String {
        match self {
            MarketplaceSource::Github { repo } => format!("https://github.com/{repo}.git"),
            MarketplaceSource::Git { url } => url.clone(),
            MarketplaceSource::Local { path } => path.to_string_lossy().into_owned(),
        }
    }

    /// Nom provisoire (dépôt) pour le répertoire temporaire de clonage.
    pub fn provisional_name(&self) -> String {
        let raw = match self {
            MarketplaceSource::Github { repo } => repo.rsplit('/').next().unwrap_or("").to_string(),
            MarketplaceSource::Git { url } => url
                .trim_end_matches('/')
                .rsplit('/')
                .next()
                .unwrap_or("")
                .trim_end_matches(".git")
                .to_string(),
            MarketplaceSource::Local { path } => path
                .file_name()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_default(),
        };
        if raw.is_empty() {
            "marketplace".to_string()
        } else {
            raw
        }
    }

    fn to_json(&self) -> Value {
        let mut m = Map::new();
        match self {
            MarketplaceSource::Github { repo } => {
                m.insert("source".into(), Value::String("github".into()));
                m.insert("repo".into(), Value::String(repo.clone()));
            }
            MarketplaceSource::Git { url } => {
                m.insert("source".into(), Value::String("git".into()));
                m.insert("url".into(), Value::String(url.clone()));
            }
            MarketplaceSource::Local { path } => {
                m.insert("source".into(), Value::String("local".into()));
                m.insert("path".into(), Value::String(path.to_string_lossy().into_owned()));
            }
        }
        Value::Object(m)
    }

    fn from_json(v: &Value) -> Option<Self> {
        let o = v.as_object()?;
        match o.get("source").and_then(|s| s.as_str())? {
            "github" => Some(MarketplaceSource::Github {
                repo: o.get("repo")?.as_str()?.to_string(),
            }),
            "git" => Some(MarketplaceSource::Git {
                url: o.get("url")?.as_str()?.to_string(),
            }),
            "local" => Some(MarketplaceSource::Local {
                path: PathBuf::from(o.get("path")?.as_str()?),
            }),
            _ => None,
        }
    }
}

fn looks_like_owner_repo(s: &str) -> bool {
    if s.contains("..") {
        return false;
    }
    let parts: Vec<&str> = s.split('/').collect();
    parts.len() == 2
        && parts.iter().all(|p| {
            !p.is_empty()
                && p.chars()
                    .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | '-'))
        })
}

/// Entrée du registre `known_marketplaces.json`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Marketplace {
    pub name: String,
    pub source: MarketplaceSource,
    pub install_location: PathBuf,
    pub last_updated: String,
}

/// Manifeste `.claude-plugin/marketplace.json`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MarketplaceManifest {
    pub name: String,
    pub description: Option<String>,
    pub owner_name: Option<String>,
    pub plugins: Vec<PluginManifestEntry>,
}

/// Entrée plugin du manifeste (minimal pour 2c-1 ; étendu en 2c-2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PluginManifestEntry {
    pub name: String,
    pub description: Option<String>,
}

fn marketplaces_dir(home: &ClaudeHome) -> PathBuf {
    home.plugins_dir().join("marketplaces")
}

fn known_marketplaces_path(home: &ClaudeHome) -> PathBuf {
    home.plugins_dir().join("known_marketplaces.json")
}

/// Nom de marketplace sûr (pas de séparateur de chemin ni de `..`).
fn is_safe_name(name: &str) -> bool {
    !name.is_empty()
        && name != "."
        && name != ".."
        && !name.contains('/')
        && !name.contains('\\')
        && !name.contains("..")
}

/// Lit le registre `known_marketplaces.json`. Absent → vec vide.
pub fn read_marketplaces(home: &ClaudeHome) -> Result<Vec<Marketplace>> {
    let doc = SettingsDoc::load(&known_marketplaces_path(home))?;
    let Some(obj) = doc.root().as_object() else {
        return Ok(Vec::new());
    };
    let mut out = Vec::new();
    for (name, entry) in obj {
        let Some(eo) = entry.as_object() else {
            continue;
        };
        let Some(source) = eo.get("source").and_then(MarketplaceSource::from_json) else {
            continue;
        };
        let install_location = eo
            .get("installLocation")
            .and_then(|v| v.as_str())
            .map(PathBuf::from)
            .unwrap_or_else(|| marketplaces_dir(home).join(name));
        let last_updated = eo
            .get("lastUpdated")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        out.push(Marketplace {
            name: name.clone(),
            source,
            install_location,
            last_updated,
        });
    }
    out.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(out)
}

fn manifest_path(dir: &Path) -> PathBuf {
    dir.join(".claude-plugin").join("marketplace.json")
}

fn parse_manifest(v: &Value) -> Option<MarketplaceManifest> {
    let o = v.as_object()?;
    let name = o.get("name")?.as_str()?.to_string();
    if name.trim().is_empty() {
        return None;
    }
    // `plugins` doit exister et être un tableau (sinon manifeste invalide).
    let arr = o.get("plugins")?.as_array()?;
    let plugins = arr
        .iter()
        .filter_map(|p| {
            let po = p.as_object()?;
            Some(PluginManifestEntry {
                name: po.get("name")?.as_str()?.to_string(),
                description: po.get("description").and_then(|d| d.as_str()).map(String::from),
            })
        })
        .collect();
    Some(MarketplaceManifest {
        name,
        description: o.get("description").and_then(|d| d.as_str()).map(String::from),
        owner_name: o
            .get("owner")
            .and_then(|ow| ow.as_object())
            .and_then(|ow| ow.get("name"))
            .and_then(|n| n.as_str())
            .map(String::from),
        plugins,
    })
}

fn read_manifest_at(dir: &Path) -> Result<MarketplaceManifest> {
    let p = manifest_path(dir);
    let content = std::fs::read_to_string(&p).map_err(|e| CoreError::io(&p, e))?;
    let v: Value = serde_json::from_str(&content).map_err(|e| CoreError::JsonParse {
        file: p.clone(),
        line: 0,
        source: e,
    })?;
    parse_manifest(&v)
        .ok_or_else(|| CoreError::Marketplace(format!("manifeste invalide : {}", p.display())))
}

/// Lit le manifeste d'une marketplace clonée.
pub fn read_marketplace_manifest(home: &ClaudeHome, name: &str) -> Result<MarketplaceManifest> {
    read_manifest_at(&marketplaces_dir(home).join(name))
}

/// Formate un instant en ISO 8601 UTC avec millisecondes : `YYYY-MM-DDThh:mm:ss.mmmZ`.
pub fn iso8601_utc(t: SystemTime) -> String {
    let dur = t.duration_since(UNIX_EPOCH).unwrap_or_default();
    let secs = dur.as_secs();
    let millis = dur.subsec_millis();
    let days = (secs / 86_400) as i64;
    let sod = secs % 86_400;
    let (hh, mm, ss) = (sod / 3600, (sod % 3600) / 60, sod % 60);
    let (year, month, day) = civil_from_days(days);
    format!("{year:04}-{month:02}-{day:02}T{hh:02}:{mm:02}:{ss:02}.{millis:03}Z")
}

/// (année, mois, jour) depuis un nombre de jours après l'époque Unix.
/// Algorithme `civil_from_days` de Howard Hinnant.
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}

mod git {
    use super::{CoreError, Result};
    use std::path::Path;
    use std::process::Command;

    fn finish(mut cmd: Command, what: &str) -> Result<()> {
        let output = cmd
            .output()
            .map_err(|e| CoreError::Marketplace(format!("git introuvable dans le PATH ({e})")))?;
        if output.status.success() {
            return Ok(());
        }
        let stderr = String::from_utf8_lossy(&output.stderr);
        let msg: String = stderr.trim().chars().take(300).collect();
        Err(CoreError::Marketplace(format!("{what} a échoué : {msg}")))
    }

    /// `git clone --depth 1 -- <url> <dest>`, durci contre l'injection d'argument.
    pub fn clone(url: &str, dest: &Path) -> Result<()> {
        // Refuse une URL ressemblant à une option (flag smuggling).
        if url.starts_with('-') {
            return Err(CoreError::Marketplace(format!("url invalide : {url}")));
        }
        let mut c = Command::new("git");
        // `protocol.ext.allow=never` neutralise le transport `ext::` (exec arbitraire) ;
        // `--` sépare les options des positionnels.
        c.arg("-c")
            .arg("protocol.ext.allow=never")
            .arg("clone")
            .arg("--depth")
            .arg("1")
            .arg("--")
            .arg(url)
            .arg(dest);
        c.env("GIT_TERMINAL_PROMPT", "0");
        finish(c, "git clone")
    }

    /// `git -C <dir> pull --ff-only`.
    pub fn pull(dir: &Path) -> Result<()> {
        let mut c = Command::new("git");
        c.arg("-C").arg(dir).arg("pull").arg("--ff-only");
        c.env("GIT_TERMINAL_PROMPT", "0");
        finish(c, "git pull")
    }
}

/// Clone une marketplace, valide son manifeste, l'enregistre. Le nom définitif
/// vient du manifeste ; rollback du clone si invalide / déjà présente.
pub fn add_marketplace(home: &ClaudeHome, source: MarketplaceSource) -> Result<Marketplace> {
    let mdir = marketplaces_dir(home);
    std::fs::create_dir_all(&mdir).map_err(|e| CoreError::io(&mdir, e))?;

    let tmp = mdir.join(format!(".tmp-add-{}", source.provisional_name()));
    if tmp.exists() {
        let _ = std::fs::remove_dir_all(&tmp);
    }
    if let Err(e) = git::clone(&source.clone_url(), &tmp) {
        let _ = std::fs::remove_dir_all(&tmp);
        return Err(e);
    }

    let manifest = match read_manifest_at(&tmp) {
        Ok(m) => m,
        Err(e) => {
            let _ = std::fs::remove_dir_all(&tmp);
            return Err(e);
        }
    };
    if !is_safe_name(&manifest.name) {
        let _ = std::fs::remove_dir_all(&tmp);
        return Err(CoreError::Marketplace(format!(
            "nom de marketplace invalide : {}",
            manifest.name
        )));
    }

    let dest = mdir.join(&manifest.name);
    if dest.exists() {
        let _ = std::fs::remove_dir_all(&tmp);
        return Err(CoreError::Marketplace(format!(
            "marketplace « {} » déjà présente",
            manifest.name
        )));
    }
    std::fs::rename(&tmp, &dest).map_err(|e| CoreError::io(&dest, e))?;

    let now = iso8601_utc(SystemTime::now());
    let mut entry = Map::new();
    entry.insert("source".into(), source.to_json());
    entry.insert(
        "installLocation".into(),
        Value::String(dest.to_string_lossy().into_owned()),
    );
    entry.insert("lastUpdated".into(), Value::String(now.clone()));

    let path = known_marketplaces_path(home);
    let mut doc = SettingsDoc::load(&path)?;
    doc.set(&[manifest.name.as_str()], Value::Object(entry));
    doc.save(&path)?;

    Ok(Marketplace {
        name: manifest.name,
        source,
        install_location: dest,
        last_updated: now,
    })
}

/// Retire une marketplace : supprime son dossier (confiné) et son entrée.
pub fn remove_marketplace(home: &ClaudeHome, name: &str) -> Result<()> {
    if !is_safe_name(name) {
        return Err(CoreError::Marketplace(format!(
            "nom de marketplace invalide : {name}"
        )));
    }
    let dir = marketplaces_dir(home).join(name);
    if dir.exists() {
        std::fs::remove_dir_all(&dir).map_err(|e| CoreError::io(&dir, e))?;
    }
    let path = known_marketplaces_path(home);
    let mut doc = SettingsDoc::load(&path)?;
    doc.unset(&[name]);
    doc.save(&path)
}

/// Met à jour une marketplace (`git pull`) et rafraîchit `lastUpdated`.
pub fn update_marketplace(home: &ClaudeHome, name: &str) -> Result<()> {
    if !is_safe_name(name) {
        return Err(CoreError::Marketplace(format!(
            "nom de marketplace invalide : {name}"
        )));
    }
    let dir = marketplaces_dir(home).join(name);
    if !dir.exists() {
        return Err(CoreError::Marketplace(format!(
            "marketplace « {name} » absente"
        )));
    }
    git::pull(&dir)?;

    let path = known_marketplaces_path(home);
    let mut doc = SettingsDoc::load(&path)?;
    if doc.get(&[name]).is_some() {
        doc.set(&[name, "lastUpdated"], Value::String(iso8601_utc(SystemTime::now())));
        doc.save(&path)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path as StdPath;
    use std::time::{Duration, UNIX_EPOCH};

    /// Home jetable (tempdir nu ; `plugins/` est créé à la demande).
    fn home() -> (tempfile::TempDir, ClaudeHome) {
        let d = tempfile::tempdir().unwrap();
        let h = ClaudeHome::from_base(d.path());
        (d, h)
    }

    #[test]
    fn parse_source_discriminates() {
        assert!(matches!(
            MarketplaceSource::parse("anthropics/claude-plugins-official"),
            Some(MarketplaceSource::Github { .. })
        ));
        assert!(matches!(
            MarketplaceSource::parse("https://example.com/x.git"),
            Some(MarketplaceSource::Git { .. })
        ));
        assert!(matches!(
            MarketplaceSource::parse("git@github.com:o/r.git"),
            Some(MarketplaceSource::Git { .. })
        ));
        // Chemin local existant → Local.
        let d = tempfile::tempdir().unwrap();
        assert!(matches!(
            MarketplaceSource::parse(&d.path().to_string_lossy()),
            Some(MarketplaceSource::Local { .. })
        ));
        assert!(MarketplaceSource::parse("").is_none());
        assert!(MarketplaceSource::parse("pas une source !!").is_none());
    }

    #[test]
    fn clone_url_for_github() {
        let s = MarketplaceSource::Github { repo: "o/r".into() };
        assert_eq!(s.clone_url(), "https://github.com/o/r.git");
    }

    #[test]
    fn iso8601_utc_formats_known_instants() {
        assert_eq!(iso8601_utc(UNIX_EPOCH), "1970-01-01T00:00:00.000Z");
        let t = UNIX_EPOCH + Duration::from_secs(1_700_000_000);
        assert_eq!(iso8601_utc(t), "2023-11-14T22:13:20.000Z");
    }

    #[test]
    fn read_marketplaces_parses_registry() {
        let (_d, home) = home();
        let reg = r#"{
            "claude-plugins-official": {
                "source": {"source":"github","repo":"anthropics/claude-plugins-official"},
                "installLocation": "/abs/marketplaces/claude-plugins-official",
                "lastUpdated": "2026-06-25T07:54:22.246Z"
            }
        }"#;
        let p = home.plugins_dir().join("known_marketplaces.json");
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(&p, reg).unwrap();

        let list = read_marketplaces(&home).unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].name, "claude-plugins-official");
        assert!(matches!(&list[0].source, MarketplaceSource::Github { repo } if repo == "anthropics/claude-plugins-official"));
        assert_eq!(list[0].last_updated, "2026-06-25T07:54:22.246Z");
    }

    #[test]
    fn read_marketplaces_absent_is_empty() {
        let (_d, home) = home();
        assert!(read_marketplaces(&home).unwrap().is_empty());
    }

    #[test]
    fn read_manifest_parses_name_and_plugins() {
        let (_d, home) = home();
        let dir = home.plugins_dir().join("marketplaces").join("mkt");
        let cp = dir.join(".claude-plugin");
        std::fs::create_dir_all(&cp).unwrap();
        std::fs::write(
            cp.join("marketplace.json"),
            r#"{"name":"mkt","owner":{"name":"Acme"},"plugins":[{"name":"p1","description":"d"}]}"#,
        )
        .unwrap();
        let man = read_marketplace_manifest(&home, "mkt").unwrap();
        assert_eq!(man.name, "mkt");
        assert_eq!(man.owner_name.as_deref(), Some("Acme"));
        assert_eq!(man.plugins.len(), 1);
        assert_eq!(man.plugins[0].name, "p1");
    }

    /// Exécute `git` dans `cwd`, isolé de la config globale/système.
    fn git(args: &[&str], cwd: &StdPath) {
        let status = std::process::Command::new("git")
            .args(args)
            .current_dir(cwd)
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .env("GIT_CONFIG_SYSTEM", "/dev/null")
            .env("GIT_TERMINAL_PROMPT", "0")
            .status()
            .expect("git doit être installé pour les tests");
        assert!(status.success(), "git {args:?} a échoué");
    }

    /// Dépôt git local jouant le rôle de marketplace, avec un manifeste donné.
    fn make_repo(manifest: &str) -> tempfile::TempDir {
        let d = tempfile::tempdir().unwrap();
        let root = d.path();
        git(&["init", "-q", "-b", "main"], root);
        git(&["config", "user.email", "t@t"], root);
        git(&["config", "user.name", "t"], root);
        let cp = root.join(".claude-plugin");
        std::fs::create_dir_all(&cp).unwrap();
        std::fs::write(cp.join("marketplace.json"), manifest).unwrap();
        git(&["add", "-A"], root);
        git(&["commit", "-q", "-m", "init"], root);
        d
    }

    fn valid_manifest(name: &str) -> String {
        format!(r#"{{"name":"{name}","owner":{{"name":"Acme"}},"plugins":[{{"name":"p1","description":"d"}}]}}"#)
    }

    #[test]
    fn add_marketplace_local_clones_validates_registers() {
        let repo = make_repo(&valid_manifest("acme-mkt"));
        let (_d, home) = home();
        let src = MarketplaceSource::Local { path: repo.path().to_path_buf() };

        let mk = add_marketplace(&home, src).unwrap();
        assert_eq!(mk.name, "acme-mkt");

        let dir = home.plugins_dir().join("marketplaces").join("acme-mkt");
        assert!(manifest_path(&dir).is_file(), "manifeste cloné présent");

        let list = read_marketplaces(&home).unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].name, "acme-mkt");
        assert!(matches!(list[0].source, MarketplaceSource::Local { .. }));
        assert!(list[0].last_updated.ends_with('Z'));

        let man = read_marketplace_manifest(&home, "acme-mkt").unwrap();
        assert_eq!(man.plugins.len(), 1);
    }

    #[test]
    fn add_marketplace_rejects_invalid_manifest_without_writing() {
        let repo = make_repo(r#"{"plugins":[]}"#); // pas de "name"
        let (_d, home) = home();
        let src = MarketplaceSource::Local { path: repo.path().to_path_buf() };

        assert!(add_marketplace(&home, src).is_err());
        assert!(read_marketplaces(&home).unwrap().is_empty(), "registre non écrit");
        // Aucun dossier résiduel (tmp nettoyé).
        let mdir = home.plugins_dir().join("marketplaces");
        if mdir.exists() {
            assert!(std::fs::read_dir(&mdir).unwrap().flatten().next().is_none());
        }
    }

    #[test]
    fn add_marketplace_duplicate_is_rejected() {
        let repo = make_repo(&valid_manifest("dup"));
        let (_d, home) = home();
        add_marketplace(&home, MarketplaceSource::Local { path: repo.path().to_path_buf() }).unwrap();
        let again = add_marketplace(&home, MarketplaceSource::Local { path: repo.path().to_path_buf() });
        assert!(again.is_err());
        assert_eq!(read_marketplaces(&home).unwrap().len(), 1);
    }

    #[test]
    fn remove_marketplace_clears_entry_and_dir() {
        let repo = make_repo(&valid_manifest("gone"));
        let (_d, home) = home();
        add_marketplace(&home, MarketplaceSource::Local { path: repo.path().to_path_buf() }).unwrap();
        let dir = home.plugins_dir().join("marketplaces").join("gone");
        assert!(dir.exists());

        remove_marketplace(&home, "gone").unwrap();
        assert!(!dir.exists());
        assert!(read_marketplaces(&home).unwrap().is_empty());
    }

    #[test]
    fn remove_marketplace_rejects_unsafe_name() {
        let (_d, home) = home();
        assert!(remove_marketplace(&home, "../evil").is_err());
    }

    #[test]
    fn update_marketplace_pulls_new_commit() {
        let repo = make_repo(&valid_manifest("upd"));
        let (_d, home) = home();
        add_marketplace(&home, MarketplaceSource::Local { path: repo.path().to_path_buf() }).unwrap();

        // Nouveau commit dans la source.
        std::fs::write(repo.path().join("NEW.txt"), "x").unwrap();
        git(&["add", "-A"], repo.path());
        git(&["commit", "-q", "-m", "more"], repo.path());

        update_marketplace(&home, "upd").unwrap();
        assert!(home
            .plugins_dir()
            .join("marketplaces")
            .join("upd")
            .join("NEW.txt")
            .exists());
    }

    #[test]
    fn add_marketplace_rejects_dash_url() {
        let (_d, home) = home();
        let src = MarketplaceSource::Git { url: "--upload-pack=evil".into() };
        assert!(add_marketplace(&home, src).is_err());
        assert!(read_marketplaces(&home).unwrap().is_empty());
    }

    #[test]
    fn add_marketplace_blocks_ext_transport() {
        // `protocol.ext.allow=never` doit faire échouer le transport ext:: (sinon RCE).
        let (_d, home) = home();
        let src = MarketplaceSource::Git { url: "ext::sh -c true".into() };
        assert!(add_marketplace(&home, src).is_err());
        assert!(read_marketplaces(&home).unwrap().is_empty());
    }
}
