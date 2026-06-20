//! Cœur logique de Claudine : lecture/écriture de la structure `~/.claude`.

pub mod error;
pub mod home;
pub mod pathcodec;
pub mod model;
pub mod scan;
pub mod manifest;
pub mod export;
pub mod remap;
pub mod import;

pub use error::{CoreError, Report, Result};
pub use home::ClaudeHome;
pub use pathcodec::encode_cwd;
pub use model::{Project, SessionMeta};
pub use scan::{read_session_meta, scan_projects};
pub use manifest::{Manifest, ManifestProject, SCHEMA_VERSION};
pub use export::{export, ExportOptions};
pub use remap::{rewrite_jsonl_line, RemapRule, RemapTable};
pub use import::{dry_run, read_manifest, ImportOptions};

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
