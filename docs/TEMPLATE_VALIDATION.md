# Template Validation Process

This document outlines the MANDATORY steps to validate any changes to MCP templates.

## When to Run This Process
- After ANY changes to files in `templates/`
- Before committing template changes
- Before creating a PR with template modifications

## Prerequisites
- Rust toolchain installed
- All dependencies up to date

## Validation Steps

### Step 1: Clean and Validate Workspace
```bash
# At repository root
cargo clean && cargo fmt && cargo clippy --fix --allow-dirty -- -D warnings
```
Fix any issues found by clippy before proceeding.

### Step 2: Run All Tests
```bash
# This includes E2E tests that scaffold projects
cargo test
```
The E2E tests will generate MCP client and server projects in `target/tmp/`.

### Step 3: Validate Generated Projects
Find the generated projects and validate them:

#### For Client:
```bash
cd target/tmp/cli_flag_tests/test_cli_flag_combinations/test_default_project_name/agenterra_mcp_client
cargo fmt && cargo clippy --fix --allow-dirty -- -D warnings
```

#### For Server:
```bash
cd ../agenterra_mcp_server  
cargo fmt && cargo clippy --fix --allow-dirty -- -D warnings
```

### Step 4: Backfill Template Fixes
If formatting or linting issues are found:

1. Identify which template file generated the problematic code
2. Apply the fix to the corresponding `.tera` file
3. Re-run the ENTIRE validation process from Step 1

## PR Checklist
Copy this checklist to your PR description:

```
- [ ] Ran `cargo clean && cargo fmt && cargo clippy` on workspace
- [ ] Fixed all clippy warnings in workspace
- [ ] Ran `cargo test` successfully (including E2E)
- [ ] Validated generated client with `cargo fmt && cargo clippy`
- [ ] Validated generated server with `cargo fmt && cargo clippy`
- [ ] Backfilled any fixes to `.tera` templates
- [ ] Re-ran validation after template fixes
```

## Common Issues and Solutions

### Trailing Whitespace
- Often found in `.tera` files
- Use your editor's "trim trailing whitespace" feature

### Format Issues
- Check indentation matches surrounding code
- Ensure proper spacing around operators

### Clippy Warnings
- `#[allow(dead_code)]` only for genuinely unused template code
- Fix all other warnings at the source