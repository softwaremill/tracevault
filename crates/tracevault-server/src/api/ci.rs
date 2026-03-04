use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::api::policies::{evaluate_condition, EvalOutcome};
use crate::extractors::AuthUser;
use crate::AppState;

#[derive(Debug, Deserialize)]
pub struct CiVerifyRequest {
    pub commits: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct CiVerifyResponse {
    pub status: String,
    pub total_commits: usize,
    pub registered_commits: usize,
    pub sealed_commits: usize,
    pub policy_passed_commits: usize,
    pub results: Vec<CommitVerifyResult>,
}

#[derive(Debug, Serialize)]
pub struct CommitVerifyResult {
    pub commit_sha: String,
    pub status: String,
    pub registered: bool,
    pub sealed: bool,
    pub signature_valid: bool,
    pub chain_valid: bool,
    pub policy_results: Vec<PolicyResult>,
}

#[derive(Debug, Serialize)]
pub struct PolicyResult {
    pub rule_name: String,
    pub result: String,
    pub action: String,
    pub severity: String,
    pub details: String,
}

/// POST /api/v1/repos/{repo_id}/ci/verify
/// Verify a list of commits against the TraceVault database.
pub async fn verify_commits(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(repo_id): Path<Uuid>,
    Json(req): Json<CiVerifyRequest>,
) -> Result<Json<CiVerifyResponse>, (StatusCode, String)> {
    // Verify repo belongs to org
    let repo_exists: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM repos WHERE id = $1 AND org_id = $2)",
    )
    .bind(repo_id)
    .bind(auth.org_id)
    .fetch_one(&state.pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    if !repo_exists {
        return Err((StatusCode::NOT_FOUND, "Repo not found".into()));
    }

    // Fetch all enabled policies for this repo
    let policies = sqlx::query_as::<_, (String, serde_json::Value, String, String)>(
        "SELECT name, condition, action, severity
         FROM policies
         WHERE org_id = $1 AND (repo_id = $2 OR repo_id IS NULL) AND enabled = true
         ORDER BY created_at",
    )
    .bind(auth.org_id)
    .bind(repo_id)
    .fetch_all(&state.pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let mut results = Vec::new();
    let mut registered_count = 0usize;
    let mut sealed_count = 0usize;
    let mut policy_passed_count = 0usize;

    for commit_sha in &req.commits {
        // Look up commit by (repo_id, commit_sha)
        let commit = sqlx::query_as::<_, (
            Uuid,
            Option<String>,
            Option<String>,
            Option<String>,
            Option<String>,
            Option<chrono::DateTime<chrono::Utc>>,
        )>(
            "SELECT id, record_hash, chain_hash, prev_chain_hash, signature, sealed_at
             FROM commits
             WHERE repo_id = $1 AND commit_sha = $2",
        )
        .bind(repo_id)
        .bind(commit_sha)
        .fetch_optional(&state.pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

        let Some((commit_id, record_hash, chain_hash, prev_chain_hash, signature, sealed_at)) =
            commit
        else {
            results.push(CommitVerifyResult {
                commit_sha: commit_sha.clone(),
                status: "unregistered".into(),
                registered: false,
                sealed: false,
                signature_valid: false,
                chain_valid: false,
                policy_results: vec![],
            });
            continue;
        };

        registered_count += 1;

        if sealed_at.is_none() {
            results.push(CommitVerifyResult {
                commit_sha: commit_sha.clone(),
                status: "unsealed".into(),
                registered: true,
                sealed: false,
                signature_valid: false,
                chain_valid: false,
                policy_results: vec![],
            });
            continue;
        }

        sealed_count += 1;

        // Verify signature and chain
        let signature_valid = match (&record_hash, &signature) {
            (Some(rh), Some(sig)) => state.extensions.signing.verify(rh, sig),
            _ => false,
        };

        let chain_valid = match (&record_hash, &chain_hash) {
            (Some(rh), Some(ch)) => {
                let expected = state.extensions.signing.chain_hash(prev_chain_hash.as_deref(), rh);
                expected == *ch
            }
            _ => false,
        };

        // Re-evaluate policies from stored session data
        let sessions = sqlx::query_as::<_, (Option<serde_json::Value>,)>(
            "SELECT session_data FROM sessions WHERE commit_id = $1",
        )
        .bind(commit_id)
        .fetch_all(&state.pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

        // Aggregate tool_calls and files_modified across all sessions
        let mut all_tool_calls: std::collections::HashMap<String, i64> =
            std::collections::HashMap::new();
        let mut all_files: Vec<String> = Vec::new();

        for (session_data,) in &sessions {
            if let Some(data) = session_data {
                if let Some(tc) = data.get("tool_calls").and_then(|v| v.as_object()) {
                    for (k, v) in tc {
                        let count = v.as_i64().unwrap_or(0);
                        *all_tool_calls.entry(k.clone()).or_insert(0) += count;
                    }
                }
                if let Some(files) = data.get("files_modified").and_then(|v| v.as_array()) {
                    for f in files {
                        if let Some(s) = f.as_str() {
                            all_files.push(s.to_string());
                        }
                    }
                }
            }
        }

        // Evaluate each policy
        let mut policy_results = Vec::new();
        let mut all_policies_passed = true;

        for (name, condition, action, severity) in &policies {
            let EvalOutcome { passed, details } =
                evaluate_condition(condition, &all_tool_calls, &all_files);
            let result_str = if passed { "pass" } else { "fail" };

            if !passed {
                all_policies_passed = false;
            }

            policy_results.push(PolicyResult {
                rule_name: name.clone(),
                result: result_str.into(),
                action: action.clone(),
                severity: severity.clone(),
                details,
            });
        }

        let commit_status =
            if signature_valid && chain_valid && all_policies_passed {
                policy_passed_count += 1;
                "pass"
            } else {
                "fail"
            };

        results.push(CommitVerifyResult {
            commit_sha: commit_sha.clone(),
            status: commit_status.into(),
            registered: true,
            sealed: true,
            signature_valid,
            chain_valid,
            policy_results,
        });
    }

    let overall_status = if results.iter().all(|r| r.status == "pass") {
        "pass"
    } else {
        "fail"
    };

    crate::audit::log(
        &state.pool,
        crate::audit::user_action(
            auth.org_id,
            auth.user_id,
            "ci.verify",
            "repo",
            Some(repo_id),
            Some(serde_json::json!({
                "status": overall_status,
                "total_commits": req.commits.len(),
                "registered": registered_count,
                "sealed": sealed_count,
                "policy_passed": policy_passed_count,
            })),
        ),
    )
    .await;

    Ok(Json(CiVerifyResponse {
        status: overall_status.into(),
        total_commits: req.commits.len(),
        registered_commits: registered_count,
        sealed_commits: sealed_count,
        policy_passed_commits: policy_passed_count,
        results,
    }))
}
