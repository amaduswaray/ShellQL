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
    // Run after EVERY handler path so that commands like `:open` trigger the
    // load immediately instead of waiting for the next key event.
    if let Some(ref mut dash) = state.dashboard {
        if let Some(query) = dash.pending_load.take() {
            let pool = dash.pool.clone();
            let table = query.table.clone();
            let result = if query.filter.is_some() || query.sort_col.is_some() {
                tokio::try_join!(
                    crate::connection::table_schema(&pool, &table),
                    crate::connection::query_rows(
                        &pool, &table,
                        query.filter.as_deref(),
                        query.sort_col.as_deref(),
                        query.sort_desc,
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
                    use crate::tui::state::dashboard::LoadedTable;
                    dash.table_cache.insert(table.clone(), LoadedTable::new(table, schema, headers, rows));
                    dash.loading = false;
                    state.cmdline.loading = None;
                }
                Err(e) => {
                    dash.error = Some(e.to_string());
                    dash.loading = false;
                }
            }
        }
    }

    // ── Async commit ──────────────────────────────────────────────────────────
    if let Some(ref mut dash) = state.dashboard {
        if let Some(commit) = dash.pending_commit.take() {
            let pool = dash.pool.clone();
            let table = commit.table.clone();
            let pk_col = commit.pk_col.clone();

            let mut success = true;
            let mut err_msg = None;

            // Apply updates.
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

            // Apply deletes.
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
                // Reload the table cache.
                match tokio::try_join!(
                    crate::connection::table_schema(&pool, &table),
                    crate::connection::table_rows(&pool, &table, 200, 0),
                ) {
                    Ok((schema, (headers, rows))) => {
                        use crate::tui::state::dashboard::LoadedTable;
                        dash.table_cache.insert(table.clone(), LoadedTable::new(table, schema, headers, rows));
                        dash.loading = false;
                    }
                    Err(e) => {
                        dash.error = Some(format!("reload after commit failed: {e}"));
                        dash.loading = false;
                    }
                }
            } else {
                dash.error = err_msg;
                dash.loading = false;
            }
        }
    }

    // ── Async query execution ─────────────────────────────────────────────────
    if let Some(ref mut dash) = state.dashboard {
        if let Some(sql) = dash.pending_query_exec.take() {
            let pool = dash.pool.clone();
            match crate::connection::execute_query(&pool, &sql).await {
                Ok((headers, rows)) => {
                    let result = crate::tui::state::dashboard::QueryResult {
                        sql: sql.clone(),
                        headers,
                        rows,
                        error: None,
                    };
                    dash.query_results = vec![result];
                    dash.query_history.push(sql);
                    // Find or create a QueryResults pane.
                    populate_query_results(dash, false);
                    dash.loading = false;
                    state.cmdline.loading = Some(format!("Query executed: {} rows", dash.query_results[0].rows.len()));
                }
                Err(e) => {
                    dash.query_results = vec![crate::tui::state::dashboard::QueryResult {
                        sql,
                        headers: vec![],
                        rows: vec![],
                        error: Some(e.to_string()),
                    }];
                    // On error, only update existing QueryResults panes, don't create new ones.
                    populate_query_results(dash, true);
                    dash.loading = false;
                    state.cmdline.set_error(format!("Query failed: {e}"));
                }
            }
        }
    }

    Ok(())
}

/// Find an existing QueryResults pane or auto-create one via hnew.
/// `error_only` = true means only update existing panes, don't create new ones.
fn populate_query_results(dash: &mut crate::tui::state::DashboardState, error_only: bool) {
    use crate::tui::state::pane_layout::{PaneType, PaneId};

    // Look for an existing QueryResults pane.
    let existing = dash.tree.panes.iter()
        .find(|(_, p)| p.kind == PaneType::QueryResults)
        .map(|(id, _)| *id);

    if let Some(id) = existing {
        if let Some(pane) = dash.tree.panes.get_mut(&id) {
            pane.bound_query_idx = Some(0);
            pane.query_result_count = dash.query_results.len();
        }
    } else if !error_only {
        // Auto-create via hnew below the active pane.
        let active = dash.tree.active_pane;
        if dash.tree.pane_count() < 8 {
            let new_id = PaneId::new();
            let display_id = dash.tree.alloc_display_id();
            dash.tree.panes.insert(new_id, crate::tui::state::pane_layout::Pane::new(new_id, PaneType::QueryResults, display_id));
            if let Some(pane) = dash.tree.panes.get_mut(&new_id) {
                pane.bound_query_idx = Some(0);
                pane.query_result_count = dash.query_results.len();
            }
            dash.tree.replace_leaf_with_split(active, false, new_id);
            dash.tree.active_pane = active; // keep focus on editor
        }
    }
}
