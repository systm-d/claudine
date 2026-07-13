//! Configuration persistée de Claudine : liste des homes Claude *enregistrées*
//! par l'utilisateur, afin d'inclure des homes que le scan automatique ne
//! trouverait pas.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::{CoreError, Result};
use crate::home::ClaudeHome;

/// Une home enregistrée explicitement par l'utilisateur dans la config.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct RegisteredHome {
    pub label: String,
    pub path: PathBuf,
}

/// Configuration persistée de Claudine.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ClaudineConfig {
    #[serde(default)]
    pub homes: Vec<RegisteredHome>,
}

/// Chemin du fichier de configuration :
/// `$XDG_CONFIG_HOME/claudine/config.json` si `XDG_CONFIG_HOME` est défini et
/// non vide, sinon `$HOME/.config/claudine/config.json`.
pub fn config_path() -> PathBuf {
    config_path_from(
        std::env::var("XDG_CONFIG_HOME").ok().as_deref(),
        std::env::var("HOME").ok().as_deref(),
    )
}

/// Résolution pure du chemin de config à partir des valeurs d'environnement
/// (testable sans muter l'environnement global du process).
fn config_path_from(xdg_config_home: Option<&str>, home: Option<&str>) -> PathBuf {
    let base = xdg_config_home
        .filter(|s| !s.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| Path::new(home.unwrap_or_default()).join(".config"));
    base.join("claudine").join("config.json")
}

/// Dérive une étiquette depuis le dernier composant du chemin, avec repli
/// `"claude"` si aucun composant exploitable.
fn label_from_path(path: &Path) -> String {
    path.file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "claude".to_string())
}

/// Canonicalise un chemin si possible, sinon le renvoie tel quel. Sert de clé
/// de déduplication tolérante aux chemins inexistants.
fn dedup_key(path: &Path) -> PathBuf {
    std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

impl ClaudineConfig {
    /// Charge la config depuis [`config_path`]. Ne panique jamais et ne renvoie
    /// jamais d'erreur : un fichier absent ou illisible retombe sur `Default`.
    pub fn load() -> ClaudineConfig {
        Self::load_from(&config_path())
    }

    /// Variante testable : charge depuis un chemin explicite. Fichier absent ou
    /// non parsable → `Default`.
    pub fn load_from(path: &Path) -> ClaudineConfig {
        match std::fs::read_to_string(path) {
            Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
            Err(_) => ClaudineConfig::default(),
        }
    }

    /// Sauvegarde la config dans [`config_path`] (JSON indenté), en créant le
    /// répertoire parent si besoin.
    pub fn save(&self) -> Result<()> {
        self.save_to(&config_path())
    }

    /// Variante testable : sauvegarde vers un chemin explicite.
    pub fn save_to(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| CoreError::io(parent, e))?;
        }
        let json = serde_json::to_string_pretty(self).map_err(|e| CoreError::JsonParse {
            file: path.to_path_buf(),
            line: 0,
            source: e,
        })?;
        std::fs::write(path, json).map_err(|e| CoreError::io(path, e))
    }

    /// Enregistre une home. Déduplique par chemin canonique : si la home existe
    /// déjà, l'étiquette est mise à jour (si une nouvelle est fournie). Une
    /// étiquette vide est dérivée du dernier composant du chemin.
    pub fn add_home(&mut self, label: impl Into<String>, path: impl Into<PathBuf>) {
        let path = path.into();
        let mut label = label.into();
        if label.is_empty() {
            label = label_from_path(&path);
        }

        let key = dedup_key(&path);
        if let Some(existing) = self.homes.iter_mut().find(|h| dedup_key(&h.path) == key) {
            existing.label = label;
            return;
        }
        self.homes.push(RegisteredHome { label, path });
    }

    /// Retire la home portant l'étiquette donnée.
    pub fn remove_home(&mut self, label: &str) {
        self.homes.retain(|h| h.label != label);
    }
}

