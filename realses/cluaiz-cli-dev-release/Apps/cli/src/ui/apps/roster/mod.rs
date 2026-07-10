use crate::core::state::AppState;
use crate::theme::Theme;
use ratatui::prelude::*;

mod details;
mod ui_flow;

pub fn render_widget(app: &mut AppState, theme: &Theme, area: Rect, buf: &mut Buffer) {
    if app.sorted_models.is_empty() {
        let loading_text = " [ INDEXING Core MODELS... PLEASE WAIT ] ";
        let x = area.x + (area.width.saturating_sub(loading_text.len() as u16) / 2);
        let y = area.y + (area.height / 2);
        if y < buf.area.bottom() {
            buf.set_string(
                x,
                y,
                loading_text,
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD | Modifier::SLOW_BLINK),
            );
        }
        return;
    }

    if app.roster_view == crate::core::state::RosterView::Details {
        details::render_widget(app, theme, area, buf);
        return;
    }

    ui_flow::render_list(app, theme, area, buf);
}
