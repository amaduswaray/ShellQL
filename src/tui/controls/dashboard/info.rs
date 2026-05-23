use super::helpers::pane_data;
use crate::tui::{AppState, state::TableMode, state::pane_layout::PaneType};

pub fn show_cell_hover(state: &mut AppState) {
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
                    let value = &rows[row][col];
                    let col_name = &headers[col];
                    state.cmdline.loading = Some(format!("{}: {}", col_name, value));
                }
            }
        }
    }
}