/// Fusionne les homes découvertes avec les homes enregistrées : ajoute en queue
/// les homes enregistrées absentes (par chemin canonique). Les homes
/// enregistrées sont incluses même si elles ne passeraient pas l'heuristique de
/// scan automatique.
pub fn merge_registered(homes: Vec<ClaudeHome>, registered: &[RegisteredHome]) -> Vec<ClaudeHome> {
    let mut out = homes;
    let mut seen: HashSet<PathBuf> = out.iter().map(|h| dedup_key(&h.base)).collect();

    for reg in registered {
        let key = dedup_key(&reg.path);
        if seen.insert(key) {
            let label = if reg.label.is_empty() {
                label_from_path(&reg.path)
            } else {
                reg.label.clone()
            };
            out.push(ClaudeHome::new(reg.path.clone(), label));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn save_to_then_load_from_round_trips() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("sub").join("config.json");

        let mut cfg = ClaudineConfig::default();
        cfg.add_home("perso", "/home/u/.claude-perso");
        cfg.add_home("boulot", "/home/u/.claude-boulot");
        cfg.save_to(&path).unwrap();

        let loaded = ClaudineConfig::load_from(&path);
        assert_eq!(loaded, cfg);
        assert_eq!(loaded.homes.len(), 2);
    }

    #[test]
    fn load_from_missing_or_garbage_is_default() {
        let dir = tempfile::tempdir().unwrap();
        // Fichier absent.
        let missing = dir.path().join("nope.json");
        assert_eq!(
            ClaudineConfig::load_from(&missing),
            ClaudineConfig::default()
        );
        // Fichier non parsable.
        let garbage = dir.path().join("bad.json");
        std::fs::write(&garbage, "pas du json {").unwrap();
        assert_eq!(
            ClaudineConfig::load_from(&garbage),
            ClaudineConfig::default()
        );
    }

    #[test]
    fn add_home_dedups_by_path_and_derives_label() {
        let mut cfg = ClaudineConfig::default();
        // Étiquette vide → dérivée du dernier composant.
        cfg.add_home("", "/home/u/.claude-perso");
        assert_eq!(cfg.homes.len(), 1);
        assert_eq!(cfg.homes[0].label, ".claude-perso");

        // Même chemin → pas de doublon, l'étiquette est mise à jour.
        cfg.add_home("renommée", "/home/u/.claude-perso");
        assert_eq!(cfg.homes.len(), 1);
        assert_eq!(cfg.homes[0].label, "renommée");
    }

    #[test]
    fn remove_home_drops_by_label() {
        let mut cfg = ClaudineConfig::default();
        cfg.add_home("a", "/x/a");
        cfg.add_home("b", "/x/b");
        cfg.remove_home("a");
        assert_eq!(cfg.homes.len(), 1);
        assert_eq!(cfg.homes[0].label, "b");
    }

    #[test]
    fn merge_registered_appends_and_dedupes() {
        let dir = tempfile::tempdir().unwrap();
        let existing = dir.path().join(".claude");
        let extra = dir.path().join(".claude-extra");
        std::fs::create_dir_all(&existing).unwrap();
        std::fs::create_dir_all(&extra).unwrap();

        let scanned = vec![ClaudeHome::from_base(&existing)];
        let registered = vec![
            // Déjà présente (même chemin) → pas de doublon.
            RegisteredHome {
                label: "déjà".to_string(),
                path: existing.clone(),
            },
            // Nouvelle → ajoutée en queue.
            RegisteredHome {
                label: "extra".to_string(),
                path: extra.clone(),
            },
        ];

        let merged = merge_registered(scanned, &registered);
        assert_eq!(merged.len(), 2);
        assert_eq!(merged[0].base, existing);
        assert_eq!(merged[1].base, extra);
        assert_eq!(merged[1].label, "extra");
    }

    #[test]
    fn merge_registered_includes_nonqualifying_home() {
        // Un chemin inexistant (ne passerait jamais le scan) est inclus.
        let scanned: Vec<ClaudeHome> = Vec::new();
        let registered = vec![RegisteredHome {
            label: String::new(),
            path: PathBuf::from("/n/existe/pas/.claude-fantome"),
        }];
        let merged = merge_registered(scanned, &registered);
        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].label, ".claude-fantome");
    }

    #[test]
    fn config_path_prefers_xdg() {
        let p = config_path_from(Some("/tmp/xdg-claudine"), Some("/home/x"));
        assert_eq!(p, PathBuf::from("/tmp/xdg-claudine/claudine/config.json"));
    }
}
