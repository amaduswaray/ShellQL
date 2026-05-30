use super::helpers::pane_data;
use crate::tui::{
    AppState,
    state::{
        TableMode,
        pane_layout::{DisplayRowRef, PaneType},
    },
};

fn mark_row_deleted(
    pane: &mut crate::tui::state::pane_layout::Pane,
    rows: &[Vec<String>],
    schema: &[crate::connection::ColumnInfo],
    real_row: usize,
) {
    if let Some(pk_idx) = schema.iter().position(|c| c.is_primary_key) {
        if real_row < rows.len() {
            let pk_val = rows[real_row][pk_idx].clone();
            if !pane.pending_deletes.contains(&pk_val) {
                pane.pending_deletes.push(pk_val);
            }
        }
    }
}

fn clamp_row_cursor_after_insert_change(
    pane: &mut crate::tui::state::pane_layout::Pane,
    loaded_rows: usize,
) {
    let total = pane.total_table_rows(loaded_rows);
    if total == 0 {
        pane.row_cursor = 0;
        pane.row_offset = 0;
    } else if pane.row_cursor >= total {
        pane.row_cursor = total - 1;
    }
}

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
                match pane.kind {
                    PaneType::TableView => {
                        let row = pane.row_cursor;
                        match pane.display_row_ref(rows.len(), row) {
                            Some(DisplayRowRef::Existing(real_row)) => {
                                mark_row_deleted(pane, rows, &schema, real_row)
                            }
                            Some(DisplayRowRef::PendingInsert(insert_idx)) => {
                                pane.remove_pending_insert(insert_idx);
                                clamp_row_cursor_after_insert_change(pane, rows.len());
                            }
                            None => {}
                        }
                    }
                    PaneType::QueryResults => {
                        let row = pane.row_cursor;
                        if row < rows.len() {
                            mark_row_deleted(pane, rows, &schema, row);
                        }
                    }
                    _ => {}
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
    let mut open_cell: Option<(usize, usize, String, String)> = None;

    {
        let active_idx = state.active_tab;
        let Some(tab) = state.tabs.get_mut(active_idx) else {
            return;
        };
        let Some(pane) = tab.tree.active_mut() else {
            return;
        };

        if (pane.kind == PaneType::TableView || pane.kind == PaneType::QueryResults)
            && pane.mode == TableMode::Normal
        {
            let row_display = pane.row_cursor;
            let col = pane.cursor_col;
            if let Some((headers, rows, _schema)) =
                pane_data(&state.table_cache, &tab.query_results, pane)
            {
                if col < headers.len() {
                    let col_name = headers[col].clone();
                    match pane.kind {
                        PaneType::TableView => {
                            match pane.display_row_ref(rows.len(), row_display) {
                                Some(DisplayRowRef::Existing(real_row)) => {
                                    if real_row < rows.len() {
                                        let current = pane
                                            .pending_updates
                                            .iter()
                                            .rev()
                                            .find(|(r, c, _)| *r == real_row && *c == col)
                                            .map(|(_, _, v)| v.clone())
                                            .unwrap_or_else(|| rows[real_row][col].clone());
                                        open_cell = Some((row_display, col, col_name, current));
                                    }
                                }
                                Some(DisplayRowRef::PendingInsert(insert_idx)) => {
                                    if let Some(staged) = pane.pending_inserts.get(insert_idx) {
                                        let current = staged
                                            .values
                                            .get(col)
                                            .cloned()
                                            .unwrap_or_else(String::new);
                                        open_cell = Some((row_display, col, col_name, current));
                                    }
                                }
                                None => {}
                            }
                        }
                        PaneType::QueryResults => {
                            if row_display < rows.len() {
                                let current = rows[row_display][col].clone();
                                open_cell = Some((row_display, col, col_name, current));
                            }
                        }
                        _ => {}
                    }
                }
            }
        } else if pane.kind == PaneType::QueryEditor && pane.mode == TableMode::Normal {
            pane.mode = TableMode::Insert;
        }
    }

    if let Some((row, col, col_name, current)) = open_cell {
        state.cmdline.open_cell_edit(row, col, &col_name, &current);
    }
}

pub fn stage_insert_row_below(state: &mut AppState) {
    stage_insert_row(state, false);
}

pub fn stage_insert_row_above(state: &mut AppState) {
    stage_insert_row(state, true);
}

fn stage_insert_row(state: &mut AppState, above: bool) {
    let active_idx = state.active_tab;
    let Some(tab) = state.tabs.get_mut(active_idx) else {
        return;
    };
    let Some(pane) = tab.tree.active_mut() else {
        return;
    };

    if pane.kind != PaneType::TableView || pane.mode != TableMode::Normal {
        return;
    }

    let Some((headers, rows, _schema)) = pane_data(&state.table_cache, &tab.query_results, pane)
    else {
        state.cmdline.set_error("table not loaded yet");
        return;
    };

    let col_count = headers.len();
    if col_count == 0 {
        state.cmdline.set_error("table has no columns");
        return;
    }

    let total_rows = pane.total_table_rows(rows.len());
    let cursor = if total_rows == 0 {
        0
    } else {
        pane.row_cursor.min(total_rows - 1)
    };
    let insert_at = if total_rows == 0 {
        0
    } else if above {
        cursor
    } else {
        (cursor + 1).min(total_rows)
    };

    pane.stage_insert_row(insert_at, col_count);
    pane.row_cursor = insert_at;
    pane.visual_anchor = None;
    pane.mode = TableMode::Normal;
    pane.cursor_col = pane.cursor_col.min(col_count.saturating_sub(1));
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

pub fn cycle_query_results_prev(state: &mut AppState) {
    let active_idx = state.active_tab;
    let Some(tab) = state.tabs.get_mut(active_idx) else {
        return;
    };
    if let Some(pane) = tab.tree.active_mut() {
        if pane.kind == PaneType::QueryResults {
            if let Some(idx) = pane.bound_query_idx {
                let total = pane.query_result_count.max(1);
                let prev = (idx + total - 1) % total;
                pane.bound_query_idx = Some(prev);
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
            } else if !pane.pending_inserts.is_empty() {
                let last_idx = pane.pending_inserts.len() - 1;
                pane.remove_pending_insert(last_idx);
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

                match pane.kind {
                    PaneType::TableView => {
                        let mut inserts_to_remove = Vec::new();
                        for display_row in start..=end {
                            match pane.display_row_ref(rows.len(), display_row) {
                                Some(DisplayRowRef::Existing(real_row)) => {
                                    mark_row_deleted(pane, rows, &schema, real_row)
                                }
                                Some(DisplayRowRef::PendingInsert(insert_idx)) => {
                                    inserts_to_remove.push(insert_idx);
                                }
                                None => {}
                            }
                        }

                        inserts_to_remove.sort_unstable();
                        inserts_to_remove.dedup();
                        for idx in inserts_to_remove.into_iter().rev() {
                            pane.remove_pending_insert(idx);
                        }
                        clamp_row_cursor_after_insert_change(pane, rows.len());
                    }
                    PaneType::QueryResults => {
                        for row in start..=end {
                            if row < rows.len() {
                                mark_row_deleted(pane, rows, &schema, row);
                            }
                        }
                    }
                    _ => {}
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
