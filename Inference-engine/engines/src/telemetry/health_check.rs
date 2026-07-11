use crate::hardware::{SiliconTruth, StorageSubsystem};
use sysinfo::System;

pub struct BitShitHealthChecker;

/// Source compatibility for extensions compiled against the previous public
/// type name. New code must use `BitShitHealthChecker`.
#[allow(non_camel_case_types)]
pub type cluaizHealthChecker = BitShitHealthChecker;

impl BitShitHealthChecker {
    pub fn execute_initial_diagnostic(mut profile: SiliconTruth) -> SiliconTruth {
        cluaiz_shared::dev_info!(
            "🩺 [1BitShit Health] Starting hardware profiling..."
        );

        let mut system = System::new();
        system.refresh_memory();
        let total_ram_gb = system.total_memory() as f64 / 1_073_741_824.0;
        profile.memory.total_capacity_gb = total_ram_gb;
        cluaiz_shared::dev_info!("📊 [Memory] {:.1} GB detected", total_ram_gb);

        let storage_speed = Self::estimate_disk_io();
        let is_nvme = storage_speed > 1000.0;
        profile.storage = vec![StorageSubsystem {
            mount_point: "/".to_string(),
            drive_type: if is_nvme {
                "NVMe (High-Performance)".into()
            } else {
                "SSD".into()
            },
            read_speed_mbps: storage_speed,
            total_gb: 512.0,
            free_gb: 256.0,
            is_primary_workspace: true,
            ..Default::default()
        }];

        cluaiz_shared::dev_info!(
            "💾 [Storage] {:.1} MB/s, {}",
            storage_speed,
            if is_nvme { "NVMe" } else { "SSD" }
        );
        profile
    }

    fn estimate_disk_io() -> f64 {
        let path = cluaiz_shared::environment::EnvironmentManager::current()
            .local_dir
            .join(".bitshit_boot_bench.tmp");
        let payload = vec![0u8; 5 * 1024 * 1024];
        let start = std::time::Instant::now();
        if let Ok(mut file) = std::fs::File::create(&path) {
            use std::io::Write;
            if file.write_all(&payload).is_ok() {
                let _ = file.sync_all();
            }
        }
        let _ = std::fs::read(&path);
        let duration = start.elapsed().as_secs_f64();
        let _ = std::fs::remove_file(&path);
        if duration > 0.0 { 10.0 / duration } else { 0.0 }
    }

    pub fn run_full_benchmark() {
        cluaiz_shared::dev_info!(
            "🚀 [1BitShit Benchmark] Starting hardware diagnostics..."
        );
        let start = std::time::Instant::now();
        let path = cluaiz_shared::environment::EnvironmentManager::current()
            .local_dir
            .join(".bitshit_io_bench.tmp");
        let payload = vec![0u8; 50 * 1024 * 1024];

        if let Ok(mut file) = std::fs::File::create(&path) {
            use std::io::Write;
            if file.write_all(&payload).is_ok() {
                let _ = file.sync_all();
            }
        }
        let bytes_read = std::fs::read(&path)
            .map(|data| data.len())
            .unwrap_or_default();
        let duration = start.elapsed();
        let speed_mbps = if duration.as_secs_f64() > 0.0 {
            100.0 / duration.as_secs_f64()
        } else {
            0.0
        };
        let _ = std::fs::remove_file(&path);

        cluaiz_shared::dev_info!(
            "✅ [1BitShit Benchmark] Read {} bytes at {:.1} MB/s in {:.2}s",
            bytes_read,
            speed_mbps,
            duration.as_secs_f64()
        );
    }
}
