use std::path::{Component, Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use futures_util::StreamExt;
use tokio::io::AsyncWriteExt;
use tokio::sync::mpsc;

use crate::models::registry::{ModelAsset, ModelManifest};

#[path = "fetch_v2.rs"]
mod base;

pub use base::DownloadEvent;

pub struct ModelDownloader;

impl ModelDownloader {
    pub fn is_model_cached(category: &str, model_id: &str, filename: &str) -> bool {
        base::ModelDownloader::is_model_cached(category, model_id, filename)
    }

    pub fn get_cached_path(
        category: &str,
        model_id: &str,
        filename: &str,
    ) -> Option<PathBuf> {
        base::ModelDownloader::get_cached_path(category, model_id, filename)
    }

    pub async fn download_gguf_async(
        category: &str,
        model_id: &str,
        download_url: &str,
        filename: &str,
        assets: Vec<ModelAsset>,
        manifest: Option<ModelManifest>,
        sender: mpsc::Sender<DownloadEvent>,
        abort: Arc<AtomicBool>,
    ) -> Result<PathBuf, String> {
        base::ModelDownloader::download_gguf_async(
            category,
            model_id,
            download_url,
            filename,
            assets,
            manifest,
            sender,
            abort,
        )
        .await
    }

    pub fn generate_bitshit_dna(
        manifest: &ModelManifest,
        destination_directory: &Path,
        weight_path: &Path,
    ) -> Result<(), String> {
        base::ModelDownloader::generate_bitshit_dna(
            manifest,
            destination_directory,
            weight_path,
        )
    }

    pub fn generate_cluaiz_dna(
        manifest: &ModelManifest,
        destination_directory: &Path,
        weight_path: &Path,
    ) -> Result<(), String> {
        Self::generate_bitshit_dna(manifest, destination_directory, weight_path)
    }

    /// Repairs an optional tokenizer/config asset without routing progress into
    /// an unread bounded channel. The file is written atomically and unsafe
    /// relative paths are rejected.
    pub async fn fetch_asset_auto_heal(
        repository_id: &str,
        destination_directory: &Path,
        asset_name: &str,
    ) -> Result<(), String> {
        let relative = Self::safe_relative_asset(asset_name)?;
        let destination = destination_directory.join(relative);
        if destination.is_file() {
            return Ok(());
        }
        if let Some(parent) = destination.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(|error| error.to_string())?;
        }

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
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

            let partial = destination.with_extension(format!(
                "{}.part",
                destination
                    .extension()
                    .and_then(|extension| extension.to_str())
                    .unwrap_or("asset")
            ));
            let mut file = tokio::fs::File::create(&partial)
                .await
                .map_err(|error| error.to_string())?;
            let mut stream = response.bytes_stream();
            let mut failed = None;
            while let Some(item) = stream.next().await {
                match item {
                    Ok(chunk) => {
                        if let Err(error) = file.write_all(&chunk).await {
                            failed = Some(error.to_string());
                            break;
                        }
                    }
                    Err(error) => {
                        failed = Some(error.to_string());
                        break;
                    }
                }
            }

            if let Some(error) = failed {
                drop(file);
                let _ = tokio::fs::remove_file(&partial).await;
                tracing::warn!(
                    "[1BitShit Model Store] Optional asset '{}' failed: {}",
                    asset_name,
                    error
                );
                continue;
            }

            file.flush().await.map_err(|error| error.to_string())?;
            drop(file);
            if destination.exists() {
                let _ = tokio::fs::remove_file(&destination).await;
            }
            tokio::fs::rename(partial, &destination)
                .await
                .map_err(|error| error.to_string())?;
            tracing::info!(
                "[1BitShit Model Store] Recovered optional asset '{}' from '{}'",
                asset_name,
                repository
            );
            return Ok(());
        }

        tracing::warn!(
            "[1BitShit Model Store] Optional asset '{}' was not found",
            asset_name
        );
        Ok(())
    }

    pub fn download_gguf(
        category: &str,
        model_id: &str,
        download_url: &str,
        filename: &str,
        assets: Vec<ModelAsset>,
        manifest: Option<ModelManifest>,
        sender: mpsc::Sender<DownloadEvent>,
        abort: Arc<AtomicBool>,
    ) -> Result<PathBuf, String> {
        base::ModelDownloader::download_gguf(
            category,
            model_id,
            download_url,
            filename,
            assets,
            manifest,
            sender,
            abort,
        )
    }

    pub fn purge_model(category: &str, model_id: &str) -> Result<(), String> {
        base::ModelDownloader::purge_model(category, model_id)
    }

    pub fn cleanup_partial_download(category: &str, model_id: &str) -> Result<(), String> {
        base::ModelDownloader::cleanup_partial_download(category, model_id)
    }

    fn safe_relative_asset(name: &str) -> Result<PathBuf, String> {
        let path = Path::new(name);
        if path.is_absolute() {
            return Err(format!("Asset path must be relative: {name}"));
        }
        let mut clean = PathBuf::new();
        for component in path.components() {
            match component {
                Component::Normal(value) => clean.push(value),
                Component::CurDir => {}
                Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                    return Err(format!("Unsafe asset path rejected: {name}"));
                }
            }
        }
        if clean.as_os_str().is_empty() {
            return Err("Asset path is empty".to_string());
        }
        Ok(clean)
    }
}
