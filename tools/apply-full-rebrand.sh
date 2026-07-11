#!/usr/bin/env bash
set -Eeuo pipefail

REPOSITORY_URL="https://github.com/eyshoit-commits/1bitshit-cpu.git"
REBRAND_BRANCH="rebrand/full-1bitshit-cpu"
REPO_DIR="${1:-$HOME/team/server/1bitshit-cpu}"
RUNTIME_DIR="${BITSHIT_HOME:-$HOME/.1bitshit}"
LOCAL_BIN_DIR="$HOME/.local/bin"
LOG="$REPO_DIR/rebrand-apply.log"

say() { printf '\n%s\n' "$*" | tee -a "$LOG"; }
die() { printf '\nFEHLER: %s\n' "$*" | tee -a "$LOG" >&2; exit 1; }
trap 'printf "\nFEHLER in Zeile %s. Bericht: %s\n" "$LINENO" "$LOG" >&2' ERR

command -v git >/dev/null 2>&1 || die "git ist nicht installiert."
command -v cargo >/dev/null 2>&1 || die "cargo ist nicht installiert."
command -v rustc >/dev/null 2>&1 || die "rustc ist nicht installiert."

[ -d "$REPO_DIR/.git" ] || die "Kein Git-Repository gefunden unter $REPO_DIR"
cd "$REPO_DIR"
: > "$LOG"

say "1BitShit CPU Vollmigration startet."
say "Projektordner: $REPO_DIR"
say "Runtime-Ordner: $RUNTIME_DIR"

if [ -n "$(git status --porcelain)" ]; then
    BACKUP_NAME="pre-full-rebrand-$(date +%Y%m%d-%H%M%S)"
    say "Lokale Änderungen werden sicher zwischengespeichert: $BACKUP_NAME"
    git stash push --include-untracked --message "$BACKUP_NAME" >/dev/null
fi

if git remote get-url full-rebrand >/dev/null 2>&1; then
    git remote set-url full-rebrand "$REPOSITORY_URL"
else
    git remote add full-rebrand "$REPOSITORY_URL"
fi

say "Aktuellen vollständigen Rebrand-Branch laden."
git fetch full-rebrand "$REBRAND_BRANCH" --prune
git switch -C full-1bitshit-cpu "full-rebrand/$REBRAND_BRANCH"

say "Vollständigen Workspace prüfen und bauen."
bash tools/verify-rebrand.sh "$REPO_DIR"

say "Neue Runtime-Struktur anlegen."
mkdir -p \
    "$RUNTIME_DIR/bin" \
    "$RUNTIME_DIR/engine/drivers" \
    "$RUNTIME_DIR/models" \
    "$RUNTIME_DIR/config" \
    "$LOCAL_BIN_DIR"

say "Release-Binary installieren."
install -m 0755 target/release/bitshit "$RUNTIME_DIR/bin/bitshit"
install -m 0755 target/release/bitshit "$LOCAL_BIN_DIR/bitshit"

copy_first_match() {
    local destination="$1"
    shift
    local candidate
    for candidate in "$@"; do
        if [ -f "$candidate" ]; then
            install -m 0755 "$candidate" "$destination"
            return 0
        fi
    done
    return 1
}

case "$(uname -s)" in
    Darwin) LIB_EXT="dylib" ;;
    Linux) LIB_EXT="so" ;;
    *) LIB_EXT="dll" ;;
esac

say "Llama- und ONNX-Runtime installieren."
copy_first_match "$RUNTIME_DIR/engine/bitshit-llama.$LIB_EXT" \
    "target/release/bitshit-llama.$LIB_EXT" \
    "target/release/libbitshit_llama.$LIB_EXT" \
    "target/release/deps/libbitshit_llama.$LIB_EXT" \
    || die "Die Llama-Release-Bibliothek wurde nicht gefunden."

copy_first_match "$RUNTIME_DIR/engine/bitshit-onnx.$LIB_EXT" \
    "target/release/bitshit-onnx.$LIB_EXT" \
    "target/release/libbitshit_onnx.$LIB_EXT" \
    "target/release/deps/libbitshit_onnx.$LIB_EXT" \
    || die "Die ONNX-Release-Bibliothek wurde nicht gefunden."

copy_first_match "$RUNTIME_DIR/engine/bitshit-engine.$LIB_EXT" \
    "target/release/bitshit-engine.$LIB_EXT" \
    "target/release/libengines.$LIB_EXT" \
    "target/release/deps/libengines.$LIB_EXT" \
    || say "Hinweis: Die Engine-CDylib wurde nicht separat gefunden; die CLI enthält weiterhin den Rust-Engine-Core."

printf '%s\n' "$(date -Iseconds)" > "$RUNTIME_DIR/engine/bitshit-llama.ready"
printf '%s\n' "$(date -Iseconds)" > "$RUNTIME_DIR/engine/bitshit-onnx.ready"
printf '%s\n' "$(date -Iseconds)" > "$RUNTIME_DIR/engine/bitshit-engine.ready"

say "ONNX-Runtime-Abhängigkeiten synchronisieren."
find target/release target/release/deps -maxdepth 1 -type f \
    \( -name 'libonnxruntime*.so*' -o -name 'onnxruntime*.dll' -o -name 'libonnxruntime*.dylib' \) \
    -exec cp -f {} "$RUNTIME_DIR/engine/drivers/" \; 2>/dev/null || true

export BITSHIT_HOME="$RUNTIME_DIR"
export BITSHIT_ROOT="$RUNTIME_DIR"
export BITSHIT_MODELS_DIR="$REPO_DIR/models"
export PATH="$LOCAL_BIN_DIR:$PATH"

say "Migration abgeschlossen."
say "Binary: $LOCAL_BIN_DIR/bitshit"
say "Llama: $RUNTIME_DIR/engine/bitshit-llama.$LIB_EXT"
say "ONNX: $RUNTIME_DIR/engine/bitshit-onnx.$LIB_EXT"
say "Modelle: $BITSHIT_MODELS_DIR"
say "1BitShit CPU wird jetzt gestartet."

exec "$LOCAL_BIN_DIR/bitshit"
