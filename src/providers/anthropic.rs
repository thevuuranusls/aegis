use async_trait::async_trait;
use futures::Stream;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::pin::Pin;
use tracing::{debug, error};

use crate::{
    error::AegisError,
    models::{Message, ProviderType},
};

use super::Provider;

pub struct AnthropicProvider {
    client: Client,
    api_key: String,
}

#[derive(Serialize, Debug)]
struct AnthropicRequest {
    model: String,
    messages: Vec<Message>,
    max_tokens: u32,
    stream: bool,
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

#[derive(Deserialize, Debug)]
struct AnthropicResponse {
    content: Vec<AnthropicContent>,
    id: String,
}

#[derive(Deserialize, Debug)]
struct AnthropicContent {
    text: String,
    r#type: String,
}

impl AnthropicProvider {
    pub fn new(api_key: String) -> Self {
        Self {
            client: Client::new(),
            api_key,
        }
    }
}

#[async_trait]
impl Provider for AnthropicProvider {
    fn provider_type(&self) -> ProviderType {
        ProviderType::Anthropic
    }

    async fn send_message(&self, messages: Vec<Message>) -> Result<String, AegisError> {
        let request = AnthropicRequest {
            model: "claude-3-sonnet-20240229".to_string(),
            messages,
            max_tokens: 4096,
            stream: false,
        };

        debug!("Sending request to Anthropic: {:?}", request);

        let response = self
            .client
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

        // Capture both status and body text before consuming the response
        let status = response.status();
        let body = response.text().await.map_err(|e| {
            error!("Failed to get response body: {:?}", e);
            AegisError::NetworkError(e)
        })?;

        debug!("Response status: {}", status);

        match status {
            reqwest::StatusCode::OK => {
                // Try to parse the successful response
                match serde_json::from_str::<AnthropicResponse>(&body) {
                    Ok(response) => {
                        let content = response
                            .content
                            .into_iter()
                            .filter(|c| c.r#type == "text")
                            .map(|c| c.text)
                            .collect::<Vec<_>>()
                            .join("");

                        debug!("Successfully parsed response with ID: {}", response.id);
                        Ok(content)
                    }
                    Err(e) => {
                        error!("Failed to parse successful response: {:?}", e);
                        Ok(body) // Fallback to raw text if parsing fails
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
                // Try to parse the error response
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
                        // Can't parse as JSON, use the raw text
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

    // TODO: Implement streaming
    async fn send_message_streaming(
        &self,
        _messages: Vec<Message>,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<String, AegisError>> + Send>>, AegisError> {
        todo!("Implement streaming")
    }
}
