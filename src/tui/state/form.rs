/// Add-connection form state — supports both URL and field-by-field Config modes.
use crate::connection::{
    ConnectionSource, DatabaseConnection, DatabaseString, Engine, MysqlConnection,
    PostgresConnection, SqliteConnection, SslOptions, SslVerifyMode,
};

// ── Text mode ────────────────────────────────────────────────────────────────

/// Vim-inspired input mode for text fields.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TextMode {
    /// Navigate with `h/l`, `0/$`, `x`; enter Insert with `i/a/I/A`.
    Normal,
    /// Type freely; `Esc` returns to Normal.
    Insert,
}

// ── Field identifiers ─────────────────────────────────────────────────────────

/// Every possible form field across all engines and input modes.
/// `visible_fields` returns the ordered subset that applies to the current state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FieldId {
    // Always visible
    Name,
    Engine,
    InputMode,
    // URL mode (all engines)
    Url,
    // Config mode — Postgres / MySQL
    Hostname,
    Port,
    Username,
    Password,
    Database,
    PoolSize,
    Ssl,
    // Config mode — SQLite
    SqlitePath,
    CreateIfMissing,
}

impl FieldId {
    pub fn label(&self) -> &'static str {
        match self {
            FieldId::Name => "Name",
            FieldId::Engine => "Engine",
            FieldId::InputMode => "Input",
            FieldId::Url => "URL",
            FieldId::Hostname => "Hostname",
            FieldId::Port => "Port",
            FieldId::Username => "Username",
            FieldId::Password => "Password",
            FieldId::Database => "Database",
            FieldId::PoolSize => "Pool size",
            FieldId::Ssl => "SSL",
            FieldId::SqlitePath => "Path",
            FieldId::CreateIfMissing => "Create DB",
        }
    }

    /// True for fields that accept free text input.
    pub fn is_text(&self) -> bool {
        matches!(
            self,
            FieldId::Name
                | FieldId::Url
                | FieldId::Hostname
                | FieldId::Port
                | FieldId::Username
                | FieldId::Password
                | FieldId::Database
                | FieldId::PoolSize
                | FieldId::SqlitePath
        )
    }
}

// ── Input mode ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FormInputMode {
    Url,
    Config,
}

// ── Form state ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct AddConnectionForm {
    /// Index into `visible_fields()` of the currently focused field.
    pub focused: usize,
    /// Char offset within the focused text field's buffer.
    pub cursor_pos: usize,
    /// Current input mode for text fields.
    pub text_mode: TextMode,
    pub input_mode: FormInputMode,
    pub engine: Engine,
    // ── Shared ────────────────────────────────────────────────────────────────
    pub name: String,

    // ── URL mode ──────────────────────────────────────────────────────────────
    pub url: String,

    // ── Config mode — Postgres / MySQL ────────────────────────────────────────
    pub hostname: String,
    pub port: String,
    pub username: String,
    pub password: String,
    pub database: String,
    pub pool_size: String,
    pub ssl_enabled: bool,

    // ── Config mode — SQLite ──────────────────────────────────────────────────
    pub sqlite_path: String,
    pub create_if_missing: bool,
}

impl AddConnectionForm {
    pub fn new() -> Self {
        Self {
            focused: 0,
            cursor_pos: 0,
            text_mode: TextMode::Normal,
            input_mode: FormInputMode::Url,
            engine: Engine::Postgres,
            name: String::new(),
            url: String::new(),
            hostname: String::new(),
            port: "5432".to_string(),
            username: String::new(),
            password: String::new(),
            database: String::new(),
            pool_size: "5".to_string(),
            ssl_enabled: false,
            sqlite_path: String::new(),
            create_if_missing: true,
        }
    }

    // ── Field list ────────────────────────────────────────────────────────────

    /// Ordered fields visible for the current engine + input mode.
    pub fn visible_fields(&self) -> Vec<FieldId> {
        let mut fields = vec![FieldId::Name, FieldId::Engine, FieldId::InputMode];

        match (&self.input_mode, &self.engine) {
            (FormInputMode::Url, _) => {
                fields.push(FieldId::Url);
            }
            (FormInputMode::Config, Engine::Postgres | Engine::Mysql) => {
                fields.extend([
                    FieldId::Hostname,
                    FieldId::Port,
                    FieldId::Username,
                    FieldId::Password,
                    FieldId::Database,
                    FieldId::PoolSize,
                    FieldId::Ssl,
                ]);
            }
            (FormInputMode::Config, Engine::Sqlite) => {
                fields.extend([
                    FieldId::SqlitePath,
                    FieldId::CreateIfMissing,
                    FieldId::PoolSize,
                ]);
            }
        }

        fields
    }

