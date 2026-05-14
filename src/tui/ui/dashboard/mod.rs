//! Dashboard — recursive pane-based layout.
pub mod panes;
pub mod sql_highlight;

use ratatui::{
    Frame,
    layout::Rect,
    widgets::Paragraph,
};

use crate::tui::{
    AppState,
    state::dashboard::DashboardState,
    ui::home::overlays::render_overlay,
};

use self::panes::render_pane;

// ── Entry point ───────────────────────────────────────────────────────────────

pub fn render_dashboard(frame: &mut Frame, area: Rect, state: &mut AppState) {
    let Some(ref mut dash) = state.dashboard else {
        frame.render_widget(
            Paragraph::new("No active connection."),
            area,
        );
        return;
    };

    // Compute pane areas from the layout tree.
    dash.tree.compute_areas(area);

    // Sync scroll offsets inside each pane.
    sync_pane_scroll(dash, area);

    // Collect all leaves and render each pane.
    let leaves = dash.tree.collect_leaves();
    for pane_id in leaves {
        let is_active = dash.tree.active_pane == pane_id;
        render_pane(frame, pane_id, dash, is_active);
    }

    // Render overlay on top of dashboard (help, connection picker, etc.).
    if state.overlay.is_some() {
        render_overlay(frame, area, state);
    }
}

// ── Scroll sync ───────────────────────────────────────────────────────────────

fn sync_pane_scroll(dash: &mut DashboardState, _area: Rect) {
    let active_id = dash.tree.active_pane;

    for (id, pane) in &mut dash.tree.panes {
        let Some(pane_area) = pane.area else { continue };
        let _is_active = *id == active_id;

        match pane.kind {
            crate::tui::state::PaneType::TableList => {
                let inner_h = pane_area.height.saturating_sub(2 + 3).max(1) as usize;
                pane.sync_nav_offset(inner_h);
            }
            crate::tui::state::PaneType::TableView | crate::tui::state::PaneType::QueryResults => {
                // Header takes 3 lines (top border pad + header text + header line),
                // so data viewport is inner height minus 3.
                let viewport = pane_area.height.saturating_sub(2).saturating_sub(3).max(1) as usize;
                pane.sync_row_offset(viewport);
                let col_viewport = (pane_area.width / 10).max(1) as usize;
                pane.sync_col_offset(col_viewport);
            }
            crate::tui::state::PaneType::SchemaView => {
                // Each card = 2 content lines + 1 separator = 3 lines.
                let card_h = 3usize;
                let viewport = pane_area.height.saturating_sub(2) as usize / card_h.max(1);
                pane.sync_nav_offset(viewport);
            }
            _ => {}
        }
    }
}
