use serde::{Deserialize, Serialize};
use std::error::Error;

pub struct ApiClient {
    base_url: String,
    api_key: Option<String>,
    client: reqwest::Client,
}

#[derive(Serialize)]
pub struct PushTraceRequest {
    pub repo_name: String,
    pub org_name: String,
    pub commit_sha: String,
    pub branch: Option<String>,
    pub author: String,
    pub model: Option<String>,
    pub tool: Option<String>,
    pub ai_percentage: Option<f32>,
    pub total_tokens: Option<i64>,
    pub input_tokens: Option<i64>,
    pub output_tokens: Option<i64>,
    pub estimated_cost_usd: Option<f64>,
    pub api_calls: Option<i32>,
    pub session_data: Option<serde_json::Value>,
    pub attribution: Option<serde_json::Value>,
}

#[derive(Deserialize)]
pub struct PushTraceResponse {
    pub id: uuid::Uuid,
}

#[derive(Serialize)]
pub struct RegisterRepoRequest {
    pub org_name: String,
    pub repo_name: String,
    pub github_url: Option<String>,
}

#[derive(Deserialize)]
pub struct RegisterRepoResponse {
    pub repo_id: uuid::Uuid,
}

impl ApiClient {
    pub fn new(base_url: &str, api_key: Option<&str>) -> Self {
        Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            api_key: api_key.map(String::from),
            client: reqwest::Client::new(),
        }
    }

    pub async fn push_trace(
        &self,
        req: PushTraceRequest,
    ) -> Result<PushTraceResponse, Box<dyn Error>> {
        let mut builder = self.client.post(format!("{}/api/v1/traces", self.base_url));

        if let Some(key) = &self.api_key {
            builder = builder.header("Authorization", format!("Bearer {key}"));
        }

        let resp = builder.json(&req).send().await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(format!("Server returned {status}: {body}").into());
        }

        Ok(resp.json().await?)
    }

    pub async fn register_repo(
        &self,
        req: RegisterRepoRequest,
    ) -> Result<RegisterRepoResponse, Box<dyn Error>> {
        let mut builder = self.client.post(format!("{}/api/v1/repos", self.base_url));

        if let Some(key) = &self.api_key {
            builder = builder.header("Authorization", format!("Bearer {key}"));
        }

        let resp = builder.json(&req).send().await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(format!("Server returned {status}: {body}").into());
        }

        Ok(resp.json().await?)
    }
}
