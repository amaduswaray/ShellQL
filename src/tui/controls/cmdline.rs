use crossterm::event::{KeyCode, KeyEvent};

use crate::{
    connection::delete_connection,
    tui::{
        AddConnectionForm, AppMode, AppState, CommandLineMode, ConfirmAction, DASHBOARD_COMMANDS,
        HOME_COMMANDS, Overlay, compute_completions,
        ui::home::{remove_selected, selected_connection},
    },
};

pub fn handle_cmdline(event: KeyEvent, state: &mut AppState) {
    match event.code {
        KeyCode::Esc => state.cmdline.reset(),

        KeyCode::Backspace => {
            if state.cmdline.input.is_empty() {
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
            CommandLineMode::Input => {
                state.cmdline.clear_completions();
                state.cmdline.push(c);
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

/// Parse and dispatch a `:command` string with optional arguments.
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
        "exit" => state.should_quit = true,

        // On dashboard :q acts like :close; on home it does nothing useful.
        "q" | "quit" => {
            if state.dashboard.is_some() {
                cmd_close(state, args);
            }
        }

        // Overlays
        "h" | "help" => state.overlay = Some(Overlay::Help),
        "add" => {
            state.overlay = Some(Overlay::AddConnection);
            state.form = Some(AddConnectionForm::new());
        }
        "connect" => state.overlay = Some(Overlay::ConnectionPicker),

        // Pane management
        "vnew" => cmd_vnew(state, args),
        "hnew" => cmd_hnew(state, args),
        "new" => cmd_vnew(state, args), // alias for vnew

        "open" => cmd_open(state, args),
        "tables" => cmd_tables(state, args),
        "schema" => cmd_schema(state, args),
        "sql" | "query" => cmd_sql(state, args),

        "close" => cmd_close(state, args),

        // Destructive actions
        "d" | "delete" => {
            if let Some(db) = selected_connection(state) {
                let name = db.name.clone();
                state.cmdline.open_confirm(ConfirmAction::DeleteConnection(name));
            } else {
                state.cmdline.set_error("no connection selected");
            }
        }

        other => state.cmdline.set_error(format!("Error: not a command `{other}`")),
    }
}

// ── Pane commands ─────────────────────────────────────────────────────────────

fn require_dashboard(state: &mut AppState) -> Option<&mut crate::tui::state::DashboardState> {
    state.dashboard.as_mut()
}

fn parse_pane_type(arg: Option<&&str>) -> crate::tui::state::PaneType {
    match arg {
        Some(s) if s.eq_ignore_ascii_case("table") => crate::tui::state::PaneType::TableView,
        Some(s) if s.eq_ignore_ascii_case("schema") => crate::tui::state::PaneType::SchemaView,
        Some(s) if s.eq_ignore_ascii_case("sql") || s.eq_ignore_ascii_case("query") => {
            crate::tui::state::PaneType::QueryEditor
        }
        _ => crate::tui::state::PaneType::TableList,
    }
}

fn cmd_vnew(state: &mut AppState, args: &[&str]) {
    let Some(dash) = require_dashboard(state) else {
        state.cmdline.set_error("not in dashboard");
        return;
    };

    let kind = parse_pane_type(args.first());
    let table_name = args.get(1).map(|s| s.to_string());

    match dash.tree.split_active_v(kind) {
        Ok(id) => {
            if let Some(table) = table_name {
                if let Some(pane) = dash.tree.panes.get_mut(&id) {
                    match pane.kind {
                        crate::tui::state::PaneType::TableView => {
                            pane.set_table_view(table.clone());
                            if !dash.table_cache.contains_key(&table) {
                                dash.pending_load = Some(table);
                                dash.loading = true;
                                dash.error = None;
                            }
                        }
                        crate::tui::state::PaneType::SchemaView => {
                            pane.set_schema_view(table);
                        }
                        _ => {}
                    }
                }
            }
        }
        Err(e) => state.cmdline.set_error(e),
    }
}

fn cmd_hnew(state: &mut AppState, args: &[&str]) {
    let Some(dash) = require_dashboard(state) else {
        state.cmdline.set_error("not in dashboard");
        return;
    };

    let kind = parse_pane_type(args.first());
    let table_name = args.get(1).map(|s| s.to_string());

    match dash.tree.split_active_h(kind) {
        Ok(id) => {
            if let Some(table) = table_name {
                if let Some(pane) = dash.tree.panes.get_mut(&id) {
                    match pane.kind {
                        crate::tui::state::PaneType::TableView => {
                            pane.set_table_view(table.clone());
                            if !dash.table_cache.contains_key(&table) {
                                dash.pending_load = Some(table);
                                dash.loading = true;
                                dash.error = None;
                            }
                        }
                        crate::tui::state::PaneType::SchemaView => {
                            pane.set_schema_view(table);
                        }
                        _ => {}
                    }
                }
            }
        }
        Err(e) => state.cmdline.set_error(e),
    }
}

fn cmd_open(state: &mut AppState, args: &[&str]) {
    let Some(dash) = require_dashboard(state) else {
        state.cmdline.set_error("not in dashboard");
        return;
    };

    let table_name = args.first().map(|s| s.to_string());

    if let Some(pane) = dash.tree.active_mut() {
        if let Some(name) = table_name {
            pane.set_table_view(name.clone());
            if !dash.table_cache.contains_key(&name) {
                dash.pending_load = Some(name);
                dash.loading = true;
                dash.error = None;
            }
        } else {
            state.cmdline.set_error(":open requires a table name");
        }
    }
}

fn cmd_tables(state: &mut AppState, _args: &[&str]) {
    let Some(dash) = require_dashboard(state) else {
        state.cmdline.set_error("not in dashboard");
        return;
    };

    if let Some(pane) = dash.tree.active_mut() {
        pane.reset_to_list();
    }
}

fn cmd_schema(state: &mut AppState, args: &[&str]) {
    let Some(dash) = require_dashboard(state) else {
        state.cmdline.set_error("not in dashboard");
        return;
    };

    // If an argument is provided, use it; otherwise fall back to the
    // active pane's bound table (useful when already viewing a table).
    let table_name = args.first().map(|s| s.to_string())
        .or_else(|| dash.tree.active().and_then(|p| p.bound_table.clone()));

    if let Some(pane) = dash.tree.active_mut() {
        if let Some(name) = table_name {
            pane.set_schema_view(name);
        } else {
            state.cmdline.set_error(":schema requires a table name (no bound table)");
        }
    }
}

fn cmd_sql(state: &mut AppState, _args: &[&str]) {
    let Some(dash) = require_dashboard(state) else {
        state.cmdline.set_error("not in dashboard");
        return;
    };

    if let Some(pane) = dash.tree.active_mut() {
        pane.set_query_editor();
    }
}

fn cmd_close(state: &mut AppState, args: &[&str]) {
    let Some(dash) = require_dashboard(state) else {
        state.cmdline.set_error("not in dashboard");
        return;
    };

    let closed = if args.is_empty() {
        dash.tree.close_active()
    } else if let Ok(id) = args[0].parse::<usize>() {
        dash.tree.close_by_display_id(id)
    } else {
        state.cmdline.set_error(format!("invalid pane id `{}`", args[0]));
        return;
    };

    if closed {
        state.mode = crate::tui::state::AppMode::Home;
        state.dashboard = None;
    }
}
