//! Éditeur de hooks dédié (modal) : navigation hiérarchique
//! évènement → groupe → commandes, et édition des champs.

#![allow(dead_code)]

use claudine_core::HookGroup;

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
