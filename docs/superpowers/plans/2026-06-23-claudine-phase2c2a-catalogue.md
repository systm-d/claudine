# Phase 2c-2a — Navigateur de catalogue de plugins + désinstallation — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Depuis le gestionnaire de marketplaces du TUI, parcourir le catalogue de plugins d'une marketplace (état installé/activé), désinstaller un plugin, et l'activer/désactiver — sans installation (réservée à 2c-2b).

**Architecture:** Le cœur réutilise le parsing plugins existant (`extensions.rs::read_plugins`) en l'exposant (`read_installed_plugins`) et ajoute `uninstall_plugin` (supprime le cache confiné + l'entrée `installed_plugins.json` + la clé `enabledPlugins`, via `SettingsDoc`). Le TUI ajoute un 2ᵉ niveau « catalogue » au gestionnaire de marketplaces (`tui/marketplaces.rs`), câblé dans `app.rs`/`mod.rs`/`ui.rs` ; toutes les opérations du catalogue sont **synchrones** (pas de réseau).

**Tech Stack:** Rust (workspace 2 crates), ratatui 0.28 (`crossterm` via `ratatui::crossterm`), serde_json (`preserve_order`), tests via `tempfile`.

## Global Constraints

- MSRV 1.74, édition 2021. **Aucune nouvelle dépendance.**
- `crates/claudine-core` ne dépend d'aucune lib d'UI.
- Écritures de `installed_plugins.json` et `settings.json` : **toujours via `SettingsDoc`** (backup `.bak-<nanos>` + temp+rename). Préserver les autres clés.
- Désinstallation : suppression de fichiers **confinée sous `plugins/cache/`** (refus sinon). Portée **utilisateur**. Home actif.
- **Réutiliser** le parsing plugins existant (`extensions.rs`) — pas de duplication. Enable/disable = `set_plugin_enabled` (2a). Liste = `read_marketplace_manifest` (2c-1).
- Style **formaté à la main** ; valider via `cargo clippy --workspace` (0 warning) + `cargo test --workspace`. **Ne jamais** lancer `cargo fmt`.

---

### Task 1: Cœur — `read_installed_plugins` (exposition) + `uninstall_plugin`

**Files:**
- Modify: `crates/claudine-core/src/extensions.rs`
- Modify: `crates/claudine-core/src/lib.rs`

**Interfaces:**
- Consumes: `read_plugins` (privé existant), `PluginEntry`, `SettingsDoc`, `CoreError::Marketplace`, `ClaudeHome` (a `plugins_dir()`, `settings_file()`, champ `base`).
- Produces:
  - `pub fn read_installed_plugins(home: &ClaudeHome) -> Vec<PluginEntry>`
  - `pub fn uninstall_plugin(home: &ClaudeHome, plugin: &str, marketplace: &str) -> Result<()>`

- [ ] **Step 1: Write the failing test**

Dans le module `tests` de `crates/claudine-core/src/extensions.rs` (le helper `home_with(&[(rel, content)])` y existe ; `ClaudeHome` a un champ public `base`) :

