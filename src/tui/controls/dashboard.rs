use crossterm::event::{KeyCode, KeyEvent};

use crate::tui::{AppMode, AppState, state::dashboard::ActivePane};

pub fn handle_dashboard(event: KeyEvent, state: &mut AppState) {
    let Some(ref mut dash) = state.dashboard else { return };

    // Handle pending 'g' for 'gg' sequence first.
    if let Some('g') = state.pending_key {
        state.pending_key = None;
        if event.code == KeyCode::Char('g') {
            match dash.active_pane {
                ActivePane::Nav => dash.nav_top(),
                ActivePane::Table => {
                    if let Some(ref mut t) = dash.loaded {
                        t.row_top();
                    }
                }
            }
            return;
        }
        // If the second key wasn't 'g', fall through to normal handling below.
    }

    match event.code {
        // ── Quit back to home ──────────────────────────────────────────────────
        KeyCode::Char('q') => {
            state.mode = AppMode::Home;
            state.dashboard = None;
        }

        // ── Pane focus ─────────────────────────────────────────────────────────
        KeyCode::Tab => dash.pane_next(),
        KeyCode::BackTab => dash.pane_prev(),

        // ── Mode switching (table only) ────────────────────────────────────────
        KeyCode::Char('v')
            if dash.active_pane == ActivePane::Table
                && let Some(ref mut t) = dash.loaded =>
        {
            t.enter_visual_row();
        }
        KeyCode::Char('V')
            if dash.active_pane == ActivePane::Table
                && let Some(ref mut t) = dash.loaded =>
        {
            t.enter_visual_column();
        }
        KeyCode::Char('i')
            if dash.active_pane == ActivePane::Table
                && let Some(ref mut t) = dash.loaded =>
        {
            t.enter_insert();
        }
        KeyCode::Esc
            if dash.active_pane == ActivePane::Table
                && let Some(ref mut t) = dash.loaded =>
        {
            t.enter_normal();
        }

        // ── Navigation ─────────────────────────────────────────────────────────
        KeyCode::Char('j') | KeyCode::Down => match dash.active_pane {
            ActivePane::Nav => dash.nav_next(),
            ActivePane::Table => {
                if let Some(ref mut t) = dash.loaded {
                    t.row_next();
                }
            }
        },
        KeyCode::Char('k') | KeyCode::Up => match dash.active_pane {
            ActivePane::Nav => dash.nav_prev(),
            ActivePane::Table => {
                if let Some(ref mut t) = dash.loaded {
                    t.row_prev();
                }
            }
        },
        KeyCode::Char('h') | KeyCode::Left => match dash.active_pane {
            ActivePane::Nav => {}
            ActivePane::Table => {
                if let Some(ref mut t) = dash.loaded {
                    if t.cursor_col > 0 {
                        t.col_left();
                    } else {
                        dash.pane_prev();
                    }
                }
            }
        },
        KeyCode::Char('l') | KeyCode::Right => match dash.active_pane {
            ActivePane::Nav => dash.pane_next(),
            ActivePane::Table => {
                if let Some(ref mut t) = dash.loaded {
                    t.col_right();
                }
            }
        },

        // ── Jump to bottom / top ───────────────────────────────────────────────
        KeyCode::Char('G') => match dash.active_pane {
            ActivePane::Nav => dash.nav_bottom(),
            ActivePane::Table => {
                if let Some(ref mut t) = dash.loaded {
                    t.row_bottom();
                }
            }
        },
        KeyCode::Char('g') => {
            state.pending_key = Some('g');
        }

        // ── Enter — load selected nav table ────────────────────────────────────
        KeyCode::Enter if dash.active_pane == ActivePane::Nav && !dash.loading => {
            dash.request_load();
        }

        _ => {}
    }
}
