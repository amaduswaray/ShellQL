use std::collections::HashMap;
use std::fmt::Display;
use std::io::Write;
use std::path::PathBuf;
use std::{fs, io};

use annotate_snippets::{AnnotationKind, Level, Renderer, Snippet};
use owo_colors::OwoColorize;
use serde::{Deserialize, Serialize};
use sqlx::mysql::MySqlPoolOptions;
use sqlx::postgres::PgPoolOptions;
use sqlx::sqlite::SqlitePoolOptions;
use sqlx::{MySqlPool, PgPool, SqlitePool};
use tabled::settings::{Color, Modify, Style, object::Columns};
use tabled::{Table, Tabled};
use url::Url;

use crate::cli::Engine;
use std::fmt;

static MAX_CONNECTIONS: u32 = 5;

fn warn(msg: impl fmt::Display) {
    eprintln!("{} {}", "⚠  Warning:".yellow().bold(), msg);
}

#[derive(Deserialize, Serialize, Default)]
pub struct DatabaseStore {
    pub databases: HashMap<String, Database>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Database {
    pub name: String,
    pub engine: Engine,
    pub connection: ConnectionSource,
}

impl Display for Database {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name)
    }
}

pub enum DbPool {
    Postgres(PgPool),
    Mysql(MySqlPool),
    Sqlite(SqlitePool),
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub enum ConnectionSource {
    Url(DatabaseString),
    Config(DatabaseConnection),
}

impl ConnectionSource {
    pub fn host(&self) -> String {
        match self {
            ConnectionSource::Url(url) => {
                let s = match url {
                    DatabaseString::Postgres(s) => s.as_str(),
                    DatabaseString::Mysql(s) => s.as_str(),
                    DatabaseString::Sqlite(s) => s.as_str(),
                };
                extract_host(s)
            }
            ConnectionSource::Config(config) => match config {
                DatabaseConnection::Postgres(c) => c.hostname.clone(),
                DatabaseConnection::Mysql(c) => c.hostname.clone(),
                DatabaseConnection::Sqlite(c) => c.path.clone(),
            },
        }
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub enum DatabaseString {
    Postgres(String),
    Mysql(String),
    Sqlite(String),
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub enum DatabaseConnection {
    Postgres(PostgresConnection),
    Mysql(MysqlConnection),
    Sqlite(SqliteConnection),
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
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

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
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

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
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

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub struct SslOptions {
    pub verify: SslVerifyMode,
    pub certfile: Option<String>,
}

#[derive(Debug)]
pub enum ConnectionError {
    /// The input string could not be parsed as a URL at all.
    InvalidUrl {
        input: String,
        error: url::ParseError,
    },
    /// The URL scheme is not one of the supported database drivers.
    UnsupportedScheme { input: String, scheme: String },
    /// A non-SQLite URL has no host component.
    MissingHost { input: String },
    /// The URL has no database-name path component (or only a bare `/`).
    MissingPath { input: String },
}

impl fmt::Display for ConnectionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let renderer = Renderer::styled();

        let output = match self {
            ConnectionError::InvalidUrl { input, error } => {
                let msg = error.to_string();
                let report = &[Level::ERROR
                    .primary_title("invalid connection string")
                    .element(
                        Snippet::source(input.as_str()).annotation(
                            AnnotationKind::Primary
                                .span(0..input.len())
                                .label(msg.as_str()),
                        ),
                    )
                    .element(Level::HELP.message(
                        "use a connection string in the form: \
                             postgres://user:pass@host/dbname",
                    ))];
                renderer.render(report).to_string()
            }

            ConnectionError::UnsupportedScheme { input, scheme } => {
                // Highlight the scheme portion (e.g. "http" in "http://...")
                let scheme_end = scheme.len();
                let report = &[Level::ERROR
                    .primary_title("unsupported database scheme")
                    .element(
                        Snippet::source(input.as_str()).annotation(
                            AnnotationKind::Primary
                                .span(0..scheme_end)
                                .label("this scheme is not supported"),
                        ),
                    )
                    .element(
                        Level::HELP
                            .message("supported schemes: postgres, postgresql, mysql, sqlite"),
                    )];
                renderer.render(report).to_string()
            }

            ConnectionError::MissingHost { input } => {
                // The host would appear immediately after "://"
                let after_scheme = input.find("://").map(|i| i + 3).unwrap_or(input.len());
                let span_end = (after_scheme + 1).min(input.len()).max(after_scheme);
                let report = &[Level::ERROR
                    .primary_title("missing host in connection string")
                    .element(
                        Snippet::source(input.as_str()).annotation(
                            AnnotationKind::Primary
                                .span(after_scheme..span_end)
                                .label("expected a hostname here"),
                        ),
                    )
                    .element(Level::HELP.message(
                        "use a connection string in the form: \
                             postgres://user:pass@host/dbname",
                    ))];
                renderer.render(report).to_string()
            }

            ConnectionError::MissingPath { input } => {
                let span_start = input.rfind('/').map(|i| i + 1).unwrap_or(input.len());
                // If span_start == input.len() the range is empty; pad by one for display
                let span_end = input.len().max(span_start + 1).min(input.len());
                let report = &[Level::ERROR
                    .primary_title("missing database name in connection string")
                    .element(
                        Snippet::source(input.as_str()).annotation(
                            AnnotationKind::Primary
                                .span(span_start..span_end)
                                .label("expected a database name after the last '/'"),
                        ),
                    )
                    .element(Level::HELP.message(
                        "use a connection string in the form: \
                             postgres://user:pass@host/dbname",
                    ))];
                renderer.render(report).to_string()
            }
        };

        write!(f, "{output}")
    }
}

impl std::error::Error for ConnectionError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ConnectionError::InvalidUrl { error, .. } => Some(error),
            _ => None,
        }
    }
}

