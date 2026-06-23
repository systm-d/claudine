//! Lecture et écriture des extensions configurées d'un home Claude :
//! hooks (`settings.json`), plugins (`plugins/installed_plugins.json` +
//! `enabledPlugins`) et serveurs MCP (`<home>/.claude.json` ou `<base>.json`).
//!
//! Lit et affiche ce qui est branché (hooks, plugins, serveurs MCP). Écrit les
//! hooks, l'état des plugins et les serveurs MCP via `write_hooks`,
//! `set_plugin_enabled` et `write_user_mcp_servers` (portée utilisateur).

use std::fs;
use std::path::{Path, PathBuf};

use serde_json::{Map, Value};

use crate::home::ClaudeHome;
use crate::settings::SettingsDoc;
use crate::error::Result;

/// Un hook : un évènement, un éventuel filtre (`matcher`) et ses commandes.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct HookEntry {
    pub event: String,
    pub matcher: Option<String>,
    pub commands: Vec<String>,
}

/// Une commande de hook (modèle d'édition, niveau « complet »).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct HookCommand {
    pub kind: String, // "command" par défaut
    pub command: String,
    pub timeout: Option<u64>,
}

/// Un groupe de hook : un évènement, un matcher optionnel, des commandes.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct HookGroup {
    pub event: String,
    pub matcher: Option<String>,
    pub commands: Vec<HookCommand>,
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

/// Transport d'un serveur MCP.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum McpTransport {
    Stdio,
    Http,
    Sse,
}

/// Un serveur MCP éditable (portée utilisateur).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct McpServer {
    pub name: String,
    pub transport: McpTransport,
    pub command: String,
    pub args: Vec<String>,
    pub env: Vec<(String, String)>,
    pub url: String,
    pub headers: Vec<(String, String)>,
}

impl Default for McpServer {
    fn default() -> Self {
        Self {
            name: String::new(),
            transport: McpTransport::Stdio,
            command: String::new(),
            args: Vec::new(),
            env: Vec::new(),
            url: String::new(),
            headers: Vec::new(),
        }
    }
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

/// Lit les hooks de `settings.json` (uniquement) sous forme éditable, en
/// préservant l'ordre du fichier. Renvoie une liste vide si absent/illisible.
pub fn read_hook_groups(home: &ClaudeHome) -> Vec<HookGroup> {
    let Some(v) = load_json(&home.settings_file()) else {
        return Vec::new();
    };
    let Some(hooks) = v.get("hooks").and_then(|h| h.as_object()) else {
        return Vec::new();
    };
    let mut out = Vec::new();
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
                        .map(|h| HookCommand {
                            kind: h
                                .get("type")
                                .and_then(|t| t.as_str())
                                .unwrap_or("command")
                                .to_string(),
                            command: h
                                .get("command")
                                .and_then(|c| c.as_str())
                                .unwrap_or("")
                                .to_string(),
                            timeout: h.get("timeout").and_then(|t| t.as_u64()),
                        })
                        .collect()
                })
                .unwrap_or_default();
            out.push(HookGroup {
                event: event.clone(),
                matcher,
                commands,
            });
        }
    }
    out
}

/// Active / désactive un plugin dans `enabledPlugins` de `settings.json`.
pub fn set_plugin_enabled(home: &ClaudeHome, name: &str, enabled: bool) -> Result<()> {
    let path = home.settings_file();
    let mut doc = SettingsDoc::load(&path)?;
    doc.set(&["enabledPlugins", name], Value::Bool(enabled));
    doc.save(&path)
}

