use crossterm::event::{KeyCode, KeyEvent};

use super::helpers::pane_data;
use crate::tui::{
    AppState,
    state::pane_layout::{PaneDirection, PaneType},
};

pub fn handle_ctrl(event: KeyEvent, state: &mut AppState, tables: &[String]) -> bool {
    let active_idx = state.active_tab;
    let Some(tab) = state.tabs.get_mut(active_idx) else {
        return false;
    };

    match event.code {
        KeyCode::Char('h') | KeyCode::Left => {
            tab.tree.navigate(PaneDirection::Left);
            tab.tree.exit_fullscreen();
            true
        }
        KeyCode::Char('j') | KeyCode::Down => {
            tab.tree.navigate(PaneDirection::Down);
            tab.tree.exit_fullscreen();
            true
        }
        KeyCode::Char('k') | KeyCode::Up => {
            tab.tree.navigate(PaneDirection::Up);
            tab.tree.exit_fullscreen();
            true
        }
        KeyCode::Char('l') | KeyCode::Right => {
            tab.tree.navigate(PaneDirection::Right);
            tab.tree.exit_fullscreen();
            true
        }
        KeyCode::Char('u') => {
            // Half-page scroll up
            if let Some(pane) = tab.tree.active_mut() {
                if pane.kind == PaneType::TableView || pane.kind == PaneType::QueryResults {
                    let viewport = pane.area.map_or(10, |a| (a.height / 2).max(1) as usize);
                    for _ in 0..viewport {
                        pane.row_prev();
                    }
                } else if pane.kind == PaneType::TableList || pane.kind == PaneType::SchemaPicker {
                    let viewport = pane.area.map_or(10, |a| (a.height / 2).max(1) as usize);
                    for _ in 0..viewport {
                        pane.nav_prev();
                    }
                } else if pane.kind == PaneType::SchemaView {
                    let viewport = pane.area.map_or(3, |a| (a.height / 6).max(1) as usize);
                    for _ in 0..viewport {
                        pane.nav_prev();
                    }
                }
            }
            true
        }
        KeyCode::Char('d') => {
            // Half-page scroll down
            if let Some(pane) = tab.tree.active_mut() {
                if pane.kind == PaneType::TableView || pane.kind == PaneType::QueryResults {
                    let loaded_rows = pane_data(&state.table_cache, &tab.query_results, pane)
                        .map_or(0, |(_, rows, _)| rows.len());
                    let bound = if pane.kind == PaneType::TableView {
                        pane.total_table_rows(loaded_rows)
                    } else {
                        loaded_rows
                    };
                    let viewport = pane.area.map_or(10, |a| (a.height / 2).max(1) as usize);
                    for _ in 0..viewport {
                        pane.row_next(bound);
                    }
                } else if pane.kind == PaneType::TableList || pane.kind == PaneType::SchemaPicker {
                    let viewport = pane.area.map_or(10, |a| (a.height / 2).max(1) as usize);
                    for _ in 0..viewport {
                        pane.nav_next(tables.len());
                    }
                } else if pane.kind == PaneType::SchemaView {
                    let bound = pane
                        .bound_table
                        .as_ref()
                        .and_then(|name| state.table_cache.get(name))
                        .map(|lt| lt.schema.len())
                        .unwrap_or(0);
                    let viewport = pane.area.map_or(3, |a| (a.height / 6).max(1) as usize);
                    for _ in 0..viewport {
                        pane.nav_next(bound);
                    }
                }
            }
            true
        }
        _ => false,
    }
}

pub fn go_top(state: &mut AppState) {
    let active_idx = state.active_tab;
    let Some(tab) = state.tabs.get_mut(active_idx) else {
        return;
    };
    if let Some(pane) = tab.tree.active_mut() {
        match pane.kind {
            PaneType::TableList | PaneType::SchemaPicker => pane.nav_top(),
            PaneType::TableView | PaneType::QueryResults => pane.row_top(),
            PaneType::SchemaView => pane.nav_top(),
            _ => {}
        }
    }
}

pub fn down(state: &mut AppState, tables: &[String]) {
    let active_idx = state.active_tab;
    let Some(tab) = state.tabs.get_mut(active_idx) else {
        return;
    };
    if let Some(pane) = tab.tree.active_mut() {
        match pane.kind {
            PaneType::TableList | PaneType::SchemaPicker => pane.nav_next(tables.len()),
            PaneType::TableView | PaneType::QueryResults => {
                let loaded_rows = pane_data(&state.table_cache, &tab.query_results, pane)
                    .map_or(0, |(_, rows, _)| rows.len());
                let bound = if pane.kind == PaneType::TableView {
                    pane.total_table_rows(loaded_rows)
                } else {
                    loaded_rows
                };
                pane.row_next(bound);
            }
            PaneType::SchemaView => {
                let bound = pane
                    .bound_table
                    .as_ref()
                    .and_then(|name| state.table_cache.get(name))
                    .map(|lt| lt.schema.len())
                    .unwrap_or(0);
                pane.nav_next(bound);
            }
            _ => {}
        }
    }
}

