use color_eyre::Result;
use colored::Colorize;
use sysinfo::System;
use cluaiz_shared::hardware::governor::HardwareGovernor;

pub async fn execute() -> Result<()> {
    println!(
        "\n  {} [1BitShit CPU] Auditing active model processes...",
        "🔍".cyan()
    );

    let mut registry = HardwareGovernor::load_process_registry();
    let mut system = System::new_all();
    system.refresh_all();

    let mut stale = Vec::new();
    let mut active = Vec::new();
    for (pid_text, process) in &registry {
        match pid_text.parse::<usize>() {
            Ok(pid) if system.process(sysinfo::Pid::from(pid)).is_some() => {
                active.push(process.clone());
            }
            _ => stale.push(pid_text.clone()),
        }
    }

    for pid in stale {
        registry.remove(&pid);
    }
    HardwareGovernor::save_process_registry(&registry);

    if active.is_empty() {
        println!("  {} No active model engines are running.", "💤".yellow());
        return Ok(());
    }

    println!(
        "\n  {0:<28} | {1:<8} | {2:<12} | {3:<10} | {4:<14}",
        "MODEL ID".bold(),
        "PID".bold(),
        "VRAM".bold(),
        "CONTEXT".bold(),
        "ENGINE".bold()
    );
    println!("  {}", "-".repeat(84).dimmed());
    for process in active {
        println!(
            "  {0:<28} | {1:<8} | {2:<12} | {3:<10} | {4:<14}",
            process.model_id.cyan(),
            process.pid.to_string().yellow(),
            format!("{:.2} GB", process.vram_gb).magenta(),
            process.context_size.to_string().green(),
            process.engine
        );
    }
    println!();
    Ok(())
}
