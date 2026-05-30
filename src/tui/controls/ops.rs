use crate::tui::{state::AppState, state::pane_layout::PaneType};

pub fn maybe_schedule_live_table_refresh(state: &mut AppState) {
    if !state.live_table_refresh_enabled {
        return;
    }
    if state.mode != crate::tui::state::AppMode::Dashboard {
        return;
    }
    if state.pool.is_none()
        || state.cmdline.is_active()
        || state.overlay.is_some()
        || state.form.is_some()
    {
        return;
    }

    let now = std::time::Instant::now();
    if let Some(last) = state.last_live_table_refresh_at {
        if now.duration_since(last) < state.live_table_refresh_interval {
            return;
        }
    }

    let pending = {
        let Some(tab) = state.active_tab() else {
            return;
        };
        if tab.pending_load.is_some()
            || tab.pending_commit.is_some()
            || tab.pending_query_exec.is_some()
        {
            return;
        }

        let active_id = tab.tree.active_pane;
        let Some(pane) = tab.tree.panes.get(&active_id) else {
            return;
        };
        if pane.kind != PaneType::TableView {
            return;
        }
        if !pane.pending_updates.is_empty()
            || !pane.pending_deletes.is_empty()
            || !pane.pending_inserts.is_empty()
        {
            return;
        }
        let Some(table) = pane.bound_table.clone() else {
            return;
        };

        crate::tui::state::tab::PendingQuery {
            table,
            filter: pane.filter.clone(),
            sort_col: pane.sort_col.clone(),
            sort_desc: pane.sort_desc,
            selected_cols: pane.selected_cols.clone(),
        }
    };

    if let Some(tab) = state.active_tab_mut() {
        tab.pending_load = Some(pending);
        // Avoid "Loading…" flicker for background refresh.
        tab.loading = false;
    }
    state.last_live_table_refresh_at = Some(now);
}

pub async fn run_pending_tasks(state: &mut AppState) -> color_eyre::Result<()> {
    run_pending_load(state).await?;
    run_pending_commit(state).await?;
    run_pending_query(state).await?;
    Ok(())
}

async fn run_pending_load(state: &mut AppState) -> color_eyre::Result<()> {
    let Some(pool) = state.pool.clone() else {
        return Ok(());
    };
    let table_cache = &mut state.table_cache;
    let active = state.active_tab;
    let Some(tab) = state.tabs.get_mut(active) else {
        return Ok(());
    };
    if let Some(query) = tab.pending_load.take() {
        let table = query.table.clone();
        let result = if query.filter.is_some()
            || query.sort_col.is_some()
            || query.selected_cols.is_some()
        {
            tokio::try_join!(
                crate::connection::table_schema(&pool, &table),
                crate::connection::query_rows(
                    &pool,
                    &table,
                    query.filter.as_deref(),
                    query.sort_col.as_deref(),
                    query.sort_desc,
                    query.selected_cols.as_deref(),
                    200,
                    0,
                ),
            )
        } else {
            tokio::try_join!(
                crate::connection::table_schema(&pool, &table),
                crate::connection::table_rows(&pool, &table, 200, 0),
            )
        };
        match result {
            Ok((schema, (headers, rows))) => {
                use crate::tui::state::tab::LoadedTable;
                table_cache.insert(
                    table.clone(),
                    LoadedTable::new(table, schema, headers, rows),
                );
                tab.loading = false;
                state.cmdline.loading = None;
            }
            Err(e) => {
                tab.error = Some(e.to_string());
                tab.loading = false;
            }
        }
    }
    Ok(())
}

