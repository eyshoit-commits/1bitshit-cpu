use color_eyre::Result;
use colored::Colorize;

const PULL_IMPLEMENTATION: &str = "visible-models-v4-canonical";

/// Downloads, registers, activates and opens a model through the same canonical
/// path used by `bitshit run`. Keeping one implementation prevents GGUF and
/// ONNX from drifting into different stores or loaders again.
pub async fn execute(model_id: &str) -> Result<()> {
    println!(
        "\n  {} [1BitShit CPU:{}] Pulling '{}' through the canonical model path...",
        "⚙️".yellow(),
        PULL_IMPLEMENTATION,
        model_id.bold()
    );

    crate::cli::run::execute(model_id, true).await
}
