use std::ffi::{c_char, c_void, CStr, CString};
use std::path::PathBuf;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

use anyhow::{anyhow, Result};
use cluaiz_shared::backend::signature::{BackendType, GlobalFeatureRegistry, KernelSignature};
use system_booster::BoosterControl;
use tokio::sync::mpsc;

use crate::shared::{
    configured_model_id, find_model_file, load_symbol, open_library, resolve_library, FreeFn,
    GenerateStreamFn, InstantiateFn,
};

pub enum EngineResponse {
    TokenStream(mpsc::Receiver<String>),
    FinalResult(String),
    Error(String),
}

#[derive(Clone)]
pub struct SafeEnginePtr(pub *mut c_void);
unsafe impl Send for SafeEnginePtr {}
unsafe impl Sync for SafeEnginePtr {}

pub struct NeuralDispatcher {
    pub booster_state: BoosterControl,
    pub current_signature: KernelSignature,
    pub cached_engine:
        Arc<tokio::sync::Mutex<Option<(PathBuf, SafeEnginePtr, Arc<libloading::Library>)>>>,
    pub inference_semaphore: Arc<tokio::sync::Semaphore>,
    pub cancel_flag: Arc<AtomicBool>,
}

impl NeuralDispatcher {
    pub fn new(booster_state: BoosterControl, signature: KernelSignature) -> Self {
        Self {
            booster_state,
            current_signature: signature,
            cached_engine: Arc::new(tokio::sync::Mutex::new(None)),
            // One native model instance is serialized. This preserves the queue
            // while avoiding concurrent mutation of llama.cpp state.
            inference_semaphore: Arc::new(tokio::sync::Semaphore::new(1)),
            cancel_flag: Arc::new(AtomicBool::new(false)),
        }
    }

