use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageCursor {
    pub created_at: DateTime<Utc>,
    pub id: Uuid,
}

impl PageCursor {
    pub fn encode(&self) -> String {
        use base64::Engine;
        let json = serde_json::to_string(self).unwrap_or_default();
        base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(json)
    }

    pub fn decode(s: &str) -> Option<Self> {
        use base64::Engine;
        let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .decode(s)
            .ok()?;
        serde_json::from_slice(&bytes).ok()
    }
}

#[derive(Debug, Serialize)]
pub struct Paginated<T: Serialize> {
    pub items: Vec<T>,
    pub next_cursor: Option<String>,
    pub total_count: i64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode_decode_roundtrip() {
        let cursor = PageCursor {
            created_at: Utc::now(),
            id: Uuid::new_v4(),
        };
        let encoded = cursor.encode();
        let decoded = PageCursor::decode(&encoded).unwrap();
        assert_eq!(decoded.id, cursor.id);
    }

    #[test]
    fn decode_invalid_base64_returns_none() {
        assert!(PageCursor::decode("not-valid-base64!!!").is_none());
    }

    #[test]
    fn decode_valid_base64_invalid_json_returns_none() {
        use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
        let encoded = URL_SAFE_NO_PAD.encode(b"not json");
        assert!(PageCursor::decode(&encoded).is_none());
    }
}
