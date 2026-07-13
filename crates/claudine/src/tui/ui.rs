//! Rendu de l'interface Claudine (ratatui).

use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Tabs, Wrap},
};

use super::app::{App, BrowseView, DeleteKind, Focus, PickerMode, PurgeScope, Section};
use crate::tui::app::{MktJob, human_size, humanize_path};
use crate::tui::hooks_editor::{HookEdit, HooksLevel, KNOWN_EVENTS};
use crate::tui::marketplaces::MktMode;
use crate::tui::marketplaces::PluginCatalog;
use crate::tui::mcp_editor::{McpEdit, McpLevel, McpRow};
use claudine_core::{MarketplaceSource, McpTransport, scan_projects};

const ACCENT: Color = Color::Cyan;
const DIM: Color = Color::DarkGray;

/// Point d'entrée du rendu : un cadre complet.
pub fn render(app: &mut App, f: &mut Frame) {
    let area = f.area();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(5), // header : onglets + logo Claude
            Constraint::Min(1),    // corps
            Constraint::Length(1), // ligne de statut
            Constraint::Length(1), // pied (raccourcis)
        ])
        .split(area);

    render_header(app, f, chunks[0]);

    match app.section {
        Section::Browse => render_browse(app, f, chunks[1]),
        Section::Memory => {
            let title = if app.aggregate {
                format!("Mémoire · {} — CLAUDE.md  (t: cible)", app.home().label)
            } else {
                "Mémoire — ~/.claude/CLAUDE.md".to_string()
            };
            render_scroll_pane(
                f,
                chunks[1],
                &title,
                &app.memory_lines,
                app.memory_scroll,
                &mut app.memory_viewport,
            )
        }
        Section::Config => render_config(app, f, chunks[1]),
        Section::Extensions => render_extensions(app, f, chunks[1]),
    }

    render_status(app, f, chunks[2]);
    render_footer(app, f, chunks[3]);

    if app.section == Section::Config && app.settings.list_state().is_some() {
        render_settings_list_editor(app, f, area);
    }

    if app.confirm_delete.is_some() {
        render_confirm_delete(app, f, area);
    }
    if app.move_targets.is_some() {
        render_move_picker(app, f, area);
    }
    if app.search.is_some() {
        render_search(app, f, area);
    }
    if app.trash_view.is_some() {
        render_trash(app, f, area);
    }
    if app.import.is_some() {
        render_import(app, f, area);
    }
    if app.hooks_editor.is_some() {
        render_hooks_editor(app, f, area);
    }
    if app.plugins_toggle.is_some() {
        render_plugins_toggle(app, f, area);
    }
    if app.mcp_editor.is_some() {
        render_mcp_editor(app, f, area);
    }
    if app.marketplaces.is_some() {
        render_marketplaces(app, f, area);
    }

    if app.show_picker {
        render_picker(app, f, area);
    }

    if app.show_help {
        render_help(f, area);
    }
}

/// Logo Claude Code (glyphe officiel de la boîte « What's new ») en demi-blocs,
/// couleur Claude.
fn claude_logo_lines() -> Vec<Line<'static>> {
    let o = Style::default().fg(Color::Rgb(0xd9, 0x77, 0x57));
    vec![
        Line::from(Span::styled(" ▐▛███▜▌", o)),
        Line::from(Span::styled("▝▜█████▛▘", o)),
        Line::from(Span::styled("  ▘▘ ▝▝", o)),
    ]
}

