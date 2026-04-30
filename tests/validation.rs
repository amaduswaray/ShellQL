/// Tests for `validate_connection_string` — pure function, no I/O.
use shellql::connection::connect::{validate_connection_string, ConnectionError};

#[test]
fn valid_postgres_url_is_accepted() {
    let result = validate_connection_string("postgres://user:pass@localhost/mydb");
    assert!(result.is_ok(), "expected Ok, got {result:?}");
}

#[test]
fn valid_postgresql_alias_is_accepted() {
    let result = validate_connection_string("postgresql://user:pass@localhost/mydb");
    assert!(result.is_ok());
}

#[test]
fn valid_mysql_url_is_accepted() {
    let result = validate_connection_string("mysql://user:pass@localhost/mydb");
    assert!(result.is_ok());
}

#[test]
fn valid_sqlite_url_is_accepted() {
    // SQLite URLs do not require a host
    let result = validate_connection_string("sqlite:///path/to/db.sqlite");
    assert!(result.is_ok());
}

#[test]
fn completely_invalid_url_is_rejected() {
    let result = validate_connection_string("not a url at all");
    assert!(
        matches!(result, Err(ConnectionError::InvalidUrl { .. })),
        "expected InvalidUrl, got {result:?}"
    );
}

#[test]
fn unsupported_scheme_is_rejected() {
    let result = validate_connection_string("http://example.com/mydb");
    assert!(
        matches!(result, Err(ConnectionError::UnsupportedScheme { .. })),
        "expected UnsupportedScheme, got {result:?}"
    );
}

#[test]
fn postgres_url_missing_host_is_rejected() {
    // url crate parses "postgres:///mydb" with an empty host string
    let result = validate_connection_string("postgres:///mydb");
    assert!(
        matches!(result, Err(ConnectionError::MissingHost { .. })),
        "expected MissingHost, got {result:?}"
    );
}

#[test]
fn postgres_url_missing_database_path_is_rejected() {
    let result = validate_connection_string("postgres://localhost/");
    assert!(
        matches!(result, Err(ConnectionError::MissingPath { .. })),
        "expected MissingPath, got {result:?}"
    );
}

#[test]
fn postgres_url_with_no_path_at_all_is_rejected() {
    let result = validate_connection_string("postgres://localhost");
    assert!(
        matches!(result, Err(ConnectionError::MissingPath { .. })),
        "expected MissingPath, got {result:?}"
    );
}

/// Verify that rendered errors echo the original input so users can see
/// exactly which string was rejected.
#[test]
fn error_display_contains_input_string() {
    let input = "http://example.com/mydb";
    let err = validate_connection_string(input).unwrap_err();
    let rendered = err.to_string();
    assert!(
        rendered.contains(input),
        "rendered error should echo the input URL back to the user\ngot:\n{rendered}"
    );
}

#[test]
fn unsupported_scheme_error_highlights_scheme() {
    let err = validate_connection_string("ftp://host/db").unwrap_err();
    let rendered = err.to_string();
    assert!(
        rendered.contains("ftp"),
        "rendered error should name the bad scheme\ngot:\n{rendered}"
    );
}
