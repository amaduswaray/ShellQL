use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::Paragraph,
};

use crate::tui::ui::{
    centered_rect,
    home::overlays::{binding_line, open_popup, render_dismiss_hint},
};

pub fn render_help(frame: &mut Frame, area: Rect) {
    let popup_area = centered_rect(46, 72, area);
    let (block, inner) = open_popup(frame, popup_area, "Help");
    frame.render_widget(block, popup_area);

    let [content_area, hint_area] =
        Layout::vertical([Constraint::Min(0), Constraint::Length(1)]).areas(inner);

    let nav_header = Style::default().fg(Color::Blue).bold();

    let key_style = Style::default().fg(Color::White).bold();

    let desc_style = Style::default().fg(Color::Gray);
    let sep_style = Style::default().fg(Color::DarkGray);

    let lines = vec![
        Line::from(""),
        Line::from(Span::styled("  Navigation", nav_header)),
        Line::from(Span::styled("  ──────────", sep_style)),
        binding_line("  j / ↓", "move down", key_style, desc_style),
        binding_line("  k / ↑", "move up", key_style, desc_style),
        Line::from(""),
        Line::from(Span::styled("  Actions", nav_header)),
        Line::from(Span::styled("  ───────", sep_style)),
        binding_line("  ↵", "connect", key_style, desc_style),
        binding_line("  a", "add connection", key_style, desc_style),
        binding_line("  d", "delete connection", key_style, desc_style),
        binding_line("  ?", "toggle this help", key_style, desc_style),
        binding_line("  :", "command line", key_style, desc_style),
        binding_line("  q", "quit", key_style, desc_style),
        binding_line("  Ctrl+C", "force quit", key_style, desc_style),
    ];

    frame.render_widget(Paragraph::new(lines), content_area);
    render_dismiss_hint(frame, hint_area, "Esc/q/?  <close> ");
}
