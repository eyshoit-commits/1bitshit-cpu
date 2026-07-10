//! ═══════════════════════════════════════════════════════════════════════
//!  Cluaiz Onboarding Ritual — Single Continuous Flow Renderer
//! ═══════════════════════════════════════════════════════════════════════
//!  ONE page. Steps appear sequentially with animations.
//!  Completed steps collapse to 1-line summaries.
//!  Active step shows full interactive UI.
//! ═══════════════════════════════════════════════════════════════════════

use crate::assets::logos::logo;
use crate::core::onboarding::textwrap;
use crate::core::state::{AppState, AuthMode, OnboardingStep, OsState};
use crate::theme::Theme;
use ::cluaiz_shared::profile::AccountType;
use ratatui::{
    prelude::*,
    widgets::{
        Block, Borders, Clear, Padding, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState,
    },
};

// ── About Text ────────────────────────────────────────────────────────

const ABOUT_TEXT: &str = "\
Archer Cluaiz — The Pinnacle of Core Orchestration. \
An open-source, local-first AI runtime built for absolute privacy and zero-latency intelligence. \
Archer runs advanced Core models directly on your hardware — \
no cloud, no tracking, no compromises. Whether you are a researcher pushing the boundaries of AI, \
a business owner automating operations, or a creator seeking an intelligent companion, \
Archer delivers Cluaiz computing power at your fingertips. \
Built with Rust for bare-metal performance, Archer supports multi-model orchestration, \
hardware-optimized inference with GPU acceleration, and a modular architecture \
that adapts to your exact needs. Your data never leaves your machine. \
Your models run under your control. Welcome to the future of Cluaiz AI.";

// ── Main Entry Point ──────────────────────────────────────────────────

pub fn render_flow(state: &mut AppState, theme: &Theme, area: Rect, buf: &mut Buffer) {
    // Clear screen — absolute transparency to avoid blue dots/lag
    Clear.render(area, buf);

    let current_step = match state.os_state {
        OsState::Onboarding(step) => step,
        _ => return,
    };

    // ── Layout Definition ─────────────────────────────────────────────
    // Bottom 2 rows are reserved for Keybar (pinned footer)
    let footer_height = 2u16;
    let footer_area = Rect::new(
        area.x,
        area.bottom().saturating_sub(footer_height),
        area.width,
        footer_height,
    );
    let view_area = Rect::new(
        area.x,
        area.y,
        area.width,
        area.height.saturating_sub(footer_height),
    );

    // ── Render Completed Steps ───────────────────────────────────────
    let scroll_idx = state.scroll_offset as usize;
    let visible_completed: Vec<_> = state.completed_steps.iter().skip(scroll_idx).collect();

    let mut current_y = view_area.y + 1; // Top padding

    for &&step in &visible_completed {
        if current_y >= view_area.bottom() {
            break;
        }
        let step_rect = Rect::new(view_area.x, current_y, view_area.width, 1);
        render_completed_step(state, step, step_rect, buf);
        current_y += 1;
    }

    // ── Render Active Step ───────────────────────────────────────────
    if current_y < view_area.bottom() {
        let separator_rows = if !visible_completed.is_empty() { 1 } else { 0 };
        current_y += separator_rows;

        let active_h = view_area.bottom().saturating_sub(current_y);
        if active_h > 0 {
            let active_area = Rect::new(view_area.x, current_y, view_area.width, active_h);
            render_active_step(state, theme, current_step, active_area, buf);
        }
    }

    // ── Render Footer (Hints + Step count) ──────────────────────────
    render_keybar(state, current_step, footer_area, buf);

    // ── Render Visual Scrollbar ──────────────────────────────────────
    let total_content_height = state.completed_steps.len() as u16 + 15; // Estimating 15 for active step
    if total_content_height > view_area.height {
        let mut scrollbar_state = ScrollbarState::new(total_content_height as usize)
            .position(state.scroll_offset as usize);

        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(Some("▲"))
            .end_symbol(Some("▼"))
            .track_symbol(Some("│"))
            .thumb_symbol("█");

        scrollbar.render(view_area, buf, &mut scrollbar_state);
    }
}

