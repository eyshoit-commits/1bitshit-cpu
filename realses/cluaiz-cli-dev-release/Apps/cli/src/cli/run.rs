use color_eyre::Result;
use colored::Colorize;
use engines::models::registry::CoreRoster;

use engines::models::registry::ModelManifest;

/// `cluaiz run <model-id>` — pulls the model (downloads if missing), then confirms ready.
pub async fn execute(model_id: &str) -> Result<ModelManifest> {
    // 🎨 Display the Gothic Logo
    let logo = crate::assets::logos::logo_gallery::LOGO_VARIANTS[9];
    println!("{}", logo.cyan());

    println!("\n  {} Checking '{}'...", "⚙️".yellow(), model_id.bold());

    // 1. Resolve Metadata: Local Library -> Registry
    let roster = CoreRoster::load_roster();
    let mut manifest = roster
        .into_iter()
        .find(|m| m.id.to_lowercase() == model_id.to_lowercase());

    if manifest.is_none() {
        println!(
            "  {} Model not found locally. Fetching registry...",
            "🌐".yellow()
        );
        let remote_models = CoreRoster::fetch_external_registry(None)
            .await
            .map_err(|e| color_eyre::eyre::eyre!(e))?;
        manifest = remote_models
            .into_iter()
            .find(|m| m.id.to_lowercase() == model_id.to_lowercase());
    }

    let manifest = manifest
        .ok_or_else(|| color_eyre::eyre::eyre!("ID '{}' not found in any registry.", model_id))?;

    // 🏛️ Display Model Intel
    println!("\n  {} [{}]", "📦".cyan(), manifest.name.bold().green());
    println!(
        "     Architecture: {} | Params: {}",
        manifest.architecture.dimmed(),
        manifest.parameters.dimmed()
    );
    println!(
        "     Context: {} | VRAM Req: {} GB\n",
        manifest.context_window.dimmed(),
        manifest.ram_required_gb.to_string().dimmed()
    );

    // 2. Initialize Manager with Global Home Path (~/.cluaiz/models)
    let home_dir = dirs::home_dir()
        .ok_or_else(|| color_eyre::eyre::eyre!("Could not resolve Home Directory"))?;
    let cluaiz_root = home_dir.join(".cluaiz").join("models");

    let manager = engines::models::manager::ModelManager::new(
        engines::models::registry::REGISTRY_URL.to_string(),
        cluaiz_root,
    );

    manager
        .pull_model(model_id)
        .await
        .map_err(|e| color_eyre::eyre::eyre!(e))?;

    println!(
        "  {} Model '{}' verified and ready. Launching dashboard...\n",
        "✅".green(),
        model_id.bold()
    );

    // Give a small pause for visual feedback before clearing screen for dashboard
    tokio::time::sleep(std::time::Duration::from_millis(800)).await;

    Ok(manifest)
}
