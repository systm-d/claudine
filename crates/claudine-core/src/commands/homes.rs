use std::path::{Path, PathBuf};

use crate::{ClaudeHome, ClaudineConfig, discover_homes, scan_projects};

/// Résout l'argument `--home` : étiquette d'une home découverte, ou chemin de
/// système de fichiers. `None` retombe sur `ClaudeHome::discover()`.
pub fn resolve_home(home_arg: Option<&str>) -> Result<ClaudeHome, String> {
    let Some(value) = home_arg else {
        return ClaudeHome::discover().map_err(|e| e.to_string());
    };

    let homes = discover_homes();
    if let Some(home) = homes.iter().find(|h| h.label == value) {
        return Ok(home.clone());
    }

    let path = Path::new(value);
    if path.is_dir() {
        return Ok(ClaudeHome::from_base(path));
    }

    let labels: Vec<&str> = homes.iter().map(|h| h.label.as_str()).collect();
    Err(format!(
        "home introuvable : « {value} » n'est ni une étiquette connue ni un répertoire existant.\nHomes disponibles : {}",
        if labels.is_empty() {
            "(aucune)".to_string()
        } else {
            labels.join(", ")
        }
    ))
}

pub fn run_homes() -> Result<(), String> {
    let homes = discover_homes();
    if homes.is_empty() {
        println!("Aucune home Claude découverte.");
        return Ok(());
    }
    for (i, home) in homes.iter().enumerate() {
        let n = scan_projects(home).map(|p| p.len()).unwrap_or(0);
        let mark = if i == 0 { "*" } else { " " };
        println!(
            "{mark} {}  {}  ({n} projets)",
            home.label,
            home.base.display()
        );
    }
    Ok(())
}

/// Enregistre une home dans la config Claudine puis sauvegarde.
pub fn run_homes_add(path: PathBuf, label: Option<String>) -> Result<(), String> {
    if !path.is_dir() {
        return Err(format!(
            "le chemin « {} » n'est pas un répertoire existant",
            path.display()
        ));
    }
    let mut config = ClaudineConfig::load();
    config.add_home(label.unwrap_or_default(), path.clone());
    config.save().map_err(|e| e.to_string())?;
    println!("Home enregistrée : {}", path.display());
    Ok(())
}

/// Retire une home enregistrée de la config Claudine puis sauvegarde.
pub fn run_homes_remove(label: String) -> Result<(), String> {
    let mut config = ClaudineConfig::load();
    let before = config.homes.len();
    config.remove_home(&label);
    if config.homes.len() == before {
        return Err(format!(
            "aucune home enregistrée nommée « {label} » (seules les homes enregistrées sont retirables)"
        ));
    }
    config.save().map_err(|e| e.to_string())?;
    println!("Home retirée : {label}");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_home_accepts_existing_path() {
        let dir = tempfile::tempdir().unwrap();
        let home = resolve_home(Some(dir.path().to_str().unwrap())).unwrap();
        assert_eq!(home.base, dir.path());
    }

    #[test]
    fn resolve_home_rejects_unknown_value() {
        let err = resolve_home(Some("/n/existe/pas/du/tout-claudine")).unwrap_err();
        assert!(err.contains("home introuvable"));
    }
}
