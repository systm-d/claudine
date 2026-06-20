//! Rendu de l'interface Claudine (ratatui).

use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{
        Block, Borders, Clear, List, ListItem, ListState, Paragraph, Tabs, Wrap,
    },
    Frame,
};

use super::app::{App, BrowseView, Focus, PickerMode, Section};
use crate::tui::app::human_size;
use claudine_core::scan_projects;

const ACCENT: Color = Color::Cyan;
const DIM: Color = Color::DarkGray;

/// Point d'entrée du rendu : un cadre complet.
pub fn render(app: &mut App, f: &mut Frame) {
    let area = f.area();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // header + onglets
            Constraint::Min(1),    // corps
            Constraint::Length(1), // ligne de statut
            Constraint::Length(1), // pied (raccourcis)
        ])
        .split(area);

    render_header(app, f, chunks[0]);

    match app.section {
        Section::Browse => render_browse(app, f, chunks[1]),
        Section::Memory => render_scroll_pane(
            f,
            chunks[1],
            "Mémoire — ~/.claude/CLAUDE.md",
            &app.memory_lines,
            app.memory_scroll,
            &mut app.memory_viewport,
        ),
        Section::Config => render_scroll_pane(
            f,
            chunks[1],
            "Config — settings.json / settings.local.json",
            &app.config_lines,
            app.config_scroll,
            &mut app.config_viewport,
        ),
    }

    render_status(app, f, chunks[2]);
    render_footer(app, f, chunks[3]);

    if app.show_picker {
        render_picker(app, f, area);
    }

    if app.show_help {
        render_help(f, area);
    }
}

fn render_header(app: &App, f: &mut Frame, area: Rect) {
    let titles = [Section::Browse, Section::Memory, Section::Config]
        .iter()
        .map(|s| Line::from(format!(" {} ", s.title())))
        .collect::<Vec<_>>();
    let title = format!(" Claudine · {} ", app.home().label);
    let tabs = Tabs::new(titles)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(Span::styled(
                    title,
                    Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
                ))
                .title(Line::from(" H homes · ? aide ").right_aligned()),
        )
        .select(app.section.index())
        .highlight_style(
            Style::default()
                .fg(Color::Black)
                .bg(ACCENT)
                .add_modifier(Modifier::BOLD),
        )
        .divider(Span::styled("│", Style::default().fg(DIM)));
    f.render_widget(tabs, area);
}

fn render_browse(app: &mut App, f: &mut Frame, area: Rect) {
    if app.is_empty() {
        render_empty(f, area);
        return;
    }
    match app.browse_view {
        BrowseView::List => render_lists(app, f, area),
        BrowseView::Transcript => render_transcript(app, f, area),
    }
}

fn render_empty(f: &mut Frame, area: Rect) {
    let block = Block::default().borders(Borders::ALL).title(" Projets ");
    let inner = block.inner(area);
    f.render_widget(block, area);
    let msg = Text::from(vec![
        Line::from(""),
        Line::from(Span::styled(
            "Aucun projet trouvé.",
            Style::default().fg(DIM).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "Aucun ~/.claude/projects ou répertoire vide.",
            Style::default().fg(DIM),
        )),
        Line::from(Span::styled(
            "Lancez Claude Code dans un projet pour générer des sessions.",
            Style::default().fg(DIM),
        )),
    ]);
    let p = Paragraph::new(msg).alignment(Alignment::Center);
    // Centre verticalement (approx.) en réservant le haut.
    let vchunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(35), Constraint::Min(1)])
        .split(inner);
    f.render_widget(p, vchunks[1]);
}

