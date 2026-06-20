use std::fs;
use std::path::Path;

use serde_json::Value;

use crate::error::{CoreError, Result};
use crate::home::ClaudeHome;
use crate::model::{Project, SessionMeta};

pub fn scan_projects(home: &ClaudeHome) -> Result<Vec<Project>> {
    let dir = home.projects_dir();
    let mut projects = Vec::new();
    if !dir.exists() {
        return Ok(projects);
    }
    let entries = fs::read_dir(&dir).map_err(|e| CoreError::io(&dir, e))?;
    for entry in entries {
        let entry = entry.map_err(|e| CoreError::io(&dir, e))?;
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let encoded_name = entry.file_name().to_string_lossy().into_owned();
        let mut sessions = Vec::new();
        let session_entries =
            fs::read_dir(&path).map_err(|e| CoreError::io(&path, e))?;
        for s in session_entries {
            let s = s.map_err(|e| CoreError::io(&path, e))?;
            let sp = s.path();
            if sp.extension().and_then(|e| e.to_str()) == Some("jsonl") {
                sessions.push(read_session_meta(&sp)?);
            }
        }
        sessions.sort_by(|a, b| a.id.cmp(&b.id));
        // cwd réel : depuis les sessions ; sinon, tentative de résolution du
        // chemin via sondage du système de fichiers (sinon on gardera le nom encodé).
        let cwd = sessions
            .iter()
            .find_map(|s| s.cwd.clone())
            .or_else(|| crate::pathcodec::decode_encoded_to_path(&encoded_name));
        projects.push(Project {
            encoded_name,
            cwd,
            sessions,
        });
    }
    projects.sort_by(|a, b| a.encoded_name.cmp(&b.encoded_name));
    Ok(projects)
}

pub fn read_session_meta(path: &Path) -> Result<SessionMeta> {
    let content = fs::read_to_string(path).map_err(|e| CoreError::io(path, e))?;
    let size = content.len() as u64;
    let id = path
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();

    let mut message_count = 0usize;
    let mut cwd: Option<String> = None;
    let mut first_ts: Option<String> = None;
    let mut last_ts: Option<String> = None;

    for line in content.lines() {
        if line.trim().is_empty() {
            continue;
        }
        message_count += 1;
        if let Ok(v) = serde_json::from_str::<Value>(line) {
            if cwd.is_none() {
                if let Some(c) = v.get("cwd").and_then(|c| c.as_str()) {
                    cwd = Some(c.to_string());
                }
            }
            if let Some(ts) = v.get("timestamp").and_then(|t| t.as_str()) {
                if first_ts.is_none() {
                    first_ts = Some(ts.to_string());
                }
                last_ts = Some(ts.to_string());
            }
        }
    }

    Ok(SessionMeta {
        id,
        path: path.to_path_buf(),
        cwd,
        message_count,
        first_ts,
        last_ts,
        size,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::home::ClaudeHome;
    use crate::testkit::FakeHome;

    #[test]
    fn scans_projects_and_extracts_cwd() {
        let fake = FakeHome::new();
        fake.add_session(
            "-home-old-proj",
            "11111111-1111-1111-1111-111111111111",
            &[
                r#"{"type":"user","cwd":"/home/old/proj","timestamp":"2026-01-01T10:00:00Z"}"#,
                r#"{"type":"assistant","timestamp":"2026-01-01T10:01:00Z"}"#,
            ],
        );
        let home = ClaudeHome::from_base(fake.base());

        let projects = scan_projects(&home).unwrap();

        assert_eq!(projects.len(), 1);
        let p = &projects[0];
        assert_eq!(p.encoded_name, "-home-old-proj");
        assert_eq!(p.cwd.as_deref(), Some("/home/old/proj"));
        assert_eq!(p.sessions.len(), 1);
        let s = &p.sessions[0];
        assert_eq!(s.id, "11111111-1111-1111-1111-111111111111");
        assert_eq!(s.message_count, 2);
        assert_eq!(s.first_ts.as_deref(), Some("2026-01-01T10:00:00Z"));
        assert_eq!(s.last_ts.as_deref(), Some("2026-01-01T10:01:00Z"));
        assert_eq!(s.cwd.as_deref(), Some("/home/old/proj"));
    }

    #[test]
    fn corrupt_line_is_not_fatal() {
        let fake = FakeHome::new();
        fake.add_session(
            "-a",
            "22222222-2222-2222-2222-222222222222",
            &["pas du json", r#"{"cwd":"/a","timestamp":"t"}"#],
        );
        let home = ClaudeHome::from_base(fake.base());
        let projects = scan_projects(&home).unwrap();
        assert_eq!(projects[0].sessions[0].message_count, 2);
        assert_eq!(projects[0].sessions[0].cwd.as_deref(), Some("/a"));
    }
}