// ── Completed Step (1-line collapsed summary) ─────────────────────────

fn render_completed_step(state: &AppState, step: OnboardingStep, area: Rect, buf: &mut Buffer) {
    let summary = ::cluaiz_shared::onboarding::get_completed_summary(step, &state.user_profile);
    let step_num = step.step_number();

    let line = Line::from(vec![
        Span::styled(
            format!("  ✅ Step {} ", step_num),
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!("│ {} ", summary),
            Style::default().fg(Color::DarkGray),
        ),
    ]);

    Paragraph::new(line).render(area, buf);
}

// ── Active Step Renderer ──────────────────────────────────────────────

fn render_active_step(
    state: &mut AppState,
    _theme: &Theme,
    step: OnboardingStep,
    area: Rect,
    buf: &mut Buffer,
) {
    match step {
        OnboardingStep::LogoAnimation => render_logo_step(state, area, buf),
        OnboardingStep::WelcomeAbout => render_welcome_step(state, area, buf),
        OnboardingStep::Auth => render_auth_step(state, area, buf),
        OnboardingStep::UsageChoice => render_usage_step(state, area, buf),
        OnboardingStep::ProfileInfo => render_profile_step(state, area, buf),
        OnboardingStep::HardwareAudit => render_hardware_step(state, area, buf),
        OnboardingStep::ModelSelection => render_model_step(state, area, buf),
        OnboardingStep::Complete => render_complete_step(state, area, buf),
    }
}

// ── Step 1: Logo Animation ────────────────────────────────────────────

fn render_logo_step(state: &mut AppState, area: Rect, buf: &mut Buffer) {
    let chunks = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(8), // logo
        Constraint::Length(1),
        Constraint::Length(1), // tagline
        Constraint::Length(1), // version
        Constraint::Length(1),
        Constraint::Length(1), // blinking prompt
    ])
    .split(area);

    let left_margin = 2; // Left-aligned indent for terminal feel

    // Animated logo color (pulse cyan)
    let progress = (state.frame_counter as f32 / 60.0).min(1.0);
    let cyan_val = (progress * 255.0) as u8;

    // Logo render wrapper to add margin
    let logo_area = Rect::new(
        area.x + left_margin,
        chunks[1].y,
        area.width.saturating_sub(left_margin),
        chunks[1].height,
    );
    logo::render_isometric(buf, logo_area, Color::Rgb(0, cyan_val, cyan_val));

    // Tagline (fade in after 30 frames)
    if state.frame_counter > 30 {
        Paragraph::new("Cluaiz Core ENGINE")
            .style(
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD | Modifier::REVERSED),
            )
            .render(
                Rect::new(area.x + left_margin, chunks[3].y, area.width, 1),
                buf,
            );
    }

    // Version
    if state.frame_counter > 50 {
        Paragraph::new("v1.0.0-Cluaiz │ ARCHER CORE V10")
            .style(Style::default().fg(Color::DarkGray))
            .render(
                Rect::new(area.x + left_margin, chunks[4].y, area.width, 1),
                buf,
            );
    }

    // Blinking prompt
    if state.frame_counter > 80 {
        let cycle = (state.frame_counter as f32 / 15.0).sin() * 0.5 + 0.5;
        let alpha = (cycle * 255.0) as u8;
        Paragraph::new("[ PRESS ENTER TO IGNITE THE Core CORE ]")
            .style(
                Style::default()
                    .fg(Color::Rgb(alpha, alpha, 255))
                    .add_modifier(Modifier::BOLD),
            )
            .render(
                Rect::new(area.x + left_margin + 2, chunks[6].y, area.width, 1),
                buf,
            );
    }
}

// ── Step 2: Welcome + About ──────────────────────────────────────────

