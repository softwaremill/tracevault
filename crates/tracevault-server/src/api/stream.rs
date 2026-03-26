use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use tracevault_core::streaming::{
    extract_file_change, is_file_modifying_tool, StreamEventRequest, StreamEventResponse,
    StreamEventType,
};
use uuid::Uuid;

use crate::{extractors::OrgAuth, AppState};

/// POST /api/v1/orgs/{slug}/repos/{repo_id}/stream
pub async fn handle_stream(
    State(state): State<AppState>,
    auth: OrgAuth,
    Path((_slug, repo_id)): Path<(String, Uuid)>,
    Json(req): Json<StreamEventRequest>,
) -> Result<Json<StreamEventResponse>, (StatusCode, String)> {
    // 1. Validate protocol version
    if req.protocol_version != 1 {
        return Err((
            StatusCode::UNPROCESSABLE_ENTITY,
            format!(
                "Unsupported protocol_version: {}, expected 1",
                req.protocol_version
            ),
        ));
    }

    // 2. Upsert session
    let session_db_id: Uuid = sqlx::query_scalar(
        "INSERT INTO sessions_v2 (org_id, repo_id, user_id, session_id, model, cwd, started_at)
         VALUES ($1, $2, $3, $4, $5, $6, $7)
         ON CONFLICT (repo_id, session_id) DO UPDATE SET
             updated_at = now(),
             model = COALESCE(EXCLUDED.model, sessions_v2.model),
             cwd = COALESCE(EXCLUDED.cwd, sessions_v2.cwd),
             status = CASE WHEN sessions_v2.status = 'completed' THEN 'active' ELSE sessions_v2.status END
         RETURNING id",
    )
    .bind(auth.org_id)
    .bind(repo_id)
    .bind(auth.user_id)
    .bind(&req.session_id)
    .bind(&req.model)
    .bind(&req.cwd)
    .bind(req.timestamp)
    .fetch_one(&state.pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let mut event_db_id: Option<Uuid> = None;

    // Process piggybacked transcript lines on any event type
    if let Some(ref lines) = req.transcript_lines {
        if !lines.is_empty() {
            let offset = req.transcript_offset.unwrap_or(0);
            let mut batch_input: i64 = 0;
            let mut batch_output: i64 = 0;
            let mut batch_cache_read: i64 = 0;
            let mut batch_cache_write: i64 = 0;
            let mut detected_model: Option<String> = None;

            for (i, line) in lines.iter().enumerate() {
                let chunk_index = offset as i32 + i as i32;
                sqlx::query(
                    "INSERT INTO transcript_chunks (session_id, chunk_index, data)
                     VALUES ($1, $2, $3)
                     ON CONFLICT (session_id, chunk_index) DO NOTHING",
                )
                .bind(session_db_id)
                .bind(chunk_index)
                .bind(line)
                .execute(&state.pool)
                .await
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

                // Extract token usage from assistant messages
                // Claude Code format: { type: "assistant", message: { model, usage: { input_tokens, output_tokens, cache_read_input_tokens, cache_creation_input_tokens } } }
                if let Some(msg) = line.get("message") {
                    if let Some(usage) = msg.get("usage") {
                        batch_input += usage
                            .get("input_tokens")
                            .and_then(|v| v.as_i64())
                            .unwrap_or(0);
                        batch_output += usage
                            .get("output_tokens")
                            .and_then(|v| v.as_i64())
                            .unwrap_or(0);
                        batch_cache_read += usage
                            .get("cache_read_input_tokens")
                            .and_then(|v| v.as_i64())
                            .unwrap_or(0);
                        // cache_creation_input_tokens = cache write
                        batch_cache_write += usage
                            .get("cache_creation_input_tokens")
                            .and_then(|v| v.as_i64())
                            .unwrap_or(0);
                    }
                    if detected_model.is_none() {
                        detected_model =
                            msg.get("model").and_then(|v| v.as_str()).map(String::from);
                    }
                }
            }

            // Update session token counts and cost if we found usage data
            let has_tokens = batch_input > 0
                || batch_output > 0
                || batch_cache_read > 0
                || batch_cache_write > 0;
            if has_tokens {
                let model_name = detected_model.as_deref().unwrap_or("unknown");
                // input_tokens from the API includes cache_read and cache_write,
                // subtract to get fresh (non-cached) input only
                let fresh_input = (batch_input - batch_cache_read - batch_cache_write).max(0);
                let batch_cost = crate::pricing::estimate_cost(
                    model_name,
                    fresh_input,
                    batch_output,
                    batch_cache_read,
                    batch_cache_write,
                );

                sqlx::query(
                    "UPDATE sessions_v2 SET
                        input_tokens = input_tokens + $2,
                        output_tokens = output_tokens + $3,
                        cache_read_tokens = cache_read_tokens + $4,
                        cache_write_tokens = cache_write_tokens + $5,
                        total_tokens = total_tokens + $2 + $3 + $4 + $5,
                        estimated_cost_usd = estimated_cost_usd + $6,
                        model = COALESCE($7, model),
                        updated_at = now()
                     WHERE id = $1",
                )
                .bind(session_db_id)
                .bind(fresh_input)
                .bind(batch_output)
                .bind(batch_cache_read)
                .bind(batch_cache_write)
                .bind(batch_cost)
                .bind(&detected_model)
                .execute(&state.pool)
                .await
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
            }
        }
    }

    match req.event_type {
        // 3. ToolUse events
        StreamEventType::ToolUse => {
            let event_index = req.event_index.ok_or((
                StatusCode::UNPROCESSABLE_ENTITY,
                "event_index required for ToolUse events".to_string(),
            ))?;

            let tool_name = req.tool_name.as_deref().unwrap_or("");
            let store_response = is_file_modifying_tool(tool_name);

            let inserted_id: Option<Uuid> = sqlx::query_scalar(
                "INSERT INTO events (session_id, event_index, event_type, tool_name, tool_input, tool_response, timestamp)
                 VALUES ($1, $2, 'tool_use', $3, $4, $5, $6)
                 ON CONFLICT (session_id, event_index) DO NOTHING
                 RETURNING id",
            )
            .bind(session_db_id)
            .bind(event_index)
            .bind(&req.tool_name)
            .bind(&req.tool_input)
            .bind(if store_response { req.tool_response.as_ref() } else { None })
            .bind(req.timestamp)
            .fetch_optional(&state.pool)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

            if let Some(eid) = inserted_id {
                event_db_id = Some(eid);

                // Extract file changes for file-modifying tools
                if is_file_modifying_tool(tool_name) {
                    if let Some(ref tool_input) = req.tool_input {
                        if let Some(change) = extract_file_change(tool_name, tool_input) {
                            sqlx::query(
                                "INSERT INTO file_changes (session_id, event_id, file_path, change_type, diff_text, content_hash, timestamp)
                                 VALUES ($1, $2, $3, $4, $5, $6, $7)
                                 ON CONFLICT (event_id, file_path) DO NOTHING",
                            )
                            .bind(session_db_id)
                            .bind(eid)
                            .bind(&change.file_path)
                            .bind(&change.change_type)
                            .bind(&change.diff_text)
                            .bind(&change.content_hash)
                            .bind(req.timestamp)
                            .execute(&state.pool)
                            .await
                            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
                        }
                    }
                }

                // Increment total_tool_calls
                sqlx::query(
                    "UPDATE sessions_v2 SET total_tool_calls = total_tool_calls + 1 WHERE id = $1",
                )
                .bind(session_db_id)
                .execute(&state.pool)
                .await
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
            }
        }

        // 4. Transcript events — lines already processed above via piggybacking
        StreamEventType::Transcript => {}

        // 5. SessionEnd
        StreamEventType::SessionEnd => {
            if let Some(ref stats) = req.final_stats {
                sqlx::query(
                    "UPDATE sessions_v2 SET
                         status = 'completed',
                         ended_at = $2,
                         duration_ms = COALESCE($3, duration_ms),
                         total_tokens = COALESCE($4, total_tokens),
                         input_tokens = COALESCE($5, input_tokens),
                         output_tokens = COALESCE($6, output_tokens),
                         cache_read_tokens = COALESCE($7, cache_read_tokens),
                         cache_write_tokens = COALESCE($8, cache_write_tokens),
                         user_messages = COALESCE($9, user_messages),
                         assistant_messages = COALESCE($10, assistant_messages),
                         total_tool_calls = COALESCE($11, total_tool_calls)
                     WHERE id = $1",
                )
                .bind(session_db_id)
                .bind(req.timestamp)
                .bind(stats.duration_ms)
                .bind(stats.total_tokens)
                .bind(stats.input_tokens)
                .bind(stats.output_tokens)
                .bind(stats.cache_read_tokens)
                .bind(stats.cache_write_tokens)
                .bind(stats.user_messages)
                .bind(stats.assistant_messages)
                .bind(stats.total_tool_calls)
                .execute(&state.pool)
                .await
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
            } else {
                sqlx::query(
                    "UPDATE sessions_v2 SET status = 'completed', ended_at = $2 WHERE id = $1",
                )
                .bind(session_db_id)
                .bind(req.timestamp)
                .execute(&state.pool)
                .await
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
            }
        }

        // 6. SessionStart — session already upserted above
        StreamEventType::SessionStart => {}
    }

    // 7. Return response
    Ok(Json(StreamEventResponse {
        session_db_id,
        event_db_id,
        status: "ok".to_string(),
    }))
}
