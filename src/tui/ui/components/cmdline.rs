use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Paragraph},
};

use crate::tui::state::{AppMode, AppState, CommandLineMode, ConfirmAction, TableMode, TextMode};

// ── Entry point ───────────────────────────────────────────────────────────────

pub fn render_cmdline(frame: &mut Frame, area: Rect, state: &AppState) {
    match &state.cmdline.mode {
        CommandLineMode::Idle => render_idle(frame, area, state),
        CommandLineMode::Input => render_input(frame, area, state),
        CommandLineMode::Confirm(action) => {
            render_confirm(frame, area, action, &state.cmdline.input)
        }
    }
}

// ── Idle — status strip ───────────────────────────────────────────────────────

fn render_idle(frame: &mut Frame, area: Rect, state: &AppState) {
    if let Some(ref loading) = state.cmdline.loading {
        let line = Line::from(Span::styled(
            loading.as_str(),
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
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
            spans.push(Span::styled("NORMAL", Style::default().fg(Color::Blue).bold()));
        }
        AppMode::Dashboard => {
            if let Some(ref dash) = state.dashboard {
                let active_id = dash.tree.active_pane;
                let active = dash.tree.panes.get(&active_id);

                let (mode_label, mode_color) = if let Some(pane) = active {
                    match pane.mode {
                        TableMode::Normal => ("NORMAL", Color::Blue),
                        TableMode::VisualRow | TableMode::VisualColumn => ("VISUAL", Color::Yellow),
                        TableMode::Insert => ("INSERT", Color::Green),
                    }
                } else {
                    ("NORMAL", Color::Blue)
                };

                spans.push(Span::styled(mode_label, Style::default().fg(mode_color).bold()));
                spans.push(Span::styled(" ", Style::default()));
                spans.push(Span::styled(dash.connection.name.as_str(), Style::default().fg(Color::Blue).bold()));

                if let Some(pane) = active {
                    if let Some(ref table_name) = pane.bound_table {
                        spans.push(Span::styled(" ", Style::default()));
                        spans.push(Span::styled(table_name.as_str(), Style::default().fg(Color::DarkGray)));

                        if let Some(ref loaded) = dash.table_cache.get(table_name) {
                            let total_rows = loaded.rows.len();
                            let total_cols = loaded.headers.len();
                            let cur_row = pane.row_cursor + 1;
                            let cur_col = pane.cursor_col + 1;
                            right_text = format!("Row {}/{}, Col {}/{}", cur_row, total_rows, cur_col, total_cols);
                        }
                    }
                }
            } else {
                spans.push(Span::styled("NORMAL", Style::default().fg(Color::Blue).bold()));
            }
        }
    }

    // Pad the right_text to push it to the right edge.
    let left_width: usize = spans.iter().map(|s| s.width()).sum();
    let right_width = right_text.chars().count();
    let gap = area.width as usize - left_width - right_width;
    if gap > 0 {
        spans.push(Span::styled(" ".repeat(gap), Style::default()));
    }
    spans.push(Span::styled(right_text, Style::default().fg(Color::DarkGray)));

    let line = Line::from(spans);
    frame.render_widget(Paragraph::new(vec![line]), area);
}

// ── Input — `:` prompt ────────────────────────────────────────────────────────

fn render_input(frame: &mut Frame, area: Rect, state: &AppState) {
    let input = &state.cmdline.input;

    let line = Line::from(vec![
        Span::styled(":", Style::default().fg(Color::White).bold()),
        Span::styled(input.clone(), Style::default().fg(Color::White)),
    ]);

    frame.render_widget(Paragraph::new(vec![line]), area);

    // Terminal cursor sits just after the last typed character.
    let cursor_x = (area.x + 1 + input.len() as u16).min(area.right().saturating_sub(1));
    frame.set_cursor_position((cursor_x, area.y));

    // Completion popup floats above the bar when candidates are available.
    if !state.cmdline.completions.is_empty() {
        render_completions(
            frame,
            area,
            &state.cmdline.completions,
            state.cmdline.completion_selected,
        );
    }
}

// ── Confirm — inline y/n prompt ───────────────────────────────────────────────

fn render_confirm(frame: &mut Frame, area: Rect, action: &ConfirmAction, input: &str) {
    match action {
        ConfirmAction::DeleteConnection(name) => render_confirm_delete(frame, area, name, input),
    }
}

fn render_confirm_delete(frame: &mut Frame, area: Rect, name: &str, input: &str) {
    let prefix_spans: Vec<Span> = vec![
        Span::styled("Delete ", Style::default().fg(Color::Red)),
        Span::styled(
            format!("\"{}\"", name),
            Style::default().fg(Color::Red).bold(),
        ),
        Span::styled("? ", Style::default().fg(Color::Red)),
        Span::styled("[y/n]: ", Style::default().fg(Color::DarkGray)),
    ];

    let prefix_width: u16 = prefix_spans.iter().map(|s| s.content.len() as u16).sum();

    let mut spans = prefix_spans;
    spans.push(Span::styled(
        input.to_string(),
        Style::default().fg(Color::White),
    ));

    frame.render_widget(Paragraph::new(vec![Line::from(spans)]), area);

    let cursor_x = (area.x + prefix_width + input.len() as u16).min(area.right().saturating_sub(1));
    frame.set_cursor_position((cursor_x, area.y));
}

// ── Completion popup ──────────────────────────────────────────────────────────

fn render_completions(
    frame: &mut Frame,
    cmdline_area: Rect,
    completions: &[(&'static str, &'static str)],
    selected: Option<usize>,
) {
    // Measure columns so everything lines up regardless of command length.
    let cmd_col_w = completions.iter().map(|(c, _)| c.len()).max().unwrap_or(4);
    let desc_col_w = completions.iter().map(|(_, d)| d.len()).max().unwrap_or(8);

    // inner: 1 pad + cmd + 2 gap + desc + 1 pad
    let inner_w = 1 + cmd_col_w + 2 + desc_col_w + 1;
    let popup_w = (inner_w + 2) as u16; // +2 for left/right borders
    let popup_h = completions.len() as u16 + 2; // +2 for top/bottom borders

    // Anchor: flush with the `:` character, growing upward.
    let popup = Rect {
        x: cmdline_area.x,
        y: cmdline_area.y.saturating_sub(popup_h),
        width: popup_w.min(cmdline_area.width),
        height: popup_h,
    };

    frame.render_widget(Clear, popup);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::White))
        .style(Style::default().bg(Color::Reset));

    let inner = block.inner(popup);
    frame.render_widget(block, popup);

    let lines: Vec<Line> = completions
        .iter()
        .enumerate()
        .map(|(i, (cmd, desc))| {
            let selected = Some(i) == selected;
            let bg = if selected {
                Style::default().bg(Color::DarkGray) //rgb(38, 35, 58)
            } else {
                Style::default()
            };
            Line::from(vec![
                Span::styled(
                    format!(" {cmd:<cmd_col_w$}  "),
                    bg.fg(if selected { Color::White } else { Color::White })
                        .add_modifier(if selected {
                            Modifier::BOLD
                        } else {
                            Modifier::empty()
                        }),
                ),
                Span::styled(
                    format!("{desc:<desc_col_w$}"),
                    bg.fg(if selected {
                        Color::White
                    } else {
                        Color::DarkGray
                    }),
                ),
            ])
        })
        .collect();

    frame.render_widget(Paragraph::new(lines), inner);
}
