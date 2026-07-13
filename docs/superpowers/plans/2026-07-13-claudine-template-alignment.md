# Alignement de claudine sur le template + josephine — Plan d'implémentation

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Réaligner claudine (claude-tui) sur le standard `_templates/cli` + josephine, in-place, sans régression : edition 2024, `cargo fmt`, core absorbant cli+tui, CI/release/packaging complets, et un site Zola.

**Architecture:** Le workspace passe en edition 2024 / MSRV 1.85. `claudine-core` absorbe `cli.rs` + `commands/*` + `tui/*` + la logique existante (gagne `clap`, `ratatui`, `anyhow`) ; le binaire `claudine` devient un shim `fn main() -> ExitCode { claudine_core::run() }`. On adopte `cargo fmt` (reformatage unique isolé). CI/release/pages/packaging et le site Zola se calquent sur josephine (référence réalisée sur disque).

**Tech Stack:** Rust (workspace 2 crates), clap 4 (derive), ratatui 0.28 (crossterm via ratatui), anyhow, thiserror, serde/serde_json, tar/flate2. Zola 0.21 (site). GitHub Actions (ci/release/pages). cargo-deb / cargo-generate-rpm / Homebrew / AUR / winget (packaging).

## Global Constraints

*(Valeurs copiées verbatim de la spec — chaque tâche les hérite implicitement.)*

- **Edition `2024`, MSRV `rust-version = "1.85"`** ; `rust-toolchain.toml` pin `channel = "stable"` + components `[rustfmt, clippy]`.
- **`rustfmt.toml`** : `edition = "2024"`, `max_width = 100`. `cargo fmt --check` doit passer à partir de la tâche 2.
- **`[workspace.lints.rust]`** : `unsafe_code = "forbid"`. **`[workspace.lints.clippy]`** : `all = { level = "warn", priority = -1 }`. Chaque crate active `[lints] workspace = true`.
- **`[profile.release]`** : `lto = true`, `codegen-units = 1`, `strip = true`.
- **Structure « à la lettre »** : `claudine-core` porte `cli.rs` + `commands/*` + `tui/*` + toute la logique ; `crates/claudine/src/main.rs` = shim de 3 lignes.
- **Aucune régression fonctionnelle.** Les 162 tests existants restent verts à chaque tâche (ils constituent le filet du refactoring). Comportement CLI/TUI identique.
- **Multi-OS conservé** (Linux x86_64, Windows x86_64, macOS aarch64). Ne pas réduire à Linux-only.
- **Licence** `MIT OR Apache-2.0` (les deux fichiers existent déjà).
- **Conventional Commits**, **Keep a Changelog**, **SemVer**. Version en sortie d'alignement : **`0.1.0`**.
- **Valeurs projet** : `brand_color = "#d97757"`, repo `https://github.com/systm-d/claudine`, `base_url = "https://systm-d.github.io/claudine"`, maintainer/auteur `k@levilainpetit.dev`.
- **Langue** : strings utilisateur en français autorisés ; identifiants de code et docs techniques en anglais. Les messages d'erreur français existants (`Erreur : …`) sont **conservés**.
- **Référence sur disque** : `_templates/cli` = `/home/kdelfour/Workspace/Professionel/_templates/cli` ; josephine = `/home/kdelfour/Workspace/Professionel/systm-D/josephine`. Le scaffold de référence généré est dans le scratchpad (`.../scratchpad/ref-scaffold/claudine`).
- **Merge sur `main` uniquement sur feu vert explicite de l'utilisateur.**

Porte qualité de référence (adapter par tâche) :

```
cargo fmt --check                                        # à partir de la tâche 2
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo build --release
cd site && zola build                                    # à partir de la tâche 10
```

---

## Table des tâches

- **A — Fondation & restructuration** : T1 config+edition · T2 fmt unique · T3 TUI→core · T4 CLI→core + shim
- **B — Standards & CI/packaging** : T5 configs deny/tarpaulin/audit · T6 docs CONVENTIONS/CLAUDE/AGENTS · T7 ci.yml · T8 release.yml · T9 packaging+dependabot+CODEOWNERS
- **C — Web** : T10 site Zola · T11 pages.yml
- **Clôture** : T12 version 0.1.0 + CHANGELOG + README + tests/cli.rs

---

### Task 1: Fondation workspace + passage en edition 2024

**Files:**
- Modify: `Cargo.toml` (racine, `[workspace.package]` + nouvelles sections)
- Modify: `crates/claudine-core/Cargo.toml`
- Modify: `crates/claudine/Cargo.toml`
- Create: `rust-toolchain.toml`
- Create: `rustfmt.toml`

**Interfaces:**
- Produces: workspace edition 2024 / MSRV 1.85 ; `[workspace.dependencies]` déclarant `serde, serde_json, thiserror, tar, flate2, clap, ratatui, anyhow, assert_cmd, predicates, tempfile` ; `[workspace.lints]` ; `[profile.release]`. Consommé par toutes les tâches suivantes.

- [ ] **Step 1: Réécrire le `Cargo.toml` racine**

Remplacer le contenu de `Cargo.toml` par :

```toml
[workspace]
resolver = "2"
members = ["crates/claudine-core", "crates/claudine"]

[workspace.package]
edition = "2024"
rust-version = "1.85"
version = "0.0.2"
license = "MIT OR Apache-2.0"
description = "Outil Rust TUI/CLI pour naviguer et gérer les données locales de Claude Code (~/.claude)."
repository = "https://github.com/systm-d/claudine"
homepage = "https://github.com/systm-d/claudine"
authors = ["systm-d <k@levilainpetit.dev>"]

[workspace.dependencies]
serde = { version = "1", features = ["derive"] }
serde_json = { version = "1", features = ["preserve_order"] }
thiserror = "1"
tar = "0.4"
flate2 = "1"
clap = { version = "4", features = ["derive"] }
ratatui = "0.28"
anyhow = "1"
assert_cmd = "2"
predicates = "3"
tempfile = "3"

[workspace.lints.rust]
unsafe_code = "forbid"

[workspace.lints.clippy]
all = { level = "warn", priority = -1 }

[profile.release]
lto = true
codegen-units = 1
strip = true
```

*(La version reste `0.0.2` ici ; le bump `0.1.0` est en tâche 12.)*

- [ ] **Step 2: Adapter `crates/claudine-core/Cargo.toml`**

Remplacer les sections `[dependencies]`/`[dev-dependencies]` et ajouter `[lints]` :

```toml
[package]
name = "claudine-core"
edition.workspace = true
rust-version.workspace = true
version.workspace = true
license.workspace = true
description.workspace = true
repository.workspace = true
homepage.workspace = true
authors.workspace = true

[dependencies]
serde.workspace = true
serde_json.workspace = true
thiserror.workspace = true
tar.workspace = true
flate2.workspace = true

[dev-dependencies]
tempfile.workspace = true

[lints]
workspace = true
```

