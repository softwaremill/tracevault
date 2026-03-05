use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::extractors::AuthUser;
use crate::AppState;

#[derive(Debug, Serialize)]
pub struct PolicyResponse {
    pub id: Uuid,
    pub org_id: Uuid,
    pub repo_id: Option<Uuid>,
    pub name: String,
    pub description: String,
    pub condition: serde_json::Value,
    pub action: String,
    pub severity: String,
    pub enabled: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreatePolicyRequest {
    pub name: String,
    pub description: Option<String>,
    pub condition: serde_json::Value,
    pub action: String,
    pub severity: Option<String>,
    pub enabled: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct UpdatePolicyRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub condition: Option<serde_json::Value>,
    pub action: Option<String>,
    pub severity: Option<String>,
    pub enabled: Option<bool>,
}

/// GET /api/v1/repos/{repo_id}/policies
/// Returns all policies for a repo (repo-specific + org-wide)
pub async fn list_repo_policies(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(repo_id): Path<Uuid>,
) -> Result<Json<Vec<PolicyResponse>>, (StatusCode, String)> {
    // Verify repo belongs to org
    let repo_exists: bool =
        sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM repos WHERE id = $1 AND org_id = $2)")
            .bind(repo_id)
            .bind(auth.org_id)
            .fetch_one(&state.pool)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    if !repo_exists {
        return Err((StatusCode::NOT_FOUND, "Repo not found".into()));
    }

    let rows = sqlx::query_as::<_, (Uuid, Uuid, Option<Uuid>, String, String, serde_json::Value, String, String, bool, chrono::DateTime<chrono::Utc>, chrono::DateTime<chrono::Utc>)>(
        "SELECT id, org_id, repo_id, name, description, condition, action, severity, enabled, created_at, updated_at
         FROM policies
         WHERE org_id = $1 AND (repo_id = $2 OR repo_id IS NULL)
         ORDER BY created_at",
    )
    .bind(auth.org_id)
    .bind(repo_id)
    .fetch_all(&state.pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let policies = rows
        .into_iter()
        .map(|r| PolicyResponse {
            id: r.0,
            org_id: r.1,
            repo_id: r.2,
            name: r.3,
            description: r.4,
            condition: r.5,
            action: r.6,
            severity: r.7,
            enabled: r.8,
            created_at: r.9,
            updated_at: r.10,
        })
        .collect();

    Ok(Json(policies))
}

/// POST /api/v1/repos/{repo_id}/policies
/// Create a policy for this repo
pub async fn create_repo_policy(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(repo_id): Path<Uuid>,
    Json(req): Json<CreatePolicyRequest>,
) -> Result<(StatusCode, Json<PolicyResponse>), (StatusCode, String)> {
    // Verify repo belongs to org
    let repo_exists: bool =
        sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM repos WHERE id = $1 AND org_id = $2)")
            .bind(repo_id)
            .bind(auth.org_id)
            .fetch_one(&state.pool)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    if !repo_exists {
        return Err((StatusCode::NOT_FOUND, "Repo not found".into()));
    }

