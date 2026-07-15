//! Shared JS utility functions for all Theme Studio admin sub-tabs.
//!
//! These helpers (tsApi, tsBtn, tsCard, tsEmpty, tsViewAiSection, etc.)
//! are used by every Theme Studio view.  Extracted from the monolithic
//! `admin.rs` so each sub-tab file can depend on them without
//! duplicating code.
//!
//! Security: All JS uses DOM methods only (createElement, textContent,
//! replaceChildren). No innerHTML, outerHTML, or insertAdjacentHTML.

/// Return the shared JS utility functions used across all Theme Studio
/// admin views.  Called once by `admin::admin_js()` and emitted before
/// any view-specific code.
pub fn shared_js() -> &'static str {
    r##"
/* ── Theme Studio: helper utilities ───────────────────────────────── */

function tsApi(path, opts) {
    return fetch('/api/modules/theme-studio' + path, opts).then(function(r) { return r.json(); });
}

function tsBtn(label, onclick) {
    var b = document.createElement('button');
    b.textContent = label;
    b.style.cssText = 'padding:6px 14px;border-radius:6px;border:1px solid var(--border);background:var(--accent);color:white;cursor:pointer;font-size:13px;';
    b.addEventListener('click', onclick);
    return b;
}

function tsBtnGhost(label, onclick) {
    var b = document.createElement('button');
    b.textContent = label;
    b.style.cssText = 'padding:6px 14px;border-radius:6px;border:1px solid var(--border);background:transparent;color:var(--text);cursor:pointer;font-size:13px;';
    b.addEventListener('click', onclick);
    return b;
}

function tsBtnDanger(label, onclick) {
    var b = document.createElement('button');
    b.textContent = label;
    b.style.cssText = 'padding:6px 14px;border-radius:6px;border:1px solid var(--border);background:var(--danger,#ef4444);color:white;cursor:pointer;font-size:13px;';
    b.addEventListener('click', onclick);
    return b;
}

function tsInput(type, value, attrs) {
    var inp = document.createElement('input');
    inp.type = type;
    if (value !== undefined && value !== null) inp.value = value;
    if (attrs) Object.assign(inp, attrs);
    inp.style.cssText = 'padding:4px 8px;border:1px solid var(--border);border-radius:4px;background:var(--bg);color:var(--text);';
    return inp;
}

function tsLabel(text) {
    var l = document.createElement('label');
    l.textContent = text;
    l.style.cssText = 'min-width:120px;font-size:13px;color:var(--text-muted);';
    return l;
}

function tsH2(text) {
    var h = document.createElement('h2');
    h.textContent = text;
    h.style.marginBottom = '16px';
    return h;
}

function tsSelect(options, selected) {
    var sel = document.createElement('select');
    sel.style.cssText = 'padding:4px 8px;border:1px solid var(--border);border-radius:4px;background:var(--bg);color:var(--text);';
    options.forEach(function(o) {
        var opt = document.createElement('option');
        opt.value = o;
        opt.textContent = o;
        if (o === selected) opt.selected = true;
        sel.appendChild(opt);
    });
    return sel;
}

function tsTokenRow(labelText, input) {
    var row = document.createElement('div');
    row.className = 'ts-token-row';
    row.appendChild(tsLabel(labelText));
    row.appendChild(input);
    return row;
}

function tsCard() {
    var c = document.createElement('div');
    c.className = 'ts-card';
    return c;
}

function tsEmpty(msg) {
    var d = document.createElement('div');
    d.className = 'ts-empty';
    d.textContent = msg;
    return d;
}

function tsStatusIsArchived(status) {
    return String(status || '').toLowerCase() === 'archived';
}

/* ── Shared: Upgrade Banner (standalone for non-palette views) ────── */
function tsViewUpgradeBanner() {
    if (_tsIsPro) return null;
    var banner = document.createElement('div');
    banner.className = 'ts-view-upgrade';
    var txt = document.createElement('div');
    txt.className = 'ts-view-upgrade-text';
    var h = document.createElement('h4');
    h.textContent = '\u2728 Unlock AI & Premium Features';
    txt.appendChild(h);
    var p = document.createElement('p');
    p.textContent = 'Get AI content generation, premium presets, and advanced customization with a Professional or Enterprise plan.';
    txt.appendChild(p);
    banner.appendChild(txt);
    var btn = document.createElement('button');
    btn.className = 'ts-upgrade-btn';
    btn.textContent = 'Upgrade';
    btn.onclick = function() { if (typeof navigateTo === 'function') navigateTo('store'); };
    banner.appendChild(btn);
    return banner;
}

