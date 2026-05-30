use super::{make_title, pane_block, search_highlight_spans};
use crate::tui::state::pane_layout::Pane;
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
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

    let mut lines: Vec<Line> = vec![];

    let viewport = inner.height.max(1) as usize;
    let start = pane.nav_offset;
    let end = (start + viewport).min(state.tables.len());

    if state.tables.is_empty() {
        lines.push(Line::from(Span::styled(
            "No tables",
            Style::default().fg(Color::DarkGray),
        )));
    } else {
        // Prefer live_search for highlighting while typing, fall back to last_search.
        let live_query = pane.live_search.as_ref().map(|s| s.query.as_str());
        let committed_query = pane.last_search.as_ref().map(|s| s.query.as_str());

        for table_idx in start..end {
            let table = &state.tables[table_idx];
            let selected = table_idx == pane.nav_cursor;

            let row_bg = if selected && focused {
                Some(Color::Rgb(28, 42, 74))
            } else {
                None
            };

            let base_style = if selected && focused {
                Style::default()
                    .bg(Color::Rgb(28, 42, 74))
                    .fg(Color::White)
                    .bold()
            } else if selected {
                Style::default().fg(Color::White).bold()
            } else {
                Style::default().fg(Color::DarkGray)
            };

            let mut count_text = "…".to_string();
            if let Some(loaded) = state.table_cache.get(table) {
                count_text = loaded.rows.len().to_string();
            }

            let total_w = inner.width as usize;
            let count_w = count_text.chars().count();
            let gap_w = 1usize;
            let name_w = total_w.saturating_sub(count_w + gap_w);

            let display_name = {
                let len = table.chars().count();
                if len <= name_w {
                    table.clone()
                } else if name_w <= 1 {
                    "…".to_string()
                } else {
                    let prefix: String = table.chars().take(name_w - 1).collect();
                    format!("{prefix}…")
                }
            };

            let name_spans = if let Some(query) = live_query {
                search_highlight_spans(&display_name, query, base_style)
                    .into_iter()
                    .map(|s| Span::styled(s.content.into_owned(), s.style))
                    .collect::<Vec<_>>()
            } else if let Some(query) = committed_query {
                search_highlight_spans(&display_name, query, base_style)
                    .into_iter()
                    .map(|s| Span::styled(s.content.into_owned(), s.style))
                    .collect::<Vec<_>>()
            } else {
                vec![Span::styled(display_name, base_style)]
            };

            let name_text_w: usize = name_spans.iter().map(|s| s.content.chars().count()).sum();
            let name_pad = name_w.saturating_sub(name_text_w);

            let mut spans = name_spans;
            if name_pad > 0 {
                spans.push(Span::styled(" ".repeat(name_pad), base_style));
            }
            spans.push(Span::styled(" ".repeat(gap_w), base_style));

            let mut count_style = Style::default().fg(Color::Blue);
            if let Some(bg) = row_bg {
                count_style = count_style.bg(bg);
            }
            if selected {
                count_style = count_style.bold();
            }
            spans.push(Span::styled(count_text, count_style));

            // Ensure row background fill reaches right edge when selected+focused.
            let line_w: usize = spans.iter().map(|s| s.content.chars().count()).sum();
            let pad = total_w.saturating_sub(line_w);
            if pad > 0 {
                spans.push(Span::styled(" ".repeat(pad), base_style));
            }

            lines.push(Line::from(spans));
        }
    }

    frame.render_widget(Paragraph::new(lines), inner);
}
