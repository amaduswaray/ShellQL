use shellql::tui::state::{
    CommandLine, CommandLineMode, ConfirmAction, COMMANDS, compute_completions,
};

// ── compute_completions ───────────────────────────────────────────────────────

#[test]
fn empty_input_returns_all_commands() {
    let results = compute_completions("");
    assert_eq!(results.len(), COMMANDS.len());
}

#[test]
fn prefix_filter_returns_only_matching_commands() {
    let results = compute_completions("d");
    assert!(!results.is_empty());
    assert!(results.iter().all(|(cmd, _)| cmd.starts_with('d')));
    assert!(results.iter().any(|(cmd, _)| *cmd == "d"));
    assert!(results.iter().any(|(cmd, _)| *cmd == "delete"));
    assert!(!results.iter().any(|(cmd, _)| *cmd == "q"));
}

#[test]
fn no_prefix_match_returns_empty() {
    let results = compute_completions("zzz");
    assert!(results.is_empty());
}

#[test]
fn full_command_name_returns_single_exact_match() {
    let results = compute_completions("quit");
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].0, "quit");
}

// ── open_completions ──────────────────────────────────────────────────────────

#[test]
fn open_completions_fills_input_with_first_entry() {
    let mut cl = CommandLine::new();
    cl.open_input();
    let matches = compute_completions("q");
    let first_cmd = matches[0].0;
    cl.open_completions(matches);
    assert_eq!(cl.input, first_cmd);
    assert_eq!(cl.completion_selected, Some(0));
}

#[test]
fn open_completions_with_empty_list_does_nothing() {
    let mut cl = CommandLine::new();
    cl.open_input();
    cl.input = "existing".to_string();
    cl.open_completions(vec![]);
    assert_eq!(cl.input, "existing");
    assert!(cl.completions.is_empty());
}

// ── next / prev completion ────────────────────────────────────────────────────

#[test]
fn next_completion_advances_selection_and_updates_input() {
    let mut cl = CommandLine::new();
    cl.open_input();
    cl.open_completions(compute_completions("q"));
    let second = cl.completions[1].0;
    cl.next_completion();
    assert_eq!(cl.completion_selected, Some(1));
    assert_eq!(cl.input, second);
}

#[test]
fn next_completion_wraps_around_to_first() {
    let mut cl = CommandLine::new();
    cl.open_input();
    let matches = compute_completions("q");
    let n = matches.len();
    let first = matches[0].0;
    cl.open_completions(matches);
    for _ in 0..n {
        cl.next_completion();
    }
    assert_eq!(cl.completion_selected, Some(0));
    assert_eq!(cl.input, first);
}

#[test]
fn prev_completion_wraps_around_to_last() {
    let mut cl = CommandLine::new();
    cl.open_input();
    let matches = compute_completions("q");
    let last = matches[matches.len() - 1].0;
    cl.open_completions(matches);
    cl.prev_completion();
    assert_eq!(cl.input, last);
}

// ── clear_completions / reset ─────────────────────────────────────────────────

#[test]
fn clear_completions_empties_list_and_selection() {
    let mut cl = CommandLine::new();
    cl.open_input();
    cl.open_completions(compute_completions(""));
    assert!(!cl.completions.is_empty());
    cl.clear_completions();
    assert!(cl.completions.is_empty());
    assert_eq!(cl.completion_selected, None);
}

#[test]
fn reset_clears_completions_and_returns_to_idle() {
    let mut cl = CommandLine::new();
    cl.open_input();
    cl.open_completions(compute_completions(""));
    cl.reset();
    assert!(cl.completions.is_empty());
    assert_eq!(cl.completion_selected, None);
    assert_eq!(cl.mode, CommandLineMode::Idle);
    assert!(cl.input.is_empty());
}

// ── is_active ─────────────────────────────────────────────────────────────────

#[test]
fn is_active_false_when_idle() {
    assert!(!CommandLine::new().is_active());
}

#[test]
fn is_active_true_in_input_mode() {
    let mut cl = CommandLine::new();
    cl.open_input();
    assert!(cl.is_active());
}

#[test]
fn is_active_true_in_confirm_mode() {
    let mut cl = CommandLine::new();
    cl.open_confirm(ConfirmAction::DeleteConnection("test".to_string()));
    assert!(cl.is_active());
}

// ── error ─────────────────────────────────────────────────────────────────────

#[test]
fn error_set_and_clear() {
    let mut cl = CommandLine::new();
    assert!(cl.error.is_none());
    cl.set_error("something went wrong");
    assert_eq!(cl.error.as_deref(), Some("something went wrong"));
    cl.clear_error();
    assert!(cl.error.is_none());
}

#[test]
fn reset_preserves_error_for_one_render_pass() {
    let mut cl = CommandLine::new();
    cl.open_input();
    cl.set_error("E: bad command");
    cl.reset();
    assert_eq!(cl.mode, CommandLineMode::Idle);
    assert_eq!(cl.error.as_deref(), Some("E: bad command"));
}
