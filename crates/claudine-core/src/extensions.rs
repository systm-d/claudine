//! Lecture (en lecture seule) des extensions configurées d'un home Claude :
//! hooks (`settings.json`), plugins (`plugins/installed_plugins.json` +
//! `enabledPlugins`) et serveurs MCP (`<home>/.claude.json` ou `<base>.json`).
//!
//! Le but est l'inspection : présenter à l'utilisateur ce qui est branché sur
//! son installation. L'édition reste déléguée à `$EDITOR` sur les fichiers
//! sous-jacents (les chemins MCP étant globaux/ambigus, on ne les réécrit pas).

use std::fs;
use std::path::{Path, PathBuf};

use serde_json::Value;

use crate::home::ClaudeHome;

/// Un hook : un évènement, un éventuel filtre (`matcher`) et ses commandes.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct HookEntry {
    pub event: String,
    pub matcher: Option<String>,
    pub commands: Vec<String>,
}

/// Un plugin installé et/ou activé.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PluginEntry {
    pub name: String,
    pub enabled: bool,
    pub version: Option<String>,
    pub scope: Option<String>,
}

/// Un serveur MCP déclaré (portée utilisateur ou projet).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct McpEntry {
    pub name: String,
    pub scope: String,
    pub summary: String,
}

/// Vue agrégée des extensions d'un home.
#[derive(Debug, Clone, Default)]
pub struct Extensions {
    pub hooks: Vec<HookEntry>,
    pub plugins: Vec<PluginEntry>,
    pub mcp: Vec<McpEntry>,
}

/// Lit, sans rien modifier, les extensions configurées du home.
pub fn read_extensions(home: &ClaudeHome) -> Extensions {
    Extensions {
        hooks: read_hooks(home),
        plugins: read_plugins(home),
        mcp: read_mcp(home),
    }
}

fn load_json(path: &Path) -> Option<Value> {
    let text = fs::read_to_string(path).ok()?;
    serde_json::from_str(&text).ok()
}

