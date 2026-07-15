//! Header builder sub-tab for the unified Design Studio.
//!
//! Controls: enabled/sticky toggles, announcement bar (top bar),
//! template picker, rotating text, visual layout builder.
//!
//! AI context: header layout, announcement bar text, color scheme.

/// Returns the header sub-tab JS.
pub fn header_js() -> &'static str {
    HEADER_JS
}

const HEADER_JS: &str = r####"
/* ── Header sub-tab ───────────────────────────────────────────────── */

TsStudio.loadHeader = async function() {
    var controls = document.getElementById('tsStudioControls');
    if (!controls) return;

    var profile = TsStudio._profile;
    var slug = TsStudio._slug;
    if (!profile || !slug) {
        controls.appendChild(tsEmpty('No active profile loaded.'));
        return;
    }
    var header = profile.header || { enabled: true, sticky: false, top_bar: {}, layout_builder: [] };
    header.top_bar = header.top_bar || { enabled: false, text: '', link: null, bg_color: '#1e40af', text_color: '#ffffff', dismissible: true };
    /* Ensure profile reference stays current */
    profile.header = header;

    /* ── AI context registration ──────────────────────────────────── */
    if (window.LiqAI && window.LiqAI.panel) {
        window.LiqAI.panel.register({
            featureKey: 'ts_header',
            placeholder: "Describe your ideal header, e.g. 'Clean white header with centered logo and dropdown navigation'",
            creditCost: 2,
            properties: [
                { key: 'enabled', label: 'Enabled', group: 'Header', tip: 'Show or hide the site header' },
                { key: 'sticky', label: 'Sticky', group: 'Header', tip: 'Keep header fixed at top when scrolling' },
                { key: 'top_bar_enabled', label: 'Announcement Bar', group: 'Top Bar', tip: 'Show a colored bar above the header' },
                { key: 'top_bar_text', label: 'Bar Text', group: 'Top Bar', tip: 'Message displayed in the announcement bar' },
                { key: 'top_bar_link', label: 'Bar Link', group: 'Top Bar', tip: 'URL the announcement bar links to' },
                { key: 'top_bar_bg', label: 'Bar BG Color', group: 'Top Bar', tip: 'Background color of the announcement bar' },
                { key: 'top_bar_text_color', label: 'Bar Text Color', group: 'Top Bar', tip: 'Text color in the announcement bar' },
            ],
            onResult: function(result, checkedKeys) {
                if (typeof result !== 'object') return;
                TsStudio.pushSnapshot('AI header');
                Object.keys(result).forEach(function(k) {
                    if (checkedKeys && checkedKeys.indexOf(k) < 0) return;
                    if (k === 'enabled') enabledCb.checked = !!result[k];
                    else if (k === 'sticky') stickyCb.checked = !!result[k];
                    else if (k === 'top_bar_enabled') tbEnabledCb.checked = !!result[k];
                    else if (k === 'top_bar_text') tbText.value = result[k] || '';
                    else if (k === 'top_bar_link') tbLink.value = result[k] || '';
                    else if (k === 'top_bar_bg') tbBg.value = result[k] || '#1e40af';
                    else if (k === 'top_bar_text_color') tbColor.value = result[k] || '#ffffff';
                });
                syncHeaderToProfile();
                TsStudio.markDirty();
                TsStudio.refreshBuilderPreview('header', header.layout_builder);
            },
            captureState: function() { return JSON.parse(JSON.stringify(header)); },
            restoreState: function(snap) { /* restore header from snap */ }
        });
    }

    /* Sync form values into TsStudio._profile.header */
    function syncHeaderToProfile() {
        header.enabled = enabledCb.checked;
        header.sticky = stickyCb.checked;
        header.top_bar = {
            enabled: tbEnabledCb.checked,
            text: tbText.value,
            link: tbLink.value || null,
            bg_color: tbBg.value || '#1e40af',
            text_color: tbColor.value || '#ffffff',
            dismissible: tbDismissCb.checked
        };
        TsStudio._profile.header = header;
    }

    /* Enabled / Sticky toggles */
    var toggleCard = tsCard();
    var enabledCb = document.createElement('input');
    enabledCb.type = 'checkbox';
    enabledCb.checked = header.enabled !== false;
    enabledCb.addEventListener('change', function() {
        TsStudio.pushSnapshot('header enabled');
        syncHeaderToProfile();
        TsStudio.markDirty();
    });
    toggleCard.appendChild(tsTokenRow('Enabled', enabledCb));

    var stickyCb = document.createElement('input');
    stickyCb.type = 'checkbox';
    stickyCb.checked = !!header.sticky;
    stickyCb.addEventListener('change', function() {
        TsStudio.pushSnapshot('header sticky');
        syncHeaderToProfile();
        TsStudio.markDirty();
    });
    toggleCard.appendChild(tsTokenRow('Sticky', stickyCb));
    controls.appendChild(toggleCard);

    /* Top Bar (Announcement Bar) */
    var tbCard = tsCard();
    tbCard.style.marginTop = '16px';
    var tbTitle = document.createElement('h3');
    tbTitle.textContent = 'Announcement Bar (Top Bar)';
    tbTitle.style.cssText = 'margin-bottom:12px;font-size:15px;color:var(--text);';
    tbCard.appendChild(tbTitle);

    var tbEnabledCb = document.createElement('input');
    tbEnabledCb.type = 'checkbox';
    tbEnabledCb.checked = !!header.top_bar.enabled;
    tbEnabledCb.addEventListener('change', function() {
        TsStudio.pushSnapshot('top bar enabled');
        syncHeaderToProfile();
        TsStudio.markDirty();
    });
    tbCard.appendChild(tsTokenRow('Enabled', tbEnabledCb));

    var tbText = tsInput('text', header.top_bar.text || '');
    tbText.placeholder = 'Let AI Build Your Site in Minutes \u2192';
    tbText.style.width = '100%';
    tbText.addEventListener('input', function() {
        syncHeaderToProfile();
        TsStudio.markDirty();
    });
    tbCard.appendChild(tsTokenRow('Text', tbText));

    var tbLink = tsInput('text', header.top_bar.link || '');
    tbLink.placeholder = '/get-started';
    tbLink.style.width = '100%';
    tbLink.addEventListener('input', function() {
        syncHeaderToProfile();
        TsStudio.markDirty();
    });
    tbCard.appendChild(tsTokenRow('Link URL', tbLink));

    var tbBg = tsInput('color', header.top_bar.bg_color || '#1e40af');
    tbBg.addEventListener('input', function() {
        TsStudio.pushSnapshot('top bar bg');
        syncHeaderToProfile();
        TsStudio.markDirty();
    });
    tbCard.appendChild(tsTokenRow('Background', tbBg));

    var tbColor = tsInput('color', header.top_bar.text_color || '#ffffff');
    tbColor.addEventListener('input', function() {
        TsStudio.pushSnapshot('top bar text color');
        syncHeaderToProfile();
        TsStudio.markDirty();
    });
    tbCard.appendChild(tsTokenRow('Text Color', tbColor));

    var tbDismissCb = document.createElement('input');
    tbDismissCb.type = 'checkbox';
    tbDismissCb.checked = header.top_bar.dismissible !== false;
    tbDismissCb.addEventListener('change', function() {
        syncHeaderToProfile();
        TsStudio.markDirty();
    });
    tbCard.appendChild(tsTokenRow('Dismissible', tbDismissCb));

    controls.appendChild(tbCard);

    /* Template picker */
    var rows = header.layout_builder || [];
    controls.appendChild(tsTemplatePicker('header', rows, function(newRows) {
        TsStudio.pushSnapshot('header template');
        rows = newRows;
        header.layout_builder = rows;
        TsStudio._profile.header = header;
        TsStudio.markDirty();
        TsStudio.switchTab('header');
    }));

    var rotatingNote = tsCard();
    rotatingNote.style.marginTop = '16px';
    var rotatingTitle = document.createElement('h3');
    rotatingTitle.textContent = 'Rotating Text Tip';
    rotatingTitle.style.cssText = 'margin-bottom:8px;font-size:15px;color:var(--text);';
    rotatingNote.appendChild(rotatingTitle);
    var rotatingCopy = document.createElement('p');
    rotatingCopy.style.cssText = 'margin:0;color:var(--muted);line-height:1.6;';
    rotatingCopy.textContent = 'Add a Rotating Text block from the Content palette to place a small swapping headline anywhere in the header. Use [[rotate]] inside Line One or Line Two where the changing word should appear, then fine-tune font size, family, weight, spacing, color, and minimum word width in the block settings so the header stays steady.';
    rotatingNote.appendChild(rotatingCopy);
    controls.appendChild(rotatingNote);

    /* Visual Layout Builder (palette + canvas side-by-side) */
    var builderWrap = document.createElement('div');
    builderWrap.className = 'ts-builder-wrap';
    var headerCanvas = tsLayoutCanvas(rows, function(updated) {
        TsStudio.pushSnapshot('header layout');
        header.layout_builder = updated;
        TsStudio._profile.header = header;
        TsStudio.markDirty();
        TsStudio.refreshBuilderPreview('header', updated);
    });
    builderWrap.appendChild(tsBlockPalette(function(block) {
        headerCanvas.tsAddBlock(block);
    }));
    builderWrap.appendChild(headerCanvas);
    /* Render builder in the right pane (builder area) instead of narrow left controls */
    var builderArea = document.getElementById('tsStudioBuilder');
    if (builderArea) builderArea.appendChild(builderWrap);
    else controls.appendChild(builderWrap);
    /* ── Mobile Responsive Config ──────────────────────────────────── */
    var mobileCard = tsCard();
    mobileCard.style.marginTop = '16px';
    var mobileTitle = document.createElement('h3');
    mobileTitle.textContent = 'Mobile Layout';
    mobileTitle.style.cssText = 'margin-bottom:12px;font-size:15px;color:var(--text);';
    mobileCard.appendChild(mobileTitle);

    var resp = header.responsive || {};
    if (!header.responsive) {
        header.responsive = { mode: 'Simple', breakpoint: 480, stack_columns: true, center_content: false, hidden_blocks: [], column_order: [], mobile_layout: [] };
        resp = header.responsive;
    }

    /* Stack columns toggle */
    var stackCb = document.createElement('input');
    stackCb.type = 'checkbox';
    stackCb.checked = resp.stack_columns !== false;
    stackCb.addEventListener('change', function() {
        TsStudio.pushSnapshot('header mobile stack');
        resp.stack_columns = stackCb.checked;
        syncHeaderToProfile();
        TsStudio.markDirty();
    });
    mobileCard.appendChild(tsTokenRow('Stack columns on mobile', stackCb));

    /* Center content toggle */
    var centerCb = document.createElement('input');
    centerCb.type = 'checkbox';
    centerCb.checked = !!resp.center_content;
    centerCb.addEventListener('change', function() {
        TsStudio.pushSnapshot('header mobile center');
        resp.center_content = centerCb.checked;
        syncHeaderToProfile();
        TsStudio.markDirty();
    });
    mobileCard.appendChild(tsTokenRow('Center content on mobile', centerCb));

    /* Hide rotating text toggle */
    var hideRotCb = document.createElement('input');
    hideRotCb.type = 'checkbox';
    hideRotCb.checked = (resp.hidden_blocks || []).indexOf('rotating_text') >= 0;
    hideRotCb.addEventListener('change', function() {
        TsStudio.pushSnapshot('header hide rotating');
        var blocks = resp.hidden_blocks || [];
        var idx = blocks.indexOf('rotating_text');
        if (hideRotCb.checked && idx < 0) blocks.push('rotating_text');
        else if (!hideRotCb.checked && idx >= 0) blocks.splice(idx, 1);
        resp.hidden_blocks = blocks;
        syncHeaderToProfile();
        TsStudio.markDirty();
    });
    mobileCard.appendChild(tsTokenRow('Hide rotating text on mobile', hideRotCb));

    /* Hide top bar toggle */
    var hideTopCb = document.createElement('input');
    hideTopCb.type = 'checkbox';
    hideTopCb.checked = header.top_bar.hide_on_mobile !== false;
    hideTopCb.addEventListener('change', function() {
        TsStudio.pushSnapshot('header hide topbar mobile');
        header.top_bar.hide_on_mobile = hideTopCb.checked;
        syncHeaderToProfile();
        TsStudio.markDirty();
    });
    mobileCard.appendChild(tsTokenRow('Hide announcement bar on mobile', hideTopCb));

    /* Breakpoint input */
    var bpInput = tsInput('number', String(resp.breakpoint || 480));
    bpInput.min = '320';
    bpInput.max = '1024';
    bpInput.step = '10';
    bpInput.style.width = '80px';
    bpInput.addEventListener('change', function() {
        TsStudio.pushSnapshot('header breakpoint');
        resp.breakpoint = parseInt(bpInput.value, 10) || 480;
        syncHeaderToProfile();
        TsStudio.markDirty();
    });
    mobileCard.appendChild(tsTokenRow('Breakpoint (px)', bpInput));

    controls.appendChild(mobileCard);

    /* Load initial preview */
    TsStudio.refreshBuilderPreview('header', rows);
};

/* ── Legacy redirect — opens unified studio on Header tab ────────── */
async function load_ts_header() {
    TsStudio._pendingTab = 'header';
    if (typeof navigateTo === 'function') { navigateTo('ts-studio'); return; }
    await load_ts_studio();
}
"####;
