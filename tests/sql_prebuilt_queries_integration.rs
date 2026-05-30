use std::env;

use serial_test::serial;
use shellql::connection::connect_db;
use shellql::connection::models::{ConnectionSource, DatabaseString, DbPool};
use shellql::connection::{count_rows_filtered, execute_query, query_rows};

#[derive(Clone, Copy)]
enum Target {
    Postgres,
    Mysql,
    Sqlite,
}

impl Target {
    fn label(self) -> &'static str {
        match self {
            Target::Postgres => "postgres",
            Target::Mysql => "mysql",
            Target::Sqlite => "sqlite",
        }
    }
}

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

async fn pool_for(target: Target) -> DbPool {
    match target {
        Target::Postgres => connect_db(ConnectionSource::Url(DatabaseString::Postgres(pg_url())))
            .await
            .expect("failed to connect to Postgres test DB"),
        Target::Mysql => connect_db(ConnectionSource::Url(DatabaseString::Mysql(mysql_url())))
            .await
            .expect("failed to connect to MySQL test DB"),
        Target::Sqlite => connect_db(ConnectionSource::Url(DatabaseString::Sqlite(sqlite_url())))
            .await
            .expect("failed to connect to SQLite test DB"),
    }
}

fn targets() -> [Target; 3] {
    [Target::Postgres, Target::Mysql, Target::Sqlite]
}

#[tokio::test]
#[serial]
async fn query_rows_filter_sort_selected_cols_across_engines() {
    for target in targets() {
        let pool = pool_for(target).await;

        let selected = vec!["emp_no".to_string(), "first_name".to_string()];
        let (headers, rows) = query_rows(
            &pool,
            "employees",
            Some("emp_no IN (10001, 10002, 10003)"),
            Some("emp_no"),
            true,
            Some(&selected),
            10,
            0,
        )
        .await
        .expect("query_rows should succeed");

        assert_eq!(headers, selected, "headers mismatch for {}", target.label());
        assert!(!rows.is_empty(), "expected rows for {}", target.label());
        assert_eq!(
            rows[0].len(),
            2,
            "selected width mismatch for {}",
            target.label()
        );
        assert_eq!(
            rows[0][0],
            "10003",
            "descending ORDER BY failed for {}",
            target.label()
        );
    }
}

#[tokio::test]
#[serial]
async fn count_rows_filtered_matches_total_and_predicate() {
    for target in targets() {
        let pool = pool_for(target).await;

        let one = count_rows_filtered(&pool, "employees", Some("emp_no = 10001"))
            .await
            .expect("count_rows_filtered should succeed");
        assert_eq!(one, 1, "filtered count mismatch for {}", target.label());

        let total = count_rows_filtered(&pool, "employees", None)
            .await
            .expect("count_rows_filtered(None) should succeed");

        assert!(
            total >= one,
            "total filtered count should be >= predicate count for {}",
            target.label()
        );
        assert!(
            total > 0,
            "total filtered count should be positive for {}",
            target.label()
        );
    }
}

#[tokio::test]
#[serial]
async fn execute_query_select_returns_headers_and_rows() {
    for target in targets() {
        let pool = pool_for(target).await;

        let (headers, rows) = execute_query(
            &pool,
            "SELECT emp_no, first_name FROM employees WHERE emp_no = 10001;",
        )
        .await
        .expect("execute_query SELECT should succeed");

        assert_eq!(
            headers.len(),
            2,
            "header width mismatch for {}",
            target.label()
        );
        assert_eq!(
            headers[0],
            "emp_no",
            "header[0] mismatch for {}",
            target.label()
        );
        assert_eq!(
            headers[1],
            "first_name",
            "header[1] mismatch for {}",
            target.label()
        );
        assert_eq!(rows.len(), 1, "row count mismatch for {}", target.label());
        assert_eq!(
            rows[0][0],
            "10001",
            "emp_no mismatch for {}",
            target.label()
        );
        assert_eq!(
            rows[0][1],
            "Georgi",
            "first_name mismatch for {}",
            target.label()
        );
    }
}

#[tokio::test]
#[serial]
async fn execute_query_dml_returns_rows_affected() {
    for target in targets() {
        let pool = pool_for(target).await;

        let temp_id = match target {
            Target::Postgres => 219901,
            Target::Mysql => 219902,
            Target::Sqlite => 219903,
        };

        let insert_sql = format!(
            "INSERT INTO employees (emp_no, birth_date, first_name, last_name, gender, hire_date, email) \
             VALUES ({temp_id}, '1990-01-01', 'Live', 'Watcher', 'M', '2024-01-01', 'live.watcher@example.com')"
        );
        let delete_sql = format!("DELETE FROM employees WHERE emp_no = {temp_id}");

        let (headers, rows) = execute_query(&pool, &insert_sql)
            .await
            .expect("execute_query INSERT should succeed");

        // Always attempt cleanup, even if assertions fail later.
        let _ = execute_query(&pool, &delete_sql).await;

        assert_eq!(
            headers,
            vec!["Rows Affected".to_string()],
            "DML headers mismatch for {}",
            target.label()
        );
        assert_eq!(
            rows.len(),
            1,
            "DML row count mismatch for {}",
            target.label()
        );
        assert_eq!(
            rows[0][0],
            "1",
            "Rows Affected should be 1 for {}",
            target.label()
        );
    }
}

#[tokio::test]
#[serial]
async fn execute_query_select_no_rows_is_empty() {
    for target in targets() {
        let pool = pool_for(target).await;

        let (_headers, rows) =
            execute_query(&pool, "SELECT emp_no FROM employees WHERE emp_no = -1")
                .await
                .expect("execute_query empty SELECT should succeed");

        assert!(rows.is_empty(), "expected no rows for {}", target.label());
    }
}
