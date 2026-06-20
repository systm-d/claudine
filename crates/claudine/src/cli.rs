use std::path::{Path, PathBuf};

use claudine_core::{
    apply, discover_homes, dry_run, export, scan_projects, ClaudeHome, ClaudineConfig,
    ExportOptions, ImportOptions, RemapRule, RemapTable, Report,
};

pub fn parse_maps(maps: &[String]) -> Result<RemapTable, String> {
    let mut rules = Vec::new();
    for m in maps {
        let (from, to) = m
            .split_once('=')
            .ok_or_else(|| format!("--map invalide (attendu ANCIEN=NOUVEAU): {m}"))?;
        rules.push(RemapRule {
            from: from.to_string(),
            to: to.to_string(),
        });
    }
    Ok(RemapTable::new(rules))
}

pub fn format_report(report: &Report) -> String {
    let mut out = String::from("Rapport :\n");
    for (k, v) in &report.counts {
        out.push_str(&format!("  {k}: {v}\n"));
    }
    if !report.warnings.is_empty() {
        out.push_str(&format!("Avertissements ({}):\n", report.warnings.len()));
        for w in &report.warnings {
            out.push_str(&format!("  - {w}\n"));
        }
    }
    out
}

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
    // La première est le défaut (cf. tri de discover_homes_in).
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

pub fn run_export(out: PathBuf, no_history: bool, home_arg: Option<String>) -> Result<(), String> {
    let home = resolve_home(home_arg.as_deref())?;
    let opts = ExportOptions {
        include_history: !no_history,
    };
    let report = export(&home, &out, &opts).map_err(|e| e.to_string())?;
    print!("{}", format_report(&report));
    println!("Bundle écrit : {}", out.display());
    Ok(())
}

pub fn run_import(
    bundle: PathBuf,
    maps: Vec<String>,
    dry_run_only: bool,
    overwrite: bool,
    home_arg: Option<String>,
) -> Result<(), String> {
    let home = resolve_home(home_arg.as_deref())?;
    let table = parse_maps(&maps)?;
    let opts = ImportOptions { overwrite };
    let report = if dry_run_only {
        dry_run(&bundle, &home, &table, &opts).map_err(|e| e.to_string())?
    } else {
        apply(&bundle, &home, &table, &opts).map_err(|e| e.to_string())?
    };
    print!("{}", format_report(&report));
    if dry_run_only {
        println!("(dry-run : rien n'a été écrit)");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_maps_ok() {
        let t = parse_maps(&["/home/old=/home/new".to_string()]).unwrap();
        assert_eq!(t.rules.len(), 1);
        assert_eq!(t.rules[0].from, "/home/old");
        assert_eq!(t.rules[0].to, "/home/new");
    }

    #[test]
    fn parse_maps_rejects_missing_equals() {
        assert!(parse_maps(&["noeq".to_string()]).is_err());
    }

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
