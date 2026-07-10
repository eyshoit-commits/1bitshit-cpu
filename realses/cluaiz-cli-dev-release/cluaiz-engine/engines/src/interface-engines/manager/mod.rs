use crate::interface_engines::manager::driver_bridge::DriverBridge;
use crate::interface_engines::manager::kernel_loader::KernelLoader;
use cluaiz_shared::hardware::governor::HardwareGovernor;
use cluaiz_shared::hardware::schema::profiles::SystemControl;
use colored::Colorize;
use libloading::{Library, Symbol};
use std::path::PathBuf;

pub mod driver_bridge;
pub mod driver_provisioner;
pub mod kernel_loader;
pub mod npu_bridge;

use driver_provisioner::DriverProvisioner;

/// Cluaiz Engine Manager
/// Orchestrates pre-compiled Kernels (BitNet, Llama, Candle) and Hardware Drivers.
pub struct EngineManager {
    kernel_dir: PathBuf,
    loader: KernelLoader,
    bridge: DriverBridge,
    // 🏛️ The Soul Link: Holds the active binary in process memory
    active_lib: Option<Library>,
}

impl EngineManager {
    pub fn new(kernel_dir: PathBuf) -> Self {
        Self {
            kernel_dir: kernel_dir.clone(),
            loader: KernelLoader::new(kernel_dir),
            bridge: DriverBridge::new(),
            active_lib: None,
        }
    }

    /// Handshake: Identify the target Hardware and ensure correct kernel/driver presence.
    pub async fn prepare_engine(&self, engine_type: &str) -> Result<PathBuf, String> {
        let config_path = self.get_system_control_path();
        let content = std::fs::read_to_string(&config_path)
            .map_err(|e| format!("Hardware Config Missing: {}", e))?;

        let control: SystemControl = serde_json::from_str(&content)
            .map_err(|e| format!("Hardware Config Parse Error: {}", e))?;

        // 🚀 Cluaiz Detection Logic: The Triple Handshake
        let os = control.identity.os_target.to_lowercase();
        let arch = control.identity.architecture.to_lowercase();
        let gpu_vendor = control
            .silicon_truth
            .accelerators
            .gpus
            .first()
            .map(|g| g.vendor.to_lowercase());
        let has_drivers = !control.silicon_truth.active_drivers.is_empty();

        println!(
            "🎯 Engine Prep: OS={}, Arch={}, GPU={:?}, Drivers={}",
            os, arch, gpu_vendor, has_drivers
        );

        // 🧠 Mission 12: Chronicle Core Activity
        // Temporarily commented out due to missing CoreGraph in cluaiz_shared
        // let _ = cluaiz_shared::Core::graph::CoreGraph::chronicle_pulse(
        //     "Hardware Handshake & Engine Preparation",
        //     engine_type,
        //     &format!("OS: {}, GPU: {:?}", os, gpu_vendor)
        // );

        // 1. Resolve Hardware Suffix based on Deep Probing
        let suffix = match (os.as_str(), arch.as_str()) {
            // --- Apple Hardware (Metal Mastery) ---
            ("macos", "aarch64")
                if gpu_vendor
                    .as_ref()
                    .map(|v| v.contains("apple"))
                    .unwrap_or(false) =>
            {
                "metal"
            }

            // --- Linux/Windows High-Performance Targets ---
            ("linux", _) | ("windows", _) if gpu_vendor.is_some() => {
                let vendor = gpu_vendor.as_ref().unwrap();
                if vendor.contains("nvidia") && has_drivers {
                    "cuda"
                } else if vendor.contains("amd") {
                    "rocm"
                } else {
                    "vulkan"
                }
            }

            // --- ARM / Raspberry Pi Optimized ---
            ("linux", "aarch64") | ("linux", "arm") => "arm64",

            // --- Mobile Cluaiz Targets ---
            ("android", _) => "android",
            ("ios", _) => "ios",

            // --- Legacy / Generic Fallback ---
            _ => "cpu",
        };

        // 🚀 NATIVE PROVISIONING: Ensure silicon drivers exist before linkage
        if suffix != "cpu" {
            if let Err(e) = DriverProvisioner::provision_for_hardware(suffix).await {
                println!(
                    "  {} [PROVISIONER] Silicon Handshake Error: {}",
                    "⚠️".yellow(),
                    e
                );
                // We continue for now, as the user might already have the drivers in PATH
            }
        }

        let binary_id = format!("{}-{}", engine_type, suffix);

        let mut target_suffix = suffix;
        let mut target_binary_id = binary_id.clone();

        // 🚀 Cluaiz VRAM Handshake: Pre-Flight Check (Hardware-Aware Routing)
        // Only enforce strict GPU VRAM limits if the target hardware is a GPU backend.
        if suffix == "cuda" || suffix == "metal" || suffix == "rocm" || suffix == "vulkan" {
            // Defaulting to 2.0GB for V1 Baseline. Future: Pull from model metadata.
            let required_vram = 2.0;
            if let Err(e) = HardwareGovernor::request_vram(&binary_id, required_vram) {
                tracing::warn!(
                    "⚠️ [Arbiter] VRAM Arbitration Failed ({}). Falling back to CPU.",
                    e
                );
                target_suffix = "cpu";
                target_binary_id = format!("{}-cpu", engine_type);
            }
        } else {
            tracing::info!("🧠 [Arbiter] Hardware Linkage targeting CPU/System RAM. Bypassing GPU VRAM limits.");
        }

        // 2. Check local presence in cluaiz/interface-engines/
        if self.loader.exists(&target_binary_id) {
            Ok(self.loader.resolve_path(&target_binary_id))
        } else {
            // 🔄 NATIVE FALLBACK: If GPU binary is missing, attempt CPU fallback
            if target_suffix != "cpu" {
                tracing::warn!(
                    "⚠️ [Linker] Engine '{}' missing. Attempting CPU fallback...",
                    target_binary_id
                );
                let fallback_id = format!("{}-cpu", engine_type);
                if self.loader.exists(&fallback_id) {
                    // Release the GPU VRAM we aggressively reserved earlier
                    let _ = HardwareGovernor::release_vram(&target_binary_id);
                    tracing::info!("✅ [Linker] CPU Fallback successful: {}", fallback_id);
                    return Ok(self.loader.resolve_path(&fallback_id));
                }
            }

            // If binary missing (and no fallback), release the reserved memory immediately
            let _ = HardwareGovernor::release_vram(&target_binary_id);
            Err(format!("Engine Binary Missing: Please pull the '{}' package for your {} Hardware into your cluaiz/interface-engines/ folder", target_binary_id, os))
        }
    }

