# 1BitShit CPU

**1BitShit CPU** is the CPU-first inference runtime for the BKG 1BitShit project. It combines a Rust runtime with a patched `llama.cpp` backend for GGUF, BitNet and ternary-model experiments.

The repository started as Cluaiz. Public product surfaces are being migrated to 1BitShit while existing `cluaiz_*` Rust crates, FFI symbols and data layouts remain temporarily available as a compatibility layer.

## Current scope

- CPU-only `llama.cpp` build
- GGUF model loading and streaming generation
- BitNet/TQ2 compatibility patches
- OpenMP CPU execution
- model and hardware metadata discovery
- KV-cache persistence
- terminal UI and headless commands
- optional ONNX support for non-text workloads
- CEL/WASM/plugin infrastructure inherited from the original runtime

## Status

This is an active engineering build, not a finished binary distribution. The CPU backend builds successfully on the current development server, but changes should still pass a release build and smoke test before merging.

The low-bit patches are experimental. Performance and numerical claims require reproducible benchmarks against pinned upstream sources and known models.

## Build

Requirements:

- Rust stable toolchain
- CMake
- Git
- GCC or Clang with C++ support
- GNU OpenMP (`libgomp`) on Linux

Build the complete release workspace:

```bash
cargo build --release
```

Build only the CPU Llama driver:

```bash
cargo run -p cluaiz-builder -- driver llama --profile release
```

The primary CLI binary is:

```text
target/release/bitshit
```

## Usage

Open the terminal interface:

```bash
bitshit
```

Inspect hardware:

```bash
bitshit status
```

Rebuild the hardware profile:

```bash
bitshit calibrate
```

Download a GGUF model:

```bash
bitshit pull <model-id>
```

Run a model:

```bash
bitshit run <model-id>
```

Start the local API:

```bash
bitshit serve
```

## CPU build contract

The Llama driver explicitly disables CUDA, Metal, Vulkan, ROCm/HIP, OpenVINO, SYCL, QNN and CANN. The CPU backend and OpenMP remain enabled.

GPU-related modules still exist in the wider codebase for future hybrid editions, but they are not part of the CPU driver build contract.

## Upstream handling

The build currently obtains `llama.cpp` during compilation. A production release must pin an audited upstream commit and verify patch preconditions before modifying upstream sources. Building against a moving `master` branch is intentionally listed as unresolved technical debt.

## Compatibility policy

During the first rebranding phase:

- product name: `1BitShit CPU`
- primary command: `bitshit`
- repository: `eyshoit-commits/1bitshit-cpu`
- existing `cluaiz_*` crate and FFI names remain valid
- existing local data directories are not automatically destroyed or silently abandoned

Internal namespace migration will be performed separately so dynamic drivers and persisted installations do not break in one theatrical explosion.

## Safety changes

Persisted hardware and booster state is decoded with safe `bincode` deserialization. Archived model DNA is validated before access. Mutable files are never passed directly to unchecked `rkyv::archived_root()`.

The Llama FFI boundary checks required pointers before dereferencing them, and driver initialization does not permanently redirect the host process's standard output streams.

## Repository layout

```text
cmd/                              bitshit CLI and TUI
interface-engines/llama/          CPU llama.cpp driver
interface-engines/neural_core/    shared inference contracts
Inference-engine/engines/         orchestration runtime
Inference-engine/engines/
  cluaiz-shared/                  compatibility-layer shared crate
inference-drivers/                driver discovery and resolution
inference-cel/                    CEL and WASM execution
tools/cluaize-builder/            build orchestrator (legacy crate name)
```

## Next engineering milestones

1. Pin the exact `llama.cpp` commit.
2. Convert source mutations into deterministic, checked patches.
3. Add a CPU smoke test with a small known GGUF model.
4. Benchmark TQ2/ternary kernels against unpatched llama.cpp.
5. Version the FFI ABI explicitly.
6. Continue internal namespace migration without breaking driver compatibility.

## License

Apache-2.0. See [LICENSE](LICENSE).