    /// The `FieldId` that currently has focus.
    pub fn focused_field(&self) -> FieldId {
        self.visible_fields()
            .into_iter()
            .nth(self.focused)
            .unwrap_or(FieldId::Name)
    }

    // ── Navigation ────────────────────────────────────────────────────────────

    pub fn focus_next(&mut self) {
        let n = self.visible_fields().len();
        self.focused = (self.focused + 1) % n;
        self.reset_text_state();
    }

    pub fn focus_prev(&mut self) {
        let n = self.visible_fields().len();
        self.focused = (self.focused + n - 1) % n;
        self.reset_text_state();
    }

    /// Reset cursor and mode when moving to a new field.
    /// Lands in Normal mode with the cursor on the last character.
    fn reset_text_state(&mut self) {
        self.text_mode = TextMode::Normal;
        // Place on the last char (vim Normal), or 0 for empty / non-text fields.
        let len = self.focused_text_len().unwrap_or(0);
        self.cursor_pos = len.saturating_sub(1);
    }

    // ── Text field access ─────────────────────────────────────────────────────

    /// Mutable reference to the text buffer of the focused field, if it is a
    /// text field. Returns `None` for selectors and toggles.
    pub fn current_text_mut(&mut self) -> Option<&mut String> {
        match self.focused_field() {
            FieldId::Name => Some(&mut self.name),
            FieldId::Url => Some(&mut self.url),
            FieldId::Hostname => Some(&mut self.hostname),
            FieldId::Port => Some(&mut self.port),
            FieldId::Username => Some(&mut self.username),
            FieldId::Password => Some(&mut self.password),
            FieldId::Database => Some(&mut self.database),
            FieldId::PoolSize => Some(&mut self.pool_size),
            FieldId::SqlitePath => Some(&mut self.sqlite_path),
            _ => None,
        }
    }

    /// Read-only text value for any given field (used by the renderer).
    pub fn text_for(&self, field: &FieldId) -> Option<&str> {
        match field {
            FieldId::Name => Some(&self.name),
            FieldId::Url => Some(&self.url),
            FieldId::Hostname => Some(&self.hostname),
            FieldId::Port => Some(&self.port),
            FieldId::Username => Some(&self.username),
            FieldId::Password => Some(&self.password),
            FieldId::Database => Some(&self.database),
            FieldId::PoolSize => Some(&self.pool_size),
            FieldId::SqlitePath => Some(&self.sqlite_path),
            _ => None,
        }
    }

    /// Character count of the focused field's text (used for cursor placement).
    /// Returns `None` if the focused field is not a text field.
    pub fn focused_text_len(&self) -> Option<usize> {
        let field = self.focused_field();
        if field == FieldId::Password {
            return Some(self.password.chars().count());
        }
        self.text_for(&field).map(|s| s.chars().count())
    }

    // ── Cursor movement ─────────────────────────────────────────────────────

    pub fn cursor_left(&mut self) {
        self.cursor_pos = self.cursor_pos.saturating_sub(1);
    }

    pub fn cursor_right(&mut self) {
        let len = self.focused_text_len().unwrap_or(0);
        let max = match self.text_mode {
            TextMode::Insert => len,
            TextMode::Normal => len.saturating_sub(1),
        };
        self.cursor_pos = (self.cursor_pos + 1).min(max);
    }

    pub fn cursor_to_start(&mut self) {
        self.cursor_pos = 0;
    }

    pub fn cursor_to_end(&mut self) {
        let len = self.focused_text_len().unwrap_or(0);
        self.cursor_pos = match self.text_mode {
            TextMode::Insert => len,
            TextMode::Normal => len.saturating_sub(1),
        };
    }

    // ── Mode transitions ────────────────────────────────────────────────────────

    /// `i` — insert before cursor.
    pub fn enter_insert_before(&mut self) {
        self.text_mode = TextMode::Insert;
    }

    /// `a` — insert after cursor.
    pub fn enter_insert_after(&mut self) {
        let len = self.focused_text_len().unwrap_or(0);
        self.cursor_pos = (self.cursor_pos + 1).min(len);
        self.text_mode = TextMode::Insert;
    }

    /// `I` — insert at start.
    pub fn enter_insert_at_start(&mut self) {
        self.cursor_pos = 0;
        self.text_mode = TextMode::Insert;
    }

    /// `A` — insert at end.
    pub fn enter_insert_at_end(&mut self) {
        self.cursor_pos = self.focused_text_len().unwrap_or(0);
        self.text_mode = TextMode::Insert;
    }

