//! Theme Studio Module — portable core logic.
//!
//! Contains: CSS generation, config types (design tokens, profiles, nav menus,
//! popups, sidebars, headers, footers), block registry, admin UI panels
//! (builder, footer, header, history, pages, presets, shared, sidebar,
//! studio, tokens), behaviors, import/export, nav menu CRUD, revisions,
//! schedules, smart block defaults, template engine, design playground.
//!
//! CMS-dependent code (module trait impl, routes, admin.rs main shell,
//! smart_blocks, starter, blocks renderer, layout renderer, nav_styles,
//! page_studio, popups renderer, sidebar renderer, interpolation,
//! ab_integration, floating_guide, builtin_blocks) stays in the glue
//! layer at luperiq-cms.

pub mod admin_builder;
pub mod admin_footer;
pub mod admin_header;
pub mod admin_history;
pub mod admin_pages;
pub mod admin_presets;
pub mod admin_shared;
pub mod admin_sidebar;
pub mod admin_studio;
pub mod admin_tokens;
pub mod behaviors;
pub mod block_registry;
pub mod config;
pub mod css;
pub mod import;
pub mod layout_themes;
pub mod nav;
pub mod page_customizations;
pub mod playground;
pub mod revisions;
pub mod schedules;
pub mod scope_style;
pub mod smart_block_defaults;
pub mod template_engine;
