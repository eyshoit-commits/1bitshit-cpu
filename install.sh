#!/usr/bin/env bash
# BitShit installer for Linux and macOS.

set -euo pipefail

VERSION="0.2.0"
REPO="eyshoit-commits/1bitshit-cpu"
REGISTRY_URL="https://raw.githubusercontent.com/${REPO}/main/package.json"
BACKEND="auto"
ASSUME_YES=0
NO_MIGRATE=0
NO_LEGACY_ALIAS=0
NO_LAUNCH=0

usage() {
  cat <<'EOF'
Usage: ./install.sh [options]

Options:
  --backend auto|cpu|cuda  Select runtime backend (default: auto)
  --yes                    Non-interactive installation
  --no-migrate             Do not import legacy ~/.cluaiz data
  --no-legacy-alias        Do not create the cluaiz compatibility command
  --no-launch              Do not calibrate or launch after installation
  --help                    Show this help

Environment:
  BITSHIT_HOME              Runtime home (default: ~/.bitshit)
  BITSHIT_REGISTRY_URL      Override package registry URL
  CLUAIZ_LEGACY_HOME        Legacy source directory (default: ~/.cluaiz)
EOF
}

while (($#)); do
  case "$1" in
    --backend) BACKEND="${2:-}"; shift 2 ;;
    --backend=*) BACKEND="${1#*=}"; shift ;;
    --yes|-y) ASSUME_YES=1; shift ;;
    --no-migrate) NO_MIGRATE=1; shift ;;
    --no-legacy-alias) NO_LEGACY_ALIAS=1; shift ;;
    --no-launch) NO_LAUNCH=1; shift ;;
    --help|-h) usage; exit 0 ;;
    *) echo "[bitshit] Unknown option: $1" >&2; usage >&2; exit 2 ;;
  esac
done

case "$BACKEND" in auto|cpu|cuda) ;; *) echo "[bitshit] Invalid backend: $BACKEND" >&2; exit 2 ;; esac

for tool in curl python3 uname; do
  command -v "$tool" >/dev/null 2>&1 || { echo "[bitshit] Missing required tool: $tool" >&2; exit 1; }
done

BITSHIT_HOME="${BITSHIT_HOME:-$HOME/.bitshit}"
BITSHIT_REGISTRY_URL="${BITSHIT_REGISTRY_URL:-$REGISTRY_URL}"
CLUAIZ_LEGACY_HOME="${CLUAIZ_LEGACY_HOME:-$HOME/.cluaiz}"
BIN_DIR="$BITSHIT_HOME/bin"
CLI_DIR="$BITSHIT_HOME/apps/cli"
ENGINE_DIR="$BITSHIT_HOME/engine"
KERNEL_DIR="$BITSHIT_HOME/interface-engines/kernels"
DRIVER_DIR="$BITSHIT_HOME/interface-engines/drivers"

step() { printf '[bitshit] %s\n' "$*"; }
fail() { printf '[bitshit] ERROR: %s\n' "$*" >&2; exit 1; }

json_get() {
  local json="$1" path="$2"
  JSON_INPUT="$json" JSON_PATH="$path" python3 - <<'PY'
import json, os, sys
value = json.loads(os.environ["JSON_INPUT"])
for part in os.environ["JSON_PATH"].split('.'):
    if not isinstance(value, dict) or part not in value:
        sys.exit(3)
    value = value[part]
if value is None:
    sys.exit(3)
if isinstance(value, (dict, list)):
    print(json.dumps(value))
else:
    print(value)
PY
}

fetch_text() {
  curl --fail --silent --show-error --location "$1"
}

download() {
  local url="$1" target="$2" label="$3"
  [[ -n "$url" ]] || fail "Missing download URL for $label"
  mkdir -p "$(dirname "$target")"
  local temp="${target}.part"
  rm -f "$temp"
  step "Downloading $label"
  curl --fail --show-error --location "$url" --output "$temp"
  [[ -s "$temp" ]] || fail "Downloaded empty artifact for $label"
  mv -f "$temp" "$target"
}

