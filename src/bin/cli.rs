use aegis::{
    config::AegisConfig,
    models::{Message, ProviderType},
    Aegis,
};
use anyhow::Result;
use clap::{Parser, Subcommand};
use colored::*;
use dialoguer::{theme::ColorfulTheme, Confirm, Input, Select};
use std::{fs, path::PathBuf};

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

        /// Save conversation to file
        #[arg(short, long)]
        save: Option<PathBuf>,
    },
    /// Load and run a conversation from a file
    Load {
        /// Path to conversation file
        #[arg(short, long)]
        file: PathBuf,
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
            model,
            save,
        } => handle_chat(provider, message, model, save).await?,
        Commands::Load { file } => handle_load(file).await?,
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
    save: Option<PathBuf>,
) -> Result<()> {
    let config = load_config()?;
    let aegis = Aegis::new(config);

    let provider_type = match provider.as_deref().unwrap_or("anthropic") {
        "anthropic" => ProviderType::Anthropic,
        "openai" => ProviderType::OpenAI,
        _ => {
            println!(
                "{}",
                "Invalid provider. Using Anthropic as default.".yellow()
            );
            ProviderType::Anthropic
        }
    };

    println!("\n{} {:?}", "Using provider:".blue(), provider_type);

    if let Some(model) = &model {
        println!("{} {}", "Model:".blue(), model);
    }

    let mut conversation = Vec::new();

    if let Some(one_shot) = message {
        // One-shot message mode
        let msg = Message {
            role: "user".to_string(),
            content: one_shot,
        };
        conversation.push(msg);

        match aegis
            .send_message(provider_type, conversation.clone())
            .await
        {
            Ok(response) => {
                println!("\n{}", response);
            }
            Err(e) => {
                println!("\n{}: {}", "Error".red(), e);
            }
        }
    } else {
        // Interactive mode
        println!(
            "{}",
            "\nStarting interactive chat session (type 'exit' to quit)".yellow()
        );

        loop {
            let input: String = Input::new().with_prompt("You").interact()?;

            if input.trim().to_lowercase() == "exit" {
                break;
            }

            conversation.push(Message {
                role: "user".to_string(),
                content: input,
            });

            match aegis
                .send_message(provider_type, conversation.clone())
                .await
            {
                Ok(response) => {
                    println!("\n{}", "Assistant:".green());
                    println!("{}\n", response);

                    conversation.push(Message {
                        role: "assistant".to_string(),
                        content: response,
                    });
                }
                Err(e) => {
                    println!("\n{}: {}", "Error".red(), e);
                }
            }
        }

        // Handle saving conversation
        if let Some(save_path) = save {
            if !conversation.is_empty() {
                let save_data = SavedConversation {
                    provider: format!("{:?}", provider_type),
                    model: model.unwrap_or_else(|| "default".to_string()),
                    messages: conversation,
                };

                let json = serde_json::to_string_pretty(&save_data)?;
                fs::write(&save_path, json)?;
                println!("\nConversation saved to: {}", save_path.display());
            }
        }
    }

    Ok(())
}

async fn handle_load(file: PathBuf) -> Result<()> {
    let content = fs::read_to_string(&file)?;
    let saved: SavedConversation = serde_json::from_str(&content)?;

    println!("\n{}", "Loaded conversation:".blue());
    println!("Provider: {}", saved.provider);
    println!("Model: {}", saved.model);
    println!("\n{}", "Messages:".yellow());

    for msg in &saved.messages {
        match msg.role.as_str() {
            "user" => println!("{}: {}", "You".blue(), msg.content),
            "assistant" => println!("{}: {}", "Assistant".green(), msg.content),
            _ => println!("{}: {}", msg.role, msg.content),
        }
    }

    if Confirm::new()
        .with_prompt("Would you like to continue this conversation?")
        .interact()?
    {
        let provider_type = match saved.provider.as_str() {
            "Anthropic" => ProviderType::Anthropic,
            "OpenAI" => ProviderType::OpenAI,
            _ => ProviderType::Anthropic,
        };

        let config = load_config()?;
        let aegis = Aegis::new(config);
        let mut conversation = saved.messages;

        loop {
            let input: String = Input::new().with_prompt("You").interact()?;

            if input.trim().to_lowercase() == "exit" {
                break;
            }

            conversation.push(Message {
                role: "user".to_string(),
                content: input,
            });

            match aegis
                .send_message(provider_type, conversation.clone())
                .await
            {
                Ok(response) => {
                    println!("\n{}", "Assistant:".green());
                    println!("{}\n", response);

                    conversation.push(Message {
                        role: "assistant".to_string(),
                        content: response,
                    });
                }
                Err(e) => {
                    println!("\n{}: {}", "Error".red(), e);
                }
            }
        }
    }

    Ok(())
}
