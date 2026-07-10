//! 🏛️ Sovereign FFI: Industrial Llama.cpp Bindings
//! This module provides direct access to the core llama.cpp parameters and lifecycle functions.

use std::os::raw::{c_char, c_int, c_float};

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct LlamaModelParams {
    pub devices: *const usize, // ggml_backend_dev_t*
    pub tensor_buft_overrides: *const std::ffi::c_void,
    pub n_gpu_layers: i32,
    pub split_mode: i32,
    pub main_gpu: i32,
    pub tensor_split: *const f32,
    pub progress_callback: Option<extern "C" fn(f32, *mut std::ffi::c_void) -> bool>,
    pub progress_callback_user_data: *mut std::ffi::c_void,
    pub kv_overrides: *const std::ffi::c_void,
    pub vocab_only: bool,
    pub use_mmap: bool,
    pub use_direct_io: bool,
    pub use_mlock: bool,
    pub check_tensors: bool,
    pub use_extra_bufts: bool,
    pub no_host: bool,
    pub no_alloc: bool,
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct LlamaContextParams {
    pub n_ctx: u32,
    pub n_batch: u32,
    pub n_ubatch: u32,
    pub n_seq_max: u32,
    pub n_threads: i32,
    pub n_threads_batch: i32,
    pub rope_scaling_type: i32,
    pub pooling_type: i32,
    pub attention_type: i32,
    pub flash_attn_type: i32,
    pub rope_freq_base: f32,
    pub rope_freq_scale: f32,
    pub yarn_ext_factor: f32,
    pub yarn_attn_factor: f32,
    pub yarn_beta_fast: f32,
    pub yarn_beta_slow: f32,
    pub yarn_orig_ctx: u32,
    pub defrag_thold: f32,
    pub cb_eval: *mut std::ffi::c_void, // ggml_backend_sched_eval_callback
    pub cb_eval_user_data: *mut std::ffi::c_void,
    pub type_k: i32,
    pub type_v: i32,
    pub abort_callback: *mut std::ffi::c_void,
    pub abort_callback_data: *mut std::ffi::c_void,
    pub embeddings: bool,
    pub offload_kqv: bool,
    pub no_perf: bool,
    pub op_offload: bool,
}

extern "C" {
    /// 🚀 Initialize the global llama + ggml backend.
    pub fn llama_backend_init();
    
    /// 🛑 Free the global llama + ggml backend.
    pub fn llama_backend_free();

    /// 🛠️ Get default model parameters.
    pub fn llama_model_default_params() -> LlamaModelParams;

    /// 🛠️ Get default context parameters.
    pub fn llama_context_default_params() -> LlamaContextParams;

    /// 🧬 Load model from file.
    pub fn llama_model_load_from_file(
        path_model: *const c_char,
        params: LlamaModelParams,
    ) -> *mut std::ffi::c_void;

    /// 🧬 Initialize context from model.
    pub fn llama_init_from_model(
        model: *mut std::ffi::c_void,
        params: LlamaContextParams,
    ) -> *mut std::ffi::c_void;

    /// 🗑️ Free model memory.
    pub fn llama_model_free(model: *mut std::ffi::c_void);

    /// 🗑️ Free context memory.
    pub fn llama_free(ctx: *mut std::ffi::c_void);

    /// 🔤 Tokenize text.
    pub fn llama_tokenize(
        vocab: *const std::ffi::c_void,
        text: *const c_char,
        text_len: i32,
        tokens: *mut i32,
        n_max_tokens: i32,
        add_special: bool,
        parse_special: bool,
    ) -> i32;

    /// 🧬 Get model vocabulary.
    pub fn llama_model_get_vocab(model: *const std::ffi::c_void) -> *const std::ffi::c_void;

    /// 🏛️ Initialize a batch for inference.
    pub fn llama_batch_init(n_tokens: i32, embd: i32, n_seq_max: i32) -> LlamaBatch;

    /// 🏛️ Free batch memory.
    pub fn llama_batch_free(batch: LlamaBatch);

    /// ⚡ Execute a decode pass.
    pub fn llama_decode(ctx: *mut std::ffi::c_void, batch: LlamaBatch) -> i32;

    /// 🎲 Industrial Sampler Suite
    pub fn llama_sampler_chain_init(params: *mut std::ffi::c_void) -> *mut std::ffi::c_void;
    pub fn llama_sampler_chain_add(chain: *mut std::ffi::c_void, sampler: *mut std::ffi::c_void);
    pub fn llama_sampler_init_greedy() -> *mut std::ffi::c_void;
    pub fn llama_sampler_init_top_p(p: f32, min_keep: usize) -> *mut std::ffi::c_void;
    pub fn llama_sampler_init_temp(t: f32) -> *mut std::ffi::c_void;
    pub fn llama_sampler_init_dist(seed: u32) -> *mut std::ffi::c_void;
    pub fn llama_sampler_sample(sampler: *mut std::ffi::c_void, ctx: *mut std::ffi::c_void, idx: i32) -> i32;
    pub fn llama_sampler_free(sampler: *mut std::ffi::c_void);

    /// 🧵 Memory Sequence Management (Skill Stitching)
    pub fn llama_get_memory(ctx: *const std::ffi::c_void) -> *mut std::ffi::c_void;
    pub fn llama_memory_seq_add(mem: *mut std::ffi::c_void, seq_id: i32, p0: i32, p1: i32, delta: i32);
    pub fn llama_memory_seq_cp(mem: *mut std::ffi::c_void, seq_id_src: i32, seq_id_dst: i32, p0: i32, p1: i32);
    pub fn llama_memory_seq_rm(mem: *mut std::ffi::c_void, seq_id: i32, p0: i32, p1: i32);
    pub fn llama_get_kv_cache_token_count(ctx: *const std::ffi::c_void) -> i32;

    /// 🔤 Convert token to piece (string).
    pub fn llama_token_to_piece(
        vocab: *const std::ffi::c_void,
        token: i32,
        buf: *mut c_char,
        length: i32,
        lstrip: bool,
        special: bool,
    ) -> i32;
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct LlamaBatch {
    pub n_tokens: i32,
    pub token: *mut i32,
    pub embd: *mut f32,
    pub pos: *mut i32,
    pub n_seq_id: *mut i32,
    pub seq_id: *mut *mut i32,
    pub logits: *mut i8,
}
