use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    prelude::Widget,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Padding, Paragraph, Wrap},
    buffer::Buffer,
};
use crate::core::state::AppState;
use crate::theme::Theme;

pub fn render_widget(app: &mut AppState, _theme: &Theme, area: Rect, buf: &mut Buffer) {
    if app.sorted_models.is_empty() { return; }
    let selected_idx = app.roster_state.selected().unwrap_or(0);
    
    // 🛡️ INDEX GUARD
    {
        let rec = &app.sorted_models[selected_idx];
        if !rec.is_cached && !rec.manifest.is_cloud_api {
            app.details_btn_idx = 0;
        }
    }

    let recommendation = &app.sorted_models[selected_idx];
    let manifest = &recommendation.manifest;

    let outer_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan))
        .title(format!(" MODEL IDENTITY: {} ", manifest.name.to_uppercase()))
        .padding(Padding::new(2, 2, 1, 1));

    let inner_area = outer_block.inner(area);
    Widget::render(outer_block, area, buf);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(12),
            Constraint::Min(4),
            Constraint::Length(4),
        ])
        .split(inner_area);

    let vram_str = if manifest.is_cloud_api {
        "CLOUD".to_string()
    } else {
        format!("{:.2} GB", manifest.ram_required_gb)
    };
    let size_str = if manifest.is_cloud_api {
        "N/A".to_string()
    } else {
        format!("{:.2} GB", manifest.download_size_gb)
    };

    let specs_lines: Vec<Line> = vec![
        Line::from(vec![
            Span::styled("  ID:          ", Style::default().fg(Color::DarkGray)),
            Span::styled(manifest.id.clone(), Style::default().fg(Color::White)),
        ]),
        Line::from(vec![
            Span::styled("  ARCH:        ", Style::default().fg(Color::DarkGray)),
            Span::styled(manifest.architecture.clone(), Style::default().fg(Color::White)),
        ]),
        Line::from(vec![
            Span::styled("  PARAMS:      ", Style::default().fg(Color::DarkGray)),
            Span::styled(manifest.parameters.clone(), Style::default().fg(Color::Cyan)),
        ]),
        Line::from(vec![
            Span::styled("  TRAINING:    ", Style::default().fg(Color::DarkGray)),
            Span::styled(manifest.training_tokens.clone(), Style::default().fg(Color::White)),
        ]),
        Line::from(vec![
            Span::styled("  CONTEXT:     ", Style::default().fg(Color::DarkGray)),
            Span::styled(manifest.context_window.clone(), Style::default().fg(Color::White)),
        ]),
        Line::from(vec![
            Span::styled("  VRAM/RAM:    ", Style::default().fg(Color::DarkGray)),
            Span::styled(vram_str, Style::default().fg(Color::Yellow)),
        ]),
        Line::from(vec![
            Span::styled("  DISK SIZE:   ", Style::default().fg(Color::DarkGray)),
            Span::styled(size_str, Style::default().fg(Color::White)),
        ]),
        Line::from(vec![
            Span::styled("  STATUS:      ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                recommendation.status.to_uppercase(),
                Style::default().fg(match recommendation.status.as_str() {
                    "Optimal"  => Color::Green,
                    "Average"  => Color::Yellow,
                    "Heavy"    => Color::Red,
                    "Cloud"    => Color::LightBlue,
                    _          => Color::DarkGray,
                }).add_modifier(Modifier::BOLD),
            ),
        ]),
    ];

    Widget::render(Paragraph::new(specs_lines), chunks[0], buf);

    Widget::render(
        Paragraph::new(manifest.description.clone())
            .wrap(Wrap { trim: true })
            .block(
                Block::default()
                    .title(" Core DESCRIPTION ")
                    .borders(Borders::TOP)
                    .border_style(Style::default().fg(Color::DarkGray)),
            ),
        chunks[1],
        buf,
    );

    render_action_bar(app, recommendation, manifest, chunks[2], buf);
}

