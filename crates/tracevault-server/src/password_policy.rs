/// Common breached passwords (top entries from known breach databases).
/// Passwords on this list are rejected regardless of length.
const BREACHED_PASSWORDS: &[&str] = &[
    "123456789012",
    "password1234",
    "qwerty123456",
    "iloveyou1234",
    "admin1234567",
    "letmein12345",
    "welcome12345",
    "monkey123456",
    "dragon123456",
    "master123456",
    "qwertyuiopas",
    "password1235",
    "123456789abc",
    "abc123456789",
    "trustno1trust",
    "changeme1234",
    "football1234",
    "baseball1234",
    "shadow123456",
    "michael12345",
    "jennifer1234",
    "superman1234",
    "batman123456",
    "whatever1234",
    "passw0rd1234",
    "p@ssword1234",
    "password12345",
    "1234567890ab",
    "qazwsx123456",
    "000000000000",
    "111111111111",
    "121212121212",
    "aaaaaaaaaaaa",
    "abcdefghijkl",
    "abcdef123456",
    "password!234",
    "charlie12345",
    "donald123456",
    "loveme123456",
    "sunshine1234",
    "princess1234",
    "starwars1234",
    "computer1234",
    "corvette1234",
    "1qaz2wsx3edc",
    "zaq12wsx3edc",
    "asdfghjkl123",
    "qwertyuiop12",
    "1q2w3e4r5t6y",
    "abcdefg12345",
];

const MIN_LENGTH: usize = 10;
const MAX_LENGTH: usize = 128;

/// Validate a password against the NIST 800-63B-inspired policy.
/// Returns `Ok(())` if the password is acceptable, or `Err(reason)` if not.
pub fn validate(password: &str) -> Result<(), &'static str> {
    if password.len() < MIN_LENGTH {
        return Err("Password must be at least 10 characters");
    }
    if password.len() > MAX_LENGTH {
        return Err("Password must be at most 128 characters");
    }
    let lower = password.to_lowercase();
    if BREACHED_PASSWORDS.contains(&lower.as_str()) {
        return Err("This password is too common and has appeared in data breaches");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_short_passwords() {
        assert!(validate("short").is_err());
        assert!(validate("9charssss").is_err());
    }

    #[test]
    fn accepts_minimum_length() {
        assert!(validate("10charssss").is_ok());
        assert!(validate("exactly10c").is_ok());
    }

    #[test]
    fn rejects_too_long() {
        let long = "a".repeat(129);
        assert!(validate(&long).is_err());
    }

    #[test]
    fn accepts_max_length() {
        let max = "a".repeat(128);
        assert!(validate(&max).is_ok());
    }

    #[test]
    fn rejects_breached_passwords() {
        assert!(validate("password1234").is_err());
        assert!(validate("qwerty123456").is_err());
        assert!(validate("123456789012").is_err());
    }

    #[test]
    fn breached_check_is_case_insensitive() {
        assert!(validate("PASSWORD1234").is_err());
        assert!(validate("Password1234").is_err());
    }

    #[test]
    fn accepts_good_passwords() {
        assert!(validate("my-secure-passphrase-2026").is_ok());
        assert!(validate("correcthorsebatterystaple").is_ok());
        assert!(validate("xK9#mP2$vL5@qR8").is_ok());
    }
}
