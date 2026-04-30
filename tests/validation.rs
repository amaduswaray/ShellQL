use shellql::connection::connect::{ConnectionError, validate_connection_string};

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
        matches!(result, Err(ConnectionError::InvalidUrl(_))),
        "expected InvalidUrl, got {result:?}"
    );
}

#[test]
fn unsupported_scheme_is_rejected() {
    let result = validate_connection_string("http://example.com/mydb");
    assert!(
        matches!(result, Err(ConnectionError::UnsupportedScheme(_))),
        "expected UnsupportedScheme, got {result:?}"
    );
}

#[test]
fn postgres_url_missing_host_is_rejected() {
    // url crate parses "postgres:///mydb" with an empty host string
    let result = validate_connection_string("postgres:///mydb");
    assert!(
        matches!(result, Err(ConnectionError::MissingHost)),
        "expected MissingHost, got {result:?}"
    );
}

#[test]
fn postgres_url_missing_database_path_is_rejected() {
    let result = validate_connection_string("postgres://localhost/");
    assert!(
        matches!(result, Err(ConnectionError::MissingPath)),
        "expected MissingPath, got {result:?}"
    );
}

#[test]
fn postgres_url_with_no_path_at_all_is_rejected() {
    let result = validate_connection_string("postgres://localhost");
    assert!(
        matches!(result, Err(ConnectionError::MissingPath)),
        "expected MissingPath, got {result:?}"
    );
}
