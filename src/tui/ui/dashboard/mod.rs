/// Dashboard — two-pane layout with scrollable nav and cell-navigable table view.
///
///  ┌────────────┬────────────────────────────────────────┐
///  │  Tables    │  #   id    name      email             │
///  │  (nav,18%) │──1───2─────alice─────a@ex.com─────────│
///  │  orders    │  2   3      bob       b@ex.com         │
///  │  products ◄│  3   4      carol     c@ex.com         │
///  └────────────┴────────────────────────────────────────┘
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    prelude::Position,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
};

use crate::tui::{
    AppState,
    state::dashboard::{ActivePane, LoadedTable, TableMode},
};

// ── Constants ─────────────────────────────────────────────────────────────────

/// Padding between columns.
const NUM_SPACES_BETWEEN_COLUMNS: u16 = 3;
/// Horizontal padding inside the row-number gutter.
const ROW_NUMBER_PADDING: u16 = 2;
/// No single column may occupy more than this fraction of the viewport width.
const MAX_COLUMN_WIDTH_FRACTION: f32 = 0.3;
/// Extra horizontal padding between the pane border and the first/last data column.
const EDGE_PADDING: u16 = 2;

// ── Entry point ───────────────────────────────────────────────────────────────

pub fn render_dashboard(frame: &mut Frame, area: Rect, state: &mut AppState) {
    let [nav_area, table_area] = Layout::horizontal([
        Constraint::Percentage(18),
        Constraint::Percentage(82),
    ])
    .areas(area);

    // Sync scroll offsets before drawing (needs mutable access).
    if let Some(ref mut dash) = state.dashboard {
        let nav_viewport = nav_area.height.saturating_sub(2) as usize;
        dash.sync_nav_offset(nav_viewport);

        if let Some(ref mut loaded) = dash.loaded {
            let table_viewport = table_area.height.saturating_sub(2) as usize;
            loaded.sync_row_offset(table_viewport);
        }
    }

    render_nav(frame, nav_area, state);
    render_table_view(frame, table_area, state);
}

// ── Nav bar ───────────────────────────────────────────────────────────────────

fn render_nav(frame: &mut Frame, area: Rect, state: &AppState) {
    let Some(ref dash) = state.dashboard else { return };
    let focused = dash.active_pane == ActivePane::Nav;

    // Connection name as a green bold header, then table rows.
    let header = Line::from(vec![Span::styled(
        format!(" {}", dash.connection.name),
        Style::default()
            .fg(Color::Green)
            .add_modifier(Modifier::BOLD),
    )]);

    let mut lines: Vec<Line> = vec![header, Line::from("")];

    let viewport = area.height.saturating_sub(2) as usize;
    let start = dash.nav_offset;
    let end = (start + viewport).min(dash.tables.len());

    for table_idx in start..end {
        let table = &dash.tables[table_idx];
        let selected = table_idx == dash.nav_cursor;

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

        // Pad to full width so the highlight background fills the entire nav pane.
        let display = format!(" {table}");
        let padded = format!("{:width$}", display, width = area.width as usize);
        lines.push(Line::from(Span::styled(padded, style)));
    }

    if dash.tables.is_empty() {
        lines.push(Line::from(Span::styled(
            " No tables found",
            Style::default().fg(Color::DarkGray),
        )));
    }

    frame.render_widget(Paragraph::new(lines), area);
}

// ── Table view ────────────────────────────────────────────────────────────────

fn render_table_view(frame: &mut Frame, area: Rect, state: &mut AppState) {
    let dash = state.dashboard.as_mut().unwrap();
    let focused = dash.active_pane == ActivePane::Table;

    let block = pane_block(
        dash.loaded.as_ref().map(|t| t.name.as_str()).unwrap_or(""),
        focused,
    );
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if dash.loading {
        frame.render_widget(
            Paragraph::new(Span::styled(
                " Loading…",
                Style::default().fg(Color::DarkGray),
            )),
            inner,
        );
        return;
    }

    if let Some(ref err) = dash.error {
        frame.render_widget(
            Paragraph::new(Span::styled(
                format!(" {err}"),
                Style::default().fg(Color::Red),
            )),
            inner,
        );
        return;
    }

    let Some(ref mut loaded) = dash.loaded else {
        frame.render_widget(
            Paragraph::new(Span::styled(
                " Select a table from the nav to load its rows.",
                Style::default().fg(Color::DarkGray),
            )),
            inner,
        );
        return;
    };

    render_loaded_table(frame, inner, loaded, focused);
}