fn render_lists(app: &mut App, f: &mut Frame, area: Rect) {
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(45), Constraint::Percentage(55)])
        .split(area);

    // --- Projets ---
    let projects_focused = app.focus == Focus::Projects;
    let proj_items: Vec<ListItem> = app
        .projects
        .iter()
        .map(|p| {
            let label = p.cwd.clone().unwrap_or_else(|| p.encoded_name.clone());
            let count = p.sessions.len();
            ListItem::new(Line::from(vec![
                Span::raw(label),
                Span::styled(
                    format!("  ({count} sess.)"),
                    Style::default().fg(DIM),
                ),
            ]))
        })
        .collect();
    let proj_list = List::new(proj_items)
        .block(pane_block(" Projets ", projects_focused))
        .highlight_style(selection_style(projects_focused))
        .highlight_symbol("▶ ");
    let mut proj_state = ListState::default();
    proj_state.select(Some(app.project_idx));
    f.render_stateful_widget(proj_list, cols[0], &mut proj_state);

    // --- Sessions ---
    let sessions_focused = app.focus == Focus::Sessions;
    let sess_items: Vec<ListItem> = app
        .selected_project()
        .map(|p| {
            p.sessions
                .iter()
                .map(|s| {
                    let short = s.id.chars().take(8).collect::<String>();
                    let last = s.last_ts.clone().unwrap_or_else(|| "—".to_string());
                    ListItem::new(Line::from(vec![
                        Span::styled(
                            short,
                            Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
                        ),
                        Span::styled(
                            format!("  {} msg", s.message_count),
                            Style::default().fg(Color::Gray),
                        ),
                        Span::styled(format!("  {last}"), Style::default().fg(DIM)),
                        Span::styled(
                            format!("  {}", human_size(s.size)),
                            Style::default().fg(DIM),
                        ),
                    ]))
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    let title = match app.selected_project() {
        Some(p) => {
            let name = p.cwd.clone().unwrap_or_else(|| p.encoded_name.clone());
            format!(" Sessions — {name} ")
        }
        None => " Sessions ".to_string(),
    };

    if sess_items.is_empty() {
        let block = pane_block(&title, sessions_focused);
        let p = Paragraph::new(Span::styled(
            "  (aucune session)",
            Style::default().fg(DIM),
        ))
        .block(block);
        f.render_widget(p, cols[1]);
    } else {
        let sess_list = List::new(sess_items)
            .block(pane_block(&title, sessions_focused))
            .highlight_style(selection_style(sessions_focused))
            .highlight_symbol("▶ ");
        let mut sess_state = ListState::default();
        sess_state.select(Some(app.session_idx));
        f.render_stateful_widget(sess_list, cols[1], &mut sess_state);
    }
}

fn render_transcript(app: &mut App, f: &mut Frame, area: Rect) {
    let (proj_name, sess_id) = match (app.selected_project(), app.selected_session()) {
        (Some(p), Some(s)) => {
            let name = p.cwd.clone().unwrap_or_else(|| p.encoded_name.clone());
            (name, s.id.chars().take(8).collect::<String>())
        }
        _ => ("?".to_string(), "?".to_string()),
    };
    let title = format!(" Transcript — {proj_name} · {sess_id} ");

    let block = Block::default()
        .borders(Borders::ALL)
        .title(Span::styled(
            title,
            Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
        ))
        .title(Line::from(" Esc retour ").right_aligned());
    let inner = block.inner(area);
    f.render_widget(block, area);
    app.transcript_viewport = inner.height as usize;

    let mut lines: Vec<Line> = Vec::new();
    for entry in &app.transcript {
        let header_style = if entry.unparsable {
            Style::default().fg(DIM).add_modifier(Modifier::DIM)
        } else {
            Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)
        };
        lines.push(Line::from(Span::styled(entry.header.clone(), header_style)));
        for body_line in entry.body.lines() {
            let style = if entry.unparsable {
                Style::default().fg(DIM).add_modifier(Modifier::DIM)
            } else {
                Style::default().fg(Color::White)
            };
            lines.push(Line::from(Span::styled(body_line.to_string(), style)));
        }
        if entry.body.is_empty() {
            lines.push(Line::from(""));
        }
        lines.push(Line::from(""));
    }

    let para = Paragraph::new(Text::from(lines))
        .wrap(Wrap { trim: false })
        .scroll((app.transcript_scroll as u16, 0));
    f.render_widget(para, inner);
}

/// Panneau scrollable simple (Mémoire / Config). Met à jour `viewport` avec la
/// hauteur effective afin que la pagination reste cohérente.
fn render_scroll_pane(
    f: &mut Frame,
    area: Rect,
    title: &str,
    lines: &[String],
    scroll: usize,
    viewport: &mut usize,
) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(Span::styled(
            format!(" {title} "),
            Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
        ));
    let inner = block.inner(area);
    f.render_widget(block, area);
    *viewport = inner.height as usize;

    let text: Vec<Line> = lines
        .iter()
        .map(|l| {
            if l.starts_with("── ") || l.starts_with("("){
                Line::from(Span::styled(l.clone(), Style::default().fg(ACCENT)))
            } else {
                Line::from(l.clone())
            }
        })
        .collect();
    let para = Paragraph::new(Text::from(text))
        .wrap(Wrap { trim: false })
        .scroll((scroll as u16, 0));
    f.render_widget(para, inner);
}

fn render_status(app: &App, f: &mut Frame, area: Rect) {
    let line = match &app.status {
        Some(msg) => {
            let style = if msg.starts_with("Échec") {
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::Green)
            };
            Line::from(Span::styled(format!(" {msg}"), style))
        }
        None => Line::from(Span::styled(
            " prêt",
            Style::default().fg(DIM),
        )),
    };
    f.render_widget(Paragraph::new(line), area);
}

