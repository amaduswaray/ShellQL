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
                        crate::tui::state::PaneType::TableList => pane.nav_cursor,
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

    let line = Line::from(vec![
        Span::styled(prefix, Style::default().fg(Color::Yellow).bold()),
        Span::styled(input.clone(), Style::default().fg(Color::White)),
        Span::styled(match_info, Style::default().fg(Color::DarkGray)),
    ]);

    frame.render_widget(Paragraph::new(vec![line]), area);

    let cursor_x = (area.x + 1 + input.len() as u16).min(area.right().saturating_sub(1));
    frame.set_cursor_position((cursor_x, area.y));
}
