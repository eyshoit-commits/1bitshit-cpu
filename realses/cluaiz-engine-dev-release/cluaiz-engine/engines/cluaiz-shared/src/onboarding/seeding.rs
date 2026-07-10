//! ═══════════════════════════════════════════════════════════════════════
//!  Workspace Seeding — Generate Identity Files from Profile
//! ═══════════════════════════════════════════════════════════════════════
//!  Creates IDENTITY.md, SOUL.md, USER.md in ~/.archer/workspace/
//!  Also saves user_profile.json for persistence.
//!  Reusable across CLI, Desktop, Web.
//! ═══════════════════════════════════════════════════════════════════════

use std::fs;
use std::path::PathBuf;
use crate::profile::{UserProfile, AccountType, persistence};

/// Get the workspace directory path
pub fn get_workspace_path() -> PathBuf {
    persistence::get_archer_dir().join("workspace")
}

/// Seed the entire workspace from a completed profile
pub fn seed_workspace(profile: &UserProfile) -> Result<(), String> {
    let workspace = get_workspace_path();
    if !workspace.exists() {
        fs::create_dir_all(&workspace)
            .map_err(|e| format!("Failed to create workspace: {}", e))?;
    }

    // 🧬 IDENTITY.md
    let identity = format!(
        "# 🧬 SOVEREIGN IDENTITY\n\n\
         - **Operator**: {}\n\
         - **Account Type**: {}\n\
         - **Auth Method**: {:?}\n\
         - **Email**: {}\n\
         - **Initialization**: {}\n",
        profile.display_name(),
        profile.account_type,
        profile.auth.method,
        profile.auth.email,
        profile.created_at,
    );
    fs::write(workspace.join("IDENTITY.md"), identity)
        .map_err(|e| format!("Failed to write IDENTITY.md: {}", e))?;

    // 🧘 SOUL.md
    let soul = match &profile.account_type {
        AccountType::Personal => {
            format!(
                "# 🧘 AGENT SOUL\n\n\
                 - **Core Directive**: Personal AI Assistant\n\
                 - **User Link**: {}\n\
                 - **Trust Level**: SOVEREIGN\n\
                 - **Mode**: Personal Assistant\n",
                profile.display_name(),
            )
        }
        AccountType::Business => {
            let biz = profile.business.as_ref();
            format!(
                "# 🧘 AGENT SOUL\n\n\
                 - **Core Directive**: Business AI Assistant\n\
                 - **Business**: {}\n\
                 - **Industry**: {}\n\
                 - **Goal**: {}\n\
                 - **User Link**: {}\n\
                 - **Trust Level**: SOVEREIGN\n",
                biz.map(|b| b.name.as_str()).unwrap_or("—"),
                biz.map(|b| b.industry.as_str()).unwrap_or("—"),
                biz.map(|b| b.primary_goal.as_str()).unwrap_or("—"),
                profile.display_name(),
            )
        }
    };
    fs::write(workspace.join("SOUL.md"), soul)
        .map_err(|e| format!("Failed to write SOUL.md: {}", e))?;

    // 📝 USER.md
    let user = "# 📝 OPERATOR LOGS\n\nInitialized as neural operator.\n";
    fs::write(workspace.join("USER.md"), user)
        .map_err(|e| format!("Failed to write USER.md: {}", e))?;

    // 💾 Save profile JSON
    persistence::save_profile(profile)?;

    Ok(())
}
