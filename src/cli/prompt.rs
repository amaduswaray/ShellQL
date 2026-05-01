use std::io::{self, Write};

use color_eyre::Section;
use color_eyre::eyre::WrapErr;
use dialoguer::{Select, theme::ColorfulTheme};

use crate::connection::models::Engine;

pub fn prompt_engine() -> color_eyre::eyre::Result<Engine> {
    let items = ["Postgres", "MySQL", "SQLite"];

    let selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Select database engine")
        .items(&items)
        .default(0)
        .interact()
        .wrap_err("Could not display engine selector")
        .suggestion("Pass --engine <postgres|mysql|sqlite> to skip interactive mode")?;

    Ok(match selection {
        0 => Engine::Postgres,
        1 => Engine::Mysql,
        2 => Engine::Sqlite,
        _ => unreachable!(),
    })
}

pub fn read_line(prompt: &str, initial: &str) -> color_eyre::eyre::Result<String> {
    let theme = ColorfulTheme::default();

    eprint!(
        "{} {} {} {} ",
        theme.prompt_prefix,
        theme.prompt_style.apply_to(prompt),
        theme.hint_style.apply_to(format!("({})", initial)),
        theme.prompt_suffix,
    );
    io::stderr()
        .flush()
        .wrap_err("Failed to write to terminal")?;

    let mut input = String::new();
    io::stdin()
        .read_line(&mut input)
        .wrap_err("Failed to read input")
        .suggestion("Make sure stdin is connected to an interactive terminal")?;

    // Strip bracketed-paste escape sequences that some terminals inject.
    let input = input.replace("\x1b[200~", "").replace("\x1b[201~", "");
    let input = input.trim();

    let result = if input.is_empty() {
        initial.to_string()
    } else {
        input.to_string()
    };

    eprint!(
        "\x1b[1A\x1b[2K{} {} {} {}\n",
        theme.success_prefix,
        theme.prompt_style.apply_to(prompt),
        theme.success_suffix,
        theme.values_style.apply_to(&result),
    );
    io::stderr()
        .flush()
        .wrap_err("Failed to write to terminal")?;

    Ok(result)
}
