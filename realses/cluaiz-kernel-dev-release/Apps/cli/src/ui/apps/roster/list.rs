use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::Span,
    widgets::{Block, Borders, List, ListItem},
    Frame,
};
use crate::core::state::AppState;

pub fn render(f: &mut Frame, app: &AppState, area: Rect) {
    let items: Vec<ListItem> = app
        .sorted_models
        .iter()
        .enumerate()
        .map(|(i, m)| {
            let color = match m.status.as_str() {
                "Highly Recommended (Fast)" => Color::Green,
                "Compatible (Warning: High RAM usage)" => Color::Yellow,
                "Cloud (0MB RAM)" => Color::Magenta,
                _ => Color::DarkGray,
            };
            
            let status_icon = if m.is_cached { "✔" } else { &format!("{:02}", i + 1) };
            
            let display_str = format!(
                "[{}] {}  {} - {} | {} {}",
                if i == app.roster_cursor { "◉" } else { " " },
                status_icon,
                m.model.name,
                m.model.parameters,
                m.model.bit_depth,
                m.status
            );
            
            let mut style = Style::default().fg(color);
            if i == app.roster_cursor {
                style = style.add_modifier(Modifier::REVERSED);
            }
            ListItem::new(Span::styled(display_str, style))
        })
        .collect();

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title(" Core Roster [Enter to Install] "));
    f.render_widget(list, area);
}