fn render_header(app: &App, f: &mut Frame, area: Rect) {
    let titles = [
        Section::Browse,
        Section::Memory,
        Section::Config,
        Section::Extensions,
    ]
    .iter()
    .map(|s| Line::from(format!(" {} ", s.title())))
    .collect::<Vec<_>>();
    let title = format!(" Claudine · {} ", app.active_label());
    let block = Block::default()
        .borders(Borders::ALL)
        .title(Span::styled(
            title,
            Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
        ))
        .title(Line::from(" h homes · / chercher · ? aide ").right_aligned());
    let inner = block.inner(area);
    f.render_widget(block, area);

    // Onglets sur la première ligne intérieure.
    let tabs = Tabs::new(titles)
        .select(app.section.index())
        .highlight_style(
            Style::default()
                .fg(Color::Black)
                .bg(ACCENT)
                .add_modifier(Modifier::BOLD),
        )
        .divider(Span::styled("│", Style::default().fg(DIM)));
    let tabs_area = Rect { height: 1, ..inner };
    f.render_widget(tabs, tabs_area);

    // Logo Claude aligné à droite, sur les 3 lignes intérieures (si assez large).
    const LOGO_W: u16 = 9;
    if inner.width > 55 && inner.height >= 3 {
        let logo_area = Rect {
            x: inner.x + inner.width - LOGO_W - 1,
            y: inner.y,
            width: LOGO_W,
            height: 3,
        };
        f.render_widget(Paragraph::new(claude_logo_lines()), logo_area);
    }
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
    // Largeur utile (bordures + symbole de surbrillance retranchés).
    let avail = (cols[0].width as usize).saturating_sub(4);
    // En vue agrégée, un en-tête par home regroupe ses projets (qui sont déjà
    // contigus par home) ; sinon, simple liste plate. `row_of_project[i]` donne
    // la ligne d'affichage du projet i (les en-têtes décalent les indices).
    let mut proj_items: Vec<ListItem> = Vec::new();
    let mut row_of_project: Vec<usize> = Vec::with_capacity(app.projects.len());
    let mut last_home: Option<String> = None;
    let mut group_collapsed = false;
    let mut header_row = 0usize;
    for (i, p) in app.projects.iter().enumerate() {
        if app.aggregate {
            let home = app.project_homes.get(i).cloned().unwrap_or_default();
            if last_home.as_deref() != Some(home.as_str()) {
                let count = app
                    .project_homes
                    .iter()
                    .filter(|h| h.as_str() == home.as_str())
                    .count();
                group_collapsed = app.is_home_collapsed(i);
                let marker = if group_collapsed { "▸" } else { "▾" };
                let state = if group_collapsed { " — replié" } else { "" };
                header_row = proj_items.len();
                proj_items.push(ListItem::new(Line::from(Span::styled(
                    format!("{marker} {home}  ({count} projets){state}"),
                    Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
                ))));
                last_home = Some(home);
            }
            // Groupe replié : les projets sont masqués ; ils pointent vers
            // l'en-tête (qui représente le groupe et reste sélectionnable).
            if group_collapsed {
                row_of_project.push(header_row);
                continue;
            }
        }
        row_of_project.push(proj_items.len());
        let raw = humanize_path(&p.cwd.clone().unwrap_or_else(|| p.encoded_name.clone()));
        let count = p.sessions.len();
        let suffix = format!("  ({count} sess.)");
        let indent_w = if app.aggregate { 2 } else { 0 };
        // Garde la fin du chemin (le nom du projet) en tronquant par la gauche.
        let budget = avail.saturating_sub(indent_w + suffix.chars().count());
        let shown = truncate_left(&raw, budget);
        let indent = if app.aggregate { "  " } else { "" };
        proj_items.push(ListItem::new(Line::from(vec![
            Span::raw(format!("{indent}{shown}")),
            Span::styled(suffix, Style::default().fg(DIM)),
        ])));
    }
    let proj_list = List::new(proj_items)
        .block(pane_block(" Projets ", projects_focused))
        .highlight_style(selection_style(projects_focused))
        .highlight_symbol("▶ ");
    let mut proj_state = ListState::default();
    proj_state.select(Some(
        row_of_project.get(app.project_idx).copied().unwrap_or(0),
    ));
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
            let name = humanize_path(&p.cwd.clone().unwrap_or_else(|| p.encoded_name.clone()));
            format!(" Sessions — {name} ")
        }
        None => " Sessions ".to_string(),
    };

    if sess_items.is_empty() {
        let block = pane_block(&title, sessions_focused);
        let p = Paragraph::new(Span::styled("  (aucune session)", Style::default().fg(DIM)))
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
            let name = humanize_path(&p.cwd.clone().unwrap_or_else(|| p.encoded_name.clone()));
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
    let block = Block::default().borders(Borders::ALL).title(Span::styled(
        format!(" {title} "),
        Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
    ));
    let inner = block.inner(area);
    f.render_widget(block, area);
    *viewport = inner.height as usize;

    let text: Vec<Line> = lines
        .iter()
        .map(|l| {
            if l.starts_with("── ") || l.starts_with("(") {
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

/// Section Extensions : hooks, plugins et serveurs MCP du home actif (lecture).
fn render_extensions(app: &mut App, f: &mut Frame, area: Rect) {
    let ext = &app.extensions;
    let title = if app.aggregate {
        format!(
            " Extensions · {} — hooks / plugins / MCP  (t: cible) ",
            app.home().label
        )
    } else {
        " Extensions — hooks / plugins / MCP ".to_string()
    };
    let block = Block::default()
        .borders(Borders::ALL)
        .title(Span::styled(
            title,
            Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
        ))
        .title(Line::from(" E éditer settings.json ").right_aligned());
    let inner = block.inner(area);
    f.render_widget(block, area);
    app.ext_viewport = inner.height as usize;

    let header = |label: String| {
        Line::from(Span::styled(
            label,
            Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
        ))
    };
    let none = || Line::from(Span::styled("  (aucun)", Style::default().fg(DIM)));

    let mut lines: Vec<Line> = Vec::new();

    // --- Hooks ---
    lines.push(header(format!("── Hooks ({}) ──", ext.hooks.len())));
    if ext.hooks.is_empty() {
        lines.push(none());
    }
    for h in &ext.hooks {
        let matcher = h
            .matcher
            .as_deref()
            .map(|m| format!("  [{m}]"))
            .unwrap_or_default();
        lines.push(Line::from(vec![
            Span::styled(
                format!("  {}", h.event),
                Style::default().add_modifier(Modifier::BOLD),
            ),
            Span::styled(matcher, Style::default().fg(Color::Cyan)),
        ]));
        for cmd in &h.commands {
            lines.push(Line::from(Span::styled(
                format!("      $ {cmd}"),
                Style::default().fg(DIM),
            )));
        }
    }
    lines.push(Line::from(""));

    // --- Plugins ---
    lines.push(header(format!("── Plugins ({}) ──", ext.plugins.len())));
    if ext.plugins.is_empty() {
        lines.push(none());
    }
    for p in &ext.plugins {
        let (mark, mark_style) = if p.enabled {
            ("✓", Style::default().fg(Color::Green))
        } else {
            ("✗", Style::default().fg(DIM))
        };
        let mut spans = vec![
            Span::styled(format!("  {mark} "), mark_style),
            Span::raw(p.name.clone()),
        ];
        if let Some(v) = &p.version {
            spans.push(Span::styled(format!("  v{v}"), Style::default().fg(DIM)));
        }
        if let Some(s) = &p.scope {
            spans.push(Span::styled(format!("  ({s})"), Style::default().fg(DIM)));
        }
        lines.push(Line::from(spans));
    }
    lines.push(Line::from(""));

    // --- MCP ---
    lines.push(header(format!("── Serveurs MCP ({}) ──", ext.mcp.len())));
    if ext.mcp.is_empty() {
        lines.push(none());
    }
    for m in &ext.mcp {
        lines.push(Line::from(vec![
            Span::styled(
                format!("  {}", m.name),
                Style::default().add_modifier(Modifier::BOLD),
            ),
            Span::styled(format!("  ⟨{}⟩", m.scope), Style::default().fg(Color::Cyan)),
        ]));
        lines.push(Line::from(Span::styled(
            format!("      {}", m.summary),
            Style::default().fg(DIM),
        )));
    }

    app.ext_total = lines.len();
    let para = Paragraph::new(Text::from(lines))
        .wrap(Wrap { trim: false })
        .scroll((app.ext_scroll as u16, 0));
    f.render_widget(para, inner);
}

/// Section Config : formulaire éditable, ou JSON brut (bascule `r`).
fn render_config(app: &mut App, f: &mut Frame, area: Rect) {
    if app.settings.raw() {
        let lines = app.settings.raw_lines();
        let title = if app.aggregate {
            format!(
                "Config · {} — settings.json (brut · r: formulaire)",
                app.home().label
            )
        } else {
            "Config — settings.json (brut · r: formulaire)".to_string()
        };
        render_scroll_pane(
            f,
            area,
            &title,
            &lines,
            app.config_scroll,
            &mut app.config_viewport,
        );
    } else {
        render_settings_form(app, f, area);
    }
}

/// Rend le formulaire de réglages : champs groupés par section, valeur courante,
/// champ surligné (avec saisie en ligne pour les champs scalaires).
fn render_settings_form(app: &mut App, f: &mut Frame, area: Rect) {
    let dirty = app.settings.dirty();
    let title = if app.aggregate {
        format!(" Config · {} — settings.json ", app.home().label)
    } else {
        " Config — settings.json ".to_string()
    };
    let right = if dirty {
        " ● modifié · s enregistrer · r JSON ".to_string()
    } else if app.aggregate {
        " t cible · s enregistrer · r JSON ".to_string()
    } else {
        " s enregistrer · r JSON brut ".to_string()
    };
    let block = Block::default()
        .borders(Borders::ALL)
        .title(Span::styled(
            title,
            Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
        ))
        .title(Line::from(right).right_aligned());
    let inner = block.inner(area);
    f.render_widget(block, area);
    let viewport = inner.height as usize;
    app.config_viewport = viewport;

    let sel = app.settings.idx();
    let scalar_buf = if app.settings.editing_scalar() {
        app.settings.scalar_buf().map(|s| s.to_string())
    } else {
        None
    };

    let mut lines: Vec<Line> = Vec::new();
    let mut sel_line = 0usize;
    let mut last_section: Option<String> = None;
    for (i, spec) in app.settings.fields().iter().enumerate() {
        if last_section.as_deref() != Some(spec.section.as_str()) {
            if !lines.is_empty() {
                lines.push(Line::from(""));
            }
            lines.push(Line::from(Span::styled(
                format!("── {} ──", spec.section),
                Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
            )));
            last_section = Some(spec.section.clone());
        }
        let selected = i == sel;
        if selected {
            sel_line = lines.len();
        }
        let value = match (selected, &scalar_buf) {
            (true, Some(buf)) => format!("{buf}▏"),
            _ => app.settings.value_display(spec),
        };
        let label_style = if selected {
            Style::default()
                .fg(Color::Black)
                .bg(ACCENT)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Gray)
        };
        let value_style = if selected {
            Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        };
        let mut spans = vec![
            Span::styled(format!("  {:<28} ", spec.label), label_style),
            Span::styled(value, value_style),
        ];
        if let Some(note) = &spec.note {
            spans.push(Span::styled(
                format!("  — {note}"),
                Style::default().fg(DIM),
            ));
        }
        lines.push(Line::from(spans));
    }

    // Auto-défilement pour garder le champ sélectionné visible.
    let scroll = (sel_line + 1).saturating_sub(viewport);
    let para = Paragraph::new(Text::from(lines)).scroll((scroll as u16, 0));
    f.render_widget(para, inner);
}

/// Popup d'édition d'une liste / map (StringList et KeyValue).
fn render_settings_list_editor(app: &App, f: &mut Frame, area: Rect) {
    let Some(le) = app.settings.list_state() else {
        return;
    };
    let spec = &app.settings.fields()[app.settings.idx()];
    let popup = centered_rect(70, 60, area);
    f.render_widget(Clear, popup);

    let editing_input = le.input.is_some();
    let block = Block::default()
        .borders(Borders::ALL)
        .title(Span::styled(
            format!(" {} ", spec.label),
            Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
        ))
        .title(
            Line::from(if editing_input {
                " Entrée valider · Esc annuler "
            } else {
                " a ajouter · Enter éditer · d retirer · Esc terminer "
            })
            .right_aligned(),
        );
    let inner = block.inner(popup);
    f.render_widget(block, popup);

    let (list_area, input_area) = if editing_input {
        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(1), Constraint::Length(2)])
            .split(inner);
        (rows[0], Some(rows[1]))
    } else {
        (inner, None)
    };

    let items: Vec<ListItem> = if le.items.is_empty() {
        vec![ListItem::new(Span::styled(
            "  (vide — « a » pour ajouter)",
            Style::default().fg(DIM),
        ))]
    } else {
        le.items
            .iter()
            .map(|it| ListItem::new(Line::from(it.clone())))
            .collect()
    };
    let list = List::new(items)
        .highlight_style(
            Style::default()
                .bg(ACCENT)
                .fg(Color::Black)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("▶ ");
    let mut state = ListState::default();
    if !le.items.is_empty() {
        state.select(Some(le.idx.min(le.items.len() - 1)));
    }
    f.render_stateful_widget(list, list_area, &mut state);

    if let (Some(input_area), Some(buf)) = (input_area, &le.input) {
        let line = Line::from(vec![
            Span::styled(
                "Valeur : ",
                Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
            ),
            Span::raw(buf.clone()),
            Span::styled("▏", Style::default().fg(ACCENT)),
        ]);
        f.render_widget(Paragraph::new(line), input_area);
    }
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
        None => Line::from(Span::styled(" prêt", Style::default().fg(DIM))),
    };
    f.render_widget(Paragraph::new(line), area);
}

fn render_footer(app: &App, f: &mut Frame, area: Rect) {
    // Corbeille : raccourcis prioritaires.
    if let Some(t) = &app.trash_view {
        let hints = if t.confirm.is_some() {
            key_hints(&[("o", "confirmer"), ("n/Esc", "annuler")])
        } else {
            key_hints(&[
                ("↑/↓", "session"),
                ("Enter/r", "restaurer"),
                ("d", "suppr. déf."),
                ("x", "vider"),
                ("Esc", "fermer"),
            ])
        };
        f.render_widget(Paragraph::new(Line::from(hints)), area);
        return;
    }

    // Recherche : raccourcis prioritaires.
    if app.search.is_some() {
        let hints = key_hints(&[
            ("saisir", "filtrer (chemin/id)"),
            ("Tab", "contenu"),
            ("↑/↓", "résultat"),
            ("Enter", "ouvrir"),
            ("Esc", "fermer"),
        ]);
        f.render_widget(Paragraph::new(Line::from(hints)), area);
        return;
    }

    // Modales de ménage : raccourcis prioritaires.
    if app.confirm_delete.is_some() {
        f.render_widget(
            Paragraph::new(Line::from(key_hints(&[
                ("o", "oui (corbeille)"),
                ("n / Esc", "annuler"),
            ]))),
            area,
        );
        return;
    }
    if app.move_targets.is_some() {
        f.render_widget(
            Paragraph::new(Line::from(key_hints(&[
                ("↑/↓", "cible"),
                ("Enter", "valider"),
                ("Esc", "annuler"),
            ]))),
            area,
        );
        return;
    }

    // Assistant d'import : raccourcis prioritaires.
    if let Some(im) = &app.import {
        let hints = if im.preview.is_some() {
            key_hints(&[
                ("Enter", "importer"),
                ("w", "écraser conflits"),
                ("Esc", "annuler"),
            ])
        } else {
            key_hints(&[
                ("saisir", "chemin .tar.gz"),
                ("Enter", "aperçu"),
                ("Esc", "annuler"),
            ])
        };
        f.render_widget(Paragraph::new(Line::from(hints)), area);
        return;
    }

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
            ("h", "homes"),
            ("?", "aide"),
            ("q", "quitter"),
        ]),
        Section::Browse if app.aggregate => key_hints(&[
            ("←/→", "panneau"),
            ("↑/↓", "naviguer"),
            ("Enter", "ouvrir"),
            ("Espace/z", "replier"),
            ("d", "suppr."),
            ("m", "déplacer"),
            ("/", "chercher"),
            ("c", "corbeille"),
            ("h", "homes"),
            ("?", "aide"),
        ]),
        Section::Browse => key_hints(&[
            ("Tab/1·2·3", "sections"),
            ("←/→", "panneau"),
            ("↑/↓", "naviguer"),
            ("Enter", "ouvrir"),
            ("d", "suppr."),
            ("m", "déplacer"),
            ("/", "chercher"),
            ("c", "corbeille"),
            ("h", "homes"),
            ("?", "aide"),
            ("q", "quitter"),
        ]),
        Section::Config if app.settings.is_editing() => key_hints(&[
            ("↑/↓", "naviguer"),
            ("a", "ajouter"),
            ("Enter", "éditer"),
            ("d", "retirer"),
            ("Esc", "terminer"),
        ]),
        Section::Config if app.settings.raw() => key_hints(&[
            ("Tab/1·2·3", "sections"),
            ("↑/↓ PgUp/Dn", "défiler"),
            ("r", "formulaire"),
            ("h", "homes"),
            ("?", "aide"),
            ("q", "quitter"),
        ]),
        Section::Config => key_hints(&[
            ("↑/↓", "champ"),
            ("Enter", "éditer"),
            ("←/→", "option"),
            ("s", "enregistrer"),
            ("r", "JSON brut"),
            ("?", "aide"),
            ("q", "quitter"),
        ]),
        Section::Memory if app.aggregate => key_hints(&[
            ("Tab/1·2·3·4", "sections"),
            ("↑/↓", "défiler"),
            ("t", "cible"),
            ("E", "éditer"),
            ("h", "homes"),
            ("?", "aide"),
        ]),
        Section::Extensions => key_hints(&[
            ("Enter", "hooks"),
            ("p", "plugins"),
            ("m", "MCP"),
            ("g", "marketplaces"),
            ("↑/↓", "défiler"),
            ("t", "cible"),
            ("E", "settings"),
            ("?", "aide"),
        ]),
        _ => key_hints(&[
            ("Tab/1·2·3·4", "sections"),
            ("↑/↓ PgUp/Dn", "défiler"),
            ("E", "éditer"),
            ("e", "export"),
            ("h", "homes"),
            ("?", "aide"),
            ("q", "quitter"),
        ]),
    };
    f.render_widget(Paragraph::new(Line::from(hints)), area);
}

