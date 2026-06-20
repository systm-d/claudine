//! Opérations de ménage sur les sessions : mise en corbeille (récupérable) et
//! déplacement vers un autre projet/home (avec remap du `cwd`).

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::error::{CoreError, Result};
use crate::pathcodec::encode_cwd;
use crate::remap::{rewrite_jsonl_line, RemapRule, RemapTable};

fn nanos() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0)
}

/// Déplace un fichier : `rename` si possible, sinon copie + suppression
/// (robuste au cross-device).
fn move_file(src: &Path, dest: &Path) -> Result<()> {
    if fs::rename(src, dest).is_ok() {
        return Ok(());
    }
    fs::copy(src, dest).map_err(|e| CoreError::io(dest, e))?;
    fs::remove_file(src).map_err(|e| CoreError::io(src, e))?;
    Ok(())
}

/// Met une session à la **corbeille** du home (récupérable), en préservant
/// `<encoded>/<fichier>` sous `<home>/trash/<horodatage>/`. Renvoie le chemin
/// en corbeille. Ne supprime jamais définitivement.
pub fn trash_session(home_base: &Path, encoded: &str, session_path: &Path) -> Result<PathBuf> {
    let file_name = session_path
        .file_name()
        .ok_or_else(|| CoreError::BundleFormat("chemin de session invalide".to_string()))?;
    let dest_dir = home_base
        .join("trash")
        .join(nanos().to_string())
        .join(encoded);
    fs::create_dir_all(&dest_dir).map_err(|e| CoreError::io(&dest_dir, e))?;
    let dest = dest_dir.join(file_name);
    move_file(session_path, &dest)?;
    Ok(dest)
}

/// Déplace une session vers un autre projet (éventuellement un autre home) :
/// réécrit `old_cwd` → `target_cwd` dans le contenu, place le fichier dans le
/// dossier encodé de `target_cwd`, puis retire l'original. Renvoie la destination.
/// Échoue (sans rien casser) si la destination existe déjà.
pub fn move_session(
    session_path: &Path,
    old_cwd: Option<&str>,
    target_home_base: &Path,
    target_cwd: &str,
) -> Result<PathBuf> {
    let file_name = session_path
        .file_name()
        .ok_or_else(|| CoreError::BundleFormat("chemin de session invalide".to_string()))?;
    let content = fs::read_to_string(session_path).map_err(|e| CoreError::io(session_path, e))?;

    // Réécrit le `cwd` (et chemins absolus) si l'ancien cwd est connu et diffère.
    let new_content = match old_cwd {
        Some(old) if old != target_cwd => {
            let table = RemapTable::new(vec![RemapRule {
                from: old.to_string(),
                to: target_cwd.to_string(),
            }]);
            let mut out = Vec::new();
            for line in content.lines() {
                if line.trim().is_empty() {
                    out.push(String::new());
                    continue;
                }
                match rewrite_jsonl_line(line, &table) {
                    Ok((rewritten, _)) => out.push(rewritten),
                    Err(_) => out.push(line.to_string()),
                }
            }
            let mut joined = out.join("\n");
            if content.ends_with('\n') {
                joined.push('\n');
            }
            joined
        }
        _ => content,
    };

    let dest_dir = target_home_base
        .join("projects")
        .join(encode_cwd(target_cwd));
    fs::create_dir_all(&dest_dir).map_err(|e| CoreError::io(&dest_dir, e))?;
    let dest = dest_dir.join(file_name);
    if dest.exists() {
        return Err(CoreError::Conflict(format!(
            "destination déjà présente : {}",
            dest.display()
        )));
    }

    // Écriture atomique de la destination, puis retrait de l'original.
    let tmp = dest_dir.join(format!("{}.tmp", file_name.to_string_lossy()));
    fs::write(&tmp, new_content.as_bytes()).map_err(|e| CoreError::io(&tmp, e))?;
    fs::rename(&tmp, &dest).map_err(|e| CoreError::io(&dest, e))?;
    fs::remove_file(session_path).map_err(|e| CoreError::io(session_path, e))?;
    Ok(dest)
}

/// Une session en corbeille : `<home>/trash/<horodatage>/<encoded>/<fichier>`.
#[derive(Debug, Clone)]
pub struct TrashItem {
    pub path: PathBuf,
    pub encoded: String,
    pub file_name: String,
    pub size: u64,
}

/// Liste les sessions présentes dans la corbeille d'un home.
pub fn list_trash(home_base: &Path) -> Vec<TrashItem> {
    let trash = home_base.join("trash");
    let mut items = Vec::new();
    let Ok(ts_dirs) = fs::read_dir(&trash) else {
        return items;
    };
    for ts in ts_dirs.flatten() {
        if !ts.path().is_dir() {
            continue;
        }
        let Ok(enc_dirs) = fs::read_dir(ts.path()) else {
            continue;
        };
        for enc in enc_dirs.flatten() {
            if !enc.path().is_dir() {
                continue;
            }
            let encoded = enc.file_name().to_string_lossy().into_owned();
            let Ok(files) = fs::read_dir(enc.path()) else {
                continue;
            };
            for fe in files.flatten() {
                let fp = fe.path();
                if fp.extension().and_then(|e| e.to_str()) == Some("jsonl") {
                    items.push(TrashItem {
                        size: fe.metadata().map(|m| m.len()).unwrap_or(0),
                        path: fp,
                        encoded: encoded.clone(),
                        file_name: fe.file_name().to_string_lossy().into_owned(),
                    });
                }
            }
        }
    }
    items.sort_by(|a, b| a.path.cmp(&b.path));
    items
}

/// Restaure une session de la corbeille vers `<home>/projects/<encoded>/<fichier>`.
/// Échoue (sans rien casser) si la destination existe déjà.
pub fn restore_session(trash_path: &Path, home_base: &Path) -> Result<PathBuf> {
    let file_name = trash_path
        .file_name()
        .ok_or_else(|| CoreError::BundleFormat("chemin de corbeille invalide".to_string()))?;
    // L'`encoded` est le dossier parent du fichier en corbeille.
    let encoded = trash_path
        .parent()
        .and_then(|p| p.file_name())
        .map(|s| s.to_string_lossy().into_owned())
        .ok_or_else(|| CoreError::BundleFormat("structure de corbeille invalide".to_string()))?;
    let dest_dir = home_base.join("projects").join(&encoded);
    fs::create_dir_all(&dest_dir).map_err(|e| CoreError::io(&dest_dir, e))?;
    let dest = dest_dir.join(file_name);
    if dest.exists() {
        return Err(CoreError::Conflict(format!(
            "déjà présent : {}",
            dest.display()
        )));
    }
    move_file(trash_path, &dest)?;
    Ok(dest)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    #[test]
    fn trash_session_moves_to_recoverable_trash() {
        let dir = tempfile::tempdir().unwrap();
        let base = dir.path();
        let pdir = base.join("projects").join("-home-x-proj");
        fs::create_dir_all(&pdir).unwrap();
        let sess = pdir.join("abc.jsonl");
        fs::write(&sess, "{\"cwd\":\"/home/x/proj\"}\n").unwrap();

        let trashed = trash_session(base, "-home-x-proj", &sess).unwrap();

        assert!(!sess.exists(), "l'original doit avoir disparu");
        assert!(trashed.exists(), "le fichier en corbeille doit exister");
        assert!(trashed.starts_with(base.join("trash")));
        assert_eq!(
            fs::read_to_string(&trashed).unwrap(),
            "{\"cwd\":\"/home/x/proj\"}\n"
        );
    }

    #[test]
    fn move_session_rewrites_cwd_and_relocates() {
        let dir = tempfile::tempdir().unwrap();
        let base = dir.path();
        let pdir = base.join("projects").join("-home-old-a");
        fs::create_dir_all(&pdir).unwrap();
        let sess = pdir.join("s1.jsonl");
        fs::write(
            &sess,
            "{\"cwd\":\"/home/old/a\",\"x\":1}\n{\"cwd\":\"/home/old/a\"}\n",
        )
        .unwrap();

        // Déplace vers /home/old/b dans le même home.
        let dest = move_session(&sess, Some("/home/old/a"), base, "/home/old/b").unwrap();

        assert!(!sess.exists(), "original retiré");
        assert_eq!(
            dest,
            base.join("projects").join("-home-old-b").join("s1.jsonl")
        );
        let content = fs::read_to_string(&dest).unwrap();
        let first: Value = serde_json::from_str(content.lines().next().unwrap()).unwrap();
        assert_eq!(first["cwd"], "/home/old/b");
        assert!(content.ends_with('\n'), "newline final préservé");
    }

    #[test]
    fn trash_then_list_then_restore_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let base = dir.path();
        let pdir = base.join("projects").join("-home-x-proj");
        fs::create_dir_all(&pdir).unwrap();
        let sess = pdir.join("abc.jsonl");
        fs::write(&sess, "{\"cwd\":\"/home/x/proj\"}\n").unwrap();

        trash_session(base, "-home-x-proj", &sess).unwrap();
        assert!(!sess.exists());

        let items = list_trash(base);
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].encoded, "-home-x-proj");
        assert_eq!(items[0].file_name, "abc.jsonl");

        let restored = restore_session(&items[0].path, base).unwrap();
        assert_eq!(restored, sess);
        assert!(sess.exists(), "session restaurée à sa place");
        assert!(!items[0].path.exists(), "retirée de la corbeille");
    }

    #[test]
    fn move_session_refuses_existing_destination() {
        let dir = tempfile::tempdir().unwrap();
        let base = dir.path();
        let src_dir = base.join("projects").join("-home-old-a");
        fs::create_dir_all(&src_dir).unwrap();
        let sess = src_dir.join("s1.jsonl");
        fs::write(&sess, "{\"cwd\":\"/home/old/a\"}\n").unwrap();
        // Pré-crée la destination en conflit.
        let dst_dir = base.join("projects").join("-home-old-b");
        fs::create_dir_all(&dst_dir).unwrap();
        fs::write(dst_dir.join("s1.jsonl"), "DÉJÀ").unwrap();

        let err = move_session(&sess, Some("/home/old/a"), base, "/home/old/b");
        assert!(err.is_err(), "doit refuser d'écraser");
        assert!(sess.exists(), "l'original doit rester intact en cas de conflit");
    }
}
