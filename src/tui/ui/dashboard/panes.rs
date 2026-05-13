//! Individual pane renderers — TableList, TableView, SchemaView, QueryEditor.

use ratatui::{
    Frame,
    layout::Rect,
    prelude::Position,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
};

use crate::tui::state::{
    TableMode,
    dashboard::{DashboardState, LoadedTable},
    pane_layout::{Pane, PaneId, PaneType},
};

// ── Constants ─────────────────────────────────────────────────────────────────

const NUM_SPACES_BETWEEN_COLUMNS: u16 = 3;
const ROW_NUMBER_PADDING: u16 = 2;
const MAX_COLUMN_WIDTH_FRACTION: f32 = 0.3;
const EDGE_PADDING: u16 = 2;

// ── Dispatcher ────────────────────────────────────────────────────────────────

pub fn render_pane(
    frame: &mut Frame,
    pane_id: PaneId,
    dash: &DashboardState,
    focused: bool,
) {
    let Some(pane) = dash.tree.panes.get(&pane_id) else { return };
    let Some(area) = pane.area else { return };

    match pane.kind {
        PaneType::TableList => render_table_list(frame, area, pane, dash, focused),
        PaneType::TableView => render_table_view(frame, area, pane, dash, focused),
        PaneType::SchemaView => render_schema_view(frame, area, pane, dash, focused),
        PaneType::QueryEditor => render_query_editor(frame, area, pane, focused),
    }
}

// ── TableList pane ────────────────────────────────────────────────────────────

fn render_table_list(
    frame: &mut Frame,
    area: Rect,
    pane: &Pane,
    dash: &DashboardState,
    focused: bool,
) {
    let border_style = if focused {
        Style::default().fg(Color::Blue)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let title = Line::from(Span::styled(
        dash.connection.name.as_str(),
        Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
    ))
    .centered();

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(border_style);

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let mut lines: Vec<Line> = vec![];

    lines.push(Line::from(Span::styled(
        "Tables",
        Style::default().fg(Color::Blue).add_modifier(Modifier::BOLD),
    )));

    let sep = "─".repeat(inner.width as usize);
    lines.push(Line::from(Span::styled(sep, Style::default().fg(Color::DarkGray))));

    let header_lines = 3;
    let viewport = inner.height.saturating_sub(header_lines).max(1) as usize;
    let start = pane.nav_offset;
    let end = (start + viewport).min(dash.tables.len());

    if dash.tables.is_empty() {
        lines.push(Line::from(Span::styled(
            "No tables",
            Style::default().fg(Color::DarkGray),
        )));
    } else {
        for table_idx in start..end {
            let table = &dash.tables[table_idx];
            let selected = table_idx == pane.nav_cursor;

            let style = if selected && focused {
                Style::default()
                    .bg(Color::Rgb(28, 42, 74))
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD)
            } else if selected {
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::DarkGray)
            };

            let padded = format!("{:width$}", table.as_str(), width = inner.width as usize);
            lines.push(Line::from(Span::styled(padded, style)));
        }
    }

    frame.render_widget(Paragraph::new(lines), inner);
}

// ── TableView pane ────────────────────────────────────────────────────────────

fn render_table_view(
    frame: &mut Frame,
    area: Rect,
    pane: &Pane,
    dash: &DashboardState,
    focused: bool,
) {
    let title = if let Some(ref loaded) = dash.loaded {
        format!(" {} ", loaded.name)
    } else {
        String::new()
    };

    let block = pane_block(&title, focused);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if dash.loading {
        frame.render_widget(
            Paragraph::new(Span::styled(" Loading…", Style::default().fg(Color::DarkGray))),
            inner,
        );
        return;
    }

    if let Some(ref err) = dash.error {
        frame.render_widget(
            Paragraph::new(Span::styled(format!(" {err}"), Style::default().fg(Color::Red))),
            inner,
        );
        return;
    }

    let Some(ref loaded) = dash.loaded else {
        frame.render_widget(
            Paragraph::new(Span::styled(
                " Select a table from a list pane to load.",
                Style::default().fg(Color::DarkGray),
            )),
            inner,
        );
        return;
    };

    render_loaded_table(frame, inner, pane, loaded, focused);
}

