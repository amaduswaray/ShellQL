use clap::Parser;
use color_eyre::{
    Section,
    eyre::{Context, eyre},
};
use shellql::{
    cli::{
        commands::{Cli, Commands, DbCommands},
        prompt::{prompt_engine, read_line},
    },
    connection::{
        ConnectionSource, DatabaseString, DbPool, add_connection, delete_connection,
        models::Engine, print_connections, validate_connection_string,
    },
};

#[tokio::main]
async fn main() -> color_eyre::eyre::Result<()> {
    color_eyre::install()?;

    match Cli::parse().command {
        Some(Commands::Connect {
            interactive,
            engine,
            url,
            name,
        }) => {
            handle_connect(interactive, engine, url, name).await?;
        }

        Some(Commands::DB { command }) => {
            handle_db(command).await?;
        }

        None => {
            // TODO: launch TUI
            println!("Hello darkness my old friend");
        }
    }

    Ok(())
}

async fn handle_connect(
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

    let source = engine_to_source(engine.clone(), validated_url);

    match add_connection(db_name, source, engine).await {
        Ok(_) => print_connections(), // TODO: Go into TUI mode with the added connection
        Err(e) => {
            print_connections();
            return Err(eyre!(e));
        }
    }

    Ok(())
}

async fn handle_db(command: DbCommands) -> color_eyre::eyre::Result<()> {
    match command {
        DbCommands::List => print_connections(),

        DbCommands::Add { name, engine, url } => {
            let connection = engine_to_source(engine.clone(), url);
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

fn engine_to_source(engine: Engine, url: String) -> ConnectionSource {
    match engine {
        Engine::Postgres => ConnectionSource::Url(DatabaseString::Postgres(url)),
        Engine::Mysql => ConnectionSource::Url(DatabaseString::Mysql(url)),
        Engine::Sqlite => ConnectionSource::Url(DatabaseString::Sqlite(url)),
    }
}

async fn _print_tables(pool: DbPool) -> color_eyre::eyre::Result<()> {
    match pool {
        DbPool::Postgres(pg_pool) => {
            let rows: Vec<(String,)> = sqlx::query_as(
                "SELECT table_name \
                 FROM information_schema.tables \
                 WHERE table_schema = 'public' \
                 ORDER BY table_name",
            )
            .fetch_all(&pg_pool)
            .await
            .wrap_err("Failed to list tables")
            .suggestion(
                "Ensure your database user has SELECT privileges on information_schema.tables",
            )?;

            for (name,) in rows {
                println!("{name}");
            }
        }

        DbPool::Mysql(my_pool) => {
            let rows: Vec<(String,)> = sqlx::query_as(
                "SELECT table_name \
                 FROM information_schema.tables \
                 WHERE table_schema = DATABASE() \
                 ORDER BY table_name",
            )
            .fetch_all(&my_pool)
            .await
            .wrap_err("Failed to list tables")
            .suggestion(
                "Ensure your database user has SELECT privileges on information_schema.tables",
            )?;

            for (name,) in rows {
                println!("{name}");
            }
        }

        DbPool::Sqlite(sq_pool) => {
            let rows: Vec<(String,)> = sqlx::query_as(
                "SELECT name FROM sqlite_master WHERE type='table' ORDER BY name",
            )
            .fetch_all(&sq_pool)
            .await
            .wrap_err("Failed to list tables")
            .suggestion(
                "Ensure the SQLite database file exists and is not locked by another process",
            )?;

            for (name,) in rows {
                println!("{name}");
            }
        }
    }

    Ok(())
}
