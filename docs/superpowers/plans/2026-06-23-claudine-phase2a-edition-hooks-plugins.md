# Phase 2a — Édition des hooks + bascule des plugins — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Rendre éditables, depuis la section Extensions du TUI, les hooks et l'activation des plugins du `settings.json` du home actif.

**Architecture:** Le cœur (`claudine-core/src/extensions.rs`) gagne un modèle d'édition des hooks et trois fonctions d'écriture qui s'appuient sur `SettingsDoc` (backup + écriture atomique + `preserve_order`). Le TUI ajoute un éditeur de hooks dédié (`hooks_editor.rs`) et un petit modal de bascule des plugins, câblés dans `app.rs` / `mod.rs` / `ui.rs` comme les autres modales (corbeille, import).

**Tech Stack:** Rust (workspace 2 crates), ratatui 0.28, serde_json (feature `preserve_order`), tests via `tempfile`.

## Global Constraints

- MSRV 1.74, édition 2021.
- `crates/claudine-core` ne dépend d'aucune lib d'UI.
- Écriture de fichiers : toujours via `SettingsDoc` (backup `.bak-<nanos>` + temp+rename). Jamais d'écriture brute.
- Style formaté à la main ; valider via `cargo clippy --workspace` (0 warning) + `cargo test --workspace`. **Ne pas** lancer `cargo fmt`.
- `crossterm` via `ratatui::crossterm` (pas de dépendance séparée).
- On n'édite que `settings.json` (jamais `settings.local.json`).

---

### Task 1: Cœur — modèle de hooks + `read_hook_groups`

**Files:**
- Modify: `crates/claudine-core/src/extensions.rs`
- Modify: `crates/claudine-core/src/lib.rs`

**Interfaces:**
- Consumes: `crate::home::ClaudeHome` (a `settings_file() -> PathBuf`), `crate::settings::SettingsDoc`.
- Produces:
  - `pub struct HookCommand { pub kind: String, pub command: String, pub timeout: Option<u64> }`
  - `pub struct HookGroup { pub event: String, pub matcher: Option<String>, pub commands: Vec<HookCommand> }`
  - `pub fn read_hook_groups(home: &ClaudeHome) -> Vec<HookGroup>`

- [ ] **Step 1: Write the failing test**

Ajouter dans le module `tests` de `crates/claudine-core/src/extensions.rs` (le helper `home_with` y existe déjà) :

```rust
    #[test]
    fn read_hook_groups_parses_event_matcher_commands() {
        let settings = r#"{
            "hooks": {
                "PreToolUse": [
                    {"matcher":"Bash","hooks":[{"type":"command","command":"echo a","timeout":30}]}
                ],
                "SessionStart": [
                    {"hooks":[{"type":"command","command":"echo b"},{"type":"command","command":"echo c"}]}
                ]
            }
        }"#;
        let (_d, home) = home_with(&[("settings.json", settings)]);
        let groups = read_hook_groups(&home);
        assert_eq!(groups.len(), 2);
        assert_eq!(groups[0].event, "PreToolUse");
        assert_eq!(groups[0].matcher.as_deref(), Some("Bash"));
        assert_eq!(groups[0].commands.len(), 1);
        assert_eq!(groups[0].commands[0].command, "echo a");
        assert_eq!(groups[0].commands[0].timeout, Some(30));
        assert_eq!(groups[1].event, "SessionStart");
        assert_eq!(groups[1].matcher, None);
        assert_eq!(groups[1].commands.len(), 2);
    }
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p claudine-core read_hook_groups_parses -- --nocapture`
Expected: FAIL — `cannot find function read_hook_groups` / `HookGroup` not found.

- [ ] **Step 3: Write minimal implementation**

Ajouter dans `crates/claudine-core/src/extensions.rs` (après les structs de lecture existantes) :

```rust
/// Une commande de hook (modèle d'édition, niveau « complet »).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct HookCommand {
    pub kind: String, // "command" par défaut
    pub command: String,
    pub timeout: Option<u64>,
}

/// Un groupe de hook : un évènement, un matcher optionnel, des commandes.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct HookGroup {
    pub event: String,
    pub matcher: Option<String>,
    pub commands: Vec<HookCommand>,
}

/// Lit les hooks de `settings.json` (uniquement) sous forme éditable, en
/// préservant l'ordre du fichier. Renvoie une liste vide si absent/illisible.
pub fn read_hook_groups(home: &ClaudeHome) -> Vec<HookGroup> {
    let Some(v) = load_json(&home.settings_file()) else {
        return Vec::new();
    };
    let Some(hooks) = v.get("hooks").and_then(|h| h.as_object()) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for (event, groups) in hooks {
        let Some(arr) = groups.as_array() else { continue };
        for group in arr {
            let matcher = group
                .get("matcher")
                .and_then(|m| m.as_str())
                .filter(|s| !s.is_empty())
                .map(|s| s.to_string());
            let commands = group
                .get("hooks")
                .and_then(|h| h.as_array())
                .map(|hs| {
                    hs.iter()
                        .map(|h| HookCommand {
                            kind: h
                                .get("type")
                                .and_then(|t| t.as_str())
                                .unwrap_or("command")
                                .to_string(),
                            command: h
                                .get("command")
                                .and_then(|c| c.as_str())
                                .unwrap_or("")
                                .to_string(),
                            timeout: h.get("timeout").and_then(|t| t.as_u64()),
                        })
                        .collect()
                })
                .unwrap_or_default();
            out.push(HookGroup {
                event: event.clone(),
                matcher,
                commands,
            });
        }
    }
    out
}
```

Puis re-exporter dans `crates/claudine-core/src/lib.rs`, en remplaçant la ligne d'export `extensions::` existante :

