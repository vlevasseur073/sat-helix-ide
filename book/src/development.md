# Development

This chapter covers how to build, test, and contribute to `sat-helix-ide`.

## Building from Source

### Prerequisites

- Rust toolchain (stable recommended)
- Cargo
- Git

Verify your Rust setup:

```bash
rustc --version
cargo --version
```

### Cloning the Repository

```bash
git clone https://github.com/vlevasseur073/sat-helix-ide.git
cd sat-helix-ide
```

### Building

```bash
# Development build (unoptimized, with debug info)
cargo build

# Release build (optimized)
cargo build --release

# Install locally
cargo install --path .
```

### Building the Documentation

```bash
# Build this book
cd book
mdbook build

# Serve the book locally for preview
mdbook serve --open
```

The book will be available at `http://localhost:3000`.

## Running Tests

`sat-helix-ide` has a comprehensive test suite.

### Running All Tests

```bash
cargo test --locked
```

The `--locked` flag ensures that only the exact dependencies specified in `Cargo.lock` are used, providing reproducible builds.

### Running Specific Tests

```bash
# Run tests in a specific module
cargo test --locked actions

# Run a single test
cargo test --locked test_specific_function

# Run tests with more verbose output
cargo test --locked -- --nocapture
```

### Test Coverage

To check test coverage (requires `cargo-tarpaulin`):

```bash
# Install tarpaulin
cargo install cargo-tarpaulin

# Run coverage analysis
cargo tarpaulin --locked
```

## Code Quality

### Formatting

`sat-helix-ide` uses `rustfmt` for code formatting:

```bash
# Format all code
cargo fmt --all

# Check formatting without applying
cargo fmt --all -- --check
```

### Linting

The project uses `clippy` for linting:

```bash
# Run clippy
cargo clippy --locked --all-targets -- -D warnings
```

The `-D warnings` flag treats all warnings as errors, ensuring clean code.

### Pre-commit Hooks

The project includes a pre-commit configuration for automated quality checks:

```bash
# Install pre-commit
pip install pre-commit

# Install hooks
pre-commit install

# Run all pre-commit checks manually
pre-commit run --all-files
```

The `.pre-commit-config.yaml` file defines checks for:
- Rust formatting
- Clippy linting
- Tests
- Documentation

## Project Structure

```text
sat-helix-ide/
├── Cargo.toml                 # Project metadata and dependencies
├── Cargo.lock                # Dependency lock file
├── README.md                 # Project README
├── LICENSE                   # License file
├── rust-toolchain.toml       # Rust toolchain specification
├── .gitignore                # Git ignore patterns
├── .pre-commit-config.yaml  # Pre-commit hooks
├── .github/                  # GitHub configuration
│   └── workflows/            # GitHub Actions workflows
├── book/                     # This documentation
│   ├── book.toml             # mdbook configuration
│   └── src/                  # Book source files
├── configs/                  # Sample configuration files
│   └── config.toml           # Default configuration
├── docs/                     # Additional documentation
│   └── architecture.md       # Architecture document (now integrated into book)
├── src/                      # Source code
│   ├── main.rs               # CLI entry point
│   ├── actions.rs            # Action handlers
│   ├── config.rs             # Configuration handling
│   ├── layout.rs             # Layout generation
│   └── error.rs              # Error types
└── tests/                    # Test files
    └── test_*.rs              # Integration tests
```

## Contributing

### Getting Started

1. Fork the repository on GitHub
2. Clone your fork locally
3. Create a feature branch
4. Make your changes
5. Run tests and linting
6. Commit your changes
7. Push to your fork
8. Open a pull request

### Pull Request Guidelines

- Follow the existing code style
- Add tests for new functionality
- Update documentation for any user-facing changes
- Keep commits atomic and well-described
- Include a clear description of the changes in the PR

### Commit Messages

- Use the imperative mood ("Add feature" not "Added feature")
- Limit the first line to 72 characters
- Reference issues and pull requests when applicable
- Include a body describing the change if needed

### Code Review Process

