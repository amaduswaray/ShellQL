use ratatui_textarea::TextArea;

/// Return (headers, rows, schema) for a TableView or QueryResults pane.
pub fn pane_data<'a>(
    table_cache: &'a std::collections::HashMap<String, crate::tui::state::tab::LoadedTable>,
    query_results: &'a [crate::tui::state::tab::QueryResult],
    pane: &crate::tui::state::pane_layout::Pane,
) -> Option<(
    Vec<String>,
    &'a Vec<Vec<String>>,
    Vec<crate::connection::ColumnInfo>,
)> {
    use crate::tui::state::pane_layout::PaneType;
    match pane.kind {
        PaneType::TableView => {
            let name = pane.bound_table.as_ref()?;
            let loaded = table_cache.get(name)?;
            Some((loaded.headers.clone(), &loaded.rows, loaded.schema.clone()))
        }
        PaneType::QueryResults => {
            let idx = pane.bound_query_idx?;
            let qr = query_results.get(idx)?;
            let schema: Vec<crate::connection::ColumnInfo> = qr
                .headers
                .iter()
                .enumerate()
                .map(|(i, name)| crate::connection::ColumnInfo {
                    name: name.clone(),
                    data_type: "TEXT".to_string(),
                    nullable: true,
                    is_primary_key: i == 0,
                    default_value: None,
                })
                .collect();
            Some((qr.headers.clone(), &qr.rows, schema))
        }
        _ => None,
    }
}

/// Restore TextArea cursor position from stored (row, col).
pub fn restore_cursor(textarea: &mut TextArea, (target_row, target_col): (usize, usize)) {
    use ratatui_textarea::CursorMove;
    // Move to top-left first.
    textarea.move_cursor(CursorMove::Top);
    textarea.move_cursor(CursorMove::Head);
    // Move down to target row.
    for _ in 0..target_row {
        textarea.move_cursor(CursorMove::Down);
    }
    // Move right to target col.
    for _ in 0..target_col {
        textarea.move_cursor(CursorMove::Forward);
    }
}

/// Convert a character (Unicode scalar) index into a byte index in `s`.
/// If `char_idx` is larger than the number of chars, returns `s.len()`.
pub fn char_idx_to_byte_idx(s: &str, char_idx: usize) -> usize {
    let mut byte_idx = 0;
    for (i, ch) in s.chars().enumerate() {
        if i == char_idx {
            return byte_idx;
        }
        byte_idx += ch.len_utf8();
    }
    byte_idx
}

/// Extract the current table-name prefix if the cursor is positioned after a
/// trigger keyword (`FROM`, `JOIN`, `INTO`, `UPDATE`, `TABLE`).
pub fn get_table_prefix(line: &str, col: usize) -> Option<String> {
    let byte_col = char_idx_to_byte_idx(line, col);
    let before = &line[..byte_col.min(line.len())];

    // Find the last space before the cursor.
    let last_space = before.rfind(' ')?;
    let prefix = &before[last_space + 1..];

    // Don't trigger on multiple consecutive spaces.
    let before_prefix = &before[..last_space];
    let trailing_spaces = before_prefix
        .len()
        .saturating_sub(before_prefix.trim_end().len());
    if trailing_spaces > 0 {
        return None;
    }

    let words: Vec<&str> = before_prefix.split_whitespace().collect();
    let prev_word = words.last().map(|w| w.to_lowercase())?;

    let triggers = ["from", "join", "into", "update", "table"];
    if triggers.contains(&prev_word.as_str()) {
        Some(prefix.to_string())
    } else {
        None
    }
}

/// Compute the replacement range for an autocomplete insertion.
/// Returns `(start_col, prefix)` where `prefix` is the text to replace.
pub fn completion_prefix(line: &str, col: usize) -> (usize, &str) {
    let byte_col = char_idx_to_byte_idx(line, col);
    let before = &line[..byte_col.min(line.len())];
    let start = before.rfind(' ').map(|i| i + 1).unwrap_or(0);
    (start, &before[start..])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_table_prefix_from_empty() {
        assert_eq!(get_table_prefix("SELECT * FROM ", 14), Some("".to_string()));
    }

    #[test]
    fn test_get_table_prefix_from_partial() {
        assert_eq!(
            get_table_prefix("SELECT * FROM u", 15),
            Some("u".to_string())
        );
    }

    #[test]
    fn test_get_table_prefix_join() {
        assert_eq!(
            get_table_prefix("SELECT * FROM users JOIN ", 25),
            Some("".to_string())
        );
    }

    #[test]
    fn test_get_table_prefix_update() {
        assert_eq!(get_table_prefix("UPDATE us", 9), Some("us".to_string()));
    }

    #[test]
    fn test_get_table_prefix_no_trigger() {
        assert_eq!(get_table_prefix("SELECT * WHERE id = 1", 21), None);
    }

    #[test]
    fn test_get_table_prefix_double_space_no_trigger() {
        assert_eq!(get_table_prefix("SELECT * FROM  ", 15), None);
    }

    #[test]
    fn test_get_table_prefix_after_table_name() {
        // After "users " the prev_word is "users" which is not a trigger
        assert_eq!(get_table_prefix("SELECT * FROM users ", 20), None);
    }

    #[test]
    fn test_completion_prefix_after_space() {
        // Cursor after "FROM " → prefix is empty, start at cursor
        assert_eq!(completion_prefix("SELECT * FROM ", 14), (14, ""));
    }

    #[test]
    fn test_completion_prefix_partial() {
        // Cursor after "FROM u" → prefix is "u"
        assert_eq!(completion_prefix("SELECT * FROM u", 15), (14, "u"));
    }

    #[test]
    fn test_completion_prefix_start_of_line() {
        // No space before cursor
        assert_eq!(completion_prefix("users", 5), (0, "users"));
    }

    #[test]
    fn test_char_idx_to_byte_idx_ascii() {
        assert_eq!(char_idx_to_byte_idx("hello", 0), 0);
        assert_eq!(char_idx_to_byte_idx("hello", 2), 2);
        assert_eq!(char_idx_to_byte_idx("hello", 5), 5);
    }

    #[test]
    fn test_char_idx_to_byte_idx_multibyte() {
        // 'æ' is 2 bytes in UTF-8
        assert_eq!(char_idx_to_byte_idx("æ", 0), 0);
        assert_eq!(char_idx_to_byte_idx("æ", 1), 2);
        assert_eq!(char_idx_to_byte_idx("æø", 1), 2);
        assert_eq!(char_idx_to_byte_idx("æø", 2), 4);
    }

    #[test]
    fn test_get_table_prefix_multibyte() {
        // After multi-byte char before trigger: "æ FROM " → should still work
        assert_eq!(get_table_prefix("æ FROM ", 7), Some("".to_string()));
    }

    #[test]
    fn test_completion_prefix_multibyte() {
        // "æ FROM " = 8 bytes, 7 chars. Cursor at char 7 (past end).
        // rfind(' ') finds space at byte 7, so start = 8.
        assert_eq!(completion_prefix("æ FROM ", 7), (8, ""));
    }
}
