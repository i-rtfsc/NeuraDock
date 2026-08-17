use neuradock_domain::notification::NotificationMessage;
use serde_json::json;

impl super::FeishuWebhookSender {
    /// Build a rich text (post) message for Feishu
    pub(super) fn build_rich_message(&self, message: &NotificationMessage) -> serde_json::Value {
        // Try to parse the content to check if it's a check-in success message with balance data
        // Accept both Chinese and English success title variants and fall back to content markers
        let title_lc = message.title.to_lowercase();
        let is_checkin_title = title_lc.contains("签到")
            || title_lc.contains("check-in")
            || title_lc.contains("checkin");
        if is_checkin_title && message.content.contains("📅") {
            // Build a card message for check-in success with balance comparison
            return self.build_card_message(message);
        }

        // Default rich text message for other cases
        let mut content_elements = vec![json!({
            "tag": "text",
            "text": message.content
        })];

        // Add link if provided
        if let Some(link) = &message.link {
            content_elements.push(json!({
                "tag": "a",
                "text": "查看详情",
                "href": link
            }));
        }

        json!({
            "msg_type": "post",
            "content": {
                "post": {
                    "zh_cn": {
                        "title": message.title,
                        "content": [content_elements]
                    }
                }
            }
        })
    }

    /// Build a card message for Feishu (for check-in success with balance data)
    pub(super) fn build_card_message(&self, message: &NotificationMessage) -> serde_json::Value {
        // Parse the content to extract account, provider and balance data
        let lines: Vec<&str> = message.content.lines().collect();

        // Build card elements
        let mut elements = vec![];

        // Add header with account, provider, and time info.
        // Lines that start with a known emoji prefix are identification,
        // not balance data.
        for line in lines.iter() {
            let trimmed = line.trim();
            if trimmed.starts_with("⏰")
                || trimmed.starts_with("账户")
                || trimmed.starts_with("Account")
                || trimmed.starts_with("服务商")
                || trimmed.starts_with("Provider")
            {
                elements.push(json!({
                    "tag": "div",
                    "text": {
                        "tag": "lark_md",
                        "content": format!("**{}**", line)
                    }
                }));
            }
        }

        // Add divider
        elements.push(json!({"tag": "hr"}));

        // Parse and add balance sections
        let mut current_section = String::new();
        let mut in_section = false;

        for line in lines.iter() {
            let trimmed = line.trim();

            if trimmed.starts_with("⏰") {
                // Already emitted by the header loop above. Skipping here is
                // what makes the time line appear at all: it matches none of
                // the section branches, so it used to fall through to the
                // catch-all and get swallowed into whichever balance section
                // happened to be open -- or dropped entirely when none was.
                continue;
            } else if trimmed.starts_with("📅") {
                // Start new section
                if !current_section.is_empty() {
                    elements.push(json!({
                        "tag": "div",
                        "text": {
                            "tag": "lark_md",
                            "content": current_section
                        }
                    }));
                    current_section.clear();
                }

                current_section = format!("**{}**\n", trimmed);
                in_section = true;
            } else if trimmed.starts_with("💰") {
                // Changes section
                if !current_section.is_empty() {
                    elements.push(json!({
                        "tag": "div",
                        "text": {
                            "tag": "lark_md",
                            "content": current_section
                        }
                    }));
                    current_section.clear();
                }

                elements.push(json!({"tag": "hr"}));
                current_section = format!("**{}**\n", trimmed);
                in_section = true;
            } else if in_section
                && !trimmed.is_empty()
                && !trimmed.starts_with("账户")
                && !trimmed.starts_with("服务商")
            {
                current_section.push_str(&format!("{}\n", trimmed));
            }
        }

        // Add last section if any
        if !current_section.is_empty() {
            elements.push(json!({
                "tag": "div",
                "text": {
                    "tag": "lark_md",
                    "content": current_section
                }
            }));
        }

        json!({
            "msg_type": "interactive",
            "card": {
                "header": {
                    "template": "green",
                    "title": {
                        "tag": "plain_text",
                        "content": message.title
                    }
                },
                "elements": elements
            }
        })
    }

    /// Build a simple text message for Feishu (fallback)
    #[allow(dead_code)]
    pub(super) fn build_text_message(&self, message: &NotificationMessage) -> serde_json::Value {
        let text = if message.link.is_some() {
            format!("{}\n\n{}", message.title, message.content)
        } else {
            format!("{}\n{}", message.title, message.content)
        };

        json!({
            "msg_type": "text",
            "content": {
                "text": text
            }
        })
    }
}
