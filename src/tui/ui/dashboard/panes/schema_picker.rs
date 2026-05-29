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
        let live_query = pane.live_search.as_ref().map(|s| s.query.as_str());
        let committed_query = pane.last_search.as_ref().map(|s| s.query.as_str());

        for table_idx in start..end {
            let table = &state.tables[table_idx];
            let selected = table_idx == pane.nav_cursor;

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

            let name_spans = if let Some(query) = live_query {
                search_highlight_spans(table, query, base_style)
            } else if let Some(query) = committed_query {
                search_highlight_spans(table, query, base_style)
            } else {
                vec![Span::styled(table.as_str(), base_style)]
            };

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
