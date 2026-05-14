use crossterm::event::{KeyCode, KeyEvent};

use crate::{
    connection::delete_connection,
    tui::{
        AddConnectionForm, AppMode, AppState, CommandLineMode, ConfirmAction, DASHBOARD_COMMANDS,
        HOME_COMMANDS, Overlay, SearchDirection, compute_completions,
        ui::home::{remove_selected, selected_connection},
    },
};

pub fn handle_cmdline(event: KeyEvent, state: &mut AppState) {
    match event.code {
        KeyCode::Esc => state.cmdline.reset(),

        KeyCode::Backspace => {
            if state.cmdline.input.is_empty() && !matches!(state.cmdline.mode, CommandLineMode::CellEdit { .. }) {
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
            CommandLineMode::Input | CommandLineMode::Search(_) | CommandLineMode::CellEdit { .. } => {
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

        CommandLineMode::Confirm(ConfirmAction::CommitWrites { .. }) => {
            if state.cmdline.input.to_lowercase() == "y" {
                // Re-build and execute the commit from the pane's current pending state.
                execute_pending_commit(state);
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
                commit_search(&query, direction, state);
            }
        }

        CommandLineMode::CellEdit { row, col, .. } => {
            let new_value = state.cmdline.input.trim().to_string();
            state.cmdline.reset();
            if let Some(ref mut dash) = state.dashboard {
                if let Some(pane) = dash.tree.active_mut() {
                    if let Some(ref table_name) = pane.bound_table {
                        if let Some(ref loaded) = dash.table_cache.get(table_name) {
                            if row < loaded.rows.len() && col < loaded.headers.len() {
                                // Remove any existing pending update for this cell.
                                pane.pending_updates.retain(|(r, c, _)| !(*r == row && *c == col));
                                pane.pending_updates.push((row, col, new_value));
                            }
                        }
                    }
                }
            }
        }

        CommandLineMode::Idle => {}
    }
}

fn commit_search(query: &str, direction: SearchDirection, state: &mut AppState) {
    let Some(dash) = state.dashboard.as_mut() else {
        return;
    };
    let Some(pane) = dash.tree.active_mut() else {
        return;
    };

    let query_lower = query.to_lowercase();

    match pane.kind {
        crate::tui::state::PaneType::TableList => {
            let matches: Vec<usize> = dash
                .tables
                .iter()
                .enumerate()
                .filter(|(_, name)| name.to_lowercase().contains(&query_lower))
                .map(|(i, _)| i)
                .collect();

            if matches.is_empty() {
                state
                    .cmdline
                    .set_error(format!("Pattern not found: {query}"));
                return;
            }

            let current = pane.nav_cursor;
            let current_idx = match direction {
                SearchDirection::Forward => matches.iter().position(|&m| m >= current).unwrap_or(0),
                SearchDirection::Backward => matches
                    .iter()
                    .rposition(|&m| m <= current)
                    .unwrap_or(matches.len() - 1),
            };

            pane.nav_cursor = matches[current_idx];
            pane.last_search = Some(crate::tui::SearchState {
                query: query.to_string(),
                direction,
                matches,
                current_idx,
            });
        }
        crate::tui::state::PaneType::TableView => {
            let Some(ref table_name) = pane.bound_table else {
                state.cmdline.set_error("no table bound");
                return;
            };
            let Some(ref loaded) = dash.table_cache.get(table_name) else {
                state.cmdline.set_error("table not loaded");
                return;
            };

            let matches: Vec<usize> = loaded
                .rows
                .iter()
                .enumerate()
                .filter(|(_, row)| {
                    row.iter().any(|cell| cell.to_lowercase().contains(&query_lower))
                })
                .map(|(i, _)| i)
                .collect();

            if matches.is_empty() {
                state
                    .cmdline
                    .set_error(format!("Pattern not found: {query}"));
                return;
            }

            let current = pane.row_cursor;
            let current_idx = match direction {
                SearchDirection::Forward => matches.iter().position(|&m| m >= current).unwrap_or(0),
                SearchDirection::Backward => matches
                    .iter()
                    .rposition(|&m| m <= current)
                    .unwrap_or(matches.len() - 1),
            };

            pane.row_cursor = matches[current_idx];
            pane.last_search = Some(crate::tui::SearchState {
                query: query.to_string(),
                direction,
                matches,
                current_idx,
            });
        }
        crate::tui::state::PaneType::QueryResults => {
            let Some(idx) = pane.bound_query_idx else {
                state.cmdline.set_error("no result set bound");
                return;
            };
            let Some(result) = dash.query_results.get(idx) else {
                state.cmdline.set_error("result set not available");
                return;
            };

            let matches: Vec<usize> = result
                .rows
                .iter()
                .enumerate()
                .filter(|(_, row)| {
                    row.iter().any(|cell| cell.to_lowercase().contains(&query_lower))
                })
                .map(|(i, _)| i)
                .collect();

            if matches.is_empty() {
                state
                    .cmdline
                    .set_error(format!("Pattern not found: {query}"));
                return;
            }

            let current = pane.row_cursor;
            let current_idx = match direction {
                SearchDirection::Forward => matches.iter().position(|&m| m >= current).unwrap_or(0),
                SearchDirection::Backward => matches
                    .iter()
                    .rposition(|&m| m <= current)
                    .unwrap_or(matches.len() - 1),
            };

            pane.row_cursor = matches[current_idx];
            pane.last_search = Some(crate::tui::SearchState {
                query: query.to_string(),
                direction,
                matches,
                current_idx,
            });
        }
        _ => {
            state
                .cmdline
                .set_error("Search only supported in table list, table view, and query results");
        }
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

        "q" | "quit" => {
            if let Some(ref dash) = state.dashboard {
                if dash.tree.pane_count() <= 1 {
                    state.should_quit = true;
                } else {
                    cmd_close(state, args);
                }
            } else {
                state.should_quit = true;
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
        "noh" => cmd_noh(state),
        "schema" => cmd_schema(state, args),
        "sql" | "query" => cmd_sql(state, args),
        "queryresults" => cmd_query_results(state, args),

        "close" => cmd_close(state, args),

        "w" | "write" => cmd_write(state, args),

        "where" => cmd_where(state, args),
        "order" => cmd_order(state, args),

        "resize" | "res" => cmd_resize(state, args),

        // Destructive actions
        "d" | "delete" => {
            if let Some(db) = selected_connection(state) {
                let name = db.name.clone();
                state
                    .cmdline
                    .open_confirm(ConfirmAction::DeleteConnection(name));
            } else {
                state.cmdline.set_error("no connection selected");
            }
        }

        other => state
            .cmdline
            .set_error(format!("Error: not a command `{other}`")),
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
        Some(s) if s.eq_ignore_ascii_case("queryresults") => {
            crate::tui::state::PaneType::QueryResults
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

    if kind == crate::tui::state::PaneType::QueryResults {
        state.cmdline.set_error("cannot create empty query results pane; use :queryResults or Ctrl+Enter");
        return;
    }

    if let Some(ref name) = table_name {
        if !dash.tables.contains(name) {
            state.cmdline.set_error(format!("table `{name}` not found"));
            return;
        }
    }

    match dash.tree.split_active_v(kind) {
        Ok(id) => {
            if let Some(table) = table_name {
                if let Some(pane) = dash.tree.panes.get_mut(&id) {
                    match pane.kind {
                        crate::tui::state::PaneType::TableView => {
                            pane.set_table_view(table.clone());
                            if !dash.table_cache.contains_key(&table) {
                                dash.pending_load = Some(crate::tui::state::dashboard::PendingQuery {
                                    table,
                                    filter: None,
                                    sort_col: None,
                                    sort_desc: false,
                                });
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

    if kind == crate::tui::state::PaneType::QueryResults {
        state.cmdline.set_error("cannot create empty query results pane; use :queryResults or Ctrl+Enter");
        return;
    }

    if let Some(ref name) = table_name {
        if !dash.tables.contains(name) {
            state.cmdline.set_error(format!("table `{name}` not found"));
            return;
        }
    }

    match dash.tree.split_active_h(kind) {
        Ok(id) => {
            if let Some(table) = table_name {
                if let Some(pane) = dash.tree.panes.get_mut(&id) {
                    match pane.kind {
                        crate::tui::state::PaneType::TableView => {
                            pane.set_table_view(table.clone());
                            if !dash.table_cache.contains_key(&table) {
                                dash.pending_load = Some(crate::tui::state::dashboard::PendingQuery {
                                    table,
                                    filter: None,
                                    sort_col: None,
                                    sort_desc: false,
                                });
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
            pane.last_search = None; // clear search highlight
            if !dash.table_cache.contains_key(&name) {
                dash.pending_load = Some(crate::tui::state::dashboard::PendingQuery {
                    table: name,
                    filter: None,
                    sort_col: None,
                    sort_desc: false,
                });
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
        pane.last_search = None; // clear search highlight
    }
}

fn cmd_noh(state: &mut AppState) {
    let Some(dash) = require_dashboard(state) else {
        state.cmdline.set_error("not in dashboard");
        return;
    };

    if let Some(pane) = dash.tree.active_mut() {
        pane.last_search = None;
    }
}

fn cmd_schema(state: &mut AppState, args: &[&str]) {
    let Some(dash) = require_dashboard(state) else {
        state.cmdline.set_error("not in dashboard");
        return;
    };

    // If an argument is provided, use it; otherwise fall back to the
    // active pane's bound table (useful when already viewing a table).
    let table_name = args
        .first()
        .map(|s| s.to_string())
        .or_else(|| dash.tree.active().and_then(|p| p.bound_table.clone()));

    if let Some(pane) = dash.tree.active_mut() {
        if let Some(name) = table_name {
            pane.set_schema_view(name);
        } else {
            state
                .cmdline
                .set_error(":schema requires a table name (no bound table)");
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

fn cmd_query_results(state: &mut AppState, _args: &[&str]) {
    let Some(dash) = require_dashboard(state) else {
        state.cmdline.set_error("not in dashboard");
        return;
    };

    if dash.query_results.is_empty() {
        state.cmdline.set_error("no query results available");
        return;
    }

    if let Some(pane) = dash.tree.active_mut() {
        pane.set_query_results(0);
        pane.query_result_count = dash.query_results.len();
    }
}

fn cmd_resize(state: &mut AppState, args: &[&str]) {
    let Some(dash) = require_dashboard(state) else {
        state.cmdline.set_error("not in dashboard");
        return;
    };

    if args.is_empty() {
        state.cmdline.set_error(":resize requires +N or -N");
        return;
    }

    let arg = args.join(" ");
    let delta = match arg.parse::<i32>() {
        Ok(v) if (-100..=100).contains(&v) => v,
        Ok(_) => {
            state.cmdline.set_error("resize value must be between -100 and 100");
            return;
        }
        Err(_) => {
            state.cmdline.set_error(format!("invalid resize value `{arg}`"));
            return;
        }
    };

    match dash.tree.resize_active(delta) {
        Ok(_) => {}
        Err(e) => state.cmdline.set_error(e),
    }
}

fn cmd_where(state: &mut AppState, args: &[&str]) {
    let Some(dash) = require_dashboard(state) else {
        state.cmdline.set_error("not in dashboard");
        return;
    };

    let active_id = dash.tree.active_pane;
    let Some(pane) = dash.tree.panes.get(&active_id) else {
        return;
    };

    if pane.kind != crate::tui::state::PaneType::TableView {
        state.cmdline.set_error(":where only works in table view");
        return;
    }

    if !pane.pending_updates.is_empty() || !pane.pending_deletes.is_empty() {
        state.cmdline.set_error("cannot filter with pending changes; :w or u to clear");
        return;
    }

    let table_name = match pane.bound_table.clone() {
        Some(t) => t,
        None => {
            state.cmdline.set_error("no table bound to active pane");
            return;
        }
    };

    let filter = if args.is_empty() {
        None
    } else {
        Some(args.join(" "))
    };

    if let Some(pane) = dash.tree.panes.get_mut(&active_id) {
        pane.filter = filter.clone();
    }

    dash.pending_load = Some(crate::tui::state::dashboard::PendingQuery {
        table: table_name,
        filter,
        sort_col: dash.tree.panes.get(&active_id).and_then(|p| p.sort_col.clone()),
        sort_desc: dash.tree.panes.get(&active_id).map_or(false, |p| p.sort_desc),
    });
    dash.loading = true;
    dash.error = None;

    if args.is_empty() {
        state.cmdline.set_loading("Filter cleared");
    }
}

fn cmd_order(state: &mut AppState, args: &[&str]) {
    let Some(dash) = require_dashboard(state) else {
        state.cmdline.set_error("not in dashboard");
        return;
    };

    let active_id = dash.tree.active_pane;
    let Some(pane) = dash.tree.panes.get(&active_id) else {
        return;
    };

    if pane.kind != crate::tui::state::PaneType::TableView {
        state.cmdline.set_error(":order only works in table view");
        return;
    }

    if !pane.pending_updates.is_empty() || !pane.pending_deletes.is_empty() {
        state.cmdline.set_error("cannot sort with pending changes; :w or u to clear");
        return;
    }

    let table_name = match pane.bound_table.clone() {
        Some(t) => t,
        None => {
            state.cmdline.set_error("no table bound to active pane");
            return;
        }
    };

    let (sort_col, sort_desc) = if args.is_empty() {
        (None, false)
    } else {
        let joined = args.join(" ");
        let parts: Vec<&str> = joined.split_whitespace().collect();
        let desc = parts.len() > 1 && parts.last().map_or(false, |s| s.eq_ignore_ascii_case("desc"));
        let col = if desc {
            parts[..parts.len() - 1].join(" ")
        } else {
            joined
        };
        (Some(col), desc)
    };

    if let Some(pane) = dash.tree.panes.get_mut(&active_id) {
        pane.sort_col = sort_col.clone();
        pane.sort_desc = sort_desc;
    }

    dash.pending_load = Some(crate::tui::state::dashboard::PendingQuery {
        table: table_name,
        filter: dash.tree.panes.get(&active_id).and_then(|p| p.filter.clone()),
        sort_col,
        sort_desc,
    });
    dash.loading = true;
    dash.error = None;

    if args.is_empty() {
        state.cmdline.set_loading("Sort cleared");
    }
}

fn cmd_write(state: &mut AppState, _args: &[&str]) {
    let Some(dash) = require_dashboard(state) else {
        state.cmdline.set_error("not in dashboard");
        return;
    };

    let active_id = dash.tree.active_pane;
    let Some(pane) = dash.tree.panes.get(&active_id) else {
        return;
    };

    // If active pane is a QueryEditor, :w executes the query.
    if pane.kind == crate::tui::state::PaneType::QueryEditor {
        let sql = pane.query_text.join("\n");
        if sql.trim().is_empty() {
            state.cmdline.set_error("query is empty");
            return;
        }
        dash.pending_query_exec = Some(sql);
        dash.loading = true;
        dash.error = None;
        return;
    }

    if pane.pending_updates.is_empty() && pane.pending_deletes.is_empty() {
        state.cmdline.set_error("no pending changes");
        return;
    }

    let table_name = match pane.bound_table.clone() {
        Some(t) => t,
        None => {
            state.cmdline.set_error("no table bound to active pane");
            return;
        }
    };

    let update_count = pane.pending_updates.len();
    let delete_count = pane.pending_deletes.len();

    // If there are deletes, ask for confirmation. Otherwise commit immediately.
    if delete_count > 0 {
        state.cmdline.open_confirm(ConfirmAction::CommitWrites {
            table: table_name,
            update_count,
            delete_count,
        });
        return;
    }

    execute_pending_commit(state);
}

/// Build a PendingCommit from the active pane's staged changes and queue it.
fn execute_pending_commit(state: &mut AppState) {
    let Some(dash) = state.dashboard.as_mut() else { return };
    let active_id = dash.tree.active_pane;
    let Some(pane) = dash.tree.panes.get(&active_id) else { return };

    if pane.pending_updates.is_empty() && pane.pending_deletes.is_empty() {
        return;
    }

    let Some(ref table_name) = pane.bound_table else { return };
    let Some(ref loaded) = dash.table_cache.get(table_name) else { return };

    let pk_col = loaded.schema.iter().find(|c| c.is_primary_key);
    let Some(pk_col) = pk_col else {
        state.cmdline.set_error("no primary key found for table");
        return;
    };

    let pk_idx = loaded.schema.iter().position(|c| c.is_primary_key).unwrap_or(0);

    let mut updates = Vec::new();
    for (row, col, new_val) in &pane.pending_updates {
        if *row < loaded.rows.len() && *col < loaded.headers.len() {
            let pk_val = loaded.rows[*row][pk_idx].clone();
            let target_col = loaded.headers[*col].clone();
            updates.push((pk_val, target_col, new_val.clone()));
        }
    }

    let deletes = pane.pending_deletes.clone();

    dash.pending_commit = Some(crate::tui::state::dashboard::PendingCommit {
        table: table_name.clone(),
        pk_col: pk_col.name.clone(),
        updates,
        deletes,
    });
    dash.loading = true;
    dash.error = None;

    // Clear pending state from the pane.
    if let Some(pane) = dash.tree.panes.get_mut(&active_id) {
        pane.pending_updates.clear();
        pane.pending_deletes.clear();
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
        state
            .cmdline
            .set_error(format!("invalid pane id `{}`", args[0]));
        return;
    };

    if closed {
        state.mode = crate::tui::state::AppMode::Home;
        state.dashboard = None;
    }
}
