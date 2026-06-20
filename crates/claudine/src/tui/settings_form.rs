//! Formulaire d'édition des `settings.json` Claude Code de la home active.
//! S'appuie sur `claudine_core::SettingsDoc` (qui préserve les clés non
//! modélisées) et `settings_catalog()` (la liste des champs exposés).

use std::path::PathBuf;

use claudine_core::{settings_catalog, ClaudeHome, FieldKind, FieldSpec, SettingsDoc};

/// État d'édition d'une liste / map (StringList et KeyValue).
pub struct ListEdit {
    pub items: Vec<String>,
    pub idx: usize,
    /// Tampon de saisie d'un élément ; `None` = navigation dans la liste.
    pub input: Option<String>,
    /// `true` si la saisie ajoute un élément, `false` si elle en édite un.
    pub adding: bool,
}

/// Mode d'édition courant du formulaire.
pub enum Edit {
    None,
    /// Saisie d'une valeur scalaire (Text / Number / via clavier).
    Scalar(String),
    /// Édition d'une liste ou d'une map.
    List(ListEdit),
}

pub struct SettingsForm {
    doc: SettingsDoc,
    path: PathBuf,
    fields: Vec<FieldSpec>,
    idx: usize,
    edit: Edit,
    dirty: bool,
    raw: bool,
}

impl SettingsForm {
    /// Charge le `settings.json` de la home active.
    pub fn load(home: &ClaudeHome) -> SettingsForm {
        let path = home.settings_file();
        let doc = SettingsDoc::load(&path).unwrap_or_else(|_| SettingsDoc::empty());
        SettingsForm {
            doc,
            path,
            fields: settings_catalog(),
            idx: 0,
            edit: Edit::None,
            dirty: false,
            raw: false,
        }
    }

    // --- Accès lecture (pour le rendu) ---

    pub fn fields(&self) -> &[FieldSpec] {
        &self.fields
    }
    pub fn idx(&self) -> usize {
        self.idx
    }
    pub fn dirty(&self) -> bool {
        self.dirty
    }
    pub fn raw(&self) -> bool {
        self.raw
    }
    pub fn is_editing(&self) -> bool {
        !matches!(self.edit, Edit::None)
    }
    pub fn editing_scalar(&self) -> bool {
        matches!(self.edit, Edit::Scalar(_))
    }
    pub fn editing_list_input(&self) -> bool {
        matches!(&self.edit, Edit::List(l) if l.input.is_some())
    }
    pub fn scalar_buf(&self) -> Option<&str> {
        match &self.edit {
            Edit::Scalar(b) => Some(b.as_str()),
            Edit::List(l) => l.input.as_deref(),
            Edit::None => None,
        }
    }
    pub fn list_state(&self) -> Option<&ListEdit> {
        match &self.edit {
            Edit::List(l) => Some(l),
            _ => None,
        }
    }

    /// JSON brut courant (reflète les éditions non enregistrées).
    pub fn raw_lines(&self) -> Vec<String> {
        self.doc.to_pretty().lines().map(|l| l.to_string()).collect()
    }

    /// Valeur affichable d'un champ.
    pub fn value_display(&self, spec: &FieldSpec) -> String {
        match &spec.kind {
            FieldKind::Bool => match self.doc.get_bool(&spec.path) {
                Some(true) => "✓ activé".to_string(),
                Some(false) => "✗ désactivé".to_string(),
                None => "· (non défini)".to_string(),
            },
            FieldKind::Enum(_) | FieldKind::Text => self
                .doc
                .get_str(&spec.path)
                .map(|s| s.to_string())
                .unwrap_or_else(|| "· (non défini)".to_string()),
            FieldKind::Number => self
                .doc
                .get_i64(&spec.path)
                .map(|n| n.to_string())
                .unwrap_or_else(|| "· (non défini)".to_string()),
            FieldKind::StringList => {
                let l = self.doc.get_str_list(&spec.path).unwrap_or_default();
                if l.is_empty() {
                    "· (vide)".to_string()
                } else {
                    format!("[{}] {}", l.len(), l.join(", "))
                }
            }
            FieldKind::KeyValue => {
                let p = self.doc.get_pairs(&spec.path);
                if p.is_empty() {
                    "· (vide)".to_string()
                } else {
                    format!(
                        "[{}] {}",
                        p.len(),
                        p.iter()
                            .map(|(k, v)| format!("{k}={v}"))
                            .collect::<Vec<_>>()
                            .join(", ")
                    )
                }
            }
        }
    }

    // --- Navigation / actions (hors édition) ---

    pub fn move_field(&mut self, delta: i32) {
        if self.is_editing() {
            return;
        }
        self.idx = step(self.idx, delta, self.fields.len());
    }

