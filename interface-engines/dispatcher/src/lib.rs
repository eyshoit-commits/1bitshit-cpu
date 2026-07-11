use anyhow::{anyhow, Result};
use cluaiz_shared::backend::signature::{BackendType, GlobalFeatureRegistry, KernelSignature};
use std::ffi::{c_char, c_void, CStr, CString};
use std::path::{Path, PathBuf};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use system_booster::BoosterControl;
use tokio::sync::mpsc;

const PRODUCT: &str = "1BitShit CPU";

type InstantiateFn = unsafe extern "C" fn(*const c_char, *const c_void) -> *mut c_void;
type FreeFn = unsafe extern "C" fn(*mut c_void);
type StreamCallback = extern "C" fn(*const c_char, *mut c_void) -> bool;
type GenerateStreamFn = unsafe extern "C" fn(
    *mut c_void,
    *const c_char,
    usize,
    StreamCallback,
    *mut c_void,
) -> i32;
type InitFn = unsafe extern "C" fn() -> *const c_char;
type GenerateEmbeddingFn = unsafe extern "C" fn(
    *mut c_void,
    *const c_char,
    *mut f32,
    usize,
    *mut usize,
) -> i32;

fn sanitized_model_id(id: &str) -> String {
    id.chars()
        .map(|character| match character {
            ':' | '/' | '\\' | ' ' => '-',
            other => other,
        })
        .collect::<String>()
        .trim_matches('-')
        .to_string()
}

fn configured_model_id(kind: &str) -> Option<String> {
    let environment = cluaiz_shared::environment::EnvironmentManager::current();
    let permission_path = environment.config_dir().join("Permission.json");
    let content = std::fs::read_to_string(permission_path).ok()?;
    let json: serde_json::Value = serde_json::from_str(&content).ok()?;
    json.get(kind)?
        .get("text")?
        .as_str()
        .map(str::to_string)
}

fn find_model_file(model_id: &str, accepted_extensions: &[&str]) -> Option<PathBuf> {
    let environment = cluaiz_shared::environment::EnvironmentManager::current();
    let models_root = environment.models_dir();
    let directory_name = sanitized_model_id(model_id);

    for category in ["chat", "embedding", "vision", "audio", "code", "multimodal"] {
        let directory = models_root.join(category).join(&directory_name);
        if let Some(path) = find_weight_in_directory(&directory, accepted_extensions) {
            return Some(path);
        }
    }

    find_weight_recursive(&models_root, &directory_name, accepted_extensions)
}

fn find_weight_in_directory(directory: &Path, accepted_extensions: &[&str]) -> Option<PathBuf> {
    let entries = std::fs::read_dir(directory).ok()?;
    entries
        .flatten()
        .map(|entry| entry.path())
        .find(|path| {
            path.is_file()
                && path
                    .extension()
                    .and_then(|extension| extension.to_str())
                    .map(str::to_ascii_lowercase)
                    .map(|extension| accepted_extensions.contains(&extension.as_str()))
                    .unwrap_or(false)
        })
}

fn find_weight_recursive(
    root: &Path,
    directory_name: &str,
    accepted_extensions: &[&str],
) -> Option<PathBuf> {
    for entry in std::fs::read_dir(root).ok()?.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        if path
            .file_name()
            .and_then(|name| name.to_str())
            .map(|name| name.eq_ignore_ascii_case(directory_name))
            .unwrap_or(false)
        {
            if let Some(weight) = find_weight_in_directory(&path, accepted_extensions) {
                return Some(weight);
            }
        }
        if let Some(weight) = find_weight_recursive(&path, directory_name, accepted_extensions) {
            return Some(weight);
        }
    }
    None
}

fn library_extension() -> &'static str {
    if cfg!(windows) {
        "dll"
    } else if cfg!(target_os = "macos") {
        "dylib"
    } else {
        "so"
    }
}

fn library_names(kind: &str) -> Vec<String> {
    let extension = library_extension();
    if cfg!(windows) {
        vec![
            format!("bitshit-{kind}.{extension}"),
            format!("bitshit_{kind}.{extension}"),
            format!("cluaiz-{kind}.{extension}"),
            format!("cluaiz_{kind}.{extension}"),
        ]
    } else {
        vec![
            format!("bitshit-{kind}.{extension}"),
            format!("libbitshit_{kind}.{extension}"),
            format!("libbitshit-{kind}.{extension}"),
            format!("cluaiz-{kind}.{extension}"),
            format!("libcluaiz_{kind}.{extension}"),
            format!("libcluaiz-{kind}.{extension}"),
        ]
    }
}

fn resolve_library(kind: &str) -> Result<PathBuf> {
    let names = library_names(kind);
    let environment = cluaiz_shared::environment::EnvironmentManager::current();
    let installed_dir = environment.engine_dir();
    let current_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

    for profile in ["release", "debug"] {
        let target = current_dir.join("target").join(profile);
        for name in &names {
            for candidate in [target.join(name), target.join("deps").join(name)] {
                if candidate.is_file() {
                    return Ok(candidate);
                }
            }
        }
    }

    for name in &names {
        for candidate in [installed_dir.join(name), installed_dir.join("drivers").join(name)] {
            if candidate.is_file() {
                return Ok(candidate);
            }
        }
    }

    Err(anyhow!(
        "{} {} library was not found under {} or target/{{release,debug}}",
        PRODUCT,
        kind,
        installed_dir.display()
    ))
}

unsafe fn load_symbol<'library, T>(
    library: &'library libloading::Library,
    primary: &[u8],
    legacy: &[u8],
) -> Result<libloading::Symbol<'library, T>> {
    library.get(primary).or_else(|_| library.get(legacy)).map_err(|error| {
        anyhow!(
            "Required ABI symbol '{}' is missing: {}",
            String::from_utf8_lossy(primary),
            error
        )
    })
}

unsafe fn open_library(path: &Path) -> Result<libloading::Library> {
    #[cfg(windows)]
    {
        let flags = 0x00000008;
        libloading::os::windows::Library::load_with_flags(path, flags)
            .map(libloading::Library::from)
            .map_err(|error| anyhow!("Failed to load {}: {}", path.display(), error))
    }
    #[cfg(not(windows))]
    {
        libloading::Library::new(path)
            .map_err(|error| anyhow!("Failed to load {}: {}", path.display(), error))
    }
}

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
            inference_semaphore: Arc::new(tokio::sync::Semaphore::new(4)),
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
                if let Ok(buffer) = callback_data.buffer.lock() {
                    if let Some(start) = buffer.find("<TRIGGER:") {
                        let trigger = buffer[start..].to_string();
                        if !trigger.is_empty() {
                            let _ = sender.send(trigger).await;
                        }
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
            drop(engine_guard);
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
