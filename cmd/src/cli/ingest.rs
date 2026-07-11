use std::path::{Path, PathBuf};

use color_eyre::Result;
use colored::Colorize;
use engines::neural_foundry::ingestion::DocumentIngestor;
use neural_core::interfaces::router_contract::EmbeddingDriver;

use cluaiz_onnx::engine::OnnxEngine;

pub async fn execute(file_path: &str) -> Result<()> {
    println!(
        "\n  {} [1BitShit CPU] Document ingestion started",
        "🚀".green()
    );
    println!("  {} Target: {}\n", "📄".cyan(), file_path.bold());

    let mut onnx_driver = OnnxEngine::new()
        .map_err(|error| color_eyre::eyre::eyre!(
            "Failed to initialize the 1BitShit ONNX engine: {error}"
        ))?;

    let models_dir = cluaiz_shared::environment::EnvironmentManager::current()
        .ensure_models_dir()
        .unwrap_or_else(|_| {
            cluaiz_shared::environment::EnvironmentManager::current().models_dir()
        });
    let model_path = find_onnx_model(&models_dir).ok_or_else(|| {
        color_eyre::eyre::eyre!(
            "No ONNX model was found under {}. Download an ONNX vision or embedding model first.",
            models_dir.display()
        )
    })?;

    println!(
        "  {} Loading ONNX model: {}",
        "🔮".magenta(),
        model_path.display()
    );
    onnx_driver
        .load_vision_model(&model_path.to_string_lossy(), None)
        .map_err(|error| color_eyre::eyre::eyre!(
            "ONNX model loading failed for {}: {error}",
            model_path.display()
        ))?;

    let ingestor = DocumentIngestor::new();
    match ingestor.ingest_and_vectorize(file_path, &onnx_driver) {
        Ok(results) => {
            println!("  {} Document processed and chunked.", "✅".green());
            println!(
                "  {} Generated {} semantic chunks.\n",
                "✂️".cyan(),
                results.len().to_string().yellow().bold()
            );

            for (index, (text, vector)) in results.iter().enumerate().take(5) {
                println!(
                    "  {} {}:\n{}",
                    "CHUNK".magenta(),
                    (index + 1).to_string().cyan(),
                    text.dimmed()
                );
                let preview: Vec<String> = vector
                    .iter()
                    .take(5)
                    .map(|value| format!("{value:.4}"))
                    .collect();
                println!(
                    "  {} [{}, ...] ({} dimensions)\n",
                    "VECTOR:".blue(),
                    preview.join(", "),
                    vector.len()
                );
            }

            if results.len() > 5 {
                println!("  ... and {} additional chunks.", results.len() - 5);
            }
        }
        Err(error) => {
            return Err(color_eyre::eyre::eyre!(
                "Document ingestion failed: {error}"
            ));
        }
    }

    Ok(())
}

fn find_onnx_model(root: &Path) -> Option<PathBuf> {
    let entries = std::fs::read_dir(root).ok()?;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if let Some(found) = find_onnx_model(&path) {
                return Some(found);
            }
        } else if path
            .extension()
            .and_then(|extension| extension.to_str())
            .map(str::to_ascii_lowercase)
            .as_deref()
            == Some("onnx")
        {
            return Some(path);
        }
    }
    None
}
