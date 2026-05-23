use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::Paragraph,
};

use crate::tui::state::ConfirmAction;

pub fn render(frame: &mut Frame, area: Rect, action: &ConfirmAction, input: &str) {
    match action {
        ConfirmAction::DeleteConnection(name) => render_delete(frame, area, name, input),
        ConfirmAction::CommitWrites {
            table,
            update_count,
            delete_count,
        } => render_commit(frame, area, table, *update_count, *delete_count, input),
    }
}

fn render_delete(frame: &mut Frame, area: Rect, name: &str, input: &str) {
    let prefix_spans: Vec<Span> = vec![
        Span::styled("Delete ", Style::default().fg(Color::Red)),
        Span::styled(
            format!("\"{}\"", name),
            Style::default().fg(Color::Red).bold(),
        ),
        Span::styled("? ", Style::default().fg(Color::Red)),
        Span::styled("[y/n]: ", Style::default().fg(Color::DarkGray)),
    ];

    let prefix_width: u16 = prefix_spans.iter().map(|s| s.content.len() as u16).sum();

    let mut spans = prefix_spans;
    spans.push(Span::styled(
        input.to_string(),
        Style::default().fg(Color::White),
    ));

    frame.render_widget(Paragraph::new(vec![Line::from(spans)]), area);

    let cursor_x = (area.x + prefix_width + input.len() as u16).min(area.right().saturating_sub(1));
    frame.set_cursor_position((cursor_x, area.y));
}

fn render_commit(
    frame: &mut Frame,
    area: Rect,
    table: &str,
    update_count: usize,
    delete_count: usize,
    input: &str,
) {
    let mut parts = vec![Span::styled("Commit ", Style::default().fg(Color::Yellow))];
    if update_count > 0 {
        parts.push(Span::styled(
            format!(
                "{update_count} update{} ",
                if update_count == 1 { "" } else { "s" }
            ),
            Style::default().fg(Color::Yellow).bold(),
        ));
    }
    if delete_count > 0 {
        parts.push(Span::styled(
            format!(
                "{delete_count} deletion{} ",
                if delete_count == 1 { "" } else { "s" }
            ),
            Style::default().fg(Color::Red).bold(),
        ));
    }
    parts.push(Span::styled(
        format!("to `{table}`"),
        Style::default().fg(Color::White),
    ));
    parts.push(Span::styled("? ", Style::default().fg(Color::Yellow)));
    parts.push(Span::styled(
        "[y/n]: ",
        Style::default().fg(Color::DarkGray),
    ));

    let prefix_width: u16 = parts.iter().map(|s| s.content.len() as u16).sum();

    let mut spans = parts;
    spans.push(Span::styled(
        input.to_string(),
        Style::default().fg(Color::White),
    ));

    frame.render_widget(Paragraph::new(vec![Line::from(spans)]), area);

    let cursor_x = (area.x + prefix_width + input.len() as u16).min(area.right().saturating_sub(1));
    frame.set_cursor_position((cursor_x, area.y));
}