/// Réécrit la clé `hooks` de `settings.json` à partir du modèle d'édition.
/// Les autres réglages sont préservés ; backup + écriture atomique via SettingsDoc.
pub fn write_hooks(home: &ClaudeHome, groups: &[HookGroup]) -> Result<()> {
    let path = home.settings_file();
    let mut doc = SettingsDoc::load(&path)?;

    if groups.is_empty() {
        doc.unset(&["hooks"]);
        return doc.save(&path);
    }

    let mut hooks: Map<String, Value> = Map::new();
    for g in groups {
        let mut grp = Map::new();
        if let Some(m) = &g.matcher {
            if !m.is_empty() {
                grp.insert("matcher".to_string(), Value::String(m.clone()));
            }
        }
        let cmds: Vec<Value> = g
            .commands
            .iter()
            .map(|c| {
                let mut cm = Map::new();
                let kind = if c.kind.is_empty() { "command" } else { &c.kind };
                cm.insert("type".to_string(), Value::String(kind.to_string()));
                cm.insert("command".to_string(), Value::String(c.command.clone()));
                if let Some(t) = c.timeout {
                    cm.insert("timeout".to_string(), Value::Number(t.into()));
                }
                Value::Object(cm)
            })
            .collect();
        grp.insert("hooks".to_string(), Value::Array(cmds));

        let entry = hooks
            .entry(g.event.clone())
            .or_insert_with(|| Value::Array(Vec::new()));
        if let Some(arr) = entry.as_array_mut() {
            arr.push(Value::Object(grp));
        }
    }
    doc.set(&["hooks"], Value::Object(hooks));
    doc.save(&path)
}

/// Fichier `.claude.json` à lire/écrire pour ce home : premier candidat existant
/// (in-home prioritaire, puis hérité voisin), sinon `<home>/.claude.json` par défaut.
pub fn mcp_config_path(home: &ClaudeHome) -> PathBuf {
    for cand in mcp_config_candidates(home) {
        if cand.is_file() {
            return cand;
        }
    }
    home.base.join(".claude.json")
}

fn read_pairs(v: Option<&Value>) -> Vec<(String, String)> {
    v.and_then(|o| o.as_object())
        .map(|o| {
            o.iter()
                .filter_map(|(k, val)| val.as_str().map(|s| (k.clone(), s.to_string())))
                .collect()
        })
        .unwrap_or_default()
}

fn pairs_to_object(pairs: &[(String, String)]) -> Option<Value> {
    let mut m = Map::new();
    for (k, v) in pairs {
        if !k.trim().is_empty() {
            m.insert(k.clone(), Value::String(v.clone()));
        }
    }
    if m.is_empty() {
        None
    } else {
        Some(Value::Object(m))
    }
}

/// Lit les serveurs MCP de portée utilisateur (`mcpServers` racine) du fichier résolu.
pub fn read_user_mcp_servers(home: &ClaudeHome) -> Vec<McpServer> {
    let Some(v) = load_json(&mcp_config_path(home)) else {
        return Vec::new();
    };
    let Some(servers) = v.get("mcpServers").and_then(|m| m.as_object()) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for (name, def) in servers {
        let transport = match def.get("type").and_then(|t| t.as_str()) {
            Some("http") => McpTransport::Http,
            Some("sse") => McpTransport::Sse,
            _ => McpTransport::Stdio,
        };
        out.push(McpServer {
            name: name.clone(),
            transport,
            command: def.get("command").and_then(|c| c.as_str()).unwrap_or("").to_string(),
            args: def
                .get("args")
                .and_then(|a| a.as_array())
                .map(|a| a.iter().filter_map(|x| x.as_str().map(String::from)).collect())
                .unwrap_or_default(),
            env: read_pairs(def.get("env")),
            url: def.get("url").and_then(|u| u.as_str()).unwrap_or("").to_string(),
            headers: read_pairs(def.get("headers")),
        });
    }
    out
}

