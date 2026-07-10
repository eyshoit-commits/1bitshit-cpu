//! BARE METAL: OS Tuning
//! Handles process priority and scheduling optimizations.

/// Elevate process priority to ensure the AI Engine gets priority CPU scheduling
pub fn elevate_process_priority() -> Result<(), &'static str> {
    let pid = std::process::id();
    tracing::info!("🚀 [Bare Metal] Attempting to elevate process priority for PID {}", pid);

    if cfg!(target_os = "windows") {
        let output = std::process::Command::new("wmic")
            .args(["process", "where", &format!("processid={}", pid), "call", "setpriority", "128"])
            .output()
            .map_err(|_| "Failed to execute wmic")?;

        if output.status.success() {
            tracing::info!("🚀 [Bare Metal] Windows priority elevated to HIGH.");
            Ok(())
        } else {
            tracing::warn!("⚠️ [Bare Metal] Windows elevation rejected.");
            Err("Priority elevation rejected by OS")
        }
    } else if cfg!(target_os = "linux") {
        let output = std::process::Command::new("renice")
            .args(["-n", "-20", "-p", &pid.to_string()])
            .output()
            .map_err(|_| "Failed to execute renice")?;

        if output.status.success() {
            tracing::info!("🚀 [Bare Metal] Linux priority elevated via renice.");
            Ok(())
        } else {
            tracing::warn!("⚠️ [Bare Metal] Linux elevation rejected.");
            Err("Priority elevation rejected by renice")
        }
    } else {
        tracing::warn!("⚠️ [Bare Metal] Process elevation not supported on this OS.");
        Err("Process elevation not supported on this architecture")
    }
}

