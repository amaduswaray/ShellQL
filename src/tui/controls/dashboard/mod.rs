use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::tui::{AppState, SearchDirection};

pub mod editor;
pub mod helpers;
pub mod info;
pub mod modes;
pub mod navigation;
pub mod search;

pub fn handle_dashboard(event: KeyEvent, state: &mut AppState) {
    // ── Tab switching (Shift+H / Shift+L) ────────────────────────────────────
    match event.code {
        KeyCode::Char('H') => {
            if state.tabs.len() > 1 {
                state.active_tab = (state.active_tab + state.tabs.len() - 1) % state.tabs.len();
            }
            return;
        }
        KeyCode::Char('L') => {
            if state.tabs.len() > 1 {
                state.active_tab = (state.active_tab + 1) % state.tabs.len();
            }
            return;
        }
        _ => {}
    }

    let tables = state.tables.clone();

    // Any keypress dismisses transient cmdline messages when idle.
    if !state.cmdline.is_active() {
        state.cmdline.loading = None;
        state.cmdline.error = None;
    }

    // ── QueryEditor vim mode (Normal/Insert) ─────────────────────────────────
    if editor::handle_query_editor(event, state, &tables) {
        return;
    }

    // Ctrl+hjkl / Ctrl+Arrows — pane navigation
    if event.modifiers.contains(KeyModifiers::CONTROL) {
        if navigation::handle_ctrl(event, state, &tables) {
            return;
        }
    }

    // Handle pending 'g' for 'gg' sequence first.
    if let Some('g') = state.pending_key {
        state.pending_key = None;
        if event.code == KeyCode::Char('g') {
            navigation::go_top(state);
            return;
        }
    }

    // Handle pending 'd' for 'dd' sequence first.
    if let Some('d') = state.pending_key {
        state.pending_key = None;
        if event.code == KeyCode::Char('d') {
            modes::handle_dd(state);
            return;
        }
    }

    match event.code {
        // ── Command line ───────────────────────────────────────────────────────
        KeyCode::Char(':') => {
            state.cmdline.open_input();
            state.pending_key = None;
            return;
        }
        // Fallback for terminals that report Shift+; as ';' with SHIFT modifier
        KeyCode::Char(';') if event.modifiers.contains(KeyModifiers::SHIFT) => {
            state.cmdline.open_input();
            state.pending_key = None;
            return;
        }

        // ── Search ─────────────────────────────────────────────────────────────
        KeyCode::Char('/') => {
            state.cmdline.open_search(SearchDirection::Forward);
            state.pending_key = None;
            return;
        }
        KeyCode::Char('?') => {
            state.cmdline.open_search(SearchDirection::Backward);
            state.pending_key = None;
            return;
        }
        KeyCode::Char('n') => search::next(state),
        KeyCode::Char('N') => search::prev(state),

        // ── Cell hover (Shift+K) ───────────────────────────────────────────────
        KeyCode::Char('K') => info::show_cell_hover(state),

        // ── Mode switching ─────────────────────────────────────────────────────
        KeyCode::Char('v') if event.modifiers.contains(KeyModifiers::CONTROL) => {
            modes::start_visual_column(state);
        }
        KeyCode::Char('v') => {
            modes::start_visual_row(state);
        }
        KeyCode::Char('V') => {
            modes::start_visual_row(state);
        }
        KeyCode::Char('i') => {
            modes::start_insert_or_cell_edit(state);
        }
        KeyCode::Tab => {
            modes::cycle_query_results(state);
        }
        KeyCode::Char('u') => {
            modes::undo_change(state);
        }
        KeyCode::Char('d') => {
            modes::handle_delete(state);
        }
        KeyCode::Esc => {
            modes::escape(state);
        }

        // ── Navigation ─────────────────────────────────────────────────────────
        KeyCode::Char('j') | KeyCode::Down => {
            navigation::down(state, &tables);
        }
        KeyCode::Char('k') | KeyCode::Up => {
            navigation::up(state);
        }
        KeyCode::Char('h') | KeyCode::Left => {
            navigation::left(state);
        }
        KeyCode::Char('l') | KeyCode::Right => {
            navigation::right(state);
        }

        // ── Jump to bottom / top ───────────────────────────────────────────────
        KeyCode::Char('G') => {
            navigation::bottom(state, &tables);
        }
        KeyCode::Char('g') => {
            state.pending_key = Some('g');
        }

        // ── Back / forward history ──────────────────────────────────────────────
        KeyCode::Char('-') => {
            navigation::history_back(state);
        }
        KeyCode::Char('_') => {
            navigation::history_forward(state);
        }

        // ── Enter — select table or load into current pane ─────────────────────
        KeyCode::Enter => {
            navigation::enter(state, &tables);
        }

        _ => {}
    }
}
