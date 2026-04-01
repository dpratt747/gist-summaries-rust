mod github;
mod llm;

use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    // let client = llm::LlmClient::from_env();
    //
    // let question = "Summarise what a GitHub Gist is in one sentence.";
    // println!("Q: {question}");
    // println!("A: {}", client.ask(question).await?);

    let github = github::GithubClient::new();
    let gists = github.get_gists("dpratt747").await?;

    for gist in &gists {
        let desc = gist.description.as_deref().unwrap_or("(no description)");
        println!("Gist {}: {desc}", gist.id);
        for file in gist.files.values() {
            println!("{}: {}", file.filename, file.content);
        }
    }

    Ok(())
}
