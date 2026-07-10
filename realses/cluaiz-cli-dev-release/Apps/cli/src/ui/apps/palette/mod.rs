use crate::core::state::{AppMode, AppState};
use ratatui::{prelude::*, widgets::*};

pub fn render_widget(state: &AppState, area: Rect, buf: &mut Buffer) {
    if state.interaction_mode == AppMode::Normal {
        return;
    }

    let items = if state.interaction_mode == AppMode::CommandPalette {
        vec!["Model List", "Settings", "Documentation", "Terminate"]
    } else {
        vec!["Context: @file", "Context: @directory"]
    };

    let list_items: Vec<ListItem> = items
        .iter()
        .enumerate()
        .map(|(i, item)| {
            let is_selected = state.palette_state.selected() == Some(i)
                || (state.interaction_mode == AppMode::ContextPalette
                    && state.context_palette_state.selected() == Some(i));

            let style = if is_selected {
                Style::default()
                    .fg(Color::Cyan)
                    .bg(Color::Rgb(40, 40, 40))
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::Gray)
            };

            ListItem::new(format!("  {}  ", item)).style(style)
        })
        .collect();

    let list = List::new(list_items).highlight_style(Style::default().bg(Color::Rgb(40, 40, 40)));

    // Clear area with dark background to differentiate from terminal background
    let bg_rect = Block::default().bg(Color::Rgb(25, 25, 25));
    bg_rect.render(area, buf);

    // Render the list directly without borders
    Widget::render(list, area, buf);
}