    /// `Esc` — return to Normal and clamp cursor to last char.
    pub fn enter_normal(&mut self) {
        self.text_mode = TextMode::Normal;
        let len = self.focused_text_len().unwrap_or(0);
        if len > 0 && self.cursor_pos >= len {
            self.cursor_pos = len - 1;
        }
    }

    // ── Editing ────────────────────────────────────────────────────────────────

    pub fn insert_char(&mut self, c: char) {
        let pos = self.cursor_pos;
        match self.focused_field() {
            FieldId::Name => insert_at(&mut self.name, pos, c),
            FieldId::Url => insert_at(&mut self.url, pos, c),
            FieldId::Hostname => insert_at(&mut self.hostname, pos, c),
            FieldId::Port => insert_at(&mut self.port, pos, c),
            FieldId::Username => insert_at(&mut self.username, pos, c),
            FieldId::Password => insert_at(&mut self.password, pos, c),
            FieldId::Database => insert_at(&mut self.database, pos, c),
            FieldId::PoolSize => insert_at(&mut self.pool_size, pos, c),
            FieldId::SqlitePath => insert_at(&mut self.sqlite_path, pos, c),
            _ => return,
        }
        self.cursor_pos += 1;
    }

    pub fn delete_before_cursor(&mut self) {
        if self.cursor_pos == 0 {
            return;
        }
        let pos = self.cursor_pos - 1;
        match self.focused_field() {
            FieldId::Name => {
                remove_at(&mut self.name, pos);
            }
            FieldId::Url => {
                remove_at(&mut self.url, pos);
            }
            FieldId::Hostname => {
                remove_at(&mut self.hostname, pos);
            }
            FieldId::Port => {
                remove_at(&mut self.port, pos);
            }
            FieldId::Username => {
                remove_at(&mut self.username, pos);
            }
            FieldId::Password => {
                remove_at(&mut self.password, pos);
            }
            FieldId::Database => {
                remove_at(&mut self.database, pos);
            }
            FieldId::PoolSize => {
                remove_at(&mut self.pool_size, pos);
            }
            FieldId::SqlitePath => {
                remove_at(&mut self.sqlite_path, pos);
            }
            _ => return,
        }
        self.cursor_pos -= 1;
    }

    pub fn delete_at_cursor(&mut self) {
        let pos = self.cursor_pos;
        let deleted = match self.focused_field() {
            FieldId::Name => remove_at(&mut self.name, pos),
            FieldId::Url => remove_at(&mut self.url, pos),
            FieldId::Hostname => remove_at(&mut self.hostname, pos),
            FieldId::Port => remove_at(&mut self.port, pos),
            FieldId::Username => remove_at(&mut self.username, pos),
            FieldId::Password => remove_at(&mut self.password, pos),
            FieldId::Database => remove_at(&mut self.database, pos),
            FieldId::PoolSize => remove_at(&mut self.pool_size, pos),
            FieldId::SqlitePath => remove_at(&mut self.sqlite_path, pos),
            _ => return,
        };
        if deleted {
            let new_len = self.focused_text_len().unwrap_or(0);
            if new_len == 0 {
                self.cursor_pos = 0;
            } else if self.cursor_pos >= new_len {
                self.cursor_pos = new_len - 1;
            }
        }
    }

    // ── Selector / toggle cycling ─────────────────────────────────────────────

    /// Cycle the focused selector or toggle one step to the right / forward.
    pub fn cycle_right(&mut self) {
        match self.focused_field() {
            FieldId::Engine => {
                self.engine = match self.engine {
                    Engine::Postgres => Engine::Mysql,
                    Engine::Mysql => Engine::Sqlite,
                    Engine::Sqlite => Engine::Postgres,
                };
                self.sync_port_default();
                self.clamp_focus();
            }
            FieldId::InputMode => {
                self.input_mode = match self.input_mode {
                    FormInputMode::Url => FormInputMode::Config,
                    FormInputMode::Config => FormInputMode::Url,
                };
                self.clamp_focus();
            }
            FieldId::Ssl => self.ssl_enabled = !self.ssl_enabled,
            FieldId::CreateIfMissing => self.create_if_missing = !self.create_if_missing,
            _ => {}
        }
    }

    /// Cycle the focused selector one step to the left / backward.
    pub fn cycle_left(&mut self) {
        match self.focused_field() {
            FieldId::Engine => {
                self.engine = match self.engine {
                    Engine::Postgres => Engine::Sqlite,
                    Engine::Mysql => Engine::Postgres,
                    Engine::Sqlite => Engine::Mysql,
                };
                self.sync_port_default();
                self.clamp_focus();
            }
            // Binary toggles and InputMode are symmetric — left == right.
            _ => self.cycle_right(),
        }
    }

