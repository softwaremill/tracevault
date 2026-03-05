use tree_sitter::{Language, Node, Parser};

#[derive(Debug, Clone, serde::Serialize)]
pub struct CodeScope {
    pub kind: String,
    pub name: String,
    pub start_line: usize, // 1-indexed
    pub end_line: usize,   // 1-indexed, inclusive
}

pub fn get_language(file_ext: &str) -> Option<Language> {
    match file_ext {
        "rs" => Some(tree_sitter_rust::LANGUAGE.into()),
        "ts" | "tsx" => Some(tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into()),
        "js" | "jsx" => Some(tree_sitter_javascript::LANGUAGE.into()),
        "py" => Some(tree_sitter_python::LANGUAGE.into()),
        "go" => Some(tree_sitter_go::LANGUAGE.into()),
        "java" => Some(tree_sitter_java::LANGUAGE.into()),
        "scala" | "sc" => Some(tree_sitter_scala::LANGUAGE.into()),
        _ => None,
    }
}

/// Find the innermost named scope (function/class/module) containing the given line.
/// `line` is 1-indexed.
pub fn find_enclosing_scope(source: &str, file_ext: &str, line: usize) -> Option<CodeScope> {
    let language = get_language(file_ext)?;
    let mut parser = Parser::new();
    parser.set_language(&language).ok()?;
    let tree = parser.parse(source, None)?;

    let target_line = line.checked_sub(1)?; // tree-sitter uses 0-indexed rows
    let scope_node_types = scope_types_for_ext(file_ext);

    let mut best: Option<(Node, &str)> = None;
    walk_tree(tree.root_node(), target_line, &scope_node_types, &mut best);

    let (node, kind) = best?;
    let name = extract_name(node, source);

    Some(CodeScope {
        kind: kind.to_string(),
        name: name.unwrap_or_else(|| "<anonymous>".to_string()),
        start_line: node.start_position().row + 1,
        end_line: node.end_position().row + 1,
    })
}

fn walk_tree<'a>(
    node: Node<'a>,
    target_line: usize,
    scope_types: &[(&str, &'a str)],
    best: &mut Option<(Node<'a>, &'a str)>,
) {
    let start = node.start_position().row;
    let end = node.end_position().row;

    if target_line < start || target_line > end {
        return;
    }

    for (node_type, kind) in scope_types {
        if node.kind() == *node_type
            && best.as_ref().is_none_or(|(b, _)| {
                let b_range = b.end_position().row - b.start_position().row;
                let n_range = end - start;
                n_range < b_range
            })
        {
            *best = Some((node, kind));
        }
    }

    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            walk_tree(child, target_line, scope_types, best);
        }
    }
}

fn scope_types_for_ext(ext: &str) -> Vec<(&'static str, &'static str)> {
    match ext {
        "rs" => vec![
            ("function_item", "function"),
            ("impl_item", "impl"),
            ("struct_item", "struct"),
            ("enum_item", "enum"),
            ("trait_item", "trait"),
            ("mod_item", "module"),
        ],
        "ts" | "tsx" | "js" | "jsx" => vec![
            ("function_declaration", "function"),
            ("method_definition", "method"),
            ("arrow_function", "function"),
            ("class_declaration", "class"),
            ("interface_declaration", "interface"),
        ],
        "py" => vec![
            ("function_definition", "function"),
            ("class_definition", "class"),
        ],
        "go" => vec![
            ("function_declaration", "function"),
            ("method_declaration", "method"),
            ("type_declaration", "type"),
        ],
        "java" => vec![
            ("method_declaration", "method"),
            ("constructor_declaration", "constructor"),
            ("class_declaration", "class"),
            ("interface_declaration", "interface"),
            ("enum_declaration", "enum"),
        ],
        "scala" | "sc" => vec![
            ("function_definition", "function"),
            ("class_definition", "class"),
            ("object_definition", "object"),
            ("trait_definition", "trait"),
        ],
        _ => vec![],
    }
}

fn extract_name(node: Node, source: &str) -> Option<String> {
    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            let field_name = node.field_name_for_child(i as u32);
            if matches!(field_name, Some("name")) {
                let text = &source[child.byte_range()];
                return Some(text.to_string());
            }
            if child.kind() == "identifier" || child.kind() == "type_identifier" {
                let text = &source[child.byte_range()];
                return Some(text.to_string());
            }
        }
    }
    None
}

/// Fallback: return a range of lines around the target line.
pub fn fallback_scope(source: &str, line: usize, context_lines: usize) -> CodeScope {
    let total_lines = source.lines().count();
    let start = line.saturating_sub(context_lines).max(1);
    let end = (line + context_lines).min(total_lines);
    CodeScope {
        kind: "region".to_string(),
        name: format!("lines {start}-{end}"),
        start_line: start,
        end_line: end,
    }
}
