use super::HardwareDriver;
use anyhow::Result;
use sysinfo::System;

pub struct CpuGenericDriver {
    sys: System,
}

impl CpuGenericDriver {
    pub fn init() -> Self {
        let mut sys = System::new_all();
        sys.refresh_all();
        Self { sys }
    }
}

impl HardwareDriver for CpuGenericDriver {
    fn name(&self) -> &str {
        "Generic CPU (sysinfo)"
    }

    fn temperature_c(&self) -> Result<f32> {
        let mut cpu_temp = 0.0;
        let components = sysinfo::Components::new_with_refreshed_list();
        for comp in &components {
            let label = comp.label().to_lowercase();
            if (label.contains("cpu")
                || label.contains("core")
                || label.contains("package")
                || label.contains("k10temp")
                || label.contains("tctl"))
                && comp.temperature() > cpu_temp
            {
                cpu_temp = comp.temperature();
            }
        }
        Ok(cpu_temp)
    }

    fn utilization_pct(&self) -> Result<f32> {
        let cpus = self.sys.cpus();
        if !cpus.is_empty() {
            Ok(cpus.iter().map(|c| c.cpu_usage()).sum::<f32>() / cpus.len() as f32)
        } else {
            Ok(0.0)
        }
    }

    fn clock_mhz(&self) -> Result<u32> {
        let cpus = self.sys.cpus();
        if !cpus.is_empty() {
            Ok((cpus.iter().map(|c| c.frequency()).sum::<u64>() / cpus.len() as u64) as u32)
        } else {
            Ok(0)
        }
    }

    fn vram_used_mb(&self) -> Result<u64> {
        Ok(self.sys.used_memory() / 1024 / 1024) // System RAM fallback
    }

    fn vram_total_mb(&self) -> Result<u64> {
        Ok(self.sys.total_memory() / 1024 / 1024)
    }

    fn power_draw_watts(&self) -> Result<f32> {
        Ok(0.0) // sysinfo doesn't easily provide CPU wattage
    }
}