    /// Toggle the focused boolean field (SSL, CreateIfMissing).
    pub fn toggle_focused(&mut self) {
        match self.focused_field() {
            FieldId::Ssl => self.ssl_enabled = !self.ssl_enabled,
            FieldId::CreateIfMissing => self.create_if_missing = !self.create_if_missing,
            FieldId::InputMode => self.cycle_right(),
            _ => {}
        }
    }

    // ── Internal helpers ──────────────────────────────────────────────────────

    /// Update the port default when the engine changes, but only if the port
    /// still holds the previous default value so manual edits are preserved.
    fn sync_port_default(&mut self) {
        if self.port == "5432" || self.port == "3306" {
            self.port = match self.engine {
                Engine::Postgres => "5432",
                Engine::Mysql => "3306",
                Engine::Sqlite => "",
            }
            .to_string();
        }
    }

    /// Ensure the focus index stays within bounds after the field list changes.
    fn clamp_focus(&mut self) {
        let n = self.visible_fields().len();
        if self.focused >= n {
            self.focused = n.saturating_sub(1);
        }
    }

    // ── Validation & build ────────────────────────────────────────────────────

    pub fn validate(&self) -> Result<(), String> {
        if self.name.trim().is_empty() {
            return Err("Name is required".to_string());
        }
        self.build_source().map(|_| ())
    }

    /// Convert the form into a `ConnectionSource` ready for `add_connection`.
    pub fn build_source(&self) -> Result<ConnectionSource, String> {
        match self.input_mode {
            FormInputMode::Url => {
                if self.url.trim().is_empty() {
                    return Err("URL is required".to_string());
                }
                Ok(match self.engine {
                    Engine::Postgres => {
                        ConnectionSource::Url(DatabaseString::Postgres(self.url.clone()))
                    }
                    Engine::Mysql => ConnectionSource::Url(DatabaseString::Mysql(self.url.clone())),
                    Engine::Sqlite => {
                        ConnectionSource::Url(DatabaseString::Sqlite(self.url.clone()))
                    }
                })
            }

            FormInputMode::Config => {
                let pool = self
                    .pool_size
                    .parse::<i8>()
                    .map_err(|_| "Pool size must be a number".to_string())?;

                match self.engine {
                    Engine::Postgres | Engine::Mysql => {
                        if self.hostname.trim().is_empty() {
                            return Err("Hostname is required".to_string());
                        }
                        if self.username.trim().is_empty() {
                            return Err("Username is required".to_string());
                        }
                        if self.database.trim().is_empty() {
                            return Err("Database is required".to_string());
                        }
                        let port = self
                            .port
                            .parse::<i16>()
                            .map_err(|_| "Port must be a valid number".to_string())?;
                        let ssl = self.ssl_enabled.then_some(SslOptions {
                            verify: SslVerifyMode::Peer,
                            certfile: None,
                        });
                        let conn = match self.engine {
                            Engine::Postgres => DatabaseConnection::Postgres(PostgresConnection {
                                username: self.username.clone(),
                                password: self.password.clone(),
                                hostname: self.hostname.clone(),
                                database: self.database.clone(),
                                stack_trace: false,
                                port,
                                pool_size: pool,
                                ssl,
                            }),
                            Engine::Mysql => DatabaseConnection::Mysql(MysqlConnection {
                                username: self.username.clone(),
                                password: self.password.clone(),
                                hostname: self.hostname.clone(),
                                database: self.database.clone(),
                                stack_trace: false,
                                port,
                                pool_size: pool,
                                ssl,
                            }),
                            _ => unreachable!(),
                        };
                        Ok(ConnectionSource::Config(conn))
                    }

                    Engine::Sqlite => {
                        if self.sqlite_path.trim().is_empty() {
                            return Err("Path is required".to_string());
                        }
                        Ok(ConnectionSource::Config(DatabaseConnection::Sqlite(
                            SqliteConnection {
                                path: self.sqlite_path.clone(),
                                stack_trace: false,
                                pool_size: pool,
                                create_if_missing: self.create_if_missing,
                            },
                        )))
                    }
                }
            }
        }
    }
}

// ── Module-level helpers ──────────────────────────────────────────────────────

fn char_to_byte(s: &str, char_idx: usize) -> usize {
    s.char_indices()
        .nth(char_idx)
        .map(|(i, _)| i)
        .unwrap_or(s.len())
}

fn insert_at(s: &mut String, char_idx: usize, c: char) {
    let byte = char_to_byte(s, char_idx);
    s.insert(byte, c);
}

/// Returns `true` if a character was removed.
fn remove_at(s: &mut String, char_idx: usize) -> bool {
    if char_idx >= s.chars().count() {
        return false;
    }
    let byte = char_to_byte(s, char_idx);
    s.remove(byte);
    true
}