fn render_help(f: &mut Frame, area: Rect) {
    let popup = centered_rect(64, 70, area);
    f.render_widget(Clear, popup);
    let block = Block::default().borders(Borders::ALL).title(Span::styled(
        " Aide — raccourcis ",
        Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
    ));
    let rows = [
        ("1 / 2 / 3 / 4", "Projets / Mémoire / Config / Extensions"),
        ("Tab", "section suivante"),
        (
            "Extensions",
            "hooks (Enter) · plugins (p) · MCP (m) · marketplaces (g → Enter: catalogue, i installe) ; E édite settings.json",
        ),
        ("← →", "changer de panneau (Browse)"),
        ("↑ ↓ / j k", "naviguer / défiler"),
        ("Enter", "ouvrir la session sélectionnée"),
        ("Espace", "replier / déplier le home courant (agrégé)"),
        ("z", "tout replier / tout déplier (agrégé)"),
        ("/", "rechercher (live chemin/id · Tab = contenu)"),
        (
            "d / Suppr",
            "→ corbeille : session (panneau Sessions) ou projet (panneau Projets)",
        ),
        ("m", "déplacer la session vers un autre projet"),
        ("c", "corbeille : restaurer / supprimer déf. / vider"),
        ("Esc", "retour (transcript) sinon quitter"),
        ("PgUp / PgDn", "défilement par page"),
        ("Home / End", "aller au début / à la fin"),
        ("e", "exporter ~/.claude en .tar.gz"),
        ("i", "importer un bundle .tar.gz (aperçu puis application)"),
        (
            "E",
            "éditer le fichier de la section dans $EDITOR (Mémoire/Config)",
        ),
        (
            "Config",
            "↑↓ champ · Enter éditer · ←→ option · s enregistrer · r JSON",
        ),
        ("h", "homes : ★ Tous les homes (agrégé) / un home précis"),
        ("t", "en agrégé : changer le home cible de Mémoire/Config"),
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

    let mut items: Vec<ListItem> = Vec::new();
    // Entrée agrégée « Tous les homes » en tête (index 0).
    {
        let mark = if app.aggregate { "● " } else { "  " };
        items.push(ListItem::new(Line::from(vec![
            Span::styled(
                mark,
                Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                "★ Tous les homes",
                Style::default().add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("  ({} homes)", app.homes.len()),
                Style::default().fg(DIM),
            ),
        ])));
    }
    for (i, h) in app.homes.iter().enumerate() {
        let n = scan_projects(h).map(|p| p.len()).unwrap_or(0);
        let mark = if !app.aggregate && i == app.active {
            "● "
        } else {
            "  "
        };
        items.push(ListItem::new(Line::from(vec![
            Span::styled(
                mark,
                Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
            ),
            Span::raw(h.label.clone()),
            Span::styled(format!("  ({n} projets)"), Style::default().fg(DIM)),
        ])));
    }

    let list = List::new(items)
        .highlight_style(
            Style::default()
                .bg(ACCENT)
                .fg(Color::Black)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("▶ ");
    let mut state = ListState::default();
    state.select(Some(app.picker_idx.min(app.homes.len())));
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

/// Popup de confirmation de suppression (mise en corbeille) d'une session.
fn render_confirm_delete(app: &App, f: &mut Frame, area: Rect) {
    let kind = app.confirm_delete.unwrap_or(DeleteKind::Session);
    let (title, target_line, prompt) = match kind {
        DeleteKind::Session => {
            let id = app
                .selected_session()
                .map(|s| s.id.chars().take(8).collect::<String>())
                .unwrap_or_default();
            (
                " Supprimer la session ",
                format!("  Session {id}"),
                "  La déplacer vers la corbeille du home ?".to_string(),
            )
        }
        DeleteKind::Project => {
            let (name, n) = app
                .selected_project()
                .map(|p| {
                    (
                        humanize_path(p.cwd.as_deref().unwrap_or(&p.encoded_name)),
                        p.sessions.len(),
                    )
                })
                .unwrap_or_default();
            // Garde la fin du chemin (le nom du projet) si trop long.
            let name = truncate_left(&name, 52);
            (
                " Supprimer le projet ",
                format!("  Projet {name}  ({n} sess.)"),
                "  Déplacer tout le projet vers la corbeille ?".to_string(),
            )
        }
    };
    let hint = "  (récupérable dans <home>/trash/…)";
    let buttons = "  [o] oui    [n] non";

    // Dimensionne la modale à son contenu (pas un grand cadre figé).
    let content_w = [
        title.chars().count(),
        target_line.chars().count(),
        prompt.chars().count(),
        hint.chars().count(),
        buttons.chars().count(),
    ]
    .into_iter()
    .max()
    .unwrap_or(0);
    let width = (content_w as u16 + 4).clamp(28, area.width.saturating_sub(2).max(28));
    let popup = centered_rect_fixed(width, 9, area);
    f.render_widget(Clear, popup);
    let block = Block::default().borders(Borders::ALL).title(Span::styled(
        title,
        Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
    ));
    let inner = block.inner(popup);
    f.render_widget(block, popup);
    let lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            target_line,
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(prompt),
        Line::from(Span::styled(hint, Style::default().fg(DIM))),
        Line::from(""),
        Line::from(vec![
            Span::styled(
                "  [o]",
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" oui    "),
            Span::styled(
                "[n]",
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            ),
            Span::raw(" non"),
        ]),
    ];
    f.render_widget(Paragraph::new(Text::from(lines)), inner);
}

/// Popup de sélection de la cible de déplacement d'une session.
fn render_move_picker(app: &App, f: &mut Frame, area: Rect) {
    let Some(targets) = &app.move_targets else {
        return;
    };
    let id = app
        .selected_session()
        .map(|s| s.id.chars().take(8).collect::<String>())
        .unwrap_or_default();
    let popup = centered_rect(74, 60, area);
    f.render_widget(Clear, popup);
    let block = Block::default()
        .borders(Borders::ALL)
        .title(Span::styled(
            format!(" Déplacer la session {id} vers… "),
            Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
        ))
        .title(Line::from(" Enter valider · Esc annuler ").right_aligned());
    let inner = block.inner(popup);
    f.render_widget(block, popup);
    let items: Vec<ListItem> = targets
        .iter()
        .map(|t| ListItem::new(Line::from(t.label.clone())))
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
    if !targets.is_empty() {
        state.select(Some(app.move_idx.min(targets.len() - 1)));
    }
    f.render_stateful_widget(list, inner, &mut state);
}

/// Overlay de recherche : ligne de saisie puis liste des résultats (label + extrait).
fn render_search(app: &App, f: &mut Frame, area: Rect) {
    let Some(s) = &app.search else {
        return;
    };
    let popup = centered_rect(82, 72, area);
    f.render_widget(Clear, popup);
    let block = Block::default()
        .borders(Borders::ALL)
        .title(Span::styled(
            " Rechercher une session ",
            Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
        ))
        .title(Line::from(" Tab contenu · Enter ouvrir · Esc fermer ").right_aligned());
    let inner = block.inner(popup);
    f.render_widget(block, popup);

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(2), Constraint::Min(1)])
        .split(inner);

    let count = if s.query.trim().is_empty() {
        String::new()
    } else {
        let kind = if s.deep { "contenu" } else { "chemin/id" };
        format!("   {} résultat(s) · {kind}", s.results.len())
    };
    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(
                "Recherche : ",
                Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
            ),
            Span::raw(s.query.clone()),
            Span::styled("▏", Style::default().fg(ACCENT)),
            Span::styled(count, Style::default().fg(DIM)),
        ])),
        rows[0],
    );

    if s.query.trim().is_empty() {
        f.render_widget(
            Paragraph::new(Span::styled(
                "  Tapez pour filtrer par chemin / id (en direct). Tab : chercher dans le contenu.",
                Style::default().fg(DIM),
            )),
            rows[1],
        );
        return;
    }
    if s.results.is_empty() {
        f.render_widget(
            Paragraph::new(Span::styled(
                "  Aucun résultat. Tab pour chercher dans le contenu des sessions.",
                Style::default().fg(DIM),
            )),
            rows[1],
        );
        return;
    }
    let items: Vec<ListItem> = s
        .results
        .iter()
        .map(|h| {
            ListItem::new(vec![
                Line::from(Span::styled(
                    h.label.clone(),
                    Style::default().add_modifier(Modifier::BOLD),
                )),
                Line::from(Span::styled(
                    format!("    {}", h.snippet),
                    Style::default().fg(DIM),
                )),
            ])
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
    state.select(Some(s.idx.min(s.results.len() - 1)));
    f.render_stateful_widget(list, rows[1], &mut state);
}

/// Overlay de la corbeille : liste des sessions supprimées (restaurables).
fn render_trash(app: &App, f: &mut Frame, area: Rect) {
    let Some(t) = &app.trash_view else {
        return;
    };
    let popup = centered_rect(80, 60, area);
    f.render_widget(Clear, popup);
    let block = Block::default()
        .borders(Borders::ALL)
        .title(Span::styled(
            " Corbeille — sessions supprimées ",
            Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
        ))
        .title(
            Line::from(" Enter/r restaurer · d suppr. déf. · x vider · Esc fermer ")
                .right_aligned(),
        );
    let inner = block.inner(popup);
    f.render_widget(block, popup);

    // Réserve une ligne en bas pour la confirmation de purge si elle est active.
    let (list_area, confirm_area) = if t.confirm.is_some() {
        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(1), Constraint::Length(1)])
            .split(inner);
        (rows[0], Some(rows[1]))
    } else {
        (inner, None)
    };

    let items: Vec<ListItem> = t
        .items
        .iter()
        .map(|e| ListItem::new(Line::from(e.label.clone())))
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
    if !t.items.is_empty() {
        state.select(Some(t.idx.min(t.items.len() - 1)));
    }
    f.render_stateful_widget(list, list_area, &mut state);

    if let (Some(area), Some(scope)) = (confirm_area, t.confirm) {
        let msg = match scope {
            PurgeScope::One => "Supprimer DÉFINITIVEMENT cette session ? (o/n)".to_string(),
            PurgeScope::All => format!(
                "VIDER toute la corbeille ({} session(s)) définitivement ? (o/n)",
                t.items.len()
            ),
        };
        let warn = Paragraph::new(Line::from(Span::styled(
            msg,
            Style::default()
                .fg(Color::Black)
                .bg(Color::Red)
                .add_modifier(Modifier::BOLD),
        )));
        f.render_widget(warn, area);
    }
}

