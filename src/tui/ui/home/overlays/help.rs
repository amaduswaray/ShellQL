use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::Paragraph,
};

use crate::tui::ui::{
    centered_rect,
    home::overlays::{binding_line, open_popup, render_dismiss_hint},
};

pub fn render_help(frame: &mut Frame, area: Rect) {
    let popup_area = centered_rect(46, 72, area);
    let (block, inner) = open_popup(frame, popup_area, "Help");
    frame.render_widget(block, popup_area);

    let [content_area, hint_area] =
        Layout::vertical([Constraint::Min(0), Constraint::Length(1)]).areas(inner);

    let nav_header = Style::default().fg(Color::Blue).bold();

    let key_style = Style::default().fg(Color::White).bold();

    let desc_style = Style::default().fg(Color::Gray);
    let sep_style = Style::default().fg(Color::DarkGray);

    let lines = vec![
        Line::from(""),
        Line::from(Span::styled("  Navigation", nav_header)),
        Line::from(Span::styled("  ──────────", sep_style)),
        binding_line("  j / ↓", "move down", key_style, desc_style),
        binding_line("  k / ↑", "move up", key_style, desc_style),
        Line::from(""),
        Line::from(Span::styled("  Actions", nav_header)),
        Line::from(Span::styled("  ───────", sep_style)),
        binding_line("  ↵", "connect", key_style, desc_style),
        binding_line("  a", "add connection", key_style, desc_style),
        binding_line("  d", "delete connection", key_style, desc_style),
        binding_line("  ?", "toggle this help", key_style, desc_style),
        binding_line("  :", "command line", key_style, desc_style),
        binding_line("  q", "quit", key_style, desc_style),
        binding_line("  Ctrl+C", "force quit", key_style, desc_style),
    ];

    frame.render_widget(Paragraph::new(lines), content_area);
    render_dismiss_hint(frame, hint_area, "Esc/q  <close> ");
}

pub fn render_dashboard_help(frame: &mut Frame, area: Rect) {
    let popup_area = centered_rect(60, 80, area);
    let (block, inner) = open_popup(frame, popup_area, "Dashboard Help");
    frame.render_widget(block, popup_area);

    let [content_area, hint_area] =
        Layout::vertical([Constraint::Min(0), Constraint::Length(1)]).areas(inner);

    let nav_header = Style::default().fg(Color::Blue).bold();
    let key_style = Style::default().fg(Color::White).bold();
    let desc_style = Style::default().fg(Color::Gray);
    let sep_style = Style::default().fg(Color::DarkGray);

    let lines = vec![
        Line::from(""),
        Line::from(Span::styled("  Navigation", nav_header)),
        Line::from(Span::styled("  ──────────", sep_style)),
        binding_line("  j / ↓", "move down / next row", key_style, desc_style),
        binding_line("  k / ↑", "move up / previous row", key_style, desc_style),
        binding_line("  h / ←", "move left / previous column", key_style, desc_style),
        binding_line("  l / →", "move right / next column", key_style, desc_style),
        binding_line("  gg", "go to top", key_style, desc_style),
        binding_line("  G", "go to bottom", key_style, desc_style),
        binding_line("  Ctrl+U", "half-page up", key_style, desc_style),
        binding_line("  Ctrl+D", "half-page down", key_style, desc_style),
        Line::from(""),
        Line::from(Span::styled("  Pane Navigation", nav_header)),
        Line::from(Span::styled("  ───────────────", sep_style)),
        binding_line("  Ctrl+H/J/K/L", "move to pane", key_style, desc_style),
        binding_line("  Ctrl+←↓↑→", "move to pane", key_style, desc_style),
        Line::from(""),
        Line::from(Span::styled("  Modes", nav_header)),
        Line::from(Span::styled("  ─────", sep_style)),
        binding_line("  v", "visual row mode", key_style, desc_style),
        binding_line("  V", "visual row mode", key_style, desc_style),
        binding_line("  Ctrl+V", "visual column mode", key_style, desc_style),
        binding_line("  i", "edit cell (Table) / insert (Query)", key_style, desc_style),
        binding_line("  Esc", "exit mode", key_style, desc_style),
        Line::from(""),
        Line::from(Span::styled("  History", nav_header)),
        Line::from(Span::styled("  ───────", sep_style)),
        binding_line("  -", "go back in pane history", key_style, desc_style),
        binding_line("  _", "go forward in pane history", key_style, desc_style),
        binding_line("  :back", "go back in pane history", key_style, desc_style),
        binding_line("  :forward", "go forward in pane history", key_style, desc_style),
        Line::from(""),
        Line::from(Span::styled("  Editing", nav_header)),
        Line::from(Span::styled("  ───────", sep_style)),
        binding_line("  dd", "stage row for delete", key_style, desc_style),
        binding_line("  d + visual", "stage selection for delete", key_style, desc_style),
        binding_line("  u", "undo last staged change", key_style, desc_style),
        binding_line("  :w", "commit staged changes", key_style, desc_style),
        Line::from(""),
        Line::from(Span::styled("  Search", nav_header)),
        Line::from(Span::styled("  ──────", sep_style)),
        binding_line("  /", "search forward", key_style, desc_style),
        binding_line("  ?", "search backward", key_style, desc_style),
        binding_line("  n", "next match", key_style, desc_style),
        binding_line("  N", "previous match", key_style, desc_style),
        binding_line("  :noh", "clear search highlights", key_style, desc_style),
        Line::from(""),
        Line::from(Span::styled("  Commands", nav_header)),
        Line::from(Span::styled("  ────────", sep_style)),
        binding_line("  : + Tab", "list all commands", key_style, desc_style),
        binding_line("  :! <sql>", "execute SQL directly", key_style, desc_style),
        binding_line("  :connect", "switch database", key_style, desc_style),
        binding_line("  :disconnect", "return to home", key_style, desc_style),
        binding_line("  :q", "close pane / quit", key_style, desc_style),
        binding_line("  :where <expr>", "filter rows", key_style, desc_style),
        binding_line("  :order <col> [desc]", "sort rows", key_style, desc_style),
        binding_line("  :select <cols>", "show only named columns", key_style, desc_style),
        binding_line("  :reset", "clear filter/sort/columns", key_style, desc_style),
        Line::from(""),
        Line::from(Span::styled("  Misc", nav_header)),
        Line::from(Span::styled("  ────", sep_style)),
        binding_line("  K", "peek cell value", key_style, desc_style),
        binding_line("  Ctrl+C", "force quit", key_style, desc_style),
    ];

    frame.render_widget(Paragraph::new(lines), content_area);
    render_dismiss_hint(frame, hint_area, "Esc/q  <close> ");
}