fn render_loaded_table(
    frame: &mut Frame,
    area: Rect,
    loaded: &mut LoadedTable,
    focused: bool,
) {
    if loaded.headers.is_empty() {
        frame.render_widget(
            Paragraph::new(Span::styled(
                " Table is empty.",
                Style::default().fg(Color::DarkGray),
            )),
            area,
        );
        return;
    }

    // ── Dimensions ────────────────────────────────────────────────────────────

    let max_row_num = loaded.rows.len().max(1);
    let row_num_width = format!("{}", max_row_num).len() as u16;
    let gutter_width = row_num_width + 2 * ROW_NUMBER_PADDING + 1; // +1 = separator

    let data_area_width = area.width.saturating_sub(gutter_width).saturating_sub(2 * EDGE_PADDING);
    let max_single_width = (data_area_width as f32 * MAX_COLUMN_WIDTH_FRACTION) as u16;

    // ── Column widths ─────────────────────────────────────────────────────────

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

    // ── Horizontal-scroll sanity ──────────────────────────────────────────────

    loaded.col_offset = loaded
        .col_offset
        .min(loaded.headers.len().saturating_sub(1));

    // Ensure the cursor column is visible; bump col_offset rightward if needed.
    loop {
        let mut visible_width = 0;
        let mut visible_cols = 0;
        for &w in column_widths.iter().skip(loaded.col_offset) {
            if visible_width + w > data_area_width {
                break;
            }
            visible_width += w;
            visible_cols += 1;
        }
        if visible_cols == 0 {
            break;
        }
        if loaded.cursor_col < loaded.col_offset && loaded.col_offset > 0 {
            loaded.col_offset -= 1;
            continue;
        }
        if loaded.cursor_col >= loaded.col_offset + visible_cols
            && loaded.col_offset < loaded.headers.len().saturating_sub(1)
        {
            loaded.col_offset += 1;
            continue;
        }
        break;
    }

    // ── Compute visible columns once ──────────────────────────────────────────

    let mut x_cursor = area.x + gutter_width + EDGE_PADDING;
    let mut visible_cols = 0;
    for col_idx in loaded.col_offset..loaded.headers.len() {
        if x_cursor >= area.x + area.width {
            break;
        }
        x_cursor += column_widths[col_idx];
        visible_cols += 1;
    }

    let has_more_left = loaded.col_offset > 0;
    let has_more_right = loaded.col_offset + visible_cols < loaded.headers.len();

    // ── Layout ────────────────────────────────────────────────────────────────

    let y_header_text = area.y + 1;
    let y_header_line = area.y + 2;
    let y_first_record = area.y + 3;

    let buf = frame.buffer_mut();

    // ── Header separator line ─────────────────────────────────────────────────
    for x in area.x..area.x + area.width {
        if let Some(cell) = buf.cell_mut(Position::new(x, y_header_line)) {
            cell.set_symbol("─");
            cell.set_style(Style::default().fg(Color::DarkGray));
        }
    }

    // ── Vertical separator after gutter ───────────────────────────────────────
    let sep_x = area.x + gutter_width - 1;
    for y in y_first_record..area.y + area.height {
        if let Some(cell) = buf.cell_mut(Position::new(sep_x, y)) {
            cell.set_symbol("│");
            cell.set_style(Style::default().fg(Color::DarkGray));
        }
    }

    // Intersection of header line and gutter separator
    if let Some(cell) = buf.cell_mut(Position::new(sep_x, y_header_line)) {
        cell.set_symbol("┼");
        cell.set_style(Style::default().fg(Color::DarkGray));
    }

    // ── Header row ────────────────────────────────────────────────────────────

    let mut x = area.x + gutter_width + EDGE_PADDING;
    for (col_idx, header) in loaded.headers.iter().enumerate().skip(loaded.col_offset) {
        if x >= area.x + area.width {
            break;
        }
        let width = column_widths[col_idx];
        let effective_width = width.saturating_sub(NUM_SPACES_BETWEEN_COLUMNS);

        let is_selected_col = matches!(
            loaded.mode,
            TableMode::VisualColumn if col_idx == loaded.cursor_col
        );

        let style = if is_selected_col && focused {
            Style::default()
                .bg(Color::Rgb(28, 42, 74))
                .fg(Color::White)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default()
                .fg(Color::Blue)
                .add_modifier(Modifier::BOLD)
        };

        // Pad to full effective width for clean full-width highlight.
        let padded = format!("{:width$}", header.as_str(), width = effective_width as usize);
        let span = Span::styled(padded, style);
        buf.set_span(x, y_header_text, &span, effective_width);

        x += width;
    }

    // Left-scroll indicator
    if has_more_left {
        let indicator_x = area.x + gutter_width + EDGE_PADDING;
        if let Some(cell) = buf.cell_mut(Position::new(indicator_x, y_header_text)) {
            cell.set_symbol("◂");
            cell.set_style(Style::default().fg(Color::DarkGray));
        }
    }
    // Right-scroll indicator
    if has_more_right {
        let indicator_x = (area.x + area.width).saturating_sub(1 + EDGE_PADDING);
        if let Some(cell) = buf.cell_mut(Position::new(indicator_x, y_header_text)) {
            cell.set_symbol("▸");
            cell.set_style(Style::default().fg(Color::DarkGray));
        }
    }

    // ── Data rows ─────────────────────────────────────────────────────────────

    let visible_rows = (area.y + area.height).saturating_sub(y_first_record) as usize;
    let start_row = loaded.row_offset;
    let end_row = (start_row + visible_rows).min(loaded.rows.len());

    for (view_row, row_idx) in (start_row..end_row).enumerate() {
        let y = y_first_record + view_row as u16;
        if y >= area.y + area.height {
            break;
        }

        let row = &loaded.rows[row_idx];
        let is_selected_row = match loaded.mode {
            TableMode::VisualRow => row_idx == loaded.row_cursor,
            _ => false,
        };

        // Row number
        let row_num_str = format!("{}", row_idx + 1);
        let row_num_style = if is_selected_row && focused {
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD | Modifier::UNDERLINED)
        } else if is_selected_row {
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::DarkGray)
        };
        let row_num_span = Span::styled(
            format!("{:>width$}", row_num_str, width = row_num_width as usize),
            row_num_style,
        );
        buf.set_span(area.x + ROW_NUMBER_PADDING, y, &row_num_span, row_num_width);

        // Cells
        let mut x = area.x + gutter_width + EDGE_PADDING;
        for (col_idx, cell_text) in row.iter().enumerate().skip(loaded.col_offset) {
            if x >= area.x + area.width {
                break;
            }
            let width = column_widths[col_idx];
            let effective_width = width.saturating_sub(NUM_SPACES_BETWEEN_COLUMNS);

            let is_selected = match loaded.mode {
                TableMode::Normal | TableMode::Insert => {
                    row_idx == loaded.row_cursor && col_idx == loaded.cursor_col
                }
                TableMode::VisualRow => row_idx == loaded.row_cursor,
                TableMode::VisualColumn => col_idx == loaded.cursor_col,
            };

            let style = if is_selected && focused {
                Style::default()
                    .bg(Color::Rgb(28, 42, 74))
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD)
            } else if is_selected {
                Style::default().add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };

            let display = if cell_text.is_empty() { " " } else { cell_text.as_str() };
            let padded = format!("{:width$}", display, width = effective_width as usize);
            let span = Span::styled(padded, style);
            buf.set_span(x, y, &span, effective_width);

            x += width;
        }

        // Left-scroll indicator at row start
        if has_more_left {
            let indicator_x = area.x + gutter_width + EDGE_PADDING;
            if let Some(cell) = buf.cell_mut(Position::new(indicator_x, y)) {
                cell.set_symbol("◂");
                cell.set_style(Style::default().fg(Color::DarkGray));
            }
        }
        // Right-scroll indicator at row end
        if has_more_right {
            let indicator_x = (area.x + area.width).saturating_sub(1 + EDGE_PADDING);
            if let Some(cell) = buf.cell_mut(Position::new(indicator_x, y)) {
                cell.set_symbol("▸");
                cell.set_style(Style::default().fg(Color::DarkGray));
            }
        }
    }

    // ── "More rows" indicator at bottom ───────────────────────────────────────

    if end_row < loaded.rows.len() {
        let indicator_y = (area.y + area.height).saturating_sub(1);
        let indicator_x = area.x + gutter_width + 1;
        if let Some(cell) = buf.cell_mut(Position::new(indicator_x, indicator_y)) {
            cell.set_symbol("▾");
            cell.set_style(Style::default().fg(Color::DarkGray));
        }
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// A minimal bordered block — accent colour when the pane has focus.
fn pane_block(title: &str, focused: bool) -> Block<'_> {
    let border_style = if focused {
        Style::default().fg(Color::Blue)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let title_style = if focused {
        Style::default()
            .fg(Color::Blue)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    Block::default()
        .title(if title.is_empty() {
            Line::from("")
        } else {
            Line::from(format!(" {title} ")).style(title_style)
        })
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(border_style)
}