/// Overlay de l'assistant d'import : saisie du chemin puis aperçu/confirmation.
fn render_import(app: &App, f: &mut Frame, area: Rect) {
    let Some(im) = &app.import else {
        return;
    };
    let popup = centered_rect(78, 52, area);
    f.render_widget(Clear, popup);
    let right = if im.preview.is_some() {
        " Enter importer · w écraser · Esc annuler "
    } else {
        " Enter aperçu · Esc annuler "
    };
    let block = Block::default()
        .borders(Borders::ALL)
        .title(Span::styled(
            " Importer un bundle ",
            Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
        ))
        .title(Line::from(right).right_aligned());
    let inner = block.inner(popup);
    f.render_widget(block, popup);

    let mut lines: Vec<Line> = Vec::new();
    lines.push(Line::from(vec![
        Span::styled(
            "Bundle : ",
            Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
        ),
        Span::raw(im.input.clone()),
        Span::styled(
            if im.preview.is_none() { "▏" } else { "" },
            Style::default().fg(ACCENT),
        ),
    ]));
    lines.push(Line::from(Span::styled(
        format!("Cible : {}", app.active_label()),
        Style::default().fg(DIM),
    )));
    lines.push(Line::from(""));

    match &im.preview {
        None => {
            lines.push(Line::from(Span::styled(
                "  Chemin d'un .tar.gz exporté par Claudine, puis Entrée pour l'aperçu.",
                Style::default().fg(DIM),
            )));
        }
        Some((_, p)) => {
            lines.push(Line::from(Span::styled(
                "Aperçu :",
                Style::default().add_modifier(Modifier::BOLD),
            )));
            lines.push(Line::from(format!("  • {} projet(s)", p.projects)));
            lines.push(Line::from(vec![Span::styled(
                format!("  • {} session(s) nouvelle(s)", p.sessions_new),
                Style::default().fg(Color::Green),
            )]));
            let conflict_style = if p.sessions_conflict > 0 {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default().fg(DIM)
            };
            let action = if im.overwrite {
                "écrasées"
            } else {
                "ignorées"
            };
            lines.push(Line::from(Span::styled(
                format!("  • {} en conflit → {action}", p.sessions_conflict),
                conflict_style,
            )));
            lines.push(Line::from(""));
            lines.push(Line::from(vec![
                Span::raw("  Écraser les conflits (w) : "),
                Span::styled(
                    if im.overwrite { "OUI" } else { "non" },
                    Style::default()
                        .fg(if im.overwrite { Color::Yellow } else { DIM })
                        .add_modifier(Modifier::BOLD),
                ),
            ]));
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                "  Entrée pour importer (sauvegarde auto avant écriture).",
                Style::default().fg(DIM),
            )));
        }
    }
    f.render_widget(Paragraph::new(lines), inner);
}

