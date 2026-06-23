//! État applicatif de la TUI Claudine et logique de navigation.

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use claudine_core::{
    apply as import_apply, decode_encoded_to_path, discover_homes, dry_run as import_dry_run,
    empty_trash, export, find_in_session, list_trash, move_session, purge_trash_item,
    read_extensions, read_hook_groups, restore_trash_entry, scan_projects, trash_project,
    trash_session, write_hooks, ClaudeHome, ClaudineConfig, Extensions, ExportOptions,
    ImportOptions, Project, RemapTable, SessionMeta,
};
use serde_json::Value;

use crate::tui::hooks_editor::HooksEditor;
use crate::tui::settings_form::SettingsForm;

/// Sections de premier niveau, sélectionnables avec Tab / 1,2,3,4.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Section {
    Browse,
    Memory,
    Config,
    Extensions,
}

impl Section {
    pub fn title(self) -> &'static str {
        match self {
            Section::Browse => "Projets",
            Section::Memory => "Mémoire",
            Section::Config => "Config",
            Section::Extensions => "Extensions",
        }
    }

    pub fn index(self) -> usize {
        match self {
            Section::Browse => 0,
            Section::Memory => 1,
            Section::Config => 2,
            Section::Extensions => 3,
        }
    }

    pub fn next(self) -> Section {
        match self {
            Section::Browse => Section::Memory,
            Section::Memory => Section::Config,
            Section::Config => Section::Extensions,
            Section::Extensions => Section::Browse,
        }
    }
}

/// Panneau actif dans la section Browse.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Focus {
    Projects,
    Sessions,
}

/// Vue courante de la section Browse.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BrowseView {
    List,
    Transcript,
}

/// Une entrée de transcript prête à l'affichage.
#[derive(Debug, Clone)]
pub struct TranscriptEntry {
    /// Ligne d'en-tête : `▌ <type/role> · <timestamp>`.
    pub header: String,
    /// Corps textuel (peut contenir plusieurs lignes).
    pub body: String,
    /// Vrai si la ligne d'origine n'a pas pu être parsée.
    pub unparsable: bool,
}

/// Une cible de déplacement de session : un projet (cwd) dans un home donné.
#[derive(Debug, Clone)]
pub struct MoveTarget {
    pub label: String,
    pub cwd: String,
    pub home_base: PathBuf,
}

/// Un résultat de recherche : pointe vers une session des projets chargés.
#[derive(Debug, Clone)]
pub struct SearchHit {
    pub project_idx: usize,
    pub session_idx: usize,
    pub label: String,
    pub snippet: String,
}

/// État de la recherche : requête éditable + résultats recalculés en direct.
/// La frappe filtre instantanément sur le **chemin / id** ; `Tab` lance une
/// recherche **dans le contenu** (lecture des fichiers, plus coûteuse).
pub struct SearchState {
    pub query: String,
    pub results: Vec<SearchHit>,
    pub idx: usize,
    /// Vrai si les résultats incluent une recherche de contenu (via `Tab`).
    pub deep: bool,
}

/// Une entrée de corbeille affichable (avec son home d'origine pour restaurer).
#[derive(Debug, Clone)]
pub struct TrashEntry {
    /// Dossier de l'entrée en corbeille (`trash/<ts>/<encoded>`).
    pub dir: PathBuf,
    pub home_base: PathBuf,
    pub label: String,
}

/// Cible d'une suppression vers la corbeille : une session, ou un projet entier.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeleteKind {
    Session,
    Project,
}

/// Portée d'une purge définitive de la corbeille.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PurgeScope {
    /// Supprimer définitivement la session surlignée.
    One,
    /// Vider toute la corbeille (tous les homes).
    All,
}

/// État du viewer de corbeille.
pub struct TrashState {
    pub items: Vec<TrashEntry>,
    pub idx: usize,
    /// Confirmation de purge en attente ; `None` = navigation normale.
    pub confirm: Option<PurgeScope>,
}

/// Aperçu d'un import (résultat d'un `dry_run`).
#[derive(Debug, Clone, Default)]
pub struct ImportPreview {
    pub projects: usize,
    pub sessions_new: usize,
    pub sessions_conflict: usize,
}

/// État de l'assistant d'import : saisie du chemin de bundle puis aperçu.
pub struct ImportState {
    /// Chemin du bundle `.tar.gz` en cours de saisie.
    pub input: String,
    /// Aperçu calculé (présent une fois le bundle validé via `dry_run`).
    pub preview: Option<(PathBuf, ImportPreview)>,
    /// Écraser les sessions en conflit (sinon elles sont ignorées).
    pub overwrite: bool,
}

/// Mode de saisie du sélecteur de home : navigation, ou saisie d'un chemin.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PickerMode {
    /// Navigation dans la liste des homes.
    List,
    /// Saisie d'un chemin pour ajouter une home (contient le tampon courant).
    AddInput(String),
}

/// État central de l'application TUI.
pub struct App {
    /// Homes disponibles (au moins une). `active` indexe la home courante.
    pub homes: Vec<ClaudeHome>,
    pub active: usize,
    pub section: Section,
    pub should_quit: bool,
    pub show_help: bool,
    pub status: Option<String>,
    /// Fichier à éditer dans `$EDITOR` (traité par la boucle d'évènements).
    pub pending_edit: Option<PathBuf>,

    // --- Sélecteur de home ---
    pub show_picker: bool,
    pub picker_idx: usize,
    pub picker_mode: PickerMode,

    // --- Browse ---
    pub projects: Vec<Project>,
    /// Label du home d'origine de chaque projet (aligné sur `projects`).
    pub project_homes: Vec<String>,
    /// Base du home d'origine de chaque projet (aligné sur `projects`).
    pub project_home_bases: Vec<PathBuf>,
    /// Vue agrégée : projets de tous les homes à la fois.
    pub aggregate: bool,
    /// Homes repliés (par chemin de base) : leurs projets sont masqués dans la
    /// liste agrégée. Persiste entre rechargements (les homes ne changent pas).
    pub collapsed: HashSet<String>,
    pub browse_view: BrowseView,

    // --- Ménage (suppression / déplacement de sessions) ---
    /// Confirmation de suppression (corbeille) en attente ; `None` = aucune.
    pub confirm_delete: Option<DeleteKind>,
    /// Cibles de déplacement proposées (popup) ; `None` = popup fermé.
    pub move_targets: Option<Vec<MoveTarget>>,
    pub move_idx: usize,

    /// Recherche de session (saisie + résultats) ; `None` = fermée.
    pub search: Option<SearchState>,

    /// Viewer de corbeille (sessions supprimées, restaurables) ; `None` = fermé.
    pub trash_view: Option<TrashState>,
    /// Assistant d'import d'un bundle ; `None` = fermé.
    pub import: Option<ImportState>,
    /// Éditeur de hooks (modal) ; `None` = fermé.
    pub hooks_editor: Option<HooksEditor>,
    pub focus: Focus,
    pub project_idx: usize,
    pub session_idx: usize,

    // --- Transcript ---
    pub transcript: Vec<TranscriptEntry>,
    pub transcript_scroll: usize,
    /// Hauteur de la zone de transcript au dernier rendu (pour le clamp du scroll).
    pub transcript_viewport: usize,

    // --- Memory ---
    pub memory_lines: Vec<String>,
    pub memory_scroll: usize,
    pub memory_viewport: usize,

    // --- Config ---
    pub config_lines: Vec<String>,
    pub config_scroll: usize,
    pub config_viewport: usize,

    // --- Extensions (hooks / plugins / MCP) du home actif ---
    pub extensions: Extensions,
    pub ext_scroll: usize,
    pub ext_viewport: usize,
    /// Nombre de lignes rendues dans la section Extensions (pour le clamp Fin).
    pub ext_total: usize,

    // --- Formulaire de réglages (édite le settings.json de la home active) ---
    pub settings: SettingsForm,
}

impl App {
    /// Construit l'application à partir d'une liste de homes (la première est
    /// active). Une liste vide retombe sur `discover_homes()`, et à défaut sur
    /// `~/.claude`. Charge les projets / mémoire / config de la home active.
    pub fn with_homes(mut homes: Vec<ClaudeHome>) -> App {
        if homes.is_empty() {
            homes = discover_homes();
        }
        if homes.is_empty() {
            homes.push(ClaudeHome::from_base(
                std::env::var("HOME")
                    .map(|h| PathBuf::from(h).join(".claude"))
                    .unwrap_or_else(|_| PathBuf::from(".claude")),
            ));
        }

        // Plusieurs homes → vue agrégée par défaut (tous visibles dès le départ).
        let aggregate = homes.len() > 1;
        let mut projects = Vec::new();
        let mut project_homes = Vec::new();
        let mut project_home_bases = Vec::new();
        let scan_set: Vec<&ClaudeHome> = if aggregate {
            homes.iter().collect()
        } else {
            vec![&homes[0]]
        };
        for h in scan_set {
            if let Ok(ps) = scan_projects(h) {
                for p in ps {
                    project_homes.push(h.label.clone());
                    project_home_bases.push(h.base.clone());
                    projects.push(p);
                }
            }
        }
        let home = &homes[0];
        let memory_lines = read_file_lines(home.memory_file(), "(aucune mémoire utilisateur)");
        let config_lines = build_config_lines(home);
        let extensions = read_extensions(home);
        let settings = SettingsForm::load(home);
        App {
            homes,
            active: 0,
            section: Section::Browse,
            should_quit: false,
            show_help: false,
            status: None,
            pending_edit: None,
            show_picker: false,
            picker_idx: 0,
            picker_mode: PickerMode::List,
            projects,
            project_homes,
            project_home_bases,
            aggregate,
            collapsed: HashSet::new(),
            browse_view: BrowseView::List,
            confirm_delete: None,
            move_targets: None,
            move_idx: 0,
            search: None,
            trash_view: None,
            import: None,
            hooks_editor: None,
            focus: Focus::Projects,
            project_idx: 0,
            session_idx: 0,
            transcript: Vec::new(),
            transcript_scroll: 0,
            transcript_viewport: 1,
            memory_lines,
            memory_scroll: 0,
            memory_viewport: 1,
            config_lines,
            config_scroll: 0,
            config_viewport: 1,
            extensions,
            ext_scroll: 0,
            ext_viewport: 1,
            ext_total: 0,
            settings,
        }
    }

    // --- Accès ---

    /// Home actuellement active.
    pub fn home(&self) -> &ClaudeHome {
        &self.homes[self.active]
    }

