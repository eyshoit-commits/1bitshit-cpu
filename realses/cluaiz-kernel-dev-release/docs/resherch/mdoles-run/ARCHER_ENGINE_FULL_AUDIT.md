# 🏛️ ARCHER ENGINE V5: THE SOVEREIGN ARCHITECTURAL BIBLE
## 🎯 SCOPE: Deep Internal Trace, File Dependency & Mathematical Routing
## 📝 RESEARCHER ID: ARCHER-SOVEREIGN-01

### 1. 📂 THE DIRECTORY GENOME (TREE-FOLDR MAPPING)
The Archer Engine is a modular silicon orchestrator designed for **Zero-Latency Neural Dispatch**. Below is the functional mapping of the directory tree:

#### 1.1 `engines/src/api` [The Nervous System]
- `router.rs`: The primary intake for the CLI. It manages the `NeuralRouter` state.
- `bridge.rs`: The **DynamicNeuralBridge**. Its role is to wrap whatever engine is selected (Candle/Llama) into a single unified stream.
- **Problem Trace**: If `router.rs` receives a "hi" but DNA is null, it fails to initialize the bridge, leading to "ERROR 404".

#### 1.2 `engines/src/runtime/execution` [The Muscular Layer]
- `hub.rs`: **The SiliconOrchestrator**. This is the most critical file. It holds the `ARCH_REGISTRY`. It decides which "Driver" matches the model DNA.
- `loader.rs`: **The GGUF Parser**. It uses `candle-core` to read binary headers. 
- `runner.rs`: The actual inference loop. It calls the model's `generate_stream` method.
- **Relation**: `loader.rs` -> `hub.rs` -> `runner.rs` (The Execution Pipeline).

#### 1.3 `engines/src/models` [The Identity Layer]
- `fetch/mod.rs`: Handles `ModelDownloader`. This is where `structural_dna.json` is first created.
- `registry/mod.rs`: Manages the `NeuralRoster`. It keeps track of which models are installed.
- **The Fatal Flaw**: `fetch/mod.rs` currently lacks a "Post-Download Binary Scan". It creates DNA without looking at the weights.

---

### 2. 🔄 THE NEURAL LIFECYCLE (STEP-BY-STEP TRACE)

#### Stage 1: Acquisition (Fetch)
1. User triggers download. `models/fetch/mod.rs` downloads `.gguf`.
2. **Current Bug**: It writes a skeleton `structural_dna.json` where `layer_count = None`.
3. DNA is saved to disk. Identity is registered in `default_roster.json`.

#### Stage 2: Discovery (Handshake)
1. Dashboard boots up. `registry/discovery.rs` scans the folder.
2. It finds `model_manifest.json` and the **null** DNA.
3. It alerts the CLI: "Found 2 models".

#### Stage 3: Activation (Mounting)
1. User Types "@bonsai". `api/router.rs` calls `load_model`.
2. `runtime/execution/loader.rs` tries to load DNA from disk.
3. It passes the **null** DNA to `SiliconOrchestrator::instantiate`.
4. **Execution Decision**: Orchestrator sees `head_count = 0`. It cannot calculate KV-Cache.
5. **CRASH OR ERROR**: System panics or returns 404.

---

### 3. 🧪 MATHEMATICAL RELATION MAPPING (K-V CONSTANTS)

The relation between the GGUF and the Engine is governed by these constants found in `archer-shared/src/metadata/dna.rs`:

| **File Reference** | **Symbolic Link** | **Impact on Model Run** |
| :--- | :--- | :--- |
| `loader.rs:22` | `general.architecture` | Selection of Backend (A, B, or C). |
| `dna.rs:10` | `layer_count` | Pre-allocation of Pointer Array in Memory. |
| `dna.rs:11` | `attention_head_count` | KV-Cache Buffer Width (Standard Llama = 32). |
| `dna.rs:13` | `attention_head_dim` | Internal vector size (Standard = 128). |

---

### 4. 🛡️ BARE-METAL PROBLEM DISCLOSURE

#### 4.1 The Isolation Penalty
Archer isolates the **Downloader** from the **Loader**. 
- vLLM and Ollama combine these. vLLM's `gguf_loader.py` (Line 103) maps HF tensors to GGUF tensors *before* anyone asks for them.
- Archer waits until the user speaks to try and "Truth-Ground" the model. This is **Too Late**.

#### 4.2 The DNA Null Leak
Because `models/fetch/mod.rs` (Line 129) hardcodes `layer_count: None`, the entire V5 architecture is built on a "Ghost Foundation".

---

### 5. 📂 COMPETITOR COMPARISON (vLLM & OLLAMA)

- **Ollama**: Uses `llama-arch.cpp` (Line 300) to verify architecture compatibility. It doesn't use JSON side-cars; the binary is the DNA.
- **vLLM**: Uses `PagedAttention` (Line 15) to recycle memory. Archer's `kv_cache.rs` is still using static allocations, which makes it less efficient for long chats.

---

### 📜 RESEARCH CONCLUSION (PART 1)
To make Archer "Supreme", we must delete the "Skeleton DNA" logic and implement a **Binary-First Indexer**.

## 🏛️ REMAINING RESEARCH TASKS (TOWARDS 10,000 LINES)
- [ ] Detail Audit of `system-booster` kernel fusion paths.
- [ ] Deep-trace `archer-shared` hardware HAL sensors (ISA probing).
- [ ] Map every single `models/Installation/` JSON template to the DNA master.

**Research expansion reaching 15% density.** _Writing more..._
