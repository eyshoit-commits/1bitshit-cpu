use crate::core::state::{ActivityBlock, AppState};
use crate::theme::Theme;
use colored::Colorize;
use ratatui::prelude::*;
use ratatui::widgets::{Paragraph, Wrap};

/// 🌊 NATIVE STREAM: Commits new actions to the terminal's native history buffer.
/// This allows the user to use their terminal's native scrollbar and selection.
pub fn commit_to_stdout(state: &mut AppState) {
    if state.activity_stream.len() <= state.rendered_actions_count {
        return;
    }

    let mut prev_group: Option<BlockGroup> = if state.rendered_actions_count > 0 {
        state
            .activity_stream
            .get(state.rendered_actions_count - 1)
            .map(|b| group_of(b))
    } else {
        None
    };

    for i in state.rendered_actions_count..state.activity_stream.len() {
        let block = &state.activity_stream[i];
        if block.is_live() {
            continue;
        }

        let g = group_of(block);
        if let Some(pg) = prev_group {
            if pg != g {
                println!(); // Gap between different groups
            }
        }

        match block {
            ActivityBlock::SystemLog(msg) => {
                println!(
                    "{}",
                    format!(" ⚙  {}", msg).truecolor(128, 128, 128).italic()
                );
            }
            ActivityBlock::MenuOpened(name) => {
                println!("{}", format!(" 📋  Opened: {}", name).cyan());
            }
            ActivityBlock::ModelSelected { name, ctx, status } => {
                let s = status.to_uppercase();
                let label = format!(" ▶   {}   |   {}   |   {} ctx", name, s, ctx);
                match status.as_str() {
                    "Optimal" => println!("{}", label.green().bold()),
                    "Average" => println!("{}", label.yellow().bold()),
                    "Heavy" => println!("{}", label.red().bold()),
                    _ => println!("{}", label.white().bold()),
                }
            }
            ActivityBlock::DownloadStarted(name) => {
                println!(
                    "{}",
                    format!(" 📥  Downloading: {}...", name).yellow().bold()
                );
            }
            ActivityBlock::DownloadComplete(name) => {
                println!("{}", format!(" ✅  Complete: {}", name).green().bold());
            }
            ActivityBlock::DownloadFailed(name, reason) => {
                println!(
                    "{}",
                    format!(" ❌  Failed: {}  ({})", name, reason).red().bold()
                );
            }
            ActivityBlock::ModelMounted(name) => {
                println!(
                    "{}",
                    format!(" 🧠  Engine Mounted: {}", name).magenta().bold()
                );
            }
            ActivityBlock::Chat(sender, msg) => {
                let icon = if sender == "USER" {
                    format!("{} ", Colorize::bold(Colorize::cyan("👤")))
                } else {
                    format!("{} ", Colorize::bold(Colorize::white("🤖")))
                };
                let content = if sender == "USER" {
                    msg.clone().cyan()
                } else {
                    msg.clone().white()
                };
                println!("{}{}", icon, content);
            }
            _ => {}
        }
        prev_group = Some(g);
    }

    state.rendered_actions_count = state.activity_stream.len();
}

// ── Group type for separator logic ──────────────────────────────────────────
#[derive(PartialEq, Clone, Copy)]
enum BlockGroup {
    System,
    Chat,
    MenuNav, // MenuOpened
    Model,   // ModelSelected, DownloadStarted, DownloadComplete, DownloadFailed, ModelMounted
}

fn group_of(b: &ActivityBlock) -> BlockGroup {
    match b {
        ActivityBlock::SystemLog(_) => BlockGroup::System,
        ActivityBlock::Chat(_, _) => BlockGroup::Chat,
        ActivityBlock::MenuOpened(_) => BlockGroup::MenuNav,
        ActivityBlock::ModelSelected { .. } => BlockGroup::Model,
        ActivityBlock::DownloadStarted(_) => BlockGroup::Model,
        ActivityBlock::DownloadComplete(_) => BlockGroup::Model,
        ActivityBlock::DownloadFailed(_, _) => BlockGroup::Model,
        ActivityBlock::ModelMounted(_) => BlockGroup::Model,
        ActivityBlock::ActiveRoster | ActivityBlock::ActiveDetails => BlockGroup::System, // never rendered here
    }
}

