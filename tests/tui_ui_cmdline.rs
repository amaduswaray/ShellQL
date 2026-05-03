use ratatui::{Terminal, backend::TestBackend};
use shellql::tui::{
    state::{AppMode, AppState, CommandLine, ConfirmAction},
    ui::render_cmdline,
};

// ── Helpers ───────────────────────────────────────────────────────────────────

const WIDTH: u16 = 40;

fn empty_state() -> AppState {
    AppState {
        mode: AppMode::Home,
        overlay: None,
        should_quit: false,
        connections: vec![],
        selected_connection: 0,
        sessions: vec![],
        active_session: 0,
        pending_key: None,
        cmdline: CommandLine::new(),
        form: None,
        dashboard: None,
    }
}

/// Draw the cmdline bar into a single-row terminal and return the raw text.
fn render(state: &AppState) -> String {
    let backend = TestBackend::new(WIDTH, 1);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal
        .draw(|frame| render_cmdline(frame, frame.area(), state))
        .unwrap();
    terminal
        .backend()
        .buffer()
        .content()
        .iter()
        .map(|c| c.symbol())
        .collect()
}

// ── Idle ──────────────────────────────────────────────────────────────────────

#[test]
fn idle_home_shows_mode_pill() {
    let output = render(&empty_state());
    assert!(output.contains(""), "expected <blank> pill in: {output:?}");
}

// ── Error ─────────────────────────────────────────────────────────────────────

#[test]
fn error_state_shows_error_text() {
    let mut state = empty_state();
    state.cmdline.set_error("E: not a command: foo");
    let output = render(&state);
    assert!(
        output.contains("E: not a command: foo"),
        "expected error in: {output:?}"
    );
}

#[test]
fn error_state_does_not_show_mode_pill() {
    let mut state = empty_state();
    state.cmdline.set_error("something failed");
    let output = render(&state);
    assert!(
        !output.contains("HOME"),
        "mode pill should be hidden when error shows: {output:?}"
    );
}

// ── Input ─────────────────────────────────────────────────────────────────────

#[test]
fn input_mode_shows_colon_prefix() {
    let mut state = empty_state();
    state.cmdline.open_input();
    state.cmdline.push('a');
    state.cmdline.push('d');
    state.cmdline.push('d');
    let output = render(&state);
    assert!(
        output.starts_with(':'),
        "expected colon prefix in: {output:?}"
    );
    assert!(
        output.contains("add"),
        "expected typed input in: {output:?}"
    );
}

#[test]
fn input_mode_empty_buffer_shows_just_colon() {
    let mut state = empty_state();
    state.cmdline.open_input();
    let output = render(&state);
    assert!(output.starts_with(':'), "expected colon: {output:?}");
}

// ── Confirm delete ────────────────────────────────────────────────────────────

#[test]
fn confirm_delete_shows_connection_name() {
    let mut state = empty_state();
    state
        .cmdline
        .open_confirm(ConfirmAction::DeleteConnection("prod-postgres".to_string()));
    let output = render(&state);
    assert!(
        output.contains("prod-postgres"),
        "expected name in: {output:?}"
    );
}

#[test]
fn confirm_delete_shows_y_n_prompt() {
    let mut state = empty_state();
    state
        .cmdline
        .open_confirm(ConfirmAction::DeleteConnection("db".to_string()));
    let output = render(&state);
    assert!(output.contains("[y/n]"), "expected prompt in: {output:?}");
}

#[test]
fn confirm_delete_shows_typed_answer() {
    let mut state = empty_state();
    state
        .cmdline
        .open_confirm(ConfirmAction::DeleteConnection("db".to_string()));
    state.cmdline.push('y');
    let output = render(&state);
    assert!(output.contains('y'), "expected typed answer in: {output:?}");
}
