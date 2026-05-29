use crate::tui::AppState;

pub fn parse_pane_type(arg: Option<&&str>) -> crate::tui::state::PaneType {
    match arg {
        Some(s) if s.eq_ignore_ascii_case("table") => crate::tui::state::PaneType::TableView,
        Some(s) if s.eq_ignore_ascii_case("schema") => crate::tui::state::PaneType::SchemaView,
        Some(s) if s.eq_ignore_ascii_case("sql") || s.eq_ignore_ascii_case("query") => {
            crate::tui::state::PaneType::QueryEditor
        }
        Some(s) if s.eq_ignore_ascii_case("queryresults") => {
            crate::tui::state::PaneType::QueryResults
        }
        _ => crate::tui::state::PaneType::TableList,
    }
}

pub fn cmd_vnew(state: &mut AppState, args: &[&str]) {
    let kind = parse_pane_type(args.first());
    let table_name = args.get(1).map(|s| s.to_string());

    if kind == crate::tui::state::PaneType::QueryResults {
        state
            .cmdline
            .set_error("cannot create empty query results pane; use :queryResults or Ctrl+Enter");
        return;
    }

    if let Some(ref name) = table_name {
        if !state.tables.contains(name) {
            state.cmdline.set_error(format!("table `{name}` not found"));
            return;
        }
    }

    let cache_has = table_name
        .as_ref()
        .map(|name| state.table_cache.contains_key(name));

    let split_kind = if kind == crate::tui::state::PaneType::SchemaView && table_name.is_none() {
        crate::tui::state::PaneType::SchemaPicker
    } else {
        kind
    };

    let Some(tab) = state.active_tab_mut() else {
        state.cmdline.set_error("not in dashboard");
        return;
    };

    match tab.tree.split_active_v(split_kind) {
        Ok(id) => {
            if let Some(table) = table_name {
                if let Some(pane) = tab.tree.panes.get_mut(&id) {
                    match pane.kind {
                        crate::tui::state::PaneType::TableView => {
                            pane.set_table_view(table.clone());
                            if !cache_has.unwrap_or(false) {
                                tab.pending_load = Some(crate::tui::state::tab::PendingQuery {
                                    table,
                                    filter: None,
                                    sort_col: None,
                                    sort_desc: false,
                                    selected_cols: None,
                                });
                                tab.loading = true;
                                tab.error = None;
                            }
                        }
                        crate::tui::state::PaneType::SchemaView => {
                            pane.set_schema_view(table.clone());
                            if !cache_has.unwrap_or(false) {
                                tab.pending_load = Some(crate::tui::state::tab::PendingQuery {
                                    table,
                                    filter: None,
                                    sort_col: None,
                                    sort_desc: false,
                                    selected_cols: None,
                                });
                                tab.loading = true;
                                tab.error = None;
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
        Err(e) => state.cmdline.set_error(e),
    }
}

pub fn cmd_hnew(state: &mut AppState, args: &[&str]) {
    let kind = parse_pane_type(args.first());
    let table_name = args.get(1).map(|s| s.to_string());

    if kind == crate::tui::state::PaneType::QueryResults {
        state
            .cmdline
            .set_error("cannot create empty query results pane; use :queryResults or Ctrl+Enter");
        return;
    }

    if let Some(ref name) = table_name {
        if !state.tables.contains(name) {
            state.cmdline.set_error(format!("table `{name}` not found"));
            return;
        }
    }

    let cache_has = table_name
        .as_ref()
        .map(|name| state.table_cache.contains_key(name));

    let split_kind = if kind == crate::tui::state::PaneType::SchemaView && table_name.is_none() {
        crate::tui::state::PaneType::SchemaPicker
    } else {
        kind
    };

    let Some(tab) = state.active_tab_mut() else {
        state.cmdline.set_error("not in dashboard");
        return;
    };

    match tab.tree.split_active_h(split_kind) {
        Ok(id) => {
            if let Some(table) = table_name {
                if let Some(pane) = tab.tree.panes.get_mut(&id) {
                    match pane.kind {
                        crate::tui::state::PaneType::TableView => {
                            pane.set_table_view(table.clone());
                            if !cache_has.unwrap_or(false) {
                                tab.pending_load = Some(crate::tui::state::tab::PendingQuery {
                                    table,
                                    filter: None,
                                    sort_col: None,
                                    sort_desc: false,
                                    selected_cols: None,
                                });
                                tab.loading = true;
                                tab.error = None;
                            }
                        }
                        crate::tui::state::PaneType::SchemaView => {
                            pane.set_schema_view(table.clone());
                            if !cache_has.unwrap_or(false) {
                                tab.pending_load = Some(crate::tui::state::tab::PendingQuery {
                                    table,
                                    filter: None,
                                    sort_col: None,
                                    sort_desc: false,
                                    selected_cols: None,
                                });
                                tab.loading = true;
                                tab.error = None;
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
        Err(e) => state.cmdline.set_error(e),
    }
}

pub fn cmd_show(state: &mut AppState, args: &[&str]) {
    let table_name = args.first().map(|s| s.to_string());
    let cache_has = table_name
        .as_ref()
        .map(|name| state.table_cache.contains_key(name));

    let Some(tab) = state.active_tab_mut() else {
        state.cmdline.set_error("not in dashboard");
        return;
    };

    if let Some(pane) = tab.tree.active_mut() {
        if let Some(name) = table_name {
            pane.set_table_view(name.clone());
            pane.last_search = None; // clear search highlight
            if !cache_has.unwrap_or(false) {
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
        } else {
            state.cmdline.set_error(":show requires a table name");
        }
    }
}

pub fn cmd_tables(state: &mut AppState, _args: &[&str]) {
    let Some(tab) = state.active_tab_mut() else {
        state.cmdline.set_error("not in dashboard");
        return;
    };

    if let Some(pane) = tab.tree.active_mut() {
        pane.reset_to_list();
        pane.last_search = None; // clear search highlight
    }
}

pub fn cmd_noh(state: &mut AppState) {
    let Some(tab) = state.active_tab_mut() else {
        state.cmdline.set_error("not in dashboard");
        return;
    };

    if let Some(pane) = tab.tree.active_mut() {
        pane.last_search = None;
        pane.live_search = None;
    }
}

pub fn cmd_schema(state: &mut AppState, args: &[&str]) {
    let table_name = args.first().map(|s| s.to_string());
    let cache_has = table_name
        .as_ref()
        .map(|name| state.table_cache.contains_key(name));

    let Some(tab) = state.active_tab_mut() else {
        state.cmdline.set_error("not in dashboard");
        return;
    };

    if let Some(pane) = tab.tree.active_mut() {
        if let Some(name) = table_name {
            pane.set_schema_view(name.clone());
            pane.last_search = None;
            if !cache_has.unwrap_or(false) {
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
        } else {
            pane.set_schema_picker();
            pane.last_search = None;
        }
    }
}

pub fn cmd_sql(state: &mut AppState, _args: &[&str]) {
    let Some(tab) = state.active_tab_mut() else {
        state.cmdline.set_error("not in dashboard");
        return;
    };

    if let Some(pane) = tab.tree.active_mut() {
        pane.set_query_editor();
    }
}

pub fn cmd_query_results(state: &mut AppState, _args: &[&str]) {
    let query_results_empty = state
        .active_tab()
        .map_or(true, |tab| tab.query_results.is_empty());
    if query_results_empty {
        state.cmdline.set_error("no query results available");
        return;
    }

    let Some(tab) = state.active_tab_mut() else {
        state.cmdline.set_error("not in dashboard");
        return;
    };

    let count = tab.query_results.len();
    if let Some(pane) = tab.tree.active_mut() {
        pane.set_query_results(0);
        pane.query_result_count = count;
    }
}

pub fn cmd_close(state: &mut AppState, args: &[&str]) {
    let Some(tab) = state.active_tab_mut() else {
        state.cmdline.set_error("not in dashboard");
        return;
    };

    let closed = if args.is_empty() {
        tab.tree.close_active()
    } else if let Ok(id) = args[0].parse::<usize>() {
        tab.tree.close_by_display_id(id)
    } else {
        state
            .cmdline
            .set_error(format!("invalid pane id `{}`", args[0]));
        return;
    };

    if closed {
        state.mode = crate::tui::state::AppMode::Home;
        state.tabs = vec![];
        state.active_tab = 0;
    }
}
