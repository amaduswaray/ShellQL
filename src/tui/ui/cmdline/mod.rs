/// Command-line bar — always rendered in the bottom row of the frame.
///
/// Three visual states:
///   Idle    —  [ HOME ]  2 connections            (status strip)
///   Input   —  :add█                              (vim : prompt)
///   Confirm —  Delete "prod"? [y/N]: █            (inline y/n)
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
};

use crate::tui::state::{
    AppMode, AppState,
    cmdline::{CommandLineMode, ConfirmAction},
};

// ── Entry point ───────────────────────────────────────────────────────────────

pub fn render_cmdline(frame: &mut Frame, area: Rect, state: &AppState) {
    match &state.cmdline.mode {
        CommandLineMode::Idle => render_idle(frame, area, state),
        CommandLineMode::Input => render_input(frame, area, state),
        CommandLineMode::Confirm(action) => {
            render_confirm(frame, area, action, &state.cmdline.input)
        }
    }
}

// ── Idle — status strip ───────────────────────────────────────────────────────

fn render_idle(frame: &mut Frame, area: Rect, state: &AppState) {
    if let Some(ref err) = state.cmdline.error {
        let line = Line::from(Span::styled(
            format!("{err}"),
            Style::default().fg(Color::Red),
        ));
        frame.render_widget(Paragraph::new(vec![line]), area);
        return;
    }

    let mode_label = match state.mode {
        AppMode::Home => " HOME ",
        AppMode::Dashboard => " NORMAL ",
    };

    let context = match state.mode {
        AppMode::Home => {
            // let n = state.connections.len();
            // format!("  {}  connection{}", n, if n == 1 { "" } else { "s" })
            String::new()
        }
        AppMode::Dashboard => String::new(),
    };

    let line = Line::from(vec![
        Span::styled(
            mode_label,
            Style::default()
                .fg(Color::Blue)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(context, Style::default().fg(Color::DarkGray)),
    ]);

    frame.render_widget(Paragraph::new(vec![line]), area);
}

fn render_input(frame: &mut Frame, area: Rect, state: &AppState) {
    let input = &state.cmdline.input;

    let line = Line::from(vec![
        Span::styled(
            ":",
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(input.clone(), Style::default().fg(Color::White)),
    ]);

    frame.render_widget(Paragraph::new(vec![line]), area);

    let cursor_x = (area.x + 1 + input.len() as u16).min(area.right().saturating_sub(1));
    frame.set_cursor_position((cursor_x, area.y));
}

fn render_confirm(frame: &mut Frame, area: Rect, action: &ConfirmAction, input: &str) {
    match action {
        ConfirmAction::DeleteConnection(name) => render_confirm_delete(frame, area, name, input),
    }
}

fn render_confirm_delete(frame: &mut Frame, area: Rect, name: &str, input: &str) {
    let prefix_spans: Vec<Span> = vec![
        Span::styled("Delete ", Style::default().fg(Color::White)),
        Span::styled(
            format!("\"{}\"", name),
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("? [y/N]:  ", Style::default().fg(Color::DarkGray)),
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
