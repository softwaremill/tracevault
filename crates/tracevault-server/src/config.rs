use std::env;

pub struct ServerConfig {
    pub database_url: String,
    pub host: String,
    pub port: u16,
    pub cors_origin: Option<String>,
    pub signing_key_seed: Option<String>,
    pub repos_dir: String,
    pub llm_provider: Option<String>,
    pub llm_api_key: Option<String>,
    pub llm_model: Option<String>,
    pub llm_base_url: Option<String>,
    pub encryption_key: Option<String>,
}

impl ServerConfig {
    pub fn from_env() -> Self {
        Self {
            database_url: env::var("DATABASE_URL").unwrap_or_else(|_| {
                "postgres://tracevault:tracevault@localhost:5432/tracevault".into()
            }),
            host: env::var("HOST").unwrap_or_else(|_| "0.0.0.0".into()),
            port: env::var("PORT")
                .ok()
                .and_then(|p| p.parse().ok())
                .unwrap_or(3000),
            cors_origin: env::var("CORS_ORIGIN").ok(),
            signing_key_seed: env::var("TRACEVAULT_SIGNING_KEY").ok(),
            repos_dir: env::var("TRACEVAULT_REPOS_DIR").unwrap_or_else(|_| "./data/repos".into()),
            llm_provider: env::var("TRACEVAULT_LLM_PROVIDER").ok(),
            llm_api_key: env::var("TRACEVAULT_LLM_API_KEY").ok(),
            llm_model: env::var("TRACEVAULT_LLM_MODEL").ok(),
            llm_base_url: env::var("TRACEVAULT_LLM_BASE_URL").ok(),
            encryption_key: env::var("TRACEVAULT_ENCRYPTION_KEY").ok(),
        }
    }

    pub fn bind_addr(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}
