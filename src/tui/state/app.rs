use std::collections::HashMap;

use crate::connection::{Database, DbPool, list_connections};

use super::cmdline::CommandLine;
use super::form::AddConnectionForm;
use super::pane::Overlay;
use super::tab::{LoadedTable, Tab};

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub enum AppMode {
    Home,
    Dashboard,
}

pub struct AppState {
    pub mode: AppMode,
    pub overlay: Option<Overlay>,
    pub should_quit: bool,
    pub connections: Vec<Database>,
    pub selected_connection: usize,
    pub pending_key: Option<char>,
    pub cmdline: CommandLine,
    /// Populated when `Overlay::AddConnection` is active.
    pub form: Option<AddConnectionForm>,

    // ── Shared database session state ─────────────────────────────────────
    /// Active connection metadata.
    pub connection: Option<Database>,
    /// Live DB pool — cheap to clone for async ops.
    pub pool: Option<DbPool>,
    /// Schema table list from the current connection.
    pub tables: Vec<String>,
    /// Shared table cache across all tabs.
    pub table_cache: HashMap<String, LoadedTable>,

    // ── Tabs ──────────────────────────────────────────────────────────────
    pub tabs: Vec<Tab>,
    pub active_tab: usize,

    /// Set by the connection picker; handled in the event loop with a spinner.
    pub pending_connection: Option<Database>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            mode: AppMode::Home,
            overlay: None,
            should_quit: false,
            connections: list_connections(),
            selected_connection: 0,
            pending_key: None,
            cmdline: CommandLine::new(),
            form: None,
            connection: None,
            pool: None,
            tables: Vec::new(),
            table_cache: HashMap::new(),
            tabs: Vec::new(),
            active_tab: 0,
            pending_connection: None,
        }
    }

    /// True if we're in a database session (tabs exist).
    pub fn has_session(&self) -> bool {
        !self.tabs.is_empty()
    }

    /// Mutable reference to the active tab, if any.
    pub fn active_tab_mut(&mut self) -> Option<&mut Tab> {
        self.tabs.get_mut(self.active_tab)
    }

    /// Reference to the active tab, if any.
    pub fn active_tab(&self) -> Option<&Tab> {
        self.tabs.get(self.active_tab)
    }
}
