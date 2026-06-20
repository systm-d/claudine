use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use flate2::read::GzDecoder;

use crate::error::{CoreError, Report, Result};
use crate::home::ClaudeHome;
use crate::manifest::{Manifest, SCHEMA_VERSION};
use crate::pathcodec::encode_cwd;
use crate::remap::{rewrite_jsonl_line, RemapTable};

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

/// Vrai si tous les composants du chemin sont `Normal` (pas de `..`, racine,
/// préfixe ni `.`) — garde contre le tar-slip avant de construire une destination.
fn entry_is_path_safe(path: &Path) -> bool {
    path.components()
        .all(|c| matches!(c, std::path::Component::Normal(_)))
}

/// Calcule le nouveau cwd (via la table) et le nouveau nom de dossier encodé.
// Note: également utilisé par la tâche 10 (apply).
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
        if let Some(cwd) = p.cwd.as_deref() {
            if table.apply_to_path(cwd).is_some() {
                report.bump("path_rewrites_planned", p.session_ids.len());
            }
        }
    }
    Ok(report)
}

fn backup_existing(target: &ClaudeHome) -> Result<Option<std::path::PathBuf>> {
    let projects = target.projects_dir();
    if !projects.exists() {
        return Ok(None);
    }
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let backup_root = target.base.join("backups").join(format!("pre-import-{ts}"));
    let dest = backup_root.join("projects");
    copy_dir_all(&projects, &dest)?;
    Ok(Some(backup_root))
}

fn copy_dir_all(src: &Path, dst: &Path) -> Result<()> {
    std::fs::create_dir_all(dst).map_err(|e| CoreError::io(dst, e))?;
    for entry in std::fs::read_dir(src).map_err(|e| CoreError::io(src, e))? {
        let entry = entry.map_err(|e| CoreError::io(src, e))?;
        let path = entry.path();
        let target = dst.join(entry.file_name());
        if path.is_dir() {
            copy_dir_all(&path, &target)?;
        } else {
            std::fs::copy(&path, &target).map_err(|e| CoreError::io(&path, e))?;
        }
    }
    Ok(())
}

/// Réécrit le contenu d'une session ligne par ligne ; préserve les lignes
/// non parsables (verbatim) et compte les réécritures.
fn rewrite_session(content: &str, table: &RemapTable, report: &mut Report, context: &str) -> String {
    let mut out_lines = Vec::new();
    for line in content.lines() {
        if line.trim().is_empty() {
            out_lines.push(String::new());
            continue;
        }
        match rewrite_jsonl_line(line, table) {
            Ok((rewritten, n)) => {
                if n > 0 {
                    report.bump("lines_rewritten", 1);
                } else {
                    report.bump("lines_preserved", 1);
                }
                out_lines.push(rewritten);
            }
            Err(_) => {
                report.warn(format!("ligne non parsable préservée verbatim dans {context}"));
                report.bump("lines_preserved", 1);
                out_lines.push(line.to_string());
            }
        }
    }
    let mut result = out_lines.join("\n");
    if content.ends_with('\n') {
        result.push('\n');
    }
    result
}

