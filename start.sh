#!/bin/zsh
set -e

SCRIPT_DIR="${0:A:h}"
cd "$SCRIPT_DIR"

if [ -f .env ]; then
  set -a
  source .env
  set +a
fi

OPENAI_MODEL="${OPENAI_MODEL:-ai/gemma3:1B-Q4_K_M}"
MODEL_RUNNER_URL="http://localhost:12434/engines/v1"

if ! docker info >/dev/null 2>&1; then
  echo "Starting Docker Desktop..."
  open -a "Docker Desktop"
  attempts=0
  until docker info >/dev/null 2>&1; do
    attempts=$((attempts + 1))
    if [ $attempts -ge 30 ]; then
      echo "Docker Desktop did not start in time."
      exit 1
    fi
    printf "."
    sleep 2
  done
  echo "Docker Desktop is ready."
fi

echo "Installing frontend dependencies..."
npm --prefix front-end install

echo "Pulling model ${OPENAI_MODEL}..."
docker model pull "$OPENAI_MODEL"

echo "Waiting for model runner..."
attempts=0
until curl -sf "${MODEL_RUNNER_URL}/models" >/dev/null 2>&1; do
  attempts=$((attempts + 1))
  if [ $attempts -ge 30 ]; then
    echo "Model runner not ready. Is Docker Desktop running?"
    exit 1
  fi
  printf "."
  sleep 2
done
echo "Model runner is ready."

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

echo "Starting Tauri app..."
export OPENAI_API_BASE_URL="$MODEL_RUNNER_URL"
export OPENAI_API_KEY="${OPENAI_API_KEY:-local}"
export OPENAI_MODEL
export GITHUB_TOKEN="${GITHUB_TOKEN:-}"

cargo run --manifest-path src-tauri/Cargo.toml

