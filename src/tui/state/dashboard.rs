//! Dashboard state — active DB session with pane-based layout.

use crate::connection::{ColumnInfo, Database, DbPool};

use super::pane_layout::{PaneTree, PaneType};

// ── Re-exports from old dashboard ─────────────────────────────────────────────

pub use super::pane_layout::{Pane, PaneDirection};

// ── Table mode ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TableMode {
    Normal,
    VisualRow,
    VisualColumn,
    Insert,
}

// ── Loaded table ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct LoadedTable {
    pub name: String,
    pub schema: Vec<ColumnInfo>,
    pub headers: Vec<String>,
    pub rows: Vec<Vec<String>>,
}

impl LoadedTable {
    pub fn new(
        name: String,
        schema: Vec<ColumnInfo>,
        headers: Vec<String>,
        rows: Vec<Vec<String>>,
    ) -> Self {
        Self { name, schema, headers, rows }
    }
}

// ── Dashboard state ───────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct DashboardState {
    /// The connection this session is attached to.
    pub connection: Database,
    /// Live pool — reused for every query.
    pub pool: DbPool,

    // ── Pane layout ───────────────────────────────────────────────────────────
    pub tree: PaneTree,

    // ── Data ──────────────────────────────────────────────────────────────────
    pub tables: Vec<String>,
    pub loaded: Option<LoadedTable>,

    // ── Async-load signal ─────────────────────────────────────────────────────
    pub pending_load: Option<String>,

    // ── Status ────────────────────────────────────────────────────────────────
    pub loading: bool,
    pub error: Option<String>,
}

impl DashboardState {
    pub fn new(connection: Database, pool: DbPool, tables: Vec<String>) -> Self {
        let mut tree = PaneTree::new(PaneType::TableList);
        // Also create a TableView pane side-by-side for the default 2-pane look.
        let _ = tree.split_active_v(PaneType::TableView);
        Self {
            connection,
            pool,
            tree,
            tables,
            loaded: None,
            pending_load: None,
            loading: false,
            error: None,
        }
    }

    /// Signal that the currently selected table in the active list pane should be loaded.
    pub fn request_load(&mut self) {
        // Find the active pane that is a TableList and load its selected table.
        if let Some(pane) = self.tree.active() {
            if pane.kind == PaneType::TableList {
                if let Some(name) = self.tables.get(pane.nav_cursor) {
                    self.pending_load = Some(name.clone());
                    self.loading = true;
                    self.error = None;
                }
            }
        }
    }
}
