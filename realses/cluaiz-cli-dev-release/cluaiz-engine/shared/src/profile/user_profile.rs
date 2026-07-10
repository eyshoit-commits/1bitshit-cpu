//! ═══════════════════════════════════════════════════════════════════════
//!  UserProfile — The Sovereign Identity Schema
//! ═══════════════════════════════════════════════════════════════════════
//!  This is THE single source of truth for user data across:
//!  CLI, Desktop App, Website, API — all share this exact schema.
//!  Saved as ~/.archer/user_profile.json
//! ═══════════════════════════════════════════════════════════════════════

use serde::{Deserialize, Serialize};

// ── Account Type ──────────────────────────────────────────────────────

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(rename_all = "lowercase")]
pub enum AccountType {
    #[default]
    Personal,
    Business,
}

impl std::fmt::Display for AccountType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AccountType::Personal => write!(f, "Personal"),
            AccountType::Business => write!(f, "Business"),
        }
    }
}

// ── Auth Info ─────────────────────────────────────────────────────────

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(rename_all = "lowercase")]
pub enum AuthMethod {
    #[default]
    None,
    Google,
    Email,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct AuthInfo {
    pub method: AuthMethod,
    pub email: String,
    pub display_name: String,
    /// Dummy token for local-first auth (no DB yet)
    pub local_token: String,
}

// ── User Identity ─────────────────────────────────────────────────────

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct UserIdentity {
    pub name: String,
}

// ── Business Profile ──────────────────────────────────────────────────
// Mirrors the web frontend's BusinessProfileView schema.
// User can change these later in Settings.

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct BusinessProfile {
    pub name: String,
    pub industry: String,
    pub sub_category: String,
    pub business_model: String,
    pub target_audience: String,
    pub primary_goal: String,
    pub hero_offering: String,
    pub company_size: u32,
}

// ── Model Selection ───────────────────────────────────────────────────

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct ModelSelection {
    pub persona_model: Option<String>,
    pub chat_model: Option<String>,
    pub embedding_model: Option<String>,
}

// ── The Master Profile ────────────────────────────────────────────────

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct UserProfile {
    pub auth: AuthInfo,
    pub account_type: AccountType,
    pub identity: UserIdentity,
    pub business: Option<BusinessProfile>,
    pub hardware_completed: bool,
    pub models: ModelSelection,
    pub onboarding_completed: bool,
    pub created_at: String,
    pub updated_at: String,
}

impl UserProfile {
    pub fn new() -> Self {
        let now = chrono::Local::now().to_rfc3339();
        Self {
            created_at: now.clone(),
            updated_at: now,
            ..Default::default()
        }
    }

    /// Touch the updated_at timestamp
    pub fn touch(&mut self) {
        self.updated_at = chrono::Local::now().to_rfc3339();
    }

    /// Get display name — returns identity name, auth name, or "Sovereign"
    pub fn display_name(&self) -> &str {
        if !self.identity.name.is_empty() {
            &self.identity.name
        } else if !self.auth.display_name.is_empty() {
            &self.auth.display_name
        } else {
            "Sovereign"
        }
    }
}
