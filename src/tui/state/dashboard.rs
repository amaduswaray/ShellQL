/// Dashboard state — active DB session with nav and table view.

use crate::connection::{ColumnInfo, Database, DbPool};

// ── Pane focus ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActivePane {
    /// Left nav bar (table list).
    Nav,
    /// Center table-data view.
    Table,
}

// ── Table mode ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TableMode {
    Normal,
    VisualRow,
    VisualColumn,
    Insert,
}

// ── Loaded table ──────────────────────────────────────────────────────────────

/// The data and cursor state for a table that has been loaded from the DB.
#[derive(Debug, Clone)]
pub struct LoadedTable {
    pub name: String,
    /// Column schema (type, nullable, PK).
    pub schema: Vec<ColumnInfo>,
    /// Ordered column names — the table header.
    pub headers: Vec<String>,
    /// String-rendered rows.
    pub rows: Vec<Vec<String>>,
    /// Row cursor (0-based).
    pub row_cursor: usize,
    /// Vertical scroll offset.
    pub row_offset: usize,
    /// Column cursor (0-based).
    pub cursor_col: usize,
    /// Horizontal scroll offset (number of columns skipped left).
    pub col_offset: usize,
    /// Navigation/editing mode.
    pub mode: TableMode,
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
            row_cursor: 0,
            row_offset: 0,
            cursor_col: 0,
            col_offset: 0,
            mode: TableMode::Normal,
        }
    }

    // ── Row navigation ────────────────────────────────────────────────────────

    pub fn row_next(&mut self) {
        if !self.rows.is_empty() {
            self.row_cursor = (self.row_cursor + 1).min(self.rows.len().saturating_sub(1));
        }
    }

    pub fn row_prev(&mut self) {
        self.row_cursor = self.row_cursor.saturating_sub(1);
    }

    pub fn row_top(&mut self) {
        self.row_cursor = 0;
    }

    pub fn row_bottom(&mut self) {
        if !self.rows.is_empty() {
            self.row_cursor = self.rows.len().saturating_sub(1);
        }
    }

    /// Update `row_offset` so `row_cursor` stays within `[offset, offset+viewport)`.
    pub fn sync_row_offset(&mut self, viewport: usize) {
        if self.row_cursor < self.row_offset {
            self.row_offset = self.row_cursor;
        } else if self.row_cursor >= self.row_offset + viewport {
            self.row_offset = self.row_cursor + 1 - viewport;
        }
    }

    // ── Column navigation ─────────────────────────────────────────────────────

    pub fn col_right(&mut self) {
        if !self.headers.is_empty() {
            self.cursor_col = (self.cursor_col + 1).min(self.headers.len().saturating_sub(1));
        }
    }

    pub fn col_left(&mut self) {
        self.cursor_col = self.cursor_col.saturating_sub(1);
    }

    pub fn col_first(&mut self) {
        self.cursor_col = 0;
    }

    pub fn col_last(&mut self) {
        if !self.headers.is_empty() {
            self.cursor_col = self.headers.len().saturating_sub(1);
        }
    }

    /// Update `col_offset` so `cursor_col` stays within the visible viewport.
    pub fn sync_col_offset(&mut self, viewport_cols: usize) {
        if self.cursor_col < self.col_offset {
            self.col_offset = self.cursor_col;
        } else if self.cursor_col >= self.col_offset + viewport_cols {
            self.col_offset = self.cursor_col + 1 - viewport_cols;
        }
    }

    // ── Mode switching ────────────────────────────────────────────────────────

    pub fn enter_normal(&mut self) {
        self.mode = TableMode::Normal;
    }

    pub fn enter_visual_row(&mut self) {
        self.mode = TableMode::VisualRow;
    }

    pub fn enter_visual_column(&mut self) {
        self.mode = TableMode::VisualColumn;
    }

    pub fn enter_insert(&mut self) {
        self.mode = TableMode::Insert;
    }
}

// ── Dashboard state ───────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct DashboardState {
    /// The connection this session is attached to.
    pub connection: Database,
    /// Live pool — reused for every query.
    pub pool: DbPool,

    // ── Nav bar ───────────────────────────────────────────────────────────────
    pub tables: Vec<String>,
    pub nav_cursor: usize,
    /// Vertical scroll offset for the nav list.
    pub nav_offset: usize,

    // ── Pane focus ────────────────────────────────────────────────────────────
    pub active_pane: ActivePane,

    // ── Loaded data ───────────────────────────────────────────────────────────
    pub loaded: Option<LoadedTable>,

    // ── Async-load signal ─────────────────────────────────────────────────────
    /// When set, `controls/mod.rs` will load this table and clear the field.
    pub pending_load: Option<String>,

    // ── Status ────────────────────────────────────────────────────────────────
    pub loading: bool,
    pub error: Option<String>,
}

impl DashboardState {
    pub fn new(connection: Database, pool: DbPool, tables: Vec<String>) -> Self {
        Self {
            connection,
            pool,
            tables,
            nav_cursor: 0,
            nav_offset: 0,
            active_pane: ActivePane::Nav,
            loaded: None,
            pending_load: None,
            loading: false,
            error: None,
        }
    }

    // ── Nav ───────────────────────────────────────────────────────────────────

    pub fn nav_next(&mut self) {
        if !self.tables.is_empty() {
            self.nav_cursor = (self.nav_cursor + 1).min(self.tables.len().saturating_sub(1));
        }
    }

    pub fn nav_prev(&mut self) {
        self.nav_cursor = self.nav_cursor.saturating_sub(1);
    }

    pub fn nav_top(&mut self) {
        self.nav_cursor = 0;
    }

    pub fn nav_bottom(&mut self) {
        if !self.tables.is_empty() {
            self.nav_cursor = self.tables.len().saturating_sub(1);
        }
    }

    /// Update `nav_offset` so `nav_cursor` stays within `[offset, offset+viewport)`.
    pub fn sync_nav_offset(&mut self, viewport: usize) {
        if self.nav_cursor < self.nav_offset {
            self.nav_offset = self.nav_cursor;
        } else if self.nav_cursor >= self.nav_offset + viewport {
            self.nav_offset = self.nav_cursor + 1 - viewport;
        }
    }

    // ── Pane focus cycling ────────────────────────────────────────────────────

    pub fn pane_next(&mut self) {
        self.active_pane = match self.active_pane {
            ActivePane::Nav => ActivePane::Table,
            ActivePane::Table => ActivePane::Nav,
        };
    }

    pub fn pane_prev(&mut self) {
        self.active_pane = match self.active_pane {
            ActivePane::Nav => ActivePane::Table,
            ActivePane::Table => ActivePane::Nav,
        };
    }

    // ── Table loading ─────────────────────────────────────────────────────────

    /// Signal that the currently selected nav table should be loaded.
    pub fn request_load(&mut self) {
        if let Some(name) = self.tables.get(self.nav_cursor) {
            self.pending_load = Some(name.clone());
            self.loading = true;
            self.error = None;
        }
    }
}
