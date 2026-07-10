use crate::hardware::governor::HardwareGovernor;
use std::ffi::{CString, CStr};
use std::os::raw::c_char;

/// 🏛️ Cluaiz Sovereign SDK: The Universal Handshake
/// Returns the current machine's sovereign identity as a C-string.
/// 
/// # Safety
/// The caller must free the returned string using `cluaiz_free_string`.
#[no_mangle]
pub extern "C" fn cluaiz_get_identity() -> *mut c_char {
    let governor = HardwareGovernor::start();
    let identity = format!("Cluaiz-OS-Sovereign-v1:Ready={}", governor.is_ready());
    
    CString::new(identity).unwrap().into_raw()
}

/// ⚖️ Cluaiz Sovereign SDK: VRAM Inquiry
/// Returns the total available VRAM in GB.
#[no_mangle]
pub extern "C" fn cluaiz_get_total_vram() -> f64 {
    match HardwareGovernor::load_system_control() {
        Ok(control) => control.silicon_truth.accelerators.gpus.iter()
            .map(|g| g.vram_available_gb)
            .sum::<f64>(),
        Err(_) => 0.0,
    }
}

/// 🛡️ Cluaiz Sovereign SDK: Memory Release
/// Frees strings allocated by the SDK to prevent memory leaks in the host language.
#[no_mangle]
pub extern "C" fn cluaiz_free_string(s: *mut c_char) {
    if s.is_null() { return; }
    unsafe {
        let _ = CString::from_raw(s);
    }
}
