//! Dashboard — recursive pane-based layout.
pub mod panes;
pub mod sql_highlight;

use ratatui::{Frame, layout::Rect, widgets::Paragraph};

use crate::tui::{AppState, ui::home::overlays::render_overlay};

use self::panes::render_pane;

// ── Entry point ───────────────────────────────────────────────────────────────

pub fn render_dashboard(frame: &mut Frame, area: Rect, state: &mut AppState) {
    // ── Phase 1: mutate active tab (layout, scroll, fullscreen area) ─────────
    let fs_id: Option<crate::tui::state::pane_layout::PaneId>;
    let leaves: Vec<crate::tui::state::pane_layout::PaneId>;
    {
        let Some(tab) = state.active_tab_mut() else {
            frame.render_widget(Paragraph::new("No active connection."), area);
            return;
        };

        fs_id = tab.tree.fullscreen_pane;
        if let Some(id) = fs_id {
            if let Some(pane) = tab.tree.panes.get_mut(&id) {
                pane.area = Some(area);
            }
            sync_pane_scroll(tab, area);
            leaves = vec![];
        } else {
            tab.tree.compute_areas(area);
            sync_pane_scroll(tab, area);
            leaves = tab.tree.collect_leaves();
        }
    }

    // ── Phase 2: render with immutable borrow ────────────────────────────────
    if let Some(id) = fs_id {
        render_pane(frame, id, state, true);
    } else {
        for pane_id in leaves {
            let is_active = state
                .active_tab()
                .map(|t| t.tree.active_pane == pane_id)
                .unwrap_or(false);
            render_pane(frame, pane_id, state, is_active);
        }
    }

    // Render overlay on top of dashboard (help, connection picker, etc.).
    if state.overlay.is_some() {
        render_overlay(frame, area, state);
    }
}

// ── Scroll sync ───────────────────────────────────────────────────────────────

fn sync_pane_scroll(tab: &mut crate::tui::state::Tab, _area: Rect) {
    let active_id = tab.tree.active_pane;

    for (id, pane) in &mut tab.tree.panes {
        let Some(pane_area) = pane.area else { continue };
        let _is_active = *id == active_id;

        match pane.kind {
            crate::tui::state::PaneType::TableList => {
                let inner_h = pane_area.height.saturating_sub(2).max(1) as usize;
                pane.sync_nav_offset(inner_h);
            }
            crate::tui::state::PaneType::SchemaPicker => {
                let inner_h = pane_area.height.saturating_sub(2).max(1) as usize;
                pane.sync_nav_offset(inner_h);
            }
            crate::tui::state::PaneType::TableView | crate::tui::state::PaneType::QueryResults => {
                let viewport = pane_area.height.saturating_sub(2).saturating_sub(3).max(1) as usize;
                pane.sync_row_offset(viewport);
                let col_viewport = (pane_area.width / 10).max(1) as usize;
                pane.sync_col_offset(col_viewport);
            }
            crate::tui::state::PaneType::SchemaView => {
                let card_h = 3usize;
                let viewport = pane_area.height.saturating_sub(2) as usize / card_h.max(1);
                pane.sync_nav_offset(viewport);
            }
            crate::tui::state::PaneType::QueryEditor => {
                // borders (2) + padding (2) = 4
                let inner_h = pane_area.height.saturating_sub(4).max(1) as usize;
                pane.sync_query_row_offset(inner_h);

                let pad = 1u16;
                let inner_w = pane_area.width.saturating_sub(2);
                let padded_w = inner_w.saturating_sub(pad * 2);
                let gutter_w = (pane.query_text.len().to_string().len().max(3) + 1) as u16;
                let text_w = padded_w.saturating_sub(gutter_w).max(1) as usize;

                let (row, col) = pane.query_cursor;
                if let Some(line) = pane.query_text.get(row) {
                    let cursor_vx = sql_highlight::cursor_visual_x(line, col);
                    if cursor_vx < pane.query_scroll_offset {
                        pane.query_scroll_offset = cursor_vx;
                    } else if cursor_vx >= pane.query_scroll_offset + text_w {
                        pane.query_scroll_offset = cursor_vx + 1 - text_w;
                    }
                }
            }
        }
    }
}
