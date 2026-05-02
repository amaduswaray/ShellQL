use crossterm::event::{KeyCode, KeyEvent};

use crate::tui::{
    AddConnectionForm, AppState, ConfirmAction, Overlay, ui::home::selected_connection,
};

pub fn handle_home(event: KeyEvent, state: &mut AppState) {
    match event.code {
        // ── Quit ──────────────────────────────────────────────────────────────
        KeyCode::Char('q') => state.should_quit = true,

        // ── Open connection picker ─────────────────────────────────────────────
        KeyCode::Char('c') => {
            state.overlay = Some(Overlay::ConnectionPicker);
            state.pending_key = None;
        }

        // ── Actions ───────────────────────────────────────────────────────────
        KeyCode::Char('a') => {
            state.overlay = Some(Overlay::AddConnection);
            state.form = Some(AddConnectionForm::new());
            state.pending_key = None;
        }
        KeyCode::Char('d') => {
            if let Some(db) = selected_connection(state) {
                let name = db.name.clone();
                state
                    .cmdline
                    .open_confirm(ConfirmAction::DeleteConnection(name));
            }
            state.pending_key = None;
        }
        KeyCode::Char(':') => {
            state.cmdline.open_input();
            state.pending_key = None;
        }
        KeyCode::Char('?') => {
            state.overlay = Some(Overlay::Help);
            state.pending_key = None;
        }

        _ => state.pending_key = None,
    }
}
