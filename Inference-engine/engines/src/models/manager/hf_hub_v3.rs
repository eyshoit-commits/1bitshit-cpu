use std::collections::HashSet;
use std::path::Path;

use reqwest::Client;

use crate::models::registry::{ModelAsset, ModelManifest};

#[path = "hf_hub_v2.rs"]
mod base;

pub use base::HfVariant;

pub struct HuggingFaceHub;

impl HuggingFaceHub {
    pub async fn list_variants(repository_id: &str) -> Result<Vec<HfVariant>, String> {
        let variants = base::HuggingFaceHub::list_variants(repository_id).await?;
        let filtered: Vec<HfVariant> = variants
            .into_iter()
            .filter(|variant| !Self::is_non_primary_gguf_shard(&variant.filename))
            .collect();
        if filtered.is_empty() {
            return Err(format!(
                "No primary GGUF or ONNX variants were found in '{repository_id}'."
            ));
        }
        Ok(filtered)
    }

    pub async fn build_manifest(
        repository_id: &str,
        filename: &str,
        download_size_gb: f64,
    ) -> Result<ModelManifest, String> {
        let mut manifest = base::HuggingFaceHub::build_manifest(
            repository_id,
            filename,
            download_size_gb,
        )
        .await?;

        if manifest.architecture_type.eq_ignore_ascii_case("gguf") {
            let (quantization, bit_depth) = Self::derive_gguf_quantization(filename);
            manifest.id = format!("{}:gguf:{}", manifest.family, quantization);
            manifest.bit_depth = bit_depth;
        } else if manifest.architecture_type.eq_ignore_ascii_case("onnx") {
            Self::append_onnx_runtime_assets(repository_id, filename, &mut manifest).await?;
        }
        Ok(manifest)
    }

    pub async fn fetch_partial_gguf_metadata(
        url: &str,
    ) -> Result<(
        std::collections::HashMap<String, String>,
        std::collections::HashMap<String, Vec<usize>>,
        usize,
    ), String> {
        base::HuggingFaceHub::fetch_partial_gguf_metadata(url).await
    }

    fn derive_gguf_quantization(filename: &str) -> (String, f64) {
        let lower = Path::new(filename)
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or(filename)
            .trim_end_matches(".gguf")
            .to_ascii_lowercase();

        if lower.contains("i2_s") {
            return ("i2_s".to_string(), 1.58);
        }
        if lower.contains("ternary") {
            return ("ternary".to_string(), 1.58);
        }

        for token in lower.split(|character| character == '-' || character == '.') {
            if token.starts_with("tq") {
                return (token.to_string(), 1.58);
            }
            if token == "f16" || token == "fp16" {
                return ("f16".to_string(), 16.0);
            }
            if token == "f32" || token == "fp32" {
                return ("f32".to_string(), 32.0);
            }
            let bytes = token.as_bytes();
            if bytes.len() >= 2 && bytes[0] == b'q' && bytes[1].is_ascii_digit() {
                let bit_depth = (bytes[1] - b'0') as f64;
                return (token.to_string(), bit_depth);
            }
        }

        ("gguf".to_string(), 4.0)
    }

    fn is_non_primary_gguf_shard(filename: &str) -> bool {
        let lower = filename.to_ascii_lowercase();
        if !lower.ends_with(".gguf") || !lower.contains("-of-") {
            return false;
        }

        let Some(of_position) = lower.rfind("-of-") else {
            return false;
        };
        let before = &lower[..of_position];
        let Some(separator) = before.rfind('-') else {
            return false;
        };
        let shard = &before[separator + 1..];
        shard.chars().all(|character| character.is_ascii_digit())
            && shard.parse::<u64>().map(|value| value != 1).unwrap_or(false)
    }

    async fn append_onnx_runtime_assets(
        repository_id: &str,
        selected_filename: &str,
        manifest: &mut ModelManifest,
    ) -> Result<(), String> {
        let url = format!(
            "https://huggingface.co/api/models/{repository_id}/tree/main?recursive=true"
        );
        let response = Client::new()
            .get(url)
            .send()
            .await
            .map_err(|error| error.to_string())?;
        if !response.status().is_success() {
            return Err(format!(
                "Failed to inspect ONNX companion assets for '{repository_id}': HTTP {}",
                response.status()
            ));
        }
        let tree: Vec<serde_json::Value> =
            response.json().await.map_err(|error| error.to_string())?;

        let selected = Path::new(selected_filename);
        let selected_parent = selected.parent().unwrap_or_else(|| Path::new(""));
        let known_names: HashSet<&'static str> = [
            "tokenizer.json",
            "tokenizer_config.json",
            "special_tokens_map.json",
            "config.json",
            "generation_config.json",
            "preprocessor_config.json",
            "processor_config.json",
            "vocab.json",
            "vocab.txt",
            "merges.txt",
            "sentencepiece.bpe.model",
            "spiece.model",
            "added_tokens.json",
            "chat_template.json",
        ]
        .into_iter()
        .collect();

        let mut existing_names: HashSet<String> = manifest
            .assets
            .iter()
            .map(|asset| asset.name.to_ascii_lowercase())
            .collect();

        for item in tree {
            let Some(repository_path) = item.get("path").and_then(|value| value.as_str()) else {
                continue;
            };
            if repository_path == selected_filename {
                continue;
            }
            let size = item.get("size").and_then(|value| value.as_u64()).unwrap_or(0);
            if size > 64 * 1024 * 1024 {
                continue;
            }

            let path = Path::new(repository_path);
            let Some(basename) = path.file_name().and_then(|name| name.to_str()) else {
                continue;
            };
            let basename_lower = basename.to_ascii_lowercase();
            if !known_names.contains(basename_lower.as_str()) {
                continue;
            }

            let parent = path.parent().unwrap_or_else(|| Path::new(""));
            let close_to_model = parent == selected_parent || parent.as_os_str().is_empty();
            if !close_to_model || existing_names.contains(&basename_lower) {
                continue;
            }

            manifest.assets.push(ModelAsset {
                name: basename.to_string(),
                url: format!(
                    "https://huggingface.co/{repository_id}/resolve/main/{repository_path}"
                ),
            });
            existing_names.insert(basename_lower);
        }

        Ok(())
    }
}
