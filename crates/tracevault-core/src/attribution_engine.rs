use crate::attribution::{AttributionSummary, FileAttribution, LineRange};

/// Compute attribution for a single file.
///
/// `old_content`: previous content (None if new file)
/// `new_content`: current content
/// `is_ai_authored`: whether the AI agent wrote/edited this file
pub fn compute_file_attribution(
    path: &str,
    old_content: Option<&str>,
    new_content: &str,
    is_ai_authored: bool,
) -> FileAttribution {
    let new_lines: Vec<&str> = new_content.lines().collect();
    let line_count = new_lines.len() as u32;

    if line_count == 0 {
        return FileAttribution {
            path: path.to_string(),
            lines_added: 0,
            lines_deleted: 0,
            ai_lines: vec![],
            human_lines: vec![],
            mixed_lines: vec![],
        };
    }

    match old_content {
        None => {
            // New file: all lines attributed to whoever created it
            let range = vec![LineRange {
                start: 1,
                end: line_count,
            }];
            FileAttribution {
                path: path.to_string(),
                lines_added: line_count,
                lines_deleted: 0,
                ai_lines: if is_ai_authored {
                    range.clone()
                } else {
                    vec![]
                },
                human_lines: if is_ai_authored { vec![] } else { range },
                mixed_lines: vec![],
            }
        }
        Some(old) => {
            let old_lines: Vec<&str> = old.lines().collect();
            let old_count = old_lines.len() as u32;

            let changed = find_changed_lines(&old_lines, &new_lines);
            let added_count = if line_count > old_count {
                line_count - old_count
            } else {
                0
            };
            let deleted_count = if old_count > line_count {
                old_count - line_count
            } else {
                0
            };

            let changed_lines_total: u32 = changed.iter().map(|r| r.end - r.start + 1).sum();

            let (ai_lines, human_lines) = if is_ai_authored {
                (changed, vec![])
            } else {
                (vec![], changed)
            };

            FileAttribution {
                path: path.to_string(),
                lines_added: added_count + changed_lines_total,
                lines_deleted: deleted_count,
                ai_lines,
                human_lines,
                mixed_lines: vec![],
            }
        }
    }
}

/// Compute summary across all file attributions.
pub fn compute_attribution_summary(files: &[FileAttribution]) -> AttributionSummary {
    let total_ai: u32 = files
        .iter()
        .flat_map(|f| &f.ai_lines)
        .map(|r| r.end - r.start + 1)
        .sum();

    let total_human: u32 = files
        .iter()
        .flat_map(|f| &f.human_lines)
        .map(|r| r.end - r.start + 1)
        .sum();

    let total_mixed: u32 = files
        .iter()
        .flat_map(|f| &f.mixed_lines)
        .map(|r| r.end - r.start + 1)
        .sum();

    let total = total_ai + total_human + total_mixed;
    let total_added: u32 = files.iter().map(|f| f.lines_added).sum();
    let total_deleted: u32 = files.iter().map(|f| f.lines_deleted).sum();

    let ai_pct = if total > 0 {
        (total_ai as f32 / total as f32) * 100.0
    } else {
        0.0
    };

    AttributionSummary {
        total_lines_added: total_added,
        total_lines_deleted: total_deleted,
        ai_percentage: ai_pct,
        human_percentage: 100.0 - ai_pct,
    }
}

/// Simple line-based diff: returns ranges of lines in `new` that differ from `old` (1-indexed).
fn find_changed_lines(old: &[&str], new: &[&str]) -> Vec<LineRange> {
    let mut ranges = vec![];
    let mut range_start: Option<u32> = None;

    for (i, new_line) in new.iter().enumerate() {
        let is_changed = old.get(i).map_or(true, |old_line| old_line != new_line);

        if is_changed {
            if range_start.is_none() {
                range_start = Some(i as u32 + 1); // 1-indexed
            }
        } else if let Some(start) = range_start.take() {
            ranges.push(LineRange {
                start,
                end: i as u32, // previous line was the end
            });
        }
    }

    // Close any open range
    if let Some(start) = range_start {
        ranges.push(LineRange {
            start,
            end: new.len() as u32,
        });
    }

    ranges
}