    /// Libellé du contexte courant (home actif, ou « tous les homes » en agrégé).
    pub fn active_label(&self) -> String {
        if self.aggregate {
            format!("tous les homes ({})", self.homes.len())
        } else {
            self.homes[self.active].label.clone()
        }
    }

    pub fn selected_project(&self) -> Option<&Project> {
        self.projects.get(self.project_idx)
    }

    pub fn selected_session(&self) -> Option<&SessionMeta> {
        self.selected_project()
            .and_then(|p| p.sessions.get(self.session_idx))
    }

    pub fn is_empty(&self) -> bool {
        self.projects.is_empty()
    }

    // --- Navigation globale ---

    pub fn quit(&mut self) {
        self.should_quit = true;
    }

    pub fn toggle_help(&mut self) {
        self.show_help = !self.show_help;
    }

    pub fn set_section(&mut self, section: Section) {
        self.section = section;
    }

    pub fn next_section(&mut self) {
        self.section = self.section.next();
    }

    // --- Browse : navigation listes ---

    // --- Repliage des groupes (homes) en vue agrégée ---

    fn base_key(&self, i: usize) -> Option<String> {
        self.project_home_bases
            .get(i)
            .map(|b| b.to_string_lossy().into_owned())
    }

    /// Le home du projet `i` est-il replié ?
    pub fn is_home_collapsed(&self, i: usize) -> bool {
        self.base_key(i)
            .map(|k| self.collapsed.contains(&k))
            .unwrap_or(false)
    }

    /// `i` est-il le premier projet de son groupe (home) ? Les groupes sont
    /// contigus, donc c'est le cas si le projet précédent a un autre home.
    fn is_group_anchor(&self, i: usize) -> bool {
        match (
            self.project_home_bases.get(i),
            i.checked_sub(1).and_then(|p| self.project_home_bases.get(p)),
        ) {
            (Some(b), Some(prev)) => b != prev,
            (Some(_), None) => true,
            _ => false,
        }
    }

    /// Index du premier projet du groupe (home) auquel `i` appartient.
    fn group_anchor(&self, i: usize) -> usize {
        let Some(base) = self.project_home_bases.get(i).cloned() else {
            return i;
        };
        let mut j = i;
        while j > 0 && self.project_home_bases.get(j - 1) == Some(&base) {
            j -= 1;
        }
        j
    }

    /// Un projet est navigable s'il est visible (home déplié), ou s'il est
    /// l'ancre d'un groupe replié (on reste posé sur l'en-tête, repliable).
    fn proj_navigable(&self, i: usize) -> bool {
        if !self.aggregate {
            return i < self.projects.len();
        }
        if !self.is_home_collapsed(i) {
            true
        } else {
            self.is_group_anchor(i)
        }
    }

    /// Projet navigable suivant dans la direction `delta`, en bornant aux
    /// extrémités (pas de bouclage, cohérent avec le reste de la navigation).
    fn next_navigable_project(&self, from: usize, delta: i32) -> usize {
        let n = self.projects.len();
        if n == 0 {
            return from;
        }
        if delta >= 0 {
            ((from + 1)..n).find(|&j| self.proj_navigable(j)).unwrap_or(from)
        } else {
            (0..from).rev().find(|&j| self.proj_navigable(j)).unwrap_or(from)
        }
    }

    fn edge_navigable_project(&self, last: bool) -> usize {
        let n = self.projects.len();
        let range: Vec<usize> = if last {
            (0..n).rev().collect()
        } else {
            (0..n).collect()
        };
        range
            .into_iter()
            .find(|&i| self.proj_navigable(i))
            .unwrap_or(0)
    }

    /// Replie / déplie le home du projet courant (vue agrégée, focus Projets).
    pub fn toggle_collapse_current(&mut self) {
        if !self.aggregate
            || self.section != Section::Browse
            || self.browse_view != BrowseView::List
            || self.focus != Focus::Projects
        {
            return;
        }
        let Some(key) = self.base_key(self.project_idx) else {
            return;
        };
        if self.collapsed.remove(&key) {
            self.status = Some("Groupe déplié".to_string());
        } else {
            self.collapsed.insert(key);
            // On se pose sur l'ancre (en-tête) qui reste navigable.
            self.project_idx = self.group_anchor(self.project_idx);
            self.session_idx = 0;
            self.status = Some("Groupe replié (Espace pour déplier)".to_string());
        }
    }

    /// Replie tous les homes sauf celui du projet courant ; ou déplie tout si
    /// tout était déjà replié.
    pub fn toggle_collapse_all(&mut self) {
        if !self.aggregate
            || self.section != Section::Browse
            || self.browse_view != BrowseView::List
        {
            return;
        }
        let all_bases: HashSet<String> = self
            .project_home_bases
            .iter()
            .map(|b| b.to_string_lossy().into_owned())
            .collect();
        if all_bases.iter().all(|k| self.collapsed.contains(k)) {
            self.collapsed.clear();
            self.status = Some("Tous les groupes dépliés".to_string());
        } else {
            self.collapsed = all_bases;
            self.project_idx = self.group_anchor(self.project_idx);
            self.session_idx = 0;
            self.status = Some("Tous les groupes repliés".to_string());
        }
    }

    fn session_count(&self) -> usize {
        self.selected_project().map(|p| p.sessions.len()).unwrap_or(0)
    }

    pub fn move_down(&mut self) {
        match self.section {
            Section::Browse => match self.browse_view {
                BrowseView::List => self.browse_move(1),
                BrowseView::Transcript => self.scroll_transcript(1),
            },
            Section::Memory => self.memory_scroll = scroll_add(self.memory_scroll, 1),
            Section::Config => {
                if self.settings.raw() {
                    self.config_scroll = scroll_add(self.config_scroll, 1);
                } else {
                    self.settings.move_field(1);
                }
            }
            Section::Extensions => self.ext_scroll = scroll_add(self.ext_scroll, 1),
        }
    }

    pub fn move_up(&mut self) {
        match self.section {
            Section::Browse => match self.browse_view {
                BrowseView::List => self.browse_move(-1),
                BrowseView::Transcript => self.scroll_transcript(-1),
            },
            Section::Memory => self.memory_scroll = self.memory_scroll.saturating_sub(1),
            Section::Config => {
                if self.settings.raw() {
                    self.config_scroll = self.config_scroll.saturating_sub(1);
                } else {
                    self.settings.move_field(-1);
                }
            }
            Section::Extensions => self.ext_scroll = self.ext_scroll.saturating_sub(1),
        }
    }

    fn browse_move(&mut self, delta: i32) {
        match self.focus {
            Focus::Projects => {
                // Saute les projets masqués par un groupe replié.
                self.project_idx = self.next_navigable_project(self.project_idx, delta);
                // Le changement de projet réinitialise la sélection de session.
                self.session_idx = 0;
            }
            Focus::Sessions => {
                let n = self.session_count();
                self.session_idx = step(self.session_idx, delta, n);
            }
        }
    }

    /// Va au début (false) ou à la fin (true) de la liste focalisée (Browse).
    fn browse_to(&mut self, to_last: bool) {
        match self.focus {
            Focus::Projects => {
                self.project_idx = self.edge_navigable_project(to_last);
                self.session_idx = 0;
            }
            Focus::Sessions => {
                let n = self.session_count();
                self.session_idx = if to_last { n.saturating_sub(1) } else { 0 };
            }
        }
    }

    pub fn focus_left(&mut self) {
        if self.section == Section::Browse && self.browse_view == BrowseView::List {
            self.focus = Focus::Projects;
        }
    }

    pub fn focus_right(&mut self) {
        if self.section == Section::Browse
            && self.browse_view == BrowseView::List
            && self.session_count() > 0
        {
            self.focus = Focus::Sessions;
        }
    }

    pub fn toggle_focus(&mut self) {
        if self.section == Section::Browse && self.browse_view == BrowseView::List {
            self.focus = match self.focus {
                Focus::Projects if self.session_count() > 0 => Focus::Sessions,
                _ => Focus::Projects,
            };
        }
    }

    // --- Transcript ---

    /// Ouvre le transcript de la session sélectionnée (Enter dans la liste).
    pub fn open_transcript(&mut self) {
        if self.section != Section::Browse || self.browse_view != BrowseView::List {
            return;
        }
        // En focus projet, Enter bascule d'abord vers la liste des sessions.
        if self.focus == Focus::Projects {
            if self.session_count() > 0 {
                self.focus = Focus::Sessions;
            }
            return;
        }
        let path = match self.selected_session() {
            Some(s) => s.path.clone(),
            None => return,
        };
        self.transcript = parse_transcript(&path);
        self.transcript_scroll = 0;
        self.browse_view = BrowseView::Transcript;
    }

    /// Esc dans Browse : remonte du transcript vers la liste. Retourne `true`
    /// si un retour a eu lieu (donc `Esc` est consommé).
    pub fn back(&mut self) -> bool {
        if self.section == Section::Browse && self.browse_view == BrowseView::Transcript {
            self.browse_view = BrowseView::List;
            true
        } else {
            false
        }
    }

    fn scroll_transcript(&mut self, delta: i32) {
        let max = self.transcript_max_scroll();
        if delta < 0 {
            self.transcript_scroll = self.transcript_scroll.saturating_sub((-delta) as usize);
        } else {
            self.transcript_scroll = (self.transcript_scroll + delta as usize).min(max);
        }
    }

    fn transcript_total_lines(&self) -> usize {
        // En-tête + corps (chaque corps peut être multi-lignes) + ligne vide de séparation.
        self.transcript
            .iter()
            .map(|e| 1 + e.body.lines().count().max(1) + 1)
            .sum()
    }

    fn transcript_max_scroll(&self) -> usize {
        self.transcript_total_lines()
            .saturating_sub(self.transcript_viewport)
    }

    // --- Pagination générique (PageUp/Down, Home/End) ---

    pub fn page_down(&mut self) {
        match self.section {
            Section::Browse if self.browse_view == BrowseView::Transcript => {
                let step = self.transcript_viewport.max(1);
                let max = self.transcript_max_scroll();
                self.transcript_scroll = (self.transcript_scroll + step).min(max);
            }
            Section::Browse if self.browse_view == BrowseView::List => self.browse_move(10),
            Section::Memory => {
                self.memory_scroll = page(self.memory_scroll, self.memory_viewport, true);
            }
            Section::Config => {
                if self.settings.raw() {
                    self.config_scroll = page(self.config_scroll, self.config_viewport, true);
                } else {
                    self.settings.move_field(8);
                }
            }
            Section::Extensions => {
                self.ext_scroll = page(self.ext_scroll, self.ext_viewport, true);
            }
            _ => {}
        }
    }

