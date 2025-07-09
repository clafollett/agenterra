# Protocols Domain Module

This module implements the protocol abstraction layer for Agenterra, following Domain-Driven Design principles.

## Architecture

The protocols module provides a clean abstraction over different communication protocols:
- **MCP** (Model Context Protocol): For server/client communication with OpenAPI requirements
- **A2A** (Agent-to-Agent): For inter-agent communication (not yet implemented)
- **ACP** (Agent Communication Protocol): For broker-based messaging (not yet implemented)
- **ANP** (Agent Notification Protocol): For simple agent notifications (not yet implemented)

## Core Components

### Types (`types.rs`)
- `Protocol`: Enum representing supported protocols
- `Role`: Enum representing participant roles (Server, Client, Agent, Broker)
- `ProtocolCapabilities`: Structure describing protocol features

### Registry (`registry.rs`)
- `ProtocolRegistry`: Thread-safe registry for protocol handlers
- Supports dynamic registration and retrieval of protocol implementations
- `with_defaults()`: Creates a registry with MCP handler pre-registered

### Traits (`traits.rs`)
- `ProtocolHandler`: Core trait that all protocol implementations must satisfy
- `ProtocolInput`: Input data for protocol processing
- `ProtocolConfig`: Configuration for protocol behavior
- `GenerationContext`: Output context from protocol preparation

### Handlers (`handlers/`)
- `McpProtocolHandler`: Implementation for Model Context Protocol
  - Validates OpenAPI requirement for server role
  - Builds MCP-specific context variables
  - Supports stdio, http, and websocket transports

### Errors (`errors.rs`)
- `ProtocolError`: Domain-specific errors for protocol operations

## Usage

```rust
use agenterra::protocols::{Protocol, ProtocolRegistry, ProtocolInput, ProtocolConfig, Role};
use serde_json::json;
use std::path::PathBuf;

// Create registry with default handlers
let registry = ProtocolRegistry::with_defaults();

// Get MCP handler
if let Some(handler) = registry.get(Protocol::Mcp) {
    // Prepare input
    let input = ProtocolInput {
        openapi_path: Some(PathBuf::from("api.yaml")),
        config: ProtocolConfig {
            name: "my-server".to_string(),
            version: Some("1.0.0".to_string()),
            settings: json!({
                "transport": "http",
                "port": 8080
            }),
        },
        role: Role::Server,
    };
    
    // Generate context
    let context = handler.prepare_context(input).await?;
}
```

## MCP Protocol Details

The MCP handler provides:
- **Server Role**: Requires OpenAPI specification, supports all MCP features
- **Client Role**: No OpenAPI required, direct connection type
- **Transports**: stdio (default), http, websocket
- **Features**: tools, resources, prompts, sampling (configurable)

## Design Decisions

1. **Protocol as First-Class Citizen**: Each protocol is a distinct type with its own capabilities
2. **Role-Based Validation**: Protocols enforce which roles they support
3. **Registry Pattern**: Allows runtime registration of protocol implementations
4. **Thread-Safe Design**: Registry uses Arc<RwLock<>> for concurrent access
5. **Trait-Based Abstraction**: ProtocolHandler trait enables polymorphic behavior
6. **Focused Implementation**: Currently only MCP is implemented, with clean extension points for future protocols