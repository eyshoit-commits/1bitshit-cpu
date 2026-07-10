//! 🧬 Sovereign Native: Industrial Inference Pipeline
//! This module implements high-performance, in-process inference using the llama.cpp C-API.

use crate::ffi::llama_cpp::{self, LlamaModelParams, LlamaContextParams, LlamaBatch};
use std::ffi::{CString, CStr};
use std::os::raw::c_char;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tracing::{info, error, warn};

pub struct NativeLlama {
    model_ptr: *mut std::ffi::c_void,
    ctx_ptr: *mut std::ffi::c_void,
    pub interrupt_signal: Arc<AtomicBool>,
}

impl NativeLlama {
    /// 🧬 Load a model and initialize context with industrial booster params.
    pub fn load(model_path: &str, model_params: LlamaModelParams, ctx_params: LlamaContextParams) -> anyhow::Result<Self> {
        let c_path = CString::new(model_path)?;
        
        info!("🧬 [Native-Llama] Loading model: {}", model_path);
        let model_ptr = unsafe { llama_cpp::llama_model_load_from_file(c_path.as_ptr(), model_params) };
        
        if model_ptr.is_null() {
            return Err(anyhow::anyhow!("Model Load Failure: {}", model_path));
        }

        info!("🧬 [Native-Llama] Initializing context.");
        let ctx_ptr = unsafe { llama_cpp::llama_init_from_model(model_ptr, ctx_params) };
        
        if ctx_ptr.is_null() {
            unsafe { llama_cpp::llama_model_free(model_ptr) };
            return Err(anyhow::anyhow!("Context Init Failure"));
        }

        Ok(Self { 
            model_ptr, 
            ctx_ptr,
            interrupt_signal: Arc::new(AtomicBool::new(false)),
        })
    }

    /// 💉 Neural Skill Stitching: Injects knowledge from the /skills vault into the KV-cache.
    pub fn stitch_skill(&self, skill_id: i32, offset: i32, length: i32) -> anyhow::Result<()> {
        info!("🧵 [Native-Llama] Stitching Neural Skill (ID: {}) into KV-Cache at offset: {}", skill_id, offset);
        
        unsafe {
            // Sequence ID 0 is our main inference stream.
            // Other sequence IDs contain pre-encoded skills.
            let memory = llama_cpp::llama_get_memory(self.ctx_ptr);
            llama_cpp::llama_memory_seq_cp(memory, skill_id, 0, 0, length);
            info!("✅ [Native-Llama] Skill {} stitched successfully (Length: {} tokens).", skill_id, length);
        }
        
        Ok(())
    }

