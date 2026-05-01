use ratatui::Frame;

use crate::tui::state::{AppMode, AppState};

pub fn render(frame: &mut Frame, state: &AppState) {
    match state.mode {
        AppMode::Home => {
            frame.render_widget("hello world", frame.area());
        }
        AppMode::Dashboard => {
            frame.render_widget("hello world", frame.area());
        }
    }
}
