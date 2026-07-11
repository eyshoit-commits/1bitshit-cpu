pub mod engine;
pub use engine::OnnxEngine;

pub mod audio;
pub mod chat;
pub mod text;
pub mod vision;

use std::ffi::{c_char, c_void};

#[no_mangle]
pub extern "C" fn bitshit_kernel_init() -> *const c_char {
    let _ = ort::init().with_name("bitshit_onnx_env").commit();
    tracing::info!("[1BitShit ONNX] Runtime initialized");
    "bitshit-onnx-active\0".as_ptr() as *const c_char
}

#[no_mangle]
pub extern "C" fn bitshit_kernel_instantiate(
    path_ptr: *const c_char,
    booster_ptr: *const cluaiz_shared::hardware::schema::booster::cluaizBoosterContext,
) -> *mut OnnxEngine {
    if path_ptr.is_null() {
        return std::ptr::null_mut();
    }

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let path = unsafe { std::ffi::CStr::from_ptr(path_ptr) }
            .to_string_lossy()
            .into_owned();
        let mut engine = match OnnxEngine::new() {
            Ok(engine) => Box::new(engine),
            Err(error) => {
                tracing::error!("[1BitShit ONNX] Engine creation failed: {}", error);
                return std::ptr::null_mut();
            }
        };

        let booster = if booster_ptr.is_null() {
            None
        } else {
            Some(unsafe { *booster_ptr })
        };

        if path != "default" && !path.is_empty() {
            let lower = path.to_ascii_lowercase();
            let load_result = if lower.contains("clip") || lower.contains("vision") {
                engine.load_vision_model(&path, booster)
            } else {
                let model_path = std::path::Path::new(&path);
                let directory = model_path.parent().unwrap_or(model_path);
                let tokenizer = directory.join("tokenizer.json");
                engine.load_text_model(&path, &tokenizer.to_string_lossy(), booster)
            };
            if let Err(error) = load_result {
                tracing::error!("[1BitShit ONNX] Model loading failed: {}", error);
                return std::ptr::null_mut();
            }
        }

        Box::into_raw(engine)
    }));

    result.unwrap_or_else(|_| {
        tracing::error!("[1BitShit ONNX] Panic caught at instantiate boundary");
        std::ptr::null_mut()
    })
}

#[no_mangle]
pub extern "C" fn bitshit_kernel_generate_embedding(
    engine_ptr: *mut OnnxEngine,
    text_ptr: *const c_char,
    output_ptr: *mut f32,
    max_len: usize,
    output_len: *mut usize,
) -> i32 {
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        if engine_ptr.is_null()
            || text_ptr.is_null()
            || output_ptr.is_null()
            || output_len.is_null()
        {
            return -1;
        }

        let engine = unsafe { &*engine_ptr };
        let text = unsafe { std::ffi::CStr::from_ptr(text_ptr) }.to_string_lossy();
        use neural_core::interfaces::router_contract::EmbeddingDriver;
        match engine.gen_embedding(&text) {
            Ok(vector) => {
                let length = vector.len().min(max_len);
                unsafe {
                    std::ptr::copy_nonoverlapping(vector.as_ptr(), output_ptr, length);
                    *output_len = length;
                }
                0
            }
            Err(error) => {
                tracing::error!("[1BitShit ONNX] Embedding failed: {}", error);
                -2
            }
        }
    }));
    result.unwrap_or(-3)
}

#[no_mangle]
pub extern "C" fn bitshit_kernel_generate_stream(
    engine_ptr: *mut OnnxEngine,
    prompt_ptr: *const c_char,
    max_tokens: usize,
    callback: extern "C" fn(*const c_char, *mut c_void) -> bool,
    user_data: *mut c_void,
) -> i32 {
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        if engine_ptr.is_null() || prompt_ptr.is_null() {
            return -1;
        }

        let engine = unsafe { &mut *engine_ptr };
        let prompt = unsafe { std::ffi::CStr::from_ptr(prompt_ptr) }.to_string_lossy();
        let callback_address = callback as usize;
        let user_data_address = user_data as usize;
        use cluaiz_shared::cluaizInference;

        let rust_callback = Box::new(move |token: String| -> bool {
            let token = std::ffi::CString::new(token).unwrap_or_default();
            let callback: extern "C" fn(*const c_char, *mut c_void) -> bool =
                unsafe { std::mem::transmute(callback_address) };
            callback(token.as_ptr(), user_data_address as *mut c_void)
        });

        match engine.generate_stream(&prompt, max_tokens, rust_callback) {
            Ok(_) => 0,
            Err(error) => {
                tracing::error!("[1BitShit ONNX] Stream generation failed: {}", error);
                -2
            }
        }
    }));
    result.unwrap_or(-3)
}

