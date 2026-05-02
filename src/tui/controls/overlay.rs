use crossterm::event::{KeyCode, KeyEvent};

use crate::tui::{
    AppMode, AppState, Overlay,
    ui::home::{goto_bottom, goto_top, select_next, select_prev, selected_connection},
};

pub fn handle_overlay(event: KeyEvent, state: &mut AppState) {
    let Some(overlay) = state.overlay else { return };

    match (overlay, event.code) {
        // Help — q / ? / Esc closes it
        (Overlay::Help, KeyCode::Char('q') | KeyCode::Char('?') | KeyCode::Esc) => {
            state.overlay = None;
        }

        // AddConnection — Esc cancels (form input handled later)
        (Overlay::AddConnection, KeyCode::Esc | KeyCode::Char('q')) => {
            state.overlay = None;
        }

        // CommandPalette — Esc cancels
        (Overlay::CommandPalette, KeyCode::Esc | KeyCode::Char('q')) => {
            state.overlay = None;
        }

        // ConfirmDelete is now handled via the command-line bar, not as an overlay.
        (Overlay::ConfirmDelete, KeyCode::Esc | KeyCode::Char('q')) => {
            state.overlay = None;
        }

        // Connection picker — vim navigation + connect
        (Overlay::ConnectionPicker, KeyCode::Char('j') | KeyCode::Down) => {
            select_next(state);
            state.pending_key = None;
        }
        (Overlay::ConnectionPicker, KeyCode::Char('k') | KeyCode::Up) => {
            select_prev(state);
            state.pending_key = None;
        }
        (Overlay::ConnectionPicker, KeyCode::Char('G')) => {
            goto_bottom(state);
            state.pending_key = None;
        }
        (Overlay::ConnectionPicker, KeyCode::Char('g')) => {
            if state.pending_key == Some('g') {
                goto_top(state);
                state.pending_key = None;
            } else {
                state.pending_key = Some('g');
            }
        }
        (Overlay::ConnectionPicker, KeyCode::Enter) => {
            if selected_connection(state).is_some() {
                state.overlay = None;
                state.mode = AppMode::Dashboard; // TODO: initialise session
            }
            state.pending_key = None;
        }
        (Overlay::ConnectionPicker, KeyCode::Esc | KeyCode::Char('q')) => {
            state.overlay = None;
            state.pending_key = None;
        }

        _ => {}
    }
}
