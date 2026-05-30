/// Command-line bar state — vim-style `:` input and inline y/n confirmations.

// ── Available commands ────────────────────────────────────────────────────────

/// Commands available on the **home** screen.
pub const HOME_COMMANDS: &[(&str, &str)] = &[
    ("exit", "quit program"),
    ("q", "quit"),
    ("quit", "quit"),
    ("h", "help overlay"),
    ("help", "help overlay"),
    ("add", "add connection"),
    ("d", "delete <connection name>"),
    ("delete", "delete <connection name>"),
    ("connect", "open connection picker"),
];

/// Commands available on the **dashboard**.
pub const DASHBOARD_COMMANDS: &[(&str, &str)] = &[
    ("exit", "quit program"),
    ("q", "close pane"),
    ("quit", "close pane"),
    ("h", "help overlay"),
    ("help", "help overlay"),
    ("new", "create new tab or pane"),
    ("split", "horizontal split"),
    ("vsplit", "vertical split"),
    ("hsplit", "horizontal split"),
    ("table", "show table"),
    ("tables", "table list view"),
    ("noh", "clear search highlight"),
    ("schema", "schema view / picker"),
    ("editor", "switch to query editor"),
    ("results", "switch to query results"),
    ("close", "close pane"),
    ("where", "filter rows"),
    ("order", "sort rows"),
    ("select", "select columns"),
    ("insert", "stage new row (:insert [above|below])"),
    ("resize", "resize pane"),
    ("reset", "clear filter/sort/columns"),
    ("full", "toggle pane fullscreen"),
    ("!", "execute SQL directly"),
    ("back", "go back in pane history"),
    ("forward", "go forward in pane history"),
    ("disconnect", "disconnect and return home"),
    ("d", "delete <connection name>"),
    ("delete", "delete <connection name>"),
    ("tab", "tab subcommand / go to tab"),
    ("tabnew", "new tab"),
    ("tabnext", "next tab"),
    ("tabprev", "previous tab"),
    ("tabclose", "close current tab"),
];

/// Return every entry in `list` whose name starts with `input`.
pub fn compute_completions(
    input: &str,
    list: &'static [(&'static str, &'static str)],
) -> Vec<(&'static str, &'static str)> {
    list.iter()
        .filter(|(cmd, _)| cmd.starts_with(input))
        .copied()
        .collect()
}

fn char_idx_to_byte_idx(s: &str, char_idx: usize) -> usize {
    let mut byte_idx = 0;
    for (i, ch) in s.chars().enumerate() {
        if i == char_idx {
            return byte_idx;
        }
        byte_idx += ch.len_utf8();
    }
    s.len()
}

// ── Search direction ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchDirection {
    Forward,  // /
    Backward, // ?
}

// ── Mode ──────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommandLineMode {
    /// Showing status; no input focus.
    Idle,
    /// The `:` prompt is open and the user is typing a command.
    Input,
    /// The `/` or `?` search prompt is open.
    Search(SearchDirection),
    /// Editing a table cell value (opened by `i` in TableView).
    CellEdit {
        row: usize,
        col: usize,
        col_name: String,
    },
    /// Awaiting a `y` / `n` answer before executing a destructive action.
    Confirm(ConfirmAction),
}

/// Which action is waiting for confirmation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfirmAction {
    DeleteConnection(String),
    /// Commit staged changes (updates + deletes + inserts) for a table.
    CommitWrites {
        table: String,
        update_count: usize,
        delete_count: usize,
        insert_count: usize,
    },
}

