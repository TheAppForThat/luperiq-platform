//! Design Playground — public-facing style exploration drawer.
//!
//! A slide-out panel that lets visitors explore 5 design presets and
//! mix-and-match header/footer/color/nav options. All changes are
//! session-local — no WAL writes, no interference with other visitors
//! or the front-end block editor.

/// JavaScript for the Design Playground drawer.
pub const PLAYGROUND_JS: &str = r##"
(function() {
    'use strict';

    // Note: we no longer hide during edit mode — the drawer's Smart Blocks
    // and Blocks tabs ARE the block palette for the front-end editor.

    // ── Bridge: insert a block via the front-end editor ─────────────
    // frontend-editor.js exposes window.liqInsertBlock when edit mode
    // is active. This inserts a block at the end of the page.
    var SMART_BLOCK_TYPES = [
        'company-hero', 'service-grid', 'trust-badges', 'about-section',
        'cta-bar', 'contact-info', 'booking-form', 'hours-location',
        'service-detail-header', 'service-inclusions', 'service-cta',
    ];

    function insertBlockViaEditor(blockType, atIndex) {
        if (!window.liqInsertBlock) {
            showToast('Enter Edit mode first (click Edit Page)');
            return;
        }
        window.liqInsertBlock(blockType, atIndex);

        // Smart blocks need server rendering — save and reload.
        // The drawer will re-open on the same tab (sessionStorage persistence).
        // Also flag to auto-enter edit mode after reload.
        if (SMART_BLOCK_TYPES.indexOf(blockType) >= 0 && window.liqSaveAndReload) {
            sessionStorage.setItem('liq-auto-edit', 'true');
            showToast('Adding ' + blockType + ' — saving...');
            window.liqSaveAndReload();
        } else {
            showToast('Added: ' + blockType);
        }
    }

    // ── Position Picker ──────────────────────────────────────────────
    function showPositionPicker(blockType, blockLabel) {
        // Must be in edit mode
        if (!window.liqInsertBlock) {
            showToast('Enter Edit mode first (click Edit Page)');
            return;
        }

        // Get current blocks from the editor bridge
        var currentBlocks = [];
        if (window.liqGetBlocks) {
            currentBlocks = window.liqGetBlocks();
        }

        // If no blocks, just insert at top
        if (currentBlocks.length === 0) {
            insertBlockViaEditor(blockType, 0);
            return;
        }

        // Build picker overlay inside the drawer
        var drawer = document.getElementById('liqPlaygroundDrawer');
        if (!drawer) return;

        // Remove any existing picker
        var old = drawer.querySelector('.liq-playground-picker');
        if (old) old.remove();

        var picker = document.createElement('div');
        picker.className = 'liq-playground-picker';

        var heading = document.createElement('h4');
        heading.textContent = 'Where to place: ' + blockLabel + '?';
        picker.appendChild(heading);

        // "Top of page" option
        var topItem = document.createElement('div');
        topItem.className = 'liq-playground-picker-item';
        var topIcon = document.createElement('span');
        topIcon.className = 'liq-playground-picker-icon';
        topIcon.textContent = '\u25B2';
        topItem.appendChild(topIcon);
        var topLabel = document.createTextNode(' Top of page');
        topItem.appendChild(topLabel);
        topItem.addEventListener('click', function() {
            picker.remove();
            insertBlockViaEditor(blockType, 0);
        });
        picker.appendChild(topItem);

        // One option per existing block
        for (var i = 0; i < currentBlocks.length; i++) {
            (function(idx, blk) {
                var item = document.createElement('div');
                item.className = 'liq-playground-picker-item';
                var icon = document.createElement('span');
                icon.className = 'liq-playground-picker-icon';
                icon.textContent = '\u25BC';
                item.appendChild(icon);
                var label = document.createTextNode(' Below: ' + (blk.label || blk.type));
                item.appendChild(label);
                item.addEventListener('click', function() {
                    picker.remove();
                    insertBlockViaEditor(blockType, idx + 1);
                });
                picker.appendChild(item);
            })(i, currentBlocks[i]);
        }

        // Cancel button
        var cancelBtn = document.createElement('button');
        cancelBtn.className = 'liq-playground-picker-cancel';
        cancelBtn.textContent = 'Cancel';
        cancelBtn.addEventListener('click', function() {
            picker.remove();
        });
        picker.appendChild(cancelBtn);

        drawer.appendChild(picker);
    }

    var state = {
        open: false,
        variants: [],
        activeKey: null,
        originalCss: {},
        activeScopeStyles: [],
        pgCurrentScopeId: null,
        pgProfileTokens: null,
        originalHeaderHtml: null,
        originalFooterHtml: null,
        undoStack: [],
        // Colors tab state
        pgTokens: null,
        pgOrigTokens: null,
        pgUndoStack: [],
        pgRedoStack: [],
        pgDirty: false,
        pgUndoBtn: null,
        pgRedoBtn: null,
        // Layouts tab state
        activeLayoutTheme: null,
        pgLayoutDirty: false,
    };

    // ── Fetch variants ───────────────────────────────────────────────
    function loadVariants() {
        return fetch('/api/modules/theme-studio/design-variants')
            .then(function(r) { return r.json(); })
            .then(function(data) {
                if (data.ok && data.variants) {
                    state.variants = data.variants;
                }
            })
            .catch(function() {});
    }

    // ── Fetch active design tokens (Colors tab) ──────────────────────
    function loadActiveTokens() {
        return fetch('/api/modules/theme-studio/active-tokens')
            .then(function(r) { return r.json(); })
            .then(function(result) {
                if (result.ok && result.data) {
                    state.pgTokens = result.data;
                    state.pgProfileTokens = JSON.parse(JSON.stringify(result.data));
                    state.pgOrigTokens = JSON.parse(JSON.stringify(result.data));
                }
            })
            .catch(function() {});
    }

    // ── Fetch active layout theme ────────────────────────────────────
    function loadActiveLayoutTheme() {
        return fetch('/api/modules/theme-studio/active-layout-theme')
            .then(function(r) { return r.json(); })
            .then(function(result) {
                state.activeLayoutTheme = result.layout_theme_id || null;
            })
            .catch(function() {});
    }

    // ── Fetch + apply scope style overrides for current page ────────
    function loadActiveScopeStyles() {
        return fetch('/api/modules/theme-studio/scope-styles')
            .then(function(r) { return r.json(); })
            .then(function(result) {
                if (result.ok && result.data && result.data.items) {
                    state.activeScopeStyles = result.data.items;
                }
            })
            .catch(function() {});
    }

    function applyActiveScopeStyles() {
        if (!state.activeScopeStyles || !state.activeScopeStyles.length) return;
        var path = window.location.pathname;
        var parts = path.split('/').filter(Boolean);
        var prefixes = parts.map(function(_, i) { return '/' + parts.slice(0, i + 1).join('/'); });

        function specificityOf(scope) {
            if (!scope || scope.type === 'sitewide') return 0;
            if (scope.type === 'url_prefix') return 1 + (scope.value || '').length;
            if (scope.type === 'page_slug') return 1000000;
            return 0;
        }

        var applicable = state.activeScopeStyles
            .filter(function(s) {
                if (!s.enabled) return false;
                if (!s.scope || s.scope.type === 'sitewide') return true;
                if (s.scope.type === 'url_prefix') return prefixes.indexOf(s.scope.value) >= 0;
                if (s.scope.type === 'page_slug') return s.scope.value === path;
                return false;
            })
            .slice()
            .sort(function(a, b) { return specificityOf(a.scope) - specificityOf(b.scope); });

        var el = document.documentElement;
        for (var i = 0; i < applicable.length; i++) {
            var ov = applicable[i].overrides || {};
            if (ov.primary)     el.style.setProperty('--luperiq-primary', ov.primary);
            if (ov.accent)      el.style.setProperty('--luperiq-accent', ov.accent);
            if (ov.link)        el.style.setProperty('--luperiq-link', ov.link);
            if (ov.button_text) el.style.setProperty('--luperiq-button-text', ov.button_text);
            if (ov.header_bg)   el.style.setProperty('--luperiq-header-bg', ov.header_bg);
            if (ov.header_text) el.style.setProperty('--luperiq-header-text', ov.header_text);
            if (ov.background)  el.style.setProperty('--luperiq-background', ov.background);
            if (ov.surface)     el.style.setProperty('--luperiq-surface', ov.surface);
            if (ov.text)        el.style.setProperty('--luperiq-text', ov.text);
            if (ov.radius != null)     el.style.setProperty('--luperiq-radius', ov.radius + 'px');
            if (ov.container != null)  el.style.setProperty('--luperiq-container', ov.container + 'px');
            if (ov.body_size != null)  el.style.setProperty('--luperiq-body-size', ov.body_size + 'px');
        }
    }

    // ── Apply full token set to CSS variables ────────────────────────
    var _pgFontStacks = {
        // System stacks
        System:       '-apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif',
        Humanist:     '"Seravek", "Gill Sans Nova", Ubuntu, Calibri, "DejaVu Sans", sans-serif',
        Transitional: 'Charter, "Bitstream Charter", "Sitka Text", Cambria, serif',
        OldStyle:     '"Iowan Old Style", "Palatino Linotype", "URW Palladio L", P052, serif',
        Geometric:    'Avenir, Montserrat, Corbel, "URW Gothic", source-sans-pro, sans-serif',
        Mono:         '"Cascadia Code", "Source Code Pro", Menlo, Consolas, "DejaVu Sans Mono", monospace',
        // Google Fonts — sans-serif
        Inter:        '"Inter", -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif',
        Roboto:       '"Roboto", -apple-system, BlinkMacSystemFont, Arial, sans-serif',
        OpenSans:     '"Open Sans", Arial, Helvetica, sans-serif',
        Lato:         '"Lato", -apple-system, BlinkMacSystemFont, sans-serif',
        Poppins:      '"Poppins", -apple-system, BlinkMacSystemFont, sans-serif',
        Nunito:       '"Nunito", -apple-system, BlinkMacSystemFont, sans-serif',
        // Google Fonts — serif
        Merriweather:    '"Merriweather", Georgia, "Times New Roman", serif',
        PlayfairDisplay: '"Playfair Display", Georgia, "Times New Roman", serif',
        Lora:            '"Lora", Georgia, "Times New Roman", serif',
        // Google Fonts — display
        Montserrat: '"Montserrat", "Gill Sans", Optima, sans-serif',
        Oswald:     '"Oswald", "Arial Narrow", Gadget, sans-serif',
        Raleway:    '"Raleway", "Gill Sans", Optima, sans-serif'
    };

    var _pgGoogleFontsUrls = {
        Inter:           'https://fonts.googleapis.com/css2?family=Inter:wght@400;500;600;700&display=swap',
        Roboto:          'https://fonts.googleapis.com/css2?family=Roboto:wght@400;500;700&display=swap',
        OpenSans:        'https://fonts.googleapis.com/css2?family=Open+Sans:wght@400;600;700&display=swap',
        Lato:            'https://fonts.googleapis.com/css2?family=Lato:wght@400;700&display=swap',
        Poppins:         'https://fonts.googleapis.com/css2?family=Poppins:wght@400;500;600;700&display=swap',
        Nunito:          'https://fonts.googleapis.com/css2?family=Nunito:wght@400;600;700&display=swap',
        Merriweather:    'https://fonts.googleapis.com/css2?family=Merriweather:wght@400;700&display=swap',
        PlayfairDisplay: 'https://fonts.googleapis.com/css2?family=Playfair+Display:wght@400;700&display=swap',
        Lora:            'https://fonts.googleapis.com/css2?family=Lora:wght@400;700&display=swap',
        Montserrat:      'https://fonts.googleapis.com/css2?family=Montserrat:wght@400;500;600;700&display=swap',
        Oswald:          'https://fonts.googleapis.com/css2?family=Oswald:wght@400;500;700&display=swap',
        Raleway:         'https://fonts.googleapis.com/css2?family=Raleway:wght@400;500;600;700&display=swap'
    };

    function _pgLoadGoogleFont(key) {
        var url = _pgGoogleFontsUrls[key];
        if (!url) return;
        var linkId = 'liq-gf-' + key;
        if (!document.getElementById(linkId)) {
            var link = document.createElement('link');
            link.id = linkId;
            link.rel = 'stylesheet';
            link.href = url;
            document.head.appendChild(link);
        }
    }

    function _pgNormalizeHex(color) {
        if (!color || typeof color !== 'string') return null;
        var v = color.trim();
        return /^#[0-9a-fA-F]{6}$/.test(v) ? v.toLowerCase() : null;
    }

    function _pgParseColor(color) {
        if (!color) return null;
        var hex = _pgNormalizeHex(color);
        if (hex) return { r: parseInt(hex.slice(1,3),16), g: parseInt(hex.slice(3,5),16), b: parseInt(hex.slice(5,7),16), a:1 };
        var m = color.match(/rgba?\(([^)]+)\)/i);
        if (!m) return null;
        var p = m[1].split(',').map(function(s){return parseFloat(s.trim());});
        return p.length >= 3 ? { r:p[0], g:p[1], b:p[2], a:p[3]||1 } : null;
    }

    function _pgSrgbLinear(ch) {
        var v = ch/255;
        return v <= 0.04045 ? v/12.92 : Math.pow((v+0.055)/1.055, 2.4);
    }

    function _pgBestText(bg) {
        var c = _pgParseColor(bg);
        if (!c) return '#ffffff';
        var lum = 0.2126*_pgSrgbLinear(c.r)+0.7152*_pgSrgbLinear(c.g)+0.0722*_pgSrgbLinear(c.b);
        return lum > 0.179 ? '#111111' : '#ffffff';
    }

    function _pgAccentHover(accent) {
        var c = _pgParseColor(accent);
        if (!c) return accent || '#1d4ed8';
        function shade(ch) { return Math.max(0, Math.min(255, Math.round(ch*0.86))); }
        return '#'+[shade(c.r),shade(c.g),shade(c.b)].map(function(v){return v.toString(16).padStart(2,'0');}).join('');
    }

    function applyFullTokensToCSS(tokens) {
        var accent = _pgNormalizeHex(tokens.accent) || tokens.accent;
        var buttonText = _pgNormalizeHex(tokens.button_text) || _pgBestText(accent || '#2563eb');
        var accentHover = _pgAccentHover(accent || '#2563eb');
        var root = document.documentElement.style;
        root.setProperty('--luperiq-primary', tokens.primary);
        root.setProperty('--luperiq-secondary', tokens.header_bg);
        root.setProperty('--luperiq-accent', accent);
        root.setProperty('--luperiq-accent-hover', accentHover);
        root.setProperty('--luperiq-link', tokens.link);
        root.setProperty('--luperiq-button-text', buttonText);
        root.setProperty('--luperiq-header-bg', tokens.header_bg);
        root.setProperty('--luperiq-header-text', tokens.header_text);
        root.setProperty('--luperiq-background', tokens.background);
        root.setProperty('--luperiq-surface', tokens.surface);
        root.setProperty('--luperiq-text', tokens.text);
        root.setProperty('--accent', accent);
        root.setProperty('--accent-hover', accentHover);
        root.setProperty('--bg', tokens.background);
        root.setProperty('--surface', tokens.surface);
        root.setProperty('--text', tokens.text);
        root.setProperty('--header-bg', tokens.header_bg);
        root.setProperty('--header-text', tokens.header_text);
        root.setProperty('--luperiq-radius', tokens.radius + 'px');
        root.setProperty('--luperiq-container', tokens.container + 'px');
        root.setProperty('--luperiq-brand-size', tokens.brand_size + 'px');
        root.setProperty('--luperiq-nav-size', tokens.nav_size + 'px');
        root.setProperty('--luperiq-nav-gap', tokens.nav_gap + 'px');
        root.setProperty('--luperiq-body-size', tokens.body_size + 'px');
        root.setProperty('--luperiq-heading-size', tokens.heading_size + 'px');
        root.setProperty('--luperiq-body-line-height', (tokens.body_line_height/10).toFixed(1));
        if (tokens.body_font && _pgFontStacks[tokens.body_font]) {
            root.setProperty('--luperiq-body-font', _pgFontStacks[tokens.body_font]);
        }
        if (tokens.heading_font && _pgFontStacks[tokens.heading_font]) {
            root.setProperty('--luperiq-heading-font', _pgFontStacks[tokens.heading_font]);
        } else {
            root.removeProperty('--luperiq-heading-font');
        }
        // Live-inject custom CSS so the admin sees changes immediately
        var _liqCustomStyle = document.getElementById('liq-custom-css-preview');
        if (!_liqCustomStyle) {
            _liqCustomStyle = document.createElement('style');
            _liqCustomStyle.id = 'liq-custom-css-preview';
            document.head.appendChild(_liqCustomStyle);
        }
        _liqCustomStyle.textContent = tokens.custom_css || '';

        // Live responsive preview via injected @media overrides
        var _liqRespStyle = document.getElementById('liq-responsive-preview');
        if (!_liqRespStyle) {
            _liqRespStyle = document.createElement('style');
            _liqRespStyle.id = 'liq-responsive-preview';
            document.head.appendChild(_liqRespStyle);
        }
        var _respCss = '';
        function _respBlock(bp, ov) {
            if (!ov) return '';
            var rules = '';
            if (ov.body_size != null)  rules += '        --luperiq-body-size: ' + ov.body_size + 'px;\n';
            if (ov.radius != null)     rules += '        --luperiq-radius: ' + ov.radius + 'px;\n';
            if (ov.container != null)  rules += '        --luperiq-container: ' + ov.container + 'px;\n';
            if (ov.nav_size != null)   rules += '        --luperiq-nav-size: ' + ov.nav_size + 'px;\n';
            if (ov.nav_gap != null)    rules += '        --luperiq-nav-gap: ' + ov.nav_gap + 'px;\n';
            return rules ? '@media (max-width: ' + bp + 'px) {\n    :root {\n' + rules + '    }\n}\n' : '';
        }
        _respCss += _respBlock(980, tokens.tablet);
        _respCss += _respBlock(860, tokens.mobile);
        _liqRespStyle.textContent = _respCss;
    }

    function pgPushTokenUndo(tokens) {
        state.pgUndoStack.push(JSON.parse(JSON.stringify(tokens)));
        if (state.pgUndoStack.length > 50) state.pgUndoStack.shift();
        state.pgRedoStack = [];
        state.pgDirty = true;
        _pgUpdateUndoRedo();
    }

    function _pgUpdateUndoRedo() {
        if (state.pgUndoBtn) { state.pgUndoBtn.disabled = state.pgUndoStack.length === 0; state.pgUndoBtn.style.opacity = state.pgUndoStack.length ? '1':'0.4'; }
        if (state.pgRedoBtn) { state.pgRedoBtn.disabled = state.pgRedoStack.length === 0; state.pgRedoBtn.style.opacity = state.pgRedoStack.length ? '1':'0.4'; }
    }

    // ── Save original state ──────────────────────────────────────────
    function captureOriginals() {
        var root = document.documentElement;
        var cs = getComputedStyle(root);
        state.originalCss = {
            primary: cs.getPropertyValue('--luperiq-primary').trim(),
            accent: cs.getPropertyValue('--luperiq-accent').trim(),
            headerBg: cs.getPropertyValue('--luperiq-header-bg').trim(),
            headerText: cs.getPropertyValue('--luperiq-header-text').trim(),
            background: cs.getPropertyValue('--luperiq-background').trim(),
            surface: cs.getPropertyValue('--luperiq-surface').trim(),
            text: cs.getPropertyValue('--luperiq-text').trim(),
            bodyFont: cs.getPropertyValue('--luperiq-body-font').trim(),
            bodySize: cs.getPropertyValue('--luperiq-body-size').trim(),
            brandSize: cs.getPropertyValue('--luperiq-brand-size').trim(),
            navSize: cs.getPropertyValue('--luperiq-nav-size').trim(),
            radius: cs.getPropertyValue('--luperiq-radius').trim(),
        };
        var header = document.querySelector('.luperiq-ts-layout--header');
        if (header) state.originalHeaderHtml = header.outerHTML;
        var footer = document.querySelector('.luperiq-ts-layout--footer');
        if (footer) {
            state.originalFooterHtml = footer.outerHTML;
            state.originalFooterBg = footer.style.background || '';
            state.originalFooterColor = footer.style.color || '';
        }
        var topBar = document.querySelector('.liq-top-bar');
        if (topBar) {
            state.originalTopBarBg = topBar.style.background || '';
            state.originalTopBarColor = topBar.style.color || '';
        }
    }

    // ── Apply colors ─────────────────────────────────────────────────
    function applyPalette(palette) {
        var root = document.documentElement.style;
        if (palette.primary) root.setProperty('--luperiq-primary', palette.primary);
        if (palette.accent) {
            root.setProperty('--luperiq-accent', palette.accent);
            root.setProperty('--luperiq-link', palette.accent);
        }
        if (palette.header_bg) {
            root.setProperty('--luperiq-header-bg', palette.header_bg);
            root.setProperty('--header-bg', palette.header_bg);
        }
        if (palette.header_text) {
            root.setProperty('--luperiq-header-text', palette.header_text);
            root.setProperty('--header-text', palette.header_text);
        }
        if (palette.background) {
            root.setProperty('--luperiq-background', palette.background);
            root.setProperty('--bg', palette.background);
        }
        if (palette.surface) {
            root.setProperty('--luperiq-surface', palette.surface);
            root.setProperty('--surface', palette.surface);
        }
        if (palette.text) {
            root.setProperty('--luperiq-text', palette.text);
            root.setProperty('--text', palette.text);
        }
        // Update header background directly (inline styles override CSS variables)
        var headerEl = document.querySelector('.luperiq-ts-layout--header');
        if (headerEl && palette.header_bg) {
            headerEl.style.background = palette.header_bg;
            headerEl.style.color = palette.header_text || '';
        }

        // Update top bar (announcement bar) — uses inline styles
        var topBar = document.querySelector('.liq-top-bar');
        if (topBar && palette.primary) {
            topBar.style.background = palette.primary;
            topBar.style.color = '#ffffff';
        }

        // Update footer background
        var footerEl = document.querySelector('.luperiq-ts-layout--footer');
        if (footerEl) {
            // Use a darker version of the primary for footer
            footerEl.style.background = palette.header_bg || palette.primary || '';
            footerEl.style.color = palette.header_text || '#ffffff';
        }

        // Update CTA buttons accent color
        var ctaButtons = document.querySelectorAll('.luperiq-ts-cta--primary');
        for (var i = 0; i < ctaButtons.length; i++) {
            ctaButtons[i].style.background = palette.accent || '';
        }

        // Apply fonts and sizes
        if (palette.font_family) {
            root.setProperty('--luperiq-body-font', palette.font_family);
        }
        if (palette.body_size) {
            root.setProperty('--luperiq-body-size', palette.body_size + 'px');
        }
        if (palette.brand_size) {
            root.setProperty('--luperiq-brand-size', palette.brand_size + 'px');
        }
        if (palette.nav_size) {
            root.setProperty('--luperiq-nav-size', palette.nav_size + 'px');
        }
        if (palette.radius !== undefined) {
            root.setProperty('--luperiq-radius', palette.radius + 'px');
        }
    }

    // ── Restore original colors ──────────────────────────────────────
    function restoreOriginals() {
        var root = document.documentElement.style;
        var o = state.originalCss;
        root.setProperty('--luperiq-primary', o.primary);
        root.setProperty('--luperiq-accent', o.accent);
        root.setProperty('--luperiq-header-bg', o.headerBg);
        root.setProperty('--luperiq-header-text', o.headerText);
        root.setProperty('--luperiq-background', o.background);
        root.setProperty('--luperiq-surface', o.surface);
        root.setProperty('--luperiq-text', o.text);
        root.setProperty('--header-bg', o.headerBg);
        root.setProperty('--header-text', o.headerText);
        root.setProperty('--bg', o.background);
        root.setProperty('--surface', o.surface);
        root.setProperty('--text', o.text);
        root.setProperty('--luperiq-link', o.accent);
        if (o.bodyFont) root.setProperty('--luperiq-body-font', o.bodyFont);
        if (o.bodySize) root.setProperty('--luperiq-body-size', o.bodySize);
        if (o.brandSize) root.setProperty('--luperiq-brand-size', o.brandSize);
        if (o.navSize) root.setProperty('--luperiq-nav-size', o.navSize);
        if (o.radius) root.setProperty('--luperiq-radius', o.radius);

        // Restore original header HTML
        if (state.originalHeaderHtml) {
            var headerEl = document.querySelector('.luperiq-ts-layout--header');
            if (headerEl && headerEl.parentNode) {
                var temp = document.createElement('div');
                temp.innerHTML = state.originalHeaderHtml;
                var orig = temp.firstElementChild;
                if (orig) headerEl.parentNode.replaceChild(orig, headerEl);
            }
        } else {
            var headerEl = document.querySelector('.luperiq-ts-layout--header');
            if (headerEl) {
                headerEl.style.background = '';
                headerEl.style.color = '';
            }
        }

        // Restore top bar
        var topBar = document.querySelector('.liq-top-bar');
        if (topBar) {
            topBar.style.background = state.originalTopBarBg || '';
            topBar.style.color = state.originalTopBarColor || '';
        }

        // Restore original footer HTML
        if (state.originalFooterHtml) {
            var footerEl = document.querySelector('.luperiq-ts-layout--footer');
            if (footerEl && footerEl.parentNode) {
                var temp2 = document.createElement('div');
                temp2.innerHTML = state.originalFooterHtml;
                var origF = temp2.firstElementChild;
                if (origF) footerEl.parentNode.replaceChild(origF, footerEl);
            }
        } else {
            var footerEl = document.querySelector('.luperiq-ts-layout--footer');
            if (footerEl) {
                footerEl.style.background = state.originalFooterBg || '';
                footerEl.style.color = state.originalFooterColor || '';
            }
        }

        // Restore CTA buttons
        var ctaButtons = document.querySelectorAll('.luperiq-ts-cta--primary');
        for (var i = 0; i < ctaButtons.length; i++) {
            ctaButtons[i].style.background = '';
        }

        if (state.applyBtn) state.applyBtn.style.display = 'none';

        state.activeKey = null;
        updateSelection();
    }

    // ── AJAX header/footer re-rendering ─────────────────────────────
    function reloadHeaderForVariant(variantKey) {
        // Don't replace the header if a preserve-marked nav is present (e.g. card_grid_mega)
        if (document.querySelector('[data-lq-preserve="true"]')) return;
        fetch('/api/modules/theme-studio/render/variant-preview?key=' + encodeURIComponent(variantKey))
            .then(function(r) { return r.json(); })
            .then(function(data) {
                if (data.ok && data.header_html) {
                    var headerEl = document.querySelector('.luperiq-ts-layout--header');
                    if (headerEl && headerEl.parentNode) {
                        var temp = document.createElement('div');
                        temp.innerHTML = data.header_html;
                        var newHeader = temp.firstElementChild;
                        if (newHeader) {
                            headerEl.parentNode.replaceChild(newHeader, headerEl);
                        }
                    }
                }
                if (data.ok && data.footer_html) {
                    var footerEl = document.querySelector('.luperiq-ts-layout--footer');
                    if (footerEl && footerEl.parentNode) {
                        var temp2 = document.createElement('div');
                        temp2.innerHTML = data.footer_html;
                        var newFooter = temp2.firstElementChild;
                        if (newFooter) {
                            footerEl.parentNode.replaceChild(newFooter, footerEl);
                        }
                    }
                }
            })
            .catch(function() {});
    }

    // ── Undo stack ─────────────────────────────────────────────────
    function pushUndo() {
        var root = document.documentElement;
        var cs = getComputedStyle(root);
        state.undoStack.push({
            activeKey: state.activeKey,
            vars: {
                primary: cs.getPropertyValue('--luperiq-primary').trim(),
                accent: cs.getPropertyValue('--luperiq-accent').trim(),
                headerBg: cs.getPropertyValue('--luperiq-header-bg').trim(),
                headerText: cs.getPropertyValue('--luperiq-header-text').trim(),
                background: cs.getPropertyValue('--luperiq-background').trim(),
                surface: cs.getPropertyValue('--luperiq-surface').trim(),
                text: cs.getPropertyValue('--luperiq-text').trim(),
                bodyFont: cs.getPropertyValue('--luperiq-body-font').trim(),
                bodySize: cs.getPropertyValue('--luperiq-body-size').trim(),
                brandSize: cs.getPropertyValue('--luperiq-brand-size').trim(),
                navSize: cs.getPropertyValue('--luperiq-nav-size').trim(),
                radius: cs.getPropertyValue('--luperiq-radius').trim(),
            },
            headerHtml: (document.querySelector('.luperiq-ts-layout--header') || {}).outerHTML || '',
            footerHtml: (document.querySelector('.luperiq-ts-layout--footer') || {}).outerHTML || '',
        });
        // Keep max 20 undo states
        if (state.undoStack.length > 20) state.undoStack.shift();
        // Show/hide undo button
        if (state.undoBtn) state.undoBtn.style.display = state.undoStack.length > 0 ? '' : 'none';
    }

    function popUndo() {
        if (state.undoStack.length === 0) return;
        var prev = state.undoStack.pop();
        var root = document.documentElement.style;
        var v = prev.vars;
        root.setProperty('--luperiq-primary', v.primary);
        root.setProperty('--luperiq-accent', v.accent);
        root.setProperty('--luperiq-link', v.accent);
        root.setProperty('--luperiq-header-bg', v.headerBg);
        root.setProperty('--luperiq-header-text', v.headerText);
        root.setProperty('--luperiq-background', v.background);
        root.setProperty('--luperiq-surface', v.surface);
        root.setProperty('--luperiq-text', v.text);
        root.setProperty('--header-bg', v.headerBg);
        root.setProperty('--header-text', v.headerText);
        root.setProperty('--bg', v.background);
        root.setProperty('--surface', v.surface);
        root.setProperty('--text', v.text);
        root.setProperty('--luperiq-body-font', v.bodyFont);
        root.setProperty('--luperiq-body-size', v.bodySize);
        root.setProperty('--luperiq-brand-size', v.brandSize);
        root.setProperty('--luperiq-nav-size', v.navSize);
        root.setProperty('--luperiq-radius', v.radius);

        // Restore header/footer HTML
        if (prev.headerHtml) {
            var hEl = document.querySelector('.luperiq-ts-layout--header');
            if (hEl && hEl.parentNode) {
                var t = document.createElement('div');
                t.innerHTML = prev.headerHtml;
                if (t.firstElementChild) hEl.parentNode.replaceChild(t.firstElementChild, hEl);
            }
        }
        if (prev.footerHtml) {
            var fEl = document.querySelector('.luperiq-ts-layout--footer');
            if (fEl && fEl.parentNode) {
                var t2 = document.createElement('div');
                t2.innerHTML = prev.footerHtml;
                if (t2.firstElementChild) fEl.parentNode.replaceChild(t2.firstElementChild, fEl);
            }
        }

        state.activeKey = prev.activeKey;
        updateSelection();
        showToast('Change undone');
        if (state.undoBtn) state.undoBtn.style.display = state.undoStack.length > 0 ? '' : 'none';
    }

    // ── Apply a full preset ──────────────────────────────────────────
    function applyPreset(variant) {
        pushUndo();
        if (state.activeKey === variant.key) {
            restoreOriginals();
            return;
        }
        state.activeKey = variant.key;
        applyPalette(variant.palette);
        if (state._pgSyncTokensFromPreset) state._pgSyncTokensFromPreset(variant.palette);
        reloadHeaderForVariant(variant.key);
        syncDropdownsToPreset(variant.key);
        updateSelection();
        if (state.applyBtn) state.applyBtn.style.display = '';
        savePlaygroundState();
        showToast('Previewing: ' + variant.label);
    }

    // ── Update chip selection state + detail ─────────────────────────
    function updateSelection() {
        var chips = document.querySelectorAll('.liq-playground-chip');
        for (var i = 0; i < chips.length; i++) {
            if (chips[i].dataset.key === state.activeKey) {
                chips[i].classList.add('is-active');
            } else {
                chips[i].classList.remove('is-active');
            }
        }
        // Show description of active preset
        if (state.detailEl) {
            var active = state.variants.find(function(v) { return v.key === state.activeKey; });
            if (active) {
                state.detailEl.textContent = active.description;
                state.detailEl.style.display = '';
            } else {
                state.detailEl.style.display = 'none';
            }
        }
    }

    // ── Toast notification ───────────────────────────────────────────
    function showToast(msg) {
        var existing = document.querySelector('.liq-playground-toast');
        if (existing) existing.remove();
        var t = document.createElement('div');
        t.className = 'liq-playground-toast';
        t.textContent = msg;
        document.body.appendChild(t);
        setTimeout(function() { t.classList.add('is-visible'); }, 10);
        setTimeout(function() {
            t.classList.remove('is-visible');
            setTimeout(function() { t.remove(); }, 300);
        }, 2000);
    }

    // ── Dropdown helper ──────────────────────────────────────────────
    // Stores dropdown refs in state.dropdowns[id] for syncing
    if (!state.dropdowns) state.dropdowns = {};
    function buildDropdown(labelText, options, onChange) {
        var id = labelText.toLowerCase().replace(/[^a-z]/g, '_');
        var wrap = document.createElement('div');
        wrap.className = 'liq-playground-control';
        var label = document.createElement('label');
        label.className = 'liq-playground-control-label';
        label.textContent = labelText;
        wrap.appendChild(label);
        var select = document.createElement('select');
        select.className = 'liq-playground-select';
        select.dataset.pgId = id;
        var placeholder = document.createElement('option');
        placeholder.value = '';
        placeholder.textContent = '\u2014 Choose \u2014';
        select.appendChild(placeholder);
        options.forEach(function(opt) {
            var o = document.createElement('option');
            o.value = opt.value;
            o.textContent = opt.label;
            select.appendChild(o);
        });
        select.addEventListener('change', function() {
            if (select.value) {
                onChange(select.value);
                savePlaygroundState();
            }
        });
        wrap.appendChild(select);
        state.dropdowns[id] = select;
        return wrap;
    }

    // ── Sync all dropdowns to match the active preset ─────────────
    var presetDefaults = {
        'trust-forward':  { nav: 'flat',      font: "system-ui, -apple-system, 'Segoe UI', Roboto, sans-serif", radius: '8' },
        'friendly-local': { nav: 'pill',      font: "Seravek, 'Gill Sans Nova', Ubuntu, Calibri, sans-serif",   radius: '12' },
        'modern-edge':    { nav: 'underline', font: "Avenir, Montserrat, Corbel, 'URW Gothic', sans-serif",     radius: '4' },
        'earth-guard':    { nav: 'flat',      font: "Charter, 'Bitstream Charter', 'Sitka Text', Cambria, serif", radius: '8' },
        'clean-slate':    { nav: 'flat',      font: "system-ui, -apple-system, 'Segoe UI', Roboto, sans-serif", radius: '12' }
    };
    function syncDropdownsToPreset(key) {
        if (state.dropdowns.color_scheme) state.dropdowns.color_scheme.value = key;
        if (state.dropdowns.header_style) state.dropdowns.header_style.value = key;
        if (state.dropdowns.footer_style) state.dropdowns.footer_style.value = key;
        var defs = presetDefaults[key];
        if (!defs) return;
        // Nav Style
        if (state.dropdowns.nav_style) {
            state.dropdowns.nav_style.value = defs.nav;
            state.dropdowns.nav_style.dispatchEvent(new Event('change'));
        }
        // Body Font
        if (state.dropdowns.body_font) {
            state.dropdowns.body_font.value = defs.font;
            state.dropdowns.body_font.dispatchEvent(new Event('change'));
        }
        // Corner Style
        if (state.dropdowns.corner_style) {
            state.dropdowns.corner_style.value = defs.radius;
            state.dropdowns.corner_style.dispatchEvent(new Event('change'));
        }
    }

    // ── Session persistence ───────────────────────────────────────
    function savePlaygroundState() {
        try {
            var data = { activeKey: state.activeKey || '' };
            Object.keys(state.dropdowns).forEach(function(id) {
                data['dd_' + id] = state.dropdowns[id].value;
            });
            data._v = 2;
            sessionStorage.setItem('liq_playground', JSON.stringify(data));
        } catch(e) {}
    }
    function restorePlaygroundState() {
        try {
            var raw = sessionStorage.getItem('liq_playground');
            if (!raw) return;
            var data = JSON.parse(raw);
            // Restore dropdown values
            Object.keys(state.dropdowns).forEach(function(id) {
                var val = data['dd_' + id];
                if (val && state.dropdowns[id]) {
                    state.dropdowns[id].value = val;
                    // Don't dispatch change for header/footer style — those replace
                    // the header/footer HTML with variant previews that lack nav items.
                    // Only restore CSS-only dropdowns (color, font, nav style, corner).
                    if (id !== 'header_style' && id !== 'footer_style') {
                        state.dropdowns[id].dispatchEvent(new Event('change'));
                    }
                }
            });
            // Restore active preset chip highlight
            if (data.activeKey) {
                state.activeKey = data.activeKey;
                updateSelection();
            }
        } catch(e) {}
    }

    // ── Build the drawer DOM ─────────────────────────────────────────
    // ── Rebuild Smart Blocks panel (called on every tab click) ──────
    function rebuildSmartBlocksPanel(panel) {
        while (panel.firstChild) panel.removeChild(panel.firstChild);
        var isEditing = !!document.querySelector('.liq-save-bar');

        if (!isEditing) {
            var editPrompt = document.createElement('div');
            editPrompt.className = 'liq-playground-edit-prompt';
            var promptIcon = document.createElement('div');
            promptIcon.className = 'liq-playground-prompt-icon';
            promptIcon.textContent = '\u270F\uFE0F';
            editPrompt.appendChild(promptIcon);
            var promptText = document.createElement('p');
            promptText.textContent = 'Click "Edit Page" in the toolbar to start adding smart blocks to your page.';
            editPrompt.appendChild(promptText);
            var editBtn2 = document.createElement('button');
            editBtn2.className = 'liq-playground-apply';
            editBtn2.textContent = 'Edit Page';
            editBtn2.addEventListener('click', function() {
                var btns = document.querySelectorAll('button');
                for (var i = 0; i < btns.length; i++) {
                    if (btns[i].textContent.trim() === 'Edit Page') {
                        btns[i].click();
                        break;
                    }
                }
            });
            editPrompt.appendChild(editBtn2);
            panel.appendChild(editPrompt);
        } else {
            var industry = document.documentElement.dataset.industry || '';
            var isFamilySite = document.documentElement.dataset.familySite === 'true';
            var communityTypes = ['church','family','roommates','club','classroom',
                'homeschool','sports-team','scouts','fitness','farm','book-club',
                'support-group','maker-space','neighborhood','band','travel',
                'elder-care','wedding','pet-owners','small-group','mission-team',
                'homeschool-coop','business','reunion','memorial'];
            var isCommunity = isFamilySite || communityTypes.indexOf(industry) !== -1;

            var smartCategories;
            if (isCommunity) {
                smartCategories = [
                    {
                        name: 'Hero & Branding',
                        blocks: [
                            { type: 'company-hero', label: 'Welcome Hero', desc: 'Main headline, tagline, and CTA' },
                            { type: 'about-section', label: 'About Section', desc: 'Our story and mission' },
                            { type: 'trust-badges', label: 'Trust Badges', desc: 'Years established, milestones' },
                        ]
                    },
                    {
                        name: 'Community',
                        blocks: [
                            { type: 'calendar-upcoming', label: 'Upcoming Events', desc: 'Next events from the calendar' },
                            { type: 'rsvp-summary', label: 'RSVP Status', desc: 'Attendance counts and event details' },
                            { type: 'fundraising-thermometer', label: 'Fundraising Progress', desc: 'Visual goal tracker with donations' },
                            { type: 'reading-progress', label: 'Reading Progress', desc: 'Group book tracker with pace' },
                            { type: 'cta-bar', label: 'CTA Bar', desc: 'Call-to-action with buttons' },
                            { type: 'contact-info', label: 'Contact Info', desc: 'Phone, email, address' },
                            { type: 'hours-location', label: 'Hours & Location', desc: 'Meeting times and location' },
                            { type: 'support-widget', label: 'Support Widget', desc: 'Floating help form with group-specific topics' },
                        ]
                    }
                ];
            } else {
                smartCategories = [
                    {
                        name: 'Hero & Branding',
                        blocks: [
                            { type: 'company-hero', label: 'Company Hero', desc: 'Main headline, tagline, and CTA' },
                            { type: 'about-section', label: 'About Section', desc: 'Company story and background' },
                            { type: 'trust-badges', label: 'Trust Badges', desc: 'Years experience, certifications' },
                        ]
                    },
                    {
                        name: 'Services & Products',
                        blocks: [
                            { type: 'service-grid', label: 'Service Grid', desc: 'Service cards with pricing' },
                            { type: 'service-detail-header', label: 'Service Detail', desc: 'Individual service page header' },
                            { type: 'service-inclusions', label: 'Service Inclusions', desc: 'What is included list' },
                        ]
                    },
                    {
                        name: 'Conversion',
                        blocks: [
                            { type: 'cta-bar', label: 'CTA Bar', desc: 'Call-to-action with buttons' },
                            { type: 'contact-info', label: 'Contact Info', desc: 'Phone, email, address' },
                            { type: 'booking-form', label: 'Booking Form', desc: 'Online appointment booking' },
                            { type: 'hours-location', label: 'Hours & Location', desc: 'Business hours and map' },
                            { type: 'support-widget', label: 'Support Widget', desc: 'Floating help form with industry-specific topics' },
                        ]
                    }
                ];
            }

            smartCategories.forEach(function(cat) {
                var catSection = document.createElement('div');
                catSection.className = 'liq-playground-section';
                var catHeading = document.createElement('h4');
                catHeading.textContent = cat.name;
                catSection.appendChild(catHeading);
                cat.blocks.forEach(function(blk) {
                    var item = document.createElement('div');
                    item.className = 'liq-playground-block-item';
                    item.addEventListener('click', function() {
                        showPositionPicker(blk.type, blk.label);
                    });
                    var itemIcon = document.createElement('span');
                    itemIcon.className = 'liq-playground-block-icon';
                    itemIcon.textContent = '\u26A1';
                    item.appendChild(itemIcon);
                    var itemInfo = document.createElement('div');
                    itemInfo.className = 'liq-playground-block-info';
                    var itemLabel = document.createElement('div');
                    itemLabel.className = 'liq-playground-block-label';
                    itemLabel.textContent = blk.label;
                    itemInfo.appendChild(itemLabel);
                    var itemDesc = document.createElement('div');
                    itemDesc.className = 'liq-playground-block-desc';
                    itemDesc.textContent = blk.desc;
                    itemInfo.appendChild(itemDesc);
                    item.appendChild(itemInfo);
                    catSection.appendChild(item);
                });
                panel.appendChild(catSection);
            });
        }
    }

    // ── Rebuild Blocks panel (called on every tab click) ─────────
    function rebuildBlocksPanel(panel) {
        while (panel.firstChild) panel.removeChild(panel.firstChild);
        var isEditing = !!document.querySelector('.liq-save-bar');

        if (!isEditing) {
            var editPrompt = document.createElement('div');
            editPrompt.className = 'liq-playground-edit-prompt';
            var promptIcon = document.createElement('div');
            promptIcon.className = 'liq-playground-prompt-icon';
            promptIcon.textContent = '\u270F\uFE0F';
            editPrompt.appendChild(promptIcon);
            var promptText = document.createElement('p');
            promptText.textContent = 'Click "Edit Page" in the toolbar to start adding blocks to your page.';
            editPrompt.appendChild(promptText);
            panel.appendChild(editPrompt);
        } else {
            var basicBlocks = [
                { cat: 'Text', items: [
                    { type: 'heading', icon: 'H', label: 'Heading' },
                    { type: 'paragraph', icon: '\u00B6', label: 'Paragraph' },
                    { type: 'list', icon: '\u2261', label: 'List' },
                    { type: 'quote', icon: '\u201C', label: 'Quote' },
                ]},
                { cat: 'Media', items: [
                    { type: 'image', icon: '\u25A3', label: 'Image' },
                ]},
                { cat: 'Layout', items: [
                    { type: 'divider', icon: '\u2500', label: 'Divider' },
                    { type: 'spacer', icon: '\u2195', label: 'Spacer' },
                ]},
                { cat: 'Interactive', items: [
                    { type: 'button', icon: '\u25A3', label: 'Button' },
                    { type: 'html', icon: '</>', label: 'Custom HTML' },
                ]}
            ];

            basicBlocks.forEach(function(group) {
                var sec = document.createElement('div');
                sec.className = 'liq-playground-section';
                var h = document.createElement('h4');
                h.textContent = group.cat;
                sec.appendChild(h);
                var grid = document.createElement('div');
                grid.className = 'liq-playground-block-grid';
                group.items.forEach(function(blk) {
                    var card = document.createElement('div');
                    card.className = 'liq-playground-block-card';
                    card.addEventListener('click', function() {
                        showPositionPicker(blk.type, blk.label);
                    });
                    var icon = document.createElement('div');
                    icon.className = 'liq-playground-block-card-icon';
                    icon.textContent = blk.icon;
                    card.appendChild(icon);
                    var label = document.createElement('div');
                    label.className = 'liq-playground-block-card-label';
                    label.textContent = blk.label;
                    card.appendChild(label);
                    grid.appendChild(card);
                });
                sec.appendChild(grid);
                panel.appendChild(sec);
            });
        }
    }

    // ── Colors Panel builder ─────────────────────────────────────────
    function buildColorsPanel(panel) {
        if (!state.pgTokens) {
            var msg = document.createElement('div');
            msg.style.cssText = 'padding:24px;text-align:center;opacity:0.5;font-size:13px;';
            msg.textContent = 'No active profile — set up Theme Studio first.';
            panel.appendChild(msg);
            return;
        }
        var tokens = (function() {
            var _t = JSON.parse(JSON.stringify(state.pgTokens || {}));
            if (state.pgCurrentScopeId) {
                var _sc = (state.activeScopeStyles || []).find(function(s) { return s.id === state.pgCurrentScopeId; });
                if (_sc && _sc.overrides) {
                    var _ov = _sc.overrides;
                    ['primary','accent','link','button_text','header_bg','header_text','background','surface','text'].forEach(function(k) { if (_ov[k] != null) _t[k] = _ov[k]; });
                    if (_ov.radius != null) _t.radius = _ov.radius;
                    if (_ov.container != null) _t.container = _ov.container;
                    if (_ov.body_size != null) _t.body_size = _ov.body_size;
                }
            }
            return _t;
        }());

        // Header with undo/redo
        var hdr = document.createElement('div');
        hdr.style.cssText = 'display:flex;align-items:center;justify-content:space-between;padding:16px 16px 8px;';
        var title = document.createElement('span');
        title.textContent = 'Design Tokens';
        title.style.cssText = 'font-weight:700;font-size:14px;color:#a78bfa;';
        var urRow = document.createElement('div');
        urRow.style.cssText = 'display:flex;gap:4px;';
        state.pgUndoBtn = document.createElement('button');
        state.pgUndoBtn.textContent = '↩';
        state.pgUndoBtn.title = 'Undo';
        state.pgUndoBtn.style.cssText = 'background:none;border:1px solid #475569;color:#e2e8f0;width:26px;height:26px;border-radius:4px;cursor:pointer;font-size:13px;opacity:0.4;';
        state.pgUndoBtn.disabled = true;
        state.pgUndoBtn.onclick = function() {
            if (state.pgUndoStack.length === 0) return;
            state.pgRedoStack.push(JSON.parse(JSON.stringify(tokens)));
            var prev = state.pgUndoStack.pop();
            Object.keys(prev).forEach(function(k) { tokens[k] = prev[k]; });
            applyFullTokensToCSS(tokens);
            _pgRefreshInputs(tokens);
            _pgUpdateUndoRedo();
        };
        state.pgRedoBtn = document.createElement('button');
        state.pgRedoBtn.textContent = '↪';
        state.pgRedoBtn.title = 'Redo';
        state.pgRedoBtn.style.cssText = 'background:none;border:1px solid #475569;color:#e2e8f0;width:26px;height:26px;border-radius:4px;cursor:pointer;font-size:13px;opacity:0.4;';
        state.pgRedoBtn.disabled = true;
        state.pgRedoBtn.onclick = function() {
            if (state.pgRedoStack.length === 0) return;
            state.pgUndoStack.push(JSON.parse(JSON.stringify(tokens)));
            var next = state.pgRedoStack.pop();
            Object.keys(next).forEach(function(k) { tokens[k] = next[k]; });
            applyFullTokensToCSS(tokens);
            _pgRefreshInputs(tokens);
            _pgUpdateUndoRedo();
        };
        urRow.appendChild(state.pgUndoBtn);
        urRow.appendChild(state.pgRedoBtn);
        hdr.appendChild(title);
        hdr.appendChild(urRow);
        panel.appendChild(hdr);

        // ── Scope picker ─────────────────────────────────────────────
        var _scopeBar = document.createElement('div');
        _scopeBar.style.cssText = 'padding:0 16px 10px;display:flex;align-items:center;gap:8px;';
        var _scopeLbl = document.createElement('span');
        _scopeLbl.textContent = 'Scope:';
        _scopeLbl.style.cssText = 'font-size:11px;color:#94a3b8;flex-shrink:0;';
        var _scopeSel = document.createElement('select');
        _scopeSel.style.cssText = 'flex:1;background:#1e293b;border:1px solid #334155;color:#e2e8f0;border-radius:4px;padding:4px 6px;font-size:12px;cursor:pointer;';
        function _rebuildScopeOpts() {
            _scopeSel.innerHTML = '';
            var _o0 = document.createElement('option');
            _o0.value = ''; _o0.textContent = 'Sitewide (profile)';
            _scopeSel.appendChild(_o0);
            (state.activeScopeStyles || []).forEach(function(s) {
                var _o = document.createElement('option');
                _o.value = s.id;
                var _sfx = (s.scope && s.scope.type !== 'sitewide') ? ' [' + (s.scope.value || '') + ']' : '';
                _o.textContent = s.label + _sfx;
                _scopeSel.appendChild(_o);
            });
            var _oAdd = document.createElement('option');
            _oAdd.value = '__add__'; _oAdd.textContent = '+ New scope…';
            _scopeSel.appendChild(_oAdd);
            _scopeSel.value = state.pgCurrentScopeId || '';
        }
        _rebuildScopeOpts();
        var _delScopeBtn = document.createElement('button');
        _delScopeBtn.textContent = '✕';
        _delScopeBtn.title = 'Delete scope';
        _delScopeBtn.style.cssText = 'background:none;border:1px solid #ef4444;color:#ef4444;width:24px;height:24px;border-radius:4px;cursor:pointer;font-size:11px;display:none;align-items:center;justify-content:center;flex-shrink:0;';
        _delScopeBtn.onclick = function() {
            if (!state.pgCurrentScopeId) return;
            var _s = (state.activeScopeStyles || []).find(function(s) { return s.id === state.pgCurrentScopeId; });
            if (!confirm('Delete scope "' + (_s ? _s.label : state.pgCurrentScopeId) + '"?')) return;
            fetch('/api/modules/theme-studio/scope-styles/' + state.pgCurrentScopeId, { method: 'DELETE' })
                .then(function(r) { return r.json(); })
                .then(function(res) {
                    if (res.ok || res.deleted) {
                        state.activeScopeStyles = (state.activeScopeStyles || []).filter(function(s) { return s.id !== state.pgCurrentScopeId; });
                        state.pgCurrentScopeId = null;
                        _rebuildScopeOpts();
                        _delScopeBtn.style.display = 'none';
                        var _bt = JSON.parse(JSON.stringify(state.pgTokens || {}));
                        Object.keys(_bt).forEach(function(k) { tokens[k] = _bt[k]; });
                        applyFullTokensToCSS(tokens);
                        _pgRefreshInputs(tokens);
                        showToast('Scope deleted');
                    } else { showToast('Delete failed'); }
                }).catch(function(e) { showToast('Error: ' + e.message); });
        };
        var _addScopeBox = document.createElement('div');
        _addScopeBox.style.cssText = 'display:none;padding:10px 16px;background:#0f172a;border-top:1px solid #334155;';
        (function() {
            var _lbl = document.createElement('div');
            _lbl.style.cssText = 'font-size:12px;color:#a78bfa;font-weight:700;margin-bottom:8px;';
            _lbl.textContent = 'New Scope';
            var _row1 = document.createElement('div');
            _row1.style.cssText = 'display:flex;gap:8px;margin-bottom:8px;';
            var _typeSel = document.createElement('select');
            _typeSel.style.cssText = 'background:#1e293b;border:1px solid #334155;color:#e2e8f0;border-radius:4px;padding:4px 6px;font-size:12px;';
            ['sitewide','url_prefix','page_slug'].forEach(function(v, i) {
                var _o = document.createElement('option');
                _o.value = v; _o.textContent = ['Sitewide','URL Prefix','Page'][i];
                _typeSel.appendChild(_o);
            });
            var _valInp = document.createElement('input');
            _valInp.placeholder = '/path';
            _valInp.style.cssText = 'display:none;flex:1;background:#1e293b;border:1px solid #334155;color:#e2e8f0;border-radius:4px;padding:4px 6px;font-size:12px;';
            _typeSel.onchange = function() { _valInp.style.display = _typeSel.value === 'sitewide' ? 'none' : ''; };
            _row1.appendChild(_typeSel); _row1.appendChild(_valInp);
            var _labelInp = document.createElement('input');
            _labelInp.placeholder = 'Label (e.g. Service Areas - Blue)';
            _labelInp.style.cssText = 'width:100%;box-sizing:border-box;background:#1e293b;border:1px solid #334155;color:#e2e8f0;border-radius:4px;padding:4px 6px;font-size:12px;margin-bottom:8px;';
            var _row2 = document.createElement('div');
            _row2.style.cssText = 'display:flex;gap:8px;';
            var _createBtn = document.createElement('button');
            _createBtn.textContent = 'Create';
            _createBtn.style.cssText = 'background:#22c55e;border:none;color:#fff;border-radius:4px;padding:5px 12px;font-size:12px;cursor:pointer;';
            _createBtn.onclick = function() {
                var _label = _labelInp.value.trim();
                if (!_label) { showToast('Enter a label'); return; }
                var _st = _typeSel.value, _sv = _valInp.value.trim();
                if (_st !== 'sitewide' && !_sv) { showToast('Enter a path'); return; }
                var _scopeObj = _st === 'sitewide' ? { type: 'sitewide' } : { type: _st, value: _sv };
                _createBtn.disabled = true; _createBtn.textContent = 'Creating…';
                fetch('/api/modules/theme-studio/scope-styles', {
                    method: 'POST',
                    headers: { 'Content-Type': 'application/json' },
                    body: JSON.stringify({ label: _label, scope: _scopeObj, overrides: {}, enabled: true })
                }).then(function(r) { return r.json(); })
                .then(function(res) {
                    if (res.ok && res.data && res.data.id) {
                        var _nid = res.data.id;
                        var _nsc = { id: _nid, label: _label, scope: _scopeObj, overrides: {}, enabled: true };
                        state.activeScopeStyles = state.activeScopeStyles || [];
                        state.activeScopeStyles.push(_nsc);
                        state.pgCurrentScopeId = _nid;
                        _rebuildScopeOpts();
                        _delScopeBtn.style.display = 'flex';
                        _addScopeBox.style.display = 'none';
                        showToast('Scope created — set colors and Save');
                    } else { showToast('Create failed: ' + (res.message || 'error')); }
                }).catch(function(e) { showToast('Error: ' + e.message); })
                .finally(function() { _createBtn.disabled = false; _createBtn.textContent = 'Create'; });
            };
            var _cancelBtn2 = document.createElement('button');
            _cancelBtn2.textContent = 'Cancel';
            _cancelBtn2.style.cssText = 'background:none;border:1px solid #475569;color:#e2e8f0;border-radius:4px;padding:5px 12px;font-size:12px;cursor:pointer;';
            _cancelBtn2.onclick = function() { _addScopeBox.style.display = 'none'; _scopeSel.value = state.pgCurrentScopeId || ''; };
            _row2.appendChild(_createBtn); _row2.appendChild(_cancelBtn2);
            _addScopeBox.appendChild(_lbl); _addScopeBox.appendChild(_row1);
            _addScopeBox.appendChild(_labelInp); _addScopeBox.appendChild(_row2);
        }());
        _scopeSel.onchange = function() {
            var _val = _scopeSel.value;
            if (_val === '__add__') {
                _addScopeBox.style.display = '';
                _scopeSel.value = state.pgCurrentScopeId || '';
                return;
            }
            state.pgCurrentScopeId = _val || null;
            _delScopeBtn.style.display = _val ? 'flex' : 'none';
            var _base = JSON.parse(JSON.stringify(state.pgTokens || {}));
            if (_val) {
                var _sc2 = (state.activeScopeStyles || []).find(function(s) { return s.id === _val; });
                if (_sc2 && _sc2.overrides) {
                    var _ov2 = _sc2.overrides;
                    ['primary','accent','link','button_text','header_bg','header_text','background','surface','text'].forEach(function(k) { if (_ov2[k] != null) _base[k] = _ov2[k]; });
                    if (_ov2.radius != null) _base.radius = _ov2.radius;
                    if (_ov2.container != null) _base.container = _ov2.container;
                    if (_ov2.body_size != null) _base.body_size = _ov2.body_size;
                }
            }
            Object.keys(_base).forEach(function(k) { tokens[k] = _base[k]; });
            applyFullTokensToCSS(tokens);
            _pgRefreshInputs(tokens);
            var _selOpt = _scopeSel.options[_scopeSel.selectedIndex];
            showToast(_val ? ('Editing scope: ' + (_selOpt ? _selOpt.text : _val)) : 'Editing sitewide profile');
        };
        _scopeBar.appendChild(_scopeLbl);
        _scopeBar.appendChild(_scopeSel);
        _scopeBar.appendChild(_delScopeBtn);
        panel.appendChild(_scopeBar);
        panel.appendChild(_addScopeBox);

        // ── Scope inheritance strip ────────────────────────────────
        var _scopeChainBar = document.createElement('div');
        _scopeChainBar.style.cssText = 'padding:0 16px 8px;display:flex;align-items:center;flex-wrap:wrap;gap:3px;';
        function _renderScopeChain() {
            while (_scopeChainBar.firstChild) _scopeChainBar.removeChild(_scopeChainBar.firstChild);
            var path = window.location.pathname;
            var prefixes = [];
            for (var i = 1; i < path.length; i++) {
                if (path[i] === '/') prefixes.push(path.slice(0, i));
            }
            prefixes.push(path);
            var applicable = [{ id: '', label: 'Sitewide', _spec: -1 }];
            (state.activeScopeStyles || []).forEach(function(s) {
                var spec = -1;
                if (!s.scope || s.scope.type === 'sitewide') spec = 0;
                else if (s.scope.type === 'url_prefix' && prefixes.indexOf(s.scope.value) >= 0) spec = 1 + (s.scope.value || '').length;
                else if (s.scope.type === 'page_slug' && s.scope.value === path) spec = 1000000;
                if (spec >= -1) applicable.push({ id: s.id, label: s.label, _spec: spec, scope: s.scope });
            });
            applicable.sort(function(a, b) { return a._spec - b._spec; });
            if (applicable.length <= 1) {
                var hint = document.createElement('span');
                hint.textContent = 'Editing sitewide tokens';
                hint.style.cssText = 'font-size:10px;color:#475569;';
                _scopeChainBar.appendChild(hint);
                return;
            }
            applicable.forEach(function(sc, i) {
                if (i > 0) {
                    var arr = document.createElement('span');
                    arr.textContent = '\u203a';
                    arr.style.cssText = 'font-size:10px;color:#475569;';
                    _scopeChainBar.appendChild(arr);
                }
                var chip = document.createElement('span');
                var isActive = sc.id === (state.pgCurrentScopeId || '');
                chip.textContent = sc.label;
                chip.style.cssText = 'font-size:9px;padding:1px 5px;border-radius:3px;cursor:pointer;' +
                    (isActive ? 'background:#7c3aed;color:#fff;font-weight:700;' : 'background:#1e293b;color:#64748b;');
                chip.addEventListener('click', function() {
                    _scopeSel.value = sc.id;
                    _scopeSel.dispatchEvent(new Event('change'));
                });
                _scopeChainBar.appendChild(chip);
            });
        }
        _renderScopeChain();
        panel.appendChild(_scopeChainBar);
        _scopeSel.addEventListener('change', function() { setTimeout(_renderScopeChain, 50); });

        var content = document.createElement('div');
        content.style.cssText = 'padding:0 16px 16px;overflow-y:auto;max-height:calc(50vh - 130px);';

        var _colorPickers = {}, _colorHexInputs = {}, _sliderInputs = {}, _sliderValues = {}, _fontSelect = null;

        function _pgRefreshInputs(t) {
            Object.keys(_colorPickers).forEach(function(k) {
                if (t[k]) { _colorPickers[k].value = t[k]; _colorHexInputs[k].value = t[k]; }
            });
            Object.keys(_sliderInputs).forEach(function(k) {
                _sliderInputs[k].value = String(t[k]);
                var sf = _pgSliderFields.find(function(f){return f.key===k;});
                if (sf && _sliderValues[k]) {
                    _sliderValues[k].textContent = k === 'body_line_height' ? (t[k]/10).toFixed(1) : t[k]+sf.unit;
                }
            });
            if (_fontSelect && t.body_font) _fontSelect.value = t.body_font;
            if (state._pgCustomCssTa) state._pgCustomCssTa.value = t.custom_css || '';
            if (state._pgHeadFontSelect) state._pgHeadFontSelect.value = t.heading_font || '';
            if (state._pgResponsiveSyncFn) state._pgResponsiveSyncFn(t);
        }

        // Store refresh ref for undo/redo
        state._pgRefreshInputs = _pgRefreshInputs;

        // Bridge: keep tokens in sync when a Styles-tab preset is applied
        state._pgSyncTokensFromPreset = function(palette) {
            if (palette.primary)              tokens.primary     = palette.primary;
            if (palette.accent)               { tokens.accent = palette.accent; tokens.link = palette.accent; }
            if (palette.header_bg)            tokens.header_bg   = palette.header_bg;
            if (palette.header_text)          tokens.header_text = palette.header_text;
            if (palette.background)           tokens.background  = palette.background;
            if (palette.surface)              tokens.surface     = palette.surface;
            if (palette.text)                 tokens.text        = palette.text;
            if (palette.radius !== undefined) tokens.radius      = parseInt(palette.radius, 10) || tokens.radius;
            _pgRefreshInputs(tokens);
        };

        var _pgColorFields = [
            { key: 'primary', label: 'Primary' },
            { key: 'accent', label: 'Accent' },
            { key: 'link', label: 'Link' },
            { key: 'header_bg', label: 'Header BG' },
            { key: 'header_text', label: 'Header Text' },
            { key: 'background', label: 'Background' },
            { key: 'surface', label: 'Surface' },
            { key: 'text', label: 'Text' }
        ];

        var _pgSliderFields = [
            { key: 'brand_size', label: 'Brand Size', min: 12, max: 64, unit: 'px' },
            { key: 'radius', label: 'Border Radius', min: 0, max: 32, unit: 'px' },
            { key: 'container', label: 'Container Width', min: 800, max: 1600, unit: 'px' },
            { key: 'heading_size', label: 'H1 Size', min: 14, max: 80, unit: 'px' },
            { key: 'body_size', label: 'Body Font Size', min: 12, max: 24, unit: 'px' },
            { key: 'body_line_height', label: 'Line Height', min: 10, max: 24, unit: '' },
            { key: 'nav_size', label: 'Nav Font Size', min: 12, max: 24, unit: 'px' },
            { key: 'nav_gap', label: 'Nav Gap', min: 4, max: 48, unit: 'px' }
        ];


        // ── Color Palette Presets ─────────────────────────────────────
        var palHdr = document.createElement('div');
        palHdr.textContent = 'Color Palettes';
        palHdr.style.cssText = 'font-weight:700;margin:12px 0 8px;font-size:11px;color:#94a3b8;text-transform:uppercase;letter-spacing:0.5px;';
        content.appendChild(palHdr);

        var _liqPalettes = [
            { name: 'Navy Pro',     colors: ['#1e3a5f','#2563eb','#f0f4f8','#1e3a5f','#f0f4f8','#ffffff','#f8fafc','#1e293b'] },
            { name: 'Forest',       colors: ['#1a3c2e','#2d6a4f','#f0f7f4','#1a3c2e','#f0f7f4','#ffffff','#f4faf6','#1c2e26'] },
            { name: 'Earth',        colors: ['#4a3728','#c2692a','#fdf6ef','#4a3728','#fdf6ef','#ffffff','#fdf8f4','#2d1f14'] },
            { name: 'Slate',        colors: ['#1e293b','#6366f1','#f1f5f9','#1e293b','#f1f5f9','#ffffff','#f8fafc','#0f172a'] },
            { name: 'Crimson',      colors: ['#7f1d1d','#dc2626','#fef2f2','#7f1d1d','#fef2f2','#ffffff','#fff5f5','#450a0a'] },
            { name: 'Midnight',     colors: ['#0a0e1a','#a78bfa','#0a0e1a','#1e1b4b','#1e1b4b','#0f172a','#1e1b4b','#e2e8f0'] },
            { name: 'Gold & Black', colors: ['#1a1a1a','#d4a017','#1a1a1a','#1a1a1a','#1a1a1a','#111111','#1f1f1f','#f5f0e0'] },
            { name: 'Ocean',        colors: ['#0c4a6e','#0ea5e9','#f0f9ff','#0c4a6e','#f0f9ff','#ffffff','#f0f9ff','#0a2540'] },
            { name: 'Sage',         colors: ['#3d5a45','#6b9e78','#f3f8f4','#3d5a45','#f3f8f4','#ffffff','#f5f9f6','#263d2c'] },
            { name: 'Charcoal',     colors: ['#374151','#6b7280','#f9fafb','#374151','#f9fafb','#ffffff','#f3f4f6','#111827'] }
        ];
        // Color keys: [primary, accent, header_bg, header_text(bg for text), background, surface, surface2→surface, text]
        var _palKeys = ['primary','accent','header_bg','header_text','background','surface','surface','text'];

        var palGrid = document.createElement('div');
        palGrid.style.cssText = 'display:grid;grid-template-columns:repeat(5,1fr);gap:6px;margin-bottom:14px;';

        _liqPalettes.forEach(function(pal) {
            var card = document.createElement('button');
            card.type = 'button';
            card.title = pal.name;
            card.style.cssText = 'border:1px solid rgba(255,255,255,0.12);border-radius:6px;padding:0;cursor:pointer;overflow:hidden;background:none;display:flex;flex-direction:column;height:36px;';
            // Color strip — 3 swatches
            var strip = document.createElement('div');
            strip.style.cssText = 'display:flex;flex:1;';
            [pal.colors[0], pal.colors[1], pal.colors[4]].forEach(function(c) {
                var sw = document.createElement('div');
                sw.style.cssText = 'flex:1;background:' + c + ';';
                strip.appendChild(sw);
            });
            card.appendChild(strip);
            // Label
            var lbl = document.createElement('div');
            lbl.textContent = pal.name;
            lbl.style.cssText = 'font-size:8px;text-align:center;padding:2px 2px;color:#94a3b8;white-space:nowrap;overflow:hidden;text-overflow:ellipsis;background:#0f172a;';
            card.appendChild(lbl);

            card.addEventListener('mouseenter', function() { card.style.borderColor = '#a78bfa'; });
            card.addEventListener('mouseleave', function() { card.style.borderColor = 'rgba(255,255,255,0.12)'; });

            card.addEventListener('click', function() {
                pgPushTokenUndo(tokens);
                _palKeys.forEach(function(k, i) {
                    if (pal.colors[i] != null) tokens[k] = pal.colors[i];
                });
                applyFullTokensToCSS(tokens);
                _pgRefreshInputs(tokens);
            });

            palGrid.appendChild(card);
        });
        content.appendChild(palGrid);

        // Colors section header
        var colHdr = document.createElement('div');
        colHdr.textContent = 'Colors';
        colHdr.style.cssText = 'font-weight:700;margin:12px 0 8px;font-size:11px;color:#94a3b8;text-transform:uppercase;letter-spacing:0.5px;';
        content.appendChild(colHdr);

        _pgColorFields.forEach(function(f) {
            var row = document.createElement('div');
            row.style.cssText = 'display:flex;align-items:center;gap:8px;margin-bottom:7px;';
            var lbl = document.createElement('span');
            lbl.textContent = f.label;
            lbl.style.cssText = 'width:76px;font-size:11px;color:#94a3b8;flex-shrink:0;';
            var picker = document.createElement('input');
            picker.type = 'color';
            picker.value = tokens[f.key] || '#000000';
            picker.style.cssText = 'width:32px;height:26px;border:1px solid #475569;border-radius:3px;background:none;cursor:pointer;padding:0;flex-shrink:0;';
            var hex = document.createElement('input');
            hex.type = 'text';
            hex.value = tokens[f.key] || '#000000';
            hex.style.cssText = 'flex:1;padding:3px 7px;background:#0f172a;border:1px solid #475569;border-radius:3px;color:#e2e8f0;font-size:11px;font-family:monospace;min-width:0;';
            _colorPickers[f.key] = picker;
            _colorHexInputs[f.key] = hex;
            picker.oninput = function() {
                pgPushTokenUndo(tokens);
                hex.value = picker.value;
                tokens[f.key] = picker.value;
                applyFullTokensToCSS(tokens);
            };
            hex.onchange = function() {
                if (/^#[0-9a-fA-F]{6}$/.test(hex.value)) {
                    pgPushTokenUndo(tokens);
                    picker.value = hex.value;
                    tokens[f.key] = hex.value;
                    applyFullTokensToCSS(tokens);
                }
            };
            row.appendChild(lbl); row.appendChild(picker); row.appendChild(hex);
            content.appendChild(row);
        });

        // Spacing & Typography section
        var spHdr = document.createElement('div');
        spHdr.textContent = 'Spacing & Typography';
        spHdr.style.cssText = 'font-weight:700;margin:16px 0 8px;font-size:11px;color:#94a3b8;text-transform:uppercase;letter-spacing:0.5px;';
        content.appendChild(spHdr);

        _pgSliderFields.forEach(function(f) {
            var row = document.createElement('div');
            row.style.cssText = 'margin-bottom:10px;';
            var labelRow = document.createElement('div');
            labelRow.style.cssText = 'display:flex;justify-content:space-between;margin-bottom:3px;';
            var lbl = document.createElement('span');
            lbl.textContent = f.label;
            lbl.style.cssText = 'font-size:11px;color:#94a3b8;';
            var val = document.createElement('span');
            val.textContent = f.key === 'body_line_height' ? (tokens[f.key]/10).toFixed(1) : tokens[f.key]+f.unit;
            val.style.cssText = 'font-size:11px;color:#e2e8f0;font-family:monospace;';
            _sliderValues[f.key] = val;
            labelRow.appendChild(lbl); labelRow.appendChild(val);
            var slider = document.createElement('input');
            slider.type = 'range';
            slider.min = String(f.min); slider.max = String(f.max); slider.value = String(tokens[f.key]);
            slider.style.cssText = 'width:100%;accent-color:#a78bfa;';
            _sliderInputs[f.key] = slider;
            var pushed = false;
            slider.onmousedown = function() { pushed = false; };
            slider.oninput = function() {
                if (!pushed) { pgPushTokenUndo(tokens); pushed = true; }
                var v = parseInt(slider.value, 10);
                tokens[f.key] = v;
                val.textContent = f.key === 'body_line_height' ? (v/10).toFixed(1) : v+f.unit;
                applyFullTokensToCSS(tokens);
            };
            row.appendChild(labelRow); row.appendChild(slider);
            content.appendChild(row);
        });

        // Font selector
        var fontHdr = document.createElement('div');
        fontHdr.textContent = 'Body Font';
        fontHdr.style.cssText = 'font-weight:700;margin:16px 0 6px;font-size:11px;color:#94a3b8;text-transform:uppercase;letter-spacing:0.5px;';
        content.appendChild(fontHdr);
        _fontSelect = document.createElement('select');
        _fontSelect.style.cssText = 'width:100%;padding:5px 7px;background:#0f172a;border:1px solid #475569;border-radius:3px;color:#e2e8f0;font-size:12px;margin-bottom:4px;';
        var _pgFontGroups = [
            { label: 'System Fonts', fonts: [
                { key: 'System', name: 'System UI' }, { key: 'Humanist', name: 'Humanist' },
                { key: 'Transitional', name: 'Transitional Serif' }, { key: 'OldStyle', name: 'Old Style' },
                { key: 'Geometric', name: 'Geometric' }, { key: 'Mono', name: 'Monospace' }
            ]},
            { label: 'Google Sans', fonts: [
                { key: 'Inter', name: 'Inter' }, { key: 'Roboto', name: 'Roboto' },
                { key: 'OpenSans', name: 'Open Sans' }, { key: 'Lato', name: 'Lato' },
                { key: 'Poppins', name: 'Poppins' }, { key: 'Nunito', name: 'Nunito' }
            ]},
            { label: 'Google Serif', fonts: [
                { key: 'Merriweather', name: 'Merriweather' },
                { key: 'PlayfairDisplay', name: 'Playfair Display' },
                { key: 'Lora', name: 'Lora' }
            ]},
            { label: 'Google Display', fonts: [
                { key: 'Montserrat', name: 'Montserrat' },
                { key: 'Oswald', name: 'Oswald' },
                { key: 'Raleway', name: 'Raleway' }
            ]}
        ];
        _pgFontGroups.forEach(function(grp) {
            var og = document.createElement('optgroup');
            og.label = grp.label;
            grp.fonts.forEach(function(f) {
                var opt = document.createElement('option');
                opt.value = f.key;
                opt.textContent = f.name;
                opt.style.fontFamily = _pgFontStacks[f.key] || '';
                if (tokens.body_font === f.key) opt.selected = true;
                og.appendChild(opt);
            });
            _fontSelect.appendChild(og);
        });
        // Pre-load any Google Font that is already active on page load
        _pgLoadGoogleFont(tokens.body_font);
        var _bodyFontPreview = document.createElement('div');
        _bodyFontPreview.style.cssText = 'padding:5px 8px;background:rgba(255,255,255,0.05);border-radius:4px;font-size:15px;color:#e2e8f0;margin-top:3px;margin-bottom:2px;letter-spacing:0.01em;min-height:26px;';
        _bodyFontPreview.textContent = 'Aa Bb — The quick brown fox 123';
        _bodyFontPreview.style.fontFamily = _pgFontStacks[tokens.body_font] || '';
        _fontSelect.onchange = function() {
            pgPushTokenUndo(tokens);
            tokens.body_font = _fontSelect.value;
            _pgLoadGoogleFont(tokens.body_font);
            _bodyFontPreview.style.fontFamily = _pgFontStacks[tokens.body_font] || '';
            applyFullTokensToCSS(tokens);
        };
        // Font preview hint
        var fontHint = document.createElement('div');
        fontHint.style.cssText = 'font-size:10px;color:#64748b;margin-bottom:8px;';
        fontHint.textContent = 'Google Fonts load from fonts.googleapis.com';
        content.appendChild(_fontSelect);
        content.appendChild(_bodyFontPreview);
        content.appendChild(fontHint);

        // ── Heading font selector ──────────────────────────────────────
        var headFontHdr = document.createElement('div');
        headFontHdr.textContent = 'Heading Font';
        headFontHdr.style.cssText = 'font-weight:700;margin:4px 0 6px;font-size:11px;color:#94a3b8;text-transform:uppercase;letter-spacing:0.5px;';
        content.appendChild(headFontHdr);
        var _headFontSelect = document.createElement('select');
        _headFontSelect.style.cssText = 'width:100%;padding:5px 7px;background:#0f172a;border:1px solid #475569;border-radius:3px;color:#e2e8f0;font-size:12px;margin-bottom:4px;';
        var _hfNoneOpt = document.createElement('option');
        _hfNoneOpt.value = '';
        _hfNoneOpt.textContent = '\u2014 Same as body font \u2014';
        if (!tokens.heading_font) _hfNoneOpt.selected = true;
        _headFontSelect.appendChild(_hfNoneOpt);
        _pgFontGroups.forEach(function(grp) {
            var og = document.createElement('optgroup');
            og.label = grp.label;
            grp.fonts.forEach(function(f) {
                var opt = document.createElement('option');
                opt.value = f.key;
                opt.textContent = f.name;
                opt.style.fontFamily = _pgFontStacks[f.key] || '';
                if (tokens.heading_font === f.key) opt.selected = true;
                og.appendChild(opt);
            });
            _headFontSelect.appendChild(og);
        });
        if (tokens.heading_font) _pgLoadGoogleFont(tokens.heading_font);
        var _headFontPreview = document.createElement('div');
        _headFontPreview.style.cssText = 'padding:5px 8px;background:rgba(255,255,255,0.05);border-radius:4px;font-size:15px;color:#e2e8f0;margin-top:3px;margin-bottom:2px;letter-spacing:0.01em;min-height:26px;';
        _headFontPreview.textContent = 'Aa Bb — The quick brown fox 123';
        _headFontPreview.style.fontFamily = tokens.heading_font ? (_pgFontStacks[tokens.heading_font] || '') : '';
        _headFontSelect.onchange = function() {
            pgPushTokenUndo(tokens);
            tokens.heading_font = _headFontSelect.value || null;
            var root = document.documentElement.style;
            if (tokens.heading_font && _pgFontStacks[tokens.heading_font]) {
                _pgLoadGoogleFont(tokens.heading_font);
                root.setProperty('--luperiq-heading-font', _pgFontStacks[tokens.heading_font]);
                _headFontPreview.style.fontFamily = _pgFontStacks[tokens.heading_font];
            } else {
                root.removeProperty('--luperiq-heading-font');
                _headFontPreview.style.fontFamily = '';
            }
        };
        var headFontHint = document.createElement('div');
        headFontHint.style.cssText = 'font-size:10px;color:#64748b;margin-bottom:16px;';
        headFontHint.textContent = 'Applies to h1\u2013h4. Defaults to body font when unset.';
        content.appendChild(_headFontSelect);
        content.appendChild(_headFontPreview);
        content.appendChild(headFontHint);
        state._pgHeadFontSelect = _headFontSelect;

        panel.appendChild(content);

        // Save / Cancel / Reset row
        var foot = document.createElement('div');
        foot.style.cssText = 'display:flex;gap:6px;padding:10px 16px;border-top:1px solid #334155;';

        var resetBtn = document.createElement('button');
        resetBtn.textContent = 'Reset';
        resetBtn.title = 'Reset to saved';
        resetBtn.style.cssText = 'padding:6px 10px;border:1px solid #475569;background:none;color:#94a3b8;border-radius:5px;cursor:pointer;font-size:12px;';
        resetBtn.onclick = function() {
            pgPushTokenUndo(tokens);
            Object.keys(state.pgOrigTokens).forEach(function(k) { tokens[k] = state.pgOrigTokens[k]; });
            applyFullTokensToCSS(tokens);
            _pgRefreshInputs(tokens);
        };

        var cancelBtn = document.createElement('button');
        cancelBtn.textContent = 'Cancel';
        cancelBtn.style.cssText = 'padding:6px 10px;border:1px solid #475569;background:none;color:#e2e8f0;border-radius:5px;cursor:pointer;font-size:12px;flex:1;';
        cancelBtn.onclick = function() {
            if (state.pgDirty && !confirm('Discard unsaved color changes?')) return;
            Object.keys(state.pgOrigTokens).forEach(function(k) { tokens[k] = state.pgOrigTokens[k]; });
            applyFullTokensToCSS(tokens);
            _pgRefreshInputs(tokens);
            state.pgDirty = false;
            state.pgUndoStack = [];
            state.pgRedoStack = [];
            _pgUpdateUndoRedo();
        };

        var saveBtn = document.createElement('button');
        saveBtn.textContent = 'Save Colors';
        saveBtn.style.cssText = 'padding:6px 12px;border:none;background:#22c55e;color:#fff;border-radius:5px;cursor:pointer;font-size:12px;font-weight:600;flex:1;';
        saveBtn.onclick = function() {
            saveBtn.disabled = true;
            saveBtn.textContent = 'Saving...';
            if (state.pgCurrentScopeId) {
                var _pbase = state.pgProfileTokens || state.pgTokens || {};
                var _sov = {};
                ['primary','accent','link','button_text','header_bg','header_text','background','surface','text'].forEach(function(k) { if (tokens[k] !== _pbase[k]) _sov[k] = tokens[k]; });
                if (tokens.radius !== _pbase.radius) _sov.radius = tokens.radius;
                if (tokens.container !== _pbase.container) _sov.container = tokens.container;
                if (tokens.body_size !== _pbase.body_size) _sov.body_size = tokens.body_size;
                var _esc = (state.activeScopeStyles || []).find(function(s) { return s.id === state.pgCurrentScopeId; });
                fetch('/api/modules/theme-studio/scope-styles/' + state.pgCurrentScopeId, {
                    method: 'PUT',
                    headers: { 'Content-Type': 'application/json' },
                    body: JSON.stringify({ label: _esc ? _esc.label : 'Scope', scope: _esc ? _esc.scope : { type: 'sitewide' }, overrides: _sov, enabled: true })
                }).then(function(r) { return r.json(); })
                .then(function(res) {
                    if (res.ok) { if (_esc) _esc.overrides = _sov; showToast('Scope overrides saved ✓'); }
                    else { showToast('Save failed: ' + (res.message || 'error')); }
                }).catch(function(e) { showToast('Error: ' + e.message); })
                .finally(function() { saveBtn.disabled = false; saveBtn.textContent = 'Save Colors'; });
            } else {
                fetch('/api/modules/theme-studio/active-tokens', {
                    method: 'PUT',
                    headers: { 'Content-Type': 'application/json' },
                    body: JSON.stringify(tokens)
                }).then(function(r) { return r.json(); })
                .then(function(result) {
                    if (result.ok) {
                        state.pgOrigTokens = JSON.parse(JSON.stringify(tokens));
                        state.pgProfileTokens = JSON.parse(JSON.stringify(tokens));
                        state.pgDirty = false;
                        state.pgUndoStack = [];
                        state.pgRedoStack = [];
                        _pgUpdateUndoRedo();
                        showToast('Colors saved ✓');
                    } else {
                        showToast('Save failed: ' + (result.message || 'error'));
                    }
                })
                .catch(function(e) { showToast('Error: ' + e.message); })
                .finally(function() { saveBtn.disabled = false; saveBtn.textContent = 'Save Colors'; });
            }
        };

        foot.appendChild(resetBtn);
        foot.appendChild(cancelBtn);
        foot.appendChild(saveBtn);
        panel.appendChild(foot);
        var _wireResponsive = buildResponsiveSection(panel);
        _wireResponsive({ tokens: tokens });
        buildHeaderFooterSection(panel);
        var _wireCustomCss = buildCustomCssSection(panel);
        _wireCustomCss(tokens);
    }

    // ── Layouts Panel builder ────────────────────────────────────────
    var _layoutThemes = [
        { id: 'clean-modern',  name: 'Clean Modern',  desc: 'Crisp whitespace, subtle shadows, centered. The balanced default.', gradient: 'linear-gradient(135deg,#3b82f6,#1e40af)', icon: '⬜' },
        { id: 'parallax-pro',  name: 'Parallax Pro',  desc: 'Full-viewport hero, parallax scroll, bold section entrances.',       gradient: 'linear-gradient(135deg,#7c3aed,#4c1d95)', icon: '🌌' },
        { id: 'magazine',      name: 'Magazine',       desc: 'Editorial grid, heavy typography, image-led sections.',              gradient: 'linear-gradient(135deg,#dc2626,#7f1d1d)', icon: '📰' },
        { id: 'landing-page',  name: 'Landing Page',  desc: 'Single-column conversion funnel. Every section drives one action.',  gradient: 'linear-gradient(135deg,#059669,#064e3b)', icon: '🎯' },
        { id: 'earth-nature',  name: 'Earth & Nature', desc: 'Organic shapes, warm earth tones, gentle fade animations.',         gradient: 'linear-gradient(135deg,#92400e,#78716c)', icon: '🌿' },
        { id: 'bold-agency',   name: 'Bold Agency',   desc: 'Dark backgrounds, overlapping elements, high-contrast type.',        gradient: 'linear-gradient(135deg,#0a0a0a,#f59e0b)', icon: '⚡' }
    ];

    // Animation init — run once when layouts tab is first shown
    var _liqAnimObserver = null;
    function _initAnimObserver() {
        if (_liqAnimObserver || !window.IntersectionObserver) return;
        _liqAnimObserver = new IntersectionObserver(function(entries) {
            entries.forEach(function(entry) {
                if (entry.isIntersecting) {
                    entry.target.classList.add('liq-visible');
                    _liqAnimObserver.unobserve(entry.target);
                }
            });
        }, { threshold: 0.12 });
        document.querySelectorAll('[data-liq-animate]').forEach(function(el) {
            _liqAnimObserver.observe(el);
        });
    }

    function _applyLayoutTheme(themeId) {
        var body = document.body;
        var classes = Array.from(body.classList).filter(function(c) { return c.startsWith('liq-lt-'); });
        classes.forEach(function(c) { body.classList.remove(c); });
        if (themeId && themeId !== 'clean-modern') {
            body.classList.add('liq-lt-' + themeId);
        }
        state.activeLayoutTheme = themeId;
        // Add data-liq-animate to smart block sections for animated themes
        var animatedThemes = ['parallax-pro', 'magazine', 'landing-page', 'earth-nature', 'bold-agency'];
        var blocks = document.querySelectorAll('[data-smart-block]');
        blocks.forEach(function(el, i) {
            if (animatedThemes.indexOf(themeId) >= 0 && i > 0) {
                var anim = (themeId === 'magazine') ? 'slide-in' : 'fade-up';
                el.setAttribute('data-liq-animate', anim);
                el.classList.remove('liq-visible');
            } else {
                el.removeAttribute('data-liq-animate');
                el.classList.remove('liq-visible');
            }
        });
        // Reset and re-wire IntersectionObserver
        if (_liqAnimObserver) { _liqAnimObserver.disconnect(); _liqAnimObserver = null; }
        setTimeout(_initAnimObserver, 80);
        var cards = document.querySelectorAll('.liq-layout-theme-card');
        cards.forEach(function(c) {
            c.classList.toggle('is-active', c.dataset.themeId === themeId);
        });
    }

    function buildLayoutsPanel(panel) {
        var intro = document.createElement('div');
        intro.style.cssText = 'padding:14px 16px 10px;font-size:12px;color:#94a3b8;line-height:1.5;';
        intro.textContent = 'Switch to a completely different site structure with one click. Applies sitewide. Carry to all page types.';
        panel.appendChild(intro);

        var grid = document.createElement('div');
        grid.style.cssText = 'display:grid;grid-template-columns:1fr 1fr;gap:10px;padding:0 12px 12px;';

        _layoutThemes.forEach(function(theme) {
            var card = document.createElement('div');
            card.className = 'liq-layout-theme-card';
            card.dataset.themeId = theme.id;
            if (state.activeLayoutTheme === theme.id || (!state.activeLayoutTheme && theme.id === 'clean-modern')) {
                card.classList.add('is-active');
            }

            // Preview swatch
            var swatch = document.createElement('div');
            swatch.style.cssText = 'height:56px;border-radius:6px 6px 0 0;background:'+theme.gradient+';display:flex;align-items:center;justify-content:center;font-size:22px;';
            swatch.textContent = theme.icon;

            var body = document.createElement('div');
            body.style.cssText = 'padding:8px;';

            var name = document.createElement('div');
            name.style.cssText = 'font-weight:700;font-size:12px;color:#e2e8f0;margin-bottom:3px;';
            name.textContent = theme.name;

            var desc = document.createElement('div');
            desc.style.cssText = 'font-size:10px;color:#94a3b8;line-height:1.4;';
            desc.textContent = theme.desc;

            body.appendChild(name);
            body.appendChild(desc);
            card.appendChild(swatch);
            card.appendChild(body);

            card.addEventListener('click', function() {
                _applyLayoutTheme(theme.id);
                showToast('Layout preview: ' + theme.name);
                state.pgLayoutDirty = true;
                saveLayoutThemeBtn.style.display = '';
            });

            grid.appendChild(card);
        });

        panel.appendChild(grid);

        // Save button (hidden until a theme is clicked)
        var saveLayoutThemeBtn = document.createElement('button');
        saveLayoutThemeBtn.textContent = 'Save Layout Theme';
        saveLayoutThemeBtn.style.cssText = 'display:none;width:calc(100% - 24px);margin:0 12px 16px;padding:10px;border:none;background:#22c55e;color:#fff;border-radius:6px;cursor:pointer;font-size:13px;font-weight:600;';
        saveLayoutThemeBtn.onclick = function() {
            saveLayoutThemeBtn.disabled = true;
            saveLayoutThemeBtn.textContent = 'Saving...';
            fetch('/api/modules/theme-studio/active-layout-theme', {
                method: 'PUT',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ layout_theme_id: state.activeLayoutTheme || null })
            }).then(function(r) { return r.json(); })
            .then(function(result) {
                if (result.ok) {
                    state.pgLayoutDirty = false;
                    showToast('Layout theme saved ✓');
                    saveLayoutThemeBtn.style.display = 'none';
                } else {
                    showToast('Save failed: ' + (result.message || 'error'));
                }
            })
            .catch(function(e) { showToast('Error: ' + e.message); })
            .finally(function() { saveLayoutThemeBtn.disabled = false; saveLayoutThemeBtn.textContent = 'Save Layout Theme'; });
        };
        panel.appendChild(saveLayoutThemeBtn);

        // Apply current active theme body class on load
        if (state.activeLayoutTheme) {
            _applyLayoutTheme(state.activeLayoutTheme);
        }
    }

    // ── Combined Blocks Panel (Smart Blocks + Primitives) ───────────
    function rebuildCombinedBlocksPanel(panel) {
        while (panel.firstChild) panel.removeChild(panel.firstChild);
        var isEditing = !!document.querySelector('.liq-save-bar');

        if (!isEditing) {
            var editPrompt = document.createElement('div');
            editPrompt.className = 'liq-playground-edit-prompt';
            var promptIcon = document.createElement('div');
            promptIcon.className = 'liq-playground-prompt-icon';
            promptIcon.textContent = '✏️';
            editPrompt.appendChild(promptIcon);
            var promptText = document.createElement('p');
            promptText.textContent = 'Click "Edit Page" to start adding blocks to your page.';
            editPrompt.appendChild(promptText);
            var editBtn2 = document.createElement('button');
            editBtn2.className = 'liq-playground-apply';
            editBtn2.textContent = 'Edit Page';
            editBtn2.addEventListener('click', function() {
                var btns = document.querySelectorAll('button');
                for (var i = 0; i < btns.length; i++) {
                    if (btns[i].textContent.trim() === 'Edit Page') { btns[i].click(); break; }
                }
            });
            editPrompt.appendChild(editBtn2);
            panel.appendChild(editPrompt);
            return;
        }

        // ── Smart Blocks section ──────────────────────────────────────
        var industry = document.documentElement.dataset.industry || '';
        var isFamilySite = document.documentElement.dataset.familySite === 'true';
        var communityTypes = ['church','family','roommates','club','classroom',
            'homeschool','sports-team','scouts','fitness','farm','book-club',
            'support-group','maker-space','neighborhood','band','travel',
            'elder-care','wedding','pet-owners','small-group','mission-team',
            'homeschool-coop','business','reunion','memorial'];
        var isCommunity = isFamilySite || communityTypes.indexOf(industry) !== -1;

        var smartCategories;
        if (isCommunity) {
            smartCategories = [
                { name: 'Hero & Branding', blocks: [
                    { type: 'company-hero', label: 'Welcome Hero', desc: 'Main headline, tagline, and CTA' },
                    { type: 'about-section', label: 'About Section', desc: 'Our story and mission' },
                    { type: 'trust-badges', label: 'Trust Badges', desc: 'Years established, milestones' },
                ]},
                { name: 'Community', blocks: [
                    { type: 'calendar-upcoming', label: 'Upcoming Events', desc: 'Next events from the calendar' },
                    { type: 'rsvp-summary', label: 'RSVP Status', desc: 'Attendance counts and event details' },
                    { type: 'fundraising-thermometer', label: 'Fundraising Progress', desc: 'Visual goal tracker' },
                    { type: 'reading-progress', label: 'Reading Progress', desc: 'Group book tracker' },
                    { type: 'cta-bar', label: 'CTA Bar', desc: 'Call-to-action with buttons' },
                    { type: 'contact-info', label: 'Contact Info', desc: 'Phone, email, address' },
                    { type: 'hours-location', label: 'Hours & Location', desc: 'Meeting times and location' },
                    { type: 'support-widget', label: 'Support Widget', desc: 'Floating help form' },
                ]}
            ];
        } else {
            smartCategories = [
                { name: 'Hero & Branding', blocks: [
                    { type: 'company-hero', label: 'Company Hero', desc: 'Main headline, tagline, and CTA' },
                    { type: 'about-section', label: 'About Section', desc: 'Company story and background' },
                    { type: 'trust-badges', label: 'Trust Badges', desc: 'Years experience, certifications' },
                ]},
                { name: 'Services & Products', blocks: [
                    { type: 'service-grid', label: 'Service Grid', desc: 'Service cards with pricing' },
                    { type: 'service-detail-header', label: 'Service Detail', desc: 'Individual service page header' },
                    { type: 'service-inclusions', label: 'Service Inclusions', desc: 'What is included list' },
                ]},
                { name: 'Conversion', blocks: [
                    { type: 'cta-bar', label: 'CTA Bar', desc: 'Call-to-action with buttons' },
                    { type: 'contact-info', label: 'Contact Info', desc: 'Phone, email, address' },
                    { type: 'booking-form', label: 'Booking Form', desc: 'Online appointment booking' },
                    { type: 'hours-location', label: 'Hours & Location', desc: 'Business hours and map' },
                    { type: 'support-widget', label: 'Support Widget', desc: 'Floating help form' },
                ]}
            ];
        }

        var smHdr = document.createElement('div');
        smHdr.textContent = 'Smart Blocks';
        smHdr.style.cssText = 'font-weight:700;font-size:11px;color:#94a3b8;text-transform:uppercase;letter-spacing:0.5px;padding:12px 16px 6px;';
        panel.appendChild(smHdr);

        smartCategories.forEach(function(cat) {
            var catSection = document.createElement('div');
            catSection.className = 'liq-playground-section';
            var catHeading = document.createElement('h4');
            catHeading.textContent = cat.name;
            catSection.appendChild(catHeading);
            cat.blocks.forEach(function(blk) {
                var item = document.createElement('div');
                item.className = 'liq-playground-block-item';
                item.addEventListener('click', function() { showPositionPicker(blk.type, blk.label); });
                var itemIcon = document.createElement('span');
                itemIcon.className = 'liq-playground-block-icon';
                itemIcon.textContent = '⚡';
                item.appendChild(itemIcon);
                var itemInfo = document.createElement('div');
                itemInfo.className = 'liq-playground-block-info';
                var itemLabel = document.createElement('div');
                itemLabel.className = 'liq-playground-block-label';
                itemLabel.textContent = blk.label;
                itemInfo.appendChild(itemLabel);
                var itemDesc = document.createElement('div');
                itemDesc.className = 'liq-playground-block-desc';
                itemDesc.textContent = blk.desc;
                itemInfo.appendChild(itemDesc);
                item.appendChild(itemInfo);
                catSection.appendChild(item);
            });
            panel.appendChild(catSection);
        });

        // Separator
        var sep = document.createElement('div');
        sep.style.cssText = 'margin:12px 16px 4px;padding-top:12px;border-top:1px solid #334155;font-weight:700;font-size:11px;color:#94a3b8;text-transform:uppercase;letter-spacing:0.5px;';
        sep.textContent = 'Basic Blocks';
        panel.appendChild(sep);

        // Primitive blocks
        var basicBlocks = [
            { cat: 'Text', items: [
                { type: 'heading', icon: 'H', label: 'Heading' },
                { type: 'paragraph', icon: '¶', label: 'Paragraph' },
                { type: 'list', icon: '≡', label: 'List' },
                { type: 'quote', icon: '"', label: 'Quote' },
            ]},
            { cat: 'Media', items: [
                { type: 'image', icon: '▣', label: 'Image' },
            ]},
            { cat: 'Layout', items: [
                { type: 'divider', icon: '─', label: 'Divider' },
                { type: 'spacer', icon: '↕', label: 'Spacer' },
            ]},
            { cat: 'Interactive', items: [
                { type: 'button', icon: '▣', label: 'Button' },
                { type: 'html', icon: '</>', label: 'Custom HTML' },
            ]}
        ];

        basicBlocks.forEach(function(group) {
            var sec = document.createElement('div');
            sec.className = 'liq-playground-section';
            var h = document.createElement('h4');
            h.textContent = group.cat;
            sec.appendChild(h);
            var grid = document.createElement('div');
            grid.className = 'liq-playground-block-grid';
            group.items.forEach(function(blk) {
                var card = document.createElement('div');
                card.className = 'liq-playground-block-card';
                card.addEventListener('click', function() { showPositionPicker(blk.type, blk.label); });
                var icon = document.createElement('div');
                icon.className = 'liq-playground-block-card-icon';
                icon.textContent = blk.icon;
                card.appendChild(icon);
                var label = document.createElement('div');
                label.className = 'liq-playground-block-card-label';
                label.textContent = blk.label;
                card.appendChild(label);
                grid.appendChild(card);
            });
            sec.appendChild(grid);
            panel.appendChild(sec);
        });
    }

    function buildDrawer() {
        // Tab trigger
        var tab = document.createElement('div');
        tab.className = 'liq-playground-tab';

        var tabIcon = document.createElement('span');
        tabIcon.className = 'liq-playground-tab-icon';
        tabIcon.textContent = '\u{1F3A8}';
        tab.appendChild(tabIcon);

        var tabText = document.createElement('span');
        tabText.className = 'liq-playground-tab-text';
        tabText.textContent = 'Design';
        tab.appendChild(tabText);

        tab.addEventListener('click', toggleDrawer);
        document.body.appendChild(tab);

        // Drawer panel
        var drawer = document.createElement('div');
        drawer.className = 'liq-playground-drawer';
        drawer.id = 'liqPlaygroundDrawer';

        // Drag handle (mobile only — CSS hides on desktop)
        var dragHandle = document.createElement('div');
        dragHandle.className = 'liq-playground-drag-handle';
        var dragBar = document.createElement('div');
        dragBar.className = 'liq-playground-drag-handle-bar';
        dragHandle.appendChild(dragBar);
        drawer.appendChild(dragHandle);

        // Mobile half-sheet: drag to resize + snap
        (function() {
            var isMobile = function() { return window.innerWidth <= 480; };
            var snapHeights = [30, 50, 75];
            var currentSnap = 1;
            var startY = 0, startH = 0, dragging = false;
            var pinnedTop = false;

            dragHandle.addEventListener('touchstart', function(e) {
                if (!isMobile()) return;
                dragging = true;
                startY = e.touches[0].clientY;
                startH = drawer.getBoundingClientRect().height;
                drawer.style.transition = 'none';
            }, { passive: true });

            document.addEventListener('touchmove', function(e) {
                if (!dragging || !isMobile()) return;
                var deltaY = startY - e.touches[0].clientY;
                if (pinnedTop) deltaY = -deltaY;
                var newH = Math.max(80, Math.min(window.innerHeight * 0.9, startH + deltaY));
                drawer.style.height = newH + 'px';
            }, { passive: true });

            document.addEventListener('touchend', function() {
                if (!dragging || !isMobile()) return;
                dragging = false;
                drawer.style.transition = '';
                var currentVh = (drawer.getBoundingClientRect().height / window.innerHeight) * 100;
                var closest = 0, closestDist = 999;
                for (var i = 0; i < snapHeights.length; i++) {
                    var dist = Math.abs(currentVh - snapHeights[i]);
                    if (dist < closestDist) { closestDist = dist; closest = i; }
                }
                currentSnap = closest;
                if (currentVh < 20) {
                    toggleDrawer();
                    drawer.style.height = '';
                    drawer.className = drawer.className.replace(/ liq-pg-h\d+/g, '');
                    return;
                }
                drawer.style.height = '';
                drawer.className = drawer.className.replace(/ liq-pg-h\d+/g, '');
                drawer.classList.add('liq-pg-h' + snapHeights[currentSnap]);
            });

            var lastTap = 0;
            dragHandle.addEventListener('click', function() {
                if (!isMobile()) return;
                var now = Date.now();
                if (now - lastTap < 300) {
                    currentSnap = (currentSnap + 1) % snapHeights.length;
                    drawer.style.height = '';
                    drawer.className = drawer.className.replace(/ liq-pg-h\d+/g, '');
                    drawer.classList.add('liq-pg-h' + snapHeights[currentSnap]);
                }
                lastTap = now;
            });

            window._pgTogglePin = function() {
                if (!isMobile()) return;
                // Temporarily close, flip position, reopen for smooth transition.
                // The slide-out transition is 300ms; wait 350ms to ensure it completes,
                // then flip position props and wait 100ms for layout recalc before reopening.
                drawer.classList.remove('is-open');
                setTimeout(function() {
                    pinnedTop = !pinnedTop;
                    if (pinnedTop) {
                        drawer.classList.add('is-pinned-top');
                        drawer.appendChild(dragHandle);
                    } else {
                        drawer.classList.remove('is-pinned-top');
                        drawer.insertBefore(dragHandle, drawer.firstChild);
                    }
                    var pinBtn = document.querySelector('.liq-playground-pin-btn');
                    if (pinBtn) pinBtn.textContent = pinnedTop ? '\u2B07 Bottom' : '\u2B06 Top';
                    // Force layout recalc before reopening
                    void drawer.offsetHeight;
                    setTimeout(function() { drawer.classList.add('is-open'); }, 100);
                }, 350);
            };
        })();

        // Header
        var header = document.createElement('div');
        header.className = 'liq-playground-header';
        var headerTitle = document.createElement('h3');
        headerTitle.textContent = 'Design Playground';
        header.appendChild(headerTitle);

        var headerRight = document.createElement('div');
        headerRight.style.cssText = 'display:flex;align-items:center;gap:6px;';
        var pinBtn = document.createElement('button');
        pinBtn.className = 'liq-playground-pin-btn';
        pinBtn.textContent = '\u2B06 Top';
        pinBtn.style.cssText = 'background:#f1f5f9;border:1px solid #e2e8f0;border-radius:6px;padding:4px 8px;font-size:11px;font-weight:600;color:#64748b;cursor:pointer;white-space:nowrap;';
        pinBtn.addEventListener('click', function() { window._pgTogglePin(); });
        headerRight.appendChild(pinBtn);
        var closeBtn = document.createElement('button');
        closeBtn.className = 'liq-playground-close';
        closeBtn.textContent = '\u00D7';
        closeBtn.addEventListener('click', function() {
            // When closing, also reset pin to bottom if pinned to top
            var d = document.getElementById('liqPlaygroundDrawer');
            if (d && d.classList.contains('is-pinned-top')) {
                d.classList.remove('is-pinned-top');
                var dh = document.querySelector('.liq-playground-drag-handle');
                if (dh) d.insertBefore(dh, d.firstChild);
                var pb = document.querySelector('.liq-playground-pin-btn');
                if (pb) pb.textContent = '\u2B06 Top';
            }
            toggleDrawer();
        });
        headerRight.appendChild(closeBtn);
        header.appendChild(headerRight);
        drawer.appendChild(header);

        // Subtitle
        var sub = document.createElement('p');
        sub.className = 'liq-playground-subtitle';
        sub.textContent = 'Explore different looks for your site. Click a preset to preview it live.';
        drawer.appendChild(sub);

        // Tab navigation
        var tabBar = document.createElement('div');
        tabBar.className = 'liq-playground-tabs';

        var tabs = [
            { id: 'colors', icon: '\uD83C\uDFA8', label: 'Colors' },
            { id: 'styles', icon: '\u2728', label: 'Styles' },
            { id: 'layouts', icon: '\u{1F5BC}', label: 'Layouts' },
            { id: 'blocks', icon: '\u26A1', label: 'Blocks' }
        ];
        var tabPanels = {};

        // Determine default active tab (first visit = colors)
        var _defaultTab = 'colors';

        tabs.forEach(function(t) {
            var tab = document.createElement('button');
            tab.className = 'liq-playground-tab-btn' + (t.id === _defaultTab ? ' is-active' : '');
            tab.dataset.tab = t.id;
            tab.textContent = t.icon + ' ' + t.label;
            tab.addEventListener('click', function() {
                tabBar.querySelectorAll('.liq-playground-tab-btn').forEach(function(b) {
                    b.classList.remove('is-active');
                });
                tab.classList.add('is-active');
                // Rebuild Blocks panel on every click to detect edit mode state
                if (t.id === 'blocks' && tabPanels['blocks']) {
                    rebuildCombinedBlocksPanel(tabPanels['blocks']);
                }
                Object.keys(tabPanels).forEach(function(k) {
                    tabPanels[k].style.display = k === t.id ? '' : 'none';
                });
                sessionStorage.setItem('liq-playground-state', JSON.stringify({
                    open: state.open,
                    activeTab: t.id
                }));
            });
            tabBar.appendChild(tab);
        });

        drawer.appendChild(tabBar);

        // ── Viewport preview bar ──────────────────────────────
        var vpBar = document.createElement('div');
        vpBar.className = 'liq-viewport-bar';
        var vpBtns = [
            { id: 'desktop', label: '🖥 Desktop', width: null },
            { id: 'tablet',  label: '📱 Tablet',  width: 768 },
            { id: 'mobile',  label: '📱 Mobile',  width: 390 }
        ];
        var _vpStyleEl = null;
        function _applyViewport(vp) {
            vpBar.querySelectorAll('.liq-vp-btn').forEach(function(b) {
                b.classList.toggle('is-active', b.dataset.vp === vp.id);
            });
            if (!_vpStyleEl) {
                _vpStyleEl = document.createElement('style');
                _vpStyleEl.id = 'liq-viewport-style';
                document.head.appendChild(_vpStyleEl);
            }
            if (vp.width) {
                _vpStyleEl.textContent = [
                    'html.liq-vp-active body {',
                    '    max-width: ' + vp.width + 'px !important;',
                    '    margin-left: 0 !important;',
                    '    margin-right: auto !important;',
                    '    overflow-x: hidden !important;',
                    '}'
                ].join('\n');
                document.documentElement.classList.add('liq-vp-active');
            } else {
                if (_vpStyleEl) _vpStyleEl.textContent = '';
                document.documentElement.classList.remove('liq-vp-active');
            }
        }
        vpBtns.forEach(function(vp) {
            var btn = document.createElement('button');
            btn.className = 'liq-vp-btn' + (vp.id === 'desktop' ? ' is-active' : '');
            btn.dataset.vp = vp.id;
            btn.textContent = vp.label;
            btn.addEventListener('click', function() { _applyViewport(vp); });
            vpBar.appendChild(btn);
        });
        drawer.appendChild(vpBar);

        // ── Colors Panel ──────────────────────────────────────────────
        var colorsPanel = document.createElement('div');
        colorsPanel.className = 'liq-playground-panel';
        colorsPanel.dataset.panel = 'colors';
        tabPanels['colors'] = colorsPanel;
        buildColorsPanel(colorsPanel);
        drawer.appendChild(colorsPanel);

        // ── Styles Panel ──
        var stylesPanel = document.createElement('div');
        stylesPanel.className = 'liq-playground-panel';
        stylesPanel.dataset.panel = 'styles';
        stylesPanel.style.display = 'none';
        tabPanels['styles'] = stylesPanel;

        // Presets as horizontal scrolling strip
        var presetsSection = document.createElement('div');
        presetsSection.className = 'liq-playground-section';
        var presetsHeading = document.createElement('h4');
        presetsHeading.textContent = 'Presets';
        presetsSection.appendChild(presetsHeading);

        var strip = document.createElement('div');
        strip.className = 'liq-playground-strip';

        state.variants.forEach(function(v) {
            var chip = document.createElement('div');
            chip.className = 'liq-playground-chip';
            chip.dataset.key = v.key;

            // Color circle showing header_bg with accent dot
            var circle = document.createElement('div');
            circle.className = 'liq-playground-chip-circle';
            circle.style.background = v.palette.header_bg || '#0f172a';
            var dot = document.createElement('span');
            dot.className = 'liq-playground-chip-dot';
            dot.style.background = v.palette.accent || '#3b82f6';
            circle.appendChild(dot);
            chip.appendChild(circle);

            // Name below circle
            var name = document.createElement('div');
            name.className = 'liq-playground-chip-name';
            name.textContent = v.label;
            chip.appendChild(name);

            chip.addEventListener('click', function() { applyPreset(v); });
            strip.appendChild(chip);
        });

        presetsSection.appendChild(strip);

        // Selected preset detail (shows description when one is active)
        var detailEl = document.createElement('div');
        detailEl.className = 'liq-playground-detail';
        detailEl.id = 'liqPlaygroundDetail';
        presetsSection.appendChild(detailEl);
        state.detailEl = detailEl;

        stylesPanel.appendChild(presetsSection);

        // ── My Designs section (saved user presets) ──
        var myDesignsSection = document.createElement('div');
        myDesignsSection.className = 'liq-playground-section';
        var myDesignsHeading = document.createElement('h4');
        myDesignsHeading.textContent = 'My Designs';
        myDesignsSection.appendChild(myDesignsHeading);
        var myDesignsList = document.createElement('div');
        myDesignsList.style.cssText = 'display:flex;flex-wrap:wrap;gap:8px;margin-bottom:8px;';
        myDesignsSection.appendChild(myDesignsList);

        var saveDesignBtn = document.createElement('button');
        saveDesignBtn.type = 'button';
        saveDesignBtn.textContent = 'Save Current Design';
        saveDesignBtn.style.cssText = 'width:100%;padding:8px;border:1px dashed rgba(255,255,255,0.25);background:none;color:inherit;border-radius:6px;cursor:pointer;font-size:12px;';
        myDesignsSection.appendChild(saveDesignBtn);

        function loadMyDesigns() {
            while (myDesignsList.firstChild) myDesignsList.removeChild(myDesignsList.firstChild);
            fetch('/api/modules/theme-studio/saved-designs')
                .then(function(r) { return r.json(); })
                .then(function(res) {
                    var designs = (res.ok !== false && Array.isArray(res.data)) ? res.data : [];
                    if (designs.length === 0) {
                        var hint = document.createElement('div');
                        hint.style.cssText = 'font-size:12px;opacity:0.5;padding:4px 0;';
                        hint.textContent = 'No saved designs yet. Customize colors and save.';
                        myDesignsList.appendChild(hint);
                        return;
                    }
                    designs.forEach(function(d) {
                        var chip = document.createElement('div');
                        chip.style.cssText = 'display:flex;align-items:center;gap:6px;padding:6px 12px;background:rgba(255,255,255,0.08);border-radius:8px;cursor:pointer;font-size:12px;';
                        var dot = document.createElement('span');
                        var pal = d.palette || {};
                        dot.style.cssText = 'width:14px;height:14px;border-radius:50%;background:' + (pal.primary || pal.accent || '#3b82f6') + ';flex-shrink:0;';
                        chip.appendChild(dot);
                        var label = document.createElement('span');
                        label.textContent = d.name;
                        chip.appendChild(label);
                        chip.addEventListener('click', function() {
                            if (d.palette) {
                                pushUndo();
                                applyPalette(d.palette);
                                if (state.applyBtn) state.applyBtn.style.display = '';
                                showToast('Applied: ' + d.name);
                            }
                        });
                        var delBtn = document.createElement('span');
                        delBtn.textContent = '\u00D7';
                        delBtn.title = 'Delete';
                        delBtn.style.cssText = 'margin-left:4px;color:#f87171;cursor:pointer;font-weight:700;font-size:14px;';
                        delBtn.addEventListener('click', function(e) {
                            e.stopPropagation();
                            if (!confirm('Delete "' + d.name + '"?')) return;
                            fetch('/api/modules/theme-studio/saved-designs/' + d.id, { method: 'DELETE' })
                                .then(function() { loadMyDesigns(); });
                        });
                        chip.appendChild(delBtn);
                        myDesignsList.appendChild(chip);
                    });
                })
                .catch(function() {});
        }
        loadMyDesigns();

        saveDesignBtn.addEventListener('click', function() {
            var name = prompt('Name this design (e.g. "Movie Night", "Mom\'s Pick"):');
            if (!name || !name.trim()) return;
            // Collect current palette from CSS variables
            var cs = getComputedStyle(document.documentElement);
            var palette = {
                primary: cs.getPropertyValue('--luperiq-primary').trim() || cs.getPropertyValue('--color-primary').trim(),
                accent: cs.getPropertyValue('--luperiq-accent').trim(),
                header_bg: cs.getPropertyValue('--luperiq-header-bg').trim(),
                header_text: cs.getPropertyValue('--luperiq-header-text').trim(),
                background: cs.getPropertyValue('--luperiq-background').trim(),
                surface: cs.getPropertyValue('--luperiq-surface').trim(),
                text: cs.getPropertyValue('--luperiq-text').trim(),
                link: cs.getPropertyValue('--luperiq-link').trim(),
            };
            saveDesignBtn.disabled = true;
            saveDesignBtn.textContent = 'Saving...';
            fetch('/api/modules/theme-studio/saved-designs', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ name: name.trim(), palette: palette })
            })
            .then(function(r) { return r.json(); })
            .then(function(res) {
                if (res.ok !== false) {
                    showToast('Design saved: ' + name.trim());
                    loadMyDesigns();
                } else {
                    showToast(res.error || 'Failed to save');
                }
            })
            .catch(function() { showToast('Network error'); })
            .finally(function() {
                saveDesignBtn.disabled = false;
                saveDesignBtn.textContent = 'Save Current Design';
            });
        });

        // ── Per-member "Save as My Theme" (family sites only) ──
        var myMemberId = null;
        (function detectFamilyMember() {
            // Fetch members and match by checking who the admin user is
            Promise.all([
                fetch('/api/modules/family-members/members').then(function(r) { return r.json(); }).catch(function() { return { data: [] }; }),
                fetch('/api/modules/industry-onboarding/needs-onboarding').then(function(r) { return r.json(); }).catch(function() { return {}; })
            ]).then(function(results) {
                var members = (results[0].data || []);
                var industryRes = results[1];
                var industrySlug = (industryRes.data && industryRes.data.industry_slug) || industryRes.industry_slug || '';
                if (industrySlug !== 'family' || members.length === 0) return;

                // Find the member linked to the current admin (has email)
                var linked = members.find(function(m) { return m.email && m.email.length > 3; });
                if (linked) myMemberId = linked.member_id;

                // Show the per-member button
                if (myMemberId) {
                    var myThemeBtn = document.createElement('button');
                    myThemeBtn.type = 'button';
                    myThemeBtn.textContent = 'Save as My Theme';
                    myThemeBtn.style.cssText = 'width:100%;padding:10px;border:none;background:#7c3aed;color:#fff;border-radius:8px;cursor:pointer;font-size:13px;font-weight:600;margin-top:6px;';
                    myThemeBtn.addEventListener('click', function() {
                        var cs = getComputedStyle(document.documentElement);
                        var palette = {
                            primary: cs.getPropertyValue('--luperiq-primary').trim() || cs.getPropertyValue('--color-primary').trim(),
                            accent: cs.getPropertyValue('--luperiq-accent').trim(),
                            header_bg: cs.getPropertyValue('--luperiq-header-bg').trim(),
                            background: cs.getPropertyValue('--luperiq-background').trim(),
                            surface: cs.getPropertyValue('--luperiq-surface').trim(),
                            text: cs.getPropertyValue('--luperiq-text').trim(),
                        };
                        myThemeBtn.disabled = true;
                        myThemeBtn.textContent = 'Saving...';
                        fetch('/api/modules/family-members/design-pref/' + myMemberId, {
                            method: 'PUT',
                            headers: { 'Content-Type': 'application/json' },
                            body: JSON.stringify({ palette: palette })
                        })
                        .then(function(r) { return r.json(); })
                        .then(function(res) {
                            if (res.ok) {
                                showToast('Saved as your personal theme! It will load when you log in.');
                                sessionStorage.setItem('liq-member-palette', JSON.stringify(palette));
                            } else {
                                showToast(res.message || 'Failed to save');
                            }
                        })
                        .catch(function() { showToast('Network error'); })
                        .finally(function() {
                            myThemeBtn.disabled = false;
                            myThemeBtn.textContent = 'Save as My Theme';
                        });
                    });
                    myDesignsSection.appendChild(myThemeBtn);

                    var hint = document.createElement('p');
                    hint.style.cssText = 'font-size:11px;opacity:0.5;margin-top:6px;text-align:center;';
                    hint.textContent = 'Your personal theme loads automatically when you log in.';
                    myDesignsSection.appendChild(hint);
                }
            });
        })();

        stylesPanel.appendChild(myDesignsSection);

        // Admin check — used by color pickers, navigation layout, menu editor, and apply button
        var isAdmin = !!document.getElementById('liqAdminToolbar');

        // ── Customize section ──
        var customSection = document.createElement('div');
        customSection.className = 'liq-playground-section';
        var customHeading = document.createElement('h4');
        customHeading.textContent = 'Customize';
        customSection.appendChild(customHeading);

        // Color scheme dropdown — independent from presets (no chip sync)
        customSection.appendChild(buildDropdown('Color Scheme', state.variants.map(function(v) {
            return { value: v.key, label: v.label };
        }), function(key) {
            pushUndo();
            var v = state.variants.find(function(x) { return x.key === key; });
            if (v) {
                // Clear preset selection — user is customizing individually
                state.activeKey = '';
                applyPalette(v.palette);
                updateSelection();
                if (state.applyBtn) state.applyBtn.style.display = '';
            }
        }));

        // ── Custom Color Pickers ──
        {
            var colorPickerWrap = document.createElement('div');
            colorPickerWrap.style.cssText = 'margin:12px 0 6px;';
            var cpLabel = document.createElement('div');
            cpLabel.style.cssText = 'font-size:12px;font-weight:600;margin-bottom:8px;opacity:0.8;';
            cpLabel.textContent = 'Custom Colors';
            colorPickerWrap.appendChild(cpLabel);

            var colorDefs = [
                { prop: '--luperiq-primary',    label: 'Primary' },
                { prop: '--luperiq-accent',     label: 'Accent' },
                { prop: '--luperiq-header-bg',  label: 'Header BG' },
                { prop: '--luperiq-header-text', label: 'Header Text' },
                { prop: '--luperiq-background', label: 'Page BG' },
                { prop: '--luperiq-text',       label: 'Text' }
            ];

            var colorGrid = document.createElement('div');
            colorGrid.style.cssText = 'display:grid;grid-template-columns:1fr 1fr 1fr;gap:6px;';

            colorDefs.forEach(function(cd) {
                var cell = document.createElement('div');
                cell.style.cssText = 'text-align:center;';

                var input = document.createElement('input');
                input.type = 'color';
                input.style.cssText = 'width:100%;height:32px;border:1px solid rgba(255,255,255,0.15);border-radius:6px;cursor:pointer;background:none;padding:0;';
                var cur = getComputedStyle(document.documentElement).getPropertyValue(cd.prop).trim();
                if (cur) input.value = cur.length === 7 ? cur : '#333333';
                input.addEventListener('input', function() {
                    pushUndo();
                    document.documentElement.style.setProperty(cd.prop, input.value);
                    state.activeKey = '';
                    updateSelection();
                    if (state.applyBtn) state.applyBtn.style.display = '';
                });
                cell.appendChild(input);

                var lbl = document.createElement('div');
                lbl.style.cssText = 'font-size:10px;opacity:0.6;margin-top:2px;';
                lbl.textContent = cd.label;
                cell.appendChild(lbl);

                colorGrid.appendChild(cell);
            });

            colorPickerWrap.appendChild(colorGrid);
            customSection.appendChild(colorPickerWrap);
        }

        // Header Style dropdown
        var headerOptions = [
            { value: 'trust-forward', label: 'Statement (Brand + Rotating Text + Nav)' },
            { value: 'friendly-local', label: 'Balanced (Brand + Nav + CTAs)' },
            { value: 'modern-edge', label: 'Editorial (Brand + Statement/Nav Split)' },
            { value: 'earth-guard', label: 'Minimal Clean (Brand + Nav Only)' },
            { value: 'clean-slate', label: 'Mega Nav Hero (Brand + Mega Nav + CTA)' }
        ];
        customSection.appendChild(buildDropdown('Header Style', headerOptions, function(key) {
            pushUndo();
            reloadHeaderForVariant(key);
            showToast('Header style updated');
            if (state.applyBtn) state.applyBtn.style.display = '';
        }));

        // Footer Style dropdown
        var footerOptions = [
            { value: 'trust-forward', label: 'Standard 3-Column' },
            { value: 'earth-guard', label: 'Dark Hero (Bold Statement)' },
            { value: 'clean-slate', label: 'Newsletter Focus (Lead Capture)' },
            { value: 'friendly-local', label: 'Minimal Bar (Clean & Simple)' },
            { value: 'modern-edge', label: 'Split CTA (Conversion Focus)' }
        ];
        customSection.appendChild(buildDropdown('Footer Style', footerOptions, function(key) {
            pushUndo();
            fetch('/api/modules/theme-studio/render/variant-preview?key=' + encodeURIComponent(key))
                .then(function(r) { return r.json(); })
                .then(function(data) {
                    if (data.ok && data.footer_html) {
                        var footerEl = document.querySelector('.luperiq-ts-layout--footer');
                        if (footerEl && footerEl.parentNode) {
                            var temp = document.createElement('div');
                            temp.innerHTML = data.footer_html;
                            var newFooter = temp.firstElementChild;
                            if (newFooter) {
                                footerEl.parentNode.replaceChild(newFooter, footerEl);
                            }
                        }
                    }
                })
                .catch(function() {});
            showToast('Footer style updated');
            if (state.applyBtn) state.applyBtn.style.display = '';
        }));

        // Nav Style dropdown
        var navStyleOptions = [
            { value: 'flat', label: 'Flat (Clean Text Links)' },
            { value: 'pill', label: 'Pill (Rounded Buttons)' },
            { value: 'underline', label: 'Underline (Animated Border)' }
        ];
        customSection.appendChild(buildDropdown('Nav Style', navStyleOptions, function(val) {
            pushUndo();
            // Remove existing nav style classes
            var navEl = document.querySelector('.luperiq-ts-nav');
            if (navEl) {
                navEl.classList.remove('luperiq-ts-nav--pill', 'luperiq-ts-nav--underline');
                if (val === 'pill') navEl.classList.add('luperiq-ts-nav--pill');
                else if (val === 'underline') navEl.classList.add('luperiq-ts-nav--underline');
            }
            // Also check mega nav
            var megaNav = document.querySelector('.luperiq-ts-mega-nav');
            if (megaNav) {
                megaNav.classList.remove('luperiq-ts-nav--pill', 'luperiq-ts-nav--underline');
                if (val === 'pill') megaNav.classList.add('luperiq-ts-nav--pill');
                else if (val === 'underline') megaNav.classList.add('luperiq-ts-nav--underline');
            }
            showToast('Nav style: ' + val);
            if (state.applyBtn) state.applyBtn.style.display = '';
        }));

        // Font dropdown
        var fontOptions = [
            { value: "system-ui, -apple-system, 'Segoe UI', Roboto, sans-serif", label: 'System (Clean)' },
            { value: "Seravek, 'Gill Sans Nova', Ubuntu, Calibri, sans-serif", label: 'Humanist (Warm)' },
            { value: "Charter, 'Bitstream Charter', 'Sitka Text', Cambria, serif", label: 'Transitional (Classic)' },
            { value: "'Iowan Old Style', 'Palatino Linotype', Palatino, serif", label: 'Old Style (Elegant)' },
            { value: "Avenir, Montserrat, Corbel, 'URW Gothic', sans-serif", label: 'Geometric (Modern)' },
            { value: "'Cascadia Code', 'Source Code Pro', Menlo, monospace", label: 'Mono (Technical)' }
        ];
        customSection.appendChild(buildDropdown('Body Font', fontOptions, function(val) {
            pushUndo();
            document.documentElement.style.setProperty('--luperiq-body-font', val);
            if (state.applyBtn) state.applyBtn.style.display = '';
        }));

        // Border radius dropdown
        var radiusOptions = [
            { value: '4', label: 'Sharp (4px)' },
            { value: '8', label: 'Subtle (8px)' },
            { value: '12', label: 'Rounded (12px)' },
            { value: '18', label: 'Soft (18px)' },
            { value: '24', label: 'Pill (24px)' }
        ];
        customSection.appendChild(buildDropdown('Corner Style', radiusOptions, function(val) {
            pushUndo();
            document.documentElement.style.setProperty('--luperiq-radius', val + 'px');
            if (state.applyBtn) state.applyBtn.style.display = '';
        }));

        stylesPanel.appendChild(customSection);

        // ── Navigation Layout section (admin only) ──
        if (isAdmin) {
            var navLayoutSection = document.createElement('div');
            navLayoutSection.className = 'liq-playground-section';
            var navLayoutHeading = document.createElement('h4');
            navLayoutHeading.textContent = 'Navigation Layout';
            navLayoutSection.appendChild(navLayoutHeading);

            var navLayoutDesc = document.createElement('p');
            navLayoutDesc.style.cssText = 'font-size:12px;opacity:0.65;margin:0 0 10px;line-height:1.4;';
            navLayoutDesc.textContent = 'Choose a navigation layout for the mega nav. This updates your site immediately.';
            navLayoutSection.appendChild(navLayoutDesc);

            var navLayoutStyles = [
                { value: 'simple_bar',      label: 'Simple Bar',       desc: 'Single row of text links, no dropdowns' },
                { value: 'classic',          label: 'Classic',          desc: 'Dropdown panels on hover/click (current)' },
                { value: 'two_row',          label: 'Two-Row',         desc: 'Category tabs on top, sub-items below' },
                { value: 'full_width_mega',  label: 'Full-Width Mega', desc: 'Full-width panel with grouped columns' },
                { value: 'card_grid_mega',   label: 'Card Grid Mega',  desc: 'Visual cards in a responsive grid' },
                { value: 'side_drawer',      label: 'Side Drawer',     desc: 'Slide-in panel from the left, always hamburger' },
                { value: 'command_palette',  label: 'Command Palette', desc: 'Search-style overlay with Ctrl+K shortcut' }
            ];

            // Detect current nav style from the DOM
            var currentNavEl = document.querySelector('[data-lq-nav-style]');
            var currentNavStyle = currentNavEl ? currentNavEl.getAttribute('data-lq-nav-style') : '';
            // Classic doesn't have data-lq-nav-style; check for the mega-nav class
            if (!currentNavStyle && document.querySelector('.luperiq-ts-mega-nav')) {
                currentNavStyle = 'classic';
            }

            var navGrid = document.createElement('div');
            navGrid.style.cssText = 'display:grid;grid-template-columns:1fr 1fr;gap:6px;';

            navLayoutStyles.forEach(function(ns) {
                var card = document.createElement('button');
                card.type = 'button';
                card.className = 'liq-playground-nav-card' + (ns.value === currentNavStyle ? ' is-active' : '');
                card.dataset.navStyle = ns.value;
                card.style.cssText = 'background:none;border:1px solid rgba(255,255,255,0.12);border-radius:8px;padding:10px;cursor:pointer;text-align:left;color:inherit;transition:border-color 0.15s, background 0.15s;';
                if (ns.value === currentNavStyle) {
                    card.style.borderColor = 'var(--luperiq-accent, #7c3aed)';
                    card.style.background = 'rgba(124,58,237,0.08)';
                }

                var cardLabel = document.createElement('div');
                cardLabel.style.cssText = 'font-size:13px;font-weight:600;margin-bottom:2px;';
                cardLabel.textContent = ns.label;
                card.appendChild(cardLabel);

                var cardDesc = document.createElement('div');
                cardDesc.style.cssText = 'font-size:11px;opacity:0.6;line-height:1.3;';
                cardDesc.textContent = ns.desc;
                card.appendChild(cardDesc);

                card.addEventListener('click', function() {
                    // Highlight selected
                    navGrid.querySelectorAll('.liq-playground-nav-card').forEach(function(c) {
                        c.classList.remove('is-active');
                        c.style.borderColor = 'rgba(255,255,255,0.12)';
                        c.style.background = 'none';
                    });
                    card.classList.add('is-active');
                    card.style.borderColor = 'var(--luperiq-accent, #7c3aed)';
                    card.style.background = 'rgba(124,58,237,0.08)';

                    // Call API to set nav style
                    showToast('Updating navigation layout...');
                    fetch('/api/modules/theme-studio/nav-layout-style', {
                        method: 'POST',
                        headers: { 'Content-Type': 'application/json' },
                        body: JSON.stringify({ nav_style: ns.value })
                    })
                    .then(function(r) { return r.json(); })
                    .then(function(data) {
                        if (data.ok) {
                            showToast('Navigation layout updated! Reloading...');
                            // Persist drawer open state so it reopens after reload
                            sessionStorage.setItem('liq-playground-state', JSON.stringify({
                                open: true,
                                activeTab: 'styles'
                            }));
                            setTimeout(function() { window.location.reload(); }, 600);
                        } else {
                            showToast(data.error || 'Failed to update nav layout');
                        }
                    })
                    .catch(function() {
                        showToast('Network error');
                    });
                });

                navGrid.appendChild(card);
            });

            navLayoutSection.appendChild(navGrid);
            stylesPanel.appendChild(navLayoutSection);

            // ── Menu Items Editor (admin only) ──
            var menuSection = document.createElement('div');
            menuSection.className = 'liq-playground-section';
            menuSection.id = 'liq-nav-menu-section';
            var menuHeading = document.createElement('h4');
            menuHeading.textContent = 'Menu Items';
            menuSection.appendChild(menuHeading);

            var menuDesc = document.createElement('p');
            menuDesc.style.cssText = 'font-size:12px;opacity:0.65;margin:0 0 10px;line-height:1.4;';
            menuDesc.textContent = 'Edit, add, or remove navigation links.';
            menuSection.appendChild(menuDesc);

            var menuList = document.createElement('div');
            menuList.id = 'liq-playground-menu-list';
            menuList.style.cssText = 'display:flex;flex-direction:column;gap:4px;margin-bottom:8px;';
            menuSection.appendChild(menuList);

            function renderMenuItems(items) {
                while (menuList.firstChild) menuList.removeChild(menuList.firstChild);
                items.sort(function(a,b) { return (a.position||0)-(b.position||0); });
                items.forEach(function(item, idx) {
                    if (item.parent_id) return;
                    var row = document.createElement('div');
                    row.style.cssText = 'display:flex;gap:4px;align-items:center;';

                    var titleInput = document.createElement('input');
                    titleInput.type = 'text';
                    titleInput.value = item.title;
                    titleInput.placeholder = 'Label';
                    titleInput.style.cssText = 'flex:1;padding:6px 8px;border-radius:6px;border:1px solid rgba(255,255,255,0.15);background:rgba(255,255,255,0.06);color:inherit;font-size:12px;';
                    row.appendChild(titleInput);

                    var urlInput = document.createElement('input');
                    urlInput.type = 'text';
                    urlInput.value = item.url;
                    urlInput.placeholder = '/path';
                    urlInput.style.cssText = 'width:80px;padding:6px 8px;border-radius:6px;border:1px solid rgba(255,255,255,0.15);background:rgba(255,255,255,0.06);color:inherit;font-size:12px;';
                    row.appendChild(urlInput);

                    if (idx > 0) {
                        var upBtn = document.createElement('button');
                        upBtn.type = 'button';
                        upBtn.textContent = '\u2191';
                        upBtn.title = 'Move up';
                        upBtn.style.cssText = 'padding:4px 6px;border:none;background:rgba(255,255,255,0.08);color:inherit;border-radius:4px;cursor:pointer;font-size:12px;';
                        upBtn.addEventListener('click', function() {
                            var t = items[idx]; items[idx] = items[idx-1]; items[idx-1] = t;
                            items.forEach(function(it,i) { it.position = i+1; });
                            renderMenuItems(items);
                        });
                        row.appendChild(upBtn);
                    }

                    var delBtn = document.createElement('button');
                    delBtn.type = 'button';
                    delBtn.textContent = '\u00D7';
                    delBtn.title = 'Remove';
                    delBtn.style.cssText = 'padding:4px 8px;border:none;background:rgba(239,68,68,0.15);color:#f87171;border-radius:4px;cursor:pointer;font-size:14px;font-weight:700;';
                    delBtn.addEventListener('click', function() {
                        items.splice(idx, 1);
                        items.forEach(function(it,i) { it.position = i+1; });
                        renderMenuItems(items);
                    });
                    row.appendChild(delBtn);

                    menuList.appendChild(row);
                });
            }

            var addItemBtn = document.createElement('button');
            addItemBtn.type = 'button';
            addItemBtn.textContent = '+ Add Link';
            addItemBtn.style.cssText = 'width:100%;padding:8px;border:1px dashed rgba(255,255,255,0.2);background:none;color:inherit;border-radius:6px;cursor:pointer;font-size:12px;margin-bottom:8px;';

            var saveMenuBtn = document.createElement('button');
            saveMenuBtn.type = 'button';
            saveMenuBtn.textContent = 'Save Menu';
            saveMenuBtn.style.cssText = 'width:100%;padding:10px;border:none;background:#3b82f6;color:#fff;border-radius:8px;cursor:pointer;font-size:13px;font-weight:600;';

            var navItems = [];

            fetch('/api/modules/theme-studio/nav/primary')
                .then(function(r) { return r.json(); })
                .then(function(data) {
                    if (data.ok !== false && data.items) {
                        navItems = data.items.filter(function(it) { return !it.parent_id; });
                    } else if (data.items) {
                        navItems = data.items;
                    }
                    renderMenuItems(navItems);
                })
                .catch(function() {});

            addItemBtn.addEventListener('click', function() {
                navItems.push({
                    item_id: 'nav-new-' + Date.now(), parent_id: null,
                    title: 'New Link', url: '/', position: navItems.length + 1,
                    description: null, icon: null, css_classes: [], category: null, badge: null
                });
                renderMenuItems(navItems);
            });

            saveMenuBtn.addEventListener('click', function() {
                var rows = menuList.querySelectorAll('div');
                var topIdx = 0;
                rows.forEach(function(row) {
                    var inputs = row.querySelectorAll('input');
                    if (inputs.length >= 2 && topIdx < navItems.length) {
                        navItems[topIdx].title = inputs[0].value.trim() || 'Link';
                        navItems[topIdx].url = inputs[1].value.trim() || '/';
                        topIdx++;
                    }
                });
                saveMenuBtn.disabled = true;
                saveMenuBtn.textContent = 'Saving...';
                fetch('/api/modules/theme-studio/nav/primary', {
                    method: 'PUT',
                    headers: { 'Content-Type': 'application/json' },
                    body: JSON.stringify({ location: 'primary', items: navItems })
                })
                .then(function(r) { return r.json(); })
                .then(function(data) {
                    if (data.ok) {
                        showToast('Menu saved! Reloading...');
                        sessionStorage.setItem('liq-playground-state', JSON.stringify({ open: true, activeTab: 'styles' }));
                        setTimeout(function() { window.location.reload(); }, 600);
                    } else {
                        showToast(data.error || 'Failed to save menu');
                        saveMenuBtn.disabled = false;
                        saveMenuBtn.textContent = 'Save Menu';
                    }
                })
                .catch(function() {
                    showToast('Network error');
                    saveMenuBtn.disabled = false;
                    saveMenuBtn.textContent = 'Save Menu';
                });
            });

            menuSection.appendChild(addItemBtn);
            menuSection.appendChild(saveMenuBtn);
            stylesPanel.appendChild(menuSection);
        }

        // Undo button
        var undoBtn = document.createElement('button');
        undoBtn.className = 'liq-playground-undo';
        undoBtn.textContent = 'Undo Last Change';
        undoBtn.style.display = 'none';
        undoBtn.addEventListener('click', popUndo);
        stylesPanel.appendChild(undoBtn);
        state.undoBtn = undoBtn;

        // Reset button
        var resetBtn = document.createElement('button');
        resetBtn.className = 'liq-playground-reset';
        resetBtn.textContent = 'Reset to Original';
        resetBtn.addEventListener('click', restoreOriginals);
        stylesPanel.appendChild(resetBtn);

        // Apply button — only for admins (non-admins can preview but not persist)
        var applyBtn = document.createElement('button');
        applyBtn.className = 'liq-playground-apply';
        applyBtn.textContent = 'Apply This Design';
        applyBtn.style.display = 'none'; // Hidden until a preset is selected
        applyBtn.addEventListener('click', function() {
            if (!state.activeKey) {
                showToast('Select a preset first');
                return;
            }
            applyBtn.disabled = true;
            applyBtn.textContent = 'Applying...';

            fetch('/api/modules/theme-studio/profiles/activate', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ variant_key: state.activeKey })
            })
            .then(function(r) { return r.json(); })
            .then(function(data) {
                if (data.ok) {
                    showToast('Design applied! Reloading...');
                    setTimeout(function() { window.location.reload(); }, 1000);
                } else {
                    showToast(data.error || 'Failed to apply design');
                    applyBtn.disabled = false;
                    applyBtn.textContent = 'Apply This Design';
                }
            })
            .catch(function() {
                showToast('Network error — please try again');
                applyBtn.disabled = false;
                applyBtn.textContent = 'Apply This Design';
            });
        });
        if (isAdmin) { stylesPanel.appendChild(applyBtn); }
        state.applyBtn = isAdmin ? applyBtn : null;

        // Save & Share section
        var shareSection = document.createElement('div');
        shareSection.className = 'liq-playground-section';
        var shareHeading = document.createElement('h4');
        shareHeading.textContent = 'Save & Share';
        shareSection.appendChild(shareHeading);

        // Export button
        var exportBtn = document.createElement('button');
        exportBtn.className = 'liq-playground-export';
        exportBtn.textContent = 'Export Design';
        exportBtn.addEventListener('click', function() {
            exportBtn.disabled = true;
            exportBtn.textContent = 'Exporting...';
            fetch('/api/modules/theme-studio/active-tokens')
                .then(function(r) { return r.json(); })
                .then(function(res) {
                    if (!res.ok || !res.data) throw new Error(res.message || 'No tokens');
                    var payload = {
                        _liq_design_version: 2,
                        tokens: res.data
                    };
                    var json = JSON.stringify(payload, null, 2);
                    var blob = new Blob([json], { type: 'application/json' });
                    var a = document.createElement('a');
                    a.href = URL.createObjectURL(blob);
                    a.download = 'luperiq-design.json';
                    a.click();
                    URL.revokeObjectURL(a.href);
                    showToast('Design exported!');
                })
                .catch(function(e) { showToast('Export failed: ' + e.message); })
                .finally(function() {
                    exportBtn.disabled = false;
                    exportBtn.textContent = 'Export Design';
                });
        });
        shareSection.appendChild(exportBtn);

        // Import button
        var importWrap = document.createElement('div');
        importWrap.style.marginTop = '8px';
        var importBtn = document.createElement('button');
        importBtn.className = 'liq-playground-import';
        importBtn.textContent = 'Import Design';
        var importInput = document.createElement('input');
        importInput.type = 'file';
        importInput.accept = '.json';
        importInput.style.display = 'none';
        importInput.addEventListener('change', function(e) {
            var file = e.target.files[0];
            if (!file) return;
            var reader = new FileReader();
            reader.onload = function(ev) {
                try {
                    var parsed = JSON.parse(ev.target.result);
                    // Support both v2 (tokens object) and legacy v1 (palette object)
                    var tokenPayload = null;
                    if (parsed._liq_design_version === 2 && parsed.tokens) {
                        tokenPayload = parsed.tokens;
                    } else if (parsed.version === 1 && parsed.palette) {
                        // Legacy: reconstruct minimal token payload from old format
                        tokenPayload = parsed.palette;
                        if (parsed.font) tokenPayload.body_font_css = parsed.font;
                        if (parsed.radius) tokenPayload.radius = parseInt(parsed.radius, 10) || 8;
                    }
                    if (!tokenPayload) { showToast('Invalid design file'); return; }
                    importBtn.disabled = true;
                    importBtn.textContent = 'Importing...';
                    fetch('/api/modules/theme-studio/active-tokens', {
                        method: 'PUT',
                        headers: { 'Content-Type': 'application/json' },
                        body: JSON.stringify(tokenPayload)
                    })
                    .then(function(r) { return r.json(); })
                    .then(function(res) {
                        if (res.ok) {
                            showToast('Design imported! Reloading...');
                            setTimeout(function() { window.location.reload(); }, 800);
                        } else {
                            showToast(res.message || 'Import failed');
                        }
                    })
                    .catch(function() { showToast('Network error during import'); })
                    .finally(function() {
                        importBtn.disabled = false;
                        importBtn.textContent = 'Import Design';
                        importInput.value = '';
                    });
                } catch(err) {
                    showToast('Could not read design file');
                }
            };
            reader.readAsText(file);
        });
        importBtn.addEventListener('click', function() { importInput.click(); });
        importWrap.appendChild(importBtn);
        importWrap.appendChild(importInput);
        shareSection.appendChild(importWrap);

        stylesPanel.appendChild(shareSection);

        // Info text
        var info = document.createElement('p');
        info.className = 'liq-playground-info';
        info.textContent = 'Changes are preview-only. Your site is not affected until you choose to apply a design.';
        stylesPanel.appendChild(info);

        drawer.appendChild(stylesPanel);

        // ── Layouts Panel ─────────────────────────────────────────────
        var layoutsPanel = document.createElement('div');
        layoutsPanel.className = 'liq-playground-panel';
        layoutsPanel.dataset.panel = 'layouts';
        layoutsPanel.style.display = 'none';
        tabPanels['layouts'] = layoutsPanel;
        buildLayoutsPanel(layoutsPanel);
        drawer.appendChild(layoutsPanel);

        // ── Blocks Panel (Smart Blocks + Primitives combined) ─────────
        var blocksPanel = document.createElement('div');
        blocksPanel.className = 'liq-playground-panel';
        blocksPanel.dataset.panel = 'blocks';
        blocksPanel.style.display = 'none';
        tabPanels['blocks'] = blocksPanel;
        rebuildCombinedBlocksPanel(blocksPanel);
        drawer.appendChild(blocksPanel);

        document.body.appendChild(drawer);
    }

    // ── Toggle drawer ────────────────────────────────────────────────
    function toggleDrawer() {
        state.open = !state.open;
        var drawer = document.getElementById('liqPlaygroundDrawer');
        var tab = document.querySelector('.liq-playground-tab');
        if (state.open) {
            drawer.classList.add('is-open');
            tab.classList.add('is-open');
        } else {
            drawer.classList.remove('is-open');
            tab.classList.remove('is-open');
        }
        // Persist drawer state across page reloads
        sessionStorage.setItem('liq-playground-state', JSON.stringify({
            open: state.open,
            activeTab: (document.querySelector('.liq-playground-tab-btn.is-active') || {}).dataset && (document.querySelector('.liq-playground-tab-btn.is-active') || {}).dataset.tab || 'styles'
        }));
    }

    // ── Custom CSS section ─────────────────────────────────────────────
    function buildCustomCssSection(panel) {
        var sec = document.createElement('div');
        sec.className = 'liq-playground-section';

        var hdr = document.createElement('h4');
        hdr.style.cssText = 'cursor:pointer;user-select:none;display:flex;justify-content:space-between;align-items:center;margin-bottom:0;';
        var htitle = document.createElement('span');
        htitle.textContent = 'Custom CSS';
        hdr.appendChild(htitle);
        var arrow = document.createElement('span');
        arrow.textContent = '▶';
        arrow.style.cssText = 'font-size:10px;opacity:0.6;';
        hdr.appendChild(arrow);
        sec.appendChild(hdr);

        var body = document.createElement('div');
        body.style.cssText = 'display:none;margin-top:10px;';
        sec.appendChild(body);

        var hint = document.createElement('div');
        hint.textContent = 'Applied after all theme tokens. Changes preview live — click Save Colors to persist.';
        hint.style.cssText = 'font-size:10px;color:#64748b;margin-bottom:8px;line-height:1.4;';
        body.appendChild(hint);

        var ta = document.createElement('textarea');
        ta.rows = 10;
        ta.placeholder = '/* e.g. */\n.site-header { border-bottom: 2px solid var(--luperiq-accent); }\n.nav-link-cta { font-size: 13px; }';
        ta.spellcheck = false;
        ta.style.cssText = [
            'width:100%;box-sizing:border-box;',
            'padding:8px;',
            'background:#0a0e1a;',
            'border:1px solid rgba(255,255,255,0.15);',
            'border-radius:6px;',
            'color:#e2e8f0;',
            'font-family:"Cascadia Code","Source Code Pro",Menlo,Consolas,monospace;',
            'font-size:11px;',
            'line-height:1.5;',
            'resize:vertical;',
            'outline:none;',
            'tab-size:2;'
        ].join('');
        ta.addEventListener('focus', function() { ta.style.borderColor = '#a78bfa'; });
        ta.addEventListener('blur',  function() { ta.style.borderColor = 'rgba(255,255,255,0.15)'; });

        // Sync textarea → tokens on input (live preview via applyFullTokensToCSS)
        var _cssTokensRef = null; // set when panel opens
        ta.addEventListener('input', function() {
            if (_cssTokensRef) {
                _cssTokensRef.custom_css = ta.value || null;
                applyFullTokensToCSS(_cssTokensRef);
            }
        });

        // Tab key inserts two spaces instead of losing focus
        ta.addEventListener('keydown', function(e) {
            if (e.key === 'Tab') {
                e.preventDefault();
                var s = ta.selectionStart, end = ta.selectionEnd;
                ta.value = ta.value.substring(0, s) + '  ' + ta.value.substring(end);
                ta.selectionStart = ta.selectionEnd = s + 2;
            }
        });

        body.appendChild(ta);

        hdr.addEventListener('click', function() {
            var collapsed = body.style.display === 'none';
            body.style.display = collapsed ? '' : 'none';
            arrow.textContent = collapsed ? '▼' : '▶';
        });

        sec.dataset.customCssSection = '1';
        panel.appendChild(sec);

        // Return a function that wires the textarea to a specific tokens object
        // Called from buildColorsPanel after tokens is in scope
        return function(tokensRef) {
            _cssTokensRef = tokensRef;
            ta.value = tokensRef.custom_css || '';
            state._pgCustomCssTa = ta;
        };
    }


    // ── Zone editing helpers (admin header/footer click-to-edit) ───────
    function injectZoneCss() {
        if (document.getElementById('liq-zone-style')) return;
        var s = document.createElement('style');
        s.id = 'liq-zone-style';
        s.textContent = [
            '[data-liq-zone] { position: relative; cursor: pointer; }',
            '[data-liq-zone]:hover { outline: 2px dashed #3b82f6; outline-offset: 3px; z-index: 1; }',
            '.liq-zone-badge {',
            '    display: none; position: absolute; top: 4px; right: 4px; z-index: 2000;',
            '    background: #3b82f6; color: #fff; font-size: 10px; font-weight: 700;',
            '    padding: 2px 6px; border-radius: 3px; pointer-events: none; white-space: nowrap;',
            '    font-family: system-ui, sans-serif; line-height: 1.4;',
            '}',
            '[data-liq-zone]:hover > .liq-zone-badge { display: block; }'
        ].join('\n');
        document.head.appendChild(s);
        var zoneLabels = {
            brand: '✎ Brand',
            nav: '✎ Nav',
            contact: '✎ Contact',
            footer: '✎ Footer'
        };
        document.querySelectorAll('[data-liq-zone]').forEach(function(el) {
            var badge = document.createElement('span');
            badge.className = 'liq-zone-badge';
            badge.textContent = zoneLabels[el.dataset.liqZone] || '✎ Edit';
            el.appendChild(badge);
        });
    }

    function wireZoneClicks() {
        // Mobile hamburger buttons redirect to Styles → Menu section
        document.querySelectorAll('.mobile-toggle').forEach(function(btn) {
            btn.addEventListener('click', function(e) {
                e.preventDefault();
                e.stopPropagation();
                e.stopImmediatePropagation();
                if (!state.open) toggleDrawer();
                var stylesBtn = document.querySelector('.liq-playground-tab-btn[data-tab="styles"]');
                if (stylesBtn && !stylesBtn.classList.contains('is-active')) stylesBtn.click();
                setTimeout(function() {
                    var sec = document.getElementById('liq-nav-menu-section');
                    if (!sec) return;
                    sec.scrollIntoView({ behavior: 'smooth', block: 'nearest' });
                    sec.style.transition = 'background 0.3s ease';
                    sec.style.background = 'rgba(59,130,246,0.12)';
                    setTimeout(function() { sec.style.background = ''; }, 1100);
                }, 120);
            }, true);
        });
        document.querySelectorAll('[data-liq-zone]').forEach(function(el) {
            var zone = el.dataset.liqZone;
            if (!zone) return;
            el.addEventListener('click', function(e) {
                e.preventDefault();
                e.stopPropagation();
                e.stopImmediatePropagation();
                if (!state.open) toggleDrawer();
                if (zone === 'nav') {
                    // Nav zone → open Styles tab → scroll to Menu Items section
                    var stylesBtn = document.querySelector('.liq-playground-tab-btn[data-tab="styles"]');
                    if (stylesBtn && !stylesBtn.classList.contains('is-active')) stylesBtn.click();
                    setTimeout(function() {
                        var sec = document.getElementById('liq-nav-menu-section');
                        if (!sec) return;
                        sec.scrollIntoView({ behavior: 'smooth', block: 'nearest' });
                        sec.style.transition = 'background 0.3s ease';
                        sec.style.background = 'rgba(59,130,246,0.12)';
                        setTimeout(function() { sec.style.background = ''; }, 1100);
                    }, 120);
                } else {
                    // Brand / contact / footer zones → open Colors tab → scroll to H&F section
                    var colorsBtn = document.querySelector('.liq-playground-tab-btn[data-tab="colors"]');
                    if (colorsBtn && !colorsBtn.classList.contains('is-active')) colorsBtn.click();
                    setTimeout(function() {
                        var sec = document.getElementById('liq-hf-section');
                        if (!sec) return;
                        var body = sec.querySelector('.liq-hf-body');
                        if (body && body.style.display === 'none') sec.querySelector('h4').click();
                        sec.scrollIntoView({ behavior: 'smooth', block: 'nearest' });
                        sec.style.transition = 'background 0.3s ease';
                        sec.style.background = 'rgba(59,130,246,0.12)';
                        setTimeout(function() { sec.style.background = ''; }, 1100);
                        var focusMap = { brand: 'liq-hf-name', contact: 'liq-hf-phone', footer: 'liq-hf-email' };
                        var fid = focusMap[zone];
                        if (fid) {
                            var fld = document.getElementById(fid);
                            if (fld) fld.focus();
                        }
                    }, 120);
                }
            }, true);
        });
    }


    // ── Responsive overrides section ───────────────────────────────────────
    function buildResponsiveSection(panel) {
        var sec = document.createElement('div');
        sec.className = 'liq-playground-section';

        var hdr = document.createElement('h4');
        hdr.style.cssText = 'cursor:pointer;user-select:none;display:flex;justify-content:space-between;align-items:center;margin-bottom:0;';
        var htitle = document.createElement('span');
        htitle.textContent = 'Responsive Overrides';
        hdr.appendChild(htitle);
        var arrow = document.createElement('span');
        arrow.textContent = '\u25b6';
        arrow.style.cssText = 'font-size:10px;opacity:0.6;';
        hdr.appendChild(arrow);
        sec.appendChild(hdr);

        var body = document.createElement('div');
        body.style.cssText = 'display:none;margin-top:10px;';
        sec.appendChild(body);

        var hint = document.createElement('div');
        hint.textContent = 'Override specific tokens at tablet (\u2264980px) and mobile (\u2264860px) breakpoints. Preview updates live as you resize the window.';
        hint.style.cssText = 'font-size:10px;color:#64748b;margin-bottom:12px;line-height:1.4;';
        body.appendChild(hint);

        var _respRef = null; // set via returned wiring fn

        var _respFields = [
            { key: 'body_size',  label: 'Body Size',       min: 10, max: 24, unit: 'px' },
            { key: 'radius',     label: 'Border Radius',   min: 0,  max: 32, unit: 'px' },
            { key: 'container',  label: 'Container Width', min: 600, max: 1600, unit: 'px' },
            { key: 'nav_size',   label: 'Nav Font Size',   min: 10, max: 24, unit: 'px' },
            { key: 'nav_gap',    label: 'Nav Gap',         min: 2,  max: 48, unit: 'px' }
        ];

        function buildBpSection(bpLabel, bpKey, bpBreakpoint) {
            var bpWrap = document.createElement('div');
            bpWrap.style.cssText = 'margin-bottom:14px;';

            var bpHdr = document.createElement('div');
            bpHdr.style.cssText = 'display:flex;align-items:center;justify-content:space-between;margin-bottom:6px;';

            var bpTitle = document.createElement('span');
            bpTitle.textContent = bpLabel + ' (\u2264' + bpBreakpoint + 'px)';
            bpTitle.style.cssText = 'font-size:10px;font-weight:700;color:#94a3b8;text-transform:uppercase;letter-spacing:0.5px;';
            bpHdr.appendChild(bpTitle);

            var clearBtn = document.createElement('button');
            clearBtn.type = 'button';
            clearBtn.textContent = 'Clear';
            clearBtn.title = 'Remove all ' + bpLabel + ' overrides';
            clearBtn.style.cssText = 'font-size:9px;padding:1px 6px;background:none;border:1px solid #475569;color:#94a3b8;border-radius:3px;cursor:pointer;';
            bpHdr.appendChild(clearBtn);
            bpWrap.appendChild(bpHdr);

            var sliders = {};
            var valueLabels = {};

            _respFields.forEach(function(f) {
                var row = document.createElement('div');
                row.style.cssText = 'display:flex;align-items:center;gap:6px;margin-bottom:5px;';

                var lbl = document.createElement('span');
                lbl.textContent = f.label;
                lbl.style.cssText = 'width:88px;font-size:10px;color:#94a3b8;flex-shrink:0;';

                var slider = document.createElement('input');
                slider.type = 'range';
                slider.min = String(f.min);
                slider.max = String(f.max);
                slider.style.cssText = 'flex:1;accent-color:#a78bfa;cursor:pointer;opacity:0.5;';

                var val = document.createElement('span');
                val.style.cssText = 'width:36px;font-size:10px;color:#e2e8f0;text-align:right;flex-shrink:0;font-family:monospace;';

                slider.addEventListener('input', function() {
                    var n = parseInt(slider.value, 10);
                    val.textContent = n + f.unit;
                    slider.style.opacity = '1';
                    if (_respRef) {
                        if (!_respRef.tokens[bpKey]) _respRef.tokens[bpKey] = {};
                        _respRef.tokens[bpKey][f.key] = n;
                        applyFullTokensToCSS(_respRef.tokens);
                    }
                });

                sliders[f.key] = slider;
                valueLabels[f.key] = val;
                row.appendChild(lbl); row.appendChild(slider); row.appendChild(val);
                bpWrap.appendChild(row);
            });

            clearBtn.addEventListener('click', function() {
                if (_respRef) {
                    _respRef.tokens[bpKey] = null;
                    applyFullTokensToCSS(_respRef.tokens);
                    // Reset sliders to desktop values
                    _respFields.forEach(function(f) {
                        if (_respRef && _respRef.tokens) {
                            var dv = _respRef.tokens[f.key] != null ? _respRef.tokens[f.key] : parseInt(sliders[f.key].min, 10);
                            sliders[f.key].value = String(dv);
                            sliders[f.key].style.opacity = '0.5';
                            valueLabels[f.key].textContent = dv + f.unit;
                        }
                    });
                }
            });

            bpWrap._syncFromTokens = function(tokens) {
                var ov = tokens[bpKey] || {};
                _respFields.forEach(function(f) {
                    var desktop = tokens[f.key] != null ? tokens[f.key] : parseInt(sliders[f.key].min, 10);
                    var v = ov[f.key] != null ? ov[f.key] : desktop;
                    sliders[f.key].value = String(v);
                    sliders[f.key].style.opacity = ov[f.key] != null ? '1' : '0.5';
                    valueLabels[f.key].textContent = v + f.unit;
                });
            };

            return bpWrap;
        }

        var tabletSec = buildBpSection('Tablet', 'tablet', 980);
        var mobileSec = buildBpSection('Mobile', 'mobile', 860);
        body.appendChild(tabletSec);
        body.appendChild(mobileSec);

        hdr.addEventListener('click', function() {
            var collapsed = body.style.display === 'none';
            body.style.display = collapsed ? '' : 'none';
            arrow.textContent = collapsed ? '\u25bc' : '\u25b6';
            if (collapsed && _respRef) {
                tabletSec._syncFromTokens(_respRef.tokens);
                mobileSec._syncFromTokens(_respRef.tokens);
            }
        });

        panel.appendChild(sec);

        return function(tokensRef) {
            _respRef = tokensRef;
            state._pgResponsiveSyncFn = function(t) {
                _respRef = { tokens: t };
                if (body.style.display !== 'none') {
                    tabletSec._syncFromTokens(t);
                    mobileSec._syncFromTokens(t);
                }
            };
        };
    }

    function buildHeaderFooterSection(panel) {
        var sec = document.createElement('div');
        sec.className = 'liq-playground-section';
        sec.id = 'liq-hf-section';
        sec.style.transition = 'background 0.4s ease';

        var hdr = document.createElement('h4');
        hdr.style.cssText = 'cursor:pointer;user-select:none;display:flex;justify-content:space-between;align-items:center;margin-bottom:0;';
        var htitle = document.createElement('span');
        htitle.textContent = 'Header & Footer';
        hdr.appendChild(htitle);
        var arrow = document.createElement('span');
        arrow.textContent = '▶';
        arrow.style.cssText = 'font-size:10px;opacity:0.6;';
        hdr.appendChild(arrow);
        sec.appendChild(hdr);

        var body = document.createElement('div');
        body.className = 'liq-hf-body';
        body.style.display = 'none';
        body.style.marginTop = '12px';
        sec.appendChild(body);

        var inputs = {};
        var fieldDefs = [
            { id: 'liq-hf-name',    label: 'Business Name', key: 'name',     type: 'text',  ph: 'Your Business Name' },
            { id: 'liq-hf-logo',    label: 'Logo URL',      key: 'logo_url', type: 'text',  ph: 'https://...' },
            { id: 'liq-hf-favicon', label: 'Favicon URL',   key: 'favicon_url', type: 'text',  ph: 'https://... or /static/favicon.png' },
            { id: 'liq-hf-phone',   label: 'Phone',         key: 'phone',    type: 'tel',   ph: '(555) 123-4567' },
            { id: 'liq-hf-email',   label: 'Email',         key: 'email',    type: 'email', ph: 'info@example.com' },
            { id: 'liq-hf-address', label: 'Address',       key: 'address',  type: 'text',  ph: '123 Main St, City, ST' }
        ];

        fieldDefs.forEach(function(f) {
            var row = document.createElement('div');
            row.style.marginBottom = '8px';

            var lbl = document.createElement('label');
            lbl.htmlFor = f.id;
            lbl.textContent = f.label;
            lbl.style.cssText = 'display:block;font-size:11px;font-weight:600;opacity:0.65;margin-bottom:3px;';

            var inp = document.createElement('input');
            inp.id = f.id;
            inp.type = f.type;
            inp.placeholder = f.ph;
            inp.style.cssText = 'width:100%;box-sizing:border-box;padding:6px 8px;border:1px solid rgba(255,255,255,0.2);border-radius:6px;background:rgba(255,255,255,0.07);color:inherit;font-size:12px;outline:none;';
            inp.addEventListener('focus', function() { inp.style.borderColor = '#3b82f6'; });
            inp.addEventListener('blur',  function() { inp.style.borderColor = 'rgba(255,255,255,0.2)'; });

            inputs[f.key] = inp;
            row.appendChild(lbl);
            row.appendChild(inp);

            if (f.key === 'logo_url' || f.key === 'favicon_url') {
                var _isLogo = f.key === 'logo_url';
                var uRow = document.createElement('div');
                uRow.style.marginTop = '4px';
                var uBtn = document.createElement('button');
                uBtn.type = 'button';
                uBtn.textContent = _isLogo ? '↑ Upload Logo' : '↑ Upload Favicon';
                uBtn.style.cssText = 'width:100%;padding:5px 8px;border:1px dashed rgba(255,255,255,0.2);background:none;color:inherit;border-radius:6px;cursor:pointer;font-size:11px;';
                var uInp = document.createElement('input');
                uInp.type = 'file';
                uInp.accept = _isLogo ? 'image/*' : 'image/x-icon,image/png,image/svg+xml,image/webp,image/*';
                uInp.style.display = 'none';
                var _fkey = f.key;
                var _btnLabel = uBtn.textContent;
                uBtn.addEventListener('click', function() { uInp.click(); });
                uInp.addEventListener('change', function() {
                    var file = uInp.files && uInp.files[0];
                    if (!file) return;
                    var fd = new FormData();
                    fd.append('file', file);
                    uBtn.disabled = true;
                    uBtn.textContent = 'Uploading...';
                    fetch('/api/media/upload', { method: 'POST', body: fd })
                        .then(function(r) { return r.json(); })
                        .then(function(res) {
                            if (res.ok && res.data && res.data.url) {
                                inputs[_fkey].value = res.data.url;
                                showToast((_isLogo ? 'Logo' : 'Favicon') + ' uploaded ✓');
                            } else {
                                showToast('Upload failed: ' + (res.message || 'error'));
                            }
                        })
                        .catch(function() { showToast('Upload error'); })
                        .finally(function() {
                            uBtn.disabled = false;
                            uBtn.textContent = _btnLabel;
                            uInp.value = '';
                        });
                });
                uRow.appendChild(uBtn);
                uRow.appendChild(uInp);
                row.appendChild(uRow);
            }

            body.appendChild(row);
        });

        // ── Social Links sub-section ──────────────────────────────────
        var socHdr = document.createElement('div');
        socHdr.style.cssText = 'font-size:10px;font-weight:700;color:#94a3b8;text-transform:uppercase;letter-spacing:0.5px;margin:12px 0 8px;';
        socHdr.textContent = 'Social Links';
        body.appendChild(socHdr);

        var socFields = [
            { key: 'facebook',  label: 'Facebook',   ph: 'https://facebook.com/yourpage' },
            { key: 'instagram', label: 'Instagram',  ph: 'https://instagram.com/yourhandle' },
            { key: 'twitter',   label: 'Twitter / X', ph: 'https://x.com/yourhandle' },
            { key: 'youtube',   label: 'YouTube',    ph: 'https://youtube.com/@yourchannel' },
            { key: 'linkedin',  label: 'LinkedIn',   ph: 'https://linkedin.com/company/yours' }
        ];
        socFields.forEach(function(sf) {
            var sRow = document.createElement('div');
            sRow.style.marginBottom = '8px';
            var sLbl = document.createElement('label');
            sLbl.textContent = sf.label;
            sLbl.style.cssText = 'display:block;font-size:11px;font-weight:600;opacity:0.65;margin-bottom:3px;';
            var sInp = document.createElement('input');
            sInp.type = 'url';
            sInp.placeholder = sf.ph;
            sInp.style.cssText = 'width:100%;box-sizing:border-box;padding:6px 8px;border:1px solid rgba(255,255,255,0.2);border-radius:6px;background:rgba(255,255,255,0.07);color:inherit;font-size:12px;outline:none;';
            sInp.addEventListener('focus', function() { sInp.style.borderColor = '#3b82f6'; });
            sInp.addEventListener('blur',  function() { sInp.style.borderColor = 'rgba(255,255,255,0.2)'; });
            inputs[sf.key] = sInp;
            sRow.appendChild(sLbl);
            sRow.appendChild(sInp);
            body.appendChild(sRow);
        });

        var saveBtn = document.createElement('button');
        saveBtn.className = 'liq-playground-apply';
        saveBtn.textContent = 'Save Header & Footer';
        saveBtn.style.cssText = 'width:100%;margin-top:4px;';
        saveBtn.addEventListener('click', function() {
            var payload = {};
            fieldDefs.forEach(function(f) {
                if (inputs[f.key]) payload[f.key] = inputs[f.key].value.trim();
            });
            var socialLinks = {};
            ['facebook', 'instagram', 'twitter', 'youtube', 'linkedin'].forEach(function(k) {
                var v = inputs[k] ? inputs[k].value.trim() : '';
                if (v) socialLinks[k] = v;
            });
            if (Object.keys(socialLinks).length) payload.social_links = socialLinks;
            if (!payload.name) { showToast('Business name is required'); return; }
            saveBtn.disabled = true;
            saveBtn.textContent = 'Saving...';
            fetch('/api/modules/company-profile/profile', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify(payload)
            })
            .then(function(r) { return r.json(); })
            .then(function(res) {
                if (res.ok) {
                    showToast('Saved ✓');
                    var nameEl = document.querySelector('.header-brand-name');
                    if (nameEl && payload.name) nameEl.textContent = payload.name;
                    var logoEl = document.querySelector('.header-brand-logo');
                    if (logoEl && payload.logo_url) {
                        logoEl.src = payload.logo_url;
                        logoEl.alt = (payload.name || '') + ' logo';
                    }
                    if (payload.phone) {
                        var dig = payload.phone.replace(/\D/g, '');
                        document.querySelectorAll('a[href^="tel:"]').forEach(function(a) {
                            a.href = 'tel:' + dig;
                            a.textContent = payload.phone;
                        });
                    }
                } else {
                    showToast(res.message || 'Save failed');
                }
            })
            .catch(function() { showToast('Network error'); })
            .finally(function() {
                saveBtn.disabled = false;
                saveBtn.textContent = 'Save Header & Footer';
            });
        });
        body.appendChild(saveBtn);

        hdr.addEventListener('click', function() {
            var collapsed = body.style.display === 'none';
            body.style.display = collapsed ? '' : 'none';
            arrow.textContent = collapsed ? '▼' : '▶';
            if (collapsed && !body._loaded) {
                body._loaded = true;
                fetch('/api/modules/company-profile/profile')
                    .then(function(r) { return r.json(); })
                    .then(function(res) {
                        if (res.ok && res.data) {
                            var p = res.data;
                            if (inputs.name)       inputs.name.value       = p.name        || '';
                            if (inputs.logo_url)   inputs.logo_url.value   = p.logo_url    || '';
                            if (inputs.favicon_url) inputs.favicon_url.value = p.favicon_url || '';
                            if (inputs.phone)      inputs.phone.value      = p.phone       || '';
                            if (inputs.email)      inputs.email.value      = p.email       || '';
                            if (inputs.address)    inputs.address.value    = p.address     || '';
                            var sl = p.social_links || {};
                            if (inputs.facebook)  inputs.facebook.value  = sl.facebook  || '';
                            if (inputs.instagram) inputs.instagram.value = sl.instagram || '';
                            if (inputs.twitter)   inputs.twitter.value   = sl.twitter   || '';
                            if (inputs.youtube)   inputs.youtube.value   = sl.youtube   || '';
                            if (inputs.linkedin)  inputs.linkedin.value  = sl.linkedin  || '';
                        }
                    })
                    .catch(function() {});
            }
        });

        panel.appendChild(sec);
    }


    // ── Init ─────────────────────────────────────────────────────────
    function init() {
        // Don't load on admin pages
        if (window.location.pathname.startsWith('/admin')) return;
        // Don't load if front-end editor is active
        // Drawer stays active during edit mode — it provides block palettes via tabs

        captureOriginals();
        Promise.allSettled([loadVariants(), loadActiveTokens(), loadActiveLayoutTheme(), loadActiveScopeStyles()]).then(function() {
            { // always run
                buildDrawer();

                // Watch for edit mode changes (pencil click, Edit Page, etc.)
                // Auto-rebuild block tabs when edit mode toggles
                var observer = new MutationObserver(function() {
                    var activeTab = document.querySelector('.liq-playground-tab-btn.is-active');
                    if (activeTab) {
                        var tabId = activeTab.dataset.tab;
                        var drawer = document.getElementById('liqPlaygroundDrawer');
                        if (drawer && tabId === 'blocks') {
                            var panel = drawer.querySelector('[data-panel="blocks"]');
                            if (panel) rebuildCombinedBlocksPanel(panel);
                        }
                    }
                });
                observer.observe(document.body, { childList: true, subtree: false });

                // Restore drawer state from sessionStorage. On mobile we
                // never auto-reopen because the bottom half-sheet covers
                // 50vh — too dominant when the admin is just navigating.
                // Tab selection is always restored.
                var savedState = sessionStorage.getItem('liq-playground-state');
                if (savedState) {
                    try {
                        var parsed = JSON.parse(savedState);
                        var isMobile = window.matchMedia && window.matchMedia('(max-width: 720px)').matches;
                        if (parsed.open && !isMobile) toggleDrawer();
                        if (parsed.activeTab) {
                            var tabs = document.querySelectorAll('.liq-playground-tab-btn');
                            for (var i = 0; i < tabs.length; i++) {
                                if (tabs[i].dataset.tab === parsed.activeTab) {
                                    tabs[i].click();
                                    break;
                                }
                            }
                        }
                    } catch(e) {}
                }

                // Migrate stale sessionStorage — clear any saved state older than v2
                (function(){
                    var raw = sessionStorage.getItem('liq_playground');
                    if (raw) {
                        try {
                            var d = JSON.parse(raw);
                            if (!d._v || d._v < 2) { sessionStorage.removeItem('liq_playground'); }
                        } catch(e) { sessionStorage.removeItem('liq_playground'); }
                    }
                })();

                // Restore preset/dropdown selections from session
                restorePlaygroundState();

                // Apply scope overrides AFTER restorePlaygroundState so they win over preset inline styles
                applyActiveScopeStyles();
                injectZoneCss();
                wireZoneClicks();

                // Auto-enter edit mode if we just saved a smart block
                if (sessionStorage.getItem('liq-auto-edit') === 'true') {
                    sessionStorage.removeItem('liq-auto-edit');
                    // Find and click the Edit Page button after a short delay
                    setTimeout(function() {
                        var btns = document.querySelectorAll('button');
                        for (var i = 0; i < btns.length; i++) {
                            var txt = btns[i].textContent.trim();
                            if (txt === 'Edit Page') {
                                btns[i].click();
                                break;
                            }
                        }
                    }, 500);
                }
            }
        });
    }

    // Wait for DOM ready
    if (document.readyState === 'loading') {
        document.addEventListener('DOMContentLoaded', init);
    } else {
        init();
    }
})();
"##;

