use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::Paragraph,
};

use super::completions;
use crate::tui::state::AppState;

pub fn render(frame: &mut Frame, area: Rect, state: &AppState) {
    let input = &state.cmdline.input;

    let line = Line::from(vec![
        Span::styled(":", Style::default().fg(Color::White).bold()),
        Span::styled(input.clone(), Style::default().fg(Color::White)),
    ]);

    frame.render_widget(Paragraph::new(vec![line]), area);

    let cursor_char = state.cmdline.input_cursor as u16;
    let cursor_x = (area.x + 1 + cursor_char).min(area.right().saturating_sub(1));
    frame.set_cursor_position((cursor_x, area.y));

    // Completion popup floats above the bar when candidates are available.
    if !state.cmdline.completions.is_empty() {
        completions::render(
            frame,
            area,
            &state.cmdline.completions,
            state.cmdline.completion_selected,
        );
    }
}