// ── State ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct CommandLine {
    pub mode: CommandLineMode,
    /// Current text the user has typed.
    pub input: String,
    /// Cursor position inside `input` as a character index.
    pub input_cursor: usize,
    /// One-shot error message shown in idle mode after a failed command.
    pub error: Option<String>,
    /// Loading / spinner message shown in idle mode during async work.
    pub loading: Option<String>,
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
            input_cursor: 0,
            error: None,
            loading: None,
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
        self.input_cursor = 0;
        self.error = None;
        self.clear_completions();
    }

    /// Open the `/` or `?` search prompt.
    pub fn open_search(&mut self, direction: SearchDirection) {
        self.mode = CommandLineMode::Search(direction);
        self.input.clear();
        self.input_cursor = 0;
        self.error = None;
        self.clear_completions();
    }

    /// Open the cell editor for `row`/`col` with `current_value` pre-filled.
    pub fn open_cell_edit(&mut self, row: usize, col: usize, col_name: &str, current_value: &str) {
        self.mode = CommandLineMode::CellEdit {
            row,
            col,
            col_name: col_name.to_string(),
        };
        self.input = current_value.to_string();
        self.input_cursor = self.input.chars().count();
        self.error = None;
        self.clear_completions();
    }

    /// Open a y/n confirmation prompt for `action`.
    pub fn open_confirm(&mut self, action: ConfirmAction) {
        self.mode = CommandLineMode::Confirm(action);
        self.input.clear();
        self.input_cursor = 0;
        self.error = None;
        self.clear_completions();
    }

    /// Return to idle and clear all transient state.
    pub fn reset(&mut self) {
        self.mode = CommandLineMode::Idle;
        self.input.clear();
        self.input_cursor = 0;
        self.error = None;
        self.loading = None;
        self.clear_completions();
    }

    // ── Input buffer ──────────────────────────────────────────────────────────

    pub fn push(&mut self, c: char) {
        let byte = char_idx_to_byte_idx(&self.input, self.input_cursor);
        self.input.insert(byte, c);
        self.input_cursor += 1;
    }

    /// Backspace behavior: delete the character before the cursor.
    pub fn pop(&mut self) {
        if self.input_cursor == 0 {
            return;
        }
        let end = char_idx_to_byte_idx(&self.input, self.input_cursor);
        let start = char_idx_to_byte_idx(&self.input, self.input_cursor - 1);
        self.input.replace_range(start..end, "");
        self.input_cursor -= 1;
    }

    /// Delete behavior: delete the character at the cursor.
    pub fn delete(&mut self) {
        let len = self.input.chars().count();
        if self.input_cursor >= len {
            return;
        }
        let start = char_idx_to_byte_idx(&self.input, self.input_cursor);
        let end = char_idx_to_byte_idx(&self.input, self.input_cursor + 1);
        self.input.replace_range(start..end, "");
    }

    pub fn move_cursor_left(&mut self) {
        self.input_cursor = self.input_cursor.saturating_sub(1);
    }

    pub fn move_cursor_right(&mut self) {
        let len = self.input.chars().count();
        self.input_cursor = (self.input_cursor + 1).min(len);
    }

    pub fn move_cursor_home(&mut self) {
        self.input_cursor = 0;
    }

    pub fn move_cursor_end(&mut self) {
        self.input_cursor = self.input.chars().count();
    }

    // ── Error ─────────────────────────────────────────────────────────────────

    pub fn set_error(&mut self, msg: impl Into<String>) {
        self.error = Some(msg.into());
    }

    pub fn clear_error(&mut self) {
        self.error = None;
    }

    // ── Loading ───────────────────────────────────────────────────────────────

    pub fn set_loading(&mut self, msg: impl Into<String>) {
        self.loading = Some(msg.into());
    }

    pub fn clear_loading(&mut self) {
        self.loading = None;
    }

    // ── Completions ───────────────────────────────────────────────────────────

    /// Load a fresh candidate list, select the first entry, and fill the input.
    pub fn open_completions(&mut self, candidates: Vec<(&'static str, &'static str)>) {
        if candidates.is_empty() {
            return;
        }
        self.input = candidates[0].0.to_string();
        self.input_cursor = self.input.chars().count();
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
        self.input_cursor = self.input.chars().count();
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
        self.input_cursor = self.input.chars().count();
    }

    /// Discard the candidate list and selection without touching the input.
    pub fn clear_completions(&mut self) {
        self.completions.clear();
        self.completion_selected = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cmdline_cursor_insert_backspace() {
        let mut c = CommandLine::new();
        c.open_input();
        c.push('a');
        c.push('c');
        c.move_cursor_left();
        c.push('b');
        assert_eq!(c.input, "abc");
        assert_eq!(c.input_cursor, 2);

        c.pop();
        assert_eq!(c.input, "ac");
        assert_eq!(c.input_cursor, 1);
    }

    #[test]
    fn cmdline_cursor_delete_at_position() {
        let mut c = CommandLine::new();
        c.open_input();
        c.push('a');
        c.push('b');
        c.push('c');
        c.move_cursor_left();
        c.delete();
        assert_eq!(c.input, "ab");
        assert_eq!(c.input_cursor, 2);
    }
}