/* ── Shared: AI Section ─────────────────────────────────────────── */
function tsDownloadJson(filename, data, successMessage) {
    var blob = new Blob([JSON.stringify(data, null, 2)], { type: 'application/json' });
    var a = document.createElement('a');
    a.href = URL.createObjectURL(blob);
    a.download = filename || 'export.json';
    document.body.appendChild(a);
    a.click();
    document.body.removeChild(a);
    URL.revokeObjectURL(a.href);
    if (successMessage && typeof showToast === 'function') showToast(successMessage, 'success');
}

/* ── Shared: AI Section ─────────────────────────────────────────── */
function tsViewAiSection(featureKey, placeholder, creditCost, onResult, options) {
    options = options || {};
    var lastSnapshot = null;
    var pendingResult = null;

    var section = document.createElement('div');
    section.className = 'ts-view-ai';
    var h = document.createElement('h4');
    h.textContent = '\u2728 Describe with AI';
    section.appendChild(h);
    var desc = document.createElement('p');
    desc.textContent = 'Describe what you want and AI will generate it. Cost: ' + creditCost + ' credits (\u2248$' + (creditCost * TS_CREDIT_PER_USD).toFixed(2) + ')';
    section.appendChild(desc);
    var ta = document.createElement('textarea');
    ta.placeholder = placeholder;
    if (window.LiqSurface && typeof window.LiqSurface.renderBar === 'function') {
        var surfaceBar = document.createElement('div');
        section.appendChild(surfaceBar);
        window.LiqSurface.renderBar(surfaceBar, {
            title: options.surfaceTitle || 'AI, history & experiments',
            description: options.surfaceDescription || 'Keep editing manually, use the prompt below for AI changes, open Theme Studio revisions, or spin this idea into an A/B test.',
            creditCost: creditCost,
            manualNote: options.manualNote || 'Manual edits and snapshots stay in your control.',
            getPrompt: function() { return ta.value.trim(); },
            onHistory: function() {
                if (options.historyPrefill) {
                    try {
                        localStorage.setItem('liq-ts-revisions-prefill', JSON.stringify(options.historyPrefill));
                    } catch (_) {}
                }
                if (typeof options.onHistory === 'function') {
                    options.onHistory();
                    return;
                }
                if (typeof navigateTo === 'function') navigateTo('ts-revisions');
            },
            historyDisabled: !!options.historyDisabled,
            historyDisabledReason: options.historyDisabledReason || 'Save once to unlock revisions.',
            getAbPrefill: function(info) {
                if (typeof options.abPrefill === 'function') return options.abPrefill(info || {});
                var promptText = info && info.prompt ? info.prompt.trim() : '';
                var label = options.surfaceName || featureKey || 'Theme Studio surface';
                return {
                    experiment_name: label + ' test',
                    hypothesis: promptText
                        ? 'Test an AI-generated Theme Studio change for ' + label + ': ' + promptText
                        : 'Test an AI-generated Theme Studio change against the current configuration.',
                    experiment_type: 'page_variant',
                    goal_type: 'conversion',
                    variants: [
                        {
                            name: 'Current',
                            value: 'Current ' + label + ' configuration',
                            description: 'Control: the current manual/saved version'
                        },
                        {
                            name: 'AI Draft',
                            value: promptText ? ('AI prompt: ' + promptText) : ('AI-generated ' + label + ' update'),
                            description: 'Variant: the AI-assisted change from this surface'
                        }
                    ]
                };
            },
            apiHint: options.apiHint || 'Theme Studio APIs: /api/modules/theme-studio/profiles/*, /api/modules/theme-studio/revisions/{entity_type}/{entity_id}, and /api/modules/theme-studio/pages/{slug}/layout'
        });
    }
    section.appendChild(ta);
    var previewBox = document.createElement('div');
    previewBox.className = 'ts-view-ai-preview';
    previewBox.style.cssText = 'display:none;margin:12px 0 0;padding:12px;border:1px solid var(--border);border-radius:10px;background:var(--surface-2, rgba(255,255,255,0.03));';
    section.appendChild(previewBox);
    var footer = document.createElement('div');
    footer.className = 'ts-view-ai-footer';
    var costEl = document.createElement('span');
    costEl.className = 'ts-view-ai-cost';
    costEl.textContent = 'LuperIQ cost: ' + creditCost + ' credits (\u2248$' + (creditCost * TS_CREDIT_PER_USD).toFixed(2) + ')';
    footer.appendChild(costEl);
    var saveSnapshotBtn = null;
    var undoBtn = null;

    function refreshSnapshotButtons() {
        if (saveSnapshotBtn) saveSnapshotBtn.disabled = !(typeof options.captureState === 'function');
        if (undoBtn) undoBtn.disabled = !lastSnapshot;
    }

    function captureSnapshot(label) {
        if (typeof options.captureState !== 'function') return null;
        var data = options.captureState();
        if (!data) return null;
        lastSnapshot = {
            label: label || 'snapshot',
            takenAt: new Date().toISOString(),
            data: data
        };
        refreshSnapshotButtons();
        return lastSnapshot;
    }

    function clearPreview() {
        pendingResult = null;
        previewBox.style.display = 'none';
        previewBox.replaceChildren();
    }

    function renderPreview(result) {
        if (!options.deferApply) return;
        pendingResult = result;
        previewBox.replaceChildren();
        previewBox.style.display = '';

        var title = document.createElement('div');
        title.style.cssText = 'font-weight:600;color:var(--text);margin-bottom:8px;';
        title.textContent = options.previewTitle || 'AI preview';
        previewBox.appendChild(title);

        var help = document.createElement('div');
        help.style.cssText = 'font-size:12px;color:var(--text-muted);margin-bottom:10px;';
        help.textContent = 'Nothing is saved yet. Review this draft, then click Keep to apply it.';
        previewBox.appendChild(help);

        if (typeof options.renderPreview === 'function') {
            options.renderPreview(result, previewBox);
            return;
        }

        var pre = document.createElement('pre');
        pre.style.cssText = 'margin:0;white-space:pre-wrap;word-break:break-word;font-size:12px;line-height:1.5;color:var(--text);';
        pre.textContent = typeof result === 'string' ? result : JSON.stringify(result, null, 2);
        previewBox.appendChild(pre);
    }

    if (typeof options.captureState === 'function') {
        saveSnapshotBtn = tsBtnGhost('Save Snapshot', function() {
            var snapshot = captureSnapshot('manual');
            if (!snapshot) return;
            var filename = (options.snapshotFilename || featureKey || 'theme-studio') + '-' + snapshot.takenAt.replace(/[:.]/g, '-') + '.json';
            tsDownloadJson(filename, snapshot.data, 'Snapshot downloaded');
        });
        footer.appendChild(saveSnapshotBtn);

        undoBtn = tsBtnGhost('Undo AI Apply', function() {
            if (!lastSnapshot || typeof options.restoreState !== 'function') return;
            options.restoreState(lastSnapshot.data);
            if (typeof showToast === 'function') showToast('Restored pre-AI snapshot', 'success');
        });
        undoBtn.disabled = true;
        footer.appendChild(undoBtn);
    }

    var genBtn = document.createElement('button');
    genBtn.className = 'ts-ai-gen-btn';
    genBtn.textContent = '\u2728 Generate';
    var feedback = document.createElement('div');
    genBtn.onclick = function() {
        var prompt = ta.value.trim();
        if (!prompt) { ta.focus(); return; }
        genBtn.disabled = true;
        genBtn.textContent = 'Generating...';
        captureSnapshot('pre-ai');
        if (window.LiqAI && typeof window.LiqAI.generate === 'function') {
            window.LiqAI.generate({
                feature: featureKey,
                userInput: prompt,
                container: feedback,
                onResult: function(result) {
                    if (options.deferApply) renderPreview(result);
                    else onResult(result);
                },
                onKeep: function() {
                    if (options.deferApply && typeof options.onKeepResult === 'function') {
                        try {
                            var maybe = options.onKeepResult(pendingResult);
                            if (maybe && typeof maybe.catch === 'function') {
                                maybe.catch(function(err) {
                                    if (typeof showToast === 'function') showToast((err && err.message) || 'Could not apply AI result', 'error');
                                });
                            }
                        } catch (err) {
                            if (typeof showToast === 'function') showToast((err && err.message) || 'Could not apply AI result', 'error');
                            return;
                        }
                        clearPreview();
                    }
                    if ((!options.deferApply || typeof options.onKeepResult !== 'function') && typeof showToast === 'function') {
                        showToast('AI generated and kept.', 'success');
                    }
                },
                onRetry: function() {
                    if (options.deferApply) clearPreview();
                },
                onError: function(message) {
                    if (options.deferApply) clearPreview();
                    if (typeof showToast === 'function') showToast(message || 'AI generation failed', 'error');
                }
            }).finally(function() {
                genBtn.disabled = false;
                genBtn.textContent = '\u2728 Generate';
            });
            return;
        }

        var request = (window.LiqAI && typeof window.LiqAI.request === 'function')
            ? window.LiqAI.request({ feature: featureKey, userInput: prompt })
            : fetch('/api/ai/generate', {
                method: 'POST', headers: {'Content-Type':'application/json'}, credentials: 'same-origin',
                body: JSON.stringify({ feature: featureKey, user_input: prompt, escalation_count: 0 })
            }).then(function(r) { return r.json(); }).then(function(data) {
                if (!data.ok || !data.result) throw new Error(data.message || 'AI generation failed');
                return data;
            });

        request.then(function(data) {
            genBtn.disabled = false; genBtn.textContent = '\u2728 Generate';
            if (options.deferApply) renderPreview(data.result);
            else onResult(data.result);
            if (typeof showToast === 'function') showToast('AI generated! (' + (data.credits_charged || creditCost) + ' credits)', 'success');
        }).catch(function(e) {
            genBtn.disabled = false; genBtn.textContent = '\u2728 Generate';
            if (options.deferApply) clearPreview();
            if (typeof showToast === 'function') showToast((e && e.message) || 'AI generation failed', 'error');
        });
    };
    footer.appendChild(genBtn);
    section.appendChild(footer);
    section.appendChild(feedback);
    refreshSnapshotButtons();
    return section;
}

