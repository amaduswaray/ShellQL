use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::helpers::{char_idx_to_byte_idx, get_table_prefix};
use crate::tui::{
    AppState,
    state::{
        TableMode,
        pane_layout::{Pane, PaneType, QueryEditorSnapshot},
    },
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct QueryCursor {
    row: usize,
    col: usize,
}

impl From<(usize, usize)> for QueryCursor {
    fn from((row, col): (usize, usize)) -> Self {
        Self { row, col }
    }
}

impl From<QueryCursor> for (usize, usize) {
    fn from(value: QueryCursor) -> Self {
        (value.row, value.col)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct QueryVisualSelection {
    start: QueryCursor,
    end: QueryCursor,
    linewise: bool,
}

pub fn handle_query_editor(event: KeyEvent, state: &mut AppState, tables: &[String]) -> bool {
    let active_idx = state.active_tab;
    let is_query_editor = state
        .tabs
        .get(active_idx)
        .and_then(|tab| tab.tree.panes.get(&tab.tree.active_pane))
        .is_some_and(|pane| pane.kind == PaneType::QueryEditor);

    if !is_query_editor {
        return false;
    }

    // Query editor keeps its own pending keys (`gg`, operators, find, ...),
    // so dashboard-level pending state should not leak while this pane is focused.
    state.pending_key = None;

    let Some(tab) = state.tabs.get_mut(active_idx) else {
        return false;
    };
    let active_id = tab.tree.active_pane;
    let Some(pane) = tab.tree.panes.get_mut(&active_id) else {
        return false;
    };

    ensure_query_buffer(pane);

    if pane.mode == TableMode::Insert {
        handle_insert_mode(event, pane, tables)
    } else if is_visual_active(pane) {
        handle_visual_mode(event, pane)
    } else {
        if pane.mode != TableMode::Normal {
            pane.mode = TableMode::Normal;
        }
        handle_normal_mode(event, pane)
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Normal mode
// ═══════════════════════════════════════════════════════════════════════════════

fn handle_normal_mode(event: KeyEvent, pane: &mut Pane) -> bool {
    if handle_pending_normal_combo(event, pane) {
        clamp_cursor_for_mode(pane, false);
        return true;
    }

    if handle_count_prefix(event, pane) {
        return true;
    }

    if event.modifiers.contains(KeyModifiers::CONTROL) {
        if event.code == KeyCode::Char('r') {
            let count = take_count(pane);
            for _ in 0..count {
                pane.query_redo();
            }
            clamp_cursor_for_mode(pane, false);
            return true;
        }
        // Let dashboard-level Ctrl mappings handle pane navigation.
        return false;
    }

    let handled = match event.code {
        KeyCode::Esc => {
            clear_query_pending_state(pane);
            close_autocomplete(pane);
            true
        }

        // ── Motions ─────────────────────────────────────────────────────────
        KeyCode::Char('h') | KeyCode::Left => {
            let count = take_count(pane);
            move_left(pane, count);
            true
        }
        KeyCode::Char('l') | KeyCode::Right => {
            let count = take_count(pane);
            move_right(pane, count, false);
            true
        }
        KeyCode::Char('j') | KeyCode::Down => {
            let count = take_count(pane);
            move_down(pane, count, false);
            true
        }
        KeyCode::Char('k') | KeyCode::Up => {
            let count = take_count(pane);
            move_up(pane, count, false);
            true
        }
        KeyCode::Char('w') => {
            let count = take_count(pane);
            move_word_forward(pane, count);
            true
        }
        KeyCode::Char('b') => {
            let count = take_count(pane);
            move_word_back(pane, count);
            true
        }
        KeyCode::Char('e') => {
            let count = take_count(pane);
            move_word_end(pane, count);
            true
        }
        KeyCode::Char('0') | KeyCode::Home => {
            pane.query_pending_count = None;
            move_to_line_start(pane);
            true
        }
        KeyCode::Char('^') => {
            pane.query_pending_count = None;
            move_to_first_non_blank(pane);
            true
        }
        KeyCode::Char('$') | KeyCode::End => {
            pane.query_pending_count = None;
            move_to_line_end(pane, false);
            true
        }
        KeyCode::Char('G') => {
            if let Some(line) = pane.query_pending_count.take() {
                move_to_line_number(pane, line);
            } else {
                move_to_bottom(pane);
            }
            true
        }
        KeyCode::Char('g') => {
            pane.query_pending_key = Some('g');
            true
        }
        KeyCode::Char('%') => {
            pane.query_pending_count = None;
            move_to_matching_bracket(pane);
            true
        }

        // ── Find/till motions ───────────────────────────────────────────────
        KeyCode::Char('f') | KeyCode::Char('F') | KeyCode::Char('t') | KeyCode::Char('T') => {
            if let KeyCode::Char(c) = event.code {
                pane.query_pending_key = Some(c);
            }
            true
        }
        KeyCode::Char(';') => {
            let count = take_count(pane);
            repeat_last_find(pane, false, count);
            true
        }
        KeyCode::Char(',') => {
            let count = take_count(pane);
            repeat_last_find(pane, true, count);
            true
        }

        // ── Visual mode ─────────────────────────────────────────────────────
        KeyCode::Char('v') => {
            pane.query_pending_count = None;
            enter_visual_char_mode(pane);
            true
        }
        KeyCode::Char('V') => {
            pane.query_pending_count = None;
            enter_visual_line_mode(pane);
            true
        }

        // ── Editing ─────────────────────────────────────────────────────────
        KeyCode::Char('u') => {
            let count = take_count(pane);
            for _ in 0..count {
                pane.query_undo();
            }
            true
        }
        KeyCode::Char('x') | KeyCode::Delete => {
            let count = take_count(pane);
            run_edit(pane, |p| {
                let mut changed = false;
                for _ in 0..count {
                    changed |= delete_char_at_cursor(p, false, true);
                }
                changed
            });
            true
        }
        KeyCode::Char('J') => {
            let count = take_count(pane);
            run_edit(pane, |p| {
                let times = if count == 1 { 1 } else { count - 1 };
                let mut changed = false;
                for _ in 0..times {
                    changed |= join_with_next_line(p);
                }
                changed
            });
            true
        }
        KeyCode::Char('D') => {
            pane.query_pending_count = None;
            run_edit(pane, delete_to_line_end);
            true
        }
        KeyCode::Char('d') => {
            pane.query_pending_key = Some('d');
            true
        }
        KeyCode::Char('y') => {
            pane.query_pending_key = Some('y');
            true
        }
        KeyCode::Char('p') => {
            run_edit(pane, paste_after_cursor);
            true
        }
        KeyCode::Char('P') => {
            run_edit(pane, paste_before_cursor);
            true
        }
        KeyCode::Char('r') => {
            pane.query_pending_key = Some('r');
            true
        }

        // ── Change ──────────────────────────────────────────────────────────
        KeyCode::Char('S') => {
            pane.query_pending_count = None;
            run_edit(pane, change_current_line);
            enter_insert_mode(pane);
            true
        }
        KeyCode::Char('C') => {
            pane.query_pending_count = None;
            run_edit(pane, delete_to_line_end);
            enter_insert_mode(pane);
            true
        }
        KeyCode::Char('c') => {
            pane.query_pending_key = Some('c');
            true
        }
        KeyCode::Char('s') => {
            pane.query_pending_count = None;
            run_edit(pane, |p| delete_char_at_cursor(p, false, true));
            enter_insert_mode(pane);
            true
        }

        // ── Insert entry points ─────────────────────────────────────────────
        KeyCode::Char('i') => {
            enter_insert_mode(pane);
            true
        }
        KeyCode::Char('a') => {
            let count = take_count(pane);
            move_right(pane, count, true);
            enter_insert_mode(pane);
            true
        }
        KeyCode::Char('I') => {
            pane.query_pending_count = None;
            move_to_first_non_blank(pane);
            enter_insert_mode(pane);
            true
        }
        KeyCode::Char('A') => {
            pane.query_pending_count = None;
            move_to_line_end(pane, true);
            enter_insert_mode(pane);
            true
        }
        KeyCode::Char('o') => {
            pane.query_pending_count = None;
            run_edit(pane, open_line_below);
            enter_insert_mode(pane);
            true
        }
        KeyCode::Char('O') => {
            pane.query_pending_count = None;
            run_edit(pane, open_line_above);
            enter_insert_mode(pane);
            true
        }

        _ => false,
    };

    if handled {
        clamp_cursor_for_mode(pane, pane.mode == TableMode::Insert);
    }

    handled
}

fn handle_visual_mode(event: KeyEvent, pane: &mut Pane) -> bool {
    if handle_count_prefix(event, pane) {
        return true;
    }

    let mut handled = true;

    match event.code {
        KeyCode::Esc => {
            exit_visual_mode(pane);
        }
        KeyCode::Char('v') => {
            if pane.query_visual_line_mode {
                pane.query_visual_line_mode = false;
            } else {
                exit_visual_mode(pane);
            }
        }
        KeyCode::Char('V') => {
            pane.query_visual_line_mode = true;
            if pane.query_visual_anchor.is_none() {
                pane.query_visual_anchor = Some((pane.query_cursor.0, pane.query_cursor.1));
            }
        }

        KeyCode::Char('h') | KeyCode::Left => {
            let count = take_count(pane);
            move_left(pane, count);
        }
        KeyCode::Char('l') | KeyCode::Right => {
            let count = take_count(pane);
            move_right(pane, count, false);
        }
        KeyCode::Char('j') | KeyCode::Down => {
            let count = take_count(pane);
            move_down(pane, count, false);
        }
        KeyCode::Char('k') | KeyCode::Up => {
            let count = take_count(pane);
            move_up(pane, count, false);
        }
        KeyCode::Char('w') => {
            let count = take_count(pane);
            move_word_forward(pane, count);
        }
        KeyCode::Char('b') => {
            let count = take_count(pane);
            move_word_back(pane, count);
        }
        KeyCode::Char('e') => {
            let count = take_count(pane);
            move_word_end(pane, count);
        }
        KeyCode::Char('0') | KeyCode::Home => {
            pane.query_pending_count = None;
            move_to_line_start(pane);
        }
        KeyCode::Char('^') => {
            pane.query_pending_count = None;
            move_to_first_non_blank(pane);
        }
        KeyCode::Char('$') | KeyCode::End => {
            pane.query_pending_count = None;
            move_to_line_end(pane, false);
        }
        KeyCode::Char('G') => {
            if let Some(line) = pane.query_pending_count.take() {
                move_to_line_number(pane, line);
            } else {
                move_to_bottom(pane);
            }
        }
        KeyCode::Char('g') => {
            pane.query_pending_count = None;
            move_to_top(pane);
        }

        KeyCode::Char('y') => {
            yank_visual_selection(pane);
            exit_visual_mode(pane);
        }
        KeyCode::Char('d') => {
            run_edit(pane, delete_visual_selection);
            exit_visual_mode(pane);
        }
        KeyCode::Char('c') => {
            run_edit(pane, delete_visual_selection);
            exit_visual_mode(pane);
            enter_insert_mode(pane);
        }

        _ => {
            handled = false;
        }
    }

    if handled {
        clamp_cursor_for_mode(pane, false);
    }

    handled
}

fn handle_pending_normal_combo(event: KeyEvent, pane: &mut Pane) -> bool {
    let Some(pending) = pane.query_pending_key.take() else {
        return false;
    };

    match pending {
        'g' => {
            if event.code == KeyCode::Char('g') {
                if let Some(line) = pane.query_pending_count.take() {
                    move_to_line_number(pane, line);
                } else {
                    move_to_top(pane);
                }
                return true;
            }
            false
        }
        'd' => handle_pending_delete(event, pane),
        'c' => handle_pending_change(event, pane),
        'y' => handle_pending_yank(event, pane),
        'r' => handle_pending_replace(event, pane),
        'f' | 'F' | 't' | 'T' => handle_pending_find(event, pane, pending),
        'I' => handle_pending_text_object(event, pane, 'd', false),
        'A' => handle_pending_text_object(event, pane, 'd', true),
        'K' => handle_pending_text_object(event, pane, 'c', false),
        'L' => handle_pending_text_object(event, pane, 'c', true),
        'M' => handle_pending_text_object(event, pane, 'y', false),
        'N' => handle_pending_text_object(event, pane, 'y', true),
        _ => false,
    }
}

fn handle_pending_delete(event: KeyEvent, pane: &mut Pane) -> bool {
    if let KeyCode::Char(c) = event.code {
        if is_count_digit(c, pane.query_pending_count.is_some()) {
            append_count_digit(pane, c);
            pane.query_pending_key = Some('d');
            return true;
        }
    }

    let recognized = match event.code {
        KeyCode::Char('d') => {
            let count = take_count(pane);
            run_edit(pane, |p| delete_line_count(p, count));
            true
        }
        KeyCode::Char('w') => {
            let count = take_count(pane);
            run_edit(pane, |p| delete_word_forward(p, count));
            true
        }
        KeyCode::Char('b') => {
            let count = take_count(pane);
            run_edit(pane, |p| delete_word_backward(p, count));
            true
        }
        KeyCode::Char('e') => {
            let count = take_count(pane);
            run_edit(pane, |p| delete_word_end(p, count));
            true
        }
        KeyCode::Char('0') | KeyCode::Home => {
            pane.query_pending_count = None;
            run_edit(pane, delete_to_line_start);
            true
        }
        KeyCode::Char('^') => {
            pane.query_pending_count = None;
            run_edit(pane, delete_to_first_non_blank);
            true
        }
        KeyCode::Char('$') | KeyCode::End => {
            pane.query_pending_count = None;
            run_edit(pane, delete_to_line_end);
            true
        }
        KeyCode::Char('j') | KeyCode::Down => {
            let count = take_count(pane);
            run_edit(pane, |p| delete_line_count(p, count.saturating_add(1)));
            true
        }
        KeyCode::Char('k') | KeyCode::Up => {
            let count = take_count(pane);
            run_edit(pane, |p| delete_previous_lines_and_current(p, count));
            true
        }
        KeyCode::Char('i') => {
            pane.query_pending_key = Some('I');
            true
        }
        KeyCode::Char('a') => {
            pane.query_pending_key = Some('A');
            true
        }
        _ => false,
    };

    if recognized {
        close_autocomplete(pane);
    }

    recognized
}

fn handle_pending_change(event: KeyEvent, pane: &mut Pane) -> bool {
    if let KeyCode::Char(c) = event.code {
        if is_count_digit(c, pane.query_pending_count.is_some()) {
            append_count_digit(pane, c);
            pane.query_pending_key = Some('c');
            return true;
        }
    }

    let recognized = match event.code {
        KeyCode::Char('c') => {
            let count = take_count(pane);
            run_edit(pane, |p| delete_line_count(p, count));
            true
        }
        KeyCode::Char('w') => {
            let count = take_count(pane);
            run_edit(pane, |p| delete_word_forward(p, count));
            true
        }
        KeyCode::Char('b') => {
            let count = take_count(pane);
            run_edit(pane, |p| delete_word_backward(p, count));
            true
        }
        KeyCode::Char('e') => {
            let count = take_count(pane);
            run_edit(pane, |p| delete_word_end(p, count));
            true
        }
        KeyCode::Char('0') | KeyCode::Home => {
            pane.query_pending_count = None;
            run_edit(pane, delete_to_line_start);
            true
        }
        KeyCode::Char('^') => {
            pane.query_pending_count = None;
            run_edit(pane, delete_to_first_non_blank);
            true
        }
        KeyCode::Char('$') | KeyCode::End => {
            pane.query_pending_count = None;
            run_edit(pane, delete_to_line_end);
            true
        }
        KeyCode::Char('i') => {
            pane.query_pending_key = Some('K');
            true
        }
        KeyCode::Char('a') => {
            pane.query_pending_key = Some('L');
            true
        }
        _ => false,
    };

    if recognized && pane.query_pending_key.is_none() {
        enter_insert_mode(pane);
    }

    recognized
}

fn handle_pending_yank(event: KeyEvent, pane: &mut Pane) -> bool {
    if let KeyCode::Char(c) = event.code {
        if is_count_digit(c, pane.query_pending_count.is_some()) {
            append_count_digit(pane, c);
            pane.query_pending_key = Some('y');
            return true;
        }
    }

    match event.code {
        KeyCode::Char('y') => {
            let count = take_count(pane);
            yank_line_count(pane, count)
        }
        KeyCode::Char('w') => {
            let count = take_count(pane);
            yank_word_forward(pane, count)
        }
        KeyCode::Char('b') => {
            let count = take_count(pane);
            yank_word_backward(pane, count)
        }
        KeyCode::Char('e') => {
            let count = take_count(pane);
            yank_word_end(pane, count)
        }
        KeyCode::Char('0') | KeyCode::Home => {
            pane.query_pending_count = None;
            yank_to_line_start(pane)
        }
        KeyCode::Char('^') => {
            pane.query_pending_count = None;
            yank_to_first_non_blank(pane)
        }
        KeyCode::Char('$') | KeyCode::End => {
            pane.query_pending_count = None;
            yank_to_line_end(pane)
        }
        KeyCode::Char('j') | KeyCode::Down => {
            let count = take_count(pane);
            yank_line_count(pane, count.saturating_add(1))
        }
        KeyCode::Char('k') | KeyCode::Up => {
            let count = take_count(pane);
            yank_previous_lines_and_current(pane, count)
        }
        KeyCode::Char('i') => {
            pane.query_pending_key = Some('M');
            true
        }
        KeyCode::Char('a') => {
            pane.query_pending_key = Some('N');
            true
        }
        _ => false,
    }
}

fn handle_pending_text_object(event: KeyEvent, pane: &mut Pane, op: char, around: bool) -> bool {
    let count = take_count(pane);
    let changed = match event.code {
        KeyCode::Char('w') => match op {
            'd' | 'c' => run_edit(pane, |p| delete_word_text_object(p, around, count)),
            'y' => yank_word_text_object(pane, around, count),
            _ => false,
        },
        _ => return false,
    };

    close_autocomplete(pane);
    if op == 'c' && changed {
        enter_insert_mode(pane);
    }

    true
}

fn handle_pending_replace(event: KeyEvent, pane: &mut Pane) -> bool {
    match event.code {
        KeyCode::Char(c) => {
            let count = take_count(pane);
            run_edit(pane, |p| replace_count_chars_at_cursor(p, c, count));
            true
        }
        _ => false,
    }
}

fn handle_pending_find(event: KeyEvent, pane: &mut Pane, action: char) -> bool {
    let KeyCode::Char(target) = event.code else {
        return false;
    };

    let count = take_count(pane);
    let mut moved = false;
    for _ in 0..count {
        if !execute_find_action(pane, action, target) {
            break;
        }
        moved = true;
    }

    if moved {
        pane.query_last_find = Some((action, target));
    }

    true
}

fn handle_count_prefix(event: KeyEvent, pane: &mut Pane) -> bool {
    if event.modifiers.contains(KeyModifiers::CONTROL)
        || event.modifiers.contains(KeyModifiers::ALT)
    {
        return false;
    }

    let KeyCode::Char(c) = event.code else {
        return false;
    };

    if !is_count_digit(c, pane.query_pending_count.is_some()) {
        return false;
    }

    append_count_digit(pane, c);
    true
}

fn is_count_digit(c: char, already_building: bool) -> bool {
    c.is_ascii_digit() && (c != '0' || already_building)
}

fn append_count_digit(pane: &mut Pane, digit: char) {
    let value = digit.to_digit(10).unwrap_or(0) as usize;
    let current = pane.query_pending_count.unwrap_or(0);
    pane.query_pending_count = Some(current.saturating_mul(10).saturating_add(value));
}

fn take_count(pane: &mut Pane) -> usize {
    pane.query_pending_count.take().unwrap_or(1)
}

fn clear_query_pending_state(pane: &mut Pane) {
    pane.query_pending_key = None;
    pane.query_pending_count = None;
}

fn is_visual_active(pane: &Pane) -> bool {
    pane.query_visual_anchor.is_some()
}

fn enter_visual_char_mode(pane: &mut Pane) {
    if pane.query_visual_anchor.is_none() {
        pane.query_visual_anchor = Some((pane.query_cursor.0, pane.query_cursor.1));
    }
    pane.query_visual_line_mode = false;
    pane.query_pending_count = None;
    pane.query_pending_key = None;
    close_autocomplete(pane);
}

fn enter_visual_line_mode(pane: &mut Pane) {
    if pane.query_visual_anchor.is_none() {
        pane.query_visual_anchor = Some((pane.query_cursor.0, pane.query_cursor.1));
    }
    pane.query_visual_line_mode = true;
    pane.query_pending_count = None;
    pane.query_pending_key = None;
    close_autocomplete(pane);
}

fn exit_visual_mode(pane: &mut Pane) {
    pane.query_visual_anchor = None;
    pane.query_visual_line_mode = false;
    pane.query_pending_count = None;
    pane.query_pending_key = None;
}

// ═══════════════════════════════════════════════════════════════════════════════
// Insert mode
// ═══════════════════════════════════════════════════════════════════════════════

fn handle_insert_mode(event: KeyEvent, pane: &mut Pane, tables: &[String]) -> bool {
    if event.code == KeyCode::Esc {
        pane.mode = TableMode::Normal;
        pane.query_pending_key = None;
        pane.query_pending_count = None;
        close_autocomplete(pane);

        // Vim behavior: on leaving Insert, cursor lands on previous char.
        if pane.query_cursor.1 > 0 {
            pane.query_cursor.1 -= 1;
        }
        clamp_cursor_for_mode(pane, false);
        return true;
    }

    let popup_open = pane.autocomplete_selected.is_some() && !pane.autocomplete_matches.is_empty();
    if popup_open {
        match event.code {
            KeyCode::Up => {
                autocomplete_prev(pane);
                return true;
            }
            KeyCode::Down => {
                autocomplete_next(pane);
                return true;
            }
            KeyCode::Tab => {
                if event.modifiers.contains(KeyModifiers::SHIFT) {
                    autocomplete_prev(pane);
                } else {
                    autocomplete_next(pane);
                }
                return true;
            }
            KeyCode::Enter => {
                run_edit(pane, apply_autocomplete_selection);
                close_autocomplete(pane);
                clamp_cursor_for_mode(pane, true);
                refresh_autocomplete(pane, tables);
                return true;
            }
            KeyCode::Char(' ') => {
                // Space should close the popup and still be inserted below.
                close_autocomplete(pane);
            }
            _ => {}
        }
    } else if event.code == KeyCode::Tab {
        trigger_manual_autocomplete(pane, tables);
        return true;
    }

    let mut changed_text = false;
    let mut moved_cursor = false;

    if !event.modifiers.contains(KeyModifiers::CONTROL)
        && !event.modifiers.contains(KeyModifiers::ALT)
    {
        match event.code {
            KeyCode::Char(c) => {
                changed_text = run_edit(pane, |p| insert_char(p, c));
            }
            KeyCode::Enter => {
                changed_text = run_edit(pane, insert_newline);
            }
            KeyCode::Backspace => {
                changed_text = run_edit(pane, backspace);
            }
            KeyCode::Delete => {
                changed_text = run_edit(pane, |p| delete_char_at_cursor(p, true, false));
            }
            KeyCode::Left => {
                move_left(pane, 1);
                moved_cursor = true;
            }
            KeyCode::Right => {
                move_right(pane, 1, true);
                moved_cursor = true;
            }
            KeyCode::Up => {
                move_up(pane, 1, true);
                moved_cursor = true;
            }
            KeyCode::Down => {
                move_down(pane, 1, true);
                moved_cursor = true;
            }
            KeyCode::Home => {
                move_to_line_start(pane);
                moved_cursor = true;
            }
            KeyCode::End => {
                move_to_line_end(pane, true);
                moved_cursor = true;
            }
            _ => {}
        }
    }

    if !changed_text && !moved_cursor {
        return true;
    }

    clamp_cursor_for_mode(pane, true);

    if changed_text || moved_cursor {
        refresh_autocomplete(pane, tables);
    }

    true
}

// ═══════════════════════════════════════════════════════════════════════════════
// Shared state/edit helpers
// ═══════════════════════════════════════════════════════════════════════════════

fn run_edit<F>(pane: &mut Pane, mutator: F) -> bool
where
    F: FnOnce(&mut Pane) -> bool,
{
    let before_text = pane.query_text.clone();
    let before_cursor = pane.query_cursor;

    if !mutator(pane) {
        return false;
    }

    if pane.query_text == before_text && pane.query_cursor == before_cursor {
        return false;
    }

    record_query_snapshot(pane, before_text, before_cursor);
    true
}

fn record_query_snapshot(pane: &mut Pane, text: Vec<String>, cursor: (usize, usize)) {
    let snapshot = QueryEditorSnapshot { text, cursor };
    if pane.query_undo_stack.last() != Some(&snapshot) {
        pane.query_undo_stack.push(snapshot);
    }
    pane.query_redo_stack.clear();
}

fn enter_insert_mode(pane: &mut Pane) {
    pane.mode = TableMode::Insert;
    pane.query_pending_key = None;
    pane.query_pending_count = None;
    close_autocomplete(pane);
}

fn ensure_query_buffer(pane: &mut Pane) {
    if pane.query_text.is_empty() {
        pane.query_text.push(String::new());
    }
}

fn clamp_cursor_for_mode(pane: &mut Pane, insert_mode: bool) {
    ensure_query_buffer(pane);

    if pane.query_cursor.0 >= pane.query_text.len() {
        pane.query_cursor.0 = pane.query_text.len().saturating_sub(1);
    }

    let row = pane.query_cursor.0;
    let line_len = line_char_len(&pane.query_text[row]);
    let max_col = if insert_mode {
        line_len
    } else {
        line_len.saturating_sub(1)
    };

    if pane.query_cursor.1 > max_col {
        pane.query_cursor.1 = max_col;
    }
}

fn line_char_len(line: &str) -> usize {
    line.chars().count()
}

fn current_line_char_len(pane: &Pane) -> usize {
    pane.query_text
        .get(pane.query_cursor.0)
        .map_or(0, |line| line_char_len(line))
}

// ═══════════════════════════════════════════════════════════════════════════════
// Autocomplete
// ═══════════════════════════════════════════════════════════════════════════════

fn close_autocomplete(pane: &mut Pane) {
    pane.autocomplete_selected = None;
    pane.autocomplete_matches.clear();
}

fn autocomplete_prev(pane: &mut Pane) {
    let Some(sel) = pane.autocomplete_selected else {
        return;
    };
    let len = pane.autocomplete_matches.len();
    if len == 0 {
        pane.autocomplete_selected = None;
        return;
    }
    pane.autocomplete_selected = Some((sel + len - 1) % len);
}

fn autocomplete_next(pane: &mut Pane) {
    let Some(sel) = pane.autocomplete_selected else {
        return;
    };
    let len = pane.autocomplete_matches.len();
    if len == 0 {
        pane.autocomplete_selected = None;
        return;
    }
    pane.autocomplete_selected = Some((sel + 1) % len);
}

fn trigger_manual_autocomplete(pane: &mut Pane, tables: &[String]) {
    let (row, col) = pane.query_cursor;
    let Some(line) = pane.query_text.get(row) else {
        close_autocomplete(pane);
        return;
    };

    if let Some(prefix) = get_table_prefix(line, col) {
        set_autocomplete_matches(pane, tables, &prefix);
        return;
    }

    let (_, prefix) = token_prefix(line, col);
    if prefix.is_empty() {
        close_autocomplete(pane);
    } else {
        set_autocomplete_matches(pane, tables, &prefix);
    }
}

fn refresh_autocomplete(pane: &mut Pane, tables: &[String]) {
    let (row, col) = pane.query_cursor;
    let Some(line) = pane.query_text.get(row) else {
        close_autocomplete(pane);
        return;
    };

    if let Some(prefix) = get_table_prefix(line, col) {
        set_autocomplete_matches(pane, tables, &prefix);
    } else {
        close_autocomplete(pane);
    }
}

fn set_autocomplete_matches(pane: &mut Pane, tables: &[String], prefix: &str) {
    let prefix_lc = prefix.to_lowercase();
    let matches: Vec<String> = tables
        .iter()
        .filter(|t| t.to_lowercase().starts_with(&prefix_lc))
        .cloned()
        .collect();

    if matches.is_empty() {
        close_autocomplete(pane);
        return;
    }

    pane.autocomplete_matches = matches;
    pane.autocomplete_selected = Some(0);
}

fn apply_autocomplete_selection(pane: &mut Pane) -> bool {
    let Some(selected) = pane.autocomplete_selected else {
        return false;
    };
    let Some(replacement) = pane.autocomplete_matches.get(selected).cloned() else {
        return false;
    };

    let quoted = if replacement.chars().any(|c| c.is_uppercase()) {
        format!("\"{}\"", replacement)
    } else {
        replacement
    };

    let (row, col) = pane.query_cursor;
    let Some(line) = pane.query_text.get_mut(row) else {
        return false;
    };

    let (start_col, _) = token_prefix(line, col);
    let start_byte = char_idx_to_byte_idx(line, start_col);
    let end_byte = char_idx_to_byte_idx(line, col);
    if start_byte > end_byte || end_byte > line.len() {
        return false;
    }

    line.replace_range(start_byte..end_byte, &quoted);
    pane.query_cursor = (row, start_col + quoted.chars().count());
    true
}

// ═══════════════════════════════════════════════════════════════════════════════
// Movements
// ═══════════════════════════════════════════════════════════════════════════════

fn move_left(pane: &mut Pane, count: usize) {
    pane.query_cursor.1 = pane.query_cursor.1.saturating_sub(count);
}

fn move_right(pane: &mut Pane, count: usize, insert_mode: bool) {
    let len = current_line_char_len(pane);
    let max_col = if insert_mode {
        len
    } else {
        len.saturating_sub(1)
    };
    pane.query_cursor.1 = (pane.query_cursor.1 + count).min(max_col);
}

fn move_down(pane: &mut Pane, count: usize, insert_mode: bool) {
    if pane.query_text.is_empty() {
        return;
    }
    pane.query_cursor.0 =
        (pane.query_cursor.0 + count).min(pane.query_text.len().saturating_sub(1));
    clamp_cursor_for_mode(pane, insert_mode);
}

fn move_up(pane: &mut Pane, count: usize, insert_mode: bool) {
    pane.query_cursor.0 = pane.query_cursor.0.saturating_sub(count);
    clamp_cursor_for_mode(pane, insert_mode);
}

fn move_to_line_start(pane: &mut Pane) {
    pane.query_cursor.1 = 0;
}

fn move_to_first_non_blank(pane: &mut Pane) {
    let Some(line) = pane.query_text.get(pane.query_cursor.0) else {
        pane.query_cursor.1 = 0;
        return;
    };
    let idx = line.chars().take_while(|c| c.is_whitespace()).count();
    pane.query_cursor.1 = idx;
}

fn move_to_line_end(pane: &mut Pane, insert_mode: bool) {
    let len = current_line_char_len(pane);
    pane.query_cursor.1 = if insert_mode {
        len
    } else {
        len.saturating_sub(1)
    };
}

fn move_to_top(pane: &mut Pane) {
    pane.query_cursor.0 = 0;
    clamp_cursor_for_mode(pane, false);
}

fn move_to_line_number(pane: &mut Pane, line: usize) {
    if line == 0 {
        move_to_top(pane);
        return;
    }
    pane.query_cursor.0 = line
        .saturating_sub(1)
        .min(pane.query_text.len().saturating_sub(1));
    clamp_cursor_for_mode(pane, false);
}

fn move_to_bottom(pane: &mut Pane) {
    pane.query_cursor.0 = pane.query_text.len().saturating_sub(1);
    clamp_cursor_for_mode(pane, false);
}

fn move_word_forward(pane: &mut Pane, count: usize) {
    for _ in 0..count {
        next_word_start(pane);
    }
}

fn move_word_back(pane: &mut Pane, count: usize) {
    for _ in 0..count {
        prev_word_start(pane);
    }
}

fn move_word_end(pane: &mut Pane, count: usize) {
    for _ in 0..count {
        next_word_end(pane);
    }
}

fn is_word_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

fn next_word_start(pane: &mut Pane) {
    let mut row = pane.query_cursor.0;
    let mut col = pane.query_cursor.1;

    loop {
        let Some(line) = pane.query_text.get(row) else {
            break;
        };
        let chars: Vec<char> = line.chars().collect();

        if chars.is_empty() {
            if row + 1 >= pane.query_text.len() {
                pane.query_cursor = (row, 0);
                return;
            }
            row += 1;
            col = 0;
            continue;
        }

        if col >= chars.len() {
            if row + 1 >= pane.query_text.len() {
                pane.query_cursor = (row, chars.len().saturating_sub(1));
                return;
            }
            row += 1;
            col = 0;
            continue;
        }

        if is_word_char(chars[col]) {
            while col < chars.len() && is_word_char(chars[col]) {
                col += 1;
            }
        } else {
            while col < chars.len() && !is_word_char(chars[col]) {
                col += 1;
            }
        }

        while col < chars.len() && chars[col].is_whitespace() {
            col += 1;
        }

        if col < chars.len() {
            pane.query_cursor = (row, col);
            return;
        }

        if row + 1 >= pane.query_text.len() {
            pane.query_cursor = (row, chars.len().saturating_sub(1));
            return;
        }

        row += 1;
        col = 0;
    }
}

fn prev_word_start(pane: &mut Pane) {
    if pane.query_cursor.0 == 0 && pane.query_cursor.1 == 0 {
        return;
    }

    let mut row = pane.query_cursor.0;
    let mut col = pane.query_cursor.1;

    loop {
        let Some(line) = pane.query_text.get(row) else {
            return;
        };
        let chars: Vec<char> = line.chars().collect();

        if chars.is_empty() || col == 0 {
            if row == 0 {
                pane.query_cursor = (0, 0);
                return;
            }
            row -= 1;
            col = pane
                .query_text
                .get(row)
                .map_or(0, |prev| line_char_len(prev).saturating_sub(1));
            continue;
        }

        col = col.min(chars.len()).saturating_sub(1);

        while col > 0 && chars[col].is_whitespace() {
            col -= 1;
        }

        let word = is_word_char(chars[col]);
        while col > 0 && is_word_char(chars[col - 1]) == word && !chars[col - 1].is_whitespace() {
            col -= 1;
        }

        pane.query_cursor = (row, col);
        return;
    }
}

fn next_word_end(pane: &mut Pane) {
    let mut row = pane.query_cursor.0;
    let mut col = pane.query_cursor.1.saturating_add(1);

    loop {
        let Some(line) = pane.query_text.get(row) else {
            return;
        };
        let chars: Vec<char> = line.chars().collect();

        if chars.is_empty() {
            if row + 1 >= pane.query_text.len() {
                pane.query_cursor = (row, 0);
                return;
            }
            row += 1;
            col = 0;
            continue;
        }

        while col < chars.len() && chars[col].is_whitespace() {
            col += 1;
        }

        if col >= chars.len() {
            if row + 1 >= pane.query_text.len() {
                pane.query_cursor = (row, chars.len().saturating_sub(1));
                return;
            }
            row += 1;
            col = 0;
            continue;
        }

        let word = is_word_char(chars[col]);
        while col + 1 < chars.len()
            && is_word_char(chars[col + 1]) == word
            && !chars[col + 1].is_whitespace()
        {
            col += 1;
        }

        pane.query_cursor = (row, col);
        return;
    }
}

fn move_to_matching_bracket(pane: &mut Pane) -> bool {
    let start = QueryCursor::from(pane.query_cursor);
    let Some(ch) = char_at_cursor(&pane.query_text, start) else {
        return false;
    };

    let (target, forward) = match ch {
        '(' => (')', true),
        ')' => ('(', false),
        '[' => (']', true),
        ']' => ('[', false),
        '{' => ('}', true),
        '}' => ('{', false),
        _ => return false,
    };

    let mut depth = 1usize;
    let mut pos = start;

    loop {
        let next_pos = if forward {
            next_char_position(&pane.query_text, pos)
        } else {
            prev_char_position(&pane.query_text, pos)
        };
        let Some(next) = next_pos else {
            return false;
        };

        pos = next;
        let Some(cur) = char_at_cursor(&pane.query_text, pos) else {
            return false;
        };

        if cur == ch {
            depth += 1;
        } else if cur == target {
            depth = depth.saturating_sub(1);
            if depth == 0 {
                pane.query_cursor = pos.into();
                return true;
            }
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Find / repeat-find
// ═══════════════════════════════════════════════════════════════════════════════

fn execute_find_action(pane: &mut Pane, action: char, target: char) -> bool {
    let row = pane.query_cursor.0;
    let col = pane.query_cursor.1;
    let Some(line) = pane.query_text.get(row) else {
        return false;
    };

    let chars: Vec<char> = line.chars().collect();
    if chars.is_empty() {
        return false;
    }

    match action {
        'f' => {
            for idx in col.saturating_add(1)..chars.len() {
                if chars[idx] == target {
                    pane.query_cursor = (row, idx);
                    return true;
                }
            }
            false
        }
        'F' => {
            if col == 0 {
                return false;
            }
            for idx in (0..col).rev() {
                if chars[idx] == target {
                    pane.query_cursor = (row, idx);
                    return true;
                }
            }
            false
        }
        't' => {
            for idx in col.saturating_add(1)..chars.len() {
                if chars[idx] == target {
                    pane.query_cursor = (row, idx.saturating_sub(1));
                    return true;
                }
            }
            false
        }
        'T' => {
            if col == 0 {
                return false;
            }
            for idx in (0..col).rev() {
                if chars[idx] == target {
                    pane.query_cursor = (row, (idx + 1).min(chars.len().saturating_sub(1)));
                    return true;
                }
            }
            false
        }
        _ => false,
    }
}

fn repeat_last_find(pane: &mut Pane, reverse: bool, count: usize) -> bool {
    let Some((action, target)) = pane.query_last_find else {
        return false;
    };

    let action = if reverse {
        match action {
            'f' => 'F',
            'F' => 'f',
            't' => 'T',
            'T' => 't',
            other => other,
        }
    } else {
        action
    };

    let mut moved = false;
    for _ in 0..count {
        if !execute_find_action(pane, action, target) {
            break;
        }
        moved = true;
    }
    moved
}

// ═══════════════════════════════════════════════════════════════════════════════
// Visual selection helpers
// ═══════════════════════════════════════════════════════════════════════════════

fn visual_selection(pane: &Pane) -> Option<QueryVisualSelection> {
    let anchor = pane.query_visual_anchor.map(QueryCursor::from)?;
    let cursor = QueryCursor::from(pane.query_cursor);

    if pane.query_visual_line_mode {
        let start_row = anchor.row.min(cursor.row);
        let end_row = anchor.row.max(cursor.row);
        let end_col = pane
            .query_text
            .get(end_row)
            .map_or(0, |line| line_char_len(line));
        return Some(QueryVisualSelection {
            start: QueryCursor {
                row: start_row,
                col: 0,
            },
            end: QueryCursor {
                row: end_row,
                col: end_col,
            },
            linewise: true,
        });
    }

    let min = anchor.min(cursor);
    let max = anchor.max(cursor);
    let end = cursor_after_current_char(&pane.query_text, max);

    if min == end {
        return None;
    }

    Some(QueryVisualSelection {
        start: min,
        end,
        linewise: false,
    })
}

fn delete_visual_selection(pane: &mut Pane) -> bool {
    let Some(sel) = visual_selection(pane) else {
        return false;
    };

    if sel.linewise {
        delete_line_range(pane, sel.start.row, sel.end.row)
    } else {
        delete_range(pane, sel.start, sel.end)
    }
}

fn yank_visual_selection(pane: &mut Pane) -> bool {
    let Some(sel) = visual_selection(pane) else {
        return false;
    };

    if sel.linewise {
        let text = collect_line_range_text(&pane.query_text, sel.start.row, sel.end.row);
        let ranges =
            collect_line_range_highlight_ranges(&pane.query_text, sel.start.row, sel.end.row);
        set_yank_register_with_flash(pane, text, true, ranges)
    } else if let Some(text) = collect_range_text(&pane.query_text, sel.start, sel.end) {
        let ranges = collect_range_highlight_ranges(&pane.query_text, sel.start, sel.end);
        set_yank_register_with_flash(pane, text, false, ranges)
    } else {
        false
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Yank / paste helpers
// ═══════════════════════════════════════════════════════════════════════════════

fn set_yank_register(
    pane: &mut Pane,
    text: String,
    linewise: bool,
    flash_ranges: Option<Vec<(usize, usize, usize)>>,
) -> bool {
    if text.is_empty() {
        return false;
    }

    pane.query_yank_register = text.clone();
    pane.query_yank_linewise = linewise;

    if let Some(ranges) = flash_ranges.filter(|r| !r.is_empty()) {
        pane.query_yank_highlight_ranges = ranges;
        pane.query_yank_highlight_at = Some(std::time::Instant::now());
    }

    let mut clipboard_text = text;
    if linewise && !clipboard_text.ends_with('\n') {
        clipboard_text.push('\n');
    }
    copy_to_system_clipboard(&clipboard_text);
    true
}

fn set_delete_register(pane: &mut Pane, text: String, linewise: bool) -> bool {
    set_yank_register(pane, text, linewise, None)
}

fn set_yank_register_with_flash(
    pane: &mut Pane,
    text: String,
    linewise: bool,
    flash_ranges: Vec<(usize, usize, usize)>,
) -> bool {
    set_yank_register(pane, text, linewise, Some(flash_ranges))
}

fn yank_line_count(pane: &mut Pane, count: usize) -> bool {
    let start = pane.query_cursor.0;
    let end = start
        .saturating_add(count.saturating_sub(1))
        .min(pane.query_text.len().saturating_sub(1));
    let text = collect_line_range_text(&pane.query_text, start, end);
    let ranges = collect_line_range_highlight_ranges(&pane.query_text, start, end);
    set_yank_register_with_flash(pane, text, true, ranges)
}

fn yank_previous_lines_and_current(pane: &mut Pane, count: usize) -> bool {
    let end = pane.query_cursor.0;
    let start = end.saturating_sub(count);
    let text = collect_line_range_text(&pane.query_text, start, end);
    let ranges = collect_line_range_highlight_ranges(&pane.query_text, start, end);
    set_yank_register_with_flash(pane, text, true, ranges)
}

fn yank_word_forward(pane: &mut Pane, count: usize) -> bool {
    let start = QueryCursor::from(pane.query_cursor);
    let end = motion_target(pane, |p| move_word_forward(p, count));
    yank_cursor_range(pane, start, end)
}

fn yank_word_end(pane: &mut Pane, count: usize) -> bool {
    let start = QueryCursor::from(pane.query_cursor);
    let end_inclusive = motion_target(pane, |p| move_word_end(p, count));
    let end = cursor_after_current_char(&pane.query_text, end_inclusive);
    yank_cursor_range(pane, start, end)
}

fn yank_word_backward(pane: &mut Pane, count: usize) -> bool {
    let cur = QueryCursor::from(pane.query_cursor);
    let start = motion_target(pane, |p| move_word_back(p, count));
    let end = cursor_after_current_char(&pane.query_text, cur);
    yank_cursor_range(pane, start, end)
}

fn yank_to_line_start(pane: &mut Pane) -> bool {
    let cur = QueryCursor::from(pane.query_cursor);
    if cur.col == 0 {
        return false;
    }
    let end = cursor_after_current_char(&pane.query_text, cur);
    yank_cursor_range(
        pane,
        QueryCursor {
            row: cur.row,
            col: 0,
        },
        end,
    )
}

fn yank_to_first_non_blank(pane: &mut Pane) -> bool {
    let cur = QueryCursor::from(pane.query_cursor);
    let Some(line) = pane.query_text.get(cur.row) else {
        return false;
    };
    let first_non_blank = line.chars().take_while(|c| c.is_whitespace()).count();
    if first_non_blank >= cur.col {
        return false;
    }
    let end = cursor_after_current_char(&pane.query_text, cur);
    yank_cursor_range(
        pane,
        QueryCursor {
            row: cur.row,
            col: first_non_blank,
        },
        end,
    )
}

fn yank_to_line_end(pane: &mut Pane) -> bool {
    let start = QueryCursor::from(pane.query_cursor);
    let end = QueryCursor {
        row: start.row,
        col: current_line_char_len(pane),
    };
    yank_cursor_range(pane, start, end)
}

fn yank_word_text_object(pane: &mut Pane, around: bool, count: usize) -> bool {
    let mut changed = false;
    for _ in 0..count {
        if !yank_single_word_text_object(pane, around) {
            break;
        }
        changed = true;
    }
    changed
}

fn yank_single_word_text_object(pane: &mut Pane, around: bool) -> bool {
    let row = pane.query_cursor.0;
    let Some(line) = pane.query_text.get(row) else {
        return false;
    };

    let chars: Vec<char> = line.chars().collect();
    if chars.is_empty() {
        return false;
    }

    let mut col = pane.query_cursor.1.min(chars.len().saturating_sub(1));
    if !is_word_char(chars[col]) {
        while col < chars.len() && !is_word_char(chars[col]) {
            col += 1;
        }
        if col >= chars.len() {
            return false;
        }
    }

    let mut start = col;
    while start > 0 && is_word_char(chars[start - 1]) {
        start -= 1;
    }

    let mut end = col;
    while end < chars.len() && is_word_char(chars[end]) {
        end += 1;
    }

    if around {
        let mut around_end = end;
        while around_end < chars.len() && chars[around_end].is_whitespace() {
            around_end += 1;
        }
        if around_end == end {
            let mut around_start = start;
            while around_start > 0 && chars[around_start - 1].is_whitespace() {
                around_start -= 1;
            }
            start = around_start;
        }
        end = around_end;
    }

    if start >= end {
        return false;
    }

    yank_cursor_range(
        pane,
        QueryCursor { row, col: start },
        QueryCursor { row, col: end },
    )
}

fn yank_cursor_range(pane: &mut Pane, a: QueryCursor, b: QueryCursor) -> bool {
    let (start, end) = order_range(a, b);
    let Some(text) = collect_range_text(&pane.query_text, start, end) else {
        return false;
    };
    let ranges = collect_range_highlight_ranges(&pane.query_text, start, end);
    set_yank_register_with_flash(pane, text, false, ranges)
}

fn collect_line_range_text(lines: &[String], start: usize, end: usize) -> String {
    if lines.is_empty() {
        return String::new();
    }

    let last = lines.len().saturating_sub(1);
    let start = start.min(last);
    let end = end.min(last);
    let (start, end) = if start <= end {
        (start, end)
    } else {
        (end, start)
    };

    lines[start..=end].join("\n")
}

fn collect_range_text(lines: &[String], a: QueryCursor, b: QueryCursor) -> Option<String> {
    if lines.is_empty() {
        return None;
    }

    let (mut start, mut end) = order_range(a, b);
    let last = lines.len().saturating_sub(1);
    start.row = start.row.min(last);
    end.row = end.row.min(last);

    let start_len = lines.get(start.row).map_or(0, |line| line_char_len(line));
    let end_len = lines.get(end.row).map_or(0, |line| line_char_len(line));
    start.col = start.col.min(start_len);
    end.col = end.col.min(end_len);

    if start == end {
        return None;
    }

    if start.row == end.row {
        let line = lines.get(start.row)?;
        let s = char_idx_to_byte_idx(line, start.col);
        let e = char_idx_to_byte_idx(line, end.col);
        if s >= e || e > line.len() {
            return None;
        }
        return Some(line[s..e].to_string());
    }

    let mut out = String::new();
    let first = lines.get(start.row)?;
    let first_s = char_idx_to_byte_idx(first, start.col);
    if first_s > first.len() {
        return None;
    }
    out.push_str(&first[first_s..]);

    for row in (start.row + 1)..end.row {
        out.push('\n');
        out.push_str(lines.get(row)?);
    }

    out.push('\n');
    let last_line = lines.get(end.row)?;
    let last_e = char_idx_to_byte_idx(last_line, end.col);
    if last_e > last_line.len() {
        return None;
    }
    out.push_str(&last_line[..last_e]);

    Some(out)
}

fn collect_line_range_highlight_ranges(
    lines: &[String],
    start: usize,
    end: usize,
) -> Vec<(usize, usize, usize)> {
    if lines.is_empty() {
        return Vec::new();
    }

    let last = lines.len().saturating_sub(1);
    let start = start.min(last);
    let end = end.min(last);
    let (start, end) = if start <= end {
        (start, end)
    } else {
        (end, start)
    };

    (start..=end)
        .map(|row| {
            let len = lines.get(row).map_or(0, |line| line_char_len(line));
            (row, 0, len)
        })
        .collect()
}

fn collect_range_highlight_ranges(
    lines: &[String],
    a: QueryCursor,
    b: QueryCursor,
) -> Vec<(usize, usize, usize)> {
    if lines.is_empty() {
        return Vec::new();
    }

    let (mut start, mut end) = order_range(a, b);
    let last = lines.len().saturating_sub(1);
    start.row = start.row.min(last);
    end.row = end.row.min(last);

    let start_len = lines.get(start.row).map_or(0, |line| line_char_len(line));
    let end_len = lines.get(end.row).map_or(0, |line| line_char_len(line));
    start.col = start.col.min(start_len);
    end.col = end.col.min(end_len);

    if start == end {
        return Vec::new();
    }

    let mut out = Vec::new();
    if start.row == end.row {
        out.push((start.row, start.col, end.col));
        return out;
    }

    let first_len = lines.get(start.row).map_or(0, |line| line_char_len(line));
    out.push((start.row, start.col, first_len));

    for row in (start.row + 1)..end.row {
        let len = lines.get(row).map_or(0, |line| line_char_len(line));
        out.push((row, 0, len));
    }

    out.push((end.row, 0, end.col));
    out
}

fn paste_after_cursor(pane: &mut Pane) -> bool {
    clamp_cursor_for_mode(pane, false);

    let text = pane.query_yank_register.clone();
    if text.is_empty() {
        return false;
    }

    if pane.query_yank_linewise {
        let insert_at = pane
            .query_cursor
            .0
            .saturating_add(1)
            .min(pane.query_text.len());
        let lines: Vec<String> = text.split('\n').map(|s| s.to_string()).collect();
        if lines.is_empty() {
            return false;
        }
        for (i, line) in lines.iter().enumerate() {
            pane.query_text.insert(insert_at + i, line.clone());
        }
        pane.query_cursor = (insert_at, 0);
        return true;
    }

    insert_text_at_cursor(pane, &text, true)
}

fn paste_before_cursor(pane: &mut Pane) -> bool {
    clamp_cursor_for_mode(pane, false);

    let text = pane.query_yank_register.clone();
    if text.is_empty() {
        return false;
    }

    if pane.query_yank_linewise {
        let insert_at = pane.query_cursor.0.min(pane.query_text.len());
        let lines: Vec<String> = text.split('\n').map(|s| s.to_string()).collect();
        if lines.is_empty() {
            return false;
        }
        for (i, line) in lines.iter().enumerate() {
            pane.query_text.insert(insert_at + i, line.clone());
        }
        pane.query_cursor = (insert_at, 0);
        return true;
    }

    insert_text_at_cursor(pane, &text, false)
}

fn insert_text_at_cursor(pane: &mut Pane, text: &str, after: bool) -> bool {
    ensure_query_buffer(pane);
    let row = pane.query_cursor.0;
    let mut col = pane.query_cursor.1;

    let Some(current) = pane.query_text.get(row).cloned() else {
        return false;
    };

    if after {
        let len = line_char_len(&current);
        if len > 0 {
            col = (col + 1).min(len);
        }
    }

    let insert_lines: Vec<&str> = text.split('\n').collect();
    if insert_lines.is_empty() {
        return false;
    }

    if insert_lines.len() == 1 {
        if let Some(line) = pane.query_text.get_mut(row) {
            let byte_col = char_idx_to_byte_idx(line, col);
            line.insert_str(byte_col, insert_lines[0]);
            pane.query_cursor = (row, col + insert_lines[0].chars().count().saturating_sub(1));
            return true;
        }
        return false;
    }

    let byte_col = char_idx_to_byte_idx(&current, col);
    let prefix = current[..byte_col].to_string();
    let suffix = current[byte_col..].to_string();

    let first = format!("{}{}", prefix, insert_lines[0]);
    let last = format!(
        "{}{}",
        insert_lines.last().copied().unwrap_or_default(),
        suffix
    );

    pane.query_text[row] = first;
    for (i, segment) in insert_lines
        .iter()
        .enumerate()
        .skip(1)
        .take(insert_lines.len().saturating_sub(2))
    {
        pane.query_text.insert(row + i, (*segment).to_string());
    }

    let last_row = row + insert_lines.len() - 1;
    pane.query_text.insert(last_row, last);
    pane.query_cursor = (
        last_row,
        insert_lines
            .last()
            .map_or(0, |s| s.chars().count().saturating_sub(1)),
    );
    true
}

fn copy_to_system_clipboard(text: &str) {
    let cmds: &[(&str, &[&str])] = &[
        ("wl-copy", &[]),
        ("xclip", &["-selection", "clipboard"]),
        ("xsel", &["--clipboard", "--input"]),
    ];

    for (cmd, args) in cmds {
        if let Ok(mut child) = std::process::Command::new(cmd)
            .args(*args)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
        {
            if let Some(mut stdin) = child.stdin.take() {
                use std::io::Write;
                let _ = stdin.write_all(text.as_bytes());
            }
            let _ = child.wait();
            return;
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Editing primitives
// ═══════════════════════════════════════════════════════════════════════════════

fn insert_char(pane: &mut Pane, c: char) -> bool {
    ensure_query_buffer(pane);
    let (row, col) = pane.query_cursor;

    let Some(line) = pane.query_text.get_mut(row) else {
        return false;
    };

    let byte_col = char_idx_to_byte_idx(line, col);
    line.insert(byte_col, c);
    pane.query_cursor = (row, col + 1);
    true
}

fn insert_newline(pane: &mut Pane) -> bool {
    ensure_query_buffer(pane);
    let (row, col) = pane.query_cursor;

    let Some(line) = pane.query_text.get_mut(row) else {
        return false;
    };

    let byte_col = char_idx_to_byte_idx(line, col);
    let indent: String = line.chars().take_while(|c| c.is_whitespace()).collect();
    let rest = line[byte_col..].to_string();
    line.truncate(byte_col);

    pane.query_text
        .insert(row + 1, format!("{}{}", indent, rest));
    pane.query_cursor = (row + 1, indent.chars().count());
    true
}

fn backspace(pane: &mut Pane) -> bool {
    ensure_query_buffer(pane);
    let (row, col) = pane.query_cursor;

    if col > 0 {
        let Some(line) = pane.query_text.get_mut(row) else {
            return false;
        };
        let start = char_idx_to_byte_idx(line, col - 1);
        let end = char_idx_to_byte_idx(line, col);
        if start >= end || end > line.len() {
            return false;
        }
        line.replace_range(start..end, "");
        pane.query_cursor = (row, col - 1);
        return true;
    }

    if row == 0 {
        return false;
    }

    let current = pane.query_text.remove(row);
    let prev_row = row - 1;
    let prev_len = line_char_len(&pane.query_text[prev_row]);
    pane.query_text[prev_row].push_str(&current);
    pane.query_cursor = (prev_row, prev_len);
    true
}

fn delete_char_at_cursor(pane: &mut Pane, join_on_eol: bool, update_register: bool) -> bool {
    ensure_query_buffer(pane);
    let (row, col) = pane.query_cursor;

    let Some(line_len) = pane.query_text.get(row).map(|line| line_char_len(line)) else {
        return false;
    };

    if col < line_len {
        if update_register {
            let removed = pane
                .query_text
                .get(row)
                .and_then(|line| {
                    let start = char_idx_to_byte_idx(line, col);
                    let end = char_idx_to_byte_idx(line, col + 1);
                    (start < end && end <= line.len()).then(|| line[start..end].to_string())
                })
                .unwrap_or_default();
            if !removed.is_empty() {
                set_delete_register(pane, removed, false);
            }
        }

        let Some(line) = pane.query_text.get_mut(row) else {
            return false;
        };
        let start = char_idx_to_byte_idx(line, col);
        let end = char_idx_to_byte_idx(line, col + 1);
        if start >= end || end > line.len() {
            return false;
        }
        line.replace_range(start..end, "");
        return true;
    }

    if join_on_eol && row + 1 < pane.query_text.len() {
        if update_register {
            set_delete_register(pane, "\n".to_string(), false);
        }

        let next = pane.query_text.remove(row + 1);
        if let Some(line) = pane.query_text.get_mut(row) {
            line.push_str(&next);
            return true;
        }
    }

    false
}

fn replace_char_at_cursor(pane: &mut Pane, replacement: char) -> bool {
    ensure_query_buffer(pane);
    let (row, col) = pane.query_cursor;
    let Some(line) = pane.query_text.get_mut(row) else {
        return false;
    };

    let line_len = line_char_len(line);
    if col >= line_len {
        return false;
    }

    let start = char_idx_to_byte_idx(line, col);
    let end = char_idx_to_byte_idx(line, col + 1);
    if start >= end || end > line.len() {
        return false;
    }

    line.replace_range(start..end, &replacement.to_string());
    true
}

fn replace_count_chars_at_cursor(pane: &mut Pane, replacement: char, count: usize) -> bool {
    let mut changed = false;
    for _ in 0..count {
        if !replace_char_at_cursor(pane, replacement) {
            break;
        }
        changed = true;
        move_right(pane, 1, false);
    }
    if changed {
        move_left(pane, 1);
    }
    changed
}

fn join_with_next_line(pane: &mut Pane) -> bool {
    ensure_query_buffer(pane);
    let row = pane
        .query_cursor
        .0
        .min(pane.query_text.len().saturating_sub(1));
    if row + 1 >= pane.query_text.len() {
        return false;
    }

    let next = pane.query_text.remove(row + 1);
    let trimmed = next.trim_start().to_string();

    let current_len = line_char_len(&pane.query_text[row]);
    if !pane.query_text[row].is_empty() && !trimmed.is_empty() {
        pane.query_text[row].push(' ');
    }
    pane.query_text[row].push_str(&trimmed);

    pane.query_cursor = (row, current_len.saturating_sub(1));
    true
}

fn delete_line_count(pane: &mut Pane, count: usize) -> bool {
    let row = pane.query_cursor.0;
    let end = row.saturating_add(count.saturating_sub(1));
    delete_line_range(pane, row, end)
}

fn delete_previous_lines_and_current(pane: &mut Pane, count: usize) -> bool {
    let row = pane.query_cursor.0;
    let start = row.saturating_sub(count);
    delete_line_range(pane, start, row)
}

fn delete_line_range(pane: &mut Pane, start_row: usize, end_row: usize) -> bool {
    ensure_query_buffer(pane);
    if pane.query_text.is_empty() {
        return false;
    }

    let last = pane.query_text.len().saturating_sub(1);
    let mut start = start_row.min(last);
    let mut end = end_row.min(last);
    if start > end {
        std::mem::swap(&mut start, &mut end);
    }

    let removed_text = collect_line_range_text(&pane.query_text, start, end);

    if start == 0 && end == 0 && pane.query_text.len() == 1 {
        if pane.query_text[0].is_empty() {
            return false;
        }
        set_delete_register(pane, removed_text, true);
        pane.query_text[0].clear();
        pane.query_cursor = (0, 0);
        return true;
    }

    set_delete_register(pane, removed_text, true);

    pane.query_text.drain(start..=end);
    if pane.query_text.is_empty() {
        pane.query_text.push(String::new());
    }

    pane.query_cursor = (start.min(pane.query_text.len().saturating_sub(1)), 0);
    clamp_cursor_for_mode(pane, false);
    true
}

fn change_current_line(pane: &mut Pane) -> bool {
    ensure_query_buffer(pane);
    let row = pane
        .query_cursor
        .0
        .min(pane.query_text.len().saturating_sub(1));
    let indent: String = pane.query_text[row]
        .chars()
        .take_while(|c| c.is_whitespace())
        .collect();

    if pane.query_text[row] == indent {
        pane.query_cursor = (row, indent.chars().count());
        return false;
    }

    let removed = pane.query_text[row].clone();
    set_delete_register(pane, removed, true);

    pane.query_text[row] = indent.clone();
    pane.query_cursor = (row, indent.chars().count());
    true
}

fn open_line_below(pane: &mut Pane) -> bool {
    ensure_query_buffer(pane);

    let row = pane
        .query_cursor
        .0
        .min(pane.query_text.len().saturating_sub(1));
    let indent: String = pane.query_text[row]
        .chars()
        .take_while(|c| c.is_whitespace())
        .collect();

    pane.query_text.insert(row + 1, indent.clone());
    pane.query_cursor = (row + 1, indent.chars().count());
    true
}

fn open_line_above(pane: &mut Pane) -> bool {
    ensure_query_buffer(pane);

    let row = pane
        .query_cursor
        .0
        .min(pane.query_text.len().saturating_sub(1));
    let indent: String = pane.query_text[row]
        .chars()
        .take_while(|c| c.is_whitespace())
        .collect();

    pane.query_text.insert(row, indent.clone());
    pane.query_cursor = (row, indent.chars().count());
    true
}

// ═══════════════════════════════════════════════════════════════════════════════
// Operator motions (d/c + motion)
// ═══════════════════════════════════════════════════════════════════════════════

fn delete_to_line_end(pane: &mut Pane) -> bool {
    let start = QueryCursor::from(pane.query_cursor);
    let end = QueryCursor {
        row: start.row,
        col: current_line_char_len(pane),
    };
    delete_range(pane, start, end)
}

fn delete_to_line_start(pane: &mut Pane) -> bool {
    let cur = QueryCursor::from(pane.query_cursor);
    if cur.col == 0 {
        return false;
    }

    let start = QueryCursor {
        row: cur.row,
        col: 0,
    };
    let end = cursor_after_current_char(&pane.query_text, cur);
    delete_range(pane, start, end)
}

fn delete_to_first_non_blank(pane: &mut Pane) -> bool {
    let cur = QueryCursor::from(pane.query_cursor);
    let Some(line) = pane.query_text.get(cur.row) else {
        return false;
    };

    let first_non_blank = line.chars().take_while(|c| c.is_whitespace()).count();
    if first_non_blank >= cur.col {
        return false;
    }

    let start = QueryCursor {
        row: cur.row,
        col: first_non_blank,
    };
    let end = cursor_after_current_char(&pane.query_text, cur);
    delete_range(pane, start, end)
}

fn delete_word_forward(pane: &mut Pane, count: usize) -> bool {
    let start = QueryCursor::from(pane.query_cursor);
    let end = motion_target(pane, |p| move_word_forward(p, count));

    if end == start {
        return false;
    }

    delete_range(pane, start, end)
}

fn delete_word_end(pane: &mut Pane, count: usize) -> bool {
    let start = QueryCursor::from(pane.query_cursor);
    let end_inclusive = motion_target(pane, |p| move_word_end(p, count));
    let end = cursor_after_current_char(&pane.query_text, end_inclusive);

    if end == start {
        return false;
    }

    delete_range(pane, start, end)
}

fn delete_word_backward(pane: &mut Pane, count: usize) -> bool {
    let cur = QueryCursor::from(pane.query_cursor);
    let start = motion_target(pane, |p| move_word_back(p, count));
    let end = cursor_after_current_char(&pane.query_text, cur);

    if start == end {
        return false;
    }

    delete_range(pane, start, end)
}

fn delete_word_text_object(pane: &mut Pane, around: bool, count: usize) -> bool {
    let mut changed = false;
    for _ in 0..count {
        if !delete_single_word_text_object(pane, around) {
            break;
        }
        changed = true;
    }
    changed
}

fn delete_single_word_text_object(pane: &mut Pane, around: bool) -> bool {
    let row = pane.query_cursor.0;
    let Some(line) = pane.query_text.get(row) else {
        return false;
    };

    let chars: Vec<char> = line.chars().collect();
    if chars.is_empty() {
        return false;
    }

    let mut col = pane.query_cursor.1.min(chars.len().saturating_sub(1));
    if !is_word_char(chars[col]) {
        while col < chars.len() && !is_word_char(chars[col]) {
            col += 1;
        }
        if col >= chars.len() {
            return false;
        }
    }

    let mut start = col;
    while start > 0 && is_word_char(chars[start - 1]) {
        start -= 1;
    }

    let mut end = col;
    while end < chars.len() && is_word_char(chars[end]) {
        end += 1;
    }

    if around {
        let mut around_end = end;
        while around_end < chars.len() && chars[around_end].is_whitespace() {
            around_end += 1;
        }

        if around_end == end {
            let mut around_start = start;
            while around_start > 0 && chars[around_start - 1].is_whitespace() {
                around_start -= 1;
            }
            start = around_start;
        }

        end = around_end;
    }

    if start >= end {
        return false;
    }

    let start_cur = QueryCursor { row, col: start };
    let end_cur = QueryCursor { row, col: end };
    delete_range(pane, start_cur, end_cur)
}

fn motion_target<F>(pane: &mut Pane, motion: F) -> QueryCursor
where
    F: FnOnce(&mut Pane),
{
    let original = pane.query_cursor;
    motion(pane);
    let target = QueryCursor::from(pane.query_cursor);
    pane.query_cursor = original;
    target
}

fn delete_range(pane: &mut Pane, start: QueryCursor, end: QueryCursor) -> bool {
    ensure_query_buffer(pane);

    let (mut start, mut end) = order_range(start, end);
    let last_row = pane.query_text.len().saturating_sub(1);
    start.row = start.row.min(last_row);
    end.row = end.row.min(last_row);

    let start_line_len = pane
        .query_text
        .get(start.row)
        .map_or(0, |line| line_char_len(line));
    let end_line_len = pane
        .query_text
        .get(end.row)
        .map_or(0, |line| line_char_len(line));
    start.col = start.col.min(start_line_len);
    end.col = end.col.min(end_line_len);

    if start == end {
        return false;
    }

    let Some(removed_text) = collect_range_text(&pane.query_text, start, end) else {
        return false;
    };
    set_delete_register(pane, removed_text, false);

    if start.row == end.row {
        let Some(line) = pane.query_text.get_mut(start.row) else {
            return false;
        };

        let start_byte = char_idx_to_byte_idx(line, start.col);
        let end_byte = char_idx_to_byte_idx(line, end.col);
        if start_byte >= end_byte || end_byte > line.len() {
            return false;
        }

        line.replace_range(start_byte..end_byte, "");
    } else {
        let start_line = pane.query_text[start.row].clone();
        let end_line = pane.query_text[end.row].clone();

        let start_byte = char_idx_to_byte_idx(&start_line, start.col);
        let end_byte = char_idx_to_byte_idx(&end_line, end.col);
        if start_byte > start_line.len() || end_byte > end_line.len() {
            return false;
        }

        let merged = format!("{}{}", &start_line[..start_byte], &end_line[end_byte..]);
        pane.query_text[start.row] = merged;
        pane.query_text.drain((start.row + 1)..=end.row);
    }

    if pane.query_text.is_empty() {
        pane.query_text.push(String::new());
    }

    pane.query_cursor = (
        start.row.min(pane.query_text.len().saturating_sub(1)),
        start.col,
    );
    true
}

fn order_range(a: QueryCursor, b: QueryCursor) -> (QueryCursor, QueryCursor) {
    if (a.row, a.col) <= (b.row, b.col) {
        (a, b)
    } else {
        (b, a)
    }
}

fn cursor_after_current_char(lines: &[String], cur: QueryCursor) -> QueryCursor {
    let line_len = lines.get(cur.row).map_or(0, |line| line_char_len(line));

    if line_len == 0 {
        if cur.row + 1 < lines.len() {
            QueryCursor {
                row: cur.row + 1,
                col: 0,
            }
        } else {
            QueryCursor {
                row: cur.row,
                col: 0,
            }
        }
    } else if cur.col + 1 < line_len {
        QueryCursor {
            row: cur.row,
            col: cur.col + 1,
        }
    } else if cur.row + 1 < lines.len() {
        QueryCursor {
            row: cur.row + 1,
            col: 0,
        }
    } else {
        QueryCursor {
            row: cur.row,
            col: line_len,
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Character-position helpers
// ═══════════════════════════════════════════════════════════════════════════════

fn char_at_cursor(lines: &[String], cursor: QueryCursor) -> Option<char> {
    lines.get(cursor.row)?.chars().nth(cursor.col)
}

fn next_char_position(lines: &[String], cursor: QueryCursor) -> Option<QueryCursor> {
    let line_len = lines.get(cursor.row).map_or(0, |line| line_char_len(line));

    if line_len > 0 && cursor.col + 1 < line_len {
        return Some(QueryCursor {
            row: cursor.row,
            col: cursor.col + 1,
        });
    }

    let mut row = cursor.row + 1;
    while row < lines.len() {
        let len = line_char_len(&lines[row]);
        if len > 0 {
            return Some(QueryCursor { row, col: 0 });
        }
        row += 1;
    }

    None
}

fn prev_char_position(lines: &[String], cursor: QueryCursor) -> Option<QueryCursor> {
    if cursor.row >= lines.len() {
        return None;
    }

    if cursor.col > 0 {
        return Some(QueryCursor {
            row: cursor.row,
            col: cursor.col - 1,
        });
    }

    if cursor.row == 0 {
        return None;
    }

    let mut row = cursor.row - 1;
    loop {
        let len = line_char_len(&lines[row]);
        if len > 0 {
            return Some(QueryCursor { row, col: len - 1 });
        }
        if row == 0 {
            break;
        }
        row -= 1;
    }

    None
}

// ═══════════════════════════════════════════════════════════════════════════════
// Token helper
// ═══════════════════════════════════════════════════════════════════════════════

fn token_prefix(line: &str, col: usize) -> (usize, String) {
    let chars: Vec<char> = line.chars().collect();
    let mut start = col.min(chars.len());
    while start > 0 && !chars[start - 1].is_whitespace() {
        start -= 1;
    }

    let prefix: String = chars[start..col.min(chars.len())].iter().collect();
    (start, prefix)
}
