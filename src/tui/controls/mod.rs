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
        if let Some(table) = dash.pending_load.take() {
            let pool = dash.pool.clone();
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

    Ok(())
}
