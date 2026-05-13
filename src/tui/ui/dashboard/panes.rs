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

// ── Border title helpers ──────────────────────────────────────────────────────

fn make_title(pane: &Pane) -> String {
    match pane.kind {
        PaneType::TableList => format!(" {} ", pane.display_id),
        PaneType::TableView => {
            if let Some(ref table) = pane.bound_table {
                format!(" {}: {} ", pane.display_id, table)
            } else {
                format!(" {} ", pane.display_id)
            }
        }
        PaneType::SchemaView => {
            if let Some(ref table) = pane.bound_table {
                format!(" {}: Schema({}) ", pane.display_id, table)
            } else {
                format!(" {}: Schema ", pane.display_id)
            }
        }
        PaneType::QueryEditor => format!(" {}: Query ", pane.display_id),
    }
}

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
        .title(Line::from(title).style(title_style))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(border_style)
}

// ── TableList pane ────────────────────────────────────────────────────────────

fn render_table_list(
    frame: &mut Frame,
    area: Rect,
    pane: &Pane,
    dash: &DashboardState,
    focused: bool,
) {
    let title = make_title(pane);
    let block = pane_block(&title, focused);
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
    let title = make_title(pane);
    let block = pane_block(&title, focused);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if dash.loading && pane.bound_table.as_ref() == dash.pending_load.as_ref() {
        frame.render_widget(
            Paragraph::new(Span::styled(" Loading…", Style::default().fg(Color::DarkGray))),
            inner,
        );
        return;
    }

    if let Some(ref err) = dash.error {
        if pane.bound_table.as_ref() == dash.pending_load.as_ref() {
            frame.render_widget(
                Paragraph::new(Span::styled(format!(" {err}"), Style::default().fg(Color::Red))),
                inner,
            );
            return;
        }
    }

    let Some(ref table_name) = pane.bound_table else {
        frame.render_widget(
            Paragraph::new(Span::styled(
                " No table bound.",
                Style::default().fg(Color::DarkGray),
            )),
            inner,
        );
        return;
    };

    let Some(ref loaded) = dash.table_cache.get(table_name) else {
        frame.render_widget(
            Paragraph::new(Span::styled(
                " Loading table data…",
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
//
// Vertical card layout inspired by TablePlus:
//   ┌────────────────────────────────────┐
//   │  emp_no                  bigint    │  ← name (bold) + type (dim, right)
//   │  PK  NOT NULL                      │  ← constraint badges
//   │                                    │  ← blank gap between cards
//   │  birth_date              date      │
//   │        DEFAULT '1970-01-01'        │
//   │                                    │
//   └────────────────────────────────────┘

const SCHEMA_CARD_HEIGHT: usize = 3; // 2 content lines + 1 blank gap
const SCHEMA_PAD: usize = 2;         // left / right padding inside the pane
const SCHEMA_SEL_BG: Color = Color::Rgb(35, 38, 55);

fn render_schema_view(
    frame: &mut Frame,
    area: Rect,
    pane: &Pane,
    dash: &DashboardState,
    focused: bool,
) {
    let title = make_title(pane);
    let block = pane_block(&title, focused);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let Some(ref table_name) = pane.bound_table else {
        frame.render_widget(
            Paragraph::new(Span::styled(" —", Style::default().fg(Color::DarkGray))),
            inner,
        );
        return;
    };

    let Some(ref loaded) = dash.table_cache.get(table_name) else {
        frame.render_widget(
            Paragraph::new(Span::styled(" Loading…", Style::default().fg(Color::DarkGray))),
            inner,
        );
        return;
    };

    let schema = &loaded.schema;
    if schema.is_empty() {
        frame.render_widget(
            Paragraph::new(Span::styled(" No columns.", Style::default().fg(Color::DarkGray))),
            inner,
        );
        return;
    }

    let cursor = pane.nav_cursor;
    let offset = pane.nav_offset;
    let viewport = (inner.height as usize / SCHEMA_CARD_HEIGHT).max(1);
    let end = (offset + viewport).min(schema.len());
    let visible = &schema[offset..end];

    let mut lines: Vec<Line> = Vec::new();
    let usable_w = inner.width.saturating_sub(2 * SCHEMA_PAD as u16) as usize;

    for (i, col) in visible.iter().enumerate() {
        let idx = offset + i;
        let sel = idx == cursor;

        // ── Line 1: column name (left) + data type (right) ──
        let name_style = if sel {
            Style::default().fg(Color::White).bold().bg(SCHEMA_SEL_BG)
        } else {
            Style::default().fg(Color::White).bold()
        };
        let type_style = if sel {
            Style::default().fg(Color::DarkGray).bg(SCHEMA_SEL_BG)
        } else {
            Style::default().fg(Color::DarkGray)
        };

        let name_text = col.name.clone();
        let type_text = col.data_type.clone();
        let name_w = name_text.chars().count();
        let type_w = type_text.chars().count();
        let gap = usable_w.saturating_sub(name_w + type_w);

        lines.push(Line::from(vec![
            Span::styled(" ".repeat(SCHEMA_PAD), Style::default().bg(if sel { SCHEMA_SEL_BG } else { Color::Reset })),
            Span::styled(name_text, name_style),
            Span::styled(" ".repeat(gap), Style::default().bg(if sel { SCHEMA_SEL_BG } else { Color::Reset })),
            Span::styled(type_text, type_style),
            Span::styled(" ".repeat(SCHEMA_PAD), Style::default().bg(if sel { SCHEMA_SEL_BG } else { Color::Reset })),
        ]));

        // ── Line 2: constraint badges ──
        let mut badge_spans: Vec<Span> = vec![];

        if col.is_primary_key {
            badge_spans.push(Span::styled(
                "PK",
                Style::default().fg(Color::Yellow).bg(if sel { SCHEMA_SEL_BG } else { Color::Reset }),
            ));
        }
        if !col.nullable {
            badge_spans.push(Span::styled(
                " NOT NULL",
                Style::default().fg(Color::Red).bg(if sel { SCHEMA_SEL_BG } else { Color::Reset }),
            ));
        }
        if let Some(ref dflt) = col.default_value {
            let display = if dflt.len() > 24 {
                format!("{}…", &dflt[..23])
            } else {
                dflt.clone()
            };
            badge_spans.push(Span::styled(
                format!(" DEFAULT {display}"),
                Style::default().fg(Color::Green).bg(if sel { SCHEMA_SEL_BG } else { Color::Reset }),
            ));
        }

        if badge_spans.is_empty() {
            badge_spans.push(Span::styled(
                "nullable",
                Style::default().fg(Color::DarkGray).bg(if sel { SCHEMA_SEL_BG } else { Color::Reset }),
            ));
        }

        // Pad badge line to full usable width so background colour fills the row.
        let badge_text_w: usize = badge_spans.iter().map(|s| s.content.chars().count()).sum();
        let pad = usable_w.saturating_sub(badge_text_w);
        if pad > 0 {
            badge_spans.push(Span::styled(
                " ".repeat(pad),
                Style::default().bg(if sel { SCHEMA_SEL_BG } else { Color::Reset }),
            ));
        }

        let mut badge_line = vec![
            Span::styled(" ".repeat(SCHEMA_PAD), Style::default().bg(if sel { SCHEMA_SEL_BG } else { Color::Reset })),
        ];
        badge_line.extend(badge_spans);
        badge_line.push(Span::styled(
            " ".repeat(SCHEMA_PAD),
            Style::default().bg(if sel { SCHEMA_SEL_BG } else { Color::Reset }),
        ));
        lines.push(Line::from(badge_line));

        // ── Line 3: blank gap between cards ──
        // Fill the entire width (padding + usable + padding) with spaces so the
        // background color forms a complete rectangle for the selected card.
        lines.push(Line::from(Span::styled(
            " ".repeat(inner.width as usize),
            Style::default().bg(if sel { SCHEMA_SEL_BG } else { Color::Reset }),
        )));
    }

    frame.render_widget(Paragraph::new(lines), inner);
}

// ── QueryEditor pane ──────────────────────────────────────────────────────────

fn render_query_editor(
    frame: &mut Frame,
    area: Rect,
    pane: &Pane,
    focused: bool,
) {
    let title = make_title(pane);
    let block = pane_block(&title, focused);
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
