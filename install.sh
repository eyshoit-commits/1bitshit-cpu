#!/usr/bin/env bash
set -Eeuo pipefail

VERSION="${BITSHIT_VERSION:-0.2.0}"
HUB_PATH="${BITSHIT_HOME:-${BITSHIT_ROOT:-$HOME/.1bitshit}}"
LEGACY_PATH="$HOME/.cluaiz"
REPOSITORY="eyshoit-commits/1bitshit-cpu"
REGISTRY_URL="https://raw.githubusercontent.com/$REPOSITORY/main/package.json"

say() { printf '\n%s\n' "$*"; }
step() { printf '  [....] %s\n' "$*"; }
done_step() { printf '  [DONE] %s\n' "$*"; }
fail() { printf '  [ERROR] %s\n' "$*" >&2; exit 1; }
trap 'fail "Installation stopped at line $LINENO"' ERR

require() {
    command -v "$1" >/dev/null 2>&1 || fail "Required command is missing: $1"
}

json_value() {
    python3 -c '
import json
import sys

path = sys.argv[1].split(".")
data = json.load(sys.stdin)
for part in path:
    data = data[part]
print(data)
' "$1"
}

download_atomic() {
    local url="$1"
    local destination="$2"
    local label="$3"
    local partial="${destination}.part"
    [ -n "$url" ] || fail "Download URL is missing for $label"
    mkdir -p "$(dirname "$destination")"
    rm -f "$partial"
    step "Downloading $label"
    curl --fail --location --silent --show-error "$url" --output "$partial"
    chmod 0755 "$partial" 2>/dev/null || true
    mv -f "$partial" "$destination"
    done_step "$label installed"
}

copy_legacy_tree() {
    local name="$1"
    local source="$LEGACY_PATH/$name"
    local destination="$HUB_PATH/$name"
    [ -d "$source" ] || return 0
    mkdir -p "$destination"
    cp -a "$source/." "$destination/"
}

require curl
require python3

say '============================================================'
say '                    1BitShit CPU Runtime'
say '          Llama, GGUF, BitNet, ONNX and Native FFI'
say '============================================================'

step "Creating runtime directories at $HUB_PATH"
mkdir -p \
    "$HUB_PATH/bin" \
    "$HUB_PATH/apps/cli" \
    "$HUB_PATH/engine/drivers" \
    "$HUB_PATH/models" \
    "$HUB_PATH/config" \
    "$HUB_PATH/brain" \
    "$HUB_PATH/skills" \
    "$HUB_PATH/extensions" \
    "$HUB_PATH/plugins" \
    "$HUB_PATH/mcp" \
    "$HUB_PATH/kv_cache" \
    "$HUB_PATH/reports"
done_step 'Runtime directories ready'

MIGRATION_MARKER="$HUB_PATH/.legacy-cluaiz-import-complete"
if [ -d "$LEGACY_PATH" ] && [ ! -f "$MIGRATION_MARKER" ]; then
    step 'Importing existing legacy data without deleting it'
    for folder in models brain skills extensions plugins mcp kv_cache reports; do
        copy_legacy_tree "$folder"
    done
    printf '%s\n' \
        'Imported by 1BitShit CPU. The previous runtime was not executed or deleted.' \
        > "$MIGRATION_MARKER"
    done_step 'Legacy data imported'
fi

step 'Loading 1BitShit CPU registry'
MASTER_JSON="$(curl --fail --location --silent --show-error "$REGISTRY_URL")"
done_step 'Registry loaded'

case "$(uname -s)" in
    Linux) OS='linux'; EXT='so' ;;
    Darwin) OS='mac'; EXT='dylib' ;;
    *) fail "Unsupported operating system: $(uname -s)" ;;
esac

case "$(uname -m)" in
    x86_64|amd64) ARCH='x64' ;;
    aarch64|arm64) ARCH='arm64' ;;
    *) fail "Unsupported architecture: $(uname -m)" ;;
esac

PLATFORM="$OS-$ARCH"
CLI_MANIFEST_URL="$(printf '%s' "$MASTER_JSON" | json_value components.cli.manifest_url)"
ENGINE_MANIFEST_URL="$(printf '%s' "$MASTER_JSON" | json_value components.engine.manifest_url)"
KERNEL_MANIFEST_URL="$(printf '%s' "$MASTER_JSON" | json_value components.kernel.manifest_url)"

