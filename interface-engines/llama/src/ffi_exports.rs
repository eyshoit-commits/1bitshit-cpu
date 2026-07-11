use super::*;

#[no_mangle]
pub extern "C" fn cluaiz_kernel_init() -> *const std::os::raw::c_char {
    unsafe {
        extern "C" fn verbose_log(
            _level: i32,
            text: *const std::os::raw::c_char,
            _data: *mut std::ffi::c_void,
        ) {
            if text.is_null() {
                return;
            }
            let text = unsafe { std::ffi::CStr::from_ptr(text) }.to_string_lossy();
            eprint!("{}", text);
        }
        crate::ffi::llama_cpp::llama_log_set(Some(verbose_log), std::ptr::null_mut());
        ffi::llama_cpp::llama_backend_init();

        #[cfg(feature = "cuda")]
        {
            let registry = ffi::llama_cpp::ggml_backend_cuda_reg();
            if !registry.is_null() {
                ffi::llama_cpp::ggml_backend_register(registry);
                tracing::info!("[1BitShit Llama] CUDA backend registered");
            }
        }
    }
    tracing::info!("[1BitShit Llama] Backend initialized");
    "bitshit-llama-active\0".as_ptr() as *const std::os::raw::c_char
}

#[used]
static FORCE_KEEP_INIT: extern "C" fn() -> *const std::os::raw::c_char = cluaiz_kernel_init;

#[no_mangle]
pub extern "C" fn cluaiz_kernel_instantiate(
    path_ptr: *const std::os::raw::c_char,
    booster_ptr: *const cluaiz_shared::hardware::schema::booster::cluaizBoosterContext,
) -> *mut RuntimeB {
    if path_ptr.is_null() {
        tracing::error!("[1BitShit Llama] Null model path received");
        return std::ptr::null_mut();
    }

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let path = unsafe { std::ffi::CStr::from_ptr(path_ptr) }
            .to_string_lossy()
            .into_owned();
        let model_path = std::path::Path::new(&path);
        let model_dir = model_path.parent().unwrap_or(model_path);

        let mut dna = cluaiz_shared::metadata::dna::StructuralDNA::load(
            &model_dir.join("structural_dna.json"),
        )
        .unwrap_or_default();
        if let Err(error) = dna.discover_from_path(model_dir) {
            tracing::warn!("[1BitShit Llama] DNA discovery failed: {}", error);
        }

        let context = cluaizContext::boot(dna, cluaiz_shared::TemplateManager::default());
        let mut engine = Box::new(RuntimeB::new(&path, context));

        if !booster_ptr.is_null() {
            let booster = unsafe { *booster_ptr };
            engine.booster.flash_attn = booster.flash_attention;
            engine.booster.n_gpu_layers = booster.n_gpu_layers;
            engine.booster.turbo_quant = if booster.turbo_quant {
                "active".to_string()
            } else {
                "none".to_string()
            };
            engine.booster.kv_cache_quantization = match booster.kv_cache_quantization_mode {
                1 => "Kv8".to_string(),
                2 => "Kv4".to_string(),
                _ => "Auto".to_string(),
            };
            engine.booster.context_shifting = match booster.context_shifting_mode {
                0 => "Off".to_string(),
                1 => "Minimal".to_string(),
                2 => "Standard".to_string(),
                3 => "Aggressive".to_string(),
                4 => "Extreme".to_string(),
                _ => "Auto".to_string(),
            };
            engine.booster.speculative_decoding = match booster.speculative_decoding_mode {
                0 => "Off".to_string(),
                1 => "On".to_string(),
                _ => "Auto".to_string(),
            };
            engine.booster.use_mmap = true;
            if booster.max_context_length > 0 {
                engine.context.dna.max_context_length =
                    Some(booster.max_context_length as usize);
            }
        } else if let Ok(booster) =
            cluaiz_shared::hardware::governor::HardwareGovernor::load_booster_settings()
        {
            let _ = engine.apply_booster(&booster);
        }

        if let Err(error) = engine.load_native() {
            tracing::error!("[1BitShit Llama] Native model load failed: {}", error);
            return std::ptr::null_mut();
        }

        Box::into_raw(engine)
    }));

    result.unwrap_or_else(|_| {
        tracing::error!("[1BitShit Llama] Panic caught at instantiate boundary");
        std::ptr::null_mut()
    })
}

#[no_mangle]
pub extern "C" fn cluaiz_kernel_generate_stream(
    engine_ptr: *mut RuntimeB,
    prompt_ptr: *const std::os::raw::c_char,
    max_tokens: usize,
    callback: extern "C" fn(
        *const std::os::raw::c_char,
        *mut std::ffi::c_void,
    ) -> bool,
    user_data: *mut std::ffi::c_void,
) -> i32 {
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        if engine_ptr.is_null() || prompt_ptr.is_null() {
            return -1;
        }
        let engine = unsafe { &mut *engine_ptr };
        let prompt = unsafe { std::ffi::CStr::from_ptr(prompt_ptr) }
            .to_string_lossy()
            .into_owned();
        let user_data_address = user_data as usize;
        let callback_address = callback as usize;

        let rust_callback = Box::new(move |token: String| -> bool {
            let token = std::ffi::CString::new(token).unwrap_or_default();
            let callback: extern "C" fn(
                *const std::os::raw::c_char,
                *mut std::ffi::c_void,
            ) -> bool = unsafe { std::mem::transmute(callback_address) };
            callback(token.as_ptr(), user_data_address as *mut std::ffi::c_void)
        });

        match engine.generate_stream(&prompt, max_tokens, rust_callback) {
            Ok(_) => 0,
            Err(error) => {
                tracing::error!("[1BitShit Llama] Generation failed: {}", error);
                -2
            }
        }
    }));

    result.unwrap_or_else(|_| {
        tracing::error!("[1BitShit Llama] Panic caught at stream boundary");
        -3
    })
}

