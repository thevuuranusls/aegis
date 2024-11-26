use thiserror::Error;

#[derive(Error, Debug)]
pub enum AegisError {
    #[error("Provider not found")]
    ProviderNotFound,

    #[error("API request failed: {0}")]
    APIError(String),

    #[error("Rate limit exceeded")]
    RateLimitExceeded,

    #[error("Invalid API key")]
    InvalidAPIKey,

    #[error("Network error: {0}")]
    NetworkError(#[from] reqwest::Error),
}