CLI_MANIFEST="$(curl --fail --location --silent --show-error "$CLI_MANIFEST_URL")"
CLI_URL="$(printf '%s' "$CLI_MANIFEST" | json_value "cli.$PLATFORM")"
download_atomic "$CLI_URL" "$HUB_PATH/apps/cli/bitshit" "1BitShit CPU CLI ($PLATFORM)"
ln -sfn "$HUB_PATH/apps/cli/bitshit" "$HUB_PATH/bin/bitshit"

ENGINE_MANIFEST="$(curl --fail --location --silent --show-error "$ENGINE_MANIFEST_URL")"
ENGINE_URL="$(printf '%s' "$ENGINE_MANIFEST" | json_value "engines.$PLATFORM")"
download_atomic "$ENGINE_URL" "$HUB_PATH/engine/bitshit-engine.$EXT" "1BitShit engine ($PLATFORM)"

if [ "$OS" = 'mac' ]; then
    KERNEL_PLATFORM="$PLATFORM"
elif [ "$ARCH" = 'arm64' ]; then
    KERNEL_PLATFORM='linux-arm64'
elif grep -qm1 'avx512f' /proc/cpuinfo 2>/dev/null; then
    KERNEL_PLATFORM='linux-x64-avx512'
else
    KERNEL_PLATFORM='linux-x64-avx2'
fi

KERNEL_MANIFEST="$(curl --fail --location --silent --show-error "$KERNEL_MANIFEST_URL")"
KERNEL_URL="$(printf '%s' "$KERNEL_MANIFEST" | json_value "kernels.$KERNEL_PLATFORM")"
download_atomic "$KERNEL_URL" "$HUB_PATH/engine/bitshit-llama.$EXT" "1BitShit Llama kernel ($KERNEL_PLATFORM)"
printf '%s\n' "$(printf '%s' "$KERNEL_MANIFEST" | json_value version)" \
    > "$HUB_PATH/engine/bitshit-llama.ready"

SHELL_RC="$HOME/.bashrc"
case "${SHELL:-}" in
    *zsh*) SHELL_RC="$HOME/.zshrc" ;;
    *fish*) SHELL_RC="$HOME/.config/fish/config.fish" ;;
esac

if [[ "${SHELL:-}" == *fish* ]]; then
    mkdir -p "$(dirname "$SHELL_RC")"
    grep -q 'BITSHIT_HOME' "$SHELL_RC" 2>/dev/null || {
        printf '\nset -gx BITSHIT_HOME "%s"\nfish_add_path "%s/bin"\n' "$HUB_PATH" "$HUB_PATH" >> "$SHELL_RC"
    }
else
    grep -q 'BITSHIT_HOME' "$SHELL_RC" 2>/dev/null || {
        printf '\n# 1BitShit CPU\nexport BITSHIT_HOME="%s"\nexport BITSHIT_ROOT="%s"\nexport PATH="$PATH:%s/bin"\n' \
            "$HUB_PATH" "$HUB_PATH" "$HUB_PATH" >> "$SHELL_RC"
    }
fi

export BITSHIT_HOME="$HUB_PATH"
export BITSHIT_ROOT="$HUB_PATH"
export PATH="$PATH:$HUB_PATH/bin"

printf '\nEnable the native FFI memory brain? Type y or n: '
read -r brain_choice
if [[ "$brain_choice" =~ ^[Yy]$ ]]; then
    export BITSHIT_FFI_BRAIN=1
    export cluaizdb_FFI=1
    done_step 'Native FFI memory brain enabled'
else
    export BITSHIT_FFI_BRAIN=0
    export cluaizdb_FFI=0
    done_step 'Native FFI memory brain disabled'
fi

step 'Calibrating hardware'
"$HUB_PATH/bin/bitshit" --calibrate
done_step 'Hardware calibration complete'

say "1BitShit CPU $VERSION installed successfully."
say "Runtime: $HUB_PATH"
say 'Command: bitshit'
exec "$HUB_PATH/bin/bitshit"
