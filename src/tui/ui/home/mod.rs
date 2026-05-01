use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, BorderType, Borders, Cell, Clear, Paragraph, Row, Scrollbar, ScrollbarOrientation,
        ScrollbarState, Table, TableState,
    },
};

use crate::{
    connection::models::{Database, Engine},
    tui::{
        state::{
            AppState, Overlay,
            form::{AddConnectionForm, FieldId, FormInputMode, TextMode},
        },
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

    // Hide home instructions while any overlay is open — the overlay's own
    // hint row replaces them and showing both is confusing.
    if state.overlay.is_none() {
        render_instructions(frame, instructions_area);
    }

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
        Constraint::Length(1),  // ●/○
        Constraint::Fill(1),    // name
        Constraint::Length(11), // badge — widest is "[Postgres] " (11 chars)
    ];

    let table = Table::new(rows, widths)
        .column_spacing(1)
        .row_highlight_style(
            Style::default()
                .fg(Color::Blue)
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
        Overlay::AddConnection => render_add_connection(frame, area, state),
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
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::White))
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

// ── Add connection form ──────────────────────────────────────────────────────

fn render_add_connection(frame: &mut Frame, area: Rect, state: &AppState) {
    let Some(ref form) = state.form else {
        return;
    };

    let fields = form.visible_fields();

    // ── Compute error height before sizing the popup ───────────────────────────
    // The popup is 56% wide. Subtract 2 borders + 2×LEFT_PAD to get the text
    // column width, then measure how many lines the error will need.
    const LABEL_W: u16 = 13;
    const LEFT_PAD: u16 = 2;
    let inner_w = (area.width * 56 / 100).saturating_sub(2 + LEFT_PAD * 2) as usize;
    let _ = inner_w; // reserved for future per-field validation hints

    // Height: 1 top-pad + fields + 1 blank + 1 hint + 2 borders
    let content_h = 1 + fields.len() as u16 + 1 + 1;
    let popup_h_pct = ((content_h + 2) * 100 / area.height.max(1)).clamp(35, 88) as u16;
    let popup_area = centered_rect(56, popup_h_pct, area);

    frame.render_widget(Clear, popup_area);

    let block = Block::default()
        .title(
            Line::from(" Add Connection ").style(
                Style::default()
                    .fg(Color::Blue)
                    .add_modifier(Modifier::BOLD),
            ),
        )
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::White))
        .style(Style::default().bg(Color::Reset));

    let inner = block.inner(popup_area);
    frame.render_widget(block, popup_area);

    let [fields_area, hint_area] = Layout::vertical([
        Constraint::Min(0),
        Constraint::Length(1),
    ])
    .areas(inner);

    // ── Field rows ────────────────────────────────────────────────────────────

    for (i, field_id) in fields.iter().enumerate() {
        let y = fields_area.y + 1 + i as u16; // +1 for top padding
        if y >= fields_area.y + fields_area.height {
            break;
        }

        let row = Rect {
            x: fields_area.x + LEFT_PAD,
            y,
            width: fields_area.width.saturating_sub(LEFT_PAD),
            height: 1,
        };
        let is_focused = i == form.focused;

        let [lbl_area, val_area] =
            Layout::horizontal([Constraint::Length(LABEL_W), Constraint::Min(0)]).areas(row);

        // Label — blue + bold when focused, muted otherwise
        let lbl_style = if is_focused {
            Style::default()
                .fg(Color::Blue)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::DarkGray)
        };
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(field_id.label(), lbl_style))),
            lbl_area,
        );

        // Horizontal scroll: keep cursor visible when text is wider than the column.
        let scroll: usize = if is_focused && field_id.is_text() {
            let w = val_area.width as usize;
            if form.cursor_pos >= w { form.cursor_pos + 1 - w } else { 0 }
        } else {
            0
        };

        // Value
        render_field_value(frame, val_area, field_id, form, is_focused, scroll);

        // Terminal cursor for focused text fields in Insert mode;
        // Normal mode uses a rendered block cursor instead.
        if is_focused && field_id.is_text() && form.text_mode == TextMode::Insert {
            let cursor_in_view = form.cursor_pos.saturating_sub(scroll);
            let cx = (val_area.x + cursor_in_view as u16)
                .min(val_area.x + val_area.width.saturating_sub(1));
            frame.set_cursor_position((cx, y));
        }
    }

    // ── Hint ──────────────────────────────────────────────────────────────────
    render_dismiss_hint(
        frame,
        hint_area,
        "Tab <next>  Shift+Tab <prev>  ←→ <cycle>  Ctrl+S <save>  Esc <cancel> ",
    );
}

