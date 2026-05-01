use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::{
    connection::store::delete_connection,
    tui::{
        state::{
            AppMode, AppState, Overlay,
            cmdline::{CommandLineMode, ConfirmAction},
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

    // Clear any one-shot error from the previous command on the next keypress.
    state.cmdline.clear_error();

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
                state.cmdline.pop();
            }
        }

        // Enter commits whatever is in the buffer.
        KeyCode::Enter => execute_cmdline(state),

        KeyCode::Char(c) => match &state.cmdline.mode {
            CommandLineMode::Input => state.cmdline.push(c),
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
        "add" => state.overlay = Some(Overlay::AddConnection),

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
        // This branch is a safety fallback.
        (Overlay::ConfirmDelete, KeyCode::Esc | KeyCode::Char('q')) => {
            state.overlay = None;
        }

        _ => {}
    }
}

// ── Home mode ─────────────────────────────────────────────────────────────────

fn handle_home(event: KeyEvent, state: &mut AppState) {
    match event.code {
        // ── Quit ─────────────────────────────────────────────────────────────
        KeyCode::Char('q') => {
            state.should_quit = true;
        }

        // ── Vim navigation ────────────────────────────────────────────────────
        KeyCode::Char('j') | KeyCode::Down => {
            select_next(state);
            state.pending_key = None;
        }
        KeyCode::Char('k') | KeyCode::Up => {
            select_prev(state);
            state.pending_key = None;
        }
        // G — jump to last connection
        KeyCode::Char('G') => {
            goto_bottom(state);
            state.pending_key = None;
        }
        // g → arms the buffer; gg → jump to first
        KeyCode::Char('g') => match state.pending_key {
            Some('g') => {
                goto_top(state);
                state.pending_key = None;
            }
            _ => state.pending_key = Some('g'),
        },

        // ── Actions ───────────────────────────────────────────────────────────
        // Enter — open a session for the selected connection
        KeyCode::Enter => {
            if selected_connection(state).is_some() {
                // TODO: initialise session, then transition
                state.mode = AppMode::Dashboard;
            }
            state.pending_key = None;
        }

        // a — open AddConnection overlay
        KeyCode::Char('a') => {
            state.overlay = Some(Overlay::AddConnection);
            state.pending_key = None;
        }

        // d — inline delete confirmation via the command line
        KeyCode::Char('d') => {
            if let Some(db) = selected_connection(state) {
                let name = db.name.clone();
                state
                    .cmdline
                    .open_confirm(ConfirmAction::DeleteConnection(name));
            }
            state.pending_key = None;
        }

        // : — open the vim-style command prompt
        KeyCode::Char(':') => {
            state.cmdline.open_input();
            state.pending_key = None;
        }

        // ? — open help overlay
        KeyCode::Char('?') => {
            state.overlay = Some(Overlay::Help);
            state.pending_key = None;
        }

        // Any unrecognised key clears the pending buffer.
        _ => state.pending_key = None,
    }
}
