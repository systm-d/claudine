use serde::{Deserialize, Serialize};

pub const SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ManifestProject {
    pub encoded_name: String,
    pub cwd: Option<String>,
    pub session_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Manifest {
    pub schema_version: u32,
    pub created_at: String,
    pub source_hostname: String,
    pub source_home: String,
    pub projects: Vec<ManifestProject>,
    pub included_categories: Vec<String>,
    pub excluded: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manifest_round_trips_json() {
        let m = Manifest {
            schema_version: SCHEMA_VERSION,
            created_at: "1750000000".to_string(),
            source_hostname: "pc1".to_string(),
            source_home: "/home/old/.claude".to_string(),
            projects: vec![ManifestProject {
                encoded_name: "-home-old-proj".to_string(),
                cwd: Some("/home/old/proj".to_string()),
                session_ids: vec!["abc".to_string()],
            }],
            included_categories: vec!["sessions".to_string()],
            excluded: vec![".credentials.json".to_string()],
        };
        let json = serde_json::to_string(&m).unwrap();
        let back: Manifest = serde_json::from_str(&json).unwrap();
        assert_eq!(m, back);
    }
}