```rust
    #[test]
    fn read_installed_plugins_exposes_keys_and_enabled() {
        let installed = r#"{"version":2,"plugins":{
            "foo@m":[{"scope":"user","installPath":"/x","version":"1.0.0"}],
            "bar@m":[{"scope":"user","installPath":"/y","version":"2.0.0"}]
        }}"#;
        let settings = r#"{"enabledPlugins":{"foo@m":true,"bar@m":false}}"#;
        let (_d, home) = home_with(&[
            ("plugins/installed_plugins.json", installed),
            ("settings.json", settings),
        ]);
        let got = read_installed_plugins(&home);
        assert!(got.iter().find(|p| p.name == "foo@m").unwrap().enabled);
        assert!(!got.iter().find(|p| p.name == "bar@m").unwrap().enabled);
    }

    #[test]
    fn uninstall_plugin_removes_cache_entry_and_enabled() {
        let (_d, home) = home_with(&[("settings.json", r#"{"enabledPlugins":{"foo@m":true,"keep@m":true}}"#)]);
        let base = home.base.clone();
        // Dossiers de cache réels sous plugins/cache/.
        let foo_cache = base.join("plugins/cache/m/foo/1.0.0");
        std::fs::create_dir_all(&foo_cache).unwrap();
        std::fs::create_dir_all(base.join("plugins/cache/m/keep/1.0.0")).unwrap();
        let installed = format!(
            r#"{{"version":2,"plugins":{{
                "foo@m":[{{"scope":"user","installPath":"{foo}","version":"1.0.0"}}],
                "keep@m":[{{"scope":"user","installPath":"{base}/plugins/cache/m/keep/1.0.0","version":"1.0.0"}}]
            }}}}"#,
            foo = foo_cache.display(),
            base = base.display(),
        );
        std::fs::write(base.join("plugins/installed_plugins.json"), installed).unwrap();

        uninstall_plugin(&home, "foo", "m").unwrap();

        assert!(!foo_cache.exists(), "dossier de cache supprimé");
        let back = read_installed_plugins(&home);
        assert!(back.iter().all(|p| p.name != "foo@m"), "entrée retirée");
        assert!(back.iter().any(|p| p.name == "keep@m"), "autre entrée préservée");
        let sdoc = SettingsDoc::load(&home.settings_file()).unwrap();
        assert!(sdoc.get(&["enabledPlugins", "foo@m"]).is_none(), "clé enabled retirée");
        assert_eq!(sdoc.get_bool(&["enabledPlugins", "keep@m"]), Some(true), "autre clé préservée");
    }

    #[test]
    fn uninstall_plugin_rejects_path_outside_cache() {
        let (_d, home) = home_with(&[]);
        let base = home.base.clone();
        let outside = base.join("evil");
        std::fs::create_dir_all(&outside).unwrap();
        let installed = format!(
            r#"{{"version":2,"plugins":{{"foo@m":[{{"scope":"user","installPath":"{}","version":"1"}}]}}}}"#,
            outside.display()
        );
        std::fs::create_dir_all(base.join("plugins")).unwrap();
        std::fs::write(base.join("plugins/installed_plugins.json"), installed).unwrap();

        assert!(uninstall_plugin(&home, "foo", "m").is_err());
        assert!(outside.exists(), "dossier hors cache non supprimé");
    }

    #[test]
    fn uninstall_plugin_unknown_key_errors() {
        let (_d, home) = home_with(&[("plugins/installed_plugins.json", r#"{"version":2,"plugins":{}}"#)]);
        assert!(uninstall_plugin(&home, "nope", "m").is_err());
    }
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p claudine-core read_installed_plugins_exposes uninstall_plugin`
Expected: FAIL — fonctions inconnues.

- [ ] **Step 3: Write minimal implementation**

Dans `crates/claudine-core/src/extensions.rs`, ajouter près de `read_plugins` / `set_plugin_enabled` (les imports `fs`, `PathBuf`, `Value`, `SettingsDoc`, `CoreError`, `Result` sont déjà présents) :