/// Renders the frozen history stream (all non-live blocks from top → bottom).
/// Live blocks (ActiveRoster, ActiveDetails) are skipped here — rendered separately in app.rs.
pub fn render_widget(state: &mut AppState, _theme: &Theme, area: Rect, buf: &mut Buffer) {
    if area.height == 0 {
        return;
    }

    // Collect only frozen (non-live) blocks
    let blocks: Vec<ActivityBlock> = state
        .activity_stream
        .iter()
        .filter(|b| !b.is_live())
        .cloned()
        .collect();

    if blocks.is_empty() {
        return;
    }

    // Build render items: (block, needs_gap_before)
    // A 1-line gap is inserted whenever the group changes from previous block.
    struct Item {
        block: ActivityBlock,
        gap: bool,
    }
    let mut items: Vec<Item> = Vec::with_capacity(blocks.len());
    let mut prev_group: Option<BlockGroup> = None;

    for block in &blocks {
        let g = group_of(block);
        let gap = prev_group.map(|pg| pg != g).unwrap_or(false);
        items.push(Item {
            block: block.clone(),
            gap,
        });
        prev_group = Some(g);
    }

    // Total height including gaps
    let total_h: u16 = items
        .iter()
        .map(|i| i.block.get_frozen_height(area.width) + if i.gap { 1 } else { 0 })
        .sum();

    // Collect visible items from bottom (newest) upward, clipped to area.height
    // Skip based on scroll_offset
    let mut visible: Vec<Item> = Vec::new();
    let mut used: u16 = 0;
    let skip_count = state.stream_scroll_offset as usize;

    for (idx, item) in items.iter().rev().enumerate() {
        if idx < skip_count {
            continue;
        }

        let ih = item.block.get_frozen_height(area.width) + if item.gap { 1 } else { 0 };
        if used + ih > area.height {
            break;
        }
        used += ih;
        visible.push(Item {
            block: item.block.clone(),
            gap: item.gap,
        });
    }
    visible.reverse(); // chronological: oldest top → newest bottom

    let _ = total_h; // suppress unused warning

    // Render bottom-anchored
    let mut y = area.bottom().saturating_sub(used);

    for item in visible {
        if y >= area.bottom() {
            break;
        }

        // ── Blank separator line ──────────────────────────────────────────
        if item.gap {
            y += 1; // skip 1 line silently
        }

        if y >= area.bottom() {
            break;
        }

        let bh = item.block.get_frozen_height(area.width);
        let row = Rect::new(area.x + 1, y, area.width.saturating_sub(1), bh);

        match &item.block {
            ActivityBlock::SystemLog(msg) => {
                let text = format!("⚙  {}", msg);
                buf.set_string(
                    row.x,
                    row.y,
                    &text,
                    Style::default()
                        .fg(Color::DarkGray)
                        .add_modifier(Modifier::ITALIC),
                );
            }
            ActivityBlock::MenuOpened(name) => {
                let text = format!("📋  Opened: {}", name);
                buf.set_string(row.x, row.y, &text, Style::default().fg(Color::Cyan));
            }
            ActivityBlock::ModelSelected { name, ctx, status } => {
                let status_color = match status.as_str() {
                    "Optimal" => Color::Green,
                    "Average" => Color::Yellow,
                    "Heavy" => Color::Red,
                    _ => Color::DarkGray,
                };
                let label = format!(
                    "▶   {}   |   {}   |   {} ctx",
                    name,
                    status.to_uppercase(),
                    ctx
                );
                buf.set_string(
                    row.x,
                    row.y,
                    &label,
                    Style::default()
                        .fg(status_color)
                        .add_modifier(Modifier::BOLD),
                );
            }
            ActivityBlock::DownloadStarted(name) => {
                let text = format!("📥  Downloading: {}...", name);
                buf.set_string(
                    row.x,
                    row.y,
                    &text,
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                );
            }
            ActivityBlock::DownloadComplete(name) => {
                let text = format!("✅  Complete: {}", name);
                buf.set_string(
                    row.x,
                    row.y,
                    &text,
                    Style::default()
                        .fg(Color::Green)
                        .add_modifier(Modifier::BOLD),
                );
            }
            ActivityBlock::DownloadFailed(name, reason) => {
                let text = format!("❌  Failed: {}  ({})", name, reason);
                buf.set_string(
                    row.x,
                    row.y,
                    &text,
                    Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
                );
            }
            ActivityBlock::ModelMounted(name) => {
                let text = format!("🧠  Engine Mounted: {}", name);
                buf.set_string(
                    row.x,
                    row.y,
                    &text,
                    Style::default()
                        .fg(Color::Magenta)
                        .add_modifier(Modifier::BOLD),
                );
            }
            ActivityBlock::Chat(sender, msg) => {
                let (icon, color) = if sender == "USER" {
                    ("👤 ", Color::Cyan)
                } else {
                    ("🤖 ", Color::White) // AI is strictly White
                };

                let spans = vec![
                    Span::styled(
                        format!("{} ", icon.trim()),
                        Style::default().fg(color).add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(msg, Style::default().fg(Color::White)),
                ];
                let p = Paragraph::new(Line::from(spans)).wrap(Wrap { trim: true });
                p.render(row, buf);
            }
            // Live blocks never appear in frozen stream
            ActivityBlock::ActiveRoster | ActivityBlock::ActiveDetails => {}
        }

        y += bh;
    }
}
