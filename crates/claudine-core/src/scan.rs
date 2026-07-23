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
        let session_entries = fs::read_dir(&path).map_err(|e| CoreError::io(&path, e))?;
        for s in session_entries {
            let s = s.map_err(|e| CoreError::io(&path, e))?;
            let sp = s.path();
            if sp.extension().and_then(|e| e.to_str()) == Some("jsonl") {
                sessions.push(read_session_meta(&sp)?);
            }
        }
        // Tri par date de création (premier timestamp) décroissante : les
        // sessions les plus récentes en tête. Les timestamps sont au format
        // RFC 3339 (UTC), donc l'ordre lexicographique est chronologique. Les
        // sessions sans timestamp passent en dernier ; `id` départage à égalité
        // pour un ordre stable.
        sessions.sort_by(|a, b| b.first_ts.cmp(&a.first_ts).then_with(|| a.id.cmp(&b.id)));
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
    // Titre de session : Claude Code écrit des lignes `{"type":"summary",
    // "summary":"…","leafUuid":"…"}` où `leafUuid` désigne le dernier message
    // que ce résumé décrit. On associe chaque résumé à son `leafUuid` et on
    // suit l'uuid du dernier message ; le titre retenu est celui dont le
    // `leafUuid` correspond à cette feuille (à défaut, le dernier vu).
    let mut summaries: Vec<(Option<String>, String)> = Vec::new();
    let mut last_uuid: Option<String> = None;

    for line in content.lines() {
        if line.trim().is_empty() {
            continue;
        }
        if let Ok(v) = serde_json::from_str::<Value>(line) {
            if v.get("type").and_then(|t| t.as_str()) == Some("summary") {
                if let Some(sum) = v.get("summary").and_then(|s| s.as_str()) {
                    let leaf = v
                        .get("leafUuid")
                        .and_then(|u| u.as_str())
                        .map(str::to_string);
                    summaries.push((leaf, sum.to_string()));
                }
                // Les lignes de résumé ne sont pas des messages de la
                // conversation : ne pas les compter ni en tirer cwd/timestamp.
                continue;
            }
            message_count += 1;
            if cwd.is_none() {
                if let Some(c) = v.get("cwd").and_then(|c| c.as_str()) {
                    cwd = Some(c.to_string());
                }
            }
            if let Some(u) = v.get("uuid").and_then(|u| u.as_str()) {
                last_uuid = Some(u.to_string());
            }
            if let Some(ts) = v.get("timestamp").and_then(|t| t.as_str()) {
                if first_ts.is_none() {
                    first_ts = Some(ts.to_string());
                }
                last_ts = Some(ts.to_string());
            }
        } else {
            // Ligne non parsable : conserve le comptage historique.
            message_count += 1;
        }
    }

    // Choix du titre : priorité au résumé de la feuille courante, sinon le
    // dernier résumé rencontré.
    let title = last_uuid
        .as_ref()
        .and_then(|leaf| {
            summaries
                .iter()
                .rev()
                .find(|(lu, _)| lu.as_deref() == Some(leaf.as_str()))
                .map(|(_, s)| s.clone())
        })
        .or_else(|| summaries.last().map(|(_, s)| s.clone()));

    Ok(SessionMeta {
        id,
        path: path.to_path_buf(),
        cwd,
        message_count,
        first_ts,
        last_ts,
        size,
        title,
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
    fn sessions_sorted_by_creation_date_desc() {
        let fake = FakeHome::new();
        // Ajoutées dans le désordre chronologique ; l'id ne reflète pas la date.
        fake.add_session(
            "-proj",
            "aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa",
            &[r#"{"type":"user","cwd":"/proj","timestamp":"2026-02-01T00:00:00Z"}"#],
        );
        fake.add_session(
            "-proj",
            "cccccccc-cccc-cccc-cccc-cccccccccccc",
            &[r#"{"type":"user","cwd":"/proj","timestamp":"2026-05-01T00:00:00Z"}"#],
        );
        fake.add_session(
            "-proj",
            "bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb",
            &[r#"{"type":"user","cwd":"/proj","timestamp":"2026-03-01T00:00:00Z"}"#],
        );
        // Sans timestamp : doit finir en dernier.
        fake.add_session("-proj", "dddddddd-dddd-dddd-dddd-dddddddddddd", &["{}"]);

        let home = ClaudeHome::from_base(fake.base());
        let projects = scan_projects(&home).unwrap();
        let ids: Vec<&str> = projects[0].sessions.iter().map(|s| s.id.as_str()).collect();

        assert_eq!(
            ids,
            vec![
                "cccccccc-cccc-cccc-cccc-cccccccccccc", // 2026-05
                "bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb", // 2026-03
                "aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa", // 2026-02
                "dddddddd-dddd-dddd-dddd-dddddddddddd", // sans date → dernier
            ]
        );
    }

    #[test]
    fn extracts_summary_title_for_current_leaf() {
        let fake = FakeHome::new();
        fake.add_session(
            "-proj",
            "eeeeeeee-eeee-eeee-eeee-eeeeeeeeeeee",
            &[
                r#"{"type":"summary","summary":"Ancien titre","leafUuid":"u1"}"#,
                r#"{"type":"summary","summary":"Titre courant","leafUuid":"u2"}"#,
                r#"{"type":"user","cwd":"/proj","uuid":"u1","timestamp":"2026-01-01T10:00:00Z","message":{"content":"a"}}"#,
                r#"{"type":"assistant","uuid":"u2","timestamp":"2026-01-01T10:01:00Z","message":{"content":"b"}}"#,
            ],
        );
        let home = ClaudeHome::from_base(fake.base());
        let projects = scan_projects(&home).unwrap();
        let s = &projects[0].sessions[0];
        // Le titre retenu est celui de la feuille courante (dernier message u2),
        // et les lignes de résumé ne sont pas comptées comme messages.
        assert_eq!(s.title.as_deref(), Some("Titre courant"));
        assert_eq!(s.message_count, 2);
        assert_eq!(s.display_label(), "Titre courant");
    }

    #[test]
    fn falls_back_to_last_summary_without_matching_leaf() {
        let fake = FakeHome::new();
        fake.add_session(
            "-proj",
            "ffffffff-ffff-ffff-ffff-ffffffffffff",
            &[
                r#"{"type":"summary","summary":"Titre orphelin","leafUuid":"zzz"}"#,
                r#"{"type":"user","cwd":"/proj","uuid":"u9","timestamp":"2026-01-01T10:00:00Z","message":{"content":"a"}}"#,
            ],
        );
        let home = ClaudeHome::from_base(fake.base());
        let projects = scan_projects(&home).unwrap();
        assert_eq!(
            projects[0].sessions[0].title.as_deref(),
            Some("Titre orphelin")
        );
    }

    #[test]
    fn no_summary_means_no_title_and_id_label() {
        let fake = FakeHome::new();
        fake.add_session(
            "-proj",
            "12345678-0000-0000-0000-000000000000",
            &[r#"{"type":"user","cwd":"/proj","message":{"content":"a"}}"#],
        );
        let home = ClaudeHome::from_base(fake.base());
        let projects = scan_projects(&home).unwrap();
        let s = &projects[0].sessions[0];
        assert_eq!(s.title, None);
        assert_eq!(s.display_label(), "12345678");
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
