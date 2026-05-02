use crate::tui::{controls::handle_key_event, render::render, AppState};
use crossterm::{
    event::{self, Event},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, prelude::CrosstermBackend};
use std::{io::stdout, time::Duration};

pub async fn run_app() -> color_eyre::Result<()> {
    let mut state = AppState::new();

    setup_terminal()?;
    let result = app(&mut state).await;
    restore_terminal()?;

    result
}

async fn app(state: &mut AppState) -> color_eyre::Result<()> {
    let backend = CrosstermBackend::new(stdout());
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;
    loop {
        terminal.draw(|f| render(f, state))?;

        tokio::select! {
            _ = tokio::signal::ctrl_c() => {
                state.should_quit = true;
            }
            ready = async { event::poll(Duration::from_millis(100)) } => {
                if ready? {
                    if let Event::Key(key) = event::read()? {
                        handle_key_event(key, state).await?;
                    }
                }
            }
        }

        if state.should_quit {
            break;
        }
    }
    Ok(())
}

fn setup_terminal() -> color_eyre::Result<()> {
    enable_raw_mode()?;
    execute!(stdout(), EnterAlternateScreen)?;
    Ok(())
}

fn restore_terminal() -> color_eyre::Result<()> {
    execute!(stdout(), LeaveAlternateScreen)?;
    disable_raw_mode()?;
    Ok(())
}
