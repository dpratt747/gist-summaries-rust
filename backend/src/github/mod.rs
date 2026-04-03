use std::collections::HashMap;

use anyhow::Result;
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

    pub async fn get_gists(&self, user: &str) -> Result<Vec<GistsInformation>> {
        let mut all_gists: Vec<GistsInformation> = Vec::new();
        let mut page = 1u32;

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
            if count < 100 {
                break;
            }
            page += 1;
        }

        for gist in &mut all_gists {
            for file in gist.files.values_mut() {
                file.content = Content(
                    self.auth_request(&file.raw_url.0).send().await?.text().await?,
                );
            }
        }

        Ok(all_gists)
    }
}
