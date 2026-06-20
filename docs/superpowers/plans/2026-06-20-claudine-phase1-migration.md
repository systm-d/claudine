# Claudine — Phase 1 (cœur + CLI migration) — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Livrer un moteur testé (`claudine-core`) et une CLI (`claudine`) qui exportent les données Claude Code (`~/.claude`) en un bundle `.tar.gz` puis les importent sur une autre machine en remappant les chemins de projets.

**Architecture:** Workspace Cargo à deux crates. `claudine-core` (lib) contient toute la logique (résolution de chemins, scan, encodage, manifest, export, remap, import) sans dépendance UI et est couverte par des tests. `claudine` (bin) est une fine couche `clap` qui appelle le cœur et imprime des rapports.

**Tech Stack:** Rust (edition 2021), `serde`/`serde_json`, `thiserror`, `tar`, `flate2`, `clap` (derive). Tests : `tempfile`, `assert_cmd`, `predicates`.

## Global Constraints

- Rust edition **2021**, MSRV **1.74+**.
- Versions de dépendances (exactes, à mettre dans les `Cargo.toml`) :
  - `serde = { version = "1", features = ["derive"] }`
  - `serde_json = "1"`
  - `thiserror = "1"`
  - `tar = "0.4"`
  - `flate2 = "1"`
  - `clap = { version = "4", features = ["derive"] }` (crate `claudine` seulement)
  - dev : `tempfile = "3"`, `assert_cmd = "2"`, `predicates = "3"`
- **PAS de `ratatui` en phase 1** : le binaire sans argument imprime un placeholder.
- **Export strictement en lecture seule** sur la source ; **import** : backup horodaté avant toute écriture, fusion **skip par défaut**, écritures **temp + rename**.
- **Secrets jamais lus ni écrits** : on n'ajoute jamais `.credentials.json`, `security_warnings_state_*`, `cache/`, `shell-snapshots/`, `session-env/`, `telemetry/`, ni `~/.claude.json` (phase 1).
- **Config phase 1** : export de `settings.json` + `settings.local.json` **verbatim** uniquement. `~/.claude.json` reporté à une itération ultérieure.
- `Manifest.schema_version` courant = **1**.
- Jamais de perte silencieuse : une ligne `.jsonl` non parsable est recopiée verbatim et signalée en avertissement.
- Encodage chemin → nom de dossier : chaque `/` **et** chaque `.` deviennent `-` (vérifié : `/home/kdelfour/Workspace/Professionel/Delfour.co/system/claude-tui` → `-home-kdelfour-Workspace-Professionel-Delfour-co-system-claude-tui`).

## File Structure

```
claude-tui/
├─ Cargo.toml                       (workspace: resolver=2, members)
├─ crates/
│  ├─ claudine-core/
│  │  ├─ Cargo.toml
│  │  └─ src/
│  │     ├─ lib.rs                  (déclarations de modules + re-exports + testkit cfg(test))
│  │     ├─ error.rs                (CoreError, Result, Report)
│  │     ├─ home.rs                 (ClaudeHome : discover/from_base + accès chemins)
│  │     ├─ pathcodec.rs            (encode_cwd)
│  │     ├─ model.rs                (Project, SessionMeta)
│  │     ├─ scan.rs                 (scan_projects, read_session_meta)
│  │     ├─ manifest.rs             (Manifest, ManifestProject, SCHEMA_VERSION)
│  │     ├─ export.rs               (ExportOptions, export)
│  │     ├─ remap.rs                (RemapTable, RemapRule, rewrite_jsonl_line)
│  │     └─ import.rs               (ImportOptions, read_manifest, dry_run, apply)
│  └─ claudine/
│     ├─ Cargo.toml
│     ├─ src/
│     │  ├─ main.rs                 (clap Cli/Cmd + dispatch + placeholder TUI)
│     │  └─ cli.rs                  (run_export, run_import, format_report)
│     └─ tests/
│        └─ cli.rs                  (tests assert_cmd)
└─ docs/superpowers/…
```

---

## Task 1 : Scaffolding du workspace et des deux crates

**Files:**
- Create: `Cargo.toml` (workspace)
- Create: `crates/claudine-core/Cargo.toml`
- Create: `crates/claudine-core/src/lib.rs`
- Create: `crates/claudine/Cargo.toml`
- Create: `crates/claudine/src/main.rs`

**Interfaces:**
- Consumes: rien.
- Produces: un workspace qui compile ; `claudine_core` exporte (vide pour l'instant) ; binaire `claudine` qui imprime un placeholder.

- [ ] **Step 1: Écrire le `Cargo.toml` du workspace**

```toml
[workspace]
resolver = "2"
members = ["crates/claudine-core", "crates/claudine"]

[workspace.package]
edition = "2021"
rust-version = "1.74"
version = "0.1.0"
```

- [ ] **Step 2: Écrire `crates/claudine-core/Cargo.toml`**

```toml
[package]
name = "claudine-core"
edition.workspace = true
rust-version.workspace = true
version.workspace = true

[dependencies]
serde = { version = "1", features = ["derive"] }
serde_json = "1"
thiserror = "1"
tar = "0.4"
flate2 = "1"

[dev-dependencies]
tempfile = "3"
```

- [ ] **Step 3: Écrire `crates/claudine-core/src/lib.rs` (vide mais valide)**

```rust
//! Cœur logique de Claudine : lecture/écriture de la structure `~/.claude`.

#[cfg(test)]
mod tests {
    #[test]
    fn smoke() {
        assert_eq!(2 + 2, 4);
    }
}
```

- [ ] **Step 4: Écrire `crates/claudine/Cargo.toml`**

```toml
[package]
name = "claudine"
edition.workspace = true
rust-version.workspace = true
version.workspace = true

[[bin]]
name = "claudine"
path = "src/main.rs"

[dependencies]
claudine-core = { path = "../claudine-core" }
clap = { version = "4", features = ["derive"] }

[dev-dependencies]
assert_cmd = "2"
predicates = "3"
tempfile = "3"
```

- [ ] **Step 5: Écrire `crates/claudine/src/main.rs` (placeholder)**

```rust
fn main() {
    println!("Claudine — TUI à venir (phase 2). Essayez `claudine --help`.");
}
```

- [ ] **Step 6: Compiler et tester**

Run: `cargo test`
Expected: build OK, le test `smoke` passe (`test result: ok. 1 passed`).

- [ ] **Step 7: Commit**

```bash
git add Cargo.toml crates/
git commit -m "feat: scaffolding workspace claudine-core + claudine"
```

---

## Task 2 : Types d'erreur et rapport (`error.rs`)

**Files:**
- Create: `crates/claudine-core/src/error.rs`
- Modify: `crates/claudine-core/src/lib.rs`

**Interfaces:**
- Consumes: rien.
- Produces:
  - `pub enum CoreError` (variantes `Io{path,source}`, `JsonParse{file,line,source}`, `ManifestVersion(u32)`, `RemapIncomplete(String)`, `Conflict(String)`, `BundleFormat(String)`)
  - `CoreError::io(path: impl Into<PathBuf>, source: std::io::Error) -> CoreError`
  - `pub type Result<T> = std::result::Result<T, CoreError>;`
  - `pub struct Report { pub warnings: Vec<String>, pub counts: BTreeMap<String, usize> }`
  - `Report::warn(&mut self, impl Into<String>)`, `Report::bump(&mut self, &str, usize)`, `Report::count(&self, &str) -> usize`

- [ ] **Step 1: Écrire le test (dans `error.rs`)**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn report_accumulates() {
        let mut r = Report::default();
        r.bump("sessions", 2);
        r.bump("sessions", 3);
        r.warn("ligne corrompue");
        assert_eq!(r.count("sessions"), 5);
        assert_eq!(r.count("absent"), 0);
        assert_eq!(r.warnings, vec!["ligne corrompue".to_string()]);
    }

    #[test]
    fn io_helper_sets_path() {
        let err = CoreError::io(
            "/tmp/x",
            std::io::Error::new(std::io::ErrorKind::NotFound, "nope"),
        );
        assert!(format!("{err}").contains("/tmp/x"));
    }
}
```

- [ ] **Step 2: Écrire l'implémentation au-dessus du `mod tests` dans `error.rs`**

```rust
use std::collections::BTreeMap;
use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum CoreError {
    #[error("erreur d'E/S sur {path}: {source}")]
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("erreur JSON dans {file} ligne {line}: {source}")]
    JsonParse {
        file: PathBuf,
        line: usize,
        source: serde_json::Error,
    },
    #[error("version de manifest non supportée: {0}")]
    ManifestVersion(u32),
    #[error("remap incomplet: aucune cible pour {0}")]
    RemapIncomplete(String),
    #[error("conflit: {0}")]
    Conflict(String),
    #[error("bundle invalide: {0}")]
    BundleFormat(String),
}

