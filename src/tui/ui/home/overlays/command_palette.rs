use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::Paragraph,
};

use crate::tui::ui::{
    centered_rect,
    home::overlays::{open_popup, render_dismiss_hint},
};

pub fn render_command_palette(frame: &mut Frame, area: Rect) {
    let popup_area = centered_rect(55, 50, area);
    let (block, inner) = open_popup(frame, popup_area, "Command Palette");
    frame.render_widget(block, popup_area);

    let [content_area, hint_area] =
        Layout::vertical([Constraint::Min(0), Constraint::Length(1)]).areas(inner);

    let lines = vec![
        Line::from(vec![
            ratatui::text::Span::styled("  > ", Style::default().fg(Color::Blue).bold()),
            Span::styled("_", Style::default().fg(Color::White)),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "  Commands coming soon.",
            Style::default().fg(Color::DarkGray),
        )),
    ];

    frame.render_widget(Paragraph::new(lines), content_area);
    render_dismiss_hint(frame, hint_area, "Esc/q  <close> ");
}
