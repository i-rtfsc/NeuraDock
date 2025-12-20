/// Utilities for sanitizing sensitive data in logs
///
/// This module provides functions to mask sensitive information before logging,
/// preventing credential leaks in log files.
/// Masks sensitive string data for logging
///
/// # Examples
///
/// ```
/// use neuradock_app::presentation::log_utils::mask_sensitive;
///
/// // API keys
/// assert_eq!(mask_sensitive("sk-proj-1234567890abcdef"), "sk-pr***cdef");
///
/// // Tokens
/// assert_eq!(mask_sensitive("eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9..."), "eyJhb***VCJ9");
///
/// // Short strings show first 3 and last 2 characters
/// assert_eq!(mask_sensitive("abc123"), "abc***23");
/// ```
pub fn mask_sensitive(value: &str) -> String {
    if value.is_empty() {
        return String::new();
    }

    let len = value.len();

    // For very short strings (< 6 chars), show nothing
    if len < 6 {
        return "***".to_string();
    }

    // For short strings (6-10 chars), show first 3 and last 2
    if len <= 10 {
        return format!("{}***{}", &value[..3], &value[len - 2..]);
    }

    // For medium strings (11-20 chars), show first 4 and last 3
    if len <= 20 {
        return format!("{}***{}", &value[..4], &value[len - 3..]);
    }

    // For long strings (> 20 chars), show first 5 and last 4
    format!("{}***{}", &value[..5], &value[len - 4..])
}

/// Masks sensitive cookie values for logging
///
/// # Examples
///
/// ```
/// use neuradock_app::presentation::log_utils::mask_cookie;
/// use std::collections::HashMap;
///
/// let mut cookies = HashMap::new();
/// cookies.insert("acw_tc".to_string(), "sensitive_value_123".to_string());
/// cookies.insert("session".to_string(), "another_secret".to_string());
///
/// let masked = mask_cookie(&cookies);
/// assert_eq!(masked, "acw_tc=sensi***t_123, session=anoth***ecret");
/// ```
pub fn mask_cookie(cookies: &std::collections::HashMap<String, String>) -> String {
    cookies
        .iter()
        .map(|(k, v)| format!("{}={}", k, mask_sensitive(v)))
        .collect::<Vec<_>>()
        .join(", ")
}

/// Masks API key for logging
///
/// Specifically designed for API keys with common formats:
/// - OpenAI: sk-proj-xxxxx
/// - Anthropic: sk-ant-api03-xxxxx
/// - Generic: xxxxx
pub fn mask_api_key(api_key: &str) -> String {
    if api_key.is_empty() {
        return String::new();
    }

    // Handle OpenAI format: sk-proj-xxxxx
    if api_key.starts_with("sk-proj-") {
        return format!("sk-proj-***{}", &api_key[api_key.len().saturating_sub(4)..]);
    }

    // Handle Anthropic format: sk-ant-api03-xxxxx
    if api_key.starts_with("sk-ant-") {
        return format!("sk-ant-***{}", &api_key[api_key.len().saturating_sub(4)..]);
    }

    // Handle generic sk- format
    if api_key.starts_with("sk-") {
        return format!("sk-***{}", &api_key[api_key.len().saturating_sub(4)..]);
    }

    // Fall back to generic masking
    mask_sensitive(api_key)
}

/// Counts the number of items without revealing their contents
///
/// Useful for logging collection sizes without exposing sensitive data
pub fn count_only<T>(items: &[T], item_type: &str) -> String {
    format!("{} {}", items.len(), item_type)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_mask_sensitive_short() {
        assert_eq!(mask_sensitive("ab"), "***");
        assert_eq!(mask_sensitive("abc"), "***");
        assert_eq!(mask_sensitive("abcd"), "***");
        assert_eq!(mask_sensitive("abcde"), "***");
    }

    #[test]
    fn test_mask_sensitive_medium() {
        assert_eq!(mask_sensitive("abcdef"), "abc***ef");
        assert_eq!(mask_sensitive("1234567890"), "123***90");
    }

    #[test]
    fn test_mask_sensitive_long() {
        assert_eq!(mask_sensitive("12345678901"), "1234***901");
        assert_eq!(mask_sensitive("1234567890123456789012345"), "12345***2345");
    }

    #[test]
    fn test_mask_api_key_openai() {
        let key = "sk-proj-1234567890abcdefghijklmnopqrstuvwxyz";
        let masked = mask_api_key(key);
        assert!(masked.starts_with("sk-proj-***"));
        assert!(masked.ends_with("wxyz"));
    }

    #[test]
    fn test_mask_api_key_anthropic() {
        let key = "sk-ant-api03-1234567890abcdefghijklmnopqrstuvwxyz";
        let masked = mask_api_key(key);
        assert!(masked.starts_with("sk-ant-***"));
        assert!(masked.ends_with("wxyz"));
    }

    #[test]
    fn test_mask_cookie() {
        let mut cookies = HashMap::new();
        cookies.insert("acw_tc".to_string(), "1234567890".to_string());
        cookies.insert("session".to_string(), "abcdefghij".to_string());

        let masked = mask_cookie(&cookies);
        assert!(masked.contains("acw_tc=123***90"));
        assert!(masked.contains("session=abc***ij"));
    }

    #[test]
    fn test_count_only() {
        let items = vec![1, 2, 3, 4, 5];
        assert_eq!(count_only(&items, "tokens"), "5 tokens");

        let empty: Vec<i32> = vec![];
        assert_eq!(count_only(&empty, "cookies"), "0 cookies");
    }
}