```rust
pub use extensions::{
    read_extensions, read_hook_groups, Extensions, HookCommand, HookGroup, HookEntry, McpEntry,
    PluginEntry,
};
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p claudine-core read_hook_groups_parses`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/claudine-core/src/extensions.rs crates/claudine-core/src/lib.rs
git commit -m "feat(core): modèle d'édition des hooks + read_hook_groups"
```

---

### Task 2: Cœur — `write_hooks`

**Files:**
- Modify: `crates/claudine-core/src/extensions.rs`
- Modify: `crates/claudine-core/src/lib.rs`

**Interfaces:**
- Consumes: `HookGroup`, `HookCommand`, `read_hook_groups` (Task 1), `SettingsDoc`.
- Produces: `pub fn write_hooks(home: &ClaudeHome, groups: &[HookGroup]) -> Result<()>`

> Note préservation : `write_hooks` reconstruit l'objet `hooks` à partir du modèle (`type`/`command`/`timeout`). Les **autres** réglages de `settings.json` sont préservés (on ne touche que la clé `hooks`). Des champs inconnus éventuels sur une commande individuelle ne sont pas conservés (cas rare, accepté ; backup en place).

- [ ] **Step 1: Write the failing test**

```rust
    #[test]
    fn write_hooks_round_trips_and_preserves_other_settings() {
        let settings = r#"{"includeCoAuthoredBy":false,"hooks":{"Stop":[{"hooks":[{"type":"command","command":"old"}]}]}}"#;
        let (_d, home) = home_with(&[("settings.json", settings)]);

        let groups = vec![
            HookGroup {
                event: "PreToolUse".into(),
                matcher: Some("Bash".into()),
                commands: vec![HookCommand {
                    kind: "command".into(),
                    command: "echo hi".into(),
                    timeout: Some(15),
                }],
            },
            HookGroup {
                event: "PreToolUse".into(),
                matcher: None,
                commands: vec![HookCommand {
                    kind: "command".into(),
                    command: "echo two".into(),
                    timeout: None,
                }],
            },
        ];
        write_hooks(&home, &groups).unwrap();

        // Relecture : deux groupes sous PreToolUse, dans l'ordre.
        let back = read_hook_groups(&home);
        assert_eq!(back.len(), 2);
        assert_eq!(back[0].event, "PreToolUse");
        assert_eq!(back[0].matcher.as_deref(), Some("Bash"));
        assert_eq!(back[0].commands[0].timeout, Some(15));
        assert_eq!(back[1].matcher, None);
        // Autre réglage préservé.
        let doc = crate::settings::SettingsDoc::load(&home.settings_file()).unwrap();
        assert_eq!(doc.get_bool(&["includeCoAuthoredBy"]), Some(false));
    }
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p claudine-core write_hooks_round_trips`
Expected: FAIL — `cannot find function write_hooks`.

- [ ] **Step 3: Write minimal implementation**

Ajouter en tête de `extensions.rs` l'import nécessaire (si absent) :

```rust
use serde_json::{Map, Value};
```

(le fichier importe déjà `serde_json::Value` ; ajouter `Map` à l'import existant, et `use crate::settings::SettingsDoc;` + `use crate::error::Result;`).

Ajouter la fonction :

```rust
/// Réécrit la clé `hooks` de `settings.json` à partir du modèle d'édition.
/// Les autres réglages sont préservés ; backup + écriture atomique via SettingsDoc.
pub fn write_hooks(home: &ClaudeHome, groups: &[HookGroup]) -> Result<()> {
    let path = home.settings_file();
    let mut doc = SettingsDoc::load(&path)?;

    if groups.is_empty() {
        doc.unset(&["hooks"]);
        return doc.save(&path);
    }

    let mut hooks: Map<String, Value> = Map::new();
    for g in groups {
        let mut grp = Map::new();
        if let Some(m) = &g.matcher {
            if !m.is_empty() {
                grp.insert("matcher".to_string(), Value::String(m.clone()));
            }
        }
        let cmds: Vec<Value> = g
            .commands
            .iter()
            .map(|c| {
                let mut cm = Map::new();
                let kind = if c.kind.is_empty() { "command" } else { &c.kind };
                cm.insert("type".to_string(), Value::String(kind.to_string()));
                cm.insert("command".to_string(), Value::String(c.command.clone()));
                if let Some(t) = c.timeout {
                    cm.insert("timeout".to_string(), Value::Number(t.into()));
                }
                Value::Object(cm)
            })
            .collect();
        grp.insert("hooks".to_string(), Value::Array(cmds));

        let entry = hooks
            .entry(g.event.clone())
            .or_insert_with(|| Value::Array(Vec::new()));
        if let Some(arr) = entry.as_array_mut() {
            arr.push(Value::Object(grp));
        }
    }
    doc.set(&["hooks"], Value::Object(hooks));
    doc.save(&path)
}
```

Re-exporter dans `lib.rs` (ajouter `write_hooks` à la liste d'export de `extensions::`).

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p claudine-core write_hooks_round_trips`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/claudine-core/src/extensions.rs crates/claudine-core/src/lib.rs
git commit -m "feat(core): write_hooks (réécrit hooks, préserve les autres réglages)"
```

---

### Task 3: Cœur — `set_plugin_enabled`

**Files:**
- Modify: `crates/claudine-core/src/extensions.rs`
- Modify: `crates/claudine-core/src/lib.rs`

**Interfaces:**
- Consumes: `SettingsDoc`.
- Produces: `pub fn set_plugin_enabled(home: &ClaudeHome, name: &str, enabled: bool) -> Result<()>`

- [ ] **Step 1: Write the failing test**

```rust
    #[test]
    fn set_plugin_enabled_writes_flag_and_preserves_others() {
        let settings = r#"{"includeCoAuthoredBy":true,"enabledPlugins":{"foo@m":true}}"#;
        let (_d, home) = home_with(&[("settings.json", settings)]);

        set_plugin_enabled(&home, "foo@m", false).unwrap();
        set_plugin_enabled(&home, "bar@m", true).unwrap();

        let ext = read_extensions(&home);
        let foo = ext.plugins.iter().find(|p| p.name == "foo@m").unwrap();
        assert!(!foo.enabled);
        let bar = ext.plugins.iter().find(|p| p.name == "bar@m").unwrap();
        assert!(bar.enabled);
        let doc = crate::settings::SettingsDoc::load(&home.settings_file()).unwrap();
        assert_eq!(doc.get_bool(&["includeCoAuthoredBy"]), Some(true));
    }
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p claudine-core set_plugin_enabled_writes`
Expected: FAIL — `cannot find function set_plugin_enabled`.

- [ ] **Step 3: Write minimal implementation**

```rust
/// Active / désactive un plugin dans `enabledPlugins` de `settings.json`.
pub fn set_plugin_enabled(home: &ClaudeHome, name: &str, enabled: bool) -> Result<()> {
    let path = home.settings_file();
    let mut doc = SettingsDoc::load(&path)?;
    doc.set(&["enabledPlugins", name], Value::Bool(enabled));
    doc.save(&path)
}
```

Re-exporter `set_plugin_enabled` dans `lib.rs`.

> Note : `read_plugins` ne liste un plugin que s'il est présent dans `installed_plugins.json` ou déjà dans `enabledPlugins`. Activer `bar@m` l'ajoute à `enabledPlugins`, donc il devient listé — cohérent avec le test.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p claudine-core set_plugin_enabled_writes`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/claudine-core/src/extensions.rs crates/claudine-core/src/lib.rs
git commit -m "feat(core): set_plugin_enabled (toggle enabledPlugins)"
```

---

### Task 4: TUI — `HooksEditor`, niveau « groupes »

**Files:**
- Create: `crates/claudine/src/tui/hooks_editor.rs`
- Modify: `crates/claudine/src/tui/mod.rs` (déclarer `pub mod hooks_editor;`)

**Interfaces:**
- Consumes: `claudine_core::{HookGroup, HookCommand}`.
- Produces (utilisés par les tâches 5-6) :
  - `pub enum HooksLevel { Groups, Group }`
  - `pub struct HooksEditor { pub groups, pub level, pub group_idx, pub field_idx, pub edit: HookEdit, pub confirm_delete: bool }`
  - `pub enum HookEdit { None, Text(String) }`
  - `HooksEditor::new(groups: Vec<HookGroup>) -> Self`
  - `fn move_sel(&mut self, delta: i32)`, `fn add_group(&mut self)`, `fn delete_current(&mut self)`, `fn enter(&mut self)`, `fn back(&mut self) -> bool`

- [ ] **Step 1: Write the failing test**

Créer `crates/claudine/src/tui/hooks_editor.rs` avec, en bas, un module de tests :

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use claudine_core::{HookCommand, HookGroup};

    fn sample() -> Vec<HookGroup> {
        vec![HookGroup {
            event: "PreToolUse".into(),
            matcher: Some("Bash".into()),
            commands: vec![HookCommand { kind: "command".into(), command: "echo a".into(), timeout: None }],
        }]
    }

    #[test]
    fn groups_level_add_and_delete() {
        let mut e = HooksEditor::new(sample());
        assert_eq!(e.level, HooksLevel::Groups);
        assert_eq!(e.groups.len(), 1);

        e.add_group();
        assert_eq!(e.groups.len(), 2);
        assert_eq!(e.group_idx, 1, "sélection sur le nouveau groupe");

        // Suppression : demande confirmation, puis applique.
        e.delete_current();
        assert!(e.confirm_delete);
        e.confirm_delete = false; // (l'app confirmera ; ici on simule l'annulation)
        assert_eq!(e.groups.len(), 2, "rien supprimé sans confirmation");
    }

    #[test]
    fn enter_and_back_navigate_levels() {
        let mut e = HooksEditor::new(sample());
        e.enter();
        assert_eq!(e.level, HooksLevel::Group);
        assert_eq!(e.field_idx, 0);
        assert!(e.back(), "back depuis Group renvoie true et remonte");
        assert_eq!(e.level, HooksLevel::Groups);
        assert!(!e.back(), "back depuis Groups renvoie false (fermer)");
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p claudine hooks_editor`
Expected: FAIL — module/type inconnus (ne compile pas).