fn render_welcome_step(state: &mut AppState, area: Rect, buf: &mut Buffer) {
    let chunks = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(1), // title
        Constraint::Length(1),
        Constraint::Length(10), // about text (typing) - approx max height
        Constraint::Length(1),
        Constraint::Length(1), // prompt
    ])
    .split(area);

    let left_margin = 2;

    // Title
    Paragraph::new("⚡ Welcome to Archer Cluaiz CLI")
        .style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .render(
            Rect::new(area.x + left_margin, chunks[1].y, area.width, 1),
            buf,
        );

    // Typing animation — show characters progressively
    let visible_chars = state.typing_char_index.min(ABOUT_TEXT.len());
    let visible_text: String = ABOUT_TEXT.chars().take(visible_chars).collect();

    let text_width = area.width.saturating_sub(6) as usize;
    let wrapped = textwrap(&visible_text, text_width);

    Paragraph::new(wrapped)
        .style(Style::default().fg(Color::Rgb(180, 180, 200)))
        .render(
            Rect::new(
                area.x + left_margin + 2,
                chunks[3].y,
                area.width.saturating_sub(left_margin + 2),
                10,
            ),
            buf,
        );

    // Show prompt only after typing is ~done
    if visible_chars >= ABOUT_TEXT.len().saturating_sub(10) {
        Paragraph::new("[ PRESS ENTER TO CONTINUE ]")
            .style(
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )
            .render(
                Rect::new(area.x + left_margin, chunks[5].y, area.width, 1),
                buf,
            );
    }
}

// ── Step 3: Auth ──────────────────────────────────────────────────────

fn render_auth_step(state: &mut AppState, area: Rect, buf: &mut Buffer) {
    let left_margin = 2;
    let input_w = 50u16.min(area.width.saturating_sub(left_margin + 2));

    match state.auth_sub_step {
        0 => {
            // Sub-step 0: Choose Google or Email
            let chunks = Layout::vertical([
                Constraint::Length(1),
                Constraint::Length(1), // title
                Constraint::Length(1),
                Constraint::Length(3), // google
                Constraint::Length(1),
                Constraint::Length(3), // email
                Constraint::Length(1),
                Constraint::Length(1), // hint
            ])
            .split(area);

            Paragraph::new("🔐 How would you like to sign in?")
                .style(
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD),
                )
                .render(
                    Rect::new(area.x + left_margin, chunks[1].y, area.width, 1),
                    buf,
                );

            let google_sel = state.auth_mode == AuthMode::Google;
            let google_area = Rect::new(area.x + left_margin + 2, chunks[3].y, input_w, 3);
            Paragraph::new("  🌐  Sign in with Google  ")
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_style(if google_sel {
                            Style::default().fg(Color::Rgb(66, 133, 244))
                        } else {
                            Style::default().fg(Color::DarkGray)
                        }),
                )
                .style(if google_sel {
                    Style::default()
                        .fg(Color::Black)
                        .bg(Color::Rgb(66, 133, 244))
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::White)
                })
                .render(google_area, buf);

            let email_sel = state.auth_mode == AuthMode::Email;
            let email_area = Rect::new(area.x + left_margin + 2, chunks[5].y, input_w, 3);
            Paragraph::new("  ✉️  Sign in with Email  ")
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_style(if email_sel {
                            Style::default().fg(Color::Cyan)
                        } else {
                            Style::default().fg(Color::DarkGray)
                        }),
                )
                .style(if email_sel {
                    Style::default()
                        .fg(Color::Black)
                        .bg(Color::Cyan)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::White)
                })
                .render(email_area, buf);

            Paragraph::new("↑↓ Navigate │ ENTER Select")
                .style(Style::default().fg(Color::DarkGray))
                .render(
                    Rect::new(area.x + left_margin, chunks[7].y, area.width, 1),
                    buf,
                );
        }
        1 => {
            // Sub-step 1: Email input
            let chunks = Layout::vertical([
                Constraint::Length(1),
                Constraint::Length(1), // title
                Constraint::Length(1),
                Constraint::Length(3), // email input
                Constraint::Length(1),
                Constraint::Length(1), // hint
            ])
            .split(area);

            Paragraph::new("✉️  Enter your email address")
                .style(
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD),
                )
                .render(
                    Rect::new(area.x + left_margin, chunks[1].y, area.width, 1),
                    buf,
                );

            let input_area = Rect::new(area.x + left_margin + 2, chunks[3].y, input_w, 3);
            Paragraph::new(format!(" > {}_", state.auth_email_input))
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(Color::Cyan))
                        .title(" Email "),
                )
                .style(Style::default().fg(Color::Cyan))
                .render(input_area, buf);

            Paragraph::new("Type your email and press ENTER")
                .style(Style::default().fg(Color::DarkGray))
                .render(
                    Rect::new(area.x + left_margin, chunks[5].y, area.width, 1),
                    buf,
                );
        }
        2 => {
            // Sub-step 2: Password input
            let chunks = Layout::vertical([
                Constraint::Length(1),
                Constraint::Length(1), // title
                Constraint::Length(1),
                Constraint::Length(1), // show entered email
                Constraint::Length(1),
                Constraint::Length(3), // password input
                Constraint::Length(1),
                Constraint::Length(1), // hint
            ])
            .split(area);

            Paragraph::new("🔑 Create your password")
                .style(
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD),
                )
                .render(
                    Rect::new(area.x + left_margin, chunks[1].y, area.width, 1),
                    buf,
                );

            Paragraph::new(format!("  Email: {}", state.auth_email_input))
                .style(Style::default().fg(Color::DarkGray))
                .render(
                    Rect::new(area.x + left_margin + 2, chunks[3].y, area.width, 1),
                    buf,
                );

            let input_area = Rect::new(area.x + left_margin + 2, chunks[5].y, input_w, 3);
            let masked: String = "•".repeat(state.auth_password_input.len());
            Paragraph::new(format!(" > {}_", masked))
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(Color::Cyan))
                        .title(" Password "),
                )
                .style(Style::default().fg(Color::Cyan))
                .render(input_area, buf);

            Paragraph::new("Type your password and press ENTER")
                .style(Style::default().fg(Color::DarkGray))
                .render(
                    Rect::new(area.x + left_margin, chunks[7].y, area.width, 1),
                    buf,
                );
        }
        _ => {}
    }
}

