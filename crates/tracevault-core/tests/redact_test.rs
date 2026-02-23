use tracevault_core::redact::Redactor;

#[test]
fn redacts_high_entropy_strings() {
    let r = Redactor::new();
    let input = "token = \"aK3bF9xZ2mQ7nR4pL8wS5vY1cD6eH0j\"";
    let output = r.redact_string(input);
    assert!(output.contains("[REDACTED]"));
    assert!(!output.contains("aK3bF9xZ2mQ7nR4pL8wS5vY1cD6eH0j"));
}

#[test]
fn redacts_aws_access_key() {
    let r = Redactor::new();
    let input = "aws_key = \"AKIAIOSFODNN7EXAMPLE\"";
    let output = r.redact_string(input);
    assert!(output.contains("[REDACTED]"));
}

#[test]
fn redacts_github_token() {
    let r = Redactor::new();
    let input = "token: ghp_ABCDEFGHIJKLMNOPQRSTUVWXYZabcdef12";
    let output = r.redact_string(input);
    assert!(output.contains("[REDACTED]"));
}

#[test]
fn preserves_normal_text() {
    let r = Redactor::new();
    let input = "This is a normal code comment with variable_name and some_function()";
    let output = r.redact_string(input);
    assert_eq!(input, output);
}

#[test]
fn preserves_short_alphanumeric() {
    let r = Redactor::new();
    let input = "id = \"abc123\"";
    let output = r.redact_string(input);
    assert_eq!(input, output);
}
