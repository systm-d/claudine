#![allow(dead_code)]

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
