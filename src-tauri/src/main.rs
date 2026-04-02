mod github;
mod llm;

use serde::Serialize;

#[derive(Debug, Serialize)]
struct GistFileRow {
    filename: String,
    gist_url: String,
    content: String,
}

#[derive(Debug, Serialize)]
struct GistSummaryRow {
    filename: String,
    gist_url: String,
    summary: String,
}

#[tauri::command]
async fn get_gists(username: String) -> Result<Vec<GistFileRow>, String> {
    let github = github::GithubClient::new();
    let gists = github
        .get_gists(&username)
        .await
        .map_err(|e| e.to_string())?;

    let mut rows = Vec::new();
    for gist in &gists {
        for file in gist.files.values() {
            rows.push(GistFileRow {
                filename: file.filename.0.clone(),
                gist_url: gist.html_url.clone(),
                content: file.content.0.clone(),
            });
        }
    }
    Ok(rows)
}

#[tauri::command]
async fn summarise_file(filename: String, content: String) -> Result<String, String> {
    let client = llm::LlmClient::from_env();
    let prompt = format!(
        "Summarise the following file ('{}') in one sentence:\n\n{}",
        filename, content
    );
    client
        .ask(&prompt)
        .await
        .map(|s| s.trim().to_string())
        .map_err(|e| e.to_string())
}

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![get_gists, summarise_file])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