#[no_mangle]
pub extern "C" fn cluaiz_kernel_free(engine_ptr: *mut RuntimeB) {
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        if !engine_ptr.is_null() {
            unsafe {
                drop(Box::from_raw(engine_ptr));
            }
        }
    }));
    if result.is_err() {
        tracing::error!("[1BitShit Llama] Panic caught while freeing engine");
    }
}

#[no_mangle]
pub extern "C" fn cluaiz_kernel_set_skip_ptr(
    pointer: *const std::sync::atomic::AtomicBool,
) {
    unsafe {
        crate::native::stream::SKIP_PTR = pointer;
    }
}

#[no_mangle]
pub extern "C" fn cluaiz_kernel_dump_kv_cache(
    engine_ptr: *mut RuntimeB,
    path_ptr: *const std::os::raw::c_char,
) -> i32 {
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        if engine_ptr.is_null() || path_ptr.is_null() {
            return -1;
        }

        let path = unsafe { std::ffi::CStr::from_ptr(path_ptr) }
            .to_string_lossy()
            .into_owned();
        let engine = unsafe { &mut *engine_ptr };
        let Some(native) = engine.native.as_ref() else {
            return -4;
        };
        if native.ctx_ptr.is_null() {
            return -3;
        }

        let path = std::ffi::CString::new(path).unwrap_or_default();
        let bytes_written = unsafe {
            if engine.last_prefilled_tokens.is_empty() {
                crate::ffi::llama_cpp::llama_state_seq_save_file(
                    native.ctx_ptr,
                    path.as_ptr(),
                    0,
                    std::ptr::null(),
                    0,
                )
            } else {
                crate::ffi::llama_cpp::llama_state_seq_save_file(
                    native.ctx_ptr,
                    path.as_ptr(),
                    0,
                    engine.last_prefilled_tokens.as_ptr(),
                    engine.last_prefilled_tokens.len(),
                )
            }
        };
        if bytes_written > 0 { 0 } else { -2 }
    }));
    result.unwrap_or(-5)
}

#[no_mangle]
pub extern "C" fn cluaiz_kernel_load_kv_cache(
    engine_ptr: *mut RuntimeB,
    path_ptr: *const std::os::raw::c_char,
) -> i32 {
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        if engine_ptr.is_null() || path_ptr.is_null() {
            return -1;
        }

        let path = unsafe { std::ffi::CStr::from_ptr(path_ptr) }
            .to_string_lossy()
            .into_owned();
        let engine = unsafe { &mut *engine_ptr };
        let Some(native) = engine.native.as_ref() else {
            return -4;
        };
        if native.ctx_ptr.is_null() {
            return -3;
        }

        let path = std::ffi::CString::new(path).unwrap_or_default();
        let mut tokens = vec![0_i32; native.n_ctx as usize];
        let mut token_count = 0_usize;
        let bytes_read = unsafe {
            crate::ffi::llama_cpp::llama_state_seq_load_file(
                native.ctx_ptr,
                path.as_ptr(),
                0,
                tokens.as_mut_ptr(),
                tokens.len(),
                &mut token_count,
            )
        };
        if bytes_read > 0 {
            engine.last_prefilled_tokens = tokens[..token_count].to_vec();
            0
        } else {
            -2
        }
    }));
    result.unwrap_or(-5)
}

// New official ABI.
#[no_mangle]
pub extern "C" fn bitshit_kernel_init() -> *const std::os::raw::c_char {
    cluaiz_kernel_init()
}

#[no_mangle]
pub extern "C" fn bitshit_kernel_instantiate(
    path_ptr: *const std::os::raw::c_char,
    booster_ptr: *const cluaiz_shared::hardware::schema::booster::cluaizBoosterContext,
) -> *mut RuntimeB {
    cluaiz_kernel_instantiate(path_ptr, booster_ptr)
}

#[no_mangle]
pub extern "C" fn bitshit_kernel_generate_stream(
    engine_ptr: *mut RuntimeB,
    prompt_ptr: *const std::os::raw::c_char,
    max_tokens: usize,
    callback: extern "C" fn(
        *const std::os::raw::c_char,
        *mut std::ffi::c_void,
    ) -> bool,
    user_data: *mut std::ffi::c_void,
) -> i32 {
    cluaiz_kernel_generate_stream(
        engine_ptr,
        prompt_ptr,
        max_tokens,
        callback,
        user_data,
    )
}

#[no_mangle]
pub extern "C" fn bitshit_kernel_free(engine_ptr: *mut RuntimeB) {
    cluaiz_kernel_free(engine_ptr)
}

#[no_mangle]
pub extern "C" fn bitshit_kernel_set_skip_ptr(
    pointer: *const std::sync::atomic::AtomicBool,
) {
    cluaiz_kernel_set_skip_ptr(pointer)
}

#[no_mangle]
pub extern "C" fn bitshit_kernel_dump_kv_cache(
    engine_ptr: *mut RuntimeB,
    path_ptr: *const std::os::raw::c_char,
) -> i32 {
    cluaiz_kernel_dump_kv_cache(engine_ptr, path_ptr)
}

#[no_mangle]
pub extern "C" fn bitshit_kernel_load_kv_cache(
    engine_ptr: *mut RuntimeB,
    path_ptr: *const std::os::raw::c_char,
) -> i32 {
    cluaiz_kernel_load_kv_cache(engine_ptr, path_ptr)
}
