use crate::tui::{AppState, SearchDirection, state::pane_layout::PaneType};

pub fn next(state: &mut AppState) {
    let active_idx = state.active_tab;
    let Some(tab) = state.tabs.get_mut(active_idx) else {
        return;
    };
    if let Some(pane) = tab.tree.active_mut() {
        if let Some(ref mut search) = pane.last_search {
            if !search.matches.is_empty() {
                match search.direction {
                    SearchDirection::Forward => {
                        search.current_idx = (search.current_idx + 1) % search.matches.len();
                    }
                    SearchDirection::Backward => {
                        search.current_idx =
                            (search.current_idx + search.matches.len() - 1) % search.matches.len();
                    }
                }
                match pane.kind {
                    PaneType::TableList => pane.nav_cursor = search.matches[search.current_idx],
                    PaneType::TableView | PaneType::QueryResults => {
                        pane.row_cursor = search.matches[search.current_idx]
                    }
                    _ => {}
                }
            }
        }
    }
}

pub fn prev(state: &mut AppState) {
    let active_idx = state.active_tab;
    let Some(tab) = state.tabs.get_mut(active_idx) else {
        return;
    };
    if let Some(pane) = tab.tree.active_mut() {
        if let Some(ref mut search) = pane.last_search {
            if !search.matches.is_empty() {
                match search.direction {
                    SearchDirection::Forward => {
                        search.current_idx =
                            (search.current_idx + search.matches.len() - 1) % search.matches.len();
                    }
                    SearchDirection::Backward => {
                        search.current_idx = (search.current_idx + 1) % search.matches.len();
                    }
                }
                match pane.kind {
                    PaneType::TableList => pane.nav_cursor = search.matches[search.current_idx],
                    PaneType::TableView | PaneType::QueryResults => {
                        pane.row_cursor = search.matches[search.current_idx]
                    }
                    _ => {}
                }
            }
        }
    }
}
