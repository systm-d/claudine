//! Gestionnaire de marketplaces (modal) : liste, ajout (saisie de source),
//! retrait (confirmation), mise à jour. Les opérations réseau (ajout/màj) sont
//! exécutées en arrière-plan par `app.rs` ; ce module ne porte que l'état UI.

use claudine_core::{Marketplace, PluginEntry, PluginManifestEntry};

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
    pub catalog: Option<PluginCatalog>,
}

impl MarketplacesManager {
    pub fn new(items: Vec<Marketplace>) -> Self {
        Self {
            items,
            idx: 0,
            mode: MktMode::List,
            input: String::new(),
            confirm_remove: false,
            catalog: None,
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

    /// Marque le plugin nommé comme installé + activé (après un job d'install réussi).
    pub fn mark_installed(&mut self, plugin: &str) {
        if let Some(e) = self.entries.iter_mut().find(|e| e.name == plugin) {
            e.installed = true;
            e.enabled = true;
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

    use claudine_core::{PluginEntry, PluginManifestEntry};

    fn pm(name: &str, desc: Option<&str>) -> PluginManifestEntry {
        PluginManifestEntry {
            name: name.into(),
            description: desc.map(|s| s.to_string()),
            source: None,
        }
    }

    #[test]
    fn catalog_new_marks_installed_and_enabled() {
        let manifest = vec![pm("a", Some("da")), pm("b", None), pm("c", None)];
        let installed = vec![
            PluginEntry {
                name: "a@m".into(),
                enabled: true,
                ..Default::default()
            },
            PluginEntry {
                name: "b@m".into(),
                enabled: false,
                ..Default::default()
            },
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
        let installed = vec![PluginEntry {
            name: "b@m".into(),
            enabled: false,
            ..Default::default()
        }];
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

    #[test]
    fn catalog_mark_installed_sets_flags() {
        let manifest = vec![pm("a", None), pm("b", None)];
        let installed = vec![]; // rien d'installé au départ
        let mut cat = PluginCatalog::new("m".into(), &manifest, &installed);
        assert!(!cat.entries[0].installed);
        cat.mark_installed("a");
        assert!(cat.entries[0].installed && cat.entries[0].enabled);
        // L'autre entrée reste intacte ; un nom inconnu est ignoré.
        assert!(!cat.entries[1].installed);
        cat.mark_installed("absent");
    }
}
