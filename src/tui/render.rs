use crossterm::{ExecutableCommand, cursor::SetCursorStyle};
use ratatui::{
    Frame,
    layout::{Constraint, Layout},
};

use crate::tui::{
    state::{AppMode, AppState, PaneType, TableMode},
    ui::{render_cmdline, render_dashboard, render_home},
};

pub fn render(frame: &mut Frame, state: &mut AppState) {
    // Reserve the bottom row for the persistent command-line bar.
    let [main_area, cmdline_area] =
        Layout::vertical([Constraint::Min(0), Constraint::Length(1)]).areas(frame.area());

    match state.mode {
        AppMode::Home => render_home(frame, main_area, state),
        AppMode::Dashboard => render_dashboard(frame, main_area, state),
    }

    render_cmdline(frame, cmdline_area, state);
    apply_cursor_style(state);
}

fn apply_cursor_style(state: &AppState) {
    let query_insert = state.mode == AppMode::Dashboard
        && state
            .active_tab()
            .and_then(|tab| tab.tree.panes.get(&tab.tree.active_pane))
            .is_some_and(|pane| {
                pane.kind == PaneType::QueryEditor && pane.mode == TableMode::Insert
            });

    let style = if query_insert {
        SetCursorStyle::SteadyBar
    } else {
        SetCursorStyle::SteadyBlock
    };

    let _ = std::io::stdout().execute(style);
}
