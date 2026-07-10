use candle_core::quantized::gguf_file;
use std::fs::File;

fn main() {
    let path = r"C:\Users\Aryan\my\Cluaiz-workspace\Cluaiz-OS\Cluaiz-ai-CURE\models\models--lmstudio-community--gemma-2-2b-it-GGUF\gemma-2-2b-it-Q4_K_M.gguf";
    let mut file = File::open(path).expect("File error");
    let content = gguf_file::Content::read(&mut file).expect("GGUF error");
    
    println!("--- Identification Metadata ---");
    for key in ["general.name", "general.basename", "general.base_model.name", "general.architecture"] {
        if let Some(value) = content.metadata.get(key) {
             println!("{}: {:?}", key, value);
        }
    }
}
