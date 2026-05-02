use ratatui::{
    Frame,
    layout::{Constraint, Layout},
};

use crate::tui::{
    state::{AppMode, AppState},
    ui::{render_cmdline, render_home},
};

pub fn render(frame: &mut Frame, state: &AppState) {
    // Reserve the bottom row for the persistent command-line bar.
    let [main_area, cmdline_area] = Layout::vertical([
        Constraint::Min(0),
        Constraint::Length(1),
    ])
    .areas(frame.area());

    match state.mode {
        AppMode::Home => render_home(frame, main_area, state),
        AppMode::Dashboard => {
            frame.render_widget("hello world", main_area);
        }
    }

    render_cmdline(frame, cmdline_area, state);
}
