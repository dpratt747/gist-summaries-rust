#!/bin/bash
# Pre-downloads the default model into a local directory so it can be
# bundled inside the Tauri app. Uses the sidecar Ollama binary.
#
# Usage: ./pull-model.sh [model]
#   model defaults to gemma3:1b
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
MODEL="${1:-gemma3:1b}"
MODELS_DIR="${SCRIPT_DIR}/../models"

# Find the ollama sidecar binary for this platform.
ARCH="$(uname -m)"
OS="$(uname -s)"
case "$OS" in
  Darwin)
    case "$ARCH" in
      arm64)  TARGET="aarch64-apple-darwin" ;;
      x86_64) TARGET="x86_64-apple-darwin" ;;
    esac
    ;;
  Linux)
    case "$ARCH" in
      x86_64)  TARGET="x86_64-unknown-linux-gnu" ;;
      aarch64) TARGET="aarch64-unknown-linux-gnu" ;;
    esac
    ;;
esac

OLLAMA_BIN="${SCRIPT_DIR}/ollama-${TARGET}"
if [ ! -x "$OLLAMA_BIN" ]; then
  echo "Error: Ollama binary not found at $OLLAMA_BIN"
  echo "Run ./download-ollama.sh first."
  exit 1
fi

mkdir -p "$MODELS_DIR"

# Check if model is already pulled into local dir.
export OLLAMA_MODELS="$MODELS_DIR"
if "$OLLAMA_BIN" list 2>/dev/null | grep -q "$MODEL"; then
  echo "Model '$MODEL' already available in $MODELS_DIR"
  exit 0
fi

echo "Starting temporary Ollama server..."
export OLLAMA_HOST="127.0.0.1:11435"
"$OLLAMA_BIN" serve &
OLLAMA_PID=$!
trap "kill $OLLAMA_PID 2>/dev/null; wait $OLLAMA_PID 2>/dev/null" EXIT

# Wait for server to be ready.
for i in $(seq 1 30); do
  if curl -sf "http://127.0.0.1:11435/api/tags" >/dev/null 2>&1; then
    break
  fi
  sleep 0.5
done

echo "Pulling model '$MODEL' into $MODELS_DIR..."
"$OLLAMA_BIN" pull "$MODEL"

echo "Done. Model stored in $MODELS_DIR"
echo "Size: $(du -sh "$MODELS_DIR" | cut -f1)"