impl CoreError {
    pub fn io(path: impl Into<PathBuf>, source: std::io::Error) -> Self {
        CoreError::Io {
            path: path.into(),
            source,
        }
    }
}

pub type Result<T> = std::result::Result<T, CoreError>;

#[derive(Debug, Default, Clone)]
pub struct Report {
    pub warnings: Vec<String>,
    pub counts: BTreeMap<String, usize>,
}

impl Report {
    pub fn warn(&mut self, msg: impl Into<String>) {
        self.warnings.push(msg.into());
    }

    pub fn bump(&mut self, key: &str, n: usize) {
        *self.counts.entry(key.to_string()).or_default() += n;
    }

    pub fn count(&self, key: &str) -> usize {
        self.counts.get(key).copied().unwrap_or(0)
    }
}
```

- [ ] **Step 3: Déclarer le module et re-exporter dans `lib.rs`**

Remplacer le contenu de `lib.rs` par :

```rust
//! Cœur logique de Claudine : lecture/écriture de la structure `~/.claude`.

pub mod error;

pub use error::{CoreError, Report, Result};
```

- [ ] **Step 4: Lancer les tests**

Run: `cargo test -p claudine-core`
Expected: PASS (`report_accumulates`, `io_helper_sets_path`).

- [ ] **Step 5: Commit**

```bash
git add crates/claudine-core/src/error.rs crates/claudine-core/src/lib.rs
git commit -m "feat(core): types d'erreur CoreError + Report"
```

---

## Task 3 : Résolution des chemins (`home.rs`)

**Files:**
- Create: `crates/claudine-core/src/home.rs`
- Modify: `crates/claudine-core/src/lib.rs`

**Interfaces:**
- Consumes: rien.
- Produces:
  - `pub struct ClaudeHome { pub base: PathBuf }`
  - `ClaudeHome::from_base(base: impl Into<PathBuf>) -> ClaudeHome`
  - `ClaudeHome::discover() -> Result<ClaudeHome>` (env `CLAUDE_CONFIG_DIR`, sinon `HOME/.claude`)
  - accès : `projects_dir`, `todos_dir`, `plugins_dir` (→ `PathBuf`) ; `memory_file`, `settings_file`, `settings_local_file`, `history_file` (→ `PathBuf`)

- [ ] **Step 1: Écrire le test (dans `home.rs`)**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_base_builds_subpaths() {
        let h = ClaudeHome::from_base("/x/.claude");
        assert_eq!(h.projects_dir(), std::path::Path::new("/x/.claude/projects"));
        assert_eq!(h.settings_file(), std::path::Path::new("/x/.claude/settings.json"));
        assert_eq!(h.history_file(), std::path::Path::new("/x/.claude/history.jsonl"));
    }

    #[test]
    fn discover_respects_env() {
        std::env::set_var("CLAUDE_CONFIG_DIR", "/custom/dir");
        let h = ClaudeHome::discover().unwrap();
        assert_eq!(h.base, std::path::Path::new("/custom/dir"));
        std::env::remove_var("CLAUDE_CONFIG_DIR");
    }
}
```

- [ ] **Step 2: Écrire l'implémentation dans `home.rs`**

```rust
use std::path::{Path, PathBuf};

use crate::error::{CoreError, Result};

#[derive(Debug, Clone)]
pub struct ClaudeHome {
    pub base: PathBuf,
}

impl ClaudeHome {
    pub fn from_base(base: impl Into<PathBuf>) -> Self {
        Self { base: base.into() }
    }

    pub fn discover() -> Result<Self> {
        if let Ok(dir) = std::env::var("CLAUDE_CONFIG_DIR") {
            return Ok(Self::from_base(dir));
        }
        let home = std::env::var("HOME").map_err(|_| {
            CoreError::io(
                "<HOME>",
                std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "variable HOME absente",
                ),
            )
        })?;
        Ok(Self::from_base(Path::new(&home).join(".claude")))
    }

    pub fn projects_dir(&self) -> PathBuf {
        self.base.join("projects")
    }
    pub fn todos_dir(&self) -> PathBuf {
        self.base.join("todos")
    }
    pub fn plugins_dir(&self) -> PathBuf {
        self.base.join("plugins")
    }
    pub fn memory_file(&self) -> PathBuf {
        self.base.join("CLAUDE.md")
    }
    pub fn settings_file(&self) -> PathBuf {
        self.base.join("settings.json")
    }
    pub fn settings_local_file(&self) -> PathBuf {
        self.base.join("settings.local.json")
    }
    pub fn history_file(&self) -> PathBuf {
        self.base.join("history.jsonl")
    }
}
```

- [ ] **Step 3: Déclarer le module dans `lib.rs`**

Ajouter après `pub mod error;` :

```rust
pub mod home;

pub use home::ClaudeHome;
```

- [ ] **Step 4: Lancer les tests**

Run: `cargo test -p claudine-core home`
Expected: PASS (`from_base_builds_subpaths`, `discover_respects_env`).

- [ ] **Step 5: Commit**

```bash
git add crates/claudine-core/src/home.rs crates/claudine-core/src/lib.rs
git commit -m "feat(core): ClaudeHome (discover + accès chemins)"
```

---

## Task 4 : Encodage chemin → nom de dossier (`pathcodec.rs`)

**Files:**
- Create: `crates/claudine-core/src/pathcodec.rs`
- Modify: `crates/claudine-core/src/lib.rs`

**Interfaces:**
- Consumes: rien.
- Produces: `pub fn encode_cwd(cwd: &str) -> String` (remplace `/` et `.` par `-`).

- [ ] **Step 1: Écrire le test (dans `pathcodec.rs`)**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encodes_slashes_and_dots() {
        assert_eq!(encode_cwd("/home/kdelfour"), "-home-kdelfour");
        assert_eq!(
            encode_cwd("/home/kdelfour/Workspace/Professionel/Delfour.co/system/claude-tui"),
            "-home-kdelfour-Workspace-Professionel-Delfour-co-system-claude-tui"
        );
    }

    #[test]
    fn preserves_existing_dashes() {
        // ambiguïté assumée : un '-' réel reste un '-'
        assert_eq!(encode_cwd("/a/generic-rag"), "-a-generic-rag");
    }
}
```

- [ ] **Step 2: Écrire l'implémentation dans `pathcodec.rs`**

```rust
/// Encode un chemin absolu en nom de dossier de projet à la mode Claude Code :
/// chaque `/` et chaque `.` deviennent `-`. L'opération est volontairement
/// non réversible (la source de vérité du `cwd` est le champ interne des `.jsonl`).
pub fn encode_cwd(cwd: &str) -> String {
    cwd.chars()
        .map(|c| if c == '/' || c == '.' { '-' } else { c })
        .collect()
}
```

- [ ] **Step 3: Déclarer le module dans `lib.rs`**

Ajouter :

```rust
pub mod pathcodec;

