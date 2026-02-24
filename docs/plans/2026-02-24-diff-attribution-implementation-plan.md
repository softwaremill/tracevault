# Diff & Attribution View Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Show a GitHub PR-style unified diff on the trace detail page with color-coded AI vs human attribution, sourced from git-ai notes.

**Architecture:** CLI reads git-ai notes and git diff at push time, sends both as JSONB to the server. Frontend cross-references diff lines with attribution line ranges to color-code AI (violet) vs human (green) additions.

**Tech Stack:** Rust (tracevault-core for types/parsing, tracevault-cli for push integration, tracevault-server for storage), PostgreSQL (JSONB), SvelteKit 5 (frontend rendering).

---

### Task 1: Add diff data types to tracevault-core

**Files:**
- Create: `crates/tracevault-core/src/diff.rs`
- Modify: `crates/tracevault-core/src/lib.rs`

**Step 1: Create diff.rs with data types**

```rust
// crates/tracevault-core/src/diff.rs
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
```

**Step 2: Register module in lib.rs**

Add `pub mod diff;` to `crates/tracevault-core/src/lib.rs`.

**Step 3: Verify it compiles**

Run: `cargo check -p tracevault-core`
Expected: success

**Step 4: Commit**

```bash
git add crates/tracevault-core/src/diff.rs crates/tracevault-core/src/lib.rs
git commit -m "feat: add diff data types to tracevault-core"
```

---

### Task 2: Add unified diff parser to tracevault-core

**Files:**
- Modify: `crates/tracevault-core/src/diff.rs`
- Create: `crates/tracevault-core/tests/diff_test.rs`

**Step 1: Write the failing tests**

```rust
// crates/tracevault-core/tests/diff_test.rs
use tracevault_core::diff::*;

#[test]
fn parse_single_file_diff() {
    let raw = "\
diff --git a/src/main.rs b/src/main.rs
index abc1234..def5678 100644
--- a/src/main.rs
+++ b/src/main.rs
@@ -1,3 +1,4 @@
 fn main() {
+    println!(\"hello\");
     let x = 1;
 }
";
    let files = parse_unified_diff(raw);
    assert_eq!(files.len(), 1);
    assert_eq!(files[0].path, "src/main.rs");
    assert_eq!(files[0].hunks.len(), 1);

    let hunk = &files[0].hunks[0];
    assert_eq!(hunk.old_start, 1);
    assert_eq!(hunk.old_count, 3);
    assert_eq!(hunk.new_start, 1);
    assert_eq!(hunk.new_count, 4);
    assert_eq!(hunk.lines.len(), 4);

    assert_eq!(hunk.lines[0].kind, DiffLineKind::Context);
    assert_eq!(hunk.lines[0].content, "fn main() {");
    assert_eq!(hunk.lines[0].old_line_number, Some(1));
    assert_eq!(hunk.lines[0].new_line_number, Some(1));

    assert_eq!(hunk.lines[1].kind, DiffLineKind::Add);
    assert_eq!(hunk.lines[1].content, "    println!(\"hello\");");
    assert_eq!(hunk.lines[1].old_line_number, None);
    assert_eq!(hunk.lines[1].new_line_number, Some(2));
}

#[test]
fn parse_new_file_diff() {
    let raw = "\
diff --git a/new.txt b/new.txt
new file mode 100644
index 0000000..abc1234
--- /dev/null
+++ b/new.txt
@@ -0,0 +1,2 @@
+line one
+line two
";
    let files = parse_unified_diff(raw);
    assert_eq!(files.len(), 1);
    assert_eq!(files[0].path, "new.txt");
    assert_eq!(files[0].hunks[0].lines.len(), 2);
    assert!(files[0].hunks[0].lines.iter().all(|l| l.kind == DiffLineKind::Add));
}

#[test]
fn parse_deleted_file_diff() {
    let raw = "\
diff --git a/old.txt b/old.txt
deleted file mode 100644
index abc1234..0000000
--- a/old.txt
+++ /dev/null
@@ -1,2 +0,0 @@
-line one
-line two
";
    let files = parse_unified_diff(raw);
    assert_eq!(files.len(), 1);
    assert_eq!(files[0].path, "old.txt");
    assert!(files[0].hunks[0].lines.iter().all(|l| l.kind == DiffLineKind::Delete));
}

#[test]
fn parse_rename_diff() {
    let raw = "\
diff --git a/old_name.rs b/new_name.rs
similarity index 90%
rename from old_name.rs
rename to new_name.rs
index abc1234..def5678 100644
--- a/old_name.rs
+++ b/new_name.rs
@@ -1,3 +1,3 @@
 fn main() {
-    old();
+    new();
 }
";
    let files = parse_unified_diff(raw);
    assert_eq!(files.len(), 1);
    assert_eq!(files[0].path, "new_name.rs");
    assert_eq!(files[0].old_path, Some("old_name.rs".to_string()));
}

#[test]
fn parse_multiple_files() {
    let raw = "\
diff --git a/a.rs b/a.rs
index 1111111..2222222 100644
--- a/a.rs
+++ b/a.rs
@@ -1,1 +1,2 @@
 line1
+line2
diff --git a/b.rs b/b.rs
index 3333333..4444444 100644
--- a/b.rs
+++ b/b.rs
@@ -1,1 +1,2 @@
 line1
+line2
";
    let files = parse_unified_diff(raw);
    assert_eq!(files.len(), 2);
    assert_eq!(files[0].path, "a.rs");
    assert_eq!(files[1].path, "b.rs");
}

#[test]
fn parse_multiple_hunks() {
    let raw = "\
diff --git a/file.rs b/file.rs
index 1111111..2222222 100644
--- a/file.rs
+++ b/file.rs
@@ -1,3 +1,4 @@
 line1
+inserted
 line2
 line3
@@ -10,3 +11,4 @@
 line10
+another
 line11
 line12
";
    let files = parse_unified_diff(raw);
    assert_eq!(files.len(), 1);
    assert_eq!(files[0].hunks.len(), 2);
    assert_eq!(files[0].hunks[0].old_start, 1);
    assert_eq!(files[0].hunks[1].old_start, 10);
}

#[test]
fn parse_empty_input() {
    let files = parse_unified_diff("");
    assert!(files.is_empty());
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test -p tracevault-core --test diff_test`
Expected: FAIL — `parse_unified_diff` doesn't exist yet.