copy_missing_tree() {
  local source="$1" target="$2"
  [[ -d "$source" ]] || return 0
  mkdir -p "$target"
  while IFS= read -r -d '' item; do
    local relative="${item#"$source"/}"
    local destination="$target/$relative"
    if [[ -d "$item" ]]; then
      mkdir -p "$destination"
    elif [[ ! -e "$destination" ]]; then
      mkdir -p "$(dirname "$destination")"
      cp -p "$item" "$destination"
    fi
  done < <(find "$source" -mindepth 1 -print0)
}

if [[ "$BACKEND" == auto ]]; then
  if command -v nvidia-smi >/dev/null 2>&1 && nvidia-smi >/dev/null 2>&1; then
    BACKEND="cuda"
  else
    BACKEND="cpu"
  fi
fi
if [[ "$BACKEND" == cuda ]] && ! command -v nvidia-smi >/dev/null 2>&1; then
  fail "CUDA backend selected, but nvidia-smi was not found"
fi

OS_NAME="$(uname -s | tr '[:upper:]' '[:lower:]')"
ARCH_NAME="$(uname -m)"
case "$OS_NAME" in
  linux) OS="linux"; EXT="so" ;;
  darwin) OS="mac"; EXT="dylib" ;;
  *) fail "Unsupported operating system: $OS_NAME" ;;
esac
case "$ARCH_NAME" in
  x86_64|amd64) ARCH="x64" ;;
  aarch64|arm64) ARCH="arm64" ;;
  *) fail "Unsupported architecture: $ARCH_NAME" ;;
esac
PLATFORM="$OS-$ARCH"

step "Installing BitShit $VERSION ($PLATFORM, backend=$BACKEND)"
mkdir -p "$BIN_DIR" "$CLI_DIR" "$ENGINE_DIR" "$KERNEL_DIR" "$DRIVER_DIR"

if [[ "$NO_MIGRATE" -eq 0 && "$CLUAIZ_LEGACY_HOME" != "$BITSHIT_HOME" && -d "$CLUAIZ_LEGACY_HOME" ]]; then
  step "Importing missing legacy data from $CLUAIZ_LEGACY_HOME"
  copy_missing_tree "$CLUAIZ_LEGACY_HOME" "$BITSHIT_HOME"
fi

MASTER_JSON="$(fetch_text "$BITSHIT_REGISTRY_URL")" || fail "Unable to retrieve package registry"

CLI_MANIFEST_URL="$(json_get "$MASTER_JSON" components.cli.manifest_url)" || fail "Registry does not define components.cli.manifest_url"
ENGINE_MANIFEST_URL="$(json_get "$MASTER_JSON" components.engine.manifest_url)" || fail "Registry does not define components.engine.manifest_url"
KERNEL_MANIFEST_URL="$(json_get "$MASTER_JSON" components.kernel.manifest_url)" || fail "Registry does not define components.kernel.manifest_url"

CLI_MANIFEST="$(fetch_text "$CLI_MANIFEST_URL")" || fail "Unable to retrieve CLI manifest"
ENGINE_MANIFEST="$(fetch_text "$ENGINE_MANIFEST_URL")" || fail "Unable to retrieve engine manifest"
KERNEL_MANIFEST="$(fetch_text "$KERNEL_MANIFEST_URL")" || fail "Unable to retrieve kernel manifest"

CLI_URL="$(json_get "$CLI_MANIFEST" "cli.$PLATFORM")" || fail "CLI manifest has no asset for $PLATFORM"
ENGINE_URL="$(json_get "$ENGINE_MANIFEST" "engines.$PLATFORM")" || fail "Engine manifest has no asset for $PLATFORM"

if [[ "$OS" == mac || "$ARCH" == arm64 ]]; then
  KERNEL_PLATFORM="$PLATFORM"
