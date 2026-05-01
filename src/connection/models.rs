use std::collections::HashMap;
use std::fmt::{self, Display, Formatter, Result};

use clap::ValueEnum;
use serde::{Deserialize, Serialize};
use sqlx::{MySqlPool, PgPool, SqlitePool};

#[derive(Debug, Clone, Serialize, Deserialize, ValueEnum)]
pub enum Engine {
    Postgres,
    Mysql,
    Sqlite,
}

impl Display for Engine {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        let s = match self {
            Engine::Postgres => "postgres",
            Engine::Mysql => "mysql",
            Engine::Sqlite => "sqlite",
        };
        write!(f, "{s}")
    }
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
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
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
        use crate::connection::store::extract_host;
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
