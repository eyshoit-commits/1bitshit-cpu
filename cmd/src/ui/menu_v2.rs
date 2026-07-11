use std::io::{stdout, Write};

use color_eyre::Result;
use colored::Colorize;
use engines::DownloadEvent;
use inquire::{
    ui::{Attributes, Color, RenderConfig, Styled},
    Select, Text,
};
use tokio::sync::mpsc;

use crate::app_enums::Mode;
use crate::core::state::{AppState, OsState};

pub async fn run_native(
    state: &mut AppState,
    tx: &mpsc::UnboundedSender<DownloadEvent>,
    mode: &mut Mode,
) -> Result<()> {
    let _ = tx;
    print_header(state);

    loop {
        let answer = match Select::new(
            "1BitShit CPU Main Menu:",
            vec![
                "💬 Start New Chat",
                "🌐 Server Control",
                "🧠 Model Hub",
                "⚡ System & Hardware",
                "🛠️ Advanced Utilities",
                "ℹ️ Help",
                "❌ Quit",
            ],
        )
        .with_render_config(render_config())
        .prompt()
        {
            Ok(answer) => answer,
            Err(_) => {
                *mode = Mode::Quit;
                return Ok(());
            }
        };
        clear_prompt()?;

        match answer {
            "💬 Start New Chat" => {
                state.os_state = OsState::Dashboard;
                return Ok(());
            }
            "🌐 Server Control" => server_control(state).await?,
            "🧠 Model Hub" => model_hub().await?,
            "⚡ System & Hardware" => system_and_hardware().await?,
            "🛠️ Advanced Utilities" => advanced_utilities().await?,
            "ℹ️ Help" => show_help(),
            "❌ Quit" => {
                *mode = Mode::Quit;
                return Ok(());
            }
            _ => {}
        }
    }
}

fn print_header(state: &mut AppState) {
    if state.printed_logo {
        return;
    }
    let _ = crossterm::execute!(
        std::io::stdout(),
        crossterm::terminal::LeaveAlternateScreen
    );
    let _ = crossterm::terminal::disable_raw_mode();
    print!("\x1B[2J\x1B[1;1H");
    crate::assets::logos::logo::print_native_logo(state.logo_index);
    println!();
    println!(
        "  {} {}",
        "1BitShit CPU".cyan().bold(),
        format!("v{}", env!("CARGO_PKG_VERSION")).bright_black()
    );
    let runtime_mode = if state.is_client_mode {
        "Client (Background API)".green().bold()
    } else {
        "Standalone CPU Runtime".yellow().bold()
    };
    println!("  {} {}", "Mode:".dimmed(), runtime_mode);
    state.printed_logo = true;
}

async fn server_control(state: &AppState) -> Result<()> {
    let mut options = Vec::new();
    if !state.is_client_mode {
        options.push("🚀 Start API Daemon");
    }
    options.push("👀 View Active Engines");
    options.push("🔙 Back");

    let Ok(answer) = Select::new("Server Control:", options)
        .with_render_config(render_config())
        .prompt()
    else {
        return Ok(());
    };
    clear_prompt()?;

    match answer {
        "🚀 Start API Daemon" => {
            let port = std::env::var("BITSHIT_PORT")
                .ok()
                .and_then(|value| value.parse::<u16>().ok())
                .unwrap_or(8000);
            println!(
                "  {} Starting 1BitShit CPU API on http://localhost:{} ...",
                "🚀".green(),
                port
            );
            cluaiz_api::run_daemon().await;
        }
        "👀 View Active Engines" => {
            let _ = crate::cli::ps::execute().await;
        }
        _ => {}
    }
    Ok(())
}

async fn model_hub() -> Result<()> {
    let Ok(answer) = Select::new(
        "Model Hub:",
        vec!["⬇️ Pull New Model", "🗑️ Delete Downloaded Model", "🔙 Back"],
    )
    .with_render_config(render_config())
    .prompt()
    else {
        return Ok(());
    };
    clear_prompt()?;

    match answer {
        "⬇️ Pull New Model" => {
            if let Ok(model_id) = Text::new("Enter Hugging Face repository or model ID:")
                .with_render_config(render_config())
                .prompt()
            {
                crate::cli::pull::execute(model_id.trim()).await?;
            }
        }
        "🗑️ Delete Downloaded Model" => delete_downloaded_model().await?,
        _ => {}
    }
    Ok(())
}

