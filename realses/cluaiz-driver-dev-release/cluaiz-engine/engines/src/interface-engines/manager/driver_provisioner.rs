use std::path::{Path, PathBuf};
use anyhow::{Result, anyhow};
use reqwest;
use std::fs;
use cluaiz_shared::HardwareGovernor;

pub struct DriverProvisioner;

impl DriverProvisioner {
    const MANIFEST_URL: &'static str = "https://github.com/cluaiz/cluaiz/releases/download/driver-dev-release/registry-synced.json";

    fn get_backend_id_for_platform(driver_type: &str) -> Option<&'static str> {
        #[cfg(target_os = "windows")]
        {
            match driver_type {
                "cuda" => Some("nvidia-cuda-v12-windows"),
                "vulkan" => Some("universal-vulkan-windows"),
                _ => None,
            }
        }
        #[cfg(target_os = "linux")]
        {
            match driver_type {
                "cuda" => Some("nvidia-cuda-v12-linux"),
                "vulkan" => Some("universal-vulkan-linux"),
                "rocm" => Some("amd-rocm-linux"),
                "hip" => Some("amd-hip-linux"),
                _ => None,
            }
        }
        #[cfg(target_os = "macos")]
        {
            match driver_type {
                "metal" => Some("apple-metal-macos"),
                _ => None,
            }
        }
        #[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
        {
            None
        }
    }

    /// 🛠️ Provision Hardware Driver: Auto-detects, downloads, and deploys missing silicon drivers.
    pub async fn provision_for_hardware(driver_type: &str) -> Result<()> {
        let root = HardwareGovernor::resolve_hub_path();
        let driver_dir = root.join("interface-engines").join("drivers");
        
        if !driver_dir.exists() {
            fs::create_dir_all(&driver_dir)?;
        }

        // 🎯 Step 1: Check if driver already exists
        let marker = driver_dir.join(format!("{}.ready", driver_type));
        if marker.exists() {
            return Ok(());
        }

        println!("🛠️ [PROVISIONER] Missing Hardware Driver detected: {}. Initiating Silicon Handshake...", driver_type);

        let backend_id = Self::get_backend_id_for_platform(driver_type)
            .ok_or_else(|| anyhow!("Driver type '{}' is not supported on this platform.", driver_type))?;

        // 🎯 Step 2: Fetch Synced Driver Registry with dual-layer offline fallback
        let client = reqwest::Client::builder()
            .user_agent("Cluaiz-Neural-Engine/0.1.0")
            .build()?;

        let manifest_str = include_str!("../../../../../inference-drivers/registry.json")
            .replace("{BASE_URL}", "https://github.com/cluaiz/cluaiz/releases/download")
            .replace("{DRIVER_TAG}", "driver-dev-release")
            .replace("{VERSION}", "dev-release");

        let manifest: serde_json::Value = if let Ok(resp) = client.get(Self::MANIFEST_URL).send().await {
            if resp.status().is_success() {
                resp.json().await.unwrap_or_else(|_| {
                    tracing::warn!("⚠️ [PROVISIONER] JSON deserialization failed, loading embedded registry...");
                    serde_json::from_str(&manifest_str).unwrap()
                })
            } else {
                tracing::warn!("⚠️ [PROVISIONER] Cloud Foundry returned {}, loading embedded registry...", resp.status());
                serde_json::from_str(&manifest_str).unwrap()
            }
        } else {
            tracing::warn!("⚠️ [PROVISIONER] Failed to connect to Cloud Foundry, loading embedded registry...");
            serde_json::from_str(&manifest_str).unwrap()
        };

        let backends = manifest["backends"]
            .as_array()
            .ok_or_else(|| anyhow!("Manifest 'backends' field is missing or not an array."))?;

        let backend = backends.iter()
            .find(|b| b["id"].as_str() == Some(backend_id))
            .ok_or_else(|| anyhow!("Backend ID '{}' not found in registry.", backend_id))?;

        let artifact = backend["artifacts"]
            .as_array()
            .and_then(|a| a.first())
            .ok_or_else(|| anyhow!("No artifacts found for backend ID '{}'.", backend_id))?;

        let download_url_template = artifact["download_url_template"]
            .as_str()
            .ok_or_else(|| anyhow!("Artifact download_url_template is missing or not a string."))?;

        let name_template = artifact["name_template"]
            .as_str()
            .ok_or_else(|| anyhow!("Artifact name_template is missing or not a string."))?;

        // 🎯 Step 3: Format and Expand URLs
        let download_url = download_url_template
            .replace("{BASE_URL}", "https://github.com/cluaiz/cluaiz/releases/download")
            .replace("{DRIVER_TAG}", "driver-dev-release")
            .replace("{VERSION}", "dev-release");

        let dest_filename = name_template
            .replace("{VERSION}", "dev-release");

        let dest_path = driver_dir.join(&dest_filename);

        println!("📥 [PROVISIONER] Downloading {} driver from Cloud Foundry...", driver_type);

        // 🎯 Step 4: Download Driver Binary Directly
        let response = client.get(&download_url).send().await?;
        if !response.status().is_success() {
            return Err(anyhow!("Failed to download driver binary: {} from {}", response.status(), download_url));
        }
        
        let bytes = response.bytes().await?;
        fs::write(&dest_path, bytes)?;

        // 🎯 Step 5: Silicon Integrity Check (Ensure we actually got the binary)
        if !dest_path.exists() || fs::metadata(&dest_path)?.len() == 0 {
            return Err(anyhow!("Driver provisioning failed: Loaded file is empty or missing."));
        }

        // 🎯 Step 6: Mark as Ready
        fs::write(marker, "PROVISIONED")?;
        println!("✅ [PROVISIONER] {} Hardware Driver successfully deployed to Bare-Metal.", driver_type);

        Ok(())
    }


    /// 🔍 Silicon Discovery: Scans the system for pre-installed drivers if local ones are missing.
    pub fn discover_system_paths() -> Vec<PathBuf> {
        let mut paths = Vec::new();
        
        // Local drivers folder
        paths.push(Self::get_driver_path());

        // Windows CUDA Discovery (Dynamic via Environment Variables)
        #[cfg(target_os = "windows")]
        {
            // Standard NVIDIA environment variable
            if let Ok(cuda_path) = std::env::var("CUDA_PATH") {
                let bin_path = PathBuf::from(cuda_path).join("bin");
                if bin_path.exists() {
                    paths.push(bin_path);
                }
            }
        }

        paths
    }

    /// Returns the driver directory for appending to search paths
    pub fn get_driver_path() -> PathBuf {
        HardwareGovernor::resolve_hub_path().join("interface-engines").join("drivers")
    }
}
