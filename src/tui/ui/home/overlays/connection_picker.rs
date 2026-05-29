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
        centered_rect::centered_rect_with_min,
        home::{render_connection_list, render_dismiss_hint, render_empty_connections},
    },
};

pub fn render_connection_picker(frame: &mut Frame, area: Rect, state: &AppState) {
    // Table columns: bullet(1) + spacing(1) + name + spacing(1) + badge(11)
    let max_name_w = state
        .connections
        .iter()
        .map(|db| db.name.len())
        .max()
        .unwrap_or(0) as u16;
    let min_w = (16 + max_name_w).min(area.width);
    let min_h = 7u16; // at least 3 lines + 2 borders + hint row
    let popup = centered_rect_with_min(40, 30, min_w, min_h, area);
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

    render_dismiss_hint(
        frame,
        hint_area,
        "j/k <nav>   ↵ <connect>   dd <delete>   esc/q <close> ",
    );
}
