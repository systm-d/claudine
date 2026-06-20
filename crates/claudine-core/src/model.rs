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
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Project {
    pub encoded_name: String,
    pub cwd: Option<String>,
    pub sessions: Vec<SessionMeta>,
}