    pub fn go_first(&mut self) {
        if !self.is_editing() {
            self.idx = 0;
        }
    }
    pub fn go_last(&mut self) {
        if !self.is_editing() {
            self.idx = self.fields.len().saturating_sub(1);
        }
    }

    pub fn toggle_raw(&mut self) {
        if !self.is_editing() {
            self.raw = !self.raw;
        }
    }

    /// Active le champ courant : toggle (Bool), cycle (Enum) ou entre en saisie
    /// (Text/Number/StringList/KeyValue).
    pub fn activate(&mut self) {
        let spec = self.fields[self.idx].clone();
        match &spec.kind {
            FieldKind::Bool => {
                let cur = self.doc.get_bool(&spec.path).unwrap_or(false);
                self.doc.set_bool(&spec.path, !cur);
                self.dirty = true;
            }
            FieldKind::Enum(opts) => self.cycle_enum(&spec.path, opts, true),
            FieldKind::Text | FieldKind::Number => {
                let cur = if matches!(spec.kind, FieldKind::Number) {
                    self.doc.get_i64(&spec.path).map(|n| n.to_string()).unwrap_or_default()
                } else {
                    self.doc.get_str(&spec.path).map(|s| s.to_string()).unwrap_or_default()
                };
                self.edit = Edit::Scalar(cur);
            }
            FieldKind::StringList => {
                let items = self.doc.get_str_list(&spec.path).unwrap_or_default();
                self.edit = Edit::List(ListEdit { items, idx: 0, input: None, adding: false });
            }
            FieldKind::KeyValue => {
                let items = self
                    .doc
                    .get_pairs(&spec.path)
                    .into_iter()
                    .map(|(k, v)| format!("{k}={v}"))
                    .collect();
                self.edit = Edit::List(ListEdit { items, idx: 0, input: None, adding: false });
            }
        }
    }

    /// Cycle un champ Enum (Left/Right).
    pub fn cycle(&mut self, forward: bool) {
        if self.is_editing() {
            return;
        }
        let spec = self.fields[self.idx].clone();
        if let FieldKind::Enum(opts) = &spec.kind {
            self.cycle_enum(&spec.path, opts, forward);
        }
    }

    fn cycle_enum(&mut self, path: &[String], opts: &[String], forward: bool) {
        if opts.is_empty() {
            return;
        }
        let cur = self.doc.get_str(path).unwrap_or("");
        let pos = opts.iter().position(|o| o == cur).unwrap_or(0);
        let n = opts.len();
        let next = if forward { (pos + 1) % n } else { (pos + n - 1) % n };
        let val = opts[next].clone();
        if val.is_empty() {
            self.doc.unset(path);
        } else {
            self.doc.set_str(path, &val);
        }
        self.dirty = true;
    }

    // --- Saisie (Scalar et input de liste) ---

    pub fn input_char(&mut self, c: char) {
        match &mut self.edit {
            Edit::Scalar(buf) => buf.push(c),
            Edit::List(l) => {
                if let Some(buf) = &mut l.input {
                    buf.push(c);
                }
            }
            Edit::None => {}
        }
    }

    pub fn input_backspace(&mut self) {
        match &mut self.edit {
            Edit::Scalar(buf) => {
                buf.pop();
            }
            Edit::List(l) => {
                if let Some(buf) = &mut l.input {
                    buf.pop();
                }
            }
            Edit::None => {}
        }
    }

    /// Annule la saisie en cours : pour un scalaire, ferme l'édition ; pour un
    /// input de liste, revient à la navigation dans la liste.
    pub fn input_cancel(&mut self) {
        match &mut self.edit {
            Edit::Scalar(_) => self.edit = Edit::None,
            Edit::List(l) => l.input = None,
            Edit::None => {}
        }
    }

    /// Valide la saisie en cours.
    pub fn input_commit(&mut self) {
        match &mut self.edit {
            Edit::Scalar(buf) => {
                let spec = self.fields[self.idx].clone();
                let trimmed = buf.trim().to_string();
                if trimmed.is_empty() {
                    self.doc.unset(&spec.path);
                    self.dirty = true;
                } else if matches!(spec.kind, FieldKind::Number) {
                    if let Ok(n) = trimmed.parse::<i64>() {
                        self.doc.set_i64(&spec.path, n);
                        self.dirty = true;
                    }
                    // nombre invalide → on ignore et on ferme la saisie
                } else {
                    self.doc.set_str(&spec.path, &trimmed);
                    self.dirty = true;
                }
                self.edit = Edit::None;
            }
            Edit::List(l) => {
                if let Some(buf) = l.input.take() {
                    let val = buf.trim().to_string();
                    if l.adding {
                        if !val.is_empty() {
                            l.items.push(val);
                            l.idx = l.items.len() - 1;
                        }
                    } else if l.idx < l.items.len() {
                        if val.is_empty() {
                            l.items.remove(l.idx);
                            l.idx = l.idx.min(l.items.len().saturating_sub(1));
                        } else {
                            l.items[l.idx] = val;
                        }
                    }
                    l.adding = false;
                }
            }
            Edit::None => {}
        }
    }

