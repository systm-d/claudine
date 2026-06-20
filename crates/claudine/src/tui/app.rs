//! État applicatif de la TUI Claudine et logique de navigation.

use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use claudine_core::{export, scan_projects, ClaudeHome, ExportOptions, Project, SessionMeta};
use serde_json::Value;

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

/// État central de l'application TUI.
pub struct App {
    pub home: ClaudeHome,
    pub section: Section,
    pub should_quit: bool,
    pub show_help: bool,
    pub status: Option<String>,

    // --- Browse ---
    pub projects: Vec<Project>,
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
}

impl App {
    /// Construit l'application à partir d'une `ClaudeHome`, en chargeant les
    /// projets / mémoire / config. Les erreurs de scan sont tolérées (liste vide).
    pub fn new(home: ClaudeHome) -> App {
        let projects = scan_projects(&home).unwrap_or_default();
        let memory_lines = read_file_lines(home.memory_file(), "(aucune mémoire utilisateur)");
        let config_lines = build_config_lines(&home);
        App {
            home,
            section: Section::Browse,
            should_quit: false,
            show_help: false,
            status: None,
            projects,
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
        }
    }

    // --- Accès ---

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
            Section::Config => self.config_scroll = scroll_add(self.config_scroll, 1),
        }
    }

    pub fn move_up(&mut self) {
        match self.section {
            Section::Browse => match self.browse_view {
                BrowseView::List => self.browse_move(-1),
                BrowseView::Transcript => self.scroll_transcript(-1),
            },
            Section::Memory => self.memory_scroll = self.memory_scroll.saturating_sub(1),
            Section::Config => self.config_scroll = self.config_scroll.saturating_sub(1),
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
                self.config_scroll = page(self.config_scroll, self.config_viewport, true);
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
                self.config_scroll = page(self.config_scroll, self.config_viewport, false);
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
            Section::Config => self.config_scroll = 0,
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
                self.config_scroll = self
                    .config_lines
                    .len()
                    .saturating_sub(self.config_viewport)
            }
            _ => {}
        }
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

        match export(&self.home, &out, &ExportOptions::default()) {
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
}

// --- Helpers libres ---

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
        let app = App::new(home);
        assert!(app.is_empty());
        assert_eq!(app.section, Section::Browse);
        // mémoire absente → message de repli
        assert_eq!(app.memory_lines, vec!["(aucune mémoire utilisateur)".to_string()]);
    }

    #[test]
    fn sections_cycle_with_tab() {
        let (_d, home) = temp_home();
        let mut app = App::new(home);
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
        let mut app = App::new(home);
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
}