/// Hooks : `settings.json` puis `settings.local.json` (qui s'ajoute).
fn read_hooks(home: &ClaudeHome) -> Vec<HookEntry> {
    let mut out = Vec::new();
    for file in [home.settings_file(), home.settings_local_file()] {
        let Some(v) = load_json(&file) else { continue };
        let Some(hooks) = v.get("hooks").and_then(|h| h.as_object()) else {
            continue;
        };
        for (event, groups) in hooks {
            let Some(arr) = groups.as_array() else { continue };
            for group in arr {
                let matcher = group
                    .get("matcher")
                    .and_then(|m| m.as_str())
                    .filter(|s| !s.is_empty())
                    .map(|s| s.to_string());
                let commands = group
                    .get("hooks")
                    .and_then(|h| h.as_array())
                    .map(|hs| {
                        hs.iter()
                            .filter_map(|h| h.get("command").and_then(|c| c.as_str()))
                            .map(|s| s.to_string())
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default();
                if commands.is_empty() && matcher.is_none() {
                    continue;
                }
                out.push(HookEntry {
                    event: event.clone(),
                    matcher,
                    commands,
                });
            }
        }
    }
    out.sort_by(|a, b| a.event.cmp(&b.event).then(a.matcher.cmp(&b.matcher)));
    out
}

/// Plugins : noms depuis `installed_plugins.json`, activation depuis
/// `enabledPlugins` (settings). Si rien d'installé, on retombe sur les clés
/// d'`enabledPlugins`.
fn read_plugins(home: &ClaudeHome) -> Vec<PluginEntry> {
    let enabled_map = load_json(&home.settings_file())
        .and_then(|v| v.get("enabledPlugins").cloned())
        .and_then(|v| v.as_object().cloned())
        .unwrap_or_default();
    let is_enabled = |name: &str| enabled_map.get(name).and_then(|v| v.as_bool()).unwrap_or(false);

    let installed = load_json(&home.plugins_dir().join("installed_plugins.json"))
        .and_then(|v| v.get("plugins").cloned())
        .and_then(|v| v.as_object().cloned())
        .unwrap_or_default();

    let mut out = Vec::new();
    if installed.is_empty() {
        // Pas de fichier d'installation : liste au moins les plugins activés.
        for name in enabled_map.keys() {
            out.push(PluginEntry {
                name: name.clone(),
                enabled: is_enabled(name),
                ..Default::default()
            });
        }
    } else {
        for (name, detail) in &installed {
            // `detail` est un tableau d'installations ; on prend la première.
            let first = detail.as_array().and_then(|a| a.first());
            let version = first
                .and_then(|d| d.get("version"))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            let scope = first
                .and_then(|d| d.get("scope"))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            out.push(PluginEntry {
                name: name.clone(),
                enabled: is_enabled(name),
                version,
                scope,
            });
        }
    }
    out.sort_by(|a, b| a.name.cmp(&b.name));
    out
}

/// Fichiers `.claude.json` candidats pour un home : à l'intérieur du home, et
/// la variante héritée à côté (`<base>.json`, ex. `~/.claude.json`).
fn mcp_config_candidates(home: &ClaudeHome) -> Vec<PathBuf> {
    let mut v = vec![home.base.join(".claude.json")];
    if let (Some(parent), Some(name)) = (home.base.parent(), home.base.file_name()) {
        v.push(parent.join(format!("{}.json", name.to_string_lossy())));
    }
    v
}

fn mcp_summary(def: &Value) -> String {
    if let Some(url) = def.get("url").and_then(|u| u.as_str()) {
        return url.to_string();
    }
    let cmd = def.get("command").and_then(|c| c.as_str()).unwrap_or("");
    let args = def
        .get("args")
        .and_then(|a| a.as_array())
        .map(|a| {
            a.iter()
                .filter_map(|x| x.as_str())
                .collect::<Vec<_>>()
                .join(" ")
        })
        .unwrap_or_default();
    let joined = format!("{cmd} {args}");
    let joined = joined.trim();
    if joined.is_empty() {
        def.get("type")
            .and_then(|t| t.as_str())
            .unwrap_or("(défini)")
            .to_string()
    } else {
        joined.to_string()
    }
}

fn collect_mcp_from(value: &Value, out: &mut Vec<McpEntry>) {
    // Portée utilisateur (top-level).
    if let Some(servers) = value.get("mcpServers").and_then(|m| m.as_object()) {
        for (name, def) in servers {
            out.push(McpEntry {
                name: name.clone(),
                scope: "utilisateur".to_string(),
                summary: mcp_summary(def),
            });
        }
    }
    // Portée projet.
    if let Some(projects) = value.get("projects").and_then(|p| p.as_object()) {
        for (path, pval) in projects {
            let Some(servers) = pval.get("mcpServers").and_then(|m| m.as_object()) else {
                continue;
            };
            let short = Path::new(path)
                .file_name()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_else(|| path.clone());
            for (name, def) in servers {
                out.push(McpEntry {
                    name: name.clone(),
                    scope: format!("projet:{short}"),
                    summary: mcp_summary(def),
                });
            }
        }
    }
}

fn read_mcp(home: &ClaudeHome) -> Vec<McpEntry> {
    let mut out = Vec::new();
    let mut seen = Vec::new();
    for cand in mcp_config_candidates(home) {
        if seen.contains(&cand) {
            continue;
        }
        seen.push(cand.clone());
        if let Some(v) = load_json(&cand) {
            collect_mcp_from(&v, &mut out);
        }
    }
    out.sort_by(|a, b| a.scope.cmp(&b.scope).then(a.name.cmp(&b.name)));
    out.dedup();
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn home_with(files: &[(&str, &str)]) -> (tempfile::TempDir, ClaudeHome) {
        let dir = tempfile::tempdir().unwrap();
        for (rel, content) in files {
            let p = dir.path().join(rel);
            fs::create_dir_all(p.parent().unwrap()).unwrap();
            fs::write(p, content).unwrap();
        }
        let home = ClaudeHome::from_base(dir.path());
        (dir, home)
    }

    #[test]
    fn reads_hooks_with_matcher_and_commands() {
        let settings = r#"{
            "hooks": {
                "PreToolUse": [
                    {"matcher": "Bash", "hooks": [{"type":"command","command":"echo a"}]}
                ],
                "SessionStart": [
                    {"hooks": [{"type":"command","command":"echo b"},{"type":"command","command":"echo c"}]}
                ]
            }
        }"#;
        let (_d, home) = home_with(&[("settings.json", settings)]);
        let ext = read_extensions(&home);
        assert_eq!(ext.hooks.len(), 2);
        // Trié par évènement : PreToolUse avant SessionStart.
        assert_eq!(ext.hooks[0].event, "PreToolUse");
        assert_eq!(ext.hooks[0].matcher.as_deref(), Some("Bash"));
        assert_eq!(ext.hooks[0].commands, vec!["echo a"]);
        assert_eq!(ext.hooks[1].event, "SessionStart");
        assert_eq!(ext.hooks[1].commands.len(), 2);
    }

    #[test]
    fn reads_plugins_with_enabled_flag() {
        let settings = r#"{"enabledPlugins":{"foo@m":true,"bar@m":false}}"#;
        let installed = r#"{"version":1,"plugins":{
            "foo@m":[{"scope":"user","version":"1.2.0"}],
            "bar@m":[{"scope":"local","version":"0.1.0"}]
        }}"#;
        let (_d, home) = home_with(&[
            ("settings.json", settings),
            ("plugins/installed_plugins.json", installed),
        ]);
        let ext = read_extensions(&home);
        assert_eq!(ext.plugins.len(), 2);
        let foo = ext.plugins.iter().find(|p| p.name == "foo@m").unwrap();
        assert!(foo.enabled);
        assert_eq!(foo.version.as_deref(), Some("1.2.0"));
        let bar = ext.plugins.iter().find(|p| p.name == "bar@m").unwrap();
        assert!(!bar.enabled);
    }

    #[test]
    fn reads_mcp_user_and_project_scopes() {
        let claude_json = r#"{
            "mcpServers": {"fs": {"command":"npx","args":["-y","server-fs"]}},
            "projects": {
                "/home/x/proj": {"mcpServers": {"db": {"type":"http","url":"http://localhost:1"}}}
            }
        }"#;
        let (_d, home) = home_with(&[(".claude.json", claude_json)]);
        let ext = read_extensions(&home);
        assert_eq!(ext.mcp.len(), 2);
        let fs_srv = ext.mcp.iter().find(|m| m.name == "fs").unwrap();
        assert_eq!(fs_srv.scope, "utilisateur");
        assert_eq!(fs_srv.summary, "npx -y server-fs");
        let db = ext.mcp.iter().find(|m| m.name == "db").unwrap();
        assert_eq!(db.scope, "projet:proj");
        assert_eq!(db.summary, "http://localhost:1");
    }

    #[test]
    fn empty_home_yields_empty_extensions() {
        let (_d, home) = home_with(&[]);
        let ext = read_extensions(&home);
        assert!(ext.hooks.is_empty());
        assert!(ext.plugins.is_empty());
        assert!(ext.mcp.is_empty());
    }
}
