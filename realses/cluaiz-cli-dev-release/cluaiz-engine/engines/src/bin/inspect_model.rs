use candle_core::quantized::gguf_file;
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        println!("Usage: inspect_model <path_to_gguf>");
        return Ok(());
    }

    let path = PathBuf::from(&args[1]);
    let mut file = std::fs::File::open(&path)?;
    let content = gguf_file::Content::read(&mut file)?;

    println!(
        "🧪 [CURE] Inspecting Model: {:?}",
        path.file_name().unwrap()
    );
    println!("════════════════════════════════════════════════");

    // 1. General Metadata
    println!("GENERAL METADATA:");
    for (k, v) in content.metadata.iter() {
        if k.starts_with("general") || k.contains("count") || k.contains("length") {
            println!("  - {}: {:?}", k, v);
        }
    }

    // 2. Specific Architecture Keys
    let arch = match content.metadata.get("general.architecture") {
        Some(v) => v
            .to_string()
            .cloned()
            .unwrap_or_else(|_| "unknown".to_string()),
        None => "unknown".to_string(),
    };

    println!("\nDetected Arch: {}", arch);

    println!("\nARCHITECTURE-SPECIFIC KEYS:");
    for (k, v) in content.metadata.iter() {
        if k.starts_with(arch.as_str()) {
            println!("  - {}: {:?}", k, v);
        }
    }

    // 3. Tensor Information
    println!("\nTENSORS: {}", content.tensor_infos.len());
    for (name, _) in content.tensor_infos.iter() {
        println!("  - {}", name);
    }

    Ok(())
}
