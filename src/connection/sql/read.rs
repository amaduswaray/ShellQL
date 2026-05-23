use super::super::models::DbPool;

// ── Rows ──────────────────────────────────────────────────────────────────────

/// Fetch up to `limit` rows from `table`, starting at `offset`.
/// Returns `(column_names, rows)` where every cell is a display string.
pub async fn table_rows(
    pool: &DbPool,
    table: &str,
    limit: u32,
    offset: u32,
) -> color_eyre::eyre::Result<(Vec<String>, Vec<Vec<String>>)> {
    match pool {
        DbPool::Postgres(pg) => {
            let col_rows: Vec<(String,)> = sqlx::query_as(
                "SELECT column_name
                 FROM information_schema.columns
                 WHERE table_schema = 'public' AND table_name = $1
                 ORDER BY ordinal_position",
            )
            .bind(table)
            .fetch_all(pg)
            .await?;

            let cols: Vec<String> = col_rows.into_iter().map(|(n,)| n).collect();
            if cols.is_empty() {
                return Ok((vec![], vec![]));
            }

            let casts = cols
                .iter()
                .map(|c| format!("\"{}\"::text", c))
                .collect::<Vec<_>>()
                .join(", ");

            let query = format!("SELECT {casts} FROM \"{table}\" LIMIT {limit} OFFSET {offset}");
            let rows = sqlx::query(&query).fetch_all(pg).await?;

            use sqlx::Row;
            let data = rows
                .iter()
                .map(|r| {
                    (0..cols.len())
                        .map(|i| {
                            r.try_get::<Option<String>, _>(i)
                                .map(|v| v.unwrap_or_else(|| "NULL".into()))
                                .unwrap_or_else(|_| "?".into())
                        })
                        .collect()
                })
                .collect();

            Ok((cols, data))
        }

        DbPool::Mysql(my) => {
            let col_rows: Vec<(String,)> = sqlx::query_as(
                "SELECT column_name
                 FROM information_schema.columns
                 WHERE table_schema = DATABASE() AND table_name = ?
                 ORDER BY ordinal_position",
            )
            .bind(table)
            .fetch_all(my)
            .await?;

            let cols: Vec<String> = col_rows.into_iter().map(|(n,)| n).collect();
            if cols.is_empty() {
                return Ok((vec![], vec![]));
            }

            let casts = cols
                .iter()
                .map(|c| format!("CONVERT(`{}`, CHAR)", c))
                .collect::<Vec<_>>()
                .join(", ");

            let query = format!("SELECT {casts} FROM `{table}` LIMIT {limit} OFFSET {offset}");
            let rows = sqlx::query(&query).fetch_all(my).await?;

            use sqlx::Row;
            let data = rows
                .iter()
                .map(|r| {
                    (0..cols.len())
                        .map(|i| {
                            r.try_get::<Option<String>, _>(i)
                                .map(|v| v.unwrap_or_else(|| "NULL".into()))
                                .unwrap_or_else(|_| "?".into())
                        })
                        .collect()
                })
                .collect();

            Ok((cols, data))
        }

        DbPool::Sqlite(sq) => {
            let query = format!("SELECT * FROM \"{table}\" LIMIT {limit} OFFSET {offset}");
            let rows = sqlx::query(&query).fetch_all(sq).await?;

            use sqlx::Row;
            let cols: Vec<String> = rows
                .first()
                .map(|r| r.columns().iter().map(|c| c.name().to_string()).collect())
                .unwrap_or_default();

            let data = rows
                .iter()
                .map(|r| (0..cols.len()).map(|i| sqlite_cell(r, i)).collect())
                .collect();

            Ok((cols, data))
        }
    }
}

