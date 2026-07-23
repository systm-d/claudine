use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionMeta {
    pub id: String,
    pub path: PathBuf,
    pub cwd: Option<String>,
    pub message_count: usize,
    pub first_ts: Option<String>,
    pub last_ts: Option<String>,
    pub size: u64,
    /// Titre lisible de la session : le `summary` (renommage ou résumé
    /// automatique) enregistré par Claude Code dans le `.jsonl`. `None` si la
    /// session n'a jamais été nommée/résumée.
    pub title: Option<String>,
}

impl SessionMeta {
    /// Libellé d'affichage : le titre s'il existe, sinon l'id court (8 car.).
    pub fn display_label(&self) -> String {
        match &self.title {
            Some(t) if !t.trim().is_empty() => t.clone(),
            _ => self.id.chars().take(8).collect(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Project {
    pub encoded_name: String,
    pub cwd: Option<String>,
    pub sessions: Vec<SessionMeta>,
}
