//! 🌉 Lucebox FFI Bindings
//! Extern "C" bindings for custom C++ CUDA kernels (DFlash, DDTree, SSM Convolutions, AtmaSteer).

use std::ffi::{c_void};
use std::os::raw::{c_int, c_float};
use libloading::{Library, Symbol};
use neural_core::SovereignBuffer;

/// 🧠 Raw C-API Declarations for Lucebox Kernels
type LuceboxInitFn = unsafe extern "C" fn() -> *mut c_void;
type LuceboxFreeFn = unsafe extern "C" fn(ctx: *mut c_void);

// AtmaSteer: Tier 1 KV-Stitching
type LuceboxKVStitchFn = unsafe extern "C" fn(
    ctx: *mut c_void,
    layer_idx: c_int,
    data_ptr: *const u8,
    data_len: usize
) -> c_int;

// DFlash DDTree Verification
type LuceboxDDTreeVerifyFn = unsafe extern "C" fn(
    ctx: *mut c_void, 
    draft_tokens: *const c_int, 
    num_drafts: c_int, 
    target_logits: *const c_float
) -> c_int;

// BitMamba / SSM Convolution 1D
type LuceboxSSMConv1DFn = unsafe extern "C" fn(
    ctx: *mut c_void,
    x: *const c_float,
    weight: *const c_float,
    bias: *const c_float,
    out: *mut c_float,
    seq_len: c_int,
    d_model: c_int
);

/// Safe Rust Wrapper for Lucebox Kernels
pub struct LuceboxBridge {
    _library: Library,
    ctx_ptr: *mut c_void,
    fn_kv_stitch: Symbol<'static, LuceboxKVStitchFn>,
    fn_ddtree_verify: Symbol<'static, LuceboxDDTreeVerifyFn>,
    fn_ssm_conv1d: Symbol<'static, LuceboxSSMConv1DFn>,
    fn_free: Symbol<'static, LuceboxFreeFn>,
}

impl LuceboxBridge {
    /// Loads the shared library containing custom Lucebox kernels
    pub fn load(lib_path: &str) -> anyhow::Result<Self> {
        unsafe {
            let lib = Library::new(lib_path)?;
            
            let init_sym: Symbol<LuceboxInitFn> = lib.get(b"lucebox_init\0")?;
            let ctx_ptr = init_sym();
            
            let fn_kv_stitch: Symbol<LuceboxKVStitchFn> = lib.get(b"lucebox_kv_stitch\0")?;
            let fn_ddtree_verify: Symbol<LuceboxDDTreeVerifyFn> = lib.get(b"lucebox_ddtree_verify\0")?;
            let fn_ssm_conv1d: Symbol<LuceboxSSMConv1DFn> = lib.get(b"lucebox_ssm_conv1d\0")?;
            let fn_free: Symbol<LuceboxFreeFn> = lib.get(b"lucebox_free\0")?;

            // Extend lifetimes to static for struct storage (safe because Library is held in struct)
            let fn_kv_stitch = std::mem::transmute(fn_kv_stitch);
            let fn_ddtree_verify = std::mem::transmute(fn_ddtree_verify);
            let fn_ssm_conv1d = std::mem::transmute(fn_ssm_conv1d);
            let fn_free = std::mem::transmute(fn_free);

            Ok(Self {
                _library: lib,
                ctx_ptr,
                fn_kv_stitch,
                fn_ddtree_verify,
                fn_ssm_conv1d,
                fn_free,
            })
        }
    }

    /// 💉 AtmaSteer: Direct Tensor Stitching
    /// Injects a pre-computed skill tensor into a specific attention layer.
    pub fn stitch_kv_layer(&self, layer_idx: i32, buffer: &dyn SovereignBuffer) -> anyhow::Result<()> {
        let result = unsafe {
            (self.fn_kv_stitch)(
                self.ctx_ptr,
                layer_idx as c_int,
                buffer.as_ptr(),
                buffer.len()
            )
        };

        if result == 0 {
            Ok(())
        } else {
            Err(anyhow::anyhow!("❌ Lucebox: KV-Stitching failed for layer {}", layer_idx))
        }
    }

    /// Safely executes DDTree Verification for DFlash
    pub fn verify_ddtree(&self, draft_tokens: &[i32], target_logits: &[f32]) -> i32 {
        unsafe {
            (self.fn_ddtree_verify)(
                self.ctx_ptr,
                draft_tokens.as_ptr(),
                draft_tokens.len() as c_int,
                target_logits.as_ptr(),
            )
        }
    }

    /// Safely executes hardware-native SSM Convolution
    pub fn ssm_conv1d(&self, x: &[f32], weight: &[f32], bias: &[f32], out: &mut [f32], d_model: usize) {
        let seq_len = x.len() / d_model;
        unsafe {
            (self.fn_ssm_conv1d)(
                self.ctx_ptr,
                x.as_ptr(),
                weight.as_ptr(),
                bias.as_ptr(),
                out.as_mut_ptr(),
                seq_len as c_int,
                d_model as c_int,
            )
        }
    }
}

unsafe impl Send for LuceboxBridge {}
unsafe impl Sync for LuceboxBridge {}

impl Drop for LuceboxBridge {
    fn drop(&mut self) {
        if !self.ctx_ptr.is_null() {
            unsafe {
                (self.fn_free)(self.ctx_ptr);
            }
        }
    }
}
