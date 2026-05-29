use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::helpers::{char_idx_to_byte_idx, get_table_prefix};
use crate::tui::{
    state::{
        pane_layout::{Pane, PaneType},
        TableMode,
    },
    AppState,
};

pub fn handle_query_editor(event: KeyEvent, state: &mut AppState, tables: &[String]) -> bool {
    let active_idx = state.active_tab;
    let is_query_editor = state
        .tabs
        .get(active_idx)
        .and_then(|tab| tab.tree.panes.get(&tab.tree.active_pane))
        .map_or(false, |pane| pane.kind == PaneType::QueryEditor);

    if !is_query_editor {
        return false;
    }

    // Query editor keeps its own pending keys (`gg`, `dd`) so dashboard-level
    // pending state should never leak while this pane is focused.
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
    } else {
        // Query editor only uses Normal/Insert semantics.
        if pane.mode != TableMode::Normal {
            pane.mode = TableMode::Normal;
        }
        handle_normal_mode(event, pane)
    }
}

fn handle_normal_mode(event: KeyEvent, pane: &mut Pane) -> bool {
    if handle_pending_normal_combo(event, pane) {
        clamp_cursor_for_mode(pane, false);
        return true;
    }

    if event.modifiers.contains(KeyModifiers::CONTROL) {
        if event.code == KeyCode::Char('r') {
            pane.query_redo();
            clamp_cursor_for_mode(pane, false);
            return true;
        }
        // Let dashboard-level Ctrl mappings handle pane navigation.
        return false;
    }

    let handled = match event.code {
        KeyCode::Esc => {
            pane.query_pending_key = None;
            close_autocomplete(pane);
            true
        }

        // Motions
        KeyCode::Char('h') | KeyCode::Left => {
            move_left(pane, 1);
            true
        }
        KeyCode::Char('l') | KeyCode::Right => {
            move_right(pane, 1, false);
            true
        }
        KeyCode::Char('j') | KeyCode::Down => {
            move_down(pane, 1, false);
            true
        }
        KeyCode::Char('k') | KeyCode::Up => {
            move_up(pane, 1, false);
            true
        }
        KeyCode::Char('w') => {
            move_word_forward(pane, 1);
            true
        }
        KeyCode::Char('b') => {
            move_word_back(pane, 1);
            true
        }
        KeyCode::Char('e') => {
            move_word_end(pane, 1);
            true
        }
        KeyCode::Char('0') | KeyCode::Home => {
            move_to_line_start(pane);
            true
        }
        KeyCode::Char('^') => {
            move_to_first_non_blank(pane);
            true
        }
        KeyCode::Char('$') | KeyCode::End => {
            move_to_line_end(pane, false);
            true
        }
        KeyCode::Char('G') => {
            move_to_bottom(pane);
            true
        }
        KeyCode::Char('g') => {
            pane.query_pending_key = Some('g');
            true
        }

        // Editing
        KeyCode::Char('u') => {
            pane.query_undo();
            true
        }
        KeyCode::Char('x') | KeyCode::Delete => {
            pane.push_query_snapshot();
            delete_char_at_cursor(pane, false);
            true
        }
        KeyCode::Char('d') => {
            pane.query_pending_key = Some('d');
            true
        }
        KeyCode::Char('s') => {
            pane.push_query_snapshot();
            delete_char_at_cursor(pane, false);
            pane.mode = TableMode::Insert;
            close_autocomplete(pane);
            true
        }

        // Insert entry points
        KeyCode::Char('i') => {
            pane.mode = TableMode::Insert;
            close_autocomplete(pane);
            true
        }
        KeyCode::Char('a') => {
            move_right(pane, 1, true);
            pane.mode = TableMode::Insert;
            close_autocomplete(pane);
            true
        }
        KeyCode::Char('I') => {
            move_to_first_non_blank(pane);
            pane.mode = TableMode::Insert;
            close_autocomplete(pane);
            true
        }
        KeyCode::Char('A') => {
            move_to_line_end(pane, true);
            pane.mode = TableMode::Insert;
            close_autocomplete(pane);
            true
        }
        KeyCode::Char('o') => {
            pane.push_query_snapshot();
            open_line_below(pane);
            pane.mode = TableMode::Insert;
            close_autocomplete(pane);
            true
        }
        KeyCode::Char('O') => {
            pane.push_query_snapshot();
            open_line_above(pane);
            pane.mode = TableMode::Insert;
            close_autocomplete(pane);
            true
        }

        _ => false,
    };

    if handled {
        clamp_cursor_for_mode(pane, pane.mode == TableMode::Insert);
    }

    handled
}

fn handle_pending_normal_combo(event: KeyEvent, pane: &mut Pane) -> bool {
    let Some(pending) = pane.query_pending_key.take() else {
        return false;
    };

    match (pending, event.code) {
        ('g', KeyCode::Char('g')) => {
            move_to_top(pane);
            true
        }
        ('d', KeyCode::Char('d')) => {
            pane.push_query_snapshot();
            delete_current_line(pane);
            true
        }
        _ => false,
    }
}

