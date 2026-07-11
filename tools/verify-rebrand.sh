#!/usr/bin/env bash
set -Eeuo pipefail

ROOT="${1:-$(pwd)}"
LOG="$ROOT/rebrand-build.log"
ERRORS="$ROOT/rebrand-errors.txt"
cd "$ROOT"
: > "$LOG"
: > "$ERRORS"

say() {
    printf '\n%s\n' "$*" | tee -a "$LOG"
}

print_error_summary() {
    printf '\n===== ECHTE COMPILERFEHLER =====\n' | tee "$ERRORS" >&2
    awk '
        /^error(\[E[0-9]+\])?:/ { remaining=22 }
        remaining > 0 { print; remaining-- }
        /could not compile/ { print }
    ' "$LOG" | tail -n 240 | tee -a "$ERRORS" >&2
    printf '\nVollständiger Bericht: %s\nFehlerauszug: %s\n' "$LOG" "$ERRORS" >&2
}

run() {
    local label="$1"
    shift
    say "===== $label ====="

    set +e
    "$@" 2>&1 | tee -a "$LOG"
    local status=${PIPESTATUS[0]}
    set -e

    if [ "$status" -ne 0 ]; then
        print_error_summary
        return "$status"
    fi
}

trap 'printf "\nFEHLER in Zeile %s. Der Build wurde abgebrochen.\n" "$LINENO" >&2' ERR

run "WORKSPACE METADATA" cargo metadata --no-deps --format-version 1
run "BITSHIT SHARED CHECK" cargo check --manifest-path Inference-engine/engines/cluaiz-shared/Cargo.toml
run "BITSHIT ONNX CPU CHECK" cargo check --manifest-path interface-engines/onnx/Cargo.toml
run "BITSHIT LLAMA CHECK" cargo check --manifest-path interface-engines/llama/Cargo.toml
run "ENGINE CORE CHECK" cargo check --manifest-path Inference-engine/engines/Cargo.toml
run "BITSHIT API CHECK" cargo check --manifest-path Inference-engine/api/Cargo.toml
run "BITSHIT CLI CHECK" cargo check --manifest-path cmd/Cargo.toml --bin bitshit
run "FULL CPU WORKSPACE CHECK" cargo check --workspace --all-targets

run "BITSHIT LLAMA RELEASE" cargo build --release --manifest-path interface-engines/llama/Cargo.toml
run "BITSHIT ONNX CPU RELEASE" cargo build --release --manifest-path interface-engines/onnx/Cargo.toml
run "ENGINE CORE RELEASE" cargo build --release --manifest-path Inference-engine/engines/Cargo.toml
run "BITSHIT API RELEASE" cargo build --release --manifest-path Inference-engine/api/Cargo.toml
run "BITSHIT CLI RELEASE" cargo build --release --manifest-path cmd/Cargo.toml --bin bitshit

say "===== RELEASE ARTIFACT AUDIT ====="
test -x "$ROOT/target/release/bitshit"
find "$ROOT/target/release" -maxdepth 2 -type f \
    \( -name '*bitshit*llama*' -o -name '*bitshit*onnx*' -o -name 'bitshit' -o -name 'bitshit.exe' \) \
    -print | sort | tee -a "$LOG"

say "===== PRESERVED FEATURE AUDIT ====="
test -d interface-engines/llama
test -d interface-engines/onnx
test -f Inference-engine/engines/src/models/entities.rs
test -f Inference-engine/engines/src/models/fetch_v2.rs
test -f Inference-engine/engines/src/models/manager/hf_hub_v3.rs
test -f cmd/src/cli/run_v3.rs
grep -q 'run_v3.rs' cmd/src/cli/mod.rs
grep -q 'bitshit_kernel_generate_stream' interface-engines/llama/src/ffi_exports.rs
grep -q 'bitshit_kernel_generate_embedding' interface-engines/onnx/src/lib.rs
grep -q 'BITSHIT_MODELS_DIR' Inference-engine/engines/src/models/fetch_v2.rs
say "Llama, CPU-ONNX, chat entities, model hub, canonical runner and visible model store are present."

say "===== VISIBLE LEGACY IDENTITY AUDIT ====="
if git grep -n -I -E '\[cluaiz\]|cluaiz Main Menu|Starting cluaiz API|cluaiz v0\.|Ghost Execution Detected' -- \
    ':!docs/**' \
    ':!cmd/src/main.rs' \
    ':!cmd/src/ui/menu.rs' \
    ':!cmd/src/cli/run.rs' \
    ':!cmd/src/cli/run_v2.rs' \
    ':!**/THIRD_PARTY_NOTICES.txt' | tee -a "$LOG"; then
    say "WARNUNG: Sichtbare Legacy-Treffer wurden gefunden."
else
    say "Keine aktiven sichtbaren Legacy-Anzeigen gefunden."
fi

say "BUILD ERFOLGREICH. Binary: $ROOT/target/release/bitshit"
