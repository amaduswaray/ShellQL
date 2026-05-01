use super::data::Row;
use super::editor::QueryEditorState;
use super::table::TableViewState;
use uuid::Uuid;

#[derive(PartialEq, Eq, Clone, Copy, Debug, Hash)]
pub struct PaneId(pub Uuid);

impl PaneId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

#[derive(Debug, Clone)]
pub enum Pane {
    TableView(TableViewState),
    QueryEditor(QueryEditorState),
}

impl Pane {
    pub fn id(&self) -> PaneId {
        match self {
            Pane::TableView(s) => s.id,
            Pane::QueryEditor(s) => s.id,
        }
    }
}

#[derive(Debug)]
pub enum FloatingPane {
    RowDetail(Row),
    FilterInput(String),
    SortMenu,
}

#[derive(Debug)]
pub enum Overlay {
    Help,
    AddConnection,
    CommandPalette,
    ConfirmDelete,
}