    pub fn page_up(&mut self) {
        match self.section {
            Section::Browse if self.browse_view == BrowseView::Transcript => {
                let step = self.transcript_viewport.max(1);
                self.transcript_scroll = self.transcript_scroll.saturating_sub(step);
            }
            Section::Browse if self.browse_view == BrowseView::List => self.browse_move(-10),
            Section::Memory => {
                self.memory_scroll = page(self.memory_scroll, self.memory_viewport, false);
            }
            Section::Config => {
                if self.settings.raw() {
                    self.config_scroll = page(self.config_scroll, self.config_viewport, false);
                } else {
                    self.settings.move_field(-8);
                }
            }
            Section::Extensions => {
                self.ext_scroll = page(self.ext_scroll, self.ext_viewport, false);
            }
            _ => {}
        }
    }

    pub fn go_home(&mut self) {
        match self.section {
            Section::Browse if self.browse_view == BrowseView::Transcript => {
                self.transcript_scroll = 0
            }
            Section::Browse if self.browse_view == BrowseView::List => self.browse_to(false),
            Section::Memory => self.memory_scroll = 0,
            Section::Config => {
                if self.settings.raw() {
                    self.config_scroll = 0;
                } else {
                    self.settings.go_first();
                }
            }
            Section::Extensions => self.ext_scroll = 0,
            _ => {}
        }
    }

    pub fn go_end(&mut self) {
        match self.section {
            Section::Browse if self.browse_view == BrowseView::Transcript => {
                self.transcript_scroll = self.transcript_max_scroll()
            }
            Section::Browse if self.browse_view == BrowseView::List => self.browse_to(true),
            Section::Memory => {
                self.memory_scroll = self
                    .memory_lines
                    .len()
                    .saturating_sub(self.memory_viewport)
            }
            Section::Config => {
                if self.settings.raw() {
                    self.config_scroll = self
                        .config_lines
                        .len()
                        .saturating_sub(self.config_viewport);
                } else {
                    self.settings.go_last();
                }
            }
            Section::Extensions => {
                self.ext_scroll = self.ext_total.saturating_sub(self.ext_viewport);
            }
            _ => {}
        }
    }

    // --- Config : formulaire de réglages ---

    /// Enter dans la section courante : ouvre un transcript (Browse), active le
    /// champ surligné du formulaire (Config, hors JSON brut), ou ouvre l'éditeur
    /// de hooks (Extensions).
    pub fn on_enter(&mut self) {
        match self.section {
            Section::Browse => self.open_transcript(),
            Section::Config if !self.settings.raw() => self.settings.activate(),
            Section::Extensions => self.open_hooks_editor(),
            _ => {}
        }
    }

    /// Flèche gauche : focus panneau (Browse) ou cycle Enum en arrière (Config).
    pub fn nav_left(&mut self) {
        if self.section == Section::Config && !self.settings.raw() {
            self.settings.cycle(false);
        } else {
            self.focus_left();
        }
    }

    /// Flèche droite : focus panneau (Browse) ou cycle Enum en avant (Config).
    pub fn nav_right(&mut self) {
        if self.section == Section::Config && !self.settings.raw() {
            self.settings.cycle(true);
        } else {
            self.focus_right();
        }
    }

    /// Enregistre le formulaire (section Config uniquement).
    pub fn save_settings(&mut self) {
        if self.section == Section::Config {
            let msg = self.settings.save();
            self.status = Some(msg);
        }
    }

    /// Bascule formulaire ↔ JSON brut (section Config uniquement).
    pub fn toggle_settings_raw(&mut self) {
        if self.section == Section::Config {
            self.settings.toggle_raw();
        }
    }

    /// En vue agrégée, change le home **cible** de Mémoire/Config (sans quitter
    /// l'agrégat ni toucher la liste fusionnée des projets).
    pub fn cycle_config_target(&mut self) {
        if !self.aggregate
            || self.homes.len() < 2
            || !matches!(
                self.section,
                Section::Memory | Section::Config | Section::Extensions
            )
        {
            return;
        }
        self.active = (self.active + 1) % self.homes.len();
        let home = self.homes[self.active].clone();
        self.memory_lines = read_file_lines(home.memory_file(), "(aucune mémoire utilisateur)");
        self.config_lines = build_config_lines(&home);
        self.extensions = read_extensions(&home);
        self.settings = SettingsForm::load(&home);
        self.memory_scroll = 0;
        self.config_scroll = 0;
        self.ext_scroll = 0;
        self.status = Some(format!("Cible Mémoire/Config/Extensions : {}", home.label));
    }

    // --- Édition externe ($EDITOR) ---

    /// Demande l'ouverture du fichier de la section courante dans `$EDITOR` :
    /// `CLAUDE.md` (Mémoire) ou `settings.json` (Config). Traité par la boucle.
    pub fn request_edit(&mut self) {
        let home = &self.homes[self.active];
        self.pending_edit = match self.section {
            Section::Memory => Some(home.memory_file()),
            // Hooks et plugins activés vivent dans settings.json.
            Section::Config | Section::Extensions => Some(home.settings_file()),
            Section::Browse => None,
        };
    }

    /// Recharge mémoire / config / formulaire du home actif (après édition externe),
    /// sans réinitialiser les sélections.
    pub fn reload_files(&mut self) {
        let home = self.homes[self.active].clone();
        self.memory_lines = read_file_lines(home.memory_file(), "(aucune mémoire utilisateur)");
        self.config_lines = build_config_lines(&home);
        self.extensions = read_extensions(&home);
        self.settings = SettingsForm::load(&home);
    }

    /// Appelé après le retour de l'éditeur externe.
    pub fn after_external_edit(&mut self, path: &Path) {
        self.reload_files();
        self.status = Some(format!("Édité : {}", path.display()));
    }

    // --- Export ---

