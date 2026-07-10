//! ═══════════════════════════════════════════════════════════════════════
//!  Onboarding Flow — Step Machine (Non-UI, Reusable)
//! ═══════════════════════════════════════════════════════════════════════
//!  Defines the onboarding steps, transitions, and validation logic.
//!  CLI, Desktop App, Website — all share this exact flow.
//!  The UI layer (ratatui, web, etc.) just renders based on current step.
//! ═══════════════════════════════════════════════════════════════════════

use crate::profile::{AccountType, AuthMethod, UserProfile};
use serde::{Deserialize, Serialize};

// ── Onboarding Steps ──────────────────────────────────────────────────

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Debug, Default)]
pub enum OnboardingStep {
    #[default]
    LogoAnimation, // Step 1: Animated CLUAIZ logo
    WelcomeAbout,   // Step 2: "Welcome to Cluaiz CURE" + about text
    Auth,           // Step 3: Google or Email+Password login
    UsageChoice,    // Step 4: Personal vs Business
    ProfileInfo,    // Step 5: Name / Business details
    HardwareAudit,  // Step 6: Hardware detection + system_control.json
    ModelSelection, // Step 7: Persona + Chat + Embedding model selection
    Complete,       // Step 8: Done → Dashboard
}

impl OnboardingStep {
    /// Total number of steps (excluding Complete)
    pub fn total_steps() -> usize {
        7
    }

    /// Current step number (1-indexed, for display)
    pub fn step_number(&self) -> usize {
        match self {
            OnboardingStep::LogoAnimation => 1,
            OnboardingStep::WelcomeAbout => 2,
            OnboardingStep::Auth => 3,
            OnboardingStep::UsageChoice => 4,
            OnboardingStep::ProfileInfo => 5,
            OnboardingStep::HardwareAudit => 6,
            OnboardingStep::ModelSelection => 7,
            OnboardingStep::Complete => 8,
        }
    }

    /// Display label for the step
    pub fn label(&self) -> &'static str {
        match self {
            OnboardingStep::LogoAnimation => "Neural Core Boot",
            OnboardingStep::WelcomeAbout => "Welcome",
            OnboardingStep::Auth => "Authentication",
            OnboardingStep::UsageChoice => "Usage Mode",
            OnboardingStep::ProfileInfo => "Identity Setup",
            OnboardingStep::HardwareAudit => "Hardware Scan",
            OnboardingStep::ModelSelection => "Model Selection",
            OnboardingStep::Complete => "Launch",
        }
    }
}

// ── Step Transitions ──────────────────────────────────────────────────

/// Get the next step (returns None if already Complete)
pub fn next_step(current: OnboardingStep) -> Option<OnboardingStep> {
    match current {
        OnboardingStep::LogoAnimation => Some(OnboardingStep::WelcomeAbout),
        OnboardingStep::WelcomeAbout => Some(OnboardingStep::Auth),
        OnboardingStep::Auth => Some(OnboardingStep::UsageChoice),
        OnboardingStep::UsageChoice => Some(OnboardingStep::ProfileInfo),
        OnboardingStep::ProfileInfo => Some(OnboardingStep::HardwareAudit),
        OnboardingStep::HardwareAudit => Some(OnboardingStep::ModelSelection),
        OnboardingStep::ModelSelection => Some(OnboardingStep::Complete),
        OnboardingStep::Complete => None,
    }
}

/// Get the previous step (returns None if at first)
pub fn prev_step(current: OnboardingStep) -> Option<OnboardingStep> {
    match current {
        OnboardingStep::LogoAnimation => None,
        OnboardingStep::WelcomeAbout => Some(OnboardingStep::LogoAnimation),
        OnboardingStep::Auth => Some(OnboardingStep::WelcomeAbout),
        OnboardingStep::UsageChoice => Some(OnboardingStep::Auth),
        OnboardingStep::ProfileInfo => Some(OnboardingStep::UsageChoice),
        OnboardingStep::HardwareAudit => Some(OnboardingStep::ProfileInfo),
        OnboardingStep::ModelSelection => Some(OnboardingStep::HardwareAudit),
        OnboardingStep::Complete => Some(OnboardingStep::ModelSelection),
    }
}

/// Check if the current step has enough data to advance
pub fn can_advance(step: OnboardingStep, profile: &UserProfile) -> bool {
    match step {
        OnboardingStep::LogoAnimation => true, // auto-advance after animation
        OnboardingStep::WelcomeAbout => true,  // auto-advance or user press
        OnboardingStep::Auth => {
            profile.auth.method != AuthMethod::None && !profile.auth.email.is_empty()
        }
        OnboardingStep::UsageChoice => true, // any selection advances
        OnboardingStep::ProfileInfo => match profile.account_type {
            AccountType::Personal => !profile.identity.name.is_empty(),
            AccountType::Business => profile
                .business
                .as_ref()
                .is_some_and(|b| !b.name.is_empty() && !b.industry.is_empty()),
        },
        OnboardingStep::HardwareAudit => profile.hardware_completed,
        OnboardingStep::ModelSelection => true, // can skip models
        OnboardingStep::Complete => true,
    }
}

/// Get a 1-line summary of a completed step (for collapsed display)
pub fn get_completed_summary(step: OnboardingStep, profile: &UserProfile) -> String {
    match step {
        OnboardingStep::LogoAnimation => "CLUAIZ Neural Core Booted".to_string(),
        OnboardingStep::WelcomeAbout => "Welcome to Cluaiz CURE CLI".to_string(),
        OnboardingStep::Auth => {
            format!(
                "Auth: {} ({})",
                profile.auth.email,
                match profile.auth.method {
                    AuthMethod::Google => "Google",
                    AuthMethod::Email => "Email",
                    AuthMethod::None => "None",
                }
            )
        }
        OnboardingStep::UsageChoice => {
            format!("Mode: {}", profile.account_type)
        }
        OnboardingStep::ProfileInfo => match profile.account_type {
            AccountType::Personal => format!("Identity: {}", profile.display_name()),
            AccountType::Business => {
                let biz = profile.business.as_ref();
                format!("Business: {}", biz.map(|b| b.name.as_str()).unwrap_or("—"))
            }
        },
        OnboardingStep::HardwareAudit => "Hardware: Scanned ✓".to_string(),
        OnboardingStep::ModelSelection => {
            let count = [
                profile.models.persona_model.is_some(),
                profile.models.chat_model.is_some(),
                profile.models.embedding_model.is_some(),
            ]
            .iter()
            .filter(|&&x| x)
            .count();
            format!("Models: {}/3 selected", count)
        }
        OnboardingStep::Complete => "Setup Complete!".to_string(),
    }
}

/// Check if onboarding should be skipped (already completed)
pub fn should_skip_onboarding() -> bool {
    crate::profile::profile_exists()
        && crate::profile::load_profile()
            .ok()
            .flatten()
            .is_some_and(|p| p.onboarding_completed)
}