- [ ] **Step 3: Write minimal implementation**

En haut de `crates/claudine/src/tui/hooks_editor.rs` :

```rust
//! Éditeur de hooks dédié (modal) : navigation hiérarchique
//! évènement → groupe → commandes, et édition des champs.

use claudine_core::{HookCommand, HookGroup};

/// Niveau de navigation courant.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HooksLevel {
    Groups,
    Group,
}

/// Édition en cours : texte d'un champ (évènement/matcher/commande) ou timeout
/// d'une commande. Le tampon est une chaîne dans les deux cas.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HookEdit {
    None,
    Text(String),
    Timeout(String),
}

/// Évènements Claude Code connus (liste curatée, affichée en aide ; la saisie
/// libre reste possible pour ne pas bloquer un nouvel évènement).
pub const KNOWN_EVENTS: &[&str] = &[
    "PreToolUse",
    "PostToolUse",
    "PostToolUseFailure",
    "UserPromptSubmit",
    "Notification",
    "Stop",
    "SubagentStart",
    "SubagentStop",
    "SessionStart",
    "SessionEnd",
    "PreCompact",
    "TaskCompleted",
    "WorktreeCreate",
    "WorktreeRemove",
];

pub struct HooksEditor {
    pub groups: Vec<HookGroup>,
    pub level: HooksLevel,
    /// Sélection au niveau Groups.
    pub group_idx: usize,
    /// Sélection de ligne au niveau Group (0=évènement, 1=matcher, 2+=commandes).
    pub field_idx: usize,
    pub edit: HookEdit,
    pub confirm_delete: bool,
}

impl HooksEditor {
    pub fn new(groups: Vec<HookGroup>) -> Self {
        Self {
            groups,
            level: HooksLevel::Groups,
            group_idx: 0,
            field_idx: 0,
            edit: HookEdit::None,
            confirm_delete: false,
        }
    }

    /// Nombre de lignes navigables au niveau Group : évènement + matcher + commandes.
    fn group_rows(&self) -> usize {
        self.groups
            .get(self.group_idx)
            .map(|g| 2 + g.commands.len())
            .unwrap_or(2)
    }

    pub fn move_sel(&mut self, delta: i32) {
        match self.level {
            HooksLevel::Groups => {
                let n = self.groups.len();
                if n == 0 {
                    return;
                }
                self.group_idx = step(self.group_idx, delta, n);
            }
            HooksLevel::Group => {
                let n = self.group_rows();
                self.field_idx = step(self.field_idx, delta, n);
            }
        }
    }

    pub fn add_group(&mut self) {
        self.groups.push(HookGroup {
            event: "PreToolUse".to_string(),
            matcher: None,
            commands: Vec::new(),
        });
        self.group_idx = self.groups.len() - 1;
    }

    /// Demande la suppression de l'élément courant (groupe au niveau Groups,
    /// commande au niveau Group si une commande est sélectionnée).
    pub fn delete_current(&mut self) {
        let deletable = match self.level {
            HooksLevel::Groups => !self.groups.is_empty(),
            HooksLevel::Group => self.field_idx >= 2,
        };
        if deletable {
            self.confirm_delete = true;
        }
    }

    /// Applique une suppression confirmée.
    pub fn apply_delete(&mut self) {
        self.confirm_delete = false;
        match self.level {
            HooksLevel::Groups => {
                if self.group_idx < self.groups.len() {
                    self.groups.remove(self.group_idx);
                    if self.group_idx > 0 && self.group_idx >= self.groups.len() {
                        self.group_idx -= 1;
                    }
                }
            }
            HooksLevel::Group => {
                let ci = self.field_idx - 2;
                if let Some(g) = self.groups.get_mut(self.group_idx) {
                    if ci < g.commands.len() {
                        g.commands.remove(ci);
                    }
                }
                let rows = self.group_rows();
                if self.field_idx >= rows {
                    self.field_idx = rows.saturating_sub(1);
                }
            }
        }
    }

    pub fn enter(&mut self) {
        if self.level == HooksLevel::Groups && !self.groups.is_empty() {
            self.level = HooksLevel::Group;
            self.field_idx = 0;
        }
    }

    /// Remonte d'un niveau. Renvoie `false` si on est déjà au niveau Groups
    /// (l'appelant ferme alors la modale).
    pub fn back(&mut self) -> bool {
        match self.level {
            HooksLevel::Group => {
                self.level = HooksLevel::Groups;
                true
            }
            HooksLevel::Groups => false,
        }
    }
}

/// Déplacement borné dans [0, len) (pas de bouclage), comme le reste du TUI.
fn step(idx: usize, delta: i32, len: usize) -> usize {
    if len == 0 {
        return 0;
    }
    let max = len - 1;
    if delta < 0 {
        idx.saturating_sub((-delta) as usize)
    } else {
        (idx + delta as usize).min(max)
    }
}
```