/// Render the value widget for a single form field.
fn render_field_value(
    frame: &mut Frame,
    area: Rect,
    field: &FieldId,
    form: &AddConnectionForm,
    focused: bool,
    scroll: usize,
) {
    match field {
        FieldId::Engine => {
            let opts = [
                ("Postgres", matches!(form.engine, Engine::Postgres)),
                ("MySQL", matches!(form.engine, Engine::Mysql)),
                ("SQLite", matches!(form.engine, Engine::Sqlite)),
            ];
            frame.render_widget(Paragraph::new(selector_line(&opts, focused)), area);
        }
        FieldId::InputMode => {
            let opts = [
                ("URL", matches!(form.input_mode, FormInputMode::Url)),
                ("Config", matches!(form.input_mode, FormInputMode::Config)),
            ];
            frame.render_widget(Paragraph::new(selector_line(&opts, focused)), area);
        }
        FieldId::Ssl => {
            let opts = [("None", !form.ssl_enabled), ("Peer", form.ssl_enabled)];
            frame.render_widget(Paragraph::new(selector_line(&opts, focused)), area);
        }
        FieldId::CreateIfMissing => {
            let opts = [
                ("No", !form.create_if_missing),
                ("Yes", form.create_if_missing),
            ];
            frame.render_widget(Paragraph::new(selector_line(&opts, focused)), area);
        }
        FieldId::Password => {
            let full: String = "•".repeat(form.password.chars().count());
            let (display, local_cur) = if focused {
                (visible_text(&full, scroll, area.width as usize),
                 form.cursor_pos.saturating_sub(scroll))
            } else {
                (full, 0)
            };
            let line = if focused {
                text_line_with_cursor(display, local_cur, &form.text_mode)
            } else {
                text_line(display, false)
            };
            frame.render_widget(Paragraph::new(line), area);
        }
        other => {
            let full = form.text_for(other).unwrap_or("").to_string();
            let (display, local_cur) = if focused {
                (visible_text(&full, scroll, area.width as usize),
                 form.cursor_pos.saturating_sub(scroll))
            } else {
                (full, 0)
            };
            let line = if focused {
                text_line_with_cursor(display, local_cur, &form.text_mode)
            } else {
                text_line(display, false)
            };
            frame.render_widget(Paragraph::new(line), area);
        }
    }
}

/// Render a row of `● label` / `○ label` selector options.
fn selector_line(options: &[(&'static str, bool)], focused: bool) -> Line<'static> {
    let mut spans: Vec<Span<'static>> = Vec::new();
    for (i, (label, selected)) in options.iter().enumerate() {
        if i > 0 {
            spans.push(Span::raw("   "));
        }
        let style = if *selected {
            Style::default()
                .fg(Color::Blue)
                .add_modifier(Modifier::BOLD)
        } else if focused {
            Style::default().fg(Color::Gray)
        } else {
            Style::default().fg(Color::DarkGray)
        };
        let bullet: &'static str = if *selected { "●" } else { "○" };
        spans.push(Span::styled(bullet, style));
        spans.push(Span::styled(format!(" {label}"), style));
    }
    Line::from(spans)
}

/// Render a plain text value with cursor awareness.
///
/// - **Insert mode**: plain white text; terminal cursor is set by the caller.
/// - **Normal mode**: block cursor (blue bg) on the character at `cursor_pos`.
fn text_line_with_cursor(value: String, cursor_pos: usize, mode: &TextMode) -> Line<'static> {
    match mode {
        TextMode::Insert => {
            if value.is_empty() {
                Line::from(Span::raw(""))
            } else {
                Line::from(Span::styled(value, Style::default().fg(Color::White)))
            }
        }
        TextMode::Normal => {
            let chars: Vec<char> = value.chars().collect();
            let len = chars.len();
            if len == 0 {
                // Empty field: show a single block to indicate cursor presence.
                return Line::from(Span::styled(
                    " ",
                    Style::default().bg(Color::Blue).fg(Color::Black),
                ));
            }
            let pos = cursor_pos.min(len - 1);
            let before: String = chars[..pos].iter().collect();
            let at: String = chars[pos..pos + 1].iter().collect();
            let after: String = chars[pos + 1..].iter().collect();
            let mut spans: Vec<Span<'static>> = Vec::new();
            if !before.is_empty() {
                spans.push(Span::styled(before, Style::default().fg(Color::White)));
            }
            spans.push(Span::styled(
                at,
                Style::default()
                    .bg(Color::Blue)
                    .fg(Color::Black)
                    .add_modifier(Modifier::BOLD),
            ));
            if !after.is_empty() {
                spans.push(Span::styled(after, Style::default().fg(Color::White)));
            }
            Line::from(spans)
        }
    }
}

/// Return the substring of `text` starting at char offset `scroll` that fits
/// within `width` visible columns. Used to implement horizontal scrolling in
/// text input fields.
pub fn visible_text(text: &str, scroll: usize, width: usize) -> String {
    let chars: Vec<char> = text.chars().collect();
    let start = scroll.min(chars.len());
    let end = (start + width).min(chars.len());
    chars[start..end].iter().collect()
}

/// Render a plain text value; shows a dim dash when empty and not focused.
fn text_line(value: String, focused: bool) -> Line<'static> {
    if focused {
        Line::from(Span::styled(value, Style::default().fg(Color::White)))
    } else if value.is_empty() {
        Line::from(Span::styled("—", Style::default().fg(Color::DarkGray)))
    } else {
        Line::from(Span::styled(value, Style::default().fg(Color::Gray)))
    }
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