```rust
/// Liste publique des plugins installés (réutilise le parsing de `read_plugins`).
/// Chaque `PluginEntry.name` est la clé `"<plugin>@<marketplace>"`.
pub fn read_installed_plugins(home: &ClaudeHome) -> Vec<PluginEntry> {
    read_plugins(home)
}

/// Désinstalle un plugin (portée user) : supprime son dossier de cache (confiné
/// sous `plugins/cache/`), retire son entrée d'`installed_plugins.json` et sa
/// clé d'`enabledPlugins`. Backup + écriture atomique via `SettingsDoc`.
pub fn uninstall_plugin(home: &ClaudeHome, plugin: &str, marketplace: &str) -> Result<()> {
    let key = format!("{plugin}@{marketplace}");
    let installed_path = home.plugins_dir().join("installed_plugins.json");
    let mut doc = SettingsDoc::load(&installed_path)?;

    let Some(entries) = doc
        .get(&["plugins", key.as_str()])
        .and_then(|v| v.as_array())
        .cloned()
    else {
        return Err(CoreError::Marketplace(format!("plugin non installé : {key}")));
    };

    // Entrée de portée `user` (à défaut, la première).
    let user_entry = entries
        .iter()
        .find(|e| e.get("scope").and_then(|s| s.as_str()) == Some("user"))
        .or_else(|| entries.first());
    let Some(user_entry) = user_entry else {
        return Err(CoreError::Marketplace(format!("plugin non installé : {key}")));
    };

    // Supprime le dossier de cache, confiné strictement sous plugins/cache/.
    if let Some(install_path) = user_entry.get("installPath").and_then(|p| p.as_str()) {
        let cache_root = home.plugins_dir().join("cache");
        let path = PathBuf::from(install_path);
        if !path.starts_with(&cache_root) || path == cache_root {
            return Err(CoreError::Marketplace(format!(
                "chemin d'installation hors cache : {install_path}"
            )));
        }
        if path.exists() {
            fs::remove_dir_all(&path).map_err(|e| CoreError::io(&path, e))?;
        }
    }

    // Retire l'entrée user du tableau (clé entière si plus rien).
    let remaining: Vec<Value> = entries
        .into_iter()
        .filter(|e| e.get("scope").and_then(|s| s.as_str()) != Some("user"))
        .collect();
    if remaining.is_empty() {
        doc.unset(&["plugins", key.as_str()]);
    } else {
        doc.set(&["plugins", key.as_str()], Value::Array(remaining));
    }
    doc.save(&installed_path)?;

    // Retire la clé d'enabledPlugins (settings.json), si présente.
    let settings_path = home.settings_file();
    let mut sdoc = SettingsDoc::load(&settings_path)?;
    if sdoc.get(&["enabledPlugins", key.as_str()]).is_some() {
        sdoc.unset(&["enabledPlugins", key.as_str()]);
        sdoc.save(&settings_path)?;
    }
    Ok(())
}
```

Re-exporter dans `crates/claudine-core/src/lib.rs` (ajouter à la liste `pub use extensions::{...}`) : `read_installed_plugins, uninstall_plugin`.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p claudine-core read_installed_plugins_exposes uninstall_plugin && cargo clippy -p claudine-core`
Expected: tests PASS, 0 warning.

- [ ] **Step 5: Commit**

```bash
git add crates/claudine-core/src/extensions.rs crates/claudine-core/src/lib.rs
git commit -m "feat(core): read_installed_plugins (exposé) + uninstall_plugin (cache confiné)"
```

---

### Task 2: TUI — état `PluginCatalog` (2ᵉ niveau du gestionnaire)

**Files:**
- Modify: `crates/claudine/src/tui/marketplaces.rs`

**Interfaces:**
- Consumes: `claudine_core::{PluginEntry, PluginManifestEntry}`.
- Produces (utilisés Tasks 3-4) :
  - `pub struct CatalogEntry { pub name: String, pub description: Option<String>, pub installed: bool, pub enabled: bool }`
  - `pub struct PluginCatalog { pub marketplace: String, pub entries: Vec<CatalogEntry>, pub idx: usize, pub confirm_uninstall: bool }`
  - `PluginCatalog::new(marketplace: String, manifest: &[PluginManifestEntry], installed: &[PluginEntry]) -> Self`, `move_sel(i32)`, `selected() -> Option<&CatalogEntry>`, `selected_name() -> Option<String>`, `begin_uninstall()`.
  - Champ `pub catalog: Option<PluginCatalog>` sur `MarketplacesManager` (init `None`).

- [ ] **Step 1: Write the failing test**

Ajouter dans le module `tests` de `crates/claudine/src/tui/marketplaces.rs` :

```rust
    use claudine_core::{PluginEntry, PluginManifestEntry};

    fn pm(name: &str, desc: Option<&str>) -> PluginManifestEntry {
        PluginManifestEntry { name: name.into(), description: desc.map(|s| s.to_string()) }
    }

    #[test]
    fn catalog_new_marks_installed_and_enabled() {
        let manifest = vec![pm("a", Some("da")), pm("b", None), pm("c", None)];
        let installed = vec![
            PluginEntry { name: "a@m".into(), enabled: true, ..Default::default() },
            PluginEntry { name: "b@m".into(), enabled: false, ..Default::default() },
        ];
        let cat = PluginCatalog::new("m".into(), &manifest, &installed);
        assert_eq!(cat.entries.len(), 3);
        assert!(cat.entries[0].installed && cat.entries[0].enabled); // a
        assert!(cat.entries[1].installed && !cat.entries[1].enabled); // b
        assert!(!cat.entries[2].installed && !cat.entries[2].enabled); // c
    }

    #[test]
    fn catalog_nav_and_uninstall_guard() {
        let manifest = vec![pm("a", None), pm("b", None)];
        let installed = vec![PluginEntry { name: "b@m".into(), enabled: false, ..Default::default() }];
        let mut cat = PluginCatalog::new("m".into(), &manifest, &installed);
        // a (idx 0) non installé → begin_uninstall ne fait rien.
        cat.begin_uninstall();
        assert!(!cat.confirm_uninstall);
        cat.move_sel(1); // b installé
        cat.begin_uninstall();
        assert!(cat.confirm_uninstall);
        assert_eq!(cat.selected_name().as_deref(), Some("b"));
    }

    #[test]
    fn manager_starts_without_catalog() {
        let m = MarketplacesManager::new(vec![]);
        assert!(m.catalog.is_none());
    }
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p claudine catalog_new catalog_nav manager_starts_without_catalog`
Expected: FAIL — types/champ inconnus.

- [ ] **Step 3: Write minimal implementation**

Dans `crates/claudine/src/tui/marketplaces.rs` :

1. Élargir l'import : `use claudine_core::{Marketplace, PluginEntry, PluginManifestEntry};`.
2. Ajouter le champ `catalog` à `MarketplacesManager` (après `confirm_remove`) :

```rust
    pub confirm_remove: bool,
    pub catalog: Option<PluginCatalog>,
