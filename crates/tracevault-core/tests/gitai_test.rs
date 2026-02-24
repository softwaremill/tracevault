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
