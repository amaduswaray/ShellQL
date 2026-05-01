use shellql::{
    connection::models::{ConnectionSource, Database, DatabaseString, Engine},
    tui::{
        state::{AppMode, AppState, CommandLine},
        ui::home::{
            goto_bottom, goto_top, remove_selected, select_next, select_prev, visible_text,
        },
    },
};

// ── Helpers ───────────────────────────────────────────────────────────────────

fn mock_db(name: &str) -> Database {
    Database {
        name: name.to_string(),
        engine: Engine::Postgres,
        connection: ConnectionSource::Url(DatabaseString::Postgres(
            format!("postgres://localhost/{name}"),
        )),
    }
}

fn state_with(names: &[&str]) -> AppState {
    AppState {
        mode: AppMode::Home,
        overlay: None,
        should_quit: false,
        connections: names.iter().map(|n| mock_db(n)).collect(),
        selected_connection: 0,
        sessions: vec![],
        active_session: 0,
        pending_key: None,
        cmdline: CommandLine::new(),
        form: None,
    }
}

// ── select_next ───────────────────────────────────────────────────────────────

#[test]
fn select_next_advances_cursor() {
    let mut s = state_with(&["a", "b", "c"]);
    select_next(&mut s);
    assert_eq!(s.selected_connection, 1);
}

#[test]
fn select_next_wraps_from_last_to_first() {
    let mut s = state_with(&["a", "b", "c"]);
    s.selected_connection = 2;
    select_next(&mut s);
    assert_eq!(s.selected_connection, 0);
}

#[test]
fn select_next_on_empty_list_is_noop() {
    let mut s = state_with(&[]);
    select_next(&mut s);
    assert_eq!(s.selected_connection, 0);
}

// ── select_prev ───────────────────────────────────────────────────────────────

#[test]
fn select_prev_retreats_cursor() {
    let mut s = state_with(&["a", "b", "c"]);
    s.selected_connection = 2;
    select_prev(&mut s);
    assert_eq!(s.selected_connection, 1);
}

#[test]
fn select_prev_wraps_from_first_to_last() {
    let mut s = state_with(&["a", "b", "c"]);
    select_prev(&mut s);
    assert_eq!(s.selected_connection, 2);
}

// ── goto_top / goto_bottom ────────────────────────────────────────────────────

#[test]
fn goto_top_sets_selection_to_zero() {
    let mut s = state_with(&["a", "b", "c"]);
    s.selected_connection = 2;
    goto_top(&mut s);
    assert_eq!(s.selected_connection, 0);
}

#[test]
fn goto_bottom_sets_selection_to_last() {
    let mut s = state_with(&["a", "b", "c"]);
    goto_bottom(&mut s);
    assert_eq!(s.selected_connection, 2);
}

#[test]
fn goto_bottom_on_empty_list_is_noop() {
    let mut s = state_with(&[]);
    goto_bottom(&mut s);
    assert_eq!(s.selected_connection, 0);
}

// ── remove_selected ───────────────────────────────────────────────────────────

#[test]
fn remove_selected_removes_the_focused_connection() {
    let mut s = state_with(&["a", "b", "c"]);
    s.selected_connection = 1;
    remove_selected(&mut s);
    assert_eq!(s.connections.len(), 2);
    assert_eq!(s.connections[0].name, "a");
    assert_eq!(s.connections[1].name, "c");
}

#[test]
fn remove_selected_clamps_cursor_after_removing_last() {
    let mut s = state_with(&["a", "b", "c"]);
    s.selected_connection = 2;
    remove_selected(&mut s);
    assert_eq!(s.connections.len(), 2);
    assert_eq!(s.selected_connection, 1);
}

#[test]
fn remove_selected_cursor_stays_when_not_at_end() {
    let mut s = state_with(&["a", "b", "c"]);
    s.selected_connection = 0;
    remove_selected(&mut s);
    assert_eq!(s.selected_connection, 0);
    assert_eq!(s.connections[0].name, "b");
}

#[test]
fn remove_selected_on_empty_list_is_noop() {
    let mut s = state_with(&[]);
    remove_selected(&mut s);
    assert_eq!(s.connections.len(), 0);
    assert_eq!(s.selected_connection, 0);
}

// ── visible_text (horizontal scroll) ─────────────────────────────────────────

#[test]
fn visible_text_no_scroll_returns_full_string() {
    assert_eq!(visible_text("hello", 0, 10), "hello");
}

#[test]
fn visible_text_clips_to_width() {
    assert_eq!(visible_text("hello world", 0, 5), "hello");
}

#[test]
fn visible_text_scroll_shifts_window() {
    assert_eq!(visible_text("hello", 2, 3), "llo");
}

#[test]
fn visible_text_scroll_past_end_returns_empty() {
    assert_eq!(visible_text("hi", 10, 5), "");
}

#[test]
fn visible_text_handles_unicode() {
    assert_eq!(visible_text("äöü", 1, 2), "öü");
}

#[test]
fn scroll_offset_keeps_cursor_in_view() {
    let scroll = |cursor: usize, width: usize| -> usize {
        if cursor >= width { cursor + 1 - width } else { 0 }
    };
    assert_eq!(scroll(0, 10), 0);
    assert_eq!(scroll(9, 10), 0);
    assert_eq!(scroll(10, 10), 1);
    assert_eq!(scroll(15, 10), 6);
}
