use aegis::config::AegisConfig;
use aegis::models::Message;
use aegis::models::ProviderType::Anthropic;
use aegis::Aegis;

#[tokio::main]
async fn main() {
    trial_anthropic().await;
}

async fn trial_anthropic() {
    let key = std::env::var("ANTHROPIC_API_KEY")
        .expect("ANTHROPIC_API_KEY must be set in .env file");
    let config = AegisConfig::new().with_anthropic(key);
    let aegis = Aegis::new(config);

    let messages = vec![Message {
        role: "user".to_string(),
        content: "What is the capital of Vietnam?".to_string(),
    }];

    match aegis.send_message(Anthropic, messages).await {
        Ok(resp) => {
            println!("Assistant: {}", resp)
        }
        Err(e) => {
            println!("Error: {}", e)
        }
    }
}
