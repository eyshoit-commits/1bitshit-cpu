use crate::hardware::schema::booster::BoosterControl;
use crate::hardware::schema::profiles::SystemControl;
use crate::hardware::system_control::HardwareOrchestrator;
use once_cell::sync::Lazy;
use rkyv::Deserialize;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;

/// 🧠 VRAM Arbiter State: Tracks real-time resource allocations.
pub struct ArbiterState {
    pub total_vram_gb: f64,
    pub allocated_vram_gb: f64,
    pub active_allocations: HashMap<String, f64>,
}

static ARBITER: Lazy<Mutex<ArbiterState>> = Lazy::new(|| {
    Mutex::new(ArbiterState {
        total_vram_gb: 0.0,
        allocated_vram_gb: 0.0,
        active_allocations: HashMap::new(),
    })
});

#[derive(Clone, Copy, Default)]
pub struct HardwareGovernor;

impl HardwareGovernor {
    /// 🚀 Initialize the Governor and resolve hardware state.
    pub fn start() -> Self {
        Self
    }

    /// 🛡️ Checks if the 'system_control.json' fingerprint exists.
    pub fn is_ready(&self) -> bool {
        Self::resolve_engine_path()
            .join("system_control.json")
            .exists()
    }

    /// 🔬 Deep surgical scan and persistence of silicon state.
    pub fn auto_calibrate() -> anyhow::Result<()> {
        let control = HardwareOrchestrator::start()?;
        Self::save_booster_settings(&Self::load_booster_settings().unwrap_or_default())?;

        // 🧠 Mission 12: Chronicle Foundry State
        let _ = crate::neural::graph::NeuralGraph::chronicle_pulse(
            "Foundry Calibration & Silicon Audit",
            "HardwareGovernor",
            &format!("Silicon: {}, Arch: {}", control.silicon_truth.cpu.brand.trim(), control.identity.architecture)
        );

        // Update Arbiter with latest hardware truth
        if let Ok(mut arbiter) = ARBITER.lock() {
            let total = control
                .silicon_truth
                .accelerators
                .gpus
                .iter()
                .map(|g| g.vram_available_gb)
                .sum::<f64>();
            arbiter.total_vram_gb = total;
        }

        Ok(())
    }

    /// ⚖️ Request VRAM allocation for a neural engine.
    /// Prevents OOM by enforcing the sovereign memory budget.
    pub fn request_vram(engine_id: &str, required_gb: f64) -> anyhow::Result<()> {
        let mut arbiter = ARBITER
            .lock()
            .map_err(|_| anyhow::anyhow!("Arbiter Lock Poisoned"))?;

        // If total_vram is 0, we try to load from the existing System Truth first (Fast)
        if arbiter.total_vram_gb == 0.0 {
            if let Ok(control) = Self::load_system_control() {
                let total = control.silicon_truth.accelerators.gpus.iter()
                    .map(|g| g.vram_available_gb)
                    .sum::<f64>();
                arbiter.total_vram_gb = total;
                tracing::info!("⚖️ [Arbiter] VRAM Truth synchronized from System Control: {:.2}GB", total);
            } else {
                // Only calibrate if absolutely no truth is found (Slow fallback)
                let _ = Self::auto_calibrate();
            }
        }

        let available = arbiter.total_vram_gb - arbiter.allocated_vram_gb;

        if required_gb > available {
            return Err(anyhow::anyhow!(
                "❌ [VRAM Arbiter] Out of Memory! Requested: {:.2}GB, Available: {:.2}GB (Total: {:.2}GB)",
                required_gb, available, arbiter.total_vram_gb
            ));
        }

        // Allocate
        arbiter.allocated_vram_gb += required_gb;
        arbiter
            .active_allocations
            .insert(engine_id.to_string(), required_gb);

        println!(
            "✅ [VRAM Arbiter] Allocated {:.2}GB to '{}'. Current Load: {:.2}/{:.2}GB",
            required_gb, engine_id, arbiter.allocated_vram_gb, arbiter.total_vram_gb
        );

        Ok(())
    }

