mod github;
mod llm;
mod ollama;

use std::collections::HashMap;
use std::sync::Mutex;

use serde::Serialize;
use tauri::{Manager, State};

#[derive(Debug, Serialize)]
struct GistFileRow {
    filename: String,
    gist_url: String,
}

struct AppState {
    llm: llm::LlmClient,
    gist_contents: Mutex<HashMap<String, String>>,
    _ollama: ollama::OllamaProcess,
}

fn content_key(gist_url: &str, filename: &str) -> String {
    format!("{gist_url}\0{filename}")
}

#[tauri::command]
async fn get_gists(
    username: String,
    token: String,
    state: State<'_, AppState>,
) -> Result<Vec<GistFileRow>, String> {
    let github = github::GithubClient::with_token(token).map_err(|e| e.to_string())?;
    let gists = github
        .get_gists(&username)
        .await
        .map_err(|e| e.to_string())?;

    let mut rows = Vec::new();
    let mut contents = state.gist_contents.lock().unwrap();
    contents.clear();

    for gist in &gists {
        for file in gist.files.values() {
            contents.insert(
                content_key(&gist.html_url, &file.filename.0),
                file.content.0.clone(),
            );
            rows.push(GistFileRow {
                filename: file.filename.0.clone(),
                gist_url: gist.html_url.clone(),
            });
        }
    }
    Ok(rows)
}

#[tauri::command]
async fn summarise_file(
    gist_url: String,
    filename: String,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let content = {
        let contents = state.gist_contents.lock().unwrap();
        contents
            .get(&content_key(&gist_url, &filename))
            .cloned()
            .ok_or_else(|| format!("no cached content for {filename}"))?
    };
    let prompt = format!(
        "Summarise the following file ('{}') in one sentence:\n\n{}",
        filename, content
    );
    state
        .llm
        .ask(&prompt)
        .await
        .map(|s| s.trim().to_string())
        .map_err(|e| e.to_string())
}

fn env_path() -> Result<std::path::PathBuf, String> {
    std::env::current_dir()
        .map(|d| d.join(".env"))
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn load_token() -> Result<String, String> {
    let path = env_path()?;
    if !path.exists() {
        return Ok(String::new());
    }
    let content = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
    for line in content.lines() {
        if let Some(val) = line.strip_prefix("GITHUB_TOKEN=") {
            return Ok(val.trim().to_string());
        }
    }
    Ok(String::new())
}

#[tauri::command]
async fn save_token(token: String) -> Result<(), String> {
    let path = env_path()?;
    let mut lines: Vec<String> = if path.exists() {
        std::fs::read_to_string(&path)
            .map_err(|e| e.to_string())?
            .lines()
            .map(String::from)
            .collect()
    } else {
        Vec::new()
    };
    let token_line = format!("GITHUB_TOKEN={}", token.trim());
    let mut found = false;
    for line in &mut lines {
        if line.starts_with("GITHUB_TOKEN=") {
            *line = token_line.clone();
            found = true;
            break;
        }
    }
    if !found {
        lines.push(token_line);
    }
    std::fs::write(&path, lines.join("\n") + "\n").map_err(|e| e.to_string())
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            let handle = app.handle().clone();
            let ollama = tauri::async_runtime::block_on(async {
                ollama::OllamaProcess::start(&handle).await
            })
            .expect("failed to start Ollama sidecar");

            let llm = llm::LlmClient::new(ollama::base_url(), ollama::model());

            app.manage(AppState {
                llm,
                gist_contents: Mutex::new(HashMap::new()),
                _ollama: ollama,
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![get_gists, summarise_file, load_token, save_token])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
