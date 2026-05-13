use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::tui::{AppMode, AppState, state::TableMode, state::pane_layout::{PaneDirection, PaneType}};

pub fn handle_dashboard(event: KeyEvent, state: &mut AppState) {
    let Some(ref mut dash) = state.dashboard else { return };

    // Ctrl+hjkl / Ctrl+Arrows — pane navigation
    if event.modifiers.contains(KeyModifiers::CONTROL) {
        match event.code {
            KeyCode::Char('h') | KeyCode::Left => {
                dash.tree.navigate(PaneDirection::Left);
                return;
            }
            KeyCode::Char('j') | KeyCode::Down => {
                dash.tree.navigate(PaneDirection::Down);
                return;
            }
            KeyCode::Char('k') | KeyCode::Up => {
                dash.tree.navigate(PaneDirection::Up);
                return;
            }
            KeyCode::Char('l') | KeyCode::Right => {
                dash.tree.navigate(PaneDirection::Right);
                return;
            }
            _ => {}
        }
    }

    // Handle pending 'g' for 'gg' sequence first.
    if let Some('g') = state.pending_key {
        state.pending_key = None;
        if event.code == KeyCode::Char('g') {
            if let Some(pane) = dash.tree.active_mut() {
                match pane.kind {
                    PaneType::TableList => pane.nav_top(),
                    PaneType::TableView => pane.row_top(),
                    _ => {}
                }
            }
            return;
        }
    }

    match event.code {
        // ── Command line ───────────────────────────────────────────────────────
        KeyCode::Char(':') => {
            state.cmdline.open_input();
            state.pending_key = None;
        }

        // ── Quit back to home ──────────────────────────────────────────────────
        KeyCode::Char('q') => {
            state.mode = AppMode::Home;
            state.dashboard = None;
        }

        // ── Mode switching ─────────────────────────────────────────────────────
        KeyCode::Char('v') => {
            if let Some(pane) = dash.tree.active_mut() {
                if pane.kind == PaneType::TableView {
                    pane.mode = TableMode::VisualRow;
                }
            }
        }
        KeyCode::Char('V') => {
            if let Some(pane) = dash.tree.active_mut() {
                if pane.kind == PaneType::TableView {
                    pane.mode = TableMode::VisualColumn;
                }
            }
        }
        KeyCode::Char('i') => {
            if let Some(pane) = dash.tree.active_mut() {
                if pane.kind == PaneType::TableView {
                    pane.mode = TableMode::Insert;
                }
            }
        }
        KeyCode::Esc => {
            if let Some(pane) = dash.tree.active_mut() {
                if pane.kind == PaneType::TableView {
                    pane.mode = TableMode::Normal;
                }
            }
        }

        // ── Navigation ─────────────────────────────────────────────────────────
        KeyCode::Char('j') | KeyCode::Down => {
            if let Some(pane) = dash.tree.active_mut() {
                match pane.kind {
                    PaneType::TableList => pane.nav_next(dash.tables.len()),
                    PaneType::TableView => {
                        if let Some(ref _loaded) = dash.loaded {
                            pane.row_next(dash.loaded.as_ref().unwrap().rows.len());
                        }
                    }
                    _ => {}
                }
            }
        }
        KeyCode::Char('k') | KeyCode::Up => {
            if let Some(pane) = dash.tree.active_mut() {
                match pane.kind {
                    PaneType::TableList => pane.nav_prev(),
                    PaneType::TableView => {
                        if dash.loaded.is_some() {
                            pane.row_prev();
                        }
                    }
                    _ => {}
                }
            }
        }
        KeyCode::Char('h') | KeyCode::Left => {
            if let Some(pane) = dash.tree.active_mut() {
                match pane.kind {
                    PaneType::TableView => {
                        if pane.cursor_col > 0 {
                            pane.col_left();
                        }
                    }
                    _ => {}
                }
            }
        }
        KeyCode::Char('l') | KeyCode::Right => {
            if let Some(pane) = dash.tree.active_mut() {
                match pane.kind {
                    PaneType::TableView => {
                        if let Some(ref loaded) = dash.loaded {
                            pane.col_right(loaded.headers.len());
                        }
                    }
                    _ => {}
                }
            }
        }

        // ── Jump to bottom / top ───────────────────────────────────────────────
        KeyCode::Char('G') => {
            if let Some(pane) = dash.tree.active_mut() {
                match pane.kind {
                    PaneType::TableList => pane.nav_bottom(dash.tables.len()),
                    PaneType::TableView => {
                        if let Some(ref loaded) = dash.loaded {
                            pane.row_bottom(loaded.rows.len());
                        }
                    }
                    _ => {}
                }
            }
        }
        KeyCode::Char('g') => {
            state.pending_key = Some('g');
        }

        // ── Enter — load selected table ────────────────────────────────────────
        KeyCode::Enter => {
            if let Some(pane) = dash.tree.active() {
                if pane.kind == PaneType::TableList && !dash.loading {
                    dash.request_load();
                }
            }
        }

        _ => {}
    }
}

