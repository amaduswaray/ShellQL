use clap::Parser;
use shellql::{
    cli::{
        commands::{Cli, Commands},
        input_handle::{handle_connect, handle_db},
    },
    tui::app::run_app,
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

        None => run_app().await?,
    }

    Ok(())
}
