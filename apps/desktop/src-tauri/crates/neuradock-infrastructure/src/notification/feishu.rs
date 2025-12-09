use async_trait::async_trait;
use reqwest::Client;
use serde_json::json;

use neuradock_domain::notification::{NotificationMessage, NotificationSender};
use neuradock_domain::shared::DomainError;

/// Feishu webhook notification sender
pub struct FeishuWebhookSender {
    webhook_key: String,
    client: Client,
}

impl FeishuWebhookSender {
    pub fn new(webhook_key: String) -> Self {
        Self {
            webhook_key,
            client: Client::new(),
        }
    }

    fn build_webhook_url(&self) -> String {
        format!(
            "https://open.feishu.cn/open-apis/bot/v2/hook/{}",
            self.webhook_key
        )
    }

    /// Build a rich text (post) message for Feishu
    fn build_rich_message(&self, message: &NotificationMessage) -> serde_json::Value {
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
    fn build_card_message(&self, message: &NotificationMessage) -> serde_json::Value {
        // Parse the content to extract account, provider and balance data
        let lines: Vec<&str> = message.content.lines().collect();

        // Build card elements
        let mut elements = vec![];

        // Add header with account and provider info
        for line in lines.iter() {
            if line.starts_with("账户:") || line.starts_with("服务商:") {
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

            if trimmed.starts_with("📅") {
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
    fn build_text_message(&self, message: &NotificationMessage) -> serde_json::Value {
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

#[async_trait]
impl NotificationSender for FeishuWebhookSender {
    async fn send(&self, message: &NotificationMessage) -> Result<(), DomainError> {
        let url = self.build_webhook_url();
        let payload = self.build_rich_message(message);

        let response = self
            .client
            .post(&url)
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await
            .map_err(|e| {
                DomainError::Infrastructure(format!("Failed to send Feishu notification: {}", e))
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(DomainError::Infrastructure(format!(
                "Feishu webhook failed with status {}: {}",
                status, body
            )));
        }

        // Parse response to check for errors
        let resp_body: serde_json::Value = response.json().await.map_err(|e| {
            DomainError::Infrastructure(format!("Failed to parse Feishu response: {}", e))
        })?;

        // Feishu returns {"code":0} for success
        if let Some(code) = resp_body.get("code").and_then(|c| c.as_i64()) {
            if code != 0 {
                let msg = resp_body
                    .get("msg")
                    .and_then(|m| m.as_str())
                    .unwrap_or("Unknown error");
                return Err(DomainError::Infrastructure(format!(
                    "Feishu webhook error code {}: {}",
                    code, msg
                )));
            }
        }

        Ok(())
    }

    async fn test(&self) -> Result<(), DomainError> {
        let test_message = NotificationMessage::new(
            "测试通知",
            "这是一条来自 NeuraDock 的测试通知，如果您收到此消息，说明通知渠道配置成功！",
        );

        self.send(&test_message).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_webhook_url() {
        let sender = FeishuWebhookSender::new("test_key_123".to_string());
        let url = sender.build_webhook_url();
        assert_eq!(
            url,
            "https://open.feishu.cn/open-apis/bot/v2/hook/test_key_123"
        );
    }

    #[test]
    fn test_build_rich_message() {
        let sender = FeishuWebhookSender::new("test_key".to_string());
        let message = NotificationMessage::new("标题", "内容").with_link("https://example.com");

        let payload = sender.build_rich_message(&message);

        assert_eq!(payload["msg_type"], "post");
        assert_eq!(payload["content"]["post"]["zh_cn"]["title"], "标题");
    }

    #[test]
    fn test_build_text_message() {
        let sender = FeishuWebhookSender::new("test_key".to_string());
        let message = NotificationMessage::new("标题", "内容");

        let payload = sender.build_text_message(&message);

        assert_eq!(payload["msg_type"], "text");
        assert!(payload["content"]["text"]
            .as_str()
            .unwrap()
            .contains("标题"));
        assert!(payload["content"]["text"]
            .as_str()
            .unwrap()
            .contains("内容"));
    }
}