1. All PRs must pass CI checks
2. Code review is required before merging
3. Address all review comments
4. Update the PR with any requested changes

## Continuous Integration

The project uses GitHub Actions for CI/CD with the following workflows:

### Main Workflow

Located at `.github/workflows/ci.yml`, it runs on:
- Pushes to main branch
- Pull requests to main branch

The workflow includes:
- Rust formatting check
- Clippy linting
- Tests
- Build verification

### Building and Testing Locally

To reproduce the CI environment locally:

```bash
# Format check
cargo fmt --all -- --check

# Clippy check
cargo clippy --locked --all-targets -- -D warnings

# Run tests
cargo test --locked

# Full pre-commit run
pre-commit run --all-files
```

## Debugging

### Debug Logging

Enable debug logging to see detailed information about `sat-helix-ide` operations:

```bash
RUST_LOG=debug sat-hx-ide init .

# For even more detail
RUST_LOG=trace sat-hx-ide init .
```

### Common Debug Scenarios

#### Debugging Session Initialization

```bash
RUST_LOG=debug sat-hx-ide init . --session test-session
```

This will show:
- Configuration loading
- Layout generation
- Zellij session creation
- Keybinding injection

#### Debugging Action Execution

```bash
# Start a session first
sat-hx-ide init .

# Then in another terminal, trace helper commands
RUST_LOG=trace sat-hx-ide __file-manager open
```

### Debugging Zellij Integration

To debug Zellij integration:

```bash
# Check Zellij version
zellij --version

# List active sessions
zellij list-sessions

# Attach to a session with debug logging
ZELLIJ_LOG=debug zellij attach <session>
```

## Architecture Decision Records (ADRs)

Major architectural decisions are documented in the codebase and in this book:

- [Process Model](./architecture.md#process-model) - Why short-lived processes
- [Non-Intrusive Design](./architecture.md#configuration-merge) - Why never modify global configs
- [Zellij Choice](./architecture.md#why-zellij) - Why Zellij was chosen

For new ADRs, create a new section in the `architecture.md` or add a new file in the `docs/` directory.

## Release Process

### Version Management

The project follows semantic versioning:
- `MAJOR` - Breaking changes
- `MINOR` - New features (backwards compatible)
- `PATCH` - Bug fixes (backwards compatible)

### Creating a Release

1. Update the version in `Cargo.toml`
2. Update the changelog (if applicable)
3. Create a Git tag
4. Push the tag to GitHub
5. GitHub Actions will build and publish the release

```bash
# Update version
hx Cargo.toml  # Update version field

# Commit version change
git commit -am "Bump version to x.y.z"

# Create tag
git tag vx.y.z

# Push tag
git push origin vx.y.z
```

## Performance Optimization

### Profiling

To profile `sat-helix-ide` performance:

```bash
# Install flamegraph tools
cargo install flamegraph

# Generate flamegraph
cargo flamegraph --bench your_benchmark -- --bench-name=your_benchmark
```

### Common Optimization Areas

1. **Zellij CLI Calls**: Reduce the number of calls to Zellij CLI
2. **Configuration Parsing**: Optimize TOML parsing
3. **Layout Generation**: Optimize KDL generation
4. **Pane Cache**: Improve cache hit rates

## Documentation

### Writing Documentation

This book uses mdbook. To add new documentation:

1. Create a new markdown file in `book/src/`
2. Add it to the SUMMARY.md
3. Build and preview with `mdbook serve`

### Documentation Standards

- Use clear, concise language
- Include examples for configuration options
- Document all user-facing features
- Include troubleshooting information
- Link to related documentation

### Generating API Documentation

For Rust API documentation:

```bash
# Generate HTML documentation
cargo doc --open

# Generate with private items
cargo doc --open --document-private-items
```

## Next Steps

- [Troubleshooting](./troubleshooting.md) - Common issues and solutions
- [Appendix](./appendix.md) - Command reference and additional resources
- [Contribute on GitHub](https://github.com/vlevasseur073/sat-helix-ide) - Start contributing
