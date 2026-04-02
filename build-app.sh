#!/bin/bash
set -e

SCRIPT_DIR="${0:A:h}"
cd "$SCRIPT_DIR"

(cd backend/binaries && ./download-ollama.sh)

(cd backend/binaries && ./pull-model.sh)

npm --prefix front-end install

npx --prefix front-end tauri build

ls -lh backend/target/release/bundle/dmg/*.dmg 2>/dev/null || true
ls -lh backend/target/release/bundle/macos/*.app 2>/dev/null || true
ls -lh backend/target/release/bundle/msi/*.msi 2>/dev/null || true
ls -lh backend/target/release/bundle/deb/*.deb 2>/dev/null || true
ls -lh backend/target/release/bundle/appimage/*.AppImage 2>/dev/null || true
