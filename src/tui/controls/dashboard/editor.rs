use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui_textarea::{Input, TextArea};

use super::helpers::{completion_prefix, get_table_prefix, restore_cursor};
use crate::tui::{AppState, state::TableMode, state::pane_layout::PaneType};

pub fn handle_insert_mode(event: KeyEvent, state: &mut AppState, tables: &[String]) -> bool {
    let active_idx = state.active_tab;
    let Some(tab) = state.tabs.get_mut(active_idx) else {
        return false;
    };

    let active_id = tab.tree.active_pane;
    let is_insert = tab.tree.panes.get(&active_id).map_or(false, |p| {
        p.kind == PaneType::QueryEditor && p.mode == TableMode::Insert
    });

    if !is_insert {
        return false;
    }

    if event.code == KeyCode::Esc {
        if let Some(pane) = tab.tree.panes.get_mut(&active_id) {
            pane.mode = TableMode::Normal;
            pane.autocomplete_selected = None;
            pane.autocomplete_matches.clear();
        }
        return true;
    }

    let popup_open = tab
        .tree
        .panes
        .get(&active_id)
        .map_or(false, |p| p.autocomplete_selected.is_some());

    if popup_open {
        match event.code {
            KeyCode::Up => {
                if let Some(pane) = tab.tree.panes.get_mut(&active_id) {
                    if let Some(sel) = pane.autocomplete_selected {
                        pane.autocomplete_selected = Some(sel.saturating_sub(1));
                    }
                }
                return true;
            }
            KeyCode::Down => {
                if let Some(pane) = tab.tree.panes.get_mut(&active_id) {
                    if let Some(sel) = pane.autocomplete_selected {
                        let max = pane.autocomplete_matches.len().saturating_sub(1);
                        pane.autocomplete_selected = Some((sel + 1).min(max));
                    }
                }
                return true;
            }
            KeyCode::Tab => {
                let shift = event.modifiers.contains(KeyModifiers::SHIFT);
                if let Some(pane) = tab.tree.panes.get_mut(&active_id) {
                    if let Some(sel) = pane.autocomplete_selected {
                        let len = pane.autocomplete_matches.len();
                        if shift {
                            pane.autocomplete_selected = Some((sel + len - 1) % len);
                        } else {
                            pane.autocomplete_selected = Some((sel + 1) % len);
                        }
                    }
                }
                return true;
            }
            KeyCode::Enter => {
                if let Some(pane) = tab.tree.panes.get_mut(&active_id) {
                    if let Some(sel) = pane.autocomplete_selected {
                        if let Some(replacement) = pane.autocomplete_matches.get(sel).cloned() {
                            let (row, col) = pane.query_cursor;
                            if let Some(line) = pane.query_text.get_mut(row) {
                                let (start, byte_end) = {
                                    let line_ref: &str = line;
                                    let s = completion_prefix(line_ref, col).0;
                                    let e = super::helpers::char_idx_to_byte_idx(line_ref, col);
                                    (s, e)
                                };
                                let quoted = if replacement.chars().any(|c| c.is_uppercase()) {
                                    format!("\"{}\"", replacement)
                                } else {
                                    replacement
                                };
                                line.replace_range(start..byte_end, &quoted);
                                pane.query_cursor = (row, start + quoted.chars().count());
                            }
                        }
                    }
                    pane.autocomplete_selected = None;
                    pane.autocomplete_matches.clear();
                }
                return true;
            }
            KeyCode::Esc => {
                if let Some(pane) = tab.tree.panes.get_mut(&active_id) {
                    pane.autocomplete_selected = None;
                    pane.autocomplete_matches.clear();
                }
                return true;
            }
            KeyCode::Char(' ') => {
                if let Some(pane) = tab.tree.panes.get_mut(&active_id) {
                    pane.autocomplete_selected = None;
                    pane.autocomplete_matches.clear();
                }
                // fall through to feed space to textarea
            }
            _ => {
                // Fall through to textarea input; popup stays open
                // and will be updated by the auto-trigger check below.
            }
        }
    } else if event.code == KeyCode::Tab {
        if let Some(pane) = tab.tree.panes.get_mut(&active_id) {
            let (row, col) = pane.query_cursor;
            if let Some(line) = pane.query_text.get(row) {
                let (_, prefix) = completion_prefix(line, col);
                if !prefix.is_empty() {
                    let matches: Vec<String> = tables
                        .iter()
                        .filter(|t| t.to_lowercase().starts_with(&prefix.to_lowercase()))
                        .cloned()
                        .collect();
                    if !matches.is_empty() {
                        pane.autocomplete_matches = matches;
                        pane.autocomplete_selected = Some(0);
                    }
                }
            }
        }
        return true;
    }

    // Close popup on any other key when it wasn't already handled above.
    if !popup_open {
        if let Some(pane) = tab.tree.panes.get_mut(&active_id) {
            pane.autocomplete_selected = None;
            pane.autocomplete_matches.clear();
        }
    }

    // Feed the key event into the textarea.
    let (query_text, query_cursor) = tab
        .tree
        .panes
        .get(&active_id)
        .map(|p| (p.query_text.clone(), p.query_cursor))
        .unwrap_or_default();
    let mut textarea = TextArea::new(query_text);
    restore_cursor(&mut textarea, query_cursor);
    textarea.input(Input::from(event));
    let cursor = textarea.cursor();
    let lines: Vec<String> = textarea.lines().iter().map(|s| s.to_string()).collect();

    if let Some(pane) = tab.tree.panes.get_mut(&active_id) {
        pane.query_text = lines;
        pane.query_cursor = (cursor.0, cursor.1);

        // Auto-trigger: show table completions after FROM / JOIN / INTO / UPDATE / TABLE.
        let (row, col) = pane.query_cursor;
        if let Some(line) = pane.query_text.get(row) {
            if let Some(prefix) = get_table_prefix(line, col) {
                let matches: Vec<String> = tables
                    .iter()
                    .filter(|t| t.to_lowercase().starts_with(&prefix.to_lowercase()))
                    .cloned()
                    .collect();
                if !matches.is_empty() {
                    pane.autocomplete_matches = matches;
                    pane.autocomplete_selected = Some(0);
                } else {
                    pane.autocomplete_selected = None;
                    pane.autocomplete_matches.clear();
                }
            } else {
                pane.autocomplete_selected = None;
                pane.autocomplete_matches.clear();
            }
        }
    }
    true
}
