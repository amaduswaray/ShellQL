use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::Paragraph,
};

use crate::tui::state::{AppState, SearchDirection};

pub fn render(frame: &mut Frame, area: Rect, state: &AppState, direction: SearchDirection) {
    let prefix = match direction {
        SearchDirection::Forward => "/",
        SearchDirection::Backward => "?",
    };
    let input = &state.cmdline.input;

    // Show live match count if available.
    let match_info = if let Some(tab) = state.active_tab() {
        let active_id = tab.tree.active_pane;
        if let Some(pane) = tab.tree.panes.get(&active_id) {
            if let Some(ref live) = pane.live_search {
                if live.matches.is_empty() {
                    String::new()
                } else {
                    // Determine which match the cursor would jump to.
                    let current_pos = match pane.kind {
                        crate::tui::state::PaneType::TableList
                        | crate::tui::state::PaneType::SchemaPicker => pane.nav_cursor,
                        _ => pane.row_cursor,
                    };
                    let idx = match live.direction {
                        SearchDirection::Forward => live
                            .matches
                            .iter()
                            .position(|&m| m >= current_pos)
                            .unwrap_or(0),
                        SearchDirection::Backward => live
                            .matches
                            .iter()
                            .rposition(|&m| m <= current_pos)
                            .unwrap_or(live.matches.len() - 1),
                    };
                    format!(" [{}/{}]", idx + 1, live.matches.len())
                }
            } else {
                String::new()
            }
        } else {
            String::new()
        }
    } else {
        String::new()
    };

    let mut spans = vec![
        Span::styled(prefix, Style::default().fg(Color::Yellow).bold()),
        Span::styled(input.clone(), Style::default().fg(Color::White)),
    ];

    if !match_info.is_empty() {
        let left_w = 1 + input.chars().count();
        let right_w = match_info.chars().count();
        let gap = (area.width as usize)
            .saturating_sub(left_w)
            .saturating_sub(right_w);
        if gap > 0 {
            spans.push(Span::styled(" ".repeat(gap), Style::default()));
        }
        spans.push(Span::styled(
            match_info,
            Style::default().fg(Color::DarkGray),
        ));
    }

    let line = Line::from(spans);
    frame.render_widget(Paragraph::new(vec![line]), area);

    // Cursor after prefix + input_cursor
    let cursor_char = state.cmdline.input_cursor as u16;
    let cursor_x = (area.x + 1 + cursor_char).min(area.right().saturating_sub(1));
    frame.set_cursor_position((cursor_x, area.y));
}
