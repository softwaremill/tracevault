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
    assert!(files[0].hunks[0]
        .lines
        .iter()
        .all(|l| l.kind == DiffLineKind::Add));
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
    assert!(files[0].hunks[0]
        .lines
        .iter()
        .all(|l| l.kind == DiffLineKind::Delete));
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
