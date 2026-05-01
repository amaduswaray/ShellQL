use crate::connection::models::ConnectionSource;
use crate::connection::models::{
    DatabaseConnection, DatabaseString, DbPool, MysqlConnection, PostgresConnection,
    SqliteConnection, SslVerifyMode,
};

static MAX_CONNECTIONS: u32 = 5;

// TODO: Perhaps extrax the add connection - Two different operantions
pub async fn connect_db(connection: ConnectionSource) -> color_eyre::eyre::Result<DbPool> {
    use sqlx::mysql::MySqlPoolOptions;
    use sqlx::postgres::PgPoolOptions;
    use sqlx::sqlite::SqlitePoolOptions;

    let pool = match &connection {
        ConnectionSource::Url(DatabaseString::Postgres(url)) => {
            let pool = PgPoolOptions::new()
                .max_connections(MAX_CONNECTIONS)
                .connect(url)
                .await?;
            // add_connection(name, connection.clone(), Engine::Postgres).await?;
            DbPool::Postgres(pool)
        }

        ConnectionSource::Url(DatabaseString::Mysql(url)) => {
            let pool = MySqlPoolOptions::new()
                .max_connections(MAX_CONNECTIONS)
                .connect(url)
                .await?;
            // add_connection(name, connection.clone(), Engine::Mysql).await?;
            DbPool::Mysql(pool)
        }

        ConnectionSource::Url(DatabaseString::Sqlite(url)) => {
            let pool = SqlitePoolOptions::new()
                .max_connections(MAX_CONNECTIONS)
                .connect(url)
                .await?;
            // add_connection(name, connection.clone(), Engine::Sqlite).await?;
            DbPool::Sqlite(pool)
        }

        ConnectionSource::Config(DatabaseConnection::Postgres(pg)) => {
            let url = pg_url(pg);
            let pool = PgPoolOptions::new()
                .max_connections(pg.pool_size as u32)
                .connect(&url)
                .await?;
            // add_connection("postgres".to_string(), connection.clone(), Engine::Postgres).await?;
            DbPool::Postgres(pool)
        }

        ConnectionSource::Config(DatabaseConnection::Mysql(my)) => {
            let url = mysql_url(my);
            let pool = MySqlPoolOptions::new()
                .max_connections(my.pool_size as u32)
                .connect(&url)
                .await?;
            // add_connection("mysql".to_string(), connection.clone(), Engine::Mysql).await?;
            DbPool::Mysql(pool)
        }

        ConnectionSource::Config(DatabaseConnection::Sqlite(sq)) => {
            let url = sqlite_url(sq);
            let pool = SqlitePoolOptions::new()
                .max_connections(sq.pool_size as u32)
                .connect(&url)
                .await?;
            // add_connection("sqlite".to_string(), connection.clone(), Engine::Sqlite).await?;
            DbPool::Sqlite(pool)
        }
    };

    Ok(pool)
}

fn pg_url(pg: &PostgresConnection) -> String {
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
        params.push(format!("sslmode={mode}"));
        if let Some(cert) = &ssl.certfile {
            params.push(format!("sslcert={cert}"));
        }
    }

    if pg.stack_trace {
        params.push("options=-c%20client_min_messages%3DLOG".to_string());
    }

    build_url(base, params)
}

fn mysql_url(my: &MysqlConnection) -> String {
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
        params.push(format!("ssl-mode={mode}"));
        if let Some(cert) = &ssl.certfile {
            params.push(format!("ssl-ca={cert}"));
        }
    }

    if my.stack_trace {
        params.push("general_log=ON".to_string());
    }

    build_url(base, params)
}

fn sqlite_url(sq: &SqliteConnection) -> String {
    let mut params = vec![];

    if sq.create_if_missing {
        params.push("mode=rwc".to_string());
    }
    if sq.stack_trace {
        params.push("immutable=0".to_string());
    }

    build_url(format!("sqlite://{}", sq.path), params)
}

fn build_url(base: String, params: Vec<String>) -> String {
    if params.is_empty() {
        base
    } else {
        format!("{}?{}", base, params.join("&"))
    }
}
