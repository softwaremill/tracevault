use tracevault_core::gitai::*;

#[test]
fn parse_simple_note() {
    let note = "\
src/main.rs
   abc1234 1-10,20
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
   sess1 1-5
src/b.rs
   sess1 10-20
   sess2 30
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
fn parse_real_git_ai_note() {
    // Actual output from git-ai v1.1.5
    let note = "\
frontend/src/routes/+page.svelte
  d731a29901a7381b 94,102,107,120,122-123,128,135,142,155,172,174-175,191
---
{
  \"schema_version\": \"authorship/3.0.0\",
  \"git_ai_version\": \"1.1.5\",
  \"base_commit_sha\": \"d936501a07ed21ad75043ad6b1618e71f9b154ab\",
  \"prompts\": {
    \"d731a29901a7381b\": {
      \"agent_id\": {
        \"tool\": \"claude\",
        \"id\": \"bef5d7a8-26d5-4aea-b339-1cc78922efbf\",
        \"model\": \"claude-opus-4-6\"
      },
      \"human_author\": \"Kris <krzysztof.grajek@googlemail.com>\",
      \"messages\": [],
      \"total_additions\": 14,
      \"total_deletions\": 14,
      \"accepted_lines\": 14,
      \"overriden_lines\": 0
    }
  }
}
";
    let result = parse_gitai_note(note);
    assert!(result.is_some());
    let log = result.unwrap();

    assert_eq!(log.files.len(), 1);
    assert_eq!(log.files[0].path, "frontend/src/routes/+page.svelte");
    // 94, 102, 107, 120, 122-123, 128, 135, 142, 155, 172, 174-175, 191
    // = 10 singles + 2 ranges = 12 entries
    assert_eq!(log.files[0].ai_line_ranges.len(), 12);
    assert_eq!(log.files[0].ai_line_ranges[0], (94, 94));
    assert_eq!(log.files[0].ai_line_ranges[4], (122, 123));
    assert_eq!(log.files[0].ai_line_ranges[10], (174, 175));
    assert_eq!(log.files[0].ai_line_ranges[11], (191, 191));

    assert!(log.metadata.is_some());
    let meta = log.metadata.unwrap();
    assert_eq!(meta["schema_version"], "authorship/3.0.0");
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
   abc 1-10,20-25
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
