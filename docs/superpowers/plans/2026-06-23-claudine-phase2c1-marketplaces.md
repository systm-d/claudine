# Phase 2c-1 — Marketplaces & socle de gestion des plugins — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Gérer les marketplaces de plugins Claude (ajouter / lister / retirer / mettre à jour) depuis la section Extensions du TUI, en clonant les dépôts via le binaire `git` système et en écrivant `plugins/known_marketplaces.json` du home actif.

**Architecture:** Un nouveau module cœur `claudine-core/src/marketplaces.rs` modélise les sources, le registre (`known_marketplaces.json`) et le manifeste (`.claude-plugin/marketplace.json`), et expose `read/add/remove/update_marketplace` — le clonage est délégué à `std::process::Command::new("git")` (aucune dépendance de build, MSRV 1.74 inchangée). Le TUI ajoute un gestionnaire modal (`tui/marketplaces.rs`) ouvert par `g` depuis Extensions ; les opérations réseau (ajout/màj) tournent dans un thread (`std::thread` + `mpsc`), la boucle d'évènements passant en `event::poll` avec un indicateur tant qu'un job est en cours.

**Tech Stack:** Rust (workspace 2 crates), ratatui 0.28 (`crossterm` via `ratatui::crossterm`), serde_json (`preserve_order`), `git` système, tests via `tempfile`.

## Global Constraints