/// Modal de l'éditeur de hooks.
fn render_hooks_editor(app: &App, f: &mut Frame, area: Rect) {
    let Some(e) = &app.hooks_editor else {
        return;
    };
    let popup = centered_rect(80, 70, area);
    f.render_widget(Clear, popup);
    let hint = match e.level {
        HooksLevel::Groups => " a ajouter · Enter ouvrir · d suppr. · s enregistrer · Esc fermer ",
        HooksLevel::Group => {
            " a commande · Enter éditer · t timeout · d suppr. · s enregistrer · Esc retour "
        }
    };
    let block = Block::default()
        .borders(Borders::ALL)
        .title(Span::styled(
            " Éditeur de hooks ",
            Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
        ))
        .title(Line::from(hint).right_aligned());
    let inner = block.inner(popup);
    f.render_widget(block, popup);

    let mut lines: Vec<Line> = Vec::new();
    match e.level {
        HooksLevel::Groups => {
            if e.groups.is_empty() {
                lines.push(Line::from(Span::styled(
                    "  (aucun hook — 'a' pour en ajouter)",
                    Style::default().fg(DIM),
                )));
            }
            for (i, g) in e.groups.iter().enumerate() {
                let sel = i == e.group_idx;
                let matcher = g
                    .matcher
                    .as_deref()
                    .map(|m| format!(" [{m}]"))
                    .unwrap_or_default();
                let txt = format!(
                    "{} {}{}  · {} cmd",
                    if sel { "▶" } else { " " },
                    g.event,
                    matcher,
                    g.commands.len()
                );
                let style = if sel {
                    selection_style(true)
                } else {
                    Style::default()
                };
                lines.push(Line::from(Span::styled(txt, style)));
            }
        }
        HooksLevel::Group => {
            let g = match e.groups.get(e.group_idx) {
                Some(g) => g,
                None => return,
            };
            let row = |sel: bool, label: String| {
                let style = if sel {
                    selection_style(true)
                } else {
                    Style::default()
                };
                Line::from(Span::styled(label, style))
            };
            lines.push(row(e.field_idx == 0, format!("  Évènement : {}", g.event)));
            lines.push(row(
                e.field_idx == 1,
                format!(
                    "  Matcher   : {}",
                    g.matcher.as_deref().unwrap_or("(aucun)")
                ),
            ));
            for (ci, c) in g.commands.iter().enumerate() {
                let to = c
                    .timeout
                    .map(|t| format!("  (timeout {t}s)"))
                    .unwrap_or_default();
                lines.push(row(
                    e.field_idx == ci + 2,
                    format!("    $ {}{}", c.command, to),
                ));
            }
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                format!("  évènements connus : {}", KNOWN_EVENTS.join(", ")),
                Style::default().fg(DIM),
            )));
        }
    }

    // Bandeau de saisie ou de confirmation.
    if let HookEdit::Text(buf) = &e.edit {
        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::styled(
                "  Saisie : ",
                Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
            ),
            Span::raw(buf.clone()),
            Span::styled("▏", Style::default().fg(ACCENT)),
        ]));
    } else if e.confirm_delete {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "  Supprimer l'élément sélectionné ? (o/n)",
            Style::default()
                .fg(Color::Black)
                .bg(Color::Red)
                .add_modifier(Modifier::BOLD),
        )));
    }

    f.render_widget(Paragraph::new(Text::from(lines)), inner);
}

