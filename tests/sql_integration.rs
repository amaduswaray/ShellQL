use std::env;

use shellql::connection::connect_db;
use shellql::connection::models::ConnectionSource;
use shellql::connection::models::DatabaseString;
use shellql::connection::{
    count_rows, delete_rows, filter_rows, insert_row, list_tables, table_rows, table_schema,
    update_cell,
};

// ── Helpers ───────────────────────────────────────────────────────────────────

fn pg_url() -> String {
    env::var("TEST_POSTGRES_URL")
        .unwrap_or_else(|_| "postgres://shellql:shellql@localhost:5433/shellql_test".to_string())
}

fn mysql_url() -> String {
    env::var("TEST_MYSQL_URL")
        .unwrap_or_else(|_| "mysql://shellql:shellql@localhost:3307/shellql_test".to_string())
}

fn sqlite_url() -> String {
    env::var("TEST_SQLITE_URL")
        .unwrap_or_else(|_| "sqlite://./tests/docker/sqlite/seed.db".to_string())
}

async fn pg_pool() -> shellql::connection::models::DbPool {
    connect_db(ConnectionSource::Url(DatabaseString::Postgres(pg_url())))
        .await
        .expect("Failed to connect to Postgres test DB — is `docker compose -f tests/docker-compose.yml up` running?")
}

async fn mysql_pool() -> shellql::connection::models::DbPool {
    connect_db(ConnectionSource::Url(DatabaseString::Mysql(mysql_url())))
        .await
        .expect("Failed to connect to MySQL test DB — is `docker compose -f tests/docker-compose.yml up` running?")
}

async fn sqlite_pool() -> shellql::connection::models::DbPool {
    connect_db(ConnectionSource::Url(DatabaseString::Sqlite(sqlite_url())))
        .await
        .expect("Failed to connect to SQLite test DB")
}

// ── list_tables ───────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_list_tables_postgres() {
    let pool = pg_pool().await;
    let tables = list_tables(&pool).await.expect("list_tables should work");
    assert!(
        tables.contains(&"employees".to_string()),
        "expected employees table in Postgres"
    );
}

#[tokio::test]
async fn test_list_tables_mysql() {
    let pool = mysql_pool().await;
    let tables = list_tables(&pool).await.expect("list_tables should work");
    assert!(
        tables.contains(&"employees".to_string()),
        "expected employees table in MySQL"
    );
}

#[tokio::test]
async fn test_list_tables_sqlite() {
    let pool = sqlite_pool().await;
    let tables = list_tables(&pool).await.expect("list_tables should work");
    assert!(
        tables.contains(&"employees".to_string()),
        "expected employees table in SQLite"
    );
}

// ── table_schema ──────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_table_schema_postgres() {
    let pool = pg_pool().await;
    let schema = table_schema(&pool, "employees")
        .await
        .expect("schema should work");
    let cols: Vec<String> = schema.iter().map(|c| c.name.clone()).collect();
    assert!(cols.contains(&"emp_no".to_string()));
    assert!(cols.contains(&"first_name".to_string()));
    assert!(cols.contains(&"email".to_string()));

    let pk = schema.iter().find(|c| c.name == "emp_no").unwrap();
    assert!(pk.is_primary_key, "emp_no should be PK");
}

#[tokio::test]
async fn test_table_schema_mysql() {
    let pool = mysql_pool().await;
    let schema = table_schema(&pool, "employees")
        .await
        .expect("schema should work");
    let cols: Vec<String> = schema.iter().map(|c| c.name.clone()).collect();
    assert!(cols.contains(&"emp_no".to_string()));

    let pk = schema.iter().find(|c| c.name == "emp_no").unwrap();
    assert!(pk.is_primary_key, "emp_no should be PK");
}

#[tokio::test]
async fn test_table_schema_sqlite() {
    let pool = sqlite_pool().await;
    let schema = table_schema(&pool, "employees")
        .await
        .expect("schema should work");
    let cols: Vec<String> = schema.iter().map(|c| c.name.clone()).collect();
    assert!(cols.contains(&"emp_no".to_string()));

    let pk = schema.iter().find(|c| c.name == "emp_no").unwrap();
    assert!(pk.is_primary_key, "emp_no should be PK");
}

// ── table_rows (with offset + limit) ──────────────────────────────────────────

#[tokio::test]
async fn test_table_rows_paginated_postgres() {
    let pool = pg_pool().await;
    let (cols, rows) = table_rows(&pool, "employees", 10, 0)
        .await
        .expect("table_rows should work");
    assert_eq!(cols.len(), 7);
    assert_eq!(rows.len(), 10);

    let (_, rows2) = table_rows(&pool, "employees", 10, 5)
        .await
        .expect("offset should work");
    assert_eq!(rows2.len(), 10);
    // Row 5 from the first query should match row 0 from the second
    assert_eq!(rows[5], rows2[0]);
}

