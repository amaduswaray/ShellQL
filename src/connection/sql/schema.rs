use super::super::models::DbPool;

// ── Schema descriptor ─────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct ColumnInfo {
    pub name: String,
    pub data_type: String,
    pub nullable: bool,
    pub is_primary_key: bool,
    pub default_value: Option<String>,
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
                .map(|(name, data_type, nullable, default)| ColumnInfo {
                    is_primary_key: pk_cols.contains(&name),
                    nullable: nullable == "YES",
                    default_value: default,
                    name,
                    data_type,
                })
                .collect())
        }

        DbPool::Mysql(my) => {
            let rows: Vec<(String, String, String, String, Option<String>)> = sqlx::query_as(
                "SELECT column_name,
                        data_type,
                        is_nullable,
                        column_key,
                        column_default
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
                .map(|(name, data_type, nullable, key, default)| ColumnInfo {
                    nullable: nullable == "YES",
                    is_primary_key: key == "PRI",
                    default_value: default,
                    name,
                    data_type,
                })
                .collect())
        }

        DbPool::Sqlite(sq) => {
            let rows: Vec<(i32, String, String, i32, Option<String>, i32)> =
                sqlx::query_as(&format!("PRAGMA table_info(\"{table}\")"))
                    .fetch_all(sq)
                    .await?;

            Ok(rows
                .into_iter()
                .map(|(_cid, name, data_type, notnull, dflt, pk)| ColumnInfo {
                    nullable: notnull == 0,
                    is_primary_key: pk > 0,
                    default_value: dflt,
                    name,
                    data_type,
                })
                .collect())
        }
    }
}