    /// 🔓 Release VRAM allocation when an engine is unloaded.
    pub fn release_vram(engine_id: &str) -> anyhow::Result<()> {
        let mut arbiter = ARBITER
            .lock()
            .map_err(|_| anyhow::anyhow!("Arbiter Lock Poisoned"))?;

        if let Some(freed_gb) = arbiter.active_allocations.remove(engine_id) {
            arbiter.allocated_vram_gb -= freed_gb;
            println!(
                "🔓 [VRAM Arbiter] Released {:.2}GB from '{}'. Current Load: {:.2}/{:.2}GB",
                freed_gb, engine_id, arbiter.allocated_vram_gb, arbiter.total_vram_gb
            );
        }

        Ok(())
    }

    /// ⚙️ Updates a specific field in the sovereign configuration.
    pub fn update_field(field: &str, value: serde_json::Value) -> anyhow::Result<()> {
        let mut control = HardwareOrchestrator::start()?;

        // ⚙️ Sovereign Configuration Dispatch
        match field {
            "machine_name" => {
                if let Some(s) = value.as_str() {
                    control.identity.machine_name = s.to_string();
                }
            }
            "runtime_engine.booster_flags.TurboQuant_Enable" => {
                let mut booster = Self::load_booster_settings().unwrap_or_default();
                if let Some(b) = value.as_bool() {
                    booster.turbo_quant = if b {
                        crate::hardware::schema::booster::FeatureState::On
                    } else {
                        crate::hardware::schema::booster::FeatureState::Off
                    };
                    Self::save_booster_settings(&booster)?;
                }
            }
            "runtime_engine.booster_flags.FlashAttention_v2" => {
                let mut booster = Self::load_booster_settings().unwrap_or_default();
                if let Some(b) = value.as_bool() {
                    booster.flash_attention = if b {
                        crate::hardware::schema::booster::FeatureState::On
                    } else {
                        crate::hardware::schema::booster::FeatureState::Off
                    };
                    Self::save_booster_settings(&booster)?;
                }
            }
            _ => println!("⚠️ [Governor] Field update NOT implemented: {}", field),
        }

        // Save back the updated control
        let base = Self::resolve_engine_path();
        let json_data = serde_json::to_string_pretty(&control)?;
        std::fs::write(base.join("system_control.json"), json_data)?;

        Ok(())
    }