fn render_loaded_table(
    frame: &mut Frame,
    area: Rect,
    pane: &Pane,
    loaded: &LoadedTable,
    focused: bool,
) {
    if loaded.headers.is_empty() {
        frame.render_widget(
            Paragraph::new(Span::styled(" Table is empty.", Style::default().fg(Color::DarkGray))),
            area,
        );
        return;
    }

    let max_row_num = loaded.rows.len().max(1);
    let row_num_width = format!("{}", max_row_num).len() as u16;
    let gutter_width = row_num_width + 2 * ROW_NUMBER_PADDING + 1;

    let data_area_width = area.width.saturating_sub(gutter_width).saturating_sub(2 * EDGE_PADDING);
    let max_single_width = (data_area_width as f32 * MAX_COLUMN_WIDTH_FRACTION) as u16;

    let column_widths: Vec<u16> = loaded
        .headers
        .iter()
        .enumerate()
        .map(|(col_idx, header)| {
            let mut w = header.len() as u16;
            for row in &loaded.rows {
                if let Some(cell) = row.get(col_idx) {
                    w = w.max(cell.len() as u16);
                }
            }
            w = w.min(max_single_width);
            w + NUM_SPACES_BETWEEN_COLUMNS
        })
        .collect();

    let mut col_offset = pane.col_offset.min(loaded.headers.len().saturating_sub(1));
    let cursor_col = pane.cursor_col;

    loop {
        let mut visible_width = 0;
        let mut visible_cols = 0;
        for &w in column_widths.iter().skip(col_offset) {
            if visible_width + w > data_area_width {
                break;
            }
            visible_width += w;
            visible_cols += 1;
        }
        if visible_cols == 0 {
            break;
        }
        if cursor_col < col_offset && col_offset > 0 {
            col_offset -= 1;
            continue;
        }
        if cursor_col >= col_offset + visible_cols && col_offset < loaded.headers.len().saturating_sub(1) {
            col_offset += 1;
            continue;
        }
        break;
    }

    let conservative_right = (area.x + area.width).saturating_sub(1 + EDGE_PADDING);
    let mut x_cursor = area.x + gutter_width + EDGE_PADDING;
    let mut visible_cols = 0;
    for col_idx in col_offset..loaded.headers.len() {
        if x_cursor >= conservative_right {
            break;
        }
        x_cursor += column_widths[col_idx];
        visible_cols += 1;
    }

    let has_more_left = col_offset > 0;
    let has_more_right = col_offset + visible_cols < loaded.headers.len();
    let right_boundary = if has_more_right { conservative_right } else { (area.x + area.width).saturating_sub(EDGE_PADDING) };

    let y_header_text = area.y + 1;
    let y_header_line = area.y + 2;
    let y_first_record = area.y + 3;

    let buf = frame.buffer_mut();

    for x in area.x..area.x + area.width {
        if let Some(cell) = buf.cell_mut(Position::new(x, y_header_line)) {
            cell.set_symbol("─");
            cell.set_style(Style::default().fg(Color::DarkGray));
        }
    }

    let sep_x = area.x + gutter_width - 1;
    for y in y_first_record..area.y + area.height {
        if let Some(cell) = buf.cell_mut(Position::new(sep_x, y)) {
            cell.set_symbol("│");
            cell.set_style(Style::default().fg(Color::DarkGray));
        }
    }
    if let Some(cell) = buf.cell_mut(Position::new(sep_x, y_header_line)) {
        cell.set_symbol("┼");
        cell.set_style(Style::default().fg(Color::DarkGray));
    }

    let mut x = area.x + gutter_width + EDGE_PADDING;
    for (col_idx, header) in loaded.headers.iter().enumerate().skip(col_offset) {
        if x >= right_boundary {
            break;
        }
        let width = column_widths[col_idx];
        let effective_width = width.saturating_sub(NUM_SPACES_BETWEEN_COLUMNS).min(right_boundary - x);

        let is_selected_col = matches!(pane.mode, TableMode::VisualColumn if col_idx == cursor_col);
        let style = if is_selected_col && focused {
            Style::default().bg(Color::Rgb(28, 42, 74)).fg(Color::White).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Blue).add_modifier(Modifier::BOLD)
        };

        let padded = format!("{:width$}", header.as_str(), width = effective_width as usize);
        buf.set_span(x, y_header_text, &Span::styled(padded, style), effective_width);
        x += width;
    }

    if has_more_left {
        let ix = area.x + gutter_width + EDGE_PADDING;
        if let Some(cell) = buf.cell_mut(Position::new(ix, y_header_text)) {
            cell.set_symbol("◂");
            cell.set_style(Style::default().fg(Color::DarkGray));
        }
    }
    if has_more_right {
        let ix = (area.x + area.width).saturating_sub(1 + EDGE_PADDING);
        if let Some(cell) = buf.cell_mut(Position::new(ix, y_header_text)) {
            cell.set_symbol("▸");
            cell.set_style(Style::default().fg(Color::DarkGray));
        }
    }

    let visible_rows = (area.y + area.height).saturating_sub(y_first_record) as usize;
    let start_row = pane.row_offset;
    let end_row = (start_row + visible_rows).min(loaded.rows.len());

    for row_idx in start_row..end_row {
        let y = y_first_record + (row_idx - start_row) as u16;
        if y >= area.y + area.height {
            break;
        }
        let row = &loaded.rows[row_idx];
        let is_selected_row = matches!(pane.mode, TableMode::VisualRow if row_idx == pane.row_cursor);

        let row_num_str = format!("{}", row_idx + 1);
        let row_num_style = if is_selected_row && focused {
            Style::default().fg(Color::White).add_modifier(Modifier::BOLD | Modifier::UNDERLINED)
        } else if is_selected_row {
            Style::default().fg(Color::White).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::DarkGray)
        };
        let row_num_span = Span::styled(
            format!("{:>width$}", row_num_str, width = row_num_width as usize),
            row_num_style,
        );
        buf.set_span(area.x + ROW_NUMBER_PADDING, y, &row_num_span, row_num_width);

        let mut x = area.x + gutter_width + EDGE_PADDING;
        for (col_idx, cell_text) in row.iter().enumerate().skip(col_offset) {
            if x >= right_boundary {
                break;
            }
            let width = column_widths[col_idx];
            let effective_width = width.saturating_sub(NUM_SPACES_BETWEEN_COLUMNS).min(right_boundary - x);

            let is_selected = match pane.mode {
                TableMode::Normal | TableMode::Insert => row_idx == pane.row_cursor && col_idx == cursor_col,
                TableMode::VisualRow => row_idx == pane.row_cursor,
                TableMode::VisualColumn => col_idx == cursor_col,
            };

            let style = if is_selected && focused {
                Style::default().bg(Color::Rgb(28, 42, 74)).fg(Color::White).add_modifier(Modifier::BOLD)
            } else if is_selected {
                Style::default().add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };

            let display = if cell_text.is_empty() { " " } else { cell_text.as_str() };
            let padded = format!("{:width$}", display, width = effective_width as usize);
            buf.set_span(x, y, &Span::styled(padded, style), effective_width);
            x += width;
        }

        if has_more_left {
            let ix = area.x + gutter_width + EDGE_PADDING;
            if let Some(cell) = buf.cell_mut(Position::new(ix, y)) {
                cell.set_symbol("◂");
                cell.set_style(Style::default().fg(Color::DarkGray));
            }
        }
        if has_more_right {
            let ix = (area.x + area.width).saturating_sub(1 + EDGE_PADDING);
            if let Some(cell) = buf.cell_mut(Position::new(ix, y)) {
                cell.set_symbol("▸");
                cell.set_style(Style::default().fg(Color::DarkGray));
            }
        }
    }

    if end_row < loaded.rows.len() {
        let indicator_y = (area.y + area.height).saturating_sub(1);
        let indicator_x = area.x + gutter_width + 1;
        if let Some(cell) = buf.cell_mut(Position::new(indicator_x, indicator_y)) {
            cell.set_symbol("▾");
            cell.set_style(Style::default().fg(Color::DarkGray));
        }
    }
}

