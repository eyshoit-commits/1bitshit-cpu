use color_eyre::Result;
use colored::Colorize;
use crossterm::event::{self, Event, KeyCode};
use engines::{DownloadEvent, ModelRecommendation};
use inquire::{
    ui::{Attributes, Color as InqColor, RenderConfig, Styled},
    Confirm, Select,
};
use std::io::{stdout, Write};
use std::time::Duration;
use tokio::sync::mpsc;

pub fn show_details(
    _idx: usize,
    rec: &mut ModelRecommendation,
    _total_ram: f64,
    breadcrumb: &str,
    tx: &mpsc::UnboundedSender<DownloadEvent>,
) -> Result<Option<String>> {
    let config = RenderConfig::default()
        .with_prompt_prefix(Styled::new(""))
        .with_answered_prompt_prefix(Styled::new(""))
        .with_selected_option(None)
        .with_highlighted_option_prefix(
            Styled::new("➤")
                .with_fg(InqColor::LightGreen)
                .with_attr(Attributes::BOLD),
        );

    loop {
        print_model_details(rec, breadcrumb);

        let mut options = Vec::new();
        if rec.is_cached && rec.manifest.category != "embedding" {
            options.push("▶  LOAD / SET ACTIVE".to_string());
        } else if !rec.is_cached {
            options.push("📥  INITIATE DOWNLOAD".to_string());
        }
        options.push("↩  BACK".to_string());
        if rec.is_cached {
            options.push("🗑️  DELETE MODEL".red().bold().to_string());
        }

        let choice = match Select::new("", options)
            .with_render_config(config)
            .with_starting_cursor(0)
            .with_formatter(&|_| String::new())
            .prompt()
        {
            Ok(choice) => choice,
            Err(_) => return Ok(None),
        };

        if choice.contains("LOAD / SET ACTIVE") {
            return Ok(Some("LOAD".to_string()));
        }

        if choice.contains("INITIATE DOWNLOAD") {
            match download_model(rec, tx, config)? {
                Some(path) => {
                    rec.is_cached = true;
                    rec.manifest.local_path = Some(path.to_string_lossy().to_string());
                    println!("\n  {} Download complete: {}", "✅".green(), path.display());
                    println!("  {} Loading the downloaded model now...", "⚙️".yellow());
                    return Ok(Some("LOAD".to_string()));
                }
                None => {
                    println!("\n  {} Download cancelled or failed. Model was not marked as cached.", "❌".red());
                    std::thread::sleep(Duration::from_millis(1200));
                    continue;
                }
            }
        }

        if choice.contains("DELETE") {
            let confirmed = Confirm::new("Delete this model from ./models/?")
                .with_default(false)
                .with_render_config(config)
                .prompt()
                .unwrap_or(false);
            if confirmed {
                match engines::ModelDownloader::purge_model(
                    &rec.manifest.category,
                    &rec.manifest.id,
                ) {
                    Ok(()) => {
                        rec.is_cached = false;
                        rec.manifest.local_path = None;
                        println!("  {} Model deleted.", "✅".green());
                    }
                    Err(error) => println!("  {} Delete failed: {}", "❌".red(), error),
                }
            }
            continue;
        }

        return Ok(Some("BACK".to_string()));
    }
}

fn print_model_details(rec: &ModelRecommendation, breadcrumb: &str) {
    let model = &rec.manifest;
    println!("{} ❯ {}", breadcrumb, model.name.dimmed());
    println!("{}   {:<13} {}", "│".cyan(), "ID:".bright_black(), model.id.white());
    println!("{}   {:<13} {}", "│".cyan(), "ARCH:".bright_black(), model.architecture.white());
    println!("{}   {:<13} {}", "│".cyan(), "PARAMS:".bright_black(), model.parameters.cyan());
    println!("{}   {:<13} {}", "│".cyan(), "CONTEXT:".bright_black(), model.context_window.white());
    println!("{}   {:<13} {:.2} GB", "│".cyan(), "VRAM/RAM:".bright_black(), model.ram_required_gb);
    println!("{}   {:<13} {:.2} GB", "│".cyan(), "DISK SIZE:".bright_black(), model.download_size_gb);
    println!(
        "{}   {:<13} {}",
        "│".cyan(),
        "STATUS:".bright_black(),
        if rec.is_cached { "DOWNLOADED".green().bold() } else { rec.status.as_str().yellow() }
    );
    println!("{}", "├─ DESCRIPTION ─────────────────────────────────────────────".cyan());
    println!("{} {}", "│".cyan(), model.description.white());
    println!("{}\n", "└───────────────────────────────────────────────────────────".cyan());
}

fn download_model(
    rec: &ModelRecommendation,
    tx: &mpsc::UnboundedSender<DownloadEvent>,
    config: RenderConfig,
) -> Result<Option<std::path::PathBuf>> {
    let model = rec.manifest.clone();
    let event_id = model.id.clone();
    let tx_clone = tx.clone();
    let abort = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let abort_worker = abort.clone();
    let (local_tx, mut local_rx) = tokio::sync::mpsc::channel(256);

    let handle = tokio::spawn(async move {
        engines::ModelDownloader::download_gguf_async(
            &model.category,
            &model.id,
            &model.download_url,
            &model.huggingface_filename,
            model.assets.clone(),
            Some(model),
            local_tx,
            abort_worker,
        )
        .await
    });

    let runtime = tokio::runtime::Handle::current();
    let result = tokio::task::block_in_place(|| {
        runtime.block_on(async {
            loop {
                if event::poll(Duration::from_millis(50))? {
                    if let Event::Key(key) = event::read()? {
                        if key.code == KeyCode::Esc || key.code == KeyCode::Char('q') {
                            let stop = Confirm::new("Abort download and remove partial file?")
                                .with_default(false)
                                .with_render_config(config)
                                .prompt()
                                .unwrap_or(false);
                            if stop {
                                abort.store(true, std::sync::atomic::Ordering::SeqCst);
                            }
                        }
                    }
                }

                while let Ok(download_event) = local_rx.try_recv() {
                    if let DownloadEvent::Progress(_, current, total, speed, _) = download_event {
                        let percent = if total > 0 {
                            current as f64 / total as f64 * 100.0
                        } else {
                            0.0
                        };
                        print!(
                            "\r  {} {:6.2}%  {:.1}/{:.1} MB  {:.1} MB/s",
                            "⬇".cyan(),
                            percent,
                            current as f64 / 1_048_576.0,
                            total as f64 / 1_048_576.0,
                            speed / 1_048_576.0,
                        );
                        let _ = stdout().flush();
                    }
                }

                if handle.is_finished() {
                    break handle.await.map_err(|error| color_eyre::eyre::eyre!(error))?;
                }
            }
        })
    });

    match result {
        Ok(path) => {
            let _ = tx_clone.send(DownloadEvent::Complete(event_id));
            Ok(Some(path))
        }
        Err(error) => {
            let _ = tx_clone.send(DownloadEvent::Error(event_id, error.clone()));
            println!("\n  {} Download failed: {}", "❌".red(), error);
            Ok(None)
        }
    }
}
