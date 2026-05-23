use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::Paragraph,
};

use crate::tui::state::{AppMode, AppState, TableMode, TextMode};

pub fn render(frame: &mut Frame, area: Rect, state: &AppState) {
    if let Some(ref loading) = state.cmdline.loading {
        let line = Line::from(Span::styled(
            loading.as_str(),
            Style::default().fg(Color::Yellow),
        ));
        frame.render_widget(Paragraph::new(vec![line]), area);
        return;
    }

    if let Some(ref err) = state.cmdline.error {
        let line = Line::from(Span::styled(
            format!("{err}"),
            Style::default().fg(Color::Red),
        ));
        frame.render_widget(Paragraph::new(vec![line]), area);
        return;
    }

    // When the add-connection form is open, reflect its current text mode.
    if let Some(ref form) = state.form {
        let line = match form.text_mode {
            TextMode::Normal => Line::from(Span::styled(
                " NORMAL ",
                Style::default().fg(Color::Blue).bold(),
            )),

            TextMode::Insert => Line::from(Span::styled(
                " INSERT ",
                Style::default().fg(Color::Green).bold(),
            )),
        };
        frame.render_widget(Paragraph::new(vec![line]), area);
        return;
    }

    // Build the cmdline as a vector of spans so each piece gets its own style.
    let mut spans: Vec<Span> = vec![];
    let mut right_text = String::new();

    match state.mode {
        AppMode::Home => {
            spans.push(Span::styled(
                "NORMAL",
                Style::default().fg(Color::Magenta).bold(),
            ));
        }
        AppMode::Dashboard => {
            if let Some(tab) = state.active_tab() {
                let active_id = tab.tree.active_pane;
                let active = tab.tree.panes.get(&active_id);

                let (mode_label, mode_color) = if let Some(pane) = active {
                    match pane.mode {
                        TableMode::Normal => ("NORMAL", Color::Magenta),
                        TableMode::VisualRow | TableMode::VisualColumn => ("VISUAL", Color::Yellow),
                        TableMode::Insert => ("INSERT", Color::Green),
                    }
                } else {
                    ("NORMAL", Color::Magenta)
                };

                spans.push(Span::styled(
                    mode_label,
                    Style::default().fg(mode_color).bold(),
                ));

                // Tmux-style tab strip: <id>:<pane_type>[*]
                if !state.tabs.is_empty() {
                    spans.push(Span::styled(" ", Style::default()));
                    for (i, tab) in state.tabs.iter().enumerate() {
                        let active_pane = tab.tree.panes.get(&tab.tree.active_pane);
                        let type_name = active_pane.map_or("list", |p| match p.kind {
                            crate::tui::state::PaneType::TableList => "list",
                            crate::tui::state::PaneType::TableView => "table",
                            crate::tui::state::PaneType::SchemaView => "schema",
                            crate::tui::state::PaneType::QueryEditor => "query",
                            crate::tui::state::PaneType::QueryResults => "results",
                        });
                        let is_active = i == state.active_tab;
                        let style = if is_active {
                            Style::default().fg(Color::White).bold()
                        } else {
                            Style::default().fg(Color::DarkGray)
                        };
                        let suffix = if is_active { "*" } else { "" };
                        spans.push(Span::styled(format!("{i}:{type_name}{suffix}"), style));
                        spans.push(Span::styled(" ", Style::default()));
                    }
                }

                if let Some(pane) = active {
                    // Show search match indicator when a committed search is active.
                    if let Some(ref search) = pane.last_search {
                        if !search.matches.is_empty() {
                            right_text =
                                format!("[{}/{}]", search.current_idx + 1, search.matches.len());
                        }
                    }

                    // Row/Col position only for TableView and QueryResults.
                    if pane.kind == crate::tui::state::PaneType::TableView
                        || pane.kind == crate::tui::state::PaneType::QueryResults
                    {
                        let (headers, rows) = match pane.kind {
                            crate::tui::state::PaneType::TableView => {
                                if let Some(ref name) = pane.bound_table {
                                    if let Some(ref loaded) = state.table_cache.get(name) {
                                        (loaded.headers.len(), loaded.rows.len())
                                    } else {
                                        (0, 0)
                                    }
                                } else {
                                    (0, 0)
                                }
                            }
                            crate::tui::state::PaneType::QueryResults => {
                                if let Some(idx) = pane.bound_query_idx {
                                    if let Some(ref qr) = tab.query_results.get(idx) {
                                        (qr.headers.len(), qr.rows.len())
                                    } else {
                                        (0, 0)
                                    }
                                } else {
                                    (0, 0)
                                }
                            }
                            _ => (0, 0),
                        };
                        if headers > 0 {
                            let pos_text = format!(
                                "Row {}/{}, Col {}/{}",
                                pane.row_cursor + 1,
                                rows,
                                pane.cursor_col + 1,
                                headers
                            );
                            if right_text.is_empty() {
                                right_text = pos_text;
                            } else {
                                right_text = format!("{}  {}", right_text, pos_text);
                            }
                        }
                    }
                }
            } else {
                spans.push(Span::styled(
                    "NORMAL",
                    Style::default().fg(Color::Blue).bold(),
                ));
            }
        }
    }

    // Assemble the right-side text (search/position info + connection name).
    let conn_text = state.connection.as_ref().map(|c| format!("  {}", c.name));
    let right_width =
        right_text.chars().count() + conn_text.as_ref().map_or(0, |s| s.chars().count());

    // Pad to push right-side content to the right edge.
    let left_width: usize = spans.iter().map(|s| s.width()).sum();
    let gap = (area.width as usize)
        .saturating_sub(left_width)
        .saturating_sub(right_width);
    if gap > 0 {
        spans.push(Span::styled(" ".repeat(gap), Style::default()));
    }

    if !right_text.is_empty() {
        spans.push(Span::styled(
            right_text,
            Style::default().fg(Color::DarkGray),
        ));
    }
    if let Some(text) = conn_text {
        spans.push(Span::styled(text, Style::default().fg(Color::Green)));
    }

    let line = Line::from(spans);
    frame.render_widget(Paragraph::new(vec![line]), area);
}
