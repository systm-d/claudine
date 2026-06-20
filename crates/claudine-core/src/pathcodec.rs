/// Encode un chemin absolu en nom de dossier de projet à la mode Claude Code :
/// chaque `/` et chaque `.` deviennent `-`. L'opération est volontairement
/// non réversible (la source de vérité du `cwd` est le champ interne des `.jsonl`).
pub fn encode_cwd(cwd: &str) -> String {
    cwd.chars()
        .map(|c| if c == '/' || c == '.' { '-' } else { c })
        .collect()
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
}
