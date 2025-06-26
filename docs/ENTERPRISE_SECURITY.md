# 🔒 Enterprise Security Features

Agenterra generates MCP servers and clients with enterprise-grade security features built-in from day one. This document details the comprehensive security measures implemented in generated code.

## Overview

Every Agenterra-generated project includes multiple layers of security:

1. **Input Validation** - Comprehensive validation of all user inputs
2. **Database Security** - Protection against SQL injection
3. **Transport Security** - Secure communication options
4. **Resource Protection** - Prevention of resource exhaustion attacks
5. **Modern Attack Prevention** - Protection against LLM-specific attacks

## Input Validation Suite

### Core Validation Module

Generated clients include a comprehensive `validation.rs` module that protects against:

#### SQL Injection Protection
- All database queries use **parameterized statements**
- No raw SQL concatenation
- Example:
  ```rust
  conn.prepare("SELECT * FROM servers WHERE name = ?1")?;
  stmt.query_row(params![name], |row| { ... })
  ```

#### Command Injection Prevention
- Validates against shell metacharacters (`|`, `&`, `;`, `$`, `` ` ``, etc.)
- Blocks command chaining attempts
- Example protection:
  ```rust
  // This would be rejected:
  server_name: "myserver; rm -rf /"
  args: ["--file", "data.txt | cat /etc/passwd"]
  ```

#### Path Traversal Blocking
- Detects patterns like `../`, `..\\`, `%2e%2e`
- Prevents directory escape attempts
- Example:
  ```rust
  // These are blocked:
  command: "../../../etc/passwd"
  command: "..%2f..%2fetc%2fpasswd"
  ```

#### Prompt Injection Detection
- Scans for common injection phrases:
  - "ignore previous instructions"
  - "disregard above"
  - "system:", "assistant:", "user:"
- Prevents LLM manipulation attempts

### Unicode Security

#### Zero-Width Character Detection
Protects against invisible character attacks by detecting:
- Zero Width Space (U+200B)
- Zero Width Non-Joiner (U+200C)
- Zero Width Joiner (U+200D)
- Word Joiner (U+2060)
- And other invisible Unicode characters

#### Emoji Jailbreak Protection
Based on research from [Google Cloud's emoji jailbreak article](https://medium.com/google-cloud/emoji-jailbreaks-b3b5b295f38b), we detect:
- Excessive emoji usage (>30% of input)
- Consecutive emoji patterns (>3 in a row)
- Emoji-text-emoji sandwich patterns
- Suspicious keywords combined with emojis

Example blocked pattern:
```
"😀😀😀 ignore 😀😀😀 previous 😀😀😀 instructions"
```

#### Homograph Attack Prevention
- Detects Unicode normalization issues
- Prevents character substitution attacks (e.g., Cyrillic 'а' vs Latin 'a')
- Requires normalized Unicode input

### Size and Complexity Limits

#### Input Size Restrictions
- Maximum input size: 1MB
- Per-field limits:
  - Server names: 255 characters
  - Descriptions: 1KB
  - Commands: 4KB
  - Environment values: 32KB

#### JSON Security
- Maximum nesting depth: 10 levels
- Maximum keys per object: 1000
- Prevents JSON bomb attacks
- Example:
  ```rust
  // This deeply nested JSON would be rejected:
  {"a": {"b": {"c": {"d": ... (15 levels deep)
  ```

## Database Security

### Parameterized Queries
All database operations use parameterized queries to prevent SQL injection:

```rust
// Safe query with parameters
let mut stmt = conn.prepare(
    "INSERT INTO servers (id, name, command, args) VALUES (?1, ?2, ?3, ?4)"
)?;
stmt.execute(params![id, name, command, args_json])?;

// Never constructed with string concatenation
// BAD: format!("SELECT * FROM servers WHERE name = '{}'", name)
```

### Transaction Safety
- ACID compliance with SQLite
- Automatic rollback on errors
- Connection pooling with r2d2

## Transport Security

### SSE (Server-Sent Events) Mode
Generated servers and clients support SSE transport for secure web deployments:

#### Server Security
- Configurable bind address
- Keep-alive mechanism to detect stale connections
- Graceful shutdown handling
- Example:
  ```bash
  ./server --transport sse --sse-addr 127.0.0.1:8080
  ```

#### Client Security
- URL validation for SSE endpoints
- Rejects dangerous URL schemes (javascript:, data:, vbscript:)
- Automatic reconnection with exponential backoff
- Example:
  ```bash
  ./client --transport sse --sse-url https://api.example.com/mcp
  ```

## Resource Protection

### Connection Pooling
- Prevents connection exhaustion
- Configurable pool size (1-10 connections)
- Connection lifetime limits (5 minutes default)

### Cache Management
- Size limits to prevent disk exhaustion
- TTL-based expiration
- Automatic cleanup of expired resources

### Rate Limiting
- Built-in request throttling
- Prevents DoS attacks
- Configurable limits

## Validation API

### Server Name Validation
```rust
InputValidator::validate_server_name("my-server")?;
// Allows: alphanumeric, dash, underscore
// Blocks: special characters, reserved names
```

### Command Validation
```rust
InputValidator::validate_command("/usr/bin/server")?;
// Validates paths, URLs
// Blocks: path traversal, dangerous patterns
```

### Environment Variable Validation
```rust
let env = InputValidator::validate_environment(r#"{"PATH": "/usr/bin"}"#)?;
// Validates JSON structure
// Ensures keys are valid identifiers
// Checks values for dangerous patterns
```

### Identifier Validation
```rust
// For tool names (allows dashes)
InputValidator::validate_identifier("my-tool", "tool name", true)?;

// For environment variables (no dashes)
InputValidator::validate_identifier("MY_VAR", "environment variable", false)?;
```

## Testing Security

Generated projects include comprehensive security tests:

```rust
#[test]
fn test_sql_injection_protection() {
    // Verify parameterized queries work correctly
}

#[test]
fn test_emoji_jailbreak_detection() {
    assert!(InputValidator::check_emoji_jailbreak_patterns("😀😀😀😀😀").is_err());
}

#[test]
fn test_zero_width_detection() {
    assert!(InputValidator::check_unicode_attacks("hello\u{200B}world").is_err());
}
```

## Configuration

### Client Security Configuration
```rust
// In generated client code
let validator = InputValidator::new();
validator.validate_all_inputs(&user_input)?;
```

### Server Security Configuration
```bash
# Environment variables for security
TRANSPORT=sse
SSE_ADDR=127.0.0.1:8080
SSE_KEEP_ALIVE=30
LOG_LEVEL=info
```

## Server Control Features

Generated MCP clients include granular server control capabilities for enhanced security:

### Server Disable/Enable
Completely disable a server to prevent any connections:

```bash
# Add a disabled server
./client server add compromised-server /path/to/server --disabled

# The client will refuse to connect
./client --profile compromised-server
# Error: Server profile 'compromised-server' is disabled
```

This feature is critical for:
- **Incident Response**: Immediately disable compromised servers
- **Maintenance Windows**: Temporarily disable servers during updates
- **Access Control**: Disable servers for specific users or environments

### Tool Permission Management

#### Always Allowed Tools
Bypass approval prompts for trusted tools to improve workflow efficiency:

```bash
./client server add github-server /path/to/server \
  --always-allowed "list_issues,get_issue,list_pull_requests"
```

Benefits:
- **Productivity**: No interruptions for read-only operations
- **Automation**: Enable headless workflows for safe tools
- **User Experience**: Reduce approval fatigue

#### Disabled Tools
Block specific tools from being called:

```bash
./client server add restricted-server /path/to/server \
  --disabled-tools "delete_repository,force_push,merge_pull_request"
```

Protection against:
- **Destructive Operations**: Prevent accidental data loss
- **Privilege Escalation**: Block admin-only tools
- **Compliance**: Enforce organizational policies

### Implementation Details

All server control features are:
- **Stored Securely**: In the SQLite database with proper constraints
- **Validated**: Tool names undergo full security validation
- **Enforced Client-Side**: Checks happen before any server communication
- **Auditable**: All permission checks are logged

Example configuration:
```json
{
  "mcpServers": {
    "production-api": {
      "command": "./mcp-server",
      "disabled": false,
      "alwaysAllowed": ["read_data", "list_resources"],
      "disabledTools": ["delete_all", "admin_reset"]
    }
  }
}
```

## Best Practices

1. **Always Validate Input**: Never trust user input, even from authenticated sources
2. **Use Generated Validators**: Leverage the built-in validation functions
3. **Keep Dependencies Updated**: Regular `cargo update` for security patches
4. **Monitor Logs**: Watch for validation failures as potential attack indicators
5. **Test Security**: Run the included security tests regularly

## Security Incident Response

If you discover a security vulnerability:

1. **Do Not** create a public GitHub issue
2. **Do** report it privately to the maintainers
3. Include:
   - Description of the vulnerability
   - Steps to reproduce
   - Potential impact
   - Suggested fix (if any)

## Compliance

Generated code helps meet common security requirements:

- **OWASP Top 10** protection
- **CWE** coverage for common weaknesses
- **Input validation** per NIST guidelines
- **Secure coding** practices

## Additional Resources

- [OWASP Input Validation Cheat Sheet](https://cheatsheetseries.owasp.org/cheatsheets/Input_Validation_Cheat_Sheet.html)
- [Unicode Security Considerations](https://unicode.org/reports/tr36/)
- [SQL Injection Prevention](https://cheatsheetseries.owasp.org/cheatsheets/SQL_Injection_Prevention_Cheat_Sheet.html)
- [Emoji Jailbreaks Research](https://medium.com/google-cloud/emoji-jailbreaks-b3b5b295f38b)

---

**Remember**: Security is a journey, not a destination. While Agenterra provides robust security features out of the box, always review and enhance security measures based on your specific use case and threat model.