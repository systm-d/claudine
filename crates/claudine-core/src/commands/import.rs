use std::path::PathBuf;

use crate::commands::export::format_report;
use crate::commands::homes::resolve_home;
use crate::{ImportOptions, RemapRule, RemapTable, apply, dry_run};

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
}
