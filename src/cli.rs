use clap::{Parser, Subcommand, ValueEnum};

#[derive(Debug, Clone, ValueEnum)]
pub enum Engine {
    Postgres,
    MySQL,
    SQLite,
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
    Connect {
        #[arg(long, short, value_enum, default_value_t = Engine::Postgres)]
        engine: Engine,

        #[arg(long, short)]
        url: Option<String>,

        #[arg(long, short)]
        name: Option<String>,
    },
    List,
}