/// Return the total row count for `table`.
pub async fn count_rows(pool: &DbPool, table: &str) -> color_eyre::eyre::Result<i64> {
    match pool {
        DbPool::Postgres(pg) => {
            let (count,): (i64,) = sqlx::query_as(&format!("SELECT COUNT(*) FROM \"{table}\""))
                .fetch_one(pg)
                .await?;
            Ok(count)
        }
        DbPool::Mysql(my) => {
            let (count,): (i64,) = sqlx::query_as(&format!("SELECT COUNT(*) FROM `{table}`"))
                .fetch_one(my)
                .await?;
            Ok(count)
        }
        DbPool::Sqlite(sq) => {
            let (count,): (i64,) = sqlx::query_as(&format!("SELECT COUNT(*) FROM \"{table}\""))
                .fetch_one(sq)
                .await?;
            Ok(count)
        }
    }
}

/// Return rows where `column` contains `query` (case-insensitive).
/// Returns `(column_names, rows)`.
pub async fn filter_rows(
    pool: &DbPool,
    table: &str,
    column: &str,
    query: &str,
    limit: u32,
    offset: u32,
) -> color_eyre::eyre::Result<(Vec<String>, Vec<Vec<String>>)> {
    match pool {
        DbPool::Postgres(pg) => {
            let col_rows: Vec<(String,)> = sqlx::query_as(
                "SELECT column_name
                 FROM information_schema.columns
                 WHERE table_schema = 'public' AND table_name = $1
                 ORDER BY ordinal_position",
            )
            .bind(table)
            .fetch_all(pg)
            .await?;

            let cols: Vec<String> = col_rows.into_iter().map(|(n,)| n).collect();
            if cols.is_empty() {
                return Ok((vec![], vec![]));
            }

            let casts = cols
                .iter()
                .map(|c| format!("\"{}\"::text", c))
                .collect::<Vec<_>>()
                .join(", ");

            let sql = format!(
                "SELECT {casts} FROM \"{table}\"
                 WHERE \"{column}\"::text ILIKE '%' || $1 || '%'
                 LIMIT {limit} OFFSET {offset}"
            );
            let rows = sqlx::query(&sql).bind(query).fetch_all(pg).await?;

            use sqlx::Row;
            let data = rows
                .iter()
                .map(|r| {
                    (0..cols.len())
                        .map(|i| {
                            r.try_get::<Option<String>, _>(i)
                                .map(|v| v.unwrap_or_else(|| "NULL".into()))
                                .unwrap_or_else(|_| "?".into())
                        })
                        .collect()
                })
                .collect();

            Ok((cols, data))
        }

        DbPool::Mysql(my) => {
            let col_rows: Vec<(String,)> = sqlx::query_as(
                "SELECT column_name
                 FROM information_schema.columns
                 WHERE table_schema = DATABASE() AND table_name = ?
                 ORDER BY ordinal_position",
            )
            .bind(table)
            .fetch_all(my)
            .await?;

            let cols: Vec<String> = col_rows.into_iter().map(|(n,)| n).collect();
            if cols.is_empty() {
                return Ok((vec![], vec![]));
            }

            let casts = cols
                .iter()
                .map(|c| format!("CONVERT(`{}`, CHAR)", c))
                .collect::<Vec<_>>()
                .join(", ");

            let sql = format!(
                "SELECT {casts} FROM `{table}`
                 WHERE CONVERT(`{column}`, CHAR) LIKE CONCAT('%', ?, '%')
                 LIMIT {limit} OFFSET {offset}"
            );
            let rows = sqlx::query(&sql).bind(query).fetch_all(my).await?;

            use sqlx::Row;
            let data = rows
                .iter()
                .map(|r| {
                    (0..cols.len())
                        .map(|i| {
                            r.try_get::<Option<String>, _>(i)
                                .map(|v| v.unwrap_or_else(|| "NULL".into()))
                                .unwrap_or_else(|_| "?".into())
                        })
                        .collect()
                })
                .collect();

            Ok((cols, data))
        }

        DbPool::Sqlite(sq) => {
            let sql = format!(
                "SELECT * FROM \"{table}\"
                 WHERE \"{column}\" LIKE '%' || ? || '%'
                 LIMIT {limit} OFFSET {offset}"
            );
            let rows = sqlx::query(&sql).bind(query).fetch_all(sq).await?;

            use sqlx::Row;
            let cols: Vec<String> = rows
                .first()
                .map(|r| r.columns().iter().map(|c| c.name().to_string()).collect())
                .unwrap_or_default();

            let data = rows
                .iter()
                .map(|r| (0..cols.len()).map(|i| sqlite_cell(r, i)).collect())
                .collect();

            Ok((cols, data))
        }
    }
}

