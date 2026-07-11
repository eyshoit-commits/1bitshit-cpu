use anyhow::Result;
use ort::session::Session;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};
use tokenizers::Tokenizer;

/// ONNX multimodal runtime for embeddings, text and vision.
/// CPU execution is the default. CUDA is only considered when the crate is
/// explicitly built with `--features cuda`.
pub struct OnnxEngine {
    pub(crate) session_pool: Vec<Arc<std::sync::Mutex<Session>>>,
    pub(crate) tokenizer: Option<Arc<Tokenizer>>,
    pub(crate) active_inferences: Arc<AtomicUsize>,
    pub(crate) active_kv_cache: Option<Vec<(Vec<usize>, Vec<f32>)>>,
}

impl OnnxEngine {
    pub fn new() -> Result<Self> {
        let _ = ort::init().with_name("bitshit_onnx_env").commit();
        tracing::info!(
            "[1BitShit ONNX] CPU runtime initialized; models can now be loaded"
        );

        Ok(Self {
            session_pool: Vec::new(),
            tokenizer: None,
            active_inferences: Arc::new(AtomicUsize::new(0)),
            active_kv_cache: None,
        })
    }

    pub(crate) fn acquire_session(
        &self,
    ) -> Result<
        Arc<std::sync::Mutex<Session>>,
        neural_core::interfaces::router_contract::EngineError,
    > {
        if self.session_pool.is_empty() {
            return Err(
                neural_core::interfaces::router_contract::EngineError::Internal(
                    "No ONNX sessions are active; load a model first".into(),
                ),
            );
        }
        for session in &self.session_pool {
            if session.try_lock().is_ok() {
                return Ok(session.clone());
            }
        }
        tracing::warn!(
            "[1BitShit ONNX] All {} sessions are busy; waiting for the first session",
            self.session_pool.len()
        );
        Ok(self.session_pool[0].clone())
    }

    pub fn load_text_model(
        &mut self,
        model_path: &str,
        tokenizer_path: &str,
        booster: Option<cluaiz_shared::hardware::schema::booster::cluaizBoosterContext>,
    ) -> Result<()> {
        self.evict_sessions("text", model_path);
        tracing::info!("[1BitShit ONNX] Loading text model from {}", model_path);

        let use_gpu = Self::gpu_requested(booster.as_ref());
        let total_threads = std::thread::available_parallelism()
            .map(|value| value.get())
            .unwrap_or(4);
        let pool_size = total_threads.min(4).max(1);
        let threads_per_session = (total_threads / pool_size).max(1);

        tracing::info!(
            "[1BitShit ONNX] Building {} session(s), {} CPU thread(s) each",
            pool_size,
            threads_per_session
        );
        for index in 0..pool_size {
            let session = Session::builder()
                .map_err(|error| anyhow::anyhow!("Session builder error: {error:?}"))?
                .with_intra_threads(threads_per_session)
                .map_err(|error| anyhow::anyhow!("Thread configuration failed: {error:?}"))?
                .commit_from_file(model_path)
                .map_err(|error| {
                    anyhow::anyhow!("ONNX text session {index} failed: {error}")
                })?;
            self.session_pool
                .push(Arc::new(std::sync::Mutex::new(session)));
        }

        Self::report_execution_provider(use_gpu, "text");
        let tokenizer = Tokenizer::from_file(tokenizer_path)
            .map_err(|error| anyhow::anyhow!("Tokenizer loading failed: {error}"))?;
        self.tokenizer = Some(Arc::new(tokenizer));
        tracing::info!(
            "[1BitShit ONNX] {} text session(s) loaded",
            self.session_pool.len()
        );
        Ok(())
    }

    pub fn load_vision_model(
        &mut self,
        model_path: &str,
        booster: Option<cluaiz_shared::hardware::schema::booster::cluaizBoosterContext>,
    ) -> Result<()> {
        self.evict_sessions("vision", model_path);
        tracing::info!("[1BitShit ONNX] Loading vision model from {}", model_path);

        let use_gpu = Self::gpu_requested(booster.as_ref());
        let threads = std::thread::available_parallelism()
            .map(|value| value.get())
            .unwrap_or(4);
        let session = Session::builder()
            .map_err(|error| anyhow::anyhow!("Vision session builder error: {error:?}"))?
            .with_intra_threads(threads)
            .map_err(|error| anyhow::anyhow!("Thread configuration failed: {error:?}"))?
            .commit_from_file(model_path)
            .map_err(|error| anyhow::anyhow!("ONNX vision session failed: {error}"))?;
        self.session_pool
            .push(Arc::new(std::sync::Mutex::new(session)));
        Self::report_execution_provider(use_gpu, "vision");
        tracing::info!("[1BitShit ONNX] Vision session loaded");
        Ok(())
    }

    fn evict_sessions(&mut self, kind: &str, model_path: &str) {
        if self.session_pool.is_empty() {
            return;
        }
        let active = self.active_inferences.load(Ordering::Relaxed);
        if active > 0 {
            tracing::warn!(
                "[1BitShit ONNX] {} active {} inference request(s) will finish on retained Arc sessions",
                active,
                kind
            );
        }
        tracing::info!(
            "[1BitShit ONNX] Replacing {} session(s) before loading {}",
            self.session_pool.len(),
            model_path
        );
        self.session_pool.clear();
        self.tokenizer = None;
    }

    fn gpu_requested(
        booster: Option<&cluaiz_shared::hardware::schema::booster::cluaizBoosterContext>,
    ) -> bool {
        #[cfg(not(feature = "cuda"))]
        {
            if booster.map(|value| value.n_gpu_layers > 0).unwrap_or(false) {
                tracing::warn!(
                    "[1BitShit ONNX] GPU execution was requested, but this binary was built CPU-only"
                );
            }
            false
        }

        #[cfg(feature = "cuda")]
        {
            if booster.map(|value| value.n_gpu_layers == 0).unwrap_or(false) {
                tracing::info!("[1BitShit ONNX] Booster explicitly selected CPU execution");
                return false;
            }

            let telemetry_allows_gpu =
                cluaiz_shared::hardware::system_performance::get_pulse()
                    .pulse
                    .read()
                    .map(|state| {
                        let free_vram = state.vram_total_gb - state.vram_used_gb;
                        free_vram > 2.0 && state.vram_pressure_pct < 95
                    })
                    .unwrap_or(false);
            let booster_forces_gpu = booster
                .map(|value| value.n_gpu_layers > 0)
                .unwrap_or(false);
            telemetry_allows_gpu || booster_forces_gpu
        }
    }

    fn report_execution_provider(use_gpu: bool, kind: &str) {
        #[cfg(feature = "cuda")]
        if use_gpu {
            tracing::info!(
                "[1BitShit ONNX] CUDA feature is enabled for the {} model",
                kind
            );
            return;
        }

        let _ = use_gpu;
        tracing::info!(
            "[1BitShit ONNX] {} model is using the CPU execution provider",
            kind
        );
    }
}

use neural_core::interfaces::router_contract::{EmbeddingDriver, EngineError, Modality};

impl EmbeddingDriver for OnnxEngine {
    fn gen_embedding(&self, text: &str) -> Result<Vec<f32>, EngineError> {
        self.execute_text_embedding(text)
    }

    fn gen_multimodal_embedding(
        &self,
        bytes: &[u8],
        modality: Modality,
    ) -> Result<Vec<f32>, EngineError> {
        match modality {
            Modality::Image => self.execute_vision_embedding(bytes),
            _ => Err(EngineError::UnsupportedModality(
                "The current ONNX multimodal engine supports image embeddings only"
                    .to_string(),
            )),
        }
    }
}
