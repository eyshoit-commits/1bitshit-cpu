use crate::core::state::AppState;
use crate::theme::Theme;
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Rect},
    style::{Color, Modifier, Style},
    widgets::{Cell, Row, Table},
};

pub fn render_list(app: &mut AppState, theme: &Theme, area: Rect, buf: &mut Buffer) {
    let header_cells = ["  #", "TYPE", "MODEL NAME", "HEALTH", "RAM REQ", "CONTEXT"]
        .iter()
        .map(|h| {
            Cell::from(*h).style(
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )
        });

    let header = Row::new(header_cells)
        .style(Style::default().bg(Color::Reset))
        .height(1)
        .bottom_margin(1);

    let rows = app.sorted_models.iter().enumerate().map(|(i, model)| {
        let is_sel = Some(i) == app.roster_state.selected();

        // ── Selection Marker ──
        let index_marker = if is_sel { "▶ " } else { "  " };
        let cached_marker = if !model.manifest.is_cloud_api && model.is_cached {
            "✔"
        } else {
            " "
        };

        // ── Family Segments & Branding ──
        let family_color = if model.manifest.name.to_lowercase().contains("gemma") {
            Color::Cyan
        } else if model.manifest.name.to_lowercase().contains("llama") {
            Color::Green
        } else if model.manifest.name.to_lowercase().contains("bonsai") {
            Color::Yellow
        } else {
            Color::White
        };

        let row_style = if is_sel {
            theme.tabs_selected
        } else if model.status == "Incompatible" || model.status == "Impossible" {
            Style::default().fg(Color::DarkGray)
        } else {
            Style::default().fg(Color::White)
        };

        let health_color = match model.status.as_str() {
            "Optimal" => Color::Green,
            "Average" => Color::Yellow,
            "Heavy" => Color::Red,
            "Cloud" => Color::LightBlue,
            _ => Color::DarkGray,
        };

        let ram_display = if model.manifest.is_cloud_api {
            if model.manifest.is_free_tier {
                "FREE".to_string()
            } else {
                "PAID".to_string()
            }
        } else {
            format!("{:.2} GB", model.manifest.ram_required_gb)
        };

        Row::new(vec![
            Cell::from(format!("{}{:02}{}", index_marker, i + 1, cached_marker)),
            Cell::from(if model.manifest.is_cloud_api {
                "CLOUD"
            } else {
                "LOCAL"
            })
            .style(Style::default().fg(family_color)),
            Cell::from(model.manifest.name.clone()).style(row_style.add_modifier(Modifier::BOLD)),
            Cell::from(model.status.to_uppercase()).style(Style::default().fg(health_color)),
            Cell::from(ram_display),
            Cell::from(model.manifest.context_window.clone()),
        ])
        .style(row_style)
    });

    // ── Systematic Table Logic ──
    let table = Table::new(
        rows,
        [
            Constraint::Length(6),  // #
            Constraint::Length(8),  // TYPE
            Constraint::Length(40), // MODEL NAME (Fixed large space)
            Constraint::Length(12), // HEALTH
            Constraint::Length(11), // RAM REQ
            Constraint::Length(8),  // CONTEXT
        ],
    )
    .header(header)
    .row_highlight_style(theme.tabs_selected)
    .column_spacing(2);

    ratatui::widgets::StatefulWidget::render(table, area, buf, &mut app.roster_state);
}
