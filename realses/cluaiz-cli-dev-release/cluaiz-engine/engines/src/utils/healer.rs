//! Healer module for automatically resolving common engine issues like missing tokenizers.
use crate::models::fetch::ModelDownloader;
use std::path::Path;

pub struct AutoHealer;

impl AutoHealer {
    /// Checks if a tokenizer exists for the given repo; if not, triggers an auto-download.
    pub async fn heal_missing_tokenizer(repo_id: &str, dest_dir: &Path) -> Result<(), String> {
        if !dest_dir.join("tokenizer.json").exists() {
            println!(
                "🛠️ [HEALER] Missing tokenizer detected for {}. Attempting recovery...",
                repo_id
            );
            ModelDownloader::fetch_asset_auto_heal(repo_id, dest_dir, "tokenizer.json").await?;
            println!("✅ [HEALER] Tokenizer successfully recovered.");
        }
        Ok(())
    }
}
