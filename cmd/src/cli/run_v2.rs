use std::io::{BufRead, Write};
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use color_eyre::Result;
use colored::Colorize;
use engines::models::registry::{CoreRoster, ModelManifest};
use tokio::sync::mpsc;

/// Runs a local model through the same visible model store and loader used by
/// `bitshit pull`. No hidden runtime, secondary downloader or legacy IPC path
/// is involved.
pub async fn execute(model_id: &str, interactive: bool) -> Result<()> {
    println!(
        "\n  {} [1BitShit CPU] Resolving '{}'...",
        "⚙️".yellow(),
        model_id.bold()
    );

    let manifest = resolve_manifest(model_id).await?;
    reject_stale_manifest(&manifest)?;
    let model_path = ensure_local_model(&manifest).await?;

    if !model_path.is_file() {
        return Err(color_eyre::eyre::eyre!(
            "Model file is missing after synchronization: {}",
            model_path.display()
        ));
    }

    if model_path
        .extension()
        .and_then(|extension| extension.to_str())
        .map(str::to_ascii_lowercase)
        .as_deref()
        == Some("onnx")
    {
        engines::neural_foundry::security::permission_schema::PermissionSchema::set_active_embedding_model(
            manifest.id.clone(),
        );
    } else {
        engines::neural_foundry::security::permission_schema::PermissionSchema::set_active_chat_model(
            manifest.id.clone(),
        );
    }

    if interactive {
        launch_dashboard(manifest, model_path).await
    } else {
        run_batch(model_path).await
    }
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
            "Model '{}' is not present in the local registry. Use a Hugging Face owner/repository ID or run 'bitshit list'.",
            normalized
        ))
}

async fn resolve_huggingface_manifest(repository: &str) -> Result<ModelManifest> {
    println!(
        "  {} Scanning Hugging Face repository '{}'...",
        "🔍".cyan(),
        repository
    );
    let variants = engines::models::manager::hf_hub::HuggingFaceHub::list_variants(repository)
        .await
        .map_err(|error| color_eyre::eyre::eyre!(error))?;
    let options: Vec<String> = variants
        .iter()
        .map(|variant| format!("{} ({:.2} GB)", variant.filename, variant.size_gb))
        .collect();
    let selection = inquire::Select::new("Select GGUF variant:", options).prompt()?;
    let filename = selection
        .split(" (")
        .next()
        .ok_or_else(|| color_eyre::eyre::eyre!("Invalid model selection"))?;
    let size_gb = variants
        .iter()
        .find(|variant| variant.filename == filename)
        .map(|variant| variant.size_gb)
        .unwrap_or(0.0);

    engines::models::manager::hf_hub::HuggingFaceHub::build_manifest(
        repository,
        filename,
        size_gb,
    )
    .await
    .map_err(|error| color_eyre::eyre::eyre!(error))
}

fn reject_stale_manifest(manifest: &ModelManifest) -> Result<()> {
    let invalid = manifest.id.trim().is_empty()
        || manifest.id.to_ascii_lowercase().contains("unknown")
        || manifest.huggingface_filename.trim().is_empty()
        || manifest
            .huggingface_filename
            .eq_ignore_ascii_case("unknown");
    if invalid {
        return Err(color_eyre::eyre::eyre!(
            "Refusing incomplete model manifest '{}'. Remove the stale legacy entry and synchronize the model again.",
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
        println!("  {} Using local model: {}", "✅".green(), path.display());
        return Ok(path);
    }

    println!(
        "  {} Downloading to ./models/{}/{} ...",
        "📥".cyan(),
        manifest.category,
        manifest.id.replace(':', "-")
    );

    let (sender, mut receiver) = mpsc::channel(256);
    let abort = Arc::new(AtomicBool::new(false));
    let download = engines::ModelDownloader::download_gguf_async(
        &manifest.category,
        &manifest.id,
        &manifest.download_url,
        &manifest.huggingface_filename,
        manifest.assets.clone(),
        Some(manifest.clone()),
        sender,
        abort,
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
                if let Some(engines::DownloadEvent::Progress(_, current, total, speed, _)) = event {
                    if total > 0 {
                        print!(
                            "\r  {} {:6.2}%  {:.1}/{:.1} MB  {:.1} MB/s",
                            "⬇".cyan(),
                            current as f64 / total as f64 * 100.0,
                            current as f64 / 1_048_576.0,
                            total as f64 / 1_048_576.0,
                            speed / 1_048_576.0,
                        );
                        let _ = std::io::stdout().flush();
                    }
                }
            }
        }
    }
}

async fn launch_dashboard(
    manifest: ModelManifest,
    model_path: std::path::PathBuf,
) -> Result<()> {
    println!(
        "  {} Loading '{}' through the 1BitShit runtime...",
        "⚙️".yellow(),
        manifest.name.bold()
    );
    let mut app = crate::core::app::App::new(Some(manifest.clone()), None)?;
    app.state
        .Core_engine
        .load_model(model_path)
        .await
        .map_err(|error| color_eyre::eyre::eyre!("Model loading failed: {error}"))?;
    app.state._active_model_id = Some(manifest.id);
    app.state.auto_mount_triggered = true;
    println!("  {} Model is active.\n", "✅".green());
    app.run().await
}

async fn run_batch(model_path: std::path::PathBuf) -> Result<()> {
    use cluaiz_shared::UnifiedBackend;

    if model_path
        .extension()
        .and_then(|extension| extension.to_str())
        .map(str::to_ascii_lowercase)
        .as_deref()
        == Some("onnx")
    {
        return Err(color_eyre::eyre::eyre!(
            "ONNX embedding models do not provide text generation in batch chat mode. They remain available through the API embedding routes."
        ));
    }

    println!("  {} Loading batch runtime...", "⚙️".yellow());
    let mut router = engines::CoreRouter::load_model(
        model_path,
        cluaiz_shared::BackendType::RuntimeB,
    )
    .await
    .map_err(|error| color_eyre::eyre::eyre!(error))?;

    println!(
        "  {} Batch mode ready. Enter one prompt per line. Type 'exit' to stop.",
        "✅".green()
    );
    let stdin = std::io::stdin();
    let mut input = stdin.lock();
    loop {
        print!("1bitshit> ");
        std::io::stdout().flush()?;
        let mut line = String::new();
        if input.read_line(&mut line)? == 0 {
            break;
        }
        let prompt = line.trim();
        if prompt.is_empty() {
            continue;
        }
        if matches!(prompt.to_ascii_lowercase().as_str(), "exit" | "quit") {
            break;
        }

        match router.active_backend.generate(prompt, 4096) {
            Ok(response) => println!("{response}"),
            Err(error) => eprintln!("Inference failed: {error}"),
        }
    }
    Ok(())
}
