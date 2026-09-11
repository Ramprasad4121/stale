use serde_json::Value;
use std::collections::HashMap;

#[derive(Clone)]
pub struct WebhookService {
    client: reqwest::Client,
}

impl WebhookService {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(5))
                .build()
                .unwrap_or_else(|_| reqwest::Client::new()),
        }
    }

    pub async fn dispatch_block_event(
        &self,
        webhook_url: Option<String>,
        slack_url: Option<String>,
        payload: Value,
    ) {
        if let Some(url) = webhook_url {
            if !url.is_empty() {
                let client = self.client.clone();
                let payload_clone = payload.clone();
                tokio::spawn(async move {
                    let _ = client.post(&url)
                        .json(&payload_clone)
                        .header("X-Stale-Event", "guardrail.blocked")
                        .header("X-Stale-Version", "2.0.0")
                        .send()
                        .await;
                });
            }
        }

        if let Some(slack_url) = slack_url {
            if !slack_url.is_empty() {
                let client = self.client.clone();
                tokio::spawn(async move {
                    let slack_payload = serde_json::json!({
                        "text": format!("🚨 Stale BLOCK: {}", payload.get("reason").and_then(|r| r.as_str()).unwrap_or("Unknown")),
                        "blocks": [
                            {
                                "type": "header",
                                "text": {
                                    "type": "plain_text",
                                    "text": "🚨 Stale Guardrail BLOCKED Transaction"
                                }
                            },
                            {
                                "type": "section",
                                "fields": [
                                    {
                                        "type": "mrkdwn",
                                        "text": format!("*Decision:*\n`BLOCK`")
                                    },
                                    {
                                        "type": "mrkdwn",
                                        "text": format!("*Blocked By:*\n`{}`", payload.get("blocked_by").and_then(|v| v.as_str()).unwrap_or("unknown"))
                                    },
                                    {
                                        "type": "mrkdwn",
                                        "text": format!("*Environment:*\n`{}`", payload.get("environment").and_then(|v| v.as_str()).unwrap_or("production"))
                                    },
                                    {
                                        "type": "mrkdwn",
                                        "text": format!("*Trace ID:*\n`{}`", payload.get("trace_id").and_then(|v| v.as_str()).unwrap_or("unknown"))
                                    }
                                ]
                            },
                            {
                                "type": "section",
                                "text": {
                                    "type": "mrkdwn",
                                    "text": format!("*Reason:*\n{}", payload.get("reason").and_then(|v| v.as_str()).unwrap_or("No reason"))
                                }
                            }
                        ]
                    });
                    let _ = client.post(&slack_url).json(&slack_payload).send().await;
                });
            }
        }
    }
}

impl Default for WebhookService {
    fn default() -> Self {
        Self::new()
    }
}
