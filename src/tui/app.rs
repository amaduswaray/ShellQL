use crate::tui::{
    AppState,
    controls::handle_key_event,
    render::render,
    state::AppMode,
};
use crossterm::{
    event::{self, Event},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, prelude::CrosstermBackend};
use std::{io::stdout, panic::AssertUnwindSafe, time::Duration};

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
        // Render with a safety net: if the render thread panics (e.g. due to a
        // dimension underflow), catch it, show an error, and keep the app alive.
        if let Err(panic_info) = std::panic::catch_unwind(AssertUnwindSafe(|| {
            terminal.draw(|f| render(f, state)).ok();
        })) {
            let msg = if let Some(s) = panic_info.downcast_ref::<String>() {
                format!("Render panic: {s}")
            } else if let Some(s) = panic_info.downcast_ref::<&str>() {
                format!("Render panic: {s}")
            } else {
                "Render panic: unknown".to_string()
            };
            state.cmdline.set_error(msg);
            // Force a minimal redraw so the error bar is visible.
            let _ = terminal.draw(|f| {
                let area = f.area();
                let line = ratatui::text::Line::from(ratatui::text::Span::styled(
                    " Render error — check cmdline ",
                    ratatui::style::Style::default().fg(ratatui::style::Color::Red),
                ));
                f.render_widget(
                    ratatui::widgets::Paragraph::new(vec![line]),
                    area,
                );
            });
        }

        // Async connection with spinner
        if state.pending_connection.is_some() {
            handle_pending_connection(state, &mut terminal).await?;
            continue;
        }

        tokio::select! {
            _ = tokio::signal::ctrl_c() => {
                state.should_quit = true;
            }
            ready = async { event::poll(Duration::from_millis(50)) } => {
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

async fn handle_pending_connection(
    state: &mut AppState,
    terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
) -> color_eyre::Result<()> {
    let Some(db) = state.pending_connection.take() else {
        return Ok(());
    };

    let mut interval = tokio::time::interval(Duration::from_millis(100));
    let mut spinner_frame = 0usize;
    const SPINNER: [char; 10] = ['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];

    let conn = db.connection.clone();
    let connect_fut = async move {
        let pool = crate::connection::connect_db(conn).await?;
        let tables = crate::connection::list_tables(&pool).await?;
        Ok::<_, color_eyre::eyre::Error>((pool, tables))
    };
    tokio::pin!(connect_fut);

    loop {
        tokio::select! {
            biased;

            _ = tokio::signal::ctrl_c() => {
                state.should_quit = true;
                break;
            }

            result = &mut connect_fut => {
                match result {
                    Ok((pool, tables)) => {
                        state.connection = Some(db);
                        state.pool = Some(pool);
                        state.tables = tables;
                        state.table_cache = std::collections::HashMap::new();
                        state.tabs = vec![crate::tui::state::Tab::new()];
                        state.active_tab = 0;
                        state.mode = AppMode::Dashboard;
                        state.cmdline.clear_loading();
                    }
                    Err(e) => {
                        state.cmdline.set_error(format!("Connection failed: {e}"));
                    }
                }
                break;
            }

            _ = interval.tick() => {
                let ch = SPINNER[spinner_frame % SPINNER.len()];
                state.cmdline.set_loading(format!("{ch}  Connecting to {}...", db.name));
                spinner_frame += 1;
                let _ = std::panic::catch_unwind(AssertUnwindSafe(|| {
                    terminal.draw(|f| render(f, state)).ok();
                }));
            }
        }
    }

    Ok(())
}
