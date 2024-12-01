use async_trait::async_trait;
use futures::{Stream, StreamExt};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::pin::Pin;

use crate::{
    error::AegisError,
    models::{Content, ContentPart, Message, Metadata, Role},
    providers::{Provider, ProviderCapabilities},
};

pub struct OpenAIProvider {
    client: Client,
    api_key: String,
}

#[derive(Debug, Serialize)]
struct OpenAIRequest {
    model: String,
    messages: Vec<OpenAIMessage>,
    temperature: f32,
    max_tokens: u32,
    stream: bool,
}

#[derive(Debug, Serialize, Deserialize)]
struct OpenAIMessage {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct OpenAIResponse {
    choices: Vec<OpenAIChoice>,
    usage: Option<OpenAIUsage>,
}

#[derive(Debug, Serialize, Deserialize)]
struct OpenAIChoice {
    message: OpenAIMessage,
}

#[derive(Debug, Serialize, Deserialize)]
struct OpenAIUsage {
    prompt_tokens: u32,
    completion_tokens: u32,
    total_tokens: u32,
}

impl OpenAIProvider {
    pub fn new(api_key: String) -> Self {
        Self { 
            client: Client::new(),
            api_key 
        }
    }

    fn convert_to_openai_messages(&self, messages: Vec<Message>) -> Vec<OpenAIMessage> {
        messages.into_iter()
            .map(|msg| OpenAIMessage {
                role: match msg.role {
                    Role::User => "user",
                    Role::Assistant => "assistant",
                    Role::System => "system",
                }.to_string(),
                content: msg.content.parts.into_iter()
                    .filter_map(|part| match part {
                        ContentPart::Text { text } => Some(text),
                        _ => None, // Skip non-text content for now
                    })
                    .collect::<Vec<_>>()
                    .join(""),
            })
            .collect()
    }

    fn convert_from_openai_message(&self, msg: OpenAIMessage, usage: Option<OpenAIUsage>) -> Message {
        Message {
            role: match msg.role.as_str() {
                "assistant" => Role::Assistant,
                "user" => Role::User,
                "system" => Role::System,
                _ => Role::Assistant,
            },
            content: Content {
                parts: vec![ContentPart::Text { 
                    text: msg.content 
                }],
            },
            metadata: Some(Metadata {
                model: Some("gpt-4-turbo-preview".to_string()),
                provider: Some("openai".to_string()),
                usage: usage.map(|u| crate::models::Usage {
                    prompt_tokens: u.prompt_tokens,
                    completion_tokens: u.completion_tokens,
                    total_tokens: u.total_tokens,
                }),
            }),
        }
    }
}

#[async_trait]
impl Provider for OpenAIProvider {
    fn provider_type(&self) -> crate::models::ProviderType {
        crate::models::ProviderType::OpenAI
    }

    async fn send_message(&self, messages: Vec<Message>) -> Result<Message, AegisError> {
        let openai_messages = self.convert_to_openai_messages(messages);
        
        let request = OpenAIRequest {
            model: "gpt-4-turbo-preview".to_string(),
            messages: openai_messages,
            temperature: 0.7,
            max_tokens: 2048,
            stream: false,
        };

        let response = self.client
            .post("https://api.openai.com/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&request)
            .send()
            .await
            .map_err(|e| AegisError::NetworkError(e))?;

        let status = response.status();
        let body = response.text().await.map_err(|e| AegisError::NetworkError(e))?;

        match status {
            reqwest::StatusCode::OK => {
                let parsed: OpenAIResponse = serde_json::from_str(&body)
                    .map_err(|e| AegisError::APIError(e.to_string()))?;
                
                if let Some(choice) = parsed.choices.into_iter().next() {
                    Ok(self.convert_from_openai_message(choice.message, parsed.usage))
                } else {
                    Err(AegisError::APIError("No response choices".to_string()))
                }
            }
            reqwest::StatusCode::TOO_MANY_REQUESTS => Err(AegisError::RateLimitExceeded),
            reqwest::StatusCode::UNAUTHORIZED => Err(AegisError::InvalidAPIKey),
            _ => Err(AegisError::APIError(format!(
                "Status: {}, Body: {}",
                status, body
            )))
        }
    }

    async fn stream_message(
        &self,
        messages: Vec<Message>,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<Message, AegisError>> + Send>>, AegisError> {
        let openai_messages = self.convert_to_openai_messages(messages);
        
        let request = OpenAIRequest {
            model: "gpt-4-turbo-preview".to_string(),
            messages: openai_messages,
            temperature: 0.7,
            max_tokens: 2048,
            stream: true,
        };

        let response = self.client
            .post("https://api.openai.com/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", self.api_key))
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
                        // Parse SSE chunk and convert to Message
                        // This is a simplified version - you'll need to implement proper SSE parsing
                        let text = String::from_utf8_lossy(&bytes);
                        Ok(Message {
                            role: Role::Assistant,
                            content: Content {
                                parts: vec![ContentPart::Text { text: text.to_string() }],
                            },
                            metadata: None,
                        })
                    })
            });

        Ok(Box::pin(stream))
    }

    fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities {
            streaming: true,
            max_tokens: 4096,
            supported_content_types: vec!["text".to_string()],
            models: vec!["gpt-4-turbo-preview".to_string()],
        }
    }
}
