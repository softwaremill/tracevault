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

#[test]
fn redacts_jwt_token() {
    let r = Redactor::new();
    let input = "token: eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.abc123def456";
    let result = r.redact_string(input);
    assert!(result.contains("[REDACTED]"));
    assert!(!result.contains("eyJhbGciOiJIUzI1NiJ9"));
}

#[test]
fn redacts_rsa_private_key() {
    let r = Redactor::new();
    let input = "-----BEGIN RSA PRIVATE KEY-----\nMIIEow...";
    let result = r.redact_string(input);
    assert!(result.contains("[REDACTED]"));
}

#[test]
fn redacts_slack_token() {
    let r = Redactor::new();
    let input = "SLACK_TOKEN=xoxb-1234567890-abcdefghij";
    let result = r.redact_string(input);
    assert!(result.contains("[REDACTED]"));
    assert!(!result.contains("xoxb-"));
}

#[test]
fn redacts_bearer_token() {
    let r = Redactor::new();
    let input = "Authorization: Bearer eyJhbGciOiJSUzI1NiIsInR5cCI6IkpXVCJ9";
    let result = r.redact_string(input);
    assert!(result.contains("[REDACTED]"));
}

#[test]
fn redacts_generic_api_key() {
    let r = Redactor::new();
    let input = "api_key=ABCDEFGHIJKLMNOPQRSTUVWXYZ1234567890abcd";
    let result = r.redact_string(input);
    assert!(result.contains("[REDACTED]"));
}

#[test]
fn redacts_multiple_patterns() {
    let r = Redactor::new();
    let input = "key=AKIAIOSFODNN7EXAMPLE and token=ghp_ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghij";
    let result = r.redact_string(input);
    assert!(!result.contains("AKIA"));
    assert!(!result.contains("ghp_"));
}

#[test]
fn empty_string_returns_empty() {
    let r = Redactor::new();
    assert_eq!(r.redact_string(""), "");
}
