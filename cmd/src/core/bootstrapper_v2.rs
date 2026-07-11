use std::path::{Path, PathBuf};

use cluaiz_shared::environment::{EnvironmentManager, EnvironmentMode};
use cluaiz_shared::HardwareGovernor;
use color_eyre::{eyre::eyre, Result};
use colored::Colorize;

pub struct Bootstrapper;

impl Bootstrapper {
    const DEFAULT_REGISTRY_URL: &'static str =
        "https://raw.githubusercontent.com/eyshoit-commits/1bitshit-cpu/main/package.json";
    const EMBEDDED_REGISTRY: &'static str = include_str!("../../../package.json");
    const PRODUCT: &'static str = "1BitShit CPU";

    pub async fn ignite(is_dev_sync: bool) -> Result<()> {
        let environment = EnvironmentManager::current();
        Self::migrate_legacy_runtime(&environment)?;

        let profile = if cfg!(debug_assertions) { "debug" } else { "release" };
        let _ = Self::sync_dev_artifacts(
            "all",
            None,
            environment.global_dir.clone(),
            profile,
        );
        Self::ensure_global_path();

        let mut permissions =
            engines::neural_foundry::security::permission_schema::PermissionSchema::load();
        permissions.auto_assign_defaults();

        let mut skill_registry = engines::neural_foundry::registry::SkillRegistry::new();
        for directory in [
            environment.skills_dir(),
            environment.extensions_dir(),
            environment.plugins_dir(),
            environment.mcp_dir(),
        ] {
            if directory.exists() {
                skill_registry.load_from_directory(&directory.to_string_lossy());
            }
        }

        let hub_path = HardwareGovernor::resolve_hub_path();
        std::fs::create_dir_all(&hub_path)?;
        std::fs::write(
            hub_path.join("THIRD_PARTY_LICENSES.txt"),
            include_str!("../assets/THIRD_PARTY_NOTICES.txt"),
        )?;

        #[cfg(windows)]
        let _ = colored::control::set_virtual_terminal(true);

        if is_dev_sync {
            tracing::info!("[1BitShit CPU] Development artifacts synchronized");
            return Ok(());
        }

        let client = reqwest::Client::builder()
            .user_agent(format!("1bitshit-cpu/{}", env!("CARGO_PKG_VERSION")))
            .build()?;
        let registry = Self::load_registry(&client).await?;

        cluaiz_shared::RegistryGovernor::seal_registry(registry.clone())
            .map_err(|error| eyre!("Registry seal failed: {error}"))?;

        let current_version = env!("CARGO_PKG_VERSION");
        let latest_version = registry["components"]["cli"]["version"]
            .as_str()
            .unwrap_or(current_version);
        if Self::is_newer_version(latest_version, current_version) {
            println!(
                "  {} [{}] Update available: {} -> {}",
                "🚀".green(),
                Self::PRODUCT,
                current_version,
                latest_version
            );
        }

        Self::sync_engine(&client, &registry).await?;
        Self::sync_neural_stack(&client, &registry).await?;
        Ok(())
    }

    async fn load_registry(client: &reqwest::Client) -> Result<serde_json::Value> {
        let embedded: serde_json::Value = serde_json::from_str(Self::EMBEDDED_REGISTRY)
            .map_err(|error| eyre!("Embedded registry is invalid: {error}"))?;

        let offline = std::env::var("BITSHIT_OFFLINE")
            .map(|value| matches!(value.to_ascii_lowercase().as_str(), "1" | "true" | "yes"))
            .unwrap_or(false);
        if offline {
            tracing::info!("[1BitShit CPU] Offline mode uses embedded registry");
            return Ok(embedded);
        }

        let registry_url = std::env::var("BITSHIT_REGISTRY_URL")
            .unwrap_or_else(|_| Self::DEFAULT_REGISTRY_URL.to_string());
        let online = async {
            let response = client.get(&registry_url).send().await?;
            if !response.status().is_success() {
                return Err(eyre!("Registry returned HTTP {}", response.status()));
            }
            let value: serde_json::Value = response.json().await?;
            Ok::<serde_json::Value, color_eyre::Report>(value)
        }
        .await;

        match online {
            Ok(value) if Self::is_bitshit_registry(&value) => Ok(value),
            Ok(_) => {
                println!(
                    "  {} [{}] Online registry belongs to another runtime. Using embedded registry.",
                    "⚠️".yellow(),
                    Self::PRODUCT
                );
                Ok(embedded)
            }
            Err(error) => {
                println!(
                    "  {} [{}] Registry unavailable: {}. Using embedded registry.",
                    "⚠️".yellow(),
                    Self::PRODUCT,
                    error
                );
                Ok(embedded)
            }
        }
    }