**Step 3: Implement `parse_unified_diff` in diff.rs**

Add the following function to the end of `crates/tracevault-core/src/diff.rs`:

```rust
/// Parse `git diff` output into structured file diffs.
pub fn parse_unified_diff(raw: &str) -> Vec<FileDiff> {
    let mut files: Vec<FileDiff> = Vec::new();
    let lines: Vec<&str> = raw.lines().collect();
    let mut i = 0;

    while i < lines.len() {
        // Look for "diff --git a/... b/..."
        if !lines[i].starts_with("diff --git ") {
            i += 1;
            continue;
        }

        let diff_line = lines[i];
        // Extract paths from "diff --git a/foo b/bar"
        let (a_path, b_path) = parse_diff_header(diff_line);
        i += 1;

        // Scan header lines for rename info
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

        // If we saw rename from but not in diff header
        if old_path.is_none() && a_path != b_path {
            old_path = Some(a_path);
        }

        // Parse hunks
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
    // "diff --git a/foo b/bar"
    let rest = line.strip_prefix("diff --git ").unwrap_or("");
    // Find the split point: " b/"
    // Handle paths that may contain spaces by finding the last " b/" pattern
    if let Some(pos) = rest.rfind(" b/") {
        let a = rest[..pos].strip_prefix("a/").unwrap_or(&rest[..pos]);
        let b = rest[pos + 1..].strip_prefix("b/").unwrap_or(&rest[pos + 1..]);
        (a.to_string(), b.to_string())
    } else {
        // Fallback: split in half
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
    // Parse "@@ -old_start,old_count +new_start,new_count @@"
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
            // Skip this metadata line
            *i += 1;
            continue;
        } else {
            // Treat as context (line with no prefix, e.g. empty context line)
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
    // "@@ -1,3 +1,4 @@" or "@@ -1,3 +1,4 @@ fn main"
    let inner = header.strip_prefix("@@ ")?;
    let end = inner.find(" @@")?;
    let range_str = &inner[..end]; // "-1,3 +1,4"

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
        (
            start.parse().unwrap_or(0),
            count.parse().unwrap_or(0),
        )
    } else {
        (s.parse().unwrap_or(0), 1)
    }
}
```

