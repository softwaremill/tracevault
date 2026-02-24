use crate::attribution::{Attribution, FileAttribution, LineRange};
use crate::diff::{DiffLineKind, FileDiff};

/// A parsed git-ai authorship log.
#[derive(Debug, Clone)]
pub struct GitAiAuthorshipLog {
    pub files: Vec<GitAiFileEntry>,
    pub metadata: Option<serde_json::Value>,
}

/// Per-file entry from attestation section.
#[derive(Debug, Clone)]
pub struct GitAiFileEntry {
    pub path: String,
    /// AI-authored line ranges as (start, end) inclusive, 1-indexed.
    pub ai_line_ranges: Vec<(u32, u32)>,
}

/// Parse a git-ai note (from `git notes --ref refs/notes/ai show <sha>`).
///
/// Format (v3.0.0):
/// ```text
/// path/to/file.rs
///    session_id 94,102,107,120,122-123,128
/// another/file.rs
///    session_id 1-5,10
/// ---
/// {"schema_version":"authorship/3.0.0",...}
/// ```
/// Line specs are comma-separated numbers and ranges (no +/- prefix).
pub fn parse_gitai_note(note: &str) -> Option<GitAiAuthorshipLog> {
    let note = note.trim();
    if note.is_empty() {
        return None;
    }

    let separator_pos = note.find("\n---\n").or_else(|| note.find("\n---"))?;
    let attestation = &note[..separator_pos];
    let metadata_str = note[separator_pos..].trim_start_matches('\n').strip_prefix("---")?;
    let metadata_str = metadata_str.trim();

    let metadata: Option<serde_json::Value> = if metadata_str.is_empty() {
        None
    } else {
        serde_json::from_str(metadata_str).ok()
    };

    let files = parse_attestation(attestation);
    if files.is_empty() {
        return None;
    }

    Some(GitAiAuthorshipLog { files, metadata })
}

fn parse_attestation(text: &str) -> Vec<GitAiFileEntry> {
    let mut files: Vec<GitAiFileEntry> = Vec::new();
    let mut current_path: Option<String> = None;
    let mut current_ranges: Vec<(u32, u32)> = Vec::new();

    for line in text.lines() {
        if line.is_empty() {
            continue;
        }

        // Indented lines are session entries (start with whitespace)
        if line.starts_with(' ') || line.starts_with('\t') {
            // Format: "  session_id 94,102,107,120,122-123,128"
            let tokens: Vec<&str> = line.split_whitespace().collect();
            // tokens[0] = session_id, tokens[1] = comma-separated line specs
            if tokens.len() >= 2 {
                for spec in tokens[1].split(',') {
                    if let Some((start, end)) = parse_line_range(spec) {
                        current_ranges.push((start, end));
                    }
                }
            }
        } else {
            // Non-indented line = file path; flush previous file
            if let Some(path) = current_path.take() {
                files.push(GitAiFileEntry {
                    path,
                    ai_line_ranges: std::mem::take(&mut current_ranges),
                });
            }
            current_path = Some(line.to_string());
        }
    }

    // Flush last file
    if let Some(path) = current_path {
        files.push(GitAiFileEntry {
            path,
            ai_line_ranges: current_ranges,
        });
    }

    files
}

/// Parse "1-10" as (1, 10) or "20" as (20, 20).
fn parse_line_range(s: &str) -> Option<(u32, u32)> {
    if let Some((start_str, end_str)) = s.split_once('-') {
        let start = start_str.parse().ok()?;
        let end = end_str.parse().ok()?;
        Some((start, end))
    } else {
        let n = s.parse().ok()?;
        Some((n, n))
    }
}

/// Convert a git-ai authorship log to tracevault's Attribution format.
/// `diff_files` provides the full diff so we can identify human-written lines
/// (lines added in the diff but not listed in the git-ai note).
pub fn gitai_to_attribution(log: &GitAiAuthorshipLog, diff_files: &[FileDiff]) -> Attribution {
    use std::collections::{HashMap, HashSet};

    // Build a lookup: file path -> set of AI-authored new line numbers
    let ai_lines_by_file: HashMap<&str, HashSet<u32>> = log
        .files
        .iter()
        .map(|entry| {
            let mut lines = HashSet::new();
            for &(start, end) in &entry.ai_line_ranges {
                for n in start..=end {
                    lines.insert(n);
                }
            }
            (entry.path.as_str(), lines)
        })
        .collect();

    let mut files: Vec<FileAttribution> = Vec::new();

    for diff_file in diff_files {
        // Collect all added line numbers and deleted count from the diff
        let mut added_lines: Vec<u32> = Vec::new();
        let mut deleted_count: u32 = 0;

        for hunk in &diff_file.hunks {
            for line in &hunk.lines {
                match line.kind {
                    DiffLineKind::Add => {
                        if let Some(n) = line.new_line_number {
                            added_lines.push(n);
                        }
                    }
                    DiffLineKind::Delete => {
                        deleted_count += 1;
                    }
                    DiffLineKind::Context => {}
                }
            }
        }

        let ai_set = ai_lines_by_file.get(diff_file.path.as_str());

        // Partition added lines into AI vs human
        let mut ai_line_nums: Vec<u32> = Vec::new();
        let mut human_line_nums: Vec<u32> = Vec::new();

        for n in &added_lines {
            if ai_set.map_or(false, |s| s.contains(n)) {
                ai_line_nums.push(*n);
            } else {
                human_line_nums.push(*n);
            }
        }

        let ai_lines = collapse_to_ranges(&mut ai_line_nums);
        let human_lines = collapse_to_ranges(&mut human_line_nums);

        files.push(FileAttribution {
            path: diff_file.path.clone(),
            lines_added: added_lines.len() as u32,
            lines_deleted: deleted_count,
            ai_lines,
            human_lines,
            mixed_lines: vec![],
        });
    }

    let summary = crate::attribution_engine::compute_attribution_summary(&files);

    Attribution { files, summary }
}

/// Collapse a list of line numbers into contiguous `LineRange`s.
fn collapse_to_ranges(nums: &mut Vec<u32>) -> Vec<LineRange> {
    if nums.is_empty() {
        return vec![];
    }
    nums.sort_unstable();

    let mut ranges = Vec::new();
    let mut start = nums[0];
    let mut end = nums[0];

    for &n in &nums[1..] {
        if n == end + 1 {
            end = n;
        } else {
            ranges.push(LineRange { start, end });
            start = n;
            end = n;
        }
    }
    ranges.push(LineRange { start, end });
    ranges
}
