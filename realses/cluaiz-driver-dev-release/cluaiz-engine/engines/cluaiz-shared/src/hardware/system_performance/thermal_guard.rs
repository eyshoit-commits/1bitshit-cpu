use super::drivers::HardwareDriver;
use anyhow::Result;

pub struct ThermalGuard {
    critical_temp: f32,
    safe_temp: f32,
    is_throttled: bool,
}

impl ThermalGuard {
    pub fn new(critical_temp: f32, safe_temp: f32) -> Self {
        Self {
            critical_temp,
            safe_temp,
            is_throttled: false,
        }
    }

    pub fn tick(&mut self, driver: &dyn HardwareDriver) -> Result<()> {
        let current_temp = driver.temperature_c()?;
        
        if current_temp > self.critical_temp && !self.is_throttled {
            tracing::warn!("🔥 [ThermalGuard] CRITICAL TEMP DETECTED ({}°C). Throttling hardware...", current_temp);
            driver.set_power_limit_watts(100)?; // Emergency Low Power
            self.is_throttled = true;
        } else if current_temp < self.safe_temp && self.is_throttled {
            tracing::info!("🟢 [ThermalGuard] Temperature stabilized ({}°C). Releasing throttle.", current_temp);
            // Defaulting to balanced power (will be handled by governor later)
            driver.set_power_limit_watts(250)?; 
            self.is_throttled = false;
        }
        
        Ok(())
    }
}
