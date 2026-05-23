use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui_textarea::{Input, TextArea};

use crate::tui::{
    AppState, SearchDirection,
    state::TableMode,
    state::pane_layout::{PaneDirection, PaneType},
};

pub fn handle_dashboard(event: KeyEvent, state: &mut AppState) {
    let Some(ref mut dash) = state.dashboard else {
        return;
    };

    // Any keypress dismisses transient cmdline messages when idle.
    if !state.cmdline.is_active() {
        state.cmdline.loading = None;
        state.cmdline.error = None;
    }

    // ── QueryEditor insert mode ───────────────────────────────────────────────
    {
        let active_id = dash.tree.active_pane;
        let is_insert = dash
            .tree
            .panes
            .get(&active_id)
            .map_or(false, |p| p.kind == PaneType::QueryEditor && p.mode == TableMode::Insert);

        if is_insert {
            if event.code == KeyCode::Esc {
                if let Some(pane) = dash.tree.panes.get_mut(&active_id) {
                    pane.mode = TableMode::Normal;
                    pane.autocomplete_selected = None;
                    pane.autocomplete_matches.clear();
                }
                return;
            }

            let tables: Vec<String> = dash.tables.clone();
            let popup_open = dash
                .tree
                .panes
                .get(&active_id)
                .map_or(false, |p| p.autocomplete_selected.is_some());

            if popup_open {
                match event.code {
                    KeyCode::Up => {
                        if let Some(pane) = dash.tree.panes.get_mut(&active_id) {
                            if let Some(sel) = pane.autocomplete_selected {
                                pane.autocomplete_selected = Some(sel.saturating_sub(1));
                            }
                        }
                        return;
                    }
                    KeyCode::Down => {
                        if let Some(pane) = dash.tree.panes.get_mut(&active_id) {
                            if let Some(sel) = pane.autocomplete_selected {
                                let max = pane.autocomplete_matches.len().saturating_sub(1);
                                pane.autocomplete_selected = Some((sel + 1).min(max));
                            }
                        }
                        return;
                    }
                    KeyCode::Tab => {
                        let shift = event.modifiers.contains(KeyModifiers::SHIFT);
                        if let Some(pane) = dash.tree.panes.get_mut(&active_id) {
                            if let Some(sel) = pane.autocomplete_selected {
                                let len = pane.autocomplete_matches.len();
                                if shift {
                                    pane.autocomplete_selected = Some((sel + len - 1) % len);
                                } else {
                                    pane.autocomplete_selected = Some((sel + 1) % len);
                                }
                            }
                        }
                        return;
                    }
                    KeyCode::Enter => {
                        if let Some(pane) = dash.tree.panes.get_mut(&active_id) {
                            if let Some(sel) = pane.autocomplete_selected {
                                if let Some(replacement) = pane.autocomplete_matches.get(sel).cloned() {
                                    let (row, col) = pane.query_cursor;
                                    if let Some(line) = pane.query_text.get_mut(row) {
                                        let (start, _) = completion_prefix(line, col);
                                        let byte_end = char_idx_to_byte_idx(line, col);
                                        let quoted = if replacement.chars().any(|c| c.is_uppercase()) {
                                            format!("\"{}\"", replacement)
                                        } else {
                                            replacement
                                        };
                                        line.replace_range(start..byte_end, &quoted);
                                        pane.query_cursor = (row, start + quoted.chars().count());
                                    }
                                }
                            }
                            pane.autocomplete_selected = None;
                            pane.autocomplete_matches.clear();
                        }
                        return;
                    }
                    KeyCode::Esc => {
                        if let Some(pane) = dash.tree.panes.get_mut(&active_id) {
                            pane.autocomplete_selected = None;
                            pane.autocomplete_matches.clear();
                        }
                        return;
                    }
                    KeyCode::Char(' ') => {
                        if let Some(pane) = dash.tree.panes.get_mut(&active_id) {
                            pane.autocomplete_selected = None;
                            pane.autocomplete_matches.clear();
                        }
                        // fall through to feed space to textarea
                    }
                    _ => {
                        // Fall through to textarea input; popup stays open
                        // and will be updated by the auto-trigger check below.
                    }
                }
            } else if event.code == KeyCode::Tab {
                if let Some(pane) = dash.tree.panes.get_mut(&active_id) {
                    let (row, col) = pane.query_cursor;
                    if let Some(line) = pane.query_text.get(row) {
                        let (_, prefix) = completion_prefix(line, col);
                        if !prefix.is_empty() {
                            let matches: Vec<String> = tables
                                .iter()
                                .filter(|t| t.to_lowercase().starts_with(&prefix.to_lowercase()))
                                .cloned()
                                .collect();
                            if !matches.is_empty() {
                                pane.autocomplete_matches = matches;
                                pane.autocomplete_selected = Some(0);
                            }
                        }
                    }
                }
                return;
            }

            // Close popup on any other key when it wasn't already handled above.
            if !popup_open {
                if let Some(pane) = dash.tree.panes.get_mut(&active_id) {
                    pane.autocomplete_selected = None;
                    pane.autocomplete_matches.clear();
                }
            }

            // Feed the key event into the textarea.
            let (query_text, query_cursor) = dash
                .tree
                .panes
                .get(&active_id)
                .map(|p| (p.query_text.clone(), p.query_cursor))
                .unwrap_or_default();
            let mut textarea = TextArea::new(query_text);
            restore_cursor(&mut textarea, query_cursor);
            textarea.input(Input::from(event));
            let cursor = textarea.cursor();
            let lines: Vec<String> = textarea.lines().iter().map(|s| s.to_string()).collect();

            if let Some(pane) = dash.tree.panes.get_mut(&active_id) {
                pane.query_text = lines;
                pane.query_cursor = (cursor.0, cursor.1);

                // Auto-trigger: show table completions after FROM / JOIN / INTO / UPDATE / TABLE.
                let (row, col) = pane.query_cursor;
                if let Some(line) = pane.query_text.get(row) {
                    if let Some(prefix) = get_table_prefix(line, col) {
                        let matches: Vec<String> = tables
                            .iter()
                            .filter(|t| t.to_lowercase().starts_with(&prefix.to_lowercase()))
                            .cloned()
                            .collect();
                        if !matches.is_empty() {
                            pane.autocomplete_matches = matches;
                            pane.autocomplete_selected = Some(0);
                        } else {
                            pane.autocomplete_selected = None;
                            pane.autocomplete_matches.clear();
                        }
                    } else {
                        pane.autocomplete_selected = None;
                        pane.autocomplete_matches.clear();
                    }
                }
            }
            return;
        }
    }

    // Ctrl+hjkl / Ctrl+Arrows — pane navigation
    if event.modifiers.contains(KeyModifiers::CONTROL) {
        match event.code {
            KeyCode::Char('h') | KeyCode::Left => {
                dash.tree.navigate(PaneDirection::Left);
                dash.tree.exit_fullscreen();
                return;
            }
            KeyCode::Char('j') | KeyCode::Down => {
                dash.tree.navigate(PaneDirection::Down);
                dash.tree.exit_fullscreen();
                return;
            }
            KeyCode::Char('k') | KeyCode::Up => {
                dash.tree.navigate(PaneDirection::Up);
                dash.tree.exit_fullscreen();
                return;
            }
            KeyCode::Char('l') | KeyCode::Right => {
                dash.tree.navigate(PaneDirection::Right);
                dash.tree.exit_fullscreen();
                return;
            }
            KeyCode::Char('u') => {
                // Half-page scroll up
                if let Some(pane) = dash.tree.active_mut() {
                    if pane.kind == PaneType::TableView || pane.kind == PaneType::QueryResults {
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
                    if pane.kind == PaneType::TableView || pane.kind == PaneType::QueryResults {
                        let bound = pane_data(&dash.table_cache, &dash.query_results, pane).map_or(0, |(_, rows, _)| rows.len());
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
                    PaneType::TableView | PaneType::QueryResults => pane.row_top(),
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
                if (pane.kind == PaneType::TableView || pane.kind == PaneType::QueryResults) && pane.mode == TableMode::Normal {
                    if let Some((_headers, rows, schema)) = pane_data(&dash.table_cache, &dash.query_results, pane) {
                        let row = pane.row_cursor;
                        if row < rows.len() {
                            if let Some(pk_idx) = schema.iter().position(|c| c.is_primary_key) {
                                let pk_val = rows[row][pk_idx].clone();
                                if !pane.pending_deletes.contains(&pk_val) {
                                    pane.pending_deletes.push(pk_val);
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
                        match pane.kind {
                            PaneType::TableList => pane.nav_cursor = search.matches[search.current_idx],
                            PaneType::TableView | PaneType::QueryResults => pane.row_cursor = search.matches[search.current_idx],
                            _ => {}
                        }
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
                        match pane.kind {
                            PaneType::TableList => pane.nav_cursor = search.matches[search.current_idx],
                            PaneType::TableView | PaneType::QueryResults => pane.row_cursor = search.matches[search.current_idx],
                            _ => {}
                        }
                    }
                }
            }
        }

        // ── Cell hover (Shift+K) ───────────────────────────────────────────────
        KeyCode::Char('K') => {
            if let Some(pane) = dash.tree.active_mut() {
                if (pane.kind == PaneType::TableView || pane.kind == PaneType::QueryResults) && pane.mode == TableMode::Normal {
                    let row = pane.row_cursor;
                    let col = pane.cursor_col;
                    if let Some((headers, rows, _schema)) = pane_data(&dash.table_cache, &dash.query_results, pane) {
                        if row < rows.len() && col < headers.len() {
                            let value = &rows[row][col];
                            let col_name = &headers[col];
                            state.cmdline.loading = Some(format!("{}: {}", col_name, value));
                        }
                    }
                }
            }
        }

        // ── Mode switching ─────────────────────────────────────────────────────
        KeyCode::Char('v') if event.modifiers.contains(KeyModifiers::CONTROL) => {
            if let Some(pane) = dash.tree.active_mut() {
                if pane.kind == PaneType::TableView || pane.kind == PaneType::QueryResults {
                    pane.mode = TableMode::VisualColumn;
                    pane.visual_anchor = Some(pane.row_cursor);
                }
            }
        }
        KeyCode::Char('v') => {
            if let Some(pane) = dash.tree.active_mut() {
                if pane.kind == PaneType::TableView || pane.kind == PaneType::QueryResults {
                    pane.mode = TableMode::VisualRow;
                    pane.visual_anchor = Some(pane.row_cursor);
                }
            }
        }
        KeyCode::Char('V') => {
            if let Some(pane) = dash.tree.active_mut() {
                if pane.kind == PaneType::TableView || pane.kind == PaneType::QueryResults {
                    pane.mode = TableMode::VisualRow;
                    pane.visual_anchor = Some(pane.row_cursor);
                }
            }
        }
        KeyCode::Char('i') => {
            if let Some(pane) = dash.tree.active_mut() {
                if (pane.kind == PaneType::TableView || pane.kind == PaneType::QueryResults) && pane.mode == TableMode::Normal {
                    let row = pane.row_cursor;
                    let col = pane.cursor_col;
                    if let Some((headers, rows, _schema)) = pane_data(&dash.table_cache, &dash.query_results, pane) {
                        if row < rows.len() && col < headers.len() {
                            let current = rows[row][col].clone();
                            let col_name = headers[col].clone();
                            state.cmdline.open_cell_edit(row, col, &col_name, &current);
                            return;
                        }
                    }
                } else if pane.kind == PaneType::QueryEditor && pane.mode == TableMode::Normal {
                    pane.mode = TableMode::Insert;
                }
            }
        }
        KeyCode::Tab => {
            if let Some(pane) = dash.tree.active_mut() {
                if pane.kind == PaneType::QueryResults {
                    if let Some(idx) = pane.bound_query_idx {
                        let next = (idx + 1) % pane.query_result_count.max(1);
                        pane.bound_query_idx = Some(next);
                    }
                }
            }
        }
        KeyCode::Char('u') => {
            if let Some(pane) = dash.tree.active_mut() {
                if (pane.kind == PaneType::TableView || pane.kind == PaneType::QueryResults) && pane.mode == TableMode::Normal {
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
                if (pane.kind == PaneType::TableView || pane.kind == PaneType::QueryResults) && pane.mode == TableMode::VisualRow {
                    if let Some((_headers, rows, schema)) = pane_data(&dash.table_cache, &dash.query_results, pane) {
                        let anchor = pane.visual_anchor.unwrap_or(pane.row_cursor);
                        let start = anchor.min(pane.row_cursor);
                        let end = anchor.max(pane.row_cursor);
                        let pk_idx = schema.iter().position(|c| c.is_primary_key);
                        if let Some(pk_col) = pk_idx {
                            for r in start..=end {
                                if r < rows.len() {
                                    let pk_val = rows[r][pk_col].clone();
                                    if !pane.pending_deletes.contains(&pk_val) {
                                        pane.pending_deletes.push(pk_val);
                                    }
                                }
                            }
                        }
                    }
                    pane.mode = TableMode::Normal;
                    pane.visual_anchor = None;
                } else if (pane.kind == PaneType::TableView || pane.kind == PaneType::QueryResults) && pane.mode == TableMode::Normal {
                    state.pending_key = Some('d');
                }
            }
        }
        KeyCode::Esc => {
            if let Some(pane) = dash.tree.active_mut() {
                if pane.kind == PaneType::TableView || pane.kind == PaneType::QueryResults {
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
                    PaneType::TableView | PaneType::QueryResults => {
                        let bound = pane_data(&dash.table_cache, &dash.query_results, pane).map_or(0, |(_, rows, _)| rows.len());
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
                    PaneType::TableView | PaneType::QueryResults => {
                        let bound = pane_data(&dash.table_cache, &dash.query_results, pane).map_or(0, |(_, rows, _)| rows.len());
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
                if (pane.kind == PaneType::TableView || pane.kind == PaneType::QueryResults) && pane.cursor_col > 0 {
                    pane.col_left();
                }
            }
        }
        KeyCode::Char('l') | KeyCode::Right => {
            if let Some(pane) = dash.tree.active_mut() {
                if pane.kind == PaneType::TableView || pane.kind == PaneType::QueryResults {
                    let bound = pane_data(&dash.table_cache, &dash.query_results, pane).map_or(0, |(headers, _, _)| headers.len());
                    pane.col_right(bound);
                }
            }
        }

        // ── Jump to bottom / top ───────────────────────────────────────────────
        KeyCode::Char('G') => {
            if let Some(pane) = dash.tree.active_mut() {
                match pane.kind {
                    PaneType::TableList => pane.nav_bottom(dash.tables.len()),
                    PaneType::TableView | PaneType::QueryResults => {
                        let bound = pane_data(&dash.table_cache, &dash.query_results, pane).map_or(0, |(_, rows, _)| rows.len());
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

        // ── Back / forward history ──────────────────────────────────────────────
        KeyCode::Char('-') => {
            if let Some(pane) = dash.tree.active_mut() {
                if pane.go_back() {
                    if pane.kind == PaneType::TableView {
                        if let Some(name) = pane.bound_table.clone() {
                            if !dash.table_cache.contains_key(&name) {
                                dash.pending_load = Some(crate::tui::state::dashboard::PendingQuery {
                                    table: name,
                                    filter: None,
                                    sort_col: None,
                                    sort_desc: false,
                                    selected_cols: None,
                                });
                                dash.loading = true;
                                dash.error = None;
                            }
                        }
                    }
                } else {
                    state.cmdline.set_error("no previous view");
                }
            }
        }
        KeyCode::Char('_') => {
            if let Some(pane) = dash.tree.active_mut() {
                if pane.go_forward() {
                    if pane.kind == PaneType::TableView {
                        if let Some(name) = pane.bound_table.clone() {
                            if !dash.table_cache.contains_key(&name) {
                                dash.pending_load = Some(crate::tui::state::dashboard::PendingQuery {
                                    table: name,
                                    filter: None,
                                    sort_col: None,
                                    sort_desc: false,
                                    selected_cols: None,
                                });
                                dash.loading = true;
                                dash.error = None;
                            }
                        }
                    }
                } else {
                    state.cmdline.set_error("no next view");
                }
            }
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
                            dash.pending_load = Some(crate::tui::state::dashboard::PendingQuery {
                                table: name,
                                filter: None,
                                sort_col: None,
                                sort_desc: false,
                                selected_cols: None,
                            });
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

/// Return (headers, rows, schema) for a TableView or QueryResults pane.
fn pane_data<'a>(
    table_cache: &'a std::collections::HashMap<String, crate::tui::state::dashboard::LoadedTable>,
    query_results: &'a [crate::tui::state::dashboard::QueryResult],
    pane: &crate::tui::state::pane_layout::Pane,
) -> Option<(Vec<String>, &'a Vec<Vec<String>>, Vec<crate::connection::ColumnInfo>)> {
    use crate::tui::state::pane_layout::PaneType;
    match pane.kind {
        PaneType::TableView => {
            let name = pane.bound_table.as_ref()?;
            let loaded = table_cache.get(name)?;
            Some((loaded.headers.clone(), &loaded.rows, loaded.schema.clone()))
        }
        PaneType::QueryResults => {
            let idx = pane.bound_query_idx?;
            let qr = query_results.get(idx)?;
            let schema: Vec<crate::connection::ColumnInfo> = qr
                .headers
                .iter()
                .enumerate()
                .map(|(i, name)| crate::connection::ColumnInfo {
                    name: name.clone(),
                    data_type: "TEXT".to_string(),
                    nullable: true,
                    is_primary_key: i == 0,
                    default_value: None,
                })
                .collect();
            Some((qr.headers.clone(), &qr.rows, schema))
        }
        _ => None,
    }
}

/// Restore TextArea cursor position from stored (row, col).
fn restore_cursor(textarea: &mut TextArea, (target_row, target_col): (usize, usize)) {
    use ratatui_textarea::CursorMove;
    // Move to top-left first.
    textarea.move_cursor(CursorMove::Top);
    textarea.move_cursor(CursorMove::Head);
    // Move down to target row.
    for _ in 0..target_row {
        textarea.move_cursor(CursorMove::Down);
    }
    // Move right to target col.
    for _ in 0..target_col {
        textarea.move_cursor(CursorMove::Forward);
    }
}

/// Convert a character (Unicode scalar) index into a byte index in `s`.
/// If `char_idx` is larger than the number of chars, returns `s.len()`.
fn char_idx_to_byte_idx(s: &str, char_idx: usize) -> usize {
    let mut byte_idx = 0;
    for (i, ch) in s.chars().enumerate() {
        if i == char_idx {
            return byte_idx;
        }
        byte_idx += ch.len_utf8();
    }
    byte_idx
}

/// Extract the current table-name prefix if the cursor is positioned after a
/// trigger keyword (`FROM`, `JOIN`, `INTO`, `UPDATE`, `TABLE`).
fn get_table_prefix(line: &str, col: usize) -> Option<String> {
    let byte_col = char_idx_to_byte_idx(line, col);
    let before = &line[..byte_col.min(line.len())];

    // Find the last space before the cursor.
    let last_space = before.rfind(' ')?;
    let prefix = &before[last_space + 1..];

    // Don't trigger on multiple consecutive spaces.
    let before_prefix = &before[..last_space];
    let trailing_spaces = before_prefix.len().saturating_sub(before_prefix.trim_end().len());
    if trailing_spaces > 0 {
        return None;
    }

    let words: Vec<&str> = before_prefix.split_whitespace().collect();
    let prev_word = words.last().map(|w| w.to_lowercase())?;

    let triggers = ["from", "join", "into", "update", "table"];
    if triggers.contains(&prev_word.as_str()) {
        Some(prefix.to_string())
    } else {
        None
    }
}

/// Compute the replacement range for an autocomplete insertion.
/// Returns `(start_col, prefix)` where `prefix` is the text to replace.
fn completion_prefix(line: &str, col: usize) -> (usize, &str) {
    let byte_col = char_idx_to_byte_idx(line, col);
    let before = &line[..byte_col.min(line.len())];
    let start = before.rfind(' ').map(|i| i + 1).unwrap_or(0);
    (start, &before[start..])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_table_prefix_from_empty() {
        assert_eq!(get_table_prefix("SELECT * FROM ", 14), Some("".to_string()));
    }

    #[test]
    fn test_get_table_prefix_from_partial() {
        assert_eq!(get_table_prefix("SELECT * FROM u", 15), Some("u".to_string()));
    }

    #[test]
    fn test_get_table_prefix_join() {
        assert_eq!(get_table_prefix("SELECT * FROM users JOIN ", 25), Some("".to_string()));
    }

    #[test]
    fn test_get_table_prefix_update() {
        assert_eq!(get_table_prefix("UPDATE us", 9), Some("us".to_string()));
    }

    #[test]
    fn test_get_table_prefix_no_trigger() {
        assert_eq!(get_table_prefix("SELECT * WHERE id = 1", 21), None);
    }

    #[test]
    fn test_get_table_prefix_double_space_no_trigger() {
        assert_eq!(get_table_prefix("SELECT * FROM  ", 15), None);
    }

    #[test]
    fn test_get_table_prefix_after_table_name() {
        // After "users " the prev_word is "users" which is not a trigger
        assert_eq!(get_table_prefix("SELECT * FROM users ", 20), None);
    }

    #[test]
    fn test_completion_prefix_after_space() {
        // Cursor after "FROM " → prefix is empty, start at cursor
        assert_eq!(completion_prefix("SELECT * FROM ", 14), (14, ""));
    }

    #[test]
    fn test_completion_prefix_partial() {
        // Cursor after "FROM u" → prefix is "u"
        assert_eq!(completion_prefix("SELECT * FROM u", 15), (14, "u"));
    }

    #[test]
    fn test_completion_prefix_start_of_line() {
        // No space before cursor
        assert_eq!(completion_prefix("users", 5), (0, "users"));
    }

    #[test]
    fn test_char_idx_to_byte_idx_ascii() {
        assert_eq!(char_idx_to_byte_idx("hello", 0), 0);
        assert_eq!(char_idx_to_byte_idx("hello", 2), 2);
        assert_eq!(char_idx_to_byte_idx("hello", 5), 5);
    }

    #[test]
    fn test_char_idx_to_byte_idx_multibyte() {
        // 'æ' is 2 bytes in UTF-8
        assert_eq!(char_idx_to_byte_idx("æ", 0), 0);
        assert_eq!(char_idx_to_byte_idx("æ", 1), 2);
        assert_eq!(char_idx_to_byte_idx("æø", 1), 2);
        assert_eq!(char_idx_to_byte_idx("æø", 2), 4);
    }

    #[test]
    fn test_get_table_prefix_multibyte() {
        // After multi-byte char before trigger: "æ FROM " → should still work
        assert_eq!(get_table_prefix("æ FROM ", 7), Some("".to_string()));
    }

    #[test]
    fn test_completion_prefix_multibyte() {
        // "æ FROM " = 8 bytes, 7 chars. Cursor at char 7 (past end).
        // rfind(' ') finds space at byte 7, so start = 8.
        assert_eq!(completion_prefix("æ FROM ", 7), (8, ""));
    }
}
