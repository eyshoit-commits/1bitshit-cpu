use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnvironmentMode {
    Development,
    Installed,
    Portable,
    Testing,
}

#[derive(Debug, Clone)]
pub struct EnvironmentManager {
    pub mode: EnvironmentMode,
    pub local_dir: PathBuf,
    pub global_dir: PathBuf,
}

impl EnvironmentManager {
    /// Resolve the active 1BitShit environment without silently falling back to
    /// the retired `.cluaiz` runtime tree.
    pub fn current() -> Self {
        if let Ok(exe_path) = std::env::current_exe() {
            if let Some(parent) = exe_path.parent() {
                if parent.join("portable.flag").exists() {
                    return Self {
                        mode: EnvironmentMode::Portable,
                        local_dir: parent.to_path_buf(),
                        global_dir: parent.to_path_buf(),
                    };
                }
            }
        }

        if let Ok(env_path) = std::env::var("BITSHIT_HOME")
            .or_else(|_| std::env::var("BITSHIT_ROOT"))
        {
            let root = PathBuf::from(env_path);
            return Self {
                mode: EnvironmentMode::Installed,
                local_dir: root.clone(),
                global_dir: root,
            };
        }

        let current_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        if Self::is_local_workspace_binary(&current_dir) {
            return Self {
                mode: EnvironmentMode::Development,
                local_dir: current_dir.join(".1bitshit"),
                global_dir: current_dir,
            };
        }

        let home_dir = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        let global_path = Self::installed_root(&home_dir);
        Self {
            mode: EnvironmentMode::Installed,
            local_dir: global_path.clone(),
            global_dir: global_path,
        }
    }

    fn is_local_workspace_binary(current_dir: &Path) -> bool {
        std::env::current_exe()
            .ok()
            .map(|exe| exe.starts_with(current_dir.join("target")))
            .unwrap_or(false)
            || std::env::var("CARGO").is_ok()
            || std::env::var("CARGO_MANIFEST_DIR").is_ok()
    }

    fn installed_root(home_dir: &Path) -> PathBuf {
        home_dir.join(".1bitshit")
    }

    pub fn engine_dir(&self) -> PathBuf {
        self.local_dir.join("engine")
    }

    pub fn kernel_dir(&self) -> PathBuf {
        self.engine_dir()
    }

    pub fn drivers_dir(&self) -> PathBuf {
        self.engine_dir().join("drivers")
    }

    pub fn config_dir(&self) -> PathBuf {
        self.engine_dir().join("config")
    }

    pub fn models_dir(&self) -> PathBuf {
        if let Some(path) = std::env::var_os("BITSHIT_MODELS_DIR") {
            return PathBuf::from(path);
        }

        if self.mode == EnvironmentMode::Development {
            return self.global_dir.join("models");
        }

        self.global_dir.join("models")
    }

    pub fn chat_models_dir(&self) -> PathBuf {
        self.models_dir().join("chat")
    }

    pub fn embedding_models_dir(&self) -> PathBuf {
        self.models_dir().join("embedding")
    }

    pub fn vision_models_dir(&self) -> PathBuf {
        self.models_dir().join("vision")
    }

    pub fn kv_cache_dir(&self) -> PathBuf {
        self.local_dir.join("kv_cache")
    }

    pub fn skills_dir(&self) -> PathBuf {
        self.global_dir.join("skills")
    }

    pub fn extensions_dir(&self) -> PathBuf {
        self.global_dir.join("extensions")
    }

    pub fn plugins_dir(&self) -> PathBuf {
        self.global_dir.join("plugins")
    }

    pub fn mcp_dir(&self) -> PathBuf {
        self.global_dir.join("mcp")
    }

    pub fn reports_dir(&self) -> PathBuf {
        self.local_dir.join("reports")
    }

    fn ensure_dir(path: PathBuf) -> std::io::Result<PathBuf> {
        std::fs::create_dir_all(&path)?;
        Ok(path)
    }

    pub fn ensure_kv_cache_dir(&self) -> std::io::Result<PathBuf> {
        Self::ensure_dir(self.kv_cache_dir())
    }

    pub fn ensure_engine_dir(&self) -> std::io::Result<PathBuf> {
        Self::ensure_dir(self.engine_dir())
    }

    pub fn ensure_kernel_dir(&self) -> std::io::Result<PathBuf> {
        Self::ensure_dir(self.kernel_dir())
    }

    pub fn ensure_drivers_dir(&self) -> std::io::Result<PathBuf> {
        Self::ensure_dir(self.drivers_dir())
    }

    pub fn ensure_config_dir(&self) -> std::io::Result<PathBuf> {
        let dir = Self::ensure_dir(self.config_dir())?;
        let engine_dir = self.engine_dir();
        let legacy_files = [
            "Permission.json",
            "Permission.bin",
            "system_control.json",
            "system_control.bin",
            "package.json",
            "package.bin",
        ];

        for file in legacy_files {
            let legacy_path = engine_dir.join(file);
            let new_path = dir.join(file);
            if legacy_path.exists() {
                if !new_path.exists() {
                    if let Err(error) = std::fs::copy(&legacy_path, &new_path) {
                        tracing::warn!(
                            "Failed to migrate legacy config {}: {}",
                            file,
                            error
                        );
                    } else {
                        let _ = std::fs::remove_file(&legacy_path);
                    }
                } else {
                    let _ = std::fs::remove_file(&legacy_path);
                }
            }
        }

        Ok(dir)
    }

    pub fn ensure_models_dir(&self) -> std::io::Result<PathBuf> {
        Self::ensure_dir(self.models_dir())
    }

    pub fn ensure_chat_models_dir(&self) -> std::io::Result<PathBuf> {
        Self::ensure_dir(self.chat_models_dir())
    }

    pub fn ensure_embedding_models_dir(&self) -> std::io::Result<PathBuf> {
        Self::ensure_dir(self.embedding_models_dir())
    }

    pub fn ensure_vision_models_dir(&self) -> std::io::Result<PathBuf> {
        Self::ensure_dir(self.vision_models_dir())
    }

    pub fn ensure_skills_dir(&self) -> std::io::Result<PathBuf> {
        Self::ensure_dir(self.skills_dir())
    }

    pub fn ensure_extensions_dir(&self) -> std::io::Result<PathBuf> {
        Self::ensure_dir(self.extensions_dir())
    }

    pub fn ensure_plugins_dir(&self) -> std::io::Result<PathBuf> {
        Self::ensure_dir(self.plugins_dir())
    }

    pub fn ensure_mcp_dir(&self) -> std::io::Result<PathBuf> {
        Self::ensure_dir(self.mcp_dir())
    }
}
