use aegis::{
    config::AegisConfig,
    models::{Message, ProviderType},
    Aegis,
};
use anyhow::Result;
use clap::{Parser, Subcommand};
use colored::*;
use dialoguer::{theme::ColorfulTheme, Input, Select};
use futures::StreamExt;
use std::{fs, process::exit};

#[derive(Parser)]
#[command(name = "aegis")]
#[command(version, about = "AI Provider CLI Interface", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Configure API keys and settings
    Config {
        /// Show current configuration
        #[arg(short, long)]
        show: bool,
    },
    /// Chat with AI models
    Chat {
        /// Select AI provider (anthropic/openai)
        #[arg(short, long)]
        provider: Option<String>,

        /// One-shot content mode
        #[arg(short, long)]
        content: Option<String>,

        /// Model to use (e.g., claude-3-sonnet, gpt-4)
        #[arg(short, long)]
        model: Option<String>,
    },
}

fn load_config() -> Result<AegisConfig> {
    dotenv::dotenv().ok();

    let config = AegisConfig::new()
        .with_anthropic(std::env::var("ANTHROPIC_API_KEY").unwrap_or_default())
        .with_openai(std::env::var("OPENAI_API_KEY").unwrap_or_default());

    Ok(config)
}

fn update_env_file(key: &str, value: &str) -> Result<()> {
    let env_path = ".env";
    let mut content = String::new();
    let mut updated = false;

    if let Ok(existing) = fs::read_to_string(env_path) {
        for line in existing.lines() {
            if line.starts_with(&format!("{}=", key)) {
                content.push_str(&format!("{}={}\n", key, value));
                updated = true;
            } else {
                content.push_str(&format!("{}\n", line));
            }
        }
    }

    if !updated {
        content.push_str(&format!("{}={}\n", key, value));
    }

    fs::write(env_path, content)?;
    Ok(())
}

#[derive(serde::Serialize, serde::Deserialize)]
struct SavedConversation {
    provider: String,
    model: String,
    messages: Vec<Message>,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Setup logging
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Config { show } => handle_config(show)?,
        Commands::Chat {
            provider,
            content: message,
            model
        } => handle_chat(provider, message, model).await?
    }

    Ok(())
}

fn handle_config(show: bool) -> Result<()> {
    if show {
        println!("\n{}", "Current Configuration:".bold());
        if let Ok(content) = fs::read_to_string(".env") {
            for line in content.lines() {
                if line.starts_with("ANTHROPIC_API_KEY=") {
                    println!("Anthropic API Key: {}", "[SET]".green());
                } else if line.starts_with("OPENAI_API_KEY=") {
                    println!("OpenAI API Key: {}", "[SET]".green());
                }
            }
        }
        return Ok(());
    }

    let theme = ColorfulTheme::default();
    let providers = vec!["Anthropic API Key", "OpenAI API Key"];

    let selection = Select::with_theme(&theme)
        .with_prompt("Select provider to configure")
        .items(&providers)
        .default(0)
        .interact()?;

    let key: String = Input::with_theme(&theme)
        .with_prompt("Enter API key")
        .interact()?;

    let env_key = match selection {
        0 => "ANTHROPIC_API_KEY",
        1 => "OPENAI_API_KEY",
        _ => unreachable!(),
    };

    update_env_file(env_key, &key)?;
    println!("\n{}", "Configuration saved successfully!".green());
    Ok(())
}

async fn handle_chat(
    provider: Option<String>,
    message: Option<String>,
    model: Option<String>,
) -> Result<()> {
    let config = load_config()?;
    let aegis = Aegis::new(config);

    let provider_type = match provider.as_deref().unwrap_or("anthropic") {
        "anthropic" => ProviderType::Anthropic,
        "openai" => ProviderType::OpenAI,
        _ => {
            println!("{}", "Invalid provider. Using Anthropic as default.".yellow());
            exit(3)
        }
    };

    println!("\n{} {:?}", "Using provider:".blue(), provider_type);
    if let Some(model) = &model {
        println!("{} {}", "Model:".blue(), model);
    }

    // Decision point: Use streaming or regular chat
    if needs_streaming(&message) {
        handle_streaming_chat(&aegis, provider_type, message).await?;
    } else {
        handle_regular_chat(&aegis, provider_type, message).await?;
    }

    Ok(())
}

// Helper to determine if streaming is needed
fn needs_streaming(message: &Option<String>) -> bool {
    match message {
        Some(content) => {
            // Long content generation needs streaming
            content.len() > 100 || 
            // Interactive mode needs streaming
            content.is_empty()
        }
        None => true // Interactive mode needs streaming
    }
}

// Handle streaming chat (real-time updates)
async fn handle_streaming_chat(
    aegis: &Aegis,
    provider_type: ProviderType,
    message: Option<String>,
) -> Result<()> {
    let mut stream = if let Some(content) = message {
        // One-shot streaming mode
        let msg = Message {
            role: aegis::models::Role::User,
            content: aegis::models::Content {
                parts: vec![aegis::models::ContentPart::Text { text: content }],
            },
            metadata: None
        };
        aegis.stream_message(provider_type, vec![msg]).await?
    } else {
        // Interactive streaming mode
        println!("{}", "\nStarting interactive chat session (type 'exit' to quit)".yellow());
        loop {
            let input: String = Input::new().with_prompt("You").interact()?;
            if input.trim().to_lowercase() == "exit" {
                break;
            }

            let msg = Message{
                role: aegis::models::Role::User,
                content: aegis::models::Content {
                    parts: vec![aegis::models::ContentPart::Text { text: input }],
                },
                metadata: None
            };
            let mut stream = aegis.stream_message(provider_type.clone(), vec![msg]).await?;

            println!("\n{}", "Assistant:".green());
            while let Some(chunk) = stream.next().await {
                match chunk {
                    Ok(response) => print!("{}", response.content),
                    Err(e) => println!("\n{}: {}", "Error".red(), e),
                }
            }
            println!("\n");
        }
        return Ok(());
    };

    // Handle one-shot streaming response
    println!("\n{}", "Assistant:".green());
    while let Some(chunk) = stream.next().await {
        match chunk {
            Ok(response) => {
                print!("{}", response.content);
            }
            Err(e) => println!("\n{}: {}", "Error".red(), e),
        }
    }
    println!("\n");

    Ok(())
}

// Handle regular chat (single response)
async fn handle_regular_chat(
    aegis: &Aegis,
    provider_type: ProviderType,
    message: Option<String>,
) -> Result<()> {
    let content = message.ok_or_else(|| anyhow::anyhow!("Message content required for regular chat"))?;
    let msg = Message {
        role: aegis::models::Role::User,
        content: aegis::models::Content {
            parts: vec![aegis::models::ContentPart::Text { text: content }],
        },
        metadata: None,
    };
    
    match aegis.send_message(provider_type, vec![msg]).await {
        Ok(response) => println!("\n{}: {}\n", "Assistant".green(), response.content),
        Err(e) => println!("\n{}: {}", "Error".red(), e),
    }

    Ok(())
}
