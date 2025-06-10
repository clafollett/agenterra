# Agenterra CLI Reference 📝

This document provides a comprehensive reference for the Agenterra command-line interface.

> **📌 Note:** The `agnt` command is provided as a short alias for `agenterra`.

## Table of Contents
- [Global Options](#global-options)
- [Commands](#commands)
  - [scaffold](#scaffold)
- [Examples](#examples)
- [Exit Codes](#exit-codes)

## Global Options

| Option | Description |
|--------|-------------|
| `-h`, `--help` | Print help |
| `-V`, `--version` | Print version |

## Commands

### scaffold

Scaffold a new MCP server from an OpenAPI specification.

```bash
agenterra scaffold --spec <SPEC> --output <OUTPUT> [OPTIONS]
```

#### Options

| Option | Description | Default |
|--------|-------------|---------|
| `--spec <LOCATION>` | Path or URL to OpenAPI spec (YAML or JSON). Can be a local file path or an HTTP/HTTPS URL. | *required* |
| `--output <DIR>` | Output directory for generated code | *required* |
| `-t`, `--template <NAME>` | Template to use (e.g., rust_axum, python_fastapi). Default is Rust with Axum framework. | `rust_axum` |
| `--template-dir <DIR>` | Custom template directory (only used with --template=custom) | |
| `--policy-plugins <PLUGINS>` | Comma-separated list of policy plugins | |
| `--port <PORT>` | Server port | `3000` |
| `--log-file <FILE>` | Log file name without extension | `mcp-server` |

#### Examples

```bash
# Basic usage with a local file
agenterra scaffold --spec api.yaml --output generated

# Use a remote OpenAPI spec from a URL
agenterra scaffold --spec https://example.com/openapi.json --output generated

# Specify a different template with a URL spec
agenterra scaffold --spec https://example.com/openapi.yaml --output generated --template python-fastapi

# Use a custom template directory with a local spec
agenterra scaffold --spec api.yaml --output generated --template custom --template-dir ./my-templates

# Configure server port and log file
agenterra scaffold --spec api.yaml --output generated --port 8080 --log-file my-server
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
| 1    | General error |
| 2    | Configuration error |
| 3    | Validation error |
| 4    | Template error |
| 5    | I/O error |
| 6    | Network error |

## See Also

- [Configuration Guide](CONFIGURATION.md)
- [Templates Documentation](TEMPLATES.md)
- [Contributing Guide](../CONTRIBUTING.md)
