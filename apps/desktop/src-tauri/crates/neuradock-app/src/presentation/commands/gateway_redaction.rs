pub(super) fn redact_content(format: &str, content: &str) -> String {
    if content.trim().is_empty() {
        return content.to_string();
    }
    if format == "toml" {
        return redact_toml_content(content);
    }
    if format != "json" {
        return content.to_string();
    }
    let Ok(mut value) = serde_json::from_str::<serde_json::Value>(content) else {
        return content.to_string();
    };
    redact_json_value(&mut value);
    serde_json::to_string_pretty(&value).unwrap_or_else(|_| content.to_string())
}

fn redact_toml_content(content: &str) -> String {
    content
        .lines()
        .map(|line| {
            let Some((key, _)) = line.split_once('=') else {
                return line.to_string();
            };
            let normalized = key.trim().trim_matches(['\'', '"']).to_ascii_uppercase();
            if ["TOKEN", "API_KEY", "SECRET", "PASSWORD"]
                .iter()
                .any(|word| normalized.contains(word))
            {
                format!("{}= \"••••••••\"", key)
            } else {
                line.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn redact_json_value(value: &mut serde_json::Value) {
    match value {
        serde_json::Value::Object(object) => {
            for (key, value) in object {
                let normalized = key.to_ascii_uppercase();
                if ["TOKEN", "API_KEY", "SECRET", "PASSWORD"]
                    .iter()
                    .any(|word| normalized.contains(word))
                    && value.is_string()
                {
                    *value = serde_json::Value::String("••••••••".to_string());
                } else {
                    redact_json_value(value);
                }
            }
        }
        serde_json::Value::Array(values) => {
            for value in values {
                redact_json_value(value);
            }
        }
        _ => {}
    }
}
