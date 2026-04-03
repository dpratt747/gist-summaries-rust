use anyhow::Result;
use langchain_rust::{
    language_models::llm::LLM,
    llm::openai::{OpenAI, OpenAIConfig},
};

pub struct LlmClient {
    llm: OpenAI<OpenAIConfig>,
}

impl LlmClient {
    pub fn new(base_url: String, model: String) -> Self {
        let api_key = std::env::var("OPENAI_API_KEY").unwrap_or_else(|_| "ollama".to_string());

        let llm = OpenAI::default()
            .with_config(
                OpenAIConfig::default()
                    .with_api_base(base_url)
                    .with_api_key(api_key),
            )
            .with_model(model);

        Self { llm }
    }

    pub async fn ask(&self, question: &str) -> Result<String> {
        let answer = self.llm.invoke(question).await?;
        Ok(answer)
    }
}
