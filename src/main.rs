mod github;
mod llm;

use anyhow::Result;
use std::collections::HashMap;
use std::fmt;

#[derive(Debug, Hash, PartialEq, Eq)]
struct FileName(pub String);
#[derive(Debug)]
struct GistUrl(pub String);
#[derive(Debug)]
struct Summary(pub String);

impl fmt::Display for FileName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { write!(f, "{}", self.0) }
}
impl fmt::Display for GistUrl {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { write!(f, "{}", self.0) }
}
impl fmt::Display for Summary {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { write!(f, "{}", self.0) }
}

#[derive(Debug)]
struct GistSummaryEntry {
    gist_url: GistUrl,
    summary: Summary,
}

#[tokio::main]
async fn main() -> Result<()> {
    let client = llm::LlmClient::from_env();
    let github = github::GithubClient::new();
    let gists = github.get_gists("dpratt747").await?;

    let mut summaries: HashMap<FileName, GistSummaryEntry> = HashMap::new();

    for gist in &gists {
        for file in gist.files.values() {
            let prompt = format!(
                "Summarise the following file ('{}') in one sentence:\n\n{}",
                file.filename, file.content
            );
            let summary = client.ask(&prompt).await?;
            summaries.insert(
                FileName(file.filename.clone()),
                GistSummaryEntry {
                    gist_url: GistUrl(gist.html_url.clone()),
                    summary: Summary(summary.trim().to_string()),
                },
            );
        }
    }

    println!("{:<50} {:<60} {}", "Filename", "Gist URL", "Summary");
    println!("{}", "-".repeat(160));
    for (filename, entry) in &summaries {
        println!("{:<50} {:<60} {}", filename, entry.gist_url, entry.summary);
    }

    Ok(())
}
