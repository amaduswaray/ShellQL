/// Engine-agnostic query helpers used by the dashboard.
///
/// All functions accept a `&DbPool` and return owned data so callers
/// never need to import sqlx types directly.

use super::models::DbPool;

// ── Schema descriptor ─────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct ColumnInfo {
    pub name: String,
    pub data_type: String,
    pub nullable: bool,
    pub is_primary_key: bool,
}

// ── Table list ────────────────────────────────────────────────────────────────

/// Return all user-visible table names for the connected database,
/// ordered alphabetically.
pub async fn list_tables(pool: &DbPool) -> color_eyre::eyre::Result<Vec<String>> {
    match pool {
        DbPool::Postgres(pg) => {
            let rows: Vec<(String,)> = sqlx::query_as(
                "SELECT table_name
                 FROM information_schema.tables
                 WHERE table_schema = 'public'
                   AND table_type = 'BASE TABLE'
                 ORDER BY table_name",
            )
            .fetch_all(pg)
            .await?;
            Ok(rows.into_iter().map(|(n,)| n).collect())
        }
        DbPool::Mysql(my) => {
            let rows: Vec<(String,)> = sqlx::query_as(
                "SELECT table_name
                 FROM information_schema.tables
                 WHERE table_schema = DATABASE()
                   AND table_type = 'BASE TABLE'
                 ORDER BY table_name",
            )
            .fetch_all(my)
            .await?;
            Ok(rows.into_iter().map(|(n,)| n).collect())
        }
        DbPool::Sqlite(sq) => {
            let rows: Vec<(String,)> = sqlx::query_as(
                "SELECT name
                 FROM sqlite_master
                 WHERE type = 'table'
                 ORDER BY name",
            )
            .fetch_all(sq)
            .await?;
            Ok(rows.into_iter().map(|(n,)| n).collect())
        }
    }
}

// ── Schema ────────────────────────────────────────────────────────────────────

/// Return the ordered column definitions for `table`.
pub async fn table_schema(
    pool: &DbPool,
    table: &str,
) -> color_eyre::eyre::Result<Vec<ColumnInfo>> {
    match pool {
        DbPool::Postgres(pg) => {
            // Join with primary key info so we can mark PK columns.
            let rows: Vec<(String, String, String, Option<String>)> = sqlx::query_as(
                "SELECT c.column_name,
                        c.data_type,
                        c.is_nullable,
                        c.column_default
                 FROM information_schema.columns c
                 WHERE c.table_schema = 'public'
                   AND c.table_name   = $1
                 ORDER BY c.ordinal_position",
            )
            .bind(table)
            .fetch_all(pg)
            .await?;

            // Separately fetch primary key column names.
            let pk_rows: Vec<(String,)> = sqlx::query_as(
                "SELECT kcu.column_name
                 FROM information_schema.table_constraints tc
                 JOIN information_schema.key_column_usage kcu
                   ON tc.constraint_name = kcu.constraint_name
                  AND tc.table_schema    = kcu.table_schema
                 WHERE tc.constraint_type = 'PRIMARY KEY'
                   AND tc.table_schema    = 'public'
                   AND tc.table_name      = $1",
            )
            .bind(table)
            .fetch_all(pg)
            .await
            .unwrap_or_default();

            let pk_cols: std::collections::HashSet<String> =
                pk_rows.into_iter().map(|(n,)| n).collect();

            Ok(rows
                .into_iter()
                .map(|(name, data_type, nullable, _default)| ColumnInfo {
                    is_primary_key: pk_cols.contains(&name),
                    nullable: nullable == "YES",
                    name,
                    data_type,
                })
                .collect())
        }

        DbPool::Mysql(my) => {
            let rows: Vec<(String, String, String, String)> = sqlx::query_as(
                "SELECT column_name,
                        data_type,
                        is_nullable,
                        column_key
                 FROM information_schema.columns
                 WHERE table_schema = DATABASE()
                   AND table_name   = ?
                 ORDER BY ordinal_position",
            )
            .bind(table)
            .fetch_all(my)
            .await?;

            Ok(rows
                .into_iter()
                .map(|(name, data_type, nullable, key)| ColumnInfo {
                    nullable: nullable == "YES",
                    is_primary_key: key == "PRI",
                    name,
                    data_type,
                })
                .collect())
        }

        DbPool::Sqlite(sq) => {
            // PRAGMA table_info returns: cid, name, type, notnull, dflt_value, pk
            let rows: Vec<(i32, String, String, i32, Option<String>, i32)> =
                sqlx::query_as(&format!("PRAGMA table_info(\"{table}\")"))
                    .fetch_all(sq)
                    .await?;

            Ok(rows
                .into_iter()
                .map(|(_cid, name, data_type, notnull, _dflt, pk)| ColumnInfo {
                    nullable: notnull == 0,
                    is_primary_key: pk > 0,
                    name,
                    data_type,
                })
                .collect())
        }
    }
}

// ── Rows ──────────────────────────────────────────────────────────────────────

/// Fetch up to `limit` rows from `table`.
/// Returns `(column_names, rows)` where every cell is a display string.
pub async fn table_rows(
    pool: &DbPool,
    table: &str,
    limit: u32,
) -> color_eyre::eyre::Result<(Vec<String>, Vec<Vec<String>>)> {
    match pool {
        DbPool::Postgres(pg) => {
            // Cast every column to text at the SQL level to avoid type-decode
            // surprises for exotic Postgres types (UUID, JSONB, TIMESTAMPTZ, …).
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

            let query = format!("SELECT {casts} FROM \"{table}\" LIMIT {limit}");
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

            let query = format!("SELECT {casts} FROM `{table}` LIMIT {limit}");
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
            let query = format!("SELECT * FROM \"{table}\" LIMIT {limit}");
            let rows = sqlx::query(&query).fetch_all(sq).await?;

            use sqlx::Row;
            let cols: Vec<String> = rows
                .first()
                .map(|r| r.columns().iter().map(|c| c.name().to_string()).collect())
                .unwrap_or_default();

            let data = rows
                .iter()
                .map(|r| {
                    (0..cols.len())
                        .map(|i| sqlite_cell(r, i))
                        .collect()
                })
                .collect();

            Ok((cols, data))
        }
    }
}

use sqlx::Column;

/// Decode a SQLite cell to a display string by trying common type affinities.
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