- [ ] **Step 3: Adapter `crates/claudine/Cargo.toml`**

Conserver le bloc `[[bin]]`, `[package.metadata.deb]` et `[package.metadata.generate-rpm]` **inchangés**. Remplacer uniquement les blocs `[dependencies]`/`[dev-dependencies]` et ajouter `[lints]` :

```toml
[dependencies]
claudine-core = { path = "../claudine-core" }
clap.workspace = true
ratatui.workspace = true
serde_json.workspace = true

[dev-dependencies]
assert_cmd.workspace = true
predicates.workspace = true
tempfile.workspace = true

[lints]
workspace = true
```

- [ ] **Step 4: Créer `rust-toolchain.toml`**

```toml
[toolchain]
channel = "stable"
components = ["rustfmt", "clippy"]
```

- [ ] **Step 5: Créer `rustfmt.toml`**

```toml
edition = "2024"
max_width = 100
```

- [ ] **Step 6: Migration edition 2024**

Run: `cargo fix --edition --workspace --allow-dirty --allow-no-vcs-commit 2>&1 | tail -20`
Expected: se termine sans erreur (peut n'appliquer aucun changement). Relire le diff éventuel.

- [ ] **Step 7: Vérifier l'absence d'`unsafe` (contrainte `unsafe_code = "forbid"`)**

Run: `grep -rn "unsafe" crates/*/src --include='*.rs' || echo "AUCUN unsafe"`
Expected: `AUCUN unsafe` (sinon, traiter avant de continuer — la compilation échouerait).

- [ ] **Step 8: Compiler + clippy + tests (sans `fmt --check` — le code n'est pas encore reformaté)**

Run: `cargo build --workspace && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace 2>&1 | tail -15`
Expected: build OK, 0 warning clippy, `test result: ok` sur les 162 tests. Corriger tout warning clippy nouveau introduit par `[workspace.lints]`.

- [ ] **Step 9: Commit**

```bash
git add Cargo.toml Cargo.lock crates/claudine-core/Cargo.toml crates/claudine/Cargo.toml rust-toolchain.toml rustfmt.toml
git commit -m "chore(build): edition 2024, MSRV 1.85, workspace lints + deps + release profile"
```

---

### Task 2: Reformatage unique (`cargo fmt`)

**Files:**
- Modify: tous les `.rs` du workspace (reformatage mécanique)

**Interfaces:**
- Produces: un arbre `.rs` conforme à `rustfmt.toml`. À partir d'ici, `cargo fmt --check` fait partie de la porte qualité de chaque tâche.

- [ ] **Step 1: Constater que `fmt --check` échoue (état hand-formatted)**

Run: `cargo fmt --check > /dev/null 2>&1; echo "exit=$?"`
Expected: `exit=1` (le code n'est pas au format rustfmt).

- [ ] **Step 2: Reformater tout le workspace**

Run: `cargo fmt`
Expected: aucune sortie (succès).

- [ ] **Step 3: Vérifier `fmt --check` + tests (le reformatage ne change pas le comportement)**

Run: `cargo fmt --check && cargo test --workspace 2>&1 | tail -6`
Expected: `fmt --check` sans sortie (exit 0) ; `test result: ok` sur les 162 tests.

- [ ] **Step 4: Commit (isolé, purement mécanique)**

```bash
git add -A
git commit -m "style: reformat workspace with cargo fmt (edition 2024, max_width 100)"
```

---

### Task 3: Déplacer la TUI dans `claudine-core`

**Files:**
- Create: `crates/claudine-core/src/tui/{mod,app,ui,hooks_editor,mcp_editor,settings_form,marketplaces}.rs` (déplacés)
- Delete: `crates/claudine/src/tui/*`
- Modify: `crates/claudine-core/src/lib.rs` (ajout `pub mod tui;`)
- Modify: `crates/claudine-core/Cargo.toml` (ajout `ratatui`)
- Modify: `crates/claudine/Cargo.toml` (retrait `ratatui`)
- Modify: `crates/claudine/src/main.rs` (`mod tui;` → `claudine_core::tui`)

**Interfaces:**
- Consumes (T1) : `[workspace.dependencies] ratatui`.
- Produces : `claudine_core::tui::run() -> std::io::Result<()>` — point d'entrée TUI, consommé par la tâche 4.

- [ ] **Step 1: Déplacer les fichiers TUI dans le core**

```bash
git mv crates/claudine/src/tui crates/claudine-core/src/tui
```

- [ ] **Step 2: Réécrire les imports `claudine_core::` → `crate::` dans les fichiers TUI**

Dans chaque fichier de `crates/claudine-core/src/tui/`, tout import du crate courant doit passer par `crate::` au lieu de `claudine_core::`.

Run: `grep -rln "claudine_core" crates/claudine-core/src/tui`
Puis remplacer, dans chaque fichier listé, `claudine_core::` par `crate::` (chemins d'`use` et chemins qualifiés). Exemple dans `mod.rs` : `use claudine_core::discover_homes;` → `use crate::discover_homes;`.

Run après édition : `grep -rn "claudine_core" crates/claudine-core/src/tui || echo "OK aucun résidu"`
Expected: `OK aucun résidu`.

- [ ] **Step 3: Déclarer le module `tui` dans `lib.rs`**

Dans `crates/claudine-core/src/lib.rs`, après la liste des `pub mod …` existants (après `pub mod marketplaces;`), ajouter :

```rust
pub mod tui;
```

- [ ] **Step 4: Ajouter `ratatui` au core, le retirer du binaire**

Dans `crates/claudine-core/Cargo.toml`, section `[dependencies]`, ajouter après `flate2.workspace = true` :

```toml
ratatui.workspace = true
```

Dans `crates/claudine/Cargo.toml`, section `[dependencies]`, **supprimer** la ligne `ratatui.workspace = true`.

- [ ] **Step 5: Adapter le binaire pour utiliser `claudine_core::tui`**

Dans `crates/claudine/src/main.rs`, supprimer la ligne `mod tui;` (le module n'existe plus dans le binaire) et remplacer l'appel `tui::run()` par `claudine_core::tui::run()` dans le bras `None` du `match`.

- [ ] **Step 6: fmt + clippy + tests**

Run: `cargo fmt && cargo fmt --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace 2>&1 | tail -8`
Expected: 0 warning, 162 tests verts (les tests TUI ont suivi dans le core).

- [ ] **Step 7: Commit**

```bash
git add -A
git commit -m "refactor(core): move TUI module into claudine-core"
```

---

### Task 4: Déplacer la CLI dans `claudine-core` + binaire shim

**Files:**
- Create: `crates/claudine-core/src/cli.rs`
- Create: `crates/claudine-core/src/commands/{mod,export,import,homes}.rs`
- Modify: `crates/claudine-core/src/lib.rs` (ajout `mod cli; mod commands; pub fn run()`)
- Modify: `crates/claudine-core/Cargo.toml` (ajout `clap`, `anyhow`)
- Delete: `crates/claudine/src/cli.rs`
- Rewrite: `crates/claudine/src/main.rs` (shim)
- Modify: `crates/claudine/Cargo.toml` (retrait `clap`, `serde_json`)

**Interfaces:**
- Consumes (T1) : `[workspace.dependencies] clap, anyhow`. (T3) : `crate::tui::run()`.
- Produces : `claudine_core::run() -> std::process::ExitCode` (consommé par le shim du binaire) ; `crate::cli::Cli` (Parser) ; `crate::commands::{export,import,homes}::run(...)`.

- [ ] **Step 1: Ajouter `clap` + `anyhow` au core**

Dans `crates/claudine-core/src/../Cargo.toml` (`crates/claudine-core/Cargo.toml`), `[dependencies]`, ajouter :

```toml
clap.workspace = true
anyhow.workspace = true
```

- [ ] **Step 2: Créer `crates/claudine-core/src/commands/mod.rs`**

```rust
//! Sous-commandes CLI : une par fichier, frontière IO fine sur la logique du core.

pub mod export;
pub mod homes;
pub mod import;
```

- [ ] **Step 3: Créer `crates/claudine-core/src/commands/homes.rs`**

Porter les fonctions `run_homes`, `run_homes_add`, `run_homes_remove`, `resolve_home` depuis l'ancien `crates/claudine/src/cli.rs`, en remplaçant `claudine_core::` par `crate::` et en gardant le type d'erreur `Result<(), String>` (comportement identique). Le contenu :

```rust
use std::path::{Path, PathBuf};

use crate::{discover_homes, scan_projects, ClaudeHome, ClaudineConfig};

/// Résout l'argument `--home` : étiquette d'une home découverte, ou chemin de
/// système de fichiers. `None` retombe sur `ClaudeHome::discover()`.
pub fn resolve_home(home_arg: Option<&str>) -> Result<ClaudeHome, String> {
    let Some(value) = home_arg else {
        return ClaudeHome::discover().map_err(|e| e.to_string());
    };

    let homes = discover_homes();
    if let Some(home) = homes.iter().find(|h| h.label == value) {
        return Ok(home.clone());
    }

    let path = Path::new(value);
    if path.is_dir() {
        return Ok(ClaudeHome::from_base(path));
    }

    let labels: Vec<&str> = homes.iter().map(|h| h.label.as_str()).collect();
    Err(format!(
        "home introuvable : « {value} » n'est ni une étiquette connue ni un répertoire existant.\nHomes disponibles : {}",
        if labels.is_empty() {
            "(aucune)".to_string()
        } else {
            labels.join(", ")
        }
    ))
}

pub fn run_homes() -> Result<(), String> {
    let homes = discover_homes();
    if homes.is_empty() {
        println!("Aucune home Claude découverte.");
        return Ok(());
    }
    for (i, home) in homes.iter().enumerate() {
        let n = scan_projects(home).map(|p| p.len()).unwrap_or(0);
        let mark = if i == 0 { "*" } else { " " };
        println!("{mark} {}  {}  ({n} projets)", home.label, home.base.display());
    }
    Ok(())
}

/// Enregistre une home dans la config Claudine puis sauvegarde.
pub fn run_homes_add(path: PathBuf, label: Option<String>) -> Result<(), String> {
    if !path.is_dir() {
        return Err(format!("le chemin « {} » n'est pas un répertoire existant", path.display()));
    }
    let mut config = ClaudineConfig::load();
    config.add_home(label.unwrap_or_default(), path.clone());
    config.save().map_err(|e| e.to_string())?;
    println!("Home enregistrée : {}", path.display());
    Ok(())
}

/// Retire une home enregistrée de la config Claudine puis sauvegarde.
pub fn run_homes_remove(label: String) -> Result<(), String> {
    let mut config = ClaudineConfig::load();
    let before = config.homes.len();
    config.remove_home(&label);
    if config.homes.len() == before {
        return Err(format!(
            "aucune home enregistrée nommée « {label} » (seules les homes enregistrées sont retirables)"
        ));
    }
    config.save().map_err(|e| e.to_string())?;
    println!("Home retirée : {label}");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_home_accepts_existing_path() {
        let dir = tempfile::tempdir().unwrap();
        let home = resolve_home(Some(dir.path().to_str().unwrap())).unwrap();
        assert_eq!(home.base, dir.path());
    }

    #[test]
    fn resolve_home_rejects_unknown_value() {
        let err = resolve_home(Some("/n/existe/pas/du/tout-claudine")).unwrap_err();
        assert!(err.contains("home introuvable"));
    }
}
```

*(Note : `tempfile` est déjà dev-dep du core.)*

- [ ] **Step 4: Créer `crates/claudine-core/src/commands/export.rs`**

```rust
use std::path::PathBuf;

use crate::commands::homes::resolve_home;
use crate::{export, ExportOptions, Report};

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

pub fn run_export(out: PathBuf, no_history: bool, home_arg: Option<String>) -> Result<(), String> {
    let home = resolve_home(home_arg.as_deref())?;
    let opts = ExportOptions { include_history: !no_history };
    let report = export(&home, &out, &opts).map_err(|e| e.to_string())?;
    print!("{}", format_report(&report));
    println!("Bundle écrit : {}", out.display());
    Ok(())
}
```

- [ ] **Step 5: Créer `crates/claudine-core/src/commands/import.rs`**

```rust
use std::path::PathBuf;

use crate::commands::export::format_report;
use crate::commands::homes::resolve_home;
use crate::{apply, dry_run, ImportOptions, RemapRule, RemapTable};

pub fn parse_maps(maps: &[String]) -> Result<RemapTable, String> {
    let mut rules = Vec::new();
    for m in maps {
        let (from, to) = m
            .split_once('=')
            .ok_or_else(|| format!("--map invalide (attendu ANCIEN=NOUVEAU): {m}"))?;
        rules.push(RemapRule { from: from.to_string(), to: to.to_string() });
    }
    Ok(RemapTable::new(rules))
}

pub fn run_import(
    bundle: PathBuf,
    maps: Vec<String>,
    dry_run_only: bool,
    overwrite: bool,
    home_arg: Option<String>,
) -> Result<(), String> {
    let home = resolve_home(home_arg.as_deref())?;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_maps_ok() {
        let t = parse_maps(&["/home/old=/home/new".to_string()]).unwrap();
        assert_eq!(t.rules.len(), 1);
        assert_eq!(t.rules[0].from, "/home/old");
        assert_eq!(t.rules[0].to, "/home/new");
    }

    #[test]
    fn parse_maps_rejects_missing_equals() {
        assert!(parse_maps(&["noeq".to_string()]).is_err());
    }
}
```

- [ ] **Step 6: Créer `crates/claudine-core/src/cli.rs`**

Porter la déclaration clap depuis l'ancien `crates/claudine/src/main.rs`, en un `Cli` avec méthode `run()` (dispatch mince), lançant la TUI sur invocation nue :

```rust
use std::path::PathBuf;

use clap::{Parser, Subcommand};

use crate::commands;

/// Navigateur/gestionnaire des données Claude Code.
#[derive(Parser)]
#[command(name = "claudine", version, about = "Navigateur/gestionnaire des données Claude Code")]
pub struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// Exporte une home Claude dans un bundle .tar.gz
    Export {
        #[arg(long)]
        out: PathBuf,
        #[arg(long)]
        no_history: bool,
        /// Étiquette d'une home découverte (ex. .claude-perso) ou chemin
        #[arg(long)]
        home: Option<String>,
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
        /// Étiquette d'une home découverte (ex. .claude-perso) ou chemin
        #[arg(long)]
        home: Option<String>,
    },
    /// Gère les homes Claude (liste / ajout / retrait)
    Homes {
        #[command(subcommand)]
        action: Option<HomesAction>,
    },
}

#[derive(Subcommand)]
enum HomesAction {
    /// Enregistre une home dans la config Claudine
    Add {
        /// Chemin du répertoire de la home (ex. ~/.claude-perso)
        path: PathBuf,
        /// Étiquette explicite (sinon dérivée du dernier composant)
        #[arg(long)]
        label: Option<String>,
    },
    /// Retire une home enregistrée de la config Claudine
    Remove {
        /// Étiquette de la home à retirer
        label: String,
    },
}

impl Cli {
    /// Dispatch mince — aucune logique ici. Conserve le type d'erreur `String`
    /// des commandes (messages utilisateur français inchangés).
    pub fn run(self) -> Result<(), String> {
        match self.command {
            // Invocation nue : lance la TUI interactive.
            None => crate::tui::run().map_err(|e| e.to_string()),
            Some(Command::Export { out, no_history, home }) => {
                commands::export::run_export(out, no_history, home)
            }
            Some(Command::Import { bundle, maps, dry_run, overwrite, home }) => {
                commands::import::run_import(bundle, maps, dry_run, overwrite, home)
            }
            Some(Command::Homes { action }) => match action {
                None => commands::homes::run_homes(),
                Some(HomesAction::Add { path, label }) => commands::homes::run_homes_add(path, label),
                Some(HomesAction::Remove { label }) => commands::homes::run_homes_remove(label),
            },
        }
    }
}
```

- [ ] **Step 7: Déclarer `cli` + `commands` + `run()` dans `lib.rs`**

Dans `crates/claudine-core/src/lib.rs`, ajouter les deux modules **après** `pub mod tui;` (garder `cli`/`commands` privés — seul `run()` est public) :

```rust
mod cli;
mod commands;
```

Puis, à la fin du bloc des `pub use …` (après le bloc `pub use marketplaces::{ … };`), ajouter la fonction d'entrée :

```rust
use std::process::ExitCode;

/// Parse les arguments, dispatch, et mappe l'erreur vers un code de sortie.
pub fn run() -> ExitCode {
    use clap::Parser;
    match cli::Cli::parse().run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("Erreur : {e}");
            ExitCode::FAILURE
        }
    }
}
```

*(`anyhow` est déclaré en dépendance pour la convention du template ; les commandes conservent `Result<(), String>` pour préserver à l'identique les messages d'erreur français. `anyhow` reste disponible pour du code futur — s'il déclenche un warning d'import inutilisé, ne pas l'importer : il n'est pas `use`d ici.)*

> ⚠️ Si `cargo` signale `anyhow` comme dépendance inutilisée via `cargo-udeps`/clippy, ce n'est **pas** bloquant (une dépendance déclarée non utilisée ne produit pas de warning `-D warnings` par défaut). Ne pas retirer `anyhow` : la convention du template l'attend et la tâche 12/évolutions futures l'utiliseront.

- [ ] **Step 8: Réécrire le binaire en shim + supprimer l'ancien `cli.rs`**

```bash
git rm crates/claudine/src/cli.rs
```

Remplacer tout le contenu de `crates/claudine/src/main.rs` par :

```rust
use std::process::ExitCode;

fn main() -> ExitCode {
    claudine_core::run()
}
```

- [ ] **Step 9: Nettoyer les dépendances du binaire**

Dans `crates/claudine/Cargo.toml`, `[dependencies]`, ne garder que :

```toml
[dependencies]
claudine-core = { path = "../claudine-core" }
```

*(Retirer `clap.workspace = true` et `serde_json.workspace = true` — le shim ne les utilise plus. Garder les `[dev-dependencies]` `assert_cmd`/`predicates`/`tempfile` pour `tests/`.)*

- [ ] **Step 10: fmt + clippy + tests + vérif CLI de bout en bout**

Run:
```
cargo fmt && cargo fmt --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace 2>&1 | tail -8
```
Expected: 0 warning, 162 tests verts (les tests de `parse_maps`/`resolve_home` ont migré dans `commands/`).

Run (fumée CLI) :
```
cargo run -q -p claudine -- --version && cargo run -q -p claudine -- homes 2>&1 | head -3
```
Expected: `claudine 0.0.2` ; la commande `homes` s'exécute sans panique.

- [ ] **Step 11: Commit**

```bash
git add -A
git commit -m "refactor(core): move CLI (clap + commands) into claudine-core; binary is a thin shim"
```

---

### Task 5: Fichiers de configuration standards (deny / tarpaulin / audit)

**Files:**
- Create: `deny.toml`
- Create: `tarpaulin.toml`
- Create: `.cargo/audit.toml`

**Interfaces:**
- Produces : politique cargo-deny + config coverage + config audit, consommées par `ci.yml` (T7).

- [ ] **Step 1: Créer `deny.toml` (adapté de josephine)**

Copier `/home/kdelfour/Workspace/Professionel/systm-D/josephine/deny.toml` vers `./deny.toml`, puis appliquer ces transformations exactes :
- Retirer les `ignore` RUSTSEC spécifiques Windows de josephine **s'ils ne concernent pas les dépendances de claudine** ; sinon les conserver. Vérifier avec `cargo deny check advisories` (step 4) et n'ajouter d'ignore que pour un avis réellement déclenché, avec justification en commentaire.
- Conserver telles quelles : la section `[licenses]` (allow-list SPDX + `confidence-threshold = 0.9`), `[bans]` (`wildcards = "deny"`, multiple-versions warn), `[sources]` (crates.io only).

- [ ] **Step 2: Créer `tarpaulin.toml` (adapté de josephine)**

Copier `/home/kdelfour/Workspace/Professionel/systm-D/josephine/tarpaulin.toml` vers `./tarpaulin.toml`, puis remplacer la liste d'exclusions par les modules IO/UI de claudine :

```toml
[default]
workspace = true
out = ["Xml", "Stdout"]
exclude-files = [
    "crates/claudine-core/src/tui/*",
    "crates/claudine-core/src/cli.rs",
    "crates/claudine-core/src/commands/*",
    "crates/claudine/src/main.rs",
    "*/tests/*",
]
# Cible documentaire : 80 % (non bloquant en CI).
```

- [ ] **Step 3: Créer `.cargo/audit.toml` (miroir des ignores de `deny.toml`)**

Copier `/home/kdelfour/Workspace/Professionel/systm-D/josephine/.cargo/audit.toml` vers `./.cargo/audit.toml`, puis aligner la liste `[advisories] ignore` sur celle réellement retenue dans `deny.toml` (step 1). Si aucun avis n'est ignoré, laisser la liste vide :

```toml
[advisories]
ignore = []
```

- [ ] **Step 4: Vérifier localement (si les outils sont installés)**

Run:
```
command -v cargo-deny >/dev/null && cargo deny check 2>&1 | tail -15 || echo "cargo-deny absent (vérifié en CI)"
command -v cargo-audit >/dev/null && cargo audit 2>&1 | tail -10 || echo "cargo-audit absent (vérifié en CI)"
```
Expected: `deny check` sans erreur de licence/source (ajuster l'allow-list SPDX si une dépendance de claudine est refusée), ou message « absent » si l'outil n'est pas là (la CI le fera).

- [ ] **Step 5: Commit**

```bash
git add deny.toml tarpaulin.toml .cargo/audit.toml
git commit -m "chore(ci): add cargo-deny, tarpaulin and audit configuration"
```

---

### Task 6: Documents de standards (CONVENTIONS / CLAUDE / AGENTS)

**Files:**
- Create: `CONVENTIONS.md`
- Create: `CLAUDE.md`
- Create: `AGENTS.md`

**Interfaces:**
- Produces : source de vérité écrite des conventions + guide agent, référencés par `CONTRIBUTING.md` et `AGENTS.md`.

- [ ] **Step 1: Créer `CONVENTIONS.md` (adapté de josephine)**

Copier `/home/kdelfour/Workspace/Professionel/systm-D/josephine/CONVENTIONS.md` vers `./CONVENTIONS.md`, puis appliquer ces transformations exactes :
- Remplacer toute occurrence `josephine`/`Joséphine` → `claudine`/`Claudine`.
- Remplacer le paragraphe « Linux-only » par : **claudine est multi-plateforme** (Linux, Windows, macOS) car il gère `~/.claude`, présent partout.
- Remplacer la forme workspace décrite par celle de claudine : `claudine-core` (bibliothèque : logique + `cli.rs` + `commands/*` + `tui/*`) et `claudine` (binaire shim).
- Retirer les sections spécifiques à josephine sans objet pour claudine : SQLite/migrations, daemon/systemd, i18n `t(en, fr)`. Conserver : edition 2024 / MSRV 1.85, fmt + lints, Conventional Commits + Keep a Changelog + SemVer, dual-license, politique langue (docs EN / strings FR), porte qualité pré-PR.
- Bloc « porte qualité » = les 5 commandes de la section *Global Constraints* de ce plan.

- [ ] **Step 2: Créer `CLAUDE.md` (adapté de josephine)**

Copier `/home/kdelfour/Workspace/Professionel/systm-D/josephine/CLAUDE.md` vers `./CLAUDE.md`, puis :
- Remplacer `josephine`→`claudine` partout.
- « Read first » : pointer vers `CONVENTIONS.md`, `docs/superpowers/specs/`, `README.md`.
- Remplacer la table « where to change what » par les modules de claudine : `crates/claudine-core/src/{home,settings,config,extensions,marketplaces,export,import,housekeeping,search}.rs` (logique), `crates/claudine-core/src/cli.rs` + `commands/*` (CLI), `crates/claudine-core/src/tui/*` (TUI).
- Retirer les règles produit josephine sans objet (i18n obligatoire, 100% local notifications, Linux-only). Conserver la porte qualité.

- [ ] **Step 3: Créer `AGENTS.md`**

```markdown
# Agents

Ce dépôt suit des conventions partagées. Avant toute contribution automatisée,
lis dans l'ordre :

1. [`CLAUDE.md`](CLAUDE.md) — guide de développement (où changer quoi, porte qualité).
2. [`CONVENTIONS.md`](CONVENTIONS.md) — source de vérité des standards du projet.
3. [`CONTRIBUTING.md`](CONTRIBUTING.md) — processus de contribution.
```

- [ ] **Step 4: Vérifier les liens et l'absence de résidu « josephine »**

Run: `grep -rin "josephine" CONVENTIONS.md CLAUDE.md AGENTS.md || echo "OK aucun résidu josephine"`
Expected: `OK aucun résidu josephine`.

- [ ] **Step 5: Commit**

```bash
git add CONVENTIONS.md CLAUDE.md AGENTS.md
git commit -m "docs: add CONVENTIONS, CLAUDE and AGENTS guides (aligned on shared standard)"
```

---

### Task 7: Refonte de `ci.yml`

**Files:**
- Modify: `.github/workflows/ci.yml`

**Interfaces:**
- Consumes : `deny.toml`/`tarpaulin.toml`/`.cargo/audit.toml` (T5), `rustfmt.toml` (T1).
- Produces : pipeline CI (lint/test/coverage/security/bench-smoke) conforme au standard.

- [ ] **Step 1: Réécrire `ci.yml` à partir de la référence josephine**

Copier `/home/kdelfour/Workspace/Professionel/systm-D/josephine/.github/workflows/ci.yml` vers `.github/workflows/ci.yml`, puis appliquer ces transformations exactes :
- Remplacer `josephine`→`claudine` (noms de crates dans les commandes éventuelles).
- **Conserver** la matrice de test multi-OS de claudine : ajouter `windows-latest` et `macos-latest` aux runners de josephine (ubuntu 22.04/24.04 + fedora 40/41). claudine est multi-plateforme.
- Retirer l'étape **bench-smoke** si aucun bench n'existe (claudine n'a pas de `benches/`), OU la garder en no-op documentée : `cargo bench --no-run --locked` échoue s'il n'y a pas de bench cible ⇒ **la retirer** pour claudine (les benches sont hors périmètre, cf. spec §12).
- Jobs finaux attendus : **lint** (`cargo fmt --check` + `cargo clippy --workspace --all-targets -- -D warnings`), **test** (matrice ubuntu/fedora/windows/macos, `cargo test --workspace --locked`), **coverage** (tarpaulin → Codecov, `continue-on-error: true`), **security** (`cargo audit` + `cargo deny check`).

- [ ] **Step 2: Valider la syntaxe YAML**

Run:
```
python3 -c "import yaml,sys; yaml.safe_load(open('.github/workflows/ci.yml')); print('YAML OK')"
command -v actionlint >/dev/null && actionlint .github/workflows/ci.yml || echo "actionlint absent (optionnel)"
```
Expected: `YAML OK` ; si `actionlint` est présent, aucune erreur.

- [ ] **Step 3: Commit**

```bash
git add .github/workflows/ci.yml
git commit -m "ci: overhaul ci.yml (fmt, clippy, test matrix, coverage, security)"
```

---

### Task 8: Compléter `release.yml` (Homebrew / AUR / crates.io)

**Files:**
- Modify: `.github/workflows/release.yml`

**Interfaces:**
- Consumes : `packaging/homebrew/claudine.rb` + `packaging/aur/PKGBUILD` (T9). *(T8 produit le workflow qui les rend ; l'ordre T8 avant T9 est acceptable car le workflow ne s'exécute qu'au tag — mais les fichiers packaging doivent exister avant un vrai tag. Le plan crée les deux ; la revue vérifie la cohérence des chemins.)*
- Produces : release multi-canal (Releases + deb + rpm + homebrew + aur) + publication crates.io opt-in.

- [ ] **Step 1: Ajouter le rendu Homebrew + AUR et le job crates.io**

Partir du `.github/workflows/release.yml` **existant** de claudine (multi-OS, deb, rpm — à conserver). En s'inspirant de `/home/kdelfour/Workspace/Professionel/systm-D/josephine/.github/workflows/release.yml`, ajouter :
- Une étape qui **rend** `packaging/homebrew/claudine.rb` (remplacer url du tarball source + `sha256`) et l'attache à la release.
- Une étape qui **rend** `packaging/aur/PKGBUILD` (`pkgver` = tag sans `v`, `sha256sums`) et l'attache.
- Un job `crates-io` (`needs:` le job de build) **opt-in** : `if: vars.PUBLISH_CRATES == 'true'`, publie `claudine-core` puis `claudine` avec `CARGO_REGISTRY_TOKEN`.
- Remplacer `josephine`→`claudine` dans les chemins/globs d'artefacts.
- **Conserver** le trigger `on: push: tags: ["v*"]` et le build multi-OS existant.

- [ ] **Step 2: Valider la syntaxe YAML**

Run:
```
python3 -c "import yaml; yaml.safe_load(open('.github/workflows/release.yml')); print('YAML OK')"
command -v actionlint >/dev/null && actionlint .github/workflows/release.yml || echo "actionlint absent (optionnel)"
```
Expected: `YAML OK`.

- [ ] **Step 3: Commit**

```bash
git add .github/workflows/release.yml
git commit -m "ci(release): add Homebrew/AUR rendering and opt-in crates.io publish"
```

---

### Task 9: Packaging (AUR / Homebrew) + dependabot + CODEOWNERS

**Files:**
- Create: `packaging/aur/PKGBUILD` (depuis `packaging/arch/PKGBUILD`)
- Create: `packaging/homebrew/claudine.rb`
- Create: `.github/dependabot.yml`
- Create: `.github/CODEOWNERS`

**Interfaces:**
- Consumes : rien.
- Produces : recettes AUR/Homebrew consommées par `release.yml` (T8) ; automatisation deps + ownership.

- [ ] **Step 1: Créer `packaging/aur/PKGBUILD`**

```bash
git mv packaging/arch/PKGBUILD packaging/aur/PKGBUILD
```
Puis, en s'inspirant de `/home/kdelfour/Workspace/Professionel/systm-D/josephine/packaging/aur/PKGBUILD`, vérifier/aligner : `pkgname=claudine`, `url` = repo claudine, build `cargo build --frozen --release`, install du binaire + licences, `sha256sums=('0000…')` placeholder (rempli par `release.yml`, **jamais `SKIP`**). Retirer l'installation de l'unité systemd (claudine n'a pas de daemon).

- [ ] **Step 2: Créer `packaging/homebrew/claudine.rb`**

Copier `/home/kdelfour/Workspace/Professionel/systm-D/josephine/packaging/homebrew/josephine.rb` vers `packaging/homebrew/claudine.rb`, puis :
- `class Josephine < Formula` → `class Claudine < Formula`.
- `desc`, `homepage`, `url` (tarball source du repo claudine), `sha256 "0" * 64` placeholder.
- `depends_on "rust" => :build` (conserver) ; retirer `depends_on :linux` (claudine est multi-OS).
- `install` : `system "cargo", "install", *std_cargo_args` (build du binaire `claudine`).
- `test do system bin/"claudine", "--version" end`.

- [ ] **Step 3: Créer `.github/dependabot.yml`**

Copier `/home/kdelfour/Workspace/Professionel/systm-D/josephine/.github/dependabot.yml` vers `.github/dependabot.yml` (contenu générique : cargo + github-actions, hebdo). Aucun remplacement de nom requis.

- [ ] **Step 4: Créer `.github/CODEOWNERS`**

```
# Propriétaires par défaut de tout le dépôt.
*       @kdelfour
```

- [ ] **Step 5: Vérifier la syntaxe shell du PKGBUILD + YAML dependabot**

Run:
```
bash -n packaging/aur/PKGBUILD && echo "PKGBUILD sh OK"
python3 -c "import yaml; yaml.safe_load(open('.github/dependabot.yml')); print('dependabot YAML OK')"
```
Expected: `PKGBUILD sh OK` et `dependabot YAML OK`.

- [ ] **Step 6: Commit**

```bash
git add -A
git commit -m "chore(packaging): AUR + Homebrew recipes, dependabot, CODEOWNERS"
```

---

### Task 10: Site Zola (`site/`)

**Files:**
- Create: `site/config.toml`
- Create: `site/content/_index.md`
- Create: `site/templates/base.html`
- Create: `site/templates/index.html`
- Create: `site/sass/main.scss`
- Modify: `.gitignore` (ajout `site/public/`)

**Interfaces:**
- Produces : site statique buildable par `zola build` ; consommé par `pages.yml` (T11).

- [ ] **Step 1: Créer `site/config.toml`**

```toml
base_url = "https://systm-d.github.io/claudine"
title = "Claudine"
description = "Gère tes données Claude Code, sans quitter le terminal."
compile_sass = true
build_search_index = false
generate_feeds = false

[markdown]
highlight_code = true

[extra]
brand_color = "#d97757"
repo_url = "https://github.com/systm-d/claudine"
```

- [ ] **Step 2: Créer `site/templates/base.html`**

S'inspirer de `/home/kdelfour/Workspace/Professionel/systm-D/josephine/site/templates/base.html` pour la structure (skeleton `<head>`, `{% block content %}`, footer), en **retirant** le canvas starfield et le JS associé (identité propre à claudine). Contenu :

```html
<!DOCTYPE html>
<html lang="{{ lang }}">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>{% block title %}{{ config.title }}{% endblock %}</title>
  <meta name="description" content="{{ config.description }}">
  <meta name="theme-color" content="{{ config.extra.brand_color }}">
  <link rel="stylesheet" href="{{ get_url(path='main.css') }}">
</head>
<body>
  {% block content %}{% endblock %}
  <footer class="footer">
    <span>Claudine · local-first · open source ·
      <a href="{{ config.extra.repo_url }}">GitHub</a></span>
  </footer>
</body>
</html>
```

- [ ] **Step 3: Créer `site/templates/index.html`**

```html
{% extends "base.html" %}
{% block content %}
<header class="hero">
  <div class="topbar"><span class="wordmark">Claudine</span></div>
  <div class="glyph" aria-hidden="true">▐▛███▜▌<br>▝▜█████▛▘<br>&nbsp;&nbsp;▘▘ ▝▝</div>
  <h1>{{ section.extra.tagline }}</h1>
  <p class="lede">{{ section.extra.lede }}</p>
  <div class="cta">
    <a class="btn btn-primary" href="#install">{{ section.extra.cta2 }}</a>
    <a class="btn" href="{{ config.extra.repo_url }}">{{ section.extra.cta }}</a>
  </div>
</header>
<main>{{ section.content | safe }}</main>
{% endblock %}
```

- [ ] **Step 4: Créer `site/content/_index.md`**

```markdown
+++
[extra]
tagline = "Gère tes données Claude Code, sans quitter le terminal."
lede = "Sessions, mémoire, configuration, extensions et marketplaces — un TUI Rust qui lit et écrit ~/.claude en toute sûreté."
cta = "Voir sur GitHub"
cta2 = "Installer"
+++

<section class="features">
  <h2>Ce que fait Claudine</h2>
  <div class="grid">
    <div class="card"><h3>Sessions &amp; projets</h3><p>Parcours, recherche, déplace, restaure les sessions de toutes tes homes.</p></div>
    <div class="card"><h3>Mémoire</h3><p>Consulte la mémoire utilisateur (CLAUDE.md) directement dans le terminal.</p></div>
    <div class="card"><h3>Configuration</h3><p>Édite settings.json avec écriture atomique et sauvegarde horodatée.</p></div>
    <div class="card"><h3>Extensions</h3><p>Hooks, serveurs MCP et plugins : lecture, édition, bascule.</p></div>
    <div class="card"><h3>Marketplaces</h3><p>Ajoute des marketplaces et installe des plugins depuis le catalogue.</p></div>
    <div class="card"><h3>Import / Export</h3><p>Bundles .tar.gz signés, remap de chemins, dry-run, exclusion des secrets.</p></div>
  </div>
</section>

<section id="install" class="install">
  <h2>Installation</h2>
  <pre><code># Depuis les sources
cargo install --git https://github.com/systm-d/claudine claudine

# Debian / Ubuntu
sudo dpkg -i claudine_*_amd64.deb

# Fedora / RHEL
sudo rpm -i claudine-*.rpm

# Arch (AUR)
yay -S claudine

# Homebrew
brew install systm-d/tap/claudine</code></pre>
</section>
```

- [ ] **Step 5: Créer `site/sass/main.scss`**

S'inspirer de `/home/kdelfour/Workspace/Professionel/systm-D/josephine/site/sass/main.scss` pour les styles (hero, grille de cartes, install, footer, media queries reduced-motion/mobile), en câblant l'accent sur la couleur claudine :

```scss
:root {
  --brand: #d97757;
  --bg: #14110f;
  --panel: #1e1a17;
  --fg: #ece6e0;
  --muted: #a89e95;
  --mono: ui-monospace, "SFMono-Regular", "JetBrains Mono", Menlo, monospace;
}
* { box-sizing: border-box; }
body { margin: 0; background: var(--bg); color: var(--fg);
  font-family: system-ui, -apple-system, "Segoe UI", Roboto, sans-serif; line-height: 1.5; }
a { color: var(--brand); }
.hero { max-width: 62rem; margin: 0 auto; padding: 5rem 1.25rem 3rem; text-align: center; }
.wordmark { font-weight: 700; letter-spacing: .02em; }
.glyph { font-family: var(--mono); color: var(--brand); font-size: 1.5rem;
  line-height: 1.1; margin: 2rem 0; white-space: pre; }
.hero h1 { font-size: clamp(1.6rem, 4vw, 2.6rem); margin: .5rem 0; }
.lede { color: var(--muted); max-width: 40rem; margin: 0 auto 1.75rem; }
.cta { display: flex; gap: .75rem; justify-content: center; flex-wrap: wrap; }
.btn { display: inline-block; padding: .6rem 1.1rem; border-radius: .5rem;
  border: 1px solid var(--brand); color: var(--brand); text-decoration: none; }
.btn-primary { background: var(--brand); color: var(--bg); }
main { max-width: 62rem; margin: 0 auto; padding: 0 1.25rem; }
.features .grid { display: grid; gap: 1rem;
  grid-template-columns: repeat(auto-fit, minmax(15rem, 1fr)); }
.card { background: var(--panel); border-radius: .75rem; padding: 1.1rem 1.25rem; }
.card h3 { margin: 0 0 .4rem; color: var(--brand); }
.install pre { background: var(--panel); padding: 1.25rem; border-radius: .75rem;
  overflow-x: auto; font-family: var(--mono); }
.footer { max-width: 62rem; margin: 3rem auto 2rem; padding: 0 1.25rem;
  color: var(--muted); text-align: center; }
@media (prefers-reduced-motion: reduce) { * { animation: none !important; transition: none !important; } }
```

- [ ] **Step 6: Ignorer la sortie de build**

Ajouter la ligne `site/public/` à `.gitignore` (créer la ligne si absente).

- [ ] **Step 7: Vérifier que le site se construit**

Run: `cd site && zola build 2>&1 | tail -10 && cd ..`
Expected: `Done in …` sans erreur ; `site/public/index.html` généré.

- [ ] **Step 8: Commit**

```bash
git add site .gitignore
git commit -m "feat(site): Zola landing page (hero, features, install) with claudine brand"
```

---

### Task 11: Workflow GitHub Pages (`pages.yml`)

**Files:**
- Create: `.github/workflows/pages.yml`

**Interfaces:**
- Consumes : `site/` (T10).
- Produces : workflow build Zola + deploy-pages.

- [ ] **Step 1: Créer `.github/workflows/pages.yml`**

Copier `/home/kdelfour/Workspace/Professionel/systm-D/josephine/.github/workflows/pages.yml` vers `.github/workflows/pages.yml`, puis :
- Remplacer `josephine`→`claudine` dans les éventuels chemins.
- Conserver : trigger `on: push: branches: [main] paths: ["site/**"]` + `workflow_dispatch` ; job **build** (`configure-pages`, install Zola via `taiki-e/install-action`, `cd site && zola build`, upload `site/public`) ; job **deploy** (`actions/deploy-pages`, env `github-pages`, permissions `pages: write`, `id-token: write`).
- Aligner la version de Zola installée sur `0.21` si josephine en épingle une autre.

- [ ] **Step 2: Valider la syntaxe YAML**

Run: `python3 -c "import yaml; yaml.safe_load(open('.github/workflows/pages.yml')); print('YAML OK')"`
Expected: `YAML OK`.

- [ ] **Step 3: Commit**

```bash
git add .github/workflows/pages.yml
git commit -m "ci(pages): build Zola site and deploy to GitHub Pages"
```

---

### Task 12: Clôture — version 0.1.0, CHANGELOG, README, tests d'intégration

**Files:**
- Modify: `Cargo.toml` (version `0.1.0`)
- Modify: `CHANGELOG.md` (entrée `[0.1.0]`)
- Modify: `README.md` (badges CI/Pages, section install alignée, lien site)
- Create: `crates/claudine/tests/cli.rs`

**Interfaces:**
- Consumes : tout ce qui précède.
- Produces : release `0.1.0` prête (structure alignée, site, CI), tests d'intégration CLI.

- [ ] **Step 1: Écrire le test d'intégration CLI (TDD — d'abord le test)**

Créer `crates/claudine/tests/cli.rs` :

```rust
use assert_cmd::Command;
use predicates::str::contains;

#[test]
fn prints_version() {
    Command::cargo_bin("claudine")
        .unwrap()
        .arg("--version")
        .assert()
        .success()
        .stdout(contains("claudine 0.1.0"));
}

#[test]
fn homes_runs_without_subcommand() {
    Command::cargo_bin("claudine")
        .unwrap()
        .arg("homes")
        .assert()
        .success();
}

#[test]
fn unknown_flag_fails() {
    Command::cargo_bin("claudine")
        .unwrap()
        .arg("--nope")
        .assert()
        .failure();
}
```

- [ ] **Step 2: Lancer le test — il échoue sur la version (encore `0.0.2`)**

Run: `cargo test -p claudine --test cli 2>&1 | tail -12`
Expected: `prints_version` ÉCHOUE (`claudine 0.0.2` ≠ attendu `claudine 0.1.0`) ; les deux autres passent.

- [ ] **Step 3: Bumper la version en `0.1.0`**

Dans `Cargo.toml` racine, `[workspace.package]`, remplacer `version = "0.0.2"` par `version = "0.1.0"`.

- [ ] **Step 4: Relancer le test — tout passe**

Run: `cargo test -p claudine --test cli 2>&1 | tail -8`
Expected: `test result: ok. 3 passed`.

- [ ] **Step 5: Entrée CHANGELOG `[0.1.0]`**

Dans `CHANGELOG.md`, sous `## [Unreleased]`, insérer :

```markdown
## [0.1.0] - 2026-07-13

### Aligné sur le standard partagé (`rust-cli-template` + josephine)

- **Structure** : `claudine-core` porte désormais la CLI (`cli.rs` + `commands/*`) et
  la TUI (`tui/*`) en plus de la logique ; le binaire `claudine` est un shim.
- **Fondation** : edition 2024, MSRV 1.85, `cargo fmt` adopté (+ gate CI), `[workspace.lints]`
  (`unsafe_code = forbid`), profil release optimisé (LTO/strip), `rust-toolchain.toml`.
- **CI/CD** : `ci.yml` (fmt, clippy, matrice de test multi-OS, coverage, sécurité) ;
  `release.yml` complété (Homebrew, AUR, publication crates.io opt-in) ; `pages.yml`.
- **Packaging** : recettes AUR et Homebrew (en plus de deb/rpm/winget).
- **Site** : page d'accueil Zola (hero, fonctionnalités, installation) déployable sur
  GitHub Pages.
- **Standards** : `CONVENTIONS.md`, `CLAUDE.md`, `AGENTS.md`, `deny.toml`, `tarpaulin.toml`,
  `.cargo/audit.toml`, `dependabot.yml`, `CODEOWNERS`.
```

Puis, en bas du fichier, ajouter la référence de lien : `[0.1.0]: https://github.com/systm-d/claudine/releases/tag/v0.1.0` et pointer `[Unreleased]` sur `compare/v0.1.0...HEAD`.

- [ ] **Step 6: Actualiser `README.md`**

Ajouter, sous le titre : les badges CI + Pages (modèle josephine, chemins claudine), un lien vers le site (`https://systm-d.github.io/claudine`), et aligner la section « Installation » sur les canaux réels (cargo, deb, rpm, AUR, Homebrew, winget). Ne pas supprimer les sections existantes (usage, raccourcis, sous-commandes) — seulement compléter en-tête + install.

- [ ] **Step 7: Porte qualité complète**

Run:
```
cargo fmt --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace 2>&1 | tail -6 && cargo build --release 2>&1 | tail -2 && (cd site && zola build 2>&1 | tail -2)
```
Expected: fmt OK, 0 warning, tous les tests verts (165 : 162 + 3 intégration), build release OK, `zola build` OK.

- [ ] **Step 8: Vérifier la version rapportée**

Run: `cargo run -q -p claudine -- --version`
Expected: `claudine 0.1.0`.

- [ ] **Step 9: Commit**

```bash
git add -A
git commit -m "release: 0.1.0 — alignment on shared template standard"
```

---

## Self-Review (rempli par l'auteur du plan)

**Couverture de la spec :**
- §3 convention fondation → T1 (config), T4 (cli.rs/commands/run) ✅
- §4 structure cible → T1–T11 (chaque fichier neuf/modifié a sa tâche) ✅
- §5 portage code → T3 (TUI), T4 (CLI + shim) ✅
- §6 fondation/standards → T1 (edition/lints/profile), T2 (fmt), T5 (deny/tarpaulin/audit), T6 (docs) ✅
- §7 CI/release/packaging → T7 (ci), T8 (release), T9 (packaging/dependabot/CODEOWNERS) ✅
- §8 site web → T10 (site), T11 (pages) ✅
- §9 portes de validation → gate dans chaque tâche + T12 step 7 ✅
- §10 exécution / version 0.1.0 → T12 ✅
- §11 risques (fmt isolé, edition, unsafe, symlinks) → T2 (commit fmt isolé), T1 (cargo fix + check unsafe) ✅

**Scan placeholders :** aucun « TODO/TBD/à compléter ». Les tâches « copier josephine puis transformer » listent des transformations exactes (source sur disque + edits nommés) — ce ne sont pas des placeholders.

**Cohérence des types :** `claudine_core::run() -> ExitCode` (T4) consommé par le shim (T4 step 8) ; `crate::tui::run() -> io::Result<()>` (T3) consommé par `cli.rs` (T4) ; `Cli::run(self) -> Result<(), String>` (T4) consommé par `run()` (T4). `commands::{export::run_export, import::run_import, homes::{run_homes,run_homes_add,run_homes_remove}}` définis en T4 steps 3–5, appelés en T4 step 6. Cohérent.