/// Fetch rows from `table` with optional `filter` (raw WHERE clause),
/// optional `sort` (`col_name` + `desc`), and optional `selected_cols`.
/// When `selected_cols` is provided, only those columns are fetched.
/// Returns `(column_names, rows)`.
pub async fn query_rows(
    pool: &DbPool,
    table: &str,
    filter: Option<&str>,
    sort_col: Option<&str>,
    sort_desc: bool,
    selected_cols: Option<&[String]>,
    limit: u32,
    offset: u32,
) -> color_eyre::eyre::Result<(Vec<String>, Vec<Vec<String>>)> {
    let order_by = sort_col.map(|col| {
        let dir = if sort_desc { "DESC" } else { "ASC" };
        format!("ORDER BY \"{}\" {dir}", col)
    });

    match pool {
        DbPool::Postgres(pg) => {
            let cols: Vec<String> = if let Some(cols) = selected_cols {
                cols.to_vec()
            } else {
                let col_rows: Vec<(String,)> = sqlx::query_as(
                    "SELECT column_name
                     FROM information_schema.columns
                     WHERE table_schema = 'public' AND table_name = $1
                     ORDER BY ordinal_position",
                )
                .bind(table)
                .fetch_all(pg)
                .await?;
                col_rows.into_iter().map(|(n,)| n).collect()
            };

            if cols.is_empty() {
                return Ok((vec![], vec![]));
            }

            let casts = cols
                .iter()
                .map(|c| format!("\"{}\"::text", c))
                .collect::<Vec<_>>()
                .join(", ");

            let mut query = format!("SELECT {casts} FROM \"{table}\"");
            if let Some(f) = filter {
                query.push_str(&format!(" WHERE {f}"));
            }
            if let Some(o) = &order_by {
                query.push_str(&format!(" {o}"));
            }
            query.push_str(&format!(" LIMIT {limit} OFFSET {offset}"));

            let rows = sqlx::query(&query).fetch_all(pg).await?;

            use sqlx::Row;
            let data = rows
                .iter()
                .map(|r| {
                    (0..cols.len())
                        .map(|i| {
                            r.try_get::<Option<String>, _>(i)
                                .map(|v| v.unwrap_or_else(|| "NULL".into()))
                                .unwrap_or_else(|_| "?".into())
                        })
                        .collect()
                })
                .collect();

            Ok((cols, data))
        }

        DbPool::Mysql(my) => {
            let cols: Vec<String> = if let Some(cols) = selected_cols {
                cols.to_vec()
            } else {
                let col_rows: Vec<(String,)> = sqlx::query_as(
                    "SELECT column_name
                     FROM information_schema.columns
                     WHERE table_schema = DATABASE() AND table_name = ?
                     ORDER BY ordinal_position",
                )
                .bind(table)
                .fetch_all(my)
                .await?;
                col_rows.into_iter().map(|(n,)| n).collect()
            };

            if cols.is_empty() {
                return Ok((vec![], vec![]));
            }

            let casts = cols
                .iter()
                .map(|c| format!("CONVERT(`{}`, CHAR)", c))
                .collect::<Vec<_>>()
                .join(", ");

            let mut query = format!("SELECT {casts} FROM `{table}`");
            if let Some(f) = filter {
                query.push_str(&format!(" WHERE {f}"));
            }
            if let Some(o) = &order_by {
                query.push_str(&format!(" {o}"));
            }
            query.push_str(&format!(" LIMIT {limit} OFFSET {offset}"));

            let rows = sqlx::query(&query).fetch_all(my).await?;

            use sqlx::Row;
            let data = rows
                .iter()
                .map(|r| {
                    (0..cols.len())
                        .map(|i| {
                            r.try_get::<Option<String>, _>(i)
                                .map(|v| v.unwrap_or_else(|| "NULL".into()))
                                .unwrap_or_else(|_| "?".into())
                        })
                        .collect()
                })
                .collect();

            Ok((cols, data))
        }

        DbPool::Sqlite(sq) => {
            let cols: Vec<String> = if let Some(cols) = selected_cols {
                cols.to_vec()
            } else {
                vec![]
            };

            let mut query = if cols.is_empty() {
                format!("SELECT * FROM \"{table}\"")
            } else {
                let col_list = cols
                    .iter()
                    .map(|c| format!("\"{}\"", c))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("SELECT {col_list} FROM \"{table}\"")
            };
            if let Some(f) = filter {
                query.push_str(&format!(" WHERE {f}"));
            }
            if let Some(o) = &order_by {
                query.push_str(&format!(" {o}"));
            }
            query.push_str(&format!(" LIMIT {limit} OFFSET {offset}"));

            let rows = sqlx::query(&query).fetch_all(sq).await?;

            use sqlx::Row;
            let returned_cols: Vec<String> = if cols.is_empty() {
                rows.first()
                    .map(|r| r.columns().iter().map(|c| c.name().to_string()).collect())
                    .unwrap_or_default()
            } else {
                cols.clone()
            };

            let data = rows
                .iter()
                .map(|r| {
                    (0..returned_cols.len())
                        .map(|i| sqlite_cell(r, i))
                        .collect()
                })
                .collect();

            Ok((returned_cols, data))
        }
    }
}