**Step 4: Run tests to verify they pass**

Run: `cargo test -p tracevault-core --test diff_test`
Expected: all 7 tests PASS

**Step 5: Commit**

```bash
git add crates/tracevault-core/src/diff.rs crates/tracevault-core/tests/diff_test.rs
git commit -m "feat: add unified diff parser with tests"
```

---

### Task 3: Add git-ai note parser to tracevault-core

**Files:**
- Create: `crates/tracevault-core/src/gitai.rs`
- Modify: `crates/tracevault-core/src/lib.rs`
- Create: `crates/tracevault-core/tests/gitai_test.rs`

git-ai authorship log format (from `refs/notes/ai`):

```
src/file.rs
   9fc943b -5 +22 +31-59 +101-103
   cda9aa2 +2
---
{"schema_version":"authorship/3.0.0","base_commit_sha":"abc123","prompts":{"9fc943b":{"agent_id":{"tool":"claude-code","model":"claude-opus-4-6"},"human_author":"dev@example.com","total_additions":42,"total_deletions":5}}}
```

Attestation lines: `+N` means a single added line, `+N-M` means added line range, `-N` means a single deleted line.

**Step 1: Write the failing tests**

```rust
// crates/tracevault-core/tests/gitai_test.rs
use tracevault_core::gitai::*;

#[test]
fn parse_simple_note() {
    let note = "\
src/main.rs
   abc1234 +1-10 +20
---
{\"schema_version\":\"authorship/3.0.0\",\"base_commit_sha\":\"000\",\"prompts\":{\"abc1234\":{\"agent_id\":{\"tool\":\"claude-code\",\"model\":\"claude-opus-4-6\"},\"human_author\":\"dev@test.com\",\"total_additions\":11,\"total_deletions\":0}}}
";
    let result = parse_gitai_note(note);
    assert!(result.is_some());
    let log = result.unwrap();

    assert_eq!(log.files.len(), 1);
    assert_eq!(log.files[0].path, "src/main.rs");
    assert_eq!(log.files[0].ai_line_ranges.len(), 2);
    assert_eq!(log.files[0].ai_line_ranges[0], (1, 10));
    assert_eq!(log.files[0].ai_line_ranges[1], (20, 20));

    assert!(log.metadata.is_some());
}

#[test]
fn parse_multi_file_note() {
    let note = "\
src/a.rs
   sess1 +1-5
src/b.rs
   sess1 +10-20
   sess2 +30
---
{}
";
    let result = parse_gitai_note(note);
    assert!(result.is_some());
    let log = result.unwrap();
    assert_eq!(log.files.len(), 2);
    assert_eq!(log.files[0].path, "src/a.rs");
    assert_eq!(log.files[0].ai_line_ranges, vec![(1, 5)]);
    assert_eq!(log.files[1].path, "src/b.rs");
    assert_eq!(log.files[1].ai_line_ranges, vec![(10, 20), (30, 30)]);
}

#[test]
fn parse_note_with_deletions() {
    let note = "\
src/main.rs
   abc1234 -5 +10-15
---
{}
";
    let result = parse_gitai_note(note);
    let log = result.unwrap();
    // Deletions are tracked but we only use additions for AI line attribution
    assert_eq!(log.files[0].ai_line_ranges, vec![(10, 15)]);
    assert_eq!(log.files[0].deleted_lines, vec![5]);
}

#[test]
fn parse_empty_returns_none() {
    assert!(parse_gitai_note("").is_none());
}

#[test]
fn parse_no_separator_returns_none() {
    assert!(parse_gitai_note("just some text").is_none());
}

#[test]
fn convert_to_attribution() {
    let note = "\
src/main.rs
   abc +1-10 +20-25
---
{}
";
    let log = parse_gitai_note(note).unwrap();
    let attribution = gitai_to_attribution(&log);

    assert_eq!(attribution.files.len(), 1);
    assert_eq!(attribution.files[0].path, "src/main.rs");
    assert_eq!(attribution.files[0].ai_lines.len(), 2);
    assert_eq!(attribution.files[0].ai_lines[0].start, 1);
    assert_eq!(attribution.files[0].ai_lines[0].end, 10);
    assert_eq!(attribution.files[0].ai_lines[1].start, 20);
    assert_eq!(attribution.files[0].ai_lines[1].end, 25);
    assert!(attribution.summary.ai_percentage > 0.0);
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test -p tracevault-core --test gitai_test`
Expected: FAIL — module doesn't exist.