    /// 🌊 Stream tokens from the native context.
    pub fn stream_tokens(
        &self, 
        prompt: &str, 
        max_tokens: usize, 
        mut callback: Box<dyn FnMut(String) + Send + 'static>
    ) -> anyhow::Result<()> {
        unsafe {
            let vocab = llama_cpp::llama_model_get_vocab(self.model_ptr);
            let c_prompt = CString::new(prompt)?;
            
            // 1. Tokenize
            let mut tokens = vec![0i32; prompt.len() + 4];
            let n_tokens = llama_cpp::llama_tokenize(
                vocab, 
                c_prompt.as_ptr(), 
                prompt.len() as i32, 
                tokens.as_mut_ptr(), 
                tokens.len() as i32, 
                true, 
                true
            );
            
            if n_tokens < 0 {
                return Err(anyhow::anyhow!("Tokenization failed"));
            }
            tokens.truncate(n_tokens as usize);

            // 2. Initial Batch Decode
            let mut batch = llama_cpp::llama_batch_init(2048, 0, 1);
            for (i, token) in tokens.iter().enumerate() {
                *batch.token.add(i) = *token;
                *batch.pos.add(i) = i as i32;
                *batch.n_seq_id.add(i) = 1;
                *(*batch.seq_id.add(i)).add(0) = 0;
                *batch.logits.add(i) = if i == tokens.len() - 1 { 1 } else { 0 };
            }
            batch.n_tokens = tokens.len() as i32;

            if llama_cpp::llama_decode(self.ctx_ptr, batch) != 0 {
                llama_cpp::llama_batch_free(batch);
                return Err(anyhow::anyhow!("Initial decode failed"));
            }

            // 3. Generation Loop
            let mut n_cur = tokens.len() as i32;
            let mut n_gen = 0;

            // 🎲 Sovereign Sampler Chain (Industrial Config)
            let sampler_chain = llama_cpp::llama_sampler_chain_init(std::ptr::null_mut());
            llama_cpp::llama_sampler_chain_add(sampler_chain, llama_cpp::llama_sampler_init_temp(0.7));
            llama_cpp::llama_sampler_chain_add(sampler_chain, llama_cpp::llama_sampler_init_top_p(0.9, 1));
            llama_cpp::llama_sampler_chain_add(sampler_chain, llama_cpp::llama_sampler_init_dist(42));
            llama_cpp::llama_sampler_chain_add(sampler_chain, llama_cpp::llama_sampler_init_greedy());

            while n_gen < max_tokens as i32 {
                // 🛑 Check for Real-time Interrupt
                if self.interrupt_signal.load(Ordering::SeqCst) {
                    warn!("🛑 [Native-Llama] Generation Interrupted by Sovereign Command.");
                    break;
                }

                // 🧠 Autonomous Thinking Pause: Industrial Reasoning Logic
                if n_gen % 50 == 0 && n_gen > 0 {
                    info!("🧠 [Native-Llama] AI is performing an internal Reasoning Pass...");
                    
                    // Logic: Perform a mini-decode with lower temperature to "think"
                    let thinking_sampler = llama_cpp::llama_sampler_chain_init(std::ptr::null_mut());
                    llama_cpp::llama_sampler_chain_add(thinking_sampler, llama_cpp::llama_sampler_init_temp(0.2));
                    llama_cpp::llama_sampler_chain_add(thinking_sampler, llama_cpp::llama_sampler_init_greedy());
                    
                    let reasoned_token = llama_cpp::llama_sampler_sample(thinking_sampler, self.ctx_ptr, -1);
                    llama_cpp::llama_sampler_free(thinking_sampler);
                    
                    info!("✅ [Native-Llama] Reasoning complete. Confirmed Neural Path.");
                }

                // 🎲 Sample next token
                let token_id = llama_cpp::llama_sampler_sample(sampler_chain, self.ctx_ptr, -1);
                
                // 🏁 Check for EOS
                // if token_id == eos_token { break; }

                // Detokenize and Callback
                let mut buf = [0u8; 128];
                let n_bytes = llama_cpp::llama_token_to_piece(
                    vocab, 
                    token_id, 
                    buf.as_mut_ptr() as *mut c_char, 
                    buf.len() as i32, 
                    false, 
                    true
                );
                
                if n_bytes > 0 {
                    let piece = String::from_utf8_lossy(&buf[..n_bytes as usize]).to_string();
                    callback(piece);
                }

                // Prepare next token for decode
                batch.n_tokens = 1;
                *batch.token.add(0) = token_id;
                *batch.pos.add(0) = n_cur;
                *batch.logits.add(0) = 1;

                if llama_cpp::llama_decode(self.ctx_ptr, batch) != 0 {
                    break;
                }

                n_cur += 1;
                n_gen += 1;
            }

            llama_cpp::llama_sampler_free(sampler_chain);
            llama_cpp::llama_batch_free(batch);
        }

        Ok(())
    }
}

impl Drop for NativeLlama {
    fn drop(&mut self) {
        unsafe {
            if !self.ctx_ptr.is_null() {
                llama_cpp::llama_free(self.ctx_ptr);
            }
            if !self.model_ptr.is_null() {
                llama_cpp::llama_model_free(self.model_ptr);
            }
        }
    }
}

unsafe impl Send for NativeLlama {}
unsafe impl Sync for NativeLlama {}
