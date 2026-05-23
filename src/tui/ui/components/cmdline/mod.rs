use ratatui::{Frame, layout::Rect};

use crate::tui::state::{AppState, CommandLineMode};

pub mod cell_edit;
pub mod completions;
pub mod confirm;
pub mod idle;
pub mod input;
pub mod search;

// ── Entry point ───────────────────────────────────────────────────────────────

pub fn render_cmdline(frame: &mut Frame, area: Rect, state: &AppState) {
    match &state.cmdline.mode {
        CommandLineMode::Idle => idle::render(frame, area, state),
        CommandLineMode::Input => input::render(frame, area, state),
        CommandLineMode::Search(direction) => search::render(frame, area, state, *direction),
        CommandLineMode::CellEdit { .. } => cell_edit::render(frame, area, state),
        CommandLineMode::Confirm(action) => {
            confirm::render(frame, area, action, &state.cmdline.input)
        }
    }
}
