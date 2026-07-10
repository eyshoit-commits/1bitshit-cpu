//! 🔬 Archer Calibration Tool
//! Surgical probe of local Hardware to generate 'system_control.json'.

use cluaiz_shared::HardwareGovernor;

fn main() -> anyhow::Result<()> {
    println!("⚔️  [CALIBRATE] Starting deep Hardware probe...");

    match HardwareGovernor::auto_calibrate() {
        Ok(_) => {
            println!("✅ [CALIBRATE] Hardware Truth & Performance Booster Synchronized.");
            Ok(())
        }
        Err(e) => {
            eprintln!("❌ [CALIBRATE] Probe FAILED: {}", e);
            Err(e)
        }
    }
}
