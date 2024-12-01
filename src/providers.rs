pub mod anthropic;
pub mod openai;

use async_trait::async_trait;
use futures::Stream;
use std::pin::Pin;

use crate::{
    error::AegisError,
    models::{Message, ProviderType},
};

#[async_trait]
pub trait Provider: Send + Sync {
    fn provider_type(&self) -> ProviderType;

    async fn send_message(&self, messages: Vec<Message>) -> Result<Message, AegisError>;

    async fn stream_message(
        &self,
        messages: Vec<Message>,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<Message, AegisError>> + Send>>, AegisError>;

    fn capabilities(&self) -> ProviderCapabilities;
}

#[derive(Debug, Clone)]
pub struct ProviderCapabilities {
    pub streaming: bool,
    pub max_tokens: usize,
    pub supported_content_types: Vec<String>,
    pub models: Vec<String>,
}
