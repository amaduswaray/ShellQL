use crate::connection::{Database, list_connections};

use super::cmdline::CommandLine;
use super::dashboard::DashboardState;
use super::form::AddConnectionForm;
use super::pane::Overlay;
use super::session::Session;

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
    pub sessions: Vec<Session>,
    pub active_session: usize,
    pub pending_key: Option<char>,
    pub cmdline: CommandLine,
    /// Populated when `Overlay::AddConnection` is active.
    pub form: Option<AddConnectionForm>,
    /// Populated once the user connects to a database.
    pub dashboard: Option<DashboardState>,
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
            sessions: vec![],
            active_session: 0,
            pending_key: None,
            cmdline: CommandLine::new(),
            form: None,
            dashboard: None,
            pending_connection: None,
        }
    }
}
