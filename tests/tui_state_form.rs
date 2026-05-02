use shellql::{
    connection::models::{ConnectionSource, DatabaseString, Engine},
    tui::state::{
        AddConnectionForm, FieldId, FormInputMode, TextMode,
    },
};

// ── Helpers ───────────────────────────────────────────────────────────────────

/// A fresh form pointing at the Name field in Insert mode — the default state.
fn name_form() -> AddConnectionForm {
    AddConnectionForm::new()
}

// ── insert_char ───────────────────────────────────────────────────────────────

#[test]
fn insert_char_appends_and_advances_cursor() {
    let mut f = name_form();
    f.insert_char('a');
    f.insert_char('b');
    assert_eq!(f.name, "ab");
    assert_eq!(f.cursor_pos, 2);
}

#[test]
fn insert_space_is_supported_in_text_fields() {
    let mut f = name_form();
    f.name = "hello".to_string();
    f.cursor_pos = 5;
    f.insert_char(' ');
    f.insert_char('w');
    assert_eq!(f.name, "hello w");
    assert_eq!(f.cursor_pos, 7);
}

#[test]
fn insert_char_in_middle_of_string() {
    let mut f = name_form();
    f.name = "hllo".to_string();
    f.cursor_pos = 1;
    f.insert_char('e');
    assert_eq!(f.name, "hello");
    assert_eq!(f.cursor_pos, 2);
}

#[test]
fn insert_unicode_multibyte() {
    let mut f = name_form();
    f.insert_char('ä');
    f.insert_char('ö');
    assert_eq!(f.name, "äö");
    assert_eq!(f.cursor_pos, 2); // char count, not byte count
}

// ── delete_before_cursor ──────────────────────────────────────────────────────

#[test]
fn delete_before_cursor_removes_previous_char() {
    let mut f = name_form();
    f.name = "hello".to_string();
    f.cursor_pos = 5;
    f.delete_before_cursor();
    assert_eq!(f.name, "hell");
    assert_eq!(f.cursor_pos, 4);
}

#[test]
fn delete_before_cursor_at_zero_is_noop() {
    let mut f = name_form();
    f.name = "hello".to_string();
    f.cursor_pos = 0;
    f.delete_before_cursor();
    assert_eq!(f.name, "hello");
    assert_eq!(f.cursor_pos, 0);
}

#[test]
fn delete_before_cursor_unicode() {
    let mut f = name_form();
    f.name = "äö".to_string();
    f.cursor_pos = 2;
    f.delete_before_cursor();
    assert_eq!(f.name, "ä");
    assert_eq!(f.cursor_pos, 1);
}

// ── delete_at_cursor ──────────────────────────────────────────────────────────

#[test]
fn delete_at_cursor_removes_current_char() {
    let mut f = name_form();
    f.name = "helo".to_string();
    f.cursor_pos = 2;
    f.delete_at_cursor();
    assert_eq!(f.name, "heo");
    assert_eq!(f.cursor_pos, 2);
}

#[test]
fn delete_at_cursor_on_last_char_clamps_position() {
    let mut f = name_form();
    f.name = "hi".to_string();
    f.cursor_pos = 1;
    f.delete_at_cursor();
    assert_eq!(f.name, "h");
    assert_eq!(f.cursor_pos, 0);
}

#[test]
fn delete_at_cursor_on_empty_is_noop() {
    let mut f = name_form();
    f.cursor_pos = 0;
    f.delete_at_cursor();
    assert_eq!(f.name, "");
    assert_eq!(f.cursor_pos, 0);
}

// ── cursor movement ───────────────────────────────────────────────────────────

#[test]
fn cursor_left_clamps_at_zero() {
    let mut f = name_form();
    f.cursor_pos = 0;
    f.cursor_left();
    assert_eq!(f.cursor_pos, 0);
}

#[test]
fn cursor_right_insert_mode_can_sit_past_last_char() {
    let mut f = name_form();
    f.name = "hi".to_string();
    f.cursor_pos = 1;
    f.text_mode = TextMode::Insert;
    f.cursor_right();
    assert_eq!(f.cursor_pos, 2);
}

#[test]
fn cursor_right_normal_mode_stops_on_last_char() {
    let mut f = name_form();
    f.name = "hi".to_string();
    f.cursor_pos = 1;
    f.text_mode = TextMode::Normal;
    f.cursor_right();
    assert_eq!(f.cursor_pos, 1);
}

#[test]
fn cursor_to_start_and_end() {
    let mut f = name_form();
    f.name = "hello".to_string();
    f.cursor_pos = 3;
    f.cursor_to_start();
    assert_eq!(f.cursor_pos, 0);
    f.cursor_to_end();
    assert_eq!(f.cursor_pos, 4); // Normal mode → on last char (len - 1)
}

// ── mode transitions ──────────────────────────────────────────────────────────

#[test]
fn enter_normal_clamps_cursor_to_last_char() {
    let mut f = name_form();
    f.name = "hi".to_string();
    f.cursor_pos = 2;
    f.enter_normal();
    assert_eq!(f.text_mode, TextMode::Normal);
    assert_eq!(f.cursor_pos, 1);
}

#[test]
fn enter_normal_on_empty_string_keeps_zero() {
    let mut f = name_form();
    f.cursor_pos = 0;
    f.enter_normal();
    assert_eq!(f.cursor_pos, 0);
}

#[test]
fn enter_insert_after_advances_cursor_one() {
    let mut f = name_form();
    f.name = "hi".to_string();
    f.cursor_pos = 0;
    f.text_mode = TextMode::Normal;
    f.enter_insert_after();
    assert_eq!(f.cursor_pos, 1);
    assert_eq!(f.text_mode, TextMode::Insert);
}

