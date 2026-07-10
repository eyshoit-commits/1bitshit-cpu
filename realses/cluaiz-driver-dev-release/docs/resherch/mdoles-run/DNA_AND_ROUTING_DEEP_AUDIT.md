# 📂 SOVEREIGN ARCHITECTURAL AUDIT (V1.0.0)
## 🎯 SCOPE: Neural DNA Resilience & Autonomous Dispatch

### 1. 🔍 INTERNAL GAP ANALYSIS (ARCHER KERNEL)
The current implementation of the Archer Neural Kernel suffers from **"Logical Isolation"** during the acquisition phase.

| **Component** | **Current Behavior** | **Fatal Flaw** |
| :--- | :--- | :--- |
| **ModelDownloader** | Dumps a skeleton `structural_dna.json` with `null` fields. | No Truth-Grounding. System guesses parameters until first load. |
| **AutonomousDiscovery** | Blindly reads existing JSON files. | If DNA is corrupted or null, the loader enters a "Guess State" which causes illegal memory access (Runtime Panic). |
| **GlobalFeatureRegistry** | Static mapping (hardcoded to Runtime B). | Cannot distinguish between BitNet (1-bit) and Standard Llama (4-bit) without pre-loading model weights. |

### 2. 🏛️ COMPETITOR AUDIT: OLLAMA / LLAMA.CPP
Ollama enforces a **"Binary-First"** architecture through `llama-model-loader.cpp`.

- **Truth Table**: Uses a comprehensive map of `general.architecture` to specific tensor expectations (e.g., `LLM_ARCH_BITNET`, `LLM_ARCH_MAMBA`).
- **Resilient Discovery**: Metadata is read **the moment** the file is opened. It doesn't rely on side-car JSON files for structural truth.
- **Dispatch Logic**: Ollama selects the compute backend (Metal/CUDA/Vulkan) based on the quantization type and architecture signature found in the KV headers.

### 3. 🐍 COMPETITOR AUDIT: vLLM (TURBO ENGINE)
vLLM utilizes **PagedAttention** and **Fused Kernels** to bypass the DNA layer entirely.

- **Dynamic Sharding**: `_get_all_gguf_files` detects multi-file GGUF shards automatically, ensuring large models don't fail due to missing parts.
- **Truth Mapping**: Uses `gguf.get_tensor_name_map` to dynamically resolve tensor identities across different architectures (Llama, Gemma, Mamba).
- **Auto-Config Modification**: Updates internal model configuration (e.g., `tie_word_embeddings`) based on the actual tensors discovered in the binary.
- **DNA Evolution**: vLLM doesn't use a "static DNA". It builds an **Execution Graph** dynamically by probing the weight shapes in real-time.

---

## 🏛️ THE SOVEREIGN COMPROMISE (PROPOSED PATH)
Archer must adopt a **"Handshake Protocol"** during the download/index cycle.

### Phase A: Post-Aquisition Intelligence (PAI)
Immediately after a GGUF is downloaded:
1.  **Binary Probe**: Open the `.gguf` and read the KV Metadata.
2.  **DNA Injection**: Overwrite the `null` values in `structural_dna.json` with binary-verified truth (layer count, heads, dimension).
3.  **Signature Sealing**: Mark the `KernelSignature` as `is_bitnet = true` if architecture matchesBitNet.

### Phase B: Autonomous Dispatch Optimizer
The `StructuralDNA` must contain a `preferred_runtime` calculated via a weighted priority:
- **PRIORITY 1**: If BitNet -> Strictly `Runtime C`.
- **PRIORITY 2**: If standard Llama/Gemma -> `Runtime A` (Candle/Sovereign) for speed.
- **PRIORITY 3**: If complex/unsupported architecture -> `Runtime B` (Llama.cpp Fallback).

---

## 📜 AUDIT CONCLUSION
The `null` DNA issue is the primary cause of hardware mismatch panics. Fixed DNA will allow **zero-latency model switching** because the hardware can be pre-configured before the weights even leave the disk.