/// Réécrit la clé racine `mcpServers` du `.claude.json` résolu à partir du modèle.
/// Préserve toutes les autres clés ; backup + écriture atomique via SettingsDoc.
pub fn write_user_mcp_servers(home: &ClaudeHome, servers: &[McpServer]) -> Result<()> {
    let path = mcp_config_path(home);
    let mut doc = SettingsDoc::load(&path)?;
    if servers.is_empty() {
        doc.unset(&["mcpServers"]);
        return doc.save(&path);
    }
    let mut map: Map<String, Value> = Map::new();
    for s in servers {
        let mut o = Map::new();
        match s.transport {
            McpTransport::Stdio => {
                o.insert("type".to_string(), Value::String("stdio".to_string()));
                o.insert("command".to_string(), Value::String(s.command.clone()));
                if !s.args.is_empty() {
                    o.insert(
                        "args".to_string(),
                        Value::Array(s.args.iter().map(|a| Value::String(a.clone())).collect()),
                    );
                }
                if let Some(env) = pairs_to_object(&s.env) {
                    o.insert("env".to_string(), env);
                }
            }
            McpTransport::Http | McpTransport::Sse => {
                let t = if matches!(s.transport, McpTransport::Http) {
                    "http"
                } else {
                    "sse"
                };
                o.insert("type".to_string(), Value::String(t.to_string()));
                o.insert("url".to_string(), Value::String(s.url.clone()));
                if let Some(h) = pairs_to_object(&s.headers) {
                    o.insert("headers".to_string(), h);
                }
            }
        }
        map.insert(s.name.clone(), Value::Object(o));
    }
    doc.set(&["mcpServers"], Value::Object(map));
    doc.save(&path)
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

    #[test]
    fn read_hook_groups_parses_event_matcher_commands() {
        let settings = r#"{
            "hooks": {
                "PreToolUse": [
                    {"matcher":"Bash","hooks":[{"type":"command","command":"echo a","timeout":30}]}
                ],
                "SessionStart": [
                    {"hooks":[{"type":"command","command":"echo b"},{"type":"command","command":"echo c"}]}
                ]
            }
        }"#;
        let (_d, home) = home_with(&[("settings.json", settings)]);
        let groups = read_hook_groups(&home);
        assert_eq!(groups.len(), 2);
        assert_eq!(groups[0].event, "PreToolUse");
        assert_eq!(groups[0].matcher.as_deref(), Some("Bash"));
        assert_eq!(groups[0].commands.len(), 1);
        assert_eq!(groups[0].commands[0].command, "echo a");
        assert_eq!(groups[0].commands[0].timeout, Some(30));
        assert_eq!(groups[1].event, "SessionStart");
        assert_eq!(groups[1].matcher, None);
        assert_eq!(groups[1].commands.len(), 2);
    }

    #[test]
    fn write_hooks_round_trips_and_preserves_other_settings() {
        let settings = r#"{"includeCoAuthoredBy":false,"hooks":{"Stop":[{"hooks":[{"type":"command","command":"old"}]}]}}"#;
        let (_d, home) = home_with(&[("settings.json", settings)]);

        let groups = vec![
            HookGroup {
                event: "PreToolUse".into(),
                matcher: Some("Bash".into()),
                commands: vec![HookCommand {
                    kind: "command".into(),
                    command: "echo hi".into(),
                    timeout: Some(15),
                }],
            },
            HookGroup {
                event: "PreToolUse".into(),
                matcher: None,
                commands: vec![HookCommand {
                    kind: "command".into(),
                    command: "echo two".into(),
                    timeout: None,
                }],
            },
        ];
        write_hooks(&home, &groups).unwrap();

        // Relecture : deux groupes sous PreToolUse, dans l'ordre.
        let back = read_hook_groups(&home);
        assert_eq!(back.len(), 2);
        assert_eq!(back[0].event, "PreToolUse");
        assert_eq!(back[0].matcher.as_deref(), Some("Bash"));
        assert_eq!(back[0].commands[0].timeout, Some(15));
        assert_eq!(back[1].matcher, None);
        // Autre réglage préservé.
        let doc = crate::settings::SettingsDoc::load(&home.settings_file()).unwrap();
        assert_eq!(doc.get_bool(&["includeCoAuthoredBy"]), Some(false));
    }

    #[test]
    fn set_plugin_enabled_writes_flag_and_preserves_others() {
        let settings = r#"{"includeCoAuthoredBy":true,"enabledPlugins":{"foo@m":true}}"#;
        let (_d, home) = home_with(&[("settings.json", settings)]);

        set_plugin_enabled(&home, "foo@m", false).unwrap();
        set_plugin_enabled(&home, "bar@m", true).unwrap();

        let ext = read_extensions(&home);
        let foo = ext.plugins.iter().find(|p| p.name == "foo@m").unwrap();
        assert!(!foo.enabled);
        let bar = ext.plugins.iter().find(|p| p.name == "bar@m").unwrap();
        assert!(bar.enabled);
        let doc = crate::settings::SettingsDoc::load(&home.settings_file()).unwrap();
        assert_eq!(doc.get_bool(&["includeCoAuthoredBy"]), Some(true));
    }

    #[test]
    fn read_user_mcp_servers_parses_stdio_and_http() {
        let cfg = r#"{
            "mcpServers": {
                "fs": {"type":"stdio","command":"npx","args":["-y","server-fs"],"env":{"TOKEN":"x"}},
                "db": {"type":"http","url":"http://localhost:1","headers":{"Authorization":"Bearer y"}}
            }
        }"#;
        let (_d, home) = home_with(&[(".claude.json", cfg)]);
        let servers = read_user_mcp_servers(&home);
        assert_eq!(servers.len(), 2);
        let fs = servers.iter().find(|s| s.name == "fs").unwrap();
        assert!(matches!(fs.transport, McpTransport::Stdio));
        assert_eq!(fs.command, "npx");
        assert_eq!(fs.args, vec!["-y", "server-fs"]);
        assert_eq!(fs.env, vec![("TOKEN".to_string(), "x".to_string())]);
        let db = servers.iter().find(|s| s.name == "db").unwrap();
        assert!(matches!(db.transport, McpTransport::Http));
        assert_eq!(db.url, "http://localhost:1");
        assert_eq!(db.headers, vec![("Authorization".to_string(), "Bearer y".to_string())]);
    }

    #[test]
    fn mcp_config_path_prefers_existing_in_home() {
        // in-home .claude.json présent → choisi.
        let (_d, home) = home_with(&[(".claude.json", "{}")]);
        assert_eq!(mcp_config_path(&home), home.base.join(".claude.json"));
    }

    #[test]
    fn write_user_mcp_servers_round_trips_and_preserves_other_keys() {
        let cfg = r#"{"numStartups":3,"mcpServers":{"old":{"type":"stdio","command":"x"}}}"#;
        let (_d, home) = home_with(&[(".claude.json", cfg)]);

        let servers = vec![
            McpServer {
                name: "fs".into(),
                transport: McpTransport::Stdio,
                command: "npx".into(),
                args: vec!["-y".into(), "server-fs".into()],
                env: vec![("TOKEN".into(), "x".into())],
                ..Default::default()
            },
            McpServer {
                name: "db".into(),
                transport: McpTransport::Http,
                url: "http://localhost:1".into(),
                headers: vec![("Authorization".into(), "Bearer y".into())],
                ..Default::default()
            },
        ];
        write_user_mcp_servers(&home, &servers).unwrap();

        let back = read_user_mcp_servers(&home);
        assert_eq!(back.len(), 2);
        let fs = back.iter().find(|s| s.name == "fs").unwrap();
        assert_eq!(fs.args, vec!["-y", "server-fs"]);
        assert_eq!(fs.env, vec![("TOKEN".to_string(), "x".to_string())]);
        let db = back.iter().find(|s| s.name == "db").unwrap();
        assert!(matches!(db.transport, McpTransport::Http));
        assert_eq!(db.url, "http://localhost:1");
        // Autre clé racine préservée.
        let doc = crate::settings::SettingsDoc::load(&mcp_config_path(&home)).unwrap();
        assert_eq!(doc.get_i64(&["numStartups"]), Some(3));
    }

    #[test]
    fn write_user_mcp_servers_empty_removes_key() {
        let cfg = r#"{"numStartups":1,"mcpServers":{"old":{"command":"x"}}}"#;
        let (_d, home) = home_with(&[(".claude.json", cfg)]);
        write_user_mcp_servers(&home, &[]).unwrap();
        assert!(read_user_mcp_servers(&home).is_empty());
        let doc = crate::settings::SettingsDoc::load(&mcp_config_path(&home)).unwrap();
        assert!(doc.get(&["mcpServers"]).is_none());
        assert_eq!(doc.get_i64(&["numStartups"]), Some(1));
    }
}
