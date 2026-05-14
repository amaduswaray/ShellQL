pub mod add_connection;
pub mod command_palette;
pub mod connection_picker;
pub mod help;

pub use add_connection::{
    goto_bottom, goto_top, remove_selected, render_connection_list,
    render_empty_connections, select_next, select_prev, selected_connection,
    visible_text,
};
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Paragraph},
};

use crate::tui::{
    AppState, Overlay,
    ui::home::overlays::{
        add_connection::render_add_connection, command_palette::render_command_palette,
        connection_picker::render_connection_picker, help::{render_help, render_dashboard_help},
    },
};

pub fn render_overlay(frame: &mut Frame, area: Rect, state: &AppState) {
    let Some(overlay) = state.overlay else { return };
    match overlay {
        Overlay::Help => render_help(frame, area),
        Overlay::DashboardHelp => render_dashboard_help(frame, area),
        Overlay::AddConnection => render_add_connection(frame, area, state),
        Overlay::CommandPalette => render_command_palette(frame, area),
        Overlay::ConnectionPicker => render_connection_picker(frame, area, state),
        Overlay::ConfirmDelete => {}
    }
}

pub fn open_popup<'a>(frame: &mut Frame, area: Rect, title: &'a str) -> (Block<'a>, Rect) {
    frame.render_widget(Clear, area);

    let block = Block::default()
        .title(Line::from(format!(" {title} ")).style(Style::default().fg(Color::Blue).bold()))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::White))
        .style(Style::default().bg(Color::Reset));

    let inner = block.inner(area);

    (block, inner)
}

/// Render a right-aligned dim hint line inside a popup's inner area.
pub fn render_dismiss_hint(frame: &mut Frame, area: Rect, hint: &str) {
    let line = Line::from(Span::styled(hint, Style::default().fg(Color::DarkGray))).right_aligned();
    frame.render_widget(Paragraph::new(vec![line]), area);
}

/// Build a two-column key → description line for the help overlay.
pub fn binding_line<'a>(key: &'a str, desc: &'a str, key_style: Style, desc_style: Style) -> Line<'a> {
    Line::from(vec![
        Span::styled(format!("{key:<14}"), key_style),
        Span::styled(desc, desc_style),
    ])
}
