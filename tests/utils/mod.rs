//! Test utilities module
//!
//! Common helper functions for generating test data

use rand::Rng;

/// Generate a random team key (e.g., "TST-ABC123")
#[allow(dead_code)]
pub fn random_team_key() -> String {
    let mut rng = rand::thread_rng();
    let chars: String = (0..6)
        .map(|_| {
            let idx = rng.gen_range(0..36);
            if idx < 26 {
                (b'a' + idx) as char
            } else {
                (b'0' + idx - 26) as char
            }
        })
        .collect();
    format!("TST-{}", chars.to_uppercase())
}

/// Generate a random email address for testing
#[allow(dead_code)]
pub fn random_email() -> String {
    let team_key = random_team_key().to_lowercase();
    format!("test_{}@example.com", team_key)
}

/// Generate a random URL key for workspaces
#[allow(dead_code)]
pub fn random_url_key() -> String {
    let mut rng = rand::thread_rng();
    (0..8)
        .map(|_| {
            let idx = rng.gen_range(0..36);
            if idx < 26 {
                (b'a' + idx) as char
            } else {
                (b'0' + idx - 26) as char
            }
        })
        .collect()
}

/// Generate a random hex color (e.g., "#A3F4C5")
#[allow(dead_code)]
pub fn random_color() -> String {
    let mut rng = rand::thread_rng();
    format!("#{:06x}", rng.gen_range(0..0xFFFFFF))
}

/// Generate random long text content for testing
#[allow(dead_code)]
pub fn random_long_text(max_length: usize) -> String {
    let mut rng = rand::thread_rng();
    let length = if max_length > 100 {
        rng.gen_range(100..max_length)
    } else {
        max_length
    };
    (0..length)
        .map(|_| {
            let idx = rng.gen_range(0..62);
            if idx < 26 {
                (b'a' + idx) as char
            } else if idx < 52 {
                (b'A' + idx - 26) as char
            } else {
                (b'0' + idx - 52) as char
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_random_team_key_format() {
        let key = random_team_key();
        assert!(key.starts_with("TST-"));
        assert_eq!(key.len(), 10); // TST- + 6 chars
    }

    #[test]
    fn test_random_team_key_uniqueness() {
        let keys: Vec<String> = (0..100).map(|_| random_team_key()).collect();
        let unique: std::collections::HashSet<String> = keys.into_iter().collect();
        // With 36^6 possibilities, collisions should be rare in 100 samples
        assert!(unique.len() > 90);
    }

    #[test]
    fn test_random_email_format() {
        let email = random_email();
        assert!(email.contains("@example.com"));
        assert!(email.starts_with("test_"));
    }

    #[test]
    fn test_random_url_key_length() {
        let url_key = random_url_key();
        assert_eq!(url_key.len(), 8);
    }

    #[test]
    fn test_random_color_format() {
        let color = random_color();
        assert!(color.starts_with("#"));
        assert_eq!(color.len(), 7);
    }

    #[test]
    fn test_random_long_text_length() {
        let text = random_long_text(1000);
        assert!(text.len() >= 100 && text.len() <= 1000);
    }
}
