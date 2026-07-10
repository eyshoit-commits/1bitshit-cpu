//! ═══════════════════════════════════════════════════════════════════════
//!  Local Auth — Dummy Authentication (No DB Yet)
//! ═══════════════════════════════════════════════════════════════════════
//!  Provides stub auth methods for onboarding.
//!  When real backend is ready, these will call the actual API.
//!  For now: saves auth info locally in UserProfile JSON.
//! ═══════════════════════════════════════════════════════════════════════

use crate::profile::{AuthInfo, AuthMethod};

/// Generate a dummy local token (just a timestamp hash for now)
fn generate_local_token() -> String {
    let now = chrono::Local::now().timestamp_millis();
    format!("local_token_{}", now)
}

/// Dummy Google auth — simulates a Google sign-in
/// In production, this would open a browser OAuth flow
pub fn dummy_google_auth(email: &str, name: &str) -> AuthInfo {
    AuthInfo {
        method: AuthMethod::Google,
        email: email.to_string(),
        display_name: name.to_string(),
        local_token: generate_local_token(),
    }
}

/// Dummy Email auth — simulates email + password registration/login
/// In production, this would call the backend API
pub fn dummy_email_auth(email: &str, _password: &str) -> AuthInfo {
    // Extract display name from email (before @)
    let display_name = email.split('@').next().unwrap_or("User").to_string();

    AuthInfo {
        method: AuthMethod::Email,
        email: email.to_string(),
        display_name,
        local_token: generate_local_token(),
    }
}

/// Check if a profile has completed auth
pub fn is_authenticated(auth: &AuthInfo) -> bool {
    auth.method != AuthMethod::None && !auth.local_token.is_empty()
}
