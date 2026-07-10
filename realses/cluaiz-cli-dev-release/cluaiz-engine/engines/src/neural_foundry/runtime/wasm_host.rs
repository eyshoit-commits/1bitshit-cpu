use anyhow::Result;
use std::path::Path;
#[cfg(feature = "wasm-runtime")]
use wasmtime::*;
#[cfg(feature = "wasm-runtime")]
use wasmtime_wasi::preview1::{self, WasiP1Ctx};
#[cfg(feature = "wasm-runtime")]
use wasmtime_wasi::WasiCtxBuilder;
// TODO: Restore once CoreGraph is implemented in archer_shared
// use cluaiz_shared::Core::graph::CoreGraph;

use std::sync::Mutex;

#[cfg(feature = "wasm-runtime")]
pub struct WasmHost {
    engine: Engine,
    result_pool: Mutex<Vec<u8>>,
}

#[cfg(not(feature = "wasm-runtime"))]
pub struct WasmHost {
    _dummy: Mutex<Vec<u8>>,
}

#[cfg(feature = "wasm-runtime")]
struct CluaizWasmState {
    wasi: WasiP1Ctx,
}

impl Default for WasmHost {
    fn default() -> Self {
        Self::new()
    }
}

impl WasmHost {
    #[cfg(feature = "wasm-runtime")]
    pub fn new() -> Self {
        let mut config = Config::new();
        config.async_support(true);
        Self {
            engine: Engine::new(&config).expect("Failed to create Wasmtime engine"),
            result_pool: Mutex::new(vec![0u8; 4096]), // Pre-allocated 4KB pool
        }
    }

    #[cfg(not(feature = "wasm-runtime"))]
    pub fn new() -> Self {
        Self {
            _dummy: Mutex::new(Vec::new()),
        }
    }

    /// Executes a WASM function with proper string ABI and WASI sandboxing.
    #[cfg(feature = "wasm-runtime")]
    pub async fn execute_skill_logic(
        &self,
        wasm_path: &Path,
        func_name: &str,
        params: &str,
    ) -> Result<String> {
        let module = Module::from_file(&self.engine, wasm_path).map_err(|e| {
            anyhow::anyhow!("WASM Module Load Error [{}]: {}", wasm_path.display(), e)
        })?;

        // 🧠 Mission 12: Chronicle Core Skill Initiation
        let skill_id = wasm_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown_skill");
        tracing::info!(
            "🧠 [CoreFoundry] Skill Execution Initiated: {} | Binary: {:?} | Func: {}",
            skill_id,
            wasm_path,
            func_name
        );

        // 🏗️ Cluaiz Sandbox: Restricted WASI Context
        let mut wasi_builder = WasiCtxBuilder::new();

        // 1. Inherit Stdio for kernel telemetry
        wasi_builder.inherit_stdout().inherit_stderr();

        // 2. Map Skill-Specific Virtual Directory (Cluaiz Immunity)
        if let Some(skill_root) = wasm_path.parent() {
            tracing::info!("🔒 [Sandbox] Mapping virtual jail: {:?}", skill_root);
            // We pre-open the skill's root directory as "." for the guest
            let _ = wasi_builder.preopened_dir(
                skill_root,
                ".",
                wasmtime_wasi::DirPerms::all(),
                wasmtime_wasi::FilePerms::all(),
            );
        }

        // 3. Inject Cluaiz-OS Environment (Cluaiz Context)
        wasi_builder.env("CLUAIZ_VERSION", "0.0.1");
        wasi_builder.env("Cluaiz_MODE", "ACTIVE");

        let wasi = wasi_builder.build_p1();

        let mut store = Store::new(&self.engine, CluaizWasmState { wasi });
        let mut linker = Linker::new(&self.engine);
        preview1::add_to_linker_async(&mut linker, |s: &mut CluaizWasmState| &mut s.wasi)?;

        // 🔗 Instantiate module
        let instance = linker.instantiate_async(&mut store, &module).await?;

        // 🧠 String Passing ABI: We use exported memory for communication
        let memory = instance
            .get_memory(&mut store, "memory")
            .ok_or_else(|| anyhow::anyhow!("WASM Memory 'memory' not found"))?;

        // 🏗️ 1. Malloc Handshake: Ask WASM for a safe buffer
        let param_bytes = params.as_bytes();
        let param_ptr =
            if let Ok(malloc) = instance.get_typed_func::<i32, i32>(&mut store, "malloc") {
                tracing::info!(
                    "🔗 [WasmHost] Malloc Handshake: Requesting {} bytes from guest...",
                    param_bytes.len()
                );
                malloc
                    .call_async(&mut store, param_bytes.len() as i32)
                    .await? as usize
            } else {
                tracing::warn!(
                    "⚠️ [WasmHost] No 'malloc' export found. Falling back to unsafe offset 0."
                );
                0_usize
            };

        // 📝 2. Write params to WASM memory
        memory.write(&mut store, param_ptr, param_bytes)?;

        // 🚀 3. Call the function
        let func = instance
            .get_func(&mut store, func_name)
            .ok_or_else(|| anyhow::anyhow!("WASM Function '{}' not found", func_name))?;

        // Call with pointer and length (Cluaiz ABI)
        let mut results = [Val::I32(0)];
        if let Err(e) = func
            .call_async(
                &mut store,
                &[
                    Val::I32(param_ptr as i32),
                    Val::I32(param_bytes.len() as i32),
                ],
                &mut results,
            )
            .await
        {
            tracing::error!(
                "🧠 [CoreFoundry] Skill Execution Failed: {} | Error: {}",
                skill_id,
                e
            );
            return Err(e);
        }

        // 📖 3. Read the result back (Pooled/Zero-Allocation)
        let res_ptr = match results[0] {
            Val::I32(p) => p as usize,
            _ => return Err(anyhow::anyhow!("Invalid return type from WASM")),
        };

        let mut pool = self
            .result_pool
            .lock()
            .map_err(|_| anyhow::anyhow!("Result pool poison"))?;
        memory.read(&mut store, res_ptr, &mut pool)?;

        let null_pos = pool.iter().position(|&b| b == 0).unwrap_or(pool.len());
        let response = String::from_utf8_lossy(&pool[..null_pos]).to_string();

        tracing::info!(
            "🧠 [CoreFoundry] Skill Execution Success: {} | Response: {} bytes",
            skill_id,
            response.len()
        );

        Ok(response)
    }

    #[cfg(not(feature = "wasm-runtime"))]
    pub async fn execute_skill_logic(
        &self,
        _wasm_path: &Path,
        _func_name: &str,
        _params: &str,
    ) -> Result<String> {
        Err(anyhow::anyhow!(
            "WASM Runtime is disabled on this platform."
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_Core_pulse_generation() {
        println!("🚀 Testing Cluaiz Core Pulse...");
        let activity = "Foundry Simulation Pulse";
        let skill_id = "test_skill_v1";

        let result = cluaiz_shared::Core::graph::CoreGraph::chronicle_pulse(
            activity,
            skill_id,
            "Metadata: [Simulation Mode Active]",
        );

        assert!(result.is_ok(), "Core Graph should be writable");
        println!("✅ Pulse Chronicled in thing.ai.nurale.md");
    }
}