pub fn validate_connection_string(conn: &str) -> Result<Url, ConnectionError> {
    let url = match Url::parse(conn) {
        Ok(u) => u,
        Err(e) => {
            return Err(ConnectionError::InvalidUrl {
                input: conn.to_string(),
                error: e,
            });
        }
    };

    match url.scheme() {
        "postgres" | "postgresql" | "mysql" | "sqlite" => {}
        other => {
            return Err(ConnectionError::UnsupportedScheme {
                input: conn.to_string(),
                scheme: other.to_string(),
            });
        }
    }

    match url.scheme() {
        "sqlite" => {}
        _ => {
            if url.host_str().is_none() {
                return Err(ConnectionError::MissingHost {
                    input: conn.to_string(),
                });
            }
        }
    }

    let path = url.path();
    if path.is_empty() || path == "/" {
        return Err(ConnectionError::MissingPath {
            input: conn.to_string(),
        });
    }

    Ok(url)
}
pub async fn connect_db(
    connection: ConnectionSource,
    name: String,
) -> color_eyre::eyre::Result<DbPool> {
    let pool = match &connection {
        ConnectionSource::Url(DatabaseString::Postgres(url)) => {
            let pool = PgPoolOptions::new()
                .max_connections(MAX_CONNECTIONS)
                .connect(url)
                .await?;

            add_connection(name, connection.clone(), Engine::Postgres)?;
            DbPool::Postgres(pool)
        }

        ConnectionSource::Url(DatabaseString::Mysql(url)) => {
            let pool = MySqlPoolOptions::new()
                .max_connections(MAX_CONNECTIONS)
                .connect(url)
                .await?;

            add_connection(name, connection.clone(), Engine::Mysql)?;
            DbPool::Mysql(pool)
        }

        ConnectionSource::Url(DatabaseString::Sqlite(url)) => {
            let pool = SqlitePoolOptions::new()
                .max_connections(MAX_CONNECTIONS)
                .connect(url)
                .await?;

            add_connection(name, connection.clone(), Engine::Sqlite)?;
            DbPool::Sqlite(pool)
        }

        ConnectionSource::Config(DatabaseConnection::Postgres(pg)) => {
            let url = generate_pg_connection_string(pg);

            let pool = PgPoolOptions::new()
                .max_connections(pg.pool_size as u32)
                .connect(&url)
                .await?;

            add_connection("postgres".to_string(), connection.clone(), Engine::Postgres)?;
            DbPool::Postgres(pool)
        }

        ConnectionSource::Config(DatabaseConnection::Mysql(my)) => {
            let url = generate_mysql_connection_string(my);

            let pool = MySqlPoolOptions::new()
                .max_connections(my.pool_size as u32)
                .connect(&url)
                .await?;

            add_connection("mysql".to_string(), connection.clone(), Engine::Mysql)?;
            DbPool::Mysql(pool)
        }

        ConnectionSource::Config(DatabaseConnection::Sqlite(sq)) => {
            let url = generate_sqlite_connection_string(sq);

            let pool = SqlitePoolOptions::new()
                .max_connections(sq.pool_size as u32)
                .connect(&url)
                .await?;

            add_connection("sqlite".to_string(), connection.clone(), Engine::Sqlite)?;
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

pub fn get_config_path() -> io::Result<PathBuf> {
    let mut path = dirs::config_dir().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::NotFound,
            "Could not locate system config directory. Set XDG_CONFIG_HOME or HOME.",
        )
    })?;
    path.push("shellql");
    fs::create_dir_all(&path)?;
    path.push(".connections.json");
    Ok(path)
}