// ── Step 4: Usage Choice ──────────────────────────────────────────────

fn render_usage_step(state: &mut AppState, area: Rect, buf: &mut Buffer) {
    let lm = 2u16; // left margin
    let opt_w = 50u16.min(area.width.saturating_sub(lm + 2));
    let selected = state.menu_state.selected().unwrap_or(0);

    // Title
    Paragraph::new(format!(
        "👋 Hey {}! How will you use Archer Cluaiz?",
        state.user_profile.auth.display_name
    ))
    .style(
        Style::default()
            .fg(Color::White)
            .add_modifier(Modifier::BOLD),
    )
    .render(Rect::new(area.x + lm, area.y + 1, area.width, 1), buf);

    // Options
    for (i, (icon, label)) in [("👤", "Personal AI Assistant"), ("🏢", "Business & Teams")]
        .iter()
        .enumerate()
    {
        let is_sel = selected == i;
        let y = area.y + 3 + (i as u16 * 4);
        if y + 3 > area.bottom() {
            break;
        }
        let opt_area = Rect::new(area.x + lm + 2, y, opt_w, 3);
        Paragraph::new(format!("  {}  {}", icon, label))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(if is_sel {
                        Style::default().fg(Color::Cyan)
                    } else {
                        Style::default().fg(Color::DarkGray)
                    }),
            )
            .style(if is_sel {
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            })
            .render(opt_area, buf);
    }
}

// ── Step 5: Profile Info ──────────────────────────────────────────────

fn render_profile_step(state: &mut AppState, area: Rect, buf: &mut Buffer) {
    match state.user_profile.account_type {
        AccountType::Personal => render_personal_profile(state, area, buf),
        AccountType::Business => render_business_profile(state, area, buf),
    }
}