/// Modal de bascule activer/désactiver des plugins.
fn render_plugins_toggle(app: &App, f: &mut Frame, area: Rect) {
    let Some(pt) = &app.plugins_toggle else {
        return;
    };
    let popup = centered_rect(70, 60, area);
    f.render_widget(Clear, popup);
    let block = Block::default()
        .borders(Borders::ALL)
        .title(Span::styled(
            " Plugins — activer / désactiver ",
            Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
        ))
        .title(Line::from(" Espace bascule · s enregistrer · Esc fermer ").right_aligned());
    let inner = block.inner(popup);
    f.render_widget(block, popup);
    let items: Vec<ListItem> = pt
        .items
        .iter()
        .map(|it| {
            let (mark, mstyle) = if it.enabled {
                ("✓", Style::default().fg(Color::Green))
            } else {
                ("✗", Style::default().fg(DIM))
            };
            ListItem::new(Line::from(vec![
                Span::styled(format!(" {mark} "), mstyle),
                Span::raw(it.name.clone()),
            ]))
        })
        .collect();
    let list = List::new(items)
        .highlight_style(selection_style(true))
        .highlight_symbol("▶ ");
    let mut state = ListState::default();
    if !pt.items.is_empty() {
        state.select(Some(pt.idx.min(pt.items.len() - 1)));
    }
    f.render_stateful_widget(list, inner, &mut state);
}