    /// Resolves the base Hub directory for Cluaiz configurations.
    /// Priority:
    /// 1. CLUAIZ_ROOT environment variable.
    /// 2. Portable Mode: Parent directory of current executable.
    /// 3. OS Standard Config Dir.
    pub fn resolve_hub_path() -> PathBuf {
        if let Ok(root) = std::env::var("CLUAIZ_ROOT") {
            return PathBuf::from(root);
        }

        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".cluaiz")
    }

    pub fn resolve_apps_path() -> PathBuf {
        let path = Self::resolve_hub_path().join("apps");
        let _ = std::fs::create_dir_all(&path);
        path
    }

    pub fn resolve_app_path(name: &str) -> PathBuf {
        let path = Self::resolve_apps_path().join(name);
        let _ = std::fs::create_dir_all(&path);
        path
    }

    pub fn resolve_engine_path() -> PathBuf {
        let path = Self::resolve_hub_path().join("engine");
        let _ = std::fs::create_dir_all(&path);
        path
    }

    pub fn resolve_interface_path() -> PathBuf {
        let path = Self::resolve_hub_path().join("interface-engines");
        let _ = std::fs::create_dir_all(&path);
        path
    }

    pub fn resolve_booster_path() -> PathBuf {
        let path = Self::resolve_engine_path().join("booster");
        let _ = std::fs::create_dir_all(&path);
        path
    }

    pub fn resolve_vault_path() -> PathBuf {
        let path = Self::resolve_hub_path().join("vault");
        let _ = std::fs::create_dir_all(&path);
        path
    }

    pub fn resolve_skills_path() -> PathBuf {
        let path = Self::resolve_hub_path().join("skills");
        let _ = std::fs::create_dir_all(&path);
        path
    }

    pub fn resolve_bin_gateway() -> PathBuf {
        let path = Self::resolve_hub_path().join("bin");
        let _ = std::fs::create_dir_all(&path);
        path
    }

    // ─── 🚀 SYSTEM CONTROL (BINARY TRUTH) ───

    /// 🏛️ Loads the sovereign hardware fingerprint from the binary truth (.bin).
    /// If missing, it triggers an automatic "Self-Healing" recovery scan.
    pub fn load_binary_truth() -> anyhow::Result<SystemControl> {
        let path = Self::resolve_engine_path().join("system_control.bin");
        
        if !path.exists() {
            println!("🛠️ [Self-Healing] Kernel Binary Missing. Regenerating...");
            Self::auto_calibrate()?;
        }

        let bytes = std::fs::read(&path)?;
        let archived = unsafe { rkyv::archived_root::<SystemControl>(&bytes) };
        let control: SystemControl = match archived.deserialize(&mut rkyv::Infallible) {
            Ok(val) => val,
            Err(_) => {
                println!("⚠️ [Self-Healing] Binary Corrupted. Regenerating...");
                Self::auto_calibrate()?;
                return Self::load_binary_truth();
            }
        };
        Ok(control)
    }

    pub fn load_system_control() -> anyhow::Result<SystemControl> {
        let base = Self::resolve_engine_path();
        let path = base.join("system_control.json");
        let bin_path = base.join("system_control.bin");
        
        if !path.exists() {
            if !bin_path.exists() {
                println!("🛠️ [Self-Healing] System Truth LOST. Initiating Full Recovery...");
                Self::auto_calibrate()?;
            } else {
                return Self::load_binary_truth();
            }
        }

        let data =
            std::fs::read_to_string(&path).map_err(|_| anyhow::anyhow!("JSON Load Failed"))?;
        let control: SystemControl = match serde_json::from_str(&data) {
            Ok(val) => val,
            Err(_) => {
                println!("⚠️ [Self-Healing] JSON Tampered. Restoring from Binary...");
                Self::load_binary_truth().unwrap_or_default()
            }
        };
        Ok(control)
    }

    pub fn save_system_control(control: &SystemControl) -> anyhow::Result<()> {
        let base = Self::resolve_engine_path();
        std::fs::create_dir_all(&base)?;

        let json_path = base.join("system_control.json");
        let bin_path = base.join("system_control.bin");

        let json_data = serde_json::to_string_pretty(control)?;
        std::fs::write(json_path, json_data)?;

        let bytes = rkyv::to_bytes::<_, 1024>(control)
            .map_err(|e| anyhow::anyhow!("Binary Serialization Failed: {}", e))?;
        std::fs::write(bin_path, bytes.as_slice())?;

        Ok(())
    }

    // ─── 🚀 BOOSTER CONTROL (USER SETTINGS) ───

    pub fn load_booster_settings() -> anyhow::Result<BoosterControl> {
        let path = Self::resolve_engine_path().join("system_booster.json");
        if !path.exists() {
            return Ok(BoosterControl::default());
        }
        let data = std::fs::read_to_string(path)?;
        let control: BoosterControl = serde_json::from_str(&data)?;
        Ok(control)
    }

    pub fn save_booster_settings(control: &BoosterControl) -> anyhow::Result<()> {
        let base = Self::resolve_engine_path();
        std::fs::create_dir_all(&base)?;
        
        let json_path = base.join("system_booster.json");
        let bin_path = base.join("system_booster.bin");

        // 🔓 Step 1: Unlock for Update
        Self::set_file_lock(&json_path, false);
        Self::set_file_lock(&bin_path, false);

        // ✍️ Step 2: Write Booster Settings
        let json_data = serde_json::to_string_pretty(control)?;
        std::fs::write(&json_path, json_data)?;

        let bytes = rkyv::to_bytes::<_, 1024>(control)
            .map_err(|e| anyhow::anyhow!("Binary Serialization Failed: {}", e))?;
        std::fs::write(&bin_path, bytes.as_slice())?;

        // 🔒 Step 3: Sovereign Lockdown
        Self::set_file_lock(&json_path, true);
        Self::set_file_lock(&bin_path, true);

        Ok(())
    }

    /// 🔒 Applies OS-level protection to a file to prevent manual deletion or tampering.
    fn set_file_lock(path: &std::path::Path, locked: bool) {
        if let Ok(metadata) = std::fs::metadata(path) {
            let mut permissions = metadata.permissions();
            permissions.set_readonly(locked);
            let _ = std::fs::set_permissions(path, permissions);
        }
    }
}
