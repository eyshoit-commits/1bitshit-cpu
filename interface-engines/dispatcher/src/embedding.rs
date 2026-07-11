use anyhow::{anyhow, Result};
use std::ffi::{c_void, CString};
use std::sync::Arc;

use crate::shared::{
    configured_model_id, find_model_file, load_symbol, open_library, resolve_library, FreeFn,
    GenerateEmbeddingFn, InitFn, InstantiateFn,
};

pub struct EmbeddingDispatcher {
    active_lib: Arc<libloading::Library>,
    engine_ptr: *mut c_void,
}

unsafe impl Send for EmbeddingDispatcher {}
unsafe impl Sync for EmbeddingDispatcher {}

impl EmbeddingDispatcher {
    pub fn new() -> Result<Self> {
        let binary_path = resolve_library("onnx")?;
        if cfg!(windows) {
            let drivers = cluaiz_shared::environment::EnvironmentManager::current()
                .engine_dir()
                .join("drivers");
            if let Ok(path) = std::env::var("PATH") {
                std::env::set_var("PATH", format!("{};{}", drivers.display(), path));
            }
        }

        unsafe {
            let library = Arc::new(open_library(&binary_path)?);
            let initialize = load_symbol::<InitFn>(
                &library,
                b"bitshit_kernel_init",
                b"cluaiz_kernel_init",
            )?;
            initialize();

            let instantiate = load_symbol::<InstantiateFn>(
                &library,
                b"bitshit_kernel_instantiate",
                b"cluaiz_kernel_instantiate",
            )?;
            let configured = configured_model_id("vector_models")
                .and_then(|id| find_model_file(&id, &["onnx"]))
                .map(|path| path.to_string_lossy().into_owned())
                .unwrap_or_else(|| "default".to_string());
            let configured = CString::new(configured)?;
            let engine_ptr = instantiate(configured.as_ptr(), std::ptr::null());
            if engine_ptr.is_null() {
                return Err(anyhow!("ONNX kernel instantiation returned null"));
            }

            tracing::info!(
                "[1BitShit Dispatcher] ONNX kernel linked from {}",
                binary_path.display()
            );
            Ok(Self {
                active_lib: library,
                engine_ptr,
            })
        }
    }

    pub fn dispatch_embedding(&self, text: &str) -> Result<Vec<f32>> {
        use neural_core::interfaces::router_contract::EmbeddingDriver;
        self.gen_embedding(text)
            .map_err(|error| anyhow!("Embedding error: {:?}", error))
    }

    pub fn dispatch_multimodal(
        &self,
        bytes: &[u8],
        modality: neural_core::interfaces::router_contract::Modality,
    ) -> Result<Vec<f32>> {
        use neural_core::interfaces::router_contract::EmbeddingDriver;
        self.gen_multimodal_embedding(bytes, modality)
            .map_err(|error| anyhow!("Multimodal error: {:?}", error))
    }
}

impl neural_core::interfaces::router_contract::EmbeddingDriver for EmbeddingDispatcher {
    fn gen_embedding(
        &self,
        text: &str,
    ) -> Result<Vec<f32>, neural_core::interfaces::router_contract::EngineError> {
        unsafe {
            let generate = load_symbol::<GenerateEmbeddingFn>(
                &self.active_lib,
                b"bitshit_kernel_generate_embedding",
                b"cluaiz_kernel_generate_embedding",
            )
            .map_err(|error| {
                neural_core::interfaces::router_contract::EngineError::EmbeddingFailed(
                    error.to_string(),
                )
            })?;
            let text = CString::new(text).map_err(|error| {
                neural_core::interfaces::router_contract::EngineError::EmbeddingFailed(
                    error.to_string(),
                )
            })?;
            let mut output = vec![0.0_f32; 8192];
            let mut output_len = 0_usize;
            let status = generate(
                self.engine_ptr,
                text.as_ptr(),
                output.as_mut_ptr(),
                output.len(),
                &mut output_len,
            );
            if status != 0 {
                return Err(
                    neural_core::interfaces::router_contract::EngineError::EmbeddingFailed(
                        format!("ONNX kernel returned status {status}"),
                    ),
                );
            }
            let length = output_len.min(output.len());
            output.truncate(length);
            Ok(output)
        }
    }

    fn gen_multimodal_embedding(
        &self,
        _bytes: &[u8],
        _modality: neural_core::interfaces::router_contract::Modality,
    ) -> Result<Vec<f32>, neural_core::interfaces::router_contract::EngineError> {
        Err(
            neural_core::interfaces::router_contract::EngineError::UnsupportedModality(
                "Multimodal FFI is not implemented by the current ONNX kernel".to_string(),
            ),
        )
    }
}

impl Drop for EmbeddingDispatcher {
    fn drop(&mut self) {
        if self.engine_ptr.is_null() {
            return;
        }
        unsafe {
            if let Ok(free) = load_symbol::<FreeFn>(
                &self.active_lib,
                b"bitshit_kernel_free",
                b"cluaiz_kernel_free",
            ) {
                free(self.engine_ptr);
            }
        }
    }
}