fn render_footer(app: &App, f: &mut Frame, area: Rect) {
    // Le sélecteur de home a ses propres raccourcis prioritaires.
    if app.show_picker {
        let hints = if matches!(app.picker_mode, PickerMode::AddInput(_)) {
            key_hints(&[
                ("saisir", "chemin"),
                ("Backspace", "effacer"),
                ("Enter", "valider"),
                ("Esc", "annuler"),
            ])
        } else {
            key_hints(&[
                ("↑/↓ · j/k", "naviguer"),
                ("Enter", "activer"),
                ("a", "ajouter"),
                ("d", "retirer"),
                ("Esc", "fermer"),
            ])
        };
        f.render_widget(Paragraph::new(Line::from(hints)), area);
        return;
    }

    let hints: Vec<Span> = match app.section {
        Section::Browse if app.browse_view == BrowseView::Transcript => key_hints(&[
            ("↑/↓", "défiler"),
            ("PgUp/PgDn", "page"),
            ("Home/End", "bornes"),
            ("Esc", "retour"),
            ("e", "export"),
            ("H", "homes"),
            ("?", "aide"),
            ("q", "quitter"),
        ]),
        Section::Browse => key_hints(&[
            ("Tab/1·2·3", "sections"),
            ("←/→", "panneau"),
            ("↑/↓", "naviguer"),
            ("Enter", "ouvrir"),
            ("e", "export"),
            ("H", "homes"),
            ("?", "aide"),
            ("q", "quitter"),
        ]),
        _ => key_hints(&[
            ("Tab/1·2·3", "sections"),
            ("↑/↓", "défiler"),
            ("PgUp/PgDn", "page"),
            ("e", "export"),
            ("H", "homes"),
            ("?", "aide"),
            ("q", "quitter"),
        ]),
    };
    f.render_widget(Paragraph::new(Line::from(hints)), area);
}

fn render_help(f: &mut Frame, area: Rect) {
    let popup = centered_rect(64, 70, area);
    f.render_widget(Clear, popup);
    let block = Block::default()
        .borders(Borders::ALL)
        .title(Span::styled(
            " Aide — raccourcis ",
            Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
        ));
    let rows = [
        ("1 / 2 / 3", "aller à Projets / Mémoire / Config"),
        ("Tab", "section suivante"),
        ("← → / h l", "changer de panneau (Browse)"),
        ("↑ ↓ / j k", "naviguer / défiler"),
        ("Enter", "ouvrir la session sélectionnée"),
        ("Esc", "retour (transcript) sinon quitter"),
        ("PgUp / PgDn", "défilement par page"),
        ("Home / End", "aller au début / à la fin"),
        ("e", "exporter ~/.claude en .tar.gz"),
        ("H", "sélecteur de home (a ajouter / d retirer)"),
        ("?", "afficher/masquer cette aide"),
        ("q / Ctrl-C", "quitter"),
    ];
    let mut lines = vec![Line::from("")];
    for (k, d) in rows {
        lines.push(Line::from(vec![
            Span::styled(
                format!("  {k:<14}"),
                Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
            ),
            Span::raw(d),
        ]));
    }
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "  (? ou Esc pour fermer)",
        Style::default().fg(DIM),
    )));
    let para = Paragraph::new(Text::from(lines)).block(block);
    f.render_widget(para, popup);
}

