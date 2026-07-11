# Full 1BitShit CPU Rebrand Scope

This branch performs a complete identity migration without deleting runtime code or features.

## Preserved engines and capabilities

- Llama GGUF runtime
- ONNX runtime
- BitNet and low-bit GGUF support
- Chat API and chat entity types
- Model Hub, Hugging Face downloads and local manifests
- Lazy load, active-model selection and Auto-Pilot
- Hardware detection, telemetry and system booster
- CLI, TUI, dashboard, daemon and FFI surfaces

## New identity

- Product: `1BitShit CPU`
- Command: `bitshit`
- Home: `~/.1bitshit`
- Environment prefix: `BITSHIT_`
- Rust package aliases: `bitshit_shared`, `bitshit_api`, `bitshit_onnx`

Legacy names may exist only inside explicit migration readers or historical notices. They must not control active runtime paths, visible UI, downloader locations or executable selection.