- MSRV **1.74**, édition 2021. **Aucune nouvelle dépendance** (clonage délégué au binaire `git`).
- `crates/claudine-core` ne dépend d'aucune lib d'UI.
- Écriture de `known_marketplaces.json` : **toujours via `SettingsDoc`** (backup `.bak-<nanos>` + temp+rename + `preserve_order`). Jamais d'écriture JSON brute du registre.
- La **clé du registre = le `name` du manifeste** (connu seulement après clonage). Garde-fou sur ce nom : pas de `/`, `\`, `..`, ni `.`/`..` seuls.
- Style **formaté à la main** ; valider via `cargo clippy --workspace` (0 warning) + `cargo test --workspace`. **Ne jamais** lancer `cargo fmt`.
- Un seul job réseau à la fois ; pendant un job, les touches de mutation (`a`/`u`/`d`) sont ignorées.
- Opère sur le **home actif** (`app.home()`).

---

### Task 1: Cœur — modèle, parsing des sources, lecture du registre + manifeste, horodatage

**Files:**
- Create: `crates/claudine-core/src/marketplaces.rs`
- Modify: `crates/claudine-core/src/error.rs` (ajout d'une variante)
- Modify: `crates/claudine-core/src/lib.rs` (déclaration + ré-exports)

**Interfaces:**
- Consumes: `ClaudeHome` (a `plugins_dir()`), `SettingsDoc`, `CoreError`/`Result`.
- Produces (utilisés Tasks 2-5) :
  - `pub enum MarketplaceSource { Github { repo: String }, Git { url: String }, Local { path: PathBuf } }`
    avec `MarketplaceSource::parse(&str) -> Option<Self>`, `clone_url(&self) -> String`, `provisional_name(&self) -> String`.
  - `pub struct Marketplace { pub name: String, pub source: MarketplaceSource, pub install_location: PathBuf, pub last_updated: String }`
  - `pub struct MarketplaceManifest { pub name: String, pub description: Option<String>, pub owner_name: Option<String>, pub plugins: Vec<PluginManifestEntry> }`
  - `pub struct PluginManifestEntry { pub name: String, pub description: Option<String> }`
  - `pub fn read_marketplaces(home: &ClaudeHome) -> Result<Vec<Marketplace>>`
  - `pub fn read_marketplace_manifest(home: &ClaudeHome, name: &str) -> Result<MarketplaceManifest>`
  - `pub fn iso8601_utc(t: std::time::SystemTime) -> String`
  - `CoreError::Marketplace(String)`

- [ ] **Step 1: Write the failing test**

Créer `crates/claudine-core/src/marketplaces.rs` avec, en bas, le module de tests :

```rust
#[cfg(test)]
mod tests {
    use super::*;
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
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p claudine-core marketplaces`
Expected: FAIL — module/types/fonctions inconnus.

- [ ] **Step 3: Write minimal implementation**

En tête de `crates/claudine-core/src/marketplaces.rs` :

```rust
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
```

Dans `crates/claudine-core/src/error.rs`, ajouter une variante à l'`enum CoreError` (après `BundleFormat`) :

```rust
    #[error("{0}")]
    Marketplace(String),
```

Dans `crates/claudine-core/src/lib.rs` :
1. Déclarer le module (après `pub mod extensions;`) : `pub mod marketplaces;`.
2. Ré-exporter (après le bloc `pub use extensions::{...};`) :

```rust
pub use marketplaces::{
    add_marketplace, iso8601_utc, read_marketplace_manifest, read_marketplaces,
    remove_marketplace, update_marketplace, Marketplace, MarketplaceManifest, MarketplaceSource,
    PluginManifestEntry,
};
```

> Note : `add_marketplace`/`remove_marketplace`/`update_marketplace` sont définies en Task 2. Si la compilation du ré-export échoue à cette étape, ajoute-les en Task 2 ; pour garder Task 1 compilable seule, tu peux d'abord ne ré-exporter que ce qui existe (`iso8601_utc, read_marketplace_manifest, read_marketplaces, Marketplace, MarketplaceManifest, MarketplaceSource, PluginManifestEntry`) et compléter le ré-export en Task 2.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p claudine-core marketplaces`
Expected: PASS (6 tests).

- [ ] **Step 5: Commit**

```bash
git add crates/claudine-core/src/marketplaces.rs crates/claudine-core/src/error.rs crates/claudine-core/src/lib.rs
git commit -m "feat(core): socle marketplaces — modèle, parsing, lecture registre/manifeste, iso8601"
```

---

### Task 2: Cœur — helper git + add / remove / update

**Files:**
- Modify: `crates/claudine-core/src/marketplaces.rs`
- Modify: `crates/claudine-core/src/lib.rs` (compléter le ré-export si Task 1 l'avait réduit)

**Interfaces:**
- Consumes: tout de la Task 1 (`MarketplaceSource`, `Marketplace`, `read_manifest_at`, `marketplaces_dir`, `known_marketplaces_path`, `is_safe_name`, `iso8601_utc`, `SettingsDoc`).
- Produces:
  - `pub fn add_marketplace(home: &ClaudeHome, source: MarketplaceSource) -> Result<Marketplace>`
  - `pub fn remove_marketplace(home: &ClaudeHome, name: &str) -> Result<()>`
  - `pub fn update_marketplace(home: &ClaudeHome, name: &str) -> Result<()>`

- [ ] **Step 1: Write the failing test**

Ajouter au module `tests` de `marketplaces.rs` :

```rust
    use std::path::Path as StdPath;

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
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p claudine-core marketplaces`
Expected: FAIL — `add_marketplace`/`remove_marketplace`/`update_marketplace` inconnues.

- [ ] **Step 3: Write minimal implementation**

Ajouter dans `marketplaces.rs` (après `read_manifest_at`, par ex.) le sous-module git et les trois opérations :

```rust
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

    /// `git clone --depth 1 <url> <dest>`.
    pub fn clone(url: &str, dest: &Path) -> Result<()> {
        let mut c = Command::new("git");
        c.arg("clone").arg("--depth").arg("1").arg(url).arg(dest);
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
    git::clone(&source.clone_url(), &tmp)?;

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
```

Si tu avais réduit le ré-export en Task 1, complète-le maintenant dans `lib.rs` pour inclure `add_marketplace, remove_marketplace, update_marketplace`.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p claudine-core marketplaces && cargo clippy -p claudine-core`
Expected: tests PASS, 0 warning.

> Si `cargo test` échoue avec « git doit être installé » : `git` est requis pour ces tests (il l'est en CI). Vérifie `git --version`.

- [ ] **Step 5: Commit**

```bash
git add crates/claudine-core/src/marketplaces.rs crates/claudine-core/src/lib.rs
git commit -m "feat(core): add/remove/update marketplace (clone via git système)"
```

---

### Task 3: TUI — état du gestionnaire de marketplaces

**Files:**
- Create: `crates/claudine/src/tui/marketplaces.rs`
- Modify: `crates/claudine/src/tui/mod.rs` (déclarer `pub mod marketplaces;`)

**Interfaces:**
- Consumes: `claudine_core::Marketplace`.
- Produces (utilisés Tasks 4-5) :
  - `pub enum MktMode { List, AddInput }`
  - `pub struct MarketplacesManager { pub items: Vec<Marketplace>, pub idx: usize, pub mode: MktMode, pub input: String, pub confirm_remove: bool }`
  - `MarketplacesManager::new(Vec<Marketplace>) -> Self`, `set_items(Vec<Marketplace>)`, `move_sel(i32)`, `selected_name() -> Option<String>`, `begin_add()`, `cancel_add()`, `begin_remove()`.

- [ ] **Step 1: Write the failing test**

Créer `crates/claudine/src/tui/marketplaces.rs` avec, en bas, le module de tests :

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use claudine_core::{Marketplace, MarketplaceSource};

    fn mk(name: &str) -> Marketplace {
        Marketplace {
            name: name.into(),
            source: MarketplaceSource::Github { repo: "o/r".into() },
            install_location: std::path::PathBuf::from("/x"),
            last_updated: "2026-01-01T00:00:00.000Z".into(),
        }
    }

    #[test]
    fn navigation_is_clamped() {
        let mut m = MarketplacesManager::new(vec![mk("a"), mk("b")]);
        assert_eq!(m.idx, 0);
        m.move_sel(-1);
        assert_eq!(m.idx, 0);
        m.move_sel(1);
        assert_eq!(m.idx, 1);
        m.move_sel(1);
        assert_eq!(m.idx, 1);
        assert_eq!(m.selected_name().as_deref(), Some("b"));
    }

    #[test]
    fn set_items_reclamps_idx() {
        let mut m = MarketplacesManager::new(vec![mk("a"), mk("b"), mk("c")]);
        m.move_sel(1);
        m.move_sel(1); // idx = 2
        m.set_items(vec![mk("a")]);
        assert_eq!(m.idx, 0);
        assert_eq!(m.selected_name().as_deref(), Some("a"));
    }

    #[test]
    fn add_and_remove_flow_state() {
        let mut m = MarketplacesManager::new(vec![mk("a")]);
        m.begin_add();
        assert_eq!(m.mode, MktMode::AddInput);
        m.input.push_str("o/r");
        m.cancel_add();
        assert_eq!(m.mode, MktMode::List);
        assert!(m.input.is_empty());

        m.begin_remove();
        assert!(m.confirm_remove);
    }

    #[test]
    fn begin_remove_noop_when_empty() {
        let mut m = MarketplacesManager::new(vec![]);
        m.begin_remove();
        assert!(!m.confirm_remove);
        assert!(m.selected_name().is_none());
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p claudine marketplaces`
Expected: FAIL — module/types inconnus.

- [ ] **Step 3: Write minimal implementation**

En tête de `crates/claudine/src/tui/marketplaces.rs` :

```rust
//! Gestionnaire de marketplaces (modal) : liste, ajout (saisie de source),
//! retrait (confirmation), mise à jour. Les opérations réseau (ajout/màj) sont
//! exécutées en arrière-plan par `app.rs` ; ce module ne porte que l'état UI.

use claudine_core::Marketplace;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MktMode {
    List,
    AddInput,
}

pub struct MarketplacesManager {
    pub items: Vec<Marketplace>,
    pub idx: usize,
    pub mode: MktMode,
    pub input: String,
    pub confirm_remove: bool,
}

impl MarketplacesManager {
    pub fn new(items: Vec<Marketplace>) -> Self {
        Self {
            items,
            idx: 0,
            mode: MktMode::List,
            input: String::new(),
            confirm_remove: false,
        }
    }

    /// Remplace la liste (après une opération) en bornant l'index courant.
    pub fn set_items(&mut self, items: Vec<Marketplace>) {
        self.items = items;
        if self.idx >= self.items.len() {
            self.idx = self.items.len().saturating_sub(1);
        }
    }

    /// Déplacement borné dans [0, len) (pas de bouclage).
    pub fn move_sel(&mut self, delta: i32) {
        if self.items.is_empty() {
            return;
        }
        let max = self.items.len() - 1;
        self.idx = if delta < 0 {
            self.idx.saturating_sub((-delta) as usize)
        } else {
            (self.idx + delta as usize).min(max)
        };
    }

    pub fn selected_name(&self) -> Option<String> {
        self.items.get(self.idx).map(|m| m.name.clone())
    }

    pub fn begin_add(&mut self) {
        self.mode = MktMode::AddInput;
        self.input.clear();
    }

    pub fn cancel_add(&mut self) {
        self.mode = MktMode::List;
        self.input.clear();
    }

    pub fn begin_remove(&mut self) {
        if !self.items.is_empty() {
            self.confirm_remove = true;
        }
    }
}
```

Déclarer le module dans `crates/claudine/src/tui/mod.rs` (à côté de `pub mod mcp_editor;`) :

```rust
pub mod marketplaces;
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p claudine marketplaces`
Expected: PASS (4 tests).

- [ ] **Step 5: Commit**

```bash
git add crates/claudine/src/tui/marketplaces.rs crates/claudine/src/tui/mod.rs
git commit -m "feat(tui): état du gestionnaire de marketplaces"
```

---

### Task 4: TUI — câblage app + concurrence (thread d'arrière-plan) + routage clavier

**Files:**
- Modify: `crates/claudine/src/tui/app.rs`
- Modify: `crates/claudine/src/tui/mod.rs`

**Interfaces:**
- Consumes: `MarketplacesManager`/`MktMode` (Task 3), `claudine_core::{add_marketplace, read_marketplaces, remove_marketplace, update_marketplace, MarketplaceSource}`.
- Produces:
  - champs `pub marketplaces: Option<MarketplacesManager>` et `pub mkt_job: Option<MktJob>` sur `App` ;
  - types `pub struct MktJob { pub label: String, pub frame: u8, rx: Receiver<MktOutcome> }` et `struct MktOutcome { result: std::result::Result<String, String> }` ;
  - méthodes `open_marketplaces`, `marketplaces_cancel`, `mkt_begin_add(&str)`, `mkt_begin_update`, `mkt_remove_confirmed`, `mkt_job_active() -> bool`, `tick_mkt_job()` ;
  - fonction libre `handle_marketplaces_key(&mut App, KeyEvent)` dans `mod.rs`.

- [ ] **Step 1: Write the failing test**

Ajouter dans le module `tests` d'`app.rs` :

```rust
    #[test]
    fn marketplaces_open_lists_existing() {
        let dir = tempfile::tempdir().unwrap();
        let base = dir.path();
        fs::create_dir_all(base.join("projects")).unwrap();
        // Un registre minimal sur disque.
        let reg = r#"{"m1":{"source":{"source":"github","repo":"o/r"},"installLocation":"/x/m1","lastUpdated":"2026-01-01T00:00:00.000Z"}}"#;
        let p = base.join("plugins").join("known_marketplaces.json");
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(&p, reg).unwrap();

        let mut app = App::with_homes(vec![ClaudeHome::from_base(base)]);
        app.set_section(Section::Extensions);
        app.open_marketplaces();
        let m = app.marketplaces.as_ref().unwrap();
        assert_eq!(m.items.len(), 1);
        assert_eq!(m.items[0].name, "m1");
    }

    #[test]
    fn marketplaces_remove_confirmed_is_synchronous() {
        let dir = tempfile::tempdir().unwrap();
        let base = dir.path();
        fs::create_dir_all(base.join("projects")).unwrap();
        let reg = r#"{"gone":{"source":{"source":"github","repo":"o/r"},"installLocation":"/x/gone","lastUpdated":"2026-01-01T00:00:00.000Z"}}"#;
        let p = base.join("plugins").join("known_marketplaces.json");
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(&p, reg).unwrap();
        // Dossier de la marketplace à supprimer.
        fs::create_dir_all(base.join("plugins").join("marketplaces").join("gone")).unwrap();

        let mut app = App::with_homes(vec![ClaudeHome::from_base(base)]);
        app.set_section(Section::Extensions);
        app.open_marketplaces();
        app.marketplaces.as_mut().unwrap().begin_remove();
        app.mkt_remove_confirmed();

        assert!(app.marketplaces.as_ref().unwrap().items.is_empty());
        assert!(!base.join("plugins").join("marketplaces").join("gone").exists());
    }

    #[test]
    fn mkt_job_tick_applies_outcome_and_clears() {
        let dir = tempfile::tempdir().unwrap();
        let base = dir.path();
        fs::create_dir_all(base.join("projects")).unwrap();
        let mut app = App::with_homes(vec![ClaudeHome::from_base(base)]);
        app.set_section(Section::Extensions);
        app.open_marketplaces();

        // Injecte un job déjà terminé.
        let (tx, rx) = std::sync::mpsc::channel();
        tx.send(MktOutcome { result: Ok("ajoutée".to_string()) }).unwrap();
        app.mkt_job = Some(MktJob { label: "ajout".into(), frame: 0, rx });
        assert!(app.mkt_job_active());

        app.tick_mkt_job();
        assert!(!app.mkt_job_active(), "job effacé après réception");
        assert_eq!(app.status.as_deref(), Some("ajoutée"));
    }

    #[test]
    fn mkt_begin_add_rejects_unparseable_source() {
        let dir = tempfile::tempdir().unwrap();
        let base = dir.path();
        fs::create_dir_all(base.join("projects")).unwrap();
        let mut app = App::with_homes(vec![ClaudeHome::from_base(base)]);
        app.set_section(Section::Extensions);
        app.open_marketplaces();
        app.marketplaces.as_mut().unwrap().begin_add();

        app.mkt_begin_add("source bidon !!");
        assert!(app.mkt_job.is_none(), "aucun job lancé");
        assert!(app.status.as_deref().unwrap().contains("non reconnue"));
        // Retour en mode liste.
        assert_eq!(app.marketplaces.as_ref().unwrap().mode, crate::tui::marketplaces::MktMode::List);
    }
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p claudine marketplaces_open marketplaces_remove mkt_job_tick mkt_begin_add`
Expected: FAIL — champs/types/méthodes inconnus.

- [ ] **Step 3: Write minimal implementation**

Dans `crates/claudine/src/tui/app.rs` :

1. Imports — ajouter à `use claudine_core::{...}` : `add_marketplace, read_marketplaces, remove_marketplace, update_marketplace, MarketplaceSource`. Et après les autres `use crate::tui::...` : `use crate::tui::marketplaces::MarketplacesManager;`.

2. Types de job — ajouter près du haut du fichier (après les `use`, avant `struct App`) :

```rust
/// Résultat d'une opération marketplace exécutée en arrière-plan (message prêt à afficher).
pub struct MktOutcome {
    pub result: std::result::Result<String, String>,
}

/// Un job marketplace en cours (clone/pull) dans un thread.
pub struct MktJob {
    pub label: String,
    pub frame: u8,
    pub rx: std::sync::mpsc::Receiver<MktOutcome>,
}
```

3. Champs `App` (près de `mcp_editor`) :

```rust
    pub marketplaces: Option<MarketplacesManager>,
    pub mkt_job: Option<MktJob>,
```

4. Init dans `with_homes` (près de `mcp_editor: None,`) :

```rust
            marketplaces: None,
            mkt_job: None,
```

5. Méthodes (dans `impl App`, près des méthodes `mcp_*`) :

```rust
    /// Ouvre le gestionnaire de marketplaces du home actif (depuis Extensions).
    pub fn open_marketplaces(&mut self) {
        if self.section != Section::Extensions {
            return;
        }
        let items = read_marketplaces(self.home()).unwrap_or_default();
        self.marketplaces = Some(MarketplacesManager::new(items));
    }

    pub fn marketplaces_cancel(&mut self) {
        self.marketplaces = None;
    }

    /// Lance l'ajout d'une marketplace en arrière-plan. Source illisible → statut d'erreur.
    pub fn mkt_begin_add(&mut self, input: &str) {
        if let Some(m) = self.marketplaces.as_mut() {
            m.cancel_add();
        }
        let Some(source) = MarketplaceSource::parse(input) else {
            self.status = Some(format!("Source non reconnue : {input}"));
            return;
        };
        let home = self.home().clone();
        let label = format!("ajout de {input}");
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let result = add_marketplace(&home, source)
                .map(|mp| format!("marketplace « {} » ajoutée", mp.name))
                .map_err(|e| e.to_string());
            let _ = tx.send(MktOutcome { result });
        });
        self.mkt_job = Some(MktJob { label, frame: 0, rx });
    }

    /// Lance la mise à jour de la marketplace sélectionnée en arrière-plan.
    pub fn mkt_begin_update(&mut self) {
        let Some(name) = self.marketplaces.as_ref().and_then(|m| m.selected_name()) else {
            return;
        };
        let home = self.home().clone();
        let label = format!("mise à jour de {name}");
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let result = update_marketplace(&home, &name)
                .map(|()| format!("marketplace « {name} » mise à jour"))
                .map_err(|e| e.to_string());
            let _ = tx.send(MktOutcome { result });
        });
        self.mkt_job = Some(MktJob { label, frame: 0, rx });
    }

    /// Retire la marketplace sélectionnée (opération locale, synchrone).
    pub fn mkt_remove_confirmed(&mut self) {
        let name = {
            let Some(m) = self.marketplaces.as_mut() else {
                return;
            };
            m.confirm_remove = false;
            match m.selected_name() {
                Some(n) => n,
                None => return,
            }
        };
        let home = self.home().clone();
        match remove_marketplace(&home, &name) {
            Ok(()) => {
                let items = read_marketplaces(&home).unwrap_or_default();
                if let Some(m) = self.marketplaces.as_mut() {
                    m.set_items(items);
                }
                self.status = Some(format!("marketplace « {name} » retirée"));
            }
            Err(e) => self.status = Some(format!("Échec retrait : {e}")),
        }
    }

    pub fn mkt_job_active(&self) -> bool {
        self.mkt_job.is_some()
    }

    /// Avance le spinner et applique le résultat du job s'il est arrivé.
    pub fn tick_mkt_job(&mut self) {
        let Some(job) = self.mkt_job.as_mut() else {
            return;
        };
        job.frame = job.frame.wrapping_add(1);
        let outcome = match job.rx.try_recv() {
            Ok(o) => o,
            Err(std::sync::mpsc::TryRecvError::Empty) => return,
            Err(std::sync::mpsc::TryRecvError::Disconnected) => MktOutcome {
                result: Err("le job s'est interrompu".to_string()),
            },
        };
        self.mkt_job = None;
        let home = self.home().clone();
        let items = read_marketplaces(&home).unwrap_or_default();
        if let Some(m) = self.marketplaces.as_mut() {
            m.set_items(items);
        }
        self.status = Some(match outcome.result {
            Ok(msg) => msg,
            Err(e) => format!("Échec : {e}"),
        });
    }
```

Dans `crates/claudine/src/tui/mod.rs` :

1. Capture de la modale — ajouter (à côté de la capture `mcp_editor`, ~ligne 196) :

```rust
    // Gestionnaire de marketplaces (modal).
    if app.marketplaces.is_some() {
        handle_marketplaces_key(app, key);
        return;
    }
```

2. Ouverture par `g` en section Extensions — dans le `match key.code` principal, ajouter une branche (près de `KeyCode::Char('p')`) :

```rust
        KeyCode::Char('g') => app.open_marketplaces(),
```

3. Fonction de routage (près de `handle_mcp_editor_key`), motif d'action différée pour éviter le double emprunt de `app` :

```rust
fn handle_marketplaces_key(app: &mut App, key: KeyEvent) {
    use crate::tui::marketplaces::MktMode;
    enum Deferred {
        Add(String),
        Update,
        Remove,
        Cancel,
    }
    // `busy` lu avant d'emprunter `app.marketplaces` (évite le conflit d'emprunt).
    let busy = app.mkt_job.is_some();
    let deferred: Option<Deferred>;
    {
        let Some(m) = app.marketplaces.as_mut() else {
            return;
        };
        if m.confirm_remove {
            deferred = match key.code {
                KeyCode::Char('o') | KeyCode::Char('O') | KeyCode::Char('y') | KeyCode::Char('Y')
                | KeyCode::Enter => Some(Deferred::Remove),
                KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                    m.confirm_remove = false;
                    None
                }
                _ => None,
            };
        } else if m.mode == MktMode::AddInput {
            deferred = match key.code {
                KeyCode::Esc => {
                    m.cancel_add();
                    None
                }
                KeyCode::Enter => Some(Deferred::Add(m.input.clone())),
                KeyCode::Backspace => {
                    m.input.pop();
                    None
                }
                KeyCode::Char(c) => {
                    m.input.push(c);
                    None
                }
                _ => None,
            };
        } else {
            deferred = match key.code {
                KeyCode::Up | KeyCode::Char('k') => {
                    m.move_sel(-1);
                    None
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    m.move_sel(1);
                    None
                }
                KeyCode::Char('a') if !busy => {
                    m.begin_add();
                    None
                }
                KeyCode::Char('u') if !busy => Some(Deferred::Update),
                KeyCode::Char('d') if !busy => {
                    m.begin_remove();
                    None
                }
                KeyCode::Esc => Some(Deferred::Cancel),
                _ => None,
            };
        }
    }
    match deferred {
        Some(Deferred::Add(src)) => app.mkt_begin_add(&src),
        Some(Deferred::Update) => app.mkt_begin_update(),
        Some(Deferred::Remove) => app.mkt_remove_confirmed(),
        Some(Deferred::Cancel) => app.marketplaces_cancel(),
        None => {}
    }
}
```

4. Boucle d'évènements — remplacer le bloc `match event::read()? { ... }` de `event_loop` par une variante qui ne bloque pas pendant un job (poll + tick) :

```rust
        if app.mkt_job_active() {
            if event::poll(std::time::Duration::from_millis(120))? {
                if let Event::Key(key) = event::read()? {
                    if key.kind == KeyEventKind::Press {
                        handle_key(&mut app, key);
                    }
                }
            }
            app.tick_mkt_job();
        } else {
            match event::read()? {
                Event::Key(key) if key.kind == KeyEventKind::Press => handle_key(&mut app, key),
                Event::Resize(_, _) => {}
                _ => {}
            }
        }
```

(Le bloc `if let Some(path) = app.pending_edit.take() { ... }` qui suit reste inchangé.)

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p claudine marketplaces_open marketplaces_remove mkt_job_tick mkt_begin_add && cargo clippy -p claudine`
Expected: tests PASS, 0 warning.

- [ ] **Step 5: Commit**

```bash
git add crates/claudine/src/tui/app.rs crates/claudine/src/tui/mod.rs
git commit -m "feat(tui): câblage marketplaces + jobs git en arrière-plan (g)"
```

---

### Task 5: TUI — rendu du modal + indicateur + footer/aide + vérification finale

**Files:**
- Modify: `crates/claudine/src/tui/ui.rs`

**Interfaces:**
- Consumes: `MarketplacesManager`/`MktMode` (Task 3), `app.mkt_job` (Task 4), `claudine_core::MarketplaceSource`.
- Produces: `fn render_marketplaces(&App, &mut Frame, Rect)` ; footer + aide mis à jour.

- [ ] **Step 1: Ajouter le rendu du modal**

Dans `crates/claudine/src/tui/ui.rs` :

1. Imports — ajouter `MarketplaceSource` à `use claudine_core::{...}` (à côté de `McpTransport`), et après les autres `use crate::tui::...` : `use crate::tui::marketplaces::MktMode;`.

2. Dispatch — dans `render(...)`, après le bloc `if app.mcp_editor.is_some() { render_mcp_editor(...); }` :

```rust
    if app.marketplaces.is_some() {
        render_marketplaces(app, f, area);
    }
```

3. La fonction de rendu (près de `render_mcp_editor`) :

```rust
/// Modal du gestionnaire de marketplaces (liste, saisie d'ajout, confirmation, indicateur de job).
fn render_marketplaces(app: &App, f: &mut Frame, area: Rect) {
    let Some(m) = &app.marketplaces else {
        return;
    };
    let popup = centered_rect(78, 68, area);
    f.render_widget(Clear, popup);

    let busy = app.mkt_job.is_some();
    let hint = if m.mode == MktMode::AddInput {
        " Enter valider · Esc annuler "
    } else if busy {
        " (opération en cours…) · Esc fermer "
    } else {
        " a ajouter · u màj · d retirer · Esc fermer "
    };
    let block = Block::default()
        .borders(Borders::ALL)
        .title(Span::styled(
            " Marketplaces de plugins ",
            Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
        ))
        .title(Line::from(hint).right_aligned());
    let inner = block.inner(popup);
    f.render_widget(block, popup);

    let mut lines: Vec<Line> = Vec::new();
    if m.items.is_empty() {
        lines.push(Line::from(Span::styled(
            "  (aucune marketplace — 'a' pour en ajouter)",
            Style::default().fg(DIM),
        )));
    }
    for (i, mk) in m.items.iter().enumerate() {
        let sel = i == m.idx;
        let src = match &mk.source {
            MarketplaceSource::Github { repo } => format!("github:{repo}"),
            MarketplaceSource::Git { url } => url.clone(),
            MarketplaceSource::Local { path } => format!("local:{}", path.display()),
        };
        let date = mk.last_updated.split('T').next().unwrap_or("");
        let label = format!("{} {}  ·  {}  ·  {}", if sel { "▶" } else { " " }, mk.name, src, date);
        let style = if sel { selection_style(true) } else { Style::default() };
        lines.push(Line::from(Span::styled(label, style)));
    }

    if m.mode == MktMode::AddInput {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "  Source (owner/repo · URL git · chemin local) :",
            Style::default().fg(ACCENT),
        )));
        lines.push(Line::from(vec![
            Span::styled("  > ", Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)),
            Span::raw(m.input.clone()),
            Span::styled("▏", Style::default().fg(ACCENT)),
        ]));
    } else if m.confirm_remove {
        lines.push(Line::from(""));
        let name = m.selected_name().unwrap_or_default();
        lines.push(Line::from(Span::styled(
            format!("  Retirer « {name} » et son dossier ? (o/n)"),
            Style::default()
                .fg(Color::Black)
                .bg(Color::Red)
                .add_modifier(Modifier::BOLD),
        )));
    } else if busy {
        if let Some(job) = &app.mkt_job {
            const SPINNER: [char; 10] = ['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];
            let s = SPINNER[(job.frame as usize) % SPINNER.len()];
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                format!("  {s} {}…", job.label),
                Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
            )));
        }
    }

    f.render_widget(Paragraph::new(Text::from(lines)), inner);
}
```

- [ ] **Step 2: Mettre à jour le footer et l'aide**

Dans `render_footer`, l'arm `Section::Extensions => key_hints(&[...])` — insérer l'entrée `g` après `("m", "MCP")` :

```rust
        Section::Extensions => key_hints(&[
            ("Enter", "hooks"),
            ("p", "plugins"),
            ("m", "MCP"),
            ("g", "marketplaces"),
            ("↑/↓", "défiler"),
            ("t", "cible"),
            ("E", "settings"),
            ("?", "aide"),
        ]),
