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
    fn get_models_dir() -> PathBuf {
        let directory = std::env::var_os("BITSHIT_MODELS_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|| {
                cluaiz_shared::environment::EnvironmentManager::current().models_dir()
            });
        if let Err(error) = std::fs::create_dir_all(&directory) {
            tracing::warn!(
                "Failed to create model directory {}: {}",
                directory.display(),
                error
            );
        }
        directory
    }

    fn model_dir_name(model_id: &str) -> String {
        let raw = model_id.split('/').next_back().unwrap_or(model_id);
        raw.chars()
            .map(|character| match character {
                ':' | '/' | '\\' | ' ' => '-',
                other => other,
            })
            .collect::<String>()
            .trim_matches('-')
            .to_string()
    }

    fn model_dir(category: &str, model_id: &str) -> PathBuf {
        Self::get_models_dir()
            .join(category)
            .join(Self::model_dir_name(model_id))
    }

    pub fn is_model_cached(category: &str, model_id: &str, filename: &str) -> bool {
        Self::get_cached_path(category, model_id, filename).is_some()
    }

    pub fn get_cached_path(
        category: &str,
        model_id: &str,
        filename: &str,
    ) -> Option<PathBuf> {
        let model_directory = Self::model_dir(category, model_id);
        let basename = Path::new(filename)
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or(filename);
        let exact = model_directory.join(basename);
        if exact.is_file() {
            return Some(exact);
        }

        std::fs::read_dir(model_directory)
            .ok()?
            .flatten()
            .map(|entry| entry.path())
            .find(|path| {
                path.is_file()
                    && path
                        .extension()
                        .and_then(|extension| extension.to_str())
                        .map(str::to_ascii_lowercase)
                        .map(|extension| {
                            matches!(
                                extension.as_str(),
                                "gguf" | "bin" | "onnx" | "safetensors"
                            )
                        })
                        .unwrap_or(false)
            })
    }

    pub async fn download_gguf_async(
        category: &str,
        model_id: &str,
        download_url: &str,
        filename: &str,
        _assets: Vec<crate::models::registry::ModelAsset>,
        manifest: Option<ModelManifest>,
        sender: mpsc::Sender<DownloadEvent>,
        abort: Arc<AtomicBool>,
    ) -> Result<PathBuf, String> {
        let destination_directory = Self::model_dir(category, model_id);
        std::fs::create_dir_all(&destination_directory).map_err(|error| error.to_string())?;

        let basename = Path::new(filename)
            .file_name()
            .and_then(|name| name.to_str())
            .filter(|name| !name.trim().is_empty() && !name.eq_ignore_ascii_case("unknown"))
            .ok_or_else(|| format!("Invalid model filename for {model_id}: {filename}"))?;
        let weight_path = destination_directory.join(basename);

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(3600))
            .user_agent(format!("1bitshit-cpu/{}", env!("CARGO_PKG_VERSION")))
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

        if let Err(error) = Self::download_single_file(
            &client,
            download_url,
            &weight_path,
            sender.clone(),
            abort,
        )
        .await
        {
            let _ = sender
                .send(DownloadEvent::Error(model_id.to_string(), error.clone()))
                .await;
            return Err(error);
        }

        if let Some(mut model_manifest) = manifest {
            model_manifest.local_path = Some(weight_path.to_string_lossy().to_string());
            let manifest_path = destination_directory.join("model_manifest.json");
            let json = serde_json::to_string_pretty(&model_manifest)
                .map_err(|error| error.to_string())?;
            std::fs::write(manifest_path, json).map_err(|error| error.to_string())?;
            Self::generate_bitshit_dna(
                &model_manifest,
                &destination_directory,
                &weight_path,
            )?;
        }

        if !weight_path.is_file() {
            let error = format!(
                "Download completed without a readable model file: {}",
                weight_path.display()
            );
            let _ = sender
                .send(DownloadEvent::Error(model_id.to_string(), error.clone()))
                .await;
            return Err(error);
        }

        let _ = sender
            .send(DownloadEvent::Complete(model_id.to_string()))
            .await;
        Ok(weight_path)
    }

    pub fn generate_bitshit_dna(
        manifest: &ModelManifest,
        destination_directory: &Path,
        weight_path: &Path,
    ) -> Result<(), String> {
        info!(
            "[1BitShit DNA] Generating structural metadata for '{}'",
            manifest.id
        );
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
        dna.dynamic_attributes.insert(
            "runtime".to_string(),
            "1bitshit-cpu".to_string(),
        );

        let is_gguf = weight_path
            .extension()
            .and_then(|extension| extension.to_str())
            .map(|extension| extension.eq_ignore_ascii_case("gguf"))
            .unwrap_or(false);
        if is_gguf {
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

        let json = serde_json::to_string_pretty(&dna).map_err(|error| error.to_string())?;
        std::fs::write(destination_directory.join("structural_dna.json"), json)
            .map_err(|error| error.to_string())
    }

    pub fn generate_cluaiz_dna(
        manifest: &ModelManifest,
        destination_directory: &Path,
        weight_path: &Path,
    ) -> Result<(), String> {
        Self::generate_bitshit_dna(manifest, destination_directory, weight_path)
    }

    async fn download_single_file(
        client: &reqwest::Client,
        url: &str,
        destination: &Path,
        sender: mpsc::Sender<DownloadEvent>,
        abort: Arc<AtomicBool>,
    ) -> Result<(), String> {
        if url.trim().is_empty() {
            return Err("Model download URL is empty".to_string());
        }
        let response = client.get(url).send().await.map_err(|error| error.to_string())?;
        if !response.status().is_success() {
            return Err(format!("Download failed for {url}: HTTP {}", response.status()));
        }

        let total = response.content_length().unwrap_or(0);
        let partial = destination.with_extension(format!(
            "{}.part",
            destination
                .extension()
                .and_then(|extension| extension.to_str())
                .unwrap_or("download")
        ));
        if partial.exists() {
            let _ = std::fs::remove_file(&partial);
        }
        let mut file = tokio::fs::File::create(&partial)
            .await
            .map_err(|error| error.to_string())?;
        let mut downloaded = 0_u64;
        let started = std::time::Instant::now();

        let progress = ProgressBar::new(total);
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
                let _ = std::fs::remove_file(&partial);
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
            let eta = if speed > 0.0 && total > downloaded {
                ((total - downloaded) as f64 / speed) as u64
            } else {
                0
            };
            let fraction = if total > 0 {
                downloaded as f32 / total as f32
            } else {
                0.0
            };
            let _ = sender
                .send(DownloadEvent::Progress(
                    fraction,
                    downloaded,
                    total,
                    speed,
                    eta,
                ))
                .await;
        }

        file.flush().await.map_err(|error| error.to_string())?;
        drop(file);
        if destination.exists() {
            tokio::fs::remove_file(destination)
                .await
                .map_err(|error| error.to_string())?;
        }
        tokio::fs::rename(&partial, destination)
            .await
            .map_err(|error| error.to_string())?;
        progress.finish_with_message("Download complete");
        Ok(())
    }

    pub async fn fetch_asset_auto_heal(
        repository_id: &str,
        destination_directory: &Path,
        asset_name: &str,
    ) -> Result<(), String> {
        let destination = destination_directory.join(asset_name);
        if destination.is_file() {
            return Ok(());
        }

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .user_agent(format!("1bitshit-cpu/{}", env!("CARGO_PKG_VERSION")))
            .build()
            .map_err(|error| error.to_string())?;
        let stripped = repository_id.replace("-GGUF", "").replace("-gguf", "");
        let repositories = if stripped == repository_id {
            vec![repository_id.to_string()]
        } else {
            vec![repository_id.to_string(), stripped]
        };

        for repository in repositories {
            let url = format!(
                "https://huggingface.co/{repository}/resolve/main/{asset_name}"
            );
            let Ok(response) = client.get(&url).send().await else {
                continue;
            };
            if !response.status().is_success() {
                continue;
            }
            let partial = destination.with_extension("part");
            let mut file = tokio::fs::File::create(&partial)
                .await
                .map_err(|error| error.to_string())?;
            let mut stream = response.bytes_stream();
            while let Some(item) = stream.next().await {
                let chunk = item.map_err(|error| error.to_string())?;
                file.write_all(&chunk)
                    .await
                    .map_err(|error| error.to_string())?;
            }
            file.flush().await.map_err(|error| error.to_string())?;
            drop(file);
            if destination.exists() {
                let _ = tokio::fs::remove_file(&destination).await;
            }
            tokio::fs::rename(partial, destination)
                .await
                .map_err(|error| error.to_string())?;
            return Ok(());
        }
        Ok(())
    }

    pub fn download_gguf(
        category: &str,
        model_id: &str,
        download_url: &str,
        filename: &str,
        assets: Vec<crate::models::registry::ModelAsset>,
        manifest: Option<ModelManifest>,
        sender: mpsc::Sender<DownloadEvent>,
        abort: Arc<AtomicBool>,
    ) -> Result<PathBuf, String> {
        let runtime = tokio::runtime::Runtime::new().map_err(|error| error.to_string())?;
        runtime.block_on(Self::download_gguf_async(
            category,
            model_id,
            download_url,
            filename,
            assets,
            manifest,
            sender,
            abort,
        ))
    }

    pub fn purge_model(category: &str, model_id: &str) -> Result<(), String> {
        let path = Self::model_dir(category, model_id);
        if !path.exists() {
            return Err(format!("Model directory not found: {}", path.display()));
        }
        std::fs::remove_dir_all(path).map_err(|error| error.to_string())
    }

    pub fn cleanup_partial_download(category: &str, model_id: &str) -> Result<(), String> {
        let path = Self::model_dir(category, model_id);
        if let Ok(entries) = std::fs::read_dir(path) {
            for entry in entries.flatten() {
                let path = entry.path();
                let partial = path
                    .extension()
                    .and_then(|extension| extension.to_str())
                    .map(|extension| extension == "part" || extension == "lock")
                    .unwrap_or(false);
                if partial {
                    let _ = std::fs::remove_file(path);
                }
            }
        }
        Ok(())
    }
}
