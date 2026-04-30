use std::io::{self, Write};

use color_eyre::{
    Section,
    eyre::{Context, eyre},
};
use clap::Parser;
use dialoguer::{Select, theme::ColorfulTheme};
use shellql::{
    cli::{Cli, Commands, DbCommands, Engine},
    connection::connect::{
        ConnectionSource, DatabaseString, DbPool, add_connection, connect_db, delete_connection,
        print_connections, validate_connection_string,
    },
};

#[tokio::main]
async fn main() -> color_eyre::eyre::Result<()> {
    color_eyre::install()?;

    let args = Cli::parse();

    match args.command {
        Some(Commands::Connect {
            interactive,
            engine,
            url,
            name,
        }) => {
            let is_interactive =
                interactive && (engine.is_none() || url.is_none() || name.is_none());

            let engine = match engine {
                Some(e) => e,
                None => prompt_engine()
                    .wrap_err("Failed to read engine selection")
                    .suggestion(
                        "Pass --engine <postgres|mysql|sqlite> to skip the interactive prompt",
                    )?,
            };

            let db_name = match name {
                Some(n) => n,
                None => {
                    if is_interactive {
                        read_line("Database name", "Example Database")
                            .wrap_err("Failed to read database name")?
                    } else {
                        engine.to_string().to_lowercase()
                    }
                }
            };

            let connection_example = format!("{}://user:pass@host/dbname", engine);
            let url = match url {
                Some(u) => u,
                None => read_line("Connection URL", &connection_example)
                    .wrap_err("Failed to read connection URL")?,
            };

            let url = validate_connection_string(&url)
                .wrap_err("Invalid connection string")
                .suggestion(format!(
                    "Connection strings must look like: {}://user:pass@host/dbname",
                    engine
                ))?
                .to_string();

            let source = match engine {
                Engine::Postgres => ConnectionSource::Url(DatabaseString::Postgres(url)),
                Engine::Mysql => ConnectionSource::Url(DatabaseString::Mysql(url)),
                Engine::Sqlite => ConnectionSource::Url(DatabaseString::Sqlite(url)),
            };

            let pool = connect_db(source, db_name)
                .await
                .wrap_err("Failed to connect to the database")
                .suggestion(
                    "Check that the host is reachable, your credentials are correct, \
                     and the database exists",
                )?;

            match pool {
                DbPool::Postgres(pg_pool) => {
                    let rows: Vec<_> = sqlx::query_as::<_, (String,)>(
                        "SELECT table_name \
                         FROM information_schema.tables \
                         WHERE table_schema = 'public' \
                         ORDER BY table_name",
                    )
                    .fetch_all(&pg_pool)
                    .await
                    .wrap_err("Failed to list tables")
                    .suggestion(
                        "Ensure your database user has SELECT privileges on \
                         information_schema.tables",
                    )?;

                    for (table_name,) in rows {
                        println!("{table_name}");
                    }
                }

                DbPool::Mysql(my_pool) => {
                    let rows: Vec<_> = sqlx::query_as::<_, (String,)>(
                        "SELECT table_name \
                         FROM information_schema.tables \
                         WHERE table_schema = DATABASE() \
                         ORDER BY table_name",
                    )
                    .fetch_all(&my_pool)
                    .await
                    .wrap_err("Failed to list tables")
                    .suggestion(
                        "Ensure your database user has SELECT privileges on \
                         information_schema.tables",
                    )?;

                    for (table_name,) in rows {
                        println!("{table_name}");
                    }
                }

                DbPool::Sqlite(sq_pool) => {
                    let rows: Vec<_> = sqlx::query_as::<_, (String,)>(
                        "SELECT name FROM sqlite_master WHERE type='table' ORDER BY name",
                    )
                    .fetch_all(&sq_pool)
                    .await
                    .wrap_err("Failed to list tables")
                    .suggestion(
                        "Ensure the SQLite database file exists and is not locked by \
                         another process",
                    )?;

                    for (table_name,) in rows {
                        println!("{table_name}");
                    }
                }
            }
        }

        Some(Commands::DB { command }) => match command {
            DbCommands::List => print_connections(),

            DbCommands::Add { name, engine, url } => {
                let connection = match engine {
                    Engine::Postgres => ConnectionSource::Url(DatabaseString::Postgres(url)),
                    Engine::Mysql => ConnectionSource::Url(DatabaseString::Mysql(url)),
                    Engine::Sqlite => ConnectionSource::Url(DatabaseString::Sqlite(url)),
                };

                match add_connection(name, connection, engine) {
                    Ok(_) => print_connections(),
                    Err(e) => {
                        let report = eyre!(e)
                            .suggestion(
                                "Run `shellql db list` to see all existing connection names \
                                 and URLs",
                            );
                        return Err(report);
                    }
                }
            }

            DbCommands::Delete { name } => match delete_connection(name.clone()) {
                Ok(_) => {
                    println!("Connection '{}' deleted.", name);
                    print_connections();
                }
                Err(e) => {
                    let report = eyre!(e)
                        .wrap_err(format!("Failed to delete connection '{name}'"))
                        .suggestion(
                            "Run `shellql db list` to verify the connection name exists",
                        );
                    return Err(report);
                }
            },
        },

        None => {
            // TODO: launch TUI
            println!("Hello darkness my old friend");
        }
    }

    Ok(())
}

fn prompt_engine() -> color_eyre::eyre::Result<Engine> {
    let items = ["Postgres", "MySQL", "SQLite"];

    let selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Select database engine")
        .items(&items)
        .default(0)
        .interact()
        .wrap_err("Could not display engine selector")
        .suggestion("Pass --engine <postgres|mysql|sqlite> to skip interactive mode")?;

    Ok(match selection {
        0 => Engine::Postgres,
        1 => Engine::Mysql,
        2 => Engine::Sqlite,
        _ => unreachable!(),
    })
}

fn read_line(prompt: &str, initial: &str) -> color_eyre::eyre::Result<String> {
    let theme = ColorfulTheme::default();

    eprint!(
        "{} {} {} {} ",
        theme.prompt_prefix,
        theme.prompt_style.apply_to(prompt),
        theme.hint_style.apply_to(format!("({})", initial)),
        theme.prompt_suffix,
    );
    io::stderr()
        .flush()
        .wrap_err("Failed to write to terminal")?;

    let mut input = String::new();
    io::stdin()
        .read_line(&mut input)
        .wrap_err("Failed to read input")
        .suggestion("Make sure stdin is connected to an interactive terminal")?;

    let input = input
        .replace("\x1b[200~", "")
        .replace("\x1b[201~", "");
    let input = input.trim();

    let result = if input.is_empty() {
        initial.to_string()
    } else {
        input.to_string()
    };

    eprint!(
        "\x1b[1A\x1b[2K{} {} {} {}\n",
        theme.success_prefix,
        theme.prompt_style.apply_to(prompt),
        theme.success_suffix,
        theme.values_style.apply_to(&result),
    );
    io::stderr()
        .flush()
        .wrap_err("Failed to write to terminal")?;

    Ok(result)
}
