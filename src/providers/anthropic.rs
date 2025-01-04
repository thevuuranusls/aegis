use async_trait::async_trait;
use futures::{Stream, StreamExt};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::pin::Pin;
use tracing::{debug, error};

use crate::{
    error::AegisError,
    models::{Content, ContentPart, Message, Metadata, Role, Usage},
    providers::{Provider, ProviderCapabilities},
};

pub struct AnthropicProvider {
    client: Client,
    api_key: String,
}

#[derive(Serialize, Debug)]
struct AnthropicRequest {
    model: String,
    messages: Vec<AnthropicMessage>,
    max_tokens: u32,
    stream: bool,
}

#[derive(Serialize, Deserialize, Debug)]
struct AnthropicMessage {
    role: String,
    content: Vec<AnthropicContent>,
}

#[derive(Serialize, Deserialize, Debug)]
struct AnthropicContent {
    #[serde(rename = "type")]
    content_type: String,
    text: Option<String>,
}

#[derive(Deserialize, Debug)]
struct AnthropicResponse {
    content: Vec<AnthropicContent>,
    id: String,
    usage: Option<AnthropicUsage>,
}

#[derive(Deserialize, Debug)]
struct AnthropicUsage {
    input_tokens: u32,
    output_tokens: u32,
}

#[derive(Deserialize, Debug)]
struct AnthropicErrorResponse {
    error: AnthropicError,
}

#[derive(Deserialize, Debug)]
struct AnthropicError {
    #[serde(default)]
    message: String,
    #[serde(default)]
    r#type: String,
}

impl AnthropicProvider {
    pub fn new(api_key: String) -> Self {
        Self {
            client: Client::new(),
            api_key,
        }
    }

    fn convert_to_anthropic_messages(&self, messages: Vec<Message>) -> Vec<AnthropicMessage> {
        messages.into_iter()
            .map(|msg| AnthropicMessage {
                role: match msg.role {
                    Role::User => "user",
                    Role::Assistant => "assistant",
                    Role::System => "system",
                }.to_string(),
                content: msg.content.parts.into_iter()
                    .map(|part| match part {
                        ContentPart::Text { text } => AnthropicContent {
                            content_type: "text".to_string(),
                            text: Some(text),
                        },
                        ContentPart::Image { image_url: _ } => AnthropicContent {
                            content_type: "image".to_string(),
                            text: None,
                        },
                    })
                    .collect(),
            })
            .collect()
    }

    fn convert_from_anthropic_response(
        &self,
        content: Vec<AnthropicContent>,
        usage: Option<AnthropicUsage>,
    ) -> Message {
        Message {
            role: Role::Assistant,
            content: Content {
                parts: content
                    .into_iter()
                    .filter_map(|c| {
                        if c.content_type == "text" {
                            c.text.map(|text| ContentPart::Text { text })
                        } else {
                            None
                        }
                    })
                    .collect(),
            },
            metadata: Some(Metadata {
                model: Some("claude-3-sonnet-20240229".to_string()),
                provider: Some("anthropic".to_string()),
                usage: usage.map(|u| Usage {
                    prompt_tokens: u.input_tokens,
                    completion_tokens: u.output_tokens,
                    total_tokens: u.input_tokens + u.output_tokens,
                }),
            }),
        }
    }
}

#[async_trait]
impl Provider for AnthropicProvider {
    fn provider_type(&self) -> crate::models::ProviderType {
        crate::models::ProviderType::Anthropic
    }

