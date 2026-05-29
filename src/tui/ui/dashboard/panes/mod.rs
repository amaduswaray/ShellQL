//! Individual pane renderers — TableList, TableView, SchemaView, QueryEditor.

use crate::tui::state::pane_layout::{Pane, PaneId, PaneType};
use ratatui::{
    Frame,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders},
};

pub mod query_editor;
pub mod query_results;
pub mod schema_view;
pub mod table_list;
pub mod table_view;

// ── Constants ─────────────────────────────────────────────────────────────────

const NUM_SPACES_BETWEEN_COLUMNS: u16 = 3;
const ROW_NUMBER_PADDING: u16 = 2;
const MAX_COLUMN_WIDTH_FRACTION: f32 = 0.3;
const EDGE_PADDING: u16 = 2;

// ── Dispatcher ────────────────────────────────────────────────────────────────

pub fn render_pane(
    frame: &mut Frame,
    pane_id: PaneId,
    state: &crate::tui::state::app::AppState,
    focused: bool,
) {
    let Some(tab) = state.active_tab() else {
        return;
    };
    let Some(pane) = tab.tree.panes.get(&pane_id) else {
        return;
    };
    let Some(area) = pane.area else { return };

    match pane.kind {
        PaneType::TableList => table_list::render(frame, area, pane, state, focused),
        PaneType::TableView => table_view::render(frame, area, pane, state, focused),
        PaneType::SchemaView => schema_view::render(frame, area, pane, state, focused),
        PaneType::QueryEditor => query_editor::render(frame, area, pane, focused),
        PaneType::QueryResults => query_results::render(frame, area, pane, state, focused),
    }
}

// ── Border title helpers ──────────────────────────────────────────────────────

fn make_title(pane: &Pane) -> String {
    match pane.kind {
        PaneType::TableList => format!(" {} ", pane.display_id),
        PaneType::TableView => {
            if let Some(ref table) = pane.bound_table {
                let dirty = !pane.pending_updates.is_empty()
                    || !pane.pending_deletes.is_empty()
                    || !pane.pending_inserts.is_empty();
                let filtered = pane.filter.is_some();
                let sorted = pane.sort_col.is_some();
                let mut tags = String::new();
                if filtered {
                    tags.push_str(" [filtered]");
                }
                if sorted {
                    tags.push_str(" [sorted]");
                }
                if dirty {
                    tags.push('*');
                }
                format!(" {}: {}{} ", pane.display_id, table, tags)
            } else {
                format!(" {} ", pane.display_id)
            }
        }
        PaneType::SchemaView => {
            if let Some(ref table) = pane.bound_table {
                format!(" {}: Schema({}) ", pane.display_id, table)
            } else {
                format!(" {}: Schema ", pane.display_id)
            }
        }
        PaneType::QueryEditor => {
            format!(" {}: Query ", pane.display_id)
        }
        PaneType::QueryResults => {
            if let Some(idx) = pane.bound_query_idx {
                format!(
                    " {}: Result {}/{} ",
                    pane.display_id,
                    idx + 1,
                    pane.query_result_count.max(1)
                )
            } else {
                format!(" {}: Result ", pane.display_id)
            }
        }
    }
}

fn pane_block(title: &str, focused: bool) -> Block<'_> {
    let border_style = if focused {
        Style::default().fg(Color::Blue)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let title_style = if focused {
        Style::default().fg(Color::Blue).bold()
    } else {
        Style::default().fg(Color::DarkGray)
    };

    Block::default()
        .title(Line::from(title).style(title_style))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(border_style)
}

// ── Match highlighting helpers ────────────────────────────────────────────────

/// Highlight the first substring match of `query` in `text` with bold+yellow.
/// Characters before and after the match keep `base` style.
pub fn search_highlight_spans<'a>(text: &'a str, query: &str, base: Style) -> Vec<Span<'a>> {
    if query.is_empty() {
        return vec![Span::styled(text, base)];
    }
    let lower_text: Vec<char> = text.to_lowercase().chars().collect();
    let lower_query: Vec<char> = query.to_lowercase().chars().collect();

    if lower_query.len() > lower_text.len() {
        return vec![Span::styled(text, base)];
    }

    if let Some(start_char) = lower_text
        .windows(lower_query.len())
        .position(|w| w == lower_query.as_slice())
    {
        let chars: Vec<char> = text.chars().collect();
        let start_byte: usize = chars[..start_char].iter().map(|c| c.len_utf8()).sum();
        let end_byte: usize = chars[..start_char + lower_query.len()]
            .iter()
            .map(|c| c.len_utf8())
            .sum();

        let mut spans = vec![];
        if start_byte > 0 {
            spans.push(Span::styled(&text[0..start_byte], base));
        }
        spans.push(Span::styled(
            &text[start_byte..end_byte],
            base.fg(Color::Yellow).bold(),
        ));
        if end_byte < text.len() {
            spans.push(Span::styled(&text[end_byte..], base));
        }
        spans
    } else {
        vec![Span::styled(text, base)]
    }
}