    fn is_bitshit_registry(value: &serde_json::Value) -> bool {
        value["name"].as_str() == Some("1bitshit-cpu")
            && value["components"]["cli"]["version"].is_string()
            && value["components"]["engine"]["version"].is_string()
            && value["components"]["kernel"]["version"].is_string()
    }

    fn is_newer_version(latest: &str, current: &str) -> bool {
        fn numeric(value: &str) -> Option<Vec<u64>> {
            let value = value.trim().trim_start_matches('v');
            let value = value.split('-').next().unwrap_or(value);
            let parts: Vec<u64> = value
                .split('.')
                .map(str::parse::<u64>)
                .collect::<std::result::Result<_, _>>()
                .ok()?;
            if parts.is_empty() { None } else { Some(parts) }
        }
        matches!((numeric(latest), numeric(current)), (Some(left), Some(right)) if left > right)
    }

    async fn sync_engine(client: &reqwest::Client, registry: &serde_json::Value) -> Result<()> {
        let information = &registry["components"]["engine"];
        if information.is_null() {
            return Ok(());
        }

        let engine_directory = HardwareGovernor::resolve_engine_path();
        std::fs::create_dir_all(&engine_directory)?;
        let extension = Self::dynamic_library_extension();
        let engine_path = engine_directory.join(format!("bitshit-engine.{extension}"));
        let marker_path = engine_directory.join("bitshit-engine.ready");
        Self::import_legacy_artifact(
            &engine_directory.join(format!("cluaiz-engine.{extension}")),
            &engine_path,
        )?;

        let version = information["version"].as_str().unwrap_or("unknown");
        let local_version = std::fs::read_to_string(&marker_path).unwrap_or_default();
        if engine_path.is_file() && local_version.trim() == version {
            return Ok(());
        }

        let Some(manifest_url) = information["manifest_url"].as_str() else {
            return if engine_path.is_file() {
                Ok(())
            } else {
                Err(eyre!("Engine manifest URL is missing"))
            };
        };

        let result = async {
            let response = client.get(manifest_url).send().await?;
            if !response.status().is_success() {
                return Err(eyre!("Engine registry returned HTTP {}", response.status()));
            }
            let manifest: serde_json::Value = response.json().await?;
            let platform = Self::platform_key();
            let url = manifest["engines"][platform]
                .as_str()
                .ok_or_else(|| eyre!("No engine binary for '{platform}'"))?;
            Self::download_asset(client, url, &engine_path).await?;
            std::fs::write(&marker_path, version)?;
            Ok::<(), color_eyre::Report>(())
        }
        .await;

        match result {
            Ok(()) => Ok(()),
            Err(error) if engine_path.is_file() => {
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

    async fn sync_neural_stack(
        client: &reqwest::Client,
        registry: &serde_json::Value,
    ) -> Result<()> {
        let information = &registry["components"]["kernel"];
        if information.is_null() {
            return Ok(());
        }

        let engine_directory = HardwareGovernor::resolve_interface_path();
        std::fs::create_dir_all(&engine_directory)?;
        let extension = Self::dynamic_library_extension();
        let llama_path = engine_directory.join(format!("bitshit-llama.{extension}"));
        let marker_path = engine_directory.join("bitshit-llama.ready");
        Self::import_legacy_artifact(
            &engine_directory.join(format!("cluaiz-llama.{extension}")),
            &llama_path,
        )?;

        let version = information["version"].as_str().unwrap_or("unknown");
        let local_version = std::fs::read_to_string(&marker_path).unwrap_or_default();
        if !llama_path.is_file() || local_version.trim() != version {
            if let Some(manifest_url) = information["manifest_url"].as_str() {
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
                    Self::download_asset(client, url, &llama_path).await?;
                    std::fs::write(&marker_path, version)?;
                    Ok::<(), color_eyre::Report>(())
                }
                .await;

                if let Err(error) = result {
                    if !llama_path.is_file() {
                        return Err(error);
                    }
                    println!(
                        "  {} [{}] Llama update failed: {}. Using local kernel.",
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
            let driver_manifest = registry["components"]["drivers"]["manifest_url"]
                .as_str()
                .unwrap_or_default();
            let _ = engines::interface_engines::manager::driver_provisioner::DriverProvisioner::provision_for_hardware(
                "cuda",
                driver_manifest,
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
                .and_then(|extension| extension.to_str())
                .unwrap_or("download")
        ));
        std::fs::write(&temporary, response.bytes().await?)?;
        if destination.exists() {
            std::fs::remove_file(destination)?;
        }
        std::fs::rename(temporary, destination)?;
        Ok(())
    }

    pub fn sync_dev_artifacts(
        target: &str,
        driver_name: Option<&str>,
        hub_path: PathBuf,
        profile: &str,
    ) -> Result<()> {
        let root = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let extension = Self::dynamic_library_extension();
        let target_directory = root
            .join("target")
            .join(if profile == "release" { "release" } else { "debug" });

        let find_artifact = |base_names: &[&str]| -> Option<PathBuf> {
            for base in base_names {
                for file_name in [
                    format!("{base}.{extension}"),
                    format!("lib{base}.{extension}"),
                ] {
                    for candidate in [
                        target_directory.join(&file_name),
                        target_directory.join("deps").join(&file_name),
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
            if let Some(source) = find_artifact(&["engines"]) {
                let destination = hub_path
                    .join("engine")
                    .join(format!("bitshit-engine.{extension}"));
                Self::copy_artifact(&source, &destination)?;
                std::fs::write(
                    hub_path.join("engine").join("bitshit-engine.ready"),
                    env!("CARGO_PKG_VERSION"),
                )?;
            }
        }

        let libraries: Vec<(Vec<&str>, &str, bool)> = vec![
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
            for (source_names, destination_name, runtime_dependency) in libraries {
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
                let destination = if runtime_dependency {
                    hub_path
                        .join("engine")
                        .join("drivers")
                        .join(format!("{destination_name}.{extension}"))
                } else {
                    hub_path
                        .join("engine")
                        .join(format!("{destination_name}.{extension}"))
                };
                Self::copy_artifact(&source, &destination)?;
                if !runtime_dependency {
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
            let source = target_directory.join(executable);
            if source.is_file() {
                Self::copy_artifact(&source, &hub_path.join("bin").join(executable))?;
            }

            let new_runtime = root.join(".1bitshit");
            let legacy_runtime = root.join(".cluaiz");
            let source_runtime = if new_runtime.is_dir() {
                Some(new_runtime)
            } else if legacy_runtime.is_dir() {
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

    fn migrate_legacy_runtime(environment: &EnvironmentManager) -> Result<()> {
        if environment.mode != EnvironmentMode::Installed {
            return Ok(());
        }
        let Some(home) = dirs::home_dir() else {
            return Ok(());
        };
        let legacy = home.join(".cluaiz");
        let destination = environment.global_dir.clone();
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
            "Legacy data imported by 1BitShit CPU. The previous runtime was not executed or deleted.\n",
        )?;
        println!(
            "  {} [{}] Legacy data imported into {}.",
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
                .and_then(|extension| extension.to_str())
                .unwrap_or("copy")
        ));
        std::fs::copy(source, &temporary)?;
        if destination.exists() {
            std::fs::remove_file(destination)?;
        }
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
        let avx512 = HardwareGovernor::load_system_control()
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
        format!("{}-{}", platform, if avx512 { "avx512" } else { "avx2" })
    }

    fn ensure_global_path() {
        #[cfg(windows)]
        {
            let directory = HardwareGovernor::resolve_bin_gateway();
            let directory = directory.to_string_lossy().replace('"', "");
            let script = format!(
                "$p=[Environment]::GetEnvironmentVariable('Path','User'); if($p -notlike '*{0}*'){{[Environment]::SetEnvironmentVariable('Path',$p+';{0}','User')}}",
                directory
            );
            let _ = std::process::Command::new("powershell")
                .args(["-NoProfile", "-Command", &script])
                .output();
        }
    }
}
