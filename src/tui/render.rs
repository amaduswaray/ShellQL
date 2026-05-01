use ratatui::Frame;

use crate::tui::{
    state::{AppMode, AppState},
    ui::home::render_home,
};

pub fn render(frame: &mut Frame, state: &AppState) {
    match state.mode {
        AppMode::Home => render_home(frame, state),
        AppMode::Dashboard => {
            frame.render_widget("hello world", frame.area());
        }
    }
}
