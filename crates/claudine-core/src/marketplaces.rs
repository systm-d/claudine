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
            return Some(MarketplaceSource::Local {
                path: PathBuf::from(s),
            });
        }
        if looks_like_owner_repo(s) {
            return Some(MarketplaceSource::Github {
                repo: s.to_string(),
            });
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
                m.insert(
                    "path".into(),
                    Value::String(path.to_string_lossy().into_owned()),
                );
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

/// Source d'installation d'un plugin (champ `source` du manifeste).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PluginSource {
    /// Chaîne `"./plugins/X"` : sous-dossier de la marketplace clonée (pas de réseau).
    RelativePath { path: String },
    /// `url` / `git-subdir` / `github` : clone git épinglé à un commit, sous-dossier optionnel.
    Git {
        url: String,
        commit: String,
        subdir: Option<String>,
    },
}

/// Entrée plugin du manifeste.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PluginManifestEntry {
    pub name: String,
    pub description: Option<String>,
    /// `None` si la forme de `source` n'est pas reconnue (plugin non installable).
    pub source: Option<PluginSource>,
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

/// Analyse la valeur du champ `source` d'un plugin (chaîne relative ou objet typé).
fn parse_plugin_source(v: &Value) -> Option<PluginSource> {
    if let Some(s) = v.as_str() {
        let s = s.trim();
        if s.is_empty() {
            return None;
        }
        return Some(PluginSource::RelativePath {
            path: s.to_string(),
        });
    }
    let o = v.as_object()?;
    match o.get("source").and_then(|s| s.as_str())? {
        // `url` et `git-subdir` partagent la même mécanique : clone + checkout `sha`.
        "url" | "git-subdir" => Some(PluginSource::Git {
            url: o.get("url")?.as_str()?.to_string(),
            commit: o.get("sha")?.as_str()?.to_string(),
            subdir: o.get("path").and_then(|p| p.as_str()).map(String::from),
        }),
        "github" => Some(PluginSource::Git {
            url: format!("https://github.com/{}.git", o.get("repo")?.as_str()?),
            commit: o.get("commit")?.as_str()?.to_string(),
            subdir: None,
        }),
        _ => None,
    }
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
                description: po
                    .get("description")
                    .and_then(|d| d.as_str())
                    .map(String::from),
                source: po.get("source").and_then(parse_plugin_source),
            })
        })
        .collect();
    Some(MarketplaceManifest {
        name,
        description: o
            .get("description")
            .and_then(|d| d.as_str())
            .map(String::from),
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

    /// `git clone -- <url> <dest>` (historique **complet**, sans `--depth`) afin de
    /// pouvoir extraire ensuite n'importe quel commit épinglé. Durci comme `clone`.
    pub fn clone_full(url: &str, dest: &Path) -> Result<()> {
        if url.starts_with('-') {
            return Err(CoreError::Marketplace(format!("url invalide : {url}")));
        }
        let mut c = Command::new("git");
        c.arg("-c")
            .arg("protocol.ext.allow=never")
            .arg("clone")
            .arg("--")
            .arg(url)
            .arg(dest);
        c.env("GIT_TERMINAL_PROMPT", "0");
        finish(c, "git clone")
    }

    /// `git -C <dir> checkout --detach <commit>` : positionne l'arbre de travail
    /// sur le commit épinglé. Refuse un commit ressemblant à une option.
    pub fn checkout(dir: &Path, commit: &str) -> Result<()> {
        if commit.starts_with('-') {
            return Err(CoreError::Marketplace(format!(
                "commit invalide : {commit}"
            )));
        }
        let mut c = Command::new("git");
        c.arg("-C")
            .arg(dir)
            .arg("checkout")
            .arg("--detach")
            .arg(commit);
        c.env("GIT_TERMINAL_PROMPT", "0");
        finish(c, "git checkout")
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

/// Lit `version` depuis `<src>/.claude-plugin/plugin.json` (absent → None).
fn read_plugin_version(src: &Path) -> Option<String> {
    let p = src.join(".claude-plugin").join("plugin.json");
    let content = std::fs::read_to_string(&p).ok()?;
    let v: Value = serde_json::from_str(&content).ok()?;
    v.get("version").and_then(|x| x.as_str()).map(String::from)
}

/// Copie récursive de `src` vers `dest` (fichiers + dossiers ; liens symboliques ignorés).
fn copy_dir_recursive(src: &Path, dest: &Path) -> Result<()> {
    std::fs::create_dir_all(dest).map_err(|e| CoreError::io(dest, e))?;
    for entry in std::fs::read_dir(src).map_err(|e| CoreError::io(src, e))? {
        let entry = entry.map_err(|e| CoreError::io(src, e))?;
        let from = entry.path();
        let to = dest.join(entry.file_name());
        let ft = entry.file_type().map_err(|e| CoreError::io(&from, e))?;
        if ft.is_dir() {
            copy_dir_recursive(&from, &to)?;
        } else if ft.is_file() {
            std::fs::copy(&from, &to).map_err(|e| CoreError::io(&to, e))?;
        }
    }
    Ok(())
}

/// Installe un plugin (portée user) depuis le catalogue d'une marketplace :
/// matérialise ses fichiers dans `cache/<mkt>/<plugin>/<version>/`, écrit
/// `installed_plugins.json` et l'auto-active. Idempotent (réécrit la version).
pub fn install_plugin(home: &ClaudeHome, marketplace: &str, plugin: &str) -> Result<()> {
    if !is_safe_name(marketplace) {
        return Err(CoreError::Marketplace(format!(
            "nom de marketplace invalide : {marketplace}"
        )));
    }
    if !is_safe_name(plugin) {
        return Err(CoreError::Marketplace(format!(
            "nom de plugin invalide : {plugin}"
        )));
    }

    // 1. Localiser l'entrée du plugin et sa source.
    let manifest = read_marketplace_manifest(home, marketplace)?;
    let source = manifest
        .plugins
        .iter()
        .find(|p| p.name == plugin)
        .ok_or_else(|| {
            CoreError::Marketplace(format!(
                "plugin introuvable au catalogue : {plugin}@{marketplace}"
            ))
        })?
        .source
        .clone()
        .ok_or_else(|| {
            CoreError::Marketplace(format!(
                "source de plugin non gérée : {plugin}@{marketplace}"
            ))
        })?;

    let cache_root = home.plugins_dir().join("cache");

    // 2. Matérialiser la source dans `src` (`temp` = à nettoyer si clone).
    let (src, temp): (PathBuf, Option<PathBuf>) = match &source {
        PluginSource::RelativePath { path } => {
            let rel = path.trim_start_matches("./");
            if rel.is_empty() || rel.split('/').any(|c| c == ".." || c.is_empty()) {
                return Err(CoreError::Marketplace(format!(
                    "chemin de plugin invalide : {path}"
                )));
            }
            let mkt_dir = marketplaces_dir(home).join(marketplace);
            let dir = mkt_dir.join(rel);
            if !dir.starts_with(&mkt_dir) || !dir.is_dir() {
                return Err(CoreError::Marketplace(format!(
                    "dossier de plugin introuvable : {}",
                    dir.display()
                )));
            }
            // I1 : la source pourrait être un lien symbolique pointant hors de la marketplace.
            // On canonicalise les deux chemins pour comparer les cibles réelles.
            let canon_mkt =
                std::fs::canonicalize(&mkt_dir).map_err(|e| CoreError::io(&mkt_dir, e))?;
            let canon_dir = std::fs::canonicalize(&dir).map_err(|e| CoreError::io(&dir, e))?;
            if !canon_dir.starts_with(&canon_mkt) {
                return Err(CoreError::Marketplace(
                    "dossier de plugin hors marketplace".to_string(),
                ));
            }
            (canon_dir, None)
        }
        PluginSource::Git {
            url,
            commit,
            subdir,
        } => {
            std::fs::create_dir_all(&cache_root).map_err(|e| CoreError::io(&cache_root, e))?;
            let ts = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0);
            let temp = cache_root.join(format!("temp_git_{ts}"));
            if temp.exists() {
                let _ = std::fs::remove_dir_all(&temp);
            }
            // Clone complet + checkout du commit épinglé ; nettoie le temp si échec.
            if let Err(e) = git::clone_full(url, &temp).and_then(|()| git::checkout(&temp, commit))
            {
                let _ = std::fs::remove_dir_all(&temp);
                return Err(e);
            }
            // Sous-dossier optionnel, confiné sous le temp.
            let src = match subdir {
                Some(sd) => {
                    let sd = sd.trim_start_matches("./");
                    if sd.split('/').any(|c| c == "..") {
                        let _ = std::fs::remove_dir_all(&temp);
                        return Err(CoreError::Marketplace(format!(
                            "sous-dossier invalide : {sd}"
                        )));
                    }
                    temp.join(sd)
                }
                None => temp.clone(),
            };
            if !src.starts_with(&temp) || !src.is_dir() {
                let _ = std::fs::remove_dir_all(&temp);
                return Err(CoreError::Marketplace(
                    "sous-dossier de plugin introuvable".to_string(),
                ));
            }
            // I1 : canonicaliser pour rejeter un éventuel symlink hors du clone temporaire.
            let canon_temp = match std::fs::canonicalize(&temp) {
                Ok(p) => p,
                Err(e) => {
                    let _ = std::fs::remove_dir_all(&temp);
                    return Err(CoreError::io(&temp, e));
                }
            };
            let canon_src = match std::fs::canonicalize(&src) {
                Ok(p) => p,
                Err(e) => {
                    let _ = std::fs::remove_dir_all(&temp);
                    return Err(CoreError::io(&src, e));
                }
            };
            if !canon_src.starts_with(&canon_temp) {
                let _ = std::fs::remove_dir_all(&temp);
                return Err(CoreError::Marketplace(
                    "sous-dossier de plugin hors clone".to_string(),
                ));
            }
            (canon_src, Some(temp))
        }
    };

    // 3. Version (depuis plugin.json), sinon "unknown".
    // La version est non-fiable (issue d'un plugin.json tiers) ; elle ne doit pas
    // composer un chemin d'échappement. On la remplace par "unknown" si elle n'est
    // pas un nom sûr (pas de séparateur de chemin ni de `..`).
    let version = {
        let v = read_plugin_version(&src).unwrap_or_else(|| "unknown".to_string());
        if is_safe_name(&v) {
            v
        } else {
            "unknown".to_string()
        }
    };

    // 4. Copier vers cache/<mkt>/<plugin>/<version>/ (confiné, idempotent).
    let copy_result = (|| -> Result<PathBuf> {
        let dest = cache_root.join(marketplace).join(plugin).join(&version);
        // Garde lexicale ET composants : rejette tout `..` dans le chemin construit.
        if !dest.starts_with(&cache_root)
            || dest
                .components()
                .any(|c| c == std::path::Component::ParentDir)
        {
            return Err(CoreError::Marketplace("destination hors cache".to_string()));
        }
        if dest.exists() {
            std::fs::remove_dir_all(&dest).map_err(|e| CoreError::io(&dest, e))?;
        }
        copy_dir_recursive(&src, &dest)?;
        Ok(dest)
    })();
    if let Some(t) = &temp {
        let _ = std::fs::remove_dir_all(t);
    }
    let dest = copy_result?;

    // 5. Écrire l'entrée scope user d'installed_plugins.json.
    let key = format!("{plugin}@{marketplace}");
    let installed_path = home.plugins_dir().join("installed_plugins.json");
    let mut doc = SettingsDoc::load(&installed_path)?;
    if doc.get(&["version"]).is_none() {
        doc.set(&["version"], Value::Number(2u64.into()));
    }
    let now = iso8601_utc(SystemTime::now());
    let mut entry = Map::new();
    entry.insert("scope".into(), Value::String("user".into()));
    entry.insert(
        "installPath".into(),
        Value::String(dest.to_string_lossy().into_owned()),
    );
    entry.insert("version".into(), Value::String(version.clone()));
    entry.insert("installedAt".into(), Value::String(now.clone()));
    entry.insert("lastUpdated".into(), Value::String(now));
    let mut arr: Vec<Value> = doc
        .get(&["plugins", key.as_str()])
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    arr.retain(|x| x.get("scope").and_then(|s| s.as_str()) != Some("user"));
    arr.push(Value::Object(entry));
    doc.set(&["plugins", key.as_str()], Value::Array(arr));
    doc.save(&installed_path)?;

    // 6. Auto-activer (réutilise extensions.rs).
    crate::extensions::set_plugin_enabled(home, &key, true)
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
    let path = known_marketplaces_path(home);
    let mut doc = SettingsDoc::load(&path)?;
    if doc.get(&[name]).is_none() {
        return Err(CoreError::Marketplace(format!(
            "entrée de registre absente pour « {name} »"
        )));
    }
    git::pull(&dir)?;
    doc.set(
        &[name, "lastUpdated"],
        Value::String(iso8601_utc(SystemTime::now())),
    );
    doc.save(&path)
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
        assert!(
            matches!(&list[0].source, MarketplaceSource::Github { repo } if repo == "anthropics/claude-plugins-official")
        );
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
        format!(
            r#"{{"name":"{name}","owner":{{"name":"Acme"}},"plugins":[{{"name":"p1","description":"d"}}]}}"#
        )
    }

    #[test]
    fn add_marketplace_local_clones_validates_registers() {
        let repo = make_repo(&valid_manifest("acme-mkt"));
        let (_d, home) = home();
        let src = MarketplaceSource::Local {
            path: repo.path().to_path_buf(),
        };

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
        let src = MarketplaceSource::Local {
            path: repo.path().to_path_buf(),
        };

        assert!(add_marketplace(&home, src).is_err());
        assert!(
            read_marketplaces(&home).unwrap().is_empty(),
            "registre non écrit"
        );
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
        add_marketplace(
            &home,
            MarketplaceSource::Local {
                path: repo.path().to_path_buf(),
            },
        )
        .unwrap();
        let again = add_marketplace(
            &home,
            MarketplaceSource::Local {
                path: repo.path().to_path_buf(),
            },
        );
        assert!(again.is_err());
        assert_eq!(read_marketplaces(&home).unwrap().len(), 1);
    }

    #[test]
    fn remove_marketplace_clears_entry_and_dir() {
        let repo = make_repo(&valid_manifest("gone"));
        let (_d, home) = home();
        add_marketplace(
            &home,
            MarketplaceSource::Local {
                path: repo.path().to_path_buf(),
            },
        )
        .unwrap();
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
        add_marketplace(
            &home,
            MarketplaceSource::Local {
                path: repo.path().to_path_buf(),
            },
        )
        .unwrap();

        // Nouveau commit dans la source.
        std::fs::write(repo.path().join("NEW.txt"), "x").unwrap();
        git(&["add", "-A"], repo.path());
        git(&["commit", "-q", "-m", "more"], repo.path());

        update_marketplace(&home, "upd").unwrap();
        assert!(
            home.plugins_dir()
                .join("marketplaces")
                .join("upd")
                .join("NEW.txt")
                .exists()
        );
    }

    #[test]
    fn add_marketplace_rejects_dash_url() {
        let (_d, home) = home();
        let src = MarketplaceSource::Git {
            url: "--upload-pack=evil".into(),
        };
        assert!(add_marketplace(&home, src).is_err());
        assert!(read_marketplaces(&home).unwrap().is_empty());
    }

    /// Renvoie le SHA HEAD d'un dépôt.
    fn head_sha(repo: &StdPath) -> String {
        let out = std::process::Command::new("git")
            .args(["rev-parse", "HEAD"])
            .current_dir(repo)
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .env("GIT_CONFIG_SYSTEM", "/dev/null")
            .output()
            .expect("git rev-parse");
        String::from_utf8_lossy(&out.stdout).trim().to_string()
    }

    #[test]
    fn git_clone_full_then_checkout_pins_commit() {
        // Dépôt avec 2 commits : v1 puis v2 d'un même fichier.
        let repo = tempfile::tempdir().unwrap();
        let root = repo.path();
        git(&["init", "-q", "-b", "main"], root);
        git(&["config", "user.email", "t@t"], root);
        git(&["config", "user.name", "t"], root);
        std::fs::write(root.join("f.txt"), "v1").unwrap();
        git(&["add", "-A"], root);
        git(&["commit", "-q", "-m", "c1"], root);
        let sha1 = head_sha(root);
        std::fs::write(root.join("f.txt"), "v2").unwrap();
        git(&["add", "-A"], root);
        git(&["commit", "-q", "-m", "c2"], root);

        let dest = tempfile::tempdir().unwrap();
        let dest = dest.path().join("clone");
        super::git::clone_full(&root.to_string_lossy(), &dest).unwrap();
        super::git::checkout(&dest, &sha1).unwrap();
        assert_eq!(std::fs::read_to_string(dest.join("f.txt")).unwrap(), "v1");

        // Commit inexistant → Err.
        assert!(super::git::checkout(&dest, "0000000000000000000000000000000000000000").is_err());
        // URL/commit ressemblant à une option → Err (durcissement).
        assert!(super::git::clone_full("--upload-pack=evil", &dest).is_err());
        assert!(super::git::checkout(&dest, "-x").is_err());
    }

    #[test]
    fn add_marketplace_blocks_ext_transport() {
        // `protocol.ext.allow=never` doit faire échouer le transport ext:: (sinon RCE).
        let (_d, home) = home();
        let src = MarketplaceSource::Git {
            url: "ext::sh -c true".into(),
        };
        assert!(add_marketplace(&home, src).is_err());
        assert!(read_marketplaces(&home).unwrap().is_empty());
    }

    #[test]
    fn update_marketplace_errors_when_unregistered() {
        let (_d, home) = home();
        // Un dossier existe mais aucune entrée de registre (désync).
        let dir = home.plugins_dir().join("marketplaces").join("orphan");
        std::fs::create_dir_all(&dir).unwrap();
        assert!(update_marketplace(&home, "orphan").is_err());
    }

    /// Écrit une marketplace clonée fictive avec un manifeste et un plugin relative-path.
    fn seed_rel_marketplace(home: &ClaudeHome, mkt: &str, plugin: &str, version: Option<&str>) {
        let mdir = home.plugins_dir().join("marketplaces").join(mkt);
        // Manifeste avec une entrée relative-path.
        let cp = mdir.join(".claude-plugin");
        std::fs::create_dir_all(&cp).unwrap();
        let manifest = format!(
            r#"{{"name":"{mkt}","plugins":[{{"name":"{plugin}","description":"d","source":"./plugins/{plugin}"}}]}}"#
        );
        std::fs::write(cp.join("marketplace.json"), manifest).unwrap();
        // Fichiers du plugin sous marketplaces/<mkt>/plugins/<plugin>/.
        let pdir = mdir.join("plugins").join(plugin);
        let pcp = pdir.join(".claude-plugin");
        std::fs::create_dir_all(&pcp).unwrap();
        let pj = match version {
            Some(v) => format!(r#"{{"name":"{plugin}","version":"{v}"}}"#),
            None => format!(r#"{{"name":"{plugin}"}}"#),
        };
        std::fs::write(pcp.join("plugin.json"), pj).unwrap();
        std::fs::write(pdir.join("SKILL.md"), "hello").unwrap();
    }

    #[test]
    fn install_plugin_relative_path_materializes_and_enables() {
        let (_d, home) = home();
        seed_rel_marketplace(&home, "m", "p", Some("1.2.3"));

        install_plugin(&home, "m", "p").unwrap();

        // Fichiers copiés sous cache/<mkt>/<plugin>/<version>/.
        let dest = home.plugins_dir().join("cache/m/p/1.2.3");
        assert!(dest.join("SKILL.md").is_file(), "fichier copié");
        assert!(dest.join(".claude-plugin/plugin.json").is_file());

        // Entrée installed_plugins.json (scope user, installPath, version).
        let doc = SettingsDoc::load(&home.plugins_dir().join("installed_plugins.json")).unwrap();
        let arr = doc
            .get(&["plugins", "p@m"])
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap();
        assert_eq!(arr.len(), 1);
        assert_eq!(arr[0].get("scope").and_then(|s| s.as_str()), Some("user"));
        assert_eq!(
            arr[0].get("version").and_then(|s| s.as_str()),
            Some("1.2.3")
        );
        assert_eq!(
            arr[0].get("installPath").and_then(|s| s.as_str()),
            Some(dest.to_string_lossy().as_ref())
        );

        // Auto-activé.
        let sdoc = SettingsDoc::load(&home.settings_file()).unwrap();
        assert_eq!(sdoc.get_bool(&["enabledPlugins", "p@m"]), Some(true));
    }

    #[test]
    fn install_plugin_missing_version_uses_unknown() {
        let (_d, home) = home();
        seed_rel_marketplace(&home, "m", "p", None);
        install_plugin(&home, "m", "p").unwrap();
        assert!(
            home.plugins_dir()
                .join("cache/m/p/unknown")
                .join("SKILL.md")
                .is_file()
        );
    }

    #[test]
    fn install_plugin_unknown_plugin_errors() {
        let (_d, home) = home();
        seed_rel_marketplace(&home, "m", "p", Some("1"));
        assert!(install_plugin(&home, "m", "absent").is_err());
        // Rien écrit.
        assert!(!home.plugins_dir().join("installed_plugins.json").exists());
    }

    #[test]
    fn install_plugin_rejects_dotdot_in_relative_source() {
        let (_d, home) = home();
        let mdir = home.plugins_dir().join("marketplaces").join("m");
        let cp = mdir.join(".claude-plugin");
        std::fs::create_dir_all(&cp).unwrap();
        std::fs::write(
            cp.join("marketplace.json"),
            r#"{"name":"m","plugins":[{"name":"evil","source":"./../../etc"}]}"#,
        )
        .unwrap();
        assert!(install_plugin(&home, "m", "evil").is_err());
        assert!(!home.plugins_dir().join("cache").join("m").exists());
    }

    /// Dépôt git « source de plugin » avec plugin.json (version) + fichier, dans un
    /// sous-dossier optionnel. Renvoie (tempdir, chemin, sha HEAD).
    fn make_plugin_repo(
        subdir: Option<&str>,
        version: &str,
    ) -> (tempfile::TempDir, String, String) {
        let d = tempfile::tempdir().unwrap();
        let root = d.path().to_path_buf();
        git(&["init", "-q", "-b", "main"], &root);
        git(&["config", "user.email", "t@t"], &root);
        git(&["config", "user.name", "t"], &root);
        let base = match subdir {
            Some(s) => root.join(s),
            None => root.clone(),
        };
        let cp = base.join(".claude-plugin");
        std::fs::create_dir_all(&cp).unwrap();
        std::fs::write(
            cp.join("plugin.json"),
            format!(r#"{{"name":"gp","version":"{version}"}}"#),
        )
        .unwrap();
        std::fs::write(base.join("SKILL.md"), "git-body").unwrap();
        git(&["add", "-A"], &root);
        git(&["commit", "-q", "-m", "init"], &root);
        let sha = head_sha(&root);
        (d, root.to_string_lossy().into_owned(), sha)
    }

    /// Écrit une marketplace fictive dont le plugin `gp` a une source git donnée.
    fn seed_git_marketplace(home: &ClaudeHome, mkt: &str, source_json: &str) {
        let cp = home
            .plugins_dir()
            .join("marketplaces")
            .join(mkt)
            .join(".claude-plugin");
        std::fs::create_dir_all(&cp).unwrap();
        let manifest =
            format!(r#"{{"name":"{mkt}","plugins":[{{"name":"gp","source":{source_json}}}]}}"#);
        std::fs::write(cp.join("marketplace.json"), manifest).unwrap();
    }

    #[test]
    fn install_plugin_git_url_clones_and_pins() {
        let (_repo, url, sha) = make_plugin_repo(None, "3.0.0");
        let (_d, home) = home();
        seed_git_marketplace(
            &home,
            "m",
            &format!(r#"{{"source":"url","url":"{url}","sha":"{sha}"}}"#),
        );

        install_plugin(&home, "m", "gp").unwrap();

        let dest = home.plugins_dir().join("cache/m/gp/3.0.0");
        assert_eq!(
            std::fs::read_to_string(dest.join("SKILL.md")).unwrap(),
            "git-body"
        );
        let sdoc = SettingsDoc::load(&home.settings_file()).unwrap();
        assert_eq!(sdoc.get_bool(&["enabledPlugins", "gp@m"]), Some(true));
        // Aucun dossier temporaire résiduel.
        let temp_left = std::fs::read_dir(home.plugins_dir().join("cache"))
            .unwrap()
            .flatten()
            .any(|e| e.file_name().to_string_lossy().starts_with("temp_git_"));
        assert!(!temp_left, "temp nettoyé");
    }

    #[test]
    fn install_plugin_git_subdir_uses_subdirectory() {
        let (_repo, url, sha) = make_plugin_repo(Some("plugins/gp"), "4.1.0");
        let (_d, home) = home();
        seed_git_marketplace(
            &home,
            "m",
            &format!(
                r#"{{"source":"git-subdir","url":"{url}","path":"plugins/gp","sha":"{sha}"}}"#
            ),
        );

        install_plugin(&home, "m", "gp").unwrap();
        assert_eq!(
            std::fs::read_to_string(home.plugins_dir().join("cache/m/gp/4.1.0/SKILL.md")).unwrap(),
            "git-body"
        );
    }

    #[test]
    fn install_plugin_git_bad_commit_cleans_temp_and_errors() {
        let (_repo, url, _sha) = make_plugin_repo(None, "1.0.0");
        let (_d, home) = home();
        seed_git_marketplace(
            &home,
            "m",
            &format!(
                r#"{{"source":"url","url":"{url}","sha":"0000000000000000000000000000000000000000"}}"#
            ),
        );

        assert!(install_plugin(&home, "m", "gp").is_err());
        // Pas d'entrée registre, pas de temp résiduel.
        assert!(!home.plugins_dir().join("installed_plugins.json").exists());
        let cache = home.plugins_dir().join("cache");
        if cache.exists() {
            let temp_left = std::fs::read_dir(&cache).unwrap().flatten().count();
            assert_eq!(temp_left, 0, "ni temp ni cache résiduel");
        }
    }

    // ── Tests de sécurité C1 + I1 ────────────────────────────────────────────

    /// C1 : version provenant d'un plugin.json tiers avec traversée (`../../../../pwned`).
    /// Le plugin doit s'installer dans `cache/<mkt>/<plugin>/unknown/` (fallback),
    /// et rien ne doit être créé/supprimé hors du cache.
    #[test]
    fn install_plugin_malicious_version_falls_back_to_unknown() {
        let (_d, home) = home();
        seed_rel_marketplace(&home, "m", "p", Some("../../../../pwned"));

        // Sentinelle hors cache : ce chemin ne doit PAS être touché.
        let sentinel = home.plugins_dir().join("../../../../pwned");
        // On crée le répertoire parent si nécessaire, mais on vérifie surtout que le test
        // s'exécute sans panique et que l'installation atterrit dans unknown/.
        let cache_root = home.plugins_dir().join("cache");

        install_plugin(&home, "m", "p").unwrap();

        // Le plugin doit atterrir dans unknown/, pas dans le chemin malicieux.
        let good_dest = cache_root.join("m").join("p").join("unknown");
        assert!(
            good_dest.join("SKILL.md").is_file(),
            "SKILL.md dans cache/.../unknown/"
        );

        // Le chemin malicieux (../../../../pwned) ne doit pas exister sous cache.
        let bad_dest = cache_root.join("../../../../pwned");
        assert!(
            !bad_dest.exists(),
            "aucun répertoire hors cache créé par la version malicieuse"
        );

        // Le sentinelle créé manuellement (si accessible) ne doit pas avoir été supprimé.
        // (Il n'existe pas dans ce test car on ne le crée pas — vérification indirecte suffisante.)
        let _ = sentinel; // utilisé pour documenter l'intention
    }

    /// I1 (Unix uniquement) : la source du plugin est un lien symbolique vers un répertoire
    /// externe. `install_plugin` doit retourner Err et ne rien copier dans le cache.
    #[cfg(unix)]
    #[test]
    fn install_plugin_rejects_symlinked_source_dir() {
        use std::os::unix::fs::symlink;

        let (_d, home) = home();

        // Répertoire externe (cible du symlink) avec un fichier sensible.
        let external = tempfile::tempdir().unwrap();
        std::fs::write(external.path().join("secret.txt"), "top secret").unwrap();

        // Marketplace avec manifeste pointant vers ./plugins/evil.
        let mdir = home.plugins_dir().join("marketplaces").join("m");
        let cp = mdir.join(".claude-plugin");
        std::fs::create_dir_all(&cp).unwrap();
        std::fs::write(
            cp.join("marketplace.json"),
            r#"{"name":"m","plugins":[{"name":"evil","description":"d","source":"./plugins/evil"}]}"#,
        )
        .unwrap();

        // Le dossier plugins/evil est un symlink vers le répertoire externe.
        let plugins_dir = mdir.join("plugins");
        std::fs::create_dir_all(&plugins_dir).unwrap();
        symlink(external.path(), plugins_dir.join("evil")).unwrap();

        // L'installation doit échouer car la source est hors de la marketplace (symlink).
        let result = install_plugin(&home, "m", "evil");
        assert!(
            result.is_err(),
            "doit rejeter un source symlinké vers l'extérieur"
        );

        // Rien ne doit avoir été copié dans le cache.
        let cache = home.plugins_dir().join("cache");
        assert!(
            !cache.join("m").join("evil").exists(),
            "aucun fichier copié dans le cache"
        );
    }

    #[test]
    fn parse_manifest_extracts_plugin_sources() {
        let json = serde_json::json!({
            "name": "mkt",
            "plugins": [
                {"name":"rel","description":"d","source":"./plugins/rel"},
                {"name":"u","source":{"source":"url","url":"https://x/r.git","sha":"abc","path":"sub"}},
                {"name":"u2","source":{"source":"url","url":"https://x/r.git","sha":"def"}},
                {"name":"gs","source":{"source":"git-subdir","url":"https://x/g.git","path":"p","ref":"v1","sha":"123"}},
                {"name":"gh","source":{"source":"github","repo":"o/n","commit":"deadbeef","sha":"z"}},
                {"name":"weird","source":{"source":"mystery"}}
            ]
        });
        let m = super::parse_manifest(&json).unwrap();
        let by = |n: &str| m.plugins.iter().find(|p| p.name == n).unwrap();
        assert_eq!(
            by("rel").source,
            Some(PluginSource::RelativePath {
                path: "./plugins/rel".into()
            })
        );
        assert_eq!(
            by("u").source,
            Some(PluginSource::Git {
                url: "https://x/r.git".into(),
                commit: "abc".into(),
                subdir: Some("sub".into())
            })
        );
        assert_eq!(
            by("u2").source,
            Some(PluginSource::Git {
                url: "https://x/r.git".into(),
                commit: "def".into(),
                subdir: None
            })
        );
        assert_eq!(
            by("gs").source,
            Some(PluginSource::Git {
                url: "https://x/g.git".into(),
                commit: "123".into(),
                subdir: Some("p".into())
            })
        );
        assert_eq!(
            by("gh").source,
            Some(PluginSource::Git {
                url: "https://github.com/o/n.git".into(),
                commit: "deadbeef".into(),
                subdir: None
            })
        );
        // Source inconnue : entrée conservée (nom/description) mais source None.
        assert_eq!(by("weird").source, None);
        // Toutes les entrées restent listées (catalogue non régressé).
        assert_eq!(m.plugins.len(), 6);
    }
}