Déclarer le module dans `crates/claudine/src/tui/mod.rs`, près des autres (`pub mod app; pub mod settings_form; pub mod ui;`) :

```rust
pub mod hooks_editor;
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p claudine hooks_editor`
Expected: PASS (2 tests).

- [ ] **Step 5: Commit**

```bash
git add crates/claudine/src/tui/hooks_editor.rs crates/claudine/src/tui/mod.rs
git commit -m "feat(tui): HooksEditor — navigation niveau groupes"
```

---

### Task 5: TUI — `HooksEditor`, édition des champs et commandes

**Files:**
- Modify: `crates/claudine/src/tui/hooks_editor.rs`

**Interfaces:**
- Consumes: tout de la Task 4.
- Produces (utilisés par la Task 6) :
  - `fn add_command(&mut self)`
  - `fn begin_edit(&mut self)` / `fn begin_edit_timeout(&mut self)`
  - `fn input_char(&mut self, c: char)` / `fn input_backspace(&mut self)` / `fn input_commit(&mut self)` / `fn input_cancel(&mut self)`
  - `fn editing(&self) -> bool`
  - `fn into_groups(self) -> Vec<HookGroup>`

- [ ] **Step 1: Write the failing test**

Ajouter dans le module `tests` de `hooks_editor.rs` :

```rust
    #[test]
    fn edit_event_matcher_and_add_command() {
        let mut e = HooksEditor::new(vec![HookGroup {
            event: "Stop".into(),
            matcher: None,
            commands: vec![],
        }]);
        e.enter(); // niveau Group, field_idx = 0 (évènement)

        // Édite l'évènement.
        e.begin_edit();
        assert!(e.editing());
        for c in "PreToolUse".chars() {
            e.input_char(c);
        }
        // efface le tampon initial d'abord : on part du contenu existant.
        e.input_commit();
        assert!(!e.editing());

        // Ajoute une commande puis l'édite (field passe sur la commande).
        e.add_command();
        assert_eq!(e.groups[0].commands.len(), 1);
        // la sélection se place sur la nouvelle commande (row 2).
        assert_eq!(e.field_idx, 2);
        e.begin_edit();
        for c in "echo hi".chars() {
            e.input_char(c);
        }
        e.input_commit();
        assert_eq!(e.groups[0].commands[0].command, "echo hi");
    }

    #[test]
    fn edit_command_timeout() {
        let mut e = HooksEditor::new(vec![HookGroup {
            event: "Stop".into(),
            matcher: None,
            commands: vec![HookCommand { kind: "command".into(), command: "x".into(), timeout: None }],
        }]);
        e.enter();
        e.field_idx = 2; // la commande
        e.begin_edit_timeout();
        assert!(e.editing());
        e.input_char('a'); // ignoré (non chiffre)
        for c in "45".chars() {
            e.input_char(c);
        }
        e.input_commit();
        assert_eq!(e.groups[0].commands[0].timeout, Some(45));
    }
```

