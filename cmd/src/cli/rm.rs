use color_eyre::Result;
use colored::Colorize;
use engines::models::registry::CoreRoster;

pub async fn execute(model_id: &str) -> Result<()> {
    println!(
        "\n  {} [1BitShit CPU] Removing model '{}'...",
        "🗑️".red(),
        model_id.bold()
    );

    let model = CoreRoster::load_roster()
        .into_iter()
        .find(|manifest| manifest.id.eq_ignore_ascii_case(model_id))
        .ok_or_else(|| color_eyre::eyre::eyre!(
            "Model ID '{}' was not found in the local registry",
            model_id
        ))?;

    engines::ModelDownloader::purge_model(&model.category, &model.id)
        .map_err(|error| color_eyre::eyre::eyre!(error))?;

    println!(
        "  {} Model '{}' was removed from the visible model store.\n",
        "✅".green(),
        model.id.cyan()
    );
    Ok(())
}