    // --- Édition de liste (navigation interne) ---

    pub fn list_move(&mut self, delta: i32) {
        if let Edit::List(l) = &mut self.edit {
            if l.input.is_none() {
                l.idx = step(l.idx, delta, l.items.len());
            }
        }
    }

    pub fn list_add(&mut self) {
        if let Edit::List(l) = &mut self.edit {
            if l.input.is_none() {
                l.input = Some(String::new());
                l.adding = true;
            }
        }
    }

    pub fn list_begin_edit(&mut self) {
        if let Edit::List(l) = &mut self.edit {
            if l.input.is_none() && l.idx < l.items.len() {
                l.input = Some(l.items[l.idx].clone());
                l.adding = false;
            }
        }
    }

    pub fn list_delete(&mut self) {
        if let Edit::List(l) = &mut self.edit {
            if l.input.is_none() && l.idx < l.items.len() {
                l.items.remove(l.idx);
                l.idx = l.idx.min(l.items.len().saturating_sub(1));
            }
        }
    }

    /// Termine l'édition de liste : réécrit la valeur dans le document.
    pub fn list_done(&mut self) {
        let spec = self.fields[self.idx].clone();
        if let Edit::List(l) = std::mem::replace(&mut self.edit, Edit::None) {
            match spec.kind {
                FieldKind::StringList => {
                    if l.items.is_empty() {
                        self.doc.unset(&spec.path);
                    } else {
                        self.doc.set_str_list(&spec.path, &l.items);
                    }
                }
                FieldKind::KeyValue => {
                    let pairs: Vec<(String, String)> = l
                        .items
                        .iter()
                        .map(|item| match item.split_once('=') {
                            Some((k, v)) => (k.trim().to_string(), v.trim().to_string()),
                            None => (item.trim().to_string(), String::new()),
                        })
                        .filter(|(k, _)| !k.is_empty())
                        .collect();
                    if pairs.is_empty() {
                        self.doc.unset(&spec.path);
                    } else {
                        self.doc.set_string_map(&spec.path, &pairs);
                    }
                }
                _ => {}
            }
            self.dirty = true;
        }
    }

    /// Enregistre le document (sauvegarde + écriture atomique). Renvoie un
    /// message de statut.
    pub fn save(&mut self) -> String {
        match self.doc.save(&self.path) {
            Ok(()) => {
                self.dirty = false;
                format!("Config enregistrée → {}", self.path.display())
            }
            Err(e) => format!("Échec enregistrement : {e}"),
        }
    }
}

fn step(idx: usize, delta: i32, len: usize) -> usize {
    if len == 0 {
        return 0;
    }
    let max = len - 1;
    if delta < 0 {
        idx.saturating_sub((-delta) as usize)
    } else {
        (idx + delta as usize).min(max)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn form_on_tempdir() -> (tempfile::TempDir, SettingsForm) {
        let dir = tempfile::tempdir().unwrap();
        let home = ClaudeHome::from_base(dir.path());
        (dir, SettingsForm::load(&home))
    }

    #[test]
    fn toggle_bool_marks_dirty_and_saves() {
        let (_d, mut form) = form_on_tempdir();
        // trouve un champ booléen
        let pos = form
            .fields()
            .iter()
            .position(|f| matches!(f.kind, FieldKind::Bool))
            .unwrap();
        // navigue jusqu'à lui
        form.idx = pos;
        form.activate(); // toggle → true
        assert!(form.dirty());
        let status = form.save();
        assert!(status.contains("enregistrée"), "statut = {status}");
        assert!(!form.dirty());
        assert!(form.path.exists());
    }

    #[test]
    fn enum_cycles_through_options() {
        let (_d, mut form) = form_on_tempdir();
        let pos = form
            .fields()
            .iter()
            .position(|f| f.path == ["permissions", "defaultMode"])
            .unwrap();
        form.idx = pos;
        // "" -> prompt
        form.cycle(true);
        // valeur écrite dans le doc
        assert_eq!(form.value_display(&form.fields()[pos]), "prompt");
    }

    #[test]
    fn stringlist_add_and_done_writes_doc() {
        let (_d, mut form) = form_on_tempdir();
        let pos = form
            .fields()
            .iter()
            .position(|f| f.path == ["permissions", "allow"])
            .unwrap();
        form.idx = pos;
        form.activate(); // entre en mode liste
        form.list_add(); // input vide
        for c in "Bash(ls)".chars() {
            form.input_char(c);
        }
        form.input_commit(); // ajoute l'élément
        form.list_done(); // réécrit dans le doc
        assert_eq!(form.value_display(&form.fields()[pos]), "[1] Bash(ls)");
    }
}
