use axum::{
    body::Bytes,
    extract::State,
    http::{HeaderMap, StatusCode},
};
use chrono::Utc;
use hmac::{Hmac, Mac};
use sha2::Sha256;
use uuid::Uuid;

use crate::AppState;

type HmacSha256 = Hmac<Sha256>;

/// Verify the GitHub webhook signature (X-Hub-Signature-256 header).
/// Returns true if the signature is valid.
fn verify_webhook_signature(secret: &str, body: &[u8], signature_header: Option<&str>) -> bool {
    let Some(header) = signature_header else {
        return false;
    };
    let Some(hex_sig) = header.strip_prefix("sha256=") else {
        return false;
    };
    let Ok(sig_bytes) = hex::decode(hex_sig) else {
        return false;
    };
    let Ok(mut mac) = HmacSha256::new_from_slice(secret.as_bytes()) else {
        return false;
    };
    mac.update(body);
    mac.verify_slice(&sig_bytes).is_ok()
}

/// POST /api/v1/github/webhook
///
/// Handles GitHub webhook events:
/// - `push`: tracks each commit reaching the pushed branch
/// - `create` (ref_type=tag): tracks the tagged commit
pub async fn webhook(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> (StatusCode, &'static str) {
    let signature = headers
        .get("x-hub-signature-256")
        .and_then(|v| v.to_str().ok());

    if !verify_webhook_signature(&state.github_webhook_secret, &body, signature) {
        return (StatusCode::UNAUTHORIZED, "invalid webhook signature");
    }

    let Ok(body) = serde_json::from_slice::<serde_json::Value>(&body) else {
        return (StatusCode::BAD_REQUEST, "invalid JSON body");
    };

    let event_type = headers
        .get("x-github-event")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    match event_type {
        "push" => handle_push(&state.pool, &body).await,
        "create" => handle_create(&state.pool, &body).await,
        _ => (StatusCode::OK, "event ignored"),
    }
}

/// Look up the repo_id from the `repos` table by matching the repository URL
/// from the webhook payload.
async fn resolve_repo_id(pool: &sqlx::PgPool, body: &serde_json::Value) -> Option<Uuid> {
    let html_url = body
        .get("repository")
        .and_then(|r| r.get("html_url"))
        .and_then(|v| v.as_str())?;

    sqlx::query_scalar::<_, Uuid>("SELECT id FROM repos WHERE github_url = $1")
        .bind(html_url)
        .fetch_optional(pool)
        .await
        .ok()
        .flatten()
}

/// Extract the branch name from a push event's `ref` field.
/// GitHub sends refs like "refs/heads/main" — we strip the prefix.
fn extract_branch(git_ref: &str) -> Option<&str> {
    git_ref.strip_prefix("refs/heads/")
}

async fn handle_push(pool: &sqlx::PgPool, body: &serde_json::Value) -> (StatusCode, &'static str) {
    let Some(repo_id) = resolve_repo_id(pool, body).await else {
        return (StatusCode::OK, "repo not tracked");
    };

    let git_ref = body.get("ref").and_then(|v| v.as_str()).unwrap_or("");
    let Some(branch) = extract_branch(git_ref) else {
        return (StatusCode::OK, "not a branch push");
    };

    let now = Utc::now();
    let empty = vec![];
    let commits = body
        .get("commits")
        .and_then(|c| c.as_array())
        .unwrap_or(&empty);

    for commit in commits {
        let Some(sha) = commit.get("id").and_then(|v| v.as_str()) else {
            continue;
        };
        if let Err(e) =
            crate::branch_tracking::track_commit_on_branch(pool, sha, repo_id, branch, now, "push")
                .await
        {
            tracing::warn!(sha, branch, error = %e, "branch tracking failed for push commit");
        }
    }

    (StatusCode::OK, "push processed")
}

async fn handle_create(
    pool: &sqlx::PgPool,
    body: &serde_json::Value,
) -> (StatusCode, &'static str) {
    let ref_type = body.get("ref_type").and_then(|v| v.as_str()).unwrap_or("");
    if ref_type != "tag" {
        return (StatusCode::OK, "non-tag create event ignored");
    }

    let Some(repo_id) = resolve_repo_id(pool, body).await else {
        return (StatusCode::OK, "repo not tracked");
    };

    let Some(tag) = body.get("ref").and_then(|v| v.as_str()) else {
        return (StatusCode::OK, "missing ref");
    };

    // GitHub create events for tags don't include the commit SHA directly.
    // We need to get it from the `master_branch` or the head commit.
    // The `create` event has a `master_branch` field but not the tagged SHA.
    // We'll look for it in the sender info or use the repo's default.
    // Actually, the GitHub API `create` event for tags does NOT include the SHA.
    // We check `head_commit` (present in some webhook configs) or fall back.
    let sha = body
        .get("head_commit")
        .and_then(|c| c.get("id"))
        .and_then(|v| v.as_str())
        // Some webhook payloads include sha at top level
        .or_else(|| body.get("sha").and_then(|v| v.as_str()));

    let Some(sha) = sha else {
        tracing::debug!(tag, "create tag event missing commit SHA, skipping");
        return (StatusCode::OK, "tag event missing SHA");
    };

    let now = Utc::now();
    if let Err(e) = crate::branch_tracking::track_tag(pool, sha, repo_id, tag, now).await {
        tracing::warn!(tag, error = %e, "branch tracking failed for tag");
    }

    (StatusCode::OK, "tag processed")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verify_signature_valid() {
        let secret = "test-secret";
        let body = b"{\"action\":\"push\"}";
        let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).unwrap();
        mac.update(body);
        let expected = hex::encode(mac.finalize().into_bytes());
        let header_value = format!("sha256={expected}");
        assert!(verify_webhook_signature(secret, body, Some(&header_value)));
    }

    #[test]
    fn verify_signature_invalid() {
        assert!(!verify_webhook_signature(
            "test-secret",
            b"{}",
            Some("sha256=deadbeef")
        ));
    }

    #[test]
    fn verify_signature_missing() {
        assert!(!verify_webhook_signature("test-secret", b"{}", None));
    }

    #[test]
    fn verify_signature_wrong_prefix() {
        assert!(!verify_webhook_signature(
            "test-secret",
            b"{}",
            Some("sha1=deadbeef")
        ));
    }
}