    let row = sqlx::query_as::<_, (Uuid, chrono::DateTime<chrono::Utc>, chrono::DateTime<chrono::Utc>)>(
        "INSERT INTO policies (org_id, repo_id, name, description, condition, action, severity, enabled)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
         RETURNING id, created_at, updated_at",
    )
    .bind(auth.org_id)
    .bind(repo_id)
    .bind(&req.name)
    .bind(req.description.as_deref().unwrap_or(""))
    .bind(&req.condition)
    .bind(&req.action)
    .bind(req.severity.as_deref().unwrap_or("medium"))
    .bind(req.enabled.unwrap_or(true))
    .fetch_one(&state.pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let policy_id = row.0;

    crate::audit::log(
        &state.pool,
        crate::audit::user_action(
            auth.org_id,
            auth.user_id,
            "policy.create",
            "policy",
            Some(policy_id),
            Some(serde_json::json!({"name": &req.name})),
        ),
    )
    .await;

    Ok((
        StatusCode::CREATED,
        Json(PolicyResponse {
            id: policy_id,
            org_id: auth.org_id,
            repo_id: Some(repo_id),
            name: req.name,
            description: req.description.unwrap_or_default(),
            condition: req.condition,
            action: req.action,
            severity: req.severity.unwrap_or_else(|| "medium".into()),
            enabled: req.enabled.unwrap_or(true),
            created_at: row.1,
            updated_at: row.2,
        }),
    ))
}

/// PUT /api/v1/policies/{id}
/// Update a policy
pub async fn update_policy(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdatePolicyRequest>,
) -> Result<Json<PolicyResponse>, (StatusCode, String)> {
    let row = sqlx::query_as::<_, (Uuid, Option<Uuid>, String, String, serde_json::Value, String, String, bool, chrono::DateTime<chrono::Utc>, chrono::DateTime<chrono::Utc>)>(
        "UPDATE policies SET
            name = COALESCE($3, name),
            description = COALESCE($4, description),
            condition = COALESCE($5, condition),
            action = COALESCE($6, action),
            severity = COALESCE($7, severity),
            enabled = COALESCE($8, enabled),
            updated_at = NOW()
         WHERE id = $1 AND org_id = $2
         RETURNING org_id, repo_id, name, description, condition, action, severity, enabled, created_at, updated_at",
    )
    .bind(id)
    .bind(auth.org_id)
    .bind(&req.name)
    .bind(&req.description)
    .bind(&req.condition)
    .bind(&req.action)
    .bind(&req.severity)
    .bind(&req.enabled)
    .fetch_optional(&state.pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    match row {
        Some(r) => {
            crate::audit::log(
                &state.pool,
                crate::audit::user_action(
                    auth.org_id,
                    auth.user_id,
                    "policy.update",
                    "policy",
                    Some(id),
                    None,
                ),
            )
            .await;

            Ok(Json(PolicyResponse {
                id,
                org_id: r.0,
                repo_id: r.1,
                name: r.2,
                description: r.3,
                condition: r.4,
                action: r.5,
                severity: r.6,
                enabled: r.7,
                created_at: r.8,
                updated_at: r.9,
            }))
        }
        None => Err((StatusCode::NOT_FOUND, "Policy not found".into())),
    }
}

/// DELETE /api/v1/policies/{id}
/// Delete a policy
pub async fn delete_policy(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, (StatusCode, String)> {
    let result = sqlx::query("DELETE FROM policies WHERE id = $1 AND org_id = $2")
        .bind(id)
        .bind(auth.org_id)
        .execute(&state.pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    if result.rows_affected() == 0 {
        return Err((StatusCode::NOT_FOUND, "Policy not found".into()));
    }

    crate::audit::log(
        &state.pool,
        crate::audit::user_action(
            auth.org_id,
            auth.user_id,
            "policy.delete",
            "policy",
            Some(id),
            None,
        ),
    )
    .await;

    Ok(StatusCode::OK)
}

// --- Policy Check (evaluation) ---

#[derive(Debug, Deserialize)]
pub struct CheckRequest {
    pub sessions: Vec<SessionCheckData>,
}

#[derive(Debug, Deserialize)]
pub struct SessionCheckData {
    #[serde(rename = "session_id")]
    pub _session_id: String,
    pub tool_calls: Option<serde_json::Value>, // {"tool_name": count}
    pub files_modified: Option<Vec<String>>,
    #[serde(rename = "total_tool_calls")]
    pub _total_tool_calls: Option<i32>,
}

#[derive(Debug, Serialize)]
pub struct CheckResponse {
    pub passed: bool,
    pub results: Vec<CheckResult>,
    pub blocked: bool,
}

#[derive(Debug, Serialize)]
pub struct CheckResult {
    pub rule_name: String,
    pub result: String, // "pass", "fail", "warn"
    pub action: String,
    pub severity: String,
    pub details: String,
}

/// POST /api/v1/repos/{repo_id}/policies/check
/// Evaluate all applicable policies against provided session data
pub async fn check_policies(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(repo_id): Path<Uuid>,
    Json(req): Json<CheckRequest>,
) -> Result<Json<CheckResponse>, (StatusCode, String)> {
    // Verify repo belongs to org
    let repo_exists: bool =
        sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM repos WHERE id = $1 AND org_id = $2)")
            .bind(repo_id)
            .bind(auth.org_id)
            .fetch_one(&state.pool)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    if !repo_exists {
        return Err((StatusCode::NOT_FOUND, "Repo not found".into()));
    }

    // Fetch all enabled policies for this repo (repo-specific + org-wide)
    let rows = sqlx::query_as::<_, (String, serde_json::Value, String, String)>(
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

    // Aggregate session data: merge tool_calls across all sessions, union files_modified
    let mut all_tool_calls: std::collections::HashMap<String, i64> =
        std::collections::HashMap::new();
    let mut all_files: Vec<String> = Vec::new();

    for session in &req.sessions {
        if let Some(tc) = &session.tool_calls {
            if let Some(obj) = tc.as_object() {
                for (k, v) in obj {
                    let count = v.as_i64().unwrap_or(0);
                    *all_tool_calls.entry(k.clone()).or_insert(0) += count;
                }
            }
        }
        if let Some(files) = &session.files_modified {
            all_files.extend(files.iter().cloned());
        }
    }

    let mut results = Vec::new();
    let mut has_block_failure = false;

    for (name, condition, action, severity) in &rows {
        let check_result = evaluate_condition(condition, &all_tool_calls, &all_files);
        let result_str = if check_result.passed { "pass" } else { "fail" };

        if !check_result.passed && action == "block_push" {
            has_block_failure = true;
        }

        results.push(CheckResult {
            rule_name: name.clone(),
            result: result_str.into(),
            action: action.clone(),
            severity: severity.clone(),
            details: check_result.details,
        });
    }

    let all_passed = results.iter().all(|r| r.result == "pass");

    crate::audit::log(
        &state.pool,
        crate::audit::user_action(
            auth.org_id,
            auth.user_id,
            "policy.check",
            "commit",
            None,
            Some(serde_json::json!({"passed": all_passed, "blocked": has_block_failure})),
        ),
    )
    .await;

    Ok(Json(CheckResponse {
        passed: all_passed,
        results,
        blocked: has_block_failure,
    }))
}

pub(crate) struct EvalOutcome {
    pub passed: bool,
    pub details: String,
}

pub(crate) fn evaluate_condition(
    condition: &serde_json::Value,
    tool_calls: &std::collections::HashMap<String, i64>,
    files_modified: &[String],
) -> EvalOutcome {
    let cond_type = condition.get("type").and_then(|v| v.as_str()).unwrap_or("");

    match cond_type {
        "RequiredToolCall" => {
            let tool_names = condition
                .get("tool_names")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(String::from))
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();

            let missing: Vec<_> = tool_names
                .iter()
                .filter(|name| !tool_calls.keys().any(|k| k.contains(name.as_str())))
                .collect();

            if missing.is_empty() {
                EvalOutcome {
                    passed: true,
                    details: "All required tools were used".into(),
                }
            } else {
                EvalOutcome {
                    passed: false,
                    details: format!(
                        "Missing required tools: {}",
                        missing
                            .iter()
                            .map(|s| s.as_str())
                            .collect::<Vec<_>>()
                            .join(", ")
                    ),
                }
            }
        }
        "ConditionalToolCall" => {
            let tool_name = condition
                .get("tool_name")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let min_count = condition
                .get("min_count")
                .and_then(|v| v.as_u64())
                .unwrap_or(1) as i64;
            let file_patterns = condition
                .get("when_files_match")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(String::from))
                        .collect::<Vec<_>>()
                });

            // Check if file patterns apply
            let patterns_match = match &file_patterns {
                None => true, // No patterns = always applies
                Some(patterns) if patterns.is_empty() => true,
                Some(patterns) => files_modified.iter().any(|file| {
                    patterns
                        .iter()
                        .any(|pattern| glob_match::glob_match(pattern, file))
                }),
            };

            if !patterns_match {
                return EvalOutcome {
                    passed: true,
                    details: "Rule skipped: no modified files match patterns".into(),
                };
            }

            // Find the tool call count (supports substring matching like existing code)
            let actual_count: i64 = tool_calls
                .iter()
                .filter(|(k, _)| k.contains(tool_name))
                .map(|(_, v)| v)
                .sum();

            if actual_count >= min_count {
                EvalOutcome {
                    passed: true,
                    details: format!(
                        "Tool '{}' called {} time(s) (required >= {})",
                        tool_name, actual_count, min_count
                    ),
                }
            } else {
                EvalOutcome {
                    passed: false,
                    details: format!(
                        "Tool '{}' called {} time(s) (required >= {})",
                        tool_name, actual_count, min_count
                    ),
                }
            }
        }
        "AiPercentageThreshold" => {
            // Not evaluable from session data alone -- pass by default
            EvalOutcome {
                passed: true,
                details: "AI percentage not evaluated in check (requires attribution data)".into(),
            }
        }
        "TokenBudget" => {
            // Not evaluable from check request -- pass by default
            EvalOutcome {
                passed: true,
                details: "Token budget not evaluated in check (requires token data)".into(),
            }
        }
        _ => EvalOutcome {
            passed: true,
            details: format!("Unknown condition type '{}', skipped", cond_type),
        },
    }
}
