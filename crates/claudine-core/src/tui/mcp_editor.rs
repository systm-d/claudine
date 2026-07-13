//! Éditeur de serveurs MCP dédié (modal) : navigation serveurs → serveur,
//! édition des champs selon le transport.

use crate::{McpServer, McpTransport};

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

#[derive(Debug)]
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
            (McpTransport::Stdio, Some(McpRow::Env(_))) => {
                s.env.push((String::new(), String::new()))
            }
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
                .map(|(k, v)| {
                    if k.is_empty() && v.is_empty() {
                        String::new()
                    } else {
                        format!("{k}={v}")
                    }
                })
                .unwrap_or_default(),
            Some(McpRow::Header(i)) => s
                .headers
                .get(i)
                .map(|(k, v)| {
                    if k.is_empty() && v.is_empty() {
                        String::new()
                    } else {
                        format!("{k}={v}")
                    }
                })
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

/// Découpe « clé=valeur » sur le premier `=` ; sans `=`, tout devient la clé.
fn split_pair(buf: &str) -> (String, String) {
    match buf.split_once('=') {
        Some((k, v)) => (k.trim().to_string(), v.to_string()),
        None => (buf.trim().to_string(), String::new()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{McpServer, McpTransport};

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
        assert_eq!(
            e.servers[0].env,
            vec![("TOKEN".to_string(), "abc".to_string())]
        );
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
}
