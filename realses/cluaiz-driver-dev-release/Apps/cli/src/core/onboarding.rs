use crate::app_enums::Mode;
use crate::core::state::{AppState, AuthMode, OnboardingStep, OsState};
use ::cluaiz_shared::profile::AccountType;
use color_eyre::Result;
use crossterm::event::{KeyCode, KeyEvent};
use colored::Colorize;

pub struct OnboardingEngine;

impl OnboardingEngine {
    /// Auto-advance logic for animation-based steps
    pub fn tick(state: &mut AppState) {
        if let OsState::Onboarding(step) = state.os_state {
            match step {
                OnboardingStep::LogoAnimation => {
                    // Auto-advance after ~2 seconds (120 frames at 60fps)
                    if state.frame_counter > 120 {
                        Self::advance(state);
                    }
                }
                OnboardingStep::WelcomeAbout => {
                    // Typing animation — advance char index each frame
                    state.typing_char_index = state.typing_char_index.saturating_add(1);
                }
                _ => {}
            }
        }
    }

    /// Advance to the next onboarding step using shared flow logic
    pub fn advance(state: &mut AppState) {
        if let OsState::Onboarding(current) = state.os_state {
            // Add current step to completed list
            if !state.completed_steps.contains(&current) {
                state.completed_steps.push(current);
            }

            match ::cluaiz_shared::onboarding::next_step(current) {
                Some(next) => {
                    state.os_state = OsState::Onboarding(next);
                    // Reset UI state for new step
                    state.typing_char_index = 0;
                    state.menu_state.select(Some(0));
                    state.auth_sub_step = 0;
                    state.profile_field_index = 0;

                    // Auto-scroll: if more than 4 completed steps, start scrolling
                    let completed_count = state.completed_steps.len();
                    if completed_count > 4 {
                        state.scroll_offset = (completed_count as u16).saturating_sub(4);
                    }
                }
                None => {
                    // 🛰️ Hardware READINESS GUARD
                    let os_str = if cfg!(windows) { "windows" } else { "linux" };
                    let binary_path = engines::runtime::execution::provisioner::BinaryProvisioner::resolve_local_kernel_path(os_str, &cluaiz_shared::hardware::schema::BackendDriver::CPU).unwrap_or_default();

                    if !binary_path.exists() {
                        println!("  {} [Onboarding] Core Core still forging. Holding transition until Hardware is ready...", "⏳".yellow());
                        state.onboarding_status = "SYNCING Core CORE...".to_string();
                        // We stay on the Complete step until the next tick/input
                        return;
                    } else {
                        state.onboarding_status = "OPTIMIZED ✓".to_string();
                    }

                    // Complete! Save profile + seed workspace + go to dashboard
                    state.user_profile.onboarding_completed = true;
                    state.user_profile.hardware_completed = true;
                    state.user_profile.touch();
                    let _ = ::cluaiz_shared::onboarding::seed_workspace(&state.user_profile);
                    state.username = state.user_profile.display_name().to_string();
                    state.os_state = OsState::Dashboard;
                }
            }
        }
    }

