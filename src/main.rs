mod cli;
mod connection;
use dialoguer::{Input, Select};

use crate::{
    cli::{Cli, Commands, DbCommands, Engine},
    connection::connect::{
        ConnectionSource, DatabaseString, DbPool, add_connection, connect_db, delete_connection,
        print_connections,
    },
};
use clap::Parser;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Cli::parse();

    match args.command {
        Some(Commands::Connect { engine, url, name }) => {
            let is_interactive = url.is_none() && name.is_none();

            let engine = engine.unwrap_or_else(|| {
                if is_interactive {
                    prompt_engine()
                } else {
                    panic!("Engine is required unless using interactive mode");
                }
            });

            let db_name = name.unwrap_or_else(|| {
                if is_interactive {
                    read_line("Database name", "Example Database")
                } else {
                    engine.to_string().to_lowercase()
                }
            });

            let url = url.unwrap_or_else(|| {
                if is_interactive {
                    read_line("Connection URL", "postgresql://....")
                } else {
                    panic!("URL is required unless using interactive mode");
                }
            });

            let source = match engine {
                Engine::Postgres => ConnectionSource::Url(DatabaseString::Postgres(url)),
                Engine::Mysql => ConnectionSource::Url(DatabaseString::Mysql(url)),
                Engine::Sqlite => ConnectionSource::Url(DatabaseString::Sqlite(url)),
            };

            let pool = connect_db(source, db_name).await.unwrap();

            match pool {
                DbPool::Postgres(pg_pool) => {
                    let row: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM account")
                        .fetch_one(&pg_pool)
                        .await
                        .unwrap();
                    println!("Postgres connected: {}", row.0);
                }
                DbPool::Mysql(_) => println!("MySQL connected"),
                DbPool::Sqlite(_) => println!("SQLite connected"),
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

                let _ = add_connection(name, connection, engine);
                print_connections();
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

    let selection = Select::new()
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

// fn read_line(prompt: &str) -> String {
//     print!("{prompt}: ");
//     stdout().flush().unwrap();
//
//     let mut input = String::new();
//     stdin().read_line(&mut input).unwrap();
//
//     input.trim().to_string()
// }

// TODO: Use console package to have different colors of the option
fn read_line(prompt: &str, initial: &str) -> String {
    Input::<String>::new()
        .with_prompt(prompt)
        .default(initial.to_string())
        .interact_text()
        .unwrap()
}