/// CSS for the Design Playground drawer.
pub const PLAYGROUND_CSS: &str = r##"
/* ── Design Playground Tab ── */
.liq-playground-tab {
    position: fixed;
    right: 0;
    top: 50%;
    transform: translateY(-50%);
    background: #1e293b;
    color: #f8fafc;
    padding: 12px 8px;
    border-radius: 8px 0 0 8px;
    cursor: pointer;
    z-index: 9998;
    writing-mode: vertical-rl;
    text-orientation: mixed;
    font-size: 13px;
    font-weight: 600;
    display: flex;
    align-items: center;
    gap: 6px;
    box-shadow: -2px 0 12px rgba(0,0,0,0.15);
    transition: right 0.3s ease, background 0.2s;
    user-select: none;
}
.liq-playground-tab:hover {
    background: #334155;
}
.liq-playground-tab.is-open {
    right: 340px;
}
.liq-playground-tab-icon {
    font-size: 18px;
    writing-mode: horizontal-tb;
}
.liq-playground-tab-text {
    letter-spacing: 0.05em;
}

/* ── Drawer Panel ── */
.liq-playground-drawer {
    position: fixed;
    right: -340px;
    top: 0;
    width: 340px;
    height: 100vh;
    background: #ffffff;
    border-left: 1px solid #e2e8f0;
    box-shadow: -4px 0 24px rgba(0,0,0,0.1);
    z-index: 9999;
    overflow-y: auto;
    transition: right 0.3s ease;
    font-family: system-ui, -apple-system, sans-serif;
    padding: 0 0 24px 0;
}
.liq-playground-drawer.is-open {
    right: 0;
}

