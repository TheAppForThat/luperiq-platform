//! Unified Design Studio shell — coordinates profile dropdown,
//! sub-tab bar, undo/redo, floating panels, and dirty state tracking.
//!
//! The studio view (`ts-studio`) loads profile data once and passes it
//! to sub-tab modules (tokens, header, footer, sidebar, pages, presets).
//!
//! Security: All JS uses DOM methods only (createElement, textContent,
//! replaceChildren). No innerHTML, outerHTML, or insertAdjacentHTML.

/// Return JS for the TsStudio coordinator object and load_ts_studio view.
pub fn studio_js() -> &'static str {
    STUDIO_JS
}

const STUDIO_JS: &str = r####"
/* ── TsStudio: Unified Design Studio coordinator ──────────────────── */

window.TsStudio = {
    // ── State ──
    _profile: null,
    _slug: null,
    _dirty: false,
    _undoStack: [],
    _redoStack: [],
    _savedState: null,
    _activeTab: 'tokens',
    _floatingPanels: {},

    // ── Profile Management ──
    loadProfile: async function(slug) {
        var res = await tsApi('/profiles/' + encodeURIComponent(slug));
        if (!res.ok) { if (typeof showToast === 'function') showToast('Failed to load profile', 'error'); return; }
        TsStudio._profile = res.data.profile;
        TsStudio._slug = slug;
        TsStudio._savedState = JSON.parse(JSON.stringify(res.data.profile));
        TsStudio._undoStack = [];
        TsStudio._redoStack = [];
        TsStudio.markClean();
    },

    saveProfile: async function() {
        if (!TsStudio._slug || !TsStudio._profile) return;
        var res = await tsApi('/profiles/' + encodeURIComponent(TsStudio._slug), {
            method: 'PUT',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify(TsStudio._profile)
        });
        if (res.ok) {
            TsStudio._savedState = JSON.parse(JSON.stringify(TsStudio._profile));
            TsStudio.markClean();
            if (typeof showToast === 'function') showToast('Profile saved', 'success');
            /* Refresh preview to pick up server-side changes */
            var iframe = TsStudio.getPreviewIframe();
            if (iframe) { iframe.contentWindow.location.reload(); }
        } else {
            if (typeof showToast === 'function') showToast('Save failed', 'error');
        }
    },

    saveAs: async function() {
        var newSlug = prompt('New profile slug (url-safe):');
        if (!newSlug) return;
        var newLabel = prompt('Profile display name:');
        if (!newLabel) return;
        var res = await tsApi('/profiles/' + encodeURIComponent(TsStudio._slug) + '/duplicate', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ new_slug: newSlug, label: newLabel })
        });
        if (res.ok) {
            TsStudio._slug = newSlug;
            await TsStudio.saveProfile();
            await TsStudio.refreshProfileDropdown();
            if (typeof showToast === 'function') showToast('Saved as "' + newLabel + '"', 'success');
        } else {
            if (typeof showToast === 'function') showToast('Save As failed: ' + (res.error || 'unknown error'), 'error');
        }
    },

    refreshProfileDropdown: async function() {
        var sel = document.getElementById('tsProfileSelect');
        if (!sel) return;
        var res = await tsApi('/profiles');
        if (!res.ok) return;
        var profiles = Array.isArray(res.data) ? res.data : (res.data.profiles || []);
        sel.replaceChildren();
        profiles.forEach(function(p) {
            var opt = document.createElement('option');
            opt.value = p.slug || p.label;
            opt.textContent = p.label || p.slug;
            if (opt.value === TsStudio._slug) opt.selected = true;
            sel.appendChild(opt);
        });
    },

    // ── Undo/Redo ──
    pushSnapshot: function(label) {
        var snap = JSON.parse(JSON.stringify(TsStudio._profile));
        snap._snapshotLabel = label || 'change';
        TsStudio._undoStack.push(snap);
        if (TsStudio._undoStack.length > 50) TsStudio._undoStack.shift();
        TsStudio._redoStack = [];
    },

    undo: function() {
        if (!TsStudio._undoStack.length) return;
        var current = JSON.parse(JSON.stringify(TsStudio._profile));
        TsStudio._redoStack.push(current);
        TsStudio._profile = TsStudio._undoStack.pop();
        TsStudio.switchTab(TsStudio._activeTab);
        TsStudio.markDirty();
        if (typeof showToast === 'function') showToast('Undo: ' + (TsStudio._profile._snapshotLabel || ''), 'info');
    },

    redo: function() {
        if (!TsStudio._redoStack.length) return;
        var current = JSON.parse(JSON.stringify(TsStudio._profile));
        TsStudio._undoStack.push(current);
        TsStudio._profile = TsStudio._redoStack.pop();
        TsStudio.switchTab(TsStudio._activeTab);
        TsStudio.markDirty();
        if (typeof showToast === 'function') showToast('Redo', 'info');
    },

    cancelChanges: function() {
        if (!TsStudio._dirty) return;
        if (!confirm('Discard all unsaved changes?')) return;
        TsStudio._profile = JSON.parse(JSON.stringify(TsStudio._savedState));
        TsStudio._undoStack = [];
        TsStudio._redoStack = [];
        TsStudio.markClean();
        TsStudio.switchTab(TsStudio._activeTab);
        if (typeof showToast === 'function') showToast('Changes discarded', 'info');
    },

    // ── Dirty tracking ──
    markDirty: function() {
        TsStudio._dirty = true;
        var dot = document.getElementById('tsDirtyDot');
        if (dot) dot.style.display = '';
        var saveBtn = document.getElementById('tsStudioSave');
        if (saveBtn) saveBtn.disabled = false;
    },

    markClean: function() {
        TsStudio._dirty = false;
        var dot = document.getElementById('tsDirtyDot');
        if (dot) dot.style.display = 'none';
    },

    // ── Sub-tab routing ──
    switchTab: async function(tabName) {
        if (typeof TsStudio.teardownAb === 'function') TsStudio.teardownAb();
        TsStudio._activeTab = tabName;
        document.querySelectorAll('.ts-studio-tab').forEach(function(b) {
            b.classList.toggle('is-active', b.dataset.tab === tabName);
        });
        var controls = document.getElementById('tsStudioControls');
        if (controls) controls.replaceChildren();

        /* Toggle preview vs builder pane for builder tabs */
        var isBuilderTab = (tabName === 'header' || tabName === 'footer');
        var pvToolbar = document.getElementById('tsStudioPreviewToolbar');
        var pvFrame = document.getElementById('tsStudioPreviewFrame');
        var pvStatus = document.getElementById('tsStudioPreviewStatus');
        var builderArea = document.getElementById('tsStudioBuilder');
        if (pvToolbar) pvToolbar.style.display = isBuilderTab ? 'none' : '';
        if (pvFrame) pvFrame.style.display = isBuilderTab ? 'none' : '';
        if (pvStatus) pvStatus.style.display = isBuilderTab ? 'none' : '';
        if (builderArea) {
            builderArea.style.display = isBuilderTab ? '' : 'none';
            /* Only remove dynamic builder wraps; preserve the persistent preview section */
            builderArea.querySelectorAll('.ts-builder-wrap').forEach(function(w) { w.remove(); });
        }

        var loaders = {
            tokens:  typeof TsStudio.loadTokens  === 'function' ? TsStudio.loadTokens  : null,
            header:  typeof TsStudio.loadHeader  === 'function' ? TsStudio.loadHeader  : null,
            footer:  typeof TsStudio.loadFooter  === 'function' ? TsStudio.loadFooter  : null,
            sidebar: typeof TsStudio.loadSidebar === 'function' ? TsStudio.loadSidebar : null,
            pages:   typeof TsStudio.loadPages === 'function' ? TsStudio.loadPages : null,
            presets: typeof TsStudio.loadPresets === 'function' ? TsStudio.loadPresets : null
        };
        var loader = loaders[tabName];
        if (loader) {
            await loader();
        } else {
            if (controls) {
                var ph = document.createElement('div');
                ph.className = 'ts-empty';
                ph.textContent = tabName.charAt(0).toUpperCase() + tabName.slice(1) + ' \u2014 coming soon';
                controls.appendChild(ph);
            }
        }

        /* Unregister AI context if tab has no AI context */
        if (!loader && window.LiqAI && window.LiqAI.panel) {
            window.LiqAI.panel.unregister();
        }
    },

    // ── Floating panels ──
    toggleFloat: function(sectionId) {
        var el = document.getElementById(sectionId);
        if (!el) return;
        if (TsStudio._floatingPanels[sectionId]) {
            TsStudio.dockPanel(sectionId);
            return;
        }
        var rect = el.getBoundingClientRect();
        var float = document.createElement('div');
        float.className = 'ts-floating-panel';
        float.style.left = rect.left + 'px';
        float.style.top = rect.top + 'px';
        float.style.width = Math.max(rect.width, 280) + 'px';

        var handle = document.createElement('div');
        handle.className = 'ts-float-handle';
        var title = el.querySelector('h2,h3,.ts-card-title');
        handle.textContent = title ? title.textContent : 'Panel';
        var closeBtn = document.createElement('button');
        closeBtn.textContent = '\u00d7';
        closeBtn.className = 'ts-float-close';
        closeBtn.onclick = function() { TsStudio.dockPanel(sectionId); };
        handle.appendChild(closeBtn);
        float.appendChild(handle);

        var content = document.createElement('div');
        content.className = 'ts-float-content';
        while (el.firstChild) content.appendChild(el.firstChild);
        float.appendChild(content);
        el.style.display = 'none';
        document.body.appendChild(float);

        // Draggable
        var dragging = false, startX = 0, startY = 0, origX = 0, origY = 0;
        handle.addEventListener('mousedown', function(e) {
            dragging = true;
            startX = e.clientX; startY = e.clientY;
            origX = parseFloat(float.style.left); origY = parseFloat(float.style.top);
            e.preventDefault();
        });
        document.addEventListener('mousemove', function(e) {
            if (!dragging) return;
            float.style.left = (origX + e.clientX - startX) + 'px';
            float.style.top = (origY + e.clientY - startY) + 'px';
        });
        document.addEventListener('mouseup', function() { dragging = false; });

        TsStudio._floatingPanels[sectionId] = { float: float, original: el, content: content };
    },

    dockPanel: function(sectionId) {
        var info = TsStudio._floatingPanels[sectionId];
        if (!info) return;
        while (info.content.firstChild) info.original.appendChild(info.content.firstChild);
        info.original.style.display = '';
        info.float.remove();
        delete TsStudio._floatingPanels[sectionId];
    },

    // ── Preview ──
    getPreviewIframe: function() {
        return document.getElementById('tsStudioPreview');
    },

    navigatePreview: function(url) {
        var iframe = TsStudio.getPreviewIframe();
        if (iframe) iframe.src = url;
    },

    _deviceCls: '',
    _previewTimer: null,
    _previewLocation: null,

    refreshBuilderPreview: function(location, rows) {
        TsStudio._previewLocation = location;
        clearTimeout(TsStudio._previewTimer);
        TsStudio._previewTimer = setTimeout(function() {
            TsStudio._doPreviewFetch(location, rows);
        }, 200);
    },

    _doPreviewFetch: function(location, rows) {
        var host = document.getElementById('tsBuilderPreviewHost');
        if (!host || !host.shadowRoot) return;
        var container = host.shadowRoot.querySelector('#tsBuilderPreviewContent');
        if (!container) return;
        container.textContent = 'Updating preview\u2026';
        fetch('/api/modules/theme-studio/render/' + location + '-preview', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ rows: rows })
        }).then(function(r) { return r.text(); })
        .then(function(html) {
            var doc = new DOMParser().parseFromString(html, 'text/html');
            container.replaceChildren();
            while (doc.body.firstChild) {
                container.appendChild(doc.body.firstChild);
            }
        });
    },

    setDeviceMode: function(cls) {
        var frame = document.getElementById('tsStudioPreviewFrame');
        if (!frame) return;
        TsStudio._deviceCls = cls;
        var iframe = frame.querySelector('iframe');
        if (!iframe) return;
        /* Reset inline styles so container measures correctly */
        iframe.style.width = '';
        iframe.style.height = '';
        iframe.style.transform = '';
        iframe.style.transformOrigin = '';
        frame.className = 'ts-preview-frame' + (cls ? ' ' + cls : '');
        /* Desktop mode: render iframe at 1280px wide then scale to fit container */
        if (!cls) {
            var containerW = frame.clientWidth;
            var desktopW = 1280;
            if (containerW < desktopW) {
                var scale = containerW / desktopW;
                iframe.style.width = desktopW + 'px';
                iframe.style.height = Math.round(frame.clientHeight / scale) + 'px';
                iframe.style.transform = 'scale(' + scale + ')';
                iframe.style.transformOrigin = 'top left';
            }
        }
    }
};

