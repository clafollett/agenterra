# 🔒 Security Features

Agenterra generates MCP servers and clients with security features designed to protect against common vulnerabilities while maintaining developer productivity. This document details the security measures implemented in generated code.

## Overview

Every Agenterra-generated project includes targeted security features:

1. **Input Validation** - Validation of user inputs with configurable security levels
2. **Database Security** - Protection against SQL injection through parameterized queries
3. **Transport Security** - Support for secure communication modes
4. **Permission Management** - Granular control over tool execution
5. **Unicode Security** - Protection against text-based attacks

## Input Validation Framework

### Validation Architecture

Generated clients include a comprehensive `validation.rs` module with configurable security policies. The validation system uses a three-tier result model:

```rust
pub enum ValidationResult {
    Ok(String),                              // Valid (possibly sanitized)
    Warning(String, Vec<ValidationIssue>),   // Has issues but allowed
    Error(Vec<ValidationIssue>),            // Rejected
}
```

### Command Validation

The command validation system provides **configurable** protection that can be adjusted based on use case:

```rust
// Default validation blocks shell metacharacters
let dangerous_chars = ['|', '&', ';', '$', '`', '\n', '\r', '(', ')', '<', '>'];
```

**Important**: This validation is applied to server commands and arguments stored in the configuration, NOT to the tools or prompts sent to MCP servers. This allows developers to:
- Use the REPL client for automation with full shell capabilities
- Send complex prompts and tool arguments without restriction
- Store safe server launch commands in configuration

The validation can be disabled or customized via `validation.toml`:
```toml
[content]
check_command_injection = false  # Disable for development environments
```

### Path Traversal Protection

Detects and blocks directory traversal attempts:
- Patterns: `../`, `..\`, `%2e%2e`, `%252e%252e`
- URL-encoded variants
- Mixed encoding attempts

### Unicode Security

#### Zero-Width Character Detection
Protects against invisible character attacks:
- Zero Width Space (U+200B)
- Zero Width Non-Joiner (U+200C) 
- Zero Width Joiner (U+200D)
- Soft Hyphen (U+00AD)
- And 15+ other invisible Unicode characters

#### Mixed Script Detection
Prevents homograph attacks by detecting mixed scripts:
```rust
// Detects strings mixing Latin with Cyrillic
"pаypal.com"  // 'а' is Cyrillic, not Latin 'a'
```

#### Emoji Security
Based on [emoji jailbreak research](https://cloud.google.com/blog/topics/developers-practitioners/red-teaming-llms-exploring-emojis-jailbreak-potential), the system can detect:
- Excessive emoji density (>30% of content)
- Suspicious emoji patterns
- Keywords hidden in emoji sequences

### Size and Complexity Limits

Configurable limits to prevent resource exhaustion:
- Maximum input size: 10MB (default)
- JSON nesting depth: 10 levels (default)
- Field-specific limits (e.g., server names: 255 chars)

## Database Security

### SQL Injection Prevention

All database operations use parameterized queries exclusively:

```rust
// Always use parameterized queries
conn.prepare("SELECT * FROM servers WHERE name = ?1")?;
stmt.query_row(params![name], |row| { ... })

// Never string concatenation
// BAD: format!("SELECT * FROM servers WHERE name = '{}'", name)
```

### Connection Management
- Connection pooling with r2d2
- Configurable pool size (1-10 connections)
- Connection lifetime limits
- Automatic cleanup on shutdown

## Transport Security

### STDIO Mode (Default)
- Direct process communication
- No network exposure
- Suitable for local development

### SSE Mode
Server-Sent Events support for web deployments:

#### Server Configuration
```bash
./server --transport sse --sse-addr 127.0.0.1:8080
```
- Configurable bind address
- Keep-alive for connection health
- Graceful shutdown support

#### Client Configuration  
```bash
./client --transport sse --sse-url https://api.example.com/mcp
```
- URL validation (blocks `javascript:`, `data:`, etc.)
- Automatic reconnection with backoff
- TLS support via HTTPS URLs

## Permission Management

### Interactive Tool Approval

The client includes an interactive permission system for tool execution:

```
🔒 Permission Required
Tool: delete_repository
Arguments: {"name": "production-db"}

Execute 'delete_repository' tool? (y)es, (n)o, (a)lways: n
```

### Server Profiles

Granular control over server behavior:

```bash
# Disable entire server
./client server add risky-server /path/to/server --disabled

# Pre-approve safe tools
./client server add dev-server /path/to/server \
  --always-allowed "list_files,read_file"

# Block dangerous tools
./client server add prod-server /path/to/server \
  --disabled-tools "delete_all,force_push"
```

### Permission Storage
- Stored in SQLite with full ACID guarantees
- Client-side enforcement (no server communication for blocked tools)
- Audit logging of all permission decisions

## Configuration

### Validation Configuration (`validation.toml`)

```toml
[validation]
enabled = true
max_input_size = 10485760  # 10MB
security_level = "balanced"  # permissive, balanced, strict

[unicode]
allow_rtl = false
block_invisible = true
normalize_text = true

[content]
check_command_injection = true
check_path_traversal = true
check_prompt_injection = false  # Disabled by default for flexibility

[patterns]
max_emoji_density = 0.3
block_suspicious_patterns = true
```

### Security Levels

- **Permissive**: Minimal validation, suitable for trusted environments
- **Balanced**: Default setting, blocks known dangerous patterns
- **Strict**: Maximum validation, may impact legitimate use cases

## Best Practices

1. **Configure for Your Environment**
   - Development: More permissive settings
   - Production: Stricter validation rules

2. **Use Permission Management**
   - Pre-approve read-only tools
   - Block destructive operations
   - Review permission requests carefully

3. **Monitor and Audit**
   - Check logs for validation failures
   - Review audit logs periodically
   - Update validation rules based on threats

4. **Keep Dependencies Updated**
   ```bash
   cargo update  # Regular security updates
   cargo audit   # Check for known vulnerabilities
   ```

## Testing Security

Generated projects include security tests:

```rust
#[test]
fn test_parameterized_queries() {
    // Verify SQL injection protection
}

#[test]
fn test_unicode_normalization() {
    // Test mixed script detection
}

#[test]
fn test_permission_enforcement() {
    // Verify tool blocking works
}
```

## Limitations and Considerations

1. **Command Validation**: Only applies to server launch commands, not MCP tool arguments
2. **Client-Side Security**: Permission checks happen in the client, not the server
3. **Transport Security**: SSE mode requires proper TLS configuration for production
4. **Unicode Validation**: May impact legitimate international content

## Security Incident Response

If you discover a vulnerability:
1. **Do Not** create a public issue
2. **Do** report privately to maintainers
3. Include: description, reproduction steps, impact assessment

## Additional Resources

- [OWASP Input Validation Cheat Sheet](https://cheatsheetseries.owasp.org/cheatsheets/Input_Validation_Cheat_Sheet.html)
- [Unicode Security Guide](https://unicode.org/reports/tr36/)
- [SQL Injection Prevention](https://cheatsheetseries.owasp.org/cheatsheets/SQL_Injection_Prevention_Cheat_Sheet.html)

---

**Remember**: Security is about finding the right balance between protection and usability. Agenterra provides configurable security features that can be tuned to your specific needs and threat model.