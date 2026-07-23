//! Recherche plein-texte simple dans les sessions.

use std::path::Path;

use serde_json::Value;

/// Cherche `needle_lower` (déjà en minuscules) dans le **texte lisible** des
/// messages d'une session (et non dans les métadonnées JSON : `parentUuid`,
/// `promptId`, etc.). Renvoie `Some(snippet)` — un extrait condensé et centré
/// sur la première occurrence — si trouvé, sinon `None`. Ne panique jamais
/// (fichier illisible → None).
pub fn find_in_session(path: &Path, needle_lower: &str) -> Option<String> {
    if needle_lower.is_empty() {
        return None;
    }
    let content = std::fs::read_to_string(path).ok()?;
    // Rejet rapide : si l'aiguille n'apparaît nulle part (même dans les
    // métadonnées), inutile de parser le fichier ligne par ligne.
    if !content.to_lowercase().contains(needle_lower) {
        return None;
    }
    for line in content.lines() {
        if line.trim().is_empty() {
            continue;
        }
        let Ok(v) = serde_json::from_str::<Value>(line) else {
            continue;
        };
        if let Some(text) = extract_line_text(&v) {
            if text.to_lowercase().contains(needle_lower) {
                return Some(snippet_around(&text, needle_lower));
            }
        }
    }
    // L'aiguille n'existe que dans les métadonnées : aucune correspondance de
    // contenu lisible à afficher.
    None
}

/// Extrait le texte lisible d'une entrée de session : contenu des messages
/// utilisateur/assistant (blocs `text`, noms d'outils). Renvoie `None` si
/// l'entrée n'a pas de texte exploitable (métadonnées, pièce jointe, etc.).
fn extract_line_text(v: &Value) -> Option<String> {
    let content = v
        .get("message")
        .and_then(|m| m.get("content"))
        .or_else(|| v.get("content"))?;
    let text = match content {
        Value::String(s) => s.clone(),
        Value::Array(items) => {
            let mut parts = Vec::new();
            for item in items {
                match item.get("type").and_then(|t| t.as_str()) {
                    Some("text") => {
                        if let Some(t) = item.get("text").and_then(|t| t.as_str()) {
                            parts.push(t.to_string());
                        }
                    }
                    Some("tool_use") => {
                        if let Some(n) = item.get("name").and_then(|n| n.as_str()) {
                            parts.push(n.to_string());
                        }
                    }
                    _ => {}
                }
            }
            parts.join(" ")
        }
        _ => return None,
    };
    let trimmed = text.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

/// Construit un extrait d'environ 140 caractères, sur une seule ligne, centré
/// sur la première occurrence de `needle_lower`, avec des ellipses aux bords
/// tronqués. Sans correspondance (cas improbable ici), renvoie le début du
/// texte.
fn snippet_around(text: &str, needle_lower: &str) -> String {
    const BEFORE: usize = 40;
    const WIDTH: usize = 140;

    let condensed = text.split_whitespace().collect::<Vec<_>>().join(" ");
    let lower = condensed.to_lowercase();
    let Some(byte_pos) = lower.find(needle_lower) else {
        return condensed.chars().take(WIDTH).collect();
    };
    // On veut afficher la casse d'origine (`condensed`), mais l'index provient
    // de `lower`. On ne l'utilise sur `condensed` que si les longueurs
    // coïncident et que l'octet tombe sur une frontière de caractère valide ;
    // sinon on se rabat sur `lower` (où l'index est toujours valide).
    let src = if condensed.len() == lower.len() && condensed.is_char_boundary(byte_pos) {
        condensed.as_str()
    } else {
        lower.as_str()
    };
    let match_char = src[..byte_pos].chars().count();
    let chars: Vec<char> = src.chars().collect();
    let start = match_char.saturating_sub(BEFORE);
    let end = (start + WIDTH).min(chars.len());

    let mut out = String::new();
    if start > 0 {
        out.push('…');
    }
    out.extend(&chars[start..end]);
    if end < chars.len() {
        out.push('…');
    }
    out
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
        // La casse d'origine est préservée.
        assert!(hit.contains("WIDGET"));
    }

    #[test]
    fn returns_none_when_absent_or_empty() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("s.jsonl");
        std::fs::write(&p, "{\"x\":1}\n").unwrap();
        assert!(find_in_session(&p, "absent").is_none());
        assert!(find_in_session(&p, "").is_none());
    }

    #[test]
    fn ignores_matches_in_json_metadata() {
        // « tooling » n'apparaît que dans une métadonnée (promptId), jamais dans
        // le texte du message : ne doit pas produire de correspondance.
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("s.jsonl");
        std::fs::write(
            &p,
            r#"{"type":"user","promptId":"tooling-42","message":{"content":"bonjour le monde"}}"#,
        )
        .unwrap();
        assert!(find_in_session(&p, "tooling").is_none());
    }

    #[test]
    fn snippet_is_centered_on_the_match() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("s.jsonl");
        let long = format!(
            "{} le mot TOOLING au milieu {}",
            "bla ".repeat(30),
            "bli ".repeat(30)
        );
        std::fs::write(
            &p,
            format!(r#"{{"type":"user","message":{{"content":"{long}"}}}}"#),
        )
        .unwrap();
        let hit = find_in_session(&p, "tooling").unwrap();
        assert!(hit.to_lowercase().contains("tooling"));
        // Tronqué des deux côtés : ellipses en tête et en queue.
        assert!(hit.starts_with('…'));
        assert!(hit.ends_with('…'));
        // Le contexte immédiat du terme est présent.
        assert!(hit.contains("au milieu"));
    }

    #[test]
    fn matches_tool_use_name() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("s.jsonl");
        std::fs::write(
            &p,
            r#"{"type":"assistant","message":{"content":[{"type":"tool_use","name":"Bash","input":{"command":"ls"}}]}}"#,
        )
        .unwrap();
        let hit = find_in_session(&p, "bash").unwrap();
        assert!(hit.to_lowercase().contains("bash"));
    }
}