#[tokio::test]
async fn test_table_rows_paginated_mysql() {
    let pool = mysql_pool().await;
    let (cols, rows) = table_rows(&pool, "employees", 10, 0)
        .await
        .expect("table_rows should work");
    assert_eq!(cols.len(), 7);
    assert_eq!(rows.len(), 10);
}

#[tokio::test]
async fn test_table_rows_paginated_sqlite() {
    let pool = sqlite_pool().await;
    let (cols, rows) = table_rows(&pool, "employees", 10, 0)
        .await
        .expect("table_rows should work");
    assert_eq!(cols.len(), 7);
    assert_eq!(rows.len(), 10);
}

// ── count_rows ────────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_count_rows_postgres() {
    let pool = pg_pool().await;
    let count = count_rows(&pool, "employees")
        .await
        .expect("count_rows should work");
    assert_eq!(count, 100);
}

#[tokio::test]
async fn test_count_rows_mysql() {
    let pool = mysql_pool().await;
    let count = count_rows(&pool, "employees")
        .await
        .expect("count_rows should work");
    assert_eq!(count, 100);
}

#[tokio::test]
async fn test_count_rows_sqlite() {
    let pool = sqlite_pool().await;
    let count = count_rows(&pool, "employees")
        .await
        .expect("count_rows should work");
    assert_eq!(count, 50); // SQLite seed has 50 rows
}

// ── filter_rows ───────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_filter_rows_postgres() {
    let pool = pg_pool().await;
    let (cols, rows) = filter_rows(&pool, "employees", "first_name", "Georgi", 10, 0)
        .await
        .expect("filter_rows should work");
    assert!(!rows.is_empty());
    let first_name_idx = cols.iter().position(|c| c == "first_name").unwrap();
    assert!(rows[0][first_name_idx].contains("Georgi"));
}

#[tokio::test]
async fn test_filter_rows_mysql() {
    let pool = mysql_pool().await;
    let (cols, rows) = filter_rows(&pool, "employees", "first_name", "Georgi", 10, 0)
        .await
        .expect("filter_rows should work");
    assert!(!rows.is_empty());
}

#[tokio::test]
async fn test_filter_rows_sqlite() {
    let pool = sqlite_pool().await;
    let (cols, rows) = filter_rows(&pool, "employees", "first_name", "Georgi", 10, 0)
        .await
        .expect("filter_rows should work");
    assert!(!rows.is_empty());
}

// ── update_cell ───────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_update_cell_postgres() {
    let pool = pg_pool().await;
    let affected = update_cell(
        &pool,
        "employees",
        "emp_no",
        "10001",
        "email",
        "updated@example.com",
    )
    .await
    .expect("update_cell should work");
    assert_eq!(affected, 1);

    // Verify
    let (_, rows) = table_rows(&pool, "employees", 1, 0).await.unwrap();
    let email_idx = rows[0].len() - 1; // email is last column
    assert_eq!(rows[0][email_idx], "updated@example.com");

    // Restore
    update_cell(
        &pool,
        "employees",
        "emp_no",
        "10001",
        "email",
        "georgi.facello@example.com",
    )
    .await
    .unwrap();
}

#[tokio::test]
async fn test_update_cell_mysql() {
    let pool = mysql_pool().await;
    let affected = update_cell(
        &pool,
        "employees",
        "emp_no",
        "10001",
        "email",
        "updated@example.com",
    )
    .await
    .expect("update_cell should work");
    assert_eq!(affected, 1);

    // Restore
    update_cell(
        &pool,
        "employees",
        "emp_no",
        "10001",
        "email",
        "georgi.facello@example.com",
    )
    .await
    .unwrap();
}

#[tokio::test]
async fn test_update_cell_sqlite() {
    let pool = sqlite_pool().await;
    let affected = update_cell(
        &pool,
        "employees",
        "emp_no",
        "10001",
        "email",
        "updated@example.com",
    )
    .await
    .expect("update_cell should work");
    assert_eq!(affected, 1);

    // Restore
    update_cell(
        &pool,
        "employees",
        "emp_no",
        "10001",
        "email",
        "georgi.facello@example.com",
    )
    .await
    .unwrap();
}

// ── insert_row ────────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_insert_row_postgres() {
    let pool = pg_pool().await;
    let cols = vec![
        "emp_no".to_string(),
        "birth_date".to_string(),
        "first_name".to_string(),
        "last_name".to_string(),
        "gender".to_string(),
        "hire_date".to_string(),
        "email".to_string(),
    ];
    let vals = vec![
        "99999".to_string(),
        "1990-01-01".to_string(),
        "Test".to_string(),
        "User".to_string(),
        "M".to_string(),
        "2024-01-01".to_string(),
        "test@example.com".to_string(),
    ];
    let affected = insert_row(&pool, "employees", &cols, &vals)
        .await
        .expect("insert_row should work");
    assert_eq!(affected, 1);

    // Cleanup
    delete_rows(&pool, "employees", "emp_no", &["99999".to_string()])
        .await
        .unwrap();
}

