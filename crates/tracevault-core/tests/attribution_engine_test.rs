use tracevault_core::attribution_engine::*;

#[test]
fn all_lines_ai_when_file_created_by_agent() {
    let result = compute_file_attribution(
        "src/new.rs",
        None,
        "fn main() {\n    println!(\"hi\");\n}\n",
        true,
    );

    assert_eq!(result.path, "src/new.rs");
    assert_eq!(result.lines_added, 3);
    assert_eq!(result.ai_lines.len(), 1);
    assert_eq!(result.ai_lines[0].start, 1);
    assert_eq!(result.ai_lines[0].end, 3);
    assert!(result.human_lines.is_empty());
}

#[test]
fn all_lines_human_when_file_not_touched_by_agent() {
    let result = compute_file_attribution("src/human.rs", None, "fn main() {}\n", false);

    assert_eq!(result.lines_added, 1);
    assert!(result.ai_lines.is_empty());
    assert_eq!(result.human_lines.len(), 1);
}

#[test]
fn summary_computes_percentages() {
    let files = vec![
        compute_file_attribution("a.rs", None, "line1\nline2\n", true),
        compute_file_attribution("b.rs", None, "line1\nline2\nline3\n", false),
    ];
    let summary = compute_attribution_summary(&files);
    assert_eq!(summary.total_lines_added, 5);
    // 2 AI lines out of 5 total = 40%
    assert!((summary.ai_percentage - 40.0).abs() < 0.1);
}
