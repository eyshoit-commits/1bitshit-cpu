#!/usr/bin/env bash
set -Eeuo pipefail

ROOT="${1:-$(pwd)}"
LOG="$ROOT/rebrand-build.log"
cd "$ROOT"
: > "$LOG"

say() { printf '\n%s\n' "$*" | tee -a "$LOG"; }
run() { say "===== $1 ====="; shift; "$@" 2>&1 | tee -a "$LOG"; }
trap 'printf "\nFEHLER in Zeile %s. Vollständiger Bericht: %s\n" "$LINENO" "$LOG" >&2' ERR

run "WORKSPACE METADATA" cargo metadata --no-deps --format-version 1
run "BITSHIT SHARED" cargo check --manifest-path Inference-engine/engines/cluaiz-shared/Cargo.toml
run "BITSHIT ONNX" cargo check --manifest-path interface-engines/onnx/Cargo.toml
run "BITSHIT LLAMA" cargo check --manifest-path interface-engines/llama/Cargo.toml
run "ENGINE CORE" cargo check --manifest-path Inference-engine/engines/Cargo.toml --all-features
run "BITSHIT API" cargo check --manifest-path Inference-engine/api/Cargo.toml
run "BITSHIT CLI" cargo check --manifest-path cmd/Cargo.toml --bin bitshit
run "RELEASE BUILD" cargo build --release --manifest-path cmd/Cargo.toml --bin bitshit
run "FULL WORKSPACE" cargo check --workspace --all-targets --all-features

say "===== RUNTIME IDENTITY AUDIT ====="
if git grep -n -I -E 'Ghost Execution Detected|\[cluaiz\]|cluaiz Main Menu|Starting cluaiz API|\.cluaiz/models' -- ':!docs/**' ':!cmd/src/ui/menu.rs' ':!cmd/src/cli/run.rs' ':!**/THIRD_PARTY_NOTICES.txt' | tee -a "$LOG"; then
    say "WARNUNG: Aktive Legacy-Treffer wurden gefunden."
else
    say "Keine aktiven Legacy-Anzeigen gefunden."
fi

say "BUILD ERFOLGREICH. Binary: $ROOT/target/release/bitshit"
