use futures_util::Stream;
use reqwest::Client;
use serde_json::Value;
use std::pin::Pin;
use tracing::{error, warn};

use super::tools::AiTools;

/// AI Client for OpenAI-compatible APIs
#[derive(Clone)]
pub struct AiClient {
    client: Client,
    base_url: String,
    api_key: String,
    model: String,
}

impl Default for AiClient {
    fn default() -> Self {
        Self {
            client: Client::new(),
            base_url: std::env::var("AI_BASE_URL")
                .unwrap_or_else(|_| "http://localhost:11434/v1".to_string()),
            api_key: std::env::var("AI_API_KEY").unwrap_or_else(|_| "ollama".to_string()),
            model: std::env::var("AI_MODEL")
                .unwrap_or_else(|_| "qwen2.5-coder:7b".to_string()),
        }
    }
}

impl AiClient {
    /// Stream a chat completion as SSE
    pub async fn stream_chat(
        &self,
        messages: Vec<Value>,
        _tools: &AiTools,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<String, String>> + Send>>, String> {
        let url = format!("{}/chat/completions", self.base_url);

        let body = serde_json::json!({
            "model": self.model,
            "messages": messages,
            "stream": true,
            // In a full implementation, add tools here
            // "tools": tools.to_openai_schema(),
        });

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("HTTP request failed: {}", e))?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(format!("API error {}: {}", status, text));
        }

        let byte_stream = response.bytes_stream();

        let stream = async_stream::stream! {
            use futures_util::StreamExt;

            let mut stream = byte_stream;
            let mut buffer = String::new();

            while let Some(chunk_result) = stream.next().await {
                match chunk_result {
                    Ok(bytes) => {
                        buffer.push_str(&String::from_utf8_lossy(&bytes));

                        // Process complete lines
                        while let Some(pos) = buffer.find('\n') {
                            let line = buffer.drain(..=pos).collect::<String>();
                            let line = line.trim();

                            if line.starts_with("data: ") {
                                let data = &line[6..];

                                if data == "[DONE]" {
                                    yield Ok(String::new());
                                    return;
                                }

                                match serde_json::from_str::<Value>(data) {
                                    Ok(json) => {
                                        if let Some(content) = json
                                            .get("choices")
                                            .and_then(|c| c.as_array())
                                            .and_then(|arr| arr.first())
                                            .and_then(|choice| choice.get("delta"))
                                            .and_then(|delta| delta.get("content"))
                                            .and_then(|c| c.as_str())
                                        {
                                            if !content.is_empty() {
                                                yield Ok(content.to_string());
                                            }
                                        }
                                    }
                                    Err(e) => {
                                        warn!("Failed to parse SSE JSON: {}", e);
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => {
                        yield Err(format!("Stream error: {}", e));
                        return;
                    }
                }
            }
        };

        Ok(Box::pin(stream))
    }

    /// Simple non-streaming chat (for tool execution)
    pub async fn chat(&self, messages: Vec<Value>) -> Result<String, String> {
        let url = format!("{}/chat/completions", self.base_url);

        let body = serde_json::json!({
            "model": self.model,
            "messages": messages,
            "stream": false,
        });

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("HTTP request failed: {}", e))?;

        let json: Value = response
            .json()
            .await
            .map_err(|e| format!("JSON parse error: {}", e))?;

        Ok(json
            .get("choices")
            .and_then(|c| c.as_array())
            .and_then(|arr| arr.first())
            .and_then(|choice| choice.get("message"))
            .and_then(|msg| msg.get("content"))
            .and_then(|c| c.as_str())
            .unwrap_or("")
            .to_string())
    }
}
