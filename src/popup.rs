//! Helpers shared by the popups drawn over the list.

use ratatui::layout::{Constraint, Flex, Layout, Rect};

/// A `width` by `height` rect in the middle of `area`.
pub fn centered(area: Rect, width: u16, height: u16) -> Rect {
    let [area] = Layout::horizontal([Constraint::Length(width)])
        .flex(Flex::Center)
        .areas(area);
    let [area] = Layout::vertical([Constraint::Length(height)])
        .flex(Flex::Center)
        .areas(area);
    area
}
