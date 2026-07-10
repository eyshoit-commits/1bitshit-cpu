use crate::assets::logos::logo_gallery;
use colored::Colorize;
use rand::seq::SliceRandom;
use ratatui::prelude::*;
use unicode_width::UnicodeWidthStr;

pub fn print_native_logo(index: usize) {
    let variants = logo_gallery::LOGO_VARIANTS;
    if index >= variants.len() {
        return;
    }
    let logo_str = variants[index];
    for line in logo_str.lines() {
        // Disambiguate between Colorize and Stylize
        println!("  {}", Colorize::cyan(line).bold());
    }
}

pub fn calculate_max_width(logo: &str) -> u16 {
    logo.lines().map(|l| l.width()).max().unwrap_or(0) as u16
}

pub fn get_intrinsic_height(index: usize) -> u16 {
    let variants = logo_gallery::LOGO_VARIANTS;
    if index >= variants.len() {
        return 0;
    }
    variants[index].lines().count() as u16
}

pub fn render_best_fit_logo(buf: &mut Buffer, area: Rect, color: ratatui::prelude::Color) -> usize {
    let mut rng = rand::thread_rng();
    let variants = logo_gallery::LOGO_VARIANTS;
    let terminal_width = area.width;

    // Find all variants that fit within terminal_width - 2
    let mut fitting_indices: Vec<usize> = Vec::new();
    let mut max_width_found = 0;

    for (i, &logo_str) in variants.iter().enumerate() {
        let logo_width = calculate_max_width(logo_str);
        if logo_width <= terminal_width.saturating_sub(2) {
            fitting_indices.push(i);
            if logo_width > max_width_found {
                max_width_found = logo_width;
            }
        }
    }

    // Heuristic:
    // To maintain quality, only pick from the "Better" fitting ones (e.g. at least 70% of max width found)
    // but ALWAYS include the very tiny ones at the beginning of the gallery if width is very small.
    let candidates: Vec<usize> = if terminal_width < 50 {
        fitting_indices // On small screens, anything that fits is fine for variety
    } else {
        fitting_indices
            .into_iter()
            .filter(|&i| calculate_max_width(variants[i]) >= (max_width_found as f32 * 0.7) as u16)
            .collect()
    };

    let index = *candidates.choose(&mut rng).unwrap_or(&0);

    render_from_gallery(index, buf, area, color);
    index
}

pub fn render_from_gallery(
    index: usize,
    buf: &mut Buffer,
    area: Rect,
    color: ratatui::prelude::Color,
) {
    let variants = logo_gallery::LOGO_VARIANTS;
    if index >= variants.len() {
        return;
    }
    let logo_str = variants[index];
    let logo_lines: Vec<&str> = logo_str.lines().collect();

    let start_y = area.y; // Top-aligned to kill the gap
    let start_x = area.x; // Left aligned

    for (i, line) in logo_lines.iter().enumerate() {
        let y = start_y + i as u16;
        if y >= area.bottom() {
            break;
        }

        for (x_off, c) in (*line).chars().enumerate() {
            let byte_idx = (*line)
                .char_indices()
                .nth(x_off)
                .map(|(i, _)| i)
                .unwrap_or(0);
            let x = start_x + (line[..byte_idx].width()) as u16;

            if x >= area.right() {
                break;
            }

            let style = match c {
                ' ' => Style::default(),
                '█' | '▓' | '▒' | '░' | '▄' | '▀' | '▌' | '▐' | '╔' | '╗' | '╚' | '╝' | '═'
                | '║' => Style::default().fg(color).add_modifier(Modifier::BOLD),
                _ => Style::default().fg(Color::Rgb(180, 180, 180)), // Light Gray for shadows
            };

            buf[(x, y)].set_char(c).set_style(style);
        }
    }
}

pub fn render_isometric(buf: &mut Buffer, area: Rect, color: ratatui::prelude::Color) {
    let variants = logo_gallery::LOGO_VARIANTS;
    render_from_gallery(variants.len() - 1, buf, area, color);
}
