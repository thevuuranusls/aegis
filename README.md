# Aegis

Aegis is a Rust-based adapter library that provides a unified interface for interacting with various AI providers, including Anthropic's Claude and OpenAI's GPT models.

## Features

- 🤖 Unified interface for multiple AI providers
- 🔑 Secure API key management
- 🖥️ Command-line interface (CLI)
- 📡 Async communication with AI providers
- 📝 Support for both single messages and streaming responses
- 🛡️ Robust error handling

## Installation

### Prerequisites

- Rust 1.82.0 or higher
- Cargo package manager

### Building from source

```bash
git clone https://github.com/thevuuranusls/aegis.git
cd aegis
cargo test
cargo build --release
```

## Usage

### CLI Tool

1. Configure your API keys:

```bash
aegis-cli config
```

2. Send a message:
```bash
aegis-cli chat --provider anthropic --content "What is the capital of Vietnam?"
```

### Library Usage

```rust
use aegis::{Aegis, AegisConfig, Message};
use aegis::models::ProviderType::Anthropic;

#[tokio::main]
async fn main() {
    // Initialize with API key
    let config = AegisConfig::new()
        .with_anthropic("your-api-key".to_string());
    let aegis = Aegis::new(config);

    // Create a message
    let messages = vec![Message {
        role: "user".to_string(),
        content: "What is the capital of Vietnam?".to_string(),
    }];

    // Send message and handle response
    match aegis.send_message(Anthropic, messages).await {
        Ok(response) => println!("Response: {}", response),
        Err(e) => eprintln!("Error: {}", e),
    }
}
```

## Configuration

API keys can be configured in two ways:
1. Environment variables:
   - `ANTHROPIC_API_KEY`
   - `OPENAI_API_KEY`
2. Using the CLI configuration tool

## Supported Providers

- [x] Anthropic (Claude)
- [ ] OpenAI (GPT models) - Coming soon
- [ ] More providers planned

### Running Tests

```bash
cargo test
```

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

1. Fork the repository
2. Create your feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add some amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

## License

This project is licensed under the MIT License - see the LICENSE file for details.

## Acknowledgments

- Built with Rust 🦀
- Powered by Anthropic's Claude and OpenAI's GPT models
```