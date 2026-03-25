use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

struct DiffHunk {
    file_path: String,
    line_start: i32,
    line_end: i32,
    added_lines: Vec<String>,
}

#[derive(sqlx::FromRow)]
struct FileChangeMatch {
    session_id: Uuid,
    event_id: Uuid,
    change_type: String,
    line_start: Option<i32>,
    line_end: Option<i32>,
    diff_text: Option<String>,
}

/// Parse diff hunks from diff_data JSON.
///
/// Expects: `{ "files": [{ "path": "...", "hunks": [{ "new_start": N, "new_count": N, "lines": ["+added", " context", ...] }] }] }`
fn parse_diff_hunks(diff_data: &serde_json::Value) -> Vec<DiffHunk> {
    let Some(files) = diff_data.get("files").and_then(|f| f.as_array()) else {
        return vec![];
    };

    let mut hunks = Vec::new();

    for file in files {
        let Some(file_path) = file.get("path").and_then(|p| p.as_str()) else {
            continue;
        };
        let Some(file_hunks) = file.get("hunks").and_then(|h| h.as_array()) else {
            // File entry with no hunks — create a file-level hunk with no lines
            hunks.push(DiffHunk {
                file_path: file_path.to_string(),
                line_start: 0,
                line_end: 0,
                added_lines: vec![],
            });
            continue;
        };

        for hunk in file_hunks {
            let new_start = hunk.get("new_start").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
            let new_count = hunk.get("new_count").and_then(|v| v.as_i64()).unwrap_or(0) as i32;

            let added_lines: Vec<String> = hunk
                .get("added_lines")
                .and_then(|l| l.as_array())
                .map(|lines| {
                    lines
                        .iter()
                        .filter_map(|l| l.as_str())
                        .map(|l| l.to_string())
                        .collect()
                })
                .or_else(|| {
                    // Fallback: "lines" field with "+"-prefixed entries
                    hunk.get("lines").and_then(|l| l.as_array()).map(|lines| {
                        lines
                            .iter()
                            .filter_map(|l| l.as_str())
                            .filter(|l| l.starts_with('+'))
                            .map(|l| l[1..].to_string())
                            .collect()
                    })
                })
                .unwrap_or_default();

            hunks.push(DiffHunk {
                file_path: file_path.to_string(),
                line_start: new_start,
                line_end: new_start + new_count - 1,
                added_lines,
            });
        }
    }

    hunks
}

/// Compute confidence score for a file change match against a diff hunk.
fn compute_confidence(hunk: &DiffHunk, fc: &FileChangeMatch) -> f32 {
    // Check exact content match: all added lines found in diff_text
    if !hunk.added_lines.is_empty() {
        if let Some(ref diff_text) = fc.diff_text {
            let all_match = hunk
                .added_lines
                .iter()
                .all(|line| diff_text.contains(line.as_str()));
            if all_match {
                return 1.0;
            }

            let any_match = hunk
                .added_lines
                .iter()
                .any(|line| diff_text.contains(line.as_str()));
            if any_match {
                return 0.8;
            }
        }
    }

    // Line range overlap
    if let (Some(fc_start), Some(fc_end)) = (fc.line_start, fc.line_end) {
        if hunk.line_start > 0 && hunk.line_end > 0 {
            let overlap_start = hunk.line_start.max(fc_start);
            let overlap_end = hunk.line_end.min(fc_end);

            if overlap_start <= overlap_end {
                let overlap_size = (overlap_end - overlap_start + 1) as f32;
                let hunk_size = (hunk.line_end - hunk.line_start + 1) as f32;
                if hunk_size > 0.0 {
                    let ratio = overlap_size / hunk_size;
                    // Scale from 0.3 to 0.7 proportional to overlap
                    return 0.3 + ratio * 0.4;
                }
            }
        }
    }

    // File-level match only: same file, right time window
    // Slightly higher for modifications vs reads
    if fc.change_type == "create" || fc.change_type == "write" || fc.change_type == "edit" {
        return 0.4;
    }

    0.3
}

/// Run line-level attribution for a commit.
///
/// Parses diff hunks from the commit's diff_data, matches them against
/// file_changes in the same repo within 48h before the commit, computes
/// confidence scores, and inserts into commit_attributions.
///
/// Returns the number of attributions created.
pub async fn attribute_commit(
    pool: &PgPool,
    commit_id: Uuid,
    repo_id: Uuid,
    diff_data: &serde_json::Value,
    committed_at: DateTime<Utc>,
) -> Result<i64, String> {
    let hunks = parse_diff_hunks(diff_data);
    if hunks.is_empty() {
        return Ok(0);
    }

    // Idempotent: clear previous attributions
    sqlx::query("DELETE FROM commit_attributions WHERE commit_id = $1")
        .bind(commit_id)
        .execute(pool)
        .await
        .map_err(|e| e.to_string())?;

    let mut count: i64 = 0;

    for hunk in &hunks {
        let matches = sqlx::query_as::<_, FileChangeMatch>(
            "SELECT fc.session_id, fc.event_id, fc.change_type,
                    fc.line_start, fc.line_end, fc.diff_text
             FROM file_changes fc
             JOIN sessions_v2 s ON fc.session_id = s.id
             WHERE s.repo_id = $1
               AND fc.timestamp >= $2 - INTERVAL '48 hours'
               AND fc.timestamp <= $2
               AND fc.file_path LIKE '%' || $3",
        )
        .bind(repo_id)
        .bind(committed_at)
        .bind(&hunk.file_path)
        .fetch_all(pool)
        .await
        .map_err(|e| e.to_string())?;

        for fc in &matches {
            let confidence = compute_confidence(hunk, fc);
            if confidence < 0.1 {
                continue;
            }

            sqlx::query(
                "INSERT INTO commit_attributions
                    (commit_id, session_id, event_id, file_path, line_start, line_end, confidence)
                 VALUES ($1, $2, $3, $4, $5, $6, $7)",
            )
            .bind(commit_id)
            .bind(fc.session_id)
            .bind(fc.event_id)
            .bind(&hunk.file_path)
            .bind(if hunk.line_start > 0 {
                Some(hunk.line_start)
            } else {
                None
            })
            .bind(if hunk.line_end > 0 {
                Some(hunk.line_end)
            } else {
                None
            })
            .bind(confidence)
            .execute(pool)
            .await
            .map_err(|e| e.to_string())?;

            count += 1;
        }
    }

    Ok(count)
}
