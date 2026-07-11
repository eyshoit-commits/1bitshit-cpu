use std::path::PathBuf;

/// Resolves native Llama, ONNX and future driver libraries for the active
/// 1BitShit CPU runtime. New artifact names are authoritative. Legacy names
/// are accepted only as a non-destructive compatibility fallback.
pub struct KernelLoader {
    base_dir: PathBuf,
}

impl KernelLoader {
    pub fn new(base_dir: PathBuf) -> Self {
        Self { base_dir }
    }

    pub fn exists_for_os(&self, kernel_name: &str, os: &str) -> bool {
        self.resolve_path_for_os(kernel_name, os).is_file()
    }

    pub fn exists(&self, kernel_name: &str) -> bool {
        self.resolve_path(kernel_name).is_file()
    }

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

    pub fn resolve_path_for_os(&self, kernel_name: &str, os: &str) -> PathBuf {
        let extension = match os {
            "Windows" => "dll",
            "Linux" | "Android" => "so",
            "macOS" | "iOS" => "dylib",
            _ => "bin",
        };

        let candidates = Self::candidate_names(kernel_name, extension);
        let current_dir = std::env::current_dir().unwrap_or_else(|_| self.base_dir.clone());

        for profile in ["release", "debug"] {
            let profile_dir = current_dir.join("target").join(profile);
            for file_name in &candidates {
                for path in [profile_dir.join(file_name), profile_dir.join("deps").join(file_name)] {
                    if path.is_file() {
                        tracing::info!(
                            "[1BitShit KernelLoader] Development kernel resolved: {}",
                            path.display()
                        );
                        return path;
                    }
                }
            }
        }

        let environment = cluaiz_shared::environment::EnvironmentManager::current();
        let engine_dir = environment.engine_dir();
        for file_name in &candidates {
            for path in [engine_dir.join(file_name), engine_dir.join("drivers").join(file_name)] {
                if path.is_file() {
                    tracing::info!(
                        "[1BitShit KernelLoader] Installed kernel resolved: {}",
                        path.display()
                    );
                    return path;
                }
            }
        }

        let fallback = engine_dir.join(Self::primary_name(kernel_name, extension));
        tracing::warn!(
            "[1BitShit KernelLoader] Kernel '{}' was not found. Expected {}",
            kernel_name,
            fallback.display()
        );
        fallback
    }

    fn primary_name(kernel_name: &str, extension: &str) -> String {
        if cfg!(windows) {
            format!("bitshit-{kernel_name}.{extension}")
        } else {
            format!("libbitshit_{kernel_name}.{extension}")
        }
    }

    fn candidate_names(kernel_name: &str, extension: &str) -> Vec<String> {
        vec![
            format!("bitshit-{kernel_name}.{extension}"),
            format!("bitshit_{kernel_name}.{extension}"),
            format!("libbitshit_{kernel_name}.{extension}"),
            format!("libbitshit-{kernel_name}.{extension}"),
            // Compatibility readers. These names are never created by the new runtime.
            format!("cluaiz-{kernel_name}.{extension}"),
            format!("cluaiz_{kernel_name}.{extension}"),
            format!("libcluaiz_{kernel_name}.{extension}"),
            format!("libcluaiz-{kernel_name}.{extension}"),
            format!("archer_{kernel_name}.{extension}"),
            format!("archer-{kernel_name}.{extension}"),
            format!("libarcher_{kernel_name}.{extension}"),
        ]
    }
}
