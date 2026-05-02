pub mod cmdline;
pub mod form;
pub mod home;
pub mod overlay;
use crate::tui::{
    AppMode, AppState,
    controls::{
        cmdline::handle_cmdline, form::handle_form, home::handle_home, overlay::handle_overlay,
    },
};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

// ── Entry point ───────────────────────────────────────────────────────────────
pub async fn handle_key_event(event: KeyEvent, state: &mut AppState) -> color_eyre::Result<()> {
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

    // Overlays intercept the pipeline before mode handlers.
    if state.overlay.is_some() {
        handle_overlay(event, state);
        return Ok(());
    }

    match state.mode {
        AppMode::Home => handle_home(event, state),
        AppMode::Dashboard => {} // TODO: route to dashboard handler
    }

    Ok(())
}
