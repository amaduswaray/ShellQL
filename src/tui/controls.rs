use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::{
    connection::store::delete_connection,
    tui::{
        state::{AppMode, AppState, Overlay},
        ui::home::{
            goto_bottom, goto_top, remove_selected, select_next, select_prev, selected_connection,
        },
    },
};

pub async fn handle_key_event(event: KeyEvent, state: &mut AppState) -> color_eyre::Result<()> {
    if event.modifiers.contains(KeyModifiers::CONTROL) && event.code == KeyCode::Char('c') {
        state.should_quit = true;
        return Ok(());
    }

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

fn handle_overlay(event: KeyEvent, state: &mut AppState) {
    let Some(overlay) = state.overlay else { return };

    match (overlay, event.code) {
        (Overlay::Help, KeyCode::Char('q') | KeyCode::Char('?') | KeyCode::Esc) => {
            state.overlay = None;
        }

        (Overlay::AddConnection, KeyCode::Esc) => {
            state.overlay = None;
        }

        (Overlay::CommandPalette, KeyCode::Esc) => {
            state.overlay = None;
        }

        (Overlay::ConfirmDelete, KeyCode::Char('y') | KeyCode::Enter) => {
            if let Some(db) = selected_connection(state) {
                let _ = delete_connection(db.name.clone());
            }
            remove_selected(state);
            state.overlay = None;
        }
        (Overlay::ConfirmDelete, KeyCode::Char('n') | KeyCode::Esc) => {
            state.overlay = None;
        }

        _ => {}
    }
}

fn handle_home(event: KeyEvent, state: &mut AppState) {
    match event.code {
        KeyCode::Char('q') => {
            state.should_quit = true;
        }

        KeyCode::Char('j') | KeyCode::Down => {
            select_next(state);
            state.pending_key = None;
        }
        KeyCode::Char('k') | KeyCode::Up => {
            select_prev(state);
            state.pending_key = None;
        }
        KeyCode::Char('G') => {
            goto_bottom(state);
            state.pending_key = None;
        }
        KeyCode::Char('g') => match state.pending_key {
            Some('g') => {
                goto_top(state);
                state.pending_key = None;
            }
            _ => state.pending_key = Some('g'),
        },

        KeyCode::Enter => {
            if selected_connection(state).is_some() {
                state.mode = AppMode::Dashboard;
            }
            state.pending_key = None;
        }
        KeyCode::Char('a') => {
            state.overlay = Some(Overlay::AddConnection);
            state.pending_key = None;
        }
        KeyCode::Char('d') => {
            if selected_connection(state).is_some() {
                state.overlay = Some(Overlay::ConfirmDelete);
            }
            state.pending_key = None;
        }
        KeyCode::Char('?') => {
            state.overlay = Some(Overlay::Help);
            state.pending_key = None;
        }

        _ => state.pending_key = None,
    }
}