    pub async fn dispatch_stream(&self, prompt: &str, _skip_brain: bool) -> EngineResponse {
        let hardware = cluaiz_shared::hardware::HardwareOrchestrator::probe().silicon_truth;
        let backend = GlobalFeatureRegistry::select_runtime(&self.current_signature, &hardware);
        tracing::info!("[1BitShit Dispatcher] Selected backend: {:?}", backend);

        match backend {
            BackendType::RuntimeA | BackendType::RuntimeB | BackendType::RuntimeC => {}
            _ => {
                return EngineResponse::Error(format!(
                    "Unsupported backend architecture: {:?}",
                    backend
                ));
            }
        }

        let (sender, receiver) = mpsc::channel::<String>(100);
        let prompt = prompt.to_string();
        let cached_engine = self.cached_engine.clone();
        let semaphore = self.inference_semaphore.clone();
        let cancel_flag = self.cancel_flag.clone();
        cancel_flag.store(false, Ordering::Relaxed);

        tokio::spawn(async move {
            let _permit = match semaphore.acquire().await {
                Ok(permit) => permit,
                Err(_) => {
                    let _ = sender.send("Error: inference queue closed".to_string()).await;
                    return;
                }
            };

            let model_id = match configured_model_id("chat_models") {
                Some(id) => id,
                None => {
                    let _ = sender
                        .send(
                            "Error: no active chat model is configured. Use 'bitshit model set-chat <id>'."
                                .to_string(),
                        )
                        .await;
                    let _ = sender.send("\n[DONE]\n".to_string()).await;
                    return;
                }
            };
            let model_path = match find_model_file(&model_id, &["gguf", "bin"]) {
                Some(path) => path,
                None => {
                    let _ = sender
                        .send(format!(
                            "Error: active model '{}' was not found in the visible model store.",
                            model_id
                        ))
                        .await;
                    let _ = sender.send("\n[DONE]\n".to_string()).await;
                    return;
                }
            };

            let mut engine_guard = cached_engine.lock().await;
            let needs_reload = !matches!(
                &*engine_guard,
                Some((cached_path, pointer, _))
                    if cached_path == &model_path && !pointer.0.is_null()
            );

            if needs_reload {
                if let Some((_, pointer, library)) = engine_guard.take() {
                    unsafe {
                        if let Ok(free) = load_symbol::<FreeFn>(
                            &library,
                            b"bitshit_kernel_free",
                            b"cluaiz_kernel_free",
                        ) {
                            free(pointer.0);
                        }
                    }
                }

                let library_path = match resolve_library("llama") {
                    Ok(path) => path,
                    Err(error) => {
                        let _ = sender.send(format!("Error: {error}")).await;
                        let _ = sender.send("\n[DONE]\n".to_string()).await;
                        return;
                    }
                };

                let loaded = unsafe {
                    let library = match open_library(&library_path) {
                        Ok(library) => Arc::new(library),
                        Err(error) => {
                            let _ = sender.try_send(format!("Error: {error}"));
                            return;
                        }
                    };
                    let instantiate = match load_symbol::<InstantiateFn>(
                        &library,
                        b"bitshit_kernel_instantiate",
                        b"cluaiz_kernel_instantiate",
                    ) {
                        Ok(symbol) => symbol,
                        Err(error) => {
                            let _ = sender.try_send(format!("Error: {error}"));
                            return;
                        }
                    };
                    let path = match CString::new(model_path.to_string_lossy().as_bytes()) {
                        Ok(path) => path,
                        Err(error) => {
                            let _ = sender.try_send(format!("Error: invalid model path: {error}"));
                            return;
                        }
                    };
                    let pointer = instantiate(path.as_ptr(), std::ptr::null());
                    if pointer.is_null() {
                        let _ = sender.try_send(
                            "Error: Llama kernel returned a null model instance".to_string(),
                        );
                        return;
                    }
                    (SafeEnginePtr(pointer), library)
                };

                *engine_guard = Some((model_path.clone(), loaded.0, loaded.1));
            }

            let (pointer, library) = match &*engine_guard {
                Some((_, pointer, library)) => (pointer.clone(), library.clone()),
                None => {
                    let _ = sender
                        .send("Error: Llama engine is not active".to_string())
                        .await;
                    let _ = sender.send("\n[DONE]\n".to_string()).await;
                    return;
                }
            };
            drop(engine_guard);

            struct CallbackData {
                sender: mpsc::Sender<String>,
                cancel_flag: Arc<AtomicBool>,
                buffer: std::sync::Mutex<String>,
            }

            extern "C" fn callback(token: *const c_char, user_data: *mut c_void) -> bool {
                if token.is_null() || user_data.is_null() {
                    return false;
                }
                let data = unsafe { &*(user_data as *const CallbackData) };
                if data.cancel_flag.load(Ordering::Relaxed) {
                    return false;
                }

                let token = unsafe { CStr::from_ptr(token) }
                    .to_string_lossy()
                    .into_owned();
                let mut visible = true;
                if let Ok(mut buffer) = data.buffer.lock() {
                    buffer.push_str(&token);
                    if let Some(start) = buffer.find("<TRIGGER:") {
                        visible = false;
                        if let Some(end) = buffer.find("</TRIGGER>") {
                            let end = end + "</TRIGGER>".len();
                            let trigger = buffer[start..end].to_string();
                            let _ = data.sender.blocking_send(trigger);
                            data.cancel_flag.store(true, Ordering::Relaxed);
                            return false;
                        }
                    } else if buffer.len() > 256 {
                        let keep_from = buffer.len().saturating_sub(256);
                        *buffer = buffer[keep_from..].to_string();
                    }
                }

                if visible {
                    data.sender.blocking_send(token).is_ok()
                } else {
                    true
                }
            }

            let callback_data = Arc::new(CallbackData {
                sender: sender.clone(),
                cancel_flag: cancel_flag.clone(),
                buffer: std::sync::Mutex::new(String::new()),
            });
            let prompt = match CString::new(prompt) {
                Ok(prompt) => prompt,
                Err(error) => {
                    let _ = sender
                        .send(format!("Error: invalid prompt encoding: {error}"))
                        .await;
                    let _ = sender.send("\n[DONE]\n".to_string()).await;
                    return;
                }
            };

            let generate = unsafe {
                match load_symbol::<GenerateStreamFn>(
                    &library,
                    b"bitshit_kernel_generate_stream",
                    b"cluaiz_kernel_generate_stream",
                ) {
                    Ok(symbol) => *symbol,
                    Err(error) => {
                        let _ = sender.try_send(format!("Error: {error}"));
                        return;
                    }
                }
            };
            let callback_raw = Arc::into_raw(callback_data.clone()) as usize;
            let pointer_for_thread = pointer.clone();
            let library_for_thread = library.clone();
            let generation_result = tokio::task::spawn_blocking(move || {
                let _library_guard = library_for_thread;
                let status = unsafe {
                    generate(
                        pointer_for_thread.0,
                        prompt.as_ptr(),
                        4096,
                        callback,
                        callback_raw as *mut c_void,
                    )
                };
                unsafe {
                    drop(Arc::from_raw(callback_raw as *const CallbackData));
                }
                status
            })
            .await;

            let generation_status = match generation_result {
                Ok(status) => status,
                Err(error) => {
                    let _ = sender
                        .send(format!("Error: native generation task failed: {error}"))
                        .await;
                    -1
                }
            };

            if !cancel_flag.load(Ordering::Relaxed) {
                let trailing_trigger = callback_data.buffer.lock().ok().and_then(|buffer| {
                    buffer
                        .find("<TRIGGER:")
                        .map(|start| buffer[start..].to_string())
                });
                if let Some(trigger) = trailing_trigger {
                    if !trigger.is_empty() {
                        let _ = sender.try_send(trigger);
                    }
                }
            }

            if generation_status != 0 && !cancel_flag.load(Ordering::Relaxed) {
                let _ = sender
                    .send(format!(
                        "Error: Llama generation failed with code {}",
                        generation_status
                    ))
                    .await;
            }
            let _ = sender.send("\n[DONE]\n".to_string()).await;
        });

        EngineResponse::TokenStream(receiver)
    }

    pub async fn dispatch_prompt(&self, prompt: &str) -> Result<String> {
        let mut receiver = match self.dispatch_stream(prompt, false).await {
            EngineResponse::TokenStream(receiver) => receiver,
            EngineResponse::FinalResult(result) => return Ok(result),
            EngineResponse::Error(error) => return Err(anyhow!(error)),
        };
        let mut output = String::new();
        while let Some(token) = receiver.recv().await {
            if token.trim() == "[DONE]" {
                break;
            }
            output.push_str(&token);
        }
        Ok(output)
    }
}