/* ── Drawer Header ── */
.liq-playground-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 16px 20px;
    border-bottom: 1px solid #e2e8f0;
    position: sticky;
    top: 0;
    background: #ffffff;
    z-index: 1;
}
.liq-playground-header h3 {
    margin: 0;
    font-size: 16px;
    font-weight: 700;
    color: #0f172a;
}
.liq-playground-close {
    background: none;
    border: none;
    font-size: 22px;
    color: #64748b;
    cursor: pointer;
    padding: 0 4px;
    line-height: 1;
}
.liq-playground-close:hover { color: #0f172a; }

/* ── Subtitle ── */
.liq-playground-subtitle {
    padding: 12px 20px 0;
    font-size: 13px;
    color: #64748b;
    margin: 0;
    line-height: 1.5;
}

/* ── Section ── */
.liq-playground-section {
    padding: 16px 20px 0;
}
.liq-playground-section h4 {
    font-size: 12px;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    color: #94a3b8;
    margin: 0 0 12px 0;
}

/* ── Preset Strip (horizontal scroll) ── */
.liq-playground-strip {
    display: flex;
    gap: 12px;
    overflow-x: auto;
    padding: 8px 0 12px;
    scroll-snap-type: x mandatory;
    -webkit-overflow-scrolling: touch;
    scrollbar-width: thin;
}
.liq-playground-strip::-webkit-scrollbar {
    height: 4px;
}
.liq-playground-strip::-webkit-scrollbar-thumb {
    background: #cbd5e1;
    border-radius: 2px;
}

/* ── Preset Chip ── */
.liq-playground-chip {
    flex-shrink: 0;
    width: 64px;
    text-align: center;
    cursor: pointer;
    scroll-snap-align: start;
    transition: transform 0.15s;
}
.liq-playground-chip:hover {
    transform: scale(1.08);
}
.liq-playground-chip.is-active {
    transform: scale(1.08);
}

.liq-playground-chip-circle {
    width: 48px;
    height: 48px;
    border-radius: 50%;
    margin: 0 auto 6px;
    display: flex;
    align-items: center;
    justify-content: center;
    border: 3px solid transparent;
    transition: border-color 0.2s;
    box-shadow: 0 2px 6px rgba(0,0,0,0.1);
}
.liq-playground-chip.is-active .liq-playground-chip-circle {
    border-color: #3b82f6;
    box-shadow: 0 0 0 3px rgba(59,130,246,0.2);
}

.liq-playground-chip-dot {
    width: 14px;
    height: 14px;
    border-radius: 50%;
    border: 2px solid rgba(255,255,255,0.7);
}

.liq-playground-chip-name {
    font-size: 10px;
    font-weight: 600;
    color: #475569;
    line-height: 1.2;
    white-space: normal;
    overflow: hidden;
    text-overflow: ellipsis;
    max-width: 64px;
    word-wrap: break-word;
    display: -webkit-box;
    -webkit-line-clamp: 2;
    -webkit-box-orient: vertical;
}
.liq-playground-chip.is-active .liq-playground-chip-name {
    color: #1e293b;
}

/* ── Preset Detail (shows when selected) ── */
.liq-playground-detail {
    font-size: 12px;
    color: #64748b;
    line-height: 1.4;
    padding: 8px 0;
    border-top: 1px solid #f1f5f9;
    display: none;
}

/* ── Customize Controls ── */
.liq-playground-control {
    margin-bottom: 12px;
}
.liq-playground-control-label {
    display: block;
    font-size: 12px;
    font-weight: 600;
    color: #475569;
    margin-bottom: 4px;
}
.liq-playground-select {
    width: 100%;
    padding: 8px 12px;
    border: 1px solid #e2e8f0;
    border-radius: 6px;
    font-size: 13px;
    color: #1e293b;
    background: #ffffff;
    cursor: pointer;
    appearance: auto;
}
.liq-playground-select:focus {
    outline: none;
    border-color: #3b82f6;
    box-shadow: 0 0 0 2px rgba(59,130,246,0.1);
}

/* ── Reset Button ── */
.liq-playground-reset {
    display: block;
    width: calc(100% - 40px);
    margin: 20px 20px 0;
    padding: 10px;
    background: #f1f5f9;
    border: 1px solid #e2e8f0;
    border-radius: 8px;
    color: #475569;
    font-size: 13px;
    font-weight: 600;
    cursor: pointer;
    transition: background 0.2s;
}
.liq-playground-reset:hover {
    background: #e2e8f0;
}

/* ── Apply Button ── */
.liq-playground-apply {
    display: block;
    width: calc(100% - 40px);
    margin: 12px 20px 0;
    padding: 12px;
    background: #2563eb;
    border: none;
    border-radius: 8px;
    color: #ffffff;
    font-size: 14px;
    font-weight: 700;
    cursor: pointer;
    transition: background 0.2s;
}
.liq-playground-apply:hover {
    background: #1d4ed8;
}

/* ── Undo Button ── */
.liq-playground-undo {
    display: block;
    width: calc(100% - 40px);
    margin: 8px 20px 0;
    padding: 8px;
    background: #fff7ed;
    border: 1px solid #fed7aa;
    border-radius: 8px;
    color: #9a3412;
    font-size: 13px;
    font-weight: 600;
    cursor: pointer;
    transition: background 0.2s;
}
.liq-playground-undo:hover {
    background: #ffedd5;
}

/* ── Export/Import Buttons ── */
.liq-playground-export,
.liq-playground-import {
    display: block;
    width: 100%;
    padding: 8px;
    background: #f8fafc;
    border: 1px solid #e2e8f0;
    border-radius: 6px;
    color: #475569;
    font-size: 12px;
    font-weight: 600;
    cursor: pointer;
    transition: background 0.2s;
    text-align: center;
}
.liq-playground-export:hover,
.liq-playground-import:hover {
    background: #e2e8f0;
}

/* ── Info Text ── */
.liq-playground-info {
    padding: 12px 20px;
    font-size: 11px;
    color: #94a3b8;
    line-height: 1.5;
    margin: 0;
}

/* ── Toast ── */
.liq-playground-toast {
    position: fixed;
    bottom: 24px;
    left: 50%;
    transform: translateX(-50%) translateY(20px);
    background: #0f172a;
    color: #f8fafc;
    padding: 10px 24px;
    border-radius: 8px;
    font-size: 14px;
    font-weight: 500;
    opacity: 0;
    transition: opacity 0.3s, transform 0.3s;
    z-index: 10000;
    pointer-events: none;
}
.liq-playground-toast.is-visible {
    opacity: 1;
    transform: translateX(-50%) translateY(0);
}

/* Drawer stays visible during edit mode — it provides the block palettes */

/* Hide the old editor palette when our drawer is present — drawer replaces it */
.liq-playground-drawer ~ .liq-palette,
body:has(.liq-playground-drawer) .liq-palette {
    display: none !important;
}
/* Hide the old "+ Blocks" button in the save bar — drawer has it now */
body:has(.liq-playground-drawer) .liq-save-bar button:first-child {
    /* Keep save bar but hide just the blocks button — tricky with no class */
}

/* ── Tab Bar ── */
.liq-playground-tabs {
    display: flex;
    gap: 4px;
    padding: 8px 16px;
    border-bottom: 1px solid #e2e8f0;
    background: #f8fafc;
}
.liq-playground-tab-btn {
    flex: 1;
    padding: 6px 4px;
    border: 1px solid #e2e8f0;
    border-radius: 6px;
    background: #ffffff;
    font-size: 11px;
    font-weight: 600;
    color: #64748b;
    cursor: pointer;
    transition: all 0.15s;
    text-align: center;
    white-space: nowrap;
}
.liq-playground-tab-btn:hover {
    background: #f1f5f9;
    color: #334155;
}
.liq-playground-tab-btn.is-active {
    background: #1e293b;
    color: #f8fafc;
    border-color: #1e293b;
}

/* ── Viewport preview bar ── */
.liq-viewport-bar {
    display: flex;
    gap: 4px;
    padding: 6px 16px;
    border-bottom: 1px solid #e2e8f0;
    background: #f1f5f9;
}
.liq-vp-btn {
    flex: 1;
    padding: 4px 4px;
    border: 1px solid #cbd5e1;
    border-radius: 5px;
    background: #ffffff;
    font-size: 10px;
    font-weight: 600;
    color: #64748b;
    cursor: pointer;
    transition: all 0.15s;
    text-align: center;
}
.liq-vp-btn:hover { background: #e2e8f0; }
.liq-vp-btn.is-active {
    background: #0f172a;
    color: #f8fafc;
    border-color: #0f172a;
}

/* ── Panel (tab content) ── */
.liq-playground-panel {
    /* No special styling needed — just a container */
}

/* ── Edit Prompt ── */
.liq-playground-edit-prompt {
    text-align: center;
    padding: 40px 20px;
    color: #64748b;
}
.liq-playground-prompt-icon {
    font-size: 32px;
    margin-bottom: 12px;
}
.liq-playground-edit-prompt p {
    font-size: 14px;
    line-height: 1.5;
    margin: 0 0 16px;
}

/* ── Block Items (smart blocks list) ── */
.liq-playground-block-item {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 10px 12px;
    margin: 4px 0;
    border: 1px solid #f1f5f9;
    border-radius: 8px;
    cursor: pointer;
    transition: all 0.15s;
}
.liq-playground-block-item:hover {
    border-color: #cbd5e1;
    background: #f8fafc;
}
.liq-playground-block-icon {
    font-size: 18px;
    width: 32px;
    height: 32px;
    display: flex;
    align-items: center;
    justify-content: center;
    background: #f1f5f9;
    border-radius: 6px;
    flex-shrink: 0;
}
.liq-playground-block-info {
    flex: 1;
    min-width: 0;
}
.liq-playground-block-label {
    font-size: 13px;
    font-weight: 600;
    color: #1e293b;
}
.liq-playground-block-desc {
    font-size: 11px;
    color: #94a3b8;
    margin-top: 1px;
}

/* ── Block Cards (basic blocks grid) ── */
.liq-playground-block-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 8px;
}
.liq-playground-block-card {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 4px;
    padding: 12px 8px;
    border: 1px solid #f1f5f9;
    border-radius: 8px;
    cursor: pointer;
    transition: all 0.15s;
}
.liq-playground-block-card:hover {
    border-color: #cbd5e1;
    background: #f8fafc;
}
.liq-playground-block-card-icon {
    font-size: 20px;
    color: #64748b;
}
.liq-playground-block-card-label {
    font-size: 11px;
    font-weight: 600;
    color: #475569;
}

/* ── Position Picker ── */
.liq-playground-picker {
    position: absolute;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    background: rgba(255,255,255,0.98);
    z-index: 10;
    overflow-y: auto;
    padding: 16px;
}
.liq-playground-picker h4 {
    font-size: 14px;
    font-weight: 700;
    color: #0f172a;
    margin: 0 0 12px;
}
.liq-playground-picker-item {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 10px 12px;
    border: 1px solid #e2e8f0;
    border-radius: 6px;
    margin-bottom: 6px;
    cursor: pointer;
    font-size: 13px;
    color: #334155;
    transition: all 0.15s;
}
.liq-playground-picker-item:hover {
    background: #f1f5f9;
    border-color: #3b82f6;
    color: #1e293b;
}
.liq-playground-picker-icon {
    font-size: 16px;
    color: #64748b;
}
.liq-playground-picker-cancel {
    display: block;
    width: 100%;
    margin-top: 12px;
    padding: 8px;
    background: #f1f5f9;
    border: 1px solid #e2e8f0;
    border-radius: 6px;
    color: #64748b;
    font-size: 13px;
    cursor: pointer;
    text-align: center;
}
.liq-playground-picker-cancel:hover {
    background: #e2e8f0;
}

/* ── Mobile: bottom half-sheet drawer ──────────────────────────── */
@media (max-width: 480px) {
    .liq-playground-tab {
        writing-mode: horizontal-tb;
        text-orientation: initial;
        top: auto;
        bottom: 72px;
        right: 12px;
        transform: none;
        border-radius: 12px;
        padding: 10px 14px;
        font-size: 12px;
        box-shadow: 0 2px 12px rgba(0,0,0,0.2);
    }
    .liq-playground-tab.is-open {
        right: 12px;
        opacity: 0;
        pointer-events: none;
    }
    .liq-playground-drawer {
        width: 100vw;
        height: 50vh;
        right: 0;
        top: auto;
        bottom: 0;
        transform: translateY(100%);
        transition: transform 0.3s ease;
        border-left: none;
        border-top: 1px solid #e2e8f0;
        border-radius: 16px 16px 0 0;
        box-shadow: 0 -4px 24px rgba(0,0,0,0.12);
        overflow: hidden;
    }
    .liq-playground-drawer.is-open {
        right: 0;
        transform: translateY(0);
    }
    .liq-playground-drawer.is-open ~ footer,
    .liq-playground-drawer.is-open ~ [role='contentinfo'] {
        padding-bottom: 52vh;
    }
    .liq-playground-drawer.is-pinned-top {
        bottom: auto;
        top: 0;
        border-radius: 0 0 16px 16px;
        border-top: none;
        border-bottom: 1px solid #e2e8f0;
        box-shadow: 0 4px 24px rgba(0,0,0,0.12);
        transform: translateY(-100%);
    }
    .liq-playground-drawer.is-pinned-top.is-open {
        transform: translateY(0);
    }
    .liq-playground-drag-handle {
        display: flex;
        align-items: center;
        justify-content: center;
        padding: 8px 0 4px;
        cursor: grab;
        touch-action: none;
    }
    .liq-playground-drag-handle-bar {
        width: 40px;
        height: 4px;
        background: #cbd5e1;
        border-radius: 2px;
    }
    .liq-playground-pin-btn {
        display: flex !important;
    }
    .liq-playground-header {
        padding: 8px 16px;
    }
    .liq-playground-header h3 {
        font-size: 14px;
    }
    .liq-playground-subtitle {
        display: none;
    }
    .liq-playground-panel {
        max-height: calc(50vh - 100px);
        overflow-y: auto;
        -webkit-overflow-scrolling: touch;
    }
    .liq-playground-drawer.liq-pg-h30 { height: 30vh; }
    .liq-playground-drawer.liq-pg-h30 .liq-playground-panel { max-height: calc(30vh - 60px); overflow-y: auto; }
    .liq-playground-drawer.liq-pg-h50 { height: 50vh; }
    .liq-playground-drawer.liq-pg-h50 .liq-playground-panel { max-height: calc(50vh - 60px); overflow-y: auto; }
    .liq-playground-drawer.liq-pg-h75 { height: 75vh; }
    .liq-playground-drawer.liq-pg-h75 .liq-playground-panel { max-height: calc(75vh - 60px); overflow-y: auto; }
    .liq-playground-panel > *:last-child { padding-bottom: 40px; }
}
/* Hidden on desktop */
.liq-playground-drag-handle { display: none; }
.liq-playground-pin-btn { display: none; }

/* ── Layout Theme Cards ── */
.liq-layout-theme-card {
    background: rgba(255,255,255,0.06);
    border: 2px solid transparent;
    border-radius: 8px;
    cursor: pointer;
    transition: border-color 0.15s, transform 0.15s;
    overflow: hidden;
}
.liq-layout-theme-card:hover { border-color: rgba(167,139,250,0.5); transform: translateY(-2px); }
.liq-layout-theme-card.is-active { border-color: #a78bfa; }

/* ── Colors panel scroll override (inherit drawer scroll) ── */
@media (max-width: 480px) {
    .liq-playground-panel[data-panel="colors"] > div:nth-child(2) {
        max-height: calc(50vh - 140px);
    }
}
"##;

/// Axum handler: serves playground.js with proper content type and caching.
pub async fn serve_playground_js() -> (
    axum::http::StatusCode,
    [(axum::http::header::HeaderName, &'static str); 2],
    &'static str,
) {
    (
        axum::http::StatusCode::OK,
        [
            (
                axum::http::header::CONTENT_TYPE,
                "application/javascript; charset=utf-8",
            ),
            (axum::http::header::CACHE_CONTROL, "public, max-age=3600"),
        ],
        PLAYGROUND_JS,
    )
}

/// Axum handler: serves playground.css with proper content type and caching.
pub async fn serve_playground_css() -> (
    axum::http::StatusCode,
    [(axum::http::header::HeaderName, &'static str); 2],
    &'static str,
) {
    (
        axum::http::StatusCode::OK,
        [
            (axum::http::header::CONTENT_TYPE, "text/css; charset=utf-8"),
            (axum::http::header::CACHE_CONTROL, "public, max-age=3600"),
        ],
        PLAYGROUND_CSS,
    )
}
