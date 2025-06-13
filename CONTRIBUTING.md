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
   git checkout -b GH-<issue-number>_<ProperCaseSummary>
   ```
   
   Example: `GH-15_AddOpenAPI31Support`

3. **Make Changes**
   - Follow our coding style
   - Add tests
   - Update documentation

4. **Run Tests**
   ```bash
   cargo test
   ```

5. **Commit**
   ```bash
   git commit -m "feat: add your feature (#<issue-number>)"
   ```

6. **Push & Create PR**
   ```bash
   git push origin feature/your-feature-name
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
   cargo test                                        # All tests
   cargo test -p agenterra-cli --test integration_test  # Integration tests
   ```

4. **Test Agenterra CLI**
   ```bash
   cargo run -p agenterra -- scaffold --schema-path ./tests/fixtures/openapi/petstore.openapi.v3.json --output-dir test-output
   ```

## Coding Guidelines 📝

1. **Rust Style**
   - Follow Rust style guidelines
   - Use `rustfmt`
   - Run `clippy`

2. **Testing**
   - Write unit tests
   - Add integration tests
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
├── crates/
│   ├── agenterra-core/      # Core library
│   └── agenterra-cli/       # CLI interface
├── docs/                    # Documentation
├── templates/               # Code generation templates
├── tests/fixtures/          # Test OpenAPI specs
└── plans/                   # Project planning docs
```

## Getting Help 💬

- Create an issue
- Join our Discord
- Check the documentation

## License 📄

By contributing, you agree that your contributions will be licensed under the MIT license.
