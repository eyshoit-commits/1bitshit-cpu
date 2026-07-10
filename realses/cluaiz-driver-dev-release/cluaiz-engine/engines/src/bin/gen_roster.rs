use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Debug, Serialize, Deserialize)]
pub struct ModelManifest {
    pub id: String,
    pub name: String,
    pub architecture: String,
    pub parameters: String,
    pub training_tokens: String,
    pub bit_depth: String,
    pub ram_required_gb: f64,
    pub tps_estimate: u32,
    pub huggingface_repo: String,
    pub description: String,
    pub is_cloud_api: bool,
}

#[derive(Debug, Serialize)]
pub struct RosterFile {
    pub models: Vec<ModelManifest>,
}

fn generate_repo(name: &str) -> String {
    let lower_name = name.to_lowercase();
    if lower_name.contains("bonsai") || lower_name.contains("cure") {
        let first = name.split_whitespace().next().unwrap().replace("(", "").replace(")", "");
        format!("cluaiz/{}", first.to_lowercase())
    } else if lower_name.contains("qwen") {
        format!("Qwen/{}-GGUF", name.split_whitespace().next().unwrap())
    } else if lower_name.contains("llama") {
        format!("meta-llama/{}-Instruct", name.split_whitespace().next().unwrap().replace("-Instruct", ""))
    } else if lower_name.contains("gemma") {
        format!("google/{}-it", name.split_whitespace().next().unwrap().to_lowercase())
    } else if lower_name.contains("mistral") || lower_name.contains("ministral") {
        format!("mistralai/{}", name.split_whitespace().next().unwrap())
    } else if lower_name.contains("deepseek") {
        format!("deepseek-ai/{}", name.split_whitespace().next().unwrap())
    } else if lower_name.contains("phi") {
        format!("microsoft/{}", name.split_whitespace().next().unwrap())
    } else if lower_name.contains("bitnet") {
        "1bitLLM/bitnet_b1_58-large".to_string()
    } else if lower_name.contains("smollm") {
        format!("HuggingFaceTB/{}", name.split_whitespace().next().unwrap())
    } else if lower_name.contains("granite") {
        format!("ibm/{}-instruct", name.split_whitespace().next().unwrap().to_lowercase())
    } else if lower_name.contains("starcoder") {
        format!("bigcode/{}", name.split_whitespace().next().unwrap().to_lowercase())
    } else {
        format!("local/{}-gguf", name.split_whitespace().next().unwrap().to_lowercase())
    }
}

fn generate_detailed_description(name: &str, params: &str, strength: &str, arch: &str, bit_depth: &str, tokens: &str) -> String {
    format!(
        "This is the highly optimized {}, featuring a robust {} parameter architecture built by leading AI engineers. Designed explicitly to master the trait of '{}', this model pushes the boundaries of modern Core networks. Unlike legacy architectures, it operates perfectly at the {} layer, meaning its memory management is flawlessly optimized for the local Cluaiz CURE engine without compromising zero-shot reasoning. \
        It has been rigorously scaled and pre-trained across a staggering {} corpus, absorbing dense human knowledge spanning logic, mathematics, multi-language translation, and coding syntax. The {} backbone ensures that token processing speeds and semantic latency remain ultra-low. For end-users seeking absolute data privacy and autonomous intelligence at the edge, this specific model guarantees an elite balance between extreme cross-platform speed and deep contextual understanding. Highly recommended for heavy edge execution and next-generation inference loops.",
        name, params, strength, bit_depth, tokens, arch
    )
}

fn main() {
    let txt_path = Path::new(r"c:\Users\Aryan\my\Cluaiz-workspace\bitnetmocle.txt");
    let content = fs::read_to_string(txt_path).expect("Failed to read bitnetmocle.txt");

    let mut models = Vec::new();
    let mut past_header = false;

    for line in content.lines() {
        if line.contains("Model Name") {
            past_header = true;
            continue;
        }
        if !past_header || line.trim().is_empty() {
            continue;
        }

        let parts: Vec<&str> = line.split('\t').collect();
        if parts.len() >= 7 {
            let name = parts[0].trim();
            let params = parts[1].trim();
            let tokens = parts[2].trim();
            let ram_str = parts[3].replace("GB", "").trim().to_string();
            let ram = ram_str.parse::<f64>().unwrap_or(0.0);
            let bit_depth = parts[4].trim();
            let tps_str = parts[5].replace("+", "").replace("-", "0").trim().to_string();
            
            // Just grab numbers
            let tps: u32 = tps_str.chars().filter(|c| c.is_ascii_digit()).collect::<String>().parse().unwrap_or(0);
            let strength = parts[6].trim();

            let is_cloud = name.to_lowercase().contains("grok") || bit_depth.to_lowercase().contains("api");
            let repo = if is_cloud { "api:openai/grok".to_string() } else { generate_repo(name) };

            let arch = if name.contains('-') { name.split('-').next().unwrap() } else { name.split_whitespace().next().unwrap() };
            
            let description = if is_cloud {
                format!("This is a powerful Cloud API bridging ({}) architecture. Serving massive scale without local hardware requirements. Capable of '{}' with near-instant reasoning throughput due to remote data centers. It heavily utilizes API routing protocols guaranteeing extreme context length management and secure parameter bounds handling.", name, strength)
            } else {
                generate_detailed_description(name, params, strength, arch, bit_depth, tokens)
            };

            models.push(ModelManifest {
                id: name.to_lowercase().replace(" ", "-").replace("(", "").replace(")", ""),
                name: name.to_string(),
                architecture: arch.to_string(),
                parameters: params.to_string(),
                training_tokens: tokens.to_string(),
                bit_depth: bit_depth.to_string(),
                ram_required_gb: ram,
                tps_estimate: tps,
                huggingface_repo: repo,
                description,
                is_cloud_api: is_cloud,
            });
        }
    }

    let out = RosterFile { models };
    let json_str = serde_json::to_string_pretty(&out).unwrap();

    let out_path = Path::new(r"c:\Users\Aryan\my\Cluaiz-workspace\Cluaiz-OS\Cluaiz-ai-CURE\engines\src\default_roster.json");
    fs::write(out_path, json_str).expect("Failed to write fast json");
    
    println!("✅ Perfectly generated default_roster.json using 100% Rust!");
}
