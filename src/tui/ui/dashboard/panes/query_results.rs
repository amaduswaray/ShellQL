use super::{make_title, pane_block};
use crate::tui::state::pane_layout::Pane;
use ratatui::{
    Frame,
    layout::Rect,
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

    let Some(result) = tab.query_results.get(idx) else {
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
    let loaded = crate::tui::state::tab::LoadedTable {
        name: format!("Result {}", idx + 1),
        schema: vec![], // no schema for arbitrary queries
        headers: result.headers.clone(),
        rows: result.rows.clone(),
    };

    super::table_view::render_loaded(frame, inner, pane, &loaded, focused);
}