#[no_mangle]
pub extern "C" fn bitshit_kernel_dump_kv_cache(
    engine_ptr: *mut OnnxEngine,
    path_ptr: *const c_char,
) -> i32 {
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        if engine_ptr.is_null() || path_ptr.is_null() {
            return -1;
        }
        let engine = unsafe { &mut *engine_ptr };
        let path = unsafe { std::ffi::CStr::from_ptr(path_ptr) }.to_string_lossy();
        use cluaiz_shared::cluaizInference;
        match engine.dump_kv_cache(&path) {
            Ok(_) => 0,
            Err(error) => {
                tracing::error!("[1BitShit ONNX] KV cache dump failed: {}", error);
                -2
            }
        }
    }));
    result.unwrap_or(-3)
}

#[no_mangle]
pub extern "C" fn bitshit_kernel_load_kv_cache(
    engine_ptr: *mut OnnxEngine,
    path_ptr: *const c_char,
) -> i32 {
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        if engine_ptr.is_null() || path_ptr.is_null() {
            return -1;
        }
        let engine = unsafe { &mut *engine_ptr };
        let path = unsafe { std::ffi::CStr::from_ptr(path_ptr) }.to_string_lossy();
        use cluaiz_shared::cluaizInference;
        match engine.load_kv_cache(&path) {
            Ok(_) => 0,
            Err(error) => {
                tracing::error!("[1BitShit ONNX] KV cache load failed: {}", error);
                -2
            }
        }
    }));
    result.unwrap_or(-3)
}

#[no_mangle]
pub extern "C" fn bitshit_kernel_free(engine_ptr: *mut OnnxEngine) {
    if !engine_ptr.is_null() {
        unsafe {
            drop(Box::from_raw(engine_ptr));
        }
    }
}

// Legacy ABI forwards. They preserve existing native clients but do not control
// artifact names, directories, product text or runtime selection.
#[no_mangle]
pub extern "C" fn cluaiz_kernel_init() -> *const c_char {
    bitshit_kernel_init()
}

#[no_mangle]
pub extern "C" fn cluaiz_kernel_instantiate(
    path_ptr: *const c_char,
    booster_ptr: *const cluaiz_shared::hardware::schema::booster::cluaizBoosterContext,
) -> *mut OnnxEngine {
    bitshit_kernel_instantiate(path_ptr, booster_ptr)
}

#[no_mangle]
pub extern "C" fn cluaiz_kernel_generate_embedding(
    engine_ptr: *mut OnnxEngine,
    text_ptr: *const c_char,
    output_ptr: *mut f32,
    max_len: usize,
    output_len: *mut usize,
) -> i32 {
    bitshit_kernel_generate_embedding(engine_ptr, text_ptr, output_ptr, max_len, output_len)
}

#[no_mangle]
pub extern "C" fn cluaiz_kernel_generate_stream(
    engine_ptr: *mut OnnxEngine,
    prompt_ptr: *const c_char,
    max_tokens: usize,
    callback: extern "C" fn(*const c_char, *mut c_void) -> bool,
    user_data: *mut c_void,
) -> i32 {
    bitshit_kernel_generate_stream(
        engine_ptr,
        prompt_ptr,
        max_tokens,
        callback,
        user_data,
    )
}

#[no_mangle]
pub extern "C" fn cluaiz_kernel_dump_kv_cache(
    engine_ptr: *mut OnnxEngine,
    path_ptr: *const c_char,
) -> i32 {
    bitshit_kernel_dump_kv_cache(engine_ptr, path_ptr)
}

#[no_mangle]
pub extern "C" fn cluaiz_kernel_load_kv_cache(
    engine_ptr: *mut OnnxEngine,
    path_ptr: *const c_char,
) -> i32 {
    bitshit_kernel_load_kv_cache(engine_ptr, path_ptr)
}

#[no_mangle]
pub extern "C" fn cluaiz_kernel_free(engine_ptr: *mut OnnxEngine) {
    bitshit_kernel_free(engine_ptr)
}
