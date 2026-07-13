//! Cœur logique de Claudine : lecture/écriture de la structure `~/.claude`.

pub mod config;
pub mod error;
pub mod export;
pub mod extensions;
pub mod home;
pub mod housekeeping;
pub mod import;
pub mod manifest;
pub mod marketplaces;
pub mod model;
pub mod pathcodec;
pub mod remap;
pub mod scan;
pub mod search;
pub mod settings;
pub mod tui;

pub use config::{ClaudineConfig, RegisteredHome, config_path, merge_registered};
pub use error::{CoreError, Report, Result};
pub use export::{ExportOptions, export};
pub use extensions::{
    Extensions, HookCommand, HookEntry, HookGroup, McpEntry, McpServer, McpTransport, PluginEntry,
    mcp_config_path, read_extensions, read_hook_groups, read_installed_plugins,
    read_user_mcp_servers, set_plugin_enabled, uninstall_plugin, write_hooks,
    write_user_mcp_servers,
};
pub use home::{ClaudeHome, discover_homes, discover_homes_in};
pub use housekeeping::{
    TrashItem, empty_trash, list_trash, move_session, purge_trash_item, restore_trash_entry,
    trash_project, trash_session,
};
pub use import::{ImportOptions, apply, dry_run, read_manifest};
pub use manifest::{Manifest, ManifestProject, SCHEMA_VERSION};
pub use marketplaces::{
    Marketplace, MarketplaceManifest, MarketplaceSource, PluginManifestEntry, PluginSource,
    add_marketplace, install_plugin, iso8601_utc, read_marketplace_manifest, read_marketplaces,
    remove_marketplace, update_marketplace,
};
pub use model::{Project, SessionMeta};
pub use pathcodec::{decode_encoded_to_path, encode_cwd};
pub use remap::{RemapRule, RemapTable, rewrite_jsonl_line};
pub use scan::{read_session_meta, scan_projects};
pub use search::find_in_session;
pub use settings::{FieldKind, FieldSpec, SettingsDoc, settings_catalog};

#[cfg(test)]
pub(crate) mod testkit {
    use std::fs;
    use std::path::Path;

    pub struct FakeHome {
        pub dir: tempfile::TempDir,
    }

    impl FakeHome {
        pub fn new() -> Self {
            let dir = tempfile::tempdir().unwrap();
            fs::create_dir_all(dir.path().join("projects")).unwrap();
            Self { dir }
        }

        pub fn base(&self) -> &Path {
            self.dir.path()
        }

        pub fn add_session(&self, encoded: &str, id: &str, lines: &[&str]) {
            let pdir = self.dir.path().join("projects").join(encoded);
            fs::create_dir_all(&pdir).unwrap();
            fs::write(pdir.join(format!("{id}.jsonl")), lines.join("\n")).unwrap();
        }

        #[allow(dead_code)]
        pub fn write_file(&self, rel: &str, content: &str) {
            let p = self.dir.path().join(rel);
            fs::create_dir_all(p.parent().unwrap()).unwrap();
            fs::write(p, content).unwrap();
        }
    }
}
