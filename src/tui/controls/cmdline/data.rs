use crate::tui::AppState;

pub fn cmd_where(state: &mut AppState, args: &[&str]) {
    let table_name = {
        let Some(tab) = state.active_tab_mut() else {
            state.cmdline.set_error("not in dashboard");
            return;
        };

        let active_id = tab.tree.active_pane;
        let Some(pane) = tab.tree.panes.get(&active_id) else {
            return;
        };

        if pane.kind != crate::tui::state::PaneType::TableView {
            state.cmdline.set_error(":where only works in table view");
            return;
        }

        if !pane.pending_updates.is_empty() || !pane.pending_deletes.is_empty() {
            state
                .cmdline
                .set_error("cannot filter with pending changes; :w or u to clear");
            return;
        }

        match pane.bound_table.clone() {
            Some(t) => t,
            None => {
                state.cmdline.set_error("no table bound to active pane");
                return;
            }
        }
    };

    let filter = if args.is_empty() {
        None
    } else {
        Some(args.join(" "))
    };

    let Some(tab) = state.active_tab_mut() else {
        return;
    };
    let active_id = tab.tree.active_pane;
    let Some(pane) = tab.tree.panes.get(&active_id) else {
        return;
    };

    let sort_col = pane.sort_col.clone();
    let sort_desc = pane.sort_desc;
    let selected_cols = pane.selected_cols.clone();

    if let Some(pane) = tab.tree.panes.get_mut(&active_id) {
        pane.filter = filter.clone();
    }

    tab.pending_load = Some(crate::tui::state::tab::PendingQuery {
        table: table_name,
        filter,
        sort_col,
        sort_desc,
        selected_cols,
    });
    tab.loading = true;
    tab.error = None;

    if args.is_empty() {
        state.cmdline.set_loading("Filter cleared");
    }
}

pub fn cmd_order(state: &mut AppState, args: &[&str]) {
    let table_name = {
        let Some(tab) = state.active_tab_mut() else {
            state.cmdline.set_error("not in dashboard");
            return;
        };

        let active_id = tab.tree.active_pane;
        let Some(pane) = tab.tree.panes.get(&active_id) else {
            return;
        };

        if pane.kind != crate::tui::state::PaneType::TableView {
            state.cmdline.set_error(":order only works in table view");
            return;
        }

        if !pane.pending_updates.is_empty() || !pane.pending_deletes.is_empty() {
            state
                .cmdline
                .set_error("cannot sort with pending changes; :w or u to clear");
            return;
        }

        match pane.bound_table.clone() {
            Some(t) => t,
            None => {
                state.cmdline.set_error("no table bound to active pane");
                return;
            }
        }
    };

    let (sort_col, sort_desc) = if args.is_empty() {
        (None, false)
    } else {
        let joined = args.join(" ");
        let parts: Vec<&str> = joined.split_whitespace().collect();
        let desc = parts.len() > 1
            && parts
                .last()
                .map_or(false, |s| s.eq_ignore_ascii_case("desc"));
        let col = if desc {
            parts[..parts.len() - 1].join(" ")
        } else {
            joined
        };
        (Some(col), desc)
    };

    let Some(tab) = state.active_tab_mut() else {
        return;
    };
    let active_id = tab.tree.active_pane;
    let Some(pane) = tab.tree.panes.get(&active_id) else {
        return;
    };

    let filter = pane.filter.clone();
    let selected_cols = pane.selected_cols.clone();

    if let Some(pane) = tab.tree.panes.get_mut(&active_id) {
        pane.sort_col = sort_col.clone();
        pane.sort_desc = sort_desc;
    }

    tab.pending_load = Some(crate::tui::state::tab::PendingQuery {
        table: table_name,
        filter,
        sort_col,
        sort_desc,
        selected_cols,
    });
    tab.loading = true;
    tab.error = None;

    if args.is_empty() {
        state.cmdline.set_loading("Sort cleared");
    }
}

pub fn cmd_select(state: &mut AppState, args: &[&str]) {
    let table_name = {
        let Some(tab) = state.active_tab_mut() else {
            state.cmdline.set_error("not in dashboard");
            return;
        };

        let active_id = tab.tree.active_pane;
        let Some(pane) = tab.tree.panes.get(&active_id) else {
            return;
        };

        if pane.kind != crate::tui::state::PaneType::TableView {
            state.cmdline.set_error(":select only works in table view");
            return;
        }

        if !pane.pending_updates.is_empty() || !pane.pending_deletes.is_empty() {
            state
                .cmdline
                .set_error("cannot select columns with pending changes; :w or u to clear");
            return;
        }

        match pane.bound_table.clone() {
            Some(t) => t,
            None => {
                state.cmdline.set_error("no table bound to active pane");
                return;
            }
        }
    };

    let selected_cols = if args.is_empty() {
        None
    } else {
        let cols: Vec<String> = args.iter().map(|s| s.to_string()).collect();
        // Validate column names against cached schema.
        if let Some(loaded) = state.table_cache.get(&table_name) {
            let valid_cols: std::collections::HashSet<String> =
                loaded.headers.iter().cloned().collect();
            for col in &cols {
                if !valid_cols.contains(col) {
                    state
                        .cmdline
                        .set_error(format!("column '{col}' not found in '{table_name}'"));
                    return;
                }
            }
        }
        Some(cols)
    };

    let Some(tab) = state.active_tab_mut() else {
        return;
    };
    let active_id = tab.tree.active_pane;
    let Some(pane) = tab.tree.panes.get(&active_id) else {
        return;
    };

    let filter = pane.filter.clone();
    let sort_col = pane.sort_col.clone();
    let sort_desc = pane.sort_desc;

    if let Some(pane) = tab.tree.panes.get_mut(&active_id) {
        pane.selected_cols = selected_cols.clone();
    }

    tab.pending_load = Some(crate::tui::state::tab::PendingQuery {
        table: table_name,
        filter,
        sort_col,
        sort_desc,
        selected_cols,
    });
    tab.loading = true;
    tab.error = None;

    if args.is_empty() {
        state.cmdline.set_loading("Column selection cleared");
    }
}

