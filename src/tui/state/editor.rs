use super::data::{Column, Row};
use super::pane::PaneId;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EditorMode {
    Normal,
    Insert,
}

#[derive(Debug, Clone)]
pub struct QueryResult {
    pub columns: Vec<Column>,
    pub rows: Vec<Row>,
    pub rows_affected: Option<u64>,
    pub error: Option<String>,
}

#[derive(Debug, Clone)]
pub struct QueryEditorState {
    pub id: PaneId,
    pub input: String,
    pub cursor_pos: usize,
    pub history: Vec<String>,
    pub history_index: Option<usize>,
    pub result: Option<QueryResult>,
    pub mode: EditorMode,
}

impl QueryEditorState {
    pub fn new() -> Self {
        Self {
            id: PaneId::new(),
            input: String::new(),
            cursor_pos: 0,
            history: vec![],
            history_index: None,
            result: None,
            mode: EditorMode::Normal,
        }
    }

    pub fn submit(&mut self) -> String {
        let query = self.input.clone();
        if !query.trim().is_empty() {
            self.history.push(query.clone());
        }
        self.history_index = None;
        self.input.clear();
        self.cursor_pos = 0;
        query
    }
}