```

et l'initialiser dans `new` (après `confirm_remove: false,`) :

```rust
            confirm_remove: false,
            catalog: None,
```

3. Ajouter les types et leur logique (après le bloc `impl MarketplacesManager`) :

```rust
/// Une ligne du catalogue d'une marketplace.
#[derive(Debug)]
pub struct CatalogEntry {
    pub name: String,
    pub description: Option<String>,
    pub installed: bool,
    pub enabled: bool,
}

/// Niveau « catalogue » : les plugins d'une marketplace avec leur état.
#[derive(Debug)]
pub struct PluginCatalog {
    pub marketplace: String,
    pub entries: Vec<CatalogEntry>,
    pub idx: usize,
    pub confirm_uninstall: bool,
}

impl PluginCatalog {
    /// Construit le catalogue : pour chaque plugin du manifeste, calcule
    /// installé/activé d'après la liste des plugins installés (clé `<nom>@<mkt>`).
    pub fn new(
        marketplace: String,
        manifest: &[PluginManifestEntry],
        installed: &[PluginEntry],
    ) -> Self {
        let entries = manifest
            .iter()
            .map(|p| {
                let key = format!("{}@{}", p.name, marketplace);
                let found = installed.iter().find(|ip| ip.name == key);
                CatalogEntry {
                    name: p.name.clone(),
                    description: p.description.clone(),
                    installed: found.is_some(),
                    enabled: found.map(|ip| ip.enabled).unwrap_or(false),
                }
            })
            .collect();
        Self {
            marketplace,
            entries,
            idx: 0,
            confirm_uninstall: false,
        }
    }

    /// Déplacement borné dans [0, len) (pas de bouclage).
    pub fn move_sel(&mut self, delta: i32) {
        if self.entries.is_empty() {
            return;
        }
        let max = self.entries.len() - 1;
        self.idx = if delta < 0 {
            self.idx.saturating_sub((-delta) as usize)
        } else {
            (self.idx + delta as usize).min(max)
        };
    }

    pub fn selected(&self) -> Option<&CatalogEntry> {
        self.entries.get(self.idx)
    }

    pub fn selected_name(&self) -> Option<String> {
        self.selected().map(|e| e.name.clone())
    }