> Note : `begin_edit` initialise le tampon avec la valeur courante du champ. Pour l'évènement « Stop », taper « PreToolUse » sans effacer donnerait « StopPreToolUse » — le test vérifie surtout `command`. Pour l'évènement, l'utilisateur effacera via Backspace en usage réel ; le test ne dépend pas de la valeur finale de l'évènement.

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p claudine edit_event_matcher_and_add_command`
Expected: FAIL — méthodes inconnues.

- [ ] **Step 3: Write minimal implementation**

Ajouter ces méthodes dans `impl HooksEditor` :

```rust
    pub fn editing(&self) -> bool {
        !matches!(self.edit, HookEdit::None)
    }

    /// Ajoute une commande vide au groupe courant et la sélectionne.
    pub fn add_command(&mut self) {
        if let Some(g) = self.groups.get_mut(self.group_idx) {
            g.commands.push(HookCommand {
                kind: "command".to_string(),
                command: String::new(),
                timeout: None,
            });
            // Sélectionne la nouvelle commande (rows : 0 évènement, 1 matcher, 2+ cmd).
            self.field_idx = 2 + g.commands.len() - 1;
        }
    }

    /// Valeur texte courante du champ sélectionné (niveau Group).
    fn current_field_value(&self) -> String {
        let Some(g) = self.groups.get(self.group_idx) else {
            return String::new();
        };
        match self.field_idx {
            0 => g.event.clone(),
            1 => g.matcher.clone().unwrap_or_default(),
            n => g
                .commands
                .get(n - 2)
                .map(|c| c.command.clone())
                .unwrap_or_default(),
        }
    }

    /// Démarre l'édition du champ sélectionné (niveau Group uniquement).
    pub fn begin_edit(&mut self) {
        if self.level == HooksLevel::Group {
            self.edit = HookEdit::Text(self.current_field_value());
        }
    }

    /// Démarre l'édition du timeout de la commande sélectionnée (rows ≥ 2).
    pub fn begin_edit_timeout(&mut self) {
        if self.level != HooksLevel::Group || self.field_idx < 2 {
            return;
        }
        let cur = self
            .groups
            .get(self.group_idx)
            .and_then(|g| g.commands.get(self.field_idx - 2))
            .and_then(|c| c.timeout)
            .map(|t| t.to_string())
            .unwrap_or_default();
        self.edit = HookEdit::Timeout(cur);
    }

    pub fn input_char(&mut self, c: char) {
        match &mut self.edit {
            HookEdit::Text(buf) => buf.push(c),
            // Le timeout n'accepte que des chiffres.
            HookEdit::Timeout(buf) if c.is_ascii_digit() => buf.push(c),
            _ => {}
        }
    }

    pub fn input_backspace(&mut self) {
        match &mut self.edit {
            HookEdit::Text(buf) | HookEdit::Timeout(buf) => {
                buf.pop();
            }
            HookEdit::None => {}
        }
    }

    pub fn input_cancel(&mut self) {
        self.edit = HookEdit::None;
    }

    /// Valide la saisie dans le champ sélectionné.
    pub fn input_commit(&mut self) {
        match std::mem::replace(&mut self.edit, HookEdit::None) {
            HookEdit::Text(buf) => {
                let Some(g) = self.groups.get_mut(self.group_idx) else {
                    return;
                };
                match self.field_idx {
                    0 => g.event = buf,
                    1 => g.matcher = if buf.is_empty() { None } else { Some(buf) },
                    n => {
                        if let Some(c) = g.commands.get_mut(n - 2) {
                            c.command = buf;
                        }
                    }
                }
            }
            HookEdit::Timeout(buf) => {
                let val = if buf.is_empty() {
                    None
                } else {
                    buf.parse::<u64>().ok()
                };
                if self.field_idx >= 2 {
                    if let Some(g) = self.groups.get_mut(self.group_idx) {
                        if let Some(c) = g.commands.get_mut(self.field_idx - 2) {
                            c.timeout = val;
                        }
                    }
                }
            }
            HookEdit::None => {}
        }
    }

    /// Consomme l'éditeur et renvoie les groupes (pour l'enregistrement).
    pub fn into_groups(self) -> Vec<HookGroup> {
        self.groups
    }
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p claudine edit_event_matcher_and_add_command`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/claudine/src/tui/hooks_editor.rs
git commit -m "feat(tui): HooksEditor — édition champs + commandes"
```

---

### Task 6: TUI — câblage de l'éditeur de hooks (app + mod + ui)

**Files:**
- Modify: `crates/claudine/src/tui/app.rs`
- Modify: `crates/claudine/src/tui/mod.rs`
- Modify: `crates/claudine/src/tui/ui.rs`

**Interfaces:**
- Consumes: `HooksEditor` (Tasks 4-5), `claudine_core::{read_hook_groups, write_hooks}`.
- Produces: `App::open_hooks_editor`, `App::hooks_save`, champ `pub hooks_editor: Option<HooksEditor>`.

- [ ] **Step 1: Write the failing test**

Ajouter dans le module `tests` de `crates/claudine/src/tui/app.rs` :

```rust
    #[test]
    fn hooks_editor_open_edit_and_save_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let base = dir.path();
        fs::create_dir_all(base.join("projects")).unwrap();
        fs::write(base.join("settings.json"), "{}").unwrap();
        let mut app = App::with_homes(vec![ClaudeHome::from_base(base)]);
        app.set_section(Section::Extensions);

        app.open_hooks_editor();
        assert!(app.hooks_editor.is_some());
        {
            let e = app.hooks_editor.as_mut().unwrap();
            e.add_group(); // un groupe PreToolUse vide
            e.enter();
            e.add_command();
            e.begin_edit();
            for c in "echo hi".chars() {
                e.input_char(c);
            }
            e.input_commit();
        }
        app.hooks_save();
        assert!(app.hooks_editor.is_none(), "fermé après enregistrement");

        let groups = claudine_core::read_hook_groups(app.home());
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].event, "PreToolUse");
        assert_eq!(groups[0].commands[0].command, "echo hi");
    }
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p claudine hooks_editor_open_edit_and_save`
Expected: FAIL — champ/méthodes inconnus.

- [ ] **Step 3: Write minimal implementation**

Dans `crates/claudine/src/tui/app.rs` :

1. Import : ajouter `read_hook_groups, write_hooks` à la liste `use claudine_core::{...}` et `use crate::tui::hooks_editor::HooksEditor;`.

2. Champ dans `struct App` (près de `trash_view` / `import`) :

```rust
    /// Éditeur de hooks (modal) ; `None` = fermé.
    pub hooks_editor: Option<HooksEditor>,
```

3. Initialisation dans `with_homes` (près de `import: None,`) : `hooks_editor: None,`.

4. Méthodes (près de la section Extensions / Config) :

