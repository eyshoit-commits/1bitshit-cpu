use std::path::PathBuf;

/// Reads `cluaiz_root` securely via the Cluaiz Hardware Governor.
/// This uses the binary truth (`system_control.bin`) as the ultimate source,
/// exactly as the Cluaiz Architecture intends. Zero custom hardcoding.
fn read_cluaiz_root() -> Option<PathBuf> {
    match cluaiz_shared::HardwareGovernor::load_system_control() {
        Ok(control) => Some(PathBuf::from(control.context.cluaiz_root)),
        Err(e) => {
            tracing::error!("❌ [KernelLoader] Failed to read System Truth: {}", e);
            None
        }
    }
}

/// Kernel Loader
/// Manages pre-compiled binaries (.dll, .so, .dylib) for different OS/Architecture pairs.
/// All paths are resolved dynamically via system_control.json. Zero hardcoding.
pub struct KernelLoader {
    base_dir: PathBuf,
}

impl KernelLoader {
    pub fn new(base_dir: PathBuf) -> Self {
        Self { base_dir }
    }

    /// Checks if a kernel binary exists locally for a target OS.
    pub fn exists_for_os(&self, kernel_name: &str, os: &str) -> bool {
        let path = self.resolve_path_for_os(kernel_name, os);
        path.exists()
    }

    /// Checks if a kernel binary exists locally for the current OS.
    pub fn exists(&self, kernel_name: &str) -> bool {
        let path = self.resolve_path(kernel_name);
        path.exists()
    }

    /// Resolves path based on current compilation target (NATIVE).
    pub fn resolve_path(&self, kernel_name: &str) -> PathBuf {
        let os = if cfg!(target_os = "windows") {
            "Windows"
        } else if cfg!(target_os = "linux") {
            "Linux"
        } else if cfg!(target_os = "android") {
            "Android"
        } else if cfg!(target_os = "macos") {
            "macOS"
        } else if cfg!(target_os = "ios") {
            "iOS"
        } else {
            "Unknown"
        };
        self.resolve_path_for_os(kernel_name, os)
    }

    /// Resolves the absolute path for a kernel binary for a SPECIFIC OS.
    /// Priority: [cluaiz_root]/interface-engines/ → fallback to base_dir/target/release/
    pub fn resolve_path_for_os(&self, kernel_name: &str, os: &str) -> PathBuf {
        let ext = match os {
            "Windows" => "dll",
            "Linux" | "Android" => "so",
            "macOS" | "iOS" => "dylib",
            _ => "bin",
        };

        // We try multiple potential naming conventions and subdirectories
        let mut candidates = Vec::new();

        // 1. Unified Cluaiz Naming Format (e.g. cluaiz-llama.dll, libcluaiz_llama.so)
        candidates.push(format!("cluaiz-{}.{}", kernel_name, ext));
        candidates.push(format!("cluaiz_{}.{}", kernel_name, ext));
        candidates.push(format!("libcluaiz_{}.{}", kernel_name, ext));
        candidates.push(format!("libcluaiz-{}.{}", kernel_name, ext));

        // 2. Legacy Archer Naming Format (e.g. archer_llama.dll, libarcher_llama.so)
        candidates.push(format!("archer_{}.{}", kernel_name, ext));
        candidates.push(format!("archer-{}.{}", kernel_name, ext));
        candidates.push(format!("libarcher_{}.{}", kernel_name, ext));

        // 3. Simple Base checks for hyphenated names (e.g. llama-cuda -> llama)
        if kernel_name.contains('-') {
            let base_name = kernel_name.split('-').next().unwrap_or(kernel_name);
            candidates.push(format!("cluaiz-{}.{}", base_name, ext));
            candidates.push(format!("cluaiz_{}.{}", base_name, ext));
            candidates.push(format!("libcluaiz_{}.{}", base_name, ext));
            candidates.push(format!("archer_{}.{}", base_name, ext));
            candidates.push(format!("libarcher_{}.{}", base_name, ext));
        }

        // 3. System Truth: Read cluaiz_root
        if let Some(cluaiz_root) = read_cluaiz_root() {
            let base_link = cluaiz_root.join("interface-engines");

            for file_name in &candidates {
                // Check root interface-engines/
                let path = base_link.join(file_name);
                if path.exists() {
                    tracing::info!("🎯 [KernelLoader] Cluaiz path resolved: {:?}", path);
                    return path;
                }

                // Check kernels/ subdirectory
                let path_kernels = base_link.join("kernels").join(file_name);
                if path_kernels.exists() {
                    tracing::info!(
                        "🎯 [KernelLoader] Cluaiz path resolved (kernels/): {:?}",
                        path_kernels
                    );
                    return path_kernels;
                }
            }
        }

        // 4. FALLBACK: Local development build output
        let mut dev_path = self.base_dir.clone();
        dev_path.push("target");
        dev_path.push("release");
        let fallback_file = format!("archer_{}.{}", kernel_name, ext);
        dev_path.push(&fallback_file);

        tracing::warn!(
            "⚠️ [KernelLoader] Cluaiz path not found for {}. Falling back to dev path: {:?}",
            kernel_name,
            dev_path
        );
        dev_path
    }
}
