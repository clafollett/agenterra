# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.2.1](https://github.com/clafollett/agenterra/compare/agenterra-v0.2.0...agenterra-v0.2.1) - 2025-07-09

### Added

- Domain-Driven Design architecture with embedded templates ([#104](https://github.com/clafollett/agenterra/pull/104))

### Added

- Embedded templates in binary for `cargo install` support ([#90](https://github.com/clafollett/agenterra/issues/90))
  - Templates are now embedded using `rust-embed` crate
  - Added `templates` CLI subcommand with `list`, `info`, and `export` actions
  - Support for exporting all templates or a single template
  - Simplified template naming (e.g., `rust` instead of `rust_axum`)

## [0.2.0](https://github.com/clafollett/agenterra/compare/agenterra-v0.1.4...agenterra-v0.2.0) - 2025-07-02

### Added

- [**breaking**] complete SSE support and comprehensive security enhancements ([#18](https://github.com/clafollett/agenterra/pull/18)) ([#101](https://github.com/clafollett/agenterra/pull/101))

## [0.1.4](https://github.com/clafollett/agenterra/compare/agenterra-v0.1.3...agenterra-v0.1.4) - 2025-06-24

### Fixed

- update templates for rmcp API compatibility ([#95](https://github.com/clafollett/agenterra/pull/95)) ([#96](https://github.com/clafollett/agenterra/pull/96))

## [0.1.3](https://github.com/clafollett/agenterra/compare/agenterra-v0.1.2...agenterra-v0.1.3) - 2025-06-23

### Added

- Transform Agenterra into pure code generator architecture ([#89](https://github.com/clafollett/agenterra/pull/89)) ([#91](https://github.com/clafollett/agenterra/pull/91))

### Fixed

- remove test project patterns from gitignore ([#92](https://github.com/clafollett/agenterra/pull/92))

## [0.1.2](https://github.com/clafollett/agenterra/compare/agenterra-v0.1.1...agenterra-v0.1.2) - 2025-06-20

### Other

- update Cargo.lock dependencies