async fn delete_downloaded_model() -> Result<()> {
    let mut downloaded: Vec<_> = engines::models::registry::CoreRoster::load_roster()
        .into_iter()
        .filter(|model| {
            model.local_path.is_some()
                || engines::models::fetch::ModelDownloader::get_cached_path(
                    &model.category,
                    &model.id,
                    &model.huggingface_filename,
                )
                .is_some()
        })
        .collect();
    downloaded.sort_by(|left, right| left.name.cmp(&right.name));
    downloaded.dedup_by(|left, right| left.id == right.id);

    if downloaded.is_empty() {
        println!("  {} No downloaded models found.", "ℹ️".blue());
        return Ok(());
    }

    let options: Vec<String> = downloaded
        .iter()
        .map(|model| format!("{} [{}]", model.name, model.architecture_type))
        .collect();
    let Ok(answer) = Select::new("Select model to delete:", options)
        .with_render_config(render_config())
        .prompt()
    else {
        return Ok(());
    };

    if let Some(model) = downloaded
        .iter()
        .find(|model| format!("{} [{}]", model.name, model.architecture_type) == answer)
    {
        crate::cli::rm::execute(&model.id).await?;
    }
    Ok(())
}

async fn system_and_hardware() -> Result<()> {
    let Ok(answer) = Select::new(
        "System & Hardware:",
        vec![
            "⚙️ System Booster Config",
            "🛡️ Firewall & Permissions",
            "📊 Hardware Status & Health",
            "🔄 Re-calibrate Hardware",
            "⏱️ Run Benchmark",
            "🔙 Back",
        ],
    )
    .with_render_config(render_config())
    .prompt()
    else {
        return Ok(());
    };
    clear_prompt()?;

    match answer {
        "⚙️ System Booster Config" => {
            crate::cli::booster::execute(None, None, None, None).await?;
        }
        "🛡️ Firewall & Permissions" => permission_control()?,
        "📊 Hardware Status & Health" => {
            engines::telemetry::health_check::cluaizHealthChecker::run_full_benchmark();
        }
        "🔄 Re-calibrate Hardware" => {
            println!("  {} Re-scanning CPU and memory topology...", "🛠️".cyan());
            engines::hardware::system_control_manager::detect_hardware();
            println!("  {} Hardware profile synchronized.", "✅".green());
        }
        "⏱️ Run Benchmark" => {
            crate::cli::benchmark::execute(None, 1).await?;
        }
        _ => {}
    }
    Ok(())
}

fn permission_control() -> Result<()> {
    loop {
        let mut permissions =
            engines::neural_foundry::security::permission_schema::PermissionSchema::load();
        let options = vec![
            format!("WASM Firewall ({})", permissions.wasm_firewall),
            format!("Telemetry ({})", permissions.stream_telemetry),
            format!("Lazy Load ({})", permissions.lazy_load_model),
            format!("Vectorize User Input ({})", permissions.vectorize_user_input),
            format!("Vectorize AI Response ({})", permissions.vectorize_ai_response),
            format!("KV Cache ({})", permissions.enable_kvcache),
            format!("Temporary Chat TTL ({} hours)", permissions.temporary_chat_ttl_hours),
            "🔙 Back".to_string(),
        ];
        let Ok(answer) = Select::new("Permission Control:", options)
            .with_render_config(render_config())
            .prompt()
        else {
            return Ok(());
        };
        if answer == "🔙 Back" {
            return Ok(());
        }

        let key = answer.split(" (").next().unwrap_or_default();
        match key {
            "WASM Firewall" => {
                permissions.wasm_firewall = select_string(
                    "Set WASM firewall:",
                    vec!["auto", "strict", "off"],
                )
                .unwrap_or(permissions.wasm_firewall);
            }
            "Telemetry" => {
                permissions.stream_telemetry = select_bool("Stream telemetry:")
                    .unwrap_or(permissions.stream_telemetry);
            }
            "Lazy Load" => {
                permissions.lazy_load_model =
                    select_bool("Use lazy model loading:").unwrap_or(permissions.lazy_load_model);
            }
            "Vectorize User Input" => {
                permissions.vectorize_user_input = select_bool("Vectorize user input:")
                    .unwrap_or(permissions.vectorize_user_input);
            }
            "Vectorize AI Response" => {
                permissions.vectorize_ai_response = select_bool("Vectorize AI response:")
                    .unwrap_or(permissions.vectorize_ai_response);
            }
            "KV Cache" => {
                permissions.enable_kvcache =
                    select_bool("Enable KV cache:").unwrap_or(permissions.enable_kvcache);
            }
            "Temporary Chat TTL" => {
                if let Ok(value) = Text::new("TTL in hours, or 'max':")
                    .with_render_config(render_config())
                    .prompt()
                {
                    permissions.temporary_chat_ttl_hours = if value.eq_ignore_ascii_case("max") {
                        u64::MAX
                    } else {
                        value.parse().unwrap_or(permissions.temporary_chat_ttl_hours)
                    };
                }
            }
            _ => {}
        }
        permissions.save();
        println!("  {} Permission configuration updated.", "✅".green());
    }
}

