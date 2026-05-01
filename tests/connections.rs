use std::path::PathBuf;

use serial_test::serial;
use shellql::connection::{
    ConnectionSource, Database, DatabaseStore, DatabaseString, load_connections_from,
    models::Engine, save_connections_to,
};

fn test_path() -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("tests");
    path.push("data");
    path.push(".test_connections.json");
    path
}

fn cleanup() {
    let _ = std::fs::remove_file(test_path());
}

fn add_connection_with_path(
    name: String,
    connection: ConnectionSource,
    engine: Engine,
    path: &PathBuf,
) -> std::io::Result<Database> {
    let mut store = load_connections_from(path);

    if store.databases.values().any(|db| db.name == name) {
        return Err(std::io::Error::new(
            std::io::ErrorKind::AlreadyExists,
            "Database name already exists. Try using a different name",
        ));
    }

    if store
        .databases
        .values()
        .any(|db| db.connection == connection)
    {
        return Err(std::io::Error::new(
            std::io::ErrorKind::AlreadyExists,
            "Database connection already exists. Try using a different connection string",
        ));
    }

    let db = Database {
        name,
        engine,
        connection,
    };
    store.databases.insert(db.name.clone(), db.clone());
    save_connections_to(&store, path)?;
    Ok(db)
}

fn delete_connection_with_path(name: String, path: &PathBuf) -> std::io::Result<()> {
    let mut store = load_connections_from(path);

    if !store.databases.contains_key(&name) {
        // Mirror production behaviour: warn and return Ok — nothing to delete.
        eprintln!("warning: no connection named '{name}' found — nothing was deleted.");
        return Ok(());
    }

    store.databases.remove(&name);
    save_connections_to(&store, path)
}

fn update_connection_with_path(updated: Database, path: &PathBuf) -> std::io::Result<()> {
    let mut store = load_connections_from(path);

    if !store.databases.contains_key(&updated.name) {
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "Database not found",
        ));
    }

    store.databases.insert(updated.name.clone(), updated);
    save_connections_to(&store, path)
}

fn pg_source(url: &str) -> ConnectionSource {
    ConnectionSource::Url(DatabaseString::Postgres(url.to_string()))
}

fn mysql_source(url: &str) -> ConnectionSource {
    ConnectionSource::Url(DatabaseString::Mysql(url.to_string()))
}

fn sqlite_source(path: &str) -> ConnectionSource {
    ConnectionSource::Url(DatabaseString::Sqlite(path.to_string()))
}

#[test]
#[serial]
fn save_and_load_roundtrip() {
    cleanup();
    let path = test_path();

    let mut store = DatabaseStore::default();
    store.databases.insert(
        "prod".to_string(),
        Database {
            name: "prod".to_string(),
            engine: Engine::Postgres,
            connection: pg_source("postgres://user:pass@localhost/prod"),
        },
    );

    save_connections_to(&store, &path).expect("save should succeed");

    let loaded = load_connections_from(&path);
    assert_eq!(loaded.databases.len(), 1);
    assert!(loaded.databases.contains_key("prod"));

    cleanup();
}

#[test]
#[serial]
fn load_from_nonexistent_path_returns_empty_store() {
    cleanup();
    let loaded = load_connections_from(&test_path());
    assert!(loaded.databases.is_empty());
}

#[test]
#[serial]
fn load_from_corrupt_file_returns_empty_store() {
    let path = test_path();
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(&path, b"{ this is not valid json !!!").unwrap();

    let loaded = load_connections_from(&path);
    assert!(
        loaded.databases.is_empty(),
        "corrupt file should yield an empty store"
    );

    cleanup();
}

#[test]
#[serial]
fn add_connection_happy_path() {
    cleanup();
    let path = test_path();

    let db = add_connection_with_path(
        "local-pg".to_string(),
        pg_source("postgres://user:pass@localhost/dev"),
        Engine::Postgres,
        &path,
    )
    .expect("add should succeed");

    assert_eq!(db.name, "local-pg");

    let store = load_connections_from(&path);
    assert!(store.databases.contains_key("local-pg"));

    cleanup();
}

