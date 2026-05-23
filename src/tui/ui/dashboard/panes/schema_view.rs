use super::{make_title, pane_block};
use crate::tui::state::pane_layout::Pane;
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::Paragraph,
};

const SCHEMA_CARD_HEIGHT: usize = 3; // 2 content lines + 1 blank gap
const SCHEMA_PAD: usize = 2; // left / right padding inside the pane
const SCHEMA_SEL_BG: Color = Color::Rgb(35, 38, 55);

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

    let Some(ref table_name) = pane.bound_table else {
        frame.render_widget(
            Paragraph::new(Span::styled(" —", Style::default().fg(Color::DarkGray))),
            inner,
        );
        return;
    };

    let Some(ref loaded) = state.table_cache.get(table_name) else {
        frame.render_widget(
            Paragraph::new(Span::styled(
                " Loading…",
                Style::default().fg(Color::DarkGray),
            )),
            inner,
        );
        return;
    };

    let schema = &loaded.schema;
    if schema.is_empty() {
        frame.render_widget(
            Paragraph::new(Span::styled(
                " No columns.",
                Style::default().fg(Color::DarkGray),
            )),
            inner,
        );
        return;
    }

    let cursor = pane.nav_cursor;
    let offset = pane.nav_offset;
    let viewport = (inner.height as usize / SCHEMA_CARD_HEIGHT).max(1);
    let end = (offset + viewport).min(schema.len());
    let visible = &schema[offset..end];

    let mut lines: Vec<Line> = Vec::new();
    let usable_w = inner.width.saturating_sub(2 * SCHEMA_PAD as u16) as usize;

    for (i, col) in visible.iter().enumerate() {
        let idx = offset + i;
        let sel = idx == cursor;

        // ── Line 1: column name (left) + data type (right) ──
        let name_style = if sel {
            Style::default().fg(Color::White).bold().bg(SCHEMA_SEL_BG)
        } else {
            Style::default().fg(Color::White).bold()
        };
        let type_style = if sel {
            Style::default().fg(Color::DarkGray).bg(SCHEMA_SEL_BG)
        } else {
            Style::default().fg(Color::DarkGray)
        };

        let name_text = col.name.clone();
        let type_text = col.data_type.clone();
        let name_w = name_text.chars().count();
        let type_w = type_text.chars().count();
        let gap = usable_w.saturating_sub(name_w + type_w);

        lines.push(Line::from(vec![
            Span::styled(
                " ".repeat(SCHEMA_PAD),
                Style::default().bg(if sel { SCHEMA_SEL_BG } else { Color::Reset }),
            ),
            Span::styled(name_text, name_style),
            Span::styled(
                " ".repeat(gap),
                Style::default().bg(if sel { SCHEMA_SEL_BG } else { Color::Reset }),
            ),
            Span::styled(type_text, type_style),
            Span::styled(
                " ".repeat(SCHEMA_PAD),
                Style::default().bg(if sel { SCHEMA_SEL_BG } else { Color::Reset }),
            ),
        ]));

        // ── Line 2: constraint badges ──
        let mut badge_spans: Vec<Span> = vec![];

        if col.is_primary_key {
            badge_spans.push(Span::styled(
                "PK",
                Style::default().fg(Color::Yellow).bg(if sel {
                    SCHEMA_SEL_BG
                } else {
                    Color::Reset
                }),
            ));
        }
        if !col.nullable {
            badge_spans.push(Span::styled(
                " NOT NULL",
                Style::default()
                    .fg(Color::Red)
                    .bg(if sel { SCHEMA_SEL_BG } else { Color::Reset }),
            ));
        }
        if let Some(ref dflt) = col.default_value {
            let display = if dflt.len() > 24 {
                format!("{}…", &dflt[..23])
            } else {
                dflt.clone()
            };
            badge_spans.push(Span::styled(
                format!(" DEFAULT {display}"),
                Style::default().fg(Color::Green).bg(if sel {
                    SCHEMA_SEL_BG
                } else {
                    Color::Reset
                }),
            ));
        }

        if badge_spans.is_empty() {
            badge_spans.push(Span::styled(
                "nullable",
                Style::default().fg(Color::DarkGray).bg(if sel {
                    SCHEMA_SEL_BG
                } else {
                    Color::Reset
                }),
            ));
        }

        // Pad badge line to full usable width so background colour fills the row.
        let badge_text_w: usize = badge_spans.iter().map(|s| s.content.chars().count()).sum();
        let pad = usable_w.saturating_sub(badge_text_w);
        if pad > 0 {
            badge_spans.push(Span::styled(
                " ".repeat(pad),
                Style::default().bg(if sel { SCHEMA_SEL_BG } else { Color::Reset }),
            ));
        }

        let mut badge_line = vec![Span::styled(
            " ".repeat(SCHEMA_PAD),
            Style::default().bg(if sel { SCHEMA_SEL_BG } else { Color::Reset }),
        )];
        badge_line.extend(badge_spans);
        badge_line.push(Span::styled(
            " ".repeat(SCHEMA_PAD),
            Style::default().bg(if sel { SCHEMA_SEL_BG } else { Color::Reset }),
        ));
        lines.push(Line::from(badge_line));

        // ── Line 3: blank gap between cards ──
        // Fill the entire width (padding + usable + padding) with spaces so the
        // background color forms a complete rectangle for the selected card.
        lines.push(Line::from(Span::styled(
            " ".repeat(inner.width as usize),
            Style::default().bg(if sel { SCHEMA_SEL_BG } else { Color::Reset }),
        )));
    }

    frame.render_widget(Paragraph::new(lines), inner);
}
