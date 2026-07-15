//! Module registry — collects registered CMS modules and provides
//! aggregated access to routes, admin views, JS, and CSS.

use crate::context::AppContext;
use crate::module_trait::AdminView;
use axum::Router;

use super::CmsModule;

// ── Built-in system view IDs ────────────────────────────────────────────────
// These four IDs are injected into the System sidebar section unconditionally
// (unless blocked). Defining them as constants prevents the string from being
// re-spelled at every call site and lets callers reference them in blocklists.
pub const VIEW_MODULE_MANAGER: &str = "module-manager";
pub const VIEW_BACKUP: &str = "backup";
pub const VIEW_GRAPHQL: &str = "graphql";
pub const VIEW_HEALTH: &str = "health";

/// A single missing-dependency violation: module `slug` declared a dependency
/// on `missing_dep`, which is not present in the registry. Collected (not
/// panicked) so callers can see ALL gaps at once.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DepError {
    pub slug: String,
    pub missing_dep: String,
}

/// Collects registered modules and provides aggregated access.
pub struct ModuleRegistry {
    modules: Vec<Box<dyn CmsModule>>,
}

impl ModuleRegistry {
    pub fn new() -> Self {
        Self { modules: vec![] }
    }
}

impl Default for ModuleRegistry {
    /// Delegates to `ModuleRegistry::new()` so `ModuleRegistry::default()` and
    /// struct-spread patterns (e.g. in test scaffolds) work without repetition.
    fn default() -> Self {
        Self::new()
    }
}

impl ModuleRegistry {

    /// Register a module. Panics if a duplicate slug is found.
    pub fn register(&mut self, module: Box<dyn CmsModule>) {
        let slug = module.slug().to_string();
        if self.modules.iter().any(|m| m.slug() == slug) {
            panic!("Duplicate module slug: {slug}");
        }
        self.modules.push(module);
    }

