# Agenterra Configuration ⚙️

This guide explains how to configure Agenterra using different methods.

## Table of Contents
- [Configuration Methods](#configuration-methods)
- [Command-Line Options](#command-line-options)
- [Configuration File](#configuration-file)
- [Environment Variables](#environment-variables)
- [Example Configurations](#example-configurations)

## Configuration Methods

Agenterra can be configured using the following methods (in order of precedence):

1. **Command-Line Arguments** (highest priority)
2. **Configuration File** (`agenterra.toml` in project root)
3. **Environment Variables**
4. **Default Values** (lowest priority)

## Command-Line Options

### Global Options

```bash
agenterra [OPTIONS] <SUBCOMMAND>
```

| Option | Description | Default |
|--------|-------------|---------|
| `-h`, `--help` | Print help | |
| `-V`, `--version` | Print version | |

### Scaffold Command

```bash
agenterra scaffold --schema-path <SCHEMA_PATH> [OPTIONS]
```

| Option | Description | Default |
|--------|-------------|---------|
| `--schema-path <SCHEMA_PATH>` | Path or URL to OpenAPI schema (YAML or JSON) | *required* |
| `--project-name <PROJECT_NAME>` | Project name | `agenterra_mcp_server` |
| `--template-kind <TEMPLATE_KIND>` | Template to use for code generation | `rust_axum` |
| `--template-dir <TEMPLATE_DIR>` | Custom template directory | |
| `--output-dir <OUTPUT_DIR>` | Output directory for generated code | |
| `--log-file <LOG_FILE>` | Log file name without extension | `mcp-server` |
| `--port <PORT>` | Server port | `3000` |
| `--base-url <BASE_URL>` | Base URL of the OpenAPI specification | |

## Configuration File

Create a `agenterra.toml` file in your project root:

```toml
[scaffold]
schema_path = "openapi.yaml"
project_name = "my_api_server"
template_kind = "rust_axum"
output_dir = "generated"
log_file = "my-server"
port = 3000
base_url = "https://api.example.com"

# Custom template directory (optional)
template_dir = "./custom-templates"
```

## Environment Variables

Configuration options can be set via environment variables with the `AGENTERRA_` prefix:

```bash
# Basic options
export AGENTERRA_SCHEMA_PATH=openapi.yaml
export AGENTERRA_OUTPUT_DIR=generated
export AGENTERRA_PROJECT_NAME=my_api_server

# Template options
export AGENTERRA_TEMPLATE_KIND=rust_axum
export AGENTERRA_TEMPLATE_DIR=./custom-templates

# Server options
export AGENTERRA_PORT=8080
export AGENTERRA_BASE_URL=https://api.example.com
export AGENTERRA_LOG_FILE=my-server
```

## Example Configurations

### Minimal Configuration

```toml
[scaffold]
schema_path = "api/openapi.yaml"
output_dir = "generated"
```

### Full Configuration

```toml
[scaffold]
schema_path = "api/openapi.yaml"
project_name = "petstore_mcp_server"
template_kind = "rust_axum"
output_dir = "generated"
log_file = "petstore-server"
port = 3000
base_url = "https://petstore3.swagger.io"
```

### Environment Variables Example

```bash
# .env file
AGENTERRA_SCHEMA_PATH=api/openapi.yaml
AGENTERRA_OUTPUT_DIR=generated
AGENTERRA_PROJECT_NAME=my_api_server
AGENTERRA_TEMPLATE_KIND=rust_axum
AGENTERRA_PORT=3000
AGENTERRA_BASE_URL=https://api.example.com
```

## Configuration Precedence

1. Command-line arguments (highest priority)
2. Environment variables
3. Configuration file (`agenterra.toml`)
4. Default values (lowest priority)

## Next Steps

- [Templates Documentation](TEMPLATES.md)
- [CLI Reference](CLI_REFERENCE.md)
- [Contributing Guide](../CONTRIBUTING.md)