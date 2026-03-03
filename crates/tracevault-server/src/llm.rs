use async_trait::async_trait;

#[async_trait]
pub trait StoryLlm: Send + Sync {
    async fn generate(&self, prompt: &str, max_tokens: u32) -> Result<String, String>;
    fn provider_name(&self) -> &str;
    fn model_name(&self) -> &str;
}

pub struct AnthropicLlm {
    client: reqwest::Client,
    api_key: String,
    model: String,
    base_url: String,
}

impl AnthropicLlm {
    pub fn new(api_key: String, model: String, base_url: Option<String>) -> Self {
        Self {
            client: reqwest::Client::new(),
            api_key,
            model,
            base_url: base_url.unwrap_or_else(|| "https://api.anthropic.com".to_string()),
        }
    }
}

#[async_trait]
impl StoryLlm for AnthropicLlm {
    async fn generate(&self, prompt: &str, max_tokens: u32) -> Result<String, String> {
        let resp = self
            .client
            .post(format!("{}/v1/messages", self.base_url))
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&serde_json::json!({
                "model": self.model,
                "max_tokens": max_tokens,
                "messages": [{"role": "user", "content": prompt}]
            }))
            .send()
            .await
            .map_err(|e| e.to_string())?;

        let body: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
        body["content"][0]["text"]
            .as_str()
            .map(|s| s.to_string())
            .ok_or_else(|| format!("Unexpected response: {body}"))
    }

    fn provider_name(&self) -> &str {
        "anthropic"
    }
    fn model_name(&self) -> &str {
        &self.model
    }
}

pub struct OpenAiLlm {
    client: reqwest::Client,
    api_key: String,
    model: String,
    base_url: String,
}

impl OpenAiLlm {
    pub fn new(api_key: String, model: String, base_url: Option<String>) -> Self {
        Self {
            client: reqwest::Client::new(),
            api_key,
            model,
            base_url: base_url.unwrap_or_else(|| "https://api.openai.com".to_string()),
        }
    }
}

#[async_trait]
impl StoryLlm for OpenAiLlm {
    async fn generate(&self, prompt: &str, max_tokens: u32) -> Result<String, String> {
        let resp = self
            .client
            .post(format!("{}/v1/chat/completions", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("content-type", "application/json")
            .json(&serde_json::json!({
                "model": self.model,
                "max_tokens": max_tokens,
                "messages": [{"role": "user", "content": prompt}]
            }))
            .send()
            .await
            .map_err(|e| e.to_string())?;

        let body: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
        body["choices"][0]["message"]["content"]
            .as_str()
            .map(|s| s.to_string())
            .ok_or_else(|| format!("Unexpected response: {body}"))
    }

    fn provider_name(&self) -> &str {
        "openai"
    }
    fn model_name(&self) -> &str {
        &self.model
    }
}

pub fn create_llm(config: &crate::config::ServerConfig) -> Option<Box<dyn StoryLlm>> {
    let provider = config.llm_provider.as_deref()?;
    let api_key = config.llm_api_key.clone()?;
    let model = config.llm_model.clone().unwrap_or_else(|| match provider {
        "anthropic" => "claude-sonnet-4-20250514".to_string(),
        "openai" => "gpt-4o".to_string(),
        _ => "unknown".to_string(),
    });

    match provider {
        "anthropic" => Some(Box::new(AnthropicLlm::new(
            api_key,
            model,
            config.llm_base_url.clone(),
        ))),
        "openai" => Some(Box::new(OpenAiLlm::new(
            api_key,
            model,
            config.llm_base_url.clone(),
        ))),
        _ => {
            tracing::warn!("Unknown LLM provider: {provider}");
            None
        }
    }
}
