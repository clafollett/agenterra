# MCPGen Configuration ⚙️

This guide explains how to configure MCPGen using different methods.

## Table of Contents
- [Configuration Methods](#configuration-methods)
- [Command-Line Options](#command-line-options)
- [Configuration File](#configuration-file)
- [Environment Variables](#environment-variables)
- [Template Configuration](#template-configuration)
- [Example Configurations](#example-configurations)

## Configuration Methods

MCPGen can be configured using the following methods (in order of precedence):

1. **Command-Line Arguments** (highest priority)
2. **Configuration File** (`mcpgen.toml` or `.mcpgen.toml` in project root)
3. **Environment Variables**
4. **Default Values** (lowest priority)

## Command-Line Options

### Global Options

```bash
mcpgen [OPTIONS] <SUBCOMMAND>
```

| Option | Description | Default |
|--------|-------------|---------|
| `-c`, `--config <FILE>` | Path to config file | `mcpgen.toml` or `.mcpgen.toml` |
| `-v`, `--verbose` | Enable verbose output | `false` |
| `-q`, `--quiet` | Suppress non-essential output | `false` |
| `-h`, `--help` | Print help | |
| `-V`, `--version` | Print version | |

### Generate Command

```bash
mcpgen generate [OPTIONS] --input <INPUT> --output <OUTPUT>
```

| Option | Description | Default |
|--------|-------------|---------|
| `-i`, `--input <FILE>` | Path to OpenAPI spec file | *required* |
| `-o`, `--output <DIR>` | Output directory | *required* |
| `-t`, `--template <NAME>` | Template name | `rust_axum` |
| `--force` | Overwrite existing files | `false` |
| `--skip-validate` | Skip OpenAPI validation | `false` |

## Configuration File

Create a `mcpgen.toml` or `.mcpgen.toml` file in your project root:

```toml
[generate]
input = "openapi.yaml"
output = "generated"
template = "rust_axum"
force = false
skip_validate = false

[template_options]
# Template-specific options
all_operations = true
include_operations = []
exclude_operations = []

[server]
port = 8080
log_level = "info"

[openapi]
# OpenAPI processing options
prefer_async = true
use_chrono = true
use_uuid = true
```

## Environment Variables

All configuration options can be set via environment variables with the `MCPGEN_` prefix:

```bash
# Basic options
export MCPGEN_INPUT=openapi.yaml
export MCPGEN_OUTPUT=generated

# Template options
export MCPGEN_TEMPLATE=rust_axum
export MCPGEN_TEMPLATE_OPTIONS_ALL_OPERATIONS=true

# Server options
export MCPGEN_SERVER_PORT=8080
export MCPGEN_SERVER_LOG_LEVEL=debug
```

## Template Configuration

Templates can be configured using the `[template_options]` section in the config file:

```toml
[template_options]
# Include only specific operations
all_operations = false
include_operations = ["getPets", "createPet"]

# Or exclude specific operations
exclude_operations = ["deprecatedOperation"]

# Custom template variables
custom_value = "example"
```

## Example Configurations

### Minimal Configuration

```toml
[generate]
input = "api/openapi.yaml"
output = "generated"
```

### Full Configuration

```toml
[generate]
input = "api/openapi.yaml"
output = "generated"
template = "rust_axum"
force = true
skip_validate = false

[template_options]
all_operations = true
include_operations = []
exclude_operations = ["deprecatedOperation"]

[server]
port = 3000
log_level = "debug"
host = "0.0.0.0"

[openapi]
prefer_async = true
use_chrono = true
use_uuid = true
use_serde = true

[logging]
level = "info"
format = "json"
```

### Environment Variables Example

```bash
# .env file
MCPGEN_INPUT=api/openapi.yaml
MCPGEN_OUTPUT=generated
MCPGEN_TEMPLATE=rust_axum
MCPGEN_SERVER_PORT=3000
MCPGEN_LOGGING_LEVEL=debug
```

## Configuration Precedence

1. Command-line arguments
2. Environment variables
3. Configuration file (`mcpgen.toml` or `.mcpgen.toml`)
4. Default values

## Next Steps

- [Templates Documentation](TEMPLATES.md)
- [CLI Reference](CLI_REFERENCE.md)
- [Contributing Guide](../CONTRIBUTING.md)
