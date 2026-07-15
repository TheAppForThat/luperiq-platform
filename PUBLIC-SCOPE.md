# Public vs. Commercial Scope

This document describes what is included in the public open-source repository and what is available exclusively through the hosted LuperIQ platform at [luperiq.com](https://luperiq.com).

## Public Repository (Apache 2.0)

The public repository provides the foundation for building a basic website — enough to evaluate the architecture, build a blog or brochure site, and develop custom modules.

### Included

| Component | Description |
|-----------|-------------|
| **Module API** | Trait-based module system, registration, lifecycle hooks |
| **Blog Engine** | Markdown posts, frontmatter metadata, categories, tags |
| **Page Generator** | Template-driven page creation with layout composition |
| **SEO** | Structured data (JSON-LD), sitemap.xml, meta tag generation |
| **Forms** | Configurable form builder with field validation |
| **Directory** | Listing management, categories, search |
| **Content Pipeline** | Content ingestion, transformation, and output rendering |
| **Reporting** | Page views, basic engagement metrics |
| **IQTags** | Semantic content tagging and classification system |
| **Themes** | Customizable theme system with template overrides |
| **Examples** | Sample sites and module implementations |

### What You Can Build

- Personal or business blogs
- Brochure/marketing websites
- Directory sites
- Sites with contact forms and basic SEO
- Custom modules that integrate with the module API

## Commercial Platform (luperiq.com)

The hosted platform adds production-grade persistence, payments, AI, and industry-specific features.

### Included (in addition to everything above)

| Component | Description |
|-----------|-------------|
| **Forge** | Event-sourced WAL persistence engine with full audit trail |
| **Payments** | Stripe Connect and Square integration |
| **Commerce Spine** | Multi-product cart, subscriptions, invoicing, receipts |
| **Checkout Pipeline** | Multi-step checkout, tax calculation, discount engine |
| **AI (Cortex)** | Walkthrough video generation, voice/TTS, content AI |
| **Industry Modules** | Pest control, HVAC, plumbing, restaurant, family services |
| **Multi-tenant Hosting** | Tenant isolation, subdomain routing, provisioning |
| **Encryption Vault** | Secrets management, PCI-scoped token storage |

### Industry Modules

Each industry module includes pre-built content, forms, workflows, and integrations specific to that vertical:

- **Pest Control** — inspection reports, treatment scheduling, service area mapping
- **HVAC** — equipment tracking, maintenance schedules, efficiency calculators
- **Plumbing** — emergency dispatch, job tracking, parts inventory
- **Restaurant** — menus, reservations, online ordering
- **Family Services** — activity scheduling, provider matching, progress tracking

## Revenue Model

LuperIQ follows a WordPress-inspired model:

1. **Open Source** — the public repo gives away the website builder (blog, pages, SEO, forms, themes). Anyone can self-host a basic site.
2. **Hosted Platform** — the commercial platform at luperiq.com charges for AI features, payment processing, industry modules, and managed multi-tenant hosting.
3. **Revenue Share** — industry module developers can publish modules to the marketplace and earn revenue share.

## Getting the Commercial Platform

Visit [luperiq.com](https://luperiq.com) to sign up for the hosted platform. Plans include:

- **Starter** — basic hosted site with SEO and forms
- **Professional** — adds AI features, payments, and one industry module
- **Enterprise** — full platform access, all industry modules, priority support, custom SLAs
