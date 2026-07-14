use std::collections::HashSet;
use std::path::{Path, PathBuf};

use crate::error::{CoreError, Result};

#[derive(Debug, Clone)]
pub struct ClaudeHome {
    pub base: PathBuf,
    /// Étiquette lisible, dérivée du dernier composant du chemin (ex. `.claude-perso`).
    pub label: String,
}

impl ClaudeHome {
    /// Construit une home avec une étiquette explicite.
    pub fn new(base: impl Into<PathBuf>, label: impl Into<String>) -> Self {
        Self {
            base: base.into(),
            label: label.into(),
        }
    }

    /// Construit une home en dérivant l'étiquette du dernier composant du chemin
    /// (ex. `.claude-perso`), avec repli sur `"claude"` si absent.
    pub fn from_base(base: impl Into<PathBuf>) -> Self {
        let base = base.into();
        let label = label_from_base(&base);
        Self { base, label }
    }

    pub fn discover() -> Result<Self> {
        Self::discover_from(
            std::env::var("CLAUDE_CONFIG_DIR").ok().as_deref(),
            std::env::var("HOME").ok().as_deref(),
        )
    }

    /// Résolution pure de la home à partir des valeurs d'environnement
    /// (testable sans muter l'environnement global du process).
    fn discover_from(claude_config_dir: Option<&str>, home: Option<&str>) -> Result<Self> {
        if let Some(dir) = claude_config_dir {
            if !dir.is_empty() {
                return Ok(Self::from_base(dir));
            }
        }
        let home = home.ok_or_else(|| {
            CoreError::io(
                "<HOME>",
                std::io::Error::new(std::io::ErrorKind::NotFound, "variable HOME absente"),
            )
        })?;
        Ok(Self::from_base(Path::new(home).join(".claude")))
    }

    pub fn projects_dir(&self) -> PathBuf {
        self.base.join("projects")
    }
    pub fn todos_dir(&self) -> PathBuf {
        self.base.join("todos")
    }
    pub fn plugins_dir(&self) -> PathBuf {
        self.base.join("plugins")
    }
    pub fn memory_file(&self) -> PathBuf {
        self.base.join("CLAUDE.md")
    }
    pub fn settings_file(&self) -> PathBuf {
        self.base.join("settings.json")
    }
    pub fn settings_local_file(&self) -> PathBuf {
        self.base.join("settings.local.json")
    }
    pub fn history_file(&self) -> PathBuf {
        self.base.join("history.jsonl")
    }
}

/// Dérive l'étiquette depuis le dernier composant du chemin, avec repli `"claude"`.
fn label_from_base(base: &Path) -> String {
    base.file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "claude".to_string())
}

/// Canonicalise un chemin si possible, sinon renvoie le chemin tel quel.
/// Sert de clé de déduplication tolérante aux homes inexistantes.
fn dedup_key(path: &Path) -> PathBuf {
    std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

/// Une home « qualifie » si elle contient un `settings.json` OU un `projects/`
/// non vide.
fn qualifies(base: &Path) -> bool {
    if base.join("settings.json").is_file() {
        return true;
    }
    let projects = base.join("projects");
    match std::fs::read_dir(&projects) {
        Ok(mut entries) => entries.next().is_some(),
        Err(_) => false,
    }
}

/// Découverte testable de homes : scanne `home_dir` pour les répertoires dont le
/// nom commence par `.claude` et qui « qualifient », puis ajoute éventuellement
/// `config_dir` (toujours inclus s'il est fourni). Déduplique par chemin
/// canonique et trie : home par défaut en tête, le reste par étiquette croissante.
pub fn discover_homes_in(home_dir: &Path, config_dir: Option<&Path>) -> Vec<ClaudeHome> {
    let mut homes: Vec<ClaudeHome> = Vec::new();
    let mut seen: HashSet<PathBuf> = HashSet::new();

    // Étiquette de l'entrée `.claude` (exacte) si elle qualifie, sert de défaut.
    let mut default_key: Option<PathBuf> = None;

    if let Ok(entries) = std::fs::read_dir(home_dir) {
        let mut candidates: Vec<PathBuf> = Vec::new();
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            let name = entry.file_name().to_string_lossy().into_owned();
            if !name.starts_with(".claude") {
                continue;
            }
            candidates.push(path);
        }
        // Tri par nom pour un parcours déterministe.
        candidates.sort();
        for path in candidates {
            if !qualifies(&path) {
                continue;
            }
            let key = dedup_key(&path);
            if !seen.insert(key.clone()) {
                continue;
            }
            let home = ClaudeHome::from_base(&path);
            if home.label == ".claude" {
                default_key = Some(key);
            }
            homes.push(home);
        }
    }

    // `config_dir` est inclus inconditionnellement (l'utilisateur l'a choisi).
    let mut config_key: Option<PathBuf> = None;
    if let Some(cfg) = config_dir {
        let key = dedup_key(cfg);
        config_key = Some(key.clone());
        if seen.insert(key) {
            homes.push(ClaudeHome::from_base(cfg));
        }
    }

    // Choix du défaut : config_dir si fourni, sinon `.claude`, sinon le premier
    // par étiquette.
    let default_key = config_key.or(default_key);

    // Tri : défaut d'abord, puis par étiquette croissante.
    homes.sort_by(|a, b| {
        let a_default = default_key
            .as_ref()
            .is_some_and(|k| dedup_key(&a.base) == *k);
        let b_default = default_key
            .as_ref()
            .is_some_and(|k| dedup_key(&b.base) == *k);
        match (a_default, b_default) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a.label.cmp(&b.label),
        }
    });

    homes
}

