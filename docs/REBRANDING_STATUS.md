# BitShit rebranding status

Last reviewed: 2026-07-11

## Completed public surfaces

- Primary product name: **BitShit / 1BitShit CPU**
- Primary executable and Cargo default binary: `bitshit`
- Repository and release asset names use `bitshit`
- Native CPU CI covers Linux x64, Windows x64 and macOS arm64
- Tagged CLI releases package `bitshit`, not the legacy `cluaiz` executable
- CI rejects regressions that reintroduce legacy public CLI artifact names

## Installer contract

The root installers now share one public contract on Linux, macOS and Windows:

- canonical runtime home: `BITSHIT_HOME`, defaulting to `~/.bitshit`
- internal compatibility variable: `CLUAIZ_HOME=$BITSHIT_HOME`
- backend selection: `auto`, `cpu` or `cuda`
- `auto` selects CUDA only when an NVIDIA runtime is detected; otherwise it selects CPU
- explicit CUDA selection fails when `nvidia-smi` is unavailable
- non-interactive mode: `--yes` on POSIX and `-Yes` on PowerShell
- optional migration bypass: `--no-migrate` / `-NoMigrate`
- optional compatibility alias bypass: `--no-legacy-alias` / `-NoLegacyAlias`
- optional first-run bypass: `--no-launch` / `-NoLaunch`
- downloaded CLI, engine, kernel and CUDA-driver URLs must be present in their manifests
- downloads use temporary `.part` files and reject empty artifacts
- installation ends with a `bitshit --version` smoke test
- installation metadata is persisted in `BITSHIT_HOME/install.json`

The POSIX installer parses registry JSON with Python's JSON parser instead of positional `grep` expressions. PowerShell uses `Invoke-RestMethod` and explicit property checks.

## Data migration

Legacy data is imported from `CLUAIZ_LEGACY_HOME`, defaulting to `~/.cluaiz`.

Migration rules:

1. migration is idempotent;
2. only missing files are copied;
3. existing BitShit files are never overwritten;
4. the legacy directory remains intact;
5. `--no-migrate` / `-NoMigrate` disables import completely.

The former root installer used `~/.1bitshit`. Importing that transitional directory into `~/.bitshit` is still an open compatibility item and must follow the same copy-missing, non-destructive rules.

## Compatibility layer intentionally retained

The following names remain internal compatibility interfaces and must not be renamed blindly:

- Rust crates such as `cluaiz-shared`
- FFI symbols such as `cluaiz_kernel_*`
- dynamic library contracts consumed by existing loaders
- the optional `cluaiz` command alias

They require a versioned migration because changing them atomically would break existing installations, driver loading and persisted state. Public command and release naming is migrated independently.

## CI policy

The installer contract job now validates:

- Bash syntax
- PowerShell parser errors
- backend flags
- canonical and legacy path variables
- non-interactive and no-launch switches
- `install.json` persistence
- removal of positional manifest parsing
- absence of legacy public release and binary names

Clippy intentionally runs without `-D warnings` while existing warning debt remains. Warning cleanup should be handled in bounded packages; once the baseline is clean, CI can promote warnings to errors.

## Current external blocker

CUDA installation depends on the driver release manifest exposing either:

- `drivers.<platform>-cuda`, or
- `drivers.<platform>`

The installer now fails clearly when neither key exists. The release pipeline still needs to guarantee those keys and publish matching driver artifacts before CUDA installation can be considered release-complete.

## Next block

1. migrate the transitional `~/.1bitshit` directory into `~/.bitshit` without overwriting newer state;
2. verify and, if necessary, repair the CLI/engine/kernel/driver release manifests;
3. add mocked installer integration tests that exercise CPU success, CUDA rejection and missing-manifest failures without downloading production binaries;
4. inspect remaining user-visible `cluaiz` strings separately from internal ABI names.