/// Reset all TableView modifiers (filter, sort, selected columns).
pub fn cmd_reset(state: &mut AppState) {
    let table_name = {
        let Some(tab) = state.active_tab_mut() else {
            state.cmdline.set_error("not in dashboard");
            return;
        };

        let active_id = tab.tree.active_pane;
        let Some(pane) = tab.tree.panes.get(&active_id) else {
            return;
        };

        if pane.kind != crate::tui::state::PaneType::TableView {
            state.cmdline.set_error(":reset only works in table view");
            return;
        }

        if !pane.pending_updates.is_empty() || !pane.pending_deletes.is_empty() {
            state
                .cmdline
                .set_error("cannot reset with pending changes; :w or u to clear");
            return;
        }

        match pane.bound_table.clone() {
            Some(t) => t,
            None => {
                state.cmdline.set_error("no table bound to active pane");
                return;
            }
        }
    };

    let Some(tab) = state.active_tab_mut() else {
        return;
    };
    let active_id = tab.tree.active_pane;
    if let Some(pane) = tab.tree.panes.get_mut(&active_id) {
        pane.filter = None;
        pane.sort_col = None;
        pane.sort_desc = false;
        pane.selected_cols = None;
    }

    tab.pending_load = Some(crate::tui::state::tab::PendingQuery {
        table: table_name,
        filter: None,
        sort_col: None,
        sort_desc: false,
        selected_cols: None,
    });
    tab.loading = true;
    tab.error = None;

    state.cmdline.set_loading("Reset cleared");
}

pub fn cmd_write(state: &mut AppState, _args: &[&str]) {
    let Some(tab) = state.active_tab_mut() else {
        state.cmdline.set_error("not in dashboard");
        return;
    };

    let active_id = tab.tree.active_pane;
    let Some(pane) = tab.tree.panes.get(&active_id) else {
        return;
    };

    // If active pane is a QueryEditor, :w formats then executes the query.
    if pane.kind == crate::tui::state::PaneType::QueryEditor {
        let raw_sql = pane.query_text.join("\n");
        if raw_sql.trim().is_empty() {
            state.cmdline.set_error("query is empty");
            return;
        }
        // Format SQL before execution.
        let opts = sqlformat::FormatOptions {
            indent: sqlformat::Indent::Spaces(2),
            uppercase: Some(true),
            ..Default::default()
        };
        let formatted = sqlformat::format(&raw_sql, &sqlformat::QueryParams::None, &opts);
        // Update the editor text with the formatted query.
        if let Some(pane) = tab.tree.panes.get_mut(&active_id) {
            pane.query_text = formatted.lines().map(|s| s.to_string()).collect();
            pane.query_cursor = (0, 0);
        }
        tab.pending_query_exec = Some(formatted);
        tab.loading = true;
        tab.error = None;
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
        state
            .cmdline
            .open_confirm(crate::tui::ConfirmAction::CommitWrites {
                table: table_name,
                update_count,
                delete_count,
            });
        return;
    }

    execute_pending_commit(state);
}

/// Build a PendingCommit from the active pane's staged changes and queue it.
pub fn execute_pending_commit(state: &mut AppState) {
    let (active_id, table_name, pending_updates, pending_deletes) = {
        let Some(tab) = state.active_tab_mut() else {
            return;
        };
        let active_id = tab.tree.active_pane;
        let Some(pane) = tab.tree.panes.get(&active_id) else {
            return;
        };

        if pane.pending_updates.is_empty() && pane.pending_deletes.is_empty() {
            return;
        }

        let Some(ref table_name) = pane.bound_table else {
            return;
        };
        (
            active_id,
            table_name.clone(),
            pane.pending_updates.clone(),
            pane.pending_deletes.clone(),
        )
    };

    let Some(loaded) = state.table_cache.get(&table_name).cloned() else {
        return;
    };

    let pk_col = loaded.schema.iter().find(|c| c.is_primary_key);
    let Some(pk_col) = pk_col else {
        state.cmdline.set_error("no primary key found for table");
        return;
    };

    let pk_idx = loaded
        .schema
        .iter()
        .position(|c| c.is_primary_key)
        .unwrap_or(0);

    let mut updates = Vec::new();
    for (row, col, new_val) in &pending_updates {
        if *row < loaded.rows.len() && *col < loaded.headers.len() {
            let pk_val = loaded.rows[*row][pk_idx].clone();
            let target_col = loaded.headers[*col].clone();
            updates.push((pk_val, target_col, new_val.clone()));
        }
    }

    let deletes = pending_deletes;

    let Some(tab) = state.active_tab_mut() else {
        return;
    };
    tab.pending_commit = Some(crate::tui::state::tab::PendingCommit {
        table: table_name,
        pk_col: pk_col.name.clone(),
        updates,
        deletes,
    });
    tab.loading = true;
    tab.error = None;

    // Clear pending state from the pane.
    if let Some(pane) = tab.tree.panes.get_mut(&active_id) {
        pane.pending_updates.clear();
        pane.pending_deletes.clear();
    }
}
