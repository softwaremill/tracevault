use std::collections::HashMap;

use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::error::AppError;
use crate::repo::commits::{CommitRepo, FileChangeMatch, InsertAttribution};

pub struct AttributionService;

impl AttributionService {
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
    ) -> Result<i64, AppError> {
        let hunks = parse_diff_hunks(diff_data);
        if hunks.is_empty() {
            return Ok(0);
        }

        // Idempotent: clear previous attributions
        CommitRepo::clear_attributions(pool, commit_id).await?;

        let mut count: i64 = 0;

        for hunk in &hunks {
            let matches = CommitRepo::find_matching_file_changes(
                pool,
                repo_id,
                committed_at,
                &hunk.file_path,
            )
            .await?;

            for fc in &matches {
                let confidence = compute_confidence(hunk, fc);
                if confidence < 0.1 {
                    continue;
                }

                CommitRepo::insert_attribution(
                    pool,
                    &InsertAttribution {
                        commit_id,
                        session_id: fc.session_id,
                        event_id: fc.event_id,
                        file_path: hunk.file_path.clone(),
                        line_start: if hunk.line_start > 0 {
                            Some(hunk.line_start)
                        } else {
                            None
                        },
                        line_end: if hunk.line_end > 0 {
                            Some(hunk.line_end)
                        } else {
                            None
                        },
                        confidence,
                    },
                )
                .await?;

                count += 1;
            }
        }

        Ok(count)
    }

    /// Compute attribution summary from commit_attributions and store in commits.attribution JSONB.
    ///
    /// After `attribute_commit` populates `commit_attributions`, this function:
    /// 1. Counts total added lines per file from diff_data
    /// 2. Queries commit_attributions for matched line ranges
    /// 3. Deduplicates overlapping ranges (union per file)
    /// 4. Computes AI vs human percentages
    /// 5. Stores the summary in commits.attribution
    pub async fn compute_summary(
        pool: &PgPool,
        commit_id: Uuid,
        diff_data: &serde_json::Value,
    ) -> Result<(), AppError> {
        let (added_per_file, total_deleted) = count_diff_lines(diff_data);
        let total_added: i64 = added_per_file.values().sum();

        if total_added == 0 {
            // No added lines -- store 0% AI
            let summary = serde_json::json!({
                "summary": {
                    "total_lines_added": 0,
                    "total_lines_deleted": total_deleted,
                    "ai_percentage": 0.0,
                    "human_percentage": 0.0,
                }
            });
            CommitRepo::update_attribution_summary(pool, commit_id, &summary).await?;
            return Ok(());
        }

        // Query all commit_attributions line ranges for this commit
        let rows = CommitRepo::get_attributions(pool, commit_id).await?;

        // Group line ranges by file and merge overlapping ranges
        let mut ranges_by_file: HashMap<String, Vec<(i32, i32)>> = HashMap::new();
        for row in &rows {
            if let (Some(start), Some(end)) = (row.line_start, row.line_end) {
                if start > 0 && end > 0 {
                    ranges_by_file
                        .entry(row.file_path.clone())
                        .or_default()
                        .push((start, end));
                }
            } else {
                // File-level match with no line ranges -- count all added lines in this file as AI
                if let Some(&file_added) = added_per_file.get(row.file_path.as_str()) {
                    if file_added > 0 {
                        ranges_by_file
                            .entry(row.file_path.clone())
                            .or_default()
                            .push((1, file_added as i32));
                    }
                }
            }
        }

        // Count AI lines by merging overlapping ranges per file
        let mut total_ai_lines: i64 = 0;
        for (file_path, ranges) in &mut ranges_by_file {
            let merged = merge_ranges(ranges);
            let ai_lines: i64 = merged.iter().map(|(s, e)| (*e - *s + 1) as i64).sum();
            // Cap AI lines at the file's total added lines
            let file_added = added_per_file.get(file_path.as_str()).copied().unwrap_or(0);
            total_ai_lines += ai_lines.min(file_added);
        }

        let ai_percentage = (total_ai_lines as f64 / total_added as f64) * 100.0;
        let human_percentage = 100.0 - ai_percentage;

        let summary = serde_json::json!({
            "summary": {
                "total_lines_added": total_added,
                "total_lines_deleted": total_deleted,
                "ai_percentage": ai_percentage,
                "human_percentage": human_percentage,
            }
        });

        CommitRepo::update_attribution_summary(pool, commit_id, &summary).await?;

        Ok(())
    }
}

// --- Pure helper functions ---

struct DiffHunk {
    file_path: String,
    line_start: i32,
    line_end: i32,
    added_lines: Vec<String>,
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
            // File entry with no hunks -- create a file-level hunk with no lines
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

/// Count total added and deleted lines per file from diff_data JSON.
fn count_diff_lines(diff_data: &serde_json::Value) -> (HashMap<String, i64>, i64) {
    let mut added_per_file: HashMap<String, i64> = HashMap::new();
    let mut total_deleted: i64 = 0;

    let Some(files) = diff_data.get("files").and_then(|f| f.as_array()) else {
        return (added_per_file, 0);
    };

    for file in files {
        let Some(file_path) = file.get("path").and_then(|p| p.as_str()) else {
            continue;
        };
        let Some(hunks) = file.get("hunks").and_then(|h| h.as_array()) else {
            continue;
        };

        let mut file_added: i64 = 0;
        for hunk in hunks {
            // Count added lines from added_lines array
            if let Some(added) = hunk.get("added_lines").and_then(|l| l.as_array()) {
                file_added += added.len() as i64;
            } else if let Some(lines) = hunk.get("lines").and_then(|l| l.as_array()) {
                // Fallback: count "+"-prefixed lines
                file_added += lines
                    .iter()
                    .filter(|l| l.as_str().is_some_and(|s| s.starts_with('+')))
                    .count() as i64;
            }

            // Count deleted lines
            if let Some(lines) = hunk.get("lines").and_then(|l| l.as_array()) {
                total_deleted += lines
                    .iter()
                    .filter(|l| l.as_str().is_some_and(|s| s.starts_with('-')))
                    .count() as i64;
            } else if let Some(del) = hunk.get("deleted_lines").and_then(|l| l.as_array()) {
                total_deleted += del.len() as i64;
            }
        }

        *added_per_file.entry(file_path.to_string()).or_default() += file_added;
    }

    (added_per_file, total_deleted)
}

/// Merge overlapping or adjacent ranges into a minimal set of non-overlapping ranges.
fn merge_ranges(ranges: &mut [(i32, i32)]) -> Vec<(i32, i32)> {
    if ranges.is_empty() {
        return vec![];
    }
    ranges.sort_unstable();

    let mut merged: Vec<(i32, i32)> = vec![ranges[0]];
    for &(start, end) in &ranges[1..] {
        let last = merged.last_mut().unwrap();
        if start <= last.1 + 1 {
            last.1 = last.1.max(end);
        } else {
            merged.push((start, end));
        }
    }
    merged
}