#[test]
#[serial]
fn add_connection_duplicate_name_is_rejected() {
    cleanup();
    let path = test_path();

    add_connection_with_path(
        "mydb".to_string(),
        pg_source("postgres://user:pass@localhost/first"),
        Engine::Postgres,
        &path,
    )
    .expect("first add should succeed");

    let err = add_connection_with_path(
        "mydb".to_string(),
        pg_source("postgres://user:pass@localhost/second"),
        Engine::Postgres,
        &path,
    )
    .expect_err("duplicate name should be rejected");

    assert_eq!(err.kind(), std::io::ErrorKind::AlreadyExists);

    cleanup();
}

#[test]
#[serial]
fn add_connection_duplicate_connection_string_is_rejected() {
    cleanup();
    let path = test_path();

    let url = "postgres://user:pass@localhost/dev";

    add_connection_with_path("first".to_string(), pg_source(url), Engine::Postgres, &path)
        .expect("first add should succeed");

    let err = add_connection_with_path(
        "second".to_string(),
        pg_source(url),
        Engine::Postgres,
        &path,
    )
    .expect_err("duplicate connection string should be rejected");

    assert_eq!(err.kind(), std::io::ErrorKind::AlreadyExists);

    cleanup();
}

#[test]
#[serial]
fn add_multiple_different_connections() {
    cleanup();
    let path = test_path();

    add_connection_with_path(
        "pg".to_string(),
        pg_source("postgres://a:b@localhost/pgdb"),
        Engine::Postgres,
        &path,
    )
    .unwrap();

    add_connection_with_path(
        "my".to_string(),
        mysql_source("mysql://a:b@localhost/mydb"),
        Engine::Mysql,
        &path,
    )
    .unwrap();

    add_connection_with_path(
        "sq".to_string(),
        sqlite_source("sqlite:///tmp/test.db"),
        Engine::Sqlite,
        &path,
    )
    .unwrap();

    let store = load_connections_from(&path);
    assert_eq!(store.databases.len(), 3);

    cleanup();
}

#[test]
#[serial]
fn delete_connection_happy_path() {
    cleanup();
    let path = test_path();

    add_connection_with_path(
        "to-delete".to_string(),
        pg_source("postgres://user:pass@localhost/gone"),
        Engine::Postgres,
        &path,
    )
    .unwrap();

    delete_connection_with_path("to-delete".to_string(), &path).expect("delete should succeed");

    let store = load_connections_from(&path);
    assert!(!store.databases.contains_key("to-delete"));

    cleanup();
}

#[test]
#[serial]
fn delete_nonexistent_connection_is_a_noop() {
    cleanup();
    let path = test_path();

    // Should not error — just emit a warning and leave the store untouched.
    delete_connection_with_path("ghost".to_string(), &path)
        .expect("deleting a non-existent connection should not error");

    // Store is still empty — nothing was written or corrupted.
    let store = load_connections_from(&path);
    assert!(store.databases.is_empty());

    cleanup();
}

#[test]
#[serial]
fn update_connection_happy_path() {
    cleanup();
    let path = test_path();

    add_connection_with_path(
        "staging".to_string(),
        pg_source("postgres://user:pass@old-host/stagingdb"),
        Engine::Postgres,
        &path,
    )
    .unwrap();

    let updated = Database {
        name: "staging".to_string(),
        engine: Engine::Postgres,
        connection: pg_source("postgres://user:pass@new-host/stagingdb"),
    };

    update_connection_with_path(updated, &path).expect("update should succeed");

    let store = load_connections_from(&path);
    let db = store
        .databases
        .get("staging")
        .expect("connection should still exist");

    assert_eq!(
        db.connection,
        pg_source("postgres://user:pass@new-host/stagingdb"),
        "connection string should have been updated"
    );

    cleanup();
}

#[test]
#[serial]
fn update_connection_not_found_returns_error() {
    cleanup();
    let path = test_path();

    let ghost = Database {
        name: "does-not-exist".to_string(),
        engine: Engine::Postgres,
        connection: pg_source("postgres://user:pass@localhost/ghost"),
    };

    let err = update_connection_with_path(ghost, &path)
        .expect_err("updating a non-existent connection should fail");

    assert_eq!(err.kind(), std::io::ErrorKind::NotFound);

    cleanup();
}
