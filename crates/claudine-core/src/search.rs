//! Recherche plein-texte simple dans les sessions.

use std::path::Path;

/// Cherche `needle_lower` (déjà en minuscules) dans le contenu d'une session.
/// Renvoie `Some(snippet)` (première ligne correspondante, condensée et
/// tronquée) si trouvé, sinon `None`. Ne panique jamais (fichier illisible → None).
pub fn find_in_session(path: &Path, needle_lower: &str) -> Option<String> {
    if needle_lower.is_empty() {
        return None;
    }
    let content = std::fs::read_to_string(path).ok()?;
    if !content.to_lowercase().contains(needle_lower) {
        return None;
    }
    for line in content.lines() {
        if line.to_lowercase().contains(needle_lower) {
            let condensed = line.split_whitespace().collect::<Vec<_>>().join(" ");
            let snippet: String = condensed.chars().take(140).collect();
            return Some(snippet);
        }
    }
    Some(String::new())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_case_insensitive_and_returns_snippet() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("s.jsonl");
        std::fs::write(
            &p,
            "{\"type\":\"user\",\"message\":{\"content\":\"Refactor the WIDGET layout\"}}\n",
        )
        .unwrap();
        let hit = find_in_session(&p, "widget").unwrap();
        assert!(hit.to_lowercase().contains("widget"));
    }

    #[test]
    fn returns_none_when_absent_or_empty() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("s.jsonl");
        std::fs::write(&p, "{\"x\":1}\n").unwrap();
        assert!(find_in_session(&p, "absent").is_none());
        assert!(find_in_session(&p, "").is_none());
    }
}
