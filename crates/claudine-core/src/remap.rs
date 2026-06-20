use std::path::PathBuf;

use serde_json::Value;

use crate::error::{CoreError, Result};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemapRule {
    pub from: String,
    pub to: String,
}

#[derive(Debug, Clone, Default)]
pub struct RemapTable {
    pub rules: Vec<RemapRule>,
}

impl RemapTable {
    pub fn new(rules: Vec<RemapRule>) -> Self {
        Self { rules }
    }

    /// Remplace le **plus long** préfixe `from` correspondant. `None` si aucun.
    pub fn apply_to_path(&self, s: &str) -> Option<String> {
        let best = self
            .rules
            .iter()
            .filter(|r| s == r.from || s.starts_with(&format!("{}/", r.from)))
            .max_by_key(|r| r.from.len())?;
        Some(s.replacen(&best.from, &best.to, 1))
    }
}

/// Réécrit récursivement toute valeur chaîne qui correspond à une règle.
/// Renvoie la ligne re-sérialisée et le nombre de remplacements.
pub fn rewrite_jsonl_line(line: &str, table: &RemapTable) -> Result<(String, usize)> {
    let mut value: Value =
        serde_json::from_str(line).map_err(|e| CoreError::JsonParse {
            file: PathBuf::from("<ligne jsonl>"),
            line: 0,
            source: e,
        })?;
    let mut count = 0usize;
    rewrite_value(&mut value, table, &mut count);
    let out = serde_json::to_string(&value).map_err(|e| CoreError::JsonParse {
        file: PathBuf::from("<ligne jsonl>"),
        line: 0,
        source: e,
    })?;
    Ok((out, count))
}

fn rewrite_value(value: &mut Value, table: &RemapTable, count: &mut usize) {
    match value {
        Value::String(s) => {
            if let Some(replaced) = table.apply_to_path(s) {
                *s = replaced;
                *count += 1;
            }
        }
        Value::Array(arr) => {
            for v in arr {
                rewrite_value(v, table, count);
            }
        }
        Value::Object(map) => {
            for (_k, v) in map.iter_mut() {
                rewrite_value(v, table, count);
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn table() -> RemapTable {
        RemapTable::new(vec![RemapRule {
            from: "/home/old".to_string(),
            to: "/home/new".to_string(),
        }])
    }

    #[test]
    fn apply_to_path_replaces_prefix() {
        let t = table();
        assert_eq!(t.apply_to_path("/home/old/proj").as_deref(), Some("/home/new/proj"));
        assert_eq!(t.apply_to_path("/other/x"), None);
    }

    #[test]
    fn apply_to_path_prefers_longest_match() {
        let t = RemapTable::new(vec![
            RemapRule { from: "/home".into(), to: "/A".into() },
            RemapRule { from: "/home/old".into(), to: "/B".into() },
        ]);
        assert_eq!(t.apply_to_path("/home/old/x").as_deref(), Some("/B/x"));
    }

    #[test]
    fn rewrite_line_rewrites_cwd_and_nested_paths() {
        let line = r#"{"cwd":"/home/old/proj","data":{"file":"/home/old/proj/a.rs"},"n":1}"#;
        let (out, count) = rewrite_jsonl_line(line, &table()).unwrap();
        let v: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert_eq!(v["cwd"], "/home/new/proj");
        assert_eq!(v["data"]["file"], "/home/new/proj/a.rs");
        assert_eq!(v["n"], 1);
        assert_eq!(count, 2);
    }

    #[test]
    fn rewrite_line_errors_on_non_json() {
        let err = rewrite_jsonl_line("pas du json", &table());
        assert!(err.is_err());
    }
}
