use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::{
    connection::store::{add_connection, delete_connection},
    tui::{
        state::{
            AppMode, AppState, Overlay,
            cmdline::{CommandLineMode, ConfirmAction, compute_completions},
            form::{AddConnectionForm, TextMode},
        },
        ui::home::{
            goto_bottom, goto_top, remove_selected, select_next, select_prev, selected_connection,
        },
    },
};

// ── Entry point ───────────────────────────────────────────────────────────────

pub async fn handle_key_event(event: KeyEvent, state: &mut AppState) -> color_eyre::Result<()> {
    if event.modifiers.contains(KeyModifiers::CONTROL) && event.code == KeyCode::Char('c') {
        state.should_quit = true;
        return Ok(());
    }

    // Command line has the highest priority when active.
    if state.cmdline.is_active() {
        handle_cmdline(event, state);
        return Ok(());
    }

    // The add-connection form intercepts all keys while open.
    if state.form.is_some() {
        handle_form(event, state).await?;
        return Ok(());
    }

    // Overlays intercept the pipeline before mode handlers.
    if state.overlay.is_some() {
        handle_overlay(event, state);
        return Ok(());
    }

    match state.mode {
        AppMode::Home => handle_home(event, state),
        AppMode::Dashboard => {} // TODO: route to dashboard handler
    }

    Ok(())
}

// ── Command-line handler ──────────────────────────────────────────────────────

fn handle_cmdline(event: KeyEvent, state: &mut AppState) {
    match event.code {
        // Esc always cancels and returns to idle.
        KeyCode::Esc => {
            state.cmdline.reset();
        }

        // Backspace on an empty buffer also cancels (vim behaviour).
        KeyCode::Backspace => {
            if state.cmdline.input.is_empty() {
                state.cmdline.reset();
            } else {
                state.cmdline.clear_completions();
                state.cmdline.pop();
            }
        }

        // Tab — open or cycle forward through completions.
        KeyCode::Tab => {
            if let CommandLineMode::Input = state.cmdline.mode {
                if state.cmdline.completions.is_empty() {
                    let matches = compute_completions(&state.cmdline.input);
                    state.cmdline.open_completions(matches);
                } else {
                    state.cmdline.next_completion();
                }
            }
        }

        // Shift+Tab — cycle backward through completions.
        KeyCode::BackTab => {
            if let CommandLineMode::Input = state.cmdline.mode {
                state.cmdline.prev_completion();
            }
        }

        // Enter commits whatever is in the buffer.
        KeyCode::Enter => execute_cmdline(state),

        KeyCode::Char(c) => match &state.cmdline.mode {
            CommandLineMode::Input => {
                // Any typed character dismisses completions and resumes free input.
                state.cmdline.clear_completions();
                state.cmdline.push(c);
            }
            // Confirm only accepts a single y/n character.
            CommandLineMode::Confirm(_) => {
                if state.cmdline.input.is_empty() && matches!(c, 'y' | 'Y' | 'n' | 'N') {
                    state.cmdline.push(c);
                }
            }
            CommandLineMode::Idle => {}
        },

        _ => {}
    }
}

fn execute_cmdline(state: &mut AppState) {
    match state.cmdline.mode.clone() {
        CommandLineMode::Confirm(ConfirmAction::DeleteConnection(ref name)) => {
            if state.cmdline.input.to_lowercase() == "y" {
                let _ = delete_connection(name.clone());
                remove_selected(state);
            }
            state.cmdline.reset();
        }

        CommandLineMode::Input => {
            let cmd = state.cmdline.input.trim().to_string();
            state.cmdline.reset();
            execute_command(&cmd, state);
        }

        CommandLineMode::Idle => {}
    }
}

/// Parse and dispatch a `:command` string.
fn execute_command(cmd: &str, state: &mut AppState) {
    match cmd {
        // Navigation / app control
        "" => {}
        "q" | "quit" => state.should_quit = true,
        "q!" => state.should_quit = true,

        // Overlays
        "h" | "help" => state.overlay = Some(Overlay::Help),
        "add" => {
            state.overlay = Some(Overlay::AddConnection);
            state.form = Some(AddConnectionForm::new());
        }
        "connect" => state.overlay = Some(Overlay::ConnectionPicker),

        // Destructive actions — flow into the confirm prompt
        "d" | "delete" => {
            if let Some(db) = selected_connection(state) {
                let name = db.name.clone();
                state
                    .cmdline
                    .open_confirm(ConfirmAction::DeleteConnection(name));
            } else {
                state
                    .cmdline
                    .set_error("no connection selected".to_string());
            }
        }

        other => {
            state
                .cmdline
                .set_error(format!("Error: not a command `{other}`"));
        }
    }
}

