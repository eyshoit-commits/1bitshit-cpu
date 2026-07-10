//! ═══════════════════════════════════════════════════════════════════════
//!  Profile Persistence — JSON Save/Load for UserProfile
//! ═══════════════════════════════════════════════════════════════════════
//!  Save path: ~/.archer/user_profile.json
//!  Reusable across CLI, Desktop, Web — all read/write same file.
//! ═══════════════════════════════════════════════════════════════════════

use std::fs;
use std::path::PathBuf;
use crate::profile::UserProfile;

const PROFILE_DIR: &str = ".archer";
const PROFILE_FILE: &str = "user_profile.json";

/// Get the ~/.archer/ directory path
pub fn get_archer_dir() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(PROFILE_DIR)
}

/// Get the full path to user_profile.json
pub fn get_profile_path() -> PathBuf {
    get_cluaiz_dir().join(PROFILE_FILE)
}

/// Check if a user profile already exists
pub fn profile_exists() -> bool {
    get_profile_path().exists()
}

/// Save user profile to JSON
pub fn save_profile(profile: &UserProfile) -> Result<(), String> {
    let dir = get_cluaiz_dir();
    if !dir.exists() {
        fs::create_dir_all(&dir)
            .map_err(|e| format!("Failed to create ~/.archer/: {}", e))?;
    }

    let json = serde_json::to_string_pretty(profile)
        .map_err(|e| format!("Failed to serialize profile: {}", e))?;

    fs::write(get_profile_path(), json)
        .map_err(|e| format!("Failed to write profile: {}", e))?;

    Ok(())
}

/// Load user profile from JSON (returns None if file doesn't exist)
pub fn load_profile() -> Result<Option<UserProfile>, String> {
    let path = get_profile_path();
    if !path.exists() {
        return Ok(None);
    }

    let content = fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read profile: {}", e))?;

    let profile: UserProfile = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse profile: {}", e))?;

    Ok(Some(profile))
}
