/// ═══════════════════════════════════════════════════════════════════════
///  CURE Engine: GGUF Core Audit Binary v2.0
///  Raw GGUF header inspection - no model load, no tokenizer needed.
///  Tests which dtype each tensor uses and catches incompatible types.
/// ═══════════════════════════════════════════════════════════════════════
use candle_core::quantized::gguf_file;
use std::fs::File;
use std::path::PathBuf;

fn inspect(path: PathBuf, model_name: &str) {
    println!("\n══════════════════════════════════════════");
    println!("🔍 Model: {}", model_name);
    println!("   Path:  {}", path.display());
    println!("══════════════════════════════════════════");

    if !path.exists() {
        println!("❌ FATAL: File not found!");
        return;
    }

    let mut file = match File::open(&path) {
        Ok(f) => f,
        Err(e) => { println!("❌ Cannot open file: {}", e); return; }
    };

    let content = match gguf_file::Content::read(&mut file) {
        Ok(c) => c,
        Err(e) => {
            println!("❌ GGUF Parse FAILED: {}", e);
            println!("   ↑ This is the root cause of the 'unknown dtype' error.");
            return;
        }
    };

    println!("✅ GGUF Header parsed successfully!");
    println!("   Magic:          {:?}", content.magic);
    println!("   Tensor count:   {}", content.tensor_infos.len());
    println!("   Metadata keys:  {}", content.metadata.len());

    // Architecture
    if let Some(arch) = content.metadata.get("general.architecture") {
        println!("   Architecture:   {:?}", arch);
    }
    if let Some(name) = content.metadata.get("general.name") {
        println!("   Model Name:     {:?}", name);
    }

    // Collect and sort tensors for orderly display
    let mut tensors: Vec<(&String, &gguf_file::TensorInfo)> = content.tensor_infos.iter().collect();
    tensors.sort_by_key(|(k, _)| k.as_str());

    println!("\n--- First 10 Tensors ---");
    for (i, (name, info)) in tensors.iter().take(10).enumerate() {
        println!("  [{:03}] {:<50} dtype={:?}", i, name, info.ggml_dtype);
    }

    println!("\n--- Scanning ALL {} tensors for problematic dtypes ---", tensors.len());
    let mut issues = 0;
    for (i, (name, info)) in tensors.iter().enumerate() {
        let dtype_str = format!("{:?}", info.ggml_dtype);
        // Flag anything that isn't a "standard" type well-supported by candle v0.10.2
        let is_standard = matches!(dtype_str.as_str(),
            "F32" | "F16" | "BF16" | "Q4_0" | "Q4_1" | "Q5_0" | "Q5_1" |
            "Q8_0" | "Q8_1" | "Q2K" | "Q3K" | "Q4K" | "Q5K" | "Q6K" | "Q8K"
        );
        if !is_standard {
            println!("  ⚠️  [{:03}] {:<50} dtype={:?}  ← UNSUPPORTED by candle v0.10.2", i, name, info.ggml_dtype);
            issues += 1;
        }
    }

    if issues == 0 {
        println!("  ✅ All tensors use standard supported dtypes. Loading SHOULD work.");
        println!("  ⚠️  If loading still fails, the issue is in the model architecture code, not the dtypes.");
    } 
}

fn main() {
    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║   CURE ENGINE: Core WEIGHT AUDIT PROTOCOL v2.0            ║");
    println!("║   Diagnosing: Target Custom GGUF Architectures              ║");
    println!("╚══════════════════════════════════════════════════════════════╝\n");

    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        println!("Please provide a GGUF file path.");
        return;
    }
    let target = PathBuf::from(&args[1]);

    if !target.exists() {
        println!("File not found: {}", target.display());
        return;
    }

    println!("================================================================");
    println!("🔍 Inspecting Target: {}", target.display());
    println!("================================================================");
    inspect(target, "User Custom Model");

    println!("\n╔══════════════════════════════════════════════════════════════╗");
    println!("║   Core AUDIT COMPLETE                                      ║");
    println!("╚══════════════════════════════════════════════════════════════╝\n");
}