/// Découverte réelle : lit `$HOME` et `$CLAUDE_CONFIG_DIR` puis délègue à
/// [`discover_homes_in`]. Garantit au moins une home si possible : à défaut de
/// résultat, retombe sur `$HOME/.claude` (même vide). Renvoie un vec vide si
/// `$HOME` est absent et qu'aucune home n'a pu être découverte.
pub fn discover_homes() -> Vec<ClaudeHome> {
    let config_dir = std::env::var("CLAUDE_CONFIG_DIR")
        .ok()
        .filter(|s| !s.is_empty())
        .map(PathBuf::from);

    let home_dir = std::env::var("HOME").ok().map(PathBuf::from);

    let mut homes = match &home_dir {
        Some(h) => discover_homes_in(h, config_dir.as_deref()),
        None => match &config_dir {
            // Pas de $HOME mais un config_dir : on l'utilise quand même.
            Some(cfg) => vec![ClaudeHome::from_base(cfg)],
            None => Vec::new(),
        },
    };

    if homes.is_empty() {
        if let Some(home) = config_dir
            .map(ClaudeHome::from_base)
            .or_else(|| home_dir.map(|h| ClaudeHome::from_base(h.join(".claude"))))
        {
            homes.push(home);
        }
    }

    // Ajoute les homes enregistrées dans la config Claudine, même si elles ne
    // passeraient pas l'heuristique de scan automatique.
    let config = crate::config::ClaudineConfig::load();
    crate::config::merge_registered(homes, &config.homes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn from_base_builds_subpaths() {
        let h = ClaudeHome::from_base("/x/.claude");
        assert_eq!(
            h.projects_dir(),
            std::path::Path::new("/x/.claude/projects")
        );
        assert_eq!(
            h.settings_file(),
            std::path::Path::new("/x/.claude/settings.json")
        );
        assert_eq!(
            h.history_file(),
            std::path::Path::new("/x/.claude/history.jsonl")
        );
    }

    #[test]
    fn from_base_derives_label() {
        assert_eq!(ClaudeHome::from_base("/x/.claude").label, ".claude");
        assert_eq!(
            ClaudeHome::from_base("/x/.claude-perso").label,
            ".claude-perso"
        );
        // Repli sur "claude" si pas de composant final exploitable.
        assert_eq!(ClaudeHome::from_base("/").label, "claude");
    }

    #[test]
    fn discover_respects_env() {
        let base = ClaudeHome::discover_from(Some("/custom/dir"), Some("/home/x"))
            .unwrap()
            .base;
        assert_eq!(base, std::path::Path::new("/custom/dir"));
    }

    /// Construit un faux `$HOME` avec 4 entrées : `.claude` (settings.json),
    /// `.claude-perso` (projects non vide), `.claude-sneakpeek` et `.claudettes`
    /// (vides, donc non qualifiantes).
    fn fake_home_dir() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();

        // .claude : qualifie via settings.json
        fs::create_dir_all(root.join(".claude")).unwrap();
        fs::write(root.join(".claude/settings.json"), "{}").unwrap();

        // .claude-perso : qualifie via projects/ non vide
        let pdir = root.join(".claude-perso/projects/-home-x");
        fs::create_dir_all(&pdir).unwrap();
        fs::write(pdir.join("x.jsonl"), "{}").unwrap();

        // .claude-sneakpeek : ni settings.json ni projects → exclue
        fs::create_dir_all(root.join(".claude-sneakpeek")).unwrap();

        // .claudettes : commence bien par .claude mais vide → exclue
        fs::create_dir_all(root.join(".claudettes")).unwrap();

        dir
    }

    #[test]
    fn discover_homes_in_keeps_only_qualifying() {
        let dir = fake_home_dir();
        let homes = discover_homes_in(dir.path(), None);

        let labels: Vec<&str> = homes.iter().map(|h| h.label.as_str()).collect();
        assert_eq!(labels, vec![".claude", ".claude-perso"]);
    }

    #[test]
    fn discover_homes_in_excludes_non_homes() {
        let dir = fake_home_dir();
        let homes = discover_homes_in(dir.path(), None);
        assert!(homes.iter().all(|h| h.label != ".claude-sneakpeek"));
        assert!(homes.iter().all(|h| h.label != ".claudettes"));
    }

    #[test]
    fn discover_homes_in_config_dir_becomes_default() {
        let dir = fake_home_dir();
        let perso = dir.path().join(".claude-perso");
        let homes = discover_homes_in(dir.path(), Some(&perso));

        // .claude-perso doit passer en tête (défaut), .claude ensuite.
        let labels: Vec<&str> = homes.iter().map(|h| h.label.as_str()).collect();
        assert_eq!(labels, vec![".claude-perso", ".claude"]);
    }

    #[test]
    fn discover_homes_in_config_dir_outside_scan_is_added() {
        let dir = fake_home_dir();
        // Un config_dir hors du scan (ne commence pas par .claude) est inclus.
        let extra = tempfile::tempdir().unwrap();
        fs::write(extra.path().join("settings.json"), "{}").unwrap();
        let homes = discover_homes_in(dir.path(), Some(extra.path()));

        // Le config_dir est le défaut → en tête.
        assert_eq!(homes.first().map(|h| h.base.as_path()), Some(extra.path()));
        // Les deux homes scannées sont toujours présentes.
        assert_eq!(homes.len(), 3);
    }

    #[test]
    fn discover_homes_in_empty_without_config_is_empty() {
        let dir = tempfile::tempdir().unwrap();
        assert!(discover_homes_in(dir.path(), None).is_empty());
    }
}
