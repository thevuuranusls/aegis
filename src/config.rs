
#[derive(Debug, Clone)]
pub struct AegisConfig {
    pub anthropic_api_key: Option<String>,
    pub openai_api_key: Option<String>,
}

impl AegisConfig {
    pub fn new() -> Self {
        Self {
            anthropic_api_key: None,
            openai_api_key: None,
        }
    }

    pub fn with_anthropic(mut self, key: String) -> Self {
        self.anthropic_api_key = if key.is_empty() { None } else { Some(key) };
        self
    }

    pub fn with_openai(mut self, key: String) -> Self {
        self.openai_api_key = if key.is_empty() { None } else { Some(key) };
        self
    }

    pub fn is_empty(&self) -> bool {
        self.anthropic_api_key.is_none() && self.openai_api_key.is_none()
    }
}

impl Default for AegisConfig {
    fn default() -> Self {
        Self::new()
    }
}