use super::pane::{FloatingPane, Pane, PaneId};
use crate::connection::Database;

#[derive(Debug)]
pub struct Session {
    pub name: String,
    pub connection: Database,
    pub dashboard: DashboardState,
}

#[derive(Debug)]
pub struct DashboardState {
    pub tabs: Vec<Tab>,
    pub active_tab: usize,
    pub active_pane: PaneId,
    pub floating_pane: Option<FloatingPane>,
}

#[derive(Debug)]
pub struct Tab {
    pub name: String,
    pub panes: Vec<Pane>,
}
