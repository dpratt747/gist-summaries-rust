#!/bin/bash
# Downloads the Ollama binary and places it in the correct location for Tauri sidecar bundling.
# Tauri expects binaries named: <name>-<target-triple>[.exe]
#
# Usage: ./download-ollama.sh [version]
#   version defaults to v0.19.0
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
# Output to backend/ (parent dir) — Tauri expects ollama-<triple> next to Cargo.toml
OUTPUT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$OUTPUT_DIR"

VERSION="${1:-v0.19.0}"
BASE_URL="https://github.com/ollama/ollama/releases/download/${VERSION}"

OS="$(uname -s)"
ARCH="$(uname -m)"

case "$OS" in
  Darwin)
    # macOS: distributed as a .tgz containing bin/ollama
    DOWNLOAD_URL="${BASE_URL}/ollama-darwin.tgz"
    ARCHIVE_TYPE="tgz"
    INNER_BIN="bin/ollama"
    case "$ARCH" in
      arm64)  TARGET="aarch64-apple-darwin" ;;
      x86_64) TARGET="x86_64-apple-darwin" ;;
      *) echo "Unsupported macOS arch: $ARCH"; exit 1 ;;
    esac
    EXT=""
    ;;
  Linux)
    ARCHIVE_TYPE="zst"
    INNER_BIN="bin/ollama"
    case "$ARCH" in
      x86_64)
        DOWNLOAD_URL="${BASE_URL}/ollama-linux-amd64.tar.zst"
        TARGET="x86_64-unknown-linux-gnu"
        ;;
      aarch64)
        DOWNLOAD_URL="${BASE_URL}/ollama-linux-arm64.tar.zst"
        TARGET="aarch64-unknown-linux-gnu"
        ;;
      *) echo "Unsupported Linux arch: $ARCH"; exit 1 ;;
    esac
    EXT=""
    ;;
  MINGW*|MSYS*|CYGWIN*)
    ARCHIVE_TYPE="zip"
    INNER_BIN="ollama.exe"
    DOWNLOAD_URL="${BASE_URL}/ollama-windows-amd64.zip"
    TARGET="x86_64-pc-windows-msvc"
    EXT=".exe"
    ;;
  *)
    echo "Unsupported OS: $OS"; exit 1 ;;
esac

BINARY_NAME="ollama-${TARGET}${EXT}"

# Skip download only if a real binary exists (>1 MB). A stub file from the
# build system will be much smaller and should be replaced.
if [ -f "$BINARY_NAME" ] && [ "$(wc -c < "$BINARY_NAME" | tr -d ' ')" -gt 1000000 ]; then
  echo "Already exists: $BINARY_NAME ($(du -h "$BINARY_NAME" | cut -f1))"
  exit 0
fi

TMPDIR_DL="$(mktemp -d)"
trap "rm -rf '$TMPDIR_DL'" EXIT

echo "Downloading Ollama ${VERSION} for ${TARGET}..."
ARCHIVE_PATH="${TMPDIR_DL}/ollama-archive"
curl -fL "$DOWNLOAD_URL" -o "$ARCHIVE_PATH"

echo "Extracting..."
case "$ARCHIVE_TYPE" in
  tgz)
    tar xzf "$ARCHIVE_PATH" -C "$TMPDIR_DL"
    ;;
  zst)
    # tar.zst — requires zstd
    if ! command -v zstd &>/dev/null; then
      echo "Error: zstd is required to extract Linux archives. Install with: apt install zstd / brew install zstd"
      exit 1
    fi
    zstd -d "$ARCHIVE_PATH" -o "${ARCHIVE_PATH}.tar"
    tar xf "${ARCHIVE_PATH}.tar" -C "$TMPDIR_DL"
    ;;
  zip)
    unzip -q "$ARCHIVE_PATH" -d "$TMPDIR_DL"
    ;;
esac

# Find the ollama binary in the extracted archive (path varies by version).
FOUND_BIN="$(find "$TMPDIR_DL" -name "ollama${EXT}" -type f ! -name "*.tgz" ! -name "*.zip" | head -1)"
if [ -z "$FOUND_BIN" ]; then
  echo "Error: could not find ollama binary in extracted archive. Contents:"
  find "$TMPDIR_DL" -type f
  exit 1
fi

cp "$FOUND_BIN" "$BINARY_NAME"
chmod +x "$BINARY_NAME"
echo "Saved: $BINARY_NAME ($(du -h "$BINARY_NAME" | cut -f1))"
echo "Done. Sidecar binary ready for Tauri."