#[tokio::test]
async fn test_insert_row_mysql() {
    let pool = mysql_pool().await;
    let cols = vec![
        "emp_no".to_string(),
        "birth_date".to_string(),
        "first_name".to_string(),
        "last_name".to_string(),
        "gender".to_string(),
        "hire_date".to_string(),
        "email".to_string(),
    ];
    let vals = vec![
        "99999".to_string(),
        "1990-01-01".to_string(),
        "Test".to_string(),
        "User".to_string(),
        "M".to_string(),
        "2024-01-01".to_string(),
        "test@example.com".to_string(),
    ];
    let affected = insert_row(&pool, "employees", &cols, &vals)
        .await
        .expect("insert_row should work");
    assert_eq!(affected, 1);

    // Cleanup
    delete_rows(&pool, "employees", "emp_no", &["99999".to_string()])
        .await
        .unwrap();
}

#[tokio::test]
async fn test_insert_row_sqlite() {
    let pool = sqlite_pool().await;
    let cols = vec![
        "emp_no".to_string(),
        "birth_date".to_string(),
        "first_name".to_string(),
        "last_name".to_string(),
        "gender".to_string(),
        "hire_date".to_string(),
        "email".to_string(),
    ];
    let vals = vec![
        "99999".to_string(),
        "1990-01-01".to_string(),
        "Test".to_string(),
        "User".to_string(),
        "M".to_string(),
        "2024-01-01".to_string(),
        "test@example.com".to_string(),
    ];
    let affected = insert_row(&pool, "employees", &cols, &vals)
        .await
        .expect("insert_row should work");
    assert_eq!(affected, 1);

    // Cleanup
    delete_rows(&pool, "employees", "emp_no", &["99999".to_string()])
        .await
        .unwrap();
}

// ── delete_rows ───────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_delete_rows_postgres() {
    let pool = pg_pool().await;

    // Insert a row to delete
    let cols = vec![
        "emp_no".to_string(),
        "birth_date".to_string(),
        "first_name".to_string(),
        "last_name".to_string(),
        "gender".to_string(),
        "hire_date".to_string(),
    ];
    let vals = vec![
        "88888".to_string(),
        "1990-01-01".to_string(),
        "ToDelete".to_string(),
        "User".to_string(),
        "M".to_string(),
        "2024-01-01".to_string(),
    ];
    insert_row(&pool, "employees", &cols, &vals).await.unwrap();

    let affected = delete_rows(&pool, "employees", "emp_no", &["88888".to_string()])
        .await
        .expect("delete_rows should work");
    assert_eq!(affected, 1);

    let count = count_rows(&pool, "employees").await.unwrap();
    assert_eq!(count, 100); // back to original count
}

#[tokio::test]
async fn test_delete_rows_mysql() {
    let pool = mysql_pool().await;

    let cols = vec![
        "emp_no".to_string(),
        "birth_date".to_string(),
        "first_name".to_string(),
        "last_name".to_string(),
        "gender".to_string(),
        "hire_date".to_string(),
    ];
    let vals = vec![
        "88888".to_string(),
        "1990-01-01".to_string(),
        "ToDelete".to_string(),
        "User".to_string(),
        "M".to_string(),
        "2024-01-01".to_string(),
    ];
    insert_row(&pool, "employees", &cols, &vals).await.unwrap();

    let affected = delete_rows(&pool, "employees", "emp_no", &["88888".to_string()])
        .await
        .expect("delete_rows should work");
    assert_eq!(affected, 1);

    let count = count_rows(&pool, "employees").await.unwrap();
    assert_eq!(count, 100);
}

#[tokio::test]
async fn test_delete_rows_sqlite() {
    let pool = sqlite_pool().await;

    let cols = vec![
        "emp_no".to_string(),
        "birth_date".to_string(),
        "first_name".to_string(),
        "last_name".to_string(),
        "gender".to_string(),
        "hire_date".to_string(),
    ];
    let vals = vec![
        "88888".to_string(),
        "1990-01-01".to_string(),
        "ToDelete".to_string(),
        "User".to_string(),
        "M".to_string(),
        "2024-01-01".to_string(),
    ];
    insert_row(&pool, "employees", &cols, &vals).await.unwrap();

    let affected = delete_rows(&pool, "employees", "emp_no", &["88888".to_string()])
        .await
        .expect("delete_rows should work");
    assert_eq!(affected, 1);

    let count = count_rows(&pool, "employees").await.unwrap();
    assert_eq!(count, 50); // back to original count
}