/// Popup centré du sélecteur de home : liste `label (n projets)`, home active
/// marquée, plus une ligne de saisie quand on ajoute une home.
fn render_picker(app: &App, f: &mut Frame, area: Rect) {
    let popup = centered_rect(60, 60, area);
    f.render_widget(Clear, popup);

    let adding = matches!(app.picker_mode, PickerMode::AddInput(_));
    let block = Block::default()
        .borders(Borders::ALL)
        .title(Span::styled(
            " Homes Claude ",
            Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
        ))
        .title(
            Line::from(if adding {
                " Entrée valider · Esc annuler "
            } else {
                " a ajouter · d retirer · Esc fermer "
            })
            .right_aligned(),
        );
    let inner = block.inner(popup);
    f.render_widget(block, popup);

    // Réserve la dernière ligne pour la saisie quand on ajoute.
    let (list_area, input_area) = if adding {
        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(1), Constraint::Length(2)])
            .split(inner);
        (rows[0], Some(rows[1]))
    } else {
        (inner, None)
    };

    let items: Vec<ListItem> = app
        .homes
        .iter()
        .enumerate()
        .map(|(i, h)| {
            let n = scan_projects(h).map(|p| p.len()).unwrap_or(0);
            let mark = if i == app.active { "● " } else { "  " };
            ListItem::new(Line::from(vec![
                Span::styled(
                    mark,
                    Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
                ),
                Span::raw(h.label.clone()),
                Span::styled(
                    format!("  ({n} projets)"),
                    Style::default().fg(DIM),
                ),
            ]))
        })
        .collect();

    let list = List::new(items)
        .highlight_style(
            Style::default()
                .bg(ACCENT)
                .fg(Color::Black)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("▶ ");
    let mut state = ListState::default();
    state.select(Some(app.picker_idx.min(app.homes.len().saturating_sub(1))));
    f.render_stateful_widget(list, list_area, &mut state);

    if let (Some(input_area), PickerMode::AddInput(buf)) = (input_area, &app.picker_mode) {
        let line = Line::from(vec![
            Span::styled(
                "Chemin : ",
                Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
            ),
            Span::raw(buf.clone()),
            Span::styled("▏", Style::default().fg(ACCENT)),
        ]);
        f.render_widget(Paragraph::new(line), input_area);
    }
}

// --- Helpers de style/layout ---

fn pane_block(title: &str, focused: bool) -> Block<'static> {
    let border_style = if focused {
        Style::default().fg(ACCENT)
    } else {
        Style::default().fg(DIM)
    };
    Block::default()
        .borders(Borders::ALL)
        .border_style(border_style)
        .title(Span::styled(
            title.to_string(),
            if focused {
                Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::Gray)
            },
        ))
}

fn selection_style(focused: bool) -> Style {
    if focused {
        Style::default()
            .bg(ACCENT)
            .fg(Color::Black)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().add_modifier(Modifier::BOLD)
    }
}

fn key_hints(pairs: &[(&str, &str)]) -> Vec<Span<'static>> {
    let mut spans = Vec::new();
    for (i, (k, d)) in pairs.iter().enumerate() {
        if i > 0 {
            spans.push(Span::styled("  ", Style::default()));
        }
        spans.push(Span::styled(
            format!(" {k} "),
            Style::default()
                .bg(DIM)
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        ));
        spans.push(Span::styled(
            format!(" {d}"),
            Style::default().fg(Color::Gray),
        ));
    }
    spans
}

fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(area);
    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(vertical[1])[1]
}

