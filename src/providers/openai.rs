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

    // TODO
    async fn send_message(&self, _messages: Vec<Message>) -> Result<String, AegisError> {
        todo!("Implement OpenAI provider")
    }

    // TODO: Implement streaming
    async fn send_message_streaming(
        &self,
        _messages: Vec<Message>,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<String, AegisError>> + Send>>, AegisError> {
        todo!("Implement streaming")
    }
}
