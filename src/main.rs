use std::io::{self, Write};

use anyhow::Context;
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
async fn main() -> anyhow::Result<()> {
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
                None => prompt_engine().context("Failed to read engine selection")?,
            };

            let db_name = match name {
                Some(n) => n,
                None => {
                    if is_interactive {
                        read_line("Database name", "Example Database")
                            .context("Failed to read database name")?
                    } else {
                        engine.to_string().to_lowercase()
                    }
                }
            };

            let connection_example = format!("{}://", engine);
            let url = match url {
                Some(u) => u,
                None => read_line("Connection URL", &connection_example)
                    .context("Failed to read connection URL")?,
            };

            let url = validate_connection_string(&url)
                .context("Invalid connection string")?
                .to_string();

            let source = match engine {
                Engine::Postgres => ConnectionSource::Url(DatabaseString::Postgres(url)),
                Engine::Mysql => ConnectionSource::Url(DatabaseString::Mysql(url)),
                Engine::Sqlite => ConnectionSource::Url(DatabaseString::Sqlite(url)),
            };

            let pool = connect_db(source, db_name)
                .await
                .context("Failed to connect to database")?;

            match pool {
                DbPool::Postgres(pg_pool) => {
                    let rows: Vec<_> = sqlx::query_as::<_, (String,)>("SELECT table_name FROM information_schema.tables WHERE table_schema = 'public' ORDER BY table_name")
                        .fetch_all(&pg_pool)
                        .await
                        .context("Failed to query tables — check your permissions and connection")?;

                    for (table_name,) in rows {
                        println!("{table_name}");
                    }
                }

                DbPool::Mysql(my_pool) => {
                    let rows: Vec<_> = sqlx::query_as::<_, (String,)>("SELECT table_name FROM information_schema.tables WHERE table_schema = DATABASE() ORDER BY table_name")
                        .fetch_all(&my_pool)
                        .await
                        .context("Failed to query tables — check your permissions and connection")?;

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
                    .context(
                        "Failed to query tables — check that the database file is accessible",
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
                    Ok(_) => {
                        print_connections();
                    }
                    Err(e) => {
                        eprintln!("Error: {e}");
                    }
                }
            }
            DbCommands::Delete { name } => match delete_connection(name.clone()) {
                Ok(_) => {
                    println!("Connection '{}' deleted.", name);
                    print_connections();
                }
                Err(e) => {
                    eprintln!("Error: Failed to delete connection: {e}");
                }
            },
        },

        None => {
            // TODO: Run the tui
            // Cli::parse_from(["shellql", "--help"]);
            println!("Hello darkness my old friend")
        }
    }

    Ok(())
}

fn prompt_engine() -> anyhow::Result<Engine> {
    let items = ["Postgres", "MySQL", "SQLite"];

    let selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Select database engine")
        .items(&items)
        .default(0)
        .interact()
        .context("Could not display engine selector — is this an interactive terminal?")?;

    Ok(match selection {
        0 => Engine::Postgres,
        1 => Engine::Mysql,
        2 => Engine::Sqlite,
        _ => unreachable!(),
    })
}

fn read_line(prompt: &str, initial: &str) -> anyhow::Result<String> {
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
        .context("Failed to write to terminal")?;

    let mut input = String::new();
    io::stdin()
        .read_line(&mut input)
        .context("Failed to read input — is stdin available?")?;

    let input = input
        .replace("\x1b[200~", "") // paste start marker
        .replace("\x1b[201~", ""); // paste end marker
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
        .context("Failed to write to terminal")?;

    Ok(result)
}
