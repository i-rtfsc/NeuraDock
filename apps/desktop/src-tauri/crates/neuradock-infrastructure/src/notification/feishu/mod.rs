mod message_builder;
mod sender;

use reqwest::Client;

/// Feishu webhook notification sender
pub struct FeishuWebhookSender {
    webhook_key: String,
    client: Client,
}

impl FeishuWebhookSender {
    pub fn new(webhook_key: String) -> Self {
        let client = Client::builder()
            .no_proxy()
            .build()
            .unwrap_or_else(|_| Client::new());

        Self {
            webhook_key,
            client,
        }
    }

    fn build_webhook_url(&self) -> String {
        format!(
            "https://open.feishu.cn/open-apis/bot/v2/hook/{}",
            self.webhook_key
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use neuradock_domain::notification::NotificationMessage;

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
    fn test_build_card_message_includes_time_line() {
        let sender = FeishuWebhookSender::new("test_key".to_string());
        let content = [
            "⏰ 时间: 2026-08-07 09:00:00",
            "账户: test@example.com",
            "服务商: AnyRouter",
            "📅 今天余额",
            "   当前余额: $4.97",
            "   历史消耗: $0.03",
            "   总额度: $5.00",
        ]
        .join("\n");
        let message = NotificationMessage::new("✅ 签到成功", content);
        let payload = sender.build_rich_message(&message);

        assert_eq!(payload["msg_type"], "interactive");
        let rendered = serde_json::to_string(&payload).unwrap();
        assert!(
            rendered.contains("2026-08-07"),
            "date line must appear in the card: {rendered}"
        );
        assert!(
            rendered.contains("test@example.com"),
            "account line must appear: {rendered}"
        );
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
