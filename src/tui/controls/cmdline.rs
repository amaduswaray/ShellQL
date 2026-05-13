use crossterm::event::{KeyCode, KeyEvent};

use crate::{
    connection::delete_connection,
    tui::{
        AddConnectionForm, AppState, CommandLineMode, ConfirmAction, Overlay, compute_completions,
        ui::home::{remove_selected, selected_connection},
    },
};

pub fn handle_cmdline(event: KeyEvent, state: &mut AppState) {
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

        // Pane management (dashboard only)
        "vnew" => {
            if let Some(ref mut dash) = state.dashboard {
                let _ = dash.tree.split_active_v(crate::tui::state::PaneType::TableView);
            } else {
                state.cmdline.set_error("not in dashboard");
            }
        }
        "new" => {
            if let Some(ref mut dash) = state.dashboard {
                let _ = dash.tree.split_active_h(crate::tui::state::PaneType::SchemaView);
            } else {
                state.cmdline.set_error("not in dashboard");
            }
        }
        "close" => {
            if let Some(ref mut dash) = state.dashboard {
                if dash.tree.close_active() {
                    state.mode = crate::tui::state::AppMode::Home;
                    state.dashboard = None;
                }
            } else {
                state.cmdline.set_error("not in dashboard");
            }
        }

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
