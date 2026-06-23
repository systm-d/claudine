# Phase 2b — Édition des serveurs MCP (portée utilisateur) — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Rendre éditables, depuis la section Extensions du TUI, les serveurs MCP de portée utilisateur du `.claude.json` du home actif.

**Architecture:** Le cœur (`claudine-core/src/extensions.rs`) gagne un modèle d'édition des serveurs MCP et trois fonctions (résolution du fichier, lecture, écriture) qui s'appuient sur `SettingsDoc` (backup + écriture atomique + `preserve_order`, ne réécrit que la clé `mcpServers`). Le TUI ajoute un éditeur MCP dédié (`mcp_editor.rs`) câblé dans `app.rs`/`mod.rs`/`ui.rs` comme l'éditeur de hooks (2a).

**Tech Stack:** Rust (workspace 2 crates), ratatui 0.28, serde_json (feature `preserve_order`), tests via `tempfile`.

## Global Constraints

- MSRV 1.74, édition 2021.
- `crates/claudine-core` ne dépend d'aucune lib d'UI.
- Écriture de fichiers : toujours via `SettingsDoc` (backup `.bak-<nanos>` + temp+rename). Jamais d'écriture brute. Seule la clé `mcpServers` est réécrite ; toutes les autres clés du `.claude.json` sont préservées.
- Style formaté à la main ; valider via `cargo clippy --workspace` (0 warning) + `cargo test --workspace`. **Ne pas** lancer `cargo fmt`.
- `crossterm` via `ratatui::crossterm`.
- Portée **utilisateur uniquement** (`mcpServers` racine). Portée projet hors périmètre.

---

### Task 1: Cœur — modèle MCP + résolution + lecture

**Files:**
- Modify: `crates/claudine-core/src/extensions.rs`
- Modify: `crates/claudine-core/src/lib.rs`

**Interfaces:**
- Consumes: `ClaudeHome`, `load_json` (helper privé existant), `mcp_config_candidates` (helper privé existant qui renvoie `Vec<PathBuf>`).
- Produces:
  - `pub enum McpTransport { Stdio, Http, Sse }`
  - `pub struct McpServer { pub name: String, pub transport: McpTransport, pub command: String, pub args: Vec<String>, pub env: Vec<(String,String)>, pub url: String, pub headers: Vec<(String,String)> }`
  - `pub fn mcp_config_path(home: &ClaudeHome) -> PathBuf`
  - `pub fn read_user_mcp_servers(home: &ClaudeHome) -> Vec<McpServer>`

- [ ] **Step 1: Write the failing test**

Ajouter dans le module `tests` de `crates/claudine-core/src/extensions.rs` (le helper `home_with` y existe) :

```rust
    #[test]
    fn read_user_mcp_servers_parses_stdio_and_http() {
        let cfg = r#"{
            "mcpServers": {
                "fs": {"type":"stdio","command":"npx","args":["-y","server-fs"],"env":{"TOKEN":"x"}},
                "db": {"type":"http","url":"http://localhost:1","headers":{"Authorization":"Bearer y"}}
            }
        }"#;
        let (_d, home) = home_with(&[(".claude.json", cfg)]);
        let servers = read_user_mcp_servers(&home);
        assert_eq!(servers.len(), 2);
        let fs = servers.iter().find(|s| s.name == "fs").unwrap();
        assert!(matches!(fs.transport, McpTransport::Stdio));
        assert_eq!(fs.command, "npx");
        assert_eq!(fs.args, vec!["-y", "server-fs"]);
        assert_eq!(fs.env, vec![("TOKEN".to_string(), "x".to_string())]);
        let db = servers.iter().find(|s| s.name == "db").unwrap();
        assert!(matches!(db.transport, McpTransport::Http));
        assert_eq!(db.url, "http://localhost:1");
        assert_eq!(db.headers, vec![("Authorization".to_string(), "Bearer y".to_string())]);
    }

    #[test]
    fn mcp_config_path_prefers_existing_in_home() {
        // in-home .claude.json présent → choisi.
        let (_d, home) = home_with(&[(".claude.json", "{}")]);
        assert_eq!(mcp_config_path(&home), home.base.join(".claude.json"));
    }
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p claudine-core read_user_mcp_servers_parses mcp_config_path_prefers`
Expected: FAIL — types/fonctions inconnus.

- [ ] **Step 3: Write minimal implementation**

Dans `crates/claudine-core/src/extensions.rs`, ajouter (après les structs existantes ; `Map`/`Value`/`SettingsDoc`/`Result`/`PathBuf` sont déjà importés) :

