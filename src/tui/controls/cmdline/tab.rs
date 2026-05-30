use crate::tui::AppState;

const MAX_TABS: usize = 10;

pub fn cmd_tab_new(state: &mut AppState, _args: &[&str]) {
    if state.tabs.len() >= MAX_TABS {
        state.cmdline.set_error("maximum 10 tabs");
        return;
    }
    if !state.has_session() {
        state.cmdline.set_error("not connected");
        return;
    }
    state.tabs.push(crate::tui::state::Tab::new());
    state.active_tab = state.tabs.len() - 1;
}

pub fn cmd_tab_next(state: &mut AppState) {
    if !state.has_session() {
        state.cmdline.set_error("not connected");
        return;
    }
    if state.tabs.len() <= 1 {
        return;
    }
    state.active_tab = (state.active_tab + 1) % state.tabs.len();
}

pub fn cmd_tab_prev(state: &mut AppState) {
    if !state.has_session() {
        state.cmdline.set_error("not connected");
        return;
    }
    if state.tabs.len() <= 1 {
        return;
    }
    state.active_tab = (state.active_tab + state.tabs.len() - 1) % state.tabs.len();
}

pub fn cmd_tab_delete(state: &mut AppState) {
    if !state.has_session() {
        state.cmdline.set_error("not connected");
        return;
    }
    if state.tabs.len() <= 1 {
        // Last tab — disconnect entirely.
        state.tabs.clear();
        state.active_tab = 0;
        state.connection = None;
        state.pool = None;
        state.tables.clear();
        state.table_cache.clear();
        state.mode = crate::tui::state::AppMode::Home;
        state.cmdline.reset();
        return;
    }
    state.tabs.remove(state.active_tab);
    if state.active_tab >= state.tabs.len() {
        state.active_tab = state.tabs.len() - 1;
    }
}

pub fn cmd_tab_goto(state: &mut AppState, args: &[&str]) {
    if !state.has_session() {
        state.cmdline.set_error("not connected");
        return;
    }
    let id = match args.first() {
        Some(s) => match s.parse::<usize>() {
            Ok(v) => v,
            Err(_) => {
                state.cmdline.set_error("usage: :tab <id>");
                return;
            }
        },
        None => {
            state.cmdline.set_error("usage: :tab <id>");
            return;
        }
    };
    if id >= state.tabs.len() {
        state.cmdline.set_error(format!("invalid tab id `{id}`"));
        return;
    }
    state.active_tab = id;
}

/// Unified tab command:
/// :tab new|next|prev|close|<id>
pub fn cmd_tab(state: &mut AppState, args: &[&str]) {
    let Some(first) = args.first().copied() else {
        state
            .cmdline
            .set_error("usage: :tab <new|next|prev|close|id>");
        return;
    };

    if first.parse::<usize>().is_ok() {
        let arg = [first];
        cmd_tab_goto(state, &arg);
        return;
    }

    match first {
        s if s.eq_ignore_ascii_case("new") => cmd_tab_new(state, &[]),
        s if s.eq_ignore_ascii_case("next") => cmd_tab_next(state),
        s if s.eq_ignore_ascii_case("prev") || s.eq_ignore_ascii_case("previous") => {
            cmd_tab_prev(state)
        }
        s if s.eq_ignore_ascii_case("close") || s.eq_ignore_ascii_case("delete") => {
            cmd_tab_delete(state)
        }
        _ => state
            .cmdline
            .set_error("usage: :tab <new|next|prev|close|id>"),
    }
}
