use crate::attribution::{Attribution, FileAttribution, LineRange};

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
    /// Added line ranges as (start, end) inclusive, 1-indexed.
    pub ai_line_ranges: Vec<(u32, u32)>,
    /// Single deleted line numbers (in the old file).
    pub deleted_lines: Vec<u32>,
}

/// Parse a git-ai note (from `git notes show --ref=refs/notes/ai <sha>`).
///
/// Format:
/// ```text
/// path/to/file.rs
///    session_id +1-10 +20 -5
/// another/file.rs
///    session_id +1-5
/// ---
/// {"schema_version":"authorship/3.0.0",...}
/// ```
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
    let mut current_deletes: Vec<u32> = Vec::new();

    for line in text.lines() {
        if line.is_empty() {
            continue;
        }

        // Indented lines are session entries (start with whitespace)
        if line.starts_with(' ') || line.starts_with('\t') {
            let tokens: Vec<&str> = line.split_whitespace().collect();
            // Skip first token (session_id)
            for token in tokens.iter().skip(1) {
                if let Some(range_str) = token.strip_prefix('+') {
                    if let Some((start, end)) = parse_line_range(range_str) {
                        current_ranges.push((start, end));
                    }
                } else if let Some(del_str) = token.strip_prefix('-') {
                    if let Ok(line_num) = del_str.parse::<u32>() {
                        current_deletes.push(line_num);
                    }
                }
            }
        } else {
            // Non-indented line = file path; flush previous file
            if let Some(path) = current_path.take() {
                files.push(GitAiFileEntry {
                    path,
                    ai_line_ranges: std::mem::take(&mut current_ranges),
                    deleted_lines: std::mem::take(&mut current_deletes),
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
            deleted_lines: current_deletes,
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
pub fn gitai_to_attribution(log: &GitAiAuthorshipLog) -> Attribution {
    let files: Vec<FileAttribution> = log
        .files
        .iter()
        .map(|entry| {
            let ai_lines: Vec<LineRange> = entry
                .ai_line_ranges
                .iter()
                .map(|&(start, end)| LineRange { start, end })
                .collect();

            let lines_added: u32 = ai_lines.iter().map(|r| r.end - r.start + 1).sum();
            let lines_deleted = entry.deleted_lines.len() as u32;

            FileAttribution {
                path: entry.path.clone(),
                lines_added,
                lines_deleted,
                ai_lines,
                human_lines: vec![],
                mixed_lines: vec![],
            }
        })
        .collect();

    let summary = crate::attribution_engine::compute_attribution_summary(&files);

    Attribution { files, summary }
}