fn handle_insert_mode(event: KeyEvent, pane: &mut Pane, tables: &[String]) -> bool {
    if event.code == KeyCode::Esc {
        pane.mode = TableMode::Normal;
        pane.query_pending_key = None;
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
                pane.push_query_snapshot();
                apply_autocomplete_selection(pane);
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
                pane.push_query_snapshot();
                insert_char(pane, c);
                changed_text = true;
            }
            KeyCode::Enter => {
                pane.push_query_snapshot();
                insert_newline(pane);
                changed_text = true;
            }
            KeyCode::Backspace => {
                pane.push_query_snapshot();
                backspace(pane);
                changed_text = true;
            }
            KeyCode::Delete => {
                pane.push_query_snapshot();
                delete_char_at_cursor(pane, true);
                changed_text = true;
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

fn apply_autocomplete_selection(pane: &mut Pane) {
    let Some(selected) = pane.autocomplete_selected else {
        return;
    };
    let Some(replacement) = pane.autocomplete_matches.get(selected).cloned() else {
        return;
    };

    let quoted = if replacement.chars().any(|c| c.is_uppercase()) {
        format!("\"{}\"", replacement)
    } else {
        replacement
    };

    let (row, col) = pane.query_cursor;
    let Some(line) = pane.query_text.get_mut(row) else {
        return;
    };

    let (start_col, _) = token_prefix(line, col);
    let start_byte = char_idx_to_byte_idx(line, start_col);
    let end_byte = char_idx_to_byte_idx(line, col);

    line.replace_range(start_byte..end_byte, &quoted);
    pane.query_cursor = (row, start_col + quoted.chars().count());
}

fn line_char_len(line: &str) -> usize {
    line.chars().count()
}

fn current_line_char_len(pane: &Pane) -> usize {
    pane.query_text
        .get(pane.query_cursor.0)
        .map_or(0, |line| line_char_len(line))
}

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

fn insert_char(pane: &mut Pane, c: char) {
    ensure_query_buffer(pane);
    let (row, col) = pane.query_cursor;

    if let Some(line) = pane.query_text.get_mut(row) {
        let byte_col = char_idx_to_byte_idx(line, col);
        line.insert(byte_col, c);
        pane.query_cursor = (row, col + 1);
    }
}

fn insert_newline(pane: &mut Pane) {
    ensure_query_buffer(pane);
    let (row, col) = pane.query_cursor;

    let Some(line) = pane.query_text.get_mut(row) else {
        return;
    };

    let byte_col = char_idx_to_byte_idx(line, col);
    let indent: String = line.chars().take_while(|c| c.is_whitespace()).collect();
    let rest = line[byte_col..].to_string();
    line.truncate(byte_col);

    pane.query_text
        .insert(row + 1, format!("{}{}", indent, rest));
    pane.query_cursor = (row + 1, indent.chars().count());
}

fn backspace(pane: &mut Pane) {
    ensure_query_buffer(pane);
    let (row, col) = pane.query_cursor;

    if col > 0 {
        if let Some(line) = pane.query_text.get_mut(row) {
            let start = char_idx_to_byte_idx(line, col - 1);
            let end = char_idx_to_byte_idx(line, col);
            if start < end && end <= line.len() {
                line.replace_range(start..end, "");
                pane.query_cursor = (row, col - 1);
            }
        }
        return;
    }

    if row == 0 {
        return;
    }

    let current = pane.query_text.remove(row);
    let prev_row = row - 1;
    let prev_len = line_char_len(&pane.query_text[prev_row]);
    pane.query_text[prev_row].push_str(&current);
    pane.query_cursor = (prev_row, prev_len);
}

fn delete_char_at_cursor(pane: &mut Pane, join_on_eol: bool) {
    ensure_query_buffer(pane);
    let (row, col) = pane.query_cursor;

    let Some(line_len) = pane.query_text.get(row).map(|line| line_char_len(line)) else {
        return;
    };

    if col < line_len {
        if let Some(line) = pane.query_text.get_mut(row) {
            let start = char_idx_to_byte_idx(line, col);
            let end = char_idx_to_byte_idx(line, col + 1);
            if start < end && end <= line.len() {
                line.replace_range(start..end, "");
            }
        }
        return;
    }

    if join_on_eol && row + 1 < pane.query_text.len() {
        let next = pane.query_text.remove(row + 1);
        if let Some(line) = pane.query_text.get_mut(row) {
            line.push_str(&next);
        }
    }
}

fn delete_current_line(pane: &mut Pane) {
    ensure_query_buffer(pane);

    if pane.query_text.len() == 1 {
        pane.query_text[0].clear();
        pane.query_cursor = (0, 0);
        return;
    }

    let row = pane
        .query_cursor
        .0
        .min(pane.query_text.len().saturating_sub(1));
    pane.query_text.remove(row);

    let new_row = row.min(pane.query_text.len().saturating_sub(1));
    pane.query_cursor.0 = new_row;
    clamp_cursor_for_mode(pane, false);
}

fn open_line_below(pane: &mut Pane) {
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
}

fn open_line_above(pane: &mut Pane) {
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
}

fn token_prefix(line: &str, col: usize) -> (usize, String) {
    let chars: Vec<char> = line.chars().collect();
    let mut start = col.min(chars.len());
    while start > 0 && !chars[start - 1].is_whitespace() {
        start -= 1;
    }

    let prefix: String = chars[start..col.min(chars.len())].iter().collect();
    (start, prefix)
}
