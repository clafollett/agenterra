# Agenterra Configuration ⚙️

This guide explains how to configure Agenterra using different methods.

## Table of Contents
- [Configuration Methods](#configuration-methods)
- [Command-Line Options](#command-line-options)
- [Environment Variables](#environment-variables)
- [Example Configurations](#example-configurations)

## Configuration Methods

Agenterra can be configured using the following methods (in order of precedence):

1. **Command-Line Arguments** (highest priority)
2. **Environment Variables**
3. **Default Values** (lowest priority)

## Command-Line Options

### Global Options

```bash
agenterra [OPTIONS] <SUBCOMMAND>
```

| Option | Description | Default |
|--------|-------------|---------|
| `-h`, `--help` | Print help | |
| `-V`, `--version` | Print version | |

### Scaffold MCP Server

```bash
agenterra scaffold mcp server --schema-path <SCHEMA_PATH> [OPTIONS]
```

| Option | Description | Default |
|--------|-------------|---------|
| `--schema-path <SCHEMA_PATH>` | Path or URL to OpenAPI schema (YAML or JSON) | *required* |
| `--project-name <PROJECT_NAME>` | Project name | `agenterra_mcp_server` |
| `--template <TEMPLATE>` | Template to use for code generation | `rust_axum` |
| `--template-dir <TEMPLATE_DIR>` | Custom template directory | |
| `--output-dir <OUTPUT_DIR>` | Output directory for generated code | |
| `--log-file <LOG_FILE>` | Log file name without extension | `mcp-server` |
| `--port <PORT>` | Server port | `3000` |
| `--base-url <BASE_URL>` | Base URL of the OpenAPI specification | |

### Scaffold MCP Client

```bash
agenterra scaffold mcp client --project-name <PROJECT_NAME> [OPTIONS]
```

| Option | Description | Default |
|--------|-------------|---------|
| `--project-name <PROJECT_NAME>` | Project name | `agenterra_mcp_client` |
| `--template <TEMPLATE>` | Template to use for code generation | `rust_reqwest` |
| `--template-dir <TEMPLATE_DIR>` | Custom template directory | |
| `--output-dir <OUTPUT_DIR>` | Output directory for generated code | |
| `--timeout <TIMEOUT>` | Connection timeout in seconds | `10` |


## Environment Variables

Configuration options can be set via environment variables with the `AGENTERRA_` prefix:

### Server Environment Variables

```bash
# Basic options
export AGENTERRA_SCHEMA_PATH=openapi.yaml
export AGENTERRA_OUTPUT_DIR=generated-server
export AGENTERRA_PROJECT_NAME=my_api_server

# Template options
export AGENTERRA_TEMPLATE=rust_axum
export AGENTERRA_TEMPLATE_DIR=./custom-templates/server

# Server options
export AGENTERRA_PORT=8080
export AGENTERRA_BASE_URL=https://api.example.com
export AGENTERRA_LOG_FILE=my-server
```

### Client Environment Variables

```bash
# Basic options
export AGENTERRA_OUTPUT_DIR=generated-client
export AGENTERRA_PROJECT_NAME=my_api_client

# Template options
export AGENTERRA_TEMPLATE=rust_reqwest
export AGENTERRA_TEMPLATE_DIR=./custom-templates/client

# Client options
export AGENTERRA_TIMEOUT=30
```

## Example Configurations

### Server Generation Example

```bash
# Basic server generation
agenterra scaffold mcp server --schema-path api/openapi.yaml

# Full server configuration with all options
agenterra scaffold mcp server \
  --schema-path api/openapi.yaml \
  --project-name petstore_mcp_server \
  --template rust_axum \
  --output-dir petstore-server \
  --log-file petstore-server \
  --port 3000 \
  --base-url https://petstore3.swagger.io
```

### Client Generation Example

```bash
# Basic client generation
agenterra scaffold mcp client --project-name my-client

# Full client configuration with all options
agenterra scaffold mcp client \
  --project-name petstore_mcp_client \
  --template rust_reqwest \
  --output-dir petstore-client \
  --timeout 30
```

### Environment Variables Example

```bash
# Set default output directory for all generations
export AGENTERRA_OUTPUT_DIR=~/my-projects

# Set custom template directory for development
export AGENTERRA_TEMPLATE_DIR=~/my-custom-templates

# Server generation using environment variables
export AGENTERRA_SCHEMA_PATH=api/openapi.yaml
export AGENTERRA_PROJECT_NAME=my_api_server
export AGENTERRA_TEMPLATE=rust_axum
export AGENTERRA_PORT=3000
export AGENTERRA_BASE_URL=https://api.example.com

agenterra scaffold mcp server

# Client generation using environment variables
export AGENTERRA_PROJECT_NAME=my_api_client
export AGENTERRA_TEMPLATE=rust_reqwest
export AGENTERRA_TIMEOUT=30

agenterra scaffold mcp client
```

## Configuration Precedence

1. Command-line arguments (highest priority)
2. Environment variables
3. Default values (lowest priority)


## Next Steps

- [CLI Reference](CLI_REFERENCE.md)
- [Templates Documentation](TEMPLATES.md)
- [Contributing Guide](../CONTRIBUTING.md)