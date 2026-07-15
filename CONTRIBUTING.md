# Contributing to LuperIQ Platform

Thank you for your interest in contributing to LuperIQ! This document provides guidelines and information for contributors.

## Getting Started

### Prerequisites

- Rust 1.75 or later
- Cargo (included with Rust)

### Building

```bash
git clone https://github.com/TheAppForThat/luperiq-platform.git
cd luperiq-platform
cargo build
cargo test
```

## Creating a Custom Module

Modules are the primary extension mechanism in LuperIQ. Each module implements the `Module` trait:

```rust
use luperiq::module::{Module, ModuleContext, ModuleResult};

pub struct MyModule {
    // module state
}

impl Module for MyModule {
    fn name(&self) -> &str {
        "my-module"
    }

    fn init(&mut self, ctx: &ModuleContext) -> ModuleResult<()> {
        // initialization logic
        Ok(())
    }

    fn render(&self, ctx: &ModuleContext) -> ModuleResult<String> {
        // rendering logic
        Ok(String::new())
    }
}
```

Register your module with the application:

```rust
let mut app = LuperiqApp::builder()
    .with_module(MyModule::default())
    .build()?;
```

See the [examples](examples/) directory for complete module implementations.

## Code Style

We follow standard Rust conventionsions:

- **Formatting**: Run `cargo fmt` before committing
- **Linting**: Run `cargo clippy` and address all warnings
- **Documentation**: Public APIs must have doc comments (`///`)
- **Tests**: Include unit tests for new functionality

```bash
cargo fmt --all
cargo clippy --all-targets --all-features -- -D warnings
cargo test
```

## Pull Request Process

1. **Fork the repository** and create a feature branch from `main`
2. **Make your changes** following the code style guidelines
3. **Add tests** for new functionality
4. **Update documentation** if you're changing public APIs
5. **Run the full test suite**:
   ```bash
   cargo test --all
   cargo clippy --all-targets --all-features -- -D warnings
   cargo fmt --all --check
   ```
6. **Submit a pull request** with a clear description of the changes

### PR Guidelines

- Keep PRs focused on a single change
- Write clear commit messages
- Link related issues if applicable
- Respond to review feedback promptly

## Reporting Issues

- Use GitHub Issues for bug reports and feature requests
- Include reproduction steps for bugs
- Provide context about your environment (Rust version, OS)

## License

By contributing to LuperIQ Platform, you agree that your contributions will be licensed under the Apache License, Version 2.0.
