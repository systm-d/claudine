//! Lecture/écriture d'un `settings.json` Claude Code en **préservant** toutes les
//! clés que Claudine ne modélise pas (hooks, plugins, `$schema`, …), plus un
//! catalogue de champs qui pilote le formulaire d'édition du TUI.

use std::fs;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::{Map, Value};

use crate::error::{CoreError, Result};

/// Un document `settings.json`. La source de vérité est un objet JSON complet :
/// on ne modifie que les feuilles éditées, tout le reste est conservé tel quel.
#[derive(Debug, Clone)]
pub struct SettingsDoc {
    root: Value,
}

impl SettingsDoc {
    /// Document vide (objet `{}`).
    pub fn empty() -> Self {
        SettingsDoc {
            root: Value::Object(Map::new()),
        }
    }

    /// Charge le fichier. Absent ou vide → document vide. Présent mais illisible →
    /// `CoreError::JsonParse`. Présent mais non-objet → document vide.
    pub fn load(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::empty());
        }
        let content = fs::read_to_string(path).map_err(|e| CoreError::io(path, e))?;
        if content.trim().is_empty() {
            return Ok(Self::empty());
        }
        let value: Value = serde_json::from_str(&content).map_err(|e| CoreError::JsonParse {
            file: path.to_path_buf(),
            line: 0,
            source: e,
        })?;
        Ok(SettingsDoc {
            root: if value.is_object() {
                value
            } else {
                Value::Object(Map::new())
            },
        })
    }

    /// Écrit le document. Si le fichier existe, en fait d'abord une copie de
    /// sauvegarde horodatée `<nom>.bak-<nanos>`, puis écrit via fichier temporaire
    /// + rename (jamais de fichier à moitié écrit).
    pub fn save(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent).map_err(|e| CoreError::io(parent, e))?;
            }
        }
        let file_name = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| "settings.json".to_string());

        if path.exists() {
            let ts = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0);
            let backup = path.with_file_name(format!("{file_name}.bak-{ts}"));
            fs::copy(path, &backup).map_err(|e| CoreError::io(&backup, e))?;
        }

        let pretty =
            serde_json::to_string_pretty(&self.root).map_err(|e| CoreError::JsonParse {
                file: path.to_path_buf(),
                line: 0,
                source: e,
            })?;
        let tmp = path.with_file_name(format!("{file_name}.tmp"));
        fs::write(&tmp, pretty.as_bytes()).map_err(|e| CoreError::io(&tmp, e))?;
        fs::rename(&tmp, path).map_err(|e| CoreError::io(path, e))?;
        Ok(())
    }

    /// L'objet racine (pour affichage brut / itération).
    pub fn root(&self) -> &Value {
        &self.root
    }

    /// JSON indenté.
    pub fn to_pretty(&self) -> String {
        serde_json::to_string_pretty(&self.root).unwrap_or_default()
    }

    /// Valeur à un chemin imbriqué (ex. `["permissions","defaultMode"]`).
    pub fn get<S: AsRef<str>>(&self, path: &[S]) -> Option<&Value> {
        let mut cur = &self.root;
        for key in path {
            cur = cur.as_object()?.get(key.as_ref())?;
        }
        Some(cur)
    }

    /// Définit une valeur à un chemin imbriqué, en créant les objets intermédiaires.
    pub fn set<S: AsRef<str>>(&mut self, path: &[S], value: Value) {
        if path.is_empty() {
            if value.is_object() {
                self.root = value;
            }
            return;
        }
        if !self.root.is_object() {
            self.root = Value::Object(Map::new());
        }
        let mut cur = &mut self.root;
        for key in &path[..path.len() - 1] {
            let map = cur.as_object_mut().expect("objet garanti");
            let entry = map
                .entry(key.as_ref().to_string())
                .or_insert_with(|| Value::Object(Map::new()));
            if !entry.is_object() {
                *entry = Value::Object(Map::new());
            }
            cur = entry;
        }
        let last = path[path.len() - 1].as_ref().to_string();
        cur.as_object_mut()
            .expect("objet garanti")
            .insert(last, value);
    }

    /// Supprime la feuille au chemin donné (sert à « vider » un champ).
    pub fn unset<S: AsRef<str>>(&mut self, path: &[S]) {
        if path.is_empty() {
            return;
        }
        let mut cur = &mut self.root;
        for key in &path[..path.len() - 1] {
            match cur.as_object_mut().and_then(|o| o.get_mut(key.as_ref())) {
                Some(next) => cur = next,
                None => return,
            }
        }
        if let Some(obj) = cur.as_object_mut() {
            obj.remove(path[path.len() - 1].as_ref());
        }
    }

    pub fn get_bool<S: AsRef<str>>(&self, path: &[S]) -> Option<bool> {
        self.get(path).and_then(Value::as_bool)
    }

    pub fn get_str<S: AsRef<str>>(&self, path: &[S]) -> Option<&str> {
        self.get(path).and_then(Value::as_str)
    }

    pub fn get_i64<S: AsRef<str>>(&self, path: &[S]) -> Option<i64> {
        self.get(path).and_then(Value::as_i64)
    }

    pub fn get_str_list<S: AsRef<str>>(&self, path: &[S]) -> Option<Vec<String>> {
        let arr = self.get(path)?.as_array()?;
        Some(
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect(),
        )
    }

    pub fn get_object<S: AsRef<str>>(&self, path: &[S]) -> Option<&Map<String, Value>> {
        self.get(path).and_then(Value::as_object)
    }

    /// Paires clé/valeur (chaînes) d'un objet, pour l'édition d'`env` par exemple.
    pub fn get_pairs<S: AsRef<str>>(&self, path: &[S]) -> Vec<(String, String)> {
        self.get(path)
            .and_then(Value::as_object)
            .map(|m| {
                m.iter()
                    .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn set_bool<S: AsRef<str>>(&mut self, path: &[S], v: bool) {
        self.set(path, Value::Bool(v));
    }

    pub fn set_str<S: AsRef<str>>(&mut self, path: &[S], v: &str) {
        self.set(path, Value::String(v.to_string()));
    }

    pub fn set_i64<S: AsRef<str>>(&mut self, path: &[S], v: i64) {
        self.set(path, Value::from(v));
    }

    pub fn set_str_list<S: AsRef<str>>(&mut self, path: &[S], items: &[String]) {
        self.set(
            path,
            Value::Array(items.iter().map(|s| Value::String(s.clone())).collect()),
        );
    }

    pub fn set_string_map<S: AsRef<str>>(&mut self, path: &[S], pairs: &[(String, String)]) {
        let mut map = Map::new();
        for (k, v) in pairs {
            map.insert(k.clone(), Value::String(v.clone()));
        }
        self.set(path, Value::Object(map));
    }
}

/// Type d'un champ éditable du formulaire.
#[derive(Debug, Clone, PartialEq)]
pub enum FieldKind {
    Bool,
    /// Valeurs autorisées ; la chaîne vide signifie « non défini ».
    Enum(Vec<String>),
    Text,
    Number,
    /// Tableau JSON de chaînes.
    StringList,
    /// Objet JSON chaîne → chaîne (ex. `env`).
    KeyValue,
}

/// Description d'un champ de configuration éditable.
#[derive(Debug, Clone)]
pub struct FieldSpec {
    pub path: Vec<String>,
    pub label: String,
    pub section: String,
    pub kind: FieldKind,
    pub note: Option<String>,
}

fn field(
    path: &[&str],
    label: &str,
    section: &str,
    kind: FieldKind,
    note: Option<&str>,
) -> FieldSpec {
    FieldSpec {
        path: path.iter().map(|s| s.to_string()).collect(),
        label: label.to_string(),
        section: section.to_string(),
        kind,
        note: note.map(|s| s.to_string()),
    }
}

fn en(opts: &[&str]) -> FieldKind {
    FieldKind::Enum(opts.iter().map(|s| s.to_string()).collect())
}

/// Catalogue ordonné et groupé des champs exposés par le formulaire. Tout champ
/// non listé reste éditable via le JSON brut et est préservé à l'écriture.
pub fn settings_catalog() -> Vec<FieldSpec> {
    use FieldKind::{Bool, Number, StringList, Text};
    vec![
        // Général
        field(&["model"], "Modèle", "Général", Text, None),
        field(
            &["effortLevel"],
            "Niveau d'effort",
            "Général",
            en(&["", "low", "medium", "high", "xhigh"]),
            None,
        ),
        field(&["outputStyle"], "Style de sortie", "Général", Text, None),
        field(&["language"], "Langue", "Général", Text, None),
        field(
            &["theme"],
            "Thème",
            "Général",
            Text,
            Some("vide = auto/null"),
        ),
        field(
            &["editorMode"],
            "Mode éditeur",
            "Général",
            en(&["", "normal", "vim"]),
            None,
        ),
        field(
            &["tui"],
            "Rendu TUI",
            "Général",
            en(&["", "classic", "fullscreen"]),
            None,
        ),
        field(
            &["autoUpdatesChannel"],
            "Canal de mise à jour",
            "Général",
            en(&["", "latest", "stable"]),
            None,
        ),
        // Comportement
        field(
            &["alwaysThinkingEnabled"],
            "Réflexion étendue par défaut",
            "Comportement",
            Bool,
            None,
        ),
        field(
            &["autoCompactEnabled"],
            "Auto-compactage",
            "Comportement",
            Bool,
            None,
        ),
        field(
            &["autoMemoryEnabled"],
            "Mémoire auto",
            "Comportement",
            Bool,
            None,
        ),
        field(
            &["fileCheckpointingEnabled"],
            "Snapshots de fichiers",
            "Comportement",
            Bool,
            None,
        ),
        field(
            &["respectGitignore"],
            "Respecter .gitignore",
            "Comportement",
            Bool,
            None,
        ),
        field(
            &["spinnerTipsEnabled"],
            "Astuces du spinner",
            "Comportement",
            Bool,
            None,
        ),
        field(
            &["verboseOutput"],
            "Sortie verbeuse",
            "Comportement",
            Bool,
            None,
        ),
        field(
            &["includeCoAuthoredBy"],
            "Co-authored-by",
            "Comportement",
            Bool,
            Some("déprécié — préférez attribution"),
        ),
        field(
            &["cleanupPeriodDays"],
            "Rétention sessions (jours)",
            "Comportement",
            Number,
            None,
        ),
        // Permissions
        field(
            &["permissions", "defaultMode"],
            "Mode par défaut",
            "Permissions",
            en(&["", "prompt", "auto", "human"]),
            None,
        ),
        field(
            &["permissions", "allow"],
            "Autorisé (allow)",
            "Permissions",
            StringList,
            None,
        ),
        field(
            &["permissions", "deny"],
            "Bloqué (deny)",
            "Permissions",
            StringList,
            None,
        ),
        field(
            &["permissions", "ask"],
            "Demander (ask)",
            "Permissions",
            StringList,
            None,
        ),
        field(
            &["permissions", "additionalDirectories"],
            "Dossiers additionnels",
            "Permissions",
            StringList,
            None,
        ),
        field(
            &["permissions", "disableBypassPermissionsMode"],
            "Bloquer le bypass",
            "Permissions",
            Bool,
            None,
        ),
        // Environnement
        field(
            &["env"],
            "Variables d'environnement",
            "Environnement",
            FieldKind::KeyValue,
            None,
        ),
        // MCP
        field(
            &["enableAllProjectMcpServers"],
            "Auto-approuver MCP projet",
            "MCP",
            Bool,
            None,
        ),
        field(
            &["enabledMcpjsonServers"],
            "Serveurs .mcp.json activés",
            "MCP",
            StringList,
            None,
        ),
        field(
            &["disabledMcpjsonServers"],
            "Serveurs .mcp.json refusés",
            "MCP",
            StringList,
            None,
        ),
        // Attribution / Git
        field(
            &["attribution", "commit"],
            "Attribution commit",
            "Attribution / Git",
            Text,
            None,
        ),
        field(
            &["attribution", "pr"],
            "Attribution PR",
            "Attribution / Git",
            Text,
            None,
        ),
        // Avancé
        field(&["apiKeyHelper"], "Script clé API", "Avancé", Text, None),
        field(
            &["minimumVersion"],
            "Version minimale",
            "Avancé",
            Text,
            None,
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn preserves_unknown_keys_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        let original = r#"{
  "$schema": "https://example/schema.json",
  "hooks": { "Stop": [ { "x": 1 } ] },
  "enabledPlugins": { "superpowers@x": true },
  "includeCoAuthoredBy": false
}"#;
        std::fs::write(&path, original).unwrap();

        let mut doc = SettingsDoc::load(&path).unwrap();
        doc.set(&["permissions", "defaultMode"], json!("auto"));
        doc.set_bool(&["alwaysThinkingEnabled"], true);
        doc.save(&path).unwrap();

        let reloaded = SettingsDoc::load(&path).unwrap();
        assert_eq!(
            reloaded.get_str(&["permissions", "defaultMode"]),
            Some("auto")
        );
        assert_eq!(reloaded.get_bool(&["alwaysThinkingEnabled"]), Some(true));
        // clés inconnues préservées
        assert!(reloaded.get(&["$schema"]).is_some());
        assert!(reloaded.get(&["hooks"]).is_some());
        assert!(reloaded.get(&["enabledPlugins"]).is_some());
        assert_eq!(reloaded.get_bool(&["includeCoAuthoredBy"]), Some(false));
        // une sauvegarde a été créée
        let backups = std::fs::read_dir(dir.path())
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_name().to_string_lossy().contains(".bak-"))
            .count();
        assert_eq!(backups, 1);
    }

    #[test]
    fn load_missing_is_empty_and_save_creates() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("sub").join("settings.json");
        let mut doc = SettingsDoc::load(&path).unwrap();
        assert!(doc.get(&["model"]).is_none());
        doc.set_str(&["model"], "claude-sonnet-4-6");
        doc.save(&path).unwrap();
        assert!(path.exists());
        let reloaded = SettingsDoc::load(&path).unwrap();
        assert_eq!(reloaded.get_str(&["model"]), Some("claude-sonnet-4-6"));
    }

    #[test]
    fn nested_set_creates_intermediates() {
        let mut doc = SettingsDoc::empty();
        doc.set(&["permissions", "allow"], json!(["Bash(ls)"]));
        assert_eq!(
            doc.get_str_list(&["permissions", "allow"]),
            Some(vec!["Bash(ls)".to_string()])
        );
    }

    #[test]
    fn typed_accessors() {
        let mut doc = SettingsDoc::empty();
        doc.set_bool(&["b"], true);
        doc.set_i64(&["n"], 30);
        doc.set_str(&["s"], "hi");
        doc.set_str_list(&["l"], &["a".to_string(), "b".to_string()]);
        doc.set_string_map(&["env"], &[("FOO".to_string(), "bar".to_string())]);
        assert_eq!(doc.get_bool(&["b"]), Some(true));
        assert_eq!(doc.get_i64(&["n"]), Some(30));
        assert_eq!(doc.get_str(&["s"]), Some("hi"));
        assert_eq!(doc.get_str_list(&["l"]), Some(vec!["a".into(), "b".into()]));
        assert_eq!(
            doc.get_pairs(&["env"]),
            vec![("FOO".to_string(), "bar".to_string())]
        );
        // type incompatible / absent
        assert_eq!(doc.get_bool(&["s"]), None);
        assert_eq!(doc.get_str(&["absent"]), None);
    }

    #[test]
    fn unset_removes_leaf() {
        let mut doc = SettingsDoc::empty();
        doc.set_bool(&["permissions", "disableBypassPermissionsMode"], true);
        doc.unset(&["permissions", "disableBypassPermissionsMode"]);
        assert!(
            doc.get(&["permissions", "disableBypassPermissionsMode"])
                .is_none()
        );
    }

    #[test]
    fn catalog_has_expected_fields() {
        let cat = settings_catalog();
        assert!(!cat.is_empty());
        let dm = cat
            .iter()
            .find(|f| f.path == ["permissions", "defaultMode"])
            .unwrap();
        assert!(matches!(dm.kind, FieldKind::Enum(_)));
        let env = cat.iter().find(|f| f.path == ["env"]).unwrap();
        assert!(matches!(env.kind, FieldKind::KeyValue));
    }
}
