//! État applicatif de la TUI Claudine et logique de navigation.

use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use claudine_core::{
    discover_homes, export, scan_projects, ClaudeHome, ClaudineConfig, ExportOptions, Project,
    SessionMeta,
};
use serde_json::Value;

use crate::tui::settings_form::SettingsForm;

/// Sections de premier niveau, sélectionnables avec Tab / 1,2,3.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Section {
    Browse,
    Memory,
    Config,
}

impl Section {
    pub fn title(self) -> &'static str {
        match self {
            Section::Browse => "Projets",
            Section::Memory => "Mémoire",
            Section::Config => "Config",
        }
    }

    pub fn index(self) -> usize {
        match self {
            Section::Browse => 0,
            Section::Memory => 1,
            Section::Config => 2,
        }
    }

    pub fn next(self) -> Section {
        match self {
            Section::Browse => Section::Memory,
            Section::Memory => Section::Config,
            Section::Config => Section::Browse,
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

    // --- Sélecteur de home ---
    pub show_picker: bool,
    pub picker_idx: usize,
    pub picker_mode: PickerMode,

    // --- Browse ---
    pub projects: Vec<Project>,
    /// Label du home d'origine de chaque projet (aligné sur `projects`).
    pub project_homes: Vec<String>,
    /// Vue agrégée : projets de tous les homes à la fois.
    pub aggregate: bool,
    pub browse_view: BrowseView,
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

        let home = &homes[0];
        let projects = scan_projects(home).unwrap_or_default();
        let project_homes = vec![home.label.clone(); projects.len()];
        let memory_lines = read_file_lines(home.memory_file(), "(aucune mémoire utilisateur)");
        let config_lines = build_config_lines(home);
        let settings = SettingsForm::load(home);
        App {
            homes,
            active: 0,
            section: Section::Browse,
            should_quit: false,
            show_help: false,
            status: None,
            show_picker: false,
            picker_idx: 0,
            picker_mode: PickerMode::List,
            projects,
            project_homes,
            aggregate: false,
            browse_view: BrowseView::List,
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

    fn project_count(&self) -> usize {
        self.projects.len()
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
        }
    }

    fn browse_move(&mut self, delta: i32) {
        match self.focus {
            Focus::Projects => {
                let n = self.project_count();
                self.project_idx = step(self.project_idx, delta, n);
                // Le changement de projet réinitialise la sélection de session.
                self.session_idx = 0;
            }
            Focus::Sessions => {
                let n = self.session_count();
                self.session_idx = step(self.session_idx, delta, n);
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
            _ => {}
        }
    }

    pub fn page_up(&mut self) {
        match self.section {
            Section::Browse if self.browse_view == BrowseView::Transcript => {
                let step = self.transcript_viewport.max(1);
                self.transcript_scroll = self.transcript_scroll.saturating_sub(step);
            }
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
            _ => {}
        }
    }

    pub fn go_home(&mut self) {
        match self.section {
            Section::Browse if self.browse_view == BrowseView::Transcript => {
                self.transcript_scroll = 0
            }
            Section::Memory => self.memory_scroll = 0,
            Section::Config => {
                if self.settings.raw() {
                    self.config_scroll = 0;
                } else {
                    self.settings.go_first();
                }
            }
            _ => {}
        }
    }

    pub fn go_end(&mut self) {
        match self.section {
            Section::Browse if self.browse_view == BrowseView::Transcript => {
                self.transcript_scroll = self.transcript_max_scroll()
            }
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
            _ => {}
        }
    }

    // --- Config : formulaire de réglages ---

    /// Enter dans la section courante : ouvre un transcript (Browse) ou active le
    /// champ surligné du formulaire (Config, hors JSON brut).
    pub fn on_enter(&mut self) {
        match self.section {
            Section::Browse => self.open_transcript(),
            Section::Config if !self.settings.raw() => self.settings.activate(),
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
            || !matches!(self.section, Section::Memory | Section::Config)
        {
            return;
        }
        self.active = (self.active + 1) % self.homes.len();
        let home = self.homes[self.active].clone();
        self.memory_lines = read_file_lines(home.memory_file(), "(aucune mémoire utilisateur)");
        self.config_lines = build_config_lines(&home);
        self.settings = SettingsForm::load(&home);
        self.memory_scroll = 0;
        self.config_scroll = 0;
        self.status = Some(format!("Cible Mémoire/Config : {}", home.label));
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
        let homes: Vec<&ClaudeHome> = if self.aggregate {
            self.homes.iter().collect()
        } else {
            vec![&self.homes[self.active]]
        };
        for h in homes {
            if let Ok(ps) = scan_projects(h) {
                for p in ps {
                    project_homes.push(h.label.clone());
                    projects.push(p);
                }
            }
        }
        self.projects = projects;
        self.project_homes = project_homes;
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

    #[test]
    fn picker_open_move_and_select_switches_home() {
        let (_d, homes) = two_homes();
        let mut app = App::with_homes(homes);
        assert!(app.is_empty()); // home active = vide

        app.open_picker();
        assert!(app.show_picker);
        // Index 0 = « Tous les homes » ; le home actif (0) est donc à l'entrée 1.
        assert_eq!(app.picker_idx, 1);

        // Descend sur la 2e home (entrée 2) puis sélectionne.
        app.picker_move(1);
        assert_eq!(app.picker_idx, 2);
        app.picker_select();
        assert!(!app.show_picker);
        assert_eq!(app.active, 1);
        assert!(!app.aggregate);
        // La home b a un projet → l'app n'est plus vide après reload.
        assert!(!app.is_empty());
        assert!(app.status.as_deref().unwrap().contains("Home active"));
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
}
