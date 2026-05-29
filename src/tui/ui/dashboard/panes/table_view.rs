use super::{
    EDGE_PADDING, MAX_COLUMN_WIDTH_FRACTION, NUM_SPACES_BETWEEN_COLUMNS, ROW_NUMBER_PADDING,
    make_title, pane_block, search_highlight_spans,
};
use crate::tui::state::{
    TableMode,
    pane_layout::{DisplayRowRef, Pane, PaneType},
};
use ratatui::{
    Frame,
    layout::Rect,
    prelude::Position,
    style::{Color, Style},
    text::Span,
    widgets::Paragraph,
};

pub fn render(
    frame: &mut Frame,
    area: Rect,
    pane: &Pane,
    state: &crate::tui::state::app::AppState,
    focused: bool,
) {
    let title = make_title(pane);
    let block = pane_block(&title, focused);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let Some(tab) = state.active_tab() else {
        return;
    };

    let loading_this = tab
        .pending_load
        .as_ref()
        .map_or(false, |q| pane.bound_table.as_ref() == Some(&q.table));

    if tab.loading && loading_this {
        frame.render_widget(
            Paragraph::new(Span::styled(
                " Loading…",
                Style::default().fg(Color::DarkGray),
            )),
            inner,
        );
        return;
    }

    if let Some(ref err) = tab.error {
        if loading_this {
            frame.render_widget(
                Paragraph::new(Span::styled(
                    format!(" {err}"),
                    Style::default().fg(Color::Red),
                )),
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

    let Some(ref loaded) = state.table_cache.get(table_name) else {
        frame.render_widget(
            Paragraph::new(Span::styled(
                " Loading table data…",
                Style::default().fg(Color::DarkGray),
            )),
            inner,
        );
        return;
    };

    render_loaded(frame, inner, pane, loaded, focused);
}

pub fn render_loaded(
    frame: &mut Frame,
    area: Rect,
    pane: &Pane,
    loaded: &crate::tui::state::dashboard::LoadedTable,
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

    let is_table_view = pane.kind == PaneType::TableView;
    let total_rows = if is_table_view {
        pane.total_table_rows(loaded.rows.len())
    } else {
        loaded.rows.len()
    };
    let max_row_num = total_rows.max(1);
    let row_num_width = format!("{}", max_row_num).len() as u16;
    let gutter_width = row_num_width + 2 * ROW_NUMBER_PADDING + 1;

    let data_area_width = area
        .width
        .saturating_sub(gutter_width)
        .saturating_sub(2 * EDGE_PADDING);
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
            if is_table_view {
                for staged in &pane.pending_inserts {
                    if let Some(cell) = staged.values.get(col_idx) {
                        w = w.max(cell.len() as u16);
                    }
                }
            }
            w = w.min(max_single_width);
            w + NUM_SPACES_BETWEEN_COLUMNS
        })
        .collect();

    let mut col_offset = pane.col_offset.min(loaded.headers.len().saturating_sub(1));
    let cursor_col = pane.cursor_col.min(loaded.headers.len().saturating_sub(1));

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
        if cursor_col >= col_offset + visible_cols
            && col_offset < loaded.headers.len().saturating_sub(1)
        {
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
    let right_boundary = if has_more_right {
        conservative_right
    } else {
        (area.x + area.width).saturating_sub(EDGE_PADDING)
    };

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
        let effective_width = width
            .saturating_sub(NUM_SPACES_BETWEEN_COLUMNS)
            .min(right_boundary - x);

        let is_selected_col = matches!(pane.mode, TableMode::VisualColumn if col_idx == cursor_col);
        let style = if is_selected_col && focused {
            Style::default()
                .bg(Color::Rgb(28, 42, 74))
                .fg(Color::White)
                .bold()
        } else {
            Style::default().fg(Color::Blue).bold()
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
                if cell_x >= max_x {
                    break;
                }
                let avail = (max_x - cell_x).min(w);
                buf.set_span(cell_x, y_header_text, &span, avail);
                cell_x += avail;
            }
            if cell_x < max_x {
                let pad = " ".repeat((max_x - cell_x) as usize);
                buf.set_span(
                    cell_x,
                    y_header_text,
                    &Span::styled(pad, style),
                    max_x - cell_x,
                );
            }
        } else {
            let padded = format!(
                "{:width$}",
                header.as_str(),
                width = effective_width as usize
            );
            buf.set_span(
                x,
                y_header_text,
                &Span::styled(padded, style),
                effective_width,
            );
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
    let start_row = if total_rows > 0 {
        pane.row_offset.min(total_rows.saturating_sub(1))
    } else {
        0
    };
    let end_row = (start_row + visible_rows).min(total_rows);
    let cursor_row = if total_rows > 0 {
        pane.row_cursor.min(total_rows.saturating_sub(1))
    } else {
        0
    };

    // Helper: is this row inside the current visual selection?
    let in_visual_row = |display_row_idx: usize| -> bool {
        if pane.mode != TableMode::VisualRow {
            return false;
        }
        let cursor = cursor_row;
        match pane.visual_anchor {
            Some(anchor)
                if display_row_idx >= anchor.min(cursor)
                    && display_row_idx <= anchor.max(cursor) =>
            {
                true
            }
            _ => display_row_idx == cursor,
        }
    };

    let pk_idx = loaded.schema.iter().position(|c| c.is_primary_key);
    let is_deleted_existing = |real_row_idx: usize| -> bool {
        pk_idx.map_or(false, |pk_col_idx| {
            loaded
                .rows
                .get(real_row_idx)
                .and_then(|r| r.get(pk_col_idx))
                .map_or(false, |pk_val| {
                    pane.pending_deletes.iter().any(|p| p == pk_val)
                })
        })
    };

    for display_row_idx in start_row..end_row {
        let y = y_first_record + (display_row_idx - start_row) as u16;
        if y >= area.y + area.height {
            break;
        }

        let row_ref = if is_table_view {
            match pane.display_row_ref(loaded.rows.len(), display_row_idx) {
                Some(r) => r,
                None => continue,
            }
        } else {
            DisplayRowRef::Existing(display_row_idx)
        };

        let is_pending_insert = matches!(row_ref, DisplayRowRef::PendingInsert(_));
        let is_selected_row = in_visual_row(display_row_idx);
        let is_cursor_row = display_row_idx == cursor_row;
        let is_deleted_row = match row_ref {
            DisplayRowRef::Existing(real_row_idx) => is_deleted_existing(real_row_idx),
            DisplayRowRef::PendingInsert(_) => false,
        };

        let alt_bg = if is_pending_insert {
            Color::Rgb(24, 40, 24)
        } else if display_row_idx % 2 == 1 && !is_selected_row {
            Color::Rgb(30, 32, 42)
        } else {
            Color::Reset
        };

        let row_num_str = format!("{}", display_row_idx + 1);
        let row_num_style = if is_cursor_row && focused {
            Style::default().fg(Color::White).bold()
        } else if is_pending_insert {
            Style::default().fg(Color::Green).bold()
        } else if is_deleted_row {
            Style::default().fg(Color::Red).crossed_out()
        } else if is_selected_row {
            Style::default().fg(Color::White).bold()
        } else {
            Style::default().fg(Color::DarkGray)
        };
        let row_num_span = Span::styled(
            format!("{:>width$}", row_num_str, width = row_num_width as usize),
            row_num_style,
        );
        buf.set_span(area.x + ROW_NUMBER_PADDING, y, &row_num_span, row_num_width);

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
        for (col_idx, _) in loaded.headers.iter().enumerate().skip(col_offset) {
            if x >= right_boundary {
                break;
            }

            let width = column_widths[col_idx];
            let effective_width = width
                .saturating_sub(NUM_SPACES_BETWEEN_COLUMNS)
                .min(right_boundary - x);

            let is_selected = match pane.mode {
                TableMode::Normal | TableMode::Insert => {
                    display_row_idx == cursor_row && col_idx == cursor_col
                }
                TableMode::VisualRow => in_visual_row(display_row_idx),
                TableMode::VisualColumn => col_idx == cursor_col,
            };

            let (base_text, staged_existing): (&str, Option<&str>) = match row_ref {
                DisplayRowRef::Existing(real_row_idx) => {
                    let staged = pane
                        .pending_updates
                        .iter()
                        .find(|(r, c, _)| *r == real_row_idx && *c == col_idx)
                        .map(|(_, _, val)| val.as_str());
                    let base = loaded
                        .rows
                        .get(real_row_idx)
                        .and_then(|r| r.get(col_idx))
                        .map(|s| s.as_str())
                        .unwrap_or("");
                    (base, staged)
                }
                DisplayRowRef::PendingInsert(insert_idx) => {
                    let val = pane
                        .pending_inserts
                        .get(insert_idx)
                        .and_then(|r| r.values.get(col_idx))
                        .map(|s| s.as_str())
                        .unwrap_or("");
                    (val, None)
                }
            };

            let display_text = staged_existing.unwrap_or(base_text);
            let has_insert_value = is_pending_insert && !display_text.trim().is_empty();
            let is_modified = staged_existing.is_some();

            let style = if is_selected && focused {
                Style::default().bg(Color::Yellow).fg(Color::Black).bold()
            } else if is_selected {
                Style::default().bg(alt_bg).bold()
            } else if is_modified || has_insert_value {
                Style::default().fg(Color::Black).bg(Color::LightGreen)
            } else if is_pending_insert {
                Style::default().fg(Color::Green).bg(alt_bg)
            } else if is_deleted_row {
                Style::default().bg(alt_bg).fg(Color::Red).bold()
            } else {
                Style::default().fg(Color::White).bg(alt_bg)
            };

            let display = if display_text.is_empty() {
                " "
            } else {
                display_text
            };

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

    if end_row < total_rows {
        let indicator_y = (area.y + area.height).saturating_sub(1);
        let indicator_x = area.x + gutter_width + 1;
        if let Some(cell) = buf.cell_mut(Position::new(indicator_x, indicator_y)) {
            cell.set_symbol("▾");
            cell.set_style(Style::default().fg(Color::DarkGray));
        }
    }
}
