use reqwest::Client;
use serde::Deserialize;

use crate::models::registry::{ModelAsset, ModelManifest};

#[derive(Debug, Clone, Deserialize)]
struct HfTreeItem {
    path: String,
    size: Option<u64>,
}

#[derive(Debug, Clone)]
pub struct HfVariant {
    pub filename: String,
    pub size_gb: f64,
}

pub struct HuggingFaceHub;

impl HuggingFaceHub {
    async fn list_tree(repository_id: &str) -> Result<Vec<HfTreeItem>, String> {
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
                "Failed to fetch Hugging Face repository '{repository_id}': HTTP {}",
                response.status()
            ));
        }
        response.json().await.map_err(|error| error.to_string())
    }

    pub async fn list_variants(repository_id: &str) -> Result<Vec<HfVariant>, String> {
        let tree = Self::list_tree(repository_id).await?;
        let mut variants = Vec::new();

        for item in &tree {
            let lower = item.path.to_ascii_lowercase();
            if !lower.ends_with(".gguf") && !lower.ends_with(".onnx") {
                continue;
            }

            let mut total_size = item.size.unwrap_or(0);
            if lower.ends_with(".onnx") {
                for asset in Self::related_onnx_assets(&tree, &item.path) {
                    total_size += asset.size.unwrap_or(0);
                }
            } else if Self::is_split_gguf(&item.path) {
                for asset in Self::related_gguf_shards(&tree, &item.path) {
                    total_size += asset.size.unwrap_or(0);
                }
            }

            variants.push(HfVariant {
                filename: item.path.clone(),
                size_gb: total_size as f64 / 1_073_741_824.0,
            });
        }

        if variants.is_empty() {
            return Err(format!(
                "No GGUF or ONNX model files were found in '{repository_id}'."
            ));
        }
        variants.sort_by(|left, right| left.filename.cmp(&right.filename));
        Ok(variants)
    }

    pub async fn build_manifest(
        repository_id: &str,
        filename: &str,
        download_size_gb: f64,
    ) -> Result<ModelManifest, String> {
        let selected_path = std::path::Path::new(filename);
        let basename = selected_path
            .file_name()
            .and_then(|name| name.to_str())
            .filter(|name| !name.trim().is_empty())
            .ok_or_else(|| format!("Invalid Hugging Face filename: {filename}"))?
            .to_string();
        let repository_slug = repository_id
            .split('/')
            .next_back()
            .unwrap_or(repository_id)
            .trim()
            .to_ascii_lowercase();
        if repository_slug.is_empty() {
            return Err("Invalid Hugging Face repository ID".to_string());
        }

        let extension = selected_path
            .extension()
            .and_then(|extension| extension.to_str())
            .unwrap_or_default()
            .to_ascii_lowercase();
        if extension != "gguf" && extension != "onnx" {
            return Err(format!("Unsupported model format: {extension}"));
        }

        let stem = basename
            .trim_end_matches(".gguf")
            .trim_end_matches(".GGUF")
            .trim_end_matches(".onnx")
            .trim_end_matches(".ONNX")
            .to_ascii_lowercase();
        let (quantization, bit_depth) = if extension == "onnx" {
            Self::onnx_precision(&stem)
        } else {
            Self::gguf_quantization(&stem)
        };

        let stable_id = format!(
            "{}:{}:{}",
            repository_slug,
            extension,
            quantization
        );
        let download_url = format!(
            "https://huggingface.co/{repository_id}/resolve/main/{filename}"
        );

        let mut architecture = if extension == "onnx" {
            "ONNX".to_string()
        } else {
            "GGUF".to_string()
        };
        let mut parameters = String::new();
        let mut context_window = "8k".to_string();

        if extension == "gguf" {
            if let Ok((metadata, _, _)) =
                Self::fetch_partial_gguf_metadata(&download_url).await
            {
                if let Some(value) = metadata.get("general.architecture") {
                    architecture = value.clone();
                }
                if let Some(value) = metadata.get("general.parameter_count") {
                    parameters = value.clone();
                }
                if let Some(value) = metadata
                    .get(&format!("{}.context_length", architecture))
                    .or_else(|| metadata.get("llama.context_length"))
                    .or_else(|| metadata.get("qwen2.context_length"))
                {
                    if let Ok(raw) = value.parse::<u64>() {
                        context_window = if raw >= 1024 {
                            format!("{}k", raw / 1024)
                        } else {
                            raw.to_string()
                        };
                    }
                }
            }
        }

        let metadata = Self::repository_metadata(repository_id).await;
        let pipeline_tag = metadata
            .as_ref()
            .and_then(|value| value.get("pipeline_tag"))
            .and_then(|value| value.as_str())
            .unwrap_or_default()
            .to_ascii_lowercase();
        let tags: Vec<String> = metadata
            .as_ref()
            .and_then(|value| value.get("tags"))
            .and_then(|value| value.as_array())
            .map(|values| {
                values
                    .iter()
                    .filter_map(|value| value.as_str())
                    .map(str::to_ascii_lowercase)
                    .collect()
            })
            .unwrap_or_default();

        let (category, has_vision, has_audio, input_modality) =
            Self::categorize(&extension, &pipeline_tag, &tags);
        let tree = Self::list_tree(repository_id).await.unwrap_or_default();
        let related = if extension == "onnx" {
            Self::related_onnx_assets(&tree, filename)
        } else if Self::is_split_gguf(filename) {
            Self::related_gguf_shards(&tree, filename)
        } else {
            Vec::new()
        };
        let selected_parent = selected_path.parent().unwrap_or_else(|| std::path::Path::new(""));
        let assets = related
            .into_iter()
            .map(|item| {
                let item_path = std::path::Path::new(&item.path);
                let relative = item_path
                    .strip_prefix(selected_parent)
                    .unwrap_or(item_path)
                    .to_string_lossy()
                    .into_owned();
                ModelAsset {
                    name: relative,
                    url: format!(
                        "https://huggingface.co/{repository_id}/resolve/main/{}",
                        item.path
                    ),
                }
            })
            .collect();

        Ok(ModelManifest {
            id: stable_id,
            name: repository_slug.replace('-', " "),
            architecture,
            architecture_type: extension.clone(),
            parameters,
            training_tokens: String::new(),
            bit_depth,
            ram_required_gb: download_size_gb + if extension == "onnx" { 0.35 } else { 0.5 },
            download_size_gb,
            huggingface_repo: repository_id.to_string(),
            huggingface_filename: basename,
            download_url,
            description: format!(
                "Hugging Face {} model from {}",
                extension.to_ascii_uppercase(),
                repository_id
            ),
            is_cloud_api: false,
            requires_gpu: false,
            is_free_tier: true,
            input_modality,
            context_window,
            family: repository_slug,
            category,
            assets,
            local_path: None,
            dna_path: None,
            has_vision,
            has_audio,
            expert_count: None,
            experts_per_token: None,
        })
    }

    fn gguf_quantization(stem: &str) -> (String, f64) {
        let quantization = stem
            .split(|character: char| character == '-' || character == '_' || character == '.')
            .rev()
            .find(|part| {
                let part = part.to_ascii_lowercase();
                part.starts_with('q')
                    || part.contains("i2_s")
                    || part.contains("tq")
                    || part.contains("ternary")
                    || part == "f16"
                    || part == "f32"
            })
            .unwrap_or("gguf")
            .to_ascii_lowercase();
        let bit_depth = if quantization.contains("i2_s")
            || quantization.contains("ternary")
            || quantization.contains("tq")
        {
            1.58
        } else if quantization.contains("q2") {
            2.0
        } else if quantization.contains("q3") {
            3.0
        } else if quantization.contains("q8") {
            8.0
        } else if quantization == "f16" {
            16.0
        } else if quantization == "f32" {
            32.0
        } else {
            4.0
        };
        (quantization, bit_depth)
    }

    fn onnx_precision(stem: &str) -> (String, f64) {
        if stem.contains("int8") || stem.contains("q8") {
            ("int8".to_string(), 8.0)
        } else if stem.contains("int4") || stem.contains("q4") {
            ("int4".to_string(), 4.0)
        } else if stem.contains("fp16") || stem.contains("f16") {
            ("fp16".to_string(), 16.0)
        } else {
            ("fp32".to_string(), 32.0)
        }
    }

    fn categorize(
        extension: &str,
        pipeline_tag: &str,
        tags: &[String],
    ) -> (String, bool, bool, String) {
        if extension == "gguf" {
            return ("chat".to_string(), false, false, "Text".to_string());
        }
        let joined_tags = tags.join(" ");
        if matches!(
            pipeline_tag,
            "feature-extraction" | "sentence-similarity" | "zero-shot-classification"
        ) || joined_tags.contains("embedding")
            || joined_tags.contains("sentence-transformers")
        {
            ("embedding".to_string(), false, false, "Text".to_string())
        } else if pipeline_tag.contains("image")
            || pipeline_tag.contains("vision")
            || joined_tags.contains("vision")
            || joined_tags.contains("clip")
        {
            ("vision".to_string(), true, false, "Image + Text".to_string())
        } else if pipeline_tag.contains("audio")
            || pipeline_tag.contains("speech")
            || joined_tags.contains("audio")
            || joined_tags.contains("whisper")
        {
            ("audio".to_string(), false, true, "Audio + Text".to_string())
        } else {
            ("chat".to_string(), false, false, "Text".to_string())
        }
    }

    async fn repository_metadata(repository_id: &str) -> Option<serde_json::Value> {
        Client::new()
            .get(format!("https://huggingface.co/api/models/{repository_id}"))
            .send()
            .await
            .ok()?
            .json()
            .await
            .ok()
    }

    fn related_onnx_assets<'a>(
        tree: &'a [HfTreeItem],
        filename: &str,
    ) -> Vec<&'a HfTreeItem> {
        let selected = std::path::Path::new(filename);
        let parent = selected.parent().unwrap_or_else(|| std::path::Path::new(""));
        let basename = selected
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or(filename);
        tree.iter()
            .filter(|item| item.path != filename)
            .filter(|item| {
                let item_path = std::path::Path::new(&item.path);
                if item_path.parent().unwrap_or_else(|| std::path::Path::new("")) != parent
                    && !item_path.starts_with(parent.join(format!("{basename}_data")))
                    && !item_path.starts_with(parent.join(format!("{basename}.data")))
                {
                    return false;
                }
                let relative = item_path
                    .strip_prefix(parent)
                    .unwrap_or(item_path)
                    .to_string_lossy();
                relative == format!("{basename}_data")
                    || relative.starts_with(&format!("{basename}_data/"))
                    || relative == format!("{basename}.data")
                    || relative.starts_with(&format!("{basename}.data/"))
                    || relative == format!("{basename}_data.bin")
            })
            .collect()
    }

    fn is_split_gguf(filename: &str) -> bool {
        let lower = filename.to_ascii_lowercase();
        lower.ends_with(".gguf") && lower.contains("-of-")
    }

    fn related_gguf_shards<'a>(
        tree: &'a [HfTreeItem],
        filename: &str,
    ) -> Vec<&'a HfTreeItem> {
        let selected = std::path::Path::new(filename);
        let parent = selected.parent().unwrap_or_else(|| std::path::Path::new(""));
        let basename = selected
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or(filename);
        let prefix = basename
            .split("-000")
            .next()
            .unwrap_or(basename)
            .to_ascii_lowercase();
        tree.iter()
            .filter(|item| item.path != filename)
            .filter(|item| {
                let path = std::path::Path::new(&item.path);
                path.parent().unwrap_or_else(|| std::path::Path::new("")) == parent
                    && path
                        .file_name()
                        .and_then(|name| name.to_str())
                        .map(|name| {
                            let lower = name.to_ascii_lowercase();
                            lower.starts_with(&prefix)
                                && lower.contains("-of-")
                                && lower.ends_with(".gguf")
                        })
                        .unwrap_or(false)
            })
            .collect()
    }

    pub async fn fetch_partial_gguf_metadata(
        url: &str,
    ) -> Result<(
        std::collections::HashMap<String, String>,
        std::collections::HashMap<String, Vec<usize>>,
        usize,
    ), String> {
        use reqwest::header::RANGE;
        use std::io::Write;

        let client = Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .map_err(|error| error.to_string())?;
        let mut target_url = url.to_string();

        for _ in 0..5 {
            let response = client
                .get(&target_url)
                .header(RANGE, "bytes=0-8388607")
                .send()
                .await
                .map_err(|error| error.to_string())?;
            if response.status().is_redirection() {
                if let Some(location) = response.headers().get(reqwest::header::LOCATION) {
                    target_url = location
                        .to_str()
                        .map_err(|error| error.to_string())?
                        .to_string();
                    continue;
                }
            }
            if !response.status().is_success()
                && response.status() != reqwest::StatusCode::PARTIAL_CONTENT
            {
                return Err(format!(
                    "Failed to fetch GGUF header: HTTP {}",
                    response.status()
                ));
            }

            let bytes = response.bytes().await.map_err(|error| error.to_string())?;
            let temporary = std::env::temp_dir().join(format!(
                "1bitshit_probe_{}_{}.gguf",
                std::process::id(),
                std::thread::current().name().unwrap_or("worker")
            ));
            let mut file = std::fs::File::create(&temporary)
                .map_err(|error| error.to_string())?;
            file.write_all(&bytes)
                .map_err(|error| error.to_string())?;
            let result = cluaiz_shared::utils::gguf_prober::GGUFProber::probe(&temporary);
            let _ = std::fs::remove_file(temporary);
            return result.map_err(|error| error.to_string());
        }
        Err("Too many redirects while fetching GGUF metadata".to_string())
    }
}