/* ── Keyboard shortcuts ───────────────────────────────────────────── */

document.addEventListener('keydown', function(e) {
    // Only active when studio is loaded
    if (!TsStudio._profile) return;
    // Do not capture when typing in inputs (except Ctrl+S)
    if (e.target.tagName === 'INPUT' || e.target.tagName === 'TEXTAREA' || e.target.tagName === 'SELECT') {
        if (e.ctrlKey && e.key === 's') { e.preventDefault(); TsStudio.saveProfile(); }
        return;
    }
    if (e.ctrlKey && e.key === 'z' && !e.shiftKey) { e.preventDefault(); TsStudio.undo(); }
    if (e.ctrlKey && e.key === 'z' && e.shiftKey)  { e.preventDefault(); TsStudio.redo(); }
    if (e.ctrlKey && e.key === 's')                 { e.preventDefault(); TsStudio.saveProfile(); }
});

/* ── load_ts_studio view ──────────────────────────────────────────── */

async function load_ts_studio() {
    var main = document.getElementById('adminMain');
    var wrap = document.createElement('div');
    wrap.className = 'ts-studio-wrap';

    // ── Profile bar ──
    var profileBar = document.createElement('div');
    profileBar.className = 'ts-studio-profile-bar';

    var profileLabel = document.createElement('span');
    profileLabel.textContent = 'Profile:';
    profileLabel.style.cssText = 'font-size:13px;color:var(--text-muted);';
    profileBar.appendChild(profileLabel);

    var profileSelect = document.createElement('select');
    profileSelect.id = 'tsProfileSelect';
    profileSelect.style.cssText = 'padding:4px 8px;border:1px solid var(--border);border-radius:4px;background:var(--bg);color:var(--text);font-size:13px;min-width:160px;';
    profileBar.appendChild(profileSelect);

    var dirtyDot = document.createElement('span');
    dirtyDot.id = 'tsDirtyDot';
    dirtyDot.textContent = '\u2022 unsaved';
    dirtyDot.style.cssText = 'color:var(--accent);font-size:12px;font-weight:600;display:none;';
    profileBar.appendChild(dirtyDot);

    var saveAsBtn = document.createElement('button');
    saveAsBtn.textContent = 'Save As';
    saveAsBtn.className = 'ts-studio-btn';
    saveAsBtn.onclick = function() { TsStudio.saveAs(); };
    profileBar.appendChild(saveAsBtn);

    var abBtn = document.createElement('button');
    abBtn.textContent = 'A/B Test';
    abBtn.className = 'ts-studio-btn';
    abBtn.onclick = function() { TsStudio.startAbTest(); };
    profileBar.appendChild(abBtn);

    var abListBtn = document.createElement('button');
    abListBtn.textContent = 'Experiments';
    abListBtn.className = 'ts-studio-btn';
    abListBtn.onclick = function() { TsStudio.showAbExperiments(); };
    profileBar.appendChild(abListBtn);

    wrap.appendChild(profileBar);

    // ── Sub-tab bar ──
    var tabBar = document.createElement('div');
    tabBar.className = 'ts-studio-tabs';
    var tabs = ['tokens', 'header', 'footer', 'sidebar', 'pages', 'presets'];
    var tabLabels = { tokens: 'Tokens', header: 'Header', footer: 'Footer', sidebar: 'Sidebar', pages: 'Pages', presets: 'Presets' };
    tabs.forEach(function(t) {
        var btn = document.createElement('button');
        btn.className = 'ts-studio-tab' + (t === 'tokens' ? ' is-active' : '');
        btn.dataset.tab = t;
        btn.textContent = tabLabels[t];
        btn.onclick = function() { TsStudio.switchTab(t); };
        tabBar.appendChild(btn);
    });
    wrap.appendChild(tabBar);

    // ── Split pane: controls + preview ──
    var splitPane = document.createElement('div');
    splitPane.className = 'ts-split';

    var controlsPane = document.createElement('div');
    controlsPane.className = 'ts-split-controls';
    controlsPane.id = 'tsStudioControls';
    splitPane.appendChild(controlsPane);

    var previewPane = document.createElement('div');
    previewPane.className = 'ts-split-preview';

    /* Preview toolbar — device selector + page picker + refresh */
    var pvToolbar = document.createElement('div');
    pvToolbar.className = 'ts-preview-toolbar';
    pvToolbar.id = 'tsStudioPreviewToolbar';

    var pvUrlSelect = document.createElement('select');
    [
        { label: 'Homepage', value: '/' },
        { label: 'Modules', value: '/modules/' },
        { label: 'AI Workflows', value: '/ai-workflows/' },
        { label: 'Pricing', value: '/luperiq-pricing-plans/' },
        { label: 'Blog', value: '/blog/' },
        { label: 'Roadmap', value: '/roadmap/' }
    ].forEach(function(opt) {
        var o = document.createElement('option');
        o.value = opt.value;
        o.textContent = opt.label;
        pvUrlSelect.appendChild(o);
    });
    pvToolbar.appendChild(pvUrlSelect);

    var devices = [
        { label: 'Desktop', cls: '' },
        { label: 'Tablet', cls: 'device-tablet' },
        { label: 'Mobile', cls: 'device-mobile' }
    ];
    var deviceBtns = [];
    devices.forEach(function(d, i) {
        var btn = document.createElement('button');
        btn.textContent = d.label;
        if (i === 0) btn.classList.add('is-active');
        btn.addEventListener('click', function() {
            deviceBtns.forEach(function(b) { b.classList.remove('is-active'); });
            btn.classList.add('is-active');
            TsStudio.setDeviceMode(d.cls);
        });
        deviceBtns.push(btn);
        pvToolbar.appendChild(btn);
    });

    var pvRefresh = document.createElement('button');
    pvRefresh.textContent = 'Refresh';
    pvRefresh.addEventListener('click', function() {
        var iframe = TsStudio.getPreviewIframe();
        if (iframe) iframe.src = iframe.src;
    });
    pvToolbar.appendChild(pvRefresh);
    previewPane.appendChild(pvToolbar);

    /* Preview frame + iframe */
    var pvFrame = document.createElement('div');
    pvFrame.className = 'ts-preview-frame';
    pvFrame.id = 'tsStudioPreviewFrame';

    var pvIframe = document.createElement('iframe');
    pvIframe.id = 'tsStudioPreview';
    pvIframe.src = '/';
    pvIframe.title = 'Live Preview';
    pvFrame.appendChild(pvIframe);
    previewPane.appendChild(pvFrame);

    /* Preview status line */
    var pvStatus = document.createElement('div');
    pvStatus.className = 'ts-preview-status';
    pvStatus.id = 'tsStudioPreviewStatus';
    pvStatus.textContent = 'Loading preview\u2026';
    previewPane.appendChild(pvStatus);

    /* Builder area — contains live preview + canvas for header/footer tabs */
    var builderArea = document.createElement('div');
    builderArea.id = 'tsStudioBuilder';
    builderArea.style.display = 'none';

    /* Live preview section (Shadow DOM for CSS isolation) */
    var previewSection = document.createElement('div');
    previewSection.style.cssText = 'margin-bottom:16px;';

    var previewLabel = document.createElement('div');
    previewLabel.textContent = 'Live Preview';
    previewLabel.style.cssText = 'font-size:12px;font-weight:600;color:var(--text-muted);margin-bottom:6px;text-transform:uppercase;letter-spacing:0.5px;';
    previewSection.appendChild(previewLabel);

    var previewHost = document.createElement('div');
    previewHost.id = 'tsBuilderPreviewHost';
    previewHost.style.cssText = 'border:1px solid var(--border);border-radius:8px;overflow:hidden;background:#fff;min-height:80px;';
    var shadow = previewHost.attachShadow({ mode: 'open' });

    var themeLink = document.createElement('link');
    themeLink.rel = 'stylesheet';
    themeLink.href = '/api/modules/theme-studio/render/css';
    shadow.appendChild(themeLink);

    var siteLink = document.createElement('link');
    siteLink.rel = 'stylesheet';
    siteLink.href = '/static/css/theme-studio.css';
    shadow.appendChild(siteLink);

    var previewContent = document.createElement('div');
    previewContent.id = 'tsBuilderPreviewContent';
    previewContent.style.cssText = 'padding:0;color:#64748b;font-size:13px;';
    previewContent.textContent = 'Preview will appear when you edit the layout.';
    shadow.appendChild(previewContent);

    previewSection.appendChild(previewHost);
    builderArea.appendChild(previewSection);

    previewPane.appendChild(builderArea);

    pvIframe.addEventListener('load', function() {
        pvStatus.textContent = 'Preview loaded \u2014 changes apply live';
        /* Re-inject tokens into the fresh iframe if we have a tokens injector */
        if (typeof TsStudio._reinjectPreview === 'function') {
            TsStudio._reinjectPreview();
        }
    });
    pvUrlSelect.addEventListener('change', function() {
        pvIframe.src = pvUrlSelect.value;
        pvStatus.textContent = 'Loading preview\u2026';
    });

    splitPane.appendChild(previewPane);
    wrap.appendChild(splitPane);

    // ── Bottom toolbar ──
    var toolbar = document.createElement('div');
    toolbar.className = 'ts-studio-toolbar';

    var undoBtn = document.createElement('button');
    undoBtn.textContent = 'Undo';
    undoBtn.title = 'Ctrl+Z';
    undoBtn.className = 'ts-studio-btn';
    undoBtn.onclick = function() { TsStudio.undo(); };
    toolbar.appendChild(undoBtn);

    var redoBtn = document.createElement('button');
    redoBtn.textContent = 'Redo';
    redoBtn.title = 'Ctrl+Shift+Z';
    redoBtn.className = 'ts-studio-btn';
    redoBtn.onclick = function() { TsStudio.redo(); };
    toolbar.appendChild(redoBtn);

    var sep = document.createElement('span');
    sep.className = 'ts-studio-sep';
    toolbar.appendChild(sep);

    var histBtn = document.createElement('button');
    histBtn.textContent = 'History';
    histBtn.className = 'ts-studio-btn';
    histBtn.onclick = function() { TsStudio.showHistory(); };
    toolbar.appendChild(histBtn);

    var saveBtn = document.createElement('button');
    saveBtn.id = 'tsStudioSave';
    saveBtn.textContent = 'Save';
    saveBtn.title = 'Ctrl+S';
    saveBtn.className = 'ts-studio-btn ts-studio-btn-primary';
    saveBtn.onclick = function() { TsStudio.saveProfile(); };
    toolbar.appendChild(saveBtn);

    var cancelBtn = document.createElement('button');
    cancelBtn.textContent = 'Cancel';
    cancelBtn.className = 'ts-studio-btn';
    cancelBtn.onclick = function() { TsStudio.cancelChanges(); };
    toolbar.appendChild(cancelBtn);

    wrap.appendChild(toolbar);

    main.replaceChildren(wrap);

    /* Apply desktop scaling after layout settles + on resize */
    requestAnimationFrame(function() { TsStudio.setDeviceMode(''); });
    window.addEventListener('resize', function() {
        TsStudio.setDeviceMode(TsStudio._deviceCls);
    });

    // ── Load profiles into dropdown ──
    var cfgRes = await tsApi('/config');
    var activeSlug = (cfgRes.data || {}).active_profile || '';

    await TsStudio.refreshProfileDropdown();
    if (activeSlug) {
        profileSelect.value = activeSlug;
        await TsStudio.loadProfile(activeSlug);
    }

    profileSelect.addEventListener('change', async function() {
        if (TsStudio._dirty) {
            if (!confirm('You have unsaved changes. Switch profile anyway?')) {
                profileSelect.value = TsStudio._slug;
                return;
            }
        }
        await TsStudio.loadProfile(profileSelect.value);
        TsStudio.switchTab(TsStudio._activeTab);
    });

    // ── Load initial tab ──
    if (TsStudio._profile) {
        var pending = TsStudio._pendingTab || 'tokens';
        TsStudio._pendingTab = null;
        TsStudio.switchTab(pending);
    }
}
"####;
