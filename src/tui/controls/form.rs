use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::{
    connection::add_connection,
    tui::{AppState, TextMode},
};

pub async fn handle_form(event: KeyEvent, state: &mut AppState) -> color_eyre::Result<()> {
    if state.form.is_none() {
        return Ok(());
    }

    // Ctrl+S always submits regardless of mode or field.
    if event.modifiers.contains(KeyModifiers::CONTROL) && event.code == KeyCode::Char('s') {
        submit_form(state).await?;
        return Ok(());
    }

    let is_text = state.form.as_ref().unwrap().focused_field().is_text();
    let text_mode = state.form.as_ref().unwrap().text_mode.clone();

    match event.code {
        // ── Universal ───────────────────────────────────────────────────────────────

        // Esc: exit Insert → Normal; in Normal → close the form.
        KeyCode::Esc => {
            if is_text && text_mode == TextMode::Insert {
                state.form.as_mut().unwrap().enter_normal();
            } else {
                state.form = None;
                state.overlay = None;
            }
        }

        // Tab / Enter — advance field (always resets to Insert on arrival).
        KeyCode::Tab | KeyCode::Enter => {
            state.form.as_mut().unwrap().focus_next();
        }

        // Shift+Tab — go back.
        KeyCode::BackTab => {
            state.form.as_mut().unwrap().focus_prev();
        }

        // ── Text field keys ────────────────────────────────────────────────────────

        // Up / Down — field navigation, always, regardless of mode.
        KeyCode::Down => {
            state.form.as_mut().unwrap().focus_next();
        }
        KeyCode::Up => {
            state.form.as_mut().unwrap().focus_prev();
        }

        // Left / Right: cursor movement on text fields; cycle on selectors.
        KeyCode::Left => {
            let form = state.form.as_mut().unwrap();
            if is_text {
                form.cursor_left();
            } else {
                form.cycle_left();
            }
        }
        KeyCode::Right => {
            let form = state.form.as_mut().unwrap();
            if is_text {
                form.cursor_right();
            } else {
                form.cycle_right();
            }
        }

        // Backspace: only delete in Insert mode.
        KeyCode::Backspace if is_text && text_mode == TextMode::Insert => {
            state.form.as_mut().unwrap().delete_before_cursor();
        }

        // Space: insert on text fields (Insert mode) or toggle selectors.
        KeyCode::Char(' ') => {
            let form = state.form.as_mut().unwrap();
            if is_text && text_mode == TextMode::Insert {
                form.insert_char(' ');
            } else if !is_text {
                form.toggle_focused();
            }
        }

        // q in Normal mode — close the form without saving.
        KeyCode::Char('q') if text_mode == TextMode::Normal => {
            state.form = None;
            state.overlay = None;
        }

        // Character input.
        KeyCode::Char(c) => {
            let form = state.form.as_mut().unwrap();
            if is_text {
                match text_mode {
                    TextMode::Insert => form.insert_char(c),
                    TextMode::Normal => match c {
                        'i' => form.enter_insert_before(),
                        'a' => form.enter_insert_after(),
                        'I' => form.enter_insert_at_start(),
                        'A' => form.enter_insert_at_end(),
                        'h' => form.cursor_left(),
                        'l' => form.cursor_right(),
                        'j' => form.focus_next(),
                        'k' => form.focus_prev(),
                        '0' => form.cursor_to_start(),
                        '$' => form.cursor_to_end(),
                        'x' => form.delete_at_cursor(),
                        _ => {}
                    },
                }
            } else {
                // Selector / toggle fields: h/l cycle, j/k navigate in Normal mode.
                if text_mode == TextMode::Normal {
                    match c {
                        'h' => form.cycle_left(),
                        'l' => form.cycle_right(),
                        'j' => form.focus_next(),
                        'k' => form.focus_prev(),
                        _ => {}
                    }
                }
            }
        }

        _ => {}
    }

    Ok(())
}

async fn submit_form(state: &mut AppState) -> color_eyre::Result<()> {
    let Some(ref form) = state.form else {
        return Ok(());
    };

    // Validate before hitting the network.
    if let Err(e) = form.validate() {
        state.cmdline.set_error(e);
        return Ok(());
    }

    let name = form.name.trim().to_string();
    let engine = form.engine.clone();
    let source = match form.build_source() {
        Ok(s) => s,
        Err(e) => {
            state.cmdline.set_error(e);
            return Ok(());
        }
    };

    // add_connection tests the connection before persisting.
    match add_connection(name, source, engine).await {
        Ok(db) => {
            state.connections.push(db);
            state.selected_connection = state.connections.len() - 1;
            state.form = None;
            state.overlay = None;
        }
        Err(e) => {
            state.cmdline.set_error(e.to_string());
        }
    }

    Ok(())
}
