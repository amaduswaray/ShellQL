use super::super::models::DbPool;

// ── Update cell ───────────────────────────────────────────────────────────────

/// Update a single cell in `table` identified by `pk_col` = `pk_val`.
/// Sets `target_col` to `new_value`.
pub async fn update_cell(
    pool: &DbPool,
    table: &str,
    pk_col: &str,
    pk_val: &str,
    target_col: &str,
    new_value: &str,
) -> color_eyre::eyre::Result<u64> {
    match pool {
        DbPool::Postgres(pg) => {
            let query = format!(
                "UPDATE \"{table}\" SET \"{target_col}\" = $1 WHERE \"{pk_col}\" = $2"
            );
            let result = sqlx::query(&query)
                .bind(new_value)
                .bind(pk_val)
                .execute(pg)
                .await?;
            Ok(result.rows_affected())
        }
        DbPool::Mysql(my) => {
            let query = format!(
                "UPDATE `{table}` SET `{target_col}` = ? WHERE `{pk_col}` = ?"
            );
            let result = sqlx::query(&query)
                .bind(new_value)
                .bind(pk_val)
                .execute(my)
                .await?;
            Ok(result.rows_affected())
        }
        DbPool::Sqlite(sq) => {
            let query = format!(
                "UPDATE \"{table}\" SET \"{target_col}\" = ? WHERE \"{pk_col}\" = ?"
            );
            let result = sqlx::query(&query)
                .bind(new_value)
                .bind(pk_val)
                .execute(sq)
                .await?;
            Ok(result.rows_affected())
        }
    }
}

// ── Insert row ────────────────────────────────────────────────────────────────

/// Insert a new row into `table`. `cols` and `vals` must be the same length.
/// Returns the number of rows inserted (should be 1).
pub async fn insert_row(
    pool: &DbPool,
    table: &str,
    cols: &[String],
    vals: &[String],
) -> color_eyre::eyre::Result<u64> {
    assert_eq!(cols.len(), vals.len(), "columns and values must match");

    let col_list = cols.iter().map(|c| format!("\"{c}\"")).collect::<Vec<_>>().join(", ");
    let placeholders: Vec<String> = (1..=cols.len()).map(|i| format!("${i}")).collect();
    let placeholder_list = placeholders.join(", ");

    match pool {
        DbPool::Postgres(pg) => {
            let query = format!(
                "INSERT INTO \"{table}\" ({col_list}) VALUES ({placeholder_list})"
            );
            let mut q = sqlx::query(&query);
            for v in vals {
                q = q.bind(v);
            }
            let result = q.execute(pg).await?;
            Ok(result.rows_affected())
        }
        DbPool::Mysql(my) => {
            let placeholders: Vec<String> = (1..=cols.len()).map(|_| "?".to_string()).collect();
            let placeholder_list = placeholders.join(", ");
            let query = format!(
                "INSERT INTO `{table}` ({col_list}) VALUES ({placeholder_list})"
            );
            let mut q = sqlx::query(&query);
            for v in vals {
                q = q.bind(v);
            }
            let result = q.execute(my).await?;
            Ok(result.rows_affected())
        }
        DbPool::Sqlite(sq) => {
            let placeholders: Vec<String> = (1..=cols.len()).map(|_| "?".to_string()).collect();
            let placeholder_list = placeholders.join(", ");
            let query = format!(
                "INSERT INTO \"{table}\" ({col_list}) VALUES ({placeholder_list})"
            );
            let mut q = sqlx::query(&query);
            for v in vals {
                q = q.bind(v);
            }
            let result = q.execute(sq).await?;
            Ok(result.rows_affected())
        }
    }
}

// ── Delete rows ───────────────────────────────────────────────────────────────

/// Delete rows from `table` where `pk_col` matches any value in `pk_vals`.
/// Returns the number of rows deleted.
pub async fn delete_rows(
    pool: &DbPool,
    table: &str,
    pk_col: &str,
    pk_vals: &[String],
) -> color_eyre::eyre::Result<u64> {
    if pk_vals.is_empty() {
        return Ok(0);
    }

    match pool {
        DbPool::Postgres(pg) => {
            let placeholders: Vec<String> =
                (1..=pk_vals.len()).map(|i| format!("${i}")).collect();
            let query = format!(
                "DELETE FROM \"{table}\" WHERE \"{pk_col}\" IN ({})"
                , placeholders.join(", ")
            );
            let mut q = sqlx::query(&query);
            for v in pk_vals {
                q = q.bind(v);
            }
            let result = q.execute(pg).await?;
            Ok(result.rows_affected())
        }
        DbPool::Mysql(my) => {
            let placeholders: Vec<String> =
                (1..=pk_vals.len()).map(|_| "?".to_string()).collect();
            let query = format!(
                "DELETE FROM `{table}` WHERE `{pk_col}` IN ({})",
                placeholders.join(", ")
            );
            let mut q = sqlx::query(&query);
            for v in pk_vals {
                q = q.bind(v);
            }
            let result = q.execute(my).await?;
            Ok(result.rows_affected())
        }
        DbPool::Sqlite(sq) => {
            let placeholders: Vec<String> =
                (1..=pk_vals.len()).map(|_| "?".to_string()).collect();
            let query = format!(
                "DELETE FROM \"{table}\" WHERE \"{pk_col}\" IN ({})",
                placeholders.join(", ")
            );
            let mut q = sqlx::query(&query);
            for v in pk_vals {
                q = q.bind(v);
            }
            let result = q.execute(sq).await?;
            Ok(result.rows_affected())
        }
    }
}
