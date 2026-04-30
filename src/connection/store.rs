use std::fmt;
use std::io::Write;
use std::path::PathBuf;
use std::{fs, io};

use owo_colors::OwoColorize;
use tabled::settings::{Color, Modify, Style, object::Columns};
use tabled::{Table, Tabled};

use super::models::{Database, DatabaseStore};
use crate::cli::Engine;
use crate::connection::models::ConnectionSource;

/// Print a styled warning line to stderr:  ⚠  Warning: <msg>
pub(crate) fn warn(msg: impl fmt::Display) {
    eprintln!("{} {}", "⚠  Warning:".yellow().bold(), msg);
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
        warn(format!(
            "no connection named '{}' found — nothing was deleted. \
             Run `shellql db list` to see available connections.",
            name
        ));
        return Ok(());
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

    after_at
        .split_once('/')
        .map(|(host, _)| host)
        .unwrap_or(after_at)
        .to_string()
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
