use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use color_eyre::Result;
use colored::Colorize;
use engines::models::registry::{CoreRoster, ModelManifest};
use tokio::sync::mpsc;

const PULL_IMPLEMENTATION: &str = "visible-models-v3-gguf-onnx";

/// Downloads a GGUF, BitNet or ONNX model into the canonical visible store.
/// Chat-capable models are loaded immediately. Embedding, vision and audio
/// ONNX models are registered without being forced through the chat loader.
pub async fn execute(model_id: &str) -> Result<()> {
    println!(
        "\n  {} [1BitShit CPU:{}] Resolving '{}'...",
        "⚙️".yellow(),
        PULL_IMPLEMENTATION,
        model_id.bold()
    );

    let manifest = resolve_manifest(model_id).await?;
    validate_manifest(&manifest)?;
    let model_path = ensure_local_model(&manifest).await?;

    if !model_path.is_file() {
        return Err(color_eyre::eyre::eyre!(
            "Downloaded model file is missing: {}",
            model_path.display()
        ));
    }

    let format = model_path
        .extension()
        .and_then(|extension| extension.to_str())
        .unwrap_or(&manifest.architecture_type)
        .to_ascii_lowercase();
    let chat_capable = manifest.category.eq_ignore_ascii_case("chat")
        || manifest.category.eq_ignore_ascii_case("code")
        || format == "gguf"
        || format == "bin";

    if !chat_capable && format == "onnx" {
        engines::neural_foundry::security::permission_schema::PermissionSchema::set_active_embedding_model(
            manifest.id.clone(),
        );
        println!(
            "\n  {} ONNX {} model '{}' is installed and active at {}.",
            "✅".green(),
            manifest.category,
            manifest.id.cyan(),
            model_path.display()
        );
        println!(
            "  {} It is available to embeddings, ingestion, vision/audio helpers and the API.\n",
            "ℹ️".cyan()
        );
        return Ok(());
    }

    println!(
        "\n  {} Loading '{}' through the {} runtime...",
        "⚙️".yellow(),
        manifest.name.bold(),
        if format == "onnx" { "ONNX" } else { "Llama/GGUF" }
    );
    let mut app = crate::core::app::App::new(Some(manifest.clone()), None)?;
    app.state
        .Core_engine
        .load_model(model_path)
        .await
        .map_err(|error| color_eyre::eyre::eyre!(
            "{} model loading failed: {}",
            format.to_ascii_uppercase(),
            error
        ))?;
    app.state._active_model_id = Some(manifest.id.clone());
    engines::neural_foundry::security::permission_schema::PermissionSchema::set_active_chat_model(
        manifest.id.clone(),
    );
    app.state.auto_mount_triggered = true;

    println!("  {} Model is active. Entering dashboard.\n", "✅".green());
    app.run().await
}

async fn resolve_manifest(model_id: &str) -> Result<ModelManifest> {
    let normalized = model_id
        .trim()
        .trim_start_matches("hf://")
        .trim_start_matches("https://huggingface.co/")
        .trim_end_matches('/');

    if normalized.contains('/') {
        return resolve_huggingface_manifest(normalized).await;
    }

    CoreRoster::load_roster()
        .into_iter()
        .find(|model| model.id.eq_ignore_ascii_case(normalized))
        .ok_or_else(|| color_eyre::eyre::eyre!(
            "Model ID '{}' was not found. Use a Hugging Face owner/repository ID or run 'bitshit list'.",
            normalized
        ))
}

fn validate_manifest(manifest: &ModelManifest) -> Result<()> {
    if manifest.id.trim().is_empty()
        || manifest.id.to_ascii_lowercase().contains("unknown")
        || manifest.huggingface_filename.trim().is_empty()
        || manifest
            .huggingface_filename
            .eq_ignore_ascii_case("unknown")
    {
        return Err(color_eyre::eyre::eyre!(
            "Refusing incomplete model manifest '{}'. Remove the stale legacy entry and synchronize again.",
            manifest.id
        ));
    }
    Ok(())
}

async fn ensure_local_model(manifest: &ModelManifest) -> Result<std::path::PathBuf> {
    if let Some(path) = engines::ModelDownloader::get_cached_path(
        &manifest.category,
        &manifest.id,
        &manifest.huggingface_filename,
    ) {
        println!("  {} Model already exists at {}", "ℹ️".cyan(), path.display());
        return Ok(path);
    }

    let models_root = cluaiz_shared::environment::EnvironmentManager::current().models_dir();
    println!(
        "  {} Downloading to {}/{}/{} ...",
        "📥".cyan(),
        models_root.display(),
        manifest.category,
        manifest.id.replace(':', "-")
    );
    if !manifest.assets.is_empty() {
        println!(
            "  {} {} companion file(s) will be synchronized.",
            "🧩".cyan(),
            manifest.assets.len()
        );
    }

    let (sender, mut receiver) = mpsc::channel(256);
    let download = engines::ModelDownloader::download_gguf_async(
        &manifest.category,
        &manifest.id,
        &manifest.download_url,
        &manifest.huggingface_filename,
        manifest.assets.clone(),
        Some(manifest.clone()),
        sender,
        Arc::new(AtomicBool::new(false)),
    );
    tokio::pin!(download);

    loop {
        tokio::select! {
            result = &mut download => {
                let path = result.map_err(|error| color_eyre::eyre::eyre!(error))?;
                println!("\n  {} Download complete: {}", "✅".green(), path.display());
                return Ok(path);
            }
            event = receiver.recv() => {
                match event {
                    Some(engines::DownloadEvent::Progress(_, current, total, speed, _)) if total > 0 => {
                        print!(
                            "\r  {} {:6.2}%  {:.1}/{:.1} MB  {:.1} MB/s",
                            "⬇".cyan(),
                            current as f64 / total as f64 * 100.0,
                            current as f64 / 1_048_576.0,
                            total as f64 / 1_048_576.0,
                            speed / 1_048_576.0,
                        );
                        use std::io::Write;
                        let _ = std::io::stdout().flush();
                    }
                    Some(engines::DownloadEvent::Error(_, error)) => {
                        eprintln!("\n  {} Download error: {}", "❌".red(), error);
                    }
                    _ => {}
                }
            }
        }
    }
}

async fn resolve_huggingface_manifest(repository_id: &str) -> Result<ModelManifest> {
    println!(
        "  {} Scanning Hugging Face repository '{}'...",
        "🔍".cyan(),
        repository_id
    );
    let variants = engines::models::manager::hf_hub::HuggingFaceHub::list_variants(repository_id)
        .await
        .map_err(|error| color_eyre::eyre::eyre!(error))?;
    let options: Vec<String> = variants
        .iter()
        .map(|variant| {
            let format = std::path::Path::new(&variant.filename)
                .extension()
                .and_then(|extension| extension.to_str())
                .unwrap_or("model")
                .to_ascii_uppercase();
            format!("[{}] {} ({:.2} GB)", format, variant.filename, variant.size_gb)
        })
        .collect();
    let selection = inquire::Select::new("Select model variant to download:", options).prompt()?;
    let filename = selection
        .split("] ")
        .nth(1)
        .and_then(|value| value.split(" (").next())
        .ok_or_else(|| color_eyre::eyre::eyre!("Invalid model selection"))?;
    let size_gb = variants
        .iter()
        .find(|variant| variant.filename == filename)
        .map(|variant| variant.size_gb)
        .unwrap_or(0.0);

    engines::models::manager::hf_hub::HuggingFaceHub::build_manifest(
        repository_id,
        filename,
        size_gb,
    )
    .await
    .map_err(|error| color_eyre::eyre::eyre!(error))
}