elif grep -q -m1 'avx512f' /proc/cpuinfo 2>/dev/null; then
  KERNEL_PLATFORM="linux-x64-avx512"
else
  KERNEL_PLATFORM="linux-x64-avx2"
fi
KERNEL_URL="$(json_get "$KERNEL_MANIFEST" "kernels.$KERNEL_PLATFORM")" || fail "Kernel manifest has no asset for $KERNEL_PLATFORM"

download "$CLI_URL" "$CLI_DIR/bitshit" "BitShit CLI ($PLATFORM)"
chmod +x "$CLI_DIR/bitshit"
ln -sfn "$CLI_DIR/bitshit" "$BIN_DIR/bitshit"
if [[ "$NO_LEGACY_ALIAS" -eq 0 ]]; then
  ln -sfn "$CLI_DIR/bitshit" "$BIN_DIR/cluaiz"
fi

download "$ENGINE_URL" "$ENGINE_DIR/bitshit-engine.$EXT" "BitShit engine ($PLATFORM)"
download "$KERNEL_URL" "$KERNEL_DIR/bitshit-llama.$EXT" "BitShit kernel ($KERNEL_PLATFORM)"

if [[ "$BACKEND" == cuda ]]; then
  DRIVER_MANIFEST_URL="$(json_get "$MASTER_JSON" components.drivers.manifest_url)" || fail "Registry does not define components.drivers.manifest_url"
  DRIVER_MANIFEST="$(fetch_text "$DRIVER_MANIFEST_URL")" || fail "Unable to retrieve driver manifest"
  DRIVER_URL="$(json_get "$DRIVER_MANIFEST" "drivers.$PLATFORM-cuda")" || DRIVER_URL="$(json_get "$DRIVER_MANIFEST" "drivers.$PLATFORM")" || fail "Driver manifest has no CUDA asset for $PLATFORM"
  download "$DRIVER_URL" "$DRIVER_DIR/bitshit-cuda.$EXT" "BitShit CUDA driver ($PLATFORM)"
fi

export BITSHIT_HOME
export CLUAIZ_HOME="$BITSHIT_HOME"
export BITSHIT_BACKEND="$BACKEND"
export PATH="$BIN_DIR:$PATH"

SHELL_RC="$HOME/.bashrc"
[[ "${SHELL:-}" == *zsh* ]] && SHELL_RC="$HOME/.zshrc"
if [[ -f "$SHELL_RC" || -d "$(dirname "$SHELL_RC")" ]]; then
  if ! grep -q '# BitShit environment' "$SHELL_RC" 2>/dev/null; then
    cat >>"$SHELL_RC" <<EOF

# BitShit environment
export BITSHIT_HOME="$BITSHIT_HOME"
export CLUAIZ_HOME="\$BITSHIT_HOME"
export PATH="\$BITSHIT_HOME/bin:\$PATH"
EOF
  fi
fi

python3 - "$BITSHIT_HOME/install.json" <<PY
import json, sys
from pathlib import Path
path = Path(sys.argv[1])
path.write_text(json.dumps({
  "product": "bitshit",
  "installer_version": "$VERSION",
  "platform": "$PLATFORM",
  "backend": "$BACKEND",
  "home": "$BITSHIT_HOME",
  "legacy_home": "$CLUAIZ_LEGACY_HOME",
  "binary": "$BIN_DIR/bitshit"
}, indent=2) + "\n", encoding="utf-8")
PY

"$BIN_DIR/bitshit" --version >/dev/null || fail "Installed BitShit binary failed its smoke test"
step "Installed $BIN_DIR/bitshit"
step "Runtime home: $BITSHIT_HOME"

if [[ "$NO_LAUNCH" -eq 0 ]]; then
  "$BIN_DIR/bitshit" --calibrate
  if [[ "$ASSUME_YES" -eq 0 ]]; then
    "$BIN_DIR/bitshit"
  fi
fi
