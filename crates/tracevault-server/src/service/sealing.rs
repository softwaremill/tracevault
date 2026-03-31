use sha2::{Digest, Sha256};
use sqlx::PgPool;
use uuid::Uuid;

use crate::error::AppError;
use crate::repo::sealing::SealingRepo;

pub struct SealingService;

impl SealingService {
    /// Seal a commit: compute record hash, sign, chain, insert seal, then snapshot attributed sessions.
    pub async fn seal_commit(
        pool: &PgPool,
        commit_id: Uuid,
        encryption_key: Option<&str>,
    ) -> Result<bool, AppError> {
        // 1. Resolve org_id
        let org_id = match SealingRepo::get_org_id_for_commit(pool, commit_id).await? {
            Some(id) => id,
            None => {
                tracing::warn!("seal_commit: commit {commit_id} has no org_id");
                return Ok(false);
            }
        };

        // 2. Check signing enabled
        if !SealingRepo::is_signing_enabled(pool, org_id).await? {
            return Ok(false);
        }

        // 3. Bail if no encryption_key
        let encryption_key = match encryption_key {
            Some(k) => k,
            None => {
                tracing::warn!("seal_commit: no encryption key available");
                return Ok(false);
            }
        };

        // 4. Load signing service
        let svc = match crate::org_signing::load_current(pool, org_id, encryption_key).await {
            Ok(Some(s)) => s,
            Ok(None) => {
                tracing::debug!("seal_commit: no signing key for org {org_id}");
                return Ok(false);
            }
            Err(e) => {
                tracing::warn!("seal_commit: failed to load signing key: {e}");
                return Ok(false);
            }
        };

        // 5. Load commit data
        let (commit_sha, branch, author, message, diff_data, committed_at, attribution) =
            match SealingRepo::get_commit_for_sealing(pool, commit_id).await? {
                Some(row) => row,
                None => {
                    tracing::warn!("seal_commit: commit {commit_id} not found");
                    return Ok(false);
                }
            };

        // 6. Compute record hash
        let record_hash = compute_commit_record_hash(
            &commit_sha,
            branch.as_deref(),
            author.as_deref(),
            message.as_deref(),
            &diff_data,
            committed_at.as_ref(),
            &attribution,
        );

        // 7. Sign
        let signature = svc.sign(&record_hash);

        // 8. Get previous chain hash
        let prev_chain_hash = SealingRepo::get_latest_commit_chain_hash(pool, org_id).await?;

        // 9. Compute chain hash
        let chain_hash = svc.chain_hash(prev_chain_hash.as_deref(), &record_hash);

        // 10. Insert commit seal
        let commit_seal_id = SealingRepo::insert_commit_seal(
            pool,
            commit_id,
            &record_hash,
            &chain_hash,
            prev_chain_hash.as_deref(),
            &signature,
        )
        .await?;

        // 11. Get attributed sessions
        let sessions = SealingRepo::get_attributed_sessions(pool, commit_id).await?;

        // 12. Snapshot each attributed session
        for session_db_id in sessions {
            if let Err(e) =
                seal_session_snapshot(pool, session_db_id, commit_seal_id, org_id, encryption_key)
                    .await
            {
                tracing::warn!("seal_commit: failed to snapshot session {session_db_id}: {e}");
            }
        }

        // 13. Audit log
        crate::audit::log(
            pool,
            crate::audit::AuditEntry {
                org_id,
                actor_id: None,
                api_key_id: None,
                action: "commit.sealed".to_string(),
                resource_type: "commit".to_string(),
                resource_id: Some(commit_id),
                details: Some(serde_json::json!({
                    "commit_sha": commit_sha,
                    "chain_hash": chain_hash,
                })),
                ip_address: None,
                user_agent: None,
            },
        )
        .await;

        Ok(true)
    }

    /// Seal a session with a final seal (session_end or timeout).
    pub async fn seal_session_final(
        pool: &PgPool,
        session_db_id: Uuid,
        org_id: Uuid,
        encryption_key: Option<&str>,
        seal_type: &str,
    ) -> Result<bool, AppError> {
        // 1. Check signing enabled
        if !SealingRepo::is_signing_enabled(pool, org_id).await? {
            return Ok(false);
        }

        // 2. Check if already has final seal
        if SealingRepo::has_final_seal(pool, session_db_id).await? {
            return Ok(false);
        }

        // 3. Bail if no encryption_key
        let encryption_key = match encryption_key {
            Some(k) => k,
            None => {
                tracing::warn!("seal_session_final: no encryption key available");
                return Ok(false);
            }
        };

        // 4. Load signing service
        let svc = match crate::org_signing::load_current(pool, org_id, encryption_key).await {
            Ok(Some(s)) => s,
            Ok(None) => return Ok(false),
            Err(e) => {
                tracing::warn!("seal_session_final: failed to load signing key: {e}");
                return Ok(false);
            }
        };

        // 5. Compute session record hash
        let record_hash = compute_session_record_hash(pool, session_db_id).await?;

        // 6. Sign
        let signature = svc.sign(&record_hash);

        // 7. Insert session seal
        SealingRepo::insert_session_seal(
            pool,
            session_db_id,
            &record_hash,
            &signature,
            seal_type,
            None,
        )
        .await?;

        Ok(true)
    }