/// Modal de l'éditeur de serveurs MCP.
fn render_mcp_editor(app: &App, f: &mut Frame, area: Rect) {
    let Some(e) = &app.mcp_editor else {
        return;
    };
    let popup = centered_rect(82, 72, area);
    f.render_widget(Clear, popup);
    let hint = match e.level {
        McpLevel::Servers => " a ajouter · Enter ouvrir · d suppr. · s enregistrer · Esc fermer ",
        McpLevel::Server => {
            " ←/→ type · Enter éditer · a ajouter · d suppr. · s enregistrer · Esc retour "
        }
    };
    let block = Block::default()
        .borders(Borders::ALL)
        .title(Span::styled(
            " Éditeur de serveurs MCP ",
            Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
        ))
        .title(Line::from(hint).right_aligned());
    let inner = block.inner(popup);
    f.render_widget(block, popup);

    let mut lines: Vec<Line> = Vec::new();
    match e.level {
        McpLevel::Servers => {
            if e.servers.is_empty() {
                lines.push(Line::from(Span::styled(
                    "  (aucun serveur — 'a' pour en ajouter)",
                    Style::default().fg(DIM),
                )));
            }
            for (i, s) in e.servers.iter().enumerate() {
                let sel = i == e.server_idx;
                let t = match s.transport {
                    McpTransport::Stdio => "stdio",
                    McpTransport::Http => "http",
                    McpTransport::Sse => "sse",
                };
                let label = format!("{} {}  [{}]", if sel { "▶" } else { " " }, s.name, t);
                let style = if sel {
                    selection_style(true)
                } else {
                    Style::default()
                };
                lines.push(Line::from(Span::styled(label, style)));
            }
        }
        McpLevel::Server => {
            let Some(s) = e.servers.get(e.server_idx) else {
                return;
            };
            let rows = e.rows();
            for (i, row) in rows.iter().enumerate() {
                let sel = i == e.field_idx;
                let text = match *row {
                    McpRow::Name => format!("  Nom      : {}", s.name),
                    McpRow::Type => {
                        let t = match s.transport {
                            McpTransport::Stdio => "stdio",
                            McpTransport::Http => "http",
                            McpTransport::Sse => "sse",
                        };
                        format!("  Type     : {t}   (←/→)")
                    }
                    McpRow::Command => format!("  Command  : {}", s.command),
                    McpRow::Url => format!("  URL      : {}", s.url),
                    McpRow::Arg(i) => format!(
                        "    arg[{i}] : {}",
                        s.args.get(i).cloned().unwrap_or_default()
                    ),
                    McpRow::Env(i) => {
                        let (k, v) = s.env.get(i).cloned().unwrap_or_default();
                        format!("    env     : {k}={v}")
                    }
                    McpRow::Header(i) => {
                        let (k, v) = s.headers.get(i).cloned().unwrap_or_default();
                        format!("    header  : {k}={v}")
                    }
                };
                let style = if sel {
                    selection_style(true)
                } else {
                    Style::default()
                };
                lines.push(Line::from(Span::styled(text, style)));
            }
        }
    }

    if let McpEdit::Text(buf) = &e.edit {
        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::styled(
                "  Saisie : ",
                Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
            ),
            Span::raw(buf.clone()),
            Span::styled("▏", Style::default().fg(ACCENT)),
        ]));
    } else if e.confirm_delete {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "  Supprimer l'élément sélectionné ? (o/n)",
            Style::default()
                .fg(Color::Black)
                .bg(Color::Red)
                .add_modifier(Modifier::BOLD),
        )));
    }

    f.render_widget(Paragraph::new(Text::from(lines)), inner);
}

