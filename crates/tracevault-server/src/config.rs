use std::env;

pub struct ServerConfig {
    pub database_url: String,
    pub host: String,
    pub port: u16,
    pub cors_origin: String,
    pub repos_dir: String,
    pub encryption_key: Option<String>,
    pub invite_expiry_minutes: u64,
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
            cors_origin: env::var("CORS_ORIGIN")
                .expect("CORS_ORIGIN environment variable is required"),
            repos_dir: env::var("TRACEVAULT_REPOS_DIR").unwrap_or_else(|_| "./data/repos".into()),
            encryption_key: env::var("TRACEVAULT_ENCRYPTION_KEY").ok(),
            invite_expiry_minutes: env::var("INVITE_EXPIRY_MINUTES")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(1440),
        }
    }

    pub fn bind_addr(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bind_addr_format() {
        let cfg = ServerConfig {
            database_url: String::new(),
            host: "127.0.0.1".into(),
            port: 8080,
            cors_origin: "http://localhost:4000".into(),
            repos_dir: ".".into(),
            encryption_key: None,

            invite_expiry_minutes: 1440,
        };
        assert_eq!(cfg.bind_addr(), "127.0.0.1:8080");
    }

    #[test]
    fn cors_origin_is_required() {
        let cfg = ServerConfig {
            database_url: String::new(),
            host: "127.0.0.1".into(),
            port: 8080,
            cors_origin: "http://localhost:4000".into(),
            repos_dir: ".".into(),
            encryption_key: None,

            invite_expiry_minutes: 1440,
        };
        assert_eq!(cfg.cors_origin, "http://localhost:4000");
    }
}