async fn run_pending_commit(state: &mut AppState) -> color_eyre::Result<()> {
    let Some(pool) = state.pool.clone() else {
        return Ok(());
    };
    let table_cache = &mut state.table_cache;
    let active = state.active_tab;
    let Some(tab) = state.tabs.get_mut(active) else {
        return Ok(());
    };
    if let Some(commit) = tab.pending_commit.take() {
        let table = commit.table.clone();
        let pk_col = commit.pk_col.clone();

        let mut success = true;
        let mut err_msg = None;

        for (pk_val, target_col, new_val) in &commit.updates {
            match crate::connection::update_cell(
                &pool, &table, &pk_col, pk_val, target_col, new_val,
            )
            .await
            {
                Ok(_) => {}
                Err(e) => {
                    success = false;
                    err_msg = Some(format!("update failed: {e}"));
                    break;
                }
            }
        }

        if success && !commit.deletes.is_empty() {
            match crate::connection::delete_rows(&pool, &table, &pk_col, &commit.deletes).await {
                Ok(_) => {}
                Err(e) => {
                    success = false;
                    err_msg = Some(format!("delete failed: {e}"));
                }
            }
        }

        if success {
            for insert in &commit.inserts {
                match crate::connection::insert_row(&pool, &table, &insert.cols, &insert.vals).await
                {
                    Ok(_) => {}
                    Err(e) => {
                        success = false;
                        err_msg = Some(format!("insert failed: {e}"));
                        break;
                    }
                }
            }
        }

        if success {
            match tokio::try_join!(
                crate::connection::table_schema(&pool, &table),
                crate::connection::table_rows(&pool, &table, 200, 0),
            ) {
                Ok((schema, (headers, rows))) => {
                    use crate::tui::state::tab::LoadedTable;
                    table_cache.insert(
                        table.clone(),
                        LoadedTable::new(table, schema, headers, rows),
                    );
                    tab.loading = false;
                }
                Err(e) => {
                    tab.error = Some(format!("reload after commit failed: {e}"));
                    tab.loading = false;
                }
            }
        } else {
            tab.error = err_msg;
            tab.loading = false;
        }
    }
    Ok(())
}

async fn run_pending_query(state: &mut AppState) -> color_eyre::Result<()> {
    let Some(pool) = state.pool.clone() else {
        return Ok(());
    };
    let active = state.active_tab;

    let mut success_msg: Option<String> = None;
    let mut error_msg: Option<String> = None;

    let Some(tab) = state.tabs.get_mut(active) else {
        return Ok(());
    };

    if let Some(sql) = tab.pending_query_exec.take() {
        let statements = split_sql_statements(&sql);
        if statements.is_empty() {
            tab.loading = false;
            error_msg = Some("query is empty".to_string());
        } else {
            let mut results = Vec::with_capacity(statements.len());
            let mut first_error: Option<(usize, String)> = None;

            for (i, stmt) in statements.iter().enumerate() {
                match crate::connection::execute_query(&pool, stmt).await {
                    Ok((headers, rows)) => {
                        results.push(crate::tui::state::tab::QueryResult {
                            sql: stmt.clone(),
                            headers,
                            rows,
                            error: None,
                        });
                    }
                    Err(e) => {
                        let msg = e.to_string();
                        results.push(crate::tui::state::tab::QueryResult {
                            sql: stmt.clone(),
                            headers: vec![],
                            rows: vec![],
                            error: Some(msg.clone()),
                        });
                        first_error = Some((i, msg));
                        break;
                    }
                }
            }

            tab.query_results = results;
            tab.loading = false;

            let has_success = tab.query_results.iter().any(|r| r.error.is_none());
            let only_errors = !tab.query_results.is_empty() && !has_success;
            populate_query_results(tab, only_errors);

            if has_success {
                tab.query_history.push(sql);
            }

            if let Some((failed_idx, err)) = first_error {
                if failed_idx == 0 {
                    error_msg = Some(format!("Query failed: {err}"));
                } else {
                    error_msg = Some(format!(
                        "statement {} failed after {} successful result set(s): {}",
                        failed_idx + 1,
                        failed_idx,
                        err
                    ));
                }
            } else {
                success_msg = Some(query_execution_status_message(&tab.query_results));
            }
        }
    }

    if let Some(msg) = success_msg {
        state.cmdline.loading = Some(msg);
    }
    if let Some(msg) = error_msg {
        state.cmdline.set_error(msg);
    }

    Ok(())
}

fn query_execution_status_message(results: &[crate::tui::state::tab::QueryResult]) -> String {
    if results.is_empty() {
        return "Query returned no rows".to_string();
    }

    if results.len() > 1 {
        return format!("Query executed: {} result sets", results.len());
    }

    let result = &results[0];
    if result.headers == ["Rows Affected"] {
        let affected = result
            .rows
            .first()
            .and_then(|r| r.first())
            .cloned()
            .unwrap_or_else(|| "0".to_string());
        format!("{affected} rows affected")
    } else if result.headers.is_empty() {
        "Query returned no rows".to_string()
    } else {
        format!("Query executed: {} rows", result.rows.len())
    }
}

