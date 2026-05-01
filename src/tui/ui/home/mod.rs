use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, Borders, Cell, Clear, Paragraph, Row, Scrollbar, ScrollbarOrientation,
        ScrollbarState, Table, TableState,
    },
};

use crate::{
    connection::models::Database,
    tui::{
        state::{AppState, Overlay},
        ui::components::centered_rect,
    },
};

pub fn render_home(frame: &mut Frame, area: Rect, state: &AppState) {
    let narrow = centered_rect(45, 50, area);
    let [title_area, connections_area, _] = Layout::vertical([
        Constraint::Length(5),
        Constraint::Min(6),
        Constraint::Length(3),
    ])
    .areas(narrow);

    let wide = centered_rect(70, 65, area);
    let [_, _, instructions_area] = Layout::vertical([
        Constraint::Length(5),
        Constraint::Min(6),
        Constraint::Length(3),
    ])
    .areas(wide);

    render_title(frame, title_area);
    render_connections(frame, connections_area, state);
    render_instructions(frame, instructions_area);

    // Overlays float on top of everything else.
    if state.overlay.is_some() {
        render_overlay(frame, area, state);
    }
}
fn render_title(frame: &mut Frame, area: Rect) {
    let lines = vec![
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
        Line::from(Span::styled(
            "────────────────────",
            Style::default().fg(Color::DarkGray),
        ))
        .centered(),
    ];

    frame.render_widget(Paragraph::new(lines), area);
}

fn render_connections(frame: &mut Frame, area: Rect, state: &AppState) {
    let block = Block::default()
        .title(
            Line::from(" Connections ").style(
                Style::default()
                    .fg(Color::Blue)
                    .add_modifier(Modifier::BOLD),
            ),
        )
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    if state.connections.is_empty() {
        render_empty_connections(frame, inner);
        return;
    }

    let viewport_height = inner.height as usize;
    let total = state.connections.len();
    let needs_scrollbar = total > viewport_height;

    let offset = state
        .selected_connection
        .saturating_sub(viewport_height.saturating_sub(1));

    let table_area = if needs_scrollbar {
        Rect {
            width: inner.width.saturating_sub(1),
            ..inner
        }
    } else {
        inner
    };

    let rows: Vec<Row> = state
        .connections
        .iter()
        .enumerate()
        .map(|(i, db)| connection_row(db, i == state.selected_connection))
        .collect();

    let widths = [
        Constraint::Length(1),
        Constraint::Fill(1),
        Constraint::Length(3),
    ];

    let table = Table::new(rows, widths)
        .column_spacing(1)
        .row_highlight_style(
            Style::default()
                .bg(Color::Rgb(28, 42, 74))
                .add_modifier(Modifier::BOLD),
        );

    let mut table_state = TableState::default()
        .with_offset(offset)
        .with_selected(Some(state.selected_connection));
    frame.render_stateful_widget(table, table_area, &mut table_state);

    if needs_scrollbar {
        let mut scrollbar_state =
            ScrollbarState::new(total.saturating_sub(viewport_height)).position(offset);

        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(None)
            .end_symbol(None)
            .track_symbol(Some("│"))
            .thumb_symbol("█");

        frame.render_stateful_widget(scrollbar, inner, &mut scrollbar_state);
    }
}

fn connection_row(db: &Database, selected: bool) -> Row<'static> {
    let bullet = if selected {
        Cell::from(Span::styled(
            "●",
            Style::default()
                .fg(Color::Blue)
                .add_modifier(Modifier::BOLD),
        ))
    } else {
        Cell::from(Span::styled("○", Style::default().fg(Color::DarkGray)))
    };

    let name = Cell::from(db.name.clone());
    let badge = Cell::from(Line::from(db.engine.badge()));

    Row::new(vec![bullet, name, badge])
}

fn render_empty_connections(frame: &mut Frame, area: Rect) {
    let lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            "No saved connections",
            Style::default().fg(Color::DarkGray),
        ))
        .centered(),
        Line::from(Span::styled(
            "Press 'a' to add one",
            Style::default().fg(Color::DarkGray),
        ))
        .centered(),
    ];

    frame.render_widget(Paragraph::new(lines), area);
}

fn render_instructions(frame: &mut Frame, area: Rect) {
    let keys: &[(&str, &str)] = &[
        ("↑ k / ↓ j", "navigate"),
        ("↵", "connect"),
        ("a", "add"),
        ("d", "delete"),
        ("?", "help"),
        ("q", "quit"),
    ];

    let mut spans: Vec<Span> = Vec::new();
    for (i, (key, label)) in keys.iter().enumerate() {
        if i > 0 {
            spans.push(Span::styled("  ·  ", Style::default().fg(Color::DarkGray)));
        }

        spans.push(Span::styled(
            format!("'{key}'"),
            Style::default()
                .fg(Color::Blue)
                .add_modifier(Modifier::BOLD),
        ));

        spans.push(Span::styled(
            format!(" {label}"),
            Style::default().fg(Color::Gray),
        ));
    }

    let lines = vec![Line::from(""), Line::from(spans).centered()];
    frame.render_widget(Paragraph::new(lines), area);
}

