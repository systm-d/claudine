use std::collections::BTreeMap;
use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum CoreError {
    #[error("erreur d'E/S sur {path}: {source}")]
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("erreur JSON dans {file} ligne {line}: {source}")]
    JsonParse {
        file: PathBuf,
        line: usize,
        source: serde_json::Error,
    },
    #[error("version de manifest non supportée: {0}")]
    ManifestVersion(u32),
    #[error("remap incomplet: aucune cible pour {0}")]
    RemapIncomplete(String),
    #[error("conflit: {0}")]
    Conflict(String),
    #[error("bundle invalide: {0}")]
    BundleFormat(String),
}

impl CoreError {
    pub fn io(path: impl Into<PathBuf>, source: std::io::Error) -> Self {
        CoreError::Io {
            path: path.into(),
            source,
        }
    }
}

pub type Result<T> = std::result::Result<T, CoreError>;

#[derive(Debug, Default, Clone)]
pub struct Report {
    pub warnings: Vec<String>,
    pub counts: BTreeMap<String, usize>,
}

impl Report {
    pub fn warn(&mut self, msg: impl Into<String>) {
        self.warnings.push(msg.into());
    }

    pub fn bump(&mut self, key: &str, n: usize) {
        *self.counts.entry(key.to_string()).or_default() += n;
    }

    pub fn count(&self, key: &str) -> usize {
        self.counts.get(key).copied().unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn report_accumulates() {
        let mut r = Report::default();
        r.bump("sessions", 2);
        r.bump("sessions", 3);
        r.warn("ligne corrompue");
        assert_eq!(r.count("sessions"), 5);
        assert_eq!(r.count("absent"), 0);
        assert_eq!(r.warnings, vec!["ligne corrompue".to_string()]);
    }

    #[test]
    fn io_helper_sets_path() {
        let err = CoreError::io(
            "/tmp/x",
            std::io::Error::new(std::io::ErrorKind::NotFound, "nope"),
        );
        assert!(format!("{err}").contains("/tmp/x"));
    }
}
