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

    async fn send_message(&self, messages: Vec<Message>) -> Result<String, AegisError>;

    async fn send_message_streaming(
        &self,
        messages: Vec<Message>,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<String, AegisError>> + Send>>, AegisError>;
}
