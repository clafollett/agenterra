# Contributing to Agenterra 🚀

First off, thank you for considering contributing to Agenterra! We're excited to have you join our community.

## Code of Conduct 🤝

This project and everyone participating in it is governed by our Code of Conduct. By participating, you are expected to uphold this code.

## How Can I Contribute? 🌟

### Reporting Bugs 🐛

1. **Check Existing Issues** - Search the issue tracker to avoid duplicates
2. **Create a Clear Report** - Include:
   - Steps to reproduce
   - Expected behavior
   - Actual behavior
   - Agenterra version
   - OpenAPI spec (if relevant)
   - Error messages
   - Environment details

### Suggesting Enhancements 💡

1. **Check the Roadmap** - See if it's already planned
2. **Create a Feature Request** - Include:
   - Use case
   - Proposed solution
   - Alternatives considered
   - Example code/specs

### Pull Requests 🔧

1. **Fork & Clone**
   ```bash
   git clone https://github.com/YOUR-USERNAME/agenterra.git
   ```

2. **Create a Branch**
   ```bash
   git checkout -b <type>/issue-<number>/<description>
   ```
   
   Examples:
   - `docs/issue-57/update-readme`
   - `feature/issue-42/add-mcp-client-template`
   - `fix/issue-123/template-generation-error`

3. **Make Changes**
   - Follow our coding style
   - Add tests for new features
   - Update documentation (especially if CLI changes)
   - Test both server and client generation if applicable

4. **Run Tests**
   ```bash
   cargo test
   ```

5. **Test CLI Changes**
   ```bash
   # Test new server generation
   cargo run -- scaffold mcp server --schema-path ./tests/fixtures/openapi/petstore.openapi.v3.json --output-dir test-output-server --base-url https://petstore3.swagger.io
   
   # Test new client generation  
   cargo run -- scaffold mcp client --project-name test-client --output-dir test-output-client
   ```

6. **Commit**
   ```bash
   git commit -m "feat: add your feature (#<issue-number>)"
   ```

7. **Push & Create PR**
   ```bash
   git push origin <type>/issue-<number>/<description>
   ```

## Development Setup 🛠️

1. **Prerequisites**
   - Rust (latest stable)
   - Cargo
   - Git

2. **Dependencies**
   ```bash
   cargo build
   ```

3. **Running Tests**
   ```bash
   cargo test                       # All tests
   cargo test --test e2e_mcp_test   # Integration tests
   ```

4. **Test Agenterra CLI**
   ```bash
   # Test MCP server generation
   cargo run -- scaffold mcp server --schema-path ./tests/fixtures/openapi/petstore.openapi.v3.json --output-dir test-server --base-url https://petstore3.swagger.io
   
   # Test MCP client generation
   cargo run -- scaffold mcp client --project-name test-client --output-dir test-client
   ```

## Coding Guidelines 📝

1. **Rust Style**
   - Follow Rust style guidelines
   - Use `cargo fmt`
   - Run `cargo clippy --fix --allow-dirty -- -D warnings`

2. **Testing**
   - Write unit tests
   - Add integration tests for both servers and clients
   - Test cache configuration and connection pooling options
   - Test edge cases

3. **Documentation**
   - Document public APIs
   - Add examples
   - Update README if needed

4. **Commit Messages**
   - Use conventional commits
   - Reference issues

## Project Structure 📁

```
agenterra/
├── src/                # Single-crate Rust application
│   ├── core/           # Core functionality
│   │   └── templates/  # Template system (embedded & filesystem)
│   └── mcp/            # MCP-specific code
│       └── builders/   # MCP server/client builders
├── docs/               # Documentation
├── templates/          # Code generation templates
│   └── mcp/            # MCP protocol templates
│       ├── server/     # MCP server templates
│       │   └── rust/   # Rust server template
│       └── client/     # MCP client templates
│           └── rust/   # Rust client template with SQLite caching
├── tests/              # Test suite
│   ├── fixtures/       # Test fixtures
│   │   └── openapi/    # OpenAPI test specs
│   ├── cli_templates_test.rs  # CLI template command tests
│   └── e2e_mcp_test.rs        # End-to-end MCP tests
├── scripts/            # Build and utility scripts
└── .github/            # GitHub configuration
    ├── workflows/      # CI/CD automation
    └── ISSUE_TEMPLATE/ # Issue templates
```

## Getting Help 💬

- Create an issue
- Check the documentation

## License 📄

By contributing, you agree that your contributions will be licensed under the MIT license.
