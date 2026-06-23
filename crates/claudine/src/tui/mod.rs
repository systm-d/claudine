//! Interface TUI Claudine : configuration du terminal, boucle d'évènements.

pub mod app;
pub mod settings_form;
pub mod ui;

use std::io::{self, Stdout};
use std::panic;
use std::path::Path;
use std::process::Command;

use ratatui::crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};

use claudine_core::discover_homes;

use app::{App, PickerMode, Section};

type Tui = Terminal<CrosstermBackend<Stdout>>;

/// Point d'entrée public : découvre la home, prépare le terminal, lance la
/// boucle et restaure le terminal quoi qu'il arrive.
pub fn run() -> io::Result<()> {
    let app = App::with_homes(discover_homes());
    run_app(app)
}

/// Prépare le terminal, exécute la boucle puis restaure systématiquement.
fn run_app(app: App) -> io::Result<()> {
    install_panic_hook();
    let mut terminal = setup_terminal()?;
    let result = event_loop(&mut terminal, app);
    // Restauration garantie, même si la boucle a échoué.
    let restore = restore_terminal(&mut terminal);
    result.and(restore)
}

fn setup_terminal() -> io::Result<Tui> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    Terminal::new(backend)
}

fn restore_terminal(terminal: &mut Tui) -> io::Result<()> {
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;
    Ok(())
}

/// Restauration de bas niveau utilisée par le hook de panique (sans `Terminal`).
fn restore_terminal_raw() {
    let _ = disable_raw_mode();
    let _ = execute!(io::stdout(), LeaveAlternateScreen, DisableMouseCapture);
}

/// Installe un hook qui restaure le terminal avant de déléguer au handler par
/// défaut, pour qu'une panique ne laisse jamais le terminal cassé.
fn install_panic_hook() {
    let default = panic::take_hook();
    panic::set_hook(Box::new(move |info| {
        restore_terminal_raw();
        default(info);
    }));
}

/// Boucle principale : rendu puis traitement des évènements clavier.
fn event_loop(terminal: &mut Tui, mut app: App) -> io::Result<()> {
    while !app.should_quit {
        terminal.draw(|f| ui::render(&mut app, f))?;
        match event::read()? {
            Event::Key(key) if key.kind == KeyEventKind::Press => handle_key(&mut app, key),
            Event::Resize(_, _) => {}
            _ => {}
        }
        // Édition externe demandée : suspend le TUI, lance l'éditeur, recharge.
        if let Some(path) = app.pending_edit.take() {
            edit_in_external_editor(terminal, &path)?;
            app.after_external_edit(&path);
        }
    }
    Ok(())
}

/// Suspend le TUI, ouvre `path` dans `$VISUAL`/`$EDITOR` (défaut `vi`), puis
/// restaure le terminal.
fn edit_in_external_editor(terminal: &mut Tui, path: &Path) -> io::Result<()> {
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen, DisableMouseCapture)?;

    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let editor = std::env::var("VISUAL")
        .or_else(|_| std::env::var("EDITOR"))
        .unwrap_or_else(|_| "vi".to_string());
    let mut parts = editor.split_whitespace();
    if let Some(prog) = parts.next() {
        let args: Vec<&str> = parts.collect();
        let _ = Command::new(prog).args(&args).arg(path).status();
    }

    enable_raw_mode()?;
    execute!(terminal.backend_mut(), EnterAlternateScreen, EnableMouseCapture)?;
    terminal.clear()?;
    Ok(())
}