    /// Unload Engine: Release resources back to the Cluaiz Governor.
    pub fn release_engine(&self, engine_type: &str) -> anyhow::Result<()> {
        // We need to know which suffix was used to reconstruct the ID
        // For simplicity in V1, we iterate and release what matches the prefix
        HardwareGovernor::release_vram(engine_type)
    }

    /// 🔗 Cluaiz Linker: Maps the binary kernel to process memory and resolves symbols.
    pub fn load_and_link(&mut self, binary_path: PathBuf) -> anyhow::Result<()> {
        tracing::info!("🧬 [Linker] Mapping binary: {:?}", binary_path);

        // 🪟 WINDOWS SEARCH PATCH: Add drivers directory to DLL search path
        #[cfg(target_os = "windows")]
        {
            use std::os::windows::ffi::OsStrExt;
            let discovery_paths = DriverProvisioner::discover_system_paths();

            unsafe {
                extern "system" {
                    fn SetDllDirectoryW(lpPathName: *const u16) -> i32;
                }

                for path in discovery_paths {
                    if path.exists() {
                        let mut path_wide: Vec<u16> = path.as_os_str().encode_wide().collect();
                        path_wide.push(0);
                        SetDllDirectoryW(path_wide.as_ptr());
                    }
                }
            }
        }

        unsafe {
            let lib = Library::new(&binary_path)
                .map_err(|e| anyhow::anyhow!("Binary Mapping Failed (libloading): {}", e))?;

            // 🎯 Phase 1: Symbol Validation
            let _init: Symbol<unsafe extern "C" fn() -> *const std::os::raw::c_char> =
                lib.get(b"cluaiz_kernel_init").map_err(|_| {
                    anyhow::anyhow!("Invalid Kernel: 'cluaiz_kernel_init' symbol missing.")
                })?;

            tracing::info!("✅ [Linker] 7ns Handshake Complete. Kernel Linked.");
            self.active_lib = Some(lib);
        }

        Ok(())
    }

    /// 🏛️ Core Instantiation: Invokes the kernel's factory method to create an active execution engine.
    pub fn instantiate(&self, model_path: &str) -> anyhow::Result<()> {
        let lib = self
            .active_lib
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Linker Error: No active kernel linked."))?;

        unsafe {
            let instantiate_fn: Symbol<
                unsafe extern "C" fn(*const std::os::raw::c_char) -> *mut std::ffi::c_void,
            > = lib.get(b"cluaiz_kernel_instantiate").map_err(|_| {
                anyhow::anyhow!("Invalid Kernel: 'cluaiz_kernel_instantiate' symbol missing.")
            })?;

            let c_path = std::ffi::CString::new(model_path)?;
            let _engine_ptr = instantiate_fn(c_path.as_ptr() as *const std::os::raw::c_char);

            tracing::info!("🚀 [Linker] Core Kernel Instantiated at Bare-Metal level.");
        }

        Ok(())
    }

    fn get_system_control_path(&self) -> PathBuf {
        HardwareGovernor::resolve_engine_path().join("system_control.json")
    }
}
