# Gist Summary

A desktop app that fetches your GitHub gists and generates one-sentence summaries using a local LLM (gemma3:1b) powered by Ollama. Built with Tauri v2, Svelte 5, and Rust.

## Requirements

- [Rust](https://rustup.rs/) (stable)
- [Node.js](https://nodejs.org/) (v22+)
- A GitHub personal access token with `gist:read` scope ([create one](https://github.com/settings/tokens))

No Docker required — Ollama is bundled as a sidecar and starts automatically.

## Running (development)

```bash
./start.sh
```

This will:
1. Download the Ollama binary (first run only, ~73MB)
2. Install frontend dependencies
3. Start the Vite dev server
4. Launch the Tauri app with Ollama running as a sidecar
5. Pull the `gemma3:1b` model if not already present (first run only, ~815MB)

## Building a distributable app

```bash
./build-app.sh
```

This will:
1. Download the Ollama binary
2. Pre-pull the model into `backend/models/` (bundled into the app so users skip the download)
3. Build the frontend
4. Package the Tauri app (`.dmg` on macOS, `.msi` on Windows, `.deb`/`.AppImage` on Linux)

To build **without** bundling the model (smaller app, users pull on first launch):

```bash
npx --prefix front-end tauri build
```

## Releasing

Push a version tag to trigger the GitHub Actions release workflow:

```bash
git tag v0.1.0
git push origin v0.1.0
```

This builds for macOS (ARM + Intel), Linux (x86_64), and Windows (x86_64), then creates a GitHub Release with all platform artifacts.

## Configuration

The app stores the GitHub token in a `.env` file in the working directory:

```
GITHUB_TOKEN=ghp_...
```

You can also set it via the app's token screen on first launch.

## Project structure

```
front-end/          Svelte 5 + Vite frontend
backend/            Tauri v2 + Rust backend
  src/
    main.rs         Tauri commands (get_gists, summarise_file, load/save_token)
    github/         GitHub API client
    llm/            LLM client (OpenAI-compatible via langchain-rust)
    ollama/         Ollama sidecar lifecycle (spawn, health check, model pull)
  binaries/         Scripts to download Ollama and pre-pull models
  tauri.conf.json   Tauri configuration
.github/workflows/  CI/CD release workflow
start.sh            Dev launcher
build-app.sh        Production build script
```
