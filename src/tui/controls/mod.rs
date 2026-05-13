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

    // Command line has the highest priority when active.
    if state.cmdline.is_active() {
        handle_cmdline(event, state);
        return Ok(());
    }

    // The add-connection form intercepts all keys while open.
    if state.form.is_some() {
        handle_form(event, state).await?;
        return Ok(());
    }

    // Overlays intercept before mode handlers (overlay handler is async
    // because ConnectionPicker::Enter needs to connect + fetch tables).
    if state.overlay.is_some() {
        handle_overlay(event, state).await?;
        return Ok(());
    }

    // Route to the active mode handler.
    match state.mode {
        AppMode::Home => handle_home(event, state),
        AppMode::Dashboard => handle_dashboard(event, state),
    }

    // ── Async table load ──────────────────────────────────────────────────────
    // `handle_dashboard` may have set `pending_load`; execute it here where
    // we can safely await.
    if let Some(ref mut dash) = state.dashboard {
        if let Some(table) = dash.pending_load.take() {
            let pool = dash.pool.clone();
            match tokio::try_join!(
                crate::connection::table_schema(&pool, &table),
                crate::connection::table_rows(&pool, &table, 200),
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

    Ok(())
}
