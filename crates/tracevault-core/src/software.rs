use std::collections::HashSet;

const PREFIX_COMMANDS: &[&str] = &[
    "sudo",
    "env",
    "nix-shell",
    "command",
    "exec",
    "nice",
    "time",
];

const SHELL_BUILTINS: &[&str] = &[
    "cd", "echo", "export", "source", "set", "unset", "true", "false", "test", "[", "]", "declare",
    "local", "readonly", "typeset", "alias", "bg", "fg", "jobs", "kill", "wait", "trap", "eval",
    "let", "shift", "return", "exit", "break", "continue", "pwd", "pushd", "popd", "dirs", "umask",
    "ulimit", "hash", "type", "builtin", "caller", "compgen", "complete", "printf", "read",
];

/// Extract external software/CLI tool names from a Bash command string.
///
/// Splits on pipes and chain operators, takes the first token of each segment,
/// strips path prefixes, skips prefix commands (sudo, env, etc.) and shell builtins.
pub fn extract_software(command: &str) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut result = Vec::new();

    let segments = split_command(command);

    for segment in segments {
        if let Some(name) = extract_first_tool(segment.trim()) {
            if !SHELL_BUILTINS.contains(&name.as_str()) && seen.insert(name.clone()) {
                result.push(name);
            }
        }
    }

    result
}

fn split_command(command: &str) -> Vec<&str> {
    let mut segments = Vec::new();
    let mut start = 0;
    let bytes = command.as_bytes();
    let len = bytes.len();
    let mut i = 0;
    let mut in_single_quote = false;
    let mut in_double_quote = false;

    while i < len {
        let b = bytes[i];

        if b == b'\'' && !in_double_quote {
            in_single_quote = !in_single_quote;
            i += 1;
            continue;
        }
        if b == b'"' && !in_single_quote {
            in_double_quote = !in_double_quote;
            i += 1;
            continue;
        }

        if in_single_quote || in_double_quote {
            i += 1;
            continue;
        }

        // Check for ||, &&
        if i + 1 < len {
            let next = bytes[i + 1];
            if (b == b'|' && next == b'|') || (b == b'&' && next == b'&') {
                segments.push(&command[start..i]);
                i += 2;
                start = i;
                continue;
            }
        }

        // Check for single | or ;
        if b == b'|' || b == b';' {
            segments.push(&command[start..i]);
            i += 1;
            start = i;
            continue;
        }

        i += 1;
    }

    if start < len {
        segments.push(&command[start..]);
    }

    segments
}

fn extract_first_tool(segment: &str) -> Option<String> {
    let mut tokens = segment.split_whitespace();
    let mut token = tokens.next()?;

    // Skip prefix commands (may chain: `sudo env X=1 cargo build`)
    while PREFIX_COMMANDS.contains(&strip_path(token).as_str()) {
        token = tokens.next()?;
        // Skip flags (e.g. `nice -n 10`, `sudo -u root`) and KEY=VALUE args (`env X=1`)
        loop {
            if token.starts_with('-') {
                // Flag — consume it and its value if the next token looks like a flag argument
                token = tokens.next()?;
                // If the next token is a plain numeric value, it's likely the flag's argument
                if token.parse::<f64>().is_ok() {
                    token = tokens.next()?;
                }
            } else if token.contains('=') {
                // KEY=VALUE style (env)
                token = tokens.next()?;
            } else {
                break;
            }
        }
    }

    let name = strip_path(token);

    if name.is_empty() {
        return None;
    }

    Some(name)
}

fn strip_path(token: &str) -> String {
    match token.rfind('/') {
        Some(idx) => token[idx + 1..].to_string(),
        None => token.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simple_command() {
        assert_eq!(extract_software("git commit -m \"fix\""), vec!["git"]);
    }

    #[test]
    fn chained_same_tool() {
        assert_eq!(extract_software("cargo build && cargo test"), vec!["cargo"]);
    }

    #[test]
    fn piped_different_tools() {
        assert_eq!(
            extract_software("cat foo.txt | grep bar | wc -l"),
            vec!["cat", "grep", "wc"]
        );
    }

    #[test]
    fn sudo_prefix() {
        assert_eq!(extract_software("sudo docker compose up"), vec!["docker"]);
    }

    #[test]
    fn cd_filtered_out() {
        assert_eq!(extract_software("cd /tmp && npm install"), vec!["npm"]);
    }

    #[test]
    fn echo_filtered_out() {
        let result = extract_software("echo \"hello\"");
        assert!(result.is_empty());
    }

    #[test]
    fn absolute_path_stripped() {
        assert_eq!(
            extract_software("/usr/local/bin/python3 script.py"),
            vec!["python3"]
        );
    }

    #[test]
    fn empty_command() {
        assert!(extract_software("").is_empty());
    }

    #[test]
    fn whitespace_only() {
        assert!(extract_software("   ").is_empty());
    }

    #[test]
    fn semicolon_separator() {
        assert_eq!(
            extract_software("git add . ; git commit -m 'test'"),
            vec!["git"]
        );
    }

    #[test]
    fn or_chain() {
        assert_eq!(
            extract_software("docker ps || docker start mycontainer"),
            vec!["docker"]
        );
    }

    #[test]
    fn env_prefix() {
        assert_eq!(
            extract_software("env NODE_ENV=production node server.js"),
            vec!["node"]
        );
    }

    #[test]
    fn time_prefix() {
        assert_eq!(extract_software("time cargo build"), vec!["cargo"]);
    }

    #[test]
    fn nice_prefix() {
        assert_eq!(extract_software("nice -n 10 make -j4"), vec!["make"]);
    }

    #[test]
    fn multiple_builtins_all_filtered() {
        assert!(extract_software("export FOO=bar && cd /tmp && echo done").is_empty());
    }

    #[test]
    fn mixed_builtins_and_real() {
        assert_eq!(
            extract_software("export PATH=$PATH:/foo && cargo build"),
            vec!["cargo"]
        );
    }

    #[test]
    fn complex_pipeline() {
        assert_eq!(
            extract_software("find . -name '*.rs' | xargs grep 'TODO' | sort -u"),
            vec!["find", "xargs", "sort"]
        );
    }

    #[test]
    fn quoted_pipe_not_split() {
        let result = extract_software(r#"echo "hello | world""#);
        assert!(result.is_empty());
    }

    #[test]
    fn flag_with_equals() {
        assert_eq!(
            extract_software("cargo build --target=x86_64"),
            vec!["cargo"]
        );
    }

    #[test]
    fn trailing_operator() {
        assert_eq!(extract_software("git add . &&"), vec!["git"]);
    }
}
