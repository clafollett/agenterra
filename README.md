# 🚀 Agenterra: Model Context Protocol Generator

**Generate production-ready MCP (Model Context Protocol) servers and clients from OpenAPI specs with minimal configuration.**

[![Crates.io](https://img.shields.io/crates/v/agenterra?style=for-the-badge)](https://crates.io/crates/agenterra)
[![CI](https://github.com/clafollett/agenterra/workflows/CI/badge.svg)](https://github.com/clafollett/agenterra/actions/workflows/ci.yml)
[![Rust](https://img.shields.io/badge/Rust-1.86.0%2B-orange?logo=rust&style=for-the-badge)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/license-MIT-blue?style=for-the-badge)](LICENSE)

---

**Agenterra** transforms your OpenAPI specifications into fully-functional MCP servers and clients with type-safe Rust code, ready for integration with AI tools and workflows. Perfect for:

- **AI/ML Engineers** 🤖 - Quickly expose APIs for LLM tool use
- **API Developers** 🛠️ - Generate production-ready MCP servers from existing OpenAPI specs
- **FinTech & Data Teams** 📊 - Build compliant financial data APIs with built-in validation
- **Startups & Enterprises** 🚀 - Accelerate development of AI-powered applications

## ✨ Features

- **⚡ Blazing Fast** - Built with Rust for maximum performance and safety
- **🔌 OpenAPI 3.0+ Support** - Seamless integration with existing API specifications
- **🦀 Type-Safe Rust** - Generate idiomatic, production-ready Rust code
- **🎨 Template-Based** - Customize every aspect with Tera templates
- **🔍 Built-in Validation** - Automatic OpenAPI schema validation
- **🚀 Production Ready** - Includes logging, error handling, and configuration out of the box
- **🔌 MCP Protocol Support** - Full compatibility with Model Context Protocol
- **💾 SQLite Resource Caching** - Built-in resource caching with connection pooling for MCP clients
- **🛡️ Client Permission Management** - Interactive tool permission prompting with persistent preferences
- **📦 Binary Distribution** - Easy installation and deployment

## 🔒 Enterprise Security

Agenterra generates code with enterprise-grade security features built-in. Every generated server and client includes comprehensive protection against modern attack vectors.

**Key Security Features:**
- **Input Validation**: Protection against SQL injection, command injection, and prompt injection
- **Unicode Security**: Detection of zero-width characters and emoji-based attacks
- **Transport Security**: Secure SSE mode with URL validation
- **Resource Protection**: Size limits and rate limiting to prevent DoS

See [Enterprise Security Features](docs/ENTERPRISE_SECURITY.md) for complete details.

## 🚀 Quick Start

### Prerequisites

- [Rust 1.86.0+](https://rustup.rs/)

### Method 1: Build & Run from Source

```bash
# Clone the repository
git clone https://github.com/clafollett/agenterra.git
cd agenterra

# Generate MCP server from a local file without install:
cargo run -- scaffold mcp server --schema-path ./tests/fixtures/openapi/petstore.openapi.v3.json --project-name petstore-server --base-url https://petstore3.swagger.io

# Generate MCP server from a remote URL without install:
cargo run -- scaffold mcp server --schema-path https://petstore3.swagger.io/api/v3/openapi.json --project-name petstore-remote

# Generate MCP client without install:
cargo run -- scaffold mcp client --project-name petstore-client

# Or install the CLI
cargo install --path .

# Generate your MCP server from a local file
agenterra scaffold mcp server --schema-path ./tests/fixtures/openapi/petstore.openapi.v3.json --project-name petstore-server --base-url https://petstore3.swagger.io

# Generate MCP server from a remote URL
agenterra scaffold mcp server --schema-path https://petstore3.swagger.io/api/v3/openapi.json --project-name petstore-remote

# Generate MCP client
agenterra scaffold mcp client --project-name petstore-client

```

> **Note:** After the single-crate refactor, you can now install directly from the project root with `cargo install --path .`

### Method 2: Install from Git

```bash
# Install latest version
cargo install --git https://github.com/clafollett/agenterra.git agenterra

# Install specific version. Example: v0.1.0
cargo install --git https://github.com/clafollett/agenterra.git --tag v<VERSION> agenterra
```

### Method 3: From Pre-built Binary (Coming soon)

1. Download the latest release for your platform from [Releases](https://github.com/clafollett/agenterra/releases)
2. Make it executable and run:
   ```bash
   chmod +x agenterra
   
   # Generate your MCP server from a local file
   ./agenterra scaffold mcp server --schema-path ./tests/fixtures/openapi/petstore.openapi.v3.json --project-name petstore-server --base-url https://petstore3.swagger.io

   # Generate MCP server from a remote URL
   ./agenterra scaffold mcp server --schema-path https://petstore3.swagger.io/api/v3/openapi.json --project-name petstore-remote
   
   # Generate MCP client
   ./agenterra scaffold mcp client --project-name petstore-client
   ```

## 🔌 Integrating with MCP Clients

### VS Code Integration

Add this to your VS Code settings (File > Preferences > Settings > Open Settings JSON):

```json
{
  "mcp": {
    "servers": {
      "petstore": {
        "command": "cargo",
        "args": ["run", "--manifest-path", "/path/to/petstore-server/Cargo.toml"]
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
      "args": ["run", "--manifest-path", "/path/to/petstore-server/Cargo.toml"],
      "disabled": false,
      "alwaysAllowed": ["listPets", "showPetById"],
      "disabledTools": ["deletePet"]
    }
  }
}
```

### 🕵️‍♂️ Testing with MCP Inspector

Test your MCP server with the MCP Inspector:

```bash
# Test STDIO mode (default)
npx @modelcontextprotocol/inspector cargo run --manifest-path=/path/to/petstore-server/Cargo.toml

# Test SSE mode
npx @modelcontextprotocol/inspector cargo run --manifest-path=/path/to/petstore-server/Cargo.toml -- --transport sse

# Or install globally
npm install -g @modelcontextprotocol/inspector
modelcontextprotocol-inspector cargo run --manifest-path=/path/to/petstore-server/Cargo.toml
```

#### Testing SSE Endpoints

When running in SSE mode, the server exposes HTTP endpoints:

```bash
# Start server in SSE mode
cargo run -- --transport sse --sse-addr 127.0.0.1:8080

# Test SSE endpoint with curl
curl -N -H "Accept: text/event-stream" http://localhost:8080/sse

# Send MCP messages via POST
curl -X POST http://localhost:8080/message \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"tools/list","id":1}'
```


## ⚡ Best Practices

Agenterra is designed to scaffold a well-structured MCP servers from OpenAPI specs. This is a great starting point, not necessarily a `Best Practice`. Wrapping an OpenAPI spec under an MCP facade is convenient, but not always the “proper” way to build MCPs. For robust, agent-friendly tools, consider how your server can best expose business logic, aggregate data, and provide clear, useful tool contracts.

**Considerations:**
- Treat the generated code as a foundation to extend and customize.
- Don't assume a 1:1 mapping of OpenAPI endpoints to MCP tools is ideal; you may want to aggregate multiple API calls into a single tool, or refactor handlers for advanced logic.
- Use the scaffold to rapidly stub out endpoints, then iterate and enhance as needed.

---

## 🤔 Why Agenterra?

Postman now offers robust support for the Model Context Protocol (MCP), including:
- MCP client and server features
- Code generation
- A catalog of hosted, discoverable MCP endpoints
- Visual agent-building and cloud collaboration

**When should you use Agenterra?**
- **Offline, air-gapped, or regulated environments** where cloud-based tools aren’t an option
- **Rust-first, codegen-centric workflows:** Generate type-safe, production-grade Rust MCP servers from OpenAPI specs, ready for CI/CD and self-hosting
- **Full template control:** Tweak every line of generated code, use custom templates, and integrate with your own infra
- **CLI-first automation:** Perfect for embedding in build scripts and automated workflows

**When should you use Postman?**
- Visual design, rapid prototyping, and cloud collaboration
- Building, testing, and deploying MCP agents with a GUI
- Discovering and consuming public MCP endpoints

**Summary:**
- Use Postman for visual, collaborative, and cloud-first agent development
- Use Agenterra for local, reproducible, code-first MCP server generation with maximum control and zero cloud dependencies

---

## 🏛️ Architecture

Agenterra is built for extensibility, automation, and code quality. Here’s how the core pieces fit together:

**Core Modules:**
- `openapi`: Loads and validates OpenAPI specs (YAML/JSON, local or URL)
- `generator`: Orchestrates code generation from the parsed OpenAPI model
- `template`: Handles Tera-based templates for idiomatic Rust code
- `cli`: Command-line interface for scaffolding, configuration, and workflow

**Code Generation Flow:**

```
OpenAPI Spec (local file or URL)
         │
         ▼
   [openapi module]
         │
         ▼
   [generator module]
         │
         ▼
   [template module]
         │
         ▼
Generated Rust MCP Server (Axum, etc.)
```

- The generated servers support both **STDIO** and **SSE (Server-Sent Events)** transports for MCP protocol
- All code is idiomatic Rust, ready for further customization and production deployment

### 🚀 Transport Configuration

Generated servers and clients support multiple transport modes:

#### Server Transport Options
```bash
# STDIO mode (default) - for direct process communication
./my-server

# SSE mode - for HTTP-based communication
./my-server --transport sse --sse-addr 127.0.0.1:8080

# With custom keep-alive interval
./my-server --transport sse --sse-addr 0.0.0.0:9000 --sse-keep-alive 60

# Configuration via command-line arguments only
./my-server --transport sse --sse-addr 127.0.0.1:8080
```

#### Client Transport Options
```bash
# STDIO mode (default) - connects to server process
./my-client --server /path/to/server

# SSE mode - connects to HTTP endpoint
./my-client --transport sse --sse-url http://localhost:8080
```

## 💾 Resource Caching

Generated MCP clients include a sophisticated SQLite-powered resource caching system:

**Features:**
- **Connection Pooling** - r2d2 connection pool for concurrent access
- **Character Encoding** - Configurable charset support for resource content
- **TTL Support** - Configurable time-to-live for cache entries
- **Analytics** - Built-in cache hit/miss tracking and performance metrics
- **ACID Transactions** - Database integrity with rollback support
- **Auto-cleanup** - Configurable expired resource cleanup

**Configuration Options:**
```rust
// Resource cache configuration
let cache_config = CacheConfig {
    default_ttl: Duration::from_secs(3600),
    max_size_mb: 100,
    auto_cleanup: true,
};

// Database connection configuration (separate from cache config)
let db_config = DatabaseConfig {
    database_path: PathBuf::from("cache.db"),
    pool_max_connections: 10,
    pool_connection_timeout: Some(Duration::from_secs(5)),
    pool_max_lifetime: Some(Duration::from_secs(300)),
};
```

---

## 🤝 Contributing

We welcome contributions from the community! To keep Agenterra high-quality and maintainable, please follow these guidelines:

- **Fork & Clone**: Fork the repo and clone your fork locally.
- **Branch Naming**: Use the convention `<type>/issue-<number>/<description>` (e.g., `docs/issue-57/update-readme`).
- **Pull Requests**:
  - All PRs require review.
  - All tests must pass (`cargo test` and integration tests).
  - Code coverage must not decrease.
  - Update documentation for any user-facing or API changes.
- **Testing**:
  - Add or update unit and integration tests for all new features or bugfixes.
  - Run: `cargo test --test e2e_mcp_test`
- **Docs**:
  - Update relevant docs and add examples for new features.
  - Document any new patterns or conventions.
- **CI/CD**:
  - Ensure your branch passes all checks before requesting review.

For more details, see [CONTRIBUTING.md](CONTRIBUTING.md) if available.

---

## 🛠️ Developer Workflow

Here’s how to work productively with Agenterra as a contributor or advanced user:

### 🧪 Running Tests
- **Unit & Integration Tests:**
  - Run all tests: `cargo test`
  - Run integration tests (all templates with OpenAPI specs):
    ```bash
    cargo test --test e2e_mcp_test
    ```
- **Test Location:** See [`tests/e2e_mcp_test.rs`](tests/e2e_mcp_test.rs) for integration coverage.
- **Test-First Principle:** Add failing tests before implementing new features or bugfixes.

### 🏗️ Building
- **Standard build:**
  ```bash
  cargo build --release
  ```

### 🧩 Custom Templates
- See [`docs/TEMPLATES.md`](docs/TEMPLATES.md) for template development
- Add templates under `templates/` directory

---

## 📚 Examples & Configuration

### Basic Example: Petstore API
```bash
# Download the Petstore OpenAPI spec
curl -o petstore.json https://petstore3.swagger.io/api/v3/openapi.json

# Generate the MCP server
agenterra scaffold mcp server --schema-path petstore.json --project-name petstore-server

# Generate the MCP client
agenterra scaffold mcp client --project-name petstore-client

# Build and run the server
cd petstore-server
cargo run
```

### Configuration Options

Agenterra is configured through command-line arguments. By default, projects are created in the current directory (like `cargo new`):

```bash
# Generate MCP server (creates ./my_server/)
agenterra scaffold mcp server --schema-path your_api.json --project-name my_server

# Generate MCP client (creates ./my_client/)
agenterra scaffold mcp client --project-name my_client

# Specify a parent directory with --output-dir
agenterra scaffold mcp server --schema-path api.json --project-name my_server --output-dir ~/projects
# Creates: ~/projects/my_server/
```

**Environment Variables:**
- `AGENTERRA_OUTPUT_DIR` - Default parent directory for generated projects
- `AGENTERRA_TEMPLATE_DIR` - Custom template directory location

### Templates

Agenterra uses [Tera](https://tera.netlify.app/) templates for code generation.

**Built-in Server Templates:**
- `rust_axum` - Rust MCP server using Axum web framework

**Built-in Client Templates:**
- `rust_reqwest` - Rust MCP client with REPL interface and SQLite resource caching

**Custom Templates:**
- Create templates under `templates/mcp/server/` or `templates/mcp/client/`
- **Details**: See [`docs/TEMPLATES.md`](docs/TEMPLATES.md)

**Project Structure:**
Each generated project includes a detailed README.md with template-specific project structure documentation, usage examples, and configuration options.

## 📄 License

MIT License - see [LICENSE](LICENSE) for details.

## 🔗 Related Projects

- [MCP Protocol](https://github.com/windsurf-eng/mcp) - Model Context Protocol specification
- [RMCP](https://github.com/windsurf-eng/rmcp) - Rust MCP implementation  
- [Axum](https://github.com/tokio-rs/axum) - Web framework for Rust
- [Tera](https://tera.netlify.app/) - Template engine for Rust
