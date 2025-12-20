use once_cell::sync::Lazy;
use serde_json::Value;
use tracing::error;

static TRANSLATIONS_ZH: Lazy<Value> = Lazy::new(|| {
    let json_str = include_str!("i18n/locales/zh-CN.json");
    serde_json::from_str(json_str).unwrap_or_else(|e| {
        error!("Failed to parse zh-CN.json: {}", e);
        Value::Object(serde_json::Map::new())
    })
});

static TRANSLATIONS_EN: Lazy<Value> = Lazy::new(|| {
    let json_str = include_str!("i18n/locales/en-US.json");
    serde_json::from_str(json_str).unwrap_or_else(|e| {
        error!("Failed to parse en-US.json: {}", e);
        Value::Object(serde_json::Map::new())
    })
});

/// Get translation by key path (e.g., "notification.checkIn.success.title")
/// Selects language from `NEURADOCK_LOCALE` env var (prefix match: "zh" => Chinese, otherwise English).
pub fn t(key: &str) -> String {
    let locale = std::env::var("NEURADOCK_LOCALE").unwrap_or_else(|_| "zh_CN".to_string());
    let translations = if locale.to_lowercase().starts_with("zh") {
        &*TRANSLATIONS_ZH
    } else {
        &*TRANSLATIONS_EN
    };

    // Navigate the nested JSON structure using the key path
    let parts: Vec<&str> = key.split('.').collect();
    let mut current = translations;

    for part in parts {
        match current.get(part) {
            Some(value) => current = value,
            None => return key.to_string(), // Fallback to key if not found
        }
    }

    // Return the string value or the key as fallback
    current.as_str().unwrap_or(key).to_string()
}