    /// Exporte ~/.claude vers `<HOME>/claudine-export-<unix>.tar.gz`.
    /// Le résultat (succès/erreur) est placé dans la ligne de statut.
    pub fn do_export(&mut self) {
        let secs = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let home_dir = std::env::var("HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|_| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
        let out = home_dir.join(format!("claudine-export-{secs}.tar.gz"));

        match export(self.home(), &out, &ExportOptions::default()) {
            Ok(report) => {
                let projets = report.count("projects").max(self.projects.len());
                let sessions = report.count("sessions");
                self.status = Some(format!(
                    "Export OK → {} ({projets} projets, {sessions} sessions)",
                    out.display()
                ));
            }
            Err(e) => {
                self.status = Some(format!("Échec export : {e}"));
            }
        }
    }

    // --- Import (assistant : saisie du chemin → aperçu → application) ---

    /// Ouvre l'assistant d'import, préremplit le chemin avec le dernier export
    /// trouvé dans `$HOME` (sinon le dossier `$HOME/`).
    pub fn open_import(&mut self) {
        let home_dir = std::env::var("HOME").map(PathBuf::from).ok();
        let prefill = home_dir
            .as_deref()
            .and_then(latest_export_bundle)
            .map(|p| p.to_string_lossy().into_owned())
            .or_else(|| home_dir.map(|h| format!("{}/", h.display())))
            .unwrap_or_default();
        self.import = Some(ImportState {
            input: prefill,
            preview: None,
            overwrite: false,
        });
    }

    pub fn import_cancel(&mut self) {
        self.import = None;
    }

    pub fn import_input_char(&mut self, c: char) {
        if let Some(im) = &mut self.import {
            if im.preview.is_none() {
                im.input.push(c);
            }
        }
    }

    pub fn import_input_backspace(&mut self) {
        if let Some(im) = &mut self.import {
            if im.preview.is_none() {
                im.input.pop();
            }
        }
    }

    pub fn import_toggle_overwrite(&mut self) {
        if let Some(im) = &mut self.import {
            im.overwrite = !im.overwrite;
        }
    }

    /// Valide le chemin saisi et calcule l'aperçu (`dry_run`) sur le home actif.
    pub fn import_preview(&mut self) {
        let raw = match &self.import {
            Some(im) if im.preview.is_none() => im.input.trim().to_string(),
            _ => return,
        };
        if raw.is_empty() {
            self.status = Some("Indiquez le chemin d'un bundle .tar.gz".to_string());
            return;
        }
        let path = expand_tilde(&raw);
        if !path.is_file() {
            self.status = Some(format!("Fichier introuvable : {}", path.display()));
            return;
        }
        let table = RemapTable::new(Vec::new());
        match import_dry_run(&path, self.home(), &table, &ImportOptions::default()) {
            Ok(report) => {
                let preview = ImportPreview {
                    projects: report.count("projects"),
                    sessions_new: report.count("sessions_new"),
                    sessions_conflict: report.count("sessions_conflict"),
                };
                if let Some(im) = &mut self.import {
                    im.preview = Some((path, preview));
                }
            }
            Err(e) => self.status = Some(format!("Bundle invalide : {e}")),
        }
    }

    /// Applique l'import prévisualisé puis recharge la liste des projets.
    pub fn import_apply(&mut self) {
        let (path, overwrite) = match &self.import {
            Some(im) => match &im.preview {
                Some((p, _)) => (p.clone(), im.overwrite),
                None => return,
            },
            None => return,
        };
        let table = RemapTable::new(Vec::new());
        let opts = ImportOptions { overwrite };
        let result = import_apply(&path, self.home(), &table, &opts);
        self.import = None;
        match result {
            Ok(report) => {
                let imported = report.count("sessions_imported");
                let skipped = report.count("sessions_skipped");
                self.reload_projects();
                self.clamp_browse_indices();
                self.status = Some(format!(
                    "Import OK ({imported} session(s) importée(s), {skipped} ignorée(s))"
                ));
            }
            Err(e) => self.status = Some(format!("Échec import : {e}")),
        }
    }

    // --- Éditeur de hooks ---

    /// Ouvre l'éditeur de hooks pour le home actif (depuis la section Extensions).
    pub fn open_hooks_editor(&mut self) {
        if self.section != Section::Extensions {
            return;
        }
        let groups = read_hook_groups(self.home());
        self.hooks_editor = Some(HooksEditor::new(groups));
    }

    pub fn hooks_cancel(&mut self) {
        self.hooks_editor = None;
    }

    /// Enregistre les hooks édités dans settings.json du home actif.
    pub fn hooks_save(&mut self) {
        let Some(editor) = self.hooks_editor.take() else {
            return;
        };
        let groups = editor.into_groups();
        match write_hooks(self.home(), &groups) {
            Ok(()) => {
                self.reload_files();
                self.status = Some("Hooks enregistrés".to_string());
            }
            Err(e) => {
                self.status = Some(format!("Échec enregistrement hooks : {e}"));
            }
        }
    }

    // --- Sélecteur de home ---

    /// Ouvre le sélecteur de home, positionné sur la home active.
    pub fn open_picker(&mut self) {
        self.show_picker = true;
        self.picker_mode = PickerMode::List;
        // Index 0 = « Tous les homes » ; les homes suivent à partir de 1.
        self.picker_idx = if self.aggregate { 0 } else { self.active + 1 };
    }

    /// Ferme le sélecteur (et annule une éventuelle saisie en cours).
    pub fn close_picker(&mut self) {
        self.show_picker = false;
        self.picker_mode = PickerMode::List;
    }

    pub fn picker_move(&mut self, delta: i32) {
        if self.picker_mode != PickerMode::List {
            return;
        }
        // +1 pour l'entrée « Tous les homes ».
        self.picker_idx = step(self.picker_idx, delta, self.homes.len() + 1);
    }

    /// (Re)construit la liste des projets : home actif seul, ou tous les homes
    /// concaténés (mode agrégé), chaque projet étiqueté par son home.
    fn reload_projects(&mut self) {
        let mut projects = Vec::new();
        let mut project_homes = Vec::new();
        let mut project_home_bases = Vec::new();
        let homes: Vec<&ClaudeHome> = if self.aggregate {
            self.homes.iter().collect()
        } else {
            vec![&self.homes[self.active]]
        };
        for h in homes {
            if let Ok(ps) = scan_projects(h) {
                for p in ps {
                    project_homes.push(h.label.clone());
                    project_home_bases.push(h.base.clone());
                    projects.push(p);
                }
            }
        }
        self.projects = projects;
        self.project_homes = project_homes;
        self.project_home_bases = project_home_bases;
    }

    /// Borne les index de Browse après un rechargement (ex. suppression).
    fn clamp_browse_indices(&mut self) {
        if self.project_idx >= self.projects.len() {
            self.project_idx = self.projects.len().saturating_sub(1);
        }
        // Si la sélection est tombée dans un groupe replié (hors ancre), la
        // ramener sur l'ancre du groupe pour rester navigable.
        if !self.projects.is_empty() && !self.proj_navigable(self.project_idx) {
            self.project_idx = self.group_anchor(self.project_idx);
        }
        let sc = self.session_count();
        if self.session_idx >= sc {
            self.session_idx = sc.saturating_sub(1);
        }
        if sc == 0 {
            self.focus = Focus::Projects;
        }
    }

    // --- Ménage : suppression (corbeille) et déplacement de sessions ---

    fn on_sessions(&self) -> bool {
        self.section == Section::Browse
            && self.browse_view == BrowseView::List
            && self.focus == Focus::Sessions
    }

    /// `d` en vue liste : supprime la session (focus Sessions) ou le projet
    /// entier (focus Projets) — toujours vers la corbeille, après confirmation.
    pub fn request_delete(&mut self) {
        if self.section != Section::Browse || self.browse_view != BrowseView::List {
            return;
        }
        match self.focus {
            Focus::Sessions => self.request_delete_session(),
            Focus::Projects => self.request_delete_project(),
        }
    }

    /// Demande la confirmation de suppression de la session sélectionnée.
    pub fn request_delete_session(&mut self) {
        if self.on_sessions() && self.selected_session().is_some() {
            self.confirm_delete = Some(DeleteKind::Session);
        }
    }

    /// Demande la confirmation de suppression du projet entier sélectionné
    /// (sessions incluses, restaurables). Couvre les projets vides « 0 sess. ».
    pub fn request_delete_project(&mut self) {
        if self.focus == Focus::Projects && self.selected_project().is_some() {
            self.confirm_delete = Some(DeleteKind::Project);
        }
    }

    pub fn confirm_delete_cancel(&mut self) {
        self.confirm_delete = None;
    }

    /// Confirme : déplace la session ou le projet vers la corbeille, recharge.
    pub fn confirm_delete_apply(&mut self) {
        let kind = match self.confirm_delete.take() {
            Some(k) => k,
            None => return,
        };
        let base = self
            .project_home_bases
            .get(self.project_idx)
            .cloned()
            .unwrap_or_else(|| self.home().base.clone());
        let result = match kind {
            DeleteKind::Session => {
                let (encoded, path) = match (self.selected_project(), self.selected_session()) {
                    (Some(p), Some(s)) => (p.encoded_name.clone(), s.path.clone()),
                    _ => return,
                };
                trash_session(&base, &encoded, &path).map(|d| format!("Session → corbeille : {}", d.display()))
            }
            DeleteKind::Project => {
                let encoded = match self.selected_project() {
                    Some(p) => p.encoded_name.clone(),
                    None => return,
                };
                trash_project(&base, &encoded).map(|_| "Projet → corbeille".to_string())
            }
        };
        match result {
            Ok(msg) => {
                self.reload_projects();
                self.clamp_browse_indices();
                self.status = Some(msg);
            }
            Err(e) => self.status = Some(format!("Échec suppression : {e}")),
        }
    }

    /// Ouvre le sélecteur de cible de déplacement (tous les projets sauf l'actuel).
    pub fn request_move_session(&mut self) {
        if !self.on_sessions() || self.selected_session().is_none() {
            return;
        }
        let mut targets = Vec::new();
        for (i, p) in self.projects.iter().enumerate() {
            if i == self.project_idx {
                continue;
            }
            if let Some(cwd) = &p.cwd {
                let home = self.project_homes.get(i).cloned().unwrap_or_default();
                let base = self.project_home_bases.get(i).cloned().unwrap_or_default();
                targets.push(MoveTarget {
                    label: format!("{}  ⟨{home}⟩", humanize_path(cwd)),
                    cwd: cwd.clone(),
                    home_base: base,
                });
            }
        }
        if targets.is_empty() {
            self.status = Some("Aucune cible de déplacement disponible".to_string());
            return;
        }
        self.move_targets = Some(targets);
        self.move_idx = 0;
    }

    pub fn move_picker_cancel(&mut self) {
        self.move_targets = None;
    }

    pub fn move_picker_move(&mut self, delta: i32) {
        if let Some(targets) = &self.move_targets {
            self.move_idx = step(self.move_idx, delta, targets.len());
        }
    }

    /// Déplace la session sélectionnée vers la cible surlignée (remap du cwd).
    pub fn move_picker_select(&mut self) {
        let target = match &self.move_targets {
            Some(t) => match t.get(self.move_idx) {
                Some(t) => t.clone(),
                None => {
                    self.move_targets = None;
                    return;
                }
            },
            None => return,
        };
        let path = match self.selected_session() {
            Some(s) => s.path.clone(),
            None => {
                self.move_targets = None;
                return;
            }
        };
        let old_cwd = self
            .selected_session()
            .and_then(|s| s.cwd.clone())
            .or_else(|| self.selected_project().and_then(|p| p.cwd.clone()));
        self.move_targets = None;
        match move_session(&path, old_cwd.as_deref(), &target.home_base, &target.cwd) {
            Ok(dest) => {
                self.reload_projects();
                self.clamp_browse_indices();
                self.status = Some(format!("Session déplacée → {}", dest.display()));
            }
            Err(e) => self.status = Some(format!("Échec déplacement : {e}")),
        }
    }

    // --- Recherche de session ---

    pub fn open_search(&mut self) {
        self.search = Some(SearchState {
            query: String::new(),
            results: Vec::new(),
            idx: 0,
            deep: false,
        });
    }

    pub fn search_input_char(&mut self, c: char) {
        if let Some(s) = &mut self.search {
            s.query.push(c);
        }
        self.search_recompute();
    }

    pub fn search_input_backspace(&mut self) {
        if let Some(s) = &mut self.search {
            s.query.pop();
        }
        self.search_recompute();
    }

    pub fn search_cancel(&mut self) {
        self.search = None;
    }

    pub fn search_move(&mut self, delta: i32) {
        if let Some(s) = &mut self.search {
            if !s.results.is_empty() {
                s.idx = step(s.idx, delta, s.results.len());
            }
        }
    }

    /// Collecte les sessions correspondant à `query` (déjà en minuscules).
    /// `deep` active la lecture du contenu ; sinon on filtre chemin + id.
    fn collect_hits(&self, query: &str, deep: bool) -> Vec<SearchHit> {
        let mut results = Vec::new();
        for (pi, p) in self.projects.iter().enumerate() {
            let proj_label = humanize_path(p.cwd.as_deref().unwrap_or(&p.encoded_name));
            let proj_lc = proj_label.to_lowercase();
            for (si, sess) in p.sessions.iter().enumerate() {
                let meta_hit = proj_lc.contains(query) || sess.id.to_lowercase().contains(query);
                let snippet = if deep {
                    match find_in_session(&sess.path, query) {
                        Some(snip) => snip,
                        None if meta_hit => "(chemin / id)".to_string(),
                        None => continue,
                    }
                } else if meta_hit {
                    "(chemin / id)".to_string()
                } else {
                    continue;
                };
                let id8: String = sess.id.chars().take(8).collect();
                results.push(SearchHit {
                    project_idx: pi,
                    session_idx: si,
                    label: format!("{id8}  {proj_label}"),
                    snippet,
                });
            }
        }
        results
    }

    /// Filtre en direct (chemin + id) à chaque frappe ; instantané même avec
    /// des centaines de sessions car aucun fichier n'est lu.
    pub fn search_recompute(&mut self) {
        let query = match &self.search {
            Some(s) => s.query.trim().to_lowercase(),
            None => return,
        };
        let results = if query.is_empty() {
            Vec::new()
        } else {
            self.collect_hits(&query, false)
        };
        if let Some(s) = &mut self.search {
            s.results = results;
            s.idx = 0;
            s.deep = false;
        }
    }

    /// Recherche **dans le contenu** des sessions (lecture des fichiers) pour la
    /// requête courante ; déclenchée par `Tab` car plus coûteuse.
    pub fn search_deep(&mut self) {
        let query = match &self.search {
            Some(s) => s.query.trim().to_lowercase(),
            None => return,
        };
        if query.is_empty() {
            return;
        }
        let results = self.collect_hits(&query, true);
        let n = results.len();
        if let Some(s) = &mut self.search {
            s.results = results;
            s.idx = 0;
            s.deep = true;
        }
        self.status = Some(format!("{n} session(s) (contenu) pour « {query} »"));
    }

    /// Ouvre le transcript de la session du résultat sélectionné.
    pub fn search_open_selected(&mut self) {
        let (pi, si) = match &self.search {
            Some(s) => match s.results.get(s.idx) {
                Some(h) => (h.project_idx, h.session_idx),
                None => {
                    self.search = None;
                    return;
                }
            },
            None => return,
        };
        self.search = None;
        if pi >= self.projects.len() {
            return;
        }
        self.project_idx = pi;
        self.session_idx = si;
        self.section = Section::Browse;
        self.focus = Focus::Sessions;
        if let Some(sess) = self.selected_session() {
            let path = sess.path.clone();
            self.transcript = parse_transcript(&path);
            self.transcript_scroll = 0;
            self.browse_view = BrowseView::Transcript;
        }
    }

    // --- Corbeille (restauration) ---

    /// Ouvre le viewer de corbeille (sessions supprimées de tous les homes).
    pub fn open_trash(&mut self) {
        let mut items = Vec::new();
        for h in &self.homes {
            for it in list_trash(&h.base) {
                let proj = decode_encoded_to_path(&it.encoded)
                    .map(|p| humanize_path(&p))
                    .unwrap_or_else(|| it.encoded.clone());
                // Détail : id de session unique, ou nombre de sessions, ou « vide ».
                let detail = match (it.sessions, it.sample.as_deref()) {
                    (1, Some(id)) => {
                        let id8: String = id.chars().take(8).collect();
                        format!("session {id8}")
                    }
                    (0, _) => "projet vide".to_string(),
                    (n, _) => format!("{n} sessions"),
                };
                items.push(TrashEntry {
                    dir: it.dir,
                    home_base: h.base.clone(),
                    label: format!("{proj}  ⟨{}⟩  · {detail}", h.label),
                });
            }
        }
        if items.is_empty() {
            self.status = Some("Corbeille vide".to_string());
            return;
        }
        self.trash_view = Some(TrashState {
            items,
            idx: 0,
            confirm: None,
        });
    }

    pub fn trash_cancel(&mut self) {
        self.trash_view = None;
    }

    pub fn trash_move(&mut self, delta: i32) {
        if let Some(t) = &mut self.trash_view {
            t.idx = step(t.idx, delta, t.items.len());
        }
    }

    /// Restaure la session surlignée vers son projet d'origine.
    pub fn trash_restore_selected(&mut self) {
        let entry = match &self.trash_view {
            Some(t) => match t.items.get(t.idx) {
                Some(e) => e.clone(),
                None => {
                    self.trash_view = None;
                    return;
                }
            },
            None => return,
        };
        match restore_trash_entry(&entry.dir, &entry.home_base) {
            Ok(dest) => {
                if let Some(t) = &mut self.trash_view {
                    t.items.retain(|e| e.dir != entry.dir);
                    if t.items.is_empty() {
                        self.trash_view = None;
                    } else {
                        t.idx = t.idx.min(t.items.len() - 1);
                    }
                }
                self.reload_projects();
                self.clamp_browse_indices();
                self.status = Some(format!("Restauré → {}", dest.display()));
            }
            Err(e) => self.status = Some(format!("Échec restauration : {e}")),
        }
    }

    /// Demande confirmation pour supprimer **définitivement** la session surlignée.
    pub fn trash_request_purge(&mut self) {
        if let Some(t) = &mut self.trash_view {
            if !t.items.is_empty() {
                t.confirm = Some(PurgeScope::One);
            }
        }
    }

    /// Demande confirmation pour **vider toute** la corbeille (tous les homes).
    pub fn trash_request_empty(&mut self) {
        if let Some(t) = &mut self.trash_view {
            if !t.items.is_empty() {
                t.confirm = Some(PurgeScope::All);
            }
        }
    }

    /// Annule une confirmation de purge en attente.
    pub fn trash_confirm_cancel(&mut self) {
        if let Some(t) = &mut self.trash_view {
            t.confirm = None;
        }
    }

    /// Applique la purge confirmée (suppression définitive, non récupérable).
    pub fn trash_confirm_apply(&mut self) {
        let scope = match self.trash_view.as_ref().and_then(|t| t.confirm) {
            Some(s) => s,
            None => return,
        };
        match scope {
            PurgeScope::One => {
                let entry = match self
                    .trash_view
                    .as_ref()
                    .and_then(|t| t.items.get(t.idx).cloned())
                {
                    Some(e) => e,
                    None => {
                        self.trash_confirm_cancel();
                        return;
                    }
                };
                match purge_trash_item(&entry.dir) {
                    Ok(()) => {
                        if let Some(t) = &mut self.trash_view {
                            t.items.retain(|e| e.dir != entry.dir);
                            t.confirm = None;
                            if t.items.is_empty() {
                                self.trash_view = None;
                            } else {
                                t.idx = t.idx.min(t.items.len() - 1);
                            }
                        }
                        self.status = Some("Entrée supprimée définitivement".to_string());
                    }
                    Err(e) => {
                        self.trash_confirm_cancel();
                        self.status = Some(format!("Échec suppression : {e}"));
                    }
                }
            }
            PurgeScope::All => {
                let mut total = 0usize;
                for h in &self.homes {
                    total += empty_trash(&h.base).unwrap_or(0);
                }
                self.trash_view = None;
                self.status = Some(format!("Corbeille vidée ({total} entrée(s))"));
            }
        }
    }

    /// Recharge projets / mémoire / config pour la home active et réinitialise
    /// les sélections et défilements.
    fn reload_active(&mut self) {
        self.reload_projects();
        let home = self.homes[self.active].clone();
        self.memory_lines = read_file_lines(home.memory_file(), "(aucune mémoire utilisateur)");
        self.config_lines = build_config_lines(&home);
        self.settings = SettingsForm::load(&home);
        self.browse_view = BrowseView::List;
        self.focus = Focus::Projects;
        self.project_idx = 0;
        self.session_idx = 0;
        self.transcript.clear();
        self.transcript_scroll = 0;
        self.memory_scroll = 0;
        self.config_scroll = 0;
    }

    /// Valide la sélection du sélecteur : active la home surlignée, recharge et
    /// ferme le popup.
    pub fn picker_select(&mut self) {
        if self.picker_mode != PickerMode::List {
            return;
        }
        if self.picker_idx == 0 {
            // « Tous les homes » : vue agrégée.
            self.aggregate = true;
            self.reload_active();
            self.status = Some(format!("Tous les homes ({})", self.homes.len()));
        } else if self.picker_idx - 1 < self.homes.len() {
            let i = self.picker_idx - 1;
            let label = self.homes[i].label.clone();
            self.aggregate = false;
            self.active = i;
            self.reload_active();
            self.status = Some(format!("Home active : {label}"));
        }
        self.close_picker();
    }

    /// Indique si la home surlignée est enregistrée dans la config (donc
    /// retirable), par comparaison de chemin canonique.
    pub fn picker_highlight_is_registered(&self) -> bool {
        if self.picker_idx == 0 {
            return false; // « Tous les homes »
        }
        let Some(home) = self.homes.get(self.picker_idx - 1) else {
            return false;
        };
        let config = ClaudineConfig::load();
        let key = canonical(&home.base);
        config.homes.iter().any(|h| canonical(&h.path) == key)
    }

    // --- Saisie d'ajout de home ---

    /// Passe en mode saisie de chemin pour ajouter une home.
    pub fn picker_start_add(&mut self) {
        if self.picker_mode == PickerMode::List {
            self.picker_mode = PickerMode::AddInput(String::new());
        }
    }

    pub fn picker_input_char(&mut self, c: char) {
        if let PickerMode::AddInput(buf) = &mut self.picker_mode {
            buf.push(c);
        }
    }

    pub fn picker_input_backspace(&mut self) {
        if let PickerMode::AddInput(buf) = &mut self.picker_mode {
            buf.pop();
        }
    }

    /// Annule la saisie et revient à la navigation dans la liste.
    pub fn picker_cancel_input(&mut self) {
        self.picker_mode = PickerMode::List;
    }

    /// Valide la saisie : si le chemin est un répertoire existant, enregistre la
    /// home (config), recharge les homes, sélectionne la nouvelle, et confirme ;
    /// sinon affiche une erreur et reste en mode saisie.
    pub fn picker_confirm_add(&mut self) {
        let path = match &self.picker_mode {
            PickerMode::AddInput(buf) => PathBuf::from(buf.trim()),
            PickerMode::List => return,
        };

        if path.as_os_str().is_empty() || !path.is_dir() {
            self.status = Some(format!("Chemin invalide : {}", path.display()));
            return;
        }

        let label = path
            .file_name()
            .map(|s| s.to_string_lossy().into_owned())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "claude".to_string());

        let mut config = ClaudineConfig::load();
        config.add_home(label.clone(), path.clone());
        if let Err(e) = config.save() {
            self.status = Some(format!("Échec sauvegarde config : {e}"));
            return;
        }

        // Recharge la liste des homes et sélectionne la home ajoutée.
        let key = canonical(&path);
        self.homes = discover_homes();
        self.active = self
            .homes
            .iter()
            .position(|h| canonical(&h.base) == key)
            .unwrap_or(self.active.min(self.homes.len().saturating_sub(1)));
        self.reload_active();
        self.picker_idx = self.active + 1;
        self.picker_mode = PickerMode::List;
        self.status = Some(format!("Home ajoutée : {label}"));
    }

