pub mod config;
pub mod error;
pub mod models;
pub mod providers;

use std::sync::Arc;

use crate::models::{Message, ProviderType};
use config::AegisConfig;
use error::AegisError;
use futures::Stream;
use providers::Provider;

pub struct Aegis {
    providers: Vec<Arc<dyn Provider>>,
}

impl Aegis {
    /// Create a new Aegis instance with the given configuration.
    pub fn new(config: AegisConfig) -> Self {
        // TODO: choose the best provider if exists multiple
        let mut providers: Vec<Arc<dyn Provider>> = Vec::new();

        if let Some(anthropic_key) = config.anthropic_api_key {
            providers.push(Arc::new(providers::anthropic::AnthropicProvider::new(
                anthropic_key,
            )));
        }

        if let Some(openai_key) = config.openai_api_key {
            providers.push(Arc::new(providers::openai::OpenAIProvider::new(openai_key)));
        }

        Self { providers }
    }

    /// Send a message to the specified provider.
    pub async fn send_message(
        &self,
        provider_type: ProviderType,
        messages: Vec<Message>,
    ) -> Result<Message, AegisError> {
        let provider = self.get_provider(provider_type)?;
        provider.send_message(messages).await
    }

    pub async fn stream_message(
        &self,
        provider_type: ProviderType,
        messages: Vec<Message>,
    ) -> Result<impl Stream<Item = Result<Message, AegisError>>, AegisError> {
        let provider = self.get_provider(provider_type)?;
        provider.stream_message(messages).await
    }

    fn get_provider(&self, provider_type: ProviderType) -> Result<&Arc<dyn Provider>, AegisError> {
        self.providers
            .iter()
            .find(|p| p.provider_type() == provider_type)
            .ok_or(AegisError::ProviderNotFound)
    }
}
