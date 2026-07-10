use reqwest::Client;
use serde::Deserialize;

use crate::models::registry::ModelManifest;

#[derive(Debug, Deserialize)]
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
    pub async fn list_variants(repo_id: &str) -> Result<Vec<HfVariant>, String> {
        let url = format!("https://huggingface.co/api/models/{repo_id}/tree/main?recursive=true");
        let response = Client::new().get(&url).send().await.map_err(|e| e.to_string())?;
        if !response.status().is_success() {
            return Err(format!("Failed to fetch repository '{repo_id}'. Does it exist?"));
        }

        let items: Vec<HfTreeItem> = response.json().await.map_err(|e| e.to_string())?;
        let mut variants = Vec::new();
        for item in &items {
            let lower = item.path.to_ascii_lowercase();
            if !lower.ends_with(".gguf") {
                continue;
            }
            variants.push(HfVariant {
                filename: item.path.clone(),
                size_gb: item.size.unwrap_or(0) as f64 / 1_073_741_824.0,
            });
        }

        if variants.is_empty() {
            return Err(format!("No GGUF files found in repository '{repo_id}'."));
        }
        variants.sort_by(|a, b| a.filename.cmp(&b.filename));
        Ok(variants)
    }

    pub async fn build_manifest(
        repo_id: &str,
        filename: &str,
        download_size_gb: f64,
    ) -> Result<ModelManifest, String> {
        let file_basename = std::path::Path::new(filename)
            .file_name()
            .and_then(|name| name.to_str())
            .filter(|name| !name.trim().is_empty())
            .ok_or_else(|| format!("Invalid HuggingFace filename: {filename}"))?
            .to_string();

        let repo_slug = repo_id
            .split('/')
            .next_back()
            .unwrap_or(repo_id)
            .trim()
            .to_ascii_lowercase();
        if repo_slug.is_empty() {
            return Err("Invalid HuggingFace repository id".to_string());
        }

        let stem = file_basename
            .trim_end_matches(".gguf")
            .trim_end_matches(".GGUF")
            .to_ascii_lowercase();

        let quant = stem
            .split(|ch: char| ch == '-' || ch == '_' || ch == '.')
            .rev()
            .find(|part| {
                let p = part.to_ascii_lowercase();
                p.starts_with('q')
                    || p.contains("i2_s")
                    || p.contains("tq")
                    || p.contains("ternary")
                    || p == "f16"
                    || p == "f32"
            })
            .unwrap_or("gguf")
            .to_ascii_lowercase();

        let stable_id = format!("{repo_slug}:gguf:{quant}");
        let download_url = format!("https://huggingface.co/{repo_id}/resolve/main/{filename}");

        let mut architecture = "gguf".to_string();
        let mut parameters = String::new();
        let mut context_window = "8k".to_string();
        let mut bit_depth = if quant.contains("i2_s") || quant.contains("ternary") || quant.contains("tq") {
            1.58
        } else {
            4.0
        };

        if let Ok((metadata, _, _)) = Self::fetch_partial_gguf_metadata(&download_url).await {
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
            if let Some(value) = metadata.get("general.file_type") {
                let lower = value.to_ascii_lowercase();
                if lower.contains("i2") || lower.contains("ternary") || lower.contains("tq") {
                    bit_depth = 1.58;
                }
            }
        }

        Ok(ModelManifest {
            id: stable_id,
            name: repo_slug.replace('-', " "),
            architecture,
            architecture_type: "gguf".to_string(),
            parameters,
            training_tokens: String::new(),
            bit_depth,
            ram_required_gb: download_size_gb + 0.5,
            download_size_gb,
            huggingface_repo: repo_id.to_string(),
            huggingface_filename: file_basename,
            download_url,
            description: format!("HuggingFace GGUF model from {repo_id}"),
            is_cloud_api: false,
            requires_gpu: false,
            is_free_tier: true,
            input_modality: "Text".to_string(),
            context_window,
            family: repo_slug,
            category: "chat".to_string(),
            assets: Vec::new(),
            local_path: None,
            dna_path: None,
            has_vision: false,
            has_audio: false,
            expert_count: None,
            experts_per_token: None,
        })
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
            .map_err(|e| e.to_string())?;
        let mut target_url = url.to_string();

        for _ in 0..5 {
            let response = client
                .get(&target_url)
                .header(RANGE, "bytes=0-8388607")
                .send()
                .await
                .map_err(|e| e.to_string())?;

            if response.status().is_redirection() {
                if let Some(location) = response.headers().get(reqwest::header::LOCATION) {
                    target_url = location.to_str().map_err(|e| e.to_string())?.to_string();
                    continue;
                }
            }

            if !response.status().is_success()
                && response.status() != reqwest::StatusCode::PARTIAL_CONTENT
            {
                return Err(format!("Failed to fetch GGUF header: HTTP {}", response.status()));
            }

            let bytes = response.bytes().await.map_err(|e| e.to_string())?;
            let temp_path = std::env::temp_dir().join(format!(
                "1bitshit_probe_{}_{}.gguf",
                std::process::id(),
                std::thread::current().name().unwrap_or("worker")
            ));
            let mut file = std::fs::File::create(&temp_path).map_err(|e| e.to_string())?;
            file.write_all(&bytes).map_err(|e| e.to_string())?;
            let result = cluaiz_shared::utils::gguf_prober::GGUFProber::probe(&temp_path);
            let _ = std::fs::remove_file(&temp_path);
            return result.map_err(|e| e.to_string());
        }

        Err("Too many redirects while fetching GGUF metadata".to_string())
    }
}
