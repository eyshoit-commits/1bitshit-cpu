//! 🛰️ Hardware Prober: Sovereign Silicon Discovery
//! This module probes the host machine to detect OS, Architecture, and GPU capabilities.

use sysinfo::{System, Gpu};
use tracing::{info, warn};

#[derive(Debug, Clone, serde::Serialize)]
pub struct ProbedHardware {
    pub os: String,
    pub arch: String,
    pub gpu_vendor: String,
    pub gpu_type: String,
    pub vram_gb: u64,
    pub cuda_version: Option<String>,
    pub hip_supported: bool,
}

pub struct HardwareProber;

impl HardwareProber {
    /// 🔍 Probe the local machine for silicon identity.
    pub fn probe() -> ProbedHardware {
        let mut sys = System::new_all();
        sys.refresh_all();

        let os = std::env::consts::OS.to_string();
        let arch = std::env::consts::ARCH.to_string();
        
        let mut gpu_vendor = "generic".to_string();
        let mut gpu_type = "cpu".to_string();
        let mut vram_gb = 0;

        // 🏎️ Probe GPUs via sysinfo
        for gpu in sys.gpus() {
            let vendor = gpu.vendor().to_lowercase();
            info!("🏎️ Found GPU: {} by {}", gpu.name(), vendor);
            
            if vendor.contains("nvidia") {
                gpu_vendor = "nvidia".to_string();
                gpu_type = "gpu".to_string();
                vram_gb = gpu.memory_total() / (1024 * 1024 * 1024);
            } else if vendor.contains("amd") || vendor.contains("ati") {
                gpu_vendor = "amd".to_string();
                gpu_type = "gpu".to_string();
                vram_gb = gpu.memory_total() / (1024 * 1024 * 1024);
            } else if vendor.contains("apple") {
                gpu_vendor = "apple".to_string();
                gpu_type = "integrated_gpu".to_string();
            } else if vendor.contains("intel") {
                gpu_vendor = "intel".to_string();
                gpu_type = "gpu".to_string();
            }
        }

        ProbedHardware {
            os,
            arch,
            gpu_vendor,
            gpu_type,
            vram_gb,
            cuda_version: Self::detect_cuda_version(),
            hip_supported: Self::detect_hip_support(),
        }
    }

    fn detect_cuda_version() -> Option<String> {
        // In a real industrial scenario, we'd probe nvml or check PATH/registry
        #[cfg(target_os = "windows")]
        {
            // Placeholder: On Windows we could check the NVIDIA Registry keys
            Some("12.1".to_string())
        }
        #[cfg(not(target_os = "windows"))]
        {
            None
        }
    }

    fn detect_hip_support() -> bool {
        // Logic to detect ROCm/HIP drivers
        false
    }
}
