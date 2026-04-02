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
    pub async fn start(app: &AppHandle) -> Result<Self> {
        let models_dir = resolve_models_dir(app);
        eprintln!("[ollama] models directory: {}", models_dir.display());

        let (mut rx, child) = app
            .shell()
            .sidecar("ollama")
            .context("failed to locate ollama sidecar binary")?
            .env("OLLAMA_HOST", format!("{OLLAMA_HOST}:{OLLAMA_PORT}"))
            .env("OLLAMA_MODELS", models_dir.to_string_lossy().as_ref())
            .args(["serve"])
            .spawn()
            .context("failed to spawn ollama serve")?;

        tokio::spawn(async move {
            while let Some(event) = rx.recv().await {
                match event {
                    CommandEvent::Stdout(line) | CommandEvent::Stderr(line) => {
                        eprintln!("[ollama] {}", String::from_utf8_lossy(&line));
                    }
                    CommandEvent::Terminated(status) => {
                        eprintln!("[ollama] terminated: {status:?}");
                        break;
                    }
                    _ => {}
                }
            }
        });

        wait_for_healthy().await?;
        ensure_model(DEFAULT_MODEL).await?;

        Ok(Self { _child: child })
    }
}

pub fn base_url() -> String {
    format!("http://{OLLAMA_HOST}:{OLLAMA_PORT}/v1")
}

pub fn model() -> String {
    std::env::var("OPENAI_MODEL").unwrap_or_else(|_| DEFAULT_MODEL.to_string())
}

fn resolve_models_dir(app: &AppHandle) -> std::path::PathBuf {
    if let Ok(resource_dir) = app.path().resource_dir() {
        let bundled = resource_dir.join("models");
        if bundled.exists() {
            return bundled;
        }
    }

    let home_var = if cfg!(windows) { "USERPROFILE" } else { "HOME" };
    if let Ok(home) = std::env::var(home_var) {
        let dir = std::path::PathBuf::from(home).join(".ollama").join("models");
        std::fs::create_dir_all(&dir).ok();
        return dir;
    }

    std::path::PathBuf::from("models")
}

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

async fn ensure_model(model_name: &str) -> Result<()> {
    let client = reqwest::Client::new();
    let base = format!("http://{OLLAMA_HOST}:{OLLAMA_PORT}/api");

    let body: serde_json::Value = client
        .get(format!("{base}/tags"))
        .send()
        .await
        .context("failed to list models")?
        .json()
        .await
        .context("failed to parse model list")?;

    if let Some(models) = body.get("models").and_then(|m| m.as_array()) {
        for m in models {
            if let Some(name) = m.get("name").and_then(|n| n.as_str()) {
                if name == model_name || name.starts_with(&format!("{model_name}:")) {
                    eprintln!("[ollama] model '{model_name}' already available");
                    return Ok(());
                }
            }
        }
    }

    eprintln!("[ollama] pulling model '{model_name}'...");
    let resp = client
        .post(format!("{base}/pull"))
        .json(&serde_json::json!({ "name": model_name }))
        .send()
        .await
        .context("failed to start model pull")?;

    if !resp.status().is_success() {
        bail!("model pull HTTP {}: {}", resp.status(), resp.text().await.unwrap_or_default());
    }

    use futures::StreamExt;
    use tokio::io::AsyncBufReadExt;
    let byte_stream = resp.bytes_stream().map(|r| {
        r.map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))
    });
    let mut reader = tokio::io::BufReader::new(tokio_util::io::StreamReader::new(byte_stream));
    let mut line = String::new();
    loop {
        line.clear();
        if reader.read_line(&mut line).await.context("failed to read pull stream")? == 0 {
            break;
        }
        if let Ok(obj) = serde_json::from_str::<serde_json::Value>(line.trim()) {
            if obj.get("error").is_some() {
                bail!("model pull error: {}", line.trim());
            }
            let status = obj.get("status").and_then(|s| s.as_str()).unwrap_or("");
            match (obj.get("completed").and_then(|v| v.as_u64()), obj.get("total").and_then(|v| v.as_u64())) {
                (Some(done), Some(total)) if total > 0 => {
                    eprint!("\r[ollama] {status}: {}/{} MB ({:.0}%)    ", done / 1_000_000, total / 1_000_000, done as f64 / total as f64 * 100.0);
                }
                _ if !status.is_empty() => eprintln!("[ollama] {status}"),
                _ => {}
            }
        }
    }
    eprintln!();
    eprintln!("[ollama] model '{model_name}' ready");
    Ok(())
}
