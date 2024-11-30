use async_trait::async_trait;
use futures::Stream;
use std::pin::Pin;

use crate::{
    error::AegisError,
    models::{Message, ProviderType},
};

use super::Provider;

pub struct OpenAIProvider {
    api_key: String,
}

impl OpenAIProvider {
    pub fn new(api_key: String) -> Self {
        Self { api_key }
    }
}

#[async_trait]
impl Provider for OpenAIProvider {
    fn provider_type(&self) -> ProviderType {
        ProviderType::OpenAI
    }

    
    async fn send_message(&self, messages: Vec<Message>) -> Result<String, AegisError> {
        let client = reqwest::Client::new();

        let request_body = serde_json::json!({
            "model": "gpt-4-turbo-preview",
            "messages": messages,
            "temperature": 0.7,
            "max_tokens": 2048
        });

        let response = client
            .post("https://api.openai.com/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&request_body)
            .send()
            .await
            .map_err(|e| AegisError::NetworkError(e))?;

        let status = response.status();
        let body = response.text().await.map_err(|e| AegisError::NetworkError(e))?;

        match status {
            reqwest::StatusCode::OK => {
                let parsed: serde_json::Value = serde_json::from_str(&body)
                    .map_err(|e| AegisError::APIError(e.to_string()))?;
                
                Ok(parsed["choices"][0]["message"]["content"]
                    .as_str()
                    .unwrap_or("")
                    .to_string())
            }
            reqwest::StatusCode::TOO_MANY_REQUESTS => Err(AegisError::RateLimitExceeded),
            reqwest::StatusCode::UNAUTHORIZED => Err(AegisError::InvalidAPIKey),
            _ => Err(AegisError::APIError(format!(
                "Status: {}, Body: {}",
                status, body
            )))
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