```

Dans `render_help`, remplacer la ligne `("Extensions", …)` par :

```rust
        ("Extensions", "hooks (Enter) · plugins (p) · MCP (m) · marketplaces (g) ; E édite settings.json"),
```

- [ ] **Step 3: Vérification complète**

Run: `cargo clippy --workspace 2>&1 | grep -cE "warning:|error"` → attendu `0`
Run: `cargo test --workspace` → tous les paquets `ok`.

- [ ] **Step 4: Commit**

```bash
git add crates/claudine/src/tui/ui.rs
git commit -m "feat(tui): rendu du gestionnaire de marketplaces + indicateur + aide (g)"
```

---

## Self-Review

**1. Couverture de la spec :**
- §3 décision technique (clone délégué à `git`, aucune dep, MSRV 1.74) → Task 2 (`mod git`), aucune nouvelle dépendance dans les Cargo.toml. ✓
- §4 résolution des chemins (`plugins_dir`/`marketplaces`/`known_marketplaces.json`/manifeste) → Task 1 (`marketplaces_dir`, `known_marketplaces_path`, `manifest_path`). ✓
- §5 modèle (`MarketplaceSource` + parse/clone_url + ser/de JSON, `Marketplace`, `MarketplaceManifest`, `PluginManifestEntry`) → Task 1. ✓
- §6 helper git (`clone`/`pull`, erreurs, `--depth 1`, git introuvable) → Task 2. ✓
- §7 API cœur (`read_marketplaces`, `read_marketplace_manifest`, `add`/`remove`/`update`, écriture via `SettingsDoc`, `iso8601_utc`) → Tasks 1-2. ✓
- §8 TUI gestionnaire (modal, `g`, liste, `a`/`d`/`u`/`Esc`, parsing heuristique) → Tasks 3-5. ✓
- §9 concurrence (thread + mpsc, `event::poll` quand job actif, spinner, un seul job, mutations ignorées si busy) → Task 4 (méthodes + boucle), Task 5 (spinner). ✓
- §10 sûreté (backup+atomique via SettingsDoc, rollback du clone, suppression confinée + garde-fou nom, confirmation, home actif) → Tasks 1-2 (`is_safe_name`, rollback, SettingsDoc) + Task 4 (confirmation). ✓
- §11 tests (cœur via fixtures git locales, TUI) → Tasks 1-4. ✓

**2. Placeholders :** aucun TODO/TBD ; code complet à chaque étape.

**3. Cohérence des types :** `MarketplaceSource`/`Marketplace`/`MarketplaceManifest`/`PluginManifestEntry` identiques cœur (Tasks 1-2) ↔ TUI (Tasks 3-5). `MarketplacesManager` (méthodes `new`/`set_items`/`move_sel`/`selected_name`/`begin_add`/`cancel_add`/`begin_remove`, champs `items`/`idx`/`mode`/`input`/`confirm_remove`) cohérent entre Task 3 et usages Tasks 4-5. `MktJob`/`MktOutcome` (champs `label`/`frame`/`rx` et `result`) cohérents entre la définition et l'usage (Task 4) et le rendu du spinner (Task 5). `MarketplaceSource::parse` utilisé par `mkt_begin_add` (Task 4) renvoie bien `Option<Self>` (Task 1). Ouverture par `g`, confinée à Extensions via `open_marketplaces` (garde `section`). ✓

**Note de fidélité (rappel spec §5)** : seul le type de source `github` est attesté sur disque ; `git`/`local` sont notre meilleure correspondance. Le backup `known_marketplaces.json` et le fait qu'`installLocation` suffit à la résolution limitent le risque.