fn render_personal_profile(state: &mut AppState, area: Rect, buf: &mut Buffer) {
    let lm = 2u16;
    let input_w = 46u16.min(area.width.saturating_sub(lm + 2));

    Paragraph::new("👤 What's your name, Cluaiz?")
        .style(
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        )
        .render(Rect::new(area.x + lm, area.y + 1, area.width, 1), buf);

    let input_area = Rect::new(area.x + lm + 2, area.y + 3, input_w, 3);
    Paragraph::new(format!(" > {}_", state.personal_name_input))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan))
                .title(" Your Name "),
        )
        .style(Style::default().fg(Color::Cyan))
        .render(input_area, buf);
}

fn render_business_profile(state: &mut AppState, area: Rect, buf: &mut Buffer) {
    let field_idx = state.profile_field_index;
    let selected = state.menu_state.selected().unwrap_or(0);
    let lm = 2u16;
    let opt_w = 56u16.min(area.width.saturating_sub(lm + 2));
    let list_area = Rect::new(
        area.x + lm + 2,
        area.y + 3,
        opt_w,
        area.height.saturating_sub(5),
    );

    let field_labels = [
        "Business Name",
        "Industry",
        "Sub-Category",
        "Business Model",
        "Target Audience",
        "Primary Goal",
    ];
    let current_label = field_labels.get(field_idx).unwrap_or(&"Complete");

    Paragraph::new(format!(
        "🏢 Business Setup — {} ({}/{})",
        current_label,
        field_idx + 1,
        field_labels.len()
    ))
    .style(
        Style::default()
            .fg(Color::White)
            .add_modifier(Modifier::BOLD),
    )
    .render(Rect::new(area.x + lm, area.y + 1, area.width, 1), buf);

    match field_idx {
        0 => {
            let input_area = Rect::new(area.x + lm + 2, area.y + 3, opt_w, 3);
            Paragraph::new(format!(" > {}_", state.business_name_input))
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(Color::Cyan))
                        .title(" Business Name "),
                )
                .style(Style::default().fg(Color::Cyan))
                .render(input_area, buf);
        }
        1 => {
            render_option_list(
                ::cluaiz_shared::profile::INDUSTRY_TAXONOMY
                    .iter()
                    .map(|i| format!("{} {}", i.icon, i.label))
                    .collect(),
                selected,
                area.x + lm + 2,
                opt_w,
                list_area,
                buf,
            );
        }
        2 => {
            let biz = state.user_profile.business.as_ref();
            let industry_id = biz.map(|b| b.industry.as_str()).unwrap_or("");
            let subs = ::cluaiz_shared::profile::get_sub_categories(industry_id);
            render_option_list(
                subs.iter()
                    .map(|s| format!("{} {}", s.icon, s.label))
                    .collect(),
                selected,
                area.x + lm + 2,
                opt_w,
                list_area,
                buf,
            );
        }
        3 => {
            render_option_list(
                ::cluaiz_shared::profile::BUSINESS_MODELS
                    .iter()
                    .map(|m| format!("{} {}", m.icon, m.label))
                    .collect(),
                selected,
                area.x + lm + 2,
                opt_w,
                list_area,
                buf,
            );
        }
        4 => {
            render_option_list(
                ::cluaiz_shared::profile::AUDIENCES
                    .iter()
                    .map(|a| format!("{} {}", a.icon, a.label))
                    .collect(),
                selected,
                area.x + lm + 2,
                opt_w,
                list_area,
                buf,
            );
        }
        5 => {
            render_option_list(
                ::cluaiz_shared::profile::PRIMARY_GOALS
                    .iter()
                    .map(|g| format!("{} {}", g.icon, g.label))
                    .collect(),
                selected,
                area.x + lm + 2,
                opt_w,
                list_area,
                buf,
            );
        }
        _ => {}
    }
}