```rust
    /// Ouvre l'éditeur de hooks pour le home actif (depuis la section Extensions).
    pub fn open_hooks_editor(&mut self) {
        if self.section != Section::Extensions {
            return;
        }
        let groups = read_hook_groups(self.home());
        self.hooks_editor = Some(HooksEditor::new(groups));
    }

    pub fn hooks_cancel(&mut self) {
        self.hooks_editor = None;
    }

    /// Enregistre les hooks édités dans settings.json du home actif.
    pub fn hooks_save(&mut self) {
        let Some(editor) = self.hooks_editor.take() else {
            return;
        };
        let groups = editor.into_groups();
        match write_hooks(self.home(), &groups) {
            Ok(()) => {
                self.reload_files();
                self.status = Some("Hooks enregistrés".to_string());
            }
            Err(e) => {
                self.status = Some(format!("Échec enregistrement hooks : {e}"));
            }
        }
    }
```

Dans `crates/claudine/src/tui/mod.rs`, ajouter la capture de touches de la modale **avant** le `match key.code` principal (à côté des autres modales) :

```rust
    // Éditeur de hooks (modal).
    if app.hooks_editor.is_some() {
        handle_hooks_editor_key(app, key);
        return;
    }
```

Et la fonction de routage (en bas du fichier, près de `handle_settings_edit_key`) :

```rust
fn handle_hooks_editor_key(app: &mut App, key: KeyEvent) {
    use crate::tui::hooks_editor::HooksLevel;
    let Some(e) = app.hooks_editor.as_mut() else {
        return;
    };
    // Confirmation de suppression prioritaire.
    if e.confirm_delete {
        match key.code {
            KeyCode::Char('o') | KeyCode::Char('O') | KeyCode::Char('y') | KeyCode::Char('Y')
            | KeyCode::Enter => e.apply_delete(),
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => e.confirm_delete = false,
            _ => {}
        }
        return;
    }
    // Saisie d'un champ.
    if e.editing() {
        match key.code {
            KeyCode::Esc => e.input_cancel(),
            KeyCode::Enter => e.input_commit(),
            KeyCode::Backspace => e.input_backspace(),
            KeyCode::Char(c) => e.input_char(c),
            _ => {}
        }
        return;
    }
    // Navigation.
    match key.code {
        KeyCode::Up | KeyCode::Char('k') => e.move_sel(-1),
        KeyCode::Down | KeyCode::Char('j') => e.move_sel(1),
        KeyCode::Char('a') => match e.level {
            HooksLevel::Groups => e.add_group(),
            HooksLevel::Group => e.add_command(),
        },
        KeyCode::Char('d') => e.delete_current(),
        KeyCode::Char('t') => e.begin_edit_timeout(),
        KeyCode::Char('s') => app.hooks_save(),
        KeyCode::Enter => match e.level {
            HooksLevel::Groups => e.enter(),
            HooksLevel::Group => e.begin_edit(),
        },
        KeyCode::Esc => {
            if !e.back() {
                app.hooks_cancel();
            }
        }
        _ => {}
    }
}
```

Dans `crates/claudine/src/tui/ui.rs` :

1. Importer `HooksLevel` : `use super::app::...` reste ; ajouter `use crate::tui::hooks_editor::{HookEdit, HooksEditor, HooksLevel, KNOWN_EVENTS};`.

2. Appeler le rendu dans `render(...)`, après `render_import` :

```rust
    if app.hooks_editor.is_some() {
        render_hooks_editor(app, f, area);
    }
```

3. La fonction de rendu (près de `render_import`) :

```rust
/// Modal de l'éditeur de hooks.
fn render_hooks_editor(app: &App, f: &mut Frame, area: Rect) {
    let Some(e) = &app.hooks_editor else {
        return;
    };
    let popup = centered_rect(80, 70, area);
    f.render_widget(Clear, popup);
    let hint = match e.level {
        HooksLevel::Groups => " a ajouter · Enter ouvrir · d suppr. · s enregistrer · Esc fermer ",
        HooksLevel::Group => " a commande · Enter éditer · t timeout · d suppr. · s enregistrer · Esc retour ",
    };
    let block = Block::default()
        .borders(Borders::ALL)
        .title(Span::styled(
            " Éditeur de hooks ",
            Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
        ))
        .title(Line::from(hint).right_aligned());
    let inner = block.inner(popup);
    f.render_widget(block, popup);

    let mut lines: Vec<Line> = Vec::new();
    match e.level {
        HooksLevel::Groups => {
            if e.groups.is_empty() {
                lines.push(Line::from(Span::styled(
                    "  (aucun hook — 'a' pour en ajouter)",
                    Style::default().fg(DIM),
                )));
            }
            for (i, g) in e.groups.iter().enumerate() {
                let sel = i == e.group_idx;
                let matcher = g
                    .matcher
                    .as_deref()
                    .map(|m| format!(" [{m}]"))
                    .unwrap_or_default();
                let txt = format!(
                    "{} {}{}  · {} cmd",
                    if sel { "▶" } else { " " },
                    g.event,
                    matcher,
                    g.commands.len()
                );
                let style = if sel {
                    selection_style(true)
                } else {
                    Style::default()
                };
                lines.push(Line::from(Span::styled(txt, style)));
            }
        }
        HooksLevel::Group => {
            let g = match e.groups.get(e.group_idx) {
                Some(g) => g,
                None => return,
            };
            let row = |sel: bool, label: String| {
                let style = if sel {
                    selection_style(true)
                } else {
                    Style::default()
                };
                Line::from(Span::styled(label, style))
            };
            lines.push(row(
                e.field_idx == 0,
                format!("  Évènement : {}", g.event),
            ));
            lines.push(row(
                e.field_idx == 1,
                format!(
                    "  Matcher   : {}",
                    g.matcher.as_deref().unwrap_or("(aucun)")
                ),
            ));
            for (ci, c) in g.commands.iter().enumerate() {
                let to = c
                    .timeout
                    .map(|t| format!("  (timeout {t}s)"))
                    .unwrap_or_default();
                lines.push(row(
                    e.field_idx == ci + 2,
                    format!("    $ {}{}", c.command, to),
                ));
            }
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                format!("  évènements connus : {}", KNOWN_EVENTS.join(", ")),
                Style::default().fg(DIM),
            )));
        }
    }

    // Bandeau de saisie ou de confirmation.
    if let HookEdit::Text(buf) = &e.edit {
        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::styled(
                "  Saisie : ",
                Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
            ),
            Span::raw(buf.clone()),
            Span::styled("▏", Style::default().fg(ACCENT)),
        ]));
    } else if e.confirm_delete {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "  Supprimer l'élément sélectionné ? (o/n)",
            Style::default().fg(Color::Black).bg(Color::Red).add_modifier(Modifier::BOLD),
        )));
    }

    f.render_widget(Paragraph::new(Text::from(lines)), inner);
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p claudine hooks_editor_open_edit_and_save && cargo clippy --workspace`
Expected: test PASS, 0 warning.

