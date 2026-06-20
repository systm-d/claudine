use std::path::{Path, PathBuf};

use crate::error::{CoreError, Result};

#[derive(Debug, Clone)]
pub struct ClaudeHome {
    pub base: PathBuf,
}

impl ClaudeHome {
    pub fn from_base(base: impl Into<PathBuf>) -> Self {
        Self { base: base.into() }
    }

    pub fn discover() -> Result<Self> {
        if let Ok(dir) = std::env::var("CLAUDE_CONFIG_DIR") {
            return Ok(Self::from_base(dir));
        }
        let home = std::env::var("HOME").map_err(|_| {
            CoreError::io(
                "<HOME>",
                std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "variable HOME absente",
                ),
            )
        })?;
        Ok(Self::from_base(Path::new(&home).join(".claude")))
    }

    pub fn projects_dir(&self) -> PathBuf {
        self.base.join("projects")
    }
    pub fn todos_dir(&self) -> PathBuf {
        self.base.join("todos")
    }
    pub fn plugins_dir(&self) -> PathBuf {
        self.base.join("plugins")
    }
    pub fn memory_file(&self) -> PathBuf {
        self.base.join("CLAUDE.md")
    }
    pub fn settings_file(&self) -> PathBuf {
        self.base.join("settings.json")
    }
    pub fn settings_local_file(&self) -> PathBuf {
        self.base.join("settings.local.json")
    }
    pub fn history_file(&self) -> PathBuf {
        self.base.join("history.jsonl")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_base_builds_subpaths() {
        let h = ClaudeHome::from_base("/x/.claude");
        assert_eq!(h.projects_dir(), std::path::Path::new("/x/.claude/projects"));
        assert_eq!(h.settings_file(), std::path::Path::new("/x/.claude/settings.json"));
        assert_eq!(h.history_file(), std::path::Path::new("/x/.claude/history.jsonl"));
    }

    #[test]
    fn discover_respects_env() {
        // CLAUDE_CONFIG_DIR est global au process : on capture le résultat et on
        // retire la variable AVANT l'assertion, pour ne pas la laisser fuiter si
        // l'assertion panique (teardown-on-panic).
        std::env::set_var("CLAUDE_CONFIG_DIR", "/custom/dir");
        let base = ClaudeHome::discover().unwrap().base;
        std::env::remove_var("CLAUDE_CONFIG_DIR");
        assert_eq!(base, std::path::Path::new("/custom/dir"));
    }
}
