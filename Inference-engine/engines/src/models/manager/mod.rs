use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use crate::models::manager::auditor::{HardwareAuditor, HealthStatus};
use crate::models::registry::ModelManifest;

pub mod auditor;
pub mod client;
#[path = "hf_hub_v3.rs"]
pub mod hf_hub;
pub mod installer;

/// Canonical 1BitShit CPU model manager.
///
/// Every install and repair goes through `ModelDownloader`, so the model hub,
/// CLI, dashboard, cache lookup and deletion all use the same visible store.
pub struct ModelManager {
    _registry_url: String,
    _base_models_dir: PathBuf,
    auditor: HardwareAuditor,
}

impl ModelManager {
    pub fn new(registry_url: String, base_models_dir: PathBuf) -> Self {
        Self {
            _registry_url: registry_url,
            _base_models_dir: base_models_dir,
            auditor: HardwareAuditor,
        }
    }

    pub async fn pull_model(&self, model_id: &str) -> Result<(), String> {
        let roster = crate::models::registry::CoreRoster::load_roster();
        let mut manifest = roster
            .into_iter()
            .find(|model| model.id.eq_ignore_ascii_case(model_id));

        if manifest.is_none() {
            let remote = crate::models::registry::CoreRoster::fetch_external_registry(None).await?;
            manifest = remote
                .into_iter()
                .find(|model| model.id.eq_ignore_ascii_case(model_id));
        }

        let manifest = manifest
            .ok_or_else(|| format!("Model ID '{model_id}' was not found in the registry"))?;
        self.pull_model_with_manifest(&manifest).await
    }

    pub async fn pull_model_with_manifest(&self, manifest: &ModelManifest) -> Result<(), String> {
        if manifest.id.trim().is_empty()
            || manifest.id.to_ascii_lowercase().contains("unknown")
            || manifest.huggingface_filename.trim().is_empty()
            || manifest
                .huggingface_filename
                .eq_ignore_ascii_case("unknown")
        {
            return Err(format!(
                "Refusing incomplete model manifest '{}'",
                manifest.id
            ));
        }

        let health = self.audit_model_health(
            manifest.ram_required_gb as f32,
            manifest.requires_gpu,
        );
        if health == HealthStatus::Disabled {
            return Err(
                "1BitShit hardware audit failed: insufficient resources for this model"
                    .to_string(),
            );
        }

        if let Some(path) = crate::models::fetch::ModelDownloader::get_cached_path(
            &manifest.category,
            &manifest.id,
            &manifest.huggingface_filename,
        ) {
            println!(
                "  ✅ Model '{}' is already available at {}",
                manifest.id,
                path.display()
            );
            return Ok(());
        }

        let (sender, mut receiver) = tokio::sync::mpsc::channel(256);
        let event_drain = tokio::spawn(async move {
            while receiver.recv().await.is_some() {}
        });

        let result = crate::models::fetch::ModelDownloader::download_gguf_async(
            &manifest.category,
            &manifest.id,
            &manifest.download_url,
            &manifest.huggingface_filename,
            manifest.assets.clone(),
            Some(manifest.clone()),
            sender,
            Arc::new(AtomicBool::new(false)),
        )
        .await;

        let _ = event_drain.await;
        let path = result?;
        println!(
            "  ✅ Model '{}' synchronized at {}",
            manifest.id,
            path.display()
        );
        Ok(())
    }

    pub fn audit_model_health(
        &self,
        ram_required: f32,
        requires_gpu: bool,
    ) -> HealthStatus {
        self.auditor
            .audit_performance(ram_required, requires_gpu)
    }
}
