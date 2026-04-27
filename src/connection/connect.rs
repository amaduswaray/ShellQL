use std::collections::HashMap;
use std::fmt::Display;
use std::io::Write;
use std::path::PathBuf;
use std::{fs, io};

use serde::{Deserialize, Serialize};
use sqlx::mysql::MySqlPoolOptions;
use sqlx::postgres::PgPoolOptions;
use sqlx::sqlite::SqlitePoolOptions;
use sqlx::{MySqlPool, PgPool, SqlitePool};
use url::Url;
use uuid::Uuid;

static MAX_CONNECTIONS: u32 = 5;

#[derive(Deserialize, Serialize, Default)]
pub struct DatabaseStore {
    pub databases: HashMap<Uuid, Database>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Database {
    pub id: Uuid,
    pub name: String,
    pub connection: ConnectionSource,
}

impl Display for Database {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name)
    }
}

// Runtime pool enum (not serialized)
pub enum DbPool {
    Postgres(PgPool),
    Mysql(MySqlPool),
    Sqlite(SqlitePool),
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq)]
pub enum ConnectionSource {
    Url(DatabaseString),
    Config(DatabaseConnection),
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq)]
pub enum DatabaseString {
    Postgres(String),
    Mysql(String),
    Sqlite(String),
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq)]
pub enum DatabaseConnection {
    Postgres(PostgresConnection),
    Mysql(MysqlConnection),
    Sqlite(SqliteConnection),
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct PostgresConnection {
    pub username: String,
    pub password: String,
    pub hostname: String,
    pub database: String,
    pub stack_trace: bool,
    pub port: i16,
    pub pool_size: i8,
    pub ssl: Option<SslOptions>,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct MysqlConnection {
    pub username: String,
    pub password: String,
    pub hostname: String,
    pub database: String,
    pub stack_trace: bool,
    pub port: i16,
    pub pool_size: i8,
    pub ssl: Option<SslOptions>,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct SqliteConnection {
    pub path: String,
    pub stack_trace: bool,
    pub pool_size: i8,
    pub create_if_missing: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum SslVerifyMode {
    None,
    Peer,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct SslOptions {
    pub verify: SslVerifyMode,
    pub certfile: Option<String>,
}

fn is_valid_connection_string(conn: &str) -> bool {
    let url = match Url::parse(conn) {
        Ok(u) => u,
        Err(_) => return false,
    };

    match url.scheme() {
        "postgres" | "postgresql" | "mysql" | "sqlite" => {}
        _ => return false,
    }

    if url.host_str().is_none() && url.scheme() != "sqlite" {
        return false;
    }

    let path = url.path();
    if path.is_empty() || path == "/" {
        return false;
    }

    true
}

pub async fn connect_db(connection: ConnectionSource, name: String) -> Result<DbPool, sqlx::Error> {
    let pool = match &connection {
        ConnectionSource::Url(DatabaseString::Postgres(url)) => {
            let pool = PgPoolOptions::new()
                .max_connections(MAX_CONNECTIONS)
                .connect(url)
                .await?;

            let _ = add_connection(name, connection.clone());

            DbPool::Postgres(pool)
        }

        ConnectionSource::Url(DatabaseString::Mysql(url)) => {
            let pool = MySqlPoolOptions::new()
                .max_connections(MAX_CONNECTIONS)
                .connect(url)
                .await?;

            let _ = add_connection(name, connection.clone());

            DbPool::Mysql(pool)
        }

        ConnectionSource::Url(DatabaseString::Sqlite(url)) => {
            let pool = SqlitePoolOptions::new()
                .max_connections(MAX_CONNECTIONS)
                .connect(url)
                .await?;

            let _ = add_connection(name, connection.clone());

            DbPool::Sqlite(pool)
        }

        ConnectionSource::Config(DatabaseConnection::Postgres(pg)) => {
            let url = generate_pg_connection_string(pg);

            let pool = PgPoolOptions::new()
                .max_connections(pg.pool_size as u32)
                .connect(&url)
                .await?;

            let _ = add_connection("postgres".to_string(), connection.clone());

            DbPool::Postgres(pool)
        }

        ConnectionSource::Config(DatabaseConnection::Mysql(my)) => {
            let url = generate_mysql_connection_string(my);

            let pool = MySqlPoolOptions::new()
                .max_connections(my.pool_size as u32)
                .connect(&url)
                .await?;

            let _ = add_connection("mysql".to_string(), connection.clone());

            DbPool::Mysql(pool)
        }

        ConnectionSource::Config(DatabaseConnection::Sqlite(sq)) => {
            let url = generate_sqlite_connection_string(sq);

            let pool = SqlitePoolOptions::new()
                .max_connections(sq.pool_size as u32)
                .connect(&url)
                .await?;

            let _ = add_connection("sqlite".to_string(), connection.clone());

            DbPool::Sqlite(pool)
        }
    };

    Ok(pool)
}

fn generate_pg_connection_string(pg: &PostgresConnection) -> String {
    let base = format!(
        "postgres://{}:{}@{}:{}/{}",
        pg.username, pg.password, pg.hostname, pg.port, pg.database
    );

    let mut params = vec![];

    if let Some(ssl) = &pg.ssl {
        let mode = match ssl.verify {
            SslVerifyMode::None => "disable",
            SslVerifyMode::Peer => "verify-full",
        };
        params.push(format!("sslmode={}", mode));

        if let Some(cert) = &ssl.certfile {
            params.push(format!("sslcert={}", cert));
        }
    }

    if pg.stack_trace {
        params.push("options=-c%20client_min_messages%3DLOG".to_string());
    }

    if params.is_empty() {
        base
    } else {
        format!("{}?{}", base, params.join("&"))
    }
}

fn generate_mysql_connection_string(my: &MysqlConnection) -> String {
    let base = format!(
        "mysql://{}:{}@{}:{}/{}",
        my.username, my.password, my.hostname, my.port, my.database
    );

    let mut params = vec![];

    if let Some(ssl) = &my.ssl {
        let mode = match ssl.verify {
            SslVerifyMode::None => "disabled",
            SslVerifyMode::Peer => "verify_ca",
        };
        params.push(format!("ssl-mode={}", mode));

        if let Some(cert) = &ssl.certfile {
            params.push(format!("ssl-ca={}", cert));
        }
    }

    if my.stack_trace {
        params.push("general_log=ON".to_string());
    }

    if params.is_empty() {
        base
    } else {
        format!("{}?{}", base, params.join("&"))
    }
}

fn generate_sqlite_connection_string(sq: &SqliteConnection) -> String {
    let mut params = vec![];

    if sq.create_if_missing {
        params.push("mode=rwc".to_string());
    }

    if sq.stack_trace {
        params.push("immutable=0".to_string());
    }

    if params.is_empty() {
        format!("sqlite://{}", sq.path)
    } else {
        format!("sqlite://{}?{}", sq.path, params.join("&"))
    }
}

pub fn get_config_path() -> PathBuf {
    let mut path = dirs::config_dir().expect("No config directory found");
    path.push("shellql");
    fs::create_dir_all(&path).ok();
    path.push(".connections.json");
    path
}

pub fn load_connections() -> DatabaseStore {
    let path = get_config_path();

    if !path.exists() {
        return DatabaseStore::default();
    }

    let data = fs::read_to_string(path).unwrap_or_default();
    serde_json::from_str(&data).unwrap_or_default()
}

pub fn save_connections(store: &DatabaseStore) -> io::Result<()> {
    let path = get_config_path();
    let json = serde_json::to_string_pretty(store).unwrap();
    let mut file = fs::File::create(path)?;
    file.write_all(json.as_bytes())
}

pub fn add_connection(name: String, connection: ConnectionSource) -> io::Result<Database> {
    let mut store = load_connections();

    if store.databases.values().any(|db| db.name == name) {
        return Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            "Database name already exists",
        ));
    }

    let db = Database {
        id: Uuid::new_v4(),
        name,
        connection,
    };

    store.databases.insert(db.id, db.clone());
    save_connections(&store)?;

    Ok(db)
}

pub fn delete_connection(id: Uuid) -> io::Result<()> {
    let mut store = load_connections();
    store.databases.remove(&id);
    save_connections(&store)
}

pub fn update_connection(updated: Database) -> io::Result<()> {
    let mut store = load_connections();

    if !store.databases.contains_key(&updated.id) {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            "Database not found",
        ));
    }

    if store
        .databases
        .values()
        .any(|db| db.name == updated.name && db.id != updated.id)
    {
        return Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            "Database name already exists",
        ));
    }

    store.databases.insert(updated.id, updated);
    save_connections(&store)
}

pub fn list_connections() -> Vec<Database> {
    load_connections().databases.values().cloned().collect()
}
