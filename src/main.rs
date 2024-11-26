use aegis::config::AegisConfig;
use aegis::models::Message;
use aegis::models::ProviderType::{Anthropic, OpenAI};
use aegis::Aegis;

#[tokio::main]
async fn main() {

    let (config, provider_type) = if let Ok(key) = std::env::var("ANTHROPIC_API_KEY") {
        (AegisConfig::new().with_anthropic(key), Anthropic)
    } else if let Ok(key) = std::env::var("OPENAI_API_KEY") {
        (AegisConfig::new().with_openai(key), OpenAI)
    } else {
        panic!("Either ANTHROPIC_API_KEY or OPENAI_API_KEY must be set in .env file");
    };

    let aegis = Aegis::new(config);
    let messages = vec![Message {
        role: "user".to_string(),
        content: "What is the capital of Vietnam?".to_string(),
    }];

    match aegis.send_message(provider_type, messages).await {
        Ok(resp) => {
            println!("Assistant: {}", resp)
        }
        Err(e) => {
            println!("Error: {}", e)
        }
    }
}