/// Traduit une touche en action sur l'`App`.
fn handle_key(app: &mut App, key: KeyEvent) {
    // Ctrl-C quitte toujours.
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
        app.quit();
        return;
    }

    // Le sélecteur de home capture les touches tant qu'il est ouvert.
    if app.show_picker {
        handle_picker_key(app, key);
        return;
    }

    // Confirmation de suppression.
    if app.confirm_delete {
        match key.code {
            KeyCode::Char('o') | KeyCode::Char('O') | KeyCode::Char('y') | KeyCode::Char('Y')
            | KeyCode::Enter => app.confirm_delete_apply(),
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => app.confirm_delete_cancel(),
            _ => {}
        }
        return;
    }

    // Sélecteur de cible de déplacement.
    if app.move_targets.is_some() {
        match key.code {
            KeyCode::Esc => app.move_picker_cancel(),
            KeyCode::Enter => app.move_picker_select(),
            KeyCode::Up | KeyCode::Char('k') => app.move_picker_move(-1),
            KeyCode::Down | KeyCode::Char('j') => app.move_picker_move(1),
            _ => {}
        }
        return;
    }

    // Corbeille (restauration / purge définitive).
    if app.trash_view.is_some() {
        let awaiting_purge = app
            .trash_view
            .as_ref()
            .map(|t| t.confirm.is_some())
            .unwrap_or(false);
        if awaiting_purge {
            match key.code {
                KeyCode::Char('o') | KeyCode::Char('O') | KeyCode::Char('y') | KeyCode::Char('Y')
                | KeyCode::Enter => app.trash_confirm_apply(),
                KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => app.trash_confirm_cancel(),
                _ => {}
            }
        } else {
            match key.code {
                KeyCode::Esc => app.trash_cancel(),
                KeyCode::Enter | KeyCode::Char('r') => app.trash_restore_selected(),
                KeyCode::Up | KeyCode::Char('k') => app.trash_move(-1),
                KeyCode::Down | KeyCode::Char('j') => app.trash_move(1),
                KeyCode::Char('d') | KeyCode::Delete => app.trash_request_purge(),
                KeyCode::Char('x') | KeyCode::Char('X') => app.trash_request_empty(),
                _ => {}
            }
        }
        return;
    }

    // Recherche de session (live : filtre chemin/id à la frappe, Tab = contenu).
    if app.search.is_some() {
        match key.code {
            KeyCode::Esc => app.search_cancel(),
            KeyCode::Enter => app.search_open_selected(),
            KeyCode::Up => app.search_move(-1),
            KeyCode::Down => app.search_move(1),
            KeyCode::Tab => app.search_deep(),
            KeyCode::Backspace => app.search_input_backspace(),
            KeyCode::Char(c) => app.search_input_char(c),
            _ => {}
        }
        return;
    }

    // L'overlay d'aide capture l'essentiel des touches.
    if app.show_help {
        match key.code {
            KeyCode::Char('?') | KeyCode::Esc | KeyCode::Char('q') => app.toggle_help(),
            _ => {}
        }
        return;
    }

    // Toute frappe efface une notification de statut affichée.
    app.status = None;

    // Le formulaire de réglages capture les touches pendant l'édition d'un champ.
    if app.section == Section::Config && app.settings.is_editing() {
        handle_settings_edit_key(app, key);
        return;
    }

    match key.code {
        KeyCode::Char('q') => app.quit(),
        KeyCode::Char('?') => app.toggle_help(),
        KeyCode::Char('e') => app.do_export(),
        KeyCode::Char('E') => app.request_edit(),
        KeyCode::Char('h') | KeyCode::Char('H') => app.open_picker(),
        KeyCode::Char('/') => app.open_search(),
        KeyCode::Char('c') => app.open_trash(),
        // Section Config : enregistrer / basculer vers le JSON brut.
        KeyCode::Char('s') => app.save_settings(),
        KeyCode::Char('r') => app.toggle_settings_raw(),
        // En vue agrégée : changer le home cible de Mémoire/Config.
        KeyCode::Char('t') => app.cycle_config_target(),

        // Sélection directe de section.
        KeyCode::Char('1') => app.set_section(Section::Browse),
        KeyCode::Char('2') => app.set_section(Section::Memory),
        KeyCode::Char('3') => app.set_section(Section::Config),
        KeyCode::Tab => app.next_section(),
        // Shift-Tab bascule le focus entre panneaux dans Browse.
        KeyCode::BackTab => app.toggle_focus(),

        // En sous-vue (transcript), Esc remonte ; sinon il quitte.
        KeyCode::Esc if !app.back() => app.quit(),
        KeyCode::Esc => {}

        KeyCode::Enter => app.on_enter(),

        // Ménage des sessions (focus Sessions dans Browse).
        KeyCode::Char('d') | KeyCode::Delete => app.request_delete_session(),
        KeyCode::Char('m') => app.request_move_session(),

        // Repliage des groupes (homes) en vue agrégée.
        KeyCode::Char(' ') => app.toggle_collapse_current(),
        KeyCode::Char('z') => app.toggle_collapse_all(),

        KeyCode::Up | KeyCode::Char('k') => app.move_up(),
        KeyCode::Down | KeyCode::Char('j') => app.move_down(),

        KeyCode::Left => app.nav_left(),
        KeyCode::Right => app.nav_right(),

        KeyCode::PageUp => app.page_up(),
        KeyCode::PageDown => app.page_down(),
        KeyCode::Home => app.go_home(),
        KeyCode::End => app.go_end(),

        _ => {}
    }
}

/// Traite les touches quand le sélecteur de home est ouvert. En mode saisie de
/// chemin, la frappe alimente le tampon ; sinon on navigue dans la liste.
fn handle_picker_key(app: &mut App, key: KeyEvent) {
    // Mode saisie : on capture les caractères, Backspace, Enter et Esc.
    if matches!(app.picker_mode, PickerMode::AddInput(_)) {
        match key.code {
            KeyCode::Esc => app.picker_cancel_input(),
            KeyCode::Enter => app.picker_confirm_add(),
            KeyCode::Backspace => app.picker_input_backspace(),
            KeyCode::Char(c) => app.picker_input_char(c),
            _ => {}
        }
        return;
    }

    // Mode navigation.
    match key.code {
        KeyCode::Esc => app.close_picker(),
        KeyCode::Enter => app.picker_select(),
        KeyCode::Up | KeyCode::Char('k') => app.picker_move(-1),
        KeyCode::Down | KeyCode::Char('j') => app.picker_move(1),
        KeyCode::Char('a') => app.picker_start_add(),
        KeyCode::Char('d') => app.picker_remove_highlight(),
        _ => {}
    }
}

/// Touches pendant l'édition d'un champ du formulaire Config.
fn handle_settings_edit_key(app: &mut App, key: KeyEvent) {
    let s = &mut app.settings;
    // Saisie scalaire, ou saisie d'un élément de liste.
    if s.editing_scalar() || s.editing_list_input() {
        match key.code {
            KeyCode::Esc => s.input_cancel(),
            KeyCode::Enter => s.input_commit(),
            KeyCode::Backspace => s.input_backspace(),
            KeyCode::Char(c) => s.input_char(c),
            _ => {}
        }
        return;
    }
    // Navigation dans l'éditeur de liste (StringList / KeyValue).
    match key.code {
        KeyCode::Esc => s.list_done(),
        KeyCode::Up | KeyCode::Char('k') => s.list_move(-1),
        KeyCode::Down | KeyCode::Char('j') => s.list_move(1),
        KeyCode::Char('a') => s.list_add(),
        KeyCode::Enter => s.list_begin_edit(),
        KeyCode::Char('d') => s.list_delete(),
        _ => {}
    }
}
