pub mod overlays;

pub use overlays::{
    binding_line, goto_bottom, goto_top, open_popup, remove_selected, render_connection_list,
    render_dismiss_hint, render_empty_connections, select_next, select_prev, selected_connection,
};
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
};

use crate::tui::{state::AppState, ui::home::overlays::render_overlay};

pub fn render_home(frame: &mut Frame, area: Rect, state: &AppState) {
    let content_h: u16 = 10;

    let [_, horiz, _] = Layout::horizontal([
        Constraint::Fill(1),
        Constraint::Percentage(40),
        Constraint::Fill(1),
    ])
    .areas(area);

    let [_, center, _] = Layout::vertical([
        Constraint::Fill(1),
        Constraint::Length(content_h),
        Constraint::Fill(1),
    ])
    .areas(horiz);

    render_landing(frame, center);

    if state.overlay.is_some() {
        render_overlay(frame, area, state);
    }
}

fn render_landing(frame: &mut Frame, area: Rect) {
    let width = area.width as usize;
    let sep = Span::styled("─".repeat(width), Style::default().fg(Color::DarkGray));

    // (command_name, single_key, description)
    let items: &[(&str, &str)] = &[
        ("connect", "connect to one of your DBs"),
        ("add", "add a new DB connection"),
        ("help", "for help"),
        ("q", "to quit"),
    ];

    let mut lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            "ShellQL",
            Style::default()
                .fg(Color::Blue)
                .add_modifier(Modifier::BOLD),
        ))
        .centered(),
        Line::from(Span::styled(
            "SQL Database Manager",
            Style::default().fg(Color::Gray),
        ))
        .centered(),
        Line::from(""),
        Line::from(sep.clone()),
    ];

    let max_cmd_len = items.iter().map(|(cmd, _)| cmd.len()).max().unwrap_or(0);

    for (cmd, desc) in items {
        lines.push(instruction_line(cmd, desc, max_cmd_len));
    }

    lines.push(Line::from(sep));

    frame.render_widget(Paragraph::new(lines), area);
}

fn instruction_line(cmd: &str, desc: &str, max_cmd_len: usize) -> Line<'static> {
    // <Enter> sits directly after the command.
    // Pad after <Enter> so every description starts at the same column.
    let after_pad = max_cmd_len.saturating_sub(cmd.len()) + 3; // 3 = minimum gap

    Line::from(vec![
        Span::styled("type  ".to_string(), Style::default().fg(Color::White)),
        Span::styled(
            ":".to_string(),
            Style::default()
                .fg(Color::Blue)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(cmd.to_string(), Style::default().fg(Color::White)),
        Span::styled(
            "<Enter>".to_string(),
            Style::default()
                .fg(Color::Blue)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" ".repeat(after_pad)),
        Span::styled(desc.to_string(), Style::default().fg(Color::White)),
    ])
}
