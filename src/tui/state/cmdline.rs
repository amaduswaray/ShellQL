#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommandLineMode {
    Idle,
    Input,
    Confirm(ConfirmAction),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfirmAction {
    DeleteConnection(String),
    // TODO: Add more confirm actions for command line
}

#[derive(Debug, Clone)]
pub struct CommandLine {
    pub mode: CommandLineMode,
    pub input: String,
    pub error: Option<String>,
}

impl CommandLine {
    pub fn new() -> Self {
        Self {
            mode: CommandLineMode::Idle,
            input: String::new(),
            error: None,
        }
    }

    /// Whether the command line has keyboard focus.
    pub fn is_active(&self) -> bool {
        self.mode != CommandLineMode::Idle
    }

    /// Open the `:` input prompt.
    pub fn open_input(&mut self) {
        self.mode = CommandLineMode::Input;
        self.input.clear();
        self.error = None;
    }

    /// Open a y/n confirmation prompt for `action`.
    pub fn open_confirm(&mut self, action: ConfirmAction) {
        self.mode = CommandLineMode::Confirm(action);
        self.input.clear();
        self.error = None;
    }

    /// Append a character to the input buffer.
    pub fn push(&mut self, c: char) {
        self.input.push(c);
    }

    /// Remove the last character from the input buffer.
    pub fn pop(&mut self) {
        self.input.pop();
    }

    /// Return to idle and clear the input buffer.
    /// Any pending error is preserved so the render pass can show it once.
    pub fn reset(&mut self) {
        self.mode = CommandLineMode::Idle;
        self.input.clear();
    }

    /// Store an error message to be displayed on the next idle render.
    pub fn set_error(&mut self, msg: impl Into<String>) {
        self.error = Some(msg.into());
    }

    /// Dismiss the current error.
    pub fn clear_error(&mut self) {
        self.error = None;
    }
}
