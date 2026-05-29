use super::{make_title, pane_block};
use crate::tui::state::{TableMode, pane_layout::Pane};
use crate::tui::ui::dashboard::sql_highlight;
use ratatui::{
    Frame,
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    text::Span,
    widgets::{
        Block, BorderType, Borders, Clear, Paragraph, Scrollbar, ScrollbarOrientation,
        ScrollbarState,
    },
};
use unicode_width::UnicodeWidthChar;

pub fn render(frame: &mut Frame, area: Rect, pane: &Pane, focused: bool) {
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

    // ── Gutter ───────────────────────────────────────────────────────────────
    let gutter_w = {
        let digits = pane.query_text.len().to_string().len().max(3);
        ((digits + 1) as u16).min(padded.width.saturating_sub(1))
    };
    let gutter_area = Rect {
        x: padded.x,
        y: padded.y,
        width: gutter_w,
        height: padded.height,
    };
    let text_area = Rect {
        x: padded.x + gutter_w,
        y: padded.y,
        width: padded.width.saturating_sub(gutter_w).max(1),
        height: padded.height,
    };

    let gutter_inner_w = gutter_w.saturating_sub(1) as usize; // reserve right padding
    let start_row = pane.query_row_offset;
    let end_row = (start_row + gutter_area.height as usize).min(pane.query_text.len());
    let line_numbers: Vec<ratatui::text::Line> = (start_row..end_row)
        .map(|line_idx| {
            let display_num = if pane.mode == TableMode::Insert || line_idx == pane.query_cursor.0 {
                line_idx + 1
            } else {
                line_idx.abs_diff(pane.query_cursor.0)
            };
            let num_str = format!("{:>width$}", display_num, width = gutter_inner_w);
            let color = if line_idx == pane.query_cursor.0 {
                Color::White
            } else {
                Color::DarkGray
            };
            ratatui::text::Line::from(Span::styled(num_str, Style::default().fg(color)))
        })
        .collect();
    frame.render_widget(Paragraph::new(line_numbers), gutter_area);

    // ── Cursor-line background (vim cursorline style) ──────────────────────────
    let (cursor_row, _cursor_col) = pane.query_cursor;
    let cursor_y_visible = cursor_row.saturating_sub(pane.query_row_offset) as u16;
    if focused && cursor_y_visible < text_area.height {
        let cursor_line_bg = Rect {
            x: text_area.x,
            y: text_area.y + cursor_y_visible,
            width: text_area.width,
            height: 1,
        };
        let bg_style = Style::default().bg(Color::Rgb(41, 46, 66));
        frame.render_widget(Paragraph::new("").style(bg_style), cursor_line_bg);
    }

    // ── Syntax-highlighted SQL rendering ───────────────────────────────────────
    let buf = frame.buffer_mut();
    for (line_idx, line) in pane
        .query_text
        .iter()
        .enumerate()
        .skip(start_row)
        .take(end_row - start_row)
    {
        let y = text_area.y + (line_idx - start_row) as u16;
        if y >= text_area.y + text_area.height {
            break;
        }
        let spans = sql_highlight::tokenize_line(line);
        render_line_spans(buf, y, text_area, pane.query_scroll_offset, &spans);
    }

    // Visual selection overlay.
    render_visual_selection_overlay(buf, pane, text_area, start_row, end_row);

    // Position terminal cursor manually.
    let (cursor_row, cursor_col) = pane.query_cursor;
    let cursor_y = text_area.y + cursor_row.saturating_sub(pane.query_row_offset) as u16;
    let cursor_vx = pane
        .query_text
        .get(cursor_row)
        .map_or(0, |line| sql_highlight::cursor_visual_x(line, cursor_col));
    let cursor_x = text_area.x + cursor_vx.saturating_sub(pane.query_scroll_offset) as u16;
    let cursor_x = cursor_x.min(text_area.right().saturating_sub(1));
    let cursor_y = cursor_y.min(text_area.bottom().saturating_sub(1));
    if focused {
        frame.set_cursor_position((cursor_x, cursor_y));
    }

    // ── Autocomplete popup ───────────────────────────────────────────────────
    if focused {
        if let Some(selected) = pane.autocomplete_selected {
            let total = pane.autocomplete_matches.len();
            if total > 0 {
                const MAX_VISIBLE: usize = 10;
                let visible_count = total.min(MAX_VISIBLE);
                let needs_scrollbar = total > visible_count;

                let max_w = pane
                    .autocomplete_matches
                    .iter()
                    .map(|m| m.len())
                    .max()
                    .unwrap_or(8);
                let popup_w = (max_w + 4) as u16;
                let popup_h = (visible_count + 2) as u16;

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

                // Compute scroll offset so selected item is always visible.
                let mut offset = 0usize;
                if selected >= offset + visible_count {
                    offset = selected + 1 - visible_count;
                }
                if selected < offset {
                    offset = selected;
                }

                let lines: Vec<ratatui::text::Line> = pane
                    .autocomplete_matches
                    .iter()
                    .enumerate()
                    .skip(offset)
                    .take(visible_count)
                    .map(|(i, m)| {
                        let is_selected = i == selected;
                        let style = if is_selected {
                            Style::default().bg(Color::DarkGray).fg(Color::White).bold()
                        } else {
                            Style::default().fg(Color::White)
                        };
                        ratatui::text::Line::from(ratatui::text::Span::styled(
                            format!(" {} ", m),
                            style,
                        ))
                    })
                    .collect();
                frame.render_widget(Paragraph::new(lines), inner_popup);

                // Scrollbar embedded in the right border.
                if needs_scrollbar {
                    let scrollbar_area = Rect {
                        x: popup.x + popup.width - 1,
                        y: popup.y + 1,
                        width: 1,
                        height: popup.height.saturating_sub(2),
                    };
                    let mut scrollbar_state = ScrollbarState::new(total)
                        .position(offset)
                        .viewport_content_length(visible_count);
                    let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
                        .begin_symbol(None)
                        .end_symbol(None)
                        .track_symbol(Some("│"))
                        .thumb_symbol("█");
                    frame.render_stateful_widget(scrollbar, scrollbar_area, &mut scrollbar_state);
                }
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct QueryCursor {
    row: usize,
    col: usize,
}

fn render_line_spans(
    buf: &mut Buffer,
    y: u16,
    text_area: Rect,
    scroll_offset: usize,
    spans: &[ratatui::text::Span<'_>],
) {
    let mut x = text_area.x;
    let max_x = text_area.x + text_area.width;
    let mut accumulated_vx = 0usize;

    for span in spans {
        let span_text = &span.content;
        let span_vx: usize = span_text
            .chars()
            .map(|c| UnicodeWidthChar::width(c).unwrap_or(1))
            .sum();

        // Skip spans completely before visible area
        if accumulated_vx + span_vx <= scroll_offset {
            accumulated_vx += span_vx;
            continue;
        }

        // Stop if we've passed the visible area
        if accumulated_vx >= scroll_offset + text_area.width as usize {
            break;
        }

        // This span is at least partially visible
        let span_start_vx = accumulated_vx;
        let skip_vx = scroll_offset.saturating_sub(span_start_vx);
        let take_vx = (scroll_offset + text_area.width as usize).saturating_sub(span_start_vx);
        let take_vx = take_vx.min(span_vx);
        let visible_vx = take_vx.saturating_sub(skip_vx);

        if visible_vx == 0 {
            accumulated_vx += span_vx;
            continue;
        }

        // Find byte positions for the visible portion
        let mut byte_start = 0usize;
        let mut byte_end = 0usize;
        let mut display_seen = 0usize;
        for ch in span_text.chars() {
            let ch_width = UnicodeWidthChar::width(ch).unwrap_or(1);
            if display_seen == skip_vx {
                byte_start = byte_end;
            }
            byte_end += ch.len_utf8();
            display_seen += ch_width;
            if display_seen >= take_vx {
                break;
            }
        }
        if display_seen <= skip_vx {
            byte_start = span_text.len();
        }

        let visible_text = &span_text[byte_start..byte_end.min(span_text.len())];
        let visible_width = visible_text
            .chars()
            .map(|c| UnicodeWidthChar::width(c).unwrap_or(1))
            .sum::<usize>() as u16;
        let draw_width = (max_x - x).min(visible_width);

        if draw_width > 0 && x < max_x {
            buf.set_span(x, y, &Span::styled(visible_text, span.style), draw_width);
            x += draw_width;
        }

        accumulated_vx += span_vx;
    }
}

fn render_visual_selection_overlay(
    buf: &mut Buffer,
    pane: &Pane,
    text_area: Rect,
    start_row: usize,
    end_row: usize,
) {
    let Some(anchor) = pane.query_visual_anchor else {
        return;
    };

    let anchor = QueryCursor {
        row: anchor.0,
        col: anchor.1,
    };
    let cursor = QueryCursor {
        row: pane.query_cursor.0,
        col: pane.query_cursor.1,
    };

    let style = Style::default().bg(Color::DarkGray).fg(Color::White);

    if pane.query_visual_line_mode {
        let line_start = anchor.row.min(cursor.row);
        let line_end = anchor.row.max(cursor.row);

        for row in line_start.max(start_row)..line_end.min(end_row.saturating_sub(1)) + 1 {
            let y = text_area.y + (row - start_row) as u16;
            if y >= text_area.bottom() {
                break;
            }
            buf.set_style(
                Rect {
                    x: text_area.x,
                    y,
                    width: text_area.width,
                    height: 1,
                },
                style,
            );
        }
        return;
    }

    let min = anchor.min(cursor);
    let max = anchor.max(cursor);
    let end = cursor_after_current_char(&pane.query_text, max);

    if min == end {
        return;
    }

    for row in min.row.max(start_row)..end.row.min(end_row.saturating_sub(1)) + 1 {
        let y = text_area.y + (row - start_row) as u16;
        if y >= text_area.bottom() {
            break;
        }

        let line = pane.query_text.get(row).map_or("", String::as_str);
        let line_len = line.chars().count();

        let start_col = if row == min.row { min.col } else { 0 };
        let end_col = if row == end.row { end.col } else { line_len };
        if start_col >= end_col {
            continue;
        }

        let start_vx = sql_highlight::cursor_visual_x(line, start_col);
        let end_vx = sql_highlight::cursor_visual_x(line, end_col);

        if end_vx <= pane.query_scroll_offset {
            continue;
        }

        let visible_start = start_vx.saturating_sub(pane.query_scroll_offset);
        let visible_end = end_vx.saturating_sub(pane.query_scroll_offset);

        let clip_start = visible_start.min(text_area.width as usize);
        let clip_end = visible_end.min(text_area.width as usize);
        if clip_start >= clip_end {
            continue;
        }

        buf.set_style(
            Rect {
                x: text_area.x + clip_start as u16,
                y,
                width: (clip_end - clip_start) as u16,
                height: 1,
            },
            style,
        );
    }
}

fn cursor_after_current_char(lines: &[String], cur: QueryCursor) -> QueryCursor {
    let line_len = lines.get(cur.row).map_or(0, |line| line.chars().count());

    if line_len == 0 {
        if cur.row + 1 < lines.len() {
            QueryCursor {
                row: cur.row + 1,
                col: 0,
            }
        } else {
            QueryCursor {
                row: cur.row,
                col: 0,
            }
        }
    } else if cur.col + 1 < line_len {
        QueryCursor {
            row: cur.row,
            col: cur.col + 1,
        }
    } else if cur.row + 1 < lines.len() {
        QueryCursor {
            row: cur.row + 1,
            col: 0,
        }
    } else {
        QueryCursor {
            row: cur.row,
            col: line_len,
        }
    }
}
