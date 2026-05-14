//! Individual pane renderers — TableList, TableView, SchemaView, QueryEditor.

use ratatui::{
    Frame,
    layout::Rect,
    prelude::Position,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Paragraph},
};
use crate::tui::state::{
    TableMode,
    dashboard::{DashboardState, LoadedTable},
    pane_layout::{Pane, PaneId, PaneType},
};
use crate::tui::ui::dashboard::sql_highlight;

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
        PaneType::QueryResults => render_query_results(frame, area, pane, dash, focused),
    }
}

// ── Border title helpers ──────────────────────────────────────────────────────

fn make_title(pane: &Pane) -> String {
    match pane.kind {
        PaneType::TableList => format!(" {} ", pane.display_id),
        PaneType::TableView => {
            if let Some(ref table) = pane.bound_table {
                let dirty = !pane.pending_updates.is_empty() || !pane.pending_deletes.is_empty();
                let filtered = pane.filter.is_some();
                let sorted = pane.sort_col.is_some();
                let mut tags = String::new();
                if filtered {
                    tags.push_str(" [filtered]");
                }
                if sorted {
                    tags.push_str(" [sorted]");
                }
                if dirty {
                    tags.push('*');
                }
                format!(" {}: {}{} ", pane.display_id, table, tags)
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
        PaneType::QueryEditor => {
            format!(" {}: Query ", pane.display_id)
        }
        PaneType::QueryResults => {
            if let Some(idx) = pane.bound_query_idx {
                format!(" {}: Result {}/{} ", pane.display_id, idx + 1, pane.query_result_count.max(1))
            } else {
                format!(" {}: Result ", pane.display_id)
            }
        }
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

// ── Match highlighting helpers ────────────────────────────────────────────────

/// Highlight the first substring match of `query` in `text` with bold+yellow.
/// Characters before and after the match keep `base` style.
fn search_highlight_spans<'a>(text: &'a str, query: &str, base: Style) -> Vec<Span<'a>> {
    if query.is_empty() {
        return vec![Span::styled(text, base)];
    }
    let lower_text: Vec<char> = text.to_lowercase().chars().collect();
    let lower_query: Vec<char> = query.to_lowercase().chars().collect();

    if lower_query.len() > lower_text.len() {
        return vec![Span::styled(text, base)];
    }

    if let Some(start_char) = lower_text
        .windows(lower_query.len())
        .position(|w| w == lower_query.as_slice())
    {
        let chars: Vec<char> = text.chars().collect();
        let start_byte: usize = chars[..start_char].iter().map(|c| c.len_utf8()).sum();
        let end_byte: usize =
            chars[..start_char + lower_query.len()].iter().map(|c| c.len_utf8()).sum();

        let mut spans = vec![];
        if start_byte > 0 {
            spans.push(Span::styled(&text[0..start_byte], base));
        }
        spans.push(Span::styled(
            &text[start_byte..end_byte],
            base.fg(Color::Yellow).add_modifier(Modifier::BOLD),
        ));
        if end_byte < text.len() {
            spans.push(Span::styled(&text[end_byte..], base));
        }
        spans
    } else {
        vec![Span::styled(text, base)]
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
        // Prefer live_search for highlighting while typing, fall back to last_search.
        let live_query = pane.live_search.as_ref().map(|s| s.query.as_str());
        let committed_query = pane.last_search.as_ref().map(|s| s.query.as_str());

        for table_idx in start..end {
            let table = &dash.tables[table_idx];
            let selected = table_idx == pane.nav_cursor;

            let base_style = if selected && focused {
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

            let name_spans = if let Some(query) = live_query {
                search_highlight_spans(table, query, base_style)
            } else if let Some(query) = committed_query {
                search_highlight_spans(table, query, base_style)
            } else {
                vec![Span::styled(table.as_str(), base_style)]
            };

            // Pad the remainder of the line with spaces so the background colour
            // extends to the right edge on selected rows.
            let text_w: usize = name_spans.iter().map(|s| s.content.chars().count()).sum();
            let pad = (inner.width as usize).saturating_sub(text_w);
            let mut spans = name_spans;
            if pad > 0 {
                spans.push(Span::styled(" ".repeat(pad), base_style));
            }
            lines.push(Line::from(spans));
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

    let loading_this = dash.pending_load.as_ref().map_or(false, |q| {
        pane.bound_table.as_ref() == Some(&q.table)
    });

    if dash.loading && loading_this {
        frame.render_widget(
            Paragraph::new(Span::styled(" Loading…", Style::default().fg(Color::DarkGray))),
            inner,
        );
        return;
    }

    if let Some(ref err) = dash.error {
        if loading_this {
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

        let search_query = pane
            .live_search
            .as_ref()
            .map(|s| s.query.as_str())
            .or_else(|| pane.last_search.as_ref().map(|s| s.query.as_str()));
        if search_query.is_some() && col_idx == cursor_col {
            let hl_spans = search_highlight_spans(header, search_query.unwrap(), style);
            let mut cell_x = x;
            let max_x = x + effective_width;
            for span in hl_spans {
                let w = span.content.chars().count() as u16;
                if cell_x >= max_x { break; }
                let avail = (max_x - cell_x).min(w);
                buf.set_span(cell_x, y_header_text, &span, avail);
                cell_x += avail;
            }
            if cell_x < max_x {
                let pad = " ".repeat((max_x - cell_x) as usize);
                buf.set_span(cell_x, y_header_text, &Span::styled(pad, style), max_x - cell_x);
            }
        } else {
            let padded = format!("{:width$}", header.as_str(), width = effective_width as usize);
            buf.set_span(x, y_header_text, &Span::styled(padded, style), effective_width);
        }
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

    // Helper: is this row inside the current visual selection?
    let in_visual_row = |row_idx: usize| -> bool {
        if pane.mode != TableMode::VisualRow {
            return false;
        }
        let cursor = pane.row_cursor;
        match pane.visual_anchor {
            Some(anchor) if row_idx >= anchor.min(cursor) && row_idx <= anchor.max(cursor) => true,
            _ => row_idx == cursor,
        }
    };

    for row_idx in start_row..end_row {
        let y = y_first_record + (row_idx - start_row) as u16;
        if y >= area.y + area.height {
            break;
        }
        let row = &loaded.rows[row_idx];
        let is_selected_row = in_visual_row(row_idx);
        let is_cursor_row = row_idx == pane.row_cursor;

        let is_deleted_row = pane.pending_deletes.iter().any(|pk| {
            loaded.schema.iter().position(|c| c.is_primary_key).map_or(false, |pk_idx| {
                row_idx < loaded.rows.len() && loaded.rows[row_idx].get(pk_idx) == Some(pk)
            })
        });

        // Alternating row background — every odd row gets a subtle dark shade.
        let alt_bg = if row_idx % 2 == 1 {
            Color::Rgb(30, 32, 42)
        } else {
            Color::Reset
        };

        let row_num_str = format!("{}", row_idx + 1);
        let row_num_style = if is_cursor_row && focused {
            Style::default().fg(Color::White).add_modifier(Modifier::BOLD | Modifier::UNDERLINED)
        } else if is_deleted_row {
            Style::default().fg(Color::Red).add_modifier(Modifier::CROSSED_OUT)
        } else if is_selected_row && focused {
            Style::default().fg(Color::White).add_modifier(Modifier::BOLD)
        } else if is_selected_row {
            Style::default().fg(Color::White).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::DarkGray).bg(alt_bg)
        };
        let row_num_span = Span::styled(
            format!("{:>width$}", row_num_str, width = row_num_width as usize),
            row_num_style,
        );
        buf.set_span(area.x + ROW_NUMBER_PADDING, y, &row_num_span, row_num_width);

        // Fill the entire data area for this row with the alternating background.
        let data_start = area.x + gutter_width;
        let data_end = right_boundary;
        if data_end > data_start {
            let fill_w = (data_end - data_start) as usize;
            buf.set_span(
                data_start,
                y,
                &Span::styled(" ".repeat(fill_w), Style::default().bg(alt_bg)),
                fill_w as u16,
            );
        }

        let mut x = area.x + gutter_width + EDGE_PADDING;
        for (col_idx, cell_text) in row.iter().enumerate().skip(col_offset) {
            if x >= right_boundary {
                break;
            }
            let width = column_widths[col_idx];
            let effective_width = width.saturating_sub(NUM_SPACES_BETWEEN_COLUMNS).min(right_boundary - x);

            let is_selected = match pane.mode {
                TableMode::Normal | TableMode::Insert => row_idx == pane.row_cursor && col_idx == cursor_col,
                TableMode::VisualRow => in_visual_row(row_idx),
                TableMode::VisualColumn => col_idx == cursor_col,
            };

            let staged_value = pane.pending_updates
                .iter()
                .find(|(r, c, _)| *r == row_idx && *c == col_idx)
                .map(|(_, _, val)| val.as_str());
            let is_modified = staged_value.is_some();
            let is_deleted_row = pane.pending_deletes.iter().any(|pk| {
                loaded.schema.iter().position(|c| c.is_primary_key).map_or(false, |pk_idx| {
                    row_idx < loaded.rows.len() && loaded.rows[row_idx].get(pk_idx) == Some(pk)
                })
            });

            let style = if is_selected && focused {
                Style::default().bg(Color::Rgb(28, 42, 74)).fg(Color::White).add_modifier(Modifier::BOLD)
            } else if is_selected {
                Style::default().bg(alt_bg).add_modifier(Modifier::BOLD)
            } else if is_modified {
                Style::default().fg(Color::Black).bg(Color::Yellow)
            } else if is_deleted_row {
                Style::default().bg(alt_bg).fg(Color::Red).add_modifier(Modifier::CROSSED_OUT)
            } else {
                Style::default().fg(Color::White).bg(alt_bg)
            };

            let display_text = staged_value.unwrap_or(cell_text.as_str());
            let display = if display_text.is_empty() { " " } else { display_text };

            // Fuzzy highlight on the selected column when a search is active.
            let search_query = pane
                .live_search
                .as_ref()
                .map(|s| s.query.as_str())
                .or_else(|| pane.last_search.as_ref().map(|s| s.query.as_str()));
            let is_search_col = search_query.is_some() && col_idx == cursor_col;
            if is_search_col {
                let hl_spans = search_highlight_spans(display, search_query.unwrap(), style);
                let mut cell_x = x;
                let max_x = x + effective_width;
                for span in hl_spans {
                    let w = span.content.chars().count() as u16;
                    if cell_x >= max_x {
                        break;
                    }
                    let avail = (max_x - cell_x).min(w);
                    buf.set_span(cell_x, y, &span, avail);
                    cell_x += avail;
                }
                // Pad remainder with spaces.
                if cell_x < max_x {
                    let pad = " ".repeat((max_x - cell_x) as usize);
                    buf.set_span(cell_x, y, &Span::styled(pad, style), max_x - cell_x);
                }
            } else {
                let padded = format!("{:width$}", display, width = effective_width as usize);
                buf.set_span(x, y, &Span::styled(padded, style), effective_width);
            }
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

    // Add padding around the text area.
    let pad = 1u16;
    let padded = Rect {
        x: inner.x + pad,
        y: inner.y + pad,
        width: inner.width.saturating_sub(pad * 2),
        height: inner.height.saturating_sub(pad * 2),
    };

    // ── Cursor-line background (vim cursorline style) ────────────────────────
    let (cursor_row, _cursor_col) = pane.query_cursor;
    let cursor_y_in_pane = cursor_row as u16;
    if cursor_y_in_pane < padded.height {
        let cursor_line_bg = Rect {
            x: padded.x,
            y: padded.y + cursor_y_in_pane,
            width: padded.width,
            height: 1,
        };
        let bg_style = Style::default().bg(Color::Rgb(41, 46, 66));
        frame.render_widget(
            Paragraph::new("").style(bg_style),
            cursor_line_bg,
        );
    }

    // Syntax-highlighted SQL rendering.
    let highlighted = sql_highlight::highlight_sql_lines(&pane.query_text);
    let paragraph = Paragraph::new(highlighted);
    frame.render_widget(paragraph, padded);

    // Position terminal cursor manually.
    let (cursor_row, cursor_col) = pane.query_cursor;
    let cursor_y = padded.y + cursor_row as u16;
    let cursor_x = if let Some(line) = pane.query_text.get(cursor_row) {
        padded.x + sql_highlight::cursor_visual_x(line, cursor_col) as u16
    } else {
        padded.x
    };
    frame.set_cursor_position((cursor_x, cursor_y));

    // ── Autocomplete popup ───────────────────────────────────────────────────
    if let Some(selected) = pane.autocomplete_selected {
        if !pane.autocomplete_matches.is_empty() {
            let max_w = pane.autocomplete_matches.iter().map(|m| m.len()).max().unwrap_or(8);
            let popup_w = (max_w + 4).min(inner.width.saturating_sub(4) as usize) as u16;
            let popup_h = (pane.autocomplete_matches.len() as u16 + 2)
                .min(inner.height.saturating_sub(2));

            let popup = Rect {
                x: cursor_x.min(inner.right().saturating_sub(popup_w)),
                y: cursor_y + 1,
                width: popup_w,
                height: popup_h,
            };

            frame.render_widget(Clear, popup);
            let block = Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(Color::White))
                .style(Style::default().bg(Color::Reset));
            let inner_popup = block.inner(popup);
            frame.render_widget(block, popup);

            let lines: Vec<Line> = pane
                .autocomplete_matches
                .iter()
                .enumerate()
                .map(|(i, m)| {
                    let is_selected = i == selected;
                    let style = if is_selected {
                        Style::default().bg(Color::DarkGray).fg(Color::White).add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(Color::White)
                    };
                    Line::from(Span::styled(format!(" {} ", m), style))
                })
                .collect();
            frame.render_widget(Paragraph::new(lines), inner_popup);
        }
    }
}

// ── QueryResults pane ─────────────────────────────────────────────────────────

fn render_query_results(
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

    let Some(idx) = pane.bound_query_idx else {
        frame.render_widget(
            Paragraph::new(Span::styled(
                " No query executed yet.",
                Style::default().fg(Color::DarkGray),
            )),
            inner,
        );
        return;
    };

    let Some(result) = dash.query_results.get(idx) else {
        frame.render_widget(
            Paragraph::new(Span::styled(
                " Result not available.",
                Style::default().fg(Color::DarkGray),
            )),
            inner,
        );
        return;
    };

    if let Some(ref err) = result.error {
        frame.render_widget(
            Paragraph::new(Span::styled(
                format!(" Error: {err}"),
                Style::default().fg(Color::Red),
            )),
            inner,
        );
        return;
    }

    if result.headers.is_empty() {
        frame.render_widget(
            Paragraph::new(Span::styled(
                " Query returned no columns.",
                Style::default().fg(Color::DarkGray),
            )),
            inner,
        );
        return;
    }

    // Build a LoadedTable-like struct to reuse the table renderer.
    let loaded = crate::tui::state::dashboard::LoadedTable {
        name: format!("Result {}", idx + 1),
        schema: vec![], // no schema for arbitrary queries
        headers: result.headers.clone(),
        rows: result.rows.clone(),
    };

    render_loaded_table(frame, inner, pane, &loaded, focused);
}