fn render_option_list(
    options: Vec<String>,
    selected: usize,
    x: u16,
    width: u16,
    area: Rect,
    buf: &mut Buffer,
) {
    let max_visible = area.height.saturating_sub(1) as usize;

    // Calculate scroll window
    let start = if selected >= max_visible {
        selected - max_visible + 1
    } else {
        0
    };
    let end = (start + max_visible).min(options.len());

    for (vi, i) in (start..end).enumerate() {
        let is_sel = i == selected;
        let style = if is_sel {
            Style::default()
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        };
        let y = area.y + vi as u16;
        if y >= area.bottom() {
            break;
        }
        let item_area = Rect::new(x, y, width, 1);
        let prefix = if is_sel { " ▸ " } else { "   " };
        Paragraph::new(format!("{}{}", prefix, &options[i]))
            .style(style)
            .render(item_area, buf);
    }
}

// ── Step 6: Hardware Audit ────────────────────────────────────────────

fn render_hardware_step(state: &mut AppState, area: Rect, buf: &mut Buffer) {
    let lm = 2u16;

    Paragraph::new("📡 HARDWARE SIGNATURE DETECTED")
        .style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )
        .render(Rect::new(area.x + lm, area.y + 1, area.width, 1), buf);

    let cpu_brand = &state.hardware.cpu.brand;
    let cpu_cores = state.hardware.cpu.physical_cores;
    let has_gpu = !state.hardware.accelerators.gpus.is_empty();
    let gpu_display = if has_gpu {
        format!(
            "Accelerator Active ({:.1} GB VRAM)",
            state.hardware.accelerators.gpus[0].vram_total_gb
        )
    } else {
        "NO ACCELERATOR DETECTED".to_string()
    };

    let specs: Vec<Line> = vec![
        Line::from(vec![
            Span::styled("  HOST PLATFORM: ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                state
                    .hardware
                    .compute_architecture_type
                    .clone()
                    .unwrap_or_default(),
                Style::default().fg(Color::White),
            ),
        ]),
        Line::from(vec![
            Span::styled("  CPU UNIT:      ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!("{} ({} cores)", cpu_brand, cpu_cores),
                Style::default().fg(Color::White),
            ),
        ]),
        Line::from(vec![
            Span::styled("  GPU COMPUTE:   ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                gpu_display,
                Style::default().fg(if has_gpu { Color::Cyan } else { Color::Yellow }),
            ),
        ]),
        Line::from(vec![
            Span::styled("  SYSTEM RAM:    ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!("{:.1} GB", state.ram_gb),
                Style::default().fg(Color::White),
            ),
        ]),
        Line::from(vec![
            Span::raw("CORE STATUS:   "),
            Span::styled(
                &state.onboarding_status,
                Style::default()
                    .fg(if state.onboarding_status.contains("✓") {
                        Color::Green
                    } else {
                        Color::Yellow
                    })
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
    ];

    let spec_w = 65u16.min(area.width.saturating_sub(lm + 2));
    let spec_area = Rect::new(area.x + lm + 2, area.y + 3, spec_w, 7);

    Paragraph::new(specs)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan))
                .title(" HARDWARE DNA ")
                .padding(Padding::new(1, 1, 0, 0)),
        )
        .render(spec_area, buf);

    Paragraph::new("[ PRESS ENTER TO VERIFY & CONTINUE ]")
        .style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .render(Rect::new(area.x + lm, area.y + 11, area.width, 1), buf);
}

// ── Step 7: Model Selection ───────────────────────────────────────────

fn render_model_step(state: &mut AppState, area: Rect, buf: &mut Buffer) {
    let lm = 2u16;
    let opt_w = 52u16.min(area.width.saturating_sub(lm + 2));
    let selected = state.menu_state.selected().unwrap_or(0);

    Paragraph::new("🧠 SELECT YOUR Core MODELS")
        .style(
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        )
        .render(Rect::new(area.x + lm, area.y + 1, area.width, 1), buf);

    let models = [
        ("🧘", "Persona / Archer Model", "Soul & personality engine"),
        ("💬", "Chat Model", "Conversational intelligence"),
        ("🔍", "Embedding Model", "Semantic understanding"),
    ];

    for (i, (icon, name, desc)) in models.iter().enumerate() {
        let is_sel = selected == i;
        let y = area.y + 3 + (i as u16 * 4);
        if y + 3 > area.bottom() {
            break;
        }
        let model_area = Rect::new(area.x + lm + 2, y, opt_w, 3);
        Paragraph::new(format!("  {}  {} — {}", icon, name, desc))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(if is_sel {
                        Style::default().fg(Color::Cyan)
                    } else {
                        Style::default().fg(Color::DarkGray)
                    }),
            )
            .style(if is_sel {
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            })
            .render(model_area, buf);
    }
}

