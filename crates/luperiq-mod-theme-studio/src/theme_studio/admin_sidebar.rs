//! Sidebar builder sub-tab for the unified Design Studio.
//!
//! Controls: enabled/sticky toggles, position (left/right), width,
//! visibility rules (pages/posts/products), presets, export/import.
//!
//! AI context: sidebar position, width, visibility rules.

/// Returns the sidebar sub-tab JS.
pub fn sidebar_js() -> &'static str {
    SIDEBAR_JS
}

const SIDEBAR_JS: &str = r####"
/* ── Sidebar sub-tab ──────────────────────────────────────────────── */

TsStudio.loadSidebar = async function() {
    var controls = document.getElementById('tsStudioControls');
    if (!controls) return;

    var profile = TsStudio._profile;
    var slug = TsStudio._slug;
    if (!profile || !slug) {
        controls.appendChild(tsEmpty('No active profile loaded.'));
        return;
    }
    var sb = profile.sidebars || {};
    /* Ensure profile reference stays current */
    profile.sidebars = sb;

    /* Upgrade banner */
    var ub = tsViewUpgradeBanner();
    if (ub) controls.appendChild(ub);

    /* ── AI context registration ──────────────────────────────────── */
    if (window.LiqAI && window.LiqAI.panel) {
        window.LiqAI.panel.register({
            featureKey: 'ts_sidebar',
            placeholder: "Describe your ideal sidebar, e.g. 'Right sidebar with contact info widget, recent posts, and a CTA button'",
            creditCost: 1,
            properties: [
                { key: 'enabled', label: 'Enabled', group: 'Sidebar', tip: 'Show or hide the sidebar' },
                { key: 'position', label: 'Position', group: 'Sidebar', tip: 'Left or right side of content' },
                { key: 'width', label: 'Width', group: 'Sidebar', tip: 'Sidebar width in pixels (200-600)' },
                { key: 'sticky', label: 'Sticky', group: 'Sidebar', tip: 'Keep sidebar visible when scrolling' },
                { key: 'show_pages', label: 'Show on Pages', group: 'Visibility', tip: 'Display sidebar on page content type' },
                { key: 'show_posts', label: 'Show on Posts', group: 'Visibility', tip: 'Display sidebar on blog posts' },
                { key: 'show_products', label: 'Show on Products', group: 'Visibility', tip: 'Display sidebar on product pages' },
            ],
            onResult: function(result, checkedKeys) {
                if (typeof result !== 'object') return;
                TsStudio.pushSnapshot('AI sidebar');
                Object.keys(result).forEach(function(k) {
                    if (checkedKeys && checkedKeys.indexOf(k) < 0) return;
                    if (k === 'enabled') enabledCb.checked = !!result[k];
                    else if (k === 'position') posSel.value = result[k] || 'Right';
                    else if (k === 'width') widthInp.value = result[k] || 340;
                    else if (k === 'sticky') stickyCb.checked = !!result[k];
                    else if (k === 'show_pages') pagesCb.checked = !!result[k];
                    else if (k === 'show_posts') postsCb.checked = !!result[k];
                    else if (k === 'show_products') productsCb.checked = !!result[k];
                });
                syncSidebarToProfile();
                TsStudio.markDirty();
            },
            captureState: function() { return JSON.parse(JSON.stringify(sb)); },
            restoreState: function(snap) { /* restore sidebar from snap */ }
        });
    }

    /* Sync form values into TsStudio._profile.sidebars */
    function syncSidebarToProfile() {
        TsStudio._profile.sidebars = {
            enabled: enabledCb.checked,
            position: posSel.value,
            width: parseInt(widthInp.value, 10) || 340,
            sticky: stickyCb.checked,
            show_on: {
                pages: pagesCb.checked,
                posts: postsCb.checked,
                products: productsCb.checked
            },
            blocks: sb.blocks || [],
            responsive: sb.responsive || { hide_on_mobile: true }
        };
        sb = TsStudio._profile.sidebars;
    }

    /* AI section */
    controls.appendChild(tsViewAiSection('ts_ai_sidebar',
        "Describe your sidebar layout, e.g. 'Right sidebar with contact info widget, recent posts, and a CTA button'", 1,
        function(result) {
            if (typeof result !== 'object') return;
            TsStudio.pushSnapshot('AI sidebar');
            if (result.position) posSel.value = result.position;
            if (result.width) widthInp.value = result.width;
            if (result.enabled !== undefined) enabledCb.checked = result.enabled;
            if (result.sticky !== undefined) stickyCb.checked = result.sticky;
            if (result.show_on) {
                if (result.show_on.pages !== undefined) pagesCb.checked = result.show_on.pages;
                if (result.show_on.posts !== undefined) postsCb.checked = result.show_on.posts;
                if (result.show_on.products !== undefined) productsCb.checked = result.show_on.products;
            }
            syncSidebarToProfile();
            TsStudio.markDirty();
            if (typeof showToast === 'function') showToast('AI sidebar config applied (save to persist)', 'success');
        }
    ));

    /* Presets */
    controls.appendChild(tsPresetPicker([
        { title: 'Classic Right', description: 'Right sidebar, 300px, sticky, show on pages.', summary: 'Right, 300px, sticky',
          data: { enabled: true, position: 'Right', width: 300, sticky: true, show_on: { pages: true, posts: false, products: false } }},
        { title: 'Blog Left', description: 'Left sidebar, 280px, show on posts.', summary: 'Left, 280px, posts only',
          data: { enabled: true, position: 'Left', width: 280, sticky: false, show_on: { pages: false, posts: true, products: false } }},
        { title: 'Minimal', description: 'Right sidebar, 240px, pages only, not sticky.', summary: 'Right, 240px, minimal',
          data: { enabled: true, position: 'Right', width: 240, sticky: false, show_on: { pages: true, posts: false, products: false } }}
    ], function(data) {
        TsStudio.pushSnapshot('sidebar preset');
        if (data.enabled !== undefined) enabledCb.checked = data.enabled;
        if (data.position) posSel.value = data.position;
        if (data.width) widthInp.value = data.width;
        if (data.sticky !== undefined) stickyCb.checked = data.sticky;
        if (data.show_on) {
            pagesCb.checked = !!data.show_on.pages;
            postsCb.checked = !!data.show_on.posts;
            productsCb.checked = !!data.show_on.products;
        }
        syncSidebarToProfile();
        TsStudio.markDirty();
        if (typeof showToast === 'function') showToast('Preset applied (save to persist)', 'success');
    }));

    /* Export / Import */
    controls.appendChild(tsExportImportBar(
        function() { return { filename: 'sidebar-config.json', data: TsStudio._profile.sidebars || {} }; },
        function(data) {
            if (typeof data !== 'object') return;
            TsStudio.pushSnapshot('import sidebar');
            if (data.enabled !== undefined) enabledCb.checked = data.enabled;
            if (data.position) posSel.value = data.position;
            if (data.width) widthInp.value = data.width;
            if (data.sticky !== undefined) stickyCb.checked = data.sticky;
            if (data.show_on) {
                pagesCb.checked = !!data.show_on.pages;
                postsCb.checked = !!data.show_on.posts;
                productsCb.checked = !!data.show_on.products;
            }
            syncSidebarToProfile();
            TsStudio.markDirty();
            if (typeof showToast === 'function') showToast('Sidebar config imported (save to persist)', 'success');
        }
    ));

    var card = tsCard();

    /* Enabled */
    var enabledCb = document.createElement('input');
    enabledCb.type = 'checkbox';
    enabledCb.checked = !!sb.enabled;
    enabledCb.addEventListener('change', function() {
        TsStudio.pushSnapshot('sidebar enabled');
        syncSidebarToProfile();
        TsStudio.markDirty();
    });
    card.appendChild(tsTokenRow('Enabled', enabledCb));

    /* Position */
    var posSel = tsSelect(['Right', 'Left'], sb.position || 'Right');
    posSel.addEventListener('change', function() {
        TsStudio.pushSnapshot('sidebar position');
        syncSidebarToProfile();
        TsStudio.markDirty();
    });
    card.appendChild(tsTokenRow('Position', posSel));

    /* Width */
    var widthInp = tsInput('number', sb.width || 340);
    widthInp.min = 200;
    widthInp.max = 600;
    widthInp.addEventListener('input', function() {
        syncSidebarToProfile();
        TsStudio.markDirty();
    });
    card.appendChild(tsTokenRow('Width (px)', widthInp));

    /* Sticky */
    var stickyCb = document.createElement('input');
    stickyCb.type = 'checkbox';
    stickyCb.checked = !!sb.sticky;
    stickyCb.addEventListener('change', function() {
        TsStudio.pushSnapshot('sidebar sticky');
        syncSidebarToProfile();
        TsStudio.markDirty();
    });
    card.appendChild(tsTokenRow('Sticky', stickyCb));

    /* Show-on checkboxes */
    var showOn = sb.show_on || { pages: true, posts: false, products: false };
    var showTitle = document.createElement('h3');
    showTitle.textContent = 'Show On';
    showTitle.style.cssText = 'margin-top:16px;margin-bottom:8px;';
    card.appendChild(showTitle);

    var pagesCb = document.createElement('input');
    pagesCb.type = 'checkbox';
    pagesCb.checked = showOn.pages !== false;
    pagesCb.addEventListener('change', function() {
        TsStudio.pushSnapshot('sidebar visibility');
        syncSidebarToProfile();
        TsStudio.markDirty();
    });
    card.appendChild(tsTokenRow('Pages', pagesCb));

    var postsCb = document.createElement('input');
    postsCb.type = 'checkbox';
    postsCb.checked = !!showOn.posts;
    postsCb.addEventListener('change', function() {
        TsStudio.pushSnapshot('sidebar visibility');
        syncSidebarToProfile();
        TsStudio.markDirty();
    });
    card.appendChild(tsTokenRow('Posts', postsCb));

    var productsCb = document.createElement('input');
    productsCb.type = 'checkbox';
    productsCb.checked = !!showOn.products;
    productsCb.addEventListener('change', function() {
        TsStudio.pushSnapshot('sidebar visibility');
        syncSidebarToProfile();
        TsStudio.markDirty();
    });
    card.appendChild(tsTokenRow('Products', productsCb));

    /* ── Mobile Responsive ─────────────────────────────────────────── */
    var mobileTitle = document.createElement('h3');
    mobileTitle.textContent = 'Mobile';
    mobileTitle.style.cssText = 'margin-top:16px;margin-bottom:8px;';
    card.appendChild(mobileTitle);

    var sidebarResp = sb.responsive || {};
    if (!sb.responsive) {
        sb.responsive = { hide_on_mobile: true };
        sidebarResp = sb.responsive;
    }

    var hideOnMobileCb = document.createElement('input');
    hideOnMobileCb.type = 'checkbox';
    hideOnMobileCb.checked = sidebarResp.hide_on_mobile !== false;
    hideOnMobileCb.addEventListener('change', function() {
        TsStudio.pushSnapshot('sidebar hide mobile');
        sidebarResp.hide_on_mobile = hideOnMobileCb.checked;
        sb.responsive = sidebarResp;
        syncSidebarToProfile();
        TsStudio.markDirty();
    });
    card.appendChild(tsTokenRow('Hide on mobile', hideOnMobileCb));

    controls.appendChild(card);
};

/* ── Legacy redirect — opens unified studio on Sidebar tab ───────── */
async function load_ts_sidebar() {
    TsStudio._pendingTab = 'sidebar';
    if (typeof navigateTo === 'function') { navigateTo('ts-studio'); return; }
    await load_ts_studio();
}
"####;
