pub mod error;
pub mod models;
pub mod pool;
pub mod sql;
pub mod store;

pub use error::{ConnectionError, validate_connection_string};
pub use models::{
    ConnectionSource, Database, DatabaseConnection, DatabaseStore, DatabaseString, DbPool, Engine,
    MysqlConnection, PostgresConnection, SqliteConnection, SslOptions, SslVerifyMode,
};
pub use pool::{build_sqlite_url, connect_db, normalize_sqlite_path};
pub use sql::{
    ColumnInfo, count_rows, count_rows_filtered, delete_rows, execute_query, filter_rows,
    insert_row, list_tables, query_rows, table_rows, table_schema, update_cell,
};
pub use store::{
    add_connection, delete_connection, extract_host, get_config_path, list_connections,
    load_connections, load_connections_from, print_connections, save_connections,
    save_connections_to, update_connection,
};
