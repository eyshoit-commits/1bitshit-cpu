use candle_core::quantized::gguf_file;
use std::fs::File;
use std::path::Path;

fn main() {
    let path = Path::new("..\\models\\chat\\gemma-4-E2B-it\\gemma-4-E2B-it-Q4_K_M.gguf");
    let mut file = File::open(path).unwrap();
    let content = gguf_file::Content::read(&mut file).unwrap();

    let mut q_dims = std::collections::HashMap::new();
    let mut k_dims = std::collections::HashMap::new();
    let mut embd_dims = std::collections::HashMap::new();

    println!("--- TENSOR SCAN ---");
    for (name, tensor_info) in content.tensor_infos.iter() {
        let dims = tensor_info.shape.dims();
        let out_dim = dims[0];
        let in_dim = *dims.last().unwrap_or(&0);

        if name.ends_with("attn_q.weight") {
            println!("Q: {} -> {:?}", name, dims);
            *q_dims.entry(out_dim).or_insert(0) += 1;
        }
        if name.ends_with("attn_k.weight") {
            println!("K: {} -> {:?}", name, dims);
            *k_dims.entry(out_dim).or_insert(0) += 1;
        }
        if name.ends_with("token_embd.weight") {
            println!("E: {} -> {:?}", name, dims);
            *embd_dims.entry(in_dim).or_insert(0) += 1;
        }
    }

    println!("\n--- RESULTS ---");
    println!("Q Freq: {:?}", q_dims);
    println!("K Freq: {:?}", k_dims);
    println!("E Freq: {:?}", embd_dims);
}
