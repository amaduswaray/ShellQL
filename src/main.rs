mod cli;
mod connection;
use std::io::{self, Write};

use anyhow::Context;

use crate::{
    cli::{Cli, Commands, DbCommands, Engine},
    connection::connect::{
        ConnectionSource, DatabaseString, DbPool, add_connection, connect_db, delete_connection,
        print_connections, validate_connection_string,
    },
};
use clap::Parser;
use dialoguer::{Input, Select, theme::ColorfulTheme};

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

            let engine = engine.unwrap_or_else(|| prompt_engine());

            let db_name = name.unwrap_or_else(|| {
                if is_interactive {
                    read_line("Database name", "Example Database")
                } else {
                    engine.to_string().to_lowercase()
                }
            });

            let connection_example = format!("{}://", engine);
            let url = url.unwrap_or_else(|| read_line("Connection URL", &connection_example));

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
                        .expect("Failed to execute test query");

                    for (table_name,) in rows {
                        println!("{table_name}");
                    }
                }

                DbPool::Mysql(my_pool) => {
                    let rows: Vec<_> = sqlx::query_as::<_, (String,)>("SELECT table_name FROM information_schema.tables WHERE table_schema = 'public' ORDER BY table_name")
                        .fetch_all(&my_pool)
                        .await
                        .expect("Failed to execute test query");

                    for (table_name,) in rows {
                        println!("{table_name}");
                    }
                }
                DbPool::Sqlite(sq_pool) => {
                    let rows: Vec<_> = sqlx::query_as::<_, (String,)>("SELECT table_name FROM information_schema.tables WHERE table_schema = 'public' ORDER BY table_name")
                        .fetch_all(&sq_pool)
                        .await
                        .expect("Failed to execute test query");

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
                        eprintln!("{e}");
                    }
                }
            }
            DbCommands::Delete { name } => match delete_connection(name.clone()) {
                Ok(_) => {
                    println!("Connection {} deleted.", name);
                    print_connections();
                }
                Err(e) => {
                    eprintln!("Failed to delete connection: {e}");
                }
            },
        },

        None => {
            // TODO: open up tui and interactive mode
            println!("Hello darkness my old friend, i must have called a thousand times");
        }
    }

    Ok(())
}

fn prompt_engine() -> Engine {
    let items = ["Postgres", "MySQL", "SQLite"];

    let selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Select database engine")
        .items(&items)
        .default(0)
        .interact()
        .unwrap();

    match selection {
        0 => Engine::Postgres,
        1 => Engine::Mysql,
        2 => Engine::Sqlite,
        _ => unreachable!(),
    }
}

fn read_line(prompt: &str, initial: &str) -> String {
    let theme = ColorfulTheme::default();

    eprint!(
        "{} {} {} {} ",
        theme.prompt_prefix,
        theme.prompt_style.apply_to(prompt),
        theme.hint_style.apply_to(format!("({})", initial)),
        theme.prompt_suffix,
    );
    io::stderr().flush().unwrap();

    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();
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
    io::stderr().flush().unwrap();

    result
}

// TODO: Fix this to work with tmux
// fn read_line(prompt: &str, initial: &str) -> String {
//     Input::<String>::with_theme(&ColorfulTheme::default())
//         .with_prompt(prompt)
//         .default(initial.to_string())
//         .interact_text()
//         .unwrap()
//         .trim()
//         .to_string()
// }