/// Modal du gestionnaire de marketplaces (liste, saisie d'ajout, confirmation, indicateur de job).
fn render_marketplaces(app: &App, f: &mut Frame, area: Rect) {
    let Some(m) = &app.marketplaces else {
        return;
    };
    if let Some(c) = &m.catalog {
        render_plugin_catalog(c, app.mkt_job.as_ref(), f, area);
        return;
    }
    let popup = centered_rect(78, 68, area);
    f.render_widget(Clear, popup);

    let busy = app.mkt_job.is_some();
    let hint = if m.mode == MktMode::AddInput {
        " Enter valider · Esc annuler "
    } else if busy {
        " (opération en cours…) · Esc fermer "
    } else {
        " a ajouter · u màj · d retirer · Esc fermer "
    };
    let block = Block::default()
        .borders(Borders::ALL)
        .title(Span::styled(
            " Marketplaces de plugins ",
            Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
        ))
        .title(Line::from(hint).right_aligned());
    let inner = block.inner(popup);
    f.render_widget(block, popup);

    let mut lines: Vec<Line> = Vec::new();
    if m.items.is_empty() {
        lines.push(Line::from(Span::styled(
            "  (aucune marketplace — 'a' pour en ajouter)",
            Style::default().fg(DIM),
        )));
    }
    for (i, mk) in m.items.iter().enumerate() {
        let sel = i == m.idx;
        let src = match &mk.source {
            MarketplaceSource::Github { repo } => format!("github:{repo}"),
            MarketplaceSource::Git { url } => url.clone(),
            MarketplaceSource::Local { path } => format!("local:{}", path.display()),
        };
        let date = mk.last_updated.split('T').next().unwrap_or("");
        let label = format!(
            "{} {}  ·  {}  ·  {}",
            if sel { "▶" } else { " " },
            mk.name,
            src,
            date
        );
        let style = if sel {
            selection_style(true)
        } else {
            Style::default()
        };
        lines.push(Line::from(Span::styled(label, style)));
    }

    if m.mode == MktMode::AddInput {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "  Source (owner/repo · URL git · chemin local) :",
            Style::default().fg(ACCENT),
        )));
        lines.push(Line::from(vec![
            Span::styled(
                "  > ",
                Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
            ),
            Span::raw(m.input.clone()),
            Span::styled("▏", Style::default().fg(ACCENT)),
        ]));
    } else if m.confirm_remove {
        lines.push(Line::from(""));
        let name = m.selected_name().unwrap_or_default();
        lines.push(Line::from(Span::styled(
            format!("  Retirer « {name} » et son dossier ? (o/n)"),
            Style::default()
                .fg(Color::Black)
                .bg(Color::Red)
                .add_modifier(Modifier::BOLD),
        )));
    } else if busy {
        if let Some(job) = &app.mkt_job {
            const SPINNER: [char; 10] = ['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];
            let s = SPINNER[(job.frame as usize) % SPINNER.len()];
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                format!("  {s} {}…", job.label),
                Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
            )));
        }
    }

    f.render_widget(Paragraph::new(Text::from(lines)), inner);
}

/// Modal du catalogue de plugins d'une marketplace (2ᵉ niveau).
fn render_plugin_catalog(c: &PluginCatalog, job: Option<&MktJob>, f: &mut Frame, area: Rect) {
    let popup = centered_rect(78, 72, area);
    f.render_widget(Clear, popup);

    let hint = if c.confirm_uninstall {
        " o/n confirmer "
    } else if job.is_some() {
        " (installation en cours…) · Esc retour "
    } else {
        " Espace activer/désact. · i installer · d désinstaller · Esc retour "
    };
    let block = Block::default()
        .borders(Borders::ALL)
        .title(Span::styled(
            format!(" Plugins de « {} » ", c.marketplace),
            Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
        ))
        .title(Line::from(hint).right_aligned());
    let inner = block.inner(popup);
    f.render_widget(block, popup);

    let mut lines: Vec<Line> = Vec::new();
    if c.entries.is_empty() {
        lines.push(Line::from(Span::styled(
            "  (aucun plugin dans ce manifeste)",
            Style::default().fg(DIM),
        )));
    }
    for (i, e) in c.entries.iter().enumerate() {
        let sel = i == c.idx;
        let state = if e.installed {
            if e.enabled {
                "[installé][activé]"
            } else {
                "[installé]"
            }
        } else {
            "(non installé)"
        };
        let label = format!("{} {}  {}", if sel { "▶" } else { " " }, e.name, state);
        let style = if sel {
            selection_style(true)
        } else {
            Style::default()
        };
        lines.push(Line::from(Span::styled(label, style)));
        if sel {
            if let Some(d) = &e.description {
                lines.push(Line::from(Span::styled(
                    format!("     {d}"),
                    Style::default().fg(DIM),
                )));
            }
        }
    }

    if c.confirm_uninstall {
        lines.push(Line::from(""));
        let name = c.selected_name().unwrap_or_default();
        lines.push(Line::from(Span::styled(
            format!("  Désinstaller « {name} » ? (o/n)"),
            Style::default()
                .fg(Color::Black)
                .bg(Color::Red)
                .add_modifier(Modifier::BOLD),
        )));
    }

    if let Some(job) = job {
        const SPINNER: [char; 10] = ['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];
        let s = SPINNER[(job.frame as usize) % SPINNER.len()];
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            format!("  {s} {}…", job.label),
            Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
        )));
    }

    f.render_widget(Paragraph::new(Text::from(lines)), inner);
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

/// Tronque par la **gauche** (garde la fin, la plus distinctive) avec un `…`.
fn truncate_left(s: &str, max: usize) -> String {
    let count = s.chars().count();
    if count <= max {
        return s.to_string();
    }
    if max <= 1 {
        return "…".to_string();
    }
    let tail: String = s.chars().skip(count - (max - 1)).collect();
    format!("…{tail}")
}

/// Rectangle centré de taille fixe (en cellules), borné à `area`.
fn centered_rect_fixed(width: u16, height: u16, area: Rect) -> Rect {
    let w = width.min(area.width);
    let h = height.min(area.height);
    let x = area.x + (area.width.saturating_sub(w)) / 2;
    let y = area.y + (area.height.saturating_sub(h)) / 2;
    Rect {
        x,
        y,
        width: w,
        height: h,
    }
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
    use ratatui::{Terminal, backend::TestBackend};
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
    fn claude_logo_is_exact_glyph() {
        let logo = claude_logo_lines();
        assert_eq!(logo.len(), 3);
        let text: Vec<String> = logo
            .iter()
            .map(|l| {
                l.spans
                    .iter()
                    .map(|s| s.content.as_ref())
                    .collect::<String>()
            })
            .collect();
        // Glyphe officiel de la boîte « What's new » de Claude Code.
        assert_eq!(text, vec![" ▐▛███▜▌", "▝▜█████▛▘", "  ▘▘ ▝▝"]);
        // Tient dans la zone réservée de l'en-tête (LOGO_W = 9).
        assert!(text.iter().all(|l| l.chars().count() <= 9));
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
        assert!(
            terminal
                .backend()
                .buffer()
                .content()
                .iter()
                .any(|c| c.symbol() != " ")
        );
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
        assert!(
            terminal
                .backend()
                .buffer()
                .content()
                .iter()
                .any(|c| c.symbol() != " ")
        );

        // Popup du sélecteur (mode liste).
        app.open_picker();
        terminal.draw(|f| render(&mut app, f)).unwrap();
        assert!(app.show_picker);
        assert!(
            terminal
                .backend()
                .buffer()
                .content()
                .iter()
                .any(|c| c.symbol() != " ")
        );

        // Popup en mode saisie de chemin (ajout de home).
        app.picker_start_add();
        app.picker_input_char('/');
        app.picker_input_char('x');
        terminal.draw(|f| render(&mut app, f)).unwrap();
        assert!(matches!(app.picker_mode, PickerMode::AddInput(_)));
    }
}
