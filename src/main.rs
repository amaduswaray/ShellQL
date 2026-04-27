mod cli;
mod connection;

use crate::{
    cli::{Cli, Commands, Engine},
    connection::connect::{ConnectionSource, DatabaseString, DbPool, connect_db},
};
use clap::Parser;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Cli::parse();

    match args.command {
        Some(Commands::Connect { engine, url }) => {
            if let Some(url) = url {
                match engine {
                    Engine::Postgres => {
                        let source = ConnectionSource::Url(DatabaseString::Postgres(url));
                        let pool = connect_db(source).await.unwrap();
                        if let DbPool::Postgres(pg_pool) = pool {
                            let row: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM account")
                                .fetch_one(&pg_pool)
                                .await
                                .unwrap();
                            println!("Count: {}", row.0);
                        }
                    }

                    Engine::MySQL => {
                        let source = ConnectionSource::Url(DatabaseString::Mysql(url));
                        let pool = connect_db(source).await.unwrap();
                        if let DbPool::Mysql(my_pool) = pool {
                            let row: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM account")
                                .fetch_one(&my_pool)
                                .await
                                .unwrap();
                            println!("Count: {}", row.0);
                        }
                    }

                    _ => {} // TODO: Implement the other engines
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