async fn advanced_utilities() -> Result<()> {
    let Ok(answer) = Select::new(
        "Advanced Utilities:",
        vec!["🧩 Manage Skills", "📄 Ingest Document", "⚙️ Setup Profile", "🔙 Back"],
    )
    .with_render_config(render_config())
    .prompt()
    else {
        return Ok(());
    };
    clear_prompt()?;

    match answer {
        "🧩 Manage Skills" => skill_manager().await?,
        "📄 Ingest Document" => {
            if let Ok(path) = Text::new("Enter file path:")
                .with_render_config(render_config())
                .prompt()
            {
                crate::cli::ingest::execute(path.trim()).await?;
            }
        }
        "⚙️ Setup Profile" => {
            crate::cli::setup::execute(crate::SetupCommand::Profile).await?;
        }
        _ => {}
    }
    Ok(())
}

async fn skill_manager() -> Result<()> {
    let Ok(answer) = Select::new(
        "Skill Manager:",
        vec!["List Installed Skills", "Install Skill", "Manage Skill Caches", "🔙 Back"],
    )
    .with_render_config(render_config())
    .prompt()
    else {
        return Ok(());
    };

    match answer {
        "List Installed Skills" => {
            crate::cli::component::execute("skill", crate::ComponentCommand::List).await?;
        }
        "Install Skill" => {
            if let Ok(name) = Text::new("Enter skill name:")
                .with_render_config(render_config())
                .prompt()
            {
                crate::cli::component::execute(
                    "skill",
                    crate::ComponentCommand::Install {
                        component_name: name,
                    },
                )
                .await?;
            }
        }
        "Manage Skill Caches" => {
            crate::cli::component::execute(
                "skill",
                crate::ComponentCommand::Cache {
                    command: crate::ComponentCacheCommand::Ls,
                },
            )
            .await?;
        }
        _ => {}
    }
    Ok(())
}

fn show_help() {
    if let Ok(registry) = crate::core::commands::CommandRegistry::load() {
        registry.generate_help();
    } else {
        println!("  {} Unable to load commands.json", "❌".red());
    }
}

fn select_bool(prompt: &str) -> Option<bool> {
    Select::new(prompt, vec!["true", "false"])
        .with_render_config(render_config())
        .prompt()
        .ok()
        .map(|value| value == "true")
}

fn select_string(prompt: &str, values: Vec<&str>) -> Option<String> {
    Select::new(prompt, values)
        .with_render_config(render_config())
        .prompt()
        .ok()
        .map(str::to_string)
}

fn render_config() -> RenderConfig {
    RenderConfig::default()
        .with_prompt_prefix(Styled::new("🏠︎").with_fg(Color::LightCyan))
        .with_highlighted_option_prefix(
            Styled::new("⮞")
                .with_fg(Color::LightCyan)
                .with_attr(Attributes::BOLD),
        )
}

fn clear_prompt() -> Result<()> {
    print!("\x1B[1A\x1B[2K\r");
    stdout().flush()?;
    Ok(())
}
