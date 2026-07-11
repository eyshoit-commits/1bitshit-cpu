#![recursion_limit = "256"]
//! 1BitShit CPU API Gateway.
//! HTTP, SSE and native FFI access for the complete local runtime.

mod state;
mod handlers;
mod routes;
mod ffi_bridge;

use std::env;
use std::sync::Arc;

use cluaiz_shared::backend::signature::KernelSignature;
use colored::*;
use dispatcher::NeuralDispatcher;
use state::AppState;
use system_booster::SystemBooster;

pub async fn run_daemon() {
    let args: Vec<String> = env::args().collect();
    if args.iter().any(|argument| argument == "--setup") {
        println!("🛠️ [1BitShit CPU] Initializing hardware calibration...");
        if let Err(error) = cluaiz_shared::HardwareGovernor::auto_calibrate() {
            eprintln!("❌ [1BitShit CPU] Calibration failed: {error}");
            std::process::exit(1);
        }
        println!("✅ [1BitShit CPU] Hardware profile updated.");
        std::process::exit(0);
    }

    let _ = tracing_subscriber::fmt()
        .with_target(false)
        .with_level(true)
        .compact()
        .try_init();

    tracing::info!("Initializing 1BitShit CPU API runtime");

    let pure_brain = cluaiz_shared::hardware::governor::HardwareGovernor::load_system_control()
        .ok()
        .map(|control| control.brain.is_pure_brain())
        .unwrap_or(false);

    if pure_brain {
        tracing::info!("Pure Brain mode active: local LLM loading is suspended");
    }

    let booster_state = if pure_brain {
        Default::default()
    } else {
        SystemBooster::ignite().unwrap_or_default()
    };

    let dispatcher = NeuralDispatcher::new(booster_state, KernelSignature::default());
    let embedding_dispatcher = Arc::new(
        dispatcher::EmbeddingDispatcher::new()
            .expect("Failed to initialize the 1BitShit ONNX embedding engine"),
    );
    let state = Arc::new(AppState {
        dispatcher,
        embedding_dispatcher,
    });
    let app = routes::build(state.clone());

    let port: u16 = env::var("BITSHIT_PORT")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(8000);
    let address = format!("0.0.0.0:{port}");

    print_banner(port);
    tracing::info!("1BitShit CPU Gateway listening on {address}");
    tracing::info!("Starting native FFI listener");
    tokio::spawn(async move {
        ffi_bridge::start_named_pipe_server(state.clone()).await;
    });

    let listener = match tokio::net::TcpListener::bind(&address).await {
        Ok(listener) => listener,
        Err(error) => {
            tracing::error!("Failed to bind {address}: {error}");
            println!(
                "\n  {} 1BitShit CPU API is already running. Continuing in client mode.\n",
                "✅".green()
            );
            return;
        }
    };

    if let Err(error) = axum::serve(
        listener,
        app.into_make_service_with_connect_info::<std::net::SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown_signal())
    .await
    {
        tracing::error!("1BitShit CPU API server stopped unexpectedly: {error}");
    }

    tracing::info!("1BitShit CPU Gateway shut down cleanly");
}

fn print_banner(port: u16) {
    println!(
        "\n{}",
        "┏━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┓"
            .bright_blue()
    );
    println!(
        "{} {}",
        "┃".bright_blue(),
        "1BitShit CPU API & Native FFI".bright_cyan().bold()
    );
    println!(
        "{} {}",
        "┃".bright_blue(),
        format!("v{} — Local Llama, GGUF, BitNet and ONNX Runtime", env!("CARGO_PKG_VERSION"))
            .bright_black()
    );
    println!(
        "{}",
        "┣━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┫"
            .bright_blue()
    );
    println!(
        "{} {}",
        "┃".bright_blue(),
        format!("🌐 Gateway:   http://localhost:{port}")
            .bright_green()
            .bold()
    );
    println!(
        "{} {}",
        "┃".bright_blue(),
        "💚 Status:    ONLINE".bright_green().bold()
    );
    println!(
        "{} {}",
        "┃".bright_blue(),
        "🧠 Engines:   LLAMA • GGUF • BITNET • ONNX".magenta().bold()
    );
    println!(
        "{}",
        "┣━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┫"
            .bright_blue()
    );
    println!("{} {}", "┃".bright_blue(), "Endpoints:".bright_magenta().bold());
    for endpoint in [
        "POST /v1/chat/completions  → Chat and streaming",
        "GET  /sessions            → List chat sessions",
        "POST /v1/db/execute       → Native FFI database query",
        "POST /v1/system/brain     → Toggle brain mode",
        "GET  /hardware            → CPU and memory status",
        "POST /models/download     → Download from Hugging Face",
        "POST /models/load         → Load and activate model",
        "GET  /v1/skills/list      → List skills",
        "POST /v1/skills/install   → Install skill",
    ] {
        println!("{}     {}", "┃".bright_blue(), endpoint.white());
    }
    println!(
        "{}",
        "┗━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┛\n"
            .bright_blue()
    );
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => println!("\n  {} [1BitShit CPU] SIGINT received.", "🛑".red()),
        _ = terminate => println!("\n  {} [1BitShit CPU] SIGTERM received.", "🛑".red()),
    }

    println!("  {} Flushing caches and releasing model memory...", "🧹".yellow());
    tokio::time::sleep(std::time::Duration::from_millis(300)).await;
    println!("  {} 1BitShit CPU terminated cleanly.", "✅".green());
}