    pub fn handle_key(
        state: &mut AppState,
        step: OnboardingStep,
        key: KeyEvent,
        mode: &mut Mode,
    ) -> Result<()> {
        // Global: ESC to go back
        if key.code == KeyCode::Esc {
            if let Some(prev) = ::cluaiz_shared::onboarding::prev_step(step) {
                state.completed_steps.retain(|&s| s != step);
                state.os_state = OsState::Onboarding(prev);
            } else {
                *mode = Mode::Quit;
            }
            return Ok(());
        }

        // Global: 'S' to skip current step
        if key.code == KeyCode::Char('s') || key.code == KeyCode::Char('S') {
            match step {
                OnboardingStep::LogoAnimation | OnboardingStep::Complete => {}
                _ => {
                    Self::advance(state);
                    return Ok(());
                }
            }
        }

        match step {
            OnboardingStep::LogoAnimation => {
                if key.code == KeyCode::Enter {
                    Self::advance(state);
                }
            }
            OnboardingStep::WelcomeAbout => {
                if key.code == KeyCode::Enter {
                    Self::advance(state);
                }
            }
            OnboardingStep::Auth => match state.auth_sub_step {
                0 => match key.code {
                    KeyCode::Up | KeyCode::Down => {
                        state.auth_mode = match state.auth_mode {
                            AuthMode::Google => AuthMode::Email,
                            AuthMode::Email => AuthMode::Google,
                        };
                    }
                    KeyCode::Enter => match state.auth_mode {
                        AuthMode::Google => {
                            state.user_profile.auth = ::cluaiz_shared::auth::dummy_google_auth(
                                "Cluaiz@cluaiz.os",
                                "Cluaiz User",
                            );
                            Self::advance(state);
                        }
                        AuthMode::Email => {
                            state.auth_sub_step = 1;
                        }
                    },
                    _ => {}
                },
                1 => match key.code {
                    KeyCode::Char(c) => state.auth_email_input.push(c),
                    KeyCode::Backspace => {
                        state.auth_email_input.pop();
                    }
                    KeyCode::Enter => {
                        if !state.auth_email_input.is_empty() {
                            state.auth_sub_step = 2;
                        }
                    }
                    _ => {}
                },
                2 => match key.code {
                    KeyCode::Char(c) => state.auth_password_input.push(c),
                    KeyCode::Backspace => {
                        state.auth_password_input.pop();
                    }
                    KeyCode::Enter => {
                        if !state.auth_password_input.is_empty() {
                            state.user_profile.auth = ::cluaiz_shared::auth::dummy_email_auth(
                                &state.auth_email_input,
                                &state.auth_password_input,
                            );
                            Self::advance(state);
                        }
                    }
                    _ => {}
                },
                _ => {}
            },
            OnboardingStep::UsageChoice => match key.code {
                KeyCode::Up | KeyCode::Down => {
                    let current = state.menu_state.selected().unwrap_or(0);
                    let next = if current == 0 { 1 } else { 0 };
                    state.menu_state.select(Some(next));
                }
                KeyCode::Enter => {
                    let selected = state.menu_state.selected().unwrap_or(0);
                    state.user_profile.account_type = match selected {
                        0 => AccountType::Personal,
                        _ => AccountType::Business,
                    };
                    Self::advance(state);
                }
                _ => {}
            },
            OnboardingStep::ProfileInfo => match state.user_profile.account_type {
                AccountType::Personal => match key.code {
                    KeyCode::Char(c) => state.personal_name_input.push(c),
                    KeyCode::Backspace => {
                        state.personal_name_input.pop();
                    }
                    KeyCode::Enter => {
                        if !state.personal_name_input.is_empty() {
                            state.user_profile.identity.name = state.personal_name_input.clone();
                            state.username = state.personal_name_input.clone();
                            Self::advance(state);
                        }
                    }
                    _ => {}
                },
                AccountType::Business => {
                    let field_idx = state.profile_field_index;
                    match key.code {
                        KeyCode::Char(c) => {
                            if field_idx == 0 {
                                state.business_name_input.push(c);
                            }
                        }
                        KeyCode::Backspace => {
                            if field_idx == 0 {
                                state.business_name_input.pop();
                            }
                        }
                        KeyCode::Up => {
                            if field_idx == 0 {
                                let current = state.menu_state.selected().unwrap_or(0);
                                if current > 0 {
                                    state.menu_state.select(Some(current - 1));
                                }
                            }
                        }
                        KeyCode::Down => {
                            if field_idx == 0 {
                                let current = state.menu_state.selected().unwrap_or(0);
                                state.menu_state.select(Some(current + 1));
                            }
                        }
                        KeyCode::Tab => {
                            state.profile_field_index = (field_idx + 1).min(6);
                            state.menu_state.select(Some(0));
                        }
                        KeyCode::Enter => {
                            let biz = state
                                .user_profile
                                .business
                                .get_or_insert_with(Default::default);
                            match field_idx {
                                0 => {
                                    if !state.business_name_input.is_empty() {
                                        biz.name = state.business_name_input.clone();
                                        state.user_profile.identity.name =
                                            state.business_name_input.clone();
                                        state.profile_field_index = 1;
                                        state.menu_state.select(Some(0));
                                    }
                                }
                                1 => {
                                    let sel = state.menu_state.selected().unwrap_or(0);
                                    if sel < ::cluaiz_shared::profile::INDUSTRY_TAXONOMY.len() {
                                        biz.industry = ::cluaiz_shared::profile::INDUSTRY_TAXONOMY
                                            [sel]
                                            .id
                                            .to_string();
                                        state.profile_field_index = 2;
                                        state.menu_state.select(Some(0));
                                    }
                                }
                                2 => {
                                    let subs =
                                        ::cluaiz_shared::profile::get_sub_categories(&biz.industry);
                                    let sel = state.menu_state.selected().unwrap_or(0);
                                    if sel < subs.len() {
                                        biz.sub_category = subs[sel].id.to_string();
                                        state.profile_field_index = 3;
                                        state.menu_state.select(Some(0));
                                    }
                                }
                                _ => {
                                    Self::advance(state);
                                }
                            }
                        }
                        _ => {}
                    }
                }
            },
            OnboardingStep::HardwareAudit => {
                if key.code == KeyCode::Enter {
                    state.user_profile.hardware_completed = true;

                    // 🚀 Core IGNITION: Trigger Background Provisioning
                    let os = if cfg!(windows) { "windows" } else { "linux" };
                    let driver = if let Some(d) = state.hardware.active_drivers.get(0) {
                        match d.driver_id.as_str() {
                            "CUDA" => cluaiz_shared::hardware::schema::BackendDriver::CUDA,
                            "METAL" => cluaiz_shared::hardware::schema::BackendDriver::METAL,
                            _ => cluaiz_shared::hardware::schema::BackendDriver::CPU,
                        }
                    } else {
                        cluaiz_shared::hardware::schema::BackendDriver::CPU
                    };
                    let binary_path = engines::runtime::execution::provisioner::BinaryProvisioner::resolve_local_kernel_path(os, &driver).unwrap_or_default();

                    println!(
                        "  {} [Onboarding] Igniting Core Core Provisioner for [{:?}]...",
                        "🔥".red(),
                        driver
                    );

                    // Fire-and-forget background task
                    tokio::spawn(async move {
                        let _ = engines::runtime::execution::provisioner::BinaryProvisioner::ensure_binary(os, &driver, &binary_path).await;
                    });

                    Self::advance(state);
                }
            }

            _ => {
                if key.code == KeyCode::Enter {
                    Self::advance(state);
                }
            }
        }
        Ok(())
    }
}

pub fn textwrap(text: &str, width: usize) -> String {
    if width == 0 {
        return text.to_string();
    }
    let mut result = String::new();
    let mut col = 0;
    for word in text.split_whitespace() {
        let wlen = word.chars().count();
        if col > 0 && col + 1 + wlen > width {
            result.push('\n');
            col = 0;
        }
        if col > 0 {
            result.push(' ');
            col += 1;
        }
        result.push_str(word);
        col += wlen;
    }
    result
}
