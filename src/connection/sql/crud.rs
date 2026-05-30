use std::collections::HashMap;

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
            // Cast the incoming string value to the target column's real type.
            let udt_name: Option<(String,)> = sqlx::query_as(
                "SELECT c.udt_name
                 FROM information_schema.columns c
                 WHERE c.table_schema = 'public'
                   AND c.table_name = $1
                   AND c.column_name = $2",
            )
            .bind(table)
            .bind(target_col)
            .fetch_optional(pg)
            .await?;

            let query = if let Some((udt,)) = udt_name {
                format!(
                    "UPDATE \"{table}\" SET \"{target_col}\" = $1::{udt} WHERE \"{pk_col}\"::text = $2"
                )
            } else {
                format!(
                    "UPDATE \"{table}\" SET \"{target_col}\" = $1 WHERE \"{pk_col}\"::text = $2"
                )
            };

            let result = sqlx::query(&query)
                .bind(new_value)
                .bind(pk_val)
                .execute(pg)
                .await?;
            Ok(result.rows_affected())
        }
        DbPool::Mysql(my) => {
            let query = format!("UPDATE `{table}` SET `{target_col}` = ? WHERE `{pk_col}` = ?");
            let result = sqlx::query(&query)
                .bind(new_value)
                .bind(pk_val)
                .execute(my)
                .await?;
            Ok(result.rows_affected())
        }
        DbPool::Sqlite(sq) => {
            let query =
                format!("UPDATE \"{table}\" SET \"{target_col}\" = ? WHERE \"{pk_col}\" = ?");
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

    match pool {
        DbPool::Postgres(pg) => {
            // Map each column to its underlying Postgres type so text input can
            // be cast safely when inserting.
            let type_rows: Vec<(String, String)> = sqlx::query_as(
                "SELECT c.column_name, c.udt_name
                 FROM information_schema.columns c
                 WHERE c.table_schema = 'public'
                   AND c.table_name = $1",
            )
            .bind(table)
            .fetch_all(pg)
            .await?;

            let type_map: HashMap<String, String> = type_rows.into_iter().collect();

            let col_list = cols
                .iter()
                .map(|c| format!("\"{c}\""))
                .collect::<Vec<_>>()
                .join(", ");

            let placeholders = cols
                .iter()
                .enumerate()
                .map(|(i, c)| {
                    let idx = i + 1;
                    match type_map.get(c) {
                        Some(udt) => format!("${idx}::{udt}"),
                        None => format!("${idx}"),
                    }
                })
                .collect::<Vec<_>>()
                .join(", ");

            let query = format!("INSERT INTO \"{table}\" ({col_list}) VALUES ({placeholders})");
            let mut q = sqlx::query(&query);
            for v in vals {
                q = q.bind(v);
            }
            let result = q.execute(pg).await?;
            Ok(result.rows_affected())
        }
        DbPool::Mysql(my) => {
            let col_list = cols
                .iter()
                .map(|c| format!("`{c}`"))
                .collect::<Vec<_>>()
                .join(", ");
            let placeholders: Vec<String> = (1..=cols.len()).map(|_| "?".to_string()).collect();
            let placeholder_list = placeholders.join(", ");
            let query = format!("INSERT INTO `{table}` ({col_list}) VALUES ({placeholder_list})");
            let mut q = sqlx::query(&query);
            for v in vals {
                q = q.bind(v);
            }
            let result = q.execute(my).await?;
            Ok(result.rows_affected())
        }
        DbPool::Sqlite(sq) => {
            let col_list = cols
                .iter()
                .map(|c| format!("\"{c}\""))
                .collect::<Vec<_>>()
                .join(", ");
            let placeholders: Vec<String> = (1..=cols.len()).map(|_| "?".to_string()).collect();
            let placeholder_list = placeholders.join(", ");
            let query = format!("INSERT INTO \"{table}\" ({col_list}) VALUES ({placeholder_list})");
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
            let placeholders: Vec<String> = (1..=pk_vals.len()).map(|i| format!("${i}")).collect();
            let query = format!(
                "DELETE FROM \"{table}\" WHERE \"{pk_col}\"::text IN ({})",
                placeholders.join(", ")
            );
            let mut q = sqlx::query(&query);
            for v in pk_vals {
                q = q.bind(v);
            }
            let result = q.execute(pg).await?;
            Ok(result.rows_affected())
        }
        DbPool::Mysql(my) => {
            let placeholders: Vec<String> = (1..=pk_vals.len()).map(|_| "?".to_string()).collect();
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
            let placeholders: Vec<String> = (1..=pk_vals.len()).map(|_| "?".to_string()).collect();
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
