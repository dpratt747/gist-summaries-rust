use std::collections::HashMap;

use anyhow::Result;
use futures::stream::{self, StreamExt};
use reqwest::Client;
use serde::Deserialize;

#[derive(Debug, Deserialize, Hash, Eq, PartialEq)]
pub struct FileName(pub String);
#[derive(Debug, Deserialize)]
pub struct Url(pub String);

#[derive(Debug, Default)]
pub struct Content(pub String);

#[derive(Debug, Deserialize)]
pub struct GistFile {
    pub filename: FileName,
    pub raw_url: Url,
    #[serde(skip)]
    pub content: Content,
}

#[derive(Debug, Deserialize)]
pub struct GistsInformation {
    pub html_url: String,
    pub files: HashMap<FileName, GistFile>,
}

#[derive(Debug, Deserialize)]
struct UserInfo {
    public_gists: usize,
}

pub struct GithubClient {
    client: Client,
    token: Option<String>,
}

impl GithubClient {
    pub fn with_token(token: String) -> Result<Self> {
        let client = Client::builder().user_agent("gist-summary").build()?;
        let trimmed = token.trim().to_string();
        let resolved = (!trimmed.is_empty())
            .then_some(trimmed)
            .or_else(|| std::env::var("GITHUB_TOKEN").ok());
        Ok(Self { client, token: resolved })
    }

    fn auth_request(&self, url: &str) -> reqwest::RequestBuilder {
        let req = self.client.get(url);
        match &self.token {
            Some(t) => req.bearer_auth(t),
            None => req,
        }
    }

    pub async fn get_gists(
        &self,
        user: &str,
        on_progress: impl Fn(&str, usize, usize),
    ) -> Result<Vec<GistsInformation>> {
        let user_resp = self
            .auth_request(&format!("https://api.github.com/users/{user}"))
            .send()
            .await?;
        let user_status = user_resp.status();
        if !user_status.is_success() {
            let body = user_resp.text().await.unwrap_or_default();
            anyhow::bail!("GitHub API request failed with status {user_status}: {body}");
        }
        let user_info: UserInfo = user_resp.json().await?;
        let total_gists = user_info.public_gists;

        let mut all_gists: Vec<GistsInformation> = Vec::new();
        let mut page = 1u32;

        on_progress("gists", 0, total_gists.max(1));
        loop {
            let url = format!(
                "https://api.github.com/users/{user}/gists?per_page=100&page={page}"
            );
            let response = self.auth_request(&url).send().await?;
            let status = response.status();
            if !status.is_success() {
                let body = response.text().await.unwrap_or_default();
                anyhow::bail!("GitHub API request failed with status {status}: {body}");
            }
            let gists: Vec<GistsInformation> = response.json().await?;
            let count = gists.len();
            all_gists.extend(gists);
            on_progress("gists", all_gists.len(), all_gists.len().max(total_gists));
            if count < 100 {
                break;
            }
            page += 1;
        }

        const CONCURRENT_DOWNLOADS: usize = 10;

        let download_tasks: Vec<(usize, String, String)> = all_gists
            .iter()
            .enumerate()
            .flat_map(|(gi, gist)| {
                gist.files
                    .values()
                    .map(move |f| (gi, f.filename.0.clone(), f.raw_url.0.clone()))
            })
            .collect();

        let total_files = download_tasks.len();
        on_progress("files", 0, total_files);

        let downloaded = std::sync::atomic::AtomicUsize::new(0);

        let results: Vec<Result<(usize, String, String)>> = stream::iter(download_tasks)
            .map(|(gi, fname, url)| {
                let req = self.auth_request(&url);
                let downloaded = &downloaded;
                let on_progress = &on_progress;
                async move {
                    let text = req.send().await?.text().await?;
                    let done = downloaded.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1;
                    on_progress("files", done, total_files);
                    Ok((gi, fname, text))
                }
            })
            .buffer_unordered(CONCURRENT_DOWNLOADS)
            .collect()
            .await;

        for result in results {
            let (gi, fname, text) = result?;
            if let Some(file) = all_gists[gi].files.get_mut(&FileName(fname)) {
                file.content = Content(text);
            }
        }

        Ok(all_gists)
    }
}
