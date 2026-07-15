# LuperIQ Platform

[![Build Status](https://img.shields.io/github/actions/workflow/status/TheAppForThat/luperiq-platform/ci.yml?branch=main)](https://github.com/TheAppForThat/luperiq-platform/actions)
[![License](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.75+-orange.svg)](https://www.rust-lang.org/)

A Rust-based platform for building industry-specific SaaS websites. LuperIQ provides a modular architecture for generating pages, blogs, directories, forms, and SEO-optimized content — with a clear path to a full hosted platform including AI features, payments, and industry-specific modules.

## Architecture

LuperIQ uses a layered architecture:

| Layer | Name | Responsibility |
|-------|------|----------------|
| L1 | Core | Module API, IQTags, content pipeline |
| L2 | Content | Blog engine, page generator, directory, themes |
| L3 | Engagement | SEO, forms, reporting |
| L4 | Commerce | Payments, checkout pipeline *(hosted)* |
| L5 | Intelligence | AI, walkthrough videos, TTS *(hosted)* |

## What's Included

- **Module API** — build and register custom modules
- **Blog Engine** — markdown-based posts with metadata
- **Page Generator** — template-driven page creation
- **SEO** — structured data, sitemap, meta generation
- **Forms** — configurable form builder with validation
- **Directory** — listing and category management
- **Content Pipeline** — content ingestion and transformation
- **Reporting** — page views, engagement metrics
- **IQTags** — semantic content tagging system
- **Themes** — customizable theme system

## What's on the Hosted Platform

The [hosted LuperIQ platform](https://luperiq.com) adds:

- **Forge** — event-sourced WAL persistence engine
- **Payments** — Stripe Connect / Square integration, checkout pipeline
- **Commerce Spine** — multi-product cart, subscriptions, invoicing
- **AI (Cortex)** — walkthrough video generation, voice/TTS, content AI
- **Industry Modules** — pest control, HVAC, plumbing, restaurant, family services
- **Multi-tenant Orchestration** — tenant isolation, routing, provisioning
- **Encryption Vault** — secrets management, PCI-scoped token storage

See [PUBLIC-SCOPE.md](PUBLIC-SCOPE.md) for a detailed breakdown.

## Quick Start

```bash
# Clone the repository
git clone https://github.com/TheAppForThat/luperiq-platform.git
cd luperiq-platform

# Build
cargo build --release

# Run the basic example
cargo run --example basic-site
```

### Minimal Example

```rust
use luperiq::prelude::*;

fn main() -> Result<()> {
    let mut app = LuperiqApp::builder()
        .with_module(BlogModule::default())
        .with_module(PageModule::default())
        .with_module(SeoModule::default())
        .with_module(FormModule::default())
        .with_theme("default")
        .build()?;

    app.generate()?;
    println!("Site generated in ./output");
    Ok(())
}
```

See the [examples](examples/) directory for more complete samples.

## Documentation

- [Public vs. Commercial Scope](PUBLIC-SCOPE.md)
- [Contributing Guide](CONTRIBUTING.md)
- [Module API Reference](docs/module-api/) *(coming soon)*

## Hosted Platform

For the full LuperIQ experience — AI-generated walkthrough videos, payment processing, industry-specific modules, and managed hosting — visit [luperiq.com](https://luperiq.com).

## License

Licensed under the Apache License, Version 2.0 ([LICENSE](LICENSE) or http://www.apache.org/licenses/LICENSE-2.0).
