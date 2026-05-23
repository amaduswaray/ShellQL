pub mod cmdline;
pub mod dashboard;
pub mod form;
pub mod home;
pub mod ops;
pub mod overlay;

use crate::tui::{
    AppMode, AppState,
    controls::{
        cmdline::handle_cmdline, dashboard::handle_dashboard, form::handle_form, home::handle_home,
        overlay::handle_overlay,
    },
};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

// ── Entry point ───────────────────────────────────────────────────────────────

pub async fn handle_key_event(event: KeyEvent, state: &mut AppState) -> color_eyre::Result<()> {
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

    ops::run_pending_tasks(state).await?;

    Ok(())
}
