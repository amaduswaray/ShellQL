/// Command-line bar state — vim-style `:` input and inline y/n confirmations.

// ── Available commands ────────────────────────────────────────────────────────

/// Every command the `:` prompt accepts, paired with a short description.
/// This drives both execution and tab-completion.
pub const COMMANDS: &[(&str, &str)] = &[
    ("q",      "quit"),
    ("quit",   "quit"),
    ("q!",     "force quit"),
    ("h",      "help overlay"),
    ("help",   "help overlay"),
    ("add",    "add connection"),
    ("d",       "delete selected"),
    ("delete",  "delete selected"),
    ("connect", "open connection picker"),
];

/// Return every entry in `COMMANDS` whose name starts with `input`.
pub fn compute_completions(input: &str) -> Vec<(&'static str, &'static str)> {
    COMMANDS
        .iter()
        .filter(|(cmd, _)| cmd.starts_with(input))
        .copied()
        .collect()
}

// ── Mode ──────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommandLineMode {
    /// Showing status; no input focus.
    Idle,
    /// The `:` prompt is open and the user is typing a command.
    Input,
    /// Awaiting a `y` / `n` answer before executing a destructive action.
    Confirm(ConfirmAction),
}

/// Which action is waiting for confirmation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfirmAction {
    DeleteConnection(String),
}

// ── State ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct CommandLine {
    pub mode: CommandLineMode,
    /// Current text the user has typed.
    pub input: String,
    /// One-shot error message shown in idle mode after a failed command.
    pub error: Option<String>,
    /// Active completion candidates `(command, description)`.
    pub completions: Vec<(&'static str, &'static str)>,
    /// Which entry in `completions` is currently highlighted.
    pub completion_selected: Option<usize>,
}

impl CommandLine {
    pub fn new() -> Self {
        Self {
            mode: CommandLineMode::Idle,
            input: String::new(),
            error: None,
            completions: Vec::new(),
            completion_selected: None,
        }
    }

    // ── Focus / mode ──────────────────────────────────────────────────────────

    /// Whether the command line has keyboard focus.
    pub fn is_active(&self) -> bool {
        self.mode != CommandLineMode::Idle
    }

    /// Open the `:` input prompt.
    pub fn open_input(&mut self) {
        self.mode = CommandLineMode::Input;
        self.input.clear();
        self.error = None;
        self.clear_completions();
    }

    /// Open a y/n confirmation prompt for `action`.
    pub fn open_confirm(&mut self, action: ConfirmAction) {
        self.mode = CommandLineMode::Confirm(action);
        self.input.clear();
        self.error = None;
        self.clear_completions();
    }

    /// Return to idle and clear all transient state.
    /// Any pending error is preserved so the render pass can show it once.
    pub fn reset(&mut self) {
        self.mode = CommandLineMode::Idle;
        self.input.clear();
        self.clear_completions();
    }

    // ── Input buffer ──────────────────────────────────────────────────────────

    pub fn push(&mut self, c: char) {
        self.input.push(c);
    }

    pub fn pop(&mut self) {
        self.input.pop();
    }

    // ── Error ─────────────────────────────────────────────────────────────────

    pub fn set_error(&mut self, msg: impl Into<String>) {
        self.error = Some(msg.into());
    }

    pub fn clear_error(&mut self) {
        self.error = None;
    }

    // ── Completions ───────────────────────────────────────────────────────────

    /// Load a fresh candidate list, select the first entry, and fill the input.
    pub fn open_completions(&mut self, candidates: Vec<(&'static str, &'static str)>) {
        if candidates.is_empty() {
            return;
        }
        self.input = candidates[0].0.to_string();
        self.completion_selected = Some(0);
        self.completions = candidates;
    }

    /// Advance to the next completion and apply it to the input buffer.
    pub fn next_completion(&mut self) {
        let len = self.completions.len();
        if len == 0 {
            return;
        }
        let next = self.completion_selected.map_or(0, |i| (i + 1) % len);
        self.completion_selected = Some(next);
        self.input = self.completions[next].0.to_string();
    }

    /// Retreat to the previous completion and apply it to the input buffer.
    pub fn prev_completion(&mut self) {
        let len = self.completions.len();
        if len == 0 {
            return;
        }
        let prev = self.completion_selected.map_or(0, |i| (i + len - 1) % len);
        self.completion_selected = Some(prev);
        self.input = self.completions[prev].0.to_string();
    }

    /// Discard the candidate list and selection without touching the input.
    pub fn clear_completions(&mut self) {
        self.completions.clear();
        self.completion_selected = None;
    }
}

