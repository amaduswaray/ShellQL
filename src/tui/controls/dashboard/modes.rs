use super::helpers::pane_data;
use crate::tui::{AppState, state::TableMode, state::pane_layout::PaneType};

pub fn handle_dd(state: &mut AppState) {
    let active_idx = state.active_tab;
    let Some(tab) = state.tabs.get_mut(active_idx) else {
        return;
    };
    if let Some(pane) = tab.tree.active_mut() {
        if (pane.kind == PaneType::TableView || pane.kind == PaneType::QueryResults)
            && pane.mode == TableMode::Normal
        {
            if let Some((_headers, rows, schema)) =
                pane_data(&state.table_cache, &tab.query_results, pane)
            {
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
}

pub fn start_visual_column(state: &mut AppState) {
    let active_idx = state.active_tab;
    let Some(tab) = state.tabs.get_mut(active_idx) else {
        return;
    };
    if let Some(pane) = tab.tree.active_mut() {
        if pane.kind == PaneType::TableView || pane.kind == PaneType::QueryResults {
            pane.mode = TableMode::VisualColumn;
            pane.visual_anchor = Some(pane.row_cursor);
        }
    }
}

pub fn start_visual_row(state: &mut AppState) {
    let active_idx = state.active_tab;
    let Some(tab) = state.tabs.get_mut(active_idx) else {
        return;
    };
    if let Some(pane) = tab.tree.active_mut() {
        if pane.kind == PaneType::TableView || pane.kind == PaneType::QueryResults {
            pane.mode = TableMode::VisualRow;
            pane.visual_anchor = Some(pane.row_cursor);
        }
    }
}

pub fn start_insert_or_cell_edit(state: &mut AppState) {
    let active_idx = state.active_tab;
    let Some(tab) = state.tabs.get_mut(active_idx) else {
        return;
    };
    if let Some(pane) = tab.tree.active_mut() {
        if (pane.kind == PaneType::TableView || pane.kind == PaneType::QueryResults)
            && pane.mode == TableMode::Normal
        {
            let row = pane.row_cursor;
            let col = pane.cursor_col;
            if let Some((headers, rows, _schema)) =
                pane_data(&state.table_cache, &tab.query_results, pane)
            {
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

pub fn cycle_query_results(state: &mut AppState) {
    let active_idx = state.active_tab;
    let Some(tab) = state.tabs.get_mut(active_idx) else {
        return;
    };
    if let Some(pane) = tab.tree.active_mut() {
        if pane.kind == PaneType::QueryResults {
            if let Some(idx) = pane.bound_query_idx {
                let next = (idx + 1) % pane.query_result_count.max(1);
                pane.bound_query_idx = Some(next);
            }
        }
    }
}

pub fn undo_change(state: &mut AppState) {
    let active_idx = state.active_tab;
    let Some(tab) = state.tabs.get_mut(active_idx) else {
        return;
    };
    if let Some(pane) = tab.tree.active_mut() {
        if (pane.kind == PaneType::TableView || pane.kind == PaneType::QueryResults)
            && pane.mode == TableMode::Normal
        {
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

pub fn handle_delete(state: &mut AppState) {
    let active_idx = state.active_tab;
    let Some(tab) = state.tabs.get_mut(active_idx) else {
        return;
    };
    if let Some(pane) = tab.tree.active_mut() {
        if (pane.kind == PaneType::TableView || pane.kind == PaneType::QueryResults)
            && pane.mode == TableMode::VisualRow
        {
            if let Some((_headers, rows, schema)) =
                pane_data(&state.table_cache, &tab.query_results, pane)
            {
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
        } else if (pane.kind == PaneType::TableView || pane.kind == PaneType::QueryResults)
            && pane.mode == TableMode::Normal
        {
            state.pending_key = Some('d');
        }
    }
}

pub fn escape(state: &mut AppState) {
    let active_idx = state.active_tab;
    let Some(tab) = state.tabs.get_mut(active_idx) else {
        return;
    };
    if let Some(pane) = tab.tree.active_mut() {
        if pane.kind == PaneType::TableView || pane.kind == PaneType::QueryResults {
            pane.mode = TableMode::Normal;
            pane.visual_anchor = None;
        }
    }
}