pub fn apply(
    bundle: &Path,
    target: &ClaudeHome,
    table: &RemapTable,
    opts: &ImportOptions,
) -> Result<Report> {
    let manifest = read_manifest(bundle)?;
    let mut report = Report::default();

    // 1. Backup avant toute mutation.
    backup_existing(target)?;

    // 2. Index cwd source -> nouveau dossier encodé (depuis le manifest).
    let dir_for = |encoded: &str| -> String {
        manifest
            .projects
            .iter()
            .find(|p| p.encoded_name == encoded)
            .and_then(|p| target_dir_name(p.cwd.as_deref(), table))
            .unwrap_or_else(|| encoded.to_string())
    };

    // 3. Parcourt les entrées `projects/<encoded>/<id>.jsonl` du bundle.
    let mut archive = open_archive(bundle)?;
    let entries = archive.entries().map_err(|e| CoreError::io(bundle, e))?;
    for entry in entries {
        let mut entry = entry.map_err(|e| CoreError::io(bundle, e))?;
        let path = entry.path().map_err(|e| CoreError::io(bundle, e))?.into_owned();
        // Ignore les entrées non régulières (symlinks/hardlinks/dirs).
        if !entry.header().entry_type().is_file() {
            continue;
        }
        // Garde anti tar-slip : rejette tout chemin contenant un composant non `Normal`.
        if !entry_is_path_safe(&path) {
            report.warn(format!(
                "entrée d'archive ignorée (chemin non sûr): {}",
                path.to_string_lossy()
            ));
            report.bump("entries_rejected", 1);
            continue;
        }
        let comps: Vec<String> = path
            .components()
            .map(|c| c.as_os_str().to_string_lossy().into_owned())
            .collect();
        if comps.len() != 3 || comps[0] != "projects" {
            continue;
        }
        let encoded = &comps[1];
        let filename = &comps[2];
        let new_dir = dir_for(encoded);
        let dest_dir = target.projects_dir().join(&new_dir);
        let dest = dest_dir.join(filename);

        if dest.exists() && !opts.overwrite {
            report.bump("sessions_skipped", 1);
            continue;
        }

        let mut content = String::new();
        entry
            .read_to_string(&mut content)
            .map_err(|e| CoreError::io(&dest, e))?;
        let rewritten = rewrite_session(&content, table, &mut report, &path.to_string_lossy());

        std::fs::create_dir_all(&dest_dir).map_err(|e| CoreError::io(&dest_dir, e))?;
        // Écriture temp + rename.
        let tmp = dest_dir.join(format!("{filename}.tmp"));
        std::fs::write(&tmp, rewritten.as_bytes()).map_err(|e| CoreError::io(&tmp, e))?;
        std::fs::rename(&tmp, &dest).map_err(|e| CoreError::io(&dest, e))?;
        report.bump("sessions_imported", 1);
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

    #[test]
    fn apply_remaps_paths_into_new_project_dir() {
        let (_src, bundle) = make_bundle();
        let target = FakeHome::new();
        let home = ClaudeHome::from_base(target.base());

        let report = apply(&bundle, &home, &table(), &ImportOptions::default()).unwrap();

        // La session est arrivée dans le dossier ré-encodé du nouveau cwd.
        let dest = home
            .projects_dir()
            .join("-home-new-proj")
            .join("abc.jsonl");
        assert!(dest.exists(), "session manquante: {dest:?}");

        // Le champ cwd interne a été remappé.
        let content = std::fs::read_to_string(&dest).unwrap();
        let v: serde_json::Value =
            serde_json::from_str(content.lines().next().unwrap()).unwrap();
        assert_eq!(v["cwd"], "/home/new/proj");

        assert_eq!(report.count("sessions_imported"), 1);
        assert_eq!(report.count("lines_rewritten"), 1);
    }

    #[test]
    fn apply_skips_existing_session_by_default() {
        let (_src, bundle) = make_bundle();
        let target = FakeHome::new();
        let home = ClaudeHome::from_base(target.base());
        // Pré-place une session en conflit (même chemin de destination).
        let pdir = home.projects_dir().join("-home-new-proj");
        std::fs::create_dir_all(&pdir).unwrap();
        std::fs::write(pdir.join("abc.jsonl"), "DÉJÀ LÀ").unwrap();

        let report = apply(&bundle, &home, &table(), &ImportOptions::default()).unwrap();

        assert_eq!(report.count("sessions_skipped"), 1);
        assert_eq!(report.count("sessions_imported"), 0);
        // Contenu d'origine préservé.
        assert_eq!(
            std::fs::read_to_string(pdir.join("abc.jsonl")).unwrap(),
            "DÉJÀ LÀ"
        );
        // Un backup a été créé.
        assert!(home.base.join("backups").exists());
        // Le backup contient l'état pré-import : le fichier en conflit avec son contenu d'origine.
        let backups_dir = home.base.join("backups");
        let backup_sub = std::fs::read_dir(&backups_dir)
            .unwrap()
            .next()
            .unwrap()
            .unwrap()
            .path();
        let backed_up = backup_sub.join("projects/-home-new-proj/abc.jsonl");
        assert_eq!(std::fs::read_to_string(&backed_up).unwrap(), "DÉJÀ LÀ");
    }

    #[test]
    fn entry_is_path_safe_rejects_traversal() {
        assert!(entry_is_path_safe(std::path::Path::new(
            "projects/-home-new-proj/abc.jsonl"
        )));
        assert!(!entry_is_path_safe(std::path::Path::new(
            "projects/../escaped.jsonl"
        )));
        assert!(!entry_is_path_safe(std::path::Path::new("/etc/passwd")));
        assert!(!entry_is_path_safe(std::path::Path::new(
            "projects/foo/../../x"
        )));
    }

    #[test]
    fn rewrite_session_preserves_trailing_newline() {
        let table = RemapTable::default();
        let mut report = Report::default();
        let with_nl = rewrite_session("{\"a\":1}\n", &table, &mut report, "t");
        assert!(with_nl.ends_with('\n'), "doit conserver le newline final");
        let without_nl = rewrite_session("{\"a\":1}", &table, &mut report, "t");
        assert!(!without_nl.ends_with('\n'), "ne doit pas en ajouter si absent");
    }
}