    /// Non-panicking dependency check. Returns `Err` with EVERY missing-dep
    /// violation (not just the first), so a multi-dep gap surfaces fully and
    /// the dep-gate CI test can print a clear, complete message.
    ///
    /// `validate_dependencies()` is a thin panic-on-`Err` wrapper over this, so
    /// boot behavior is unchanged.
    pub fn validate_dependencies_result(&self) -> Result<(), Vec<DepError>> {
        let slugs: Vec<&str> = self.modules.iter().map(|m| m.slug()).collect();
        let mut errors: Vec<DepError> = Vec::new();
        for m in &self.modules {
            for dep in m.dependencies() {
                if !slugs.contains(dep) {
                    errors.push(DepError {
                        slug: m.slug().to_string(),
                        missing_dep: (*dep).to_string(),
                    });
                }
            }
        }
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    /// Validate that all declared dependencies are present.
    /// Panics on the first missing dependency (boot-time fail-fast). Delegates
    /// to `validate_dependencies_result` so there is one source of truth.
    pub fn validate_dependencies(&self) {
        if let Err(errors) = self.validate_dependencies_result() {
            let e = &errors[0];
            panic!(
                "Module '{}' depends on '{}' which is not registered",
                e.slug, e.missing_dep
            );
        }
    }

    /// Merge all module routes into a single Router.
    pub fn routes(&self, ctx: &AppContext) -> Router {
        let mut app = Router::new();
        for m in &self.modules {
            if let Some(r) = m.routes(ctx) {
                app = app.merge(r);
            }
        }
        app
    }

    /// Merge module routes excluding the given slugs.
    /// Use this to keep certain module routes out of an auth-wrapped block
    /// so they can be added separately as public routes.
    pub fn routes_excluding(&self, ctx: &AppContext, exclude_slugs: &[&str]) -> Router {
        let mut app = Router::new();
        for m in &self.modules {
            if exclude_slugs.contains(&m.slug()) {
                continue;
            }
            if let Some(r) = m.routes(ctx) {
                app = app.merge(r);
            }
        }
        app
    }

    /// Generate sidebar HTML for all module admin views (collapsible sections).
    ///
    /// Sections are ordered by priority so that domain-specific modules
    /// (Family Website, Industry, Repair OS) appear first, followed by content,
    /// design, and system sections. Within a priority tier, sections are
    /// sorted alphabetically.
    pub fn sidebar_html(&self, terms: Option<&luperiq_forge::GroupTerminology>) -> String {
        self.sidebar_html_filtered(&[], terms)
    }

    /// Render sidebar HTML, excluding views whose IDs are in the blocklist.
    /// When `terms` is provided, section headings and module labels are
    /// overridden with the group terminology config.
    pub fn sidebar_html_filtered(
        &self,
        blocked_views: &[&str],
        terms: Option<&luperiq_forge::GroupTerminology>,
    ) -> String {
        let is_blocked = |id: &str| blocked_views.contains(&id);
        /// Maps module admin-view IDs to terminology concept keys.
        const MODULE_CONCEPT_MAP: &[(&str, &str)] = &[
            ("chore-board", "task_board"),
            ("calendar", "calendar"),
            ("family-calendar", "calendar"),
            ("family-recipes", "resources"),
            ("family-feed", "feed"),
            ("family-vault", "vault"),
            ("family-shopping", "shopping"),
            ("family-games", "games"),
            ("family-members", "members"),
        ];
        let mut sections: std::collections::BTreeMap<String, Vec<AdminView>> =
            std::collections::BTreeMap::new();
        for m in &self.modules {
            for view in m.admin_views() {
                if !blocked_views.contains(&view.id.as_str()) {
                    sections.entry(view.section.clone()).or_default().push(view);
                }
            }
        }

        // Priority ordering: lower number = higher in the sidebar.
        // Domain-specific sections come first so they're the most visible.
        let section_priority = |name: &str| -> u32 {
            match name {
                // Domain-specific sections first (the main reason the customer uses this CMS)
                "Family Website" | "Family Hub" => 10,
                "Repair OS" => 10,
                "Industry" => 10,
                "Operations" => 20,
                "Customers" => 25,
                // Content and design sections
                "Content" => 30,
                "Design" => 35,
                "Media" => 36,
                "SEO" => 37,
                "Commerce" => 40,
                "Marketing" => 45,
                "Analytics" => 50,
                // Utility sections
                "Communication" => 60,
                "Data" => 65,
                "AI" | "AI & Automation" => 70,
                "Education" | "Learning" => 75,
                "Security" => 80,
                "Platform" => 85,
                "WordPress" => 88,
                "System" => 90,
                _ => 50, // Unknown sections in the middle
            }
        };

        // Check for System section before consuming the BTreeMap
        let has_system_section = sections.contains_key("System");

        // Sort sections by priority, then alphabetically within the same priority.
        let mut sorted_sections: Vec<_> = sections.into_iter().collect();
        sorted_sections.sort_by(|(a, _), (b, _)| {
            section_priority(a)
                .cmp(&section_priority(b))
                .then_with(|| a.cmp(b))
        });

        let mut html = String::new();
        for (section, views) in &sorted_sections {
            // Override legacy "Family Hub" with the terminology admin_section_label
            let display_section = if let Some(t) = terms {
                if section == "Family Hub" {
                    t.admin_section_label.as_str()
                } else {
                    section.as_str()
                }
            } else {
                section.as_str()
            };
            let key = section.to_lowercase().replace(' ', "-");
            html.push_str(&format!(
                "<div class=\"section-toggle\" data-section=\"{}\">{}<span class=\"section-arrow\"></span></div>\n<div class=\"section-links\" data-section=\"{}\">\n",
                key, display_section, key
            ));
            for v in views {
                // Override module labels with terminology when available
                let display_label = if let Some(t) = terms {
                    MODULE_CONCEPT_MAP
                        .iter()
                        .find(|(slug, _)| *slug == v.id.as_str())
                        .and_then(|(_, concept)| {
                            if *concept == "members" {
                                Some(t.member_noun_plural.as_str())
                            } else {
                                t.modules.get(*concept).map(|m| m.label.as_str())
                            }
                        })
                        .unwrap_or(&v.label)
                } else {
                    &v.label
                };
                html.push_str(&format!(
                    "            <a href=\"#\" data-view=\"{}\">{}</a>\n",
                    v.id, display_label
                ));
            }
            // Append built-in system links at the end of the System section
            if key == "system" {
                if !is_blocked("module-manager") {
                    html.push_str(
                        "            <a href=\"#\" data-view=\"module-manager\">Module Manager</a>\n",
                    );
                }
                if !is_blocked("backup") {
                    html.push_str(
                        "            <a href=\"#\" data-view=\"backup\">Backup &amp; Restore</a>\n",
                    );
                }
                if !is_blocked("graphql") {
                    html.push_str(
                        "            <a href=\"/api/graphql\" target=\"_blank\">GraphQL</a>\n",
                    );
                }
                if !is_blocked("health") {
                    html.push_str("            <a href=\"/health\" target=\"_blank\">Health</a>\n");
                }
            }
            html.push_str("</div>\n");
        }
        // If no module created a System section, add it ourselves
        if !has_system_section {
            html.push_str("<div class=\"section-toggle\" data-section=\"system\">System<span class=\"section-arrow\"></span></div>\n");
            html.push_str("<div class=\"section-links\" data-section=\"system\">\n");
            if !is_blocked("module-manager") {
                html.push_str(
                    "            <a href=\"#\" data-view=\"module-manager\">Module Manager</a>\n",
                );
            }
            if !is_blocked("backup") {
                html.push_str(
                    "            <a href=\"#\" data-view=\"backup\">Backup &amp; Restore</a>\n",
                );
            }
            if !is_blocked("graphql") {
                html.push_str(
                    "            <a href=\"/api/graphql\" target=\"_blank\">GraphQL</a>\n",
                );
            }
            if !is_blocked("health") {
                html.push_str("            <a href=\"/health\" target=\"_blank\">Health</a>\n");
            }
            html.push_str("</div>\n");
        }
        html
    }

    /// Collect all module JavaScript snippets.
    pub fn admin_js(&self) -> String {
        let mut js = String::new();
        // Build the view metadata first so help/debug surfaces know what exists.
        js.push_str("// Module view handlers\n");
        js.push_str("window.__registeredModuleViews = window.__registeredModuleViews || {};\n");
        js.push_str("Object.assign(window.__registeredModuleViews, {\n");
        for m in &self.modules {
            for view in m.admin_views() {
                let loader = format!("load_{}", view.id.replace('-', "_"));
                js.push_str(&format!(
                    "  '{}': {{ loader: '{}', module_slug: '{}', module_name: '{}', label: '{}' }},\n",
                    view.id,
                    loader,
                    m.slug(),
                    m.name().replace('\'', "\\'"),
                    view.label.replace('\'', "\\'")
                ));
            }
        }
        js.push_str("});\n\n");

        // Append each module's JS before wiring handlers so we do not rely on
        // function hoisting or throw TDZ errors for future const/let loaders.
        for m in &self.modules {
            if let Some(code) = m.admin_js() {
                js.push_str(&format!("// ── Module: {} ──\n", m.name()));
                js.push_str(&code);
                js.push_str("\n\n");
            }
        }

        js.push_str("window.moduleViews = window.moduleViews || {};\n");
        js.push_str("Object.assign(moduleViews, {\n");
        for m in &self.modules {
            for view in m.admin_views() {
                js.push_str(&format!(
                    "  '{}': (typeof load_{} === 'function' ? load_{} : undefined),\n",
                    view.id,
                    view.id.replace('-', "_"),
                    view.id.replace('-', "_")
                ));
            }
        }
        js.push_str("});\n\n");

        js.push_str(
            r#"
window.__missingModuleViews = Object.keys(window.__registeredModuleViews || {}).filter(function(viewId) {
  return !(window.moduleViews && typeof window.moduleViews[viewId] === 'function');
}).map(function(viewId) {
  var meta = window.__registeredModuleViews[viewId] || {};
  return {
    id: viewId,
    loader: meta.loader || '',
    module_slug: meta.module_slug || '',
    module_name: meta.module_name || '',
    label: meta.label || viewId
  };
});
if (window.__missingModuleViews.length) {
  console.warn('Missing admin module view handlers:', window.__missingModuleViews);
}

"#,
        );
        js
    }

    /// Build JS objects mapping view IDs to module slugs and names (for context-aware help).
    pub fn view_module_map_js(&self) -> String {
        let mut js = String::from("window.__viewModuleMap = {\n");
        for m in &self.modules {
            for view in m.admin_views() {
                js.push_str(&format!("  '{}': '{}',\n", view.id, m.slug()));
            }
        }
        js.push_str("};\nwindow.__moduleNames = {\n");
        for m in &self.modules {
            js.push_str(&format!("  '{}': '{}',\n", m.slug(), m.name()));
        }
        js.push_str("};\n");
        // Include registered view metadata for diagnostics (missing handler detection)
        js.push_str("window.__registeredModuleViews = window.__registeredModuleViews || {};\n");
        js.push_str("Object.assign(window.__registeredModuleViews, {\n");
        for m in &self.modules {
            for view in m.admin_views() {
                let loader = format!("load_{}", view.id.replace('-', "_"));
                js.push_str(&format!(
                    "  '{}': {{ loader: '{}', module_slug: '{}', module_name: '{}', label: '{}' }},\n",
                    view.id,
                    loader,
                    m.slug(),
                    m.name().replace('\'', "\\'"),
                    view.label.replace('\'', "\\'")
                ));
            }
        }
        js.push_str("});\n");
        // Module dependency map: slug -> [dep_slug, ...]
        js.push_str("window.__moduleDeps = {\n");
        for m in &self.modules {
            let deps = m.admin_js_deps();
            if !deps.is_empty() {
                let dep_list: Vec<String> = deps.iter().map(|d| format!("'{d}'")).collect();
                js.push_str(&format!("  '{}': [{}],\n", m.slug(), dep_list.join(",")));
            }
        }
        js.push_str("};\n");
        js
    }

    /// Collect all module CSS snippets.
    pub fn admin_css(&self) -> String {
        let mut css = String::new();
        for m in &self.modules {
            if let Some(code) = m.admin_css() {
                css.push_str(&code);
                css.push_str("\n");
            }
        }
        css
    }

    /// Build a map of module slug -> admin JS for lazy-loading via /admin/js/{slug}.js.
    /// Each module's JS is wrapped with self-registration code so views are
    /// automatically wired into `window.moduleViews` when the script loads.
    pub fn module_js_map(&self) -> std::collections::HashMap<String, String> {
        let mut map = std::collections::HashMap::new();
        for m in &self.modules {
            if let Some(js) = m.admin_js() {
                let mut wrapped = js;
                // Self-register views into moduleViews when loaded lazily
                let views = m.admin_views();
                if !views.is_empty() {
                    wrapped.push_str("\n// Auto-register views for lazy loading\n");
                    wrapped.push_str("window.moduleViews = window.moduleViews || {};\n");
                    for view in &views {
                        let loader = format!("load_{}", view.id.replace('-', "_"));
                        wrapped.push_str(&format!(
                            "if (typeof {} === 'function') {{ window.moduleViews['{}'] = {}; }}\n",
                            loader, view.id, loader
                        ));
                    }
                }
                map.insert(m.slug().to_string(), wrapped);
            }
        }
        map
    }

    /// List of registered modules (slug, name, version).
    pub fn list(&self) -> Vec<(&str, &str, &str)> {
        self.modules
            .iter()
            .map(|m| (m.slug(), m.name(), m.version()))
            .collect()
    }

    /// (slug, declared dependencies) for every registered module, OWNED so the
    /// result outlives any borrow of the registry. Mirrors `list()`. Used by
    /// the dep-gate tests to assert that every dependency-BEARING real
    /// provisioned module is modeled in the blueprint (forward CI guard against
    /// a future unmodeled provision module orphaning a dependency at boot).
    pub fn dependency_pairs(&self) -> Vec<(String, Vec<String>)> {
        self.modules
            .iter()
            .map(|m| {
                (
                    m.slug().to_string(),
                    m.dependencies().iter().map(|d| d.to_string()).collect(),
                )
            })
            .collect()
    }
}
