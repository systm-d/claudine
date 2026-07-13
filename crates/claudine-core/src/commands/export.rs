use std::path::PathBuf;

use crate::commands::homes::resolve_home;
use crate::{ExportOptions, Report, export};

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
