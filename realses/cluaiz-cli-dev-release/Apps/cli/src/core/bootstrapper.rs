use cluaiz_shared::HardwareGovernor;
use color_eyre::{eyre::eyre, Result};
use colored::Colorize;
use std::path::Path;
use std::process::Command;

pub struct Bootstrapper;

impl Bootstrapper {
    /// 🚀 Cluaiz BOOTSTRAP: Ensures the Core Engine is present and initialized.
    pub async fn ignite() -> Result<()> {
        #[cfg(windows)]
        let _ = colored::control::set_virtual_terminal(true);

        let engine_dir = HardwareGovernor::resolve_engine_path();

        let ext = if cfg!(windows) {
            "dll"
        } else if cfg!(target_os = "macos") {
            "dylib"
        } else {
            "so"
        };
        let engine_name = format!("cluaiz-engine.{}", ext);
        let engine_path = engine_dir.join(engine_name);

        if !engine_path.exists() {
            println!(
                "  {} [Cluaiz] Core Engine missing. Please run the Installer.",
                "🛠️".red()
            );
            return Err(eyre!("Core Engine not found."));
        }

        // --- FULL STACK SYNC: Kernels & Drivers ---
        Self::sync_neural_stack().await?;

        // Verify if system_control.bin exists in Hub
        let bin_truth = HardwareGovernor::resolve_interface_path().join("system_control.bin");
        if !bin_truth.exists() {
            // Background calibration happens here
        }

        Ok(())
    }

