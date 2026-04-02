use anyhow::{bail, Context, Result};
use tauri::{AppHandle, Manager};
use tauri_plugin_shell::ShellExt;
use tauri_plugin_shell::process::{CommandChild, CommandEvent};

const OLLAMA_HOST: &str = "127.0.0.1";
const OLLAMA_PORT: u16 = 11434;
const DEFAULT_MODEL: &str = "gemma3:1b";

pub struct OllamaProcess {
    _child: CommandChild,
}

impl OllamaProcess {
    /// Spawns the bundled Ollama sidecar (`ollama serve`), waits for it to be
    /// healthy, and pulls the default model if it isn't already present.
    pub async fn start(app: &AppHandle) -> Result<Self> {
        let models_dir = resolve_models_dir(app);
        eprintln!("[ollama] models directory: {}", models_dir.display());

        // Spawn `ollama serve` as a sidecar process.
        let mut cmd = app
            .shell()
            .sidecar("binaries/ollama")
            .context("failed to locate ollama sidecar binary")?;
        cmd = cmd
            .env("OLLAMA_HOST", format!("{OLLAMA_HOST}:{OLLAMA_PORT}"))
            .env("OLLAMA_MODELS", models_dir.to_string_lossy().as_ref())
            .args(["serve"]);

        let (mut rx, child) = cmd
            .spawn()
            .context("failed to spawn ollama serve")?;

        // Drain sidecar stdout/stderr in the background so the pipe doesn't block.
        tokio::spawn(async move {
            while let Some(event) = rx.recv().await {
                match event {
                    CommandEvent::Stdout(line) | CommandEvent::Stderr(line) => {
                        let text = String::from_utf8_lossy(&line);
                        eprintln!("[ollama] {text}");
                    }
                    CommandEvent::Error(err) => {
                        eprintln!("[ollama] error: {err}");
                    }
                    CommandEvent::Terminated(status) => {
                        eprintln!("[ollama] terminated: {status:?}");
                        break;
                    }
                    _ => {}
                }
            }
        });

        // Wait for the HTTP API to become reachable.
        wait_for_healthy().await?;

        // Pull the default model if not already available.
        ensure_model(DEFAULT_MODEL).await?;

        Ok(Self { _child: child })
    }
}

/// Returns the base URL that the LLM client should use.
pub fn base_url() -> String {
    format!("http://{OLLAMA_HOST}:{OLLAMA_PORT}/v1")
}

pub fn model() -> String {
    std::env::var("OPENAI_MODEL").unwrap_or_else(|_| DEFAULT_MODEL.to_string())
}

/// Determines where Ollama should store/find models.
///
/// In a bundled app, uses the bundled `models/` resource directory so
/// pre-downloaded models work immediately. In dev mode, uses the default
/// Ollama location (`~/.ollama/models`) so models persist across runs.
fn resolve_models_dir(app: &AppHandle) -> std::path::PathBuf {
    // Bundled app: use the resource directory with pre-pulled models.
    if let Ok(resource_dir) = app.path().resource_dir() {
        let bundled = resource_dir.join("models");
        if bundled.exists() {
            return bundled;
        }
    }

    // Dev mode / fallback: use Ollama's default location so models
    // persist across runs and aren't re-downloaded every time.
    let home_var = if cfg!(windows) { "USERPROFILE" } else { "HOME" };
    if let Ok(home) = std::env::var(home_var) {
        let default_dir = std::path::PathBuf::from(home).join(".ollama").join("models");
        std::fs::create_dir_all(&default_dir).ok();
        return default_dir;
    }

    // Last resort: next to the executable.
    if let Ok(exe) = std::env::current_exe() {
        if let Some(exe_dir) = exe.parent() {
            let local = exe_dir.join("models");
            std::fs::create_dir_all(&local).ok();
            return local;
        }
    }

    std::path::PathBuf::from("models")
}

/// Polls the Ollama `/api/tags` endpoint until it responds.
async fn wait_for_healthy() -> Result<()> {
    let url = format!("http://{OLLAMA_HOST}:{OLLAMA_PORT}/api/tags");
    let client = reqwest::Client::new();

    for attempt in 1..=60 {
        match client.get(&url).send().await {
            Ok(resp) if resp.status().is_success() => return Ok(()),
            _ => {
                if attempt % 10 == 0 {
                    eprintln!("[ollama] waiting for server (attempt {attempt}/60)...");
                }
                tokio::time::sleep(std::time::Duration::from_millis(500)).await;
            }
        }
    }
    bail!("Ollama did not become healthy within 30 seconds")
}

/// Checks whether `model_name` is already pulled; if not, pulls it.
async fn ensure_model(model_name: &str) -> Result<()> {
    let client = reqwest::Client::new();
    let tags_url = format!("http://{OLLAMA_HOST}:{OLLAMA_PORT}/api/tags");

    let resp = client
        .get(&tags_url)
        .send()
        .await
        .context("failed to list models")?;
    let body: serde_json::Value = resp.json().await.context("failed to parse model list")?;

    // Check if model is already present.
    if let Some(models) = body.get("models").and_then(|m| m.as_array()) {
        for m in models {
            if let Some(name) = m.get("name").and_then(|n| n.as_str()) {
                // Ollama names include `:latest` suffix, so check prefix.
                if name == model_name || name.starts_with(&format!("{model_name}:")) {
                    eprintln!("[ollama] model '{model_name}' already available");
                    return Ok(());
                }
            }
        }
    }

    eprintln!("[ollama] pulling model '{model_name}' — this may take a while on first run...");
    let pull_url = format!("http://{OLLAMA_HOST}:{OLLAMA_PORT}/api/pull");
    let resp = client
        .post(&pull_url)
        .json(&serde_json::json!({ "name": model_name }))
        .send()
        .await
        .context("failed to start model pull")?;

    if !resp.status().is_success() {
        bail!(
            "model pull returned HTTP {}: {}",
            resp.status(),
            resp.text().await.unwrap_or_default()
        );
    }

    // The pull endpoint streams newline-delimited JSON with progress updates.
    // Read line by line so progress is printed in real time.
    use futures::StreamExt;
    use tokio::io::AsyncBufReadExt;
    let byte_stream = resp.bytes_stream().map(|r| {
        r.map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))
    });
    let mut stream = tokio::io::BufReader::new(
        tokio_util::io::StreamReader::new(byte_stream),
    );
    let mut line_buf = String::new();
    loop {
        line_buf.clear();
        let n = stream.read_line(&mut line_buf).await.context("failed to read pull stream")?;
        if n == 0 { break; }
        if let Ok(obj) = serde_json::from_str::<serde_json::Value>(line_buf.trim()) {
            if obj.get("error").is_some() {
                bail!("model pull error: {}", line_buf.trim());
            }
            let status = obj.get("status").and_then(|s| s.as_str()).unwrap_or("");
            if let (Some(completed), Some(total)) = (
                obj.get("completed").and_then(|v| v.as_u64()),
                obj.get("total").and_then(|v| v.as_u64()),
            ) {
                if total > 0 {
                    let pct = completed as f64 / total as f64 * 100.0;
                    let completed_mb = completed / 1_000_000;
                    let total_mb = total / 1_000_000;
                    eprint!("\r[ollama] {status}: {completed_mb}/{total_mb} MB ({pct:.0}%)    ");
                }
            } else if !status.is_empty() {
                eprintln!("[ollama] {status}");
            }
        }
    }
    eprintln!(); // newline after progress updates

    eprintln!("[ollama] model '{model_name}' ready");
    Ok(())
}