    /// Retire la home surlignée si elle est enregistrée (config). Sinon affiche
    /// un statut explicatif. Recharge et réajuste l'index actif.
    pub fn picker_remove_highlight(&mut self) {
        if self.picker_mode != PickerMode::List {
            return;
        }
        if self.picker_idx == 0 {
            self.status = Some("« Tous les homes » n'est pas supprimable".to_string());
            return;
        }
        let Some(home) = self.homes.get(self.picker_idx - 1) else {
            return;
        };
        if !self.picker_highlight_is_registered() {
            self.status = Some("home auto-découvert : non supprimable".to_string());
            return;
        }

        let label = home.label.clone();
        let removed_key = canonical(&home.base);

        let mut config = ClaudineConfig::load();
        config.remove_home(&label);
        if let Err(e) = config.save() {
            self.status = Some(format!("Échec sauvegarde config : {e}"));
            return;
        }

        // Mémorise la home active pour la retrouver après rechargement.
        let active_key = canonical(&self.homes[self.active].base);
        self.homes = discover_homes();
        if self.homes.is_empty() {
            // Garde-fou : ne jamais rester sans home.
            self.homes.push(ClaudeHome::from_base(
                std::env::var("HOME")
                    .map(|h| PathBuf::from(h).join(".claude"))
                    .unwrap_or_else(|_| PathBuf::from(".claude")),
            ));
        }

        // Si la home active a été retirée, retombe sur la première.
        self.active = if active_key == removed_key {
            0
        } else {
            self.homes
                .iter()
                .position(|h| canonical(&h.base) == active_key)
                .unwrap_or(0)
        };
        self.reload_active();
        self.picker_idx = self.picker_idx.min(self.homes.len());
        self.status = Some(format!("Home retirée : {label}"));
    }
}

