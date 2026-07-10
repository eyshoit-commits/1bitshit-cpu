use crate::core::state::AppState;
use crate::theme::Theme;
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Paragraph, Widget},
};

pub fn render_menu(app: &AppState, theme: &Theme, area: Rect, buf: &mut Buffer) {
    let lm = 0u16; // Full width in primary flow
    let view_width = area.width;
    let mut current_y = area.y;

    // ── 1. HEADER ─────────────────────────────────────────────────────
    let header = Line::from(vec![Span::styled(
        " ⚙️  SYSTEM CONFIGURATION ",
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD | Modifier::REVERSED),
    )]);
    Paragraph::new(header).render(Rect::new(area.x + lm, current_y, view_width, 1), buf);
    current_y += 2;

    // ── 2. MENU OPTIONS (Flowing List) ────────────────────────────────
    let menu_options = [
        (
            "🧬",
            "Hardware DNA & Identity",
            "Kernel specs and Hardware signature",
        ),
        (
            "👤",
            "User Profile & Locale",
            "Identity mapping and regional sets",
        ),
        (
            "🎛️",
            "System Core & Reset",
            "Core engine parameters and recovery",
        ),
    ];

    let selected = app.settings_state.selected().unwrap_or(0);

    for (idx, (icon, title, desc)) in menu_options.iter().enumerate() {
        let is_sel = idx == selected;
        let style = if is_sel {
            theme.tabs_selected
        } else {
            Style::default().fg(Color::White)
        };

        let marker = if is_sel { "▶ " } else { "  " };

        let item = Line::from(vec![
            Span::styled(
                marker,
                if is_sel {
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::DarkGray)
                },
            ),
            Span::styled(format!(" {} ", icon), style),
            Span::styled(format!("{:<30}", title), style.add_modifier(Modifier::BOLD)),
            Span::styled(format!(" │ {}", desc), Style::default().fg(Color::DarkGray)),
        ]);

        Paragraph::new(item).render(Rect::new(area.x + lm, current_y, view_width, 1), buf);
        current_y += 1;
    }
}
