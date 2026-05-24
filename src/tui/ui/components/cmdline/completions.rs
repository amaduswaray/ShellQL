use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Paragraph},
};

pub fn render(
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
                    bg.fg(Color::White).add_modifier(if selected {
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
