use ratatui::layout::Rect;

pub fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    use ratatui::layout::{Constraint, Layout};

    let [_, vertical, _] = Layout::vertical([
        Constraint::Percentage((100 - percent_y) / 2),
        Constraint::Percentage(percent_y),
        Constraint::Percentage((100 - percent_y) / 2),
    ])
    .areas(area);

    let [_, centered, _] = Layout::horizontal([
        Constraint::Percentage((100 - percent_x) / 2),
        Constraint::Percentage(percent_x),
        Constraint::Percentage((100 - percent_x) / 2),
    ])
    .areas(vertical);

    centered
}

/// Same as `centered_rect`, but never smaller than `min_w` × `min_h`.
/// The result is still perfectly centred inside `area` and clamped so it
/// never exceeds the terminal bounds.
pub fn centered_rect_with_min(
    percent_x: u16,
    percent_y: u16,
    min_w: u16,
    min_h: u16,
    area: Rect,
) -> Rect {
    let w = (area.width * percent_x / 100).max(min_w).min(area.width);
    let h = (area.height * percent_y / 100).max(min_h).min(area.height);
    let x = area.x + (area.width.saturating_sub(w)) / 2;
    let y = area.y + (area.height.saturating_sub(h)) / 2;
    Rect {
        x,
        y,
        width: w,
        height: h,
    }
}