pub fn select_next(state: &mut AppState) {
    if state.connections.is_empty() {
        return;
    }
    state.selected_connection = (state.selected_connection + 1) % state.connections.len();
}

pub fn select_prev(state: &mut AppState) {
    if state.connections.is_empty() {
        return;
    }

    let len = state.connections.len();
    state.selected_connection = (state.selected_connection + len - 1) % len;
}

/// Jump to the first connection.
pub fn goto_top(state: &mut AppState) {
    state.selected_connection = 0;
}

/// Jump to the last connection.
pub fn goto_bottom(state: &mut AppState) {
    if !state.connections.is_empty() {
        state.selected_connection = state.connections.len() - 1;
    }
}

pub fn selected_connection(state: &AppState) -> Option<&Database> {
    state.connections.get(state.selected_connection)
}

pub fn remove_selected(state: &mut AppState) {
    if state.connections.is_empty() {
        return;
    }
    state.connections.remove(state.selected_connection);
    if state.selected_connection > 0 && state.selected_connection >= state.connections.len() {
        state.selected_connection -= 1;
    }
}

// ── Overlay rendering ─────────────────────────────────────────────────────────

fn render_overlay(frame: &mut Frame, area: Rect, state: &AppState) {
    let Some(overlay) = state.overlay else { return };
    match overlay {
        Overlay::Help => render_help(frame, area),
        Overlay::AddConnection => render_add_connection(frame, area),
        Overlay::CommandPalette => render_command_palette(frame, area),
        // ConfirmDelete is handled by the command-line bar, not as an overlay.
        Overlay::ConfirmDelete => {}
    }
}

/// Clear a rect, draw a dark bordered popup block, and return the inner area.
fn open_popup<'a>(frame: &mut Frame, area: Rect, title: &'a str) -> (Block<'a>, Rect) {
    frame.render_widget(Clear, area);

    let block = Block::default()
        .title(
            Line::from(format!(" {title} ")).style(
                Style::default()
                    .fg(Color::Blue)
                    .add_modifier(Modifier::BOLD),
            ),
        )
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Blue))
        .style(Style::default().bg(Color::Reset));

    let inner = block.inner(area);

    (block, inner)
}

/// Render a right-aligned dim hint line inside a popup's inner area.
fn render_dismiss_hint(frame: &mut Frame, area: Rect, hint: &str) {
    let line = Line::from(Span::styled(hint, Style::default().fg(Color::DarkGray))).right_aligned();
    frame.render_widget(Paragraph::new(vec![line]), area);
}

// ── Help ──────────────────────────────────────────────────────────────────────

fn render_help(frame: &mut Frame, area: Rect) {
    let popup_area = centered_rect(46, 72, area);
    let (block, inner) = open_popup(frame, popup_area, "Help");
    frame.render_widget(block, popup_area);

    let [content_area, hint_area] =
        Layout::vertical([Constraint::Min(0), Constraint::Length(1)]).areas(inner);

    let nav_header = Style::default()
        .fg(Color::Blue)
        .add_modifier(Modifier::BOLD);

    let key_style = Style::default()
        .fg(Color::White)
        .add_modifier(Modifier::BOLD);

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

/// Build a two-column key → description line for the help overlay.
fn binding_line<'a>(key: &'a str, desc: &'a str, key_style: Style, desc_style: Style) -> Line<'a> {
    Line::from(vec![
        Span::styled(format!("{key:<14}"), key_style),
        Span::styled(desc, desc_style),
    ])
}

// ── Add connection ──────────────────────────────────────────────────────────

fn render_add_connection(frame: &mut Frame, area: Rect) {
    let popup_area = centered_rect(46, 38, area);
    let (block, inner) = open_popup(frame, popup_area, "Add Connection");
    frame.render_widget(block, popup_area);

    let [content_area, hint_area] =
        Layout::vertical([Constraint::Min(0), Constraint::Length(1)]).areas(inner);

    let lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            "  Connection form coming soon.",
            Style::default().fg(Color::Gray),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "  In the meantime, use the CLI:",
            Style::default().fg(Color::DarkGray),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "  shql add --name <name> --url <url>",
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        )),
    ];

    frame.render_widget(Paragraph::new(lines), content_area);
    render_dismiss_hint(frame, hint_area, "Esc/q  <cancel> ");
}

// ── Command palette ───────────────────────────────────────────────────────────

fn render_command_palette(frame: &mut Frame, area: Rect) {
    let popup_area = centered_rect(55, 50, area);
    let (block, inner) = open_popup(frame, popup_area, "Command Palette");
    frame.render_widget(block, popup_area);

    let [content_area, hint_area] =
        Layout::vertical([Constraint::Min(0), Constraint::Length(1)]).areas(inner);

    let lines = vec![
        Line::from(vec![
            Span::styled(
                "  > ",
                Style::default()
                    .fg(Color::Blue)
                    .add_modifier(Modifier::BOLD),
            ),
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
