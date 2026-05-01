use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::tui::state::{AppState, Overlay};

pub async fn handle_key_event(event: KeyEvent, state: &mut AppState) -> color_eyre::Result<()> {
    let code = event.code;
    let modifiers = event.modifiers;

    match (code, modifiers) {
        (KeyCode::Char('c'), KeyModifiers::CONTROL) | (KeyCode::Char('q'), KeyModifiers::NONE) => {
            state.should_quit = true;
        }
        (KeyCode::Char('?'), KeyModifiers::NONE) => {
            state.overlay = match state.overlay {
                Some(Overlay::Help) => None,
                _ => Some(Overlay::Help),
            };
        }
        _ => {}
    }

    Ok(())
}