#[test]
fn enter_insert_at_start_sets_zero() {
    let mut f = name_form();
    f.name = "hello".to_string();
    f.cursor_pos = 3;
    f.enter_insert_at_start();
    assert_eq!(f.cursor_pos, 0);
    assert_eq!(f.text_mode, TextMode::Insert);
}

#[test]
fn enter_insert_at_end_sets_length() {
    let mut f = name_form();
    f.name = "hello".to_string();
    f.cursor_pos = 0;
    f.enter_insert_at_end();
    assert_eq!(f.cursor_pos, 5);
    assert_eq!(f.text_mode, TextMode::Insert);
}

// ── field navigation ──────────────────────────────────────────────────────────

#[test]
fn focus_next_resets_to_insert_at_text_end() {
    let mut f = name_form();
    f.name = "test".to_string();
    f.cursor_pos = 2;
    f.text_mode = TextMode::Normal;
    f.focus_next(); // Name(0) → Engine(1)
    assert_eq!(f.focused, 1);
    assert_eq!(f.text_mode, TextMode::Normal);
    assert_eq!(f.cursor_pos, 0); // Engine is not a text field
}

#[test]
fn focus_next_on_text_field_puts_cursor_at_end() {
    let mut f = name_form();
    f.focused = 3; // URL field in URL mode
    f.url = "postgres://localhost/db".to_string();
    f.cursor_pos = 0;
    f.text_mode = TextMode::Normal;
    f.focus_prev(); // → InputMode(2)
    f.focus_next(); // → URL(3) again
    assert_eq!(f.text_mode, TextMode::Normal);
    assert_eq!(f.cursor_pos, f.url.chars().count().saturating_sub(1));
}

#[test]
fn focus_prev_wraps_from_first_to_last() {
    let mut f = name_form();
    f.focused = 0;
    f.focus_prev();
    assert!(f.focused > 0);
}

// ── visible_fields ────────────────────────────────────────────────────────────

#[test]
fn visible_fields_url_mode_has_url_not_hostname() {
    let f = AddConnectionForm::new();
    let fields = f.visible_fields();
    assert!(fields.contains(&FieldId::Url));
    assert!(!fields.contains(&FieldId::Hostname));
    assert!(!fields.contains(&FieldId::SqlitePath));
}

#[test]
fn visible_fields_config_postgres_has_hostname_not_sqlite_path() {
    let mut f = AddConnectionForm::new();
    f.input_mode = FormInputMode::Config;
    let fields = f.visible_fields();
    assert!(fields.contains(&FieldId::Hostname));
    assert!(fields.contains(&FieldId::Port));
    assert!(fields.contains(&FieldId::Ssl));
    assert!(!fields.contains(&FieldId::Url));
    assert!(!fields.contains(&FieldId::SqlitePath));
}

#[test]
fn visible_fields_config_sqlite_has_path_not_hostname() {
    let mut f = AddConnectionForm::new();
    f.input_mode = FormInputMode::Config;
    f.engine = Engine::Sqlite;
    let fields = f.visible_fields();
    assert!(fields.contains(&FieldId::SqlitePath));
    assert!(fields.contains(&FieldId::CreateIfMissing));
    assert!(!fields.contains(&FieldId::Hostname));
    assert!(!fields.contains(&FieldId::Url));
    assert!(!fields.contains(&FieldId::Ssl));
}

// ── build_source / validate ───────────────────────────────────────────────────

#[test]
fn build_source_url_postgres_produces_correct_variant() {
    let mut f = AddConnectionForm::new();
    f.url = "postgres://user:pass@localhost/mydb".to_string();
    let src = f.build_source().unwrap();
    assert!(matches!(
        src,
        ConnectionSource::Url(DatabaseString::Postgres(_))
    ));
}

#[test]
fn build_source_url_empty_returns_error() {
    let f = AddConnectionForm::new();
    assert!(f.build_source().is_err());
}

#[test]
fn build_source_config_postgres_validates_required_fields() {
    let mut f = AddConnectionForm::new();
    f.input_mode = FormInputMode::Config;
    assert!(f.build_source().is_err()); // hostname missing
    f.hostname = "localhost".to_string();
    assert!(f.build_source().is_err()); // username missing
    f.username = "user".to_string();
    assert!(f.build_source().is_err()); // database missing
    f.database = "mydb".to_string();
    assert!(f.build_source().is_ok());
}

#[test]
fn validate_requires_non_empty_name() {
    let mut f = AddConnectionForm::new();
    f.url = "postgres://localhost/db".to_string();
    assert!(f.validate().is_err());
    f.name = "prod".to_string();
    assert!(f.validate().is_ok());
}

// ── engine / port defaults ────────────────────────────────────────────────────

#[test]
fn engine_switch_updates_default_port() {
    let mut f = AddConnectionForm::new(); // Postgres, port "5432"
    assert_eq!(f.port, "5432");
    f.focused = 1; // Engine field
    f.cycle_right(); // Postgres → MySQL
    assert_eq!(f.port, "3306");
    f.cycle_right(); // MySQL → SQLite
    assert_eq!(f.port, "");
}

#[test]
fn engine_switch_preserves_manually_edited_port() {
    let mut f = AddConnectionForm::new();
    f.port = "5555".to_string();
    f.focused = 1;
    f.cycle_right(); // Postgres → MySQL — port was not a default, keep it
    assert_eq!(f.port, "5555");
}