fn render_action_bar(
    app: &AppState,
    recommendation: &engines::ModelRecommendation,
    manifest: &engines::ModelManifest,
    area: Rect,
    buf: &mut Buffer,
) {
    for y in area.top()..area.bottom() {
        for x in area.left()..area.right() {
            let cell = &mut buf[(x, y)];
            cell.set_char(' ');
            cell.set_style(Style::default());
        }
    }

    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(2),
        ])
        .split(area);
    let render_area = layout[1];

    if app.show_download_confirm {
        let btn_row = Layout::horizontal([
            Constraint::Fill(1),
            Constraint::Length(18),
            Constraint::Length(2),
            Constraint::Length(12),
            Constraint::Length(2),
            Constraint::Length(12),
            Constraint::Fill(1),
        ]).split(render_area);

        Widget::render(
            Paragraph::new("INITIATE DOWNLOAD?").style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
            btn_row[1],
            buf,
        );

        let no_sel = app.delete_confirm_idx == 0;
        let yes_sel = app.delete_confirm_idx == 1;

        let no_style = if no_sel {
            Style::default().fg(Color::Black).bg(Color::White).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White).add_modifier(Modifier::DIM)
        };

        let yes_style = if yes_sel {
            Style::default().fg(Color::Black).bg(Color::Cyan).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Cyan).add_modifier(Modifier::DIM)
        };

        Widget::render(Paragraph::new("[   NO   ]").centered().style(no_style), btn_row[3], buf);
        Widget::render(Paragraph::new("[  YES   ]").centered().style(yes_style), btn_row[5], buf);
        return;
    }

    if let Some(dl_id) = &app.downloading_id {
        if dl_id == &manifest.id {
            let elapsed = app.download_start_time.map(|t| t.elapsed().as_secs()).unwrap_or(0);
            let time_str = format!("[{:02}:{:02}]", elapsed / 60, elapsed % 60);
            
            let current_mib = app.download_current_bytes as f64 / (1024.0 * 1024.0);
            let total_mib = app.download_total_bytes as f64 / (1024.0 * 1024.0);
            let speed_mib = app.download_bytes_per_sec / (1024.0 * 1024.0);
            
            let bar_width = area.width.saturating_sub(time_str.len() as u16 + 40).max(10);
            let filled = (app.download_progress * bar_width as f64) as u16;
            
            let bar_line = Line::from(vec![
                Span::styled(format!("{} ", time_str), Style::default().fg(Color::DarkGray)),
                Span::styled("[", Style::default().fg(Color::White)),
                Span::styled("█".repeat(filled as usize), Style::default().fg(Color::Cyan)),
                Span::styled("░".repeat(bar_width.saturating_sub(filled) as usize), Style::default().fg(Color::DarkGray)),
                Span::styled("] ", Style::default().fg(Color::White)),
                Span::styled(format!("{:.2} MiB/{:.2} MiB ", current_mib, total_mib), Style::default().fg(Color::White)),
                Span::styled(format!("{:.2} MiB/s", speed_mib), Style::default().fg(Color::Green)),
            ]);

            Widget::render(Paragraph::new(bar_line).centered(), render_area, buf);
            return;
        }
    }

    let primary_sel = app.details_btn_idx == 0;
    let delete_sel = app.details_btn_idx == 1;
    let has_delete = recommendation.is_cached && !manifest.is_cloud_api;

    let btn_row = if has_delete {
        Layout::horizontal([
            Constraint::Fill(1),
            Constraint::Length(22),
            Constraint::Length(4),
            Constraint::Length(22),
            Constraint::Fill(1),
        ]).split(render_area)
    } else {
        Layout::horizontal([
            Constraint::Fill(1),
            Constraint::Length(22),
            Constraint::Fill(1),
        ]).split(render_area)
    };

    let primary_label = if recommendation.is_cached { "▶  RUN ENGINE" } else { "📥  DOWNLOAD GGUF" };
    let primary_style = if primary_sel {
        Style::default().fg(Color::Black).bg(Color::Green).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Green).add_modifier(Modifier::DIM)
    };

    Widget::render(Paragraph::new(primary_label).centered().style(primary_style), btn_row[1], buf);

    if has_delete {
        let delete_label = "🗑  DELETE WEIGHTS";
        let delete_style = if delete_sel {
            Style::default().fg(Color::Black).bg(Color::Red).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Red).add_modifier(Modifier::DIM)
        };
        Widget::render(Paragraph::new(delete_label).centered().style(delete_style), btn_row[3], buf);
    }
}
