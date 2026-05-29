use crate::tui::{AppState, SearchDirection, state::pane_layout::LiveSearchState};

/// Compute live fuzzy matches while the user is typing in / or ?.
/// Stores the result in `pane.live_search` without moving the cursor.
pub fn compute_live_search(direction: SearchDirection, state: &mut AppState) {
    let query = state.cmdline.input.trim().to_string();
    let query_lower = query.to_lowercase();

    // Collect needed info from the active pane first.
    let pane_info = state.active_tab_mut().and_then(|tab| {
        tab.tree.active_mut().map(|pane| {
            (
                pane.kind.clone(),
                pane.bound_table.clone(),
                pane.bound_query_idx,
                pane.cursor_col,
            )
        })
    });
    let Some((kind, bound_table, bound_query_idx, cursor_col)) = pane_info else {
        return;
    };

    let matches = if query.is_empty() {
        vec![]
    } else {
        match kind {
            crate::tui::state::PaneType::TableList | crate::tui::state::PaneType::SchemaPicker => {
                state
                    .tables
                    .iter()
                    .enumerate()
                    .filter(|(_, name)| name.to_lowercase().contains(&query_lower))
                    .map(|(i, _)| i)
                    .collect()
            }
            crate::tui::state::PaneType::TableView => {
                let Some(ref table_name) = bound_table else {
                    return;
                };
                let Some(ref loaded) = state.table_cache.get(table_name) else {
                    return;
                };
                loaded
                    .rows
                    .iter()
                    .enumerate()
                    .filter(|(_, row)| {
                        row.get(cursor_col)
                            .map_or(false, |cell| cell.to_lowercase().contains(&query_lower))
                    })
                    .map(|(i, _)| i)
                    .collect()
            }
            crate::tui::state::PaneType::QueryResults => {
                let Some(idx) = bound_query_idx else { return };
                let result = state
                    .active_tab()
                    .and_then(|tab| tab.query_results.get(idx).cloned());
                let Some(result) = result else { return };
                result
                    .rows
                    .iter()
                    .enumerate()
                    .filter(|(_, row)| {
                        row.get(cursor_col)
                            .map_or(false, |cell| cell.to_lowercase().contains(&query_lower))
                    })
                    .map(|(i, _)| i)
                    .collect()
            }
            _ => vec![],
        }
    };

    let Some(tab) = state.active_tab_mut() else {
        return;
    };
    let Some(pane) = tab.tree.active_mut() else {
        return;
    };
    pane.live_search = Some(LiveSearchState {
        query: query.clone(),
        direction,
        matches,
    });
}

pub fn commit_search(query: &str, direction: SearchDirection, state: &mut AppState) {
    let query_lower = query.to_lowercase();

    // Collect all pane info first.
    let pane_info = state.active_tab_mut().and_then(|tab| {
        tab.tree.active_mut().map(|pane| {
            pane.live_search = None;
            (
                pane.kind.clone(),
                pane.nav_cursor,
                pane.row_cursor,
                pane.cursor_col,
                pane.bound_table.clone(),
                pane.bound_query_idx,
            )
        })
    });
    let Some((kind, nav_cursor, row_cursor, cursor_col, bound_table, bound_query_idx)) = pane_info
    else {
        return;
    };

    match kind {
        crate::tui::state::PaneType::TableList | crate::tui::state::PaneType::SchemaPicker => {
            let matches: Vec<usize> = state
                .tables
                .iter()
                .enumerate()
                .filter(|(_, name)| name.to_lowercase().contains(&query_lower))
                .map(|(i, _)| i)
                .collect();

            if matches.is_empty() {
                state
                    .cmdline
                    .set_error(format!("Pattern not found: {query}"));
                return;
            }

            let current_idx = match direction {
                SearchDirection::Forward => {
                    matches.iter().position(|&m| m >= nav_cursor).unwrap_or(0)
                }
                SearchDirection::Backward => matches
                    .iter()
                    .rposition(|&m| m <= nav_cursor)
                    .unwrap_or(matches.len() - 1),
            };

            let Some(tab) = state.active_tab_mut() else {
                return;
            };
            let Some(pane) = tab.tree.active_mut() else {
                return;
            };
            pane.nav_cursor = matches[current_idx];
            pane.last_search = Some(crate::tui::SearchState {
                query: query.to_string(),
                direction,
                matches,
                current_idx,
            });
        }
        crate::tui::state::PaneType::TableView => {
            let Some(ref table_name) = bound_table else {
                state.cmdline.set_error("no table bound");
                return;
            };
            let Some(ref loaded) = state.table_cache.get(table_name) else {
                state.cmdline.set_error("table not loaded");
                return;
            };

            let matches: Vec<usize> = loaded
                .rows
                .iter()
                .enumerate()
                .filter(|(_, row)| {
                    row.get(cursor_col)
                        .map_or(false, |cell| cell.to_lowercase().contains(&query_lower))
                })
                .map(|(i, _)| i)
                .collect();

            if matches.is_empty() {
                state
                    .cmdline
                    .set_error(format!("Pattern not found: {query}"));
                return;
            }

            let current_idx = match direction {
                SearchDirection::Forward => {
                    matches.iter().position(|&m| m >= row_cursor).unwrap_or(0)
                }
                SearchDirection::Backward => matches
                    .iter()
                    .rposition(|&m| m <= row_cursor)
                    .unwrap_or(matches.len() - 1),
            };

            let Some(tab) = state.active_tab_mut() else {
                return;
            };
            let Some(pane) = tab.tree.active_mut() else {
                return;
            };
            pane.row_cursor = matches[current_idx];
            pane.last_search = Some(crate::tui::SearchState {
                query: query.to_string(),
                direction,
                matches,
                current_idx,
            });
        }
        crate::tui::state::PaneType::QueryResults => {
            let Some(idx) = bound_query_idx else {
                state.cmdline.set_error("no result set bound");
                return;
            };
            let result = state
                .active_tab()
                .and_then(|tab| tab.query_results.get(idx).cloned());
            let Some(result) = result else {
                state.cmdline.set_error("result set not available");
                return;
            };

            let matches: Vec<usize> = result
                .rows
                .iter()
                .enumerate()
                .filter(|(_, row)| {
                    row.get(cursor_col)
                        .map_or(false, |cell| cell.to_lowercase().contains(&query_lower))
                })
                .map(|(i, _)| i)
                .collect();

            if matches.is_empty() {
                state
                    .cmdline
                    .set_error(format!("Pattern not found: {query}"));
                return;
            }

            let current_idx = match direction {
                SearchDirection::Forward => {
                    matches.iter().position(|&m| m >= row_cursor).unwrap_or(0)
                }
                SearchDirection::Backward => matches
                    .iter()
                    .rposition(|&m| m <= row_cursor)
                    .unwrap_or(matches.len() - 1),
            };

            let Some(tab) = state.active_tab_mut() else {
                return;
            };
            let Some(pane) = tab.tree.active_mut() else {
                return;
            };
            pane.row_cursor = matches[current_idx];
            pane.last_search = Some(crate::tui::SearchState {
                query: query.to_string(),
                direction,
                matches,
                current_idx,
            });
        }
        _ => {
            state
                .cmdline
                .set_error("Search only supported in table list, table view, and query results");
        }
    }
}
