use std::collections::HashMap;

use anyhow::Result;
use reqwest::Client;
use serde::Deserialize;
use std::env;

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

    pub fn with_token(token: String) -> Self {
        let client = Client::builder()
            .user_agent("gist-summary")
            .build()
            .expect("failed to build HTTP client");
        // Use the provided token, falling back to the GITHUB_TOKEN env var
        let resolved = (!token.is_empty())
            .then_some(token)
            .or_else(|| env::var("GITHUB_TOKEN").ok());
        Self { client, token: resolved }
    }

    fn auth_request(&self, url: &str) -> reqwest::RequestBuilder {
        let req = self.client.get(url);
        match &self.token {
            Some(t) => req.bearer_auth(t),
            None => req,
        }
    }

    pub async fn get_gists(&self, user: &str) -> Result<Vec<GistsInformation>> {
        let url = format!("https://api.github.com/users/{user}/gists");
        let response = self.auth_request(&url).send().await?;
        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("GitHub API request failed with status {status}: {body}");
        }
        let mut gists: Vec<GistsInformation> = response.json().await?;

        for gist in &mut gists {
            for file in gist.files.values_mut() {
                file.content = Content(self.auth_request(&file.raw_url.0)
                    .send()
                    .await?
                    .text()
                    .await?);
            }
        }

        Ok(gists)
    }
}