pub use pathcodec::encode_cwd;
```

- [ ] **Step 4: Lancer les tests**

Run: `cargo test -p claudine-core pathcodec`
Expected: PASS (`encodes_slashes_and_dots`, `preserves_existing_dashes`).

- [ ] **Step 5: Commit**

```bash
git add crates/claudine-core/src/pathcodec.rs crates/claudine-core/src/lib.rs
git commit -m "feat(core): encode_cwd (chemin -> nom de dossier)"
```

---

## Task 5 : Modèle + scan des sessions (`model.rs`, `scan.rs`) + testkit

**Files:**
- Create: `crates/claudine-core/src/model.rs`
- Create: `crates/claudine-core/src/scan.rs`
- Modify: `crates/claudine-core/src/lib.rs` (déclarations + `mod testkit` sous `cfg(test)`)

**Interfaces:**
- Consumes: `ClaudeHome`, `Result`, `CoreError`.
- Produces:
  - `pub struct SessionMeta { pub id: String, pub path: PathBuf, pub cwd: Option<String>, pub message_count: usize, pub first_ts: Option<String>, pub last_ts: Option<String>, pub size: u64 }`
  - `pub struct Project { pub encoded_name: String, pub cwd: Option<String>, pub sessions: Vec<SessionMeta> }`
  - `pub fn scan_projects(home: &ClaudeHome) -> Result<Vec<Project>>`
  - `pub fn read_session_meta(path: &Path) -> Result<SessionMeta>`
  - testkit (cfg(test)) : `testkit::FakeHome` avec `new()`, `base() -> &Path`, `add_session(encoded, id, lines: &[&str])`, `write_file(rel, content)`

- [ ] **Step 1: Ajouter le `testkit` (cfg(test)) dans `lib.rs`**

Ajouter à la fin de `lib.rs` :

```rust
#[cfg(test)]
pub(crate) mod testkit {
    use std::fs;
    use std::path::Path;

    pub struct FakeHome {
        pub dir: tempfile::TempDir,
    }

    impl FakeHome {
        pub fn new() -> Self {
            let dir = tempfile::tempdir().unwrap();
            fs::create_dir_all(dir.path().join("projects")).unwrap();
            Self { dir }
        }

        pub fn base(&self) -> &Path {
            self.dir.path()
        }

        pub fn add_session(&self, encoded: &str, id: &str, lines: &[&str]) {
            let pdir = self.dir.path().join("projects").join(encoded);
            fs::create_dir_all(&pdir).unwrap();
            fs::write(pdir.join(format!("{id}.jsonl")), lines.join("\n")).unwrap();
        }

        pub fn write_file(&self, rel: &str, content: &str) {
            let p = self.dir.path().join(rel);
            fs::create_dir_all(p.parent().unwrap()).unwrap();
            fs::write(p, content).unwrap();
        }
    }
}
```

- [ ] **Step 2: Écrire `model.rs`**

```rust
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionMeta {
    pub id: String,
    pub path: PathBuf,
    pub cwd: Option<String>,
    pub message_count: usize,
    pub first_ts: Option<String>,
    pub last_ts: Option<String>,
    pub size: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Project {
    pub encoded_name: String,
    pub cwd: Option<String>,
    pub sessions: Vec<SessionMeta>,
}
```

- [ ] **Step 3: Écrire le test de scan (dans `scan.rs`)**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::home::ClaudeHome;
    use crate::testkit::FakeHome;

    #[test]
    fn scans_projects_and_extracts_cwd() {
        let fake = FakeHome::new();
        fake.add_session(
            "-home-old-proj",
            "11111111-1111-1111-1111-111111111111",
            &[
                r#"{"type":"user","cwd":"/home/old/proj","timestamp":"2026-01-01T10:00:00Z"}"#,
                r#"{"type":"assistant","timestamp":"2026-01-01T10:01:00Z"}"#,
            ],
        );
        let home = ClaudeHome::from_base(fake.base());

        let projects = scan_projects(&home).unwrap();

        assert_eq!(projects.len(), 1);
        let p = &projects[0];
        assert_eq!(p.encoded_name, "-home-old-proj");
        assert_eq!(p.cwd.as_deref(), Some("/home/old/proj"));
        assert_eq!(p.sessions.len(), 1);
        let s = &p.sessions[0];
        assert_eq!(s.id, "11111111-1111-1111-1111-111111111111");
        assert_eq!(s.message_count, 2);
        assert_eq!(s.first_ts.as_deref(), Some("2026-01-01T10:00:00Z"));
        assert_eq!(s.last_ts.as_deref(), Some("2026-01-01T10:01:00Z"));
        assert_eq!(s.cwd.as_deref(), Some("/home/old/proj"));
    }

    #[test]
    fn corrupt_line_is_not_fatal() {
        let fake = FakeHome::new();
        fake.add_session(
            "-a",
            "22222222-2222-2222-2222-222222222222",
            &["pas du json", r#"{"cwd":"/a","timestamp":"t"}"#],
        );
        let home = ClaudeHome::from_base(fake.base());
        let projects = scan_projects(&home).unwrap();
        assert_eq!(projects[0].sessions[0].message_count, 2);
        assert_eq!(projects[0].sessions[0].cwd.as_deref(), Some("/a"));
    }
}
```

- [ ] **Step 4: Écrire l'implémentation dans `scan.rs`**

```rust
use std::fs;
use std::path::Path;

use serde_json::Value;

use crate::error::{CoreError, Result};
use crate::home::ClaudeHome;
use crate::model::{Project, SessionMeta};

pub fn scan_projects(home: &ClaudeHome) -> Result<Vec<Project>> {
    let dir = home.projects_dir();
    let mut projects = Vec::new();
    if !dir.exists() {
        return Ok(projects);
    }
    let entries = fs::read_dir(&dir).map_err(|e| CoreError::io(&dir, e))?;
    for entry in entries {
        let entry = entry.map_err(|e| CoreError::io(&dir, e))?;
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let encoded_name = entry.file_name().to_string_lossy().into_owned();
        let mut sessions = Vec::new();
        let session_entries =
            fs::read_dir(&path).map_err(|e| CoreError::io(&path, e))?;
        for s in session_entries {
            let s = s.map_err(|e| CoreError::io(&path, e))?;
            let sp = s.path();
            if sp.extension().and_then(|e| e.to_str()) == Some("jsonl") {
                sessions.push(read_session_meta(&sp)?);
            }
        }
        sessions.sort_by(|a, b| a.id.cmp(&b.id));
        let cwd = sessions.iter().find_map(|s| s.cwd.clone());
        projects.push(Project {
            encoded_name,
            cwd,
            sessions,
        });
    }
    projects.sort_by(|a, b| a.encoded_name.cmp(&b.encoded_name));
    Ok(projects)
}

pub fn read_session_meta(path: &Path) -> Result<SessionMeta> {
    let content = fs::read_to_string(path).map_err(|e| CoreError::io(path, e))?;
    let size = content.len() as u64;
    let id = path
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();

    let mut message_count = 0usize;
    let mut cwd: Option<String> = None;
    let mut first_ts: Option<String> = None;
    let mut last_ts: Option<String> = None;

    for line in content.lines() {
        if line.trim().is_empty() {
            continue;
        }
        message_count += 1;
        if let Ok(v) = serde_json::from_str::<Value>(line) {
            if cwd.is_none() {
                if let Some(c) = v.get("cwd").and_then(|c| c.as_str()) {
                    cwd = Some(c.to_string());
                }
            }
            if let Some(ts) = v.get("timestamp").and_then(|t| t.as_str()) {
                if first_ts.is_none() {
                    first_ts = Some(ts.to_string());
                }
                last_ts = Some(ts.to_string());
            }
        }
    }

    Ok(SessionMeta {
        id,
        path: path.to_path_buf(),
        cwd,
        message_count,
        first_ts,
        last_ts,
        size,
    })
}
```

- [ ] **Step 5: Déclarer les modules dans `lib.rs`**

Ajouter (avant le bloc `testkit`) :

```rust
pub mod model;
pub mod scan;

pub use model::{Project, SessionMeta};
pub use scan::{read_session_meta, scan_projects};
```

- [ ] **Step 6: Lancer les tests**

Run: `cargo test -p claudine-core scan`
Expected: PASS (`scans_projects_and_extracts_cwd`, `corrupt_line_is_not_fatal`).

- [ ] **Step 7: Commit**

```bash
git add crates/claudine-core/src/model.rs crates/claudine-core/src/scan.rs crates/claudine-core/src/lib.rs
git commit -m "feat(core): modèle Project/SessionMeta + scan + testkit"
```

---

## Task 6 : Manifest (`manifest.rs`)

**Files:**
- Create: `crates/claudine-core/src/manifest.rs`
- Modify: `crates/claudine-core/src/lib.rs`

**Interfaces:**
- Consumes: `serde`.
- Produces:
  - `pub const SCHEMA_VERSION: u32 = 1;`
  - `pub struct ManifestProject { pub encoded_name: String, pub cwd: Option<String>, pub session_ids: Vec<String> }`
  - `pub struct Manifest { pub schema_version: u32, pub created_at: String, pub source_hostname: String, pub source_home: String, pub projects: Vec<ManifestProject>, pub included_categories: Vec<String>, pub excluded: Vec<String> }`
  - dérive `Serialize, Deserialize, Debug, Clone, PartialEq`

- [ ] **Step 1: Écrire le test (dans `manifest.rs`)**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manifest_round_trips_json() {
        let m = Manifest {
            schema_version: SCHEMA_VERSION,
            created_at: "1750000000".to_string(),
            source_hostname: "pc1".to_string(),
            source_home: "/home/old/.claude".to_string(),
            projects: vec![ManifestProject {
                encoded_name: "-home-old-proj".to_string(),
                cwd: Some("/home/old/proj".to_string()),
                session_ids: vec!["abc".to_string()],
            }],
            included_categories: vec!["sessions".to_string()],
            excluded: vec![".credentials.json".to_string()],
        };
        let json = serde_json::to_string(&m).unwrap();
        let back: Manifest = serde_json::from_str(&json).unwrap();
        assert_eq!(m, back);
    }
}
```

- [ ] **Step 2: Écrire l'implémentation dans `manifest.rs`**

```rust
use serde::{Deserialize, Serialize};

