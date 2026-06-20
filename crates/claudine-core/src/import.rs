use std::fs::File;
use std::io::Read;
use std::path::Path;

use flate2::read::GzDecoder;

use crate::error::{CoreError, Report, Result};
use crate::home::ClaudeHome;
use crate::manifest::{Manifest, SCHEMA_VERSION};
use crate::pathcodec::encode_cwd;
use crate::remap::RemapTable;

#[derive(Debug, Clone, Default)]
pub struct ImportOptions {
    pub overwrite: bool,
}

fn open_archive(bundle: &Path) -> Result<tar::Archive<GzDecoder<File>>> {
    let f = File::open(bundle).map_err(|e| CoreError::io(bundle, e))?;
    Ok(tar::Archive::new(GzDecoder::new(f)))
}

pub fn read_manifest(bundle: &Path) -> Result<Manifest> {
    let mut archive = open_archive(bundle)?;
    let entries = archive
        .entries()
        .map_err(|e| CoreError::io(bundle, e))?;
    for entry in entries {
        let mut entry = entry.map_err(|e| CoreError::io(bundle, e))?;
        let path = entry.path().map_err(|e| CoreError::io(bundle, e))?;
        if path.to_string_lossy() == "manifest.json" {
            let mut buf = String::new();
            entry
                .read_to_string(&mut buf)
                .map_err(|e| CoreError::io(bundle, e))?;
            let manifest: Manifest = serde_json::from_str(&buf).map_err(|e| {
                CoreError::JsonParse {
                    file: bundle.to_path_buf(),
                    line: 0,
                    source: e,
                }
            })?;
            if manifest.schema_version != SCHEMA_VERSION {
                return Err(CoreError::ManifestVersion(manifest.schema_version));
            }
            return Ok(manifest);
        }
    }
    Err(CoreError::BundleFormat("manifest.json absent".to_string()))
}

/// Calcule le nouveau cwd (via la table) et le nouveau nom de dossier encodé.
// Note: également utilisé par la tâche 10 (apply).
#[allow(dead_code)]
fn target_dir_name(old_cwd: Option<&str>, table: &RemapTable) -> Option<String> {
    let cwd = old_cwd?;
    let new_cwd = table.apply_to_path(cwd).unwrap_or_else(|| cwd.to_string());
    Some(encode_cwd(&new_cwd))
}

pub fn dry_run(
    bundle: &Path,
    target: &ClaudeHome,
    table: &RemapTable,
    _opts: &ImportOptions,
) -> Result<Report> {
    let manifest = read_manifest(bundle)?;
    let mut report = Report::default();
    for p in &manifest.projects {
        report.bump("projects", 1);
        let new_dir = target_dir_name(p.cwd.as_deref(), table)
            .unwrap_or_else(|| p.encoded_name.clone());
        for sid in &p.session_ids {
            let dest = target
                .projects_dir()
                .join(&new_dir)
                .join(format!("{sid}.jsonl"));
            if dest.exists() {
                report.bump("sessions_conflict", 1);
            } else {
                report.bump("sessions_new", 1);
            }
        }
        if p.cwd.is_some() && table.apply_to_path(p.cwd.as_deref().unwrap()).is_some() {
            report.bump("path_rewrites_planned", p.session_ids.len());
        }
    }
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::export::{export, ExportOptions};
    use crate::home::ClaudeHome;
    use crate::remap::{RemapRule, RemapTable};
    use crate::testkit::FakeHome;

    fn make_bundle() -> (FakeHome, std::path::PathBuf) {
        let src = FakeHome::new();
        src.add_session(
            "-home-old-proj",
            "abc",
            &[r#"{"cwd":"/home/old/proj","timestamp":"t"}"#],
        );
        let out = src.base().join("bundle.tar.gz");
        export(
            &ClaudeHome::from_base(src.base()),
            &out,
            &ExportOptions::default(),
        )
        .unwrap();
        (src, out)
    }

    fn table() -> RemapTable {
        RemapTable::new(vec![RemapRule {
            from: "/home/old".into(),
            to: "/home/new".into(),
        }])
    }

    #[test]
    fn read_manifest_returns_projects() {
        let (_src, bundle) = make_bundle();
        let m = read_manifest(&bundle).unwrap();
        assert_eq!(m.schema_version, crate::manifest::SCHEMA_VERSION);
        assert_eq!(m.projects.len(), 1);
        assert_eq!(m.projects[0].cwd.as_deref(), Some("/home/old/proj"));
    }

    #[test]
    fn dry_run_counts_new_sessions_without_writing() {
        let (_src, bundle) = make_bundle();
        let target = FakeHome::new();
        let home = ClaudeHome::from_base(target.base());

        let report = dry_run(&bundle, &home, &table(), &ImportOptions::default()).unwrap();

        assert_eq!(report.count("projects"), 1);
        assert_eq!(report.count("sessions_new"), 1);
        assert_eq!(report.count("sessions_conflict"), 0);
        // rien n'a été écrit dans la cible
        assert!(!home.projects_dir().join("-home-new-proj").exists());
    }
}
