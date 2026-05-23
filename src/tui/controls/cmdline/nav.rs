use crate::tui::{
    AddConnectionForm, AppMode, AppState, ConfirmAction, Overlay, ui::home::selected_connection,
};

pub fn cmd_exit(state: &mut AppState) {
    state.should_quit = true;
}

pub fn cmd_quit(state: &mut AppState, args: &[&str]) {
    if let Some(ref tab) = state.active_tab() {
        if tab.tree.pane_count() <= 1 {
            state.should_quit = true;
        } else {
            super::pane::cmd_close(state, args);
        }
    } else {
        state.should_quit = true;
    }
}

pub fn cmd_help(state: &mut AppState) {
    let overlay = if state.mode == AppMode::Dashboard {
        Overlay::DashboardHelp
    } else {
        Overlay::Help
    };
    state.overlay = Some(overlay);
}

pub fn cmd_add(state: &mut AppState) {
    state.overlay = Some(Overlay::AddConnection);
    state.form = Some(AddConnectionForm::new());
}

pub fn cmd_connect(state: &mut AppState) {
    state.overlay = Some(Overlay::ConnectionPicker);
}

pub fn cmd_disconnect(state: &mut AppState) {
    if state.tabs.is_empty() {
        state.cmdline.set_error("not connected");
        return;
    }
    state.tabs = vec![];
    state.active_tab = 0;
    state.mode = AppMode::Home;
    state.cmdline.reset();
}

pub fn cmd_delete(state: &mut AppState) {
    if let Some(db) = selected_connection(state) {
        let name = db.name.clone();
        state
            .cmdline
            .open_confirm(ConfirmAction::DeleteConnection(name));
    } else {
        state.cmdline.set_error("no connection selected");
    }
}

/// Navigate back in the active pane's view history.
pub fn cmd_back(state: &mut AppState) {
    let result = state.active_tab_mut().and_then(|tab| {
        tab.tree.active_mut().map(|pane| {
            let went_back = pane.go_back();
            let needs_load = if went_back && pane.kind == crate::tui::state::PaneType::TableView {
                pane.bound_table.clone()
            } else {
                None
            };
            (went_back, needs_load)
        })
    });

    let Some((went_back, table_name)) = result else {
        state.cmdline.set_error("not in dashboard");
        return;
    };

    if went_back {
        if let Some(name) = table_name {
            let cache_has = state.table_cache.contains_key(&name);
            let Some(tab) = state.active_tab_mut() else {
                return;
            };
            if !cache_has {
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
    } else {
        state.cmdline.set_error("no previous view");
    }
}

/// Navigate forward in the active pane's view history.
pub fn cmd_forward(state: &mut AppState) {
    let result = state.active_tab_mut().and_then(|tab| {
        tab.tree.active_mut().map(|pane| {
            let went_forward = pane.go_forward();
            let needs_load = if went_forward && pane.kind == crate::tui::state::PaneType::TableView
            {
                pane.bound_table.clone()
            } else {
                None
            };
            (went_forward, needs_load)
        })
    });

    let Some((went_forward, table_name)) = result else {
        state.cmdline.set_error("not in dashboard");
        return;
    };

    if went_forward {
        if let Some(name) = table_name {
            let cache_has = state.table_cache.contains_key(&name);
            let Some(tab) = state.active_tab_mut() else {
                return;
            };
            if !cache_has {
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
    } else {
        state.cmdline.set_error("no next view");
    }
}

pub fn cmd_resize(state: &mut AppState, args: &[&str]) {
    let Some(tab) = state.active_tab_mut() else {
        state.cmdline.set_error("not in dashboard");
        return;
    };

    if args.is_empty() {
        state.cmdline.set_error(":resize requires +N or -N");
        return;
    }

    let arg = args.join(" ");
    let delta = match arg.parse::<i32>() {
        Ok(v) if (-100..=100).contains(&v) => v,
        Ok(_) => {
            state
                .cmdline
                .set_error("resize value must be between -100 and 100");
            return;
        }
        Err(_) => {
            state
                .cmdline
                .set_error(format!("invalid resize value `{arg}`"));
            return;
        }
    };

    match tab.tree.resize_active(delta) {
        Ok(_) => {}
        Err(e) => state.cmdline.set_error(e),
    }
}

/// Toggle fullscreen (zoom) on the active pane. Like tmux `<prefix>z`.
pub fn cmd_fullscreen(state: &mut AppState) {
    let Some(tab) = state.active_tab_mut() else {
        state.cmdline.set_error("not in dashboard");
        return;
    };

    if tab.tree.pane_count() <= 1 {
        state
            .cmdline
            .set_error("only one pane — nothing to fullscreen");
        return;
    }

    tab.tree.toggle_fullscreen();
}

/// Execute SQL directly from the command line (like vim's `:!`).
/// Skips the query editor and immediately runs the statement.
pub fn cmd_bang(state: &mut AppState, args: &[&str]) {
    let sql = args.join(" ").trim().to_string();
    if sql.is_empty() {
        state.cmdline.set_error("! requires an SQL query");
        return;
    }

    let Some(tab) = state.active_tab_mut() else {
        state.cmdline.set_error("not in dashboard");
        return;
    };

    tab.pending_query_exec = Some(sql);
    tab.loading = true;
    tab.error = None;

    // Replace the active pane with a QueryResults view (no new pane created).
    let active_id = tab.tree.active_pane;
    if let Some(pane) = tab.tree.panes.get_mut(&active_id) {
        pane.set_query_results(0);
        pane.query_result_count = 1;
    }
}
