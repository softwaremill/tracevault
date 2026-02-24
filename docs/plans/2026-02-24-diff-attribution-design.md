# Diff & Attribution View Design

## Goal

Show which lines in a commit were written by AI agents vs humans. Display a GitHub PR-style unified diff on the trace detail page with color-coded attribution. Data sourced from git-ai notes.

## Decisions

- **Data source**: git-ai notes (`refs/notes/ai`) only. No fallback computation. Show info message when git-ai data is absent.
- **Diff content**: CLI computes `git diff` at push time, stores as JSONB on traces table.
- **git-ai coupling**: Parse native format directly (attestation + JSON metadata, schema v3.0.0).
- **Diff style**: Unified diff with AI/human color highlighting.
- **Layout**: Section on existing `/traces/[id]` page, below transcript table.

## Architecture: Diff + Line-Range Overlay

Store diff and attribution separately. Frontend maps AI line ranges onto diff hunks at render time.

1. CLI reads git-ai note + git diff at push time
2. Both sent as JSONB fields on trace
3. Frontend cross-references line numbers to determine AI vs human authorship per added line

## CLI Changes (`tracevault push`)

### git-ai Note Parsing

Run `git notes show --ref=refs/notes/ai HEAD`. If present, parse:

- Split on `---` separator
- **Attestation section**: file paths and line ranges (e.g. `src/file.rs\n   9fc943b +22 +31-59`)
- **Metadata section**: JSON with `schema_version`, `prompts` (agent_id, model, human_author, line counts)

Map into existing `Attribution` struct:
- `FileAttribution.ai_lines` from attestation ranges
- `human_lines` as complement within the diff
- `AttributionSummary` computed from aggregates

### Diff Capture

Run `git diff {parent_sha}..{commit_sha}`. Parse into structured format:

```rust
struct FileDiff {
    path: String,
    old_path: Option<String>,  // for renames
    hunks: Vec<DiffHunk>,
}

struct DiffHunk {
    old_start: u32,
    old_count: u32,
    new_start: u32,
    new_count: u32,
    lines: Vec<DiffLine>,
}

struct DiffLine {
    kind: DiffLineKind,  // Add, Delete, Context
    content: String,
    new_line_number: Option<u32>,
    old_line_number: Option<u32>,
}
```

### Push Request

`PushTraceRequest` gains `diff_data: Option<Vec<FileDiff>>`. Existing `attribution` field gets populated from git-ai notes. When git-ai is absent, `attribution` stays `None` but `diff_data` is still populated.

## Database Migration (004)

```sql
ALTER TABLE traces ADD COLUMN diff_data JSONB;
```

Existing `attribution` column already exists as JSONB. No schema change needed for it.

## API

No route changes. `POST /api/v1/traces` accepts `diff_data` in body. `GET /api/v1/traces/{id}` already returns all columns.

## Frontend: Diff View Component

### Location

New "Changes" section on `/traces/[id]` page, below transcript table, above session data toggle.

### Summary Header

Shows file count, total lines added/deleted, and AI percentage bar.

### Per-File Sections

Each file is collapsible. Header shows path, +/- counts, AI line count. Expanded view shows unified diff with hunks.

### Color Scheme

- **Context lines**: no background
- **AI-added lines**: light purple/violet tint (`bg-violet-500/10`)
- **Human-added lines**: light green tint (`bg-green-500/10`)
- **Deleted lines**: light red tint (`bg-red-500/10`)
- Line numbers in muted monospace, gutter separator

### Line-Number Mapping

For each added line in the diff:
1. Get its `new_line_number`
2. Check if it falls within any `attribution.files[path].ai_lines[].{start, end}` range
3. If yes: AI color. If no: human color.

Context and deletion lines are unaffected by attribution.

### Missing Attribution

When `attribution` is null, show notice:

> Attribution data not available. Install git-ai to track which lines were written by AI agents vs humans.

Diff still renders with standard green/red coloring.

### Missing Diff

When `diff_data` is null, section is hidden entirely.
