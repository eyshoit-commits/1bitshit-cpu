use std::path::{Path, PathBuf};

use cluaiz_shared::environment::{EnvironmentManager, EnvironmentMode};
use cluaiz_shared::HardwareGovernor;
use color_eyre::{eyre::eyre, Result};
use colored::Colorize;

pub struct Bootstrapper;

impl Bootstrapper {
    const MASTER_REGISTRY_URL: &'static str =
        "https://raw.githubusercontent.com/eyshoit-commits/1bitshit-cpu/main/package.json";
    const PRODUCT: &'static str = "1BitShit CPU";

    /// Boots the complete local runtime. Legacy Cluaiz data is only imported;
    /// no legacy executable, model loader or hidden runtime path is activated.
    pub async fn ignite(is_dev_sync: bool) -> Result<()> {
        let env = EnvironmentManager::current();
        Self::migrate_legacy_runtime(&env)?;

        let profile = if cfg!(debug_assertions) { "debug" } else { "release" };
        let _ = Self::sync_dev_artifacts("all", None, env.global_dir.clone(), profile);
        Self::ensure_global_path();

        tracing::info!("[1BitShit CPU] Initializing permissions, skills and extensions");
        let mut permissions =
            engines::neural_foundry::security::permission_schema::PermissionSchema::load();
        permissions.auto_assign_defaults();

        let mut registry = engines::neural_foundry::registry::SkillRegistry::new();
        for dir in [
            env.skills_dir(),
            env.extensions_dir(),
            env.plugins_dir(),
            env.mcp_dir(),
        ] {
            if dir.exists() {
                registry.load_from_directory(&dir.to_string_lossy());
            }
        }

        let hub_path = HardwareGovernor::resolve_hub_path();
        std::fs::create_dir_all(&hub_path)?;
        let license_text = include_str!("../assets/THIRD_PARTY_NOTICES.txt");
        std::fs::write(hub_path.join("THIRD_PARTY_LICENSES.txt"), license_text)?;

        #[cfg(windows)]
        let _ = colored::control::set_virtual_terminal(true);

        if is_dev_sync {
            tracing::info!("[1BitShit CPU] Dev-sync mode: network provisioning skipped");
            return Ok(());
        }

        let client = reqwest::Client::builder()
            .user_agent(format!("1bitshit-cpu/{}", env!("CARGO_PKG_VERSION")))
            .build()?;

        let master_registry = match client.get(Self::MASTER_REGISTRY_URL).send().await {
            Ok(response) if response.status().is_success() => match response.json().await {
                Ok(json) => json,
                Err(error) => {
                    println!(
                        "  {} [{}] Registry response could not be parsed: {}. Continuing offline.",
                        "⚠️".yellow(),
                        Self::PRODUCT,
                        error
                    );
                    return Ok(());
                }
            },
            Ok(response) => {
                println!(
                    "  {} [{}] Registry unavailable (HTTP {}). Continuing offline.",
                    "⚠️".yellow(),
                    Self::PRODUCT,
                    response.status()
                );
                return Ok(());
            }
            Err(error) => {
                println!(
                    "  {} [{}] Offline mode: {}",
                    "⚠️".yellow(),
                    Self::PRODUCT,
                    error
                );
                return Ok(());
            }
        };

        cluaiz_shared::RegistryGovernor::seal_registry(master_registry.clone())
            .map_err(|error| eyre!("Registry seal failed: {error}"))?;

        let latest_cli = master_registry["components"]["cli"]["version"]
            .as_str()
            .unwrap_or("");
        let current_cli = env!("CARGO_PKG_VERSION");
        if Self::is_newer_version(latest_cli, current_cli) {
            println!(
                "  {} [{}] Update available: {} -> {}",
                "🚀".green(),
                Self::PRODUCT,
                current_cli,
                latest_cli
            );
        }

        Self::sync_engine(&client, &master_registry).await?;
        Self::sync_neural_stack(&client, &master_registry).await?;
        Ok(())
    }

    fn is_newer_version(latest: &str, current: &str) -> bool {
        fn numeric_version(value: &str) -> Option<Vec<u64>> {
            let normalized = value.trim().trim_start_matches('v');
            if normalized.is_empty()
                || !normalized
                    .chars()
                    .all(|ch| ch.is_ascii_digit() || ch == '.' || ch == '-')
            {
                return None;
            }
            let numeric = normalized.split('-').next().unwrap_or(normalized);
            let parts: Vec<u64> = numeric
                .split('.')
                .map(str::parse::<u64>)
                .collect::<std::result::Result<_, _>>()
                .ok()?;
            if parts.is_empty() { None } else { Some(parts) }
        }

        match (numeric_version(latest), numeric_version(current)) {
            (Some(latest), Some(current)) => latest > current,
            _ => false,
        }
    }

