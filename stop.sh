#!/bin/bash
set -e

SCRIPT_DIR="${0:A:h}"
cd "$SCRIPT_DIR"

echo "Stopping Vite dev server..."
pkill -f "vite" 2>/dev/null && echo "Vite stopped." || echo "Vite was not running."

echo "Stopping Tauri / Cargo process..."
pkill -f "gist-summary" 2>/dev/null && echo "Tauri app stopped." || echo "Tauri app was not running."

echo "Done."