    /// Sweep sessions that have been inactive for too long and seal them with a timeout seal.
    pub async fn sweep_stale_sessions(
        pool: &PgPool,
        encryption_key: Option<&str>,
        inactive_minutes: i64,
    ) {
        let stale = match SealingRepo::find_stale_sessions(pool, inactive_minutes).await {
            Ok(s) => s,
            Err(e) => {
                tracing::error!("sweep_stale_sessions: failed to find stale sessions: {e}");
                return;
            }
        };

        if !stale.is_empty() {
            tracing::info!("sweep_stale_sessions: found {} stale sessions", stale.len());
        }

        for (session_db_id, org_id) in stale {
            match Self::seal_session_final(pool, session_db_id, org_id, encryption_key, "timeout")
                .await
            {
                Ok(_) => {
                    tracing::debug!("sweep_stale_sessions: sealed session {session_db_id}");
                }
                Err(e) => {
                    tracing::warn!(
                        "sweep_stale_sessions: failed to seal session {session_db_id}: {e}"
                    );
                }
            }
        }
    }
}

/// Seal a session snapshot linked to a commit seal.
async fn seal_session_snapshot(
    pool: &PgPool,
    session_db_id: Uuid,
    commit_seal_id: Uuid,
    org_id: Uuid,
    encryption_key: &str,
) -> Result<(), AppError> {
    // 1. Load signing service
    let svc = match crate::org_signing::load_current(pool, org_id, encryption_key).await {
        Ok(Some(s)) => s,
        Ok(None) => return Ok(()),
        Err(e) => {
            tracing::warn!("seal_session_snapshot: failed to load signing key: {e}");
            return Ok(());
        }
    };

    // 2. Compute session record hash
    let record_hash = compute_session_record_hash(pool, session_db_id).await?;

    // 3. Sign
    let signature = svc.sign(&record_hash);

    // 4. Insert session seal
    SealingRepo::insert_session_seal(
        pool,
        session_db_id,
        &record_hash,
        &signature,
        "commit_snapshot",
        Some(commit_seal_id),
    )
    .await?;

    Ok(())
}

/// Compute SHA-256 hex digest of the canonical JSON for a commit record.
fn compute_commit_record_hash(
    commit_sha: &str,
    branch: Option<&str>,
    author: Option<&str>,
    message: Option<&str>,
    diff_data: &Option<serde_json::Value>,
    committed_at: Option<&chrono::DateTime<chrono::Utc>>,
    attribution: &Option<serde_json::Value>,
) -> String {
    let canonical = serde_json::json!({
        "commit_sha": commit_sha,
        "branch": branch,
        "author": author,
        "message": message,
        "diff_data": diff_data,
        "committed_at": committed_at.map(|t| t.to_rfc3339()),
        "attribution": attribution,
    });

    let mut hasher = Sha256::new();
    hasher.update(canonical.to_string().as_bytes());
    hex::encode(hasher.finalize())
}

/// Compute SHA-256 hex digest of the canonical JSON for a session record.
async fn compute_session_record_hash(
    pool: &PgPool,
    session_db_id: Uuid,
) -> Result<String, AppError> {
    // 1. Load session data
    let (session_id, repo_id, input_tokens, output_tokens, cache_read, cache_write, cost) =
        SealingRepo::get_session_for_sealing(pool, session_db_id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Session {session_db_id} not found")))?;

    // 2. Load events
    let events = SealingRepo::get_session_events_for_sealing(pool, session_db_id).await?;

    // 3. Load file changes
    let file_changes =
        SealingRepo::get_session_file_changes_for_sealing(pool, session_db_id).await?;

    // 4. Compute event content hashes
    let event_hashes: Vec<serde_json::Value> = events
        .iter()
        .map(
            |(event_index, event_type, tool_name, tool_input, tool_response)| {
                let input_str = tool_input
                    .as_ref()
                    .map(|v| v.to_string())
                    .unwrap_or_default();
                let response_str = tool_response
                    .as_ref()
                    .map(|v| v.to_string())
                    .unwrap_or_default();

                let mut hasher = Sha256::new();
                hasher.update(input_str.as_bytes());
                hasher.update(response_str.as_bytes());
                let content_hash = hex::encode(hasher.finalize());

                serde_json::json!({
                    "event_index": event_index,
                    "event_type": event_type,
                    "tool_name": tool_name,
                    "content_hash": content_hash,
                })
            },
        )
        .collect();

    // 5. Build file change entries
    let file_change_entries: Vec<serde_json::Value> = file_changes
        .iter()
        .map(|(file_path, change_type, content_hash)| {
            serde_json::json!({
                "file_path": file_path,
                "change_type": change_type,
                "content_hash": content_hash,
            })
        })
        .collect();

    // 6. Build canonical JSON and hash
    let canonical = serde_json::json!({
        "session_id": session_id,
        "repo_id": repo_id.to_string(),
        "events": event_hashes,
        "file_changes": file_change_entries,
        "tokens": {
            "input": input_tokens,
            "output": output_tokens,
            "cache_read": cache_read,
            "cache_write": cache_write,
        },
        "estimated_cost_usd": cost,
    });

    let mut hasher = Sha256::new();
    hasher.update(canonical.to_string().as_bytes());
    Ok(hex::encode(hasher.finalize()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn commit_record_hash_deterministic() {
        let h1 = compute_commit_record_hash(
            "abc123",
            Some("main"),
            Some("alice"),
            Some("fix bug"),
            &None,
            None,
            &None,
        );
        let h2 = compute_commit_record_hash(
            "abc123",
            Some("main"),
            Some("alice"),
            Some("fix bug"),
            &None,
            None,
            &None,
        );
        assert_eq!(h1, h2);
        assert_eq!(h1.len(), 64);
    }

    #[test]
    fn commit_record_hash_changes_with_data() {
        let h1 = compute_commit_record_hash(
            "abc123",
            Some("main"),
            Some("alice"),
            Some("fix bug"),
            &None,
            None,
            &None,
        );
        let h2 = compute_commit_record_hash(
            "abc123",
            Some("main"),
            Some("alice"),
            Some("different msg"),
            &None,
            None,
            &None,
        );
        assert_ne!(h1, h2);
    }
}