    async fn sync_engine(client: &reqwest::Client, registry: &serde_json::Value) -> Result<()> {
        let engine_info = &registry["components"]["engine"];
        if engine_info.is_null() {
            return Ok(());
        }

        let engine_dir = HardwareGovernor::resolve_engine_path();
        std::fs::create_dir_all(&engine_dir)?;
        let ext = Self::dynamic_library_extension();
        let engine_path = engine_dir.join(format!("bitshit-engine.{ext}"));
        let marker_path = engine_dir.join("bitshit-engine.ready");

        Self::import_legacy_artifact(
            &engine_dir.join(format!("cluaiz-engine.{ext}")),
            &engine_path,
        )?;

        let manifest_version = engine_info["version"].as_str().unwrap_or("unknown");
        let local_version = std::fs::read_to_string(&marker_path).unwrap_or_default();
        if engine_path.exists() && local_version.trim() == manifest_version {
            return Ok(());
        }

        let Some(manifest_url) = engine_info["manifest_url"].as_str() else {
            if engine_path.exists() {
                return Ok(());
            }
            return Err(eyre!("Engine manifest URL is missing"));
        };

        let result = async {
            let response = client.get(manifest_url).send().await?;
            if !response.status().is_success() {
                return Err(eyre!("Engine registry returned HTTP {}", response.status()));
            }
            let manifest: serde_json::Value = response.json().await?;
            Self::download_engine_with_manifest(client, &engine_path, &manifest).await?;
            std::fs::write(&marker_path, manifest_version)?;
            Ok::<(), color_eyre::Report>(())
        }
        .await;

        match result {
            Ok(()) => Ok(()),
            Err(error) if engine_path.exists() => {
                println!(
                    "  {} [{}] Engine update failed: {}. Using local engine.",
                    "⚠️".yellow(),
                    Self::PRODUCT,
                    error
                );
                Ok(())
            }
            Err(error) => Err(error),
        }
    }

    async fn download_engine_with_manifest(
        client: &reqwest::Client,
        destination: &Path,
        manifest: &serde_json::Value,
    ) -> Result<()> {
        let platform = Self::platform_key();
        let Some(url) = manifest["engines"][platform].as_str() else {
            return Err(eyre!("No engine binary for platform '{platform}'"));
        };
        Self::download_asset(client, url, destination).await
    }

    async fn sync_neural_stack(
        client: &reqwest::Client,
        registry: &serde_json::Value,
    ) -> Result<()> {
        let kernel_info = &registry["components"]["kernel"];
        if kernel_info.is_null() {
            return Ok(());
        }

        let engine_dir = HardwareGovernor::resolve_interface_path();
        std::fs::create_dir_all(&engine_dir)?;
        let ext = Self::dynamic_library_extension();
        let kernel_path = engine_dir.join(format!("bitshit-llama.{ext}"));
        let marker_path = engine_dir.join("bitshit-llama.ready");

        Self::import_legacy_artifact(
            &engine_dir.join(format!("cluaiz-llama.{ext}")),
            &kernel_path,
        )?;

        let manifest_version = kernel_info["version"].as_str().unwrap_or("unknown");
        let local_version = std::fs::read_to_string(&marker_path).unwrap_or_default();
        if !kernel_path.exists() || local_version.trim() != manifest_version {
            if let Some(manifest_url) = kernel_info["manifest_url"].as_str() {
                let result = async {
                    let response = client.get(manifest_url).send().await?;
                    if !response.status().is_success() {
                        return Err(eyre!("Kernel registry returned HTTP {}", response.status()));
                    }
                    let manifest: serde_json::Value = response.json().await?;
                    let platform = Self::platform_key();
                    let specialization = Self::specialized_platform_key(platform);
                    let url = manifest["kernels"][&specialization]
                        .as_str()
                        .or_else(|| manifest["kernels"][platform].as_str())
                        .ok_or_else(|| eyre!("No Llama kernel for '{specialization}'"))?;
                    Self::download_asset(client, url, &kernel_path).await?;
                    std::fs::write(&marker_path, manifest_version)?;
                    Ok::<(), color_eyre::Report>(())
                }
                .await;

                if let Err(error) = result {
                    if !kernel_path.exists() {
                        return Err(error);
                    }
                    println!(
                        "  {} [{}] Llama kernel update failed: {}. Using local kernel.",
                        "⚠️".yellow(),
                        Self::PRODUCT,
                        error
                    );
                }
            }
        }

        let has_nvidia = HardwareGovernor::load_system_control()
            .ok()
            .and_then(|control| control.silicon_truth.accelerators.gpus.first().cloned())
            .map(|gpu| gpu.vendor.to_ascii_uppercase().contains("NVIDIA"))
            .unwrap_or(false);

        if has_nvidia {
            let driver_manifest_url = registry["components"]["drivers"]["manifest_url"]
                .as_str()
                .unwrap_or_default();
            let _ = engines::interface_engines::manager::driver_provisioner::DriverProvisioner::provision_for_hardware(
                "cuda",
                driver_manifest_url,
            )
            .await;
        }

        Ok(())
    }