```rust
/// Transport d'un serveur MCP.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum McpTransport {
    Stdio,
    Http,
    Sse,
}

/// Un serveur MCP éditable (portée utilisateur).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct McpServer {
    pub name: String,
    pub transport: McpTransport,
    pub command: String,
    pub args: Vec<String>,
    pub env: Vec<(String, String)>,
    pub url: String,
    pub headers: Vec<(String, String)>,
}

impl Default for McpServer {
    fn default() -> Self {
        Self {
            name: String::new(),
            transport: McpTransport::Stdio,
            command: String::new(),
            args: Vec::new(),
            env: Vec::new(),
            url: String::new(),
            headers: Vec::new(),
        }
    }
}

/// Fichier `.claude.json` à lire/écrire pour ce home : premier candidat existant
/// (in-home prioritaire, puis hérité voisin), sinon `<home>/.claude.json` par défaut.
pub fn mcp_config_path(home: &ClaudeHome) -> PathBuf {
    for cand in mcp_config_candidates(home) {
        if cand.is_file() {
            return cand;
        }
    }
    home.base.join(".claude.json")
}

fn read_pairs(v: Option<&Value>) -> Vec<(String, String)> {
    v.and_then(|o| o.as_object())
        .map(|o| {
            o.iter()
                .filter_map(|(k, val)| val.as_str().map(|s| (k.clone(), s.to_string())))
                .collect()
        })
        .unwrap_or_default()
}

/// Lit les serveurs MCP de portée utilisateur (`mcpServers` racine) du fichier résolu.
pub fn read_user_mcp_servers(home: &ClaudeHome) -> Vec<McpServer> {
    let Some(v) = load_json(&mcp_config_path(home)) else {
        return Vec::new();
    };
    let Some(servers) = v.get("mcpServers").and_then(|m| m.as_object()) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for (name, def) in servers {
        let transport = match def.get("type").and_then(|t| t.as_str()) {
            Some("http") => McpTransport::Http,
            Some("sse") => McpTransport::Sse,
            _ => McpTransport::Stdio,
        };
        out.push(McpServer {
            name: name.clone(),
            transport,
            command: def.get("command").and_then(|c| c.as_str()).unwrap_or("").to_string(),
            args: def
                .get("args")
                .and_then(|a| a.as_array())
                .map(|a| a.iter().filter_map(|x| x.as_str().map(String::from)).collect())
                .unwrap_or_default(),
            env: read_pairs(def.get("env")),
            url: def.get("url").and_then(|u| u.as_str()).unwrap_or("").to_string(),
            headers: read_pairs(def.get("headers")),
        });
    }
    out
}
```

