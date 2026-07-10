use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SovereignRegistry {
    pub registry_version: String,
    pub registry_name: String,
    pub version: String,
    pub generated_at: String,
    pub distribution_channel: String,
    pub global_policies: GlobalPolicies,
    pub capabilities_matrix: HashMap<String, String>,
    pub model_architectures: Vec<String>,
    pub runtime_matrix: HashMap<String, RuntimePlatform>,
    pub backends: Vec<Backend>,
    pub compatibility: CompatibilityInfo,
    pub routing_rules: Vec<RoutingRule>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalPolicies {
    pub verify_sha256: bool,
    pub verify_signature: bool,
    pub max_parallel_downloads: u32,
    pub fallback_order: Vec<String>,
    pub artifact_base_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimePlatform {
    pub extensions: RuntimeExtensions,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeExtensions {
    pub library: String,
    pub executable: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Backend {
    pub id: String,
    pub priority: u32,
    pub hardware: HardwareInfo,
    pub platform: PlatformInfo,
    pub requirements: HashMap<String, serde_json::Value>,
    pub engine: EngineInfo,
    pub artifacts: Vec<Artifact>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HardwareInfo {
    pub vendor: String,
    #[serde(rename = "type")]
    pub hw_type: String,
    pub supported_architectures: Option<Vec<String>>,
    pub supported_chipsets: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformInfo {
    pub os: serde_json::Value,
    pub arch: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EngineInfo {
    pub name: String,
    pub variant: String,
    pub capabilities: HashMap<String, bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Artifact {
    pub name_template: Option<String>,
    pub download_url_template: Option<String>,
    pub os: Option<String>,
    pub arch: Option<String>,
    pub name: Option<String>,
    pub url: Option<String>,
    pub sha256: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompatibilityInfo {
    pub gguf_versions: Vec<String>,
    pub supported_quantizations: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingRule {
    #[serde(rename = "if")]
    pub condition: Option<HashMap<String, serde_json::Value>>,
    pub fallback_to: Option<String>,
    pub use_backend: Option<String>,
}

impl SovereignRegistry {
    pub fn from_json(json: &str) -> anyhow::Result<Self> {
        Ok(serde_json::from_str(json)?)
    }
}