    async fn send_message(&self, messages: Vec<Message>) -> Result<Message, AegisError> {
        let anthropic_messages = self.convert_to_anthropic_messages(messages);
        
        let request = AnthropicRequest {
            model: "claude-3-sonnet-20240229".to_string(),
            messages: anthropic_messages,
            max_tokens: 4096,
            stream: false,
        };

        debug!("Sending request to Anthropic: {:?}", request);

        let response = self.client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .json(&request)
            .send()
            .await
            .map_err(|e| {
                error!("Network error: {:?}", e);
                AegisError::NetworkError(e)
            })?;

        let status = response.status();
        let body = response.text().await.map_err(|e| {
            error!("Failed to get response body: {:?}", e);
            AegisError::NetworkError(e)
        })?;

        debug!("Response status: {}", status);

        match status {
            reqwest::StatusCode::OK => {
                match serde_json::from_str::<AnthropicResponse>(&body) {
                    Ok(response) => {
                        debug!("Successfully parsed response with ID: {}", response.id);
                        Ok(self.convert_from_anthropic_response(
                            response.content,
                            response.usage,
                        ))
                    }
                    Err(e) => {
                        error!("Failed to parse successful response: {:?}", e);
                        Err(AegisError::APIError(e.to_string()))
                    }
                }
            }
            reqwest::StatusCode::TOO_MANY_REQUESTS => {
                error!("Rate limit exceeded");
                Err(AegisError::RateLimitExceeded)
            }
            reqwest::StatusCode::UNAUTHORIZED => {
                error!("Invalid API key");
                Err(AegisError::InvalidAPIKey)
            }
            _ => {
                match serde_json::from_str::<AnthropicErrorResponse>(&body) {
                    Ok(error_response) => {
                        error!(
                            "API error - Type: {}, Message: {}",
                            error_response.error.r#type, error_response.error.message
                        );
                        Err(AegisError::APIError(format!(
                            "Type: {}, Message: {}",
                            error_response.error.r#type, error_response.error.message
                        )))
                    }
                    Err(_) => {
                        error!("Unexpected response: Status {}, Body: {}", status, body);
                        Err(AegisError::APIError(format!(
                            "Status: {}, Body: {}",
                            status, body
                        )))
                    }
                }
            }
        }
    }

    async fn stream_message(
        &self,
        messages: Vec<Message>,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<Message, AegisError>> + Send>>, AegisError> {
        let anthropic_messages = self.convert_to_anthropic_messages(messages);
        
        let request = AnthropicRequest {
            model: "claude-3-sonnet-20240229".to_string(),
            messages: anthropic_messages,
            max_tokens: 4096,
            stream: true,
        };

        let response = self.client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("Accept", "text/event-stream")
            .json(&request)
            .send()
            .await
            .map_err(|e| AegisError::NetworkError(e))?;

        if !response.status().is_success() {
            return Err(AegisError::APIError("Stream request failed".to_string()));
        }

        let stream = response
            .bytes_stream()
            .map(move |chunk| {
                chunk
                    .map_err(|e| AegisError::NetworkError(e))
                    .and_then(|bytes| {
                        let text = String::from_utf8_lossy(&bytes);
                        let lines: Vec<&str> = text.lines().collect();
                        
                        // Only process content_block_delta events
                        if lines.len() >= 2 {
                            let event_line = lines[0];
                            let data_line = lines[1];

                            if event_line.contains("content_block_delta") {
                                if let Some(json_str) = data_line.strip_prefix("data:") {
                                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(json_str.trim()) {
                                        if let Some(delta) = json.get("delta") {
                                            if let Some(text_delta) = delta.get("text") {
                                                if let Some(content) = text_delta.as_str() {
                                                    if !content.is_empty() {
                                                        return Ok(Message {
                                                            role: Role::Assistant,
                                                            content: Content {
                                                                parts: vec![ContentPart::Text { text: content.to_string() }],
                                                            },
                                                            metadata: None,
                                                        });
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        // Skip other events by returning an empty message that will be filtered out
                        Ok(Message {
                            role: Role::Assistant,
                            content: Content {
                                parts: vec![],
                            },
                            metadata: None,
                        })
                    })
            })
            .filter(|msg| futures::future::ready(matches!(msg, Ok(msg) if !msg.content.parts.is_empty())));

        Ok(Box::pin(stream))
    }

    fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities {
            streaming: true,
            max_tokens: 4096,
            supported_content_types: vec!["text".to_string(), "image".to_string()],
            models: vec!["claude-3-sonnet-20240229".to_string()],
        }
    }
}
