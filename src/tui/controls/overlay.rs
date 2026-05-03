use crossterm::event::{KeyCode, KeyEvent};

use crate::tui::{
    AppMode, AppState, DashboardState, Overlay,
    ui::home::{goto_bottom, goto_top, select_next, select_prev, selected_connection},
};

pub async fn handle_overlay(
    event: KeyEvent,
    state: &mut AppState,
) -> color_eyre::Result<()> {
    let Some(overlay) = state.overlay else {
        return Ok(());
    };

    match (overlay, event.code) {
        // ── Help ──────────────────────────────────────────────────────────────
        (Overlay::Help, KeyCode::Char('q') | KeyCode::Char('?') | KeyCode::Esc) => {
            state.overlay = None;
        }

        // ── AddConnection ─────────────────────────────────────────────────────
        (Overlay::AddConnection, KeyCode::Esc | KeyCode::Char('q')) => {
            state.overlay = None;
        }

        // ── CommandPalette ────────────────────────────────────────────────────
        (Overlay::CommandPalette, KeyCode::Esc | KeyCode::Char('q')) => {
            state.overlay = None;
        }

        // ── ConfirmDelete (handled via cmdline bar — safety fallback) ─────────
        (Overlay::ConfirmDelete, KeyCode::Esc | KeyCode::Char('q')) => {
            state.overlay = None;
        }

        // ── Connection picker — navigate ───────────────────────────────────────
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

        // ── Connection picker — connect ────────────────────────────────────────
        (Overlay::ConnectionPicker, KeyCode::Enter) => {
            state.pending_key = None;

            let Some(db) = selected_connection(state).cloned() else {
                return Ok(());
            };

            // Show a loading signal in the cmdline while connecting.
            state.cmdline.set_error(String::new()); // clears any old error

            // Connect and fetch the table list.
            match connect_and_load(&db).await {
                Ok((pool, tables)) => {
                    state.dashboard = Some(DashboardState::new(db, pool, tables));
                    state.overlay = None;
                    state.mode = AppMode::Dashboard;
                }
                Err(e) => {
                    state.cmdline.set_error(format!("Connection failed: {e}"));
                }
            }
        }

        (Overlay::ConnectionPicker, KeyCode::Esc | KeyCode::Char('q')) => {
            state.overlay = None;
            state.pending_key = None;
        }

        _ => {}
    }

    Ok(())
}

// ── Helpers ───────────────────────────────────────────────────────────────────

async fn connect_and_load(
    db: &crate::connection::Database,
) -> color_eyre::eyre::Result<(crate::connection::DbPool, Vec<String>)> {
    let pool = crate::connection::connect_db(db.connection.clone()).await?;
    let tables = crate::connection::list_tables(&pool).await?;
    Ok((pool, tables))
}
