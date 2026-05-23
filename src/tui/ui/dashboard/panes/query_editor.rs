use super::{make_title, pane_block};
use crate::tui::state::pane_layout::Pane;
use crate::tui::ui::dashboard::sql_highlight;
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Style},
    widgets::{Block, BorderType, Borders, Clear, Paragraph},
};

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
        frame.render_widget(Paragraph::new("").style(bg_style), cursor_line_bg);
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
            let max_w = pane
                .autocomplete_matches
                .iter()
                .map(|m| m.len())
                .max()
                .unwrap_or(8);
            let popup_w = (max_w + 4).min(inner.width.saturating_sub(4) as usize) as u16;
            let popup_h =
                (pane.autocomplete_matches.len() as u16 + 2).min(inner.height.saturating_sub(2));

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

            let lines: Vec<ratatui::text::Line> = pane
                .autocomplete_matches
                .iter()
                .enumerate()
                .map(|(i, m)| {
                    let is_selected = i == selected;
                    let style = if is_selected {
                        Style::default()
                            .bg(Color::DarkGray)
                            .fg(Color::White)
                            .add_modifier(ratatui::style::Modifier::BOLD)
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
        }
    }
}
