mod cli;
mod connection;

use crate::{
    cli::{Cli, Commands, Engine},
    connection::connect::{ConnectionSource, DatabaseString, DbPool, connect_db, list_connections},
};
use clap::Parser;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Cli::parse();

    match args.command {
        Some(Commands::Connect { engine, url, name }) => {
            if let Some(url) = url {
                match engine {
                    Engine::Postgres => {
                        let db_name = name.unwrap_or_else(|| "postgres".to_string());
                        let source = ConnectionSource::Url(DatabaseString::Postgres(url));
                        let pool = connect_db(source, db_name).await.unwrap();

                        if let DbPool::Postgres(pg_pool) = pool {
                            let row: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM account")
                                .fetch_one(&pg_pool)
                                .await
                                .unwrap();
                            println!("Count: {}", row.0);
                        }
                    }

                    Engine::MySQL => {
                        let db_name = name.unwrap_or_else(|| "mysql".to_string());
                        let source = ConnectionSource::Url(DatabaseString::Mysql(url));
                        let pool = connect_db(source, db_name).await.unwrap();

                        if let DbPool::Mysql(_) = pool {
                            println!("MySQL connected");
                        }
                    }

                    Engine::SQLite => {
                        let db_name = name.unwrap_or_else(|| "sqlite".to_string());
                        let source = ConnectionSource::Url(DatabaseString::Sqlite(url));
                        let pool = connect_db(source, db_name).await.unwrap();

                        if let DbPool::Sqlite(_) = pool {
                            println!("SQLite connected");
                        }
                    }
                }
            }
        }
        Some(Commands::List) => {
            let dbs = list_connections();

            if dbs.is_empty() {
                println!("No databases configured.");
            } else {
                // 1. Print the Header
                println!("{:<20}", "DATABASE NAME");
                println!("{}", "-".repeat(20));

                // 2. Print the Rows
                for db in dbs {
                    // {:<20} left-aligns the name in a 20-character wide "cell"
                    println!("{:<20}", db.name);
                }
            }
        }

        None => {
            // TODO: open up tui and interactive mode
            println!("Hello darkness my old friend, i must have called a thousand times");
        }
    }

    Ok(())
}