// ── Step 8: Complete ──────────────────────────────────────────────────

fn render_complete_step(state: &mut AppState, area: Rect, buf: &mut Buffer) {
    let lm = 2u16;

    Paragraph::new(" 🧿 Cluaiz Core ENGINE — ONLINE ")
        .style(
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD | Modifier::REVERSED),
        )
        .render(
            Rect::new(area.x + lm, area.y + 1, area.width.saturating_sub(lm), 1),
            buf,
        );

    let summary = format!(
        "  Identity : {}\n  Mode     : {}\n  Hardware : Verified ✓\n\n  Welcome to the future of Cluaiz AI, {}.",
        state.user_profile.display_name(),
        state.user_profile.account_type,
        state.user_profile.display_name(),
    );
    Paragraph::new(summary)
        .style(Style::default().fg(Color::White))
        .render(
            Rect::new(area.x + lm, area.y + 3, area.width.saturating_sub(lm), 5),
            buf,
        );

    let cycle = (state.frame_counter as f32 / 15.0).sin() * 0.5 + 0.5;
    let alpha = (cycle * 255.0) as u8;
    Paragraph::new("[ PRESS ENTER TO LAUNCH DASHBOARD ]")
        .style(
            Style::default()
                .fg(Color::Rgb(alpha, 255, alpha))
                .add_modifier(Modifier::BOLD),
        )
        .render(Rect::new(area.x + lm, area.y + 9, area.width, 1), buf);
}

// ── Bottom Keybar ─────────────────────────────────────────────────────

fn render_keybar(state: &AppState, step: OnboardingStep, area: Rect, buf: &mut Buffer) {
    let step_num = step.step_number();
    let total = OnboardingStep::total_steps();

    // ── Separator Line ──
    Paragraph::new("─".repeat(area.width as usize))
        .style(Style::default().fg(Color::DarkGray))
        .render(Rect::new(area.x, area.y, area.width, 1), buf);

    // ── Left: Step indicator ──
    let step_label = format!(" {}/{} {} ", step_num, total, step.label());

    // ── Middle: Navigation Hints ──────────────────────────────────────
    let hint = match step {
        OnboardingStep::UsageChoice
        | OnboardingStep::ModelSelection
        | OnboardingStep::HardwareAudit => "↑↓ Navigate │ ENTER Select",
        OnboardingStep::Auth => {
            if state.auth_sub_step == 0 {
                "↑↓ Choose │ ENTER Select"
            } else {
                "Type input │ ENTER Submit"
            }
        }
        OnboardingStep::ProfileInfo => {
            if state.profile_field_index == 0
                && state.user_profile.account_type == AccountType::Personal
            {
                "Type name │ ENTER Submit"
            } else {
                "↑↓ Navigate │ ENTER Select"
            }
        }
        OnboardingStep::Complete => "ENTER to Launch Dashboard",
        _ => "ENTER to Continue",
    };

    let spans = vec![
        Span::styled(
            step_label,
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" │ ", Style::default().fg(Color::DarkGray)),
        Span::styled(hint, Style::default().fg(Color::White)),
        Span::styled(" │ ", Style::default().fg(Color::DarkGray)),
        Span::styled(" ENTER ", Style::default().bg(Color::Cyan).fg(Color::Black)),
        Span::styled(" Next ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            " ESC ",
            Style::default().bg(Color::DarkGray).fg(Color::Black),
        ),
        Span::styled(" Back ", Style::default().fg(Color::DarkGray)),
    ];

    Line::from(spans).render(
        Rect::new(area.x + 2, area.y + 1, area.width.saturating_sub(4), 1),
        buf,
    );
}
