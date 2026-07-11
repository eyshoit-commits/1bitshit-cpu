use crate::SetupCommand;

pub async fn execute(command: SetupCommand) -> color_eyre::Result<()> {
    match command {
        SetupCommand::Profile => {
            println!("⚙️ [1BitShit Setup] Initializing node purpose vectorization...");
            println!("⚙️ [1BitShit Setup] Generating SKILL_NODE_ROOT identity vector...");

            let prompt_vector =
                engines::memory::embedding_generator::EmbeddingGenerator::generate_vector(
                    "SKILL_NODE_ROOT",
                );
            let storage_bridge = engines::memory::storage_bridge::load_storage_bridge();
            storage_bridge
                .save_context(
                    "SKILL_NODE_ROOT_IDENTITY",
                    "System Profile Node",
                    &prompt_vector,
                )
                .map_err(|error| color_eyre::eyre::eyre!(
                    "Failed to save the profile vector: {error}"
                ))?;

            println!("✅ [1BitShit Setup] Purpose vector saved to the local brain.");
        }
    }
    Ok(())
}
