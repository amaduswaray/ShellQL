use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Color, Style},
    text::Line,
    widgets::{Block, BorderType, Borders, Clear},
};

use crate::tui::{
    AppState,
    ui::{
        centered_rect,
        home::{render_connection_list, render_dismiss_hint, render_empty_connections},
    },
};

pub fn render_connection_picker(frame: &mut Frame, area: Rect, state: &AppState) {
    let popup = centered_rect(40, 30, area);
    frame.render_widget(Clear, popup);

    let block = Block::default()
        .title(Line::from(" Connections ").style(Style::default().fg(Color::White).bold()))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::White))
        .style(Style::default().bg(Color::Reset));

    let inner = block.inner(popup);
    frame.render_widget(block, popup);

    let [list_area, hint_area] =
        Layout::vertical([Constraint::Min(1), Constraint::Length(1)]).areas(inner);

    if state.connections.is_empty() {
        render_empty_connections(frame, list_area);
    } else {
        render_connection_list(frame, list_area, state);
    }

    render_dismiss_hint(frame, hint_area, "j/k <nav>   ↵ <connect>   esc/q <close> ");
}
