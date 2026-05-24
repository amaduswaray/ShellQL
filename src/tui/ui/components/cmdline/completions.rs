use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, BorderType, Borders, Clear, Paragraph, Scrollbar, ScrollbarOrientation,
        ScrollbarState,
    },
};

const MAX_VISIBLE: usize = 10;

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
    let total = completions.len();
    let visible_count = total.min(MAX_VISIBLE);
    let needs_scrollbar = total > visible_count;

    let popup_w = (inner_w + 2) as u16; // +2 for left/right borders
    let popup_h = (visible_count + 2) as u16; // +2 for top/bottom borders

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

    // Compute scroll offset so selected item is always visible.
    let mut offset = 0usize;
    if let Some(sel) = selected {
        if sel >= offset + visible_count {
            offset = sel + 1 - visible_count;
        }
        if sel < offset {
            offset = sel;
        }
    }

    let lines: Vec<Line> = completions
        .iter()
        .enumerate()
        .skip(offset)
        .take(visible_count)
        .map(|(i, (cmd, desc))| {
            let is_selected = Some(i) == selected;
            let bg = if is_selected {
                Style::default().bg(Color::DarkGray)
            } else {
                Style::default()
            };
            Line::from(vec![
                Span::styled(
                    format!(" {cmd:<cmd_col_w$}  "),
                    bg.fg(Color::White).add_modifier(if is_selected {
                        Modifier::BOLD
                    } else {
                        Modifier::empty()
                    }),
                ),
                Span::styled(
                    format!("{desc:<desc_col_w$}"),
                    bg.fg(if is_selected {
                        Color::White
                    } else {
                        Color::DarkGray
                    }),
                ),
            ])
        })
        .collect();

    frame.render_widget(Paragraph::new(lines), inner);

    // ── Scrollbar (embedded in the right border) ───────────────────────────
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
