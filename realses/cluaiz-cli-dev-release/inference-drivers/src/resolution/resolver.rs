//! ⚖️ Capability Resolver: Sovereign Backend Selection
//! This module matches probed hardware against the Industrial Registry to select the optimal backend.

use crate::manifest::registry::{SovereignRegistry, Backend};
use crate::resolution::prober::ProbedHardware;
use tracing::{info, warn};

pub struct CapabilityResolver;

impl CapabilityResolver {
    /// ⚖️ Resolve the optimal backend based on hardware identity and registry rules.
    pub fn resolve(hardware: &ProbedHardware, registry: &SovereignRegistry) -> Option<Backend> {
        info!("⚖️ Resolving optimal backend for {} on {}...", hardware.gpu_vendor, hardware.os);

        // 1. Iterate through routing rules in the registry
        for rule in &registry.routing_rules {
            if let Some(ref condition) = rule.condition {
                if Self::matches(hardware, condition) {
                    if let Some(ref backend_id) = rule.use_backend {
                        if let Some(backend) = registry.backends.iter().find(|b| &b.id == backend_id) {
                            info!("✅ [Resolver] Matched Rule! Selected Backend: {}", backend_id);
                            return Some(backend.clone());
                        }
                    }
                }
            }
        }

        // 2. Fallback Chain logic (if no direct rule matches)
        warn!("⚠️ [Resolver] No direct routing rule matched. Entering Fallback Chain.");
        for fallback_id in &registry.global_policies.fallback_order {
            // Find a backend that matches the fallback type (e.g., 'vulkan', 'cpu')
            if let Some(backend) = registry.backends.iter()
                .filter(|b| &b.engine.variant == fallback_id)
                .find(|b| b.platform.os == hardware.os || b.platform.os == "universal") 
            {
                info!("⚡ [Resolver] Fallback Matched: {}", backend.id);
                return Some(backend.clone());
            }
        }

        None
    }

    /// 🛠️ Check if probed hardware matches a specific registry condition.
    fn matches(hardware: &ProbedHardware, condition: &serde_json::Value) -> bool {
        let obj = condition.as_object().unwrap();
        
        // Match Hardware Vendor
        if let Some(vendor) = obj.get("hardware") {
            if vendor.as_str() != Some(&hardware.gpu_vendor) { return false; }
        }

        // Match OS
        if let Some(os) = obj.get("os") {
            if os.as_str() != Some(&hardware.os) { return false; }
        }

        // Match CUDA Version Requirement
        if let Some(cuda_req) = obj.get("cuda_version") {
            if let Some(probed_cuda) = &hardware.cuda_version {
                // Simplified string matching for Phase 2
                if !probed_cuda.contains(cuda_req.as_str().unwrap().trim_start_matches(">=")) {
                    return false;
                }
            } else {
                return false;
            }
        }

        true
    }

    /// 🔗 Resolve the final artifact URL by substituting placeholders.
    pub fn resolve_artifact_url(backend: &Backend, registry: &SovereignRegistry, version: &str) -> String {
        let base_url = &registry.global_policies.artifact_base_url;
        let mut url = backend.artifacts[0].download_url_template.clone();
        
        url = url.replace("{BASE_URL}", base_url);
        url = url.replace("{VERSION}", version);
        url = url.replace("{os}", &std::env::consts::OS);
        url = url.replace("{arch}", &std::env::consts::ARCH);
        
        url
    }
}