    async fn download_asset(
        client: &reqwest::Client,
        url: &str,
        destination: &Path,
    ) -> Result<()> {
        if let Some(parent) = destination.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let response = client.get(url).send().await?;
        if !response.status().is_success() {
            return Err(eyre!("Download returned HTTP {} for {url}", response.status()));
        }
        let temporary = destination.with_extension(format!(
            "{}.part",
            destination
                .extension()
                .and_then(|ext| ext.to_str())
                .unwrap_or("download")
        ));
        std::fs::write(&temporary, response.bytes().await?)?;
        std::fs::rename(temporary, destination)?;
        Ok(())
    }

    /// Synchronizes locally compiled artifacts into the active 1BitShit runtime.
    pub fn sync_dev_artifacts(
        target: &str,
        driver_name: Option<&str>,
        hub_path: PathBuf,
        profile: &str,
    ) -> Result<()> {
        let root = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let ext = Self::dynamic_library_extension();
        let target_dir = root
            .join("target")
            .join(if profile == "release" { "release" } else { "debug" });

        let find_artifact = |base_names: &[&str]| -> Option<PathBuf> {
            for base in base_names {
                for file_name in [
                    format!("{base}.{ext}"),
                    format!("lib{base}.{ext}"),
                ] {
                    for candidate in [
                        target_dir.join(&file_name),
                        target_dir.join("deps").join(&file_name),
                    ] {
                        if candidate.is_file() {
                            return Some(candidate);
                        }
                    }
                }
            }
            None
        };

        if target == "all" || target == "core" {
            let engine_destination = hub_path.join("engine").join(format!("bitshit-engine.{ext}"));
            if let Some(engine_source) = find_artifact(&["engines"]) {
                Self::copy_artifact(&engine_source, &engine_destination)?;
                std::fs::write(
                    hub_path.join("engine").join("bitshit-engine.ready"),
                    env!("CARGO_PKG_VERSION"),
                )?;
            }
        }

        let kernels: Vec<(Vec<&str>, &str, bool)> = vec![
            (vec!["bitshit_llama", "cluaiz_llama"], "bitshit-llama", false),
            (vec!["bitshit_onnx", "cluaiz_onnx"], "bitshit-onnx", false),
            (vec!["onnxruntime"], "onnxruntime", true),
            (
                vec!["onnxruntime_providers_shared"],
                "onnxruntime_providers_shared",
                true,
            ),
            (
                vec!["onnxruntime_providers_cuda"],
                "onnxruntime_providers_cuda",
                true,
            ),
            (
                vec!["onnxruntime_providers_tensorrt"],
                "onnxruntime_providers_tensorrt",
                true,
            ),
            (
                vec!["onnxruntime_providers_nv_tensorrt_rtx"],
                "onnxruntime_providers_nv_tensorrt_rtx",
                true,
            ),
        ];

        if target != "core" {
            for (source_names, destination_name, is_runtime_dependency) in kernels {
                if target == "driver" {
                    if let Some(requested) = driver_name {
                        if !destination_name.contains(requested)
                            && !source_names.iter().any(|name| name.contains(requested))
                        {
                            continue;
                        }
                    }
                }

                let Some(source) = find_artifact(&source_names) else {
                    continue;
                };
                let destination = if is_runtime_dependency {
                    hub_path
                        .join("engine")
                        .join("drivers")
                        .join(format!("{destination_name}.{ext}"))
                } else {
                    hub_path
                        .join("engine")
                        .join(format!("{destination_name}.{ext}"))
                };
                Self::copy_artifact(&source, &destination)?;
                if !is_runtime_dependency {
                    std::fs::write(
                        hub_path
                            .join("engine")
                            .join(format!("{destination_name}.ready")),
                        env!("CARGO_PKG_VERSION"),
                    )?;
                }
            }
        }

        if target == "all" || target == "core" {
            let executable = if cfg!(windows) { "bitshit.exe" } else { "bitshit" };
            let executable_source = target_dir.join(executable);
            if executable_source.is_file() {
                Self::copy_artifact(
                    &executable_source,
                    &hub_path.join("bin").join(executable),
                )?;
            }

            let local_runtime = root.join(".1bitshit");
            let legacy_runtime = root.join(".cluaiz");
            let source_runtime = if local_runtime.exists() {
                Some(local_runtime)
            } else if legacy_runtime.exists() {
                Some(legacy_runtime)
            } else {
                None
            };
            if let Some(source_runtime) = source_runtime {
                for folder in ["brain", "skills", "extensions", "plugins", "mcp"] {
                    let source = source_runtime.join(folder);
                    if source.exists() {
                        Self::copy_dir_recursive(&source, &hub_path.join(folder))?;
                    }
                }
            }
        }

        Ok(())
    }

