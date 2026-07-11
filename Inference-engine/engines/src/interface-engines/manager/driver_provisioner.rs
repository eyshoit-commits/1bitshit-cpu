use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, Result};
use cluaiz_shared::HardwareGovernor;

pub struct DriverProvisioner;

impl DriverProvisioner {
    fn get_registry_key(driver_type: &str) -> String {
        let platform = if cfg!(windows) {
            "win-x64"
        } else if cfg!(target_os = "macos") {
            "mac-arm64"
        } else if cfg!(target_os = "android") {
            "android-arm64"
        } else {
            "linux-x64"
        };

        match driver_type {
            "cuda" => format!("{platform}-cuda-12"),
            "rocm" | "hip" => format!("{platform}-{driver_type}"),
            "vulkan" => format!("{platform}-vulkan"),
            "openvino" => format!("{platform}-openvino"),
            "cann" => format!("{platform}-cann"),
            "qnn" => format!("{platform}-qnn"),
            "metal" => format!("{platform}-metal"),
            _ => format!("{platform}-{driver_type}"),
        }
    }

    pub async fn provision_kernel(
        kernel_type: &str,
        backend: &str,
        manifest_url: &str,
    ) -> Result<PathBuf> {
        let kernel_dir = HardwareGovernor::resolve_interface_path();
        fs::create_dir_all(&kernel_dir)?;

        let registry_key = Self::get_registry_key(backend);
        let marker = kernel_dir.join(format!("bitshit-{kernel_type}.ready"));
        let extension = dynamic_library_extension();
        let destination = kernel_dir.join(format!("bitshit-{kernel_type}.{extension}"));

        // Preserve locally compiled accelerated kernels. A large binary generally
        // contains the CUDA/ROCm backend and must not be replaced by a CPU artifact.
        if destination.is_file() {
            let size_mb = destination.metadata().map(|metadata| metadata.len()).unwrap_or(0)
                / (1024 * 1024);
            if size_mb > 30 {
                tracing::info!(
                    "[1BitShit Provisioner] Preserving local accelerated kernel ({} MB)",
                    size_mb
                );
                return Ok(destination);
            }
        }

        let client = reqwest::Client::builder()
            .user_agent(format!("1bitshit-cpu/{}", env!("CARGO_PKG_VERSION")))
            .build()?;
        let manifest = fetch_manifest(&client, manifest_url).await?;
        let manifest_version = manifest["version"].as_str().unwrap_or("unknown");

        if marker.is_file()
            && destination.is_file()
            && fs::read_to_string(&marker).unwrap_or_default().trim() == manifest_version
        {
            return Ok(destination);
        }

        let download_url = manifest["kernel"][kernel_type][&registry_key]
            .as_str()
            .ok_or_else(|| {
                anyhow!(
                    "Kernel '{}' for registry key '{}' was not found",
                    kernel_type,
                    registry_key
                )
            })?;

        download_atomic(&client, download_url, &destination).await?;
        fs::write(marker, manifest_version)?;
        tracing::info!(
            "[1BitShit Provisioner] Kernel '{}' installed at {}",
            kernel_type,
            destination.display()
        );
        Ok(destination)
    }

    pub async fn provision_for_hardware(driver_type: &str, manifest_url: &str) -> Result<()> {
        if manifest_url.trim().is_empty() {
            return Ok(());
        }

        let driver_dir = HardwareGovernor::resolve_interface_path().join("drivers");
        fs::create_dir_all(&driver_dir)?;

        let client = reqwest::Client::builder()
            .user_agent(format!("1bitshit-cpu/{}", env!("CARGO_PKG_VERSION")))
            .build()?;
        let manifest = fetch_manifest(&client, manifest_url).await?;
        let manifest_version = manifest["version"].as_str().unwrap_or("unknown");
        let marker = driver_dir.join(format!("bitshit-{driver_type}.ready"));

        if marker.is_file()
            && fs::read_to_string(&marker).unwrap_or_default().trim() == manifest_version
        {
            return Ok(());
        }

        let registry_key = Self::get_registry_key(driver_type);
        let download_url = manifest["drivers"][&registry_key]
            .as_str()
            .ok_or_else(|| anyhow!("Driver key '{}' was not found", registry_key))?;
        let file_name = download_url
            .split('/')
            .next_back()
            .filter(|name| !name.is_empty())
            .unwrap_or("driver.bin");
        let destination = driver_dir.join(file_name);

        if file_name.ends_with(".zip") {
            let response = client.get(download_url).send().await?;
            if !response.status().is_success() {
                return Err(anyhow!(
                    "Driver download returned HTTP {}",
                    response.status()
                ));
            }
            let bytes = response.bytes().await?;
            extract_zip(&bytes, &driver_dir)?;
        } else {
            download_atomic(&client, download_url, &destination).await?;
        }

        fs::write(marker, manifest_version)?;
        tracing::info!(
            "[1BitShit Provisioner] Driver '{}' installed",
            driver_type
        );
        Ok(())
    }

    pub fn discover_system_paths() -> Vec<PathBuf> {
        let mut paths = vec![Self::get_driver_path()];
        #[cfg(target_os = "windows")]
        if let Ok(cuda_path) = std::env::var("CUDA_PATH") {
            let binary_path = PathBuf::from(cuda_path).join("bin");
            if binary_path.exists() {
                paths.push(binary_path);
            }
        }
        paths
    }

    pub fn get_driver_path() -> PathBuf {
        HardwareGovernor::resolve_interface_path().join("drivers")
    }
}

async fn fetch_manifest(
    client: &reqwest::Client,
    manifest_url: &str,
) -> Result<serde_json::Value> {
    let response = client.get(manifest_url).send().await?;
    if !response.status().is_success() {
        return Err(anyhow!(
            "Driver registry returned HTTP {}",
            response.status()
        ));
    }
    let text = response.text().await?;
    let normalized = text
        .lines()
        .filter(|line| !line.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n");
    Ok(serde_json::from_str(&normalized)?)
}

async fn download_atomic(
    client: &reqwest::Client,
    url: &str,
    destination: &Path,
) -> Result<()> {
    let response = client.get(url).send().await?;
    if !response.status().is_success() {
        return Err(anyhow!("Download returned HTTP {}", response.status()));
    }
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent)?;
    }
    let partial = destination.with_extension(format!(
        "{}.part",
        destination
            .extension()
            .and_then(|extension| extension.to_str())
            .unwrap_or("download")
    ));
    fs::write(&partial, response.bytes().await?)?;
    fs::rename(partial, destination)?;
    Ok(())
}

fn extract_zip(bytes: &[u8], destination: &Path) -> Result<()> {
    let cursor = std::io::Cursor::new(bytes);
    let mut archive = zip::ZipArchive::new(cursor)
        .map_err(|error| anyhow!("ZIP extraction failed: {error}"))?;
    for index in 0..archive.len() {
        let mut file = archive.by_index(index)?;
        let Some(relative_path) = file.enclosed_name() else {
            continue;
        };
        let output = destination.join(relative_path);
        if file.is_dir() {
            fs::create_dir_all(&output)?;
        } else {
            if let Some(parent) = output.parent() {
                fs::create_dir_all(parent)?;
            }
            let mut output_file = fs::File::create(output)?;
            std::io::copy(&mut file, &mut output_file)?;
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
