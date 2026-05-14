use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::tui::{
    AppState, SearchDirection,
    state::TableMode,
    state::pane_layout::{PaneDirection, PaneType},
};

pub fn handle_dashboard(event: KeyEvent, state: &mut AppState) {
    let Some(ref mut dash) = state.dashboard else {
        return;
    };

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
            KeyCode::Char('u') => {
                // Half-page scroll up
                if let Some(pane) = dash.tree.active_mut() {
                    if pane.kind == PaneType::TableView {
                        let viewport = pane.area.map_or(10, |a| (a.height / 2).max(1) as usize);
                        for _ in 0..viewport {
                            pane.row_prev();
                        }
                    } else if pane.kind == PaneType::TableList {
                        let viewport = pane.area.map_or(10, |a| (a.height / 2).max(1) as usize);
                        for _ in 0..viewport {
                            pane.nav_prev();
                        }
                    } else if pane.kind == PaneType::SchemaView {
                        let viewport = pane.area.map_or(3, |a| (a.height / 6).max(1) as usize);
                        for _ in 0..viewport {
                            pane.nav_prev();
                        }
                    }
                }
                return;
            }
            KeyCode::Char('d') => {
                // Half-page scroll down
                if let Some(pane) = dash.tree.active_mut() {
                    if pane.kind == PaneType::TableView {
                        let bound = pane
                            .bound_table
                            .as_ref()
                            .and_then(|name| dash.table_cache.get(name))
                            .map(|lt| lt.rows.len())
                            .unwrap_or(0);
                        let viewport = pane.area.map_or(10, |a| (a.height / 2).max(1) as usize);
                        for _ in 0..viewport {
                            pane.row_next(bound);
                        }
                    } else if pane.kind == PaneType::TableList {
                        let viewport = pane.area.map_or(10, |a| (a.height / 2).max(1) as usize);
                        for _ in 0..viewport {
                            pane.nav_next(dash.tables.len());
                        }
                    } else if pane.kind == PaneType::SchemaView {
                        let bound = pane
                            .bound_table
                            .as_ref()
                            .and_then(|name| dash.table_cache.get(name))
                            .map(|lt| lt.schema.len())
                            .unwrap_or(0);
                        let viewport = pane.area.map_or(3, |a| (a.height / 6).max(1) as usize);
                        for _ in 0..viewport {
                            pane.nav_next(bound);
                        }
                    }
                }
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
                    PaneType::SchemaView => pane.nav_top(),
                    _ => {}
                }
            }
            return;
        }
    }

    // Handle pending 'd' for 'dd' sequence first.
    if let Some('d') = state.pending_key {
        state.pending_key = None;
        if event.code == KeyCode::Char('d') {
            if let Some(pane) = dash.tree.active_mut() {
                if pane.kind == PaneType::TableView && pane.mode == TableMode::Normal {
                    if let Some(ref table_name) = pane.bound_table {
                        if let Some(ref loaded) = dash.table_cache.get(table_name) {
                            let row = pane.row_cursor;
                            if row < loaded.rows.len() {
                                if let Some(pk_idx) = loaded.schema.iter().position(|c| c.is_primary_key) {
                                    let pk_val = loaded.rows[row][pk_idx].clone();
                                    if !pane.pending_deletes.contains(&pk_val) {
                                        pane.pending_deletes.push(pk_val);
                                    }
                                }
                            }
                        }
                    }
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

        // ── Search ─────────────────────────────────────────────────────────────
        KeyCode::Char('/') => {
            state.cmdline.open_search(SearchDirection::Forward);
            state.pending_key = None;
            return;
        }
        KeyCode::Char('?') => {
            state.cmdline.open_search(SearchDirection::Backward);
            state.pending_key = None;
            return;
        }
        KeyCode::Char('n') => {
            if let Some(pane) = dash.tree.active_mut() {
                if let Some(ref mut search) = pane.last_search {
                    if !search.matches.is_empty() {
                        match search.direction {
                            SearchDirection::Forward => {
                                search.current_idx =
                                    (search.current_idx + 1) % search.matches.len();
                            }
                            SearchDirection::Backward => {
                                search.current_idx = (search.current_idx + search.matches.len()
                                    - 1)
                                    % search.matches.len();
                            }
                        }
                        pane.nav_cursor = search.matches[search.current_idx];
                    }
                }
            }
        }
        KeyCode::Char('N') => {
            if let Some(pane) = dash.tree.active_mut() {
                if let Some(ref mut search) = pane.last_search {
                    if !search.matches.is_empty() {
                        match search.direction {
                            SearchDirection::Forward => {
                                search.current_idx = (search.current_idx + search.matches.len()
                                    - 1)
                                    % search.matches.len();
                            }
                            SearchDirection::Backward => {
                                search.current_idx =
                                    (search.current_idx + 1) % search.matches.len();
                            }
                        }
                        pane.nav_cursor = search.matches[search.current_idx];
                    }
                }
            }
        }

        // ── Mode switching ─────────────────────────────────────────────────────
        KeyCode::Char('v') if event.modifiers.contains(KeyModifiers::CONTROL) => {
            if let Some(pane) = dash.tree.active_mut() {
                if pane.kind == PaneType::TableView {
                    pane.mode = TableMode::VisualColumn;
                    pane.visual_anchor = Some(pane.row_cursor);
                }
            }
        }
        KeyCode::Char('v') => {
            if let Some(pane) = dash.tree.active_mut() {
                if pane.kind == PaneType::TableView {
                    pane.mode = TableMode::VisualRow;
                    pane.visual_anchor = Some(pane.row_cursor);
                }
            }
        }
        KeyCode::Char('V') => {
            if let Some(pane) = dash.tree.active_mut() {
                if pane.kind == PaneType::TableView {
                    pane.mode = TableMode::VisualRow;
                    pane.visual_anchor = Some(pane.row_cursor);
                }
            }
        }
        KeyCode::Char('i') => {
            if let Some(pane) = dash.tree.active_mut() {
                if pane.kind == PaneType::TableView {
                    if pane.mode == TableMode::Normal {
                        // Open cell editor for the current cell.
                        let row = pane.row_cursor;
                        let col = pane.cursor_col;
                        if let Some(ref table_name) = pane.bound_table {
                            if let Some(ref loaded) = dash.table_cache.get(table_name) {
                                if row < loaded.rows.len() && col < loaded.headers.len() {
                                    let current = loaded.rows[row][col].clone();
                                    let col_name = loaded.headers[col].clone();
                                    state.cmdline.open_cell_edit(row, col, &col_name, &current);
                                    return;
                                }
                            }
                        }
                    }
                }
            }
        }
        KeyCode::Char('u') => {
            if let Some(pane) = dash.tree.active_mut() {
                if pane.kind == PaneType::TableView && pane.mode == TableMode::Normal {
                    if !pane.pending_updates.is_empty() {
                        pane.pending_updates.pop();
                    } else if !pane.pending_deletes.is_empty() {
                        pane.pending_deletes.pop();
                    } else {
                        state.cmdline.set_error("already at oldest change");
                    }
                }
            }
        }
        KeyCode::Char('d') => {
            if let Some(pane) = dash.tree.active_mut() {
                if pane.kind == PaneType::TableView && pane.mode == TableMode::VisualRow {
                    if let Some(ref table_name) = pane.bound_table {
                        if let Some(ref loaded) = dash.table_cache.get(table_name) {
                            let anchor = pane.visual_anchor.unwrap_or(pane.row_cursor);
                            let start = anchor.min(pane.row_cursor);
                            let end = anchor.max(pane.row_cursor);
                            // Find PK column index.
                            let pk_idx = loaded.schema.iter().position(|c| c.is_primary_key);
                            if let Some(pk_col) = pk_idx {
                                for r in start..=end {
                                    if r < loaded.rows.len() {
                                        let pk_val = loaded.rows[r][pk_col].clone();
                                        if !pane.pending_deletes.contains(&pk_val) {
                                            pane.pending_deletes.push(pk_val);
                                        }
                                    }
                                }
                            }
                        }
                    }
                    pane.mode = TableMode::Normal;
                    pane.visual_anchor = None;
                } else if pane.kind == PaneType::TableView && pane.mode == TableMode::Normal {
                    // Start 'dd' sequence.
                    state.pending_key = Some('d');
                }
            }
        }
        KeyCode::Esc => {
            if let Some(pane) = dash.tree.active_mut() {
                if pane.kind == PaneType::TableView {
                    pane.mode = TableMode::Normal;
                    pane.visual_anchor = None;
                }
            }
        }

        // ── Navigation ─────────────────────────────────────────────────────────
        KeyCode::Char('j') | KeyCode::Down => {
            if let Some(pane) = dash.tree.active_mut() {
                match pane.kind {
                    PaneType::TableList => pane.nav_next(dash.tables.len()),
                    PaneType::TableView => {
                        let bound = pane
                            .bound_table
                            .as_ref()
                            .and_then(|name| dash.table_cache.get(name))
                            .map(|lt| lt.rows.len())
                            .unwrap_or(0);
                        pane.row_next(bound);
                    }
                    PaneType::SchemaView => {
                        let bound = pane
                            .bound_table
                            .as_ref()
                            .and_then(|name| dash.table_cache.get(name))
                            .map(|lt| lt.schema.len())
                            .unwrap_or(0);
                        pane.nav_next(bound);
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
                        let bound = pane
                            .bound_table
                            .as_ref()
                            .and_then(|name| dash.table_cache.get(name))
                            .map(|lt| lt.rows.len())
                            .unwrap_or(0);
                        if bound > 0 {
                            pane.row_prev();
                        }
                    }
                    PaneType::SchemaView => {
                        let bound = pane
                            .bound_table
                            .as_ref()
                            .and_then(|name| dash.table_cache.get(name))
                            .map(|lt| lt.schema.len())
                            .unwrap_or(0);
                        if bound > 0 {
                            pane.nav_prev();
                        }
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
                    let bound = pane
                        .bound_table
                        .as_ref()
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
                        let bound = pane
                            .bound_table
                            .as_ref()
                            .and_then(|name| dash.table_cache.get(name))
                            .map(|lt| lt.rows.len())
                            .unwrap_or(0);
                        pane.row_bottom(bound);
                    }
                    PaneType::SchemaView => {
                        let bound = pane
                            .bound_table
                            .as_ref()
                            .and_then(|name| dash.table_cache.get(name))
                            .map(|lt| lt.schema.len())
                            .unwrap_or(0);
                        pane.nav_bottom(bound);
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
                        pane.last_search = None; // clear search highlight
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
