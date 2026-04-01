use std::collections::HashMap;

use anyhow::Result;
use reqwest::Client;
use serde::Deserialize;
use std::env;

#[derive(Debug, Deserialize)]
pub struct GistFile {
    pub filename: String,
    pub raw_url: String,
    #[serde(skip)]
    pub content: String,
}

#[derive(Debug, Deserialize)]
pub struct Gist {
    pub id: String,
    pub html_url: String,
    pub description: Option<String>,
    pub files: HashMap<String, GistFile>,
}

pub struct GithubClient {
    client: Client,
    token: Option<String>,
}

impl GithubClient {
    pub fn new() -> Self {
        let client = Client::builder()
            .user_agent("gist-summary")
            .build()
            .expect("failed to build HTTP client");
        let token = env::var("GITHUB_TOKEN").ok();
        Self { client, token }
    }

    fn auth_request(&self, url: &str) -> reqwest::RequestBuilder {
        let req = self.client.get(url);
        match &self.token {
            Some(t) => req.bearer_auth(t),
            None => req,
        }
    }

    pub async fn get_gists(&self, user: &str) -> Result<Vec<Gist>> {
        let url = format!("https://api.github.com/users/{user}/gists");
        let response = self.auth_request(&url).send().await?;
        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("GitHub API request failed with status {status}: {body}");
        }
        let mut gists: Vec<Gist> = response.json().await?;

        // Fetch the raw content of every file in every gist
        for gist in &mut gists {
            for file in gist.files.values_mut() {
                file.content = self.auth_request(&file.raw_url)
                    .send()
                    .await?
                    .text()
                    .await?;
            }
        }

        Ok(gists)
    }
}

