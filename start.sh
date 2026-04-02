#!/bin/bash
set -e

SCRIPT_DIR="${0:A:h}"
cd "$SCRIPT_DIR"

if [ -f .env ]; then
  set -a
  source .env
  set +a
fi

# Ensure the Ollama sidecar binary is downloaded.
ARCH="$(uname -m)"
case "$ARCH" in
  arm64)  TARGET="aarch64-apple-darwin" ;;
  x86_64) TARGET="x86_64-apple-darwin" ;;
  *) echo "Unsupported arch: $ARCH"; exit 1 ;;
esac

SIDECAR_BIN="backend/ollama-${TARGET}"
if [ ! -f "$SIDECAR_BIN" ] || [ "$(wc -c < "$SIDECAR_BIN" | tr -d ' ')" -lt 1000000 ]; then
  echo "Downloading Ollama sidecar binary..."
  (cd backend/binaries && ./download-ollama.sh)
fi

# In dev mode, Tauri resolves sidecars relative to the compiled binary
# (target/debug/) using just the program name — no target triple suffix.
ln -sf "$(pwd)/${SIDECAR_BIN}" "target/debug/ollama"

echo "Installing frontend dependencies..."
npm --prefix front-end install

echo "Starting Vite dev server..."
npm --prefix front-end run dev &
VITE_PID=$!

echo "Waiting for Vite..."
attempts=0
until curl -sf "http://127.0.0.1:5173" >/dev/null 2>&1; do
  attempts=$((attempts + 1))
  if [ $attempts -ge 30 ]; then
    echo "Vite did not start in time."
    kill $VITE_PID 2>/dev/null
    exit 1
  fi
  printf "."
  sleep 1
done
echo "Vite is ready."

trap "kill $VITE_PID 2>/dev/null" EXIT

echo "Starting Tauri app (Ollama starts automatically as a sidecar)..."
export GITHUB_TOKEN="${GITHUB_TOKEN:-}"

cargo run --manifest-path backend/Cargo.toml
