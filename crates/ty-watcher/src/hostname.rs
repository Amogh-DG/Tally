/// Hostname sanitization and bucket ID generation helpers.

/// Sanitizes a raw hostname string by splitting on whitespace and joining with hyphens.
/// If the resulting string is empty, returns `"localhost"`.
pub fn sanitize_hostname(raw: &str) -> String {
    let sanitized = raw.split_whitespace().collect::<Vec<_>>().join("-");
    if sanitized.is_empty() {
        "localhost".to_string()
    } else {
        sanitized
    }
}

/// Retrieves the current system hostname and returns its sanitized form.
pub fn get_sanitized_hostname() -> String {
    let raw = gethostname::gethostname().to_string_lossy().to_string();
    sanitize_hostname(&raw)
}

/// Returns the window bucket identifier for a given hostname.
pub fn window_bucket_id(hostname: &str) -> String {
    format!("aw-watcher-window_{}", hostname)
}

/// Returns the AFK bucket identifier for a given hostname.
pub fn afk_bucket_id(hostname: &str) -> String {
    format!("aw-watcher-afk_{}", hostname)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_hostname() {
        assert_eq!(sanitize_hostname("my-pc"), "my-pc");
        assert_eq!(sanitize_hostname("DESKTOP 123"), "DESKTOP-123");
        assert_eq!(sanitize_hostname("  alpha   beta  gamma  "), "alpha-beta-gamma");
        assert_eq!(sanitize_hostname("   "), "localhost");
        assert_eq!(sanitize_hostname(""), "localhost");
    }

    #[test]
    fn test_bucket_ids() {
        assert_eq!(window_bucket_id("my-pc"), "aw-watcher-window_my-pc");
        assert_eq!(afk_bucket_id("my-pc"), "aw-watcher-afk_my-pc");
    }
}
