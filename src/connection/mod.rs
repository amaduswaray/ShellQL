pub mod error;
pub mod models;
pub mod pool;
pub mod sql;
pub mod store;

pub use error::{ConnectionError, validate_connection_string};
pub use models::{
    ConnectionSource, Database, DatabaseConnection, DatabaseStore, DatabaseString, DbPool,
    Engine, MysqlConnection, PostgresConnection, SqliteConnection, SslOptions, SslVerifyMode,
};
pub use pool::connect_db;
pub use sql::{ColumnInfo, list_tables, table_schema, table_rows, count_rows, filter_rows, update_cell, insert_row, delete_rows};
pub use store::{
    add_connection, delete_connection, extract_host, get_config_path, list_connections,
    load_connections, load_connections_from, print_connections, save_connections,
    save_connections_to, update_connection,
};
