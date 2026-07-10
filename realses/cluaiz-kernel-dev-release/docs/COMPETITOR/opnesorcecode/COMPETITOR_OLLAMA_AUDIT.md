# Sovereign Audit: Ollama (Sovereign Local Serving)

This document provides a deep-dive into **Ollama's** hardware orchestration logic. Ollama's focus on **User Accessibility** and **Zero-Configuration** stability makes it the gold standard for "Agnostic Silicon Support".

## 🧠 Core Pillar: Dynamic Symbol Loading (The Linker Fix)

Ollama's biggest architectural win is the **Dynamic Loader**. It does not link against `nvml` or `cuda` at compile-time.

- **Mechanism**: Use `dlsym` (Linux) and `GetProcAddress` (Windows) to find driver functions ONLY if the hardware is present.
- **Impact**: Resolves all `LNK1136` (Invalid/Corrupt) and "DLL Missing" errors.
- **Archer Action**: We have already adopted this in `gpu.rs`. We must expand this to Vulkan/OpenCL fallback.

## 🧬 Layer Offloading (The Hybrid Engine)

Ollama intelligently splits LLM layers between the GPU and CPU.

- **Granular Control**: Users can specify `num_gpu` layers.
- **Auto-Detect**: Ollama calculates available VRAM and automatically pushes the maximum possible layers to the GPU, leaving the rest for the CPU.
- **Impact**: Ensures that even on low-end hardware, the model *runs* instead of crashing.

## 🔌 Hardware Probing (The Multi-OS Shield)

Ollama uses build tags and specialized source files for every OS.

- **Linux**: Probes `/sys/class/drm` and `/proc`.
- **Windows**: Uses `DXGI` and `WMI`.
- **MacOS**: Uses `Metal` and `sysctl`.
- **Impact**: 100% "World-Wide" coverage.

## 🏁 Best Practices for Archer V8.0

1. **Incremental Offloading**: Implement a "Layer Balancing" algorithm in `scheduler.rs`.
2. **Dynamic Fallback**: If NVIDIA detection fails, immediately probe for AMD (ROCm) then Intel (Vulkan).
3. **Environment Injection**: Support `OLLAMA_` style environment variables for power-users (e.g., `ARCHER_GPU_OVERRIDE`).
