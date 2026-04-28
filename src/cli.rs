use std::fmt::{Display, Formatter, Result};

use clap::{Parser, Subcommand, ValueEnum};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, ValueEnum)]
pub enum Engine {
    Postgres,
    Mysql,
    Sqlite,
}

impl Display for Engine {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        let s = match self {
            Engine::Postgres => "postgres",
            Engine::Mysql => "mysql",
            Engine::Sqlite => "sqlite",
        };
        write!(f, "{s}")
    }
}

#[derive(Parser, Debug)]
#[command(name = "ShellQL")]
#[command(about = "Terminal DB management tool")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    #[command(
        about = "Connect to a database",
        long_about = "Establish a connection to a database using a specified engine and optional connection details."
    )]
    Connect {
        #[arg(long, short, value_enum, help = "Connect to DB in interactive mode")]
        interactive: bool,

        #[arg(
            long,
            short,
            value_enum,
            help = "Database engine to use (postgres, mysql, sqlite)",
            required_unless_present = "interactive"
        )]
        engine: Option<Engine>,

        #[arg(
            long,
            short,
            help = "Connection URL (overrides individual connection params)",
            required_unless_present = "interactive"
        )]
        url: Option<String>,

        #[arg(
            long,
            short,
            help = "Friendly name for this connection",
            required_unless_present = "interactive"
        )]
        name: Option<String>,
    },
    #[command(about = "Manage your database connections from the CLI")]
    DB {
        #[command(subcommand)]
        command: DbCommands,
    },
}

#[derive(Subcommand, Debug)]
pub enum DbCommands {
    #[command(about = "List all saved database connections")]
    List,

    #[command(about = "Add a new database connection")]
    Add {
        #[arg(long, short)]
        name: String,

        #[arg(long, short, value_enum)]
        engine: Engine,

        #[arg(long, short)]
        url: String,
    },

    #[command(about = "Remove a saved database connection")]
    Delete {
        #[arg(long, short)]
        name: String,
    },
}
