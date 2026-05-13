use super::dashboard::TableMode;
use super::data::{Column, Row, SortState};
use super::pane::PaneId;

#[derive(Debug, Clone)]
pub struct TableViewState {
    pub id: PaneId,
    pub table_name: String,
    pub columns: Vec<Column>,
    pub rows: Vec<Row>,

    // Vim navigation
    pub cursor_row: usize,
    pub cursor_col: usize,
    pub offset: usize,

    // State
    pub mode: TableMode,
    pub filter: Option<String>,
    pub sort: Option<SortState>,
    pub selected_rows: Vec<usize>,
}

impl TableViewState {
    pub fn new(table_name: String) -> Self {
        Self {
            id: PaneId::new(),
            table_name,
            columns: vec![],
            rows: vec![],
            cursor_row: 0,
            cursor_col: 0,
            offset: 0,
            mode: TableMode::Normal,
            filter: None,
            sort: None,
            selected_rows: vec![],
        }
    }

    pub fn clamp_cursor(&mut self) {
        self.cursor_row = self.cursor_row.min(self.rows.len().saturating_sub(1));
        self.cursor_col = self.cursor_col.min(self.columns.len().saturating_sub(1));
    }
}
