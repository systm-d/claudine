use std::fs::File;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use flate2::write::GzEncoder;
use flate2::Compression;

use crate::error::{CoreError, Report, Result};
use crate::home::ClaudeHome;
use crate::manifest::{Manifest, ManifestProject, SCHEMA_VERSION};
use crate::scan::scan_projects;

pub const EXCLUDED: &[&str] = &[
    ".credentials.json",
    "security_warnings_state_*",
    "cache/",
    "shell-snapshots/",
    "session-env/",
    "telemetry/",
    ".claude.json",
];

#[derive(Debug, Clone)]
pub struct ExportOptions {
    pub include_history: bool,
}

impl Default for ExportOptions {
    fn default() -> Self {
        Self {
            include_history: true,
        }
    }
}

pub fn export(home: &ClaudeHome, output: &Path, opts: &ExportOptions) -> Result<Report> {
    let mut report = Report::default();
    let projects = scan_projects(home)?;

    let mut included = vec!["sessions".to_string()];

    let file = File::create(output).map_err(|e| CoreError::io(output, e))?;
    let enc = GzEncoder::new(file, Compression::default());
    let mut builder = tar::Builder::new(enc);

    // Sessions (dossier projects/ entier : aucun secret à l'intérieur).
    let projects_dir = home.projects_dir();
    if projects_dir.exists() {
        builder
            .append_dir_all("projects", &projects_dir)
            .map_err(|e| CoreError::io(&projects_dir, e))?;
    }
    for p in &projects {
        report.bump("projects", 1);
        report.bump("sessions", p.sessions.len());
    }

    // Todos.
    let todos_dir = home.todos_dir();
    if todos_dir.exists() {
        builder
            .append_dir_all("todos", &todos_dir)
            .map_err(|e| CoreError::io(&todos_dir, e))?;
        included.push("todos".to_string());
    }

    // Mémoire user.
    let memory = home.memory_file();
    if memory.exists() {
        builder
            .append_path_with_name(&memory, "memory/CLAUDE.md")
            .map_err(|e| CoreError::io(&memory, e))?;
        included.push("memory".to_string());
    }

    // Config (verbatim, phase 1 : settings + settings.local seulement).
    for (src, name) in [
        (home.settings_file(), "config/settings.json"),
        (home.settings_local_file(), "config/settings.local.json"),
    ] {
        if src.exists() {
            builder
                .append_path_with_name(&src, name)
                .map_err(|e| CoreError::io(&src, e))?;
            if !included.contains(&"config".to_string()) {
                included.push("config".to_string());
            }
        }
    }

    // Plugins/skills/agents.
    let plugins_dir = home.plugins_dir();
    if plugins_dir.exists() {
        builder
            .append_dir_all("plugins", &plugins_dir)
            .map_err(|e| CoreError::io(&plugins_dir, e))?;
        included.push("plugins".to_string());
    }

    // Historique (optionnel).
    if opts.include_history {
        let hist = home.history_file();
        if hist.exists() {
            builder
                .append_path_with_name(&hist, "history.jsonl")
                .map_err(|e| CoreError::io(&hist, e))?;
            included.push("history".to_string());
        }
    }

    // Manifest.
    let created_at = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs().to_string())
        .unwrap_or_else(|_| "0".to_string());
    let manifest = Manifest {
        schema_version: SCHEMA_VERSION,
        created_at,
        source_hostname: std::env::var("HOSTNAME").unwrap_or_else(|_| "unknown".to_string()),
        source_home: home.base.to_string_lossy().into_owned(),
        projects: projects
            .iter()
            .map(|p| ManifestProject {
                encoded_name: p.encoded_name.clone(),
                cwd: p.cwd.clone(),
                session_ids: p.sessions.iter().map(|s| s.id.clone()).collect(),
            })
            .collect(),
        included_categories: included,
        excluded: EXCLUDED.iter().map(|s| s.to_string()).collect(),
    };
    let manifest_bytes = serde_json::to_vec_pretty(&manifest)
        .map_err(|e| CoreError::JsonParse {
            file: output.to_path_buf(),
            line: 0,
            source: e,
        })?;
    let mut header = tar::Header::new_gnu();
    header.set_size(manifest_bytes.len() as u64);
    header.set_mode(0o644);
    header.set_cksum();
    builder
        .append_data(&mut header, "manifest.json", manifest_bytes.as_slice())
        .map_err(|e| CoreError::io(output, e))?;

    let enc = builder.into_inner().map_err(|e| CoreError::io(output, e))?;
    enc.finish().map_err(|e| CoreError::io(output, e))?;

    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::home::ClaudeHome;
    use crate::testkit::FakeHome;
    use flate2::read::GzDecoder;
    use std::collections::BTreeSet;

    fn archive_entries(path: &std::path::Path) -> BTreeSet<String> {
        let f = std::fs::File::open(path).unwrap();
        let mut ar = tar::Archive::new(GzDecoder::new(f));
        ar.entries()
            .unwrap()
            .map(|e| e.unwrap().path().unwrap().to_string_lossy().into_owned())
            .collect()
    }

    #[test]
    fn export_contains_sessions_and_manifest_excludes_secrets() {
        let fake = FakeHome::new();
        fake.add_session(
            "-home-old-proj",
            "abc",
            &[r#"{"cwd":"/home/old/proj","timestamp":"t"}"#],
        );
        fake.write_file("CLAUDE.md", "# mémoire");
        fake.write_file("settings.json", "{}");
        fake.write_file(".credentials.json", "SECRET");
        let home = ClaudeHome::from_base(fake.base());

        let out = fake.base().join("bundle.tar.gz");
        let report = export(&home, &out, &ExportOptions::default()).unwrap();

        assert!(out.exists());
        let entries = archive_entries(&out);
        assert!(entries.contains("manifest.json"));
        assert!(entries.contains("projects/-home-old-proj/abc.jsonl"));
        assert!(entries.contains("memory/CLAUDE.md"));
        assert!(entries.contains("config/settings.json"));
        // secret jamais embarqué
        assert!(!entries.iter().any(|e| e.contains("credentials")));
        assert_eq!(report.count("projects"), 1);
        assert_eq!(report.count("sessions"), 1);
    }
}
