# 🚀 MCPGen: Model Context Protocol Generator

**Generate production-ready MCP (Model Context Protocol) servers from OpenAPI specs with minimal configuration.**

[![GitHub release (latest by date)](https://img.shields.io/github/v/release/clafollett/mcpgen?style=for-the-badge)](https://github.com/clafollett/mcpgen/releases)
[![Rust](https://img.shields.io/badge/Rust-1.86.0%2B-orange?logo=rust&style=for-the-badge)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue?style=for-the-badge)](https://github.com/clafollett/mcpgen)
[![OpenAPI](https://img.shields.io/badge/OpenAPI-3.0-85ea2d?logo=openapi-initiative&style=for-the-badge)](https://www.openapis.org/)

---

**MCPGen** transforms your OpenAPI specifications into fully-functional MCP servers with type-safe Rust code, ready for integration with AI tools and workflows. Perfect for:

- **AI/ML Engineers** 🤖 - Quickly expose APIs for LLM tool use
- **API Developers** 🛠️ - Generate production-ready MCP servers from existing OpenAPI specs
- **FinTech & Data Teams** 📊 - Build compliant financial data APIs with built-in validation
- **Startups & Enterprises** 🚀 - Accelerate development of AI-powered applications

---

## ✨ Features

- **⚡ Blazing Fast** - Built with Rust for maximum performance and safety
- **🔌 OpenAPI 3.0+ Support** - Seamless integration with existing API specifications
- **🦀 Type-Safe Rust** - Generate idiomatic, production-ready Rust code
- **🎨 Template-Based** - Customize every aspect with Tera templates
- **🔍 Built-in Validation** - Automatic OpenAPI schema validation
- **🚀 Production Ready** - Includes logging, error handling, and configuration out of the box
- **🔌 MCP Protocol Support** - Full compatibility with Model Context Protocol
- **📦 Docker & Binary** - Multiple deployment options for any environment

---

## 🚀 Quick Start

### Prerequisites

- [Rust 1.86.0+](https://rustup.rs/)
- [Docker](https://www.docker.com/) (optional, for containerized deployment)

### Method 1: Using Docker (Recommended)

```bash
# Use a local OpenAPI spec file
docker run -p 3000:3000 -v $(pwd):/app ghcr.io/clafollett/mcpgen:latest scaffold --spec /app/your-api.yaml

# Or use a remote OpenAPI spec URL
docker run -p 3000:3000 ghcr.io/clafollett/mcpgen:latest scaffold --spec https://example.com/openapi.json
```

### Method 2: From Pre-built Binary

1. Download the latest release for your platform from [Releases](https://github.com/clafollett/mcpgen/releases)
2. Make it executable and run:
   ```bash
   chmod +x mcpgen
   ./mcpgen scaffold --spec your-api.yaml
   ```

### Method 3: Build from Source

```bash
# Clone the repository
git clone https://github.com/clafollett/mcpgen.git
cd mcpgen

# Build and install
cargo install --path .

# Generate your MCP server from a local file
mcpgen scaffold --spec examples/petstore.yaml --output my-server

# Or generate from a remote URL
mcpgen scaffold --spec https://example.com/openapi.json --output my-server
```

---

## 🏗️ Generate Your First MCP Server

1. **Prepare Your OpenAPI Spec**
   ```bash
   # Option 1: Use a local file
   curl -o petstore.yaml https://raw.githubusercontent.com/OAI/OpenAPI-Specification/main/examples/v3.0/petstore.yaml
   
   # Option 2: Use a remote URL directly
   # (No download needed, MCPGen can fetch it directly)
   ```

2. **Generate the Server**
   
   Using a local file:
   ```bash
   mcpgen scaffold --spec petstore.yaml --output my-server
   ```
   
   Or using a remote URL:
   ```bash
   mcpgen scaffold --spec https://raw.githubusercontent.com/OAI/OpenAPI-Specification/main/examples/v3.0/petstore.yaml --output my-server
   ```
   ```bash
   mcpgen scaffold --spec petstore.yaml --output petstore-server
   ```

3. **Run the Server (STDIO Mode for MCP)**
   ```bash
   cd petstore-server
   cargo run
   ```

4. **Test the API**
   ```bash
   # List all pets
   curl http://localhost:3000/pets
   
   # Get pet by ID
   curl http://localhost:3000/pets/1
   ```

5. **Access MCP Endpoint**
   ```bash
   # MCP endpoint for AI tool integration
   curl -X POST http://localhost:3000/mcp \
     -H "Content-Type: application/json" \
     -d '{"method": "listPets", "params": {}}'
   ```

---

## 🔌 Integrating with MCP Clients

### VS Code Integration

Add this to your VS Code settings (File > Preferences > Settings > Open Settings JSON):

```json
{
  "mcp": {
    "servers": {
      "petstore": {
        "command": "cargo",
        "args": ["run", "--manifest-path=/path/to/petstore-server/Cargo.toml"],
        "cwd": "/path/to/petstore-server"
      }
    }
  }
}
```

### Cursor Integration

Add this to your Cursor settings (File > Preferences > Settings > Extensions > MCP):

```json
{
  "mcpServers": {
    "petstore": {
      "command": "cargo",
      "args": ["run", "--manifest-path=/path/to/petstore-server/Cargo.toml"],
      "cwd": "/path/to/petstore-server"
    }
  }
}
```

### 🕵️‍♂️ Testing with MCP Inspector

Test your MCP server with the MCP Inspector:

```bash
# Run directly with npx
npx @modelcontextprotocol/inspector cargo run --manifest-path=/path/to/petstore-server/Cargo.toml

# Or install globally
npm install -g @modelcontextprotocol/inspector
modelcontextprotocol-inspector cargo run --manifest-path=/path/to/petstore-server/Cargo.toml
```

### 🐳 Docker Integration

For production use, build and run with Docker:

```bash
# Build the image
cd petstore-server
docker build -t petstore-mcp .

# Run the container
docker run -i --rm petstore-mcp
```

Then update your MCP client configuration to use the Docker container:

```json
{
  "mcp": {
    "servers": {
      "petstore": {
        "command": "docker",
        "args": ["run", "-i", "--rm", "petstore-mcp"]
      }
    }
  }
}
```

---

## 🏗️ Project Structure

```
petstore-server/
├── Cargo.toml          # Rust project manifest
├── src/
│   ├── mcp/            # MCP protocol implementation
│   │   ├── mod.rs       # MCP server implementation
│   │   └── handlers/    # MCP request handlers
│   ├── api/             # Generated API code
│   │   ├── mod.rs       # API module exports
│   │   ├── models/      # Generated data models
│   │   └── operations/  # API operation handlers
│   ├── config.rs        # Server configuration
│   ├── error.rs         # Error handling
│   └── main.rs          # MCP server entry point
├── .env                # Environment variables
└── README.md           # Project documentation
```

## 📚 Documentation

For detailed documentation, check out:

- [CLI Reference](docs/CLI_REFERENCE.md) - Complete command-line options
- [Configuration Guide](docs/CONFIGURATION.md) - Customizing your MCP server
- [Template System](docs/TEMPLATES.md) - Creating custom templates
- [MCP Protocol](https://modelcontextprotocol.io) - Learn about Model Context Protocol

---

## 🤝 Contributing

Contributions are welcome! Please read our [Contributing Guide](CONTRIBUTING.md) for details on our code of conduct and the process for submitting pull requests.

```bash
# Clone the repository
git clone https://github.com/clafollett/mcpgen.git

# Build in development mode
cargo build

# Run tests
cargo test

# Run lints
cargo clippy
```

## 📄 License

This project is licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT license ([LICENSE-MIT](LICENSE-MIT))

at your option.

---

## 🙌 Acknowledgments

- Built with ❤️ and [Rust](https://www.rust-lang.org/)
- Inspired by the [Model Context Protocol](https://modelcontextprotocol.io/)
- Uses [axum](https://github.com/tokio-rs/axum) for the web server
- Powered by [clap](https://github.com/clap-rs/clap) for CLI parsing
│   └── main.rs          # Server entry point
└── templates/           # Custom templates (optional)
```

## Configuration ⚙️

MCPGen can be configured through multiple methods (in order of precedence):

1. **Command-line arguments**
   ```bash
   mcpgen generate --input spec.yaml --output my_server --template rust-axum
   ```

2. **Configuration file** (`mcpgen.toml` in project root)
   ```toml
   [generate]
   input = "openapi.yaml"
   output = "my_server"
   template = "rust-axum"
   ```

3. **Environment variables**
   ```bash
   export MCPGEN_INPUT=openapi.yaml
   export MCPGEN_OUTPUT=my_server
   mcpgen generate
   ```

## Templates 🎨

MCPGen uses [Tera](https://tera.netlify.app/) templates for code generation. You can use built-in templates or create your own.

### Built-in Templates
- `rust-axum`: Generate a server using the [Axum](https://github.com/tokio-rs/axum) web framework

### Custom Templates
Create a `templates` directory in your project root and add your template files. MCPGen will use these instead of the built-in templates.

## Documentation 📚

For detailed documentation, please refer to:

- [Configuration Guide](docs/CONFIGURATION.md) - Complete reference for configuring MCPGen
- [CLI Reference](docs/CLI_REFERENCE.md) - Detailed documentation of all commands and options
- [Templates](docs/TEMPLATES.md) - Guide to creating and customizing templates
- [Contributing](CONTRIBUTING.md) - How to contribute to the project

## Examples 📚

### Generate a server from Petstore API
```bash
# Download the Petstore OpenAPI spec
curl -o petstore.yaml https://raw.githubusercontent.com/OAI/OpenAPI-Specification/main/examples/v3.0/petstore.yaml

# Generate the server
mcpgen generate --input petstore.yaml --output petstore-server

# Build and run
cd petstore-server
cargo run
```

## Contributing 🤝

Contributions are welcome! Please read our [Contributing Guide](CONTRIBUTING.md) for details on our code of conduct and the process for submitting pull requests.

## License 📄

This project is dual-licensed under either:

- MIT License ([LICENSE-MIT](LICENSE-MIT))
- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))

at your option.

## Related Projects 🔗

- [RMCP](https://github.com/windsurf-eng/rmcp) - Rust MCP implementation
- [MCP Protocol](https://github.com/windsurf-eng/mcp) - Model Context Protocol specification
- [Axum](https://github.com/tokio-rs/axum) - Web framework for Rust
- [Tera](https://tera.netlify.app/) - Template engine for Rust
