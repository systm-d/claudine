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

/// Met **tout le dossier projet** `projects/<encoded>` à la corbeille
/// (récupérable), sous `trash/<horodatage>/<encoded>/`. Les sessions y restent
/// individuellement restaurables (la disposition correspond à `list_trash`).
/// Ne supprime jamais définitivement. Renvoie le chemin en corbeille.
pub fn trash_project(home_base: &Path, encoded: &str) -> Result<PathBuf> {
    let src = home_base.join("projects").join(encoded);
    if !src.exists() {
        return Err(CoreError::BundleFormat(format!(
            "projet introuvable : {}",
            src.display()
        )));
    }
    let dest_root = home_base.join("trash").join(nanos().to_string());
    fs::create_dir_all(&dest_root).map_err(|e| CoreError::io(&dest_root, e))?;
    let dest = dest_root.join(encoded);
    // `rename` si possible, sinon copie récursive + suppression (cross-device).
    if fs::rename(&src, &dest).is_err() {
        copy_dir_all(&src, &dest)?;
        fs::remove_dir_all(&src).map_err(|e| CoreError::io(&src, e))?;
    }
    Ok(dest)
}

fn copy_dir_all(src: &Path, dst: &Path) -> Result<()> {
    fs::create_dir_all(dst).map_err(|e| CoreError::io(dst, e))?;
    for entry in fs::read_dir(src).map_err(|e| CoreError::io(src, e))? {
        let entry = entry.map_err(|e| CoreError::io(src, e))?;
        let from = entry.path();
        let to = dst.join(entry.file_name());
        if from.is_dir() {
            copy_dir_all(&from, &to)?;
        } else {
            fs::copy(&from, &to).map_err(|e| CoreError::io(&from, e))?;
        }
    }
    Ok(())
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

/// Une entrée de corbeille = un évènement de suppression : le dossier
/// `<home>/trash/<horodatage>/<encoded>/` et son contenu (une session, un
/// projet entier, ou un projet vide). Restaurée / purgée comme un tout.
#[derive(Debug, Clone)]
pub struct TrashItem {
    /// Dossier `trash/<horodatage>/<encoded>`.
    pub dir: PathBuf,
    pub encoded: String,
    /// Nombre de sessions (`.jsonl`) contenues.
    pub sessions: usize,
    /// Nombre total de fichiers contenus.
    pub files: usize,
    pub size: u64,
    /// Id de session si l'entrée n'en contient qu'une (pour l'affichage).
    pub sample: Option<String>,
}

/// Élague les dossiers vides en remontant depuis `start`, sans jamais retirer
/// le dossier `trash` lui-même.
fn prune_empty_dirs(start: &Path) {
    let mut cur = Some(start.to_path_buf());
    while let Some(d) = cur {
        if d.file_name().map(|n| n == "trash").unwrap_or(false) {
            break;
        }
        let empty = match fs::read_dir(&d) {
            Ok(mut rd) => rd.next().is_none(),
            Err(_) => false,
        };
        if !empty || fs::remove_dir(&d).is_err() {
            break; // non vide, illisible, ou suppression impossible : on s'arrête.
        }
        cur = d.parent().map(|p| p.to_path_buf());
    }
}

/// Liste les entrées de la corbeille d'un home (une par dossier supprimé).
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
            let dir = enc.path();
            if !dir.is_dir() {
                continue;
            }
            let encoded = enc.file_name().to_string_lossy().into_owned();
            let mut sessions = 0usize;
            let mut files = 0usize;
            let mut size = 0u64;
            let mut sample = None;
            let Ok(fes) = fs::read_dir(&dir) else {
                continue;
            };
            for fe in fes.flatten() {
                let fp = fe.path();
                if !fp.is_file() {
                    continue;
                }
                files += 1;
                size += fe.metadata().map(|m| m.len()).unwrap_or(0);
                if fp.extension().and_then(|e| e.to_str()) == Some("jsonl") {
                    sessions += 1;
                    sample = fp.file_stem().map(|s| s.to_string_lossy().into_owned());
                }
            }
            if files == 0 {
                continue;
            }
            items.push(TrashItem {
                dir,
                encoded,
                sessions,
                files,
                size,
                sample: if sessions == 1 { sample } else { None },
            });
        }
    }
    items.sort_by(|a, b| a.dir.cmp(&b.dir));
    items
}

/// Restaure une entrée de corbeille vers `<home>/projects/<encoded>/` (tous ses
/// fichiers). Échoue **sans rien toucher** si un fichier de destination existe déjà.
pub fn restore_trash_entry(trash_dir: &Path, home_base: &Path) -> Result<PathBuf> {
    let encoded = trash_dir
        .file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .ok_or_else(|| CoreError::BundleFormat("structure de corbeille invalide".to_string()))?;
    let files: Vec<PathBuf> = fs::read_dir(trash_dir)
        .map_err(|e| CoreError::io(trash_dir, e))?
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.is_file())
        .collect();
    let dest_dir = home_base.join("projects").join(&encoded);
    // Pré-vérifie tous les conflits avant de bouger quoi que ce soit.
    for f in &files {
        if let Some(name) = f.file_name() {
            let dest = dest_dir.join(name);
            if dest.exists() {
                return Err(CoreError::Conflict(format!(
                    "déjà présent : {}",
                    dest.display()
                )));
            }
        }
    }
    fs::create_dir_all(&dest_dir).map_err(|e| CoreError::io(&dest_dir, e))?;
    for f in &files {
        if let Some(name) = f.file_name() {
            move_file(f, &dest_dir.join(name))?;
        }
    }
    prune_empty_dirs(trash_dir);
    Ok(dest_dir)
}

/// Supprime **définitivement** une entrée de corbeille (le dossier entier,
/// non récupérable) et élague les dossiers parents devenus vides.
pub fn purge_trash_item(trash_dir: &Path) -> Result<()> {
    if trash_dir.is_dir() {
        fs::remove_dir_all(trash_dir).map_err(|e| CoreError::io(trash_dir, e))?;
    } else if trash_dir.exists() {
        fs::remove_file(trash_dir).map_err(|e| CoreError::io(trash_dir, e))?;
    }
    if let Some(parent) = trash_dir.parent() {
        prune_empty_dirs(parent);
    }
    Ok(())
}

/// Vide **toute** la corbeille d'un home (non récupérable). Renvoie le nombre
/// d'entrées supprimées.
pub fn empty_trash(home_base: &Path) -> Result<usize> {
    let count = list_trash(home_base).len();
    let trash = home_base.join("trash");
    if trash.exists() {
        fs::remove_dir_all(&trash).map_err(|e| CoreError::io(&trash, e))?;
    }
    Ok(count)
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
        assert_eq!(items[0].sessions, 1);
        assert_eq!(items[0].sample.as_deref(), Some("abc"));

        let restored = restore_trash_entry(&items[0].dir, base).unwrap();
        assert_eq!(restored, pdir);
        assert!(sess.exists(), "session restaurée à sa place");
        assert!(!items[0].dir.exists(), "retirée de la corbeille");
    }

    #[test]
    fn trash_project_moves_whole_dir_and_sessions_stay_restorable() {
        let dir = tempfile::tempdir().unwrap();
        let base = dir.path();
        let pdir = base.join("projects").join("-home-x-proj");
        fs::create_dir_all(&pdir).unwrap();
        fs::write(pdir.join("a.jsonl"), "{\"cwd\":\"/home/x/proj\"}\n").unwrap();
        fs::write(pdir.join("b.jsonl"), "{\"cwd\":\"/home/x/proj\"}\n").unwrap();
        // Fichier auxiliaire (non-session) toléré.
        fs::write(pdir.join("sessions-index.json"), "{}").unwrap();

        trash_project(base, "-home-x-proj").unwrap();
        assert!(!pdir.exists(), "le dossier projet a disparu de projects/");

        // Une seule entrée (le projet), restaurée comme un tout : 2 sessions + index.
        let items = list_trash(base);
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].encoded, "-home-x-proj");
        assert_eq!(items[0].sessions, 2);
        assert_eq!(items[0].files, 3);

        restore_trash_entry(&items[0].dir, base).unwrap();
        assert!(pdir.join("a.jsonl").exists());
        assert!(pdir.join("b.jsonl").exists());
        assert!(pdir.join("sessions-index.json").exists());
        assert!(list_trash(base).is_empty(), "corbeille vidée après restauration");
    }

    #[test]
    fn trash_project_handles_empty_project() {
        let dir = tempfile::tempdir().unwrap();
        let base = dir.path();
        let pdir = base.join("projects").join("-home-kdelfour");
        fs::create_dir_all(&pdir).unwrap();
        // Projet « vide » : aucun .jsonl, juste un index (cas du bug ~ (0 sess.)).
        fs::write(pdir.join("sessions-index.json"), "{}").unwrap();

        let dest = trash_project(base, "-home-kdelfour").unwrap();
        assert!(!pdir.exists(), "projet vide retiré");
        assert!(dest.join("sessions-index.json").exists(), "contenu préservé en corbeille");
    }

    #[test]
    fn purge_trash_item_deletes_and_prunes_empty_dirs() {
        let dir = tempfile::tempdir().unwrap();
        let base = dir.path();
        let pdir = base.join("projects").join("-home-x-proj");
        fs::create_dir_all(&pdir).unwrap();
        let sess = pdir.join("abc.jsonl");
        fs::write(&sess, "{\"cwd\":\"/home/x/proj\"}\n").unwrap();

        trash_session(base, "-home-x-proj", &sess).unwrap();
        let items = list_trash(base);
        assert_eq!(items.len(), 1);
        let ts_dir = items[0].dir.parent().unwrap().to_path_buf();

        purge_trash_item(&items[0].dir).unwrap();

        assert!(!items[0].dir.exists(), "entrée supprimée définitivement");
        assert!(!ts_dir.exists(), "dossiers parents vides élagués");
        assert!(base.join("trash").exists(), "le dossier trash subsiste");
        assert!(list_trash(base).is_empty());
    }

    #[test]
    fn empty_trash_removes_everything_and_counts() {
        let dir = tempfile::tempdir().unwrap();
        let base = dir.path();
        for (enc, id) in [("-home-x-a", "s1"), ("-home-x-b", "s2")] {
            let pdir = base.join("projects").join(enc);
            fs::create_dir_all(&pdir).unwrap();
            let sess = pdir.join(format!("{id}.jsonl"));
            fs::write(&sess, "{\"cwd\":\"/home/x\"}\n").unwrap();
            trash_session(base, enc, &sess).unwrap();
        }
        assert_eq!(list_trash(base).len(), 2);

        let n = empty_trash(base).unwrap();
        assert_eq!(n, 2, "compte les sessions vidées");
        assert!(list_trash(base).is_empty());
        assert!(!base.join("trash").exists());
        // Idempotent : vider une corbeille déjà absente ne casse rien.
        assert_eq!(empty_trash(base).unwrap(), 0);
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
