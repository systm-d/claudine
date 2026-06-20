use std::path::PathBuf;

use claudine_core::{
    apply, dry_run, export, ClaudeHome, ExportOptions, ImportOptions, RemapRule, RemapTable,
    Report,
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

pub fn run_export(out: PathBuf, no_history: bool) -> Result<(), String> {
    let home = ClaudeHome::discover().map_err(|e| e.to_string())?;
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
) -> Result<(), String> {
    let home = ClaudeHome::discover().map_err(|e| e.to_string())?;
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