// ── Overlay handler ───────────────────────────────────────────────────────────

fn handle_overlay(event: KeyEvent, state: &mut AppState) {
    let Some(overlay) = state.overlay else { return };

    match (overlay, event.code) {
        // Help — q / ? / Esc closes it
        (Overlay::Help, KeyCode::Char('q') | KeyCode::Char('?') | KeyCode::Esc) => {
            state.overlay = None;
        }

        // AddConnection — Esc cancels (form input handled later)
        (Overlay::AddConnection, KeyCode::Esc | KeyCode::Char('q')) => {
            state.overlay = None;
        }

        // CommandPalette — Esc cancels
        (Overlay::CommandPalette, KeyCode::Esc | KeyCode::Char('q')) => {
            state.overlay = None;
        }

        // ConfirmDelete is now handled via the command-line bar, not as an overlay.
        (Overlay::ConfirmDelete, KeyCode::Esc | KeyCode::Char('q')) => {
            state.overlay = None;
        }

        // Connection picker — vim navigation + connect
        (Overlay::ConnectionPicker, KeyCode::Char('j') | KeyCode::Down) => {
            select_next(state);
            state.pending_key = None;
        }
        (Overlay::ConnectionPicker, KeyCode::Char('k') | KeyCode::Up) => {
            select_prev(state);
            state.pending_key = None;
        }
        (Overlay::ConnectionPicker, KeyCode::Char('G')) => {
            goto_bottom(state);
            state.pending_key = None;
        }
        (Overlay::ConnectionPicker, KeyCode::Char('g')) => {
            if state.pending_key == Some('g') {
                goto_top(state);
                state.pending_key = None;
            } else {
                state.pending_key = Some('g');
            }
        }
        (Overlay::ConnectionPicker, KeyCode::Enter) => {
            if selected_connection(state).is_some() {
                state.overlay = None;
                state.mode = AppMode::Dashboard; // TODO: initialise session
            }
            state.pending_key = None;
        }
        (Overlay::ConnectionPicker, KeyCode::Esc | KeyCode::Char('q')) => {
            state.overlay = None;
            state.pending_key = None;
        }

        _ => {}
    }
}

// ── Home mode ─────────────────────────────────────────────────────────────────

fn handle_home(event: KeyEvent, state: &mut AppState) {
    match event.code {
        // ── Quit ──────────────────────────────────────────────────────────────
        KeyCode::Char('q') => state.should_quit = true,

        // ── Open connection picker ─────────────────────────────────────────────
        KeyCode::Char('c') => {
            state.overlay = Some(Overlay::ConnectionPicker);
            state.pending_key = None;
        }

        // ── Actions ───────────────────────────────────────────────────────────
        KeyCode::Char('a') => {
            state.overlay = Some(Overlay::AddConnection);
            state.form = Some(AddConnectionForm::new());
            state.pending_key = None;
        }
        KeyCode::Char('d') => {
            if let Some(db) = selected_connection(state) {
                let name = db.name.clone();
                state.cmdline.open_confirm(ConfirmAction::DeleteConnection(name));
            }
            state.pending_key = None;
        }
        KeyCode::Char(':') => {
            state.cmdline.open_input();
            state.pending_key = None;
        }
        KeyCode::Char('?') => {
            state.overlay = Some(Overlay::Help);
            state.pending_key = None;
        }

        _ => state.pending_key = None,
    }
}

// ── Add-connection form handler ───────────────────────────────────────────────

async fn handle_form(event: KeyEvent, state: &mut AppState) -> color_eyre::Result<()> {
    if state.form.is_none() {
        return Ok(());
    }

    // Ctrl+S always submits regardless of mode or field.
    if event.modifiers.contains(KeyModifiers::CONTROL) && event.code == KeyCode::Char('s') {
        submit_form(state).await?;
        return Ok(());
    }

    let is_text = state.form.as_ref().unwrap().focused_field().is_text();
    let text_mode = state.form.as_ref().unwrap().text_mode.clone();

    match event.code {
        // ── Universal ───────────────────────────────────────────────────────────────

        // Esc: exit Insert → Normal; in Normal → close the form.
        KeyCode::Esc => {
            if is_text && text_mode == TextMode::Insert {
                state.form.as_mut().unwrap().enter_normal();
            } else {
                state.form = None;
                state.overlay = None;
            }
        }

        // Tab / Enter — advance field (always resets to Insert on arrival).
        KeyCode::Tab | KeyCode::Enter => {
            state.form.as_mut().unwrap().focus_next();
        }

        // Shift+Tab — go back.
        KeyCode::BackTab => {
            state.form.as_mut().unwrap().focus_prev();
        }

        // ── Text field keys ────────────────────────────────────────────────────────

        // Up / Down — field navigation, always, regardless of mode.
        KeyCode::Down => {
            state.form.as_mut().unwrap().focus_next();
        }
        KeyCode::Up => {
            state.form.as_mut().unwrap().focus_prev();
        }

        // Left / Right: cursor movement on text fields; cycle on selectors.
        KeyCode::Left => {
            let form = state.form.as_mut().unwrap();
            if is_text {
                form.cursor_left();
            } else {
                form.cycle_left();
            }
        }
        KeyCode::Right => {
            let form = state.form.as_mut().unwrap();
            if is_text {
                form.cursor_right();
            } else {
                form.cycle_right();
            }
        }

        // Backspace: only delete in Insert mode.
        KeyCode::Backspace if is_text && text_mode == TextMode::Insert => {
            state.form.as_mut().unwrap().delete_before_cursor();
        }

        // Space: insert on text fields (Insert mode) or toggle selectors.
        KeyCode::Char(' ') => {
            let form = state.form.as_mut().unwrap();
            if is_text && text_mode == TextMode::Insert {
                form.insert_char(' ');
            } else if !is_text {
                form.toggle_focused();
            }
        }

        // q in Normal mode — close the form without saving.
        KeyCode::Char('q') if text_mode == TextMode::Normal => {
            state.form = None;
            state.overlay = None;
        }

        // Character input.
        KeyCode::Char(c) => {
            let form = state.form.as_mut().unwrap();
            if is_text {
                match text_mode {
                    TextMode::Insert => form.insert_char(c),
                    TextMode::Normal => match c {
                        'i' => form.enter_insert_before(),
                        'a' => form.enter_insert_after(),
                        'I' => form.enter_insert_at_start(),
                        'A' => form.enter_insert_at_end(),
                        'h' => form.cursor_left(),
                        'l' => form.cursor_right(),
                        'j' => form.focus_next(),
                        'k' => form.focus_prev(),
                        '0' => form.cursor_to_start(),
                        '$' => form.cursor_to_end(),
                        'x' => form.delete_at_cursor(),
                        _ => {}
                    },
                }
            } else {
                // Selector / toggle fields: h/l cycle, j/k navigate in Normal mode.
                if text_mode == TextMode::Normal {
                    match c {
                        'h' => form.cycle_left(),
                        'l' => form.cycle_right(),
                        'j' => form.focus_next(),
                        'k' => form.focus_prev(),
                        _ => {}
                    }
                }
            }
        }

        _ => {}
    }

    Ok(())
}

async fn submit_form(state: &mut AppState) -> color_eyre::Result<()> {
    let Some(ref form) = state.form else {
        return Ok(());
    };

    // Validate before hitting the network.
    if let Err(e) = form.validate() {
        state.cmdline.set_error(e);
        return Ok(());
    }

    let name = form.name.trim().to_string();
    let engine = form.engine.clone();
    let source = match form.build_source() {
        Ok(s) => s,
        Err(e) => {
            state.cmdline.set_error(e);
            return Ok(());
        }
    };

    // add_connection tests the connection before persisting.
    match add_connection(name, source, engine).await {
        Ok(db) => {
            state.connections.push(db);
            state.selected_connection = state.connections.len() - 1;
            state.form = None;
            state.overlay = None;
        }
        Err(e) => {
            state.cmdline.set_error(e.to_string());
        }
    }

    Ok(())
}