    /// Demande la désinstallation (no-op si l'entrée sélectionnée n'est pas installée).
    pub fn begin_uninstall(&mut self) {
        if self.selected().map(|e| e.installed).unwrap_or(false) {
            self.confirm_uninstall = true;
        }
    }
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p claudine catalog_new catalog_nav manager_starts_without_catalog`
Expected: PASS (3 tests).

- [ ] **Step 5: Commit**

```bash
git add crates/claudine/src/tui/marketplaces.rs
git commit -m "feat(tui): état PluginCatalog (2e niveau du gestionnaire de marketplaces)"
```

---

### Task 3: TUI — câblage du catalogue (app + routage)

**Files:**
- Modify: `crates/claudine/src/tui/app.rs`
- Modify: `crates/claudine/src/tui/mod.rs`

**Interfaces:**
- Consumes: `PluginCatalog` (Task 2), `claudine_core::{read_marketplace_manifest, read_installed_plugins, uninstall_plugin, set_plugin_enabled}`.
- Produces: méthodes `open_catalog`, `catalog_close`, `catalog_toggle_enable`, `catalog_uninstall_confirmed` ; routage du niveau catalogue dans `handle_marketplaces_key` (+ `Enter` en liste ouvre le catalogue).

- [ ] **Step 1: Write the failing test**

Ajouter dans le module `tests` de `crates/claudine/src/tui/app.rs` (le module utilise déjà `fs`, `tempfile`, `ClaudeHome`, `Section`) :

```rust
    /// Home avec une marketplace « m » clonée (manifeste 2 plugins) + plugin « a » installé/activé.
    fn home_with_catalog() -> (tempfile::TempDir, std::path::PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let base = dir.path().to_path_buf();
        fs::create_dir_all(base.join("projects")).unwrap();
        let mdir = base.join("plugins/marketplaces/m/.claude-plugin");
        fs::create_dir_all(&mdir).unwrap();
        fs::write(
            mdir.join("marketplace.json"),
            r#"{"name":"m","plugins":[{"name":"a","description":"da"},{"name":"b"}]}"#,
        )
        .unwrap();
        let reg = format!(
            r#"{{"m":{{"source":{{"source":"github","repo":"o/r"}},"installLocation":"{}/plugins/marketplaces/m","lastUpdated":"x"}}}}"#,
            base.display()
        );
        fs::write(base.join("plugins/known_marketplaces.json"), reg).unwrap();
        let cache = base.join("plugins/cache/m/a/1.0.0");
        fs::create_dir_all(&cache).unwrap();
        let installed = format!(
            r#"{{"version":2,"plugins":{{"a@m":[{{"scope":"user","installPath":"{}","version":"1.0.0"}}]}}}}"#,
            cache.display()
        );
        fs::write(base.join("plugins/installed_plugins.json"), installed).unwrap();
        fs::write(base.join("settings.json"), r#"{"enabledPlugins":{"a@m":true}}"#).unwrap();
        (dir, base)
    }

    #[test]
    fn catalog_open_toggle_and_uninstall() {
        let (_d, base) = home_with_catalog();
        let cache = base.join("plugins/cache/m/a/1.0.0");
        let mut app = App::with_homes(vec![ClaudeHome::from_base(&base)]);
        app.set_section(Section::Extensions);
        app.open_marketplaces();
        app.open_catalog();

        {
            let c = app.marketplaces.as_ref().unwrap().catalog.as_ref().unwrap();
            assert_eq!(c.entries.len(), 2);
            assert!(c.entries[0].installed && c.entries[0].enabled); // a
            assert!(!c.entries[1].installed); // b
        }

        // Espace sur « a » (idx 0) → désactivé.
        app.catalog_toggle_enable();
        assert!(!app.marketplaces.as_ref().unwrap().catalog.as_ref().unwrap().entries[0].enabled);

        // Désinstallation de « a ».
        app.marketplaces.as_mut().unwrap().catalog.as_mut().unwrap().begin_uninstall();
        app.catalog_uninstall_confirmed();
        let c = app.marketplaces.as_ref().unwrap().catalog.as_ref().unwrap();
        assert!(!c.entries[0].installed);
        assert!(!cache.exists(), "dossier de cache supprimé");
    }

    #[test]
    fn catalog_close_returns_to_list() {
        let (_d, base) = home_with_catalog();
        let mut app = App::with_homes(vec![ClaudeHome::from_base(&base)]);
        app.set_section(Section::Extensions);
        app.open_marketplaces();
        app.open_catalog();
        assert!(app.marketplaces.as_ref().unwrap().catalog.is_some());
        app.catalog_close();
        assert!(app.marketplaces.as_ref().unwrap().catalog.is_none());
    }
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p claudine catalog_open_toggle catalog_close_returns`
Expected: FAIL — méthodes inconnues.

- [ ] **Step 3: Write minimal implementation**

Dans `crates/claudine/src/tui/app.rs` :

1. Imports — ajouter à `use claudine_core::{...}` : `read_installed_plugins, read_marketplace_manifest, uninstall_plugin` (`set_plugin_enabled` y est déjà). Et après `use crate::tui::marketplaces::MarketplacesManager;` : `use crate::tui::marketplaces::PluginCatalog;`.

2. Méthodes (dans `impl App`, près de `tick_mkt_job`) :

```rust
    /// Ouvre le catalogue de la marketplace sélectionnée (Enter en liste).
    pub fn open_catalog(&mut self) {
        let Some(name) = self.marketplaces.as_ref().and_then(|m| m.selected_name()) else {
            return;
        };
        let home = self.home().clone();
        match read_marketplace_manifest(&home, &name) {
            Ok(manifest) => {
                let installed = read_installed_plugins(&home);
                let catalog = PluginCatalog::new(name, &manifest.plugins, &installed);
                if let Some(m) = self.marketplaces.as_mut() {
                    m.catalog = Some(catalog);
                }
            }
            Err(e) => self.status = Some(format!("Catalogue indisponible : {e}")),
        }
    }

    pub fn catalog_close(&mut self) {
        if let Some(m) = self.marketplaces.as_mut() {
            m.catalog = None;
        }
    }

    /// Active/désactive le plugin sélectionné (si installé).
    pub fn catalog_toggle_enable(&mut self) {
        let info = self
            .marketplaces
            .as_ref()
            .and_then(|m| m.catalog.as_ref())
            .and_then(|c| {
                c.selected()
                    .filter(|e| e.installed)
                    .map(|e| (c.marketplace.clone(), e.name.clone(), e.enabled))
            });
        let Some((mkt, plugin, enabled)) = info else {
            return;
        };
        let key = format!("{plugin}@{mkt}");
        let home = self.home().clone();
        match set_plugin_enabled(&home, &key, !enabled) {
            Ok(()) => {
                if let Some(c) = self.marketplaces.as_mut().and_then(|m| m.catalog.as_mut()) {
                    if let Some(e) = c.entries.iter_mut().find(|e| e.name == plugin) {
                        e.enabled = !enabled;
                    }
                }
                let verb = if enabled { "désactivé" } else { "activé" };
                self.status = Some(format!("Plugin « {plugin} » {verb}"));
            }
            Err(e) => self.status = Some(format!("Échec : {e}")),
        }
    }

    /// Désinstalle le plugin sélectionné (après confirmation).
    pub fn catalog_uninstall_confirmed(&mut self) {
        let info = {
            let Some(c) = self.marketplaces.as_mut().and_then(|m| m.catalog.as_mut()) else {
                return;
            };
            c.confirm_uninstall = false;
            c.selected()
                .filter(|e| e.installed)
                .map(|e| (c.marketplace.clone(), e.name.clone()))
        };
        let Some((mkt, plugin)) = info else {
            return;
        };
        let home = self.home().clone();
        match uninstall_plugin(&home, &plugin, &mkt) {
            Ok(()) => {
                if let Some(c) = self.marketplaces.as_mut().and_then(|m| m.catalog.as_mut()) {
                    if let Some(e) = c.entries.iter_mut().find(|e| e.name == plugin) {
                        e.installed = false;
                        e.enabled = false;
                    }
                }
                self.status = Some(format!("Plugin « {plugin} » désinstallé"));
            }
            Err(e) => self.status = Some(format!("Échec désinstallation : {e}")),
        }
    }
```

Dans `crates/claudine/src/tui/mod.rs`, remplacer **entièrement** la fonction `handle_marketplaces_key` par cette version (ajoute le niveau catalogue + `Enter` ouvre le catalogue en liste) :

```rust
fn handle_marketplaces_key(app: &mut App, key: KeyEvent) {
    use crate::tui::marketplaces::MktMode;
    enum Deferred {
        Add(String),
        Update,
        Remove,
        Cancel,
        OpenCatalog,
        CatalogClose,
        ToggleEnable,
        Uninstall,
    }
    // `busy` lu avant d'emprunter `app.marketplaces` (évite le conflit d'emprunt).
    let busy = app.mkt_job.is_some();
    let deferred: Option<Deferred>;
    {
        let Some(m) = app.marketplaces.as_mut() else {
            return;
        };
        if let Some(c) = m.catalog.as_mut() {
            // Niveau catalogue.
            if c.confirm_uninstall {
                deferred = match key.code {
                    KeyCode::Char('o') | KeyCode::Char('O') | KeyCode::Char('y')
                    | KeyCode::Char('Y') | KeyCode::Enter => Some(Deferred::Uninstall),
                    KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                        c.confirm_uninstall = false;
                        None
                    }
                    _ => None,
                };
            } else {
                deferred = match key.code {
                    KeyCode::Up | KeyCode::Char('k') => {
                        c.move_sel(-1);
                        None
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        c.move_sel(1);
                        None
                    }
                    KeyCode::Char(' ') => Some(Deferred::ToggleEnable),
                    KeyCode::Char('d') => {
                        c.begin_uninstall();
                        None
                    }
                    KeyCode::Esc => Some(Deferred::CatalogClose),
                    _ => None,
                };
            }
        } else if m.confirm_remove {
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
                KeyCode::Enter => Some(Deferred::OpenCatalog),
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
        Some(Deferred::OpenCatalog) => app.open_catalog(),
        Some(Deferred::CatalogClose) => app.catalog_close(),
        Some(Deferred::ToggleEnable) => app.catalog_toggle_enable(),
        Some(Deferred::Uninstall) => app.catalog_uninstall_confirmed(),
        None => {}
    }
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p claudine catalog_open_toggle catalog_close_returns && cargo clippy -p claudine`
Expected: tests PASS, 0 warning.

- [ ] **Step 5: Commit**

```bash
git add crates/claudine/src/tui/app.rs crates/claudine/src/tui/mod.rs
git commit -m "feat(tui): câblage du catalogue (Enter ouvre, Espace active, d désinstalle)"
```

---

### Task 4: TUI — rendu du catalogue + aide + vérification finale

**Files:**
- Modify: `crates/claudine/src/tui/ui.rs`

**Interfaces:**
- Consumes: `PluginCatalog` (Task 2), `app.marketplaces.catalog`.
- Produces: rendu du niveau catalogue ; aide mise à jour. Pas de nouveau test unitaire (le rendu n'est pas testé unitairement ; couverture via la vérification workspace).

- [ ] **Step 1: Rendre le catalogue**

Dans `crates/claudine/src/tui/ui.rs` :

1. Import — ajouter après `use crate::tui::marketplaces::MktMode;` : `use crate::tui::marketplaces::PluginCatalog;`.

2. Au tout début de `render_marketplaces` (juste après le `let Some(m) = &app.marketplaces else { return; };`), dévier vers le catalogue s'il est ouvert :

```rust
    if let Some(c) = &m.catalog {
        render_plugin_catalog(c, f, area);
        return;
    }
```

3. Ajouter la fonction de rendu du catalogue (juste après `render_marketplaces`) :

```rust
/// Modal du catalogue de plugins d'une marketplace (2ᵉ niveau).
fn render_plugin_catalog(c: &PluginCatalog, f: &mut Frame, area: Rect) {
    let popup = centered_rect(78, 72, area);
    f.render_widget(Clear, popup);

    let hint = if c.confirm_uninstall {
        " o/n confirmer "
    } else {
        " Espace activer/désact. · d désinstaller · Esc retour "
    };
    let block = Block::default()
        .borders(Borders::ALL)
        .title(Span::styled(
            format!(" Plugins de « {} » ", c.marketplace),
            Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
        ))
        .title(Line::from(hint).right_aligned());
    let inner = block.inner(popup);
    f.render_widget(block, popup);

    let mut lines: Vec<Line> = Vec::new();
    if c.entries.is_empty() {
        lines.push(Line::from(Span::styled(
            "  (aucun plugin dans ce manifeste)",
            Style::default().fg(DIM),
        )));
    }
    for (i, e) in c.entries.iter().enumerate() {
        let sel = i == c.idx;
        let state = if e.installed {
            if e.enabled {
                "[installé][activé]"
            } else {
                "[installé]"
            }
        } else {
            "(non installé)"
        };
        let label = format!("{} {}  {}", if sel { "▶" } else { " " }, e.name, state);
        let style = if sel { selection_style(true) } else { Style::default() };
        lines.push(Line::from(Span::styled(label, style)));
        if sel {
            if let Some(d) = &e.description {
                lines.push(Line::from(Span::styled(
                    format!("     {d}"),
                    Style::default().fg(DIM),
                )));
            }
        }
    }

    if c.confirm_uninstall {
        lines.push(Line::from(""));
        let name = c.selected_name().unwrap_or_default();
        lines.push(Line::from(Span::styled(
            format!("  Désinstaller « {name} » ? (o/n)"),
            Style::default()
                .fg(Color::Black)
                .bg(Color::Red)
                .add_modifier(Modifier::BOLD),
        )));
    }

    f.render_widget(Paragraph::new(Text::from(lines)), inner);
}
```

- [ ] **Step 2: Mettre à jour l'aide**

Dans `render_help`, remplacer la ligne `("Extensions", …)` par (mentionne le catalogue) :

```rust
        ("Extensions", "hooks (Enter) · plugins (p) · MCP (m) · marketplaces (g → Enter: catalogue) ; E édite settings.json"),
```

- [ ] **Step 3: Vérification complète**

Run: `cargo clippy --workspace 2>&1 | grep -cE "warning:|error"` → attendu `0`
Run: `cargo test --workspace` → tous les paquets `ok`.

- [ ] **Step 4: Commit**

```bash
git add crates/claudine/src/tui/ui.rs
git commit -m "feat(tui): rendu du catalogue de plugins + aide (Enter sur une marketplace)"
```

---

## Self-Review

**1. Couverture de la spec :**
- §3 cœur (`read_installed_plugins` exposé, `uninstall_plugin` : cache confiné + `installed_plugins.json` + `enabledPlugins` via `SettingsDoc`, clé absente → erreur) → Task 1. ✓
- §4 modèle TUI (`CatalogEntry`, `PluginCatalog::new` calculant installed/enabled, `catalog` sur le manager) → Task 2. ✓
- §5 flux/raccourcis (Enter ouvre, Espace active si installé, d désinstalle avec confirmation, Esc retour ; `open_catalog`/`catalog_close`/`catalog_toggle_enable`/`catalog_uninstall_confirmed`) → Tasks 3-4. ✓
- §6 sûreté (cache confiné, SettingsDoc backup+atomique, confirmation, no-op sur non installé, home actif) → Task 1 (confinement/écritures) + Tasks 2-3 (gardes `begin_uninstall`/toggle si installé). ✓
- §7 tests cœur + TUI → présents Tasks 1-3 (le rendu Task 4 couvert par la vérif workspace). ✓

**2. Placeholders :** aucun TODO/TBD ; code complet à chaque étape.

**3. Cohérence des types :** `PluginCatalog`/`CatalogEntry` (champs `marketplace`/`entries`/`idx`/`confirm_uninstall` et `name`/`description`/`installed`/`enabled`) identiques Task 2 ↔ usages Tasks 3-4. `read_installed_plugins`/`uninstall_plugin`/`read_marketplace_manifest`/`set_plugin_enabled` cohérents cœur (Task 1) ↔ app (Task 3). Clé plugin `"<nom>@<marketplace>"` construite de façon identique dans `PluginCatalog::new`, `catalog_toggle_enable` et `uninstall_plugin`. Routage `handle_marketplaces_key` : variantes `OpenCatalog`/`CatalogClose`/`ToggleEnable`/`Uninstall` mappées vers les méthodes App correspondantes. ✓

## Suite
- **2c-2b** — installation des plugins (4 types de source `url`/`git-subdir`/`relative-path`/`github` → `cache/<mkt>/<plugin>/<version>/`, écriture `installed_plugins.json` + activation par défaut), via le helper git épinglé `@sha` (réutilise 2c-1).