    async fn download_engine(dest: &Path) -> Result<()> {
        let url = Self::resolve_engine_url()?;

        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent)?;
        }

        println!(
            "  {} [Cluaiz] Downloading Engine from: {}",
            "📥".cyan(),
            url
        );

        let client = reqwest::Client::builder()
            .user_agent("Cluaiz-Bootstrapper/0.1 (Windows; x64)")
            .danger_accept_invalid_certs(true)
            .build()?;

        let response = client
            .get(&url)
            .send()
            .await
            .map_err(|e| eyre!("Failed to connect to Cluaiz Registry: {}", e))?;

        if !response.status().is_success() {
            return Err(eyre!(
                "Registry Error: Server returned status {}",
                response.status()
            ));
        }

        let content = response.bytes().await?;
        std::fs::write(dest, content)?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = std::fs::metadata(dest)?.permissions();
            perms.set_mode(0o755);
            std::fs::set_permissions(dest, perms)?;
        }

        println!(
            "  {} [Cluaiz] Engine binary mounted and sealed.",
            "✅".green()
        );
        Ok(())
    }

    fn trigger_setup(engine_path: &Path) -> Result<()> {
        println!("  {} [Cluaiz] Initializing hardware...", "⚙️".yellow());

        let status = Command::new(engine_path)
            .arg("--setup")
            .status()
            .map_err(|e| eyre!("Failed to execute engine setup: {}", e))?;

        if !status.success() {
            return Err(eyre!("Engine setup failed with status: {}", status));
        }

        Ok(())
    }

    async fn sync_neural_stack() -> Result<()> {
        let control_path = HardwareGovernor::resolve_engine_path().join("system_control.json");

        if !control_path.exists() {
            return Ok(()); // Wait for first calibration
        }

        let control_data: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&control_path)?)?;
        let has_nvidia = control_data["silicon_truth"]["accelerators"]["gpus"]
            .as_array()
            .map(|gpus| {
                gpus.iter().any(|g| {
                    g["vendor"]
                        .as_str()
                        .map(|v| v.to_uppercase())
                        .unwrap_or_default()
                        .contains("NVIDIA")
                })
            })
            .unwrap_or(false);

        let backend = if has_nvidia { "cuda" } else { "cpu" };
        let platform = if cfg!(windows) {
            "win-x64"
        } else if cfg!(target_os = "macos") {
            "mac-arm64"
        } else {
            "linux-x64"
        };

        // 1. Kernel Sync
        let kernel_dir = HardwareGovernor::resolve_hub_path().join("interface-engines/kernels");
        let kernel_ext = if cfg!(windows) {
            "dll"
        } else if cfg!(target_os = "macos") {
            "dylib"
        } else {
            "so"
        };

        // We support both cluaiz-llama (new) and archer_llama (legacy) locally
        let kernel_name = format!("cluaiz-llama.{}", kernel_ext);
        let kernel_path = kernel_dir.join(&kernel_name);

        let has_cluaiz_kernel = kernel_path.exists();
        let legacy_kernel_name = if cfg!(windows) {
            "archer_llama.dll"
        } else if cfg!(target_os = "macos") {
            "libarcher_llama.dylib"
        } else {
            "libarcher_llama.so"
        };
        let legacy_kernel_path = kernel_dir.join(legacy_kernel_name);
        let has_legacy_kernel = legacy_kernel_path.exists();

        let unix_cluaiz_name = format!("libcluaiz_llama.{}", kernel_ext);
        let unix_cluaiz_path = kernel_dir.join(&unix_cluaiz_name);
        let has_unix_cluaiz = unix_cluaiz_path.exists();

        if !has_cluaiz_kernel && !has_legacy_kernel && !has_unix_cluaiz {
            println!(
                "  {} [Cluaiz] Downloading kernel ({})...",
                "📦".magenta(),
                backend
            );

            let version = cluaiz_shared::CluaizDNA::KERNEL;
            let tag_name = if version == "dev-release" || version == "v1.0.0" {
                "kernel-dev-release".to_string()
            } else {
                version.to_string()
            };

            // Resolve optimized SIMD platform dynamically from Hardware Truth
            let mut spec_platform = platform.to_string();
            if platform == "win-x64" || platform == "linux-x64" {
                let isa_features = control_data["silicon_truth"]["cpu"]["isa_features"].as_array();
                let has_avx512 = isa_features
                    .map(|feats| feats.iter().any(|f| f.as_str() == Some("AVX-512")))
                    .unwrap_or(false);
                if has_avx512 {
                    spec_platform = format!("{}-avx512", platform);
                } else {
                    spec_platform = format!("{}-avx2", platform);
                }
            }

            // GHA artifact name format is: cluaiz-kernel-{version}-{platform}.{ext}
            let download_name =
                format!("cluaiz-kernel-{}-{}.{}", version, spec_platform, kernel_ext);
            let url = format!(
                "https://github.com/cluaiz/cluaiz/releases/download/{}/{}",
                tag_name, download_name
            );

            // Attempt to download. If download succeeds, save to kernel_path
            if let Err(_e) = Self::download_asset(&url, &kernel_path).await {
                println!(
                    "  {} [Cluaiz] Primary download failed, attempting tag fallback...",
                    "⚠️".yellow()
                );
                let fallback_url = format!(
                    "https://github.com/cluaiz/cluaiz/releases/download/kernel-dev-release/cluaiz-kernel-dev-release-{}.{}",
                    spec_platform, kernel_ext
                );
                if let Err(err) = Self::download_asset(&fallback_url, &kernel_path).await {
                    return Err(color_eyre::eyre::eyre!(
                        "Registry Error: Primary URL ({}) and Fallback URL ({}) both failed.\nDetail: {}",
                        url, fallback_url, err
                    ));
                }
            }
        }

        // 2. Driver Sync (If needed)
        if has_nvidia {
            if let Err(e) = engines::interface_engines::manager::driver_provisioner::DriverProvisioner::provision_for_hardware("cuda").await {
                println!("  {} [Cluaiz] Driver deployment failed: {}. Continuing bootstrap...", "⚠️".yellow(), e);
            }
        }

        Ok(())
    }

    async fn download_asset(url: &str, dest: &Path) -> Result<()> {
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let client = reqwest::Client::builder()
            .user_agent("Cluaiz-Bootstrapper/0.1 (Windows; x64)")
            .danger_accept_invalid_certs(true)
            .build()?;

        let response = client
            .get(url)
            .send()
            .await
            .map_err(|e| eyre!("Failed to connect to Cluaiz Registry: {}", e))?;

        if !response.status().is_success() {
            return Err(eyre!(
                "Registry Error: {} returned {}",
                url,
                response.status()
            ));
        }

        let content = response.bytes().await?;
        std::fs::write(dest, content)?;
        Ok(())
    }

    fn resolve_engine_url() -> Result<String> {
        let version = cluaiz_shared::CluaizDNA::ENGINE;
        let base = format!(
            "https://github.com/cluaiz/cluaiz/releases/download/{}",
            version
        );

        #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
        return Ok(format!("{}/cluaiz-engine-{}-win-x64.dll", base, version));

        #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
        return Ok(format!("{}/cluaiz-engine-{}-linux-x64.so", base, version));

        #[cfg(all(target_os = "linux", target_arch = "aarch64"))]
        return Ok(format!("{}/cluaiz-engine-{}-linux-arm64.so", base, version));

        #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
        return Ok(format!(
            "{}/cluaiz-engine-{}-mac-arm64.dylib",
            base, version
        ));

        Err(eyre!(
            "Cluaiz Registry: Platform not supported in this build."
        ))
    }
}
