# Agenterra CLI Reference 📝

This document provides a comprehensive reference for the Agenterra command-line interface.

## Table of Contents
- [Global Options](#global-options)
- [Commands](#commands)
  - [scaffold](#scaffold)
- [Examples](#examples)
- [Exit Codes](#exit-codes)
- [Environment Variables](#environment-variables)

## Global Options

| Option | Description |
|--------|-------------|
| `-h`, `--help` | Print help |
| `-V`, `--version` | Print version |

## Commands

### scaffold

Scaffold a new MCP server from an OpenAPI specification.

```bash
agenterra scaffold --schema-path <SCHEMA_PATH> [OPTIONS]
```

#### Options

| Option | Description | Default |
|--------|-------------|---------|
| `--schema-path <SCHEMA_PATH>` | Path or URL to OpenAPI schema (YAML or JSON). Can be a local file path or an HTTP/HTTPS URL. | *required* |
| `--project-name <PROJECT_NAME>` | Project name | `agenterra_mcp_server` |
| `--template-kind <TEMPLATE_KIND>` | Template to use for code generation (e.g., rust_axum, python_fastapi) | `rust_axum` |
| `--template-dir <TEMPLATE_DIR>` | Custom template directory (only used with --template-kind=custom) | |
| `--output-dir <OUTPUT_DIR>` | Output directory for generated code | |
| `--log-file <LOG_FILE>` | Log file name without extension | `mcp-server` |
| `--port <PORT>` | Server port | `3000` |
| `--base-url <BASE_URL>` | Base URL of the OpenAPI specification (Optional) | |

#### Examples

```bash
# Basic usage with a local file
agenterra scaffold --schema-path api.yaml --output-dir generated

# Use a remote OpenAPI spec from a URL
agenterra scaffold --schema-path https://petstore3.swagger.io/api/v3/openapi.json --output-dir generated

# Specify a different project name and template
agenterra scaffold --schema-path api.yaml --output-dir generated --project-name my-api-server --template-kind rust_axum

# Use a custom template directory
agenterra scaffold --schema-path api.yaml --output-dir generated --template-kind custom --template-dir ./my-templates

# Configure server port and log file with base URL
agenterra scaffold --schema-path api.yaml --output-dir generated --port 8080 --log-file my-server --base-url https://api.example.com
```

## Exit Codes

| Code | Description |
|------|-------------|
| 0    | Success |
| 1    | General error |
| 2    | Invalid command line arguments |
| 3    | File I/O error |
| 4    | Template processing error |
| 5    | OpenAPI spec validation error |

## Environment Variables

| Variable | Description |
|----------|-------------|
| `AGENTERRA_TEMPLATE` | Default template to use |
| `AGENTERRA_TEMPLATE_DIR` | Default template directory |
| `AGENTERRA_LOG_LEVEL` | Log level (debug, info, warn, error) |

Note: Command-line arguments take precedence over environment variables.

## See Also

- [Configuration Guide](CONFIGURATION.md)
- [Templates Documentation](TEMPLATES.md)
- [Contributing Guide](../CONTRIBUTING.md)