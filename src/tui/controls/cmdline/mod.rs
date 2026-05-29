use crossterm::event::{KeyCode, KeyEvent};

use crate::tui::{
    AppMode, AppState, CommandLineMode, ConfirmAction, DASHBOARD_COMMANDS, HOME_COMMANDS,
    compute_completions,
};

pub mod data;
pub mod nav;
pub mod pane;
pub mod search;
pub mod sql;
pub mod tab;

pub fn handle_cmdline(event: KeyEvent, state: &mut AppState) {
    match event.code {
        KeyCode::Esc => {
            // Cancel live search: clear live_search from the active pane but
            // leave last_search intact so previous committed search stays.
            if let CommandLineMode::Search(_) = state.cmdline.mode {
                if let Some(tab) = state.active_tab_mut() {
                    if let Some(pane) = tab.tree.active_mut() {
                        pane.live_search = None;
                    }
                }
            }
            state.cmdline.reset();
        }

        KeyCode::Backspace => {
            if state.cmdline.input.is_empty()
                && !matches!(state.cmdline.mode, CommandLineMode::CellEdit { .. })
            {
                state.cmdline.reset();
            } else {
                state.cmdline.clear_completions();
                state.cmdline.pop();
            }
        }

        KeyCode::Tab => {
            if let CommandLineMode::Input = state.cmdline.mode {
                if state.cmdline.completions.is_empty() {
                    let list = match state.mode {
                        AppMode::Home => HOME_COMMANDS,
                        AppMode::Dashboard => DASHBOARD_COMMANDS,
                    };

                    let matches = compute_completions(&state.cmdline.input, list);
                    state.cmdline.open_completions(matches);
                } else {
                    state.cmdline.next_completion();
                }
            }
        }

        KeyCode::BackTab => {
            if let CommandLineMode::Input = state.cmdline.mode {
                state.cmdline.prev_completion();
            }
        }

        KeyCode::Enter => execute_cmdline(state),

        KeyCode::Char(c) => match &state.cmdline.mode {
            CommandLineMode::Input
            | CommandLineMode::Search(_)
            | CommandLineMode::CellEdit { .. } => {
                state.cmdline.clear_completions();
                state.cmdline.push(c);
                // For search mode, recompute live fuzzy matches after every keystroke.
                if let CommandLineMode::Search(direction) = state.cmdline.mode {
                    search::compute_live_search(direction, state);
                }
            }
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
                let _ = crate::connection::delete_connection(name.clone());
                remove_connection_by_name(state, name);
            }
            state.cmdline.reset();
        }

        CommandLineMode::Confirm(ConfirmAction::CommitWrites { .. }) => {
            if state.cmdline.input.to_lowercase() == "y" {
                // Re-build and execute the commit from the pane's current pending state.
                data::execute_pending_commit(state);
            }
            state.cmdline.reset();
        }

        CommandLineMode::Input => {
            let cmd = state.cmdline.input.trim().to_string();
            state.cmdline.reset();
            execute_command(&cmd, state);
        }

        CommandLineMode::Search(direction) => {
            let query = state.cmdline.input.trim().to_string();
            state.cmdline.reset();
            if !query.is_empty() {
                search::commit_search(&query, direction, state);
            }
        }

        CommandLineMode::CellEdit { row, col, .. } => {
            let new_value = state.cmdline.input.trim().to_string();
            state.cmdline.reset();
            let table_name = state
                .active_tab_mut()
                .and_then(|tab| tab.tree.active_mut())
                .and_then(|pane| pane.bound_table.clone());

            if let Some(ref table_name) = table_name {
                let bounds = state
                    .table_cache
                    .get(table_name)
                    .map(|l| (l.rows.len(), l.headers.len()));
                if let Some((row_count, col_count)) = bounds {
                    if row < row_count && col < col_count {
                        let Some(tab) = state.active_tab_mut() else {
                            return;
                        };
                        let Some(pane) = tab.tree.active_mut() else {
                            return;
                        };
                        pane.pending_updates
                            .retain(|(r, c, _)| !(*r == row && *c == col));
                        pane.pending_updates.push((row, col, new_value));
                    }
                }
            }
        }

        CommandLineMode::Idle => {}
    }
}

/// Parse and dispatch a `:command` string with optional arguments.
fn remove_connection_by_name(state: &mut AppState, name: &str) {
    let Some(idx) = state.connections.iter().position(|db| db.name == name) else {
        return;
    };

    state.connections.remove(idx);
    if state.connections.is_empty() {
        state.selected_connection = 0;
        return;
    }

    if state.selected_connection > idx {
        state.selected_connection -= 1;
    } else if state.selected_connection >= state.connections.len() {
        state.selected_connection = state.connections.len() - 1;
    }
}

fn execute_command(cmd: &str, state: &mut AppState) {
    let parts: Vec<&str> = cmd.split_whitespace().collect();
    if parts.is_empty() {
        return;
    }

    let name = parts[0];
    let args = &parts[1..];

    match name {
        "" => {}

        // Quit
        "exit" => nav::cmd_exit(state),

        "q" | "quit" => nav::cmd_quit(state, args),

        // Overlays
        "h" | "help" => nav::cmd_help(state),
        "add" => nav::cmd_add(state),
        "connect" => nav::cmd_connect(state),

        // Pane management
        "vnew" => pane::cmd_vnew(state, args),
        "hnew" => pane::cmd_hnew(state, args),
        "new" => pane::cmd_vnew(state, args), // alias for vnew

        "show" => pane::cmd_show(state, args),
        "tables" => pane::cmd_tables(state, args),
        "noh" => pane::cmd_noh(state),
        "schema" => pane::cmd_schema(state, args),
        "sql" | "query" => pane::cmd_sql(state, args),
        "queryresults" => pane::cmd_query_results(state, args),

        "disconnect" => nav::cmd_disconnect(state),

        "close" => pane::cmd_close(state, args),

        "w" | "write" => data::cmd_write(state, args),

        "where" => data::cmd_where(state, args),
        "order" => data::cmd_order(state, args),
        "select" => data::cmd_select(state, args),

        "resize" => nav::cmd_resize(state, args),
        "reset" => data::cmd_reset(state),
        "full" => nav::cmd_fullscreen(state),

        // Destructive actions
        "d" | "delete" => nav::cmd_delete(state, args),

        "back" => nav::cmd_back(state),
        "forward" => nav::cmd_forward(state),

        "tnew" => tab::cmd_tab_new(state, args),
        "tnext" => tab::cmd_tab_next(state),
        "tprev" => tab::cmd_tab_prev(state),
        "tdelete" => tab::cmd_tab_delete(state),
        "t" => tab::cmd_tab_goto(state, args),

        "!" => nav::cmd_bang(state, args),

        other => state
            .cmdline
            .set_error(format!("Error: not a command `{other}`")),
    }
}