pub fn load_connections() -> DatabaseStore {
    match get_config_path() {
        Ok(path) => load_connections_from(&path),
        Err(e) => {
            warn(format!("could not locate config directory: {e}"));
            DatabaseStore::default()
        }
    }
}

pub fn load_connections_from(path: &PathBuf) -> DatabaseStore {
    if !path.exists() {
        return DatabaseStore::default();
    }

    let data = match fs::read_to_string(path) {
        Ok(d) => d,
        Err(e) => {
            warn(format!("could not read connections file: {e}"));
            return DatabaseStore::default();
        }
    };

    match serde_json::from_str(&data) {
        Ok(store) => store,
        Err(e) => {
            warn(format!(
                "connections file appears corrupt and will be ignored ({e}). \
                Your saved connections may be missing. Check: {}",
                path.display()
            ));
            DatabaseStore::default()
        }
    }
}

pub fn save_connections(store: &DatabaseStore) -> io::Result<()> {
    let path = get_config_path()?;
    save_connections_to(store, &path)
}

pub fn save_connections_to(store: &DatabaseStore, path: &PathBuf) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let json = serde_json::to_string_pretty(store)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    let mut file = fs::File::create(path)?;
    file.write_all(json.as_bytes())
}

pub fn add_connection(
    name: String,
    connection: ConnectionSource,
    engine: Engine,
) -> io::Result<Database> {
    let path = get_config_path()?;
    let mut store = load_connections_from(&path);

    if store.databases.values().any(|db| db.name == name) {
        return Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            "Database name already exists. Try using a different name",
        ));
    }

    if store
        .databases
        .values()
        .any(|db| db.connection == connection)
    {
        return Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            "Database connection already exists. Try using a different connection string",
        ));
    }

    let db = Database {
        name,
        engine,
        connection,
    };

    store.databases.insert(db.name.clone(), db.clone());
    save_connections_to(&store, &path)?;

    Ok(db)
}

pub fn delete_connection(name: String) -> io::Result<()> {
    let path = get_config_path()?;
    let mut store = load_connections_from(&path);

    if !store.databases.contains_key(&name) {
        return Err(io::Error::new(io::ErrorKind::NotFound, "Name not found"));
    }

    store.databases.remove(&name);
    save_connections_to(&store, &path)
}

pub fn update_connection(updated: Database) -> io::Result<()> {
    let path = get_config_path()?;
    let mut store = load_connections_from(&path);

    if !store.databases.contains_key(&updated.name) {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            "Database not found",
        ));
    }

    store.databases.insert(updated.name.clone(), updated);
    save_connections_to(&store, &path)
}

pub fn list_connections() -> Vec<Database> {
    load_connections().databases.values().cloned().collect()
}

pub fn extract_host(url: &str) -> String {
    if url.starts_with("sqlite://") {
        return url.trim_start_matches("sqlite://").to_string();
    }

    let without_scheme = url.split_once("://").map(|(_, rest)| rest).unwrap_or(url);

    let after_at = without_scheme
        .rsplit_once('@')
        .map(|(_, rest)| rest)
        .unwrap_or(without_scheme);

    let host_and_port = after_at
        .split_once('/')
        .map(|(host, _)| host)
        .unwrap_or(after_at);

    host_and_port.to_string()
}

pub fn print_connections() {
    let dbs = list_connections();
    if dbs.is_empty() {
        println!("No databases configured.");
        return;
    }

    #[derive(Tabled)]
    struct ConnectionRow {
        #[tabled(rename = "NAME")]
        name: String,
        #[tabled(rename = "ENGINE")]
        engine: String,
        #[tabled(rename = "HOST")]
        host: String,
    }

    let rows: Vec<ConnectionRow> = dbs
        .into_iter()
        .map(|db| ConnectionRow {
            host: db.connection.host(),
            name: db.name,
            engine: db.engine.to_string(),
        })
        .collect();

    let table = Table::new(rows)
        .with(Style::modern_rounded())
        .with(Modify::new(Columns::first()).with(Color::BOLD))
        .to_string();

    println!("{table}");
}