pub fn up(state: &mut AppState) {
    let active_idx = state.active_tab;
    let Some(tab) = state.tabs.get_mut(active_idx) else {
        return;
    };
    if let Some(pane) = tab.tree.active_mut() {
        match pane.kind {
            PaneType::TableList | PaneType::SchemaPicker => pane.nav_prev(),
            PaneType::TableView | PaneType::QueryResults => {
                let loaded_rows = pane_data(&state.table_cache, &tab.query_results, pane)
                    .map_or(0, |(_, rows, _)| rows.len());
                let bound = if pane.kind == PaneType::TableView {
                    pane.total_table_rows(loaded_rows)
                } else {
                    loaded_rows
                };
                if bound > 0 {
                    pane.row_prev();
                }
            }
            PaneType::SchemaView => {
                let bound = pane
                    .bound_table
                    .as_ref()
                    .and_then(|name| state.table_cache.get(name))
                    .map(|lt| lt.schema.len())
                    .unwrap_or(0);
                if bound > 0 {
                    pane.nav_prev();
                }
            }
            _ => {}
        }
    }
}

pub fn left(state: &mut AppState) {
    let active_idx = state.active_tab;
    let Some(tab) = state.tabs.get_mut(active_idx) else {
        return;
    };
    if let Some(pane) = tab.tree.active_mut() {
        if (pane.kind == PaneType::TableView || pane.kind == PaneType::QueryResults)
            && pane.cursor_col > 0
        {
            pane.col_left();
        }
    }
}

pub fn right(state: &mut AppState) {
    let active_idx = state.active_tab;
    let Some(tab) = state.tabs.get_mut(active_idx) else {
        return;
    };
    if let Some(pane) = tab.tree.active_mut() {
        if pane.kind == PaneType::TableView || pane.kind == PaneType::QueryResults {
            let bound = pane_data(&state.table_cache, &tab.query_results, pane)
                .map_or(0, |(headers, _, _)| headers.len());
            pane.col_right(bound);
        }
    }
}

pub fn bottom(state: &mut AppState, tables: &[String]) {
    let active_idx = state.active_tab;
    let Some(tab) = state.tabs.get_mut(active_idx) else {
        return;
    };
    if let Some(pane) = tab.tree.active_mut() {
        match pane.kind {
            PaneType::TableList | PaneType::SchemaPicker => pane.nav_bottom(tables.len()),
            PaneType::TableView | PaneType::QueryResults => {
                let loaded_rows = pane_data(&state.table_cache, &tab.query_results, pane)
                    .map_or(0, |(_, rows, _)| rows.len());
                let bound = if pane.kind == PaneType::TableView {
                    pane.total_table_rows(loaded_rows)
                } else {
                    loaded_rows
                };
                pane.row_bottom(bound);
            }
            PaneType::SchemaView => {
                let bound = pane
                    .bound_table
                    .as_ref()
                    .and_then(|name| state.table_cache.get(name))
                    .map(|lt| lt.schema.len())
                    .unwrap_or(0);
                pane.nav_bottom(bound);
            }
            _ => {}
        }
    }
}

pub fn history_back(state: &mut AppState) {
    let active_idx = state.active_tab;
    let Some(tab) = state.tabs.get_mut(active_idx) else {
        return;
    };
    if let Some(pane) = tab.tree.active_mut() {
        if pane.go_back() {
            if pane.kind == PaneType::TableView {
                if let Some(name) = pane.bound_table.clone() {
                    if !state.table_cache.contains_key(&name) {
                        tab.pending_load = Some(crate::tui::state::tab::PendingQuery {
                            table: name,
                            filter: None,
                            sort_col: None,
                            sort_desc: false,
                            selected_cols: None,
                        });
                        tab.loading = true;
                        tab.error = None;
                    }
                }
            }
        } else {
            state.cmdline.set_error("no previous view");
        }
    }
}

pub fn history_forward(state: &mut AppState) {
    let active_idx = state.active_tab;
    let Some(tab) = state.tabs.get_mut(active_idx) else {
        return;
    };
    if let Some(pane) = tab.tree.active_mut() {
        if pane.go_forward() {
            if pane.kind == PaneType::TableView {
                if let Some(name) = pane.bound_table.clone() {
                    if !state.table_cache.contains_key(&name) {
                        tab.pending_load = Some(crate::tui::state::tab::PendingQuery {
                            table: name,
                            filter: None,
                            sort_col: None,
                            sort_desc: false,
                            selected_cols: None,
                        });
                        tab.loading = true;
                        tab.error = None;
                    }
                }
            }
        } else {
            state.cmdline.set_error("no next view");
        }
    }
}

pub fn enter(state: &mut AppState, tables: &[String]) {
    let active_idx = state.active_tab;
    let Some(tab) = state.tabs.get_mut(active_idx) else {
        return;
    };
    if let Some(pane) = tab.tree.active_mut() {
        if (pane.kind == PaneType::TableList || pane.kind == PaneType::SchemaPicker) && !tab.loading
        {
            if let Some(name) = tables.get(pane.nav_cursor).cloned() {
                if pane.kind == PaneType::SchemaPicker {
                    pane.set_schema_view(name.clone());
                    pane.last_search = None;
                } else {
                    // Convert the active pane to a TableView bound to this table.
                    pane.set_table_view(name.clone());
                    pane.last_search = None; // clear search highlight
                }

                // If not cached, trigger an async load.
                if !state.table_cache.contains_key(&name) {
                    tab.pending_load = Some(crate::tui::state::tab::PendingQuery {
                        table: name,
                        filter: None,
                        sort_col: None,
                        sort_desc: false,
                        selected_cols: None,
                    });
                    tab.loading = true;
                    tab.error = None;
                }
            }
        }
    }
}