- [ ] **Step 5: Commit**

```bash
git add crates/claudine/src/tui/app.rs crates/claudine/src/tui/mod.rs crates/claudine/src/tui/ui.rs
git commit -m "feat(tui): câblage de l'éditeur de hooks (Enter depuis Extensions)"
```

---

### Task 7: TUI — modal de bascule des plugins

**Files:**
- Modify: `crates/claudine/src/tui/app.rs`
- Modify: `crates/claudine/src/tui/mod.rs`
- Modify: `crates/claudine/src/tui/ui.rs`

**Interfaces:**
- Consumes: `claudine_core::{read_extensions, set_plugin_enabled}`.
- Produces: `pub struct PluginToggleItem { pub name: String, pub enabled: bool }`, champ `pub plugins_toggle: Option<PluginsToggle>` avec `pub struct PluginsToggle { pub items: Vec<PluginToggleItem>, pub idx: usize }`, méthodes `open_plugins_toggle`, `plugins_toggle_move`, `plugins_toggle_flip`, `plugins_toggle_save`, `plugins_toggle_cancel`.

- [ ] **Step 1: Write the failing test**

Ajouter dans le module `tests` de `app.rs` :

```rust
    #[test]
    fn plugins_toggle_flips_and_saves() {
        let dir = tempfile::tempdir().unwrap();
        let base = dir.path();
        fs::create_dir_all(base.join("projects")).unwrap();
        fs::write(
            base.join("settings.json"),
            r#"{"enabledPlugins":{"foo@m":true}}"#,
        )
        .unwrap();
        fs::create_dir_all(base.join("plugins")).unwrap();
        fs::write(
            base.join("plugins/installed_plugins.json"),
            r#"{"version":1,"plugins":{"foo@m":[{"scope":"user","version":"1.0.0"}]}}"#,
        )
        .unwrap();
        let mut app = App::with_homes(vec![ClaudeHome::from_base(base)]);
        app.set_section(Section::Extensions);

        app.open_plugins_toggle();
        assert!(app.plugins_toggle.is_some());
        assert_eq!(app.plugins_toggle.as_ref().unwrap().items.len(), 1);
        app.plugins_toggle_flip(); // foo@m : true -> false
        app.plugins_toggle_save();
        assert!(app.plugins_toggle.is_none());

        let ext = claudine_core::read_extensions(app.home());
        assert!(!ext.plugins.iter().find(|p| p.name == "foo@m").unwrap().enabled);
    }
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p claudine plugins_toggle_flips_and_saves`
Expected: FAIL — types/méthodes inconnus.

- [ ] **Step 3: Write minimal implementation**

Dans `app.rs` :

1. Import : ajouter `set_plugin_enabled` à `use claudine_core::{...}`.

2. Types (près des autres structs de modale, ex. `TrashState`) :

```rust
/// Une ligne du modal de bascule des plugins.
#[derive(Debug, Clone)]
pub struct PluginToggleItem {
    pub name: String,
    pub enabled: bool,
}

/// État du modal de bascule des plugins.
pub struct PluginsToggle {
    pub items: Vec<PluginToggleItem>,
    pub idx: usize,
}
```

3. Champ dans `App` : `pub plugins_toggle: Option<PluginsToggle>,` ; init `plugins_toggle: None,`.

4. Méthodes :

```rust
    pub fn open_plugins_toggle(&mut self) {
        if self.section != Section::Extensions {
            return;
        }
        let items: Vec<PluginToggleItem> = self
            .extensions
            .plugins
            .iter()
            .map(|p| PluginToggleItem {
                name: p.name.clone(),
                enabled: p.enabled,
            })
            .collect();
        if items.is_empty() {
            self.status = Some("Aucun plugin installé".to_string());
            return;
        }
        self.plugins_toggle = Some(PluginsToggle { items, idx: 0 });
    }

    pub fn plugins_toggle_cancel(&mut self) {
        self.plugins_toggle = None;
    }

    pub fn plugins_toggle_move(&mut self, delta: i32) {
        if let Some(pt) = &mut self.plugins_toggle {
            pt.idx = step(pt.idx, delta, pt.items.len());
        }
    }

    pub fn plugins_toggle_flip(&mut self) {
        if let Some(pt) = &mut self.plugins_toggle {
            if let Some(it) = pt.items.get_mut(pt.idx) {
                it.enabled = !it.enabled;
            }
        }
    }

    /// Écrit l'état (un set_plugin_enabled par plugin) puis recharge.
    pub fn plugins_toggle_save(&mut self) {
        let Some(pt) = self.plugins_toggle.take() else {
            return;
        };
        let home = self.home().clone();
        let mut err = None;
        for it in &pt.items {
            if let Err(e) = set_plugin_enabled(&home, &it.name, it.enabled) {
                err = Some(e);
                break;
            }
        }
        self.reload_files();
        self.status = Some(match err {
            Some(e) => format!("Échec enregistrement plugins : {e}"),
            None => "Plugins enregistrés".to_string(),
        });
    }
```

Dans `mod.rs`, capture de la modale (avant le match principal) :

```rust
    // Bascule des plugins (modal).
    if app.plugins_toggle.is_some() {
        match key.code {
            KeyCode::Esc => app.plugins_toggle_cancel(),
            KeyCode::Up | KeyCode::Char('k') => app.plugins_toggle_move(-1),
            KeyCode::Down | KeyCode::Char('j') => app.plugins_toggle_move(1),
            KeyCode::Char(' ') => app.plugins_toggle_flip(),
            KeyCode::Char('s') => app.plugins_toggle_save(),
            _ => {}
        }
        return;
    }
```

