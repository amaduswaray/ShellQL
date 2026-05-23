use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::Paragraph,
};

use crate::tui::state::{AppState, CommandLineMode};

pub fn render(frame: &mut Frame, area: Rect, state: &AppState) {
    let input = &state.cmdline.input;

    let prefix = if let CommandLineMode::CellEdit { ref col_name, .. } = state.cmdline.mode {
        format!("EDIT {col_name}: ")
    } else {
        "EDIT ".to_string()
    };

    let line = Line::from(vec![
        Span::styled(prefix.clone(), Style::default().fg(Color::Green).bold()),
        Span::styled(input.clone(), Style::default().fg(Color::White)),
    ]);

    frame.render_widget(Paragraph::new(vec![line]), area);

    let cursor_x =
        (area.x + prefix.len() as u16 + input.len() as u16).min(area.right().saturating_sub(1));
    frame.set_cursor_position((cursor_x, area.y));
}
