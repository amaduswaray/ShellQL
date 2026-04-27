use serde::{Deserialize, Serialize};
use sqlx::mysql::MySqlPoolOptions;
use sqlx::postgres::PgPoolOptions;
use sqlx::{MySqlPool, PgPool};

static MAX_CONNECTIONS: u32 = 5;

pub enum DbPool {
    Postgres(PgPool),
    Mysql(MySqlPool),
}

pub enum ConnectionSource {
    Url(DatabaseString),
    Config(DatabaseConnection),
}

#[derive(Serialize, Deserialize, Clone)]
pub enum DatabaseString {
    Postgres(String),
    Mysql(String),
}

#[derive(Serialize, Deserialize, Clone)]
pub enum DatabaseConnection {
    Postgres(PostgresConnection),
    Mysql(MysqlConnection),
}

#[derive(Serialize, Deserialize, Clone)]
pub struct PostgresConnection {
    pub name: String,
    pub username: String,
    pub password: String,
    pub hostname: String,
    pub database: String,
    pub stack_trace: bool,
    pub port: i16,
    pub pool_size: i8,
    pub ssl: Option<SslOptions>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct MysqlConnection {
    pub name: String,
    pub username: String,
    pub password: String,
    pub hostname: String,
    pub database: String,
    pub stack_trace: bool,
    pub port: i16,
    pub pool_size: i8,
    pub ssl: Option<SslOptions>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum SslVerifyMode {
    None,
    Peer,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct SslOptions {
    pub verify: SslVerifyMode,
    pub certfile: Option<String>,
}

pub async fn connect_db(connection: ConnectionSource) -> Result<DbPool, sqlx::Error> {
    let pool = match connection {
        ConnectionSource::Url(DatabaseString::Postgres(url)) => {
            let pool = PgPoolOptions::new()
                .max_connections(MAX_CONNECTIONS)
                .connect(&url)
                .await?;

            DbPool::Postgres(pool)
        }

        ConnectionSource::Url(DatabaseString::Mysql(url)) => {
            let pool = MySqlPoolOptions::new()
                .max_connections(MAX_CONNECTIONS)
                .connect(&url)
                .await?;

            DbPool::Mysql(pool)
        }

        ConnectionSource::Config(DatabaseConnection::Postgres(pg)) => {
            let url = generate_pg_connection_string(&pg);

            let pool = PgPoolOptions::new()
                .max_connections(pg.pool_size as u32)
                .connect(&url)
                .await?;

            DbPool::Postgres(pool)
        }

        ConnectionSource::Config(DatabaseConnection::Mysql(my)) => {
            let url = generate_mysql_connection_string(&my);

            let pool = PgPoolOptions::new()
                .max_connections(my.pool_size as u32)
                .connect(&url)
                .await?;

            DbPool::Postgres(pool)
        }
    };

    Ok(pool)
}

fn generate_pg_connection_string(pg: &PostgresConnection) -> String {
    let base_url = format!(
        "postgres://{}:{}@{}:{}/{}",
        pg.username, pg.password, pg.hostname, pg.port, pg.database
    );

    let mut params: Vec<String> = Vec::new();

    if let Some(ssl) = &pg.ssl {
        let ssl_mode = match ssl.verify {
            SslVerifyMode::None => "disable",
            SslVerifyMode::Peer => "verify-full",
        };
        params.push(format!("sslmode={}", ssl_mode));

        if let Some(certfile) = &ssl.certfile {
            params.push(format!("sslcert={}", certfile));
        }
    }

    if pg.stack_trace {
        params.push("options=-c%20client_min_messages%3DLOG".to_string());
    }

    let url = if params.is_empty() {
        base_url
    } else {
        format!("{}?{}", base_url, params.join("&"))
    };
    url
}
fn generate_mysql_connection_string(my: &MysqlConnection) -> String {
    let base_url = format!(
        "mysql://{}:{}@{}:{}/{}",
        my.username, my.password, my.hostname, my.port, my.database
    );
    let mut params: Vec<String> = Vec::new();

    if let Some(ssl) = &my.ssl {
        let ssl_mode = match ssl.verify {
            SslVerifyMode::None => "disabled",
            SslVerifyMode::Peer => "verify_ca",
        };
        params.push(format!("ssl-mode={}", ssl_mode));
        if let Some(certfile) = &my.ssl.as_ref().unwrap().certfile {
            params.push(format!("ssl-ca={}", certfile));
        }
    }

    if my.stack_trace {
        params.push("general_log=ON".to_string());
    }

    if params.is_empty() {
        base_url
    } else {
        format!("{}?{}", base_url, params.join("&"))
    }
}

// #[derive(Default)]
// pub struct ConnectionStore {
//     pub postgres: Vec<PgConnection>,
// }
//
// pub fn get_config_path() -> PathBuf {
//     let mut path = dirs::config_dir().expect("No config directory found");
//     path.push("shellql");
//     std::fs::create_dir_all(&path).ok();
//     path.push(".connections.json");
//     path
// }
//
// pub fn load_connections() -> ConnectionStore {
//     let path = get_config_path();
//
//     if !path.exists() {
//         return ConnectionStore::default();
//     }
//
//     let data = fs::read_to_string(path).unwrap_or_default();
//
//     serde_json::from_str(&data).unwrap_or_default()
// }
//
// pub fn save_connections(store: &ConnectionStore) -> io::Result<()> {
//     let path = get_config_path();
//
//     let json = serde_json::to_string_pretty(store).unwrap();
//
//     let mut file = fs::File::create(path)?;
//     file.write_all(json.as_bytes())?;
//
//     Ok(())
// }
//
// pub fn add_connection(new_conn: PgConnection) -> io::Result<()> {
//     let mut store = load_connections();
//
//     store.postgres.push(new_conn);
//
//     save_connections(&store)
// }
//
// pub fn delete_connection(name: &str) -> io::Result<()> {
//     let mut store = load_connections();
//
//     store.postgres.retain(|c| c.name != name);
//
//     save_connections(&store)
// }
//
// pub fn update_connection(updated: PgConnection) -> io::Result<()> {
//     let mut store = load_connections();
//
//     if let Some(conn) = store.postgres.iter_mut().find(|c| c.name == updated.name) {
//         *conn = updated;
//     }
//
//     save_connections(&store)
// }
//
// pub fn list_connections() -> Vec<PgConnection> {
//     load_connections().postgres
// }
