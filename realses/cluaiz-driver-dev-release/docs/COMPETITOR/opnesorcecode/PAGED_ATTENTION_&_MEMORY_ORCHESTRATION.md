# Sovereign Audit: PagedAttention & Memory Orchestration

This research paper analyzes the **vLLM PagedAttention** mechanism. This is a critical component for Archer's "Silicon Nichoding" mission, as it defines the gold standard for KV-cache memory utilization and fragmentation control.

## 🏛️ The "Virtual OS" Memory Model

vLLM's core innovation is treating GPU/System VRAM as a **Paged Virtual Memory** system. Traditional LLM engines allocate memory for the Key-Value (KV) cache in large, contiguous chunks. This leads to:
1. **Internal Fragmentation**: Reserved memory that is never used.
2. **External Fragmentation**: Free memory segments that are too small for new requests.

**vLLM Logic**:
- Memory is divided into **Logical Blocks** (e.g., 16 tokens per block).
- These map to **Physical Blocks** in a non-contiguous way.
- A **Block Manager** (Virtual Memory Manager) tracks which sequence owns which physical blocks.

## 🧬 Kernel-Level Optimization (The Math)

vLLM bypasses the overhead of standard deep-learning frameworks by using a custom query-attention kernel (`csrc/attention/attention_kernels.cu`).

### 1. Memory Coalescing (The Speed)
To saturate the GPU's memory bandwidth, threads are grouped into `THREAD_GROUP_SIZE`. 
- **Mechanism**: Threads fetch 16 bytes of data (Query/Key vecs) simultaneously.
- **Archer Insight**: In our Rust ISA Probes (V8.0), we must use **SIMD Vectorization** (AVX-512) to achieve similar coalescing on the CPU.

### 2. Register vs. Shared Memory
- **Query Data**: Stored in **Shared Memory** because it is accessed by multiple threads multiple times during the Dot Product.
- **Key Data**: Stored in **Register Memory** because it is only accessed once by a single thread group.
- **Result**: This reduces memory latency to nearly 0 by minimizing Global Memory round-trips.

## 🛡️ Throttling & Preemption

vLLM handles "Silicon Pressure" (Out of Memory) through **Preemption**.
- **Strategy**: When the hardware is full, vLLM "evicts" the blocks of the newest sequence (Saving them to CPU RAM) and resumes them later once VRAM is free.
- **Archer Action**: Our `telemetry.rs` (Ghost Observer) must trigger this "Offloading" to CPU when VRAM pressure > 95%.

## 🏁 Technical Blueprint for Archer V8.0

| Mechanism | Implementation Target | Goal |
| :--- | :--- | :--- |
| **Block Paging** | `memory.rs` | <1% Memory Fragmentation |
| **Vector Coalescing** | `isa_probe.rs` | 0.000ms Latency (SIMD) |
| **Dynamic Preemption** | `telemetry.rs` | Zero-Crash Inference (VRAM Safety) |

---
**Source Reference**: `vllm-main/docs/design/paged_attention.md`
**Status**: Deep Research Verified (Archer V8 Requirement).
