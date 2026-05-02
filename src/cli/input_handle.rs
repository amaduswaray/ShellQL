use color_eyre::{
    Section,
    eyre::{Context, eyre},
};

use crate::{
    cli::{
        commands::DbCommands,
        prompt::{prompt_engine, read_line},
    },
    connection::{
        add_connection, delete_connection, Engine, print_connections,
        validate_connection_string,
    },
};

pub async fn handle_connect(
    interactive: bool,
    engine: Option<Engine>,
    url: Option<String>,
    name: Option<String>,
) -> color_eyre::eyre::Result<()> {
    let is_interactive = interactive && (engine.is_none() || url.is_none() || name.is_none());

    let engine = match engine {
        Some(e) => e,
        None => prompt_engine()
            .wrap_err("Failed to read engine selection")
            .suggestion("Pass --engine <postgres|mysql|sqlite> to skip the interactive prompt")?,
    };

    let db_name = match name {
        Some(n) => n,
        None if is_interactive => read_line("Database name", "Example Database")
            .wrap_err("Failed to read database name")?,
        None => engine.to_string().to_lowercase(),
    };

    let raw_url = match url {
        Some(u) => u,
        None => read_line(
            "Connection URL",
            &format!("{}://user:pass@host/dbname", engine),
        )
        .wrap_err("Failed to read connection URL")?,
    };

    let validated_url = validate_connection_string(&raw_url)
        .wrap_err("Invalid connection string")
        .suggestion(format!(
            "Connection strings must look like: {}://user:pass@host/dbname",
            engine
        ))?
        .to_string();

    let source = engine.clone().to_source(validated_url);

    match add_connection(db_name, source, engine).await {
        Ok(_) => print_connections(), // TODO: Go into TUI mode with the added connection
        Err(e) => {
            print_connections();
            return Err(eyre!(e));
        }
    }

    Ok(())
}

pub async fn handle_db(command: DbCommands) -> color_eyre::eyre::Result<()> {
    match command {
        DbCommands::List => print_connections(),

        DbCommands::Add { name, engine, url } => {
            let connection = engine.clone().to_source(url);
            match add_connection(name, connection, engine).await {
                Ok(_) => print_connections(),
                Err(e) => {
                    print_connections();
                    return Err(eyre!(e));
                }
            }
        }

        DbCommands::Delete { name } => match delete_connection(name.clone()) {
            Ok(_) => {
                println!("Connection '{}' deleted.", name);
                print_connections();
            }
            Err(e) => {
                print_connections();
                return Err(
                    eyre!(e).suggestion("Run `shql db list` to verify the connection name exists")
                );
            }
        },
    }

    Ok(())
}