// --- Helpers libres ---

/// Canonicalise un chemin si possible, sinon le renvoie tel quel.
fn canonical(path: &std::path::Path) -> PathBuf {
    std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
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

fn scroll_add(scroll: usize, n: usize) -> usize {
    scroll.saturating_add(n)
}

fn page(scroll: usize, viewport: usize, down: bool) -> usize {
    let step = viewport.max(1);
    if down {
        scroll.saturating_add(step)
    } else {
        scroll.saturating_sub(step)
    }
}

/// Lit un fichier en lignes, ou renvoie une ligne de repli si absent/illisible.
fn read_file_lines(path: PathBuf, fallback: &str) -> Vec<String> {
    match fs::read_to_string(&path) {
        Ok(content) if !content.trim().is_empty() => {
            content.lines().map(|l| l.to_string()).collect()
        }
        _ => vec![fallback.to_string()],
    }
}

/// Construit l'affichage de la section Config : settings.json puis
/// settings.local.json, chacun sous un en-tête, l'absence notée en ligne.
fn build_config_lines(home: &ClaudeHome) -> Vec<String> {
    let mut out = Vec::new();
    for (label, path) in [
        ("settings.json", home.settings_file()),
        ("settings.local.json", home.settings_local_file()),
    ] {
        out.push(format!("── {label} ──"));
        match fs::read_to_string(&path) {
            Ok(content) if !content.trim().is_empty() => {
                out.extend(content.lines().map(|l| l.to_string()));
            }
            Ok(_) => out.push("(fichier vide)".to_string()),
            Err(_) => out.push("(fichier absent)".to_string()),
        }
        out.push(String::new());
    }
    out
}

/// Parse un fichier `.jsonl` de session en entrées lisibles. Ne panique jamais :
/// une ligne illisible devient une entrée `unparsable`.
pub fn parse_transcript(path: &std::path::Path) -> Vec<TranscriptEntry> {
    let content = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(e) => {
            return vec![TranscriptEntry {
                header: "▌ erreur".to_string(),
                body: format!("Impossible de lire la session : {e}"),
                unparsable: true,
            }]
        }
    };

    let mut entries = Vec::new();
    for line in content.lines() {
        if line.trim().is_empty() {
            continue;
        }
        match serde_json::from_str::<Value>(line) {
            Ok(v) => entries.push(entry_from_value(&v)),
            Err(_) => entries.push(TranscriptEntry {
                header: "▌ ?".to_string(),
                body: "⚠ (ligne non parsable)".to_string(),
                unparsable: true,
            }),
        }
    }
    if entries.is_empty() {
        entries.push(TranscriptEntry {
            header: "▌ vide".to_string(),
            body: "(session sans message)".to_string(),
            unparsable: false,
        });
    }
    entries
}

fn entry_from_value(v: &Value) -> TranscriptEntry {
    // Rôle/type : `message.role` en priorité, sinon `type`.
    let role = v
        .get("message")
        .and_then(|m| m.get("role"))
        .and_then(|r| r.as_str())
        .or_else(|| v.get("type").and_then(|t| t.as_str()))
        .unwrap_or("?");
    let ts = v
        .get("timestamp")
        .and_then(|t| t.as_str())
        .unwrap_or("");

    let header = if ts.is_empty() {
        format!("▌ {role}")
    } else {
        format!("▌ {role} · {ts}")
    };

    // Le contenu peut être `message.content` ou `content`.
    let content = v
        .get("message")
        .and_then(|m| m.get("content"))
        .or_else(|| v.get("content"));
    let body = extract_text(content);

    TranscriptEntry {
        header,
        body,
        unparsable: false,
    }
}

/// Extrait le texte d'un champ `content` : chaîne brute, ou tableau de blocs.
fn extract_text(content: Option<&Value>) -> String {
    match content {
        Some(Value::String(s)) => s.clone(),
        Some(Value::Array(items)) => {
            let mut parts = Vec::new();
            for item in items {
                let kind = item.get("type").and_then(|t| t.as_str()).unwrap_or("");
                match kind {
                    "text" => {
                        if let Some(t) = item.get("text").and_then(|t| t.as_str()) {
                            parts.push(t.to_string());
                        }
                    }
                    "tool_use" => {
                        let name = item.get("name").and_then(|n| n.as_str()).unwrap_or("?");
                        parts.push(format!("⚙ tool_use: {name}"));
                    }
                    "tool_result" => parts.push("↳ tool_result".to_string()),
                    _ => {}
                }
            }
            if parts.is_empty() {
                "(contenu non textuel)".to_string()
            } else {
                parts.join("\n")
            }
        }
        _ => "(contenu absent)".to_string(),
    }
}

/// Raccourcit un chemin pour l'affichage : remplace le `$HOME` de tête par `~`.
pub fn humanize_path(p: &str) -> String {
    if let Ok(home) = std::env::var("HOME") {
        if !home.is_empty() {
            if p == home {
                return "~".to_string();
            }
            if let Some(rest) = p.strip_prefix(&format!("{home}/")) {
                return format!("~/{rest}");
            }
        }
    }
    p.to_string()
}

/// Étend un `~` ou `~/...` de tête en chemin absolu via `$HOME`.
pub fn expand_tilde(p: &str) -> PathBuf {
    if let Ok(home) = std::env::var("HOME") {
        if p == "~" {
            return PathBuf::from(home);
        }
        if let Some(rest) = p.strip_prefix("~/") {
            return PathBuf::from(home).join(rest);
        }
    }
    PathBuf::from(p)
}

/// Renvoie le bundle d'export le plus récent de `dir` (préfixe `claudine-export-`,
/// suffixe `.tar.gz`), choisi par ordre lexicographique du nom (l'horodatage y figure).
fn latest_export_bundle(dir: &Path) -> Option<PathBuf> {
    let mut best: Option<(String, PathBuf)> = None;
    for entry in fs::read_dir(dir).ok()?.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.starts_with("claudine-export-") && name.ends_with(".tar.gz") {
            let take = best.as_ref().map(|(b, _)| name > *b).unwrap_or(true);
            if take {
                best = Some((name, entry.path()));
            }
        }
    }
    best.map(|(_, p)| p)
}

