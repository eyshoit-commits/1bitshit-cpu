use crate::core::state::AppState;
use crate::theme::Theme;
use ratatui::prelude::*;

pub mod native;
pub mod ritual;
pub mod seeding;

// ── Sentinel Logic (Inlined to resolve E0583) ────────────────────────
pub mod sentinel {
    pub fn _create_sentinel() -> std::io::Result<()> {
        Ok(())
    }
    pub fn _delete_sentinel() -> std::io::Result<()> {
        Ok(())
    }
    pub fn _is_igniting() -> bool {
        false
    }
}

pub fn render_widget(state: &mut AppState, theme: &Theme, area: Rect, buf: &mut Buffer) {
    ritual::render_flow(state, theme, area, buf);
}
