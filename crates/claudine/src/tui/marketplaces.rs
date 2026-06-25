//! Gestionnaire de marketplaces (modal) : liste, ajout (saisie de source),
//! retrait (confirmation), mise à jour. Les opérations réseau (ajout/màj) sont
//! exécutées en arrière-plan par `app.rs` ; ce module ne porte que l'état UI.

use claudine_core::Marketplace;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MktMode {
    List,
    AddInput,
}

#[derive(Debug)]
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