/// Return the row count for `table` with optional `filter`.
pub async fn count_rows_filtered(
    pool: &DbPool,
    table: &str,
    filter: Option<&str>,
) -> color_eyre::eyre::Result<i64> {
    match pool {
        DbPool::Postgres(pg) => {
            let mut query = format!("SELECT COUNT(*) FROM \"{table}\"");
            if let Some(f) = filter {
                query.push_str(&format!(" WHERE {f}"));
            }
            let (count,): (i64,) = sqlx::query_as(&query).fetch_one(pg).await?;
            Ok(count)
        }
        DbPool::Mysql(my) => {
            let mut query = format!("SELECT COUNT(*) FROM `{table}`");
            if let Some(f) = filter {
                query.push_str(&format!(" WHERE {f}"));
            }
            let (count,): (i64,) = sqlx::query_as(&query).fetch_one(my).await?;
            Ok(count)
        }
        DbPool::Sqlite(sq) => {
            let mut query = format!("SELECT COUNT(*) FROM \"{table}\"");
            if let Some(f) = filter {
                query.push_str(&format!(" WHERE {f}"));
            }
            let (count,): (i64,) = sqlx::query_as(&query).fetch_one(sq).await?;
            Ok(count)
        }
    }
}

/// Execute an arbitrary SQL query.
/// For SELECT-like queries, returns `(column_names, rows)`.
/// For DML queries (INSERT/UPDATE/DELETE), returns `(["Rows Affected"], [[count]])`.
/// For SELECTs returning 0 rows, returns `(headers, [])`.
pub async fn execute_query(
    pool: &DbPool,
    sql: &str,
) -> color_eyre::eyre::Result<(Vec<String>, Vec<Vec<String>>)> {
    let is_select = sql.trim().to_lowercase().starts_with("select");

    match pool {
        DbPool::Postgres(pg) => {
            if is_select {
                let clean = sql.trim().trim_end_matches(';');
                // First pass: discover column names so we can cast everything to text.
                let rows = sqlx::query(sql).fetch_all(pg).await?;
                if rows.is_empty() {
                    return Ok((vec![], vec![]));
                }
                use sqlx::Row;
                let cols: Vec<String> = rows[0]
                    .columns()
                    .iter()
                    .map(|c| c.name().to_string())
                    .collect();
                let casts = cols
                    .iter()
                    .map(|c| format!("\"{}\"::text", c))
                    .collect::<Vec<_>>()
                    .join(", ");
                let casted = format!("SELECT {casts} FROM ({clean}) AS _subquery");
                let rows = sqlx::query(&casted).fetch_all(pg).await?;
                let data = rows
                    .iter()
                    .map(|r| {
                        (0..cols.len())
                            .map(|i| {
                                r.try_get::<Option<String>, _>(i)
                                    .map(|v| v.unwrap_or_else(|| "NULL".into()))
                                    .unwrap_or_else(|_| "?".into())
                            })
                            .collect()
                    })
                    .collect();
                Ok((cols, data))
            } else {
                let result = sqlx::query(sql).execute(pg).await?;
                Ok((
                    vec!["Rows Affected".to_string()],
                    vec![vec![result.rows_affected().to_string()]],
                ))
            }
        }
        DbPool::Mysql(my) => {
            if is_select {
                let clean = sql.trim().trim_end_matches(';');
                let rows = sqlx::query(sql).fetch_all(my).await?;
                if rows.is_empty() {
                    return Ok((vec![], vec![]));
                }
                use sqlx::Row;
                let cols: Vec<String> = rows[0]
                    .columns()
                    .iter()
                    .map(|c| c.name().to_string())
                    .collect();
                let casts = cols
                    .iter()
                    .map(|c| format!("CONVERT(`{}`, CHAR)", c))
                    .collect::<Vec<_>>()
                    .join(", ");
                let casted = format!("SELECT {casts} FROM ({clean}) AS _subquery");
                let rows = sqlx::query(&casted).fetch_all(my).await?;
                let data = rows
                    .iter()
                    .map(|r| {
                        (0..cols.len())
                            .map(|i| {
                                r.try_get::<Option<String>, _>(i)
                                    .map(|v| v.unwrap_or_else(|| "NULL".into()))
                                    .unwrap_or_else(|_| "?".into())
                            })
                            .collect()
                    })
                    .collect();
                Ok((cols, data))
            } else {
                let result = sqlx::query(sql).execute(my).await?;
                Ok((
                    vec!["Rows Affected".to_string()],
                    vec![vec![result.rows_affected().to_string()]],
                ))
            }
        }
        DbPool::Sqlite(sq) => {
            if is_select {
                let rows = sqlx::query(sql).fetch_all(sq).await?;
                if rows.is_empty() {
                    return Ok((vec![], vec![]));
                }
                use sqlx::Row;
                let cols: Vec<String> = rows[0]
                    .columns()
                    .iter()
                    .map(|c| c.name().to_string())
                    .collect();
                let data = rows
                    .iter()
                    .map(|r| (0..cols.len()).map(|i| sqlite_cell(r, i)).collect())
                    .collect();
                Ok((cols, data))
            } else {
                let result = sqlx::query(sql).execute(sq).await?;
                Ok((
                    vec!["Rows Affected".to_string()],
                    vec![vec![result.rows_affected().to_string()]],
                ))
            }
        }
    }
}

use sqlx::Column;

fn sqlite_cell(row: &sqlx::sqlite::SqliteRow, i: usize) -> String {
    use sqlx::Row;
    row.try_get::<Option<String>, _>(i)
        .map(|v| v.unwrap_or_else(|| "NULL".into()))
        .or_else(|_| {
            row.try_get::<Option<i64>, _>(i)
                .map(|v| v.map(|n| n.to_string()).unwrap_or_else(|| "NULL".into()))
        })
        .or_else(|_| {
            row.try_get::<Option<f64>, _>(i)
                .map(|v| v.map(|n| n.to_string()).unwrap_or_else(|| "NULL".into()))
        })
        .unwrap_or_else(|_| "NULL".into())
}