/* ── Shared: Preset Picker ──────────────────────────────────────── */
function tsPresetPicker(presets, onApply) {
    var wrap = document.createElement('div');
    var h = document.createElement('h3');
    h.textContent = 'Quick Presets';
    h.style.marginBottom = '10px';
    wrap.appendChild(h);
    var grid = document.createElement('div');
    grid.className = 'ts-presets';
    presets.forEach(function(pr) {
        var card = document.createElement('div');
        card.className = 'ts-preset-card';
        var t = document.createElement('h4');
        t.textContent = pr.title;
        card.appendChild(t);
        var d = document.createElement('p');
        d.textContent = pr.description;
        card.appendChild(d);
        if (pr.summary) {
            var s = document.createElement('div');
            s.className = 'ts-preset-summary';
            s.textContent = pr.summary;
            card.appendChild(s);
        }
        var btn = tsBtn('Apply', function() { onApply(pr.data); });
        card.appendChild(btn);
        grid.appendChild(card);
    });
    wrap.appendChild(grid);
    return wrap;
}

/* ── Shared: Export / Import Bar ─────────────────────────────────── */
function tsExportImportBar(exportFn, importFn) {
    var bar = document.createElement('div');
    bar.className = 'ts-export-import-bar';
    bar.appendChild(tsBtnGhost('Export JSON', function() {
        var result = exportFn();
        if (!result) return;
        var blob = new Blob([JSON.stringify(result.data, null, 2)], { type: 'application/json' });
        var a = document.createElement('a');
        a.href = URL.createObjectURL(blob);
        a.download = result.filename || 'export.json';
        document.body.appendChild(a); a.click(); document.body.removeChild(a);
        URL.revokeObjectURL(a.href);
        if (typeof showToast === 'function') showToast('Exported ' + (result.filename || 'data'), 'success');
    }));
    bar.appendChild(tsBtnGhost('Import JSON', function() {
        var inp = document.createElement('input');
        inp.type = 'file'; inp.accept = '.json,application/json';
        inp.onchange = function() {
            var file = inp.files && inp.files[0];
            if (!file) return;
            file.text().then(function(text) {
                try { var data = JSON.parse(text); importFn(data); }
                catch(e) { if (typeof showToast === 'function') showToast('Invalid JSON file', 'error'); }
            });
        };
        inp.click();
    }));
    bar.appendChild(tsBuyCreditsLink());
    return bar;
}

/* ── Shared: Buy Credits Link ───────────────────────────────────── */
function tsBuyCreditsLink() {
    var a = document.createElement('a');
    a.className = 'ts-credits-link';
    a.textContent = 'Buy AI Credits';
    a.href = '#';
    a.onclick = function(e) { e.preventDefault(); if (typeof navigateTo === 'function') navigateTo('store'); };
    return a;
}
"##
}
