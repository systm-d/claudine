use std::fs;
use std::path::PathBuf;

/// Encode un chemin absolu en nom de dossier de projet à la mode Claude Code :
/// chaque `/` et chaque `.` deviennent `-`. L'opération est volontairement
/// non réversible (la source de vérité du `cwd` est le champ interne des `.jsonl`).
pub fn encode_cwd(cwd: &str) -> String {
    cwd.chars()
        .map(|c| if c == '/' || c == '.' { '-' } else { c })
        .collect()
}

/// Encode un **segment** de chemin (sans `/`) : seuls les `.` deviennent `-`.
fn encode_segment(segment: &str) -> String {
    segment.replace('.', "-")
}

/// Tente de reconstruire le chemin absolu réel à partir d'un nom de dossier
/// encodé, en **sondant le système de fichiers** (le `-` encode `/`, `.` ou un
/// vrai `-`, donc le décodage est ambigu sans le disque). Descend depuis `/` en
/// faisant correspondre, à chaque niveau, le plus long groupe de jetons à une
/// entrée existante. Renvoie `None` si le chemin n'existe plus / est irrésoluble.
pub fn decode_encoded_to_path(encoded: &str) -> Option<String> {
    let body = encoded.strip_prefix('-')?;
    let tokens: Vec<&str> = body.split('-').collect();
    if tokens.is_empty() {
        return None;
    }

    let mut current = PathBuf::from("/");
    let mut i = 0;
    while i < tokens.len() {
        let names: Vec<String> = fs::read_dir(&current)
            .ok()?
            .filter_map(|e| e.ok())
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .collect();

        // Essaie le plus long groupe de jetons d'abord (gère `Delfour.co`,
        // `generic-rag`, etc.).
        let mut matched: Option<(String, usize)> = None;
        for k in (1..=tokens.len() - i).rev() {
            let candidate = tokens[i..i + k].join("-");
            if let Some(name) = names.iter().find(|n| encode_segment(n) == candidate) {
                matched = Some((name.clone(), k));
                break;
            }
        }
        let (name, k) = matched?;
        current.push(name);
        i += k;
    }
    Some(current.to_string_lossy().into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encodes_slashes_and_dots() {
        assert_eq!(encode_cwd("/home/kdelfour"), "-home-kdelfour");
        assert_eq!(
            encode_cwd("/home/kdelfour/Workspace/Professionel/Delfour.co/system/claude-tui"),
            "-home-kdelfour-Workspace-Professionel-Delfour-co-system-claude-tui"
        );
    }

    #[test]
    fn preserves_existing_dashes() {
        // ambiguïté assumée : un '-' réel reste un '-'
        assert_eq!(encode_cwd("/a/generic-rag"), "-a-generic-rag");
    }

    #[test]
    fn decode_probes_filesystem_for_dots_and_dashes() {
        let dir = tempfile::tempdir().unwrap();
        // Reproduit les cas ambigus : un segment avec un '.' et un avec un '-'.
        let deep = dir.path().join("Delfour.co").join("generic-rag");
        std::fs::create_dir_all(&deep).unwrap();

        let encoded = encode_cwd(&deep.to_string_lossy());
        let decoded = decode_encoded_to_path(&encoded).unwrap();
        assert_eq!(decoded, deep.to_string_lossy());
    }

    #[test]
    fn decode_returns_none_for_missing_path() {
        assert_eq!(
            decode_encoded_to_path("-n-existe-vraiment-pas-claudine-xyz"),
            None
        );
    }
}
