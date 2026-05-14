//! Dashboard state — active DB session with pane-based layout.

use std::collections::HashMap;

use crate::connection::{ColumnInfo, Database, DbPool};

use super::pane_layout::{PaneTree, PaneType};

// ── Re-exports ────────────────────────────────────────────────────────────────

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
        Self {
            name,
            schema,
            headers,
            rows,
        }
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

    // ── Shared table cache (name → loaded data) ───────────────────────────────
    /// Populated on first load; reused when another pane views the same table.
    pub table_cache: HashMap<String, LoadedTable>,

    // ── Async-load signal ─────────────────────────────────────────────────────
    pub pending_load: Option<String>,

    // ── Async-commit signal ───────────────────────────────────────────────────
    pub pending_commit: Option<PendingCommit>,

    // ── Status ────────────────────────────────────────────────────────────────
    pub loading: bool,
    pub error: Option<String>,
}

/// Staged changes waiting to be written to the database.
#[derive(Debug, Clone)]
pub struct PendingCommit {
    pub table: String,
    pub pk_col: String,
    pub updates: Vec<(String, String, String)>, // (pk_val, target_col, new_value)
    pub deletes: Vec<String>,                   // pk_vals
}

impl DashboardState {
    pub fn new(connection: Database, pool: DbPool, tables: Vec<String>) -> Self {
        let tree = PaneTree::new(PaneType::TableList);
        Self {
            connection,
            pool,
            tree,
            tables,
            table_cache: HashMap::new(),
            pending_load: None,
            pending_commit: None,
            loading: false,
            error: None,
        }
    }

    /// Signal that the currently selected table in the active list pane should be loaded.
    /// The pane will be converted to a TableView after the async load completes.
    pub fn request_load(&mut self) {
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