    fn migrate_legacy_runtime(env: &EnvironmentManager) -> Result<()> {
        if env.mode != EnvironmentMode::Installed {
            return Ok(());
        }
        let Some(home) = dirs::home_dir() else {
            return Ok(());
        };
        let legacy = home.join(".cluaiz");
        let destination = env.global_dir.clone();
        let marker = destination.join(".legacy-cluaiz-import-complete");
        if !legacy.is_dir() || marker.exists() || legacy == destination {
            return Ok(());
        }

        std::fs::create_dir_all(&destination)?;
        for folder in [
            "engine",
            "models",
            "brain",
            "skills",
            "extensions",
            "plugins",
            "mcp",
            "kv_cache",
            "reports",
        ] {
            let source = legacy.join(folder);
            if source.exists() {
                Self::copy_dir_recursive(&source, &destination.join(folder))?;
            }
        }
        std::fs::write(
            marker,
            "Legacy data imported by 1BitShit CPU. The old runtime was not executed or deleted.\n",
        )?;
        println!(
            "  {} [{}] Existing legacy data was imported into {}.",
            "✅".green(),
            Self::PRODUCT,
            destination.display()
        );
        Ok(())
    }

    fn import_legacy_artifact(source: &Path, destination: &Path) -> Result<()> {
        if destination.exists() || !source.is_file() {
            return Ok(());
        }
        Self::copy_artifact(source, destination)
    }

    fn copy_artifact(source: &Path, destination: &Path) -> Result<()> {
        if let Some(parent) = destination.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let temporary = destination.with_extension(format!(
            "{}.part",
            destination
                .extension()
                .and_then(|ext| ext.to_str())
                .unwrap_or("copy")
        ));
        std::fs::copy(source, &temporary)?;
        std::fs::rename(temporary, destination)?;
        Ok(())
    }

    fn copy_dir_recursive(source: &Path, destination: &Path) -> std::io::Result<()> {
        std::fs::create_dir_all(destination)?;
        for entry in std::fs::read_dir(source)? {
            let entry = entry?;
            let target = destination.join(entry.file_name());
            if entry.file_type()?.is_dir() {
                Self::copy_dir_recursive(&entry.path(), &target)?;
            } else if !target.exists() {
                std::fs::copy(entry.path(), target)?;
            }
        }
        Ok(())
    }

    fn dynamic_library_extension() -> &'static str {
        if cfg!(windows) {
            "dll"
        } else if cfg!(target_os = "macos") {
            "dylib"
        } else {
            "so"
        }
    }

    fn platform_key() -> &'static str {
        if cfg!(windows) {
            "win-x64"
        } else if cfg!(target_os = "macos") {
            "mac-arm64"
        } else {
            "linux-x64"
        }
    }

    fn specialized_platform_key(platform: &str) -> String {
        if platform != "win-x64" && platform != "linux-x64" {
            return platform.to_string();
        }
        let has_avx512 = HardwareGovernor::load_system_control()
            .ok()
            .map(|control| {
                control
                    .silicon_truth
                    .cpu
                    .isa_features
                    .iter()
                    .any(|feature| feature.eq_ignore_ascii_case("AVX-512"))
            })
            .unwrap_or(false);
        format!("{}-{}", platform, if has_avx512 { "avx512" } else { "avx2" })
    }

    fn ensure_global_path() {
        #[cfg(windows)]
        {
            let bin_dir = HardwareGovernor::resolve_bin_gateway();
            let bin = bin_dir.to_string_lossy().replace('"', "");
            let script = format!(
                "$p=[Environment]::GetEnvironmentVariable('Path','User'); if($p -notlike '*{0}*'){{[Environment]::SetEnvironmentVariable('Path',$p+';{0}','User')}}",
                bin
            );
            let _ = std::process::Command::new("powershell")
                .args(["-NoProfile", "-Command", &script])
                .output();
        }
    }
}
