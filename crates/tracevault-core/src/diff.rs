use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum DiffLineKind {
    Add,
    Delete,
    Context,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DiffLine {
    pub kind: DiffLineKind,
    pub content: String,
    pub new_line_number: Option<u32>,
    pub old_line_number: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DiffHunk {
    pub old_start: u32,
    pub old_count: u32,
    pub new_start: u32,
    pub new_count: u32,
    pub lines: Vec<DiffLine>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FileDiff {
    pub path: String,
    pub old_path: Option<String>,
    pub hunks: Vec<DiffHunk>,
}

/// Parse `git diff` output into structured file diffs.
pub fn parse_unified_diff(raw: &str) -> Vec<FileDiff> {
    let mut files: Vec<FileDiff> = Vec::new();
    let lines: Vec<&str> = raw.lines().collect();
    let mut i = 0;

    while i < lines.len() {
        if !lines[i].starts_with("diff --git ") {
            i += 1;
            continue;
        }

        let diff_line = lines[i];
        let (a_path, b_path) = parse_diff_header(diff_line);
        i += 1;

        let mut old_path: Option<String> = None;
        let mut new_path = b_path.clone();

        while i < lines.len()
            && !lines[i].starts_with("diff --git ")
            && !lines[i].starts_with("@@ ")
        {
            if let Some(stripped) = lines[i].strip_prefix("rename from ") {
                old_path = Some(stripped.to_string());
            }
            if let Some(stripped) = lines[i].strip_prefix("rename to ") {
                new_path = stripped.to_string();
            }
            i += 1;
        }

        if old_path.is_none() && a_path != b_path {
            old_path = Some(a_path);
        }

        let mut hunks: Vec<DiffHunk> = Vec::new();

        while i < lines.len() && !lines[i].starts_with("diff --git ") {
            if lines[i].starts_with("@@ ") {
                if let Some(hunk) = parse_hunk(&lines, &mut i) {
                    hunks.push(hunk);
                } else {
                    i += 1;
                }
            } else {
                i += 1;
            }
        }

        files.push(FileDiff {
            path: new_path,
            old_path,
            hunks,
        });
    }

    files
}

fn parse_diff_header(line: &str) -> (String, String) {
    let rest = line.strip_prefix("diff --git ").unwrap_or("");
    if let Some(pos) = rest.rfind(" b/") {
        let a = rest[..pos].strip_prefix("a/").unwrap_or(&rest[..pos]);
        let b = rest[pos + 1..]
            .strip_prefix("b/")
            .unwrap_or(&rest[pos + 1..]);
        (a.to_string(), b.to_string())
    } else {
        let parts: Vec<&str> = rest.splitn(2, ' ').collect();
        let a = parts
            .first()
            .unwrap_or(&"")
            .strip_prefix("a/")
            .unwrap_or(parts.first().unwrap_or(&""));
        let b = parts
            .get(1)
            .unwrap_or(&"")
            .strip_prefix("b/")
            .unwrap_or(parts.get(1).unwrap_or(&""));
        (a.to_string(), b.to_string())
    }
}

fn parse_hunk(lines: &[&str], i: &mut usize) -> Option<DiffHunk> {
    let header = lines[*i];
    let (old_start, old_count, new_start, new_count) = parse_hunk_header(header)?;
    *i += 1;

    let mut hunk_lines: Vec<DiffLine> = Vec::new();
    let mut old_line = old_start;
    let mut new_line = new_start;

    while *i < lines.len() {
        let line = lines[*i];
        if line.starts_with("diff --git ") || line.starts_with("@@ ") {
            break;
        }

        if let Some(content) = line.strip_prefix('+') {
            hunk_lines.push(DiffLine {
                kind: DiffLineKind::Add,
                content: content.to_string(),
                new_line_number: Some(new_line),
                old_line_number: None,
            });
            new_line += 1;
        } else if let Some(content) = line.strip_prefix('-') {
            hunk_lines.push(DiffLine {
                kind: DiffLineKind::Delete,
                content: content.to_string(),
                new_line_number: None,
                old_line_number: Some(old_line),
            });
            old_line += 1;
        } else if let Some(content) = line.strip_prefix(' ') {
            hunk_lines.push(DiffLine {
                kind: DiffLineKind::Context,
                content: content.to_string(),
                new_line_number: Some(new_line),
                old_line_number: Some(old_line),
            });
            old_line += 1;
            new_line += 1;
        } else if line == "\\ No newline at end of file" {
            *i += 1;
            continue;
        } else {
            hunk_lines.push(DiffLine {
                kind: DiffLineKind::Context,
                content: line.to_string(),
                new_line_number: Some(new_line),
                old_line_number: Some(old_line),
            });
            old_line += 1;
            new_line += 1;
        }

        *i += 1;
    }

    Some(DiffHunk {
        old_start,
        old_count,
        new_start,
        new_count,
        lines: hunk_lines,
    })
}

fn parse_hunk_header(header: &str) -> Option<(u32, u32, u32, u32)> {
    let inner = header.strip_prefix("@@ ")?;
    let end = inner.find(" @@")?;
    let range_str = &inner[..end];

    let parts: Vec<&str> = range_str.split_whitespace().collect();
    if parts.len() != 2 {
        return None;
    }

    let old = parts[0].strip_prefix('-')?;
    let new = parts[1].strip_prefix('+')?;

    let (old_start, old_count) = parse_range(old);
    let (new_start, new_count) = parse_range(new);

    Some((old_start, old_count, new_start, new_count))
}

fn parse_range(s: &str) -> (u32, u32) {
    if let Some((start, count)) = s.split_once(',') {
        (start.parse().unwrap_or(0), count.parse().unwrap_or(0))
    } else {
        (s.parse().unwrap_or(0), 1)
    }
}
