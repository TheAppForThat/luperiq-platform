//! Design Tokens sub-tab for the unified Design Studio.
//!
//! Renders: 9 color pickers, 7 numeric inputs, font selector,
//! full bleed toggle, responsive overrides, preset picker,
//! export/import bar, and live preview CSS injection.
//!
//! AI context: registers 18 properties in 4 groups with tooltips.

/// Returns the design tokens sub-tab JS.
pub fn tokens_js() -> &'static str {
    TOKENS_JS
}

const TOKENS_JS: &str = r####"
/* ── Design Tokens sub-tab ─────────────────────────────────────────── */

TsStudio.loadTokens = async function() {
    var controls = document.getElementById('tsStudioControls');
    if (!controls) return;

    var profile = TsStudio._profile;
    var slug = TsStudio._slug;
    if (!profile || !slug) {
        controls.appendChild(tsEmpty('No active profile loaded.'));
        return;
    }
    var tokens = profile.tokens || {};

    /* Upgrade banner */
    var ub = tsViewUpgradeBanner();
    if (ub) controls.appendChild(ub);

    /* AI section — register with global AI panel instead of inline section */
    if (window.LiqAI && window.LiqAI.panel && typeof window.LiqAI.panel.register === 'function') {
        window.LiqAI.panel.register({
            featureKey: 'ts_ai_design_tokens',
            placeholder: "Describe your brand style, e.g. 'Modern minimalist with navy blue and warm gold accents'",
            creditCost: 2,
            properties: [
                { key: 'primary', label: 'Primary', group: 'Colors', tip: 'Main brand color for buttons, links, and key UI elements' },
                { key: 'accent', label: 'Accent', group: 'Colors', tip: 'Secondary color for hover states, badges, and highlights' },
                { key: 'link', label: 'Link', group: 'Colors', tip: 'Color for clickable text links' },
                { key: 'button_text', label: 'Button Text', group: 'Colors', tip: 'Text color inside primary buttons' },
                { key: 'header_bg', label: 'Header BG', group: 'Colors', tip: 'Header/navigation background color' },
                { key: 'header_text', label: 'Header Text', group: 'Colors', tip: 'Text and icon color in the header' },
                { key: 'background', label: 'Background', group: 'Colors', tip: 'Page body background color' },
                { key: 'surface', label: 'Surface', group: 'Colors', tip: 'Card and panel background color' },
                { key: 'text', label: 'Text', group: 'Colors', tip: 'Default body text color' },
                { key: 'radius', label: 'Border Radius', group: 'Sizes', tip: 'Corner rounding for buttons, cards, inputs (px)' },
                { key: 'container', label: 'Container Width', group: 'Sizes', tip: 'Maximum content width in pixels' },
                { key: 'brand_size', label: 'Brand Size', group: 'Sizes', tip: 'Logo/brand text size in the header (px)' },
                { key: 'nav_size', label: 'Nav Font', group: 'Sizes', tip: 'Navigation link font size (px)' },
                { key: 'nav_gap', label: 'Nav Gap', group: 'Sizes', tip: 'Spacing between navigation links (px)' },
                { key: 'body_size', label: 'Body Size', group: 'Sizes', tip: 'Default body text font size (px)' },
                { key: 'body_line_height', label: 'Line Height', group: 'Sizes', tip: 'Line height multiplier for body text' },
                { key: 'body_font', label: 'Body Font', group: 'Typography', tip: 'Font family for all body text' },
                { key: 'full_bleed', label: 'Full Bleed', group: 'Layout', tip: 'Remove container max-width for edge-to-edge content' }
            ],
            onResult: function(result, checkedKeys) {
                if (typeof result === 'object') {
                    TsStudio.pushSnapshot('AI tokens');
                    Object.keys(result).forEach(function(k) {
                        if (checkedKeys && checkedKeys.indexOf(k) < 0) return;
                        if (colorInputs[k]) colorInputs[k].value = result[k];
                        else if (numInputs[k]) numInputs[k].value = result[k];
                        else if (k === 'body_font') fontSel.value = result[k];
                        else if (k === 'full_bleed') fbCheck.checked = !!result[k];
                    });
                    syncTokensToProfile();
                    injectAllTokens();
                    TsStudio.markDirty();
                    if (typeof showToast === 'function') showToast('AI tokens applied to fields', 'success');
                }
            },
            captureState: function() {
                var snapshot = {};
                Object.keys(colorInputs).forEach(function(k) { snapshot[k] = colorInputs[k].value; });
                Object.keys(numInputs).forEach(function(k) { snapshot[k] = Number(numInputs[k].value || 0); });
                snapshot.body_font = fontSel.value;
                snapshot.full_bleed = fbCheck.checked;
                return snapshot;
            },
            restoreState: function(snapshot) {
                if (!snapshot || typeof snapshot !== 'object') return;
                Object.keys(colorInputs).forEach(function(k) {
                    if (snapshot[k] !== undefined) colorInputs[k].value = snapshot[k];
                });
                Object.keys(numInputs).forEach(function(k) {
                    if (snapshot[k] !== undefined) numInputs[k].value = snapshot[k];
                });
                if (snapshot.body_font !== undefined) fontSel.value = snapshot.body_font;
                if (snapshot.full_bleed !== undefined) fbCheck.checked = snapshot.full_bleed;
            }
        });
    }

    /* Presets */
    controls.appendChild(tsPresetPicker([
        { title: 'Clean Modern', description: 'Blue and white with rounded corners and system fonts.', summary: '8 colors, radius 8, System font',
          data: { primary: '#2563eb', accent: '#3b82f6', link: '#2563eb', button_text: '#ffffff', header_bg: '#ffffff', header_text: '#1e293b', background: '#f8fafc', surface: '#ffffff', text: '#1e293b', radius: 8, container: 1200, body_font: 'System', body_size: 16, body_line_height: 16, brand_size: 28, nav_size: 15, nav_gap: 24 }},
        { title: 'Bold & Warm', description: 'Orange and cream with generous rounding and humanist type.', summary: '8 colors, radius 12, Humanist font',
          data: { primary: '#ea580c', accent: '#f97316', link: '#ea580c', button_text: '#111111', header_bg: '#fff7ed', header_text: '#431407', background: '#fffbeb', surface: '#ffffff', text: '#292524', radius: 12, container: 1140, body_font: 'Humanist', body_size: 17, body_line_height: 17, brand_size: 30, nav_size: 15, nav_gap: 20 }},
        { title: 'Dark Professional', description: 'Slate tones on dark with tight spacing and geometric type.', summary: '8 colors, radius 4, Geometric font',
          data: { primary: '#6366f1', accent: '#818cf8', link: '#818cf8', button_text: '#111111', header_bg: '#0f172a', header_text: '#e2e8f0', background: '#0f172a', surface: '#1e293b', text: '#e2e8f0', radius: 4, container: 1280, body_font: 'Geometric', body_size: 15, body_line_height: 15, brand_size: 26, nav_size: 14, nav_gap: 28 }}
    ], function(data) {
        TsStudio.pushSnapshot('preset');
        Object.keys(data).forEach(function(k) {
            if (colorInputs[k]) colorInputs[k].value = data[k];
            else if (numInputs[k]) numInputs[k].value = data[k];
            else if (k === 'body_font') fontSel.value = data[k];
            else if (k === 'full_bleed') fbCheck.checked = !!data[k];
        });
        syncTokensToProfile();
        injectAllTokens();
        TsStudio.markDirty();
        if (typeof showToast === 'function') showToast('Preset applied (save to persist)', 'success');
    }));

    /* Export / Import bar */
    controls.appendChild(tsExportImportBar(
        function() { return { filename: 'design-tokens.json', data: TsStudio._profile.tokens || {} }; },
        function(data) {
            if (typeof data !== 'object') return;
            TsStudio.pushSnapshot('import tokens');
            Object.keys(data).forEach(function(k) {
                if (colorInputs[k]) colorInputs[k].value = data[k];
                else if (numInputs[k]) numInputs[k].value = data[k];
                else if (k === 'body_font') fontSel.value = data[k];
                else if (k === 'full_bleed') fbCheck.checked = !!data[k];
            });
            syncTokensToProfile();
            injectAllTokens();
            TsStudio.markDirty();
            if (typeof showToast === 'function') showToast('Tokens imported (save to persist)', 'success');
        }
    ));

    var info = document.createElement('p');
    info.style.cssText = 'color:var(--text-muted);font-size:13px;margin-bottom:16px;';
    info.textContent = 'Editing profile: ' + slug;
    controls.appendChild(info);

    /* Color pickers */
    var colorCard = tsCard();
    var colorTitle = document.createElement('h3');
    colorTitle.textContent = 'Colors';
    colorCard.appendChild(colorTitle);

    var colorFields = [
        { key: 'primary', label: 'Primary' },
        { key: 'accent', label: 'Accent' },
        { key: 'link', label: 'Link' },
        { key: 'button_text', label: 'Button Text' },
        { key: 'header_bg', label: 'Header BG' },
        { key: 'header_text', label: 'Header Text' },
        { key: 'background', label: 'Background' },
        { key: 'surface', label: 'Surface' },
        { key: 'text', label: 'Text' }
    ];

    var colorInputs = {};
    colorFields.forEach(function(f) {
        var inp = tsInput('color', tokens[f.key] || '#000000');
        inp.style.cssText = 'width:40px;height:30px;border:1px solid var(--border);border-radius:4px;cursor:pointer;';
        colorInputs[f.key] = inp;
        colorCard.appendChild(tsTokenRow(f.label, inp));
    });
    controls.appendChild(colorCard);

    /* Numeric inputs */
    var numCard = tsCard();
    numCard.style.marginTop = '16px';
    var numTitle = document.createElement('h3');
    numTitle.textContent = 'Sizes & Spacing';
    numCard.appendChild(numTitle);

    var numFields = [
        { key: 'radius', label: 'Border Radius', min: 0, max: 40 },
        { key: 'container', label: 'Container Width', min: 860, max: 1600 },
        { key: 'brand_size', label: 'Brand Size', min: 10, max: 80 },
        { key: 'nav_size', label: 'Nav Font Size', min: 10, max: 60 },
        { key: 'nav_gap', label: 'Nav Gap', min: 10, max: 60 },
        { key: 'body_size', label: 'Body Font Size', min: 12, max: 28 },
        { key: 'body_line_height', label: 'Line Height', min: 10, max: 24 }
    ];

    var numInputs = {};
    numFields.forEach(function(f) {
        var inp = tsInput('number', tokens[f.key] !== undefined ? tokens[f.key] : '');
        inp.min = f.min;
        inp.max = f.max;
        inp.style.cssText = 'width:80px;padding:4px 8px;border:1px solid var(--border);border-radius:4px;background:var(--bg);color:var(--text);';
        numInputs[f.key] = inp;
        numCard.appendChild(tsTokenRow(f.label, inp));
    });
    controls.appendChild(numCard);

    /* Font selector */
    var fontCard = tsCard();
    fontCard.style.marginTop = '16px';
    var fontTitle = document.createElement('h3');
    fontTitle.textContent = 'Typography';
    fontCard.appendChild(fontTitle);

    var fontOpts = ['System', 'Humanist', 'Transitional', 'OldStyle', 'Geometric', 'Mono'];
    var fontSel = tsSelect(fontOpts, tokens.body_font || 'System');
    fontCard.appendChild(tsTokenRow('Body Font', fontSel));
    controls.appendChild(fontCard);

    /* Layout options */
    var layoutCard = tsCard();
    layoutCard.style.marginTop = '16px';
    var layoutTitle = document.createElement('h3');
    layoutTitle.textContent = 'Layout';
    layoutCard.appendChild(layoutTitle);

    var fbRow = document.createElement('div');
    fbRow.className = 'ts-token-row';
    fbRow.style.cssText = 'display:flex;align-items:center;gap:10px;';
    var fbCheck = document.createElement('input');
    fbCheck.type = 'checkbox';
    fbCheck.id = 'ts_full_bleed';
    fbCheck.checked = tokens.full_bleed !== false;
    fbRow.appendChild(fbCheck);
    var fbLabel = document.createElement('label');
    fbLabel.htmlFor = 'ts_full_bleed';
    fbLabel.textContent = 'Full-bleed layout';
    fbLabel.style.cssText = 'font-weight:600;cursor:pointer;';
    fbRow.appendChild(fbLabel);
    layoutCard.appendChild(fbRow);
    var fbDesc = document.createElement('p');
    fbDesc.style.cssText = 'font-size:12px;color:var(--text-muted);margin:4px 0 0;';
    fbDesc.textContent = 'When enabled, page backgrounds extend edge-to-edge while content stays centered at the container width. Great for dark hero sections and full-width gradients.';
    layoutCard.appendChild(fbDesc);
    controls.appendChild(layoutCard);

    /* Responsive overrides (Tablet / Mobile) */
    var respCard = tsCard();
    respCard.style.marginTop = '16px';
    var respTitle = document.createElement('h3');
    respTitle.textContent = 'Responsive Overrides';
    respCard.appendChild(respTitle);

    var breakpointTabs = document.createElement('div');
    breakpointTabs.className = 'ts-breakpoint-tabs';
    var bpContent = document.createElement('div');
    var currentBp = 'tablet';

    var overrideFields = [
        { key: 'radius', label: 'Border Radius', min: 0, max: 40 },
        { key: 'container', label: 'Container Width', min: 860, max: 1600 },
        { key: 'brand_size', label: 'Brand Size', min: 10, max: 80 },
        { key: 'nav_size', label: 'Nav Font Size', min: 10, max: 60 },
        { key: 'nav_gap', label: 'Nav Gap', min: 10, max: 60 },
        { key: 'body_size', label: 'Body Font Size', min: 12, max: 28 },
        { key: 'body_line_height', label: 'Line Height', min: 10, max: 24 }
    ];
    var bpInputs = { tablet: {}, mobile: {} };

    function renderBpTab(bp) {
        currentBp = bp;
        /* Update tab active state */
        breakpointTabs.querySelectorAll('.ts-breakpoint-tab').forEach(function(t) {
            if (t.dataset.bp === bp) t.classList.add('is-active');
            else t.classList.remove('is-active');
        });

        while (bpContent.firstChild) bpContent.removeChild(bpContent.firstChild);
        var overrides = tokens[bp] || {};
        var desc = document.createElement('p');
        desc.style.cssText = 'font-size:12px;color:var(--text-muted);margin-bottom:8px;';
        desc.textContent = bp === 'tablet' ? 'Overrides for screens <= 980px. Leave empty to inherit desktop value.' : 'Overrides for screens <= 860px. Leave empty to inherit desktop value.';
        bpContent.appendChild(desc);

        overrideFields.forEach(function(f) {
            var row = document.createElement('div');
            row.className = 'ts-override-row ts-token-row';

            var lbl = tsLabel(f.label);
            row.appendChild(lbl);

            var val = overrides[f.key];
            var inp = tsInput('number', val !== undefined && val !== null ? val : '');
            inp.min = f.min;
            inp.max = f.max;
            inp.placeholder = 'inherit';
            inp.style.cssText = 'width:80px;padding:4px 8px;border:1px solid var(--border);border-radius:4px;background:var(--bg);color:var(--text);';
            inp.addEventListener('input', function() {
                syncTokensToProfile();
                TsStudio.markDirty();
            });
            bpInputs[bp][f.key] = inp;
            row.appendChild(inp);

            var clearBtn = document.createElement('button');
            clearBtn.className = 'ts-override-clear';
            clearBtn.textContent = '\u00D7';
            clearBtn.title = 'Clear (inherit desktop)';
            clearBtn.addEventListener('click', function() {
                inp.value = '';
                syncTokensToProfile();
                TsStudio.markDirty();
            });
            row.appendChild(clearBtn);

            bpContent.appendChild(row);
        });
    }

    ['tablet', 'mobile'].forEach(function(bp) {
        var tab = document.createElement('button');
        tab.className = 'ts-breakpoint-tab';
        tab.dataset.bp = bp;
        tab.textContent = bp.charAt(0).toUpperCase() + bp.slice(1) + (bp === 'tablet' ? ' (<= 980px)' : ' (<= 860px)');
        tab.addEventListener('click', function() { renderBpTab(bp); });
        breakpointTabs.appendChild(tab);
    });

    respCard.appendChild(breakpointTabs);
    respCard.appendChild(bpContent);
    renderBpTab('tablet');
    controls.appendChild(respCard);

    /* -- Live CSS injection bridge -- */
    var pvIframe = TsStudio.getPreviewIframe();

    var colorVarMap = {
        primary: '--luperiq-primary', accent: '--luperiq-accent',
        link: '--luperiq-link', button_text: '--luperiq-button-text',
        header_bg: '--luperiq-header-bg', header_text: '--luperiq-header-text',
        background: '--luperiq-background', surface: '--luperiq-surface',
        text: '--luperiq-text'
    };
    var colorAliasMap = {
        accent: '--accent', background: '--bg', surface: '--surface',
        text: '--text', header_bg: '--header-bg', header_text: '--header-text'
    };
    var numVarMap = {
        radius: { v: '--luperiq-radius', unit: 'px' },
        container: { v: '--luperiq-container', unit: 'px' },
        brand_size: { v: '--luperiq-brand-size', unit: 'px' },
        nav_size: { v: '--luperiq-nav-size', unit: 'px' },
        nav_gap: { v: '--luperiq-nav-gap', unit: 'px' },
        body_size: { v: '--luperiq-body-size', unit: 'px' },
        body_line_height: { v: '--luperiq-body-line-height', unit: '' }
    };
    var fontStacks = {
        System: '-apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif',
        Humanist: 'Optima, Candara, "Noto Sans", source-sans-pro, sans-serif',
        Transitional: 'Charter, "Bitstream Charter", "Sitka Text", Cambria, serif',
        OldStyle: '"Iowan Old Style", "Palatino Linotype", "URW Palladio L", P052, serif',
        Geometric: 'Avenir, Montserrat, Corbel, "URW Gothic", source-sans-pro, sans-serif',
        Mono: '"Cascadia Code", "Source Code Pro", Menlo, Consolas, monospace'
    };

    function injectVar(name, value) {
        try {
            var iframe = TsStudio.getPreviewIframe();
            var doc = iframe && iframe.contentWindow && iframe.contentWindow.document;
            if (doc) doc.documentElement.style.setProperty(name, value);
        } catch(e) {}
    }
    function injectAllTokens() {
        Object.keys(colorInputs).forEach(function(k) {
            var val = colorInputs[k].value;
            if (colorVarMap[k]) injectVar(colorVarMap[k], val);
            if (colorAliasMap[k]) injectVar(colorAliasMap[k], val);
        });
        var accentVal = colorInputs.accent ? colorInputs.accent.value : '';
        if (accentVal) { injectVar('--luperiq-accent-hover', accentVal); injectVar('--accent-hover', accentVal); }
        Object.keys(numInputs).forEach(function(k) {
            var m = numVarMap[k];
            if (m) {
                var val = numInputs[k].value;
                if (val !== '') injectVar(m.v, val + m.unit);
            }
        });
        var fontVal = fontStacks[fontSel.value] || fontStacks.System;
        injectVar('--luperiq-body-font', fontVal);
        var lhVal = numInputs.body_line_height ? numInputs.body_line_height.value : '';
        if (lhVal !== '') {
            var lhNum = parseInt(lhVal, 10);
            if (!isNaN(lhNum) && lhNum > 0) injectVar('--luperiq-body-line-height', (lhNum / 10).toFixed(1));
        }
    }

    /* Register reinject callback so studio shell can call it on iframe reload */
    TsStudio._reinjectPreview = injectAllTokens;

    /* Sync form values into TsStudio._profile.tokens */
    function syncTokensToProfile() {
        var updated = Object.assign({}, TsStudio._profile.tokens || {});
        colorFields.forEach(function(f) { updated[f.key] = colorInputs[f.key].value; });
        numFields.forEach(function(f) {
            var v = parseInt(numInputs[f.key].value, 10);
            if (!isNaN(v)) updated[f.key] = v;
        });
        updated.body_font = fontSel.value;
        updated.full_bleed = fbCheck.checked;

        /* Collect responsive overrides */
        ['tablet', 'mobile'].forEach(function(bp) {
            var overObj = {};
            var hasAny = false;
            overrideFields.forEach(function(f) {
                var inp = bpInputs[bp][f.key];
                if (inp && inp.value !== '') {
                    var v = parseInt(inp.value, 10);
                    if (!isNaN(v)) { overObj[f.key] = v; hasAny = true; }
                }
            });
            updated[bp] = hasAny ? overObj : null;
        });

        TsStudio._profile.tokens = updated;
    }

    /* Wire input change events — pushSnapshot, sync, inject, markDirty */
    colorFields.forEach(function(f) {
        colorInputs[f.key].addEventListener('input', function() {
            TsStudio.pushSnapshot('token: ' + f.key);
            if (colorVarMap[f.key]) injectVar(colorVarMap[f.key], this.value);
            if (colorAliasMap[f.key]) injectVar(colorAliasMap[f.key], this.value);
            syncTokensToProfile();
            TsStudio.markDirty();
        });
    });
    numFields.forEach(function(f) {
        numInputs[f.key].addEventListener('input', function() {
            TsStudio.pushSnapshot('token: ' + f.key);
            var m = numVarMap[f.key];
            if (m && this.value !== '') injectVar(m.v, this.value + m.unit);
            syncTokensToProfile();
            TsStudio.markDirty();
        });
    });
    fontSel.addEventListener('change', function() {
        TsStudio.pushSnapshot('token: body_font');
        var stack = fontStacks[this.value] || fontStacks.System;
        injectVar('--luperiq-body-font', stack);
        syncTokensToProfile();
        TsStudio.markDirty();
    });
    fbCheck.addEventListener('change', function() {
        TsStudio.pushSnapshot('token: full_bleed');
        syncTokensToProfile();
        TsStudio.markDirty();
    });

    /* Inject tokens into preview on initial load */
    injectAllTokens();
};

/* ── Legacy redirect — opens unified studio on Tokens tab ────────── */
async function load_ts_design() {
    TsStudio._pendingTab = 'tokens';
    if (typeof navigateTo === 'function') { navigateTo('ts-studio'); return; }
    await load_ts_studio();
}
"####;
