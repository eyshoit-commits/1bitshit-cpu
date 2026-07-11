# BitShit rebranding status

Last reviewed: 2026-07-11

## Completed public surfaces

- Primary product name: **1BitShit CPU**
- Primary executable and Cargo default binary: `bitshit`
- Repository and release asset names use `bitshit`
- Native CPU CI covers Linux x64, Windows x64 and macOS arm64
- Tagged CLI releases package `bitshit`, not the legacy `cluaiz` executable
- CI rejects regressions that reintroduce legacy public CLI artifact names

## Compatibility layer still intentionally present

The following names remain internal compatibility interfaces and must not be renamed blindly:

- Rust crates such as `cluaiz-shared`
- FFI symbols such as `cluaiz_kernel_*`
- legacy persisted state under `.cluaiz`
- dynamic library contracts consumed by existing loaders

They require a versioned migration because changing them atomically would break existing installations, driver loading and persisted state. Public command and release naming can be migrated independently.

## Installer and data migration work still open

The current root `install.sh` is a registry downloader and still needs to be reconciled with the source-building installers used by the hybrid repository. The remaining installer block is:

1. define one canonical home directory (`BITSHIT_HOME`),
2. migrate data from `.cluaiz` without overwriting newer BitShit state,
3. retain an optional `cluaiz` command alias only for compatibility,
4. support explicit `cpu`, `cuda` and `auto` backend selection,
5. validate downloaded manifests and fail when platform URLs are absent,
6. avoid parsing JSON with positional `grep` expressions,
7. provide a non-interactive mode for CI and server installation,
8. smoke-test the installed binary and expected engine/kernel locations.

## CI policy

The current CI intentionally runs Clippy without `-D warnings`. The codebase has existing warning debt, so treating every warning as a hard failure would make the new matrix permanently red before it provides useful platform feedback. Warning cleanup should be handled in bounded packages; once the baseline is clean, CI can promote warnings to errors.

## Next block

The next implementation block should replace the root registry installer with a deterministic cross-platform installer contract shared by Linux and Windows, including idempotent `.cluaiz` to `.1bitshit` migration and explicit CPU/GPU backend selection.
