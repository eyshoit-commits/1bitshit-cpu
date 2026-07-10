use crate::core::state::AppState;
use crate::theme::Theme;
use ratatui::prelude::*;

mod ui_stream;

pub fn render_widget(app: &mut AppState, _theme: &Theme, area: Rect, buf: &mut Buffer) {
    ui_stream::render_stream(app, area, buf);
}
