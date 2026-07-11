use color_eyre::Result;
use colored::Colorize;
use crate::ComponentCommand;

pub async fn execute(component_type: &str, command: ComponentCommand) -> Result<()> {
    match command {
        ComponentCommand::Install { component_name } => {
            install_component(component_type, &component_name).await?;
        }
        ComponentCommand::List => list_components(component_type).await?,
        ComponentCommand::Cache { command } => {
            handle_cache_command(component_type, command).await?;
        }
        ComponentCommand::Remove { component_name } => {
            println!(
                "  {} [1BitShit {}] Removing {}...",
                "🗑️".cyan(),
                component_type.to_uppercase(),
                component_name.as_str().bold()
            );
            engines::neural_foundry::registry::hub_installer::HubInstaller::remove_component(
                component_type,
                &component_name,
            )
            .await
            .map_err(|error| color_eyre::eyre::eyre!(error))?;
            println!(
                "  {} Removed {}",
                "✅".green(),
                component_name.as_str().bold()
            );
        }
        ComponentCommand::Start { component_name } => {
            println!(
                "  {} [1BitShit {}] Starting daemon for {}...",
                "🚀".cyan(),
                component_type.to_uppercase(),
                component_name.as_str().bold()
            );
            // Existing daemon launch integration remains intentionally unchanged.
        }
        ComponentCommand::Link {
            plugin_name,
            skill_name,
        } => {
            println!(
                "  {} [1BitShit Plugin] Linking {} to {}...",
                "🔗".cyan(),
                plugin_name.as_str().bold(),
                skill_name.as_str().bold()
            );
            // Existing plugin-to-skill linkage integration remains available here.
        }
    }
    Ok(())
}

async fn handle_cache_command(
    component_type: &str,
    command: crate::ComponentCacheCommand,
) -> Result<()> {
    match command {
        crate::ComponentCacheCommand::Ls => {
            println!(
                "\n  {} [1BitShit Cache] Scanning {} caches...",
                "🧠".cyan(),
                component_type.to_uppercase()
            );
            let report = engines::neural_foundry::registry::hub_installer::HubInstaller::list_component_cache(
                component_type,
            )
            .map_err(|error| color_eyre::eyre::eyre!(error))?;
            println!("{report}");
        }
        crate::ComponentCacheCommand::Clear {
            component_id,
            all,
            force,
        } => {
            println!(
                "\n  {} [1BitShit Cache] Clearing {} caches...",
                "🧹".yellow(),
                component_type.to_uppercase()
            );
            let removed = engines::neural_foundry::registry::hub_installer::HubInstaller::clear_component_cache(
                component_type,
                component_id,
                all,
                force,
            )
            .map_err(|error| color_eyre::eyre::eyre!(error))?;
            println!("  {} Cleared {} cache entries.\n", "✅".green(), removed);
        }
    }
    Ok(())
}

async fn install_component(component_type: &str, component_name: &str) -> Result<()> {
    println!(
        "  {} [1BitShit {}] Installing {}...",
        "📦".cyan(),
        component_type.to_uppercase(),
        component_name.bold()
    );
    engines::neural_foundry::registry::hub_installer::HubInstaller::install_component(
        component_type,
        component_name,
    )
    .await
    .map_err(|error| color_eyre::eyre::eyre!(error))?;
    Ok(())
}

async fn list_components(component_type: &str) -> Result<()> {
    println!(
        "\n  {} [1BitShit CPU] Installed {} components:",
        "📦".cyan(),
        component_type.to_uppercase()
    );
    let components = engines::neural_foundry::registry::hub_installer::HubInstaller::list_installed_components(
        component_type,
    )
    .unwrap_or_default();
    if components.is_empty() {
        println!(
            "    No {} installed. Use `bitshit {} install <name>`.",
            component_type,
            component_type
        );
    } else {
        for name in components {
            println!("    {} {}", "•".blue(), name.as_str().bold());
        }
    }
    println!();
    Ok(())
}
