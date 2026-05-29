//! Tab state — per-tab workspace within a database session.
//!
//! A tab owns its own pane tree, query results, and pending operations,
//! but shares the database connection, pool, and table cache with other tabs.

use crate::connection::ColumnInfo;

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

// ── Query result ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct QueryResult {
    pub sql: String,
    pub headers: Vec<String>,
    pub rows: Vec<Vec<String>>,
    pub error: Option<String>,
}

// ── Async query parameters ───────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct PendingQuery {
    pub table: String,
    pub filter: Option<String>,
    pub sort_col: Option<String>,
    pub sort_desc: bool,
    pub selected_cols: Option<Vec<String>>,
}

// ── Async commit parameters ─────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct PendingInsert {
    pub cols: Vec<String>,
    pub vals: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct PendingCommit {
    pub table: String,
    pub pk_col: String,
    pub updates: Vec<(String, String, String)>, // (pk_val, target_col, new_value)
    pub deletes: Vec<String>,                   // pk_vals
    pub inserts: Vec<PendingInsert>,
}

// ── Tab state ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct Tab {
    /// Pane layout tree for this tab.
    pub tree: PaneTree,

    // ── Async-load signal ─────────────────────────────────────────────────────
    pub pending_load: Option<PendingQuery>,

    // ── Async-commit signal ───────────────────────────────────────────────────
    pub pending_commit: Option<PendingCommit>,

    // ── Query execution results ───────────────────────────────────────────────
    /// Results from the last executed query.
    pub query_results: Vec<QueryResult>,
    /// SQL to execute async.
    pub pending_query_exec: Option<String>,
    /// Per-tab query history.
    pub query_history: Vec<String>,

    // ── Status ────────────────────────────────────────────────────────────────
    pub loading: bool,
    pub error: Option<String>,
}

impl Tab {
    pub fn new() -> Self {
        Self {
            tree: PaneTree::new(PaneType::TableList),
            pending_load: None,
            pending_commit: None,
            query_results: Vec::new(),
            pending_query_exec: None,
            query_history: Vec::new(),
            loading: false,
            error: None,
        }
    }

    /// Signal that the currently selected table in the active list pane should be loaded.
    pub fn request_load(&mut self, tables: &[String]) {
        if let Some(pane) = self.tree.active() {
            if pane.kind == PaneType::TableList || pane.kind == PaneType::SchemaPicker {
                if let Some(name) = tables.get(pane.nav_cursor) {
                    self.pending_load = Some(PendingQuery {
                        table: name.clone(),
                        filter: None,
                        sort_col: None,
                        sort_desc: false,
                        selected_cols: None,
                    });
                    self.loading = true;
                    self.error = None;
                }
            }
        }
    }
}
