# 🏛️ CLUAIZ-OS: THE SOVEREIGN SYSTEM DESIGN

This document serves as the **Single Source of Truth** for the Cluaiz Neural Ecosystem. It defines the architectural DNA, industrial standards, and universal deployment laws that govern every line of code in this repository.

---

## 🛰️ 1. THE VISION: UNIVERSAL NEURAL SOVEREIGNTY
Cluaiz-OS is designed to be a **Universal Neural Kernel**. Our mission is to provide high-performance, native inference for any model, on any silicon, under any operating system, eliminating hardware boundaries.

### 🚀 Core Directives:
- **Silicon Mastery**: Extract peak performance from CPU, GPU, NPU, and TPU natively.
- **Hardware Agnosticism**: Unified execution across Windows, Linux, Android, iOS, and macOS.
- **Modular Integrity**: Decoupled architecture where components communicate via standardized FFI handshakes.

---

## 🧬 2. MODULAR ARCHITECTURE (THE NEURAL STACK)

The ecosystem is divided into four sovereign, decoupled layers. Any change to one layer MUST NOT break the handshake of others.

| Layer | Component | Responsibility |
| :--- | :--- | :--- |
| **Edge** | `cluaiz-cli` | User interface, model management, and system orchestration. |
| **Brain** | `cluaiz-engine` | The core orchestrator. Manages memory, context, and driver dispatch. |
| **Kernel** | `cluaiz-kernel` | Base CPU/SIMD interpreters (AVX512, AVX2, NEON) compiled for 9 operating system targets. |
| **Bridge** | `cluaiz-driver` | Specialized hardware/GPU drivers (CUDA, Metal, Vulkan, OpenVINO, ROCm, HIP). |

---

## 🛰️ 3. THE UNIVERSAL MATRIX LAW
Cluaiz-OS MUST run everywhere. The baseline CPU kernels and drivers support:

- **Windows**: x64 (Desktop/Surface/Server).
- **Linux**: x86_64 (Server), aarch64 (Cloud/Edge), armv7 (IoT).
- **Android**: aarch64 (Mobile/Tablet/Auto).
- **macOS**: arm64 (Apple Silicon), x86_64 (Intel Mac).
- **iOS**: arm64 (iPhone/iPad).

---

## 💎 4. THE NAMING & VERSIONING CONSTITUTION
To ensure zero-latency binary mapping, all artifacts MUST follow the **Sovereign Naming Convention**:

### 📦 Baseline CPU Kernels:
`cluaiz-kernel-<version>-<platform>.<ext>`
- **Platform**: Matches the 9 core OS targets (e.g., `win-x64-avx512`, `linux-x64-avx2`, `linux-arm64`, `mac-arm64`, etc.).
- **Releases**: Pushed to `kernel-v*` release tags.

### 🔌 Specialized Accelerator Drivers:
`cluaiz-driver-<version>-<platform>-<backend>.<ext>`
- **Backend**: Specialized silicon modules (e.g., `cuda-v13`, `cuda-v12`, `cuda-v11`, `metal`, `vulkan`, `openvino`, `rocm`, `hip`).
- **Releases**: Pushed to `driver-v*` release tags and indexed in `registry.json`.

---

## ⚡ 5. CI/CD PIPELINE INTEGRITY (ZERO-CRASH DEPLOYMENT)
The GitHub Actions pipelines are divided into two distinct, isolated factories:

### ⚙️ 1. Baseline Inference Kernels (`inference-kernel.yml`)
- **Compilation**: Parallel builds for exactly 9 core platforms using CPU instructions (AVX512, AVX2, NEON, ARM_NEON).
- **Tooling**: Uses `cross` for Docker-based cross-compilation on target architectures (Android, Linux Aarch64).
- **Releases**: Uploads baseline library binaries to `kernel-v*` release tags.

### ⚙️ 2. Dynamic Silicon Drivers (`inference-driver.yml`)
- **Compilation**: Parallel builds for specialized backend matrices (including CUDA v13/v12/v11, Metal mac-arm64/mac-x64/ios, Vulkan Windows/Linux, OpenVINO, ROCm, HIP).
- **Synchronization**: Automatically rewrites `registry.json` placeholders with the compiled release's `{DRIVER_TAG}` and `{VERSION}` to guarantee immediate auto-update updates.
- **Releases**: Uploads accelerator driver library binaries to `driver-v*` release tags.

---

## 🏛️ 6. THE FOUNDER'S MANDATE
1. **Never Drift**: Do not change naming conventions or matrix structures once established.
2. **Standard over Ad-hoc**: Every fix must be architectural, not a "Kach-Khas" (quick-fix).
3. **Total Coverage**: A build is only successful if ALL platforms in the matrix pass.

**This is the Cluaiz Standard. Professional. Optimized. Sovereign.**
