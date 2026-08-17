/// Utilities for sanitizing sensitive data in logs
///
/// This module provides functions to mask sensitive information before logging,
/// preventing credential leaks in log files.
/// Masks sensitive string data for logging
///
/// # Examples
///
/// ```
/// use neuradock_infrastructure::logging::log_utils::mask_sensitive;
///
/// // API keys
/// assert_eq!(mask_sensitive("sk-proj-1234567890abcdef"), "sk-pr***cdef");
///
/// // Tokens
/// assert_eq!(
///     mask_sensitive("eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9"),
///     "eyJhb***VCJ9"
/// );
///
/// // Short strings show first 3 and last 2 characters
/// assert_eq!(mask_sensitive("abc123"), "abc***23");
/// ```
pub fn mask_sensitive(value: &str) -> String {
    if value.is_empty() {
        return String::new();
    }

    // Count characters, not bytes: a non-ASCII secret (or any multi-byte text
    // that reaches this function) would otherwise be sliced mid-codepoint and
    // panic.
    let len = value.chars().count();

    // For very short strings (< 6 chars), show nothing
    if len < 6 {
        return "***".to_string();
    }

    let (head, tail) = if len <= 10 {
        (3, 2)
    } else if len <= 20 {
        (4, 3)
    } else {
        (5, 4)
    };

    let prefix: String = value.chars().take(head).collect();
    let suffix: String = value
        .chars()
        .skip(len.saturating_sub(tail))
        .collect::<String>();
    format!("{prefix}***{suffix}")
}

/// Truncate `value` to at most `max_chars` characters for inclusion in a log
/// line or error message.
///
/// Response bodies were previously cut with `&body[..body.len().min(N)]`, which
/// indexes by **bytes**. Any provider returning a non-ASCII body -- a Chinese
/// error page, for instance -- lands mid-codepoint and panics. Because these
/// sites sit inside Tauri command futures, the panic never surfaces as an
/// error: the IPC promise simply never resolves and the UI hangs until the app
/// is restarted.
pub fn truncate_for_log(value: &str, max_chars: usize) -> &str {
    match value.char_indices().nth(max_chars) {
        Some((byte_idx, _)) => &value[..byte_idx],
        None => value,
    }
}

/// Masks sensitive cookie values for logging
///
/// # Examples
///
/// ```
/// use neuradock_infrastructure::logging::log_utils::mask_cookie;
/// use std::collections::HashMap;
///
/// let mut cookies = HashMap::new();
/// cookies.insert("acw_tc".to_string(), "sensitive_value_123".to_string());
/// cookies.insert("session".to_string(), "another_secret".to_string());
///
/// let masked = mask_cookie(&cookies);
/// // HashMap iteration order is not stable, so assert by substrings.
/// assert!(masked.contains("acw_tc=sens***123"));
/// assert!(masked.contains("session=anot***ret"));
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

    // Keep the last 4 *characters*. Byte indexing would panic if a key ever
    // contains non-ASCII (a mistyped or pasted value reaching a log line).
    let last4 = |value: &str| -> String {
        let len = value.chars().count();
        value.chars().skip(len.saturating_sub(4)).collect()
    };

    // Handle OpenAI format: sk-proj-xxxxx
    if api_key.starts_with("sk-proj-") {
        return format!("sk-proj-***{}", last4(api_key));
    }

    // Handle Anthropic format: sk-ant-api03-xxxxx
    if api_key.starts_with("sk-ant-") {
        return format!("sk-ant-***{}", last4(api_key));
    }

    // Handle generic sk- format
    if api_key.starts_with("sk-") {
        return format!("sk-***{}", last4(api_key));
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

    /// The exact shape that used to panic: a Chinese error page truncated at a
    /// byte offset that lands inside a multi-byte character.
    #[test]
    fn truncate_for_log_does_not_split_multibyte_characters() {
        let body = "系统繁忙，请稍后再试。当前请求已被风控拦截。";

        for n in 0..=body.chars().count() + 5 {
            let out = truncate_for_log(body, n);
            assert!(body.starts_with(out), "truncation must be a prefix (n={n})");
            assert!(out.chars().count() <= n.min(body.chars().count()));
        }
    }

    #[test]
    fn truncate_for_log_returns_whole_string_when_shorter() {
        assert_eq!(truncate_for_log("short", 100), "short");
        assert_eq!(truncate_for_log("", 10), "");
    }

    #[test]
    fn truncate_for_log_cuts_at_character_count() {
        assert_eq!(truncate_for_log("abcdef", 3), "abc");
        // 3 characters, not 3 bytes.
        assert_eq!(truncate_for_log("中文字符", 3), "中文字");
    }

    #[test]
    fn mask_sensitive_handles_non_ascii_without_panicking() {
        // Previously sliced by byte index and panicked mid-codepoint.
        let masked = mask_sensitive("密钥密钥密钥密钥密钥密钥");
        assert!(masked.contains("***"));
        let short = mask_sensitive("密钥密");
        assert_eq!(short, "***");
    }

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
