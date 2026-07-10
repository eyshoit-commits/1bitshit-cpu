use clap::{Parser, Subcommand};
use color_eyre::Result;
use colored::Colorize;
use engines::DownloadEvent;
use std::fs::File;
use tokio::spawn;

mod app_enums;
mod assets;
mod cli;
mod core;
mod theme;
mod ui;

use crate::core::app::App;

// ── Cluaiz CLI Definition ──────────────────────────────────────────────────

#[derive(Parser)]
#[command(
    name = "cluaiz",
    about = "Cluaiz CLI",
    version = "0.1.0",
    disable_help_subcommand = true
)]
struct Cli {
    #[command(subcommand)]
    command: Option<CliCommand>,

    #[arg(long, hide = true)]
    benchmark: bool,

    #[arg(long, hide = true)]
    calibrate: bool,
}

#[derive(Subcommand)]
enum CliCommand {
    /// Pull & run a model. Downloads if not cached.
    Run {
        /// Model ID  (e.g. bonsai:8b)
        model_id: String,
    },
}

// ──────────────────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() -> Result<()> {
    if let Err(e) = run_app().await {
        eprintln!("\n  {} [Cluaiz] Fatal Error: {}\n", "❌".red(), e);
        println!("  Press [Enter] to exit...");
        let mut temp = String::new();
        let _ = std::io::stdin().read_line(&mut temp);
        std::process::exit(1);
    }
    Ok(())
}

async fn run_app() -> Result<()> {
    // ══ PHASE 1 — HEADLESS FAST PATH ══════════════════════════════════════
    // These commands exit BEFORE bootstrap so no stray output contaminates
    // the clean terminal UX.

    let raw_args: Vec<String> = std::env::args().collect();
    let mut auto_start_model = None;

    if let Some("run") = raw_args.get(1).map(|s| s.as_str()) {
        let model_id = match raw_args.get(2) {
            Some(id) => id.clone(),
            None => {
                println!(
                    "\n  {} Usage: cluaiz run <model-id>\n  {} Example: cluaiz run bonsai:8b\n",
                    "⚠️ ".yellow(),
                    "💡".cyan()
                );
                return Ok(());
            }
        };
        // Minimal init for network commands (log redirect only, no TUI bootstrap)
        if let Ok(log_file) = File::create("cluaiz_Core.log") {
            let _ = tracing_subscriber::fmt()
                .with_writer(log_file)
                .with_ansi(false)
                .try_init();
        }
        color_eyre::install()?;
        auto_start_model = Some(crate::cli::run::execute(&model_id).await?);
    } else if let Some("help") | Some("-h") | Some("--help") = raw_args.get(1).map(|s| s.as_str()) {
        return crate::cli::help::print_help();
    }

    // ══ PHASE 2 — FULL BOOT (TUI Dashboard path) ══════════════════════════

    // 🚀 SILENCE THE VOID: Redirect all logs to file before anything else
    if let Ok(log_file) = File::create("cluaiz_Core.log") {
        let _ = tracing_subscriber::fmt()
            .with_writer(log_file)
            .with_ansi(false)
            .try_init();
    }

    color_eyre::install()?;

    // 🚀 Cluaiz BOOTSTRAP
    crate::core::bootstrapper::Bootstrapper::ignite().await?;

    // 📡 Hardware IGNITION
    engines::telemetry::ignite_watchtower();

    // Parse remaining flags (--benchmark, --calibrate)
    let cli_args = Cli::parse();

    if cli_args.benchmark {
        engines::telemetry::health_check::CluaizHealthChecker::run_full_benchmark();
        return Ok(());
    }

    if cli_args.calibrate {
        println!("  {} [Cluaiz] Calibrating hardware...", "🧬".cyan());
        cluaiz_shared::hardware::HardwareGovernor::auto_calibrate()
            .map_err(|e| color_eyre::eyre::eyre!("Calibration failed: {}", e))?;
        println!("  {} [Cluaiz] SiliconTruth synchronized.", "✅".green());
        return Ok(());
    }

    // ── NATIVE ONBOARDING FLOW ────────────────────────────────────────────
    let profile_over = if !::cluaiz_shared::onboarding::should_skip_onboarding() {
        Some(crate::ui::apps::onboarding::native::run_native_flow()?)
    } else {
        None
    };

    // 🛡️ Panic Guard: Ensure terminal recovery on crash
    let hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = crate::core::flow::FlowEngine::restore();
        let _ = ratatui::restore();
        hook(info);
    }));

    // ── Cluaiz PRIMARY FLOW ───────────────────────────────────────────────
    let app = App::new(profile_over, auto_start_model)?;

    let tx = app.tx.clone();
    let hardware = app.state.hardware.clone();
    let ram = app.state.ram_gb;

    // 🧠 Background Initialization: Load recommendations asynchronously
    spawn(async move {
        let _models = tokio::task::spawn_blocking(move || {
            engines::CoreRoster::get_recommendations(&hardware.to_Hardware_truth(), ram)
        })
        .await
        .unwrap_or_default();

        let _ = tx.send(DownloadEvent::Complete("INITIAL_LOAD".to_string()));
    });

    let app_result = app.run().await;

    // ── Cluaiz TEARDOWN ───────────────────────────────────────────────────
    let _ = crate::core::flow::FlowEngine::restore();

    app_result
}
