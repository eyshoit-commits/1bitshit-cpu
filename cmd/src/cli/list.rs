use color_eyre::Result;
use colored::Colorize;
use engines::models::registry::CoreRoster;

pub async fn execute() -> Result<()> {
    println!(
        "\n  {} [1BitShit CPU] Scanning the visible model store...\n",
        "🔍".cyan()
    );

    let roster = CoreRoster::load_roster();
    if roster.is_empty() {
        println!("  {} No model manifests were found.", "⚠️".yellow());
        println!(
            "  {} Use 'bitshit run owner/repository' to download a model.\n",
            "💡".cyan()
        );
        return Ok(());
    }

    println!(
        "  {:<38} {:<12} {:<10} {}",
        "MODEL ID".bold(),
        "STATUS".bold(),
        "RUNTIME".bold(),
        "LOCAL PATH".bold()
    );
    println!("  {}", "-".repeat(110).dimmed());

    let mut local_count = 0usize;
    for model in &roster {
        let local_path = engines::ModelDownloader::get_cached_path(
            &model.category,
            &model.id,
            &model.huggingface_filename,
        );
        let (status, path) = if let Some(path) = local_path {
            local_count += 1;
            ("LOCAL".green().bold(), path.display().to_string())
        } else {
            ("AVAILABLE".bright_black(), "-".to_string())
        };
        println!(
            "  {:<38} {:<12} {:<10} {}",
            model.id,
            status,
            model.architecture_type,
            path
        );
    }

    println!(
        "\n  {} {} local model(s), {} catalog entry/entries.\n",
        "📊".blue(),
        local_count,
        roster.len()
    );
    Ok(())
}