/// Formate une taille en octets de façon lisible (Kio/Mio).
pub fn human_size(bytes: u64) -> String {
    const KIB: f64 = 1024.0;
    let b = bytes as f64;
    if b < KIB {
        format!("{bytes} o")
    } else if b < KIB * KIB {
        format!("{:.1} Kio", b / KIB)
    } else {
        format!("{:.1} Mio", b / (KIB * KIB))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn temp_home() -> (tempfile::TempDir, ClaudeHome) {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir_all(dir.path().join("projects")).unwrap();
        let home = ClaudeHome::from_base(dir.path());
        (dir, home)
    }

    #[test]
    fn app_from_empty_home_is_empty() {
        let (_d, home) = temp_home();
        let app = App::with_homes(vec![home]);
        assert!(app.is_empty());
        assert_eq!(app.section, Section::Browse);
        // mémoire absente → message de repli
        assert_eq!(app.memory_lines, vec!["(aucune mémoire utilisateur)".to_string()]);
    }

    #[test]
    fn sections_cycle_with_tab() {
        let (_d, home) = temp_home();
        let mut app = App::with_homes(vec![home]);
        app.next_section();
        assert_eq!(app.section, Section::Memory);
        app.next_section();
        assert_eq!(app.section, Section::Config);
        app.next_section();
        assert_eq!(app.section, Section::Extensions);
        app.next_section();
        assert_eq!(app.section, Section::Browse);
    }

    #[test]
    fn human_size_formats() {
        assert_eq!(human_size(512), "512 o");
        assert_eq!(human_size(2048), "2.0 Kio");
        assert!(human_size(5 * 1024 * 1024).ends_with("Mio"));
    }

    #[test]
    fn extract_text_handles_blocks() {
        let v: Value = serde_json::from_str(
            r#"{"message":{"role":"assistant","content":[
                {"type":"text","text":"bonjour"},
                {"type":"tool_use","name":"Read"},
                {"type":"tool_result"}
            ]}}"#,
        )
        .unwrap();
        let e = entry_from_value(&v);
        assert!(e.header.contains("assistant"));
        assert!(e.body.contains("bonjour"));
        assert!(e.body.contains("⚙ tool_use: Read"));
        assert!(e.body.contains("↳ tool_result"));
    }

    #[test]
    fn parse_transcript_tolerates_garbage() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("s.jsonl");
        fs::write(
            &p,
            "pas du json\n{\"type\":\"user\",\"message\":{\"content\":\"salut\"}}\n",
        )
        .unwrap();
        let entries = parse_transcript(&p);
        assert_eq!(entries.len(), 2);
        assert!(entries[0].unparsable);
        assert!(entries[1].body.contains("salut"));
    }

    #[test]
    fn navigation_respects_bounds() {
        let dir = tempfile::tempdir().unwrap();
        let pdir = dir.path().join("projects").join("-home-x");
        fs::create_dir_all(&pdir).unwrap();
        fs::write(
            pdir.join("aaaaaaaa.jsonl"),
            r#"{"type":"user","cwd":"/home/x","message":{"content":"hi"}}"#,
        )
        .unwrap();
        let home = ClaudeHome::from_base(dir.path());
        let mut app = App::with_homes(vec![home]);
        assert!(!app.is_empty());

        // descendre sur les projets ne déborde pas (un seul projet)
        app.move_down();
        assert_eq!(app.project_idx, 0);

        // passer aux sessions et ouvrir le transcript
        app.toggle_focus();
        assert_eq!(app.focus, Focus::Sessions);
        app.open_transcript();
        assert_eq!(app.browse_view, BrowseView::Transcript);
        assert!(!app.transcript.is_empty());

        // Esc remonte à la liste
        assert!(app.back());
        assert_eq!(app.browse_view, BrowseView::List);
        // Esc à la racine ne consomme rien
        assert!(!app.back());
    }

    /// Construit deux homes (dont une avec un projet) et vérifie la sélection :
    /// `picker_select` change la home active et recharge les projets. N'écrit
    /// jamais dans la config réelle (méthodes purement en mémoire).
    fn two_homes() -> (tempfile::TempDir, Vec<ClaudeHome>) {
        let dir = tempfile::tempdir().unwrap();
        // home 0 : vide
        let h0 = dir.path().join("a");
        fs::create_dir_all(h0.join("projects")).unwrap();
        // home 1 : un projet
        let h1 = dir.path().join("b");
        let pdir = h1.join("projects").join("-home-x");
        fs::create_dir_all(&pdir).unwrap();
        fs::write(
            pdir.join("aaaa.jsonl"),
            r#"{"type":"user","cwd":"/home/x","message":{"content":"hi"}}"#,
        )
        .unwrap();
        let homes = vec![ClaudeHome::from_base(&h0), ClaudeHome::from_base(&h1)];
        (dir, homes)
    }

    fn two_homes_two_projects_each() -> (tempfile::TempDir, Vec<ClaudeHome>) {
        let dir = tempfile::tempdir().unwrap();
        for (home, encs) in [("a", ["-home-a1", "-home-a2"]), ("b", ["-home-b1", "-home-b2"])] {
            for enc in encs {
                let pd = dir.path().join(home).join("projects").join(enc);
                fs::create_dir_all(&pd).unwrap();
                fs::write(
                    pd.join("s.jsonl"),
                    format!(r#"{{"type":"user","cwd":"/home/{enc}"}}"#),
                )
                .unwrap();
            }
        }
        let homes = vec![
            ClaudeHome::from_base(dir.path().join("a")),
            ClaudeHome::from_base(dir.path().join("b")),
        ];
        (dir, homes)
    }

    #[test]
    fn collapse_hides_group_projects_and_navigates_anchors() {
        let (_d, homes) = two_homes_two_projects_each();
        let mut app = App::with_homes(homes);
        assert!(app.aggregate);
        assert_eq!(app.projects.len(), 4);
        // [0,1] = home a, [2,3] = home b (contigus par home).

        // Replie le home courant (a) : on reste sur l'ancre 0.
        app.toggle_collapse_current();
        assert!(app.is_home_collapsed(0));
        assert_eq!(app.project_idx, 0);

        // Descendre saute les projets repliés de a → première ancre de b.
        app.move_down();
        assert_eq!(app.project_idx, 2, "saute les projets repliés");

        // Replie b aussi : seules les ancres 0 et 2 restent navigables.
        app.toggle_collapse_current();
        assert!(app.is_home_collapsed(2));
        app.move_down();
        assert_eq!(app.project_idx, 2, "borné : dernière ancre navigable");
        app.move_up();
        assert_eq!(app.project_idx, 0, "remonte vers l'ancre précédente");

        // Tout était replié → tout déplier ; les projets internes réapparaissent.
        app.toggle_collapse_all();
        assert!(app.collapsed.is_empty());
        app.move_down();
        assert_eq!(app.project_idx, 1, "projet interne de nouveau navigable");
    }

    #[test]
    fn picker_open_move_and_select_switches_home() {
        let (_d, homes) = two_homes();
        let mut app = App::with_homes(homes);
        // Multi-home → agrégé par défaut (le projet de b est déjà visible).
        assert!(app.aggregate);
        assert!(!app.is_empty());

        app.open_picker();
        assert!(app.show_picker);
        // Agrégé → le sélecteur surligne « Tous les homes » (entrée 0).
        assert_eq!(app.picker_idx, 0);

        // Va sur la 2e home (entrée 2 : 0=Tous, 1=home a, 2=home b) puis sélectionne.
        app.picker_move(2);
        assert_eq!(app.picker_idx, 2);
        app.picker_select();
        assert!(!app.show_picker);
        assert!(!app.aggregate);
        assert_eq!(app.active, 1);
        assert!(!app.is_empty());
        assert!(app.status.as_deref().unwrap().contains("Home active"));
    }

    #[test]
    fn request_edit_targets_section_file() {
        let (_d, home) = temp_home();
        let mut app = App::with_homes(vec![home]);

        app.set_section(Section::Memory);
        let mem = app.home().memory_file();
        app.request_edit();
        assert_eq!(app.pending_edit, Some(mem));

        app.pending_edit = None;
        app.set_section(Section::Config);
        let cfg = app.home().settings_file();
        app.request_edit();
        assert_eq!(app.pending_edit, Some(cfg));

        app.pending_edit = None;
        app.set_section(Section::Browse);
        app.request_edit();
        assert!(app.pending_edit.is_none());
    }

    #[test]
    fn delete_session_trashes_and_reloads() {
        let dir = tempfile::tempdir().unwrap();
        let pdir = dir.path().join("projects").join("-home-x");
        fs::create_dir_all(&pdir).unwrap();
        fs::write(
            pdir.join("aaaa.jsonl"),
            r#"{"type":"user","cwd":"/home/x","message":{"content":"hi"}}"#,
        )
        .unwrap();
        let home = ClaudeHome::from_base(dir.path());
        let mut app = App::with_homes(vec![home]);
        app.toggle_focus();
        assert_eq!(app.focus, Focus::Sessions);

        app.request_delete_session();
        assert_eq!(app.confirm_delete, Some(DeleteKind::Session));
        app.confirm_delete_apply();
        assert!(app.confirm_delete.is_none());

        assert_eq!(app.session_count(), 0, "session partie en corbeille");
        assert!(dir.path().join("trash").exists(), "corbeille créée");
        assert!(!pdir.join("aaaa.jsonl").exists(), "original retiré");
    }

    #[test]
    fn delete_empty_project_from_projects_focus() {
        let dir = tempfile::tempdir().unwrap();
        let base = dir.path();
        // Projet « vide » (0 sess., juste un index) — le cas du ~ (0 sess.).
        let empty = base.join("projects").join("-home-kdelfour");
        fs::create_dir_all(&empty).unwrap();
        fs::write(empty.join("sessions-index.json"), "{}").unwrap();
        // Un projet avec session, qui doit survivre.
        let full = base.join("projects").join("-home-x-proj");
        fs::create_dir_all(&full).unwrap();
        fs::write(full.join("a.jsonl"), "{\"cwd\":\"/home/x/proj\"}\n").unwrap();

        let mut app = App::with_homes(vec![ClaudeHome::from_base(base)]);
        assert_eq!(app.focus, Focus::Projects);
        let idx = app
            .projects
            .iter()
            .position(|p| p.encoded_name == "-home-kdelfour")
            .unwrap();
        app.project_idx = idx;

        app.request_delete();
        assert_eq!(app.confirm_delete, Some(DeleteKind::Project));
        app.confirm_delete_apply();
        assert!(app.confirm_delete.is_none());

        assert!(!empty.exists(), "projet vide envoyé en corbeille");
        assert!(full.join("a.jsonl").exists(), "projet non vide intact");
        assert!(base.join("trash").exists(), "corbeille créée");
    }

    #[test]
    fn move_session_between_projects() {
        let dir = tempfile::tempdir().unwrap();
        let a = dir.path().join("projects").join("-home-a");
        fs::create_dir_all(&a).unwrap();
        fs::write(
            a.join("s1.jsonl"),
            r#"{"type":"user","cwd":"/home/a","message":{"content":"hi"}}"#,
        )
        .unwrap();
        let b = dir.path().join("projects").join("-home-b");
        fs::create_dir_all(&b).unwrap();
        fs::write(
            b.join("other.jsonl"),
            r#"{"type":"user","cwd":"/home/b","message":{"content":"yo"}}"#,
        )
        .unwrap();
        let home = ClaudeHome::from_base(dir.path());
        let mut app = App::with_homes(vec![home]);
        // Projets triés par nom : -home-a (idx 0), -home-b (idx 1). On part de A.
        app.project_idx = 0;
        app.toggle_focus();
        assert_eq!(app.focus, Focus::Sessions);

        app.request_move_session();
        let targets = app.move_targets.as_ref().expect("cibles");
        assert_eq!(targets.len(), 1, "seule cible = projet B");
        app.move_picker_select();
        assert!(app.move_targets.is_none());

        assert!(b.join("s1.jsonl").exists(), "session déplacée dans B");
        assert!(!a.join("s1.jsonl").exists(), "retirée de A");
    }

    #[test]
    fn extensions_section_loads_hooks_plugins_mcp() {
        let dir = tempfile::tempdir().unwrap();
        let base = dir.path();
        fs::create_dir_all(base.join("projects")).unwrap();
        fs::write(
            base.join("settings.json"),
            r#"{"hooks":{"PreToolUse":[{"matcher":"Bash","hooks":[{"type":"command","command":"echo hi"}]}]},"enabledPlugins":{"foo@m":true}}"#,
        )
        .unwrap();
        fs::create_dir_all(base.join("plugins")).unwrap();
        fs::write(
            base.join("plugins/installed_plugins.json"),
            r#"{"version":1,"plugins":{"foo@m":[{"scope":"user","version":"1.0.0"}]}}"#,
        )
        .unwrap();
        fs::write(
            base.join(".claude.json"),
            r#"{"mcpServers":{"fs":{"command":"npx","args":["server"]}}}"#,
        )
        .unwrap();

        let mut app = App::with_homes(vec![ClaudeHome::from_base(base)]);
        // Tab parcourt jusqu'à Extensions.
        app.set_section(Section::Config);
        app.next_section();
        assert_eq!(app.section, Section::Extensions);

        assert_eq!(app.extensions.hooks.len(), 1);
        assert_eq!(app.extensions.hooks[0].event, "PreToolUse");
        assert_eq!(app.extensions.plugins.len(), 1);
        assert!(app.extensions.plugins[0].enabled);
        assert_eq!(app.extensions.mcp.len(), 1);
        assert_eq!(app.extensions.mcp[0].name, "fs");
    }

    #[test]
    fn import_preview_then_apply_brings_sessions_in() {
        let dir = tempfile::tempdir().unwrap();
        // Source : un home avec une session, exporté en bundle.
        let src = dir.path().join("src");
        let spd = src.join("projects").join("-home-x-proj");
        fs::create_dir_all(&spd).unwrap();
        fs::write(spd.join("sess.jsonl"), "{\"cwd\":\"/home/x/proj\"}\n").unwrap();
        let bundle = dir.path().join("b.tar.gz");
        export(
            &ClaudeHome::from_base(&src),
            &bundle,
            &ExportOptions::default(),
        )
        .unwrap();

        // Cible : home vide.
        let tgt = dir.path().join("tgt");
        fs::create_dir_all(tgt.join("projects")).unwrap();
        let mut app = App::with_homes(vec![ClaudeHome::from_base(&tgt)]);
        assert!(app.is_empty());

        app.open_import();
        app.import.as_mut().unwrap().input = bundle.to_string_lossy().into_owned();
        app.import_preview();
        let (_, prev) = app.import.as_ref().unwrap().preview.as_ref().unwrap();
        assert_eq!(prev.sessions_new, 1);
        assert_eq!(prev.sessions_conflict, 0);

        app.import_apply();
        assert!(app.import.is_none());
        assert!(!app.is_empty(), "session importée visible");
        assert!(tgt
            .join("projects")
            .join("-home-x-proj")
            .join("sess.jsonl")
            .exists());
    }

    #[test]
    fn import_rejects_missing_bundle() {
        let dir = tempfile::tempdir().unwrap();
        let tgt = dir.path().join("tgt");
        fs::create_dir_all(tgt.join("projects")).unwrap();
        let mut app = App::with_homes(vec![ClaudeHome::from_base(&tgt)]);
        app.open_import();
        app.import.as_mut().unwrap().input = "/does/not/exist.tar.gz".to_string();
        app.import_preview();
        assert!(app.import.as_ref().unwrap().preview.is_none(), "pas d'aperçu");
        assert!(app.status.as_deref().unwrap().contains("introuvable"));
    }

    #[test]
    fn search_live_filters_by_path_without_reading_content() {
        let dir = tempfile::tempdir().unwrap();
        let pdir = dir.path().join("projects").join("-home-x-alpha");
        fs::create_dir_all(&pdir).unwrap();
        fs::write(
            pdir.join("aaaa.jsonl"),
            r#"{"type":"user","cwd":"/home/x/alpha","message":{"content":"zzz"}}"#,
        )
        .unwrap();
        let home = ClaudeHome::from_base(dir.path());
        let mut app = App::with_homes(vec![home]);

        app.open_search();
        for c in "alpha".chars() {
            app.search_input_char(c);
        }
        // Filtrage live par chemin, sans recherche profonde.
        assert!(!app.search.as_ref().unwrap().deep);
        assert_eq!(app.search.as_ref().unwrap().results.len(), 1);

        // Effacer la requête vide les résultats.
        for _ in 0..5 {
            app.search_input_backspace();
        }
        assert!(app.search.as_ref().unwrap().results.is_empty());
    }

    #[test]
    fn search_finds_and_opens_session() {
        let dir = tempfile::tempdir().unwrap();
        let pdir = dir.path().join("projects").join("-home-x");
        fs::create_dir_all(&pdir).unwrap();
        fs::write(
            pdir.join("aaaa.jsonl"),
            r#"{"type":"user","cwd":"/home/x","message":{"content":"refactor the WIDGET layout"}}"#,
        )
        .unwrap();
        let home = ClaudeHome::from_base(dir.path());
        let mut app = App::with_homes(vec![home]);

        app.open_search();
        for c in "widget".chars() {
            app.search_input_char(c);
        }
        // Le contenu n'est trouvé qu'avec la recherche profonde (Tab).
        assert!(app.search.as_ref().unwrap().results.is_empty(), "live = chemin/id seulement");
        app.search_deep();
        assert!(app.search.as_ref().unwrap().deep);
        assert_eq!(app.search.as_ref().unwrap().results.len(), 1);

        app.search_open_selected();
        assert!(app.search.is_none());
        assert_eq!(app.section, Section::Browse);
        assert_eq!(app.browse_view, BrowseView::Transcript);
        assert!(!app.transcript.is_empty());
    }

    #[test]
    fn delete_then_restore_from_trash() {
        let dir = tempfile::tempdir().unwrap();
        let pdir = dir.path().join("projects").join("-home-x");
        fs::create_dir_all(&pdir).unwrap();
        fs::write(
            pdir.join("aaaa.jsonl"),
            r#"{"type":"user","cwd":"/home/x","message":{"content":"hi"}}"#,
        )
        .unwrap();
        let home = ClaudeHome::from_base(dir.path());
        let mut app = App::with_homes(vec![home]);
        app.toggle_focus();

        app.request_delete_session();
        app.confirm_delete_apply();
        assert_eq!(app.session_count(), 0);

        app.open_trash();
        assert_eq!(app.trash_view.as_ref().unwrap().items.len(), 1);
        app.trash_restore_selected();
        assert!(app.trash_view.is_none(), "corbeille vidée → fermée");
        assert!(pdir.join("aaaa.jsonl").exists(), "session restaurée");
    }

    #[test]
    fn delete_then_purge_from_trash_is_permanent() {
        let dir = tempfile::tempdir().unwrap();
        let pdir = dir.path().join("projects").join("-home-x");
        fs::create_dir_all(&pdir).unwrap();
        fs::write(
            pdir.join("aaaa.jsonl"),
            r#"{"type":"user","cwd":"/home/x","message":{"content":"hi"}}"#,
        )
        .unwrap();
        let home = ClaudeHome::from_base(dir.path());
        let mut app = App::with_homes(vec![home]);
        app.toggle_focus();

        app.request_delete_session();
        app.confirm_delete_apply();

        app.open_trash();
        let trashed = app.trash_view.as_ref().unwrap().items[0].dir.clone();
        // Demande de purge → confirmation requise avant suppression.
        app.trash_request_purge();
        assert_eq!(app.trash_view.as_ref().unwrap().confirm, Some(PurgeScope::One));
        assert!(trashed.exists(), "rien supprimé avant confirmation");

        app.trash_confirm_apply();
        assert!(!trashed.exists(), "entrée supprimée définitivement");
        assert!(app.trash_view.is_none(), "corbeille vidée → fermée");
        assert!(!pdir.join("aaaa.jsonl").exists(), "non restaurable");
    }

    #[test]
    fn empty_trash_clears_all_homes() {
        let dir = tempfile::tempdir().unwrap();
        let pdir = dir.path().join("projects").join("-home-x");
        fs::create_dir_all(&pdir).unwrap();
        for id in ["aaaa", "bbbb"] {
            fs::write(
                pdir.join(format!("{id}.jsonl")),
                r#"{"type":"user","cwd":"/home/x","message":{"content":"hi"}}"#,
            )
            .unwrap();
        }
        let home = ClaudeHome::from_base(dir.path());
        let mut app = App::with_homes(vec![home]);
        app.toggle_focus();

        app.request_delete_session();
        app.confirm_delete_apply();
        app.request_delete_session();
        app.confirm_delete_apply();

        app.open_trash();
        assert!(!app.trash_view.as_ref().unwrap().items.is_empty());
        // Robuste à un éventuel partage d'horodatage : 2 sessions au total.
        let sessions: usize = list_trash(dir.path()).iter().map(|e| e.sessions).sum();
        assert_eq!(sessions, 2);
        app.trash_request_empty();
        assert_eq!(app.trash_view.as_ref().unwrap().confirm, Some(PurgeScope::All));
        app.trash_confirm_apply();

        assert!(app.trash_view.is_none(), "corbeille fermée après vidage");
        assert!(list_trash(dir.path()).is_empty(), "corbeille effectivement vidée");
    }

    #[test]
    fn cycle_config_target_in_aggregate_changes_active_only() {
        let (_d, homes) = two_homes();
        let mut app = App::with_homes(homes);
        app.open_picker();
        app.picker_idx = 0;
        app.picker_select(); // → agrégé
        assert!(app.aggregate);
        let before = app.active;
        let proj_count = app.projects.len();

        app.set_section(Section::Config);
        app.cycle_config_target();
        assert_ne!(app.active, before, "la cible doit changer");
        assert!(app.aggregate, "doit rester en agrégé");
        assert_eq!(app.projects.len(), proj_count, "liste fusionnée inchangée");
    }

    #[test]
    fn picker_select_all_homes_aggregates() {
        let (_d, homes) = two_homes();
        let mut app = App::with_homes(homes);
        app.open_picker();
        app.picker_idx = 0; // « Tous les homes »
        app.picker_select();
        assert!(app.aggregate);
        // Agrégé : projets des deux homes (a vide + b avec 1 projet).
        assert_eq!(app.projects.len(), 1);
        assert_eq!(app.project_homes.len(), app.projects.len());
        assert!(app.status.as_deref().unwrap().contains("Tous les homes"));
    }

    #[test]
    fn picker_input_mode_buffers_and_cancels() {
        let (_d, homes) = two_homes();
        let mut app = App::with_homes(homes);
        app.open_picker();
        app.picker_start_add();
        app.picker_input_char('/');
        app.picker_input_char('t');
        app.picker_input_char('z');
        app.picker_input_backspace();
        match &app.picker_mode {
            PickerMode::AddInput(buf) => assert_eq!(buf, "/t"),
            _ => panic!("attendu AddInput"),
        }
        // Esc (cancel) revient en mode liste sans fermer le popup.
        app.picker_cancel_input();
        assert_eq!(app.picker_mode, PickerMode::List);
        assert!(app.show_picker);
    }

    #[test]
    fn picker_confirm_invalid_path_sets_error_status() {
        let (_d, homes) = two_homes();
        let mut app = App::with_homes(homes);
        app.open_picker();
        app.picker_start_add();
        for c in "/n/existe/pas-claudine".chars() {
            app.picker_input_char(c);
        }
        app.picker_confirm_add();
        // Reste en mode saisie, statut d'erreur affiché.
        assert!(matches!(app.picker_mode, PickerMode::AddInput(_)));
        assert!(app.status.as_deref().unwrap().contains("invalide"));
    }

    #[test]
    fn hooks_editor_open_edit_and_save_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let base = dir.path();
        fs::create_dir_all(base.join("projects")).unwrap();
        fs::write(base.join("settings.json"), "{}").unwrap();
        let mut app = App::with_homes(vec![ClaudeHome::from_base(base)]);
        app.set_section(Section::Extensions);

        app.open_hooks_editor();
        assert!(app.hooks_editor.is_some());
        {
            let e = app.hooks_editor.as_mut().unwrap();
            e.add_group(); // un groupe PreToolUse vide
            e.enter();
            e.add_command();
            e.begin_edit();
            for c in "echo hi".chars() {
                e.input_char(c);
            }
            e.input_commit();
        }
        app.hooks_save();
        assert!(app.hooks_editor.is_none(), "fermé après enregistrement");

        let groups = claudine_core::read_hook_groups(app.home());
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].event, "PreToolUse");
        assert_eq!(groups[0].commands[0].command, "echo hi");
    }
}

