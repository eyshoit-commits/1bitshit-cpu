// cluaiz-engine: Core Foundry - The Cluaiz Engine Core
// Final integration of Registry, Intelligence, Runtime, and Security.

pub mod registry;
pub mod intelligence;
pub mod runtime;
pub mod security;

use registry::SkillRegistry;
use intelligence::skill_router::SkillRouter;
use runtime::wasm_host::WasmHost;
use runtime::mcp_gateway::McpGateway;
use security::guard::{PermissionGuard, PermissionLevel};
use tracing::{info, warn};
use cluaiz_shared::hardware::memory::kv_cache::stitching::CluaizSignal;
use neural_core::interfaces::memory_contract::MappedBuffer;
use std::sync::{Mutex, Arc};

const MAX_ACTIVE_SKILLS: usize = 3;

pub struct IntentResult {
    pub responses: Vec<String>,
    pub signals: Vec<CluaizSignal>,
}

pub struct CoreFoundry {
    pub registry: SkillRegistry,
    pub router: SkillRouter,
    pub wasm_runtime: WasmHost,
    pub mcp_gateway: McpGateway,
    pub guard: PermissionGuard,
    pub active_skill_ids: Mutex<Vec<String>>,
}

impl Default for CoreFoundry {
    fn default() -> Self {
        Self::new()
    }
}

impl CoreFoundry {
    pub fn new() -> Self {
        Self {
            registry: SkillRegistry::new(),
            router: SkillRouter::new(),
            wasm_runtime: WasmHost::new(),
            mcp_gateway: McpGateway::new(),
            guard: PermissionGuard::new(),
            active_skill_ids: Mutex::new(Vec::new()),
        }
    }

    /// Initializes the foundry by scanning the skills directory.
    pub fn initialize(&mut self, skills_dir: &str) {
        println!("[CLUAIZ] Initializing Core Foundry from: {}", skills_dir);
        self.registry.load_from_directory(skills_dir);
    }

    /// The Cluaiz Flow: Prompt -> Multi-Route -> Execute
    pub async fn process_intent(&self, prompt: &str) -> anyhow::Result<IntentResult> {
        let skill_ids = self.router.match_intent(prompt, &self.registry);
        let mut result = IntentResult { responses: Vec::new(), signals: Vec::new() };

        if skill_ids.is_empty() {
            return Ok(result);
        }

        info!("🧬 [CoreFoundry] Multi-Skill Fusion Active: {} skills detected.", skill_ids.len());

        for (i, skill_id) in skill_ids.iter().enumerate() {
            if i >= MAX_ACTIVE_SKILLS {
                warn!("⚠️ [CoreFoundry] Max active skills reached. Skipping remaining skills.");
                break;
            }

            // 1. Fetch skill from registry
            let skill = match self.registry.skills.iter().find(|s| &s.manifest.id == skill_id) {
                Some(s) => s,
                None => continue,
            };
            
            // 2. LRU Management
            {
                let mut active_ids = self.active_skill_ids.lock().unwrap();
                active_ids.retain(|id| id != skill_id);
                active_ids.push(skill_id.to_string());

                if active_ids.len() > MAX_ACTIVE_SKILLS {
                    let evicted_id = active_ids.remove(0);
                    println!("[CLUAIZ] [VRAM] Evicting LRU skill: {}", evicted_id);
                }
            }

            // 3. Map Cluaiz Signal (Zero-Copy)
            let kv_cache_path = skill.path.join("state.kv-cache");
            if kv_cache_path.exists() {
                if let Ok(mapped_buffer) = MappedBuffer::from_file(&kv_cache_path) {
                    result.signals.push(CluaizSignal {
                        raw_data: Arc::new(mapped_buffer),
                        token_count: skill.manifest.Core_metadata.token_count,
                        head_dim: skill.manifest.Core_metadata.head_dim,
                    });
                }
            }
            
            // 4. Logic execution (WASM)
            self.guard.validate_action(&skill.manifest, PermissionLevel::ReadOnly)?;
            let logic_path = skill.path.join("logic.wasm");
            if logic_path.exists() {
                match self.wasm_runtime.execute_skill_logic(&logic_path, "run", prompt).await {
                    Ok(resp) => result.responses.push(resp),
                    Err(e) => tracing::error!("⚠️ [CoreFoundry] Logic failed for '{}': {}", skill_id, e),
                }
            }
        }
        
        Ok(result)
    }
}