Re-exporter dans `crates/claudine-core/src/lib.rs` (ajouter à la liste d'export de `extensions::`) : `mcp_config_path, read_user_mcp_servers, McpServer, McpTransport`.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p claudine-core read_user_mcp_servers_parses mcp_config_path_prefers`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/claudine-core/src/extensions.rs crates/claudine-core/src/lib.rs
git commit -m "feat(core): modèle MCP éditable + mcp_config_path + read_user_mcp_servers"
```

---

### Task 2: Cœur — `write_user_mcp_servers`

**Files:**
- Modify: `crates/claudine-core/src/extensions.rs`
- Modify: `crates/claudine-core/src/lib.rs`

**Interfaces:**
- Consumes: `McpServer`, `McpTransport`, `read_user_mcp_servers` (Task 1), `SettingsDoc`.
- Produces: `pub fn write_user_mcp_servers(home: &ClaudeHome, servers: &[McpServer]) -> Result<()>`

- [ ] **Step 1: Write the failing test**

```rust
    #[test]
    fn write_user_mcp_servers_round_trips_and_preserves_other_keys() {
        let cfg = r#"{"numStartups":3,"mcpServers":{"old":{"type":"stdio","command":"x"}}}"#;
        let (_d, home) = home_with(&[(".claude.json", cfg)]);

        let servers = vec![
            McpServer {
                name: "fs".into(),
                transport: McpTransport::Stdio,
                command: "npx".into(),
                args: vec!["-y".into(), "server-fs".into()],
                env: vec![("TOKEN".into(), "x".into())],
                ..Default::default()
            },
            McpServer {
                name: "db".into(),
                transport: McpTransport::Http,
                url: "http://localhost:1".into(),
                headers: vec![("Authorization".into(), "Bearer y".into())],
                ..Default::default()
            },
        ];
        write_user_mcp_servers(&home, &servers).unwrap();

        let back = read_user_mcp_servers(&home);
        assert_eq!(back.len(), 2);
        let fs = back.iter().find(|s| s.name == "fs").unwrap();
        assert_eq!(fs.args, vec!["-y", "server-fs"]);
        assert_eq!(fs.env, vec![("TOKEN".to_string(), "x".to_string())]);
        let db = back.iter().find(|s| s.name == "db").unwrap();
        assert!(matches!(db.transport, McpTransport::Http));
        assert_eq!(db.url, "http://localhost:1");
        // Autre clé racine préservée.
        let doc = crate::settings::SettingsDoc::load(&mcp_config_path(&home)).unwrap();
        assert_eq!(doc.get_i64(&["numStartups"]), Some(3));
    }

    #[test]
    fn write_user_mcp_servers_empty_removes_key() {
        let cfg = r#"{"numStartups":1,"mcpServers":{"old":{"command":"x"}}}"#;
        let (_d, home) = home_with(&[(".claude.json", cfg)]);
        write_user_mcp_servers(&home, &[]).unwrap();
        assert!(read_user_mcp_servers(&home).is_empty());
        let doc = crate::settings::SettingsDoc::load(&mcp_config_path(&home)).unwrap();
        assert!(doc.get(&["mcpServers"]).is_none());
        assert_eq!(doc.get_i64(&["numStartups"]), Some(1));
    }
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p claudine-core write_user_mcp_servers`
Expected: FAIL — `cannot find function write_user_mcp_servers`.

- [ ] **Step 3: Write minimal implementation**

```rust
fn pairs_to_object(pairs: &[(String, String)]) -> Option<Value> {
    let mut m = Map::new();
    for (k, v) in pairs {
        if !k.trim().is_empty() {
            m.insert(k.clone(), Value::String(v.clone()));
        }
    }
    if m.is_empty() {
        None
    } else {
        Some(Value::Object(m))
    }
}

/// Réécrit la clé racine `mcpServers` du `.claude.json` résolu à partir du modèle.
/// Préserve toutes les autres clés ; backup + écriture atomique via SettingsDoc.
pub fn write_user_mcp_servers(home: &ClaudeHome, servers: &[McpServer]) -> Result<()> {
    let path = mcp_config_path(home);
    let mut doc = SettingsDoc::load(&path)?;
    if servers.is_empty() {
        doc.unset(&["mcpServers"]);
        return doc.save(&path);
    }
    let mut map: Map<String, Value> = Map::new();
    for s in servers {
        let mut o = Map::new();
        match s.transport {
            McpTransport::Stdio => {
                o.insert("type".to_string(), Value::String("stdio".to_string()));
                o.insert("command".to_string(), Value::String(s.command.clone()));
                if !s.args.is_empty() {
                    o.insert(
                        "args".to_string(),
                        Value::Array(s.args.iter().map(|a| Value::String(a.clone())).collect()),
                    );
                }
                if let Some(env) = pairs_to_object(&s.env) {
                    o.insert("env".to_string(), env);
                }
            }
            McpTransport::Http | McpTransport::Sse => {
                let t = if matches!(s.transport, McpTransport::Http) {
                    "http"
                } else {
                    "sse"
                };
                o.insert("type".to_string(), Value::String(t.to_string()));
                o.insert("url".to_string(), Value::String(s.url.clone()));
                if let Some(h) = pairs_to_object(&s.headers) {
                    o.insert("headers".to_string(), h);
                }
            }
        }
        map.insert(s.name.clone(), Value::Object(o));
    }
    doc.set(&["mcpServers"], Value::Object(map));
    doc.save(&path)
}
```

Re-exporter `write_user_mcp_servers` dans `lib.rs`.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p claudine-core write_user_mcp_servers`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/claudine-core/src/extensions.rs crates/claudine-core/src/lib.rs
git commit -m "feat(core): write_user_mcp_servers (préserve les autres clés)"
```

---

### Task 3: TUI — `McpEditor`, niveau « serveurs »

**Files:**
- Create: `crates/claudine/src/tui/mcp_editor.rs`
- Modify: `crates/claudine/src/tui/mod.rs` (déclarer `pub mod mcp_editor;`)

**Interfaces:**
- Consumes: `claudine_core::{McpServer, McpTransport}`.
- Produces (utilisés Tasks 4-5) :
  - `pub enum McpLevel { Servers, Server }`
  - `pub enum McpEdit { None, Text(String) }`
  - `pub enum McpRow { Name, Type, Command, Url, Arg(usize), Env(usize), Header(usize) }`
  - `pub struct McpEditor { pub servers, pub level, pub server_idx, pub field_idx, pub edit, pub confirm_delete }`
  - `McpEditor::new(Vec<McpServer>) -> Self`, `fn rows(&self) -> Vec<McpRow>`, `fn move_sel(i32)`, `fn add_server()`, `fn delete_current()`, `fn apply_delete()`, `fn enter()`, `fn back() -> bool`

- [ ] **Step 1: Write the failing test**

Créer `crates/claudine/src/tui/mcp_editor.rs` avec, en bas, le module de tests :

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use claudine_core::{McpServer, McpTransport};

    fn sample() -> Vec<McpServer> {
        vec![McpServer {
            name: "fs".into(),
            transport: McpTransport::Stdio,
            command: "npx".into(),
            args: vec!["-y".into()],
            ..Default::default()
        }]
    }

    #[test]
    fn servers_level_add_enter_back() {
        let mut e = McpEditor::new(sample());
        assert_eq!(e.level, McpLevel::Servers);
        e.add_server();
        assert_eq!(e.servers.len(), 2);
        assert_eq!(e.server_idx, 1);
        e.enter();
        assert_eq!(e.level, McpLevel::Server);
        // rows pour un serveur stdio vide : Name, Type, Command.
        assert_eq!(e.rows(), vec![McpRow::Name, McpRow::Type, McpRow::Command]);
        assert!(e.back());
        assert_eq!(e.level, McpLevel::Servers);
        assert!(!e.back());
    }

    #[test]
    fn delete_server_needs_confirmation() {
        let mut e = McpEditor::new(sample());
        e.delete_current();
        assert!(e.confirm_delete);
        e.apply_delete();
        assert!(e.servers.is_empty());
        assert!(!e.confirm_delete);
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p claudine mcp_editor`
Expected: FAIL — module/types inconnus.

- [ ] **Step 3: Write minimal implementation**

En tête de `crates/claudine/src/tui/mcp_editor.rs` :

```rust
//! Éditeur de serveurs MCP dédié (modal) : navigation serveurs → serveur,
//! édition des champs selon le transport.

use claudine_core::{McpServer, McpTransport};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum McpLevel {
    Servers,
    Server,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum McpEdit {
    None,
    Text(String),
}

/// Une ligne éditable au niveau « serveur ».
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum McpRow {
    Name,
    Type,
    Command,
    Url,
    Arg(usize),
    Env(usize),
    Header(usize),
}

pub struct McpEditor {
    pub servers: Vec<McpServer>,
    pub level: McpLevel,
    pub server_idx: usize,
    pub field_idx: usize,
    pub edit: McpEdit,
    pub confirm_delete: bool,
}

impl McpEditor {
    pub fn new(servers: Vec<McpServer>) -> Self {
        Self {
            servers,
            level: McpLevel::Servers,
            server_idx: 0,
            field_idx: 0,
            edit: McpEdit::None,
            confirm_delete: false,
        }
    }

    /// Disposition des lignes du serveur courant selon son transport.
    pub fn rows(&self) -> Vec<McpRow> {
        let mut v = vec![McpRow::Name, McpRow::Type];
        if let Some(s) = self.servers.get(self.server_idx) {
            match s.transport {
                McpTransport::Stdio => {
                    v.push(McpRow::Command);
                    for i in 0..s.args.len() {
                        v.push(McpRow::Arg(i));
                    }
                    for i in 0..s.env.len() {
                        v.push(McpRow::Env(i));
                    }
                }
                McpTransport::Http | McpTransport::Sse => {
                    v.push(McpRow::Url);
                    for i in 0..s.headers.len() {
                        v.push(McpRow::Header(i));
                    }
                }
            }
        }
        v
    }

    pub fn move_sel(&mut self, delta: i32) {
        match self.level {
            McpLevel::Servers => {
                self.server_idx = step(self.server_idx, delta, self.servers.len());
            }
            McpLevel::Server => {
                self.field_idx = step(self.field_idx, delta, self.rows().len());
            }
        }
    }

    pub fn add_server(&mut self) {
        self.servers.push(McpServer::default());
        self.server_idx = self.servers.len() - 1;
    }

    /// Demande la suppression : un serveur (niveau Servers) ou l'élément de liste
    /// sélectionné (Arg/Env/Header au niveau Server).
    pub fn delete_current(&mut self) {
        let deletable = match self.level {
            McpLevel::Servers => !self.servers.is_empty(),
            McpLevel::Server => matches!(
                self.rows().get(self.field_idx),
                Some(McpRow::Arg(_) | McpRow::Env(_) | McpRow::Header(_))
            ),
        };
        if deletable {
            self.confirm_delete = true;
        }
    }

    pub fn apply_delete(&mut self) {
        self.confirm_delete = false;
        match self.level {
            McpLevel::Servers => {
                if self.server_idx < self.servers.len() {
                    self.servers.remove(self.server_idx);
                    if self.server_idx > 0 && self.server_idx >= self.servers.len() {
                        self.server_idx -= 1;
                    }
                }
            }
            McpLevel::Server => {
                if let Some(row) = self.rows().get(self.field_idx).copied() {
                    if let Some(s) = self.servers.get_mut(self.server_idx) {
                        match row {
                            McpRow::Arg(i) if i < s.args.len() => {
                                s.args.remove(i);
                            }
                            McpRow::Env(i) if i < s.env.len() => {
                                s.env.remove(i);
                            }
                            McpRow::Header(i) if i < s.headers.len() => {
                                s.headers.remove(i);
                            }
                            _ => {}
                        }
                    }
                }
                let n = self.rows().len();
                if self.field_idx >= n {
                    self.field_idx = n.saturating_sub(1);
                }
            }
        }
    }

    pub fn enter(&mut self) {
        if self.level == McpLevel::Servers && !self.servers.is_empty() {
            self.level = McpLevel::Server;
            self.field_idx = 0;
        }
    }

    /// Remonte d'un niveau. `false` au niveau Servers (l'appelant ferme alors).
    pub fn back(&mut self) -> bool {
        match self.level {
            McpLevel::Server => {
                self.level = McpLevel::Servers;
                true
            }
            McpLevel::Servers => false,
        }
    }
}

/// Déplacement borné dans [0, len) (pas de bouclage).
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

Déclarer le module dans `crates/claudine/src/tui/mod.rs` (à côté de `pub mod hooks_editor;`) :

```rust
pub mod mcp_editor;
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p claudine mcp_editor`
Expected: PASS (2 tests).

- [ ] **Step 5: Commit**

```bash
git add crates/claudine/src/tui/mcp_editor.rs crates/claudine/src/tui/mod.rs
git commit -m "feat(tui): McpEditor — navigation niveau serveurs"
```

---

### Task 4: TUI — `McpEditor`, édition des champs (type, scalaires, args, env/headers)

**Files:**
- Modify: `crates/claudine/src/tui/mcp_editor.rs`

**Interfaces:**
- Consumes: tout de la Task 3.
- Produces (utilisés Task 5) :
  - `fn cycle_type(&mut self, delta: i32)`
  - `fn add_item(&mut self)`
  - `fn begin_edit(&mut self)` / `fn input_char(char)` / `fn input_backspace()` / `fn input_commit()` / `fn input_cancel()` / `fn editing() -> bool`
  - `fn validation_error(&self) -> Option<String>`
  - `fn into_servers(self) -> Vec<McpServer>`

- [ ] **Step 1: Write the failing test**

Ajouter dans le module `tests` de `mcp_editor.rs` :

```rust
    #[test]
    fn edit_name_command_and_add_arg() {
        let mut e = McpEditor::new(vec![McpServer::default()]);
        e.enter(); // Server level, field 0 = Name
        e.begin_edit();
        for c in "fs".chars() {
            e.input_char(c);
        }
        e.input_commit();
        assert_eq!(e.servers[0].name, "fs");

        // Sélectionne Command (row 2) et édite.
        e.field_idx = 2;
        e.begin_edit();
        for c in "npx".chars() {
            e.input_char(c);
        }
        e.input_commit();
        assert_eq!(e.servers[0].command, "npx");

        // Ajoute un arg (sélection sur Command → ajoute à la liste args).
        e.add_item();
        assert_eq!(e.servers[0].args, vec![String::new()]);
    }

    #[test]
    fn cycle_type_changes_transport() {
        let mut e = McpEditor::new(vec![McpServer::default()]);
        e.enter();
        e.field_idx = 1; // Type
        e.cycle_type(1); // stdio -> http
        assert!(matches!(e.servers[0].transport, McpTransport::Http));
        e.cycle_type(1); // http -> sse
        assert!(matches!(e.servers[0].transport, McpTransport::Sse));
    }

    #[test]
    fn edit_env_pair_as_key_value() {
        let mut e = McpEditor::new(vec![McpServer {
            transport: McpTransport::Stdio,
            env: vec![(String::new(), String::new())],
            ..Default::default()
        }]);
        e.enter();
        // rows: Name, Type, Command, Env(0) → index 3.
        e.field_idx = 3;
        e.begin_edit();
        for c in "TOKEN=abc".chars() {
            e.input_char(c);
        }
        e.input_commit();
        assert_eq!(e.servers[0].env, vec![("TOKEN".to_string(), "abc".to_string())]);
    }

    #[test]
    fn validation_requires_name_and_command() {
        let mut e = McpEditor::new(vec![McpServer::default()]); // name vide
        assert!(e.validation_error().is_some());
        e.servers[0].name = "fs".into(); // command vide (stdio)
        assert!(e.validation_error().is_some());
        e.servers[0].command = "npx".into();
        assert!(e.validation_error().is_none());
    }
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p claudine mcp_editor`
Expected: FAIL — méthodes inconnues.

- [ ] **Step 3: Write minimal implementation**

Ajouter dans `impl McpEditor` :

```rust
    pub fn editing(&self) -> bool {
        matches!(self.edit, McpEdit::Text(_))
    }

    /// Fait défiler le transport stdio → http → sse → stdio.
    pub fn cycle_type(&mut self, delta: i32) {
        if let Some(s) = self.servers.get_mut(self.server_idx) {
            let order = [McpTransport::Stdio, McpTransport::Http, McpTransport::Sse];
            let cur = order.iter().position(|t| *t == s.transport).unwrap_or(0);
            let n = order.len() as i32;
            let next = (((cur as i32 + delta) % n + n) % n) as usize;
            s.transport = order[next];
            // Réinitialise la sélection (la disposition des lignes a changé).
            self.field_idx = self.field_idx.min(self.rows().len().saturating_sub(1));
        }
    }

    /// Ajoute un élément à la liste pertinente selon la ligne sélectionnée et le
    /// transport : env si une ligne env est sélectionnée, header si une ligne
    /// header l'est, sinon la liste principale (args en stdio, headers sinon).
    pub fn add_item(&mut self) {
        let row = self.rows().get(self.field_idx).copied();
        let Some(s) = self.servers.get_mut(self.server_idx) else {
            return;
        };
        match (s.transport, row) {
            (McpTransport::Stdio, Some(McpRow::Env(_))) => s.env.push((String::new(), String::new())),
            (McpTransport::Stdio, _) => s.args.push(String::new()),
            (_, _) => s.headers.push((String::new(), String::new())),
        }
        // Sélectionne le nouvel élément (dernier de sa section).
        self.field_idx = self.rows().len().saturating_sub(1);
    }

    fn current_value(&self) -> String {
        let Some(s) = self.servers.get(self.server_idx) else {
            return String::new();
        };
        match self.rows().get(self.field_idx).copied() {
            Some(McpRow::Name) => s.name.clone(),
            Some(McpRow::Command) => s.command.clone(),
            Some(McpRow::Url) => s.url.clone(),
            Some(McpRow::Arg(i)) => s.args.get(i).cloned().unwrap_or_default(),
            Some(McpRow::Env(i)) => s
                .env
                .get(i)
                .map(|(k, v)| format!("{k}={v}"))
                .unwrap_or_default(),
            Some(McpRow::Header(i)) => s
                .headers
                .get(i)
                .map(|(k, v)| format!("{k}={v}"))
                .unwrap_or_default(),
            _ => String::new(),
        }
    }

    /// Démarre l'édition de la ligne sélectionnée (sauf Type, qui se règle par ←/→).
    pub fn begin_edit(&mut self) {
        if self.level != McpLevel::Server {
            return;
        }
        if matches!(self.rows().get(self.field_idx), Some(McpRow::Type)) {
            return;
        }
        self.edit = McpEdit::Text(self.current_value());
    }

    pub fn input_char(&mut self, c: char) {
        if let McpEdit::Text(buf) = &mut self.edit {
            buf.push(c);
        }
    }

    pub fn input_backspace(&mut self) {
        if let McpEdit::Text(buf) = &mut self.edit {
            buf.pop();
        }
    }

    pub fn input_cancel(&mut self) {
        self.edit = McpEdit::None;
    }

    pub fn input_commit(&mut self) {
        let McpEdit::Text(buf) = std::mem::replace(&mut self.edit, McpEdit::None) else {
            return;
        };
        let row = self.rows().get(self.field_idx).copied();
        let Some(s) = self.servers.get_mut(self.server_idx) else {
            return;
        };
        match row {
            Some(McpRow::Name) => s.name = buf,
            Some(McpRow::Command) => s.command = buf,
            Some(McpRow::Url) => s.url = buf,
            Some(McpRow::Arg(i)) => {
                if let Some(a) = s.args.get_mut(i) {
                    *a = buf;
                }
            }
            Some(McpRow::Env(i)) => {
                if let Some(p) = s.env.get_mut(i) {
                    *p = split_pair(&buf);
                }
            }
            Some(McpRow::Header(i)) => {
                if let Some(p) = s.headers.get_mut(i) {
                    *p = split_pair(&buf);
                }
            }
            _ => {}
        }
    }

    /// Première erreur de validation, ou `None` si tout est valide.
    pub fn validation_error(&self) -> Option<String> {
        for s in &self.servers {
            if s.name.trim().is_empty() {
                return Some("nom de serveur vide".to_string());
            }
            match s.transport {
                McpTransport::Stdio if s.command.trim().is_empty() => {
                    return Some(format!("commande vide pour « {} »", s.name));
                }
                McpTransport::Http | McpTransport::Sse if s.url.trim().is_empty() => {
                    return Some(format!("url vide pour « {} »", s.name));
                }
                _ => {}
            }
        }
        None
    }

    pub fn into_servers(self) -> Vec<McpServer> {
        self.servers
    }
```

Et le helper libre (près de `step`) :

```rust
/// Découpe « clé=valeur » sur le premier `=` ; sans `=`, tout devient la clé.
fn split_pair(buf: &str) -> (String, String) {
    match buf.split_once('=') {
        Some((k, v)) => (k.trim().to_string(), v.to_string()),
        None => (buf.trim().to_string(), String::new()),
    }
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p claudine mcp_editor`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/claudine/src/tui/mcp_editor.rs
git commit -m "feat(tui): McpEditor — édition type/scalaires/args/env/headers + validation"
```

---

### Task 5: TUI — câblage de l'éditeur MCP (app + mod + ui)

**Files:**
- Modify: `crates/claudine/src/tui/app.rs`
- Modify: `crates/claudine/src/tui/mod.rs`
- Modify: `crates/claudine/src/tui/ui.rs`

**Interfaces:**
- Consumes: `McpEditor` (Tasks 3-4), `claudine_core::{read_user_mcp_servers, write_user_mcp_servers}`.
- Produces: champ `pub mcp_editor: Option<McpEditor>`, méthodes `open_mcp_editor`, `mcp_cancel`, `mcp_save`.

- [ ] **Step 1: Write the failing test**

Ajouter dans le module `tests` d'`app.rs` :

```rust
    #[test]
    fn mcp_editor_open_edit_and_save_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let base = dir.path();
        fs::create_dir_all(base.join("projects")).unwrap();
        fs::write(base.join(".claude.json"), "{}").unwrap();
        let mut app = App::with_homes(vec![ClaudeHome::from_base(base)]);
        app.set_section(Section::Extensions);

        app.open_mcp_editor();
        assert!(app.mcp_editor.is_some());
        {
            let e = app.mcp_editor.as_mut().unwrap();
            e.add_server();
            e.enter();
            // Name
            e.begin_edit();
            for c in "fs".chars() {
                e.input_char(c);
            }
            e.input_commit();
            // Command (row 2)
            e.field_idx = 2;
            e.begin_edit();
            for c in "npx".chars() {
                e.input_char(c);
            }
            e.input_commit();
        }
        app.mcp_save();
        assert!(app.mcp_editor.is_none(), "fermé après enregistrement");

        let servers = claudine_core::read_user_mcp_servers(app.home());
        assert_eq!(servers.len(), 1);
        assert_eq!(servers[0].name, "fs");
        assert_eq!(servers[0].command, "npx");
    }

    #[test]
    fn mcp_save_blocked_on_invalid() {
        let dir = tempfile::tempdir().unwrap();
        let base = dir.path();
        fs::create_dir_all(base.join("projects")).unwrap();
        fs::write(base.join(".claude.json"), "{}").unwrap();
        let mut app = App::with_homes(vec![ClaudeHome::from_base(base)]);
        app.set_section(Section::Extensions);
        app.open_mcp_editor();
        app.mcp_editor.as_mut().unwrap().add_server(); // nom vide
        app.mcp_save();
        assert!(app.mcp_editor.is_some(), "éditeur reste ouvert");
        assert!(app.status.as_deref().unwrap().contains("bloqué"));
    }
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p claudine mcp_editor_open_edit_and_save mcp_save_blocked`
Expected: FAIL — champ/méthodes inconnus.

- [ ] **Step 3: Write minimal implementation**

Dans `crates/claudine/src/tui/app.rs` :

1. Imports : ajouter `read_user_mcp_servers, write_user_mcp_servers` à `use claudine_core::{...}` et `use crate::tui::mcp_editor::McpEditor;`.
2. Champ dans `struct App` (près de `hooks_editor`) : `pub mcp_editor: Option<McpEditor>,`.
3. Init dans `with_homes` (près de `hooks_editor: None,`) : `mcp_editor: None,`.
4. Câbler l'ouverture par `Enter` est déjà pris par les hooks ; le MCP s'ouvre par `m` (cf. mod.rs). Méthodes :

```rust
    /// Ouvre l'éditeur MCP (portée utilisateur) du home actif, depuis Extensions.
    pub fn open_mcp_editor(&mut self) {
        if self.section != Section::Extensions {
            return;
        }
        let servers = read_user_mcp_servers(self.home());
        self.mcp_editor = Some(McpEditor::new(servers));
    }

    pub fn mcp_cancel(&mut self) {
        self.mcp_editor = None;
    }

    /// Enregistre les serveurs MCP édités ; bloque si invalide (éditeur maintenu ouvert).
    pub fn mcp_save(&mut self) {
        if let Some(e) = self.mcp_editor.as_ref() {
            if let Some(err) = e.validation_error() {
                self.status = Some(format!("Enregistrement bloqué : {err}"));
                return;
            }
        }
        let Some(editor) = self.mcp_editor.take() else {
            return;
        };
        let servers = editor.into_servers();
        match write_user_mcp_servers(self.home(), &servers) {
            Ok(()) => {
                self.reload_files();
                self.status = Some("Serveurs MCP enregistrés".to_string());
            }
            Err(e) => self.status = Some(format!("Échec enregistrement MCP : {e}")),
        }
    }
```

Dans `crates/claudine/src/tui/mod.rs`, capture de la modale (avant le `match` principal, à côté de `hooks_editor`) :

```rust
    // Éditeur MCP (modal).
    if app.mcp_editor.is_some() {
        handle_mcp_editor_key(app, key);
        return;
    }
```

Ouverture par `m` en section Extensions : dans le `match key.code` principal, ajouter :

```rust
        KeyCode::Char('m') => app.open_mcp_editor(),
```

> Note : `m` en section Browse est déjà « déplacer une session » ; cette nouvelle branche `KeyCode::Char('m')` ne doit PAS écraser la branche existante de Browse. Si une branche `KeyCode::Char('m') => app.request_move_session()` existe déjà dans le `match` principal, fusionne les deux comportements en une seule branche qui dispatche selon `app.section` :
> ```rust
>         KeyCode::Char('m') => {
>             if app.section == Section::Extensions {
>                 app.open_mcp_editor();
>             } else {
>                 app.request_move_session();
>             }
>         }
> ```
> (`request_move_session` ne fait déjà rien hors du panneau Sessions de Browse.)

Et la fonction de routage (près de `handle_hooks_editor_key`), en réutilisant le motif d'action différée pour éviter le conflit d'emprunt sur `app` :

```rust
fn handle_mcp_editor_key(app: &mut App, key: KeyEvent) {
    use crate::tui::mcp_editor::{McpLevel, McpRow};
    enum Deferred {
        Save,
        Cancel,
    }
    let deferred: Option<Deferred>;
    {
        let Some(e) = app.mcp_editor.as_mut() else {
            return;
        };
        if e.confirm_delete {
            match key.code {
                KeyCode::Char('o') | KeyCode::Char('O') | KeyCode::Char('y') | KeyCode::Char('Y')
                | KeyCode::Enter => e.apply_delete(),
                KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => e.confirm_delete = false,
                _ => {}
            }
            return;
        }
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
        deferred = match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                e.move_sel(-1);
                None
            }
            KeyCode::Down | KeyCode::Char('j') => {
                e.move_sel(1);
                None
            }
            KeyCode::Left => {
                if matches!(e.rows().get(e.field_idx), Some(McpRow::Type)) {
                    e.cycle_type(-1);
                }
                None
            }
            KeyCode::Right => {
                if matches!(e.rows().get(e.field_idx), Some(McpRow::Type)) {
                    e.cycle_type(1);
                }
                None
            }
            KeyCode::Char('a') => {
                match e.level {
                    McpLevel::Servers => e.add_server(),
                    McpLevel::Server => e.add_item(),
                }
                None
            }
            KeyCode::Char('d') => {
                e.delete_current();
                None
            }
            KeyCode::Char('s') => Some(Deferred::Save),
            KeyCode::Enter => {
                match e.level {
                    McpLevel::Servers => e.enter(),
                    McpLevel::Server => e.begin_edit(),
                }
                None
            }
            KeyCode::Esc => {
                if e.back() {
                    None
                } else {
                    Some(Deferred::Cancel)
                }
            }
            _ => None,
        };
    }
    match deferred {
        Some(Deferred::Save) => app.mcp_save(),
        Some(Deferred::Cancel) => app.mcp_cancel(),
        None => {}
    }
}
```

Dans `crates/claudine/src/tui/ui.rs` :

1. Import : `use crate::tui::mcp_editor::{McpEdit, McpEditor, McpLevel, McpRow};`.
2. Appel dans `render(...)`, après `render_hooks_editor` (et `render_plugins_toggle`) :

```rust
    if app.mcp_editor.is_some() {
        render_mcp_editor(app, f, area);
    }
```

3. La fonction de rendu :

```rust
/// Modal de l'éditeur de serveurs MCP.
fn render_mcp_editor(app: &App, f: &mut Frame, area: Rect) {
    let Some(e) = &app.mcp_editor else {
        return;
    };
    let popup = centered_rect(82, 72, area);
    f.render_widget(Clear, popup);
    let hint = match e.level {
        McpLevel::Servers => " a ajouter · Enter ouvrir · d suppr. · s enregistrer · Esc fermer ",
        McpLevel::Server => " ←/→ type · Enter éditer · a ajouter · d suppr. · s enregistrer · Esc retour ",
    };
    let block = Block::default()
        .borders(Borders::ALL)
        .title(Span::styled(
            " Éditeur de serveurs MCP ",
            Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
        ))
        .title(Line::from(hint).right_aligned());
    let inner = block.inner(popup);
    f.render_widget(block, popup);

    let mut lines: Vec<Line> = Vec::new();
    match e.level {
        McpLevel::Servers => {
            if e.servers.is_empty() {
                lines.push(Line::from(Span::styled(
                    "  (aucun serveur — 'a' pour en ajouter)",
                    Style::default().fg(DIM),
                )));
            }
            for (i, s) in e.servers.iter().enumerate() {
                let sel = i == e.server_idx;
                let t = match s.transport {
                    McpTransport::Stdio => "stdio",
                    McpTransport::Http => "http",
                    McpTransport::Sse => "sse",
                };
                let label = format!("{} {}  [{}]", if sel { "▶" } else { " " }, s.name, t);
                let style = if sel {
                    selection_style(true)
                } else {
                    Style::default()
                };
                lines.push(Line::from(Span::styled(label, style)));
            }
        }
        McpLevel::Server => {
            let Some(s) = e.servers.get(e.server_idx) else {
                return;
            };
            let rows = e.rows();
            for (i, row) in rows.iter().enumerate() {
                let sel = i == e.field_idx;
                let text = match *row {
                    McpRow::Name => format!("  Nom      : {}", s.name),
                    McpRow::Type => {
                        let t = match s.transport {
                            McpTransport::Stdio => "stdio",
                            McpTransport::Http => "http",
                            McpTransport::Sse => "sse",
                        };
                        format!("  Type     : {t}   (←/→)")
                    }
                    McpRow::Command => format!("  Command  : {}", s.command),
                    McpRow::Url => format!("  URL      : {}", s.url),
                    McpRow::Arg(i) => format!("    arg[{i}] : {}", s.args.get(i).cloned().unwrap_or_default()),
                    McpRow::Env(i) => {
                        let (k, v) = s.env.get(i).cloned().unwrap_or_default();
                        format!("    env     : {k}={v}")
                    }
                    McpRow::Header(i) => {
                        let (k, v) = s.headers.get(i).cloned().unwrap_or_default();
                        format!("    header  : {k}={v}")
                    }
                };
                let style = if sel {
                    selection_style(true)
                } else {
                    Style::default()
                };
                lines.push(Line::from(Span::styled(text, style)));
            }
        }
    }

    if let McpEdit::Text(buf) = &e.edit {
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

> `McpTransport` doit être importé dans `ui.rs` : il l'est déjà via `claudine_core` ? Sinon, ajoute `use claudine_core::McpTransport;` en tête de `ui.rs`.

- [ ] **Step 4: Run tests + clippy**

Run: `cargo test -p claudine mcp_editor_open_edit_and_save mcp_save_blocked && cargo clippy --workspace`
Expected: tests PASS, 0 warning.

- [ ] **Step 5: Commit**

```bash
git add crates/claudine/src/tui/app.rs crates/claudine/src/tui/mod.rs crates/claudine/src/tui/ui.rs
git commit -m "feat(tui): câblage de l'éditeur MCP (m depuis Extensions)"
```

---

### Task 6: Raccourcis (footer/aide) + vérification finale

**Files:**
- Modify: `crates/claudine/src/tui/ui.rs`

**Interfaces:**
- Consumes: tout ce qui précède. Produces: rien de nouveau.

- [ ] **Step 1: Mettre à jour le footer Extensions**

Dans `render_footer`, l'arm `Section::Extensions` — ajouter l'entrée `m`. Remplacer ses `key_hints` par :

```rust
        Section::Extensions => key_hints(&[
            ("Enter", "hooks"),
            ("p", "plugins"),
            ("m", "MCP"),
            ("↑/↓", "défiler"),
            ("t", "cible"),
            ("E", "settings"),
            ("?", "aide"),
        ]),
```

- [ ] **Step 2: Mettre à jour l'aide**

Dans `render_help`, remplacer la ligne `("Extensions", …)` par :

```rust
        ("Extensions", "hooks (Enter) · plugins (p) · serveurs MCP (m) — éditables ; E édite settings.json"),
```

- [ ] **Step 3: Vérification complète**

Run: `cargo clippy --workspace 2>&1 | grep -cE "warning|error"` → attendu `0`
Run: `cargo test --workspace` → tous les paquets `ok`.

- [ ] **Step 4: Commit**

```bash
git add crates/claudine/src/tui/ui.rs
git commit -m "feat(tui): raccourci MCP (m) dans Extensions + aide"
```

---

## Self-Review

**1. Couverture de la spec :**
- §3 résolution du fichier → `mcp_config_path` (Task 1). ✓
- §4 modèle (McpTransport/McpServer, sérialisation par transport) → Tasks 1-2. ✓
- §5 API cœur (mcp_config_path, read_user_mcp_servers, write_user_mcp_servers) → Tasks 1-2. ✓
- §6 éditeur MCP modal (navigation 2 niveaux, type ←/→, scalaires, args, env/headers en « clé=valeur ») → Tasks 3-5. ✓
- §7 raccourci `m` → Tasks 5-6 ; cohabitation avec `m`=déplacer en Browse traitée explicitement (Task 5). ✓
- §8 sûreté/validation (backup+atomique via SettingsDoc, préservation des autres clés, validation nom/command/url, confirmation suppression, home actif) → Tasks 2 (write), 4 (validation), 5 (save bloquant + confirmation). ✓
- §9 tests → présents dans chaque tâche. ✓

**2. Placeholders :** aucun TODO/TBD ; code complet fourni.

**3. Cohérence des types :** `McpServer`/`McpTransport` (champs `name`/`transport`/`command`/`args`/`env`/`url`/`headers`) identiques cœur (1-2) ↔ TUI (3-5). `McpEditor` méthodes (`new`, `rows`, `move_sel`, `add_server`, `delete_current`, `apply_delete`, `enter`, `back`, `cycle_type`, `add_item`, `begin_edit`, `editing`, `input_*`, `validation_error`, `into_servers`) cohérentes entre Tasks 3/4 et l'usage Task 5. `env`/`headers` édités via « clé=valeur » (`split_pair`) de bout en bout.
