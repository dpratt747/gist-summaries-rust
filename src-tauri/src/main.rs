mod github;
mod llm;

use std::collections::HashMap;
use std::sync::Mutex;

use serde::Serialize;
use tauri::State;

#[derive(Debug, Serialize)]
struct GistFileRow {
    filename: String,
    gist_url: String,
}

struct AppState {
    llm: llm::LlmClient,
    gist_contents: Mutex<HashMap<String, String>>,
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

fn main() {
    tauri::Builder::default()
        .manage(AppState {
            llm: llm::LlmClient::from_env(),
            gist_contents: Mutex::new(HashMap::new()),
        })
        .invoke_handler(tauri::generate_handler![get_gists, summarise_file])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
