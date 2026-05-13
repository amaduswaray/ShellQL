use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::tui::{AppState, state::TableMode, state::pane_layout::{PaneDirection, PaneType}};

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
            return;
        }
        // Fallback for terminals that report Shift+; as ';' with SHIFT modifier
        KeyCode::Char(';') if event.modifiers.contains(KeyModifiers::SHIFT) => {
            state.cmdline.open_input();
            state.pending_key = None;
            return;
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
                        let bound = pane.bound_table.as_ref()
                            .and_then(|name| dash.table_cache.get(name))
                            .map(|lt| lt.rows.len())
                            .unwrap_or(0);
                        pane.row_next(bound);
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
                        let bound = pane.bound_table.as_ref()
                            .and_then(|name| dash.table_cache.get(name))
                            .map(|lt| lt.rows.len())
                            .unwrap_or(0);
                        if bound > 0 { pane.row_prev(); }
                    }
                    _ => {}
                }
            }
        }
        KeyCode::Char('h') | KeyCode::Left => {
            if let Some(pane) = dash.tree.active_mut() {
                if pane.kind == PaneType::TableView && pane.cursor_col > 0 {
                    pane.col_left();
                }
            }
        }
        KeyCode::Char('l') | KeyCode::Right => {
            if let Some(pane) = dash.tree.active_mut() {
                if pane.kind == PaneType::TableView {
                    let bound = pane.bound_table.as_ref()
                        .and_then(|name| dash.table_cache.get(name))
                        .map(|lt| lt.headers.len())
                        .unwrap_or(0);
                    pane.col_right(bound);
                }
            }
        }

        // ── Jump to bottom / top ───────────────────────────────────────────────
        KeyCode::Char('G') => {
            if let Some(pane) = dash.tree.active_mut() {
                match pane.kind {
                    PaneType::TableList => pane.nav_bottom(dash.tables.len()),
                    PaneType::TableView => {
                        let bound = pane.bound_table.as_ref()
                            .and_then(|name| dash.table_cache.get(name))
                            .map(|lt| lt.rows.len())
                            .unwrap_or(0);
                        pane.row_bottom(bound);
                    }
                    _ => {}
                }
            }
        }
        KeyCode::Char('g') => {
            state.pending_key = Some('g');
        }

        // ── Enter — select table or load into current pane ─────────────────────
        KeyCode::Enter => {
            if let Some(pane) = dash.tree.active_mut() {
                if pane.kind == PaneType::TableList && !dash.loading {
                    if let Some(name) = dash.tables.get(pane.nav_cursor).cloned() {
                        // Convert the active pane to a TableView bound to this table.
                        pane.set_table_view(name.clone());
                        // If not cached, trigger an async load.
                        if !dash.table_cache.contains_key(&name) {
                            dash.pending_load = Some(name);
                            dash.loading = true;
                            dash.error = None;
                        }
                    }
                }
            }
        }

        _ => {}
    }
}
