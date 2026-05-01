use clap::builder::Styles;
use clap::builder::styling::AnsiColor;
use clap::{Parser, Subcommand};

use crate::connection::models::Engine;

const STYLES: Styles = Styles::styled()
    .header(AnsiColor::Yellow.on_default().bold())
    .usage(AnsiColor::Yellow.on_default().bold())
    .literal(AnsiColor::Green.on_default().bold())
    .placeholder(AnsiColor::Green.on_default())
    .error(AnsiColor::Red.on_default().bold())
    .valid(AnsiColor::Green.on_default())
    .invalid(AnsiColor::Red.on_default());

#[derive(Parser, Debug)]
#[command(name = "ShellQL")]
#[command(about = "Terminal DB management tool")]
#[command(styles = STYLES)]
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