#[cfg(test)]
mod tests {
    use super::*;
    use claudine_core::ClaudeHome;
    use ratatui::{backend::TestBackend, Terminal};
    use std::fs;

    fn render_section(home: ClaudeHome, section: Section) {
        let mut app = App::with_homes(vec![home]);
        app.set_section(section);
        let backend = TestBackend::new(90, 30);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|f| render(&mut app, f)).unwrap();
        let buf = terminal.backend().buffer().clone();
        // Le buffer ne doit pas être vide (au moins un caractère non-espace).
        let non_empty = buf.content().iter().any(|c| c.symbol() != " ");
        assert!(non_empty, "buffer vide pour la section {section:?}");
    }

    fn empty_home() -> (tempfile::TempDir, ClaudeHome) {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir_all(dir.path().join("projects")).unwrap();
        let home = ClaudeHome::from_base(dir.path());
        (dir, home)
    }

    #[test]
    fn smoke_renders_all_sections() {
        let (_d, home) = empty_home();
        render_section(home.clone(), Section::Browse);
        render_section(home.clone(), Section::Memory);
        render_section(home, Section::Config);
    }

    #[test]
    fn smoke_renders_with_data_and_transcript() {
        let dir = tempfile::tempdir().unwrap();
        let pdir = dir.path().join("projects").join("-home-demo");
        fs::create_dir_all(&pdir).unwrap();
        fs::write(
            pdir.join("deadbeef-0000.jsonl"),
            "{\"type\":\"user\",\"cwd\":\"/home/demo\",\"timestamp\":\"2026-01-01T00:00:00Z\",\"message\":{\"role\":\"user\",\"content\":\"bonjour\"}}\nnot-json\n",
        )
        .unwrap();
        let home = ClaudeHome::from_base(dir.path());
        let mut app = App::with_homes(vec![home]);
        app.toggle_focus();
        app.open_transcript();

        let backend = TestBackend::new(100, 30);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|f| render(&mut app, f)).unwrap();
        let buf = terminal.backend().buffer().clone();
        assert!(buf.content().iter().any(|c| c.symbol() != " "));
    }

    #[test]
    fn smoke_renders_help_overlay() {
        let (_d, home) = empty_home();
        let mut app = App::with_homes(vec![home]);
        app.toggle_help();
        let backend = TestBackend::new(90, 30);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|f| render(&mut app, f)).unwrap();
        assert!(terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .any(|c| c.symbol() != " "));
    }

    /// Construit deux fausses homes via `discover_homes_in` sur un tempdir, puis
    /// rend le cadre principal ET le popup du sélecteur sans paniquer.
    #[test]
    fn smoke_renders_home_picker() {
        use claudine_core::discover_homes_in;

        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();

        // .claude : qualifie via settings.json
        fs::create_dir_all(root.join(".claude")).unwrap();
        fs::write(root.join(".claude/settings.json"), "{}").unwrap();
        // .claude-perso : qualifie via projects/ non vide
        let pdir = root.join(".claude-perso/projects/-home-x");
        fs::create_dir_all(&pdir).unwrap();
        fs::write(pdir.join("x.jsonl"), "{}").unwrap();

        let homes = discover_homes_in(root, None);
        assert_eq!(homes.len(), 2, "deux homes attendues");

        let mut app = App::with_homes(homes);

        let backend = TestBackend::new(90, 30);
        let mut terminal = Terminal::new(backend).unwrap();

        // Cadre principal (titre = home active).
        terminal.draw(|f| render(&mut app, f)).unwrap();
        assert!(terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .any(|c| c.symbol() != " "));

        // Popup du sélecteur (mode liste).
        app.open_picker();
        terminal.draw(|f| render(&mut app, f)).unwrap();
        assert!(app.show_picker);
        assert!(terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .any(|c| c.symbol() != " "));

        // Popup en mode saisie de chemin (ajout de home).
        app.picker_start_add();
        app.picker_input_char('/');
        app.picker_input_char('x');
        terminal.draw(|f| render(&mut app, f)).unwrap();
        assert!(matches!(app.picker_mode, PickerMode::AddInput(_)));
    }
}