fn split_sql_statements(sql: &str) -> Vec<String> {
    let mut statements = Vec::new();
    let mut current = String::new();

    let chars: Vec<char> = sql.chars().collect();
    let mut i = 0usize;

    let mut in_single = false;
    let mut in_double = false;
    let mut in_line_comment = false;
    let mut in_block_comment = false;

    while i < chars.len() {
        let c = chars[i];
        let next = chars.get(i + 1).copied();

        if in_line_comment {
            current.push(c);
            if c == '\n' {
                in_line_comment = false;
            }
            i += 1;
            continue;
        }

        if in_block_comment {
            current.push(c);
            if c == '*' && next == Some('/') {
                current.push('/');
                i += 2;
                in_block_comment = false;
            } else {
                i += 1;
            }
            continue;
        }

        if in_single {
            current.push(c);
            if c == '\'' {
                if next == Some('\'') {
                    current.push('\'');
                    i += 2;
                    continue;
                }
                in_single = false;
            }
            i += 1;
            continue;
        }

        if in_double {
            current.push(c);
            if c == '"' {
                if next == Some('"') {
                    current.push('"');
                    i += 2;
                    continue;
                }
                in_double = false;
            }
            i += 1;
            continue;
        }

        if c == '-' && next == Some('-') {
            current.push('-');
            current.push('-');
            in_line_comment = true;
            i += 2;
            continue;
        }

        if c == '/' && next == Some('*') {
            current.push('/');
            current.push('*');
            in_block_comment = true;
            i += 2;
            continue;
        }

        if c == '\'' {
            current.push(c);
            in_single = true;
            i += 1;
            continue;
        }

        if c == '"' {
            current.push(c);
            in_double = true;
            i += 1;
            continue;
        }

        if c == ';' {
            let stmt = current.trim();
            if !stmt.is_empty() {
                statements.push(stmt.to_string());
            }
            current.clear();
            i += 1;
            continue;
        }

        current.push(c);
        i += 1;
    }

    let tail = current.trim();
    if !tail.is_empty() {
        statements.push(tail.to_string());
    }

    statements
}

fn populate_query_results(tab: &mut crate::tui::state::Tab, error_only: bool) {
    use crate::tui::state::pane_layout::{PaneId, PaneType};

    let existing = tab
        .tree
        .panes
        .iter()
        .find(|(_, p)| p.kind == PaneType::QueryResults)
        .map(|(id, _)| *id);

    if let Some(id) = existing {
        if let Some(pane) = tab.tree.panes.get_mut(&id) {
            pane.bound_query_idx = Some(0);
            pane.query_result_count = tab.query_results.len();
        }
    } else if !error_only {
        let active = tab.tree.active_pane;
        if tab.tree.pane_count() < 8 {
            let new_id = PaneId::new();
            let display_id = tab.tree.alloc_display_id();
            tab.tree.panes.insert(
                new_id,
                crate::tui::state::pane_layout::Pane::new(
                    new_id,
                    PaneType::QueryResults,
                    display_id,
                ),
            );
            if let Some(pane) = tab.tree.panes.get_mut(&new_id) {
                pane.bound_query_idx = Some(0);
                pane.query_result_count = tab.query_results.len();
            }
            tab.tree.replace_leaf_with_split(active, false, new_id);
            tab.tree.active_pane = active;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::split_sql_statements;

    #[test]
    fn split_sql_statements_splits_basic_multi_select() {
        let sql = "SELECT 1; SELECT 2;";
        let stmts = split_sql_statements(sql);
        assert_eq!(stmts, vec!["SELECT 1", "SELECT 2"]);
    }

    #[test]
    fn split_sql_statements_ignores_semicolons_in_strings() {
        let sql = "SELECT ';' AS s; SELECT 'x;y' AS t;";
        let stmts = split_sql_statements(sql);
        assert_eq!(stmts, vec!["SELECT ';' AS s", "SELECT 'x;y' AS t"]);
    }

    #[test]
    fn split_sql_statements_ignores_semicolons_in_comments() {
        let sql = "-- a;\nSELECT 1; /* b; */ SELECT 2;";
        let stmts = split_sql_statements(sql);
        assert_eq!(stmts, vec!["-- a;\nSELECT 1", "/* b; */ SELECT 2"]);
    }
}