pub const SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ManifestProject {
    pub encoded_name: String,
    pub cwd: Option<String>,
    pub session_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Manifest {
    pub schema_version: u32,
    pub created_at: String,
    pub source_hostname: String,
    pub source_home: String,
    pub projects: Vec<ManifestProject>,
    pub included_categories: Vec<String>,
    pub excluded: Vec<String>,
}
```

- [ ] **Step 3: Déclarer le module dans `lib.rs`**

```rust
pub mod manifest;

pub use manifest::{Manifest, ManifestProject, SCHEMA_VERSION};
```

- [ ] **Step 4: Lancer les tests**

Run: `cargo test -p claudine-core manifest`
Expected: PASS (`manifest_round_trips_json`).

- [ ] **Step 5: Commit**

```bash
git add crates/claudine-core/src/manifest.rs crates/claudine-core/src/lib.rs
git commit -m "feat(core): structure Manifest (serde)"
```

---

## Task 7 : Export du bundle (`export.rs`)

**Files:**
- Create: `crates/claudine-core/src/export.rs`
- Modify: `crates/claudine-core/src/lib.rs`

**Interfaces:**
- Consumes: `ClaudeHome`, `scan_projects`, `Manifest`/`ManifestProject`/`SCHEMA_VERSION`, `Report`, `Result`, `CoreError`.
- Produces:
  - `pub struct ExportOptions { pub include_history: bool }` + `Default` (history = `true`)
  - `pub fn export(home: &ClaudeHome, output: &Path, opts: &ExportOptions) -> Result<Report>`
  - Constante interne de la liste d'exclusions (informative dans le manifest).

- [ ] **Step 1: Écrire le test (dans `export.rs`)**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::home::ClaudeHome;
    use crate::testkit::FakeHome;
    use flate2::read::GzDecoder;
    use std::collections::BTreeSet;

    fn archive_entries(path: &std::path::Path) -> BTreeSet<String> {
        let f = std::fs::File::open(path).unwrap();
        let mut ar = tar::Archive::new(GzDecoder::new(f));
        ar.entries()
            .unwrap()
            .map(|e| e.unwrap().path().unwrap().to_string_lossy().into_owned())
            .collect()
    }

    #[test]
    fn export_contains_sessions_and_manifest_excludes_secrets() {
        let fake = FakeHome::new();
        fake.add_session(
            "-home-old-proj",
            "abc",
            &[r#"{"cwd":"/home/old/proj","timestamp":"t"}"#],
        );
        fake.write_file("CLAUDE.md", "# mémoire");
        fake.write_file("settings.json", "{}");
        fake.write_file(".credentials.json", "SECRET");
        let home = ClaudeHome::from_base(fake.base());

        let out = fake.base().join("bundle.tar.gz");
        let report = export(&home, &out, &ExportOptions::default()).unwrap();

        assert!(out.exists());
        let entries = archive_entries(&out);
        assert!(entries.contains("manifest.json"));
        assert!(entries.contains("projects/-home-old-proj/abc.jsonl"));
        assert!(entries.contains("memory/CLAUDE.md"));
        assert!(entries.contains("config/settings.json"));
        // secret jamais embarqué
        assert!(!entries.iter().any(|e| e.contains("credentials")));
        assert_eq!(report.count("projects"), 1);
        assert_eq!(report.count("sessions"), 1);
    }
}
```

- [ ] **Step 2: Écrire l'implémentation dans `export.rs`**

```rust
use std::fs::File;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use flate2::write::GzEncoder;
use flate2::Compression;

use crate::error::{CoreError, Report, Result};
use crate::home::ClaudeHome;
use crate::manifest::{Manifest, ManifestProject, SCHEMA_VERSION};
use crate::scan::scan_projects;

pub const EXCLUDED: &[&str] = &[
    ".credentials.json",
    "security_warnings_state_*",
    "cache/",
    "shell-snapshots/",
    "session-env/",
    "telemetry/",
    ".claude.json",
];

#[derive(Debug, Clone)]
pub struct ExportOptions {
    pub include_history: bool,
}

impl Default for ExportOptions {
    fn default() -> Self {
        Self {
            include_history: true,
        }
    }
}

pub fn export(home: &ClaudeHome, output: &Path, opts: &ExportOptions) -> Result<Report> {
    let mut report = Report::default();
    let projects = scan_projects(home)?;

    let mut included = vec!["sessions".to_string()];

    let file = File::create(output).map_err(|e| CoreError::io(output, e))?;
    let enc = GzEncoder::new(file, Compression::default());
    let mut builder = tar::Builder::new(enc);

    // Sessions (dossier projects/ entier : aucun secret à l'intérieur).
    let projects_dir = home.projects_dir();
    if projects_dir.exists() {
        builder
            .append_dir_all("projects", &projects_dir)
            .map_err(|e| CoreError::io(&projects_dir, e))?;
    }
    for p in &projects {
        report.bump("projects", 1);
        report.bump("sessions", p.sessions.len());
    }

    // Todos.
    let todos_dir = home.todos_dir();
    if todos_dir.exists() {
        builder
            .append_dir_all("todos", &todos_dir)
            .map_err(|e| CoreError::io(&todos_dir, e))?;
        included.push("todos".to_string());
    }

    // Mémoire user.
    let memory = home.memory_file();
    if memory.exists() {
        builder
            .append_path_with_name(&memory, "memory/CLAUDE.md")
            .map_err(|e| CoreError::io(&memory, e))?;
        included.push("memory".to_string());
    }

    // Config (verbatim, phase 1 : settings + settings.local seulement).
    for (src, name) in [
        (home.settings_file(), "config/settings.json"),
        (home.settings_local_file(), "config/settings.local.json"),
    ] {
        if src.exists() {
            builder
                .append_path_with_name(&src, name)
                .map_err(|e| CoreError::io(&src, e))?;
            if !included.contains(&"config".to_string()) {
                included.push("config".to_string());
            }
        }
    }

    // Plugins/skills/agents.
    let plugins_dir = home.plugins_dir();
    if plugins_dir.exists() {
        builder
            .append_dir_all("plugins", &plugins_dir)
            .map_err(|e| CoreError::io(&plugins_dir, e))?;
        included.push("plugins".to_string());
    }

    // Historique (optionnel).
    if opts.include_history {
        let hist = home.history_file();
        if hist.exists() {
            builder
                .append_path_with_name(&hist, "history.jsonl")
                .map_err(|e| CoreError::io(&hist, e))?;
            included.push("history".to_string());
        }
    }

    // Manifest.
    let created_at = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs().to_string())
        .unwrap_or_else(|_| "0".to_string());
    let manifest = Manifest {
        schema_version: SCHEMA_VERSION,
        created_at,
        source_hostname: std::env::var("HOSTNAME").unwrap_or_else(|_| "unknown".to_string()),
        source_home: home.base.to_string_lossy().into_owned(),
        projects: projects
            .iter()
            .map(|p| ManifestProject {
                encoded_name: p.encoded_name.clone(),
                cwd: p.cwd.clone(),
                session_ids: p.sessions.iter().map(|s| s.id.clone()).collect(),
            })
            .collect(),
        included_categories: included,
        excluded: EXCLUDED.iter().map(|s| s.to_string()).collect(),
    };
    let manifest_bytes = serde_json::to_vec_pretty(&manifest)
        .map_err(|e| CoreError::JsonParse {
            file: output.to_path_buf(),
            line: 0,
            source: e,
        })?;
    let mut header = tar::Header::new_gnu();
    header.set_size(manifest_bytes.len() as u64);
    header.set_mode(0o644);
    header.set_cksum();
    builder
        .append_data(&mut header, "manifest.json", manifest_bytes.as_slice())
        .map_err(|e| CoreError::io(output, e))?;

    let enc = builder.into_inner().map_err(|e| CoreError::io(output, e))?;
    enc.finish().map_err(|e| CoreError::io(output, e))?;

    Ok(report)
}
```

- [ ] **Step 3: Déclarer le module dans `lib.rs`**

```rust
pub mod export;

pub use export::{export, ExportOptions};
```

- [ ] **Step 4: Lancer les tests**

Run: `cargo test -p claudine-core export`
Expected: PASS (`export_contains_sessions_and_manifest_excludes_secrets`).

- [ ] **Step 5: Commit**

```bash
git add crates/claudine-core/src/export.rs crates/claudine-core/src/lib.rs
git commit -m "feat(core): export du bundle .tar.gz + manifest"
```

---

## Task 8 : Moteur de remap (`remap.rs`)

**Files:**
- Create: `crates/claudine-core/src/remap.rs`
- Modify: `crates/claudine-core/src/lib.rs`

**Interfaces:**
- Consumes: `serde_json`, `Result`, `CoreError`.
- Produces:
  - `pub struct RemapRule { pub from: String, pub to: String }`
  - `pub struct RemapTable { pub rules: Vec<RemapRule> }`
  - `RemapTable::new(rules: Vec<RemapRule>) -> RemapTable`
  - `RemapTable::apply_to_path(&self, s: &str) -> Option<String>` (remplacement du **plus long** préfixe correspondant)
  - `pub fn rewrite_jsonl_line(line: &str, table: &RemapTable) -> Result<(String, usize)>` (Err `JsonParse` si la ligne n'est pas du JSON ; renvoie `(ligne_réécrite, nb_remplacements)`)

- [ ] **Step 1: Écrire les tests (dans `remap.rs`)**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn table() -> RemapTable {
        RemapTable::new(vec![RemapRule {
            from: "/home/old".to_string(),
            to: "/home/new".to_string(),
        }])
    }

    #[test]
    fn apply_to_path_replaces_prefix() {
        let t = table();
        assert_eq!(t.apply_to_path("/home/old/proj").as_deref(), Some("/home/new/proj"));
        assert_eq!(t.apply_to_path("/other/x"), None);
    }

    #[test]
    fn apply_to_path_prefers_longest_match() {
        let t = RemapTable::new(vec![
            RemapRule { from: "/home".into(), to: "/A".into() },
            RemapRule { from: "/home/old".into(), to: "/B".into() },
        ]);
        assert_eq!(t.apply_to_path("/home/old/x").as_deref(), Some("/B/x"));
    }

    #[test]
    fn rewrite_line_rewrites_cwd_and_nested_paths() {
        let line = r#"{"cwd":"/home/old/proj","data":{"file":"/home/old/proj/a.rs"},"n":1}"#;
        let (out, count) = rewrite_jsonl_line(line, &table()).unwrap();
        let v: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert_eq!(v["cwd"], "/home/new/proj");
        assert_eq!(v["data"]["file"], "/home/new/proj/a.rs");
        assert_eq!(v["n"], 1);
        assert_eq!(count, 2);
    }

    #[test]
    fn rewrite_line_errors_on_non_json() {
        let err = rewrite_jsonl_line("pas du json", &table());
        assert!(err.is_err());
    }
}
```

- [ ] **Step 2: Écrire l'implémentation dans `remap.rs`**

```rust
use std::path::PathBuf;

use serde_json::Value;

use crate::error::{CoreError, Result};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemapRule {
    pub from: String,
    pub to: String,
}

#[derive(Debug, Clone, Default)]
pub struct RemapTable {
    pub rules: Vec<RemapRule>,
}

impl RemapTable {
    pub fn new(rules: Vec<RemapRule>) -> Self {
        Self { rules }
    }

    /// Remplace le **plus long** préfixe `from` correspondant. `None` si aucun.
    pub fn apply_to_path(&self, s: &str) -> Option<String> {
        let best = self
            .rules
            .iter()
            .filter(|r| s == r.from || s.starts_with(&format!("{}/", r.from)))
            .max_by_key(|r| r.from.len())?;
        Some(s.replacen(&best.from, &best.to, 1))
    }
}

/// Réécrit récursivement toute valeur chaîne qui correspond à une règle.
/// Renvoie la ligne re-sérialisée et le nombre de remplacements.
pub fn rewrite_jsonl_line(line: &str, table: &RemapTable) -> Result<(String, usize)> {
    let mut value: Value =
        serde_json::from_str(line).map_err(|e| CoreError::JsonParse {
            file: PathBuf::from("<ligne jsonl>"),
            line: 0,
            source: e,
        })?;
    let mut count = 0usize;
    rewrite_value(&mut value, table, &mut count);
    let out = serde_json::to_string(&value).map_err(|e| CoreError::JsonParse {
        file: PathBuf::from("<ligne jsonl>"),
        line: 0,
        source: e,
    })?;
    Ok((out, count))
}

fn rewrite_value(value: &mut Value, table: &RemapTable, count: &mut usize) {
    match value {
        Value::String(s) => {
            if let Some(replaced) = table.apply_to_path(s) {
                *s = replaced;
                *count += 1;
            }
        }
        Value::Array(arr) => {
            for v in arr {
                rewrite_value(v, table, count);
            }
        }
        Value::Object(map) => {
            for (_k, v) in map.iter_mut() {
                rewrite_value(v, table, count);
            }
        }
        _ => {}
    }
}
```

- [ ] **Step 3: Déclarer le module dans `lib.rs`**

```rust
pub mod remap;

pub use remap::{rewrite_jsonl_line, RemapRule, RemapTable};
```

- [ ] **Step 4: Lancer les tests**

Run: `cargo test -p claudine-core remap`
Expected: PASS (4 tests).

- [ ] **Step 5: Commit**

```bash
git add crates/claudine-core/src/remap.rs crates/claudine-core/src/lib.rs
git commit -m "feat(core): moteur de remap (RemapTable + rewrite_jsonl_line)"
```

---

## Task 9 : Lecture du manifest + dry-run d'import (`import.rs` partie 1)

**Files:**
- Create: `crates/claudine-core/src/import.rs`
- Modify: `crates/claudine-core/src/lib.rs`

**Interfaces:**
- Consumes: `ClaudeHome`, `Manifest`/`ManifestProject`/`SCHEMA_VERSION`, `RemapTable`, `encode_cwd`, `Report`, `Result`, `CoreError`, `tar`/`flate2`.
- Produces:
  - `pub struct ImportOptions { pub overwrite: bool }` + `Default` (`overwrite = false`)
  - `pub fn read_manifest(bundle: &Path) -> Result<Manifest>`
  - `pub fn dry_run(bundle: &Path, target: &ClaudeHome, table: &RemapTable, opts: &ImportOptions) -> Result<Report>` (n'écrit rien ; compteurs `projects`, `sessions_new`, `sessions_conflict`, `path_rewrites_planned`)

- [ ] **Step 1: Écrire les tests (dans `import.rs`)**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::export::{export, ExportOptions};
    use crate::home::ClaudeHome;
    use crate::remap::{RemapRule, RemapTable};
    use crate::testkit::FakeHome;

    fn make_bundle() -> (FakeHome, std::path::PathBuf) {
        let src = FakeHome::new();
        src.add_session(
            "-home-old-proj",
            "abc",
            &[r#"{"cwd":"/home/old/proj","timestamp":"t"}"#],
        );
        let out = src.base().join("bundle.tar.gz");
        export(
            &ClaudeHome::from_base(src.base()),
            &out,
            &ExportOptions::default(),
        )
        .unwrap();
        (src, out)
    }

    fn table() -> RemapTable {
        RemapTable::new(vec![RemapRule {
            from: "/home/old".into(),
            to: "/home/new".into(),
        }])
    }

    #[test]
    fn read_manifest_returns_projects() {
        let (_src, bundle) = make_bundle();
        let m = read_manifest(&bundle).unwrap();
        assert_eq!(m.schema_version, crate::manifest::SCHEMA_VERSION);
        assert_eq!(m.projects.len(), 1);
        assert_eq!(m.projects[0].cwd.as_deref(), Some("/home/old/proj"));
    }

    #[test]
    fn dry_run_counts_new_sessions_without_writing() {
        let (_src, bundle) = make_bundle();
        let target = FakeHome::new();
        let home = ClaudeHome::from_base(target.base());

        let report = dry_run(&bundle, &home, &table(), &ImportOptions::default()).unwrap();

        assert_eq!(report.count("projects"), 1);
        assert_eq!(report.count("sessions_new"), 1);
        assert_eq!(report.count("sessions_conflict"), 0);
        // rien n'a été écrit dans la cible
        assert!(!home.projects_dir().join("-home-new-proj").exists());
    }
}
```

- [ ] **Step 2: Écrire l'implémentation dans `import.rs`**

```rust
use std::fs::File;
use std::io::Read;
use std::path::Path;

use flate2::read::GzDecoder;

use crate::error::{CoreError, Report, Result};
use crate::home::ClaudeHome;
use crate::manifest::{Manifest, SCHEMA_VERSION};
use crate::pathcodec::encode_cwd;
use crate::remap::RemapTable;

#[derive(Debug, Clone, Default)]
pub struct ImportOptions {
    pub overwrite: bool,
}

fn open_archive(bundle: &Path) -> Result<tar::Archive<GzDecoder<File>>> {
    let f = File::open(bundle).map_err(|e| CoreError::io(bundle, e))?;
    Ok(tar::Archive::new(GzDecoder::new(f)))
}

pub fn read_manifest(bundle: &Path) -> Result<Manifest> {
    let mut archive = open_archive(bundle)?;
    let entries = archive
        .entries()
        .map_err(|e| CoreError::io(bundle, e))?;
    for entry in entries {
        let mut entry = entry.map_err(|e| CoreError::io(bundle, e))?;
        let path = entry.path().map_err(|e| CoreError::io(bundle, e))?;
        if path.to_string_lossy() == "manifest.json" {
            let mut buf = String::new();
            entry
                .read_to_string(&mut buf)
                .map_err(|e| CoreError::io(bundle, e))?;
            let manifest: Manifest = serde_json::from_str(&buf).map_err(|e| {
                CoreError::JsonParse {
                    file: bundle.to_path_buf(),
                    line: 0,
                    source: e,
                }
            })?;
            if manifest.schema_version != SCHEMA_VERSION {
                return Err(CoreError::ManifestVersion(manifest.schema_version));
            }
            return Ok(manifest);
        }
    }
    Err(CoreError::BundleFormat("manifest.json absent".to_string()))
}

/// Calcule le nouveau cwd (via la table) et le nouveau nom de dossier encodé.
fn target_dir_name(old_cwd: Option<&str>, table: &RemapTable) -> Option<String> {
    let cwd = old_cwd?;
    let new_cwd = table.apply_to_path(cwd).unwrap_or_else(|| cwd.to_string());
    Some(encode_cwd(&new_cwd))
}

pub fn dry_run(
    bundle: &Path,
    target: &ClaudeHome,
    table: &RemapTable,
    _opts: &ImportOptions,
) -> Result<Report> {
    let manifest = read_manifest(bundle)?;
    let mut report = Report::default();
    for p in &manifest.projects {
        report.bump("projects", 1);
        let new_dir = target_dir_name(p.cwd.as_deref(), table)
            .unwrap_or_else(|| p.encoded_name.clone());
        for sid in &p.session_ids {
            let dest = target
                .projects_dir()
                .join(&new_dir)
                .join(format!("{sid}.jsonl"));
            if dest.exists() {
                report.bump("sessions_conflict", 1);
            } else {
                report.bump("sessions_new", 1);
            }
        }
        if p.cwd.is_some() && table.apply_to_path(p.cwd.as_deref().unwrap()).is_some() {
            report.bump("path_rewrites_planned", p.session_ids.len());
        }
    }
    Ok(report)
}
```

- [ ] **Step 3: Déclarer le module dans `lib.rs`**

```rust
pub mod import;

pub use import::{dry_run, read_manifest, ImportOptions};
```

- [ ] **Step 4: Lancer les tests**

Run: `cargo test -p claudine-core import`
Expected: PASS (`read_manifest_returns_projects`, `dry_run_counts_new_sessions_without_writing`).

- [ ] **Step 5: Commit**

```bash
git add crates/claudine-core/src/import.rs crates/claudine-core/src/lib.rs
git commit -m "feat(core): read_manifest + dry_run d'import"
```

---

## Task 10 : Application de l'import avec backup, remap et fusion (`import.rs` partie 2)

**Files:**
- Modify: `crates/claudine-core/src/import.rs`

**Interfaces:**
- Consumes: tout ce de la Task 9 + `rewrite_jsonl_line`.
- Produces:
  - `pub fn apply(bundle: &Path, target: &ClaudeHome, table: &RemapTable, opts: &ImportOptions) -> Result<Report>`
    - effectue un **backup horodaté** de `target.base` (si présent) sous `<base>/backups/pre-import-<unix_ts>`
    - extrait les sessions du bundle, réécrit `cwd` + chemins via `table`, place dans le **nouveau** dossier encodé
    - **skip** si le fichier destination existe (sauf `opts.overwrite`)
    - compteurs : `sessions_imported`, `sessions_skipped`, `lines_rewritten`, `lines_preserved`
    - lignes non parsables : recopiées verbatim + `report.warn(...)`

- [ ] **Step 1: Écrire le test round-trip (ajouter dans le `mod tests` de `import.rs`)**

```rust
    #[test]
    fn apply_remaps_paths_into_new_project_dir() {
        let (_src, bundle) = make_bundle();
        let target = FakeHome::new();
        let home = ClaudeHome::from_base(target.base());

        let report = apply(&bundle, &home, &table(), &ImportOptions::default()).unwrap();

        // La session est arrivée dans le dossier ré-encodé du nouveau cwd.
        let dest = home
            .projects_dir()
            .join("-home-new-proj")
            .join("abc.jsonl");
        assert!(dest.exists(), "session manquante: {dest:?}");

        // Le champ cwd interne a été remappé.
        let content = std::fs::read_to_string(&dest).unwrap();
        let v: serde_json::Value =
            serde_json::from_str(content.lines().next().unwrap()).unwrap();
        assert_eq!(v["cwd"], "/home/new/proj");

        assert_eq!(report.count("sessions_imported"), 1);
        assert_eq!(report.count("lines_rewritten"), 1);
    }

    #[test]
    fn apply_skips_existing_session_by_default() {
        let (_src, bundle) = make_bundle();
        let target = FakeHome::new();
        let home = ClaudeHome::from_base(target.base());
        // Pré-place une session en conflit (même chemin de destination).
        let pdir = home.projects_dir().join("-home-new-proj");
        std::fs::create_dir_all(&pdir).unwrap();
        std::fs::write(pdir.join("abc.jsonl"), "DÉJÀ LÀ").unwrap();

        let report = apply(&bundle, &home, &table(), &ImportOptions::default()).unwrap();

        assert_eq!(report.count("sessions_skipped"), 1);
        assert_eq!(report.count("sessions_imported"), 0);
        // Contenu d'origine préservé.
        assert_eq!(
            std::fs::read_to_string(pdir.join("abc.jsonl")).unwrap(),
            "DÉJÀ LÀ"
        );
        // Un backup a été créé.
        assert!(home.base.join("backups").exists());
    }
```

- [ ] **Step 2: Ajouter les imports nécessaires en haut de `import.rs`**

Compléter le bloc `use` existant avec :

```rust
use std::time::{SystemTime, UNIX_EPOCH};

use crate::remap::rewrite_jsonl_line;
```

(et ajouter `Read` est déjà importé ; ajouter `std::io::Write` n'est pas nécessaire car on passe par `std::fs::write`.)

- [ ] **Step 3: Écrire `apply` (et ses helpers) dans `import.rs`**

```rust
fn backup_existing(target: &ClaudeHome) -> Result<Option<std::path::PathBuf>> {
    let projects = target.projects_dir();
    if !projects.exists() {
        return Ok(None);
    }
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let backup_root = target.base.join("backups").join(format!("pre-import-{ts}"));
    let dest = backup_root.join("projects");
    copy_dir_all(&projects, &dest)?;
    Ok(Some(backup_root))
}

fn copy_dir_all(src: &Path, dst: &Path) -> Result<()> {
    std::fs::create_dir_all(dst).map_err(|e| CoreError::io(dst, e))?;
    for entry in std::fs::read_dir(src).map_err(|e| CoreError::io(src, e))? {
        let entry = entry.map_err(|e| CoreError::io(src, e))?;
        let path = entry.path();
        let target = dst.join(entry.file_name());
        if path.is_dir() {
            copy_dir_all(&path, &target)?;
        } else {
            std::fs::copy(&path, &target).map_err(|e| CoreError::io(&path, e))?;
        }
    }
    Ok(())
}

/// Réécrit le contenu d'une session ligne par ligne ; préserve les lignes
/// non parsables (verbatim) et compte les réécritures.
fn rewrite_session(content: &str, table: &RemapTable, report: &mut Report) -> String {
    let mut out_lines = Vec::new();
    for line in content.lines() {
        if line.trim().is_empty() {
            out_lines.push(String::new());
            continue;
        }
        match rewrite_jsonl_line(line, table) {
            Ok((rewritten, n)) => {
                if n > 0 {
                    report.bump("lines_rewritten", 1);
                } else {
                    report.bump("lines_preserved", 1);
                }
                out_lines.push(rewritten);
            }
            Err(_) => {
                report.warn(format!("ligne non parsable préservée verbatim"));
                report.bump("lines_preserved", 1);
                out_lines.push(line.to_string());
            }
        }
    }
    out_lines.join("\n")
}

pub fn apply(
    bundle: &Path,
    target: &ClaudeHome,
    table: &RemapTable,
    opts: &ImportOptions,
) -> Result<Report> {
    let manifest = read_manifest(bundle)?;
    let mut report = Report::default();

    // 1. Backup avant toute mutation.
    backup_existing(target)?;

    // 2. Index cwd source -> nouveau dossier encodé (depuis le manifest).
    let dir_for = |encoded: &str| -> String {
        manifest
            .projects
            .iter()
            .find(|p| p.encoded_name == encoded)
            .and_then(|p| target_dir_name(p.cwd.as_deref(), table))
            .unwrap_or_else(|| encoded.to_string())
    };

    // 3. Parcourt les entrées `projects/<encoded>/<id>.jsonl` du bundle.
    let mut archive = open_archive(bundle)?;
    let entries = archive.entries().map_err(|e| CoreError::io(bundle, e))?;
    for entry in entries {
        let mut entry = entry.map_err(|e| CoreError::io(bundle, e))?;
        let path = entry.path().map_err(|e| CoreError::io(bundle, e))?.into_owned();
        let comps: Vec<String> = path
            .components()
            .map(|c| c.as_os_str().to_string_lossy().into_owned())
            .collect();
        if comps.len() != 3 || comps[0] != "projects" {
            continue;
        }
        let encoded = &comps[1];
        let filename = &comps[2];
        let new_dir = dir_for(encoded);
        let dest_dir = target.projects_dir().join(&new_dir);
        let dest = dest_dir.join(filename);

        if dest.exists() && !opts.overwrite {
            report.bump("sessions_skipped", 1);
            continue;
        }

        let mut content = String::new();
        entry
            .read_to_string(&mut content)
            .map_err(|e| CoreError::io(&dest, e))?;
        let rewritten = rewrite_session(&content, table, &mut report);

        std::fs::create_dir_all(&dest_dir).map_err(|e| CoreError::io(&dest_dir, e))?;
        // Écriture temp + rename.
        let tmp = dest_dir.join(format!("{filename}.tmp"));
        std::fs::write(&tmp, rewritten.as_bytes()).map_err(|e| CoreError::io(&tmp, e))?;
        std::fs::rename(&tmp, &dest).map_err(|e| CoreError::io(&dest, e))?;
        report.bump("sessions_imported", 1);
    }

    Ok(report)
}
```

- [ ] **Step 4: Lancer les tests**

Run: `cargo test -p claudine-core import`
Expected: PASS (les 4 tests d'import, dont `apply_remaps_paths_into_new_project_dir` et `apply_skips_existing_session_by_default`).

- [ ] **Step 5: Lancer toute la suite du cœur**

Run: `cargo test -p claudine-core`
Expected: tous les tests passent.

- [ ] **Step 6: Commit**

```bash
git add crates/claudine-core/src/import.rs
git commit -m "feat(core): apply import (backup + remap + fusion skip/overwrite)"
```

---

## Task 11 : CLI `claudine` (export / import)

**Files:**
- Create: `crates/claudine/src/cli.rs`
- Modify: `crates/claudine/src/main.rs`
- Create: `crates/claudine/tests/cli.rs`

**Interfaces:**
- Consumes: `claudine_core::{ClaudeHome, export, ExportOptions, apply, dry_run, ImportOptions, RemapRule, RemapTable, Report}`.
- Produces: binaire `claudine` avec :
  - `claudine export --out <fichier.tar.gz> [--no-history]`
  - `claudine import <bundle.tar.gz> [--map ANCIEN=NOUVEAU ...] [--dry-run] [--overwrite]`
  - sans sous-commande → placeholder TUI.

- [ ] **Step 1: Écrire `crates/claudine/src/cli.rs`**

```rust
use std::path::PathBuf;

use claudine_core::{
    apply, dry_run, export, ClaudeHome, ExportOptions, ImportOptions, RemapRule, RemapTable,
    Report,
};

pub fn parse_maps(maps: &[String]) -> Result<RemapTable, String> {
    let mut rules = Vec::new();
    for m in maps {
        let (from, to) = m
            .split_once('=')
            .ok_or_else(|| format!("--map invalide (attendu ANCIEN=NOUVEAU): {m}"))?;
        rules.push(RemapRule {
            from: from.to_string(),
            to: to.to_string(),
        });
    }
    Ok(RemapTable::new(rules))
}

pub fn format_report(report: &Report) -> String {
    let mut out = String::from("Rapport :\n");
    for (k, v) in &report.counts {
        out.push_str(&format!("  {k}: {v}\n"));
    }
    if !report.warnings.is_empty() {
        out.push_str(&format!("Avertissements ({}):\n", report.warnings.len()));
        for w in &report.warnings {
            out.push_str(&format!("  - {w}\n"));
        }
    }
    out
}

pub fn run_export(out: PathBuf, no_history: bool) -> Result<(), String> {
    let home = ClaudeHome::discover().map_err(|e| e.to_string())?;
    let opts = ExportOptions {
        include_history: !no_history,
    };
    let report = export(&home, &out, &opts).map_err(|e| e.to_string())?;
    print!("{}", format_report(&report));
    println!("Bundle écrit : {}", out.display());
    Ok(())
}

pub fn run_import(
    bundle: PathBuf,
    maps: Vec<String>,
    dry_run_only: bool,
    overwrite: bool,
) -> Result<(), String> {
    let home = ClaudeHome::discover().map_err(|e| e.to_string())?;
    let table = parse_maps(&maps)?;
    let opts = ImportOptions { overwrite };
    let report = if dry_run_only {
        dry_run(&bundle, &home, &table, &opts).map_err(|e| e.to_string())?
    } else {
        apply(&bundle, &home, &table, &opts).map_err(|e| e.to_string())?
    };
    print!("{}", format_report(&report));
    if dry_run_only {
        println!("(dry-run : rien n'a été écrit)");
    }
    Ok(())
}
```

- [ ] **Step 2: Remplacer `crates/claudine/src/main.rs`**

```rust
mod cli;

use std::path::PathBuf;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "claudine", about = "Navigateur/gestionnaire des données Claude Code")]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// Exporte ~/.claude dans un bundle .tar.gz
    Export {
        #[arg(long)]
        out: PathBuf,
        #[arg(long)]
        no_history: bool,
    },
    /// Importe un bundle (avec remap des chemins)
    Import {
        bundle: PathBuf,
        #[arg(long = "map")]
        maps: Vec<String>,
        #[arg(long)]
        dry_run: bool,
        #[arg(long)]
        overwrite: bool,
    },
}

fn main() {
    let cli = Cli::parse();
    let result = match cli.command {
        None => {
            println!("Claudine — TUI à venir (phase 2). Essayez `claudine --help`.");
            Ok(())
        }
        Some(Command::Export { out, no_history }) => cli::run_export(out, no_history),
        Some(Command::Import {
            bundle,
            maps,
            dry_run,
            overwrite,
        }) => cli::run_import(bundle, maps, dry_run, overwrite),
    };
    if let Err(e) = result {
        eprintln!("Erreur : {e}");
        std::process::exit(1);
    }
}
```

- [ ] **Step 3: Écrire le test d'intégration `crates/claudine/tests/cli.rs`**

```rust
use assert_cmd::Command;
use predicates::str::contains;

fn fake_home_with_session(base: &std::path::Path) {
    let pdir = base.join("projects").join("-home-old-proj");
    std::fs::create_dir_all(&pdir).unwrap();
    std::fs::write(
        pdir.join("abc.jsonl"),
        r#"{"cwd":"/home/old/proj","timestamp":"t"}"#,
    )
    .unwrap();
}

#[test]
fn bare_invocation_prints_placeholder() {
    Command::cargo_bin("claudine")
        .unwrap()
        .assert()
        .success()
        .stdout(contains("TUI à venir"));
}

#[test]
fn export_then_import_dry_run_roundtrip() {
    let src = tempfile::tempdir().unwrap();
    fake_home_with_session(src.path());
    let bundle = src.path().join("bundle.tar.gz");

    // export
    Command::cargo_bin("claudine")
        .unwrap()
        .env("CLAUDE_CONFIG_DIR", src.path())
        .args(["export", "--out", bundle.to_str().unwrap()])
        .assert()
        .success()
        .stdout(contains("Bundle écrit"));
    assert!(bundle.exists());

    // import dry-run dans une nouvelle home
    let dst = tempfile::tempdir().unwrap();
    Command::cargo_bin("claudine")
        .unwrap()
        .env("CLAUDE_CONFIG_DIR", dst.path())
        .args([
            "import",
            bundle.to_str().unwrap(),
            "--map",
            "/home/old=/home/new",
            "--dry-run",
        ])
        .assert()
        .success()
        .stdout(contains("sessions_new: 1"))
        .stdout(contains("dry-run"));

    // la cible n'a rien reçu
    assert!(!dst.path().join("projects/-home-new-proj").exists());
}
```

- [ ] **Step 4: Lancer les tests de la CLI**

Run: `cargo test -p claudine`
Expected: PASS (`bare_invocation_prints_placeholder`, `export_then_import_dry_run_roundtrip`).

- [ ] **Step 5: Vérification manuelle rapide (optionnelle mais recommandée)**

Run: `cargo run -p claudine -- --help`
Expected: l'aide liste les sous-commandes `export` et `import`.

- [ ] **Step 6: Lancer toute la suite**

Run: `cargo test`
Expected: tous les tests des deux crates passent.

- [ ] **Step 7: Commit**

```bash
git add crates/claudine/src/cli.rs crates/claudine/src/main.rs crates/claudine/tests/cli.rs
git commit -m "feat(cli): commandes export/import + placeholder TUI"
```

---

## Self-Review (effectuée)

**1. Couverture de la spec :**
- §3 Architecture (workspace, binaire unique, briques) → Task 1 + Cargo.toml. ✅
- §4 Modèle (ClaudeHome, Project/SessionMeta, cwd depuis jsonl) → Tasks 3, 5. ✅
- §4 Encodage `/` et `.` → Task 4. ✅
- §5 Format de bundle + manifest + exclusions secrets → Tasks 6, 7. ✅
- §6 Flux export (lecture seule) → Task 7. ✅
- §7 Flux import + remap (table, dry-run, backup, fusion skip, temp+rename) → Tasks 8, 9, 10. ✅
- §8 Erreurs (CoreError, jamais de perte silencieuse) → Task 2 + préservation verbatim Task 10. ✅
- §9 Sûreté (backup, dry-run, temp+rename) → Tasks 9, 10. ✅
- §10 Tests (unitaires tempdir, golden remap, round-trip, CLI) → Tasks 5, 8, 10, 11. ✅
- §11 Phase 1 = cœur + CLI migration → tout le plan. ✅
- Écarts assumés (documentés en Global Constraints) : `~/.claude.json` non exporté en phase 1 ; pas de `ratatui`.

**2. Placeholders :** aucun « TODO/TBD/à compléter » dans le code des steps. Le seul `report.warn("ligne non parsable…")` est du contenu réel.

**3. Cohérence des types :** `Report` (counts/warn/bump/count), `ClaudeHome` (from_base/discover/*_dir/*_file), `RemapTable::new`/`apply_to_path`, `rewrite_jsonl_line -> (String, usize)`, `encode_cwd`, `export(home, &Path, &ExportOptions) -> Report`, `dry_run`/`apply(bundle, &ClaudeHome, &RemapTable, &ImportOptions) -> Report` — signatures identiques entre tâches productrices et consommatrices. ✅