Et l'ouverture par `p` en section Extensions : dans le `match key.code` principal, ajouter une branche :

```rust
        KeyCode::Char('p') => app.open_plugins_toggle(),
```

(elle ne fait rien hors Extensions — `open_plugins_toggle` vérifie la section).

Dans `ui.rs`, appel du rendu dans `render(...)` après `render_hooks_editor` :

```rust
    if app.plugins_toggle.is_some() {
        render_plugins_toggle(app, f, area);
    }
```

Et la fonction :

```rust
/// Modal de bascule activer/désactiver des plugins.
fn render_plugins_toggle(app: &App, f: &mut Frame, area: Rect) {
    let Some(pt) = &app.plugins_toggle else {
        return;
    };
    let popup = centered_rect(70, 60, area);
    f.render_widget(Clear, popup);
    let block = Block::default()
        .borders(Borders::ALL)
        .title(Span::styled(
            " Plugins — activer / désactiver ",
            Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
        ))
        .title(Line::from(" Espace bascule · s enregistrer · Esc fermer ").right_aligned());
    let inner = block.inner(popup);
    f.render_widget(block, popup);
    let items: Vec<ListItem> = pt
        .items
        .iter()
        .map(|it| {
            let (mark, mstyle) = if it.enabled {
                ("✓", Style::default().fg(Color::Green))
            } else {
                ("✗", Style::default().fg(DIM))
            };
            ListItem::new(Line::from(vec![
                Span::styled(format!(" {mark} "), mstyle),
                Span::raw(it.name.clone()),
            ]))
        })
        .collect();
    let list = List::new(items)
        .highlight_style(selection_style(true))
        .highlight_symbol("▶ ");
    let mut state = ListState::default();
    if !pt.items.is_empty() {
        state.select(Some(pt.idx.min(pt.items.len() - 1)));
    }
    f.render_stateful_widget(list, inner, &mut state);
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p claudine plugins_toggle_flips_and_saves && cargo clippy --workspace`
Expected: test PASS, 0 warning.

- [ ] **Step 5: Commit**

```bash
git add crates/claudine/src/tui/app.rs crates/claudine/src/tui/mod.rs crates/claudine/src/tui/ui.rs
git commit -m "feat(tui): modal de bascule des plugins (p depuis Extensions)"
```

---

### Task 8: Raccourcis (footer/aide) + vérification finale

**Files:**
- Modify: `crates/claudine/src/tui/ui.rs`

**Interfaces:**
- Consumes: tout ce qui précède.
- Produces: rien de nouveau (mise à jour de l'affichage des raccourcis).

- [ ] **Step 1: Mettre à jour le footer de la section Extensions**

Dans `render_footer`, l'arm `Section::Extensions` (créé en phase 1) — remplacer ses `key_hints` par :

```rust
        Section::Extensions => key_hints(&[
            ("Tab/1·2·3·4", "sections"),
            ("Enter", "éditer hooks"),
            ("p", "plugins"),
            ("↑/↓", "défiler"),
            ("t", "cible"),
            ("E", "settings.json"),
            ("?", "aide"),
        ]),
```

- [ ] **Step 2: Mettre à jour l'aide**

Dans `render_help`, sous la ligne `("Extensions", ...)` existante, remplacer par :

```rust
        ("Extensions", "hooks · plugins · MCP (lecture) ; Enter édite les hooks, p (dés)active les plugins"),
```

- [ ] **Step 3: Vérification complète**

Run: `cargo clippy --workspace 2>&1 | grep -cE "warning|error"` → attendu `0`
Run: `cargo test --workspace` → attendu : tous les paquets `ok`, 0 échec.

- [ ] **Step 4: Vérification manuelle (facultative mais recommandée)**

Sur un home de démo isolé :
```bash
HOME=/tmp/cl-demo CLAUDE_CONFIG_DIR=/tmp/cl-demo/.claude cargo run -p claudine
```
Aller en Extensions (`4`), `Enter` (éditer un hook), `a`/`Enter`/`s` ; puis `p` pour basculer un plugin. Vérifier `settings.json` et le backup `.bak-<nanos>`.

- [ ] **Step 5: Commit**

```bash
git add crates/claudine/src/tui/ui.rs
git commit -m "feat(tui): raccourcis Extensions (Enter hooks, p plugins) + aide"
```

---

## Self-Review

**1. Couverture de la spec :**
- §3 modèle (HookCommand/HookGroup) → Task 1. ✓
- §4 API cœur (read_hook_groups, write_hooks, set_plugin_enabled) → Tasks 1-3. ✓
- §5 éditeur de hooks modal (navigation 2 niveaux ; évènement/matcher/commandes éditables ; timeout éditable via `t`) → Tasks 4-6. ✓
- §6 bascule plugins → Task 7. ✓
- §7 raccourcis (Enter hooks, p plugins, t timeout, E) → Tasks 6-8. ✓
- §8 sûreté (backup/atomique via SettingsDoc, confirmation suppression, validation, multi-home) → Tasks 2/3 (backup), 4/6 (confirmation suppression), multi-home via `self.home()`. Validation : le timeout n'accepte que des chiffres (saisie filtrée). **Simplification assumée** : une commande vide n'est pas rejetée à l'enregistrement (elle est écrite telle quelle ; un hook à commande vide est inoffensif) — un blocage dur pourra être ajouté si besoin.
- §9 tests → présents dans chaque tâche. ✓

**2. Placeholders :** aucun TODO/TBD ; tout le code est fourni.

**3. Cohérence des types :** `HookGroup`/`HookCommand` (champs `event`/`matcher`/`commands`, `kind`/`command`/`timeout`) identiques entre cœur (Tasks 1-3) et TUI (Tasks 4-7). `HooksEditor` méthodes (`new`, `move_sel`, `add_group`, `delete_current`, `apply_delete`, `enter`, `back`, `add_command`, `begin_edit`, `editing`, `input_*`, `into_groups`) cohérentes entre Tasks 4/5 et l'usage Task 6.