**Step 3: Implement gitai.rs**

```rust
// crates/tracevault-core/src/gitai.rs
use crate::attribution::{Attribution, AttributionSummary, FileAttribution, LineRange};

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
            // Parse "   session_id +1-10 +20 -5"
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
            // Non-indented line = file path
            // Flush previous file
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
                human_lines: vec![], // Will be computed by frontend from diff context
                mixed_lines: vec![],
            }
        })
        .collect();

    let summary = crate::attribution_engine::compute_attribution_summary(&files);

    Attribution { files, summary }
}
```

**Step 4: Register module in lib.rs**

Add `pub mod gitai;` to `crates/tracevault-core/src/lib.rs`.

**Step 5: Run tests**

Run: `cargo test -p tracevault-core --test gitai_test`
Expected: all 6 tests PASS

**Step 6: Commit**

```bash
git add crates/tracevault-core/src/gitai.rs crates/tracevault-core/src/lib.rs crates/tracevault-core/tests/gitai_test.rs
git commit -m "feat: add git-ai note parser with attribution conversion"
```

---

### Task 4: Database migration — add diff_data column

**Files:**
- Create: `crates/tracevault-server/migrations/004_diff_data.sql`

**Step 1: Create migration file**

```sql
ALTER TABLE traces ADD COLUMN diff_data JSONB;
```

**Step 2: Verify the server starts (migration runs on startup)**

Run: `cargo check -p tracevault-server`
Expected: compiles. (Actual migration runs at startup with `docker compose up`.)

**Step 3: Commit**

```bash
git add crates/tracevault-server/migrations/004_diff_data.sql
git commit -m "feat: add diff_data column to traces table"
```

---

### Task 5: Update server API to accept and return diff_data

**Files:**
- Modify: `crates/tracevault-server/src/api/traces.rs`

**Step 1: Add `diff_data` to `CreateTraceRequest`**

In `crates/tracevault-server/src/api/traces.rs`, add to the `CreateTraceRequest` struct after the `transcript` field:

```rust
    pub diff_data: Option<serde_json::Value>,
```

**Step 2: Update the `create_trace` INSERT query**

Change the INSERT to include `diff_data`:

```sql
INSERT INTO traces (repo_id, commit_sha, branch, author, model, tool, tool_version, ai_percentage, total_tokens, input_tokens, output_tokens, estimated_cost_usd, api_calls, session_data, attribution, transcript, diff_data)
VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17)
RETURNING id, commit_sha, branch, author, model, tool, ai_percentage, total_tokens, estimated_cost_usd, created_at
```

Add `.bind(&req.diff_data)` after `.bind(&req.transcript)`.

**Step 3: Update `get_trace` to return diff_data**

In the `get_trace` function, update the SELECT query to include `t.diff_data` and add it to the tuple type and JSON response:

Change the query to:
```sql
SELECT t.id, t.repo_id, t.commit_sha, t.branch, t.author, t.model, t.tool, t.ai_percentage, t.total_tokens, t.estimated_cost_usd, t.session_data, t.attribution, t.transcript, t.diff_data, t.created_at
FROM traces t JOIN repos r ON t.repo_id = r.id WHERE t.id = $1 AND r.org_id = $2
```

Update the tuple type to add `Option<serde_json::Value>` for `diff_data` (position 13), and shift `created_at` to position 14.

