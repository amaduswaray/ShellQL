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

        if !pane.pending_updates.is_empty()
            || !pane.pending_deletes.is_empty()
            || !pane.pending_inserts.is_empty()
        {
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

        if !pane.pending_updates.is_empty()
            || !pane.pending_deletes.is_empty()
            || !pane.pending_inserts.is_empty()
        {
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

        if !pane.pending_updates.is_empty()
            || !pane.pending_deletes.is_empty()
            || !pane.pending_inserts.is_empty()
        {
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

        if !pane.pending_updates.is_empty()
            || !pane.pending_deletes.is_empty()
            || !pane.pending_inserts.is_empty()
        {
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

/// Stage a new row insert in TableView. Defaults to below the cursor.
/// Usage: :insert [above|below]
pub fn cmd_insert(state: &mut AppState, args: &[&str]) {
    let place_above = match args.first().copied() {
        None => false,
        Some(arg)
            if arg.eq_ignore_ascii_case("below")
                || arg.eq_ignore_ascii_case("after")
                || arg.eq_ignore_ascii_case("down") =>
        {
            false
        }
        Some(arg)
            if arg.eq_ignore_ascii_case("above")
                || arg.eq_ignore_ascii_case("before")
                || arg.eq_ignore_ascii_case("up") =>
        {
            true
        }
        Some(_) => {
            state.cmdline.set_error("usage: :insert [above|below]");
            return;
        }
    };

    let Some(tab) = state.active_tab_mut() else {
        state.cmdline.set_error("not in dashboard");
        return;
    };
    let Some(pane) = tab.tree.active_mut() else {
        state.cmdline.set_error("not in dashboard");
        return;
    };
    if pane.kind != crate::tui::state::PaneType::TableView {
        state.cmdline.set_error(":insert only works in table view");
        return;
    }

    if place_above {
        crate::tui::controls::dashboard::modes::stage_insert_row_above(state);
    } else {
        crate::tui::controls::dashboard::modes::stage_insert_row_below(state);
    }
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
            pane.query_scroll_offset = 0;
        }
        tab.pending_query_exec = Some(formatted);
        tab.loading = true;
        tab.error = None;
        return;
    }

    if pane.pending_updates.is_empty()
        && pane.pending_deletes.is_empty()
        && pane.pending_inserts.is_empty()
    {
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
    let insert_count = pane.pending_inserts.len();

    // If there are deletes, ask for confirmation. Otherwise commit immediately.
    if delete_count > 0 {
        state
            .cmdline
            .open_confirm(crate::tui::ConfirmAction::CommitWrites {
                table: table_name,
                update_count,
                delete_count,
                insert_count,
            });
        return;
    }

    execute_pending_commit(state);
}

/// Build a PendingCommit from the active pane's staged changes and queue it.
pub fn execute_pending_commit(state: &mut AppState) {
    let (active_id, table_name, pending_updates, pending_deletes, mut pending_inserts) = {
        let Some(tab) = state.active_tab_mut() else {
            return;
        };
        let active_id = tab.tree.active_pane;
        let Some(pane) = tab.tree.panes.get(&active_id) else {
            return;
        };

        if pane.pending_updates.is_empty()
            && pane.pending_deletes.is_empty()
            && pane.pending_inserts.is_empty()
        {
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
            pane.pending_inserts.clone(),
        )
    };

    let Some(loaded) = state.table_cache.get(&table_name).cloned() else {
        return;
    };

    let needs_pk = !pending_updates.is_empty() || !pending_deletes.is_empty();

    let mut pk_col_name = String::new();
    let mut updates = Vec::new();
    let mut deletes = Vec::new();

    if needs_pk {
        let Some(pk_col) = loaded.schema.iter().find(|c| c.is_primary_key) else {
            state.cmdline.set_error("no primary key found for table");
            return;
        };
        let Some(pk_idx) = loaded.schema.iter().position(|c| c.is_primary_key) else {
            state.cmdline.set_error("no primary key found for table");
            return;
        };

        pk_col_name = pk_col.name.clone();

        for (row, col, new_val) in &pending_updates {
            if *row < loaded.rows.len() && *col < loaded.headers.len() {
                let pk_val = loaded.rows[*row][pk_idx].clone();
                let target_col = loaded.headers[*col].clone();
                updates.push((pk_val, target_col, new_val.clone()));
            }
        }

        deletes = pending_deletes;
    }

    let header_idx: std::collections::HashMap<&str, usize> = loaded
        .headers
        .iter()
        .enumerate()
        .map(|(i, h)| (h.as_str(), i))
        .collect();

    pending_inserts.sort_unstable_by_key(|r| r.position);

    let mut inserts = Vec::new();
    for (insert_order, staged) in pending_inserts.iter().enumerate() {
        let mut missing_required = Vec::new();
        for col in &loaded.schema {
            if !is_required_insert_column(col) {
                continue;
            }
            match header_idx.get(col.name.as_str()).copied() {
                Some(col_idx) => {
                    let val = staged
                        .values
                        .get(col_idx)
                        .map(|s| s.trim())
                        .unwrap_or_default();
                    if val.is_empty() {
                        missing_required.push(col.name.clone());
                    }
                }
                None => missing_required.push(col.name.clone()),
            }
        }

        if !missing_required.is_empty() {
            state.cmdline.set_error(format!(
                "insert row {} missing required values: {}",
                insert_order + 1,
                missing_required.join(", ")
            ));
            return;
        }

        let mut cols = Vec::new();
        let mut vals = Vec::new();
        for (col_idx, col_name) in loaded.headers.iter().enumerate() {
            let val = staged
                .values
                .get(col_idx)
                .map(|s| s.trim())
                .unwrap_or_default();
            if !val.is_empty() {
                cols.push(col_name.clone());
                vals.push(val.to_string());
            }
        }

        if cols.is_empty() {
            state
                .cmdline
                .set_error(format!("insert row {} has no values", insert_order + 1));
            return;
        }

        inserts.push(crate::tui::state::tab::PendingInsert { cols, vals });
    }

    let Some(tab) = state.active_tab_mut() else {
        return;
    };
    tab.pending_commit = Some(crate::tui::state::tab::PendingCommit {
        table: table_name,
        pk_col: pk_col_name,
        updates,
        deletes,
        inserts,
    });
    tab.loading = true;
    tab.error = None;

    // Clear pending state from the pane.
    if let Some(pane) = tab.tree.panes.get_mut(&active_id) {
        pane.pending_updates.clear();
        pane.pending_deletes.clear();
        pane.pending_inserts.clear();
    }
}

fn is_required_insert_column(col: &crate::connection::ColumnInfo) -> bool {
    if col.nullable || col.default_value.is_some() {
        return false;
    }
    true
}