// ── SchemaView pane ───────────────────────────────────────────────────────────

fn render_schema_view(
    frame: &mut Frame,
    area: Rect,
    _pane: &Pane,
    dash: &DashboardState,
    focused: bool,
) {
    let block = pane_block(" Schema ", focused);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let Some(ref loaded) = dash.loaded else {
        frame.render_widget(
            Paragraph::new(Span::styled(" —", Style::default().fg(Color::DarkGray))),
            inner,
        );
        return;
    };

    let header_style = Style::default().fg(Color::Blue).add_modifier(Modifier::BOLD);
    let mut lines: Vec<Line> = vec![];

    lines.push(Line::from(vec![
        Span::styled("Column", header_style),
        Span::styled("  Type", header_style),
        Span::styled("  Flags", header_style),
    ]));

    let sep = "─".repeat(inner.width as usize);
    lines.push(Line::from(Span::styled(sep, Style::default().fg(Color::DarkGray))));

    for col in &loaded.schema {
        let mut flags = String::new();
        if col.is_primary_key {
            flags.push_str("PK ");
        }
        if !col.nullable {
            flags.push('!');
        }
        lines.push(Line::from(vec![
            Span::styled(format!("{:<16}", col.name), Style::default().fg(Color::White)),
            Span::styled(format!("{:<12}", col.data_type), Style::default().fg(Color::DarkGray)),
            Span::styled(flags, Style::default().fg(Color::Yellow)),
        ]));
    }

    frame.render_widget(Paragraph::new(lines), inner);
}

// ── QueryEditor pane ──────────────────────────────────────────────────────────

fn render_query_editor(
    frame: &mut Frame,
    area: Rect,
    _pane: &Pane,
    focused: bool,
) {
    let block = pane_block(" Query Editor ", focused);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let lines: Vec<Line> = vec![
        Line::from(""),
        Line::from(Span::styled(
            "  SQL query editor coming soon...",
            Style::default().fg(Color::DarkGray),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "  Press i to enter insert mode",
            Style::default().fg(Color::DarkGray),
        )),
    ];

    frame.render_widget(Paragraph::new(lines), inner);
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn pane_block(title: &str, focused: bool) -> Block<'_> {
    let border_style = if focused {
        Style::default().fg(Color::Blue)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let title_style = if focused {
        Style::default().fg(Color::Blue).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    Block::default()
        .title(if title.is_empty() {
            Line::from("")
        } else {
            Line::from(title).style(title_style)
        })
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(border_style)
}
