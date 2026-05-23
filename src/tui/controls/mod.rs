pub mod cmdline;
pub mod dashboard;
pub mod form;
pub mod home;
pub mod overlay;

use crate::tui::{
    AppMode, AppState,
    controls::{
        cmdline::handle_cmdline,
        dashboard::handle_dashboard,
        form::handle_form,
        home::handle_home,
        overlay::handle_overlay,
    },
};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

// ── Entry point ───────────────────────────────────────────────────────────────

pub async fn handle_key_event(
    event: KeyEvent,
    state: &mut AppState,
) -> color_eyre::Result<()> {
    // Ctrl+C — hard exit regardless of mode or overlay.
    if event.modifiers.contains(KeyModifiers::CONTROL) && event.code == KeyCode::Char('c') {
        state.should_quit = true;
        return Ok(());
    }

    // Dispatch to the appropriate handler (cmdline has highest priority).
    if state.cmdline.is_active() {
        handle_cmdline(event, state);
    } else if state.form.is_some() {
        handle_form(event, state).await?;
    } else if state.overlay.is_some() {
        handle_overlay(event, state).await?;
    } else {
        match state.mode {
            AppMode::Home => handle_home(event, state),
            AppMode::Dashboard => handle_dashboard(event, state),
        }
    }

    // ── Async table load ──────────────────────────────────────────────────────
    {
        let Some(pool) = state.pool.clone() else { return Ok(()); };
        let table_cache = &mut state.table_cache;
        let active = state.active_tab;
        let Some(tab) = state.tabs.get_mut(active) else { return Ok(()); };
        if let Some(query) = tab.pending_load.take() {
            let table = query.table.clone();
            let result = if query.filter.is_some() || query.sort_col.is_some() || query.selected_cols.is_some() {
                tokio::try_join!(
                    crate::connection::table_schema(&pool, &table),
                    crate::connection::query_rows(
                        &pool, &table,
                        query.filter.as_deref(),
                        query.sort_col.as_deref(),
                        query.sort_desc,
                        query.selected_cols.as_deref(),
                        200, 0,
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
                    table_cache.insert(table.clone(), LoadedTable::new(table, schema, headers, rows));
                    tab.loading = false;
                    state.cmdline.loading = None;
                }
                Err(e) => {
                    tab.error = Some(e.to_string());
                    tab.loading = false;
                }
            }
        }
    }

    // ── Async commit ──────────────────────────────────────────────────────────
    {
        let Some(pool) = state.pool.clone() else { return Ok(()); };
        let table_cache = &mut state.table_cache;
        let active = state.active_tab;
        let Some(tab) = state.tabs.get_mut(active) else { return Ok(()); };
        if let Some(commit) = tab.pending_commit.take() {
            let table = commit.table.clone();
            let pk_col = commit.pk_col.clone();

            let mut success = true;
            let mut err_msg = None;

            for (pk_val, target_col, new_val) in &commit.updates {
                match crate::connection::update_cell(
                    &pool, &table, &pk_col, pk_val, target_col, new_val,
                ).await {
                    Ok(_) => {}
                    Err(e) => {
                        success = false;
                        err_msg = Some(format!("update failed: {e}"));
                        break;
                    }
                }
            }

            if success && !commit.deletes.is_empty() {
                match crate::connection::delete_rows(
                    &pool, &table, &pk_col, &commit.deletes,
                ).await {
                    Ok(_) => {}
                    Err(e) => {
                        success = false;
                        err_msg = Some(format!("delete failed: {e}"));
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
                        table_cache.insert(table.clone(), LoadedTable::new(table, schema, headers, rows));
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
    }

    // ── Async query execution ─────────────────────────────────────────────────
    {
        let Some(pool) = state.pool.clone() else { return Ok(()); };
        let active = state.active_tab;
        let Some(tab) = state.tabs.get_mut(active) else { return Ok(()); };
        if let Some(sql) = tab.pending_query_exec.take() {
            match crate::connection::execute_query(&pool, &sql).await {
                Ok((headers, rows)) => {
                    let result = crate::tui::state::tab::QueryResult {
                        sql: sql.clone(),
                        headers,
                        rows,
                        error: None,
                    };
                    tab.query_results = vec![result];
                    tab.query_history.push(sql);
                    populate_query_results(tab, false);
                    tab.loading = false;

                    let msg = if tab.query_results[0].headers == vec!["Rows Affected"] {
                        let affected = tab.query_results[0]
                            .rows
                            .first()
                            .and_then(|r| r.first())
                            .cloned()
                            .unwrap_or_else(|| "0".to_string());
                        format!("{affected} rows affected")
                    } else if tab.query_results[0].headers.is_empty() {
                        "Query returned no rows".to_string()
                    } else {
                        format!("Query executed: {} rows", tab.query_results[0].rows.len())
                    };
                    state.cmdline.loading = Some(msg);
                }
                Err(e) => {
                    tab.query_results = vec![crate::tui::state::tab::QueryResult {
                        sql,
                        headers: vec![],
                        rows: vec![],
                        error: Some(e.to_string()),
                    }];
                    populate_query_results(tab, true);
                    tab.loading = false;
                    state.cmdline.set_error(format!("Query failed: {e}"));
                }
            }
        }
    }

    Ok(())
}

fn populate_query_results(tab: &mut crate::tui::state::Tab, error_only: bool) {
    use crate::tui::state::pane_layout::{PaneType, PaneId};

    let existing = tab.tree.panes.iter()
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
            tab.tree.panes.insert(new_id, crate::tui::state::pane_layout::Pane::new(new_id, PaneType::QueryResults, display_id));
            if let Some(pane) = tab.tree.panes.get_mut(&new_id) {
                pane.bound_query_idx = Some(0);
                pane.query_result_count = tab.query_results.len();
            }
            tab.tree.replace_leaf_with_split(active, false, new_id);
            tab.tree.active_pane = active;
        }
    }
}
