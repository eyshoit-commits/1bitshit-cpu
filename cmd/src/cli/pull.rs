use color_eyre::Result;
use colored::Colorize;
use engines::models::registry::{CoreRoster, ModelManifest};
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use tokio::sync::mpsc;

const PULL_IMPLEMENTATION: &str = "visible-models-v2";

/// Pull a model into the visible `./models/` tree, load it and enter the dashboard.
pub async fn execute(model_id: &str) -> Result<()> {
    println!(
        "\n  {} [1BitShit CPU:{}] Resolving '{}'...",
        "⚙️".yellow(),
        PULL_IMPLEMENTATION,
        model_id.bold()
    );

    let resolved_id = if !model_id.starts_with("hf://")
        && !model_id.starts_with("https://")
        && model_id.contains('/')
    {
        format!("hf://{model_id}")
    } else {
        model_id.to_string()
    };

    let manifest = if resolved_id.starts_with("hf://")
        || resolved_id.starts_with("https://huggingface.co/")
    {
        resolve_huggingface_manifest(&resolved_id).await?
    } else {
        let roster = CoreRoster::load_roster();
        roster
            .into_iter()
            .find(|model| model.id.eq_ignore_ascii_case(model_id))
            .ok_or_else(|| color_eyre::eyre::eyre!("Model ID '{model_id}' not found"))?
    };

    if manifest.id.to_ascii_lowercase().contains("unknown") {
        return Err(color_eyre::eyre::eyre!(
            "Refusing stale model id '{}'. Delete the legacy hidden model entry and retry.",
            manifest.id
        ));
    }

    let cached = engines::ModelDownloader::get_cached_path(
        &manifest.category,
        &manifest.id,
        &manifest.huggingface_filename,
    );

    let model_path = if let Some(path) = cached {
        println!("  {} Model already exists at {}", "ℹ️".cyan(), path.display());
        path
    } else {
        println!(
            "  {} Downloading to ./models/{}/{} ...",
            "📥".cyan(),
            manifest.category,
            manifest.id.replace(':', "-")
        );

        let (tx, mut rx) = mpsc::channel(256);
        let abort = Arc::new(AtomicBool::new(false));
        let download = engines::ModelDownloader::download_gguf_async(
            &manifest.category,
            &manifest.id,
            &manifest.download_url,
            &manifest.huggingface_filename,
            manifest.assets.clone(),
            Some(manifest.clone()),
            tx,
            abort,
        );

        tokio::pin!(download);
        loop {
            tokio::select! {
                result = &mut download => {
                    break result.map_err(|error| color_eyre::eyre::eyre!(error))?;
                }
                event = rx.recv() => {
                    if let Some(engines::DownloadEvent::Progress(_, current, total, speed, _)) = event {
                        if total > 0 {
                            print!(
                                "\r  {} {:.1}%  {:.1}/{:.1} MB  {:.1} MB/s",
                                "⬇".cyan(),
                                current as f64 / total as f64 * 100.0,
                                current as f64 / 1_048_576.0,
                                total as f64 / 1_048_576.0,
                                speed / 1_048_576.0,
                            );
                            use std::io::Write;
                            let _ = std::io::stdout().flush();
                        }
                    }
                }
            }
        }
    };

    if !model_path.is_file() {
        return Err(color_eyre::eyre::eyre!(
            "Downloaded model file is missing: {}",
            model_path.display()
        ));
    }

    println!("\n  {} Loading {}...", "⚙️".yellow(), manifest.name.bold());
    let mut app = crate::core::app::App::new(Some(manifest.clone()), None)?;
    app.state
        .Core_engine
        .load_model(model_path)
        .await
        .map_err(|error| color_eyre::eyre::eyre!(error))?;
    app.state._active_model_id = Some(manifest.id.clone());
    engines::neural_foundry::security::permission_schema::PermissionSchema::set_active_chat_model(
        manifest.id.clone(),
    );
    app.state.auto_mount_triggered = true;

    println!("  {} Model is active. Entering dashboard.\n", "✅".green());
    app.run().await
}

async fn resolve_huggingface_manifest(resolved_id: &str) -> Result<ModelManifest> {
    let repo_id = resolved_id
        .trim_start_matches("hf://")
        .trim_start_matches("https://huggingface.co/")
        .trim_end_matches('/');

    println!("  {} Scanning HuggingFace Hub for '{}'...", "🔍".cyan(), repo_id);
    let variants = engines::models::manager::hf_hub::HuggingFaceHub::list_variants(repo_id)
        .await
        .map_err(|error| color_eyre::eyre::eyre!(error))?;
    let options: Vec<String> = variants
        .iter()
        .map(|variant| format!("{} ({:.2} GB)", variant.filename, variant.size_gb))
        .collect();
    let selection = inquire::Select::new("Select GGUF variant to download:", options).prompt()?;
    let filename = selection
        .split(" (")
        .next()
        .ok_or_else(|| color_eyre::eyre::eyre!("Invalid GGUF selection"))?;
    let size_gb = variants
        .iter()
        .find(|variant| variant.filename == filename)
        .map(|variant| variant.size_gb)
        .unwrap_or(0.0);

    engines::models::manager::hf_hub::HuggingFaceHub::build_manifest(
        repo_id,
        filename,
        size_gb,
    )
    .await
    .map_err(|error| color_eyre::eyre::eyre!(error))
}
