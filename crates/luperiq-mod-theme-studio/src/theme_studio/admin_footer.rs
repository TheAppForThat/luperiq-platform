//! Footer builder sub-tab for the unified Design Studio.
//!
//! Controls: enabled toggle, visual layout builder, sticky bar
//! (text, schedule/portal/call buttons).
//!
//! AI context: footer layout, sticky bar text, button visibility.

/// Returns the footer sub-tab JS.
pub fn footer_js() -> &'static str {
    FOOTER_JS
}

const FOOTER_JS: &str = r####"
/* ── Footer sub-tab ───────────────────────────────────────────────── */

TsStudio.loadFooter = async function() {
    var controls = document.getElementById('tsStudioControls');
    if (!controls) return;

    var profile = TsStudio._profile;
    var slug = TsStudio._slug;
    if (!profile || !slug) {
        controls.appendChild(tsEmpty('No active profile loaded.'));
        return;
    }
    var footer = profile.footer || { enabled: true, layout_builder: [] };
    /* Ensure profile reference stays current */
    profile.footer = footer;

    /* ── AI context registration ──────────────────────────────────── */
    if (window.LiqAI && window.LiqAI.panel) {
        window.LiqAI.panel.register({
            featureKey: 'ts_footer',
            placeholder: "Describe your ideal footer, e.g. 'Simple footer with contact info, social links, and a sticky call-to-action bar'",
            creditCost: 2,
            properties: [
                { key: 'enabled', label: 'Enabled', group: 'Footer', tip: 'Show or hide the site footer' },
                { key: 'sticky_text', label: 'Sticky Bar Text', group: 'Sticky Bar', tip: 'Text displayed in the sticky bottom bar' },
                { key: 'show_schedule', label: 'Schedule Button', group: 'Sticky Bar', tip: 'Show scheduling/appointment button' },
                { key: 'show_portal', label: 'Portal Button', group: 'Sticky Bar', tip: 'Show customer portal button' },
                { key: 'show_call', label: 'Call Button', group: 'Sticky Bar', tip: 'Show click-to-call button' },
            ],
            onResult: function(result, checkedKeys) {
                if (typeof result !== 'object') return;
                TsStudio.pushSnapshot('AI footer');
                Object.keys(result).forEach(function(k) {
                    if (checkedKeys && checkedKeys.indexOf(k) < 0) return;
                    if (k === 'enabled') enabledCb.checked = !!result[k];
                    else if (k === 'sticky_text') stickyText.value = result[k] || '';
                    else if (k === 'show_schedule') showScheduleCb.checked = !!result[k];
                    else if (k === 'show_portal') showPortalCb.checked = !!result[k];
                    else if (k === 'show_call') showCallCb.checked = !!result[k];
                });
                syncFooterToProfile();
                TsStudio.markDirty();
                TsStudio.refreshBuilderPreview('footer', footer.layout_builder);
            },
            captureState: function() { return JSON.parse(JSON.stringify(footer)); },
            restoreState: function(snap) { /* restore footer from snap */ }
        });
    }

    /* Sync form values into TsStudio._profile.footer */
    function syncFooterToProfile() {
        footer.enabled = enabledCb.checked;
        footer.sticky_bar = {
            text: stickyText.value,
            show_schedule: showScheduleCb.checked,
            show_portal: showPortalCb.checked,
            show_call: showCallCb.checked,
            buttons: (footer.sticky_bar || {}).buttons || []
        };
        TsStudio._profile.footer = footer;
    }

    /* Enabled toggle */
    var toggleCard = tsCard();
    var enabledCb = document.createElement('input');
    enabledCb.type = 'checkbox';
    enabledCb.checked = footer.enabled !== false;
    enabledCb.addEventListener('change', function() {
        TsStudio.pushSnapshot('footer enabled');
        syncFooterToProfile();
        TsStudio.markDirty();
    });
    toggleCard.appendChild(tsTokenRow('Enabled', enabledCb));
    controls.appendChild(toggleCard);

    /* Template picker */
    var rows = footer.layout_builder || [];
    controls.appendChild(tsTemplatePicker('footer', rows, function(newRows) {
        TsStudio.pushSnapshot('footer template');
        rows = newRows;
        footer.layout_builder = rows;
        TsStudio._profile.footer = footer;
        TsStudio.markDirty();
        TsStudio.switchTab('footer');
    }));

    /* Visual Layout Builder */
    var builderWrap = document.createElement('div');
    builderWrap.className = 'ts-builder-wrap';
    var footerCanvas = tsLayoutCanvas(rows, function(updated) {
        TsStudio.pushSnapshot('footer layout');
        footer.layout_builder = updated;
        TsStudio._profile.footer = footer;
        TsStudio.markDirty();
        TsStudio.refreshBuilderPreview('footer', updated);
    });
    builderWrap.appendChild(tsBlockPalette(function(block) {
        footerCanvas.tsAddBlock(block);
    }));
    builderWrap.appendChild(footerCanvas);
    /* Render builder in the right pane (builder area) instead of narrow left controls */
    var builderArea = document.getElementById('tsStudioBuilder');
    if (builderArea) builderArea.appendChild(builderWrap);
    else controls.appendChild(builderWrap);

    /* Sticky Bar config */
    var stickyCard = tsCard();
    stickyCard.style.marginTop = '16px';
    var stickyTitle = document.createElement('h3');
    stickyTitle.textContent = 'Sticky Bar';
    stickyCard.appendChild(stickyTitle);
    var stickyBar = footer.sticky_bar || { text: '', show_schedule: false, show_portal: false, show_call: false, buttons: [] };

    var stickyText = tsInput('text', stickyBar.text || '');
    stickyText.placeholder = 'Sticky bar text';
    stickyText.style.width = '100%';
    stickyText.addEventListener('input', function() {
        syncFooterToProfile();
        TsStudio.markDirty();
    });
    stickyCard.appendChild(tsTokenRow('Text', stickyText));

    var showScheduleCb = document.createElement('input');
    showScheduleCb.type = 'checkbox';
    showScheduleCb.checked = !!stickyBar.show_schedule;
    showScheduleCb.addEventListener('change', function() {
        TsStudio.pushSnapshot('footer sticky bar');
        syncFooterToProfile();
        TsStudio.markDirty();
    });
    stickyCard.appendChild(tsTokenRow('Show Schedule', showScheduleCb));

    var showPortalCb = document.createElement('input');
    showPortalCb.type = 'checkbox';
    showPortalCb.checked = !!stickyBar.show_portal;
    showPortalCb.addEventListener('change', function() {
        TsStudio.pushSnapshot('footer sticky bar');
        syncFooterToProfile();
        TsStudio.markDirty();
    });
    stickyCard.appendChild(tsTokenRow('Show Portal', showPortalCb));

    var showCallCb = document.createElement('input');
    showCallCb.type = 'checkbox';
    showCallCb.checked = !!stickyBar.show_call;
    showCallCb.addEventListener('change', function() {
        TsStudio.pushSnapshot('footer sticky bar');
        syncFooterToProfile();
        TsStudio.markDirty();
    });
    stickyCard.appendChild(tsTokenRow('Show Call', showCallCb));
    controls.appendChild(stickyCard);

    /* ── Mobile Responsive Config ──────────────────────────────────── */
    var mobileCard = tsCard();
    mobileCard.style.marginTop = '16px';
    var mobileTitle = document.createElement('h3');
    mobileTitle.textContent = 'Mobile Layout';
    mobileTitle.style.cssText = 'margin-bottom:12px;font-size:15px;color:var(--text);';
    mobileCard.appendChild(mobileTitle);

    var resp = footer.responsive || {};
    if (!footer.responsive) {
        footer.responsive = { mode: 'Simple', breakpoint: 480, stack_columns: true, center_content: false, hidden_blocks: [], column_order: [], mobile_layout: [] };
        resp = footer.responsive;
    }

    /* Stack columns toggle */
    var fStackCb = document.createElement('input');
    fStackCb.type = 'checkbox';
    fStackCb.checked = resp.stack_columns !== false;
    fStackCb.addEventListener('change', function() {
        TsStudio.pushSnapshot('footer mobile stack');
        resp.stack_columns = fStackCb.checked;
        syncFooterToProfile();
        TsStudio.markDirty();
    });
    mobileCard.appendChild(tsTokenRow('Stack columns on mobile', fStackCb));

    /* Center content toggle */
    var fCenterCb = document.createElement('input');
    fCenterCb.type = 'checkbox';
    fCenterCb.checked = !!resp.center_content;
    fCenterCb.addEventListener('change', function() {
        TsStudio.pushSnapshot('footer mobile center');
        resp.center_content = fCenterCb.checked;
        syncFooterToProfile();
        TsStudio.markDirty();
    });
    mobileCard.appendChild(tsTokenRow('Center content on mobile', fCenterCb));

    /* Breakpoint input */
    var fBpInput = tsInput('number', String(resp.breakpoint || 480));
    fBpInput.min = '320';
    fBpInput.max = '1024';
    fBpInput.step = '10';
    fBpInput.style.width = '80px';
    fBpInput.addEventListener('change', function() {
        TsStudio.pushSnapshot('footer breakpoint');
        resp.breakpoint = parseInt(fBpInput.value, 10) || 480;
        syncFooterToProfile();
        TsStudio.markDirty();
    });
    mobileCard.appendChild(tsTokenRow('Breakpoint (px)', fBpInput));

    controls.appendChild(mobileCard);

    /* Load initial preview */
    TsStudio.refreshBuilderPreview('footer', rows);
};

/* ── Legacy redirect — opens unified studio on Footer tab ────────── */
async function load_ts_footer() {
    TsStudio._pendingTab = 'footer';
    if (typeof navigateTo === 'function') { navigateTo('ts-studio'); return; }
    await load_ts_studio();
}
"####;
