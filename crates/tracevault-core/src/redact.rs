use regex::Regex;

pub struct Redactor {
    patterns: Vec<Regex>,
    high_entropy_pattern: Regex,
}

const REDACTED: &str = "[REDACTED]";

impl Redactor {
    pub fn new() -> Self {
        let patterns = vec![
            // AWS Access Key
            r"AKIA[0-9A-Z]{16}",
            // GitHub token
            r"gh[ps]_[A-Za-z0-9]{36,}",
            // Generic API key patterns
            r#"(?i)(api[_-]?key|apikey|secret[_-]?key)\s*[:=]\s*["']?[A-Za-z0-9/+=]{20,}"#,
            // JWT
            r"eyJ[A-Za-z0-9_-]+\.eyJ[A-Za-z0-9_-]+\.[A-Za-z0-9_-]+",
            // RSA private key header
            r"-----BEGIN (?:RSA )?PRIVATE KEY-----",
            // Slack token
            r"xox[bpras]-[0-9A-Za-z\-]+",
            // Generic bearer token
            r"(?i)bearer\s+[A-Za-z0-9\-._~+/]+=*",
        ];

        Self {
            patterns: patterns
                .iter()
                .map(|p| Regex::new(p).unwrap())
                .collect(),
            high_entropy_pattern: Regex::new(r"[A-Za-z0-9/+_=\-]{16,}").unwrap(),
        }
    }

    pub fn redact_string(&self, input: &str) -> String {
        let mut result = input.to_string();

        // Pattern-based redaction first
        for pattern in &self.patterns {
            result = pattern.replace_all(&result, REDACTED).to_string();
        }

        // Entropy-based redaction
        let entropy_re = &self.high_entropy_pattern;
        result = entropy_re
            .replace_all(&result, |caps: &regex::Captures| {
                let matched = caps.get(0).unwrap().as_str();
                if shannon_entropy(matched) > 4.5 {
                    REDACTED.to_string()
                } else {
                    matched.to_string()
                }
            })
            .to_string();

        result
    }
}

impl Default for Redactor {
    fn default() -> Self {
        Self::new()
    }
}

fn shannon_entropy(s: &str) -> f64 {
    if s.is_empty() {
        return 0.0;
    }
    let mut freq = [0u32; 256];
    for b in s.bytes() {
        freq[b as usize] += 1;
    }
    let len = s.len() as f64;
    freq.iter()
        .filter(|&&c| c > 0)
        .map(|&c| {
            let p = c as f64 / len;
            -p * p.log2()
        })
        .sum()
}
