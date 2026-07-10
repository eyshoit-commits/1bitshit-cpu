use crate::core::state::{AppState, SettingsTab};
use crate::theme::Theme;
use ratatui::prelude::*;

pub mod hardware;
mod ui_menu;

pub fn render_widget(app: &mut AppState, theme: &Theme, area: Rect, buf: &mut Buffer) {
    match app.settings_tab {
        SettingsTab::Main => ui_menu::render_menu(app, theme, area, buf),
    }
}
