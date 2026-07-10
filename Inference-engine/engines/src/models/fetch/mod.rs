use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use futures_util::StreamExt;
use indicatif::{ProgressBar, ProgressStyle};
use tokio::io::AsyncWriteExt;
use tokio::sync::mpsc;
use tracing::info;

use crate::models::registry::ModelManifest;

#[derive(Debug, Clone)]
pub enum DownloadEvent {
    Progress(f32, u64, u64, f64, u64),
    Complete(String),
    Error(String, String),
    PurgeComplete(String),
    PurgeError(String, String),
}

pub struct ModelDownloader;

impl ModelDownloader {
    /// Models are intentionally stored in a visible `models/` directory.
    /// `BITSHIT_MODELS_DIR` may override the location for installed or portable setups.
    fn get_models_dir() -> PathBuf {
        let dir = std::env::var_os("BITSHIT_MODELS_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|| {
                std::env::current_dir()
                    .unwrap_or_else(|_| PathBuf::from("."))
                    .join("models")
            });

        if let Err(error) = std::fs::create_dir_all(&dir) {
            tracing::warn!("Failed to create visible model directory {}: {}", dir.display(), error);
        }
        dir
    }

    /// One canonical directory name is used by download, lookup, purge and cleanup.
    /// Previously download kept ':' while cache lookup replaced it with '-', making
    /// freshly downloaded models immediately invisible to the roster.
    fn model_dir_name(repo_id: &str) -> String {
        let raw = repo_id.split('/').next_back().unwrap_or(repo_id);
        raw.chars()
            .map(|ch| match ch {
                ':' | '/' | '\\' | ' ' => '-',
                _ => ch,
            })
            .collect::<String>()
            .trim_matches('-')
            .to_string()
    }

    fn model_dir(category: &str, repo_id: &str) -> PathBuf {
        Self::get_models_dir()
            .join(category)
            .join(Self::model_dir_name(repo_id))
    }

    pub fn is_model_cached(category: &str, repo_id: &str, filename: &str) -> bool {
        Self::get_cached_path(category, repo_id, filename).is_some()
    }

    pub fn get_cached_path(category: &str, repo_id: &str, filename: &str) -> Option<PathBuf> {
        let repo_path = Self::model_dir(category, repo_id);
        let file_basename = Path::new(filename)
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or(filename);

        let exact_path = repo_path.join(file_basename);
        if exact_path.is_file() {
            return Some(exact_path);
        }

        let entries = std::fs::read_dir(&repo_path).ok()?;
        entries
            .flatten()
            .map(|entry| entry.path())
            .find(|path| {
                path.is_file()
                    && matches!(
                        path.extension().and_then(|ext| ext.to_str()).map(str::to_ascii_lowercase),
                        Some(ext) if ext == "gguf" || ext == "bin"
                    )
            })
    }

    pub async fn download_gguf_async(
        category: &str,
        repo_id: &str,
        download_url: &str,
        filename: &str,
        _assets: Vec<crate::models::registry::ModelAsset>,
        manifest: Option<ModelManifest>,
        tx: mpsc::Sender<DownloadEvent>,
        abort: Arc<AtomicBool>,
    ) -> Result<PathBuf, String> {
        let dest_dir = Self::model_dir(category, repo_id);
        std::fs::create_dir_all(&dest_dir).map_err(|error| error.to_string())?;

        let file_basename = Path::new(filename)
            .file_name()
            .and_then(|name| name.to_str())
            .filter(|name| !name.is_empty() && *name != "unknown")
            .ok_or_else(|| format!("Invalid model filename for {repo_id}: {filename}"))?;

        let weight_path = dest_dir.join(file_basename);
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(3600))
            .user_agent("1bitshit-cpu/0.2")
            .default_headers({
                let mut headers = reqwest::header::HeaderMap::new();
                headers.insert(
                    reqwest::header::REFERER,
                    reqwest::header::HeaderValue::from_static("https://huggingface.co/"),
                );
                headers.insert(
                    reqwest::header::ACCEPT,
                    reqwest::header::HeaderValue::from_static("*/*"),
                );
                headers
            })
            .build()
            .map_err(|error| error.to_string())?;

        Self::download_single_file(
            &client,
            download_url,
            &weight_path,
            tx,
            abort,
        )
        .await?;

        if let Some(mut model_manifest) = manifest {
            model_manifest.local_path = Some(weight_path.to_string_lossy().to_string());
            let manifest_path = dest_dir.join("model_manifest.json");
            let json = serde_json::to_string_pretty(&model_manifest)
                .map_err(|error| error.to_string())?;
            std::fs::write(&manifest_path, json).map_err(|error| error.to_string())?;
            Self::generate_cluaiz_dna(&model_manifest, &dest_dir, &weight_path)?;
        }

        if !weight_path.is_file() {
            return Err(format!(
                "Download completed without a readable model file: {}",
                weight_path.display()
            ));
        }

        Ok(weight_path)
    }

    pub fn generate_cluaiz_dna(
        manifest: &ModelManifest,
        dest_dir: &Path,
        weight_path: &Path,
    ) -> Result<(), String> {
        info!("[DNA] Generating structural metadata for '{}'", manifest.id);

        let mut dna = crate::models::registry::StructuralDNA::create_skeleton(
            manifest.id.clone(),
            manifest.has_vision,
            manifest.expert_count,
            manifest.bit_depth,
            &manifest.context_window,
        );

        dna.dynamic_attributes
            .insert("bit_depth".to_string(), manifest.bit_depth.to_string());
        dna.dynamic_attributes
            .insert("parameters".to_string(), manifest.parameters.clone());
        dna.dynamic_attributes.insert(
            "training_tokens".to_string(),
            manifest.training_tokens.clone(),
        );
        dna.dynamic_attributes
            .insert("category".to_string(), manifest.category.clone());

        if weight_path.is_file() {
            if let Ok((metadata, _, _)) =
                cluaiz_shared::utils::gguf_prober::GGUFProber::probe(weight_path)
            {
                if let Some(context) = metadata
                    .get("llama.context_length")
                    .or_else(|| metadata.get("qwen2.context_length"))
                {
                    dna.max_context_length = context.parse().ok();
                }
                if let Some(template) = metadata.get("tokenizer.chat_template") {
                    dna.chat_template = Some(template.clone());
                }
                if let Some(eos) = metadata.get("tokenizer.ggml.eos_token_id") {
                    dna.eos_token = Some(eos.clone());
                }
            }
        }

        let dna_path = dest_dir.join("structural_dna.json");
        let json = serde_json::to_string_pretty(&dna).map_err(|error| error.to_string())?;
        std::fs::write(dna_path, json).map_err(|error| error.to_string())
    }

    async fn download_single_file(
        client: &reqwest::Client,
        url: &str,
        dest_path: &Path,
        tx: mpsc::Sender<DownloadEvent>,
        abort: Arc<AtomicBool>,
    ) -> Result<(), String> {
        if url.trim().is_empty() {
            return Err("Model download URL is empty".to_string());
        }

        let response = client.get(url).send().await.map_err(|error| error.to_string())?;
        if !response.status().is_success() {
            return Err(format!("Download failed for {url}: HTTP {}", response.status()));
        }

        let total_size = response.content_length().unwrap_or(0);
        let partial_path = dest_path.with_extension(format!(
            "{}.part",
            dest_path
                .extension()
                .and_then(|ext| ext.to_str())
                .unwrap_or("download")
        ));
        let mut file = tokio::fs::File::create(&partial_path)
            .await
            .map_err(|error| error.to_string())?;
        let mut downloaded = 0_u64;
        let started = std::time::Instant::now();

        let progress = ProgressBar::new(total_size);
        progress.set_style(
            ProgressStyle::default_bar()
                .template("{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} ({eta})")
                .map_err(|error| error.to_string())?
                .progress_chars("#>-"),
        );

        let mut stream = response.bytes_stream();
        while let Some(item) = stream.next().await {
            if abort.load(Ordering::SeqCst) {
                drop(file);
                let _ = std::fs::remove_file(&partial_path);
                progress.abandon_with_message("Download aborted");
                return Err("ABORTED".to_string());
            }

            let chunk = item.map_err(|error| error.to_string())?;
            file.write_all(&chunk)
                .await
                .map_err(|error| error.to_string())?;
            downloaded += chunk.len() as u64;
            progress.set_position(downloaded);

            let elapsed = started.elapsed().as_secs_f64().max(0.001);
            let speed = downloaded as f64 / elapsed;
            let eta = if speed > 0.0 && total_size > downloaded {
                ((total_size - downloaded) as f64 / speed) as u64
            } else {
                0
            };
            let fraction = if total_size > 0 {
                downloaded as f32 / total_size as f32
            } else {
                0.0
            };
            let _ = tx.send(DownloadEvent::Progress(
                fraction,
                downloaded,
                total_size,
                speed,
                eta,
            ));
        }

        file.flush().await.map_err(|error| error.to_string())?;
        drop(file);
        tokio::fs::rename(&partial_path, dest_path)
            .await
            .map_err(|error| error.to_string())?;
        progress.finish_with_message("Download complete");
        Ok(())
    }

    pub async fn fetch_asset_auto_heal(
        repo_id: &str,
        dest_dir: &Path,
        asset_name: &str,
    ) -> Result<(), String> {
        let asset_path = dest_dir.join(asset_name);
        if asset_path.exists() {
            return Ok(());
        }

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(5))
            .build()
            .map_err(|error| error.to_string())?;
        let mut repo_ids = vec![repo_id.to_string()];
        let stripped = repo_id.replace("-GGUF", "").replace("-gguf", "");
        if stripped != repo_id {
            repo_ids.push(stripped);
        }

        for id in repo_ids {
            let url = format!("https://huggingface.co/{id}/resolve/main/{asset_name}");
            let Ok(response) = client.get(&url).send().await else {
                continue;
            };
            if !response.status().is_success() {
                continue;
            }

            let mut file = tokio::fs::File::create(&asset_path)
                .await
                .map_err(|error| error.to_string())?;
            let mut stream = response.bytes_stream();
            while let Some(item) = stream.next().await {
                let chunk = item.map_err(|error| error.to_string())?;
                file.write_all(&chunk)
                    .await
                    .map_err(|error| error.to_string())?;
            }
            return Ok(());
        }

        Ok(())
    }

    pub fn download_gguf(
        category: &str,
        repo_id: &str,
        download_url: &str,
        filename: &str,
        assets: Vec<crate::models::registry::ModelAsset>,
        manifest: Option<ModelManifest>,
        tx: mpsc::Sender<DownloadEvent>,
        abort: Arc<AtomicBool>,
    ) -> Result<PathBuf, String> {
        let runtime = tokio::runtime::Runtime::new().map_err(|error| error.to_string())?;
        runtime.block_on(Self::download_gguf_async(
            category,
            repo_id,
            download_url,
            filename,
            assets,
            manifest,
            tx,
            abort,
        ))
    }

    pub fn purge_model(category: &str, repo_id: &str) -> Result<(), String> {
        let path = Self::model_dir(category, repo_id);
        if !path.exists() {
            return Err("Model directory not found".to_string());
        }
        std::fs::remove_dir_all(path).map_err(|error| error.to_string())
    }

    pub fn cleanup_partial_download(category: &str, repo_id: &str) -> Result<(), String> {
        let path = Self::model_dir(category, repo_id);
        if let Ok(entries) = std::fs::read_dir(path) {
            for entry in entries.flatten() {
                let file_path = entry.path();
                let is_partial = file_path
                    .extension()
                    .and_then(|ext| ext.to_str())
                    .map(|ext| ext == "part" || ext == "lock")
                    .unwrap_or(false);
                if is_partial {
                    let _ = std::fs::remove_file(file_path);
                }
            }
        }
        Ok(())
    }
}
