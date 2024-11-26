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
        self.anthropic_api_key = Some(key);
        self
    }

    pub fn with_openai(mut self, key: String) -> Self {
        self.openai_api_key = Some(key);
        self
    }
}

impl Default for AegisConfig {
    fn default() -> Self {
        Self::new()
    }
}