Update the JSON response to include `"diff_data": row.13` and `"created_at": row.14`.

**Step 4: Verify it compiles**

Run: `cargo check -p tracevault-server`
Expected: success

**Step 5: Commit**

```bash
git add crates/tracevault-server/src/api/traces.rs
git commit -m "feat: accept and return diff_data in trace API"
```

---

### Task 6: Update CLI push to compute and send diff + attribution

**Files:**
- Modify: `crates/tracevault-cli/src/commands/push.rs`
- Modify: `crates/tracevault-cli/src/api_client.rs`

**Step 1: Add `diff_data` to `PushTraceRequest`**

In `crates/tracevault-cli/src/api_client.rs`, add to `PushTraceRequest` after `transcript`:

```rust
    pub diff_data: Option<serde_json::Value>,
```

**Step 2: Add git diff and git-ai note reading to push.rs**

Add these functions to `crates/tracevault-cli/src/commands/push.rs`:

```rust
use tracevault_core::diff::parse_unified_diff;
use tracevault_core::gitai::{parse_gitai_note, gitai_to_attribution};

fn read_git_diff(project_root: &Path, commit_sha: &str) -> Option<serde_json::Value> {
    let output = Command::new("git")
        .args(["diff", &format!("{commit_sha}~1..{commit_sha}")])
        .current_dir(project_root)
        .output()
        .ok()?;

    if !output.status.success() {
        // May fail for initial commit, try diffing against empty tree
        let output = Command::new("git")
            .args(["diff", "--root", commit_sha])
            .current_dir(project_root)
            .output()
            .ok()?;
        if !output.status.success() {
            return None;
        }
        let raw = String::from_utf8_lossy(&output.stdout);
        let files = parse_unified_diff(&raw);
        return serde_json::to_value(&files).ok();
    }

    let raw = String::from_utf8_lossy(&output.stdout);
    if raw.is_empty() {
        return None;
    }
    let files = parse_unified_diff(&raw);
    serde_json::to_value(&files).ok()
}

fn read_gitai_attribution(project_root: &Path, commit_sha: &str) -> Option<serde_json::Value> {
    let output = Command::new("git")
        .args(["notes", "show", "--ref=refs/notes/ai", commit_sha])
        .current_dir(project_root)
        .output()
        .ok()?;

    if !output.status.success() {
        return None; // git-ai not installed or no note for this commit
    }

    let note = String::from_utf8_lossy(&output.stdout);
    let log = parse_gitai_note(&note)?;
    let attribution = gitai_to_attribution(&log);
    serde_json::to_value(&attribution).ok()
}
```

**Step 3: Wire into `push_traces`**

In the `push_traces` function, before creating `PushTraceRequest`, add:

```rust
let diff_data = read_git_diff(project_root, &git.commit_sha);
let attribution = read_gitai_attribution(project_root, &git.commit_sha);
```

Update the `PushTraceRequest` construction to use these:

```rust
attribution,      // was: None
// ... existing fields ...
diff_data,        // new field at end
```

**Step 4: Verify it compiles**

Run: `cargo check -p tracevault-cli`
Expected: success

**Step 5: Commit**

```bash
git add crates/tracevault-cli/src/commands/push.rs crates/tracevault-cli/src/api_client.rs
git commit -m "feat: compute git diff and read git-ai attribution on push"
```

---

### Task 7: Frontend — add diff view to trace detail page

**Files:**
- Modify: `web/src/routes/traces/[id]/+page.svelte`

This is the largest task. The diff view section goes below the transcript table and above the session data toggle.

**Step 1: Add TypeScript types for diff data**

Add these interfaces to the `<script>` tag, after the existing `TranscriptStats` interface:

```typescript
interface DiffLine {
    kind: 'add' | 'delete' | 'context';
    content: string;
    new_line_number: number | null;
    old_line_number: number | null;
}

interface DiffHunk {
    old_start: number;
    old_count: number;
    new_start: number;
    new_count: number;
    lines: DiffLine[];
}

interface FileDiff {
    path: string;
    old_path: string | null;
    hunks: DiffHunk[];
}

interface AttrLineRange {
    start: number;
    end: number;
}

interface FileAttribution {
    path: string;
    lines_added: number;
    lines_deleted: number;
    ai_lines: AttrLineRange[];
    human_lines: AttrLineRange[];
    mixed_lines: AttrLineRange[];
}

interface AttributionData {
    files: FileAttribution[];
    summary: {
        total_lines_added: number;
        total_lines_deleted: number;
        ai_percentage: number;
        human_percentage: number;
    };
}
```

**Step 2: Update the `TraceDetail` interface**

Add `diff_data` to the existing `TraceDetail` interface:

```typescript
diff_data: FileDiff[] | null;
```

**Step 3: Add reactive state for diff view**

After the existing reactive state declarations:

```typescript
let expandedFiles: Set<string> = $state(new Set());

const diffFiles = $derived.by(() => {
    if (!trace?.diff_data) return [] as FileDiff[];
    return trace.diff_data as FileDiff[];
});

const attrData = $derived.by(() => {
    if (!trace?.attribution) return null;
    return trace.attribution as unknown as AttributionData;
});

const diffSummary = $derived.by(() => {
    let totalAdded = 0;
    let totalDeleted = 0;
    for (const file of diffFiles) {
        for (const hunk of file.hunks) {
            for (const line of hunk.lines) {
                if (line.kind === 'add') totalAdded++;
                else if (line.kind === 'delete') totalDeleted++;
            }
        }
    }
    return { totalAdded, totalDeleted, fileCount: diffFiles.length };
});

function toggleFile(path: string) {
    const next = new Set(expandedFiles);
    if (next.has(path)) next.delete(path);
    else next.add(path);
    expandedFiles = next;
}

function isAiLine(filePath: string, lineNum: number): boolean {
    if (!attrData) return false;
    const fileAttr = attrData.files.find((f) => f.path === filePath);
    if (!fileAttr) return false;
    return fileAttr.ai_lines.some((r) => lineNum >= r.start && lineNum <= r.end);
}

function fileAiLineCount(filePath: string): number {
    if (!attrData) return 0;
    const fileAttr = attrData.files.find((f) => f.path === filePath);
    if (!fileAttr) return 0;
    return fileAttr.ai_lines.reduce((sum, r) => sum + (r.end - r.start + 1), 0);
}

function fileAddedCount(file: FileDiff): number {
    return file.hunks.reduce(
        (sum, h) => sum + h.lines.filter((l) => l.kind === 'add').length,
        0
    );
}

function fileDeletedCount(file: FileDiff): number {
    return file.hunks.reduce(
        (sum, h) => sum + h.lines.filter((l) => l.kind === 'delete').length,
        0
    );
}
```

**Step 4: Add diff view template**

Insert this template block between the transcript `{/if}` closing tag and the session data `{#if trace.session_data}` block:

```svelte
{#if diffFiles.length > 0}
    <Card.Root>
        <Card.Header class="pb-2">
            <Card.Title class="flex items-center gap-3">
                <span>Changes</span>
                <Badge variant="secondary">{diffSummary.fileCount} file{diffSummary.fileCount !== 1 ? 's' : ''}</Badge>
                <span class="text-sm font-normal">
                    <span class="text-green-600">+{diffSummary.totalAdded}</span>
                    <span class="text-red-600 ml-1">-{diffSummary.totalDeleted}</span>
                </span>
                {#if attrData}
                    <Badge variant="outline" class="text-xs">
                        {attrData.summary.ai_percentage.toFixed(0)}% AI
                    </Badge>
                {/if}
            </Card.Title>
        </Card.Header>
        <Card.Content class="space-y-2 p-0">
            {#if !attrData}
                <div class="mx-4 mt-2 mb-2 rounded border border-blue-200 bg-blue-50 dark:border-blue-800 dark:bg-blue-950 p-3 text-sm text-blue-700 dark:text-blue-300">
                    Attribution data not available. Install <a href="https://usegitai.com" class="underline font-medium" target="_blank" rel="noopener">git-ai</a> to track which lines were written by AI agents vs humans.
                </div>
            {/if}
            {#each diffFiles as file}
                <div class="border-t first:border-t-0">
                    <button
                        class="flex w-full items-center gap-2 px-4 py-2 text-left text-sm hover:bg-muted/50"
                        onclick={() => toggleFile(file.path)}
                    >
                        <span class="text-muted-foreground">{expandedFiles.has(file.path) ? '▼' : '▶'}</span>
                        <span class="font-mono font-medium">{file.path}</span>
                        {#if file.old_path}
                            <span class="text-muted-foreground text-xs">(renamed from {file.old_path})</span>
                        {/if}
                        <span class="ml-auto text-xs">
                            <span class="text-green-600">+{fileAddedCount(file)}</span>
                            <span class="text-red-600 ml-1">-{fileDeletedCount(file)}</span>
                        </span>
                        {#if attrData && fileAiLineCount(file.path) > 0}
                            <Badge variant="outline" class="text-xs">AI: {fileAiLineCount(file.path)} lines</Badge>
                        {:else if attrData}
                            <span class="text-xs text-muted-foreground">Human only</span>
                        {/if}
                    </button>
                    {#if expandedFiles.has(file.path)}
                        <div class="overflow-x-auto">
                            {#each file.hunks as hunk}
                                <div class="bg-blue-50 dark:bg-blue-950/30 px-4 py-1 text-xs font-mono text-muted-foreground border-y">
                                    @@ -{hunk.old_start},{hunk.old_count} +{hunk.new_start},{hunk.new_count} @@
                                </div>
                                {#each hunk.lines as line}
                                    {@const isAi = line.kind === 'add' && line.new_line_number != null && isAiLine(file.path, line.new_line_number)}
                                    <div
                                        class="flex font-mono text-xs leading-5 {
                                            line.kind === 'delete'
                                                ? 'bg-red-500/10'
                                                : line.kind === 'add'
                                                    ? isAi
                                                        ? 'bg-violet-500/10'
                                                        : 'bg-green-500/10'
                                                    : ''
                                        }"
                                    >
                                        <span class="w-12 shrink-0 select-none text-right pr-2 text-muted-foreground/50 border-r">
                                            {line.old_line_number ?? ''}
                                        </span>
                                        <span class="w-12 shrink-0 select-none text-right pr-2 text-muted-foreground/50 border-r">
                                            {line.new_line_number ?? ''}
                                        </span>
                                        <span class="w-5 shrink-0 select-none text-center {
                                            line.kind === 'add' ? 'text-green-600' : line.kind === 'delete' ? 'text-red-600' : 'text-muted-foreground/30'
                                        }">
                                            {line.kind === 'add' ? '+' : line.kind === 'delete' ? '-' : ' '}
                                        </span>
                                        <span class="whitespace-pre pl-1">{line.content}</span>
                                    </div>
                                {/each}
                            {/each}
                        </div>
                    {/if}
                </div>
            {/each}
        </Card.Content>
    </Card.Root>
{/if}
```

**Step 5: Verify it compiles**

Run: `cd web && npx svelte-check --threshold error`
Expected: 0 errors

**Step 6: Commit**

```bash
git add 'web/src/routes/traces/[id]/+page.svelte'
git commit -m "feat: add diff view with AI/human line attribution on trace detail"
```

---

### Task 8: Verify end-to-end

**Step 1: Run all Rust tests**

Run: `cargo test --workspace`
Expected: all tests pass

**Step 2: Run svelte-check**

Run: `cd web && npx svelte-check --threshold error`
Expected: 0 errors

**Step 3: Manual verification checklist**

- Start the stack: `docker compose up -d`
- Push a trace from a repo with git-ai installed → verify attribution + diff_data stored
- Push a trace from a repo without git-ai → verify attribution is null, diff_data is present
- View trace detail page:
  - With attribution: diff shows violet (AI) and green (human) added lines
  - Without attribution: info banner shown, diff shows standard green/red
  - Files are collapsible
  - Line numbers render correctly
  - Hunk headers render correctly
  - Multiple files in a diff render correctly
- View trace without diff_data (older trace): Changes section hidden entirely

**Step 4: Commit any final fixes, then create a single summary commit if needed**
