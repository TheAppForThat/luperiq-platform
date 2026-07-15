//! Inline JavaScript for the SEO Insights admin panel (meta editor, A/B, crawl, redirects, timeline).

pub(crate) const SEO_ADMIN_JS: &str = r##"
// ── Pro tier detection ────────────────────────────────────────────
var _seoRole = (window.__CMS && window.__CMS.nexusRole) || '';
var _seoIsPro = _seoRole === 'central' || _seoRole === 'professional' || _seoRole === 'enterprise';

// ── Score badge helper ─────────────────────────────────────────────
function seoScoreBadge(score) {
    const badge = document.createElement('span');
    badge.className = 'status-badge';
    badge.textContent = score;
    if (score >= 70) { badge.style.cssText = 'background:rgba(34,197,94,0.15);color:#22c55e;'; }
    else if (score >= 40) { badge.style.cssText = 'background:rgba(245,158,11,0.15);color:#f59e0b;'; }
    else { badge.style.cssText = 'background:rgba(239,68,68,0.15);color:#ef4444;'; }
    return badge;
}

function seoTruncate(str, maxLen) {
    if (!str) return '';
    return str.length > maxLen ? str.substring(0, maxLen) + '...' : str;
}

function seoProBadge() {
    var b = document.createElement('span');
    b.className = 'pro-badge';
    b.textContent = 'PRO';
    return b;
}

function seoUpgradeCta(container) {
    var cta = document.createElement('div');
    cta.className = 'tc-upgrade-cta';
    var h = document.createElement('strong'); h.textContent = 'Upgrade to Pro'; cta.appendChild(h);
    var p = document.createElement('p');
    p.textContent = 'AI-powered SEO optimization, redirect management, SERP preview, and bulk tools. Upgrade your plan to unlock Pro SEO features.';
    cta.appendChild(p);
    var link = document.createElement('a');
    link.href = '#settings'; link.className = 'btn btn-primary btn-sm'; link.textContent = 'View Plans';
    link.onclick = function(e) { e.preventDefault(); navigateTo('settings'); };
    cta.appendChild(link);
    container.appendChild(cta);
}

// ── AI helper functions ───────────────────────────────────────────
async function seoAiTitle(contentId) {
    var r = await fetch('/api/modules/seo/ai/title', {
        method: 'POST', headers: {'Content-Type':'application/json'},
        body: JSON.stringify({content_id: contentId})
    }).then(function(r) { return r.json(); });
    if (!r.ok) { toast('AI error: ' + r.message); return null; }
    return r.data.suggested_title;
}
async function seoAiDescription(contentId) {
    var r = await fetch('/api/modules/seo/ai/description', {
        method: 'POST', headers: {'Content-Type':'application/json'},
        body: JSON.stringify({content_id: contentId})
    }).then(function(r) { return r.json(); });
    if (!r.ok) { toast('AI error: ' + r.message); return null; }
    return r.data.suggested_description;
}
async function seoAiSchema(contentId) {
    var r = await fetch('/api/modules/seo/ai/schema', {
        method: 'POST', headers: {'Content-Type':'application/json'},
        body: JSON.stringify({content_id: contentId})
    }).then(function(r) { return r.json(); });
    if (!r.ok) { toast('AI error: ' + r.message); return null; }
    return r.data.suggested_schema;
}
async function seoAiKeywords(contentId) {
    var r = await fetch('/api/modules/seo/ai/keywords', {
        method: 'POST', headers: {'Content-Type':'application/json'},
        body: JSON.stringify({content_id: contentId})
    }).then(function(r) { return r.json(); });
    if (!r.ok) { toast('AI error: ' + r.message); return null; }
    return r.data.keywords;
}

// Credit costs for SEO AI operations
var _seoCreditCosts = { title: 1, description: 1, keywords: 1, schema: 2 };

function seoAiButton(label, creditCost, onclick) {
    var btn = document.createElement('button');
    btn.className = 'btn btn-ghost btn-sm';
    btn.style.cssText = 'font-size:11px;padding:2px 8px;';
    btn.textContent = label + ' (' + creditCost + ' cr)';
    btn.onclick = onclick;
    return btn;
}

// ── LiqAI-powered SEO field generation with feedback bar ─────────
// Wraps the SEO AI endpoints with the reusable LiqAI feedback UI
// Shows credit cost, before/after preview, Keep / Try Again / Didn't Work Right / Help
function seoAiWithFeedback(opts) {
    // opts: { contentId, field, fieldId, container, label }
    var container = opts.container;
    if (!container) return;
    var creditCost = _seoCreditCosts[opts.field] || 1;

    container.textContent = '';
    var statusDiv = document.createElement('div');
    statusDiv.className = 'ai-status';
    var spinner = document.createElement('span');
    spinner.className = 'spinner';
    statusDiv.appendChild(spinner);
    var statusText = document.createElement('span');
    statusText.textContent = ' Generating ' + opts.label + '...';
    statusDiv.appendChild(statusText);
    var costTag = document.createElement('span');
    costTag.style.cssText = 'margin-left:8px;font-size:11px;background:rgba(99,102,241,0.15);color:#818cf8;padding:1px 6px;border-radius:4px;';
    costTag.textContent = creditCost + ' credit' + (creditCost > 1 ? 's' : '');
    statusDiv.appendChild(costTag);
    container.appendChild(statusDiv);

    var endpoint = '/api/modules/seo/ai/' + opts.field;
    var prevValue = '';
    var inputEl = document.getElementById(opts.fieldId);
    if (inputEl) prevValue = inputEl.value;

    fetch(endpoint, {
        method: 'POST', headers: {'Content-Type':'application/json'},
        body: JSON.stringify({content_id: opts.contentId})
    }).then(function(r) { return r.json(); }).then(function(data) {
        if (!data.ok) {
            container.textContent = '';
            var errDiv = document.createElement('div');
            errDiv.className = 'ai-status';
            errDiv.style.color = 'var(--danger)';
            errDiv.textContent = '\u26A0 ' + (data.message || 'Generation failed');
            container.appendChild(errDiv);
            return;
        }
        // Extract result based on field type
        var value;
        var extraInfo = '';
        if (opts.field === 'title') value = data.data.suggested_title;
        else if (opts.field === 'description') value = data.data.suggested_description;
        else if (opts.field === 'schema') value = data.data.suggested_schema;
        else if (opts.field === 'keywords') {
            var kws = data.data.keywords || [];
            if (kws.length > 0) value = kws[0];
            if (kws.length > 1) extraInfo = 'Other suggestions: ' + kws.slice(1).join(', ');
        }

        // Show before/after preview panel
        container.textContent = '';
        var previewWrap = document.createElement('div');
        previewWrap.style.cssText = 'background:var(--surface2,#1e293b);border:1px solid var(--border);border-radius:8px;padding:12px;margin:8px 0;';

        var previewHeader = document.createElement('div');
        previewHeader.style.cssText = 'display:flex;align-items:center;gap:8px;margin-bottom:8px;';
        var previewLabel = document.createElement('div');
        previewLabel.style.cssText = 'font-size:11px;color:var(--text-muted);font-weight:600;';
        previewLabel.textContent = 'AI SUGGESTION \u2014 ' + opts.label.toUpperCase();
        previewHeader.appendChild(previewLabel);
        var costBadge = document.createElement('span');
        costBadge.style.cssText = 'font-size:10px;background:rgba(99,102,241,0.15);color:#818cf8;padding:1px 6px;border-radius:4px;';
        costBadge.textContent = creditCost + ' credit' + (creditCost > 1 ? 's' : '') + ' used';
        previewHeader.appendChild(costBadge);
        previewWrap.appendChild(previewHeader);

        // Before value (strikethrough)
        if (prevValue && prevValue !== value) {
            var beforeDiv = document.createElement('div');
            beforeDiv.style.cssText = 'font-size:12px;color:var(--text-muted);margin-bottom:6px;';
            var beforeLabel = document.createElement('span');
            beforeLabel.style.cssText = 'font-weight:600;color:#ef4444;margin-right:4px;';
            beforeLabel.textContent = 'Before:';
            beforeDiv.appendChild(beforeLabel);
            var beforeText = document.createElement('span');
            beforeText.style.textDecoration = 'line-through';
            beforeText.textContent = ' ' + seoTruncate(prevValue, 120);
            beforeDiv.appendChild(beforeText);
            previewWrap.appendChild(beforeDiv);
        }

        // After value (the suggestion, highlighted green)
        var afterDiv = document.createElement('div');
        afterDiv.style.cssText = 'font-size:13px;color:#22c55e;';
        var afterLabel = document.createElement('span');
        afterLabel.style.cssText = 'font-weight:600;margin-right:4px;';
        afterLabel.textContent = 'After:';
        afterDiv.appendChild(afterLabel);
        var afterText = document.createElement('span');
        if (opts.field === 'schema') {
            afterText.textContent = ' ' + seoTruncate(value || '(no suggestion)', 200);
        } else {
            afterText.textContent = ' ' + (value || '(no suggestion)');
        }
        afterDiv.appendChild(afterText);
        previewWrap.appendChild(afterDiv);

        // Extra info (e.g., other keyword suggestions)
        if (extraInfo) {
            var extraDiv = document.createElement('div');
            extraDiv.style.cssText = 'font-size:11px;color:var(--text-muted);margin-top:6px;font-style:italic;';
            extraDiv.textContent = extraInfo;
            previewWrap.appendChild(extraDiv);
        }

        container.appendChild(previewWrap);

        // Apply to field as live preview (like theme generator applies colors)
        if (value && inputEl) inputEl.value = value;

        // Render LiqAI feedback bar
        var feedbackDiv = document.createElement('div');
        container.appendChild(feedbackDiv);

        if (window.LiqAI && window.LiqAI.renderFeedbackBar) {
            var interactionId = 'seo_' + opts.field + '_' + Date.now();
            window.LiqAI.renderFeedbackBar(feedbackDiv, interactionId, {
                feature: 'seo_' + opts.field,
                userInput: 'content_id:' + opts.contentId,
                container: feedbackDiv,
                onKeep: function() {
                    toast(opts.label + ' accepted \u2014 click Save to keep it.');
                    container.textContent = '';
                },
                onRetry: function() {
                    if (inputEl) inputEl.value = prevValue;
                    seoAiWithFeedback(opts);
                },
                onError: function(msg) {
                    if (inputEl) inputEl.value = prevValue;
                }
            }, 0);
        } else {
            // Fallback if LiqAI not loaded
            var bar = document.createElement('div');
            bar.className = 'ai-feedback-bar';
            var keepBtn = document.createElement('button');
            keepBtn.className = 'btn btn-sm btn-success';
            keepBtn.textContent = 'Keep';
            keepBtn.onclick = function() { toast(opts.label + ' accepted \u2014 click Save'); container.textContent = ''; };
            bar.appendChild(keepBtn);
            var retryBtn = document.createElement('button');
            retryBtn.className = 'btn btn-sm';
            retryBtn.textContent = 'Try Again';
            retryBtn.onclick = function() { if (inputEl) inputEl.value = prevValue; seoAiWithFeedback(opts); };
            bar.appendChild(retryBtn);
            var revertBtn = document.createElement('button');
            revertBtn.className = 'btn btn-sm btn-ghost';
            revertBtn.textContent = 'Revert';
            revertBtn.onclick = function() { if (inputEl) inputEl.value = prevValue; container.textContent = ''; };
            bar.appendChild(revertBtn);
            container.appendChild(bar);
        }
    }).catch(function(err) {
        container.textContent = '';
        var errDiv = document.createElement('div');
        errDiv.className = 'ai-status';
        errDiv.style.color = 'var(--danger)';
        errDiv.textContent = '\u26A0 Network error';
        container.appendChild(errDiv);
    });
}

// ── SERP Preview ──────────────────────────────────────────────────
function renderSerpPreview(container, titleId, descId, slugText) {
    var preview = document.createElement('div');
    preview.style.cssText = 'margin:16px 0;padding:16px;background:var(--bg);border:1px solid var(--border);border-radius:8px;';
    var label = document.createElement('div');
    label.style.cssText = 'font-size:11px;color:var(--text-muted);margin-bottom:8px;font-weight:600;';
    label.textContent = 'SERP PREVIEW';
    if (!_seoIsPro) { label.appendChild(document.createTextNode(' ')); label.appendChild(seoProBadge()); }
    preview.appendChild(label);

    var serpTitle = document.createElement('div');
    serpTitle.style.cssText = 'font-size:18px;color:#1a0dab;font-family:arial,sans-serif;cursor:pointer;line-height:1.3;';
    serpTitle.id = 'serp-title-preview';
    preview.appendChild(serpTitle);

    var serpUrl = document.createElement('div');
    serpUrl.style.cssText = 'font-size:13px;color:#006621;font-family:arial,sans-serif;margin:2px 0;';
    serpUrl.textContent = 'luperiq.com > ' + (slugText || 'page');
    preview.appendChild(serpUrl);

    var serpDesc = document.createElement('div');
    serpDesc.style.cssText = 'font-size:13px;color:#545454;font-family:arial,sans-serif;line-height:1.4;';
    serpDesc.id = 'serp-desc-preview';
    preview.appendChild(serpDesc);

    function updatePreview() {
        var tEl = document.getElementById(titleId);
        var dEl = document.getElementById(descId);
        if (tEl) serpTitle.textContent = tEl.value || 'Page Title';
        if (dEl) serpDesc.textContent = dEl.value || 'Meta description will appear here...';
        // Color title red if over 60 chars
        if (tEl && tEl.value.length > 60) serpTitle.style.color = '#ef4444';
        else serpTitle.style.color = '#1a0dab';
    }

    container.appendChild(preview);

    // Attach live update listeners
    setTimeout(function() {
        var tEl = document.getElementById(titleId);
        var dEl = document.getElementById(descId);
        if (tEl) tEl.addEventListener('input', updatePreview);
        if (dEl) dEl.addEventListener('input', updatePreview);
        updatePreview();
    }, 0);
}

// ── SEO Dashboard — master view with all pages, slugs, inline editing ──
let _seoDashData = null;
let _seoDashGoogleData = {};
let _seoDashInsights = [];
let _seoDashOpportunities = [];
let _seoDashFilter = 'all';
let _seoDashSort = 'slug';
let _seoDashSearch = '';

async function load_seo_dashboard() {
    const main = document.getElementById('adminMain');
    const el = document.createElement('div');

    var _role = (window.__CMS && window.__CMS.nexusRole) || '';
    var _isPro = _role === 'central' || _role === 'professional' || _role === 'enterprise';
    var _isStarter = _isPro || _role === 'starter';

    // Pricing card
    var _pc = lqModulePricingCard({ name: 'SEO Insights', monthly: 14, annual: 139, lifetime: 399, tier: 'starter', deps: [], slug: 'seo' });
    if (_pc) el.appendChild(_pc);

    const h = document.createElement('h2'); h.textContent = 'SEO Dashboard'; el.appendChild(h);

    // Fetch export data + Google data in parallel
    var dateEnd = new Date().toISOString().slice(0,10);
    var dateStart = new Date(Date.now()-28*86400000).toISOString().slice(0,10);
    const [r, gscR, gscQR, gscOppR] = await Promise.all([
        fetch('/api/modules/seo/export').then(r => r.json()).catch(() => ({data:[]})),
        fetch('/api/modules/seo/google/gsc/pages?start_date=' + dateStart + '&end_date=' + dateEnd).then(r => r.json()).catch(() => ({ok:false})),
        fetch('/api/modules/seo/google/gsc/queries?start_date=' + dateStart + '&end_date=' + dateEnd).then(r => r.json()).catch(() => ({ok:false})),
        fetch('/api/modules/seo/google/gsc/opportunities?start_date=' + dateStart + '&end_date=' + dateEnd).then(r => r.json()).catch(() => ({ok:false})),
    ]);
    _seoDashData = r.data || [];

    // Build Google data lookup by page path
    _seoDashGoogleData = {};
    var gscPages = (gscR && gscR.ok && gscR.data) ? (gscR.data.pages || gscR.data.items || gscR.data || []) : [];
    if (Array.isArray(gscPages)) {
        gscPages.forEach(function(p) {
            var pagePath = p.page || p.keys || '';
            try { pagePath = new URL(pagePath).pathname; } catch(e) {}
            pagePath = pagePath.replace(/^\//, '').replace(/\/$/, '');
            if (pagePath) _seoDashGoogleData[pagePath] = p;
        });
    }

    // Fetch insights using real GSC data
    var gscQueries = (gscQR && gscQR.ok && gscQR.data) ? (gscQR.data.queries || gscQR.data.items || gscQR.data || []) : [];
    if (!Array.isArray(gscQueries)) gscQueries = [];
    _seoDashOpportunities = (gscOppR && gscOppR.ok && gscOppR.data)
        ? (gscOppR.data.items || [])
        : [];
    if (!Array.isArray(_seoDashOpportunities)) _seoDashOpportunities = [];
    _seoDashInsights = [];
    if (gscQueries.length > 0 || gscPages.length > 0) {
        try {
            var insR = await fetch('/api/modules/seo/insights', {
                method:'POST', headers:{'Content-Type':'application/json'},
                body: JSON.stringify({ queries: gscQueries.slice(0, 100), pages: gscPages.slice(0, 100) })
            }).then(function(r) { return r.json(); });
            _seoDashInsights = (insR && insR.ok && insR.data) ? (insR.data.insights || []) : [];
        } catch(e) {}
    }

    // Stats row
    const total = _seoDashData.length;
    const withTitle = _seoDashData.filter(d => d.seo_title).length;
    const withKw = _seoDashData.filter(d => d.focus_keyword).length;
    const scores = _seoDashData.filter(d => d.seo_score > 0).map(d => d.seo_score);
    const avgScore = scores.length ? Math.round(scores.reduce((a,b) => a+b, 0) / scores.length) : 0;
    const lowScore = _seoDashData.filter(d => d.seo_score > 0 && d.seo_score < 40).length;

    var totalClicks = 0; var totalImpressions = 0; var pagesWithGoogle = 0;
    Object.values(_seoDashGoogleData).forEach(function(g) {
        totalClicks += (g.clicks || 0);
        totalImpressions += (g.impressions || 0);
        pagesWithGoogle++;
    });

    const statsDiv = document.createElement('div'); statsDiv.className = 'stats';
    var statItems = [
        ['Total Pages', total, false],
        ['With Meta', withTitle, false],
        ['Avg Score', avgScore, true],
        ['Low Score', lowScore, true],
    ];
    if (pagesWithGoogle > 0) {
        statItems.push(['Clicks (28d)', totalClicks.toLocaleString(), false]);
        statItems.push(['Impressions', totalImpressions.toLocaleString(), false]);
    }
    statItems.forEach(([label, value, accent]) => {
        const card = document.createElement('div'); card.className = 'stat-card';
        const l = document.createElement('div'); l.className = 'label'; l.textContent = label; card.appendChild(l);
        const v = document.createElement('div'); v.className = 'value' + (accent ? ' accent' : ''); v.textContent = value; card.appendChild(v);
        statsDiv.appendChild(card);
    });
    el.appendChild(statsDiv);

    // Filter bar + search + export/import
    const filterBar = document.createElement('div');
    filterBar.style.cssText = 'display:flex;gap:8px;align-items:center;flex-wrap:wrap;margin:16px 0;';

    var noGoogleCount = _seoDashData.filter(function(d) { return !_seoDashGoogleData[d.slug]; }).length;
    var lowCtrCount = _seoDashData.filter(function(d) {
        var g = _seoDashGoogleData[d.slug]; return g && g.ctr != null && parseFloat(g.ctr) < 0.03 && (g.impressions || 0) > 10;
    }).length;
    var page2Count = _seoDashData.filter(function(d) {
        var g = _seoDashGoogleData[d.slug]; return g && g.position != null && parseFloat(g.position) >= 11 && parseFloat(g.position) <= 20;
    }).length;

    const filters = [
        ['all', 'All (' + total + ')'],
        ['missing', 'Missing Meta (' + (total - withTitle) + ')'],
        ['low-score', 'Low Score (' + lowScore + ')'],
        ['low-ctr', 'Low CTR (' + lowCtrCount + ')'],
        ['page-2', 'Page 2 (' + page2Count + ')'],
        ['no-google', 'No Google Data (' + noGoogleCount + ')'],
        ['pages', 'Pages'],
        ['posts', 'Posts'],
    ];
    filters.forEach(([val, label]) => {
        const btn = document.createElement('button');
        btn.className = 'btn btn-sm ' + (val === _seoDashFilter ? 'btn-primary' : 'btn-ghost');
        btn.textContent = label;
        btn.onclick = () => { _seoDashFilter = val; renderDashTable(el); };
        filterBar.appendChild(btn);
    });

    const searchInput = document.createElement('input');
    searchInput.type = 'text';
    searchInput.className = 'admin-input';
    searchInput.placeholder = 'Search slugs, titles...';
    searchInput.value = _seoDashSearch;
    searchInput.style.cssText = 'width:200px;margin-left:auto;';
    searchInput.oninput = () => { _seoDashSearch = searchInput.value.toLowerCase(); renderDashTable(el); };
    filterBar.appendChild(searchInput);

    el.appendChild(filterBar);

    // Export / Import buttons
    const eiBar = document.createElement('div');
    eiBar.style.cssText = 'display:flex;gap:8px;margin-bottom:16px;';
    const exportBtn = document.createElement('button');
    exportBtn.className = 'btn btn-ghost btn-sm';
    exportBtn.textContent = 'Export JSON';
    exportBtn.onclick = () => lqExportJSON(_seoDashData, 'seo-export.json');
    eiBar.appendChild(exportBtn);

    const exportCsvBtn = document.createElement('button');
    exportCsvBtn.className = 'btn btn-ghost btn-sm';
    exportCsvBtn.textContent = 'Export CSV';
    exportCsvBtn.onclick = () => {
        lqExportCSV(_seoDashData, ['content_id','content_type','slug','page_title','seo_title','seo_description','focus_keyword','seo_score'], 'seo-export.csv');
    };
    eiBar.appendChild(exportCsvBtn);

    const importBtn = document.createElement('button');
    importBtn.className = 'btn btn-ghost btn-sm';
    importBtn.textContent = 'Import JSON';
    importBtn.onclick = () => {
        lqImportJSON(async function(data) {
            const items = Array.isArray(data) ? data : (data.items || [data]);
            const body = { items: items.map(d => ({
                slug: d.slug,
                new_slug: d.new_slug || undefined,
                title: d.seo_title || d.title || '',
                description: d.seo_description || d.description || '',
                focus_keyword: d.focus_keyword || '',
            }))};
            const r = await fetch('/api/modules/seo/import', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify(body),
            }).then(r => r.json());
            showToast(r.ok ? r.message : 'Import failed: ' + r.message, r.ok ? 'success' : 'error');
            if (r.ok) load_seo_dashboard();
        });
    };
    eiBar.appendChild(importBtn);

    var bulkBtn = document.createElement('button');
    bulkBtn.className = 'btn btn-sm ' + (_seoIsPro ? 'btn-primary' : 'btn-ghost');
    bulkBtn.textContent = 'Bulk Optimize';
    if (!_seoIsPro) {
        bulkBtn.disabled = true;
        bulkBtn.appendChild(document.createTextNode(' '));
        bulkBtn.appendChild(seoProBadge());
    } else {
        bulkBtn.onclick = function() { navigateTo('seo-bulk'); };
    }
    eiBar.appendChild(bulkBtn);

    const helpNote = document.createElement('span');
    helpNote.style.cssText = 'font-size:11px;color:var(--text-muted);margin-left:8px;';
    helpNote.textContent = 'Import format: [{slug, new_slug?, title, description, focus_keyword}]';
    eiBar.appendChild(helpNote);

    el.appendChild(eiBar);

    if (_seoDashOpportunities.length > 0) {
        var oppSection = document.createElement('div');
        oppSection.style.cssText = 'margin-bottom:20px;';
        var oppH = document.createElement('h3');
        oppH.style.cssText = 'font-size:14px;margin-bottom:8px;';
        oppH.textContent = 'Search Console Opportunities';
        oppSection.appendChild(oppH);
        var oppGrid = document.createElement('div');
        oppGrid.style.cssText = 'display:grid;grid-template-columns:repeat(auto-fill,minmax(280px,1fr));gap:10px;';
        _seoDashOpportunities.slice(0, 6).forEach(function(opp) {
            var card = document.createElement('div');
            card.style.cssText = 'padding:12px;background:var(--surface);border:1px solid var(--border);border-radius:8px;border-left:3px solid #0ea5e9;';
            var title = document.createElement('div');
            title.style.cssText = 'font-weight:600;font-size:13px;margin-bottom:4px;';
            title.textContent = opp.query || opp.label || 'Opportunity';
            card.appendChild(title);
            var badge = document.createElement('div');
            badge.style.cssText = 'font-size:11px;color:#0ea5e9;margin-bottom:6px;font-weight:600;';
            badge.textContent = (opp.label || opp.opportunity_type || 'Opportunity');
            card.appendChild(badge);
            var metrics = document.createElement('div');
            metrics.style.cssText = 'font-size:12px;color:var(--text-muted);margin-bottom:6px;';
            var ctrPct = opp.ctr != null ? (parseFloat(opp.ctr) * 100).toFixed(1) + '%' : 'n/a';
            var posLabel = opp.position != null ? Number(opp.position).toFixed(1) : 'n/a';
            metrics.textContent = 'Impressions: ' + (opp.impressions || 0) + ' | Position: ' + posLabel + ' | CTR: ' + ctrPct;
            card.appendChild(metrics);
            var rec = document.createElement('div');
            rec.style.cssText = 'font-size:12px;color:var(--text-muted);line-height:1.5;';
            rec.textContent = opp.recommendation || '';
            card.appendChild(rec);
            oppGrid.appendChild(card);
        });
        oppSection.appendChild(oppGrid);
        el.appendChild(oppSection);
    }

    // Insights panel (if Google data available)
    if (_seoDashInsights.length > 0) {
        var insightsSection = document.createElement('div');
        insightsSection.style.cssText = 'margin-bottom:20px;';
        var insH = document.createElement('h3');
        insH.style.cssText = 'font-size:14px;margin-bottom:8px;';
        insH.textContent = 'SEO Insights';
        insightsSection.appendChild(insH);
        var insGrid = document.createElement('div');
        insGrid.style.cssText = 'display:grid;grid-template-columns:repeat(auto-fill,minmax(280px,1fr));gap:10px;';
        _seoDashInsights.slice(0, 6).forEach(function(ins) {
            var card = document.createElement('div');
            var borderColor = ins.severity === 'critical' ? '#ef4444' : ins.severity === 'warning' ? '#f59e0b' : ins.severity === 'opportunity' ? '#3b82f6' : 'var(--border)';
            card.style.cssText = 'padding:12px;background:var(--surface);border:1px solid var(--border);border-radius:8px;border-left:3px solid ' + borderColor + ';';
            var insTitle = document.createElement('div');
            insTitle.style.cssText = 'font-weight:500;font-size:13px;margin-bottom:4px;';
            insTitle.textContent = ins.title;
            card.appendChild(insTitle);
            var insDesc = document.createElement('div');
            insDesc.style.cssText = 'font-size:12px;color:var(--text-muted);margin-bottom:6px;';
            insDesc.textContent = ins.description;
            card.appendChild(insDesc);
            if (ins.metric_label && ins.metric_value) {
                var metric = document.createElement('div');
                metric.style.cssText = 'font-size:11px;color:var(--accent);';
                metric.textContent = ins.metric_label + ': ' + ins.metric_value;
                card.appendChild(metric);
            }
            if (ins.action) {
                var action = document.createElement('div');
                action.style.cssText = 'font-size:11px;color:var(--success);margin-top:4px;';
                action.textContent = ins.action;
                card.appendChild(action);
            }
            insGrid.appendChild(card);
        });
        insightsSection.appendChild(insGrid);
        el.appendChild(insightsSection);
    }

    // Table container (will be re-rendered on filter/search)
    const tableDiv = document.createElement('div');
    tableDiv.id = 'seoDashTable';
    el.appendChild(tableDiv);

    main.replaceChildren(el);
    renderDashTable(el);
}

function renderDashTable(container) {
    const tableDiv = document.getElementById('seoDashTable') || container.querySelector('#seoDashTable');
    if (!tableDiv) return;
    tableDiv.replaceChildren();

    let items = _seoDashData || [];

    // Apply filter
    if (_seoDashFilter === 'missing') items = items.filter(d => !d.seo_title);
    else if (_seoDashFilter === 'no-keyword') items = items.filter(d => !d.focus_keyword);
    else if (_seoDashFilter === 'low-score') items = items.filter(d => d.seo_score > 0 && d.seo_score < 40);
    else if (_seoDashFilter === 'pages') items = items.filter(d => d.content_type === 'page' || d.content_type === 'static_page');
    else if (_seoDashFilter === 'posts') items = items.filter(d => d.content_type === 'post');
    else if (_seoDashFilter === 'low-ctr') items = items.filter(d => { var g = _seoDashGoogleData[d.slug]; return g && g.ctr != null && parseFloat(g.ctr) < 0.03 && (g.impressions || 0) > 10; });
    else if (_seoDashFilter === 'page-2') items = items.filter(d => { var g = _seoDashGoogleData[d.slug]; return g && g.position != null && parseFloat(g.position) >= 11 && parseFloat(g.position) <= 20; });
    else if (_seoDashFilter === 'no-google') items = items.filter(d => !_seoDashGoogleData[d.slug]);
    else if (_seoDashFilter === 'missing-desc') items = items.filter(d => !d.seo_description);
    else if (_seoDashFilter === 'no-og') items = items.filter(d => !d.og_image);
    else if (_seoDashFilter === 'no-schema') items = items.filter(d => !d.has_schema);
    else if (_seoDashFilter === 'missing-robots') items = items.filter(d => !d.robots);

    // Apply search
    if (_seoDashSearch) {
        items = items.filter(d =>
            (d.slug || '').toLowerCase().includes(_seoDashSearch) ||
            (d.page_title || '').toLowerCase().includes(_seoDashSearch) ||
            (d.seo_title || '').toLowerCase().includes(_seoDashSearch) ||
            (d.focus_keyword || '').toLowerCase().includes(_seoDashSearch)
        );
    }

    // Sort by slug
    items.sort((a, b) => (a.slug || '').localeCompare(b.slug || ''));

    const countLabel = document.createElement('div');
    countLabel.style.cssText = 'font-size:12px;color:var(--text-muted);margin-bottom:8px;';
    countLabel.textContent = 'Showing ' + items.length + ' of ' + (_seoDashData || []).length + ' pages';
    tableDiv.appendChild(countLabel);

    var hasGoogleData = Object.keys(_seoDashGoogleData).length > 0;

    const wrapper = document.createElement('div');
    wrapper.className = 'content-table';
    wrapper.style.cssText = 'overflow-x:auto;';
    const t = document.createElement('table');
    t.style.cssText = 'width:100%;';
    const hdr = document.createElement('tr');
    var cols = ['Slug','SEO Title','Keyword','Score'];
    if (hasGoogleData) cols.push('Clicks', 'Impressions', 'Position', 'CTR');
    cols.push('');
    cols.forEach(col => {
        const th = document.createElement('th');
        th.textContent = col;
        if (col === 'Clicks' || col === 'Impressions' || col === 'Position' || col === 'CTR') th.style.cssText = 'text-align:right;font-size:11px;';
        hdr.appendChild(th);
    });
    t.appendChild(hdr);

    if (items.length === 0) {
        const tr = document.createElement('tr');
        const td = document.createElement('td'); td.colSpan = cols.length;
        td.style.cssText = 'text-align:center;padding:20px;color:var(--text-muted);';
        td.textContent = 'No pages match the current filter.';
        tr.appendChild(td); t.appendChild(tr);
    }

    items.forEach(item => {
        const tr = document.createElement('tr');
        var gData = _seoDashGoogleData[item.slug] || null;

        // Slug column with page title subtitle
        const slugTd = document.createElement('td');
        slugTd.style.cssText = 'max-width:250px;';
        const slugLink = document.createElement('a');
        slugLink.href = '/' + item.slug;
        slugLink.target = '_blank';
        slugLink.textContent = '/' + item.slug;
        slugLink.style.cssText = 'color:var(--accent);text-decoration:none;font-family:monospace;font-size:12px;word-break:break-all;';
        slugTd.appendChild(slugLink);
        if (item.page_title) {
            var ptSub = document.createElement('div');
            ptSub.style.cssText = 'font-size:11px;color:var(--text-muted);margin-top:1px;';
            ptSub.textContent = seoTruncate(item.page_title, 35);
            ptSub.title = item.page_title;
            slugTd.appendChild(ptSub);
        }
        tr.appendChild(slugTd);

        // SEO title
        const stTd = document.createElement('td');
        stTd.style.cssText = 'font-size:12px;max-width:200px;';
        if (item.seo_title) {
            stTd.textContent = seoTruncate(item.seo_title, 40);
            stTd.title = item.seo_title;
        } else {
            stTd.textContent = '(none)';
            stTd.style.color = 'var(--danger)';
        }
        tr.appendChild(stTd);

        // Keyword
        const kwTd = document.createElement('td');
        kwTd.style.cssText = 'font-size:12px;';
        if (item.focus_keyword) {
            const kwBadge = document.createElement('span');
            kwBadge.style.cssText = 'background:rgba(59,130,246,0.12);color:var(--accent);padding:2px 6px;border-radius:4px;font-size:11px;';
            kwBadge.textContent = item.focus_keyword;
            kwTd.appendChild(kwBadge);
        } else {
            kwTd.textContent = '-';
            kwTd.style.color = 'var(--text-muted)';
        }
        tr.appendChild(kwTd);

        // Score
        const scoreTd = document.createElement('td');
        if (item.seo_score > 0) { scoreTd.appendChild(seoScoreBadge(item.seo_score)); }
        else { scoreTd.textContent = '-'; scoreTd.style.color = 'var(--text-muted)'; }
        tr.appendChild(scoreTd);

        // Google columns
        if (hasGoogleData) {
            var clicksTd = document.createElement('td');
            clicksTd.style.cssText = 'text-align:right;font-size:12px;font-variant-numeric:tabular-nums;';
            clicksTd.textContent = gData ? (gData.clicks || 0).toLocaleString() : '-';
            if (!gData) clicksTd.style.color = 'var(--text-muted)';
            tr.appendChild(clicksTd);

            var impTd = document.createElement('td');
            impTd.style.cssText = 'text-align:right;font-size:12px;font-variant-numeric:tabular-nums;';
            impTd.textContent = gData ? (gData.impressions || 0).toLocaleString() : '-';
            if (!gData) impTd.style.color = 'var(--text-muted)';
            tr.appendChild(impTd);

            var posTd = document.createElement('td');
            posTd.style.cssText = 'text-align:right;font-size:12px;';
            if (gData && gData.position != null) {
                var pos = parseFloat(gData.position).toFixed(1);
                posTd.textContent = pos;
                if (parseFloat(pos) <= 3) posTd.style.color = '#22c55e';
                else if (parseFloat(pos) <= 10) posTd.style.color = '#f59e0b';
                else posTd.style.color = '#ef4444';
            } else { posTd.textContent = '-'; posTd.style.color = 'var(--text-muted)'; }
            tr.appendChild(posTd);

            var ctrTd = document.createElement('td');
            ctrTd.style.cssText = 'text-align:right;font-size:12px;';
            if (gData && gData.ctr != null) {
                var ctrVal = (parseFloat(gData.ctr) * 100).toFixed(1) + '%';
                ctrTd.textContent = ctrVal;
                if (parseFloat(gData.ctr) < 0.03) ctrTd.style.color = '#ef4444';
            } else { ctrTd.textContent = '-'; ctrTd.style.color = 'var(--text-muted)'; }
            tr.appendChild(ctrTd);
        }

        // Actions
        const actTd = document.createElement('td');
        actTd.style.whiteSpace = 'nowrap';
        const editBtn = document.createElement('button');
        editBtn.className = 'btn btn-ghost btn-sm';
        editBtn.textContent = 'Edit';
        editBtn.onclick = () => openDashEditor(item);
        actTd.appendChild(editBtn);
        if (hasGoogleData && gData && typeof openPageDetailModal === 'function') {
            var gBtn = document.createElement('button');
            gBtn.className = 'btn btn-ghost btn-sm';
            gBtn.style.cssText = 'font-size:11px;margin-left:4px;';
            gBtn.textContent = 'Google';
            gBtn.onclick = function() { openPageDetailModal('/' + item.slug, {}); };
            actTd.appendChild(gBtn);
        }
        tr.appendChild(actTd);

        t.appendChild(tr);
    });

    wrapper.appendChild(t);
    tableDiv.appendChild(wrapper);
}

function openSlugRename(item) {
    const newSlug = prompt('Rename slug (old URL will 301 redirect):\n\nCurrent: /' + item.slug + '\n\nNew slug:', item.slug);
    if (!newSlug || newSlug === item.slug) return;
    const clean = newSlug.replace(/^\//, '').replace(/\/$/, '').trim();
    if (!clean) return;
    fetch('/api/modules/seo/import', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ items: [{
            slug: item.slug,
            new_slug: clean,
            title: item.seo_title || item.page_title || '',
            description: item.seo_description || '',
            focus_keyword: item.focus_keyword || '',
        }]}),
    }).then(r => r.json()).then(r => {
        showToast(r.ok ? 'Slug renamed! 301 redirect created.' : 'Error: ' + r.message, r.ok ? 'success' : 'error');
        if (r.ok) load_seo_dashboard();
    });
}

function openDashEditor(item) {
    const main = document.getElementById('adminMain');
    const el = document.createElement('div');

    const backBtn = document.createElement('button');
    backBtn.className = 'btn btn-ghost btn-sm';
    backBtn.textContent = 'Back to Dashboard';
    backBtn.style.marginBottom = '16px';
    backBtn.onclick = () => load_seo_dashboard();
    el.appendChild(backBtn);

    const h = document.createElement('h2');
    h.textContent = 'Edit SEO: ' + (item.page_title || item.slug);
    el.appendChild(h);

    // Info line
    const info = document.createElement('div');
    info.style.cssText = 'font-size:12px;color:var(--text-muted);margin-bottom:12px;';
    info.textContent = 'Type: ' + item.content_type + ' | Slug: /' + item.slug + ' | ID: ' + item.content_id;
    el.appendChild(info);

    if (item.seo_score > 0) {
        const scoreLine = document.createElement('div');
        scoreLine.style.cssText = 'margin-bottom:16px;display:flex;align-items:center;gap:8px;';
        const scoreLbl = document.createElement('span'); scoreLbl.textContent = 'Current Score:'; scoreLbl.style.fontSize = '13px';
        scoreLine.appendChild(scoreLbl);
        scoreLine.appendChild(seoScoreBadge(item.seo_score));
        el.appendChild(scoreLine);
    }

    const card = document.createElement('div');
    card.style.cssText = 'max-width:700px;background:var(--surface);border:1px solid var(--border);border-radius:8px;padding:20px;';

    function mkRow(id, label, type, value, placeholder, charLimit) {
        const row = document.createElement('div'); row.style.marginBottom = '12px';
        const lbl = document.createElement('label');
        lbl.style.cssText = 'font-size:12px;color:var(--text-muted);display:block;margin-bottom:4px;';
        lbl.textContent = label; row.appendChild(lbl);
        let input;
        if (type === 'textarea') {
            input = document.createElement('textarea'); input.rows = 3; input.className = 'admin-input';
        } else {
            input = document.createElement('input'); input.type = 'text'; input.className = 'admin-input';
        }
        input.id = id; input.value = value || ''; if (placeholder) input.placeholder = placeholder;
        row.appendChild(input);
        if (charLimit) {
            const counter = document.createElement('div');
            counter.style.cssText = 'font-size:11px;color:var(--text-muted);margin-top:2px;';
            function updateCounter() {
                const len = input.value.length;
                counter.textContent = len + '/' + charLimit + ' characters';
                counter.style.color = len > charLimit ? 'var(--danger)' : 'var(--text-muted)';
            }
            updateCounter();
            input.oninput = updateCounter;
            row.appendChild(counter);
        }
        card.appendChild(row);
    }

    // Slug rename
    mkRow('dash-slug', 'URL Slug (change to rename + auto-301 redirect)', 'text', item.slug, 'keyword-rich-url-slug');
    mkRow('dash-title', 'SEO Title', 'text', item.seo_title || '', item.page_title, 60);
    mkRow('dash-desc', 'Meta Description', 'textarea', item.seo_description || '', 'Describe this page for search engines...', 160);
    mkRow('dash-keyword', 'Focus Keyword', 'text', item.focus_keyword || '', 'Primary keyword from the URL');

    // Keyword hint
    const kwHint = document.createElement('div');
    kwHint.style.cssText = 'font-size:11px;color:var(--text-muted);margin:-8px 0 12px 0;';
    kwHint.textContent = 'Tip: The focus keyword should match the URL slug and appear in the title and description.';
    card.appendChild(kwHint);

    // AI buttons row
    if (typeof LiqAI !== 'undefined') {
        var _aiRow = document.createElement('div');
        _aiRow.style.cssText = 'display:flex;gap:8px;margin:12px 0;';
        var _aiMetaBtn = LiqAI.button({
            label: 'AI Generate Meta',
            feature: 'seo_ai_meta',
            credits: 2,
            tier: 'free',
            getInput: function() {
                var title = document.getElementById('dash-title') ? document.getElementById('dash-title').value : '';
                var keyword = document.getElementById('dash-keyword') ? document.getElementById('dash-keyword').value : '';
                var slug = item.slug || '';
                return 'Page: ' + (item.page_title || slug) + '\nCurrent title: ' + title + '\nFocus keyword: ' + keyword + '\nSlug: /' + slug;
            },
            onResult: function(result) {
                if (result && result.title) {
                    var titleEl = document.getElementById('dash-title');
                    var descEl = document.getElementById('dash-desc');
                    if (titleEl && result.title) { titleEl.value = result.title; if (titleEl.oninput) titleEl.oninput(); }
                    if (descEl && result.description) { descEl.value = result.description; if (descEl.oninput) descEl.oninput(); }
                    showToast('SEO meta generated by AI', 'success');
                }
            },
        });
        if (_aiMetaBtn) _aiRow.appendChild(_aiMetaBtn);

        var _aiSchemaBtn = LiqAI.button({
            label: 'AI Generate Schema',
            feature: 'seo_ai_schema',
            credits: 3,
            tier: 'free',
            getInput: function() {
                var title = document.getElementById('dash-title') ? document.getElementById('dash-title').value : '';
                var desc = document.getElementById('dash-desc') ? document.getElementById('dash-desc').value : '';
                return 'Page: ' + (item.page_title || item.slug) + '\nType: ' + item.content_type + '\nSEO Title: ' + title + '\nDescription: ' + desc;
            },
            onResult: function(result) {
                if (result) {
                    showToast('Schema.org JSON-LD generated', 'success');
                }
            },
        });
        if (_aiSchemaBtn) _aiRow.appendChild(_aiSchemaBtn);
        card.appendChild(_aiRow);
    }

    const btns = document.createElement('div');
    btns.style.cssText = 'margin-top:16px;display:flex;gap:8px;';
    const saveBtn = document.createElement('button');
    saveBtn.className = 'btn btn-primary';
    saveBtn.textContent = 'Save Changes';
    saveBtn.onclick = async () => {
        const newSlug = document.getElementById('dash-slug').value.trim().replace(/^\//, '').replace(/\/$/, '');
        const title = document.getElementById('dash-title').value.trim();
        const desc = document.getElementById('dash-desc').value.trim();
        const kw = document.getElementById('dash-keyword').value.trim();

        if (!title && !desc) { showToast('Enter at least a title or description', 'error'); return; }

        // If slug changed, use the import endpoint for rename + meta update
        if (newSlug && newSlug !== item.slug) {
            const r = await fetch('/api/modules/seo/import', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ items: [{ slug: item.slug, new_slug: newSlug, title: title, description: desc, focus_keyword: kw }]}),
            }).then(r => r.json());
            showToast(r.ok ? 'Saved! Slug renamed with 301 redirect.' : r.message, r.ok ? 'success' : 'error');
            if (r.ok) load_seo_dashboard();
        } else {
            // Just update SEO meta via PUT
            const body = { title: title, description: desc, focus_keyword: kw, og_image: '', canonical_url: '', robots: '', schema_json: '' };
            const r = await fetch('/api/modules/seo/meta/' + encodeURIComponent(item.content_id), {
                method: 'PUT', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify(body),
            }).then(r => r.json());
            showToast(r.ok ? 'SEO meta saved!' : r.message, r.ok ? 'success' : 'error');
            if (r.ok) load_seo_dashboard();
        }
    };
    btns.appendChild(saveBtn);
    const cancelBtn = document.createElement('button');
    cancelBtn.className = 'btn btn-ghost';
    cancelBtn.textContent = 'Cancel';
    cancelBtn.onclick = () => load_seo_dashboard();
    btns.appendChild(cancelBtn);
    card.appendChild(btns);

    el.appendChild(card);
    main.replaceChildren(el);
}

// ── SEO Meta view (Page Editor — single-page deep edit) ────────────
var _metaSortCol = 0;
var _metaSortAsc = true;
var _metaPages = [];
var _metaSeoMap = {};

async function load_seo_meta() {
    const main = document.getElementById('adminMain');
    const el = document.createElement('div');

    var _role = (window.__CMS && window.__CMS.nexusRole) || '';
    var _isPro = _role === 'central' || _role === 'professional' || _role === 'enterprise';
    var _isStarter = _isPro || _role === 'starter';

    const toolbar = document.createElement('div');
    toolbar.className = 'toolbar';
    const h = document.createElement('h2');
    h.textContent = 'SEO Meta';
    if (!_isStarter) {
        var _tierBadge = document.createElement('span');
        _tierBadge.className = 'status-badge status-published';
        _tierBadge.style.cssText = 'margin-left:8px;background:#f59e0b;color:#000;font-size:11px;';
        _tierBadge.textContent = 'STARTER';
        h.appendChild(_tierBadge);
    }
    toolbar.appendChild(h);
    el.appendChild(toolbar);

    // Single fetch from export endpoint (has all pages + SEO data)
    const exportR = await fetch('/api/modules/seo/export').then(r => r.json()).catch(() => ({data:[]}));
    const allItems = exportR.data || [];

    _metaPages = allItems;
    _metaSeoMap = {};
    allItems.forEach(function(p) { _metaSeoMap[p.content_id] = p; });

    const withSeo = allItems.filter(function(p) { return p.seo_title; }).length;
    const avgScore = allItems.length > 0 ? Math.round(allItems.reduce(function(a, p) { return a + (p.seo_score || 0); }, 0) / allItems.length) : 0;

    const statsDiv = document.createElement('div');
    statsDiv.className = 'stats';
    [
        ['Pages', allItems.length, false],
        ['With SEO', withSeo, false],
        ['Missing', Math.max(0, allItems.length - withSeo), true],
        ['Avg Score', avgScore, true],
    ].forEach(function(item) {
        const card = document.createElement('div');
        card.className = 'stat-card';
        const l = document.createElement('div'); l.className = 'label'; l.textContent = item[0]; card.appendChild(l);
        const v = document.createElement('div'); v.className = 'value' + (item[2] ? ' accent' : ''); v.textContent = item[1]; card.appendChild(v);
        statsDiv.appendChild(card);
    });
    el.appendChild(statsDiv);

    lqAddExportImportBar(el, function(format) {
        if (format === 'json') {
            lqExportJSON(allItems, 'seo-meta.json');
        } else {
            lqExportCSV(allItems, ['content_id','page_title','seo_title','seo_score','og_image','has_schema'], 'seo-meta.csv');
        }
    }, function() {
        lqImportJSON(async function(data) {
            var arr = Array.isArray(data) ? data : [data];
            var ok = 0;
            for (var i = 0; i < arr.length; i++) {
                try {
                    await fetch('/api/modules/seo/meta/' + encodeURIComponent(arr[i].content_id), { method: 'PUT', headers: {'Content-Type':'application/json'}, body: JSON.stringify(arr[i]) });
                    ok++;
                } catch(e) {}
            }
            showToast('Imported ' + ok + ' of ' + arr.length + ' SEO entries', 'success');
            load_seo_meta();
        });
    });

    var tableContainer = document.createElement('div');
    tableContainer.id = 'seo-meta-table';
    el.appendChild(tableContainer);
    renderMetaTable(tableContainer);

    main.replaceChildren(el);
}

function renderMetaTable(container) {
    container.replaceChildren();
    var sorted = _metaPages.slice().sort(function(a, b) {
        var fields = ['page_title', 'seo_title', 'seo_score'];
        var f = fields[_metaSortCol] || 'page_title';
        var av = a[f] != null ? a[f] : '';
        var bv = b[f] != null ? b[f] : '';
        if (typeof av === 'number' && typeof bv === 'number') {
            return _metaSortAsc ? av - bv : bv - av;
        }
        return _metaSortAsc ? String(av).localeCompare(String(bv)) : String(bv).localeCompare(String(av));
    });

    var table = document.createElement('div');
    table.className = 'content-table';
    var t = document.createElement('table');
    var hdr = document.createElement('tr');
    var colNames = ['Page','SEO Title','Score','OG','Schema',''];
    colNames.forEach(function(col, i) {
        var th = document.createElement('th');
        if (i < 3) {
            th.style.cssText = 'cursor:pointer;user-select:none;';
            th.textContent = col + (_metaSortCol === i ? (_metaSortAsc ? ' ▲' : ' ▼') : '');
            th.onclick = function() {
                if (_metaSortCol === i) { _metaSortAsc = !_metaSortAsc; }
                else { _metaSortCol = i; _metaSortAsc = true; }
                var c = document.getElementById('seo-meta-table');
                if (c) renderMetaTable(c);
            };
        } else {
            th.textContent = col;
        }
        hdr.appendChild(th);
    });
    t.appendChild(hdr);

    if (sorted.length === 0) {
        var tr = document.createElement('tr');
        var td = document.createElement('td'); td.colSpan = 6;
        td.style.cssText = 'text-align:center;padding:20px;color:var(--text-muted);';
        td.textContent = 'No published pages found';
        tr.appendChild(td); t.appendChild(tr);
    } else {
        sorted.forEach(function(p) {
            var tr = document.createElement('tr');

            var titleTd = document.createElement('td');
            var strong = document.createElement('strong'); strong.textContent = p.page_title || '(untitled)'; titleTd.appendChild(strong);
            var br = document.createElement('br'); titleTd.appendChild(br);
            var slug = document.createElement('span'); slug.style.cssText = 'color:var(--text-muted);font-size:12px;'; slug.textContent = '/' + (p.slug || ''); titleTd.appendChild(slug);
            tr.appendChild(titleTd);

            var seoTitleTd = document.createElement('td');
            seoTitleTd.textContent = p.seo_title || '-';
            if (!p.seo_title) seoTitleTd.style.color = 'var(--text-muted)';
            tr.appendChild(seoTitleTd);

            var scoreTd = document.createElement('td');
            if (p.seo_score > 0) { scoreTd.appendChild(seoScoreBadge(p.seo_score)); }
            else { scoreTd.textContent = '-'; scoreTd.style.color = 'var(--text-muted)'; }
            tr.appendChild(scoreTd);

            var ogTd = document.createElement('td');
            ogTd.textContent = p.og_image ? 'Yes' : '-';
            ogTd.style.color = p.og_image ? 'var(--success)' : 'var(--text-muted)';
            ogTd.style.fontSize = '13px';
            tr.appendChild(ogTd);

            var schemaTd = document.createElement('td');
            schemaTd.textContent = p.has_schema ? 'Yes' : '-';
            schemaTd.style.color = p.has_schema ? 'var(--success)' : 'var(--text-muted)';
            schemaTd.style.fontSize = '13px';
            tr.appendChild(schemaTd);

            var actionTd = document.createElement('td');
            var editBtn = document.createElement('button');
            editBtn.className = 'btn btn-ghost btn-sm';
            editBtn.textContent = 'Edit';
            editBtn.onclick = function() { openSeoEditor(p.content_id, p.page_title || '', {
                title: p.seo_title, description: p.seo_description, focus_keyword: p.focus_keyword,
                og_image: p.og_image || '', robots: p.robots || '', schema_json: '', seo_score: p.seo_score,
                slug: p.slug, content_type: p.content_type
            }); };
            actionTd.appendChild(editBtn);
            tr.appendChild(actionTd);

            t.appendChild(tr);
        });
    }

    table.appendChild(t);
    container.appendChild(table);
}

var _seoEditorExportData = null; // cached export data for keyword checks

function openSeoEditor(contentId, pageTitle, existing) {
    const main = document.getElementById('adminMain');
    const el = document.createElement('div');

    const backBtn = document.createElement('button');
    backBtn.className = 'btn btn-ghost btn-sm';
    backBtn.textContent = '\u2190 Back';
    backBtn.style.marginBottom = '16px';
    backBtn.onclick = () => load_seo_meta();
    el.appendChild(backBtn);

    const h = document.createElement('h2');
    h.textContent = 'Edit SEO: ' + (pageTitle || '');
    el.appendChild(h);

    // ── Top info bar: score + slug + page link ──
    var topBar = document.createElement('div');
    topBar.style.cssText = 'display:flex;align-items:center;gap:12px;margin-bottom:16px;flex-wrap:wrap;';
    if (existing && existing.seo_score !== undefined) {
        topBar.appendChild(seoScoreBadge(existing.seo_score));
    }
    if (existing && existing.slug) {
        var slugLink = document.createElement('a');
        slugLink.href = '/' + existing.slug;
        slugLink.target = '_blank';
        slugLink.style.cssText = 'font-family:monospace;font-size:12px;color:var(--accent);text-decoration:none;';
        slugLink.textContent = '/' + existing.slug + ' \u2197';
        topBar.appendChild(slugLink);
    }
    if (existing && existing.slug) {
        topBar.appendChild(seoChangeUrlButton(contentId, existing.slug));
    }
    el.appendChild(topBar);

    // ── Google data section (if available) ──
    var googleSection = document.createElement('div');
    googleSection.id = 'seoEditorGoogleData';
    googleSection.style.cssText = 'margin-bottom:20px;';
    el.appendChild(googleSection);

    // Fetch Google data for this page
    (async function() {
        try {
            var slug = existing ? existing.slug : '';
            var gscRes = await fetch('/api/modules/seo/google/gsc/pages').then(function(r) { return r.json(); });
            if (gscRes.ok && gscRes.data) {
                var pageData = null;
                gscRes.data.forEach(function(g) {
                    var s = g.page.replace(/^https?:\/\/[^\/]+\//, '').replace(/\/$/, '') || 'home';
                    if (s === slug) pageData = g;
                });
                if (pageData) {
                    var gBox = document.createElement('div');
                    gBox.style.cssText = 'background:var(--surface);border:1px solid var(--border);border-radius:8px;padding:16px;display:flex;gap:20px;flex-wrap:wrap;';
                    var gTitle = document.createElement('div');
                    gTitle.style.cssText = 'font-weight:600;font-size:13px;width:100%;margin-bottom:4px;color:var(--text-muted);';
                    gTitle.textContent = 'Google Search Console (last 28 days)';
                    gBox.appendChild(gTitle);
                    [
                        { label: 'Clicks', value: pageData.clicks || 0 },
                        { label: 'Impressions', value: pageData.impressions || 0 },
                        { label: 'CTR', value: ((parseFloat(pageData.ctr || 0)) * 100).toFixed(1) + '%' },
                        { label: 'Avg Position', value: parseFloat(pageData.position || 0).toFixed(1) },
                    ].forEach(function(s) {
                        var item = document.createElement('div');
                        item.style.cssText = 'text-align:center;min-width:80px;';
                        var val = document.createElement('div');
                        val.style.cssText = 'font-size:18px;font-weight:700;';
                        val.textContent = s.value;
                        item.appendChild(val);
                        var lbl = document.createElement('div');
                        lbl.style.cssText = 'font-size:11px;color:var(--text-muted);';
                        lbl.textContent = s.label;
                        item.appendChild(lbl);
                        gBox.appendChild(item);
                    });
                    googleSection.appendChild(gBox);
                }
            }
        } catch(e) {}
    })();

    // ── Two-column layout: form left, info right ──
    var layout = document.createElement('div');
    layout.style.cssText = 'display:grid;grid-template-columns:1fr 320px;gap:20px;';

    const card = document.createElement('div');
    card.style.cssText = 'background:var(--surface);border:1px solid var(--border);border-radius:8px;padding:20px;';

    function addField(id, label, type, value, placeholder, charLimit) {
        const row = document.createElement('div'); row.style.marginBottom = '12px';
        const lbl = document.createElement('label');
        lbl.style.cssText = 'font-size:12px;color:var(--text-muted);display:block;margin-bottom:4px;';
        lbl.textContent = label; row.appendChild(lbl);
        let input;
        if (type === 'textarea') {
            input = document.createElement('textarea'); input.rows = 3; input.className = 'admin-input';
        } else if (type === 'select') {
            input = document.createElement('select'); input.className = 'admin-input';
            ['', 'index,follow', 'noindex,follow', 'index,nofollow', 'noindex,nofollow'].forEach(opt => {
                const o = document.createElement('option'); o.value = opt; o.textContent = opt || '(default)'; input.appendChild(o);
            });
        } else {
            input = document.createElement('input'); input.type = 'text'; input.className = 'admin-input';
        }
        input.id = id; input.value = value || ''; if (placeholder) input.placeholder = placeholder;
        row.appendChild(input);
        if (charLimit) {
            const counter = document.createElement('div');
            counter.style.cssText = 'font-size:11px;color:var(--text-muted);margin-top:2px;';
            counter.textContent = (input.value.length || 0) + '/' + charLimit + ' characters';
            input.oninput = () => { counter.textContent = input.value.length + '/' + charLimit + ' characters'; };
            row.appendChild(counter);
        }
        card.appendChild(row);
        return input;
    }

    addField('seo-title', 'SEO Title', 'text', existing?.title || existing?.seo_title, pageTitle, 60);
    if (_seoIsPro) {
        var aiTitleRow = document.createElement('div'); aiTitleRow.style.cssText = 'margin:-8px 0 12px 0;display:flex;gap:6px;align-items:center;';
        var aiTitleFeedback = document.createElement('div'); aiTitleFeedback.style.cssText = 'flex:1;';
        aiTitleRow.appendChild(seoAiButton('AI Generate Title', 1, function() {
            seoAiWithFeedback({ contentId: contentId, field: 'title', fieldId: 'seo-title', container: aiTitleFeedback, label: 'Title' });
        }));
        aiTitleRow.appendChild(aiTitleFeedback);
        card.appendChild(aiTitleRow);
    }
    if (_seoIsPro) {
        var abTitleRow = document.createElement('div'); abTitleRow.style.cssText = 'margin:-4px 0 8px 0;';
        abTitleRow.appendChild(seoAbTestButton(contentId, 'Title', existing?.title || ''));
        card.appendChild(abTitleRow);
    }

    addField('seo-desc', 'Meta Description', 'textarea', existing?.description || existing?.seo_description, 'Describe this page for search engines...', 160);
    if (_seoIsPro) {
        var aiDescRow = document.createElement('div'); aiDescRow.style.cssText = 'margin:-8px 0 12px 0;display:flex;gap:6px;align-items:center;';
        var aiDescFeedback = document.createElement('div'); aiDescFeedback.style.cssText = 'flex:1;';
        aiDescRow.appendChild(seoAiButton('AI Generate Description', 1, function() {
            seoAiWithFeedback({ contentId: contentId, field: 'description', fieldId: 'seo-desc', container: aiDescFeedback, label: 'Description' });
        }));
        aiDescRow.appendChild(aiDescFeedback);
        card.appendChild(aiDescRow);
    }
    if (_seoIsPro) {
        var abDescRow = document.createElement('div'); abDescRow.style.cssText = 'margin:-4px 0 8px 0;';
        abDescRow.appendChild(seoAbTestButton(contentId, 'Description', existing?.description || ''));
        card.appendChild(abDescRow);
    }

    var kwInput = addField('seo-keyword', 'Focus Keyword', 'text', existing?.focus_keyword, 'Primary keyword for scoring');
    // Keyword cannibalization check
    var kwWarnDiv = document.createElement('div');
    kwWarnDiv.id = 'seo-kw-cannibalization';
    kwWarnDiv.style.cssText = 'font-size:12px;margin:-8px 0 12px 0;';
    card.appendChild(kwWarnDiv);

    // Check keyword against other pages on blur
    kwInput.onblur = async function() {
        var kw = kwInput.value.trim().toLowerCase();
        if (!kw) { kwWarnDiv.textContent = ''; return; }
        try {
            if (!_seoEditorExportData) {
                var r = await fetch('/api/modules/seo/export').then(function(r) { return r.json(); });
                if (r.ok) _seoEditorExportData = r.data || [];
            }
            var matches = (_seoEditorExportData || []).filter(function(p) {
                return p.content_id !== contentId && p.focus_keyword && p.focus_keyword.toLowerCase() === kw;
            });
            if (matches.length > 0) {
                kwWarnDiv.style.color = '#f59e0b';
                kwWarnDiv.textContent = '\u26A0 Keyword "' + kw + '" is also used by: ' + matches.map(function(m) { return '/' + m.slug; }).join(', ') + ' — consider using a unique keyword to avoid cannibalization.';
            } else {
                kwWarnDiv.style.color = '#22c55e';
                kwWarnDiv.textContent = '\u2713 Keyword is unique across your site.';
            }
        } catch(e) { kwWarnDiv.textContent = ''; }
    };

    if (_seoIsPro) {
        var aiKwRow = document.createElement('div'); aiKwRow.style.cssText = 'margin:-8px 0 12px 0;display:flex;gap:6px;align-items:center;';
        var aiKwFeedback = document.createElement('div'); aiKwFeedback.style.cssText = 'flex:1;';
        aiKwRow.appendChild(seoAiButton('AI Suggest Keywords', 1, function() {
            seoAiWithFeedback({ contentId: contentId, field: 'keywords', fieldId: 'seo-keyword', container: aiKwFeedback, label: 'Keywords' });
        }));
        aiKwRow.appendChild(aiKwFeedback);
        card.appendChild(aiKwRow);
    }
    if (_seoIsPro) {
        var abKwRow = document.createElement('div'); abKwRow.style.cssText = 'margin:-4px 0 8px 0;';
        abKwRow.appendChild(seoAbTestButton(contentId, 'FocusKeyword', existing?.focus_keyword || ''));
        card.appendChild(abKwRow);
    }

    addField('seo-og-image', 'OG Image URL', 'text', existing?.og_image, 'https://...');
    addField('seo-canonical', 'Canonical URL', 'text', existing?.canonical_url, 'https://...');
    addField('seo-robots', 'Robots', 'select', existing?.robots);

    addField('seo-schema', 'Schema JSON-LD', 'textarea', existing?.schema_json, '{"@context":"https://schema.org",...}');
    if (_seoIsPro) {
        var aiSchemaRow = document.createElement('div'); aiSchemaRow.style.cssText = 'margin:-8px 0 12px 0;display:flex;gap:6px;align-items:center;';
        var aiSchemaFeedback = document.createElement('div'); aiSchemaFeedback.style.cssText = 'flex:1;';
        aiSchemaRow.appendChild(seoAiButton('AI Generate Schema', 2, function() {
            seoAiWithFeedback({ contentId: contentId, field: 'schema', fieldId: 'seo-schema', container: aiSchemaFeedback, label: 'Schema' });
        }));
        aiSchemaRow.appendChild(aiSchemaFeedback);
        card.appendChild(aiSchemaRow);
    }

    // SERP Preview (Pro)
    renderSerpPreview(card, 'seo-title', 'seo-desc', existing?.canonical_url || contentId);

    const btns = document.createElement('div');
    btns.style.cssText = 'margin-top:16px;display:flex;gap:8px;';
    const saveBtn = document.createElement('button');
    saveBtn.className = 'btn btn-primary';
    saveBtn.textContent = 'Save SEO Meta';
    saveBtn.onclick = async () => {
        const body = {
            title: document.getElementById('seo-title').value.trim(),
            description: document.getElementById('seo-desc').value.trim(),
            focus_keyword: document.getElementById('seo-keyword').value.trim(),
            og_image: document.getElementById('seo-og-image').value.trim(),
            canonical_url: document.getElementById('seo-canonical').value.trim(),
            robots: document.getElementById('seo-robots').value,
            schema_json: document.getElementById('seo-schema').value.trim(),
        };
        if (!body.title && !body.description) { showToast('Enter at least a title or description', 'error'); return; }
        const r = await fetch('/api/modules/seo/meta/' + encodeURIComponent(contentId), {
            method: 'PUT', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify(body),
        }).then(r => r.json());
        showToast(r.ok ? 'SEO meta saved!' : r.message, r.ok ? 'success' : 'error');
        _seoEditorExportData = null; // invalidate cache
        if (r.ok) load_seo_meta();
    };
    btns.appendChild(saveBtn);
    const cancelBtn = document.createElement('button');
    cancelBtn.className = 'btn btn-ghost';
    cancelBtn.textContent = 'Cancel';
    cancelBtn.onclick = () => load_seo_meta();
    btns.appendChild(cancelBtn);
    card.appendChild(btns);

    layout.appendChild(card);

    // ── Right sidebar: page info + tips ──
    var sidebar = document.createElement('div');

    // Page info card
    var infoCard = document.createElement('div');
    infoCard.style.cssText = 'background:var(--surface);border:1px solid var(--border);border-radius:8px;padding:16px;margin-bottom:16px;';
    var infoTitle = document.createElement('div');
    infoTitle.style.cssText = 'font-weight:600;margin-bottom:8px;font-size:13px;';
    infoTitle.textContent = 'Page Info';
    infoCard.appendChild(infoTitle);
    [
        { label: 'Content ID', value: contentId },
        { label: 'Type', value: existing?.content_type || 'page' },
        { label: 'Slug', value: '/' + (existing?.slug || '') },
    ].forEach(function(info) {
        var row = document.createElement('div');
        row.style.cssText = 'display:flex;justify-content:space-between;font-size:12px;padding:4px 0;border-bottom:1px solid var(--border);';
        var lbl = document.createElement('span');
        lbl.style.color = 'var(--text-muted)';
        lbl.textContent = info.label;
        row.appendChild(lbl);
        var val = document.createElement('span');
        val.style.cssText = 'font-family:monospace;max-width:180px;overflow:hidden;text-overflow:ellipsis;white-space:nowrap;';
        val.textContent = info.value;
        val.title = info.value;
        row.appendChild(val);
        infoCard.appendChild(row);
    });
    // View page link
    if (existing && existing.slug) {
        var viewLink = document.createElement('a');
        viewLink.href = '/' + existing.slug;
        viewLink.target = '_blank';
        viewLink.style.cssText = 'display:block;margin-top:8px;font-size:12px;color:var(--accent);text-decoration:none;';
        viewLink.textContent = 'View Page \u2192';
        infoCard.appendChild(viewLink);
        // Edit page link
        var editLink = document.createElement('a');
        editLink.href = '/admin#pages:edit:' + contentId;
        editLink.style.cssText = 'display:block;margin-top:4px;font-size:12px;color:var(--accent);text-decoration:none;';
        editLink.textContent = 'Edit Page Content \u2192';
        editLink.onclick = function(e) { e.preventDefault(); navigateTo('pages:edit:' + contentId); };
        infoCard.appendChild(editLink);
    }
    sidebar.appendChild(infoCard);

    // SEO checklist card
    var checkCard = document.createElement('div');
    checkCard.style.cssText = 'background:var(--surface);border:1px solid var(--border);border-radius:8px;padding:16px;';
    var checkTitle = document.createElement('div');
    checkTitle.style.cssText = 'font-weight:600;margin-bottom:8px;font-size:13px;';
    checkTitle.textContent = 'SEO Checklist';
    checkCard.appendChild(checkTitle);
    var checks = [
        { ok: !!(existing?.seo_title || existing?.title), label: 'SEO title set', field: 'title' },
        { ok: !!(existing?.seo_description || existing?.description), label: 'Meta description set', field: 'description' },
        { ok: !!existing?.focus_keyword, label: 'Focus keyword set', field: 'keyword' },
        { ok: !!existing?.og_image, label: 'OG image set', field: 'og_image' },
        { ok: !!existing?.has_schema || !!existing?.schema_json, label: 'Schema JSON-LD', field: 'schema' },
        { ok: !!existing?.robots, label: 'Robots directive set', field: 'robots' },
    ];
    checks.forEach(function(c) {
        var row = document.createElement('div');
        row.style.cssText = 'display:flex;align-items:center;gap:6px;font-size:12px;padding:3px 0;';
        var icon = document.createElement('span');
        icon.textContent = c.ok ? '\u2713' : '\u2717';
        icon.style.color = c.ok ? '#22c55e' : '#ef4444';
        icon.style.fontWeight = '600';
        row.appendChild(icon);
        var txt = document.createElement('span');
        txt.textContent = c.label;
        txt.style.flex = '1';
        if (!c.ok) txt.style.color = 'var(--text-muted)';
        row.appendChild(txt);
        // AI fix button for missing items (Pro only)
        if (!c.ok && _seoIsPro && (c.field === 'title' || c.field === 'description' || c.field === 'keyword' || c.field === 'schema')) {
            var fixCost = c.field === 'schema' ? 2 : 1;
            var fixBtn = document.createElement('button');
            fixBtn.className = 'btn btn-ghost';
            fixBtn.style.cssText = 'font-size:10px;padding:1px 6px;color:var(--accent);';
            fixBtn.textContent = 'AI Fix (' + fixCost + ' cr)';
            fixBtn.onclick = (function(field) { return async function(e) {
                e.stopPropagation();
                this.disabled = true; this.textContent = '...';
                try {
                    if (field === 'title') {
                        var v = await seoAiTitle(contentId);
                        if (v) document.getElementById('seo-title').value = v;
                    } else if (field === 'description') {
                        var v = await seoAiDescription(contentId);
                        if (v) document.getElementById('seo-desc').value = v;
                    } else if (field === 'keyword') {
                        var kws = await seoAiKeywords(contentId);
                        if (kws && kws.length > 0) document.getElementById('seo-keyword').value = kws[0];
                    } else if (field === 'schema') {
                        var v = await seoAiSchema(contentId);
                        if (v) document.getElementById('seo-schema').value = v;
                    }
                    this.textContent = '\u2713';
                    this.style.color = '#22c55e';
                } catch(err) { this.textContent = 'Err'; this.style.color = '#ef4444'; }
            }; })(c.field);
            row.appendChild(fixBtn);
        }
        if (!c.ok && c.field === 'robots') {
            var setBtn = document.createElement('button');
            setBtn.className = 'btn btn-ghost';
            setBtn.style.cssText = 'font-size:10px;padding:1px 6px;color:var(--accent);';
            setBtn.textContent = 'Set';
            setBtn.onclick = function(e) {
                e.stopPropagation();
                document.getElementById('seo-robots').value = 'index,follow';
                this.textContent = '\u2713';
                this.style.color = '#22c55e';
            };
            row.appendChild(setBtn);
        }
        checkCard.appendChild(row);
    });
    // AI Fix All button
    if (_seoIsPro) {
        var fixAllMissing = checks.filter(function(c) { return !c.ok && (c.field === 'title' || c.field === 'description' || c.field === 'keyword' || c.field === 'schema'); });
        if (fixAllMissing.length > 0) {
            var totalCost = fixAllMissing.reduce(function(sum, c) { return sum + (c.field === 'schema' ? 2 : 1); }, 0);
            var fixAllBtn = document.createElement('button');
            fixAllBtn.className = 'btn btn-primary btn-sm';
            fixAllBtn.style.cssText = 'margin-top:10px;width:100%;font-size:12px;';
            fixAllBtn.textContent = 'AI Fix All Missing (' + fixAllMissing.length + ' fields, ' + totalCost + ' credits)';
            fixAllBtn.onclick = async function() {
                fixAllBtn.disabled = true; fixAllBtn.textContent = 'Generating... (' + totalCost + ' credits)';
                var prevValues = {};
                try {
                    // Capture before state
                    fixAllMissing.forEach(function(c) {
                        var fid = c.field === 'title' ? 'seo-title' : c.field === 'description' ? 'seo-desc' : c.field === 'keyword' ? 'seo-keyword' : 'seo-schema';
                        var el = document.getElementById(fid);
                        if (el) prevValues[c.field] = el.value;
                    });

                    var results = {};
                    var promises = [];
                    fixAllMissing.forEach(function(c) {
                        if (c.field === 'title') promises.push(seoAiTitle(contentId).then(function(v) { results.title = v; if (v) document.getElementById('seo-title').value = v; }));
                        if (c.field === 'description') promises.push(seoAiDescription(contentId).then(function(v) { results.description = v; if (v) document.getElementById('seo-desc').value = v; }));
                        if (c.field === 'keyword') promises.push(seoAiKeywords(contentId).then(function(kws) { results.keyword = kws && kws.length > 0 ? kws[0] : null; if (results.keyword) document.getElementById('seo-keyword').value = results.keyword; }));
                        if (c.field === 'schema') promises.push(seoAiSchema(contentId).then(function(v) { results.schema = v; if (v) document.getElementById('seo-schema').value = v; }));
                    });
                    await Promise.all(promises);

                    // Show before/after summary
                    fixAllBtn.remove();
                    var summaryDiv = document.createElement('div');
                    summaryDiv.style.cssText = 'margin-top:10px;background:var(--surface2,#1e293b);border:1px solid var(--border);border-radius:8px;padding:10px;font-size:12px;';
                    var summaryTitle = document.createElement('div');
                    summaryTitle.style.cssText = 'font-size:11px;font-weight:600;color:var(--text-muted);margin-bottom:6px;';
                    summaryTitle.textContent = 'AI GENERATED (' + totalCost + ' credits used)';
                    summaryDiv.appendChild(summaryTitle);

                    fixAllMissing.forEach(function(c) {
                        var newVal = results[c.field];
                        if (!newVal) return;
                        var row = document.createElement('div');
                        row.style.cssText = 'margin-bottom:6px;';
                        var fieldLabel = document.createElement('span');
                        fieldLabel.style.cssText = 'font-weight:600;color:var(--text-muted);';
                        fieldLabel.textContent = c.label.split(' ')[0] + ': ';
                        row.appendChild(fieldLabel);
                        var valSpan = document.createElement('span');
                        valSpan.style.color = '#22c55e';
                        valSpan.textContent = seoTruncate(newVal, 80);
                        row.appendChild(valSpan);
                        summaryDiv.appendChild(row);
                    });

                    // Feedback bar for Fix All
                    var fixAllFeedbackDiv = document.createElement('div');
                    fixAllFeedbackDiv.style.cssText = 'margin-top:8px;';
                    summaryDiv.appendChild(fixAllFeedbackDiv);

                    if (window.LiqAI && window.LiqAI.renderFeedbackBar) {
                        var interactionId = 'seo_fixall_' + Date.now();
                        window.LiqAI.renderFeedbackBar(fixAllFeedbackDiv, interactionId, {
                            feature: 'seo_fix_all',
                            userInput: 'content_id:' + contentId,
                            container: fixAllFeedbackDiv,
                            onKeep: function() {
                                toast('All fields accepted \u2014 click Save to keep them.');
                                summaryDiv.remove();
                            },
                            onRetry: function() {
                                // Revert all fields
                                fixAllMissing.forEach(function(c) {
                                    var fid = c.field === 'title' ? 'seo-title' : c.field === 'description' ? 'seo-desc' : c.field === 'keyword' ? 'seo-keyword' : 'seo-schema';
                                    var el = document.getElementById(fid);
                                    if (el && prevValues[c.field] !== undefined) el.value = prevValues[c.field];
                                });
                                summaryDiv.remove();
                            },
                            onError: function() {
                                fixAllMissing.forEach(function(c) {
                                    var fid = c.field === 'title' ? 'seo-title' : c.field === 'description' ? 'seo-desc' : c.field === 'keyword' ? 'seo-keyword' : 'seo-schema';
                                    var el = document.getElementById(fid);
                                    if (el && prevValues[c.field] !== undefined) el.value = prevValues[c.field];
                                });
                            }
                        }, 0);
                    } else {
                        var bar = document.createElement('div');
                        bar.className = 'ai-feedback-bar';
                        var keepBtn = document.createElement('button');
                        keepBtn.className = 'btn btn-sm btn-success';
                        keepBtn.textContent = 'Keep All';
                        keepBtn.onclick = function() { toast('Accepted \u2014 click Save'); summaryDiv.remove(); };
                        bar.appendChild(keepBtn);
                        var revertBtn = document.createElement('button');
                        revertBtn.className = 'btn btn-sm btn-ghost';
                        revertBtn.textContent = 'Revert All';
                        revertBtn.onclick = function() {
                            fixAllMissing.forEach(function(c) {
                                var fid = c.field === 'title' ? 'seo-title' : c.field === 'description' ? 'seo-desc' : c.field === 'keyword' ? 'seo-keyword' : 'seo-schema';
                                var el = document.getElementById(fid);
                                if (el && prevValues[c.field] !== undefined) el.value = prevValues[c.field];
                            });
                            summaryDiv.remove();
                        };
                        bar.appendChild(revertBtn);
                        fixAllFeedbackDiv.appendChild(bar);
                    }
                    checkCard.appendChild(summaryDiv);
                } catch(e) { fixAllBtn.textContent = 'Error: ' + e.message; fixAllBtn.style.color = '#ef4444'; }
            };
            checkCard.appendChild(fixAllBtn);
        }
    }
    sidebar.appendChild(checkCard);

    // ── Keyword Consistency Checklist ──
    var kwCheckContainer = document.createElement('div');
    kwCheckContainer.style.cssText = 'background:var(--surface);border:1px solid var(--border);border-radius:8px;padding:16px;margin-top:16px;';
    var kwTitle = document.createElement('strong');
    kwTitle.style.fontSize = '13px';
    kwTitle.textContent = 'Keyword Checklist';
    kwCheckContainer.appendChild(kwTitle);
    var kwBody = document.createElement('div');
    kwBody.style.marginTop = '8px';
    kwCheckContainer.appendChild(kwBody);
    sidebar.appendChild(kwCheckContainer);
    seoKeywordChecklist(contentId, kwBody);

    layout.appendChild(sidebar);
    el.appendChild(layout);
    main.replaceChildren(el);
}

// ── SEO Site Health view ───────────────────────────────────────────
async function load_seo_health() {
    var main = document.getElementById('adminMain');
    var el = document.createElement('div');
    var headerRow = document.createElement('div'); headerRow.style.cssText = 'display:flex;align-items:center;gap:12px;margin-bottom:4px;';
    var h = document.createElement('h2'); h.textContent = 'SEO Site Health'; h.style.margin = '0';
    headerRow.appendChild(h);
    var scanBtn = document.createElement('button'); scanBtn.className = 'btn btn-primary btn-sm'; scanBtn.textContent = 'Run Fresh Scan';
    scanBtn.onclick = async function() {
        scanBtn.disabled = true; scanBtn.textContent = 'Scanning...';
        var spinner = document.createElement('span');
        spinner.style.cssText = 'display:inline-block;width:14px;height:14px;border:2px solid var(--text-muted);border-top-color:var(--accent);border-radius:50%;animation:spin 0.6s linear infinite;margin-left:6px;vertical-align:middle;';
        scanBtn.appendChild(spinner);
        // Add spin keyframe if not present
        if (!document.getElementById('seo-spin-style')) {
            var style = document.createElement('style'); style.id = 'seo-spin-style';
            style.textContent = '@keyframes spin{to{transform:rotate(360deg)}}';
            document.head.appendChild(style);
        }
        await new Promise(function(r) { setTimeout(r, 500); }); // brief delay so user sees scan happening
        load_seo_health();
    };
    headerRow.appendChild(scanBtn);
    el.appendChild(headerRow);

    // Last scan time
    var scanTime = document.createElement('div');
    scanTime.style.cssText = 'font-size:11px;color:var(--text-muted);margin-bottom:16px;';
    scanTime.textContent = 'Last scanned: ' + new Date().toLocaleString();
    el.appendChild(scanTime);

    try {
        var r = await fetch('/api/modules/seo/health').then(function(r) { return r.json(); });
        var d = r.data || {};

        var statsDiv = document.createElement('div'); statsDiv.className = 'stats';
        [
            ['Avg Score', d.average_score || 0, true],
            ['Pages', d.total_pages || 0, false],
            ['With Meta', d.pages_with_meta || 0, false],
            ['No Meta', d.pages_without_meta || 0, true],
        ].forEach(function(item) {
            var card = document.createElement('div'); card.className = 'stat-card';
            var l = document.createElement('div'); l.className = 'label'; l.textContent = item[0]; card.appendChild(l);
            var v = document.createElement('div'); v.className = 'value' + (item[2] ? ' accent' : ''); v.textContent = item[1]; card.appendChild(v);
            statsDiv.appendChild(card);
        });
        el.appendChild(statsDiv);

        var issues = [
            ['Missing Title', d.missing_title || 0, 'missing'],
            ['Missing Description', d.missing_description || 0, 'missing-desc'],
            ['Missing OG Image', d.missing_og_image || 0, 'no-og'],
            ['Missing Schema', d.missing_schema || 0, 'no-schema'],
            ['Missing Robots', d.missing_robots || 0, 'missing-robots'],
        ];

        var wrapper = document.createElement('div'); wrapper.className = 'content-table'; wrapper.style.maxWidth = '800px';
        var t = document.createElement('table');
        var hdr = document.createElement('tr');
        ['Issue', 'Count', 'Action', ''].forEach(function(col) { var th = document.createElement('th'); th.textContent = col; hdr.appendChild(th); });
        t.appendChild(hdr);

        // Results area for AI Fix
        var aiResultsArea = document.createElement('div');
        aiResultsArea.id = 'health-ai-results';
        aiResultsArea.style.cssText = 'margin-top:16px;';

        issues.forEach(function(item) {
            var label = item[0], count = item[1], filter = item[2];
            var tr = document.createElement('tr');

            if (count > 0) {
                tr.style.cursor = 'pointer';
                tr.onmouseenter = function() { tr.style.background = 'rgba(59,130,246,0.08)'; };
                tr.onmouseleave = function() { tr.style.background = ''; };
                tr.onclick = function() {
                    window._seoDashFilter = filter;
                    navigateTo('seo-dashboard');
                };
            }

            var tdL = document.createElement('td'); tdL.textContent = label; tr.appendChild(tdL);
            var tdC = document.createElement('td');
            if (count > 0) { tdC.appendChild(seoScoreBadge(0)); tdC.lastChild.textContent = count; }
            else { var ok = document.createElement('span'); ok.className = 'status-badge status-published'; ok.textContent = '0'; tdC.appendChild(ok); }
            tr.appendChild(tdC);

            // AI Fix button (Pro only)
            var tdA = document.createElement('td');
            if (count > 0) {
                var aiBtn = document.createElement('button');
                aiBtn.className = 'btn btn-sm ' + (_seoIsPro ? 'btn-primary' : 'btn-ghost');
                aiBtn.textContent = 'AI Fix';
                if (!_seoIsPro) {
                    aiBtn.disabled = true;
                    aiBtn.appendChild(document.createTextNode(' '));
                    aiBtn.appendChild(seoProBadge());
                } else {
                    aiBtn.onclick = async function(ev) {
                        ev.stopPropagation();
                        aiBtn.disabled = true; aiBtn.textContent = 'Fixing...';
                        var progressSpan = document.createElement('span');
                        progressSpan.style.cssText = 'font-size:11px;color:var(--text-muted);margin-left:8px;';
                        aiBtn.parentNode.appendChild(progressSpan);
                        try {
                            var exportR = await fetch('/api/modules/seo/export').then(function(r) { return r.json(); });
                            var pages = exportR.data || [];
                            var affected = pages.filter(function(p) {
                                if (filter === 'missing') return !p.seo_title;
                                if (filter === 'missing-desc') return !p.seo_description;
                                if (filter === 'no-og') return !p.og_image;
                                if (filter === 'no-schema') return !p.has_schema;
                                if (filter === 'missing-robots') return !p.robots;
                                return false;
                            }).slice(0, 10);
                            if (affected.length === 0) { toast('No affected pages found'); aiBtn.disabled = false; aiBtn.textContent = 'AI Fix'; return; }
                            progressSpan.textContent = '0/' + affected.length + ' pages...';
                            // Capture before state
                            var beforeState = {};
                            affected.forEach(function(p) {
                                beforeState[p.content_id] = { slug: p.slug, title: p.seo_title || '', description: p.seo_description || '', keyword: p.focus_keyword || '', score: p.seo_score || 0 };
                            });
                            var ids = affected.map(function(p) { return p.content_id; });
                            var br = await fetch('/api/modules/seo/ai/bulk', {
                                method: 'POST', headers: {'Content-Type':'application/json'},
                                body: JSON.stringify({content_ids: ids})
                            }).then(function(r) { return r.json(); });
                            progressSpan.textContent = '';
                            aiBtn.textContent = br.ok ? 'Done!' : 'Error';
                            aiBtn.style.background = br.ok ? 'rgba(34,197,94,0.15)' : '';
                            aiBtn.style.color = br.ok ? '#22c55e' : '';
                            // Display before/after results
                            var rArea = document.getElementById('health-ai-results');
                            while (rArea.firstChild) rArea.removeChild(rArea.firstChild);
                            var rTitle = document.createElement('h3');
                            rTitle.textContent = 'AI Fix Results: ' + label;
                            rTitle.style.marginBottom = '8px';
                            rArea.appendChild(rTitle);
                            var aiResults = br.data ? br.data.results || [] : [];
                            var aiErrors = br.data ? br.data.errors || [] : [];
                            if (aiResults.length > 0) {
                                aiResults.forEach(function(res) {
                                    var before = beforeState[res.content_id] || {};
                                    var card = document.createElement('div');
                                    card.style.cssText = 'padding:12px;margin-bottom:8px;background:var(--surface);border:1px solid var(--border);border-radius:8px;';
                                    // Page slug header
                                    var slugHeader = document.createElement('div');
                                    slugHeader.style.cssText = 'font-family:monospace;font-size:12px;color:var(--accent);margin-bottom:8px;';
                                    slugHeader.textContent = '/' + (before.slug || res.content_id);
                                    card.appendChild(slugHeader);
                                    // Before/After grid
                                    var grid = document.createElement('div');
                                    grid.style.cssText = 'display:grid;grid-template-columns:1fr 1fr;gap:8px;font-size:12px;';
                                    function addComparison(fieldLabel, oldVal, newVal) {
                                        var bCell = document.createElement('div');
                                        bCell.style.cssText = 'padding:6px;background:rgba(239,68,68,0.05);border-radius:4px;';
                                        var bLbl = document.createElement('div'); bLbl.style.cssText = 'font-size:10px;color:var(--text-muted);margin-bottom:2px;'; bLbl.textContent = 'Before: ' + fieldLabel;
                                        bCell.appendChild(bLbl);
                                        var bVal = document.createElement('div'); bVal.textContent = oldVal || '(empty)';
                                        if (!oldVal) bVal.style.color = 'var(--text-muted)';
                                        bCell.appendChild(bVal);
                                        grid.appendChild(bCell);
                                        var aCell = document.createElement('div');
                                        aCell.style.cssText = 'padding:6px;background:rgba(34,197,94,0.05);border-radius:4px;';
                                        var aLbl = document.createElement('div'); aLbl.style.cssText = 'font-size:10px;color:var(--text-muted);margin-bottom:2px;'; aLbl.textContent = 'After: ' + fieldLabel;
                                        aCell.appendChild(aLbl);
                                        var aVal = document.createElement('div'); aVal.style.cssText = 'color:#22c55e;'; aVal.textContent = newVal || '(empty)';
                                        aCell.appendChild(aVal);
                                        grid.appendChild(aCell);
                                    }
                                    addComparison('Title', before.title, res.title);
                                    addComparison('Description', before.description, res.description);
                                    addComparison('Keyword', before.keyword, res.focus_keyword);
                                    card.appendChild(grid);
                                    // Score
                                    var scoreRow = document.createElement('div');
                                    scoreRow.style.cssText = 'margin-top:8px;display:flex;align-items:center;gap:8px;font-size:12px;';
                                    scoreRow.appendChild(document.createTextNode('Score: '));
                                    if (before.score) { scoreRow.appendChild(seoScoreBadge(before.score)); scoreRow.appendChild(document.createTextNode(' \u2192 ')); }
                                    scoreRow.appendChild(seoScoreBadge(res.score));
                                    card.appendChild(scoreRow);
                                    rArea.appendChild(card);
                                });
                            }
                            if (aiErrors.length > 0) {
                                aiErrors.forEach(function(err) {
                                    var p = document.createElement('p'); p.style.cssText = 'font-size:12px;color:var(--danger);margin:2px 0;'; p.textContent = err; rArea.appendChild(p);
                                });
                            }
                        } catch(e) { toast('Error: ' + e.message); aiBtn.disabled = false; aiBtn.textContent = 'AI Fix'; }
                    };
                }
                tdA.appendChild(aiBtn);
            }
            tr.appendChild(tdA);

            // View affected pages button
            var tdV = document.createElement('td');
            if (count > 0) {
                var viewBtn = document.createElement('button');
                viewBtn.className = 'btn btn-ghost btn-sm';
                viewBtn.textContent = 'View Pages';
                viewBtn.onclick = function(ev) {
                    ev.stopPropagation();
                    window._seoDashFilter = filter;
                    navigateTo('seo-dashboard');
                };
                tdV.appendChild(viewBtn);
            }
            tr.appendChild(tdV);
            t.appendChild(tr);
        });
        wrapper.appendChild(t); el.appendChild(wrapper);
        el.appendChild(aiResultsArea);
    } catch(e) {
        var p = document.createElement('p'); p.style.color = 'var(--danger)'; p.textContent = 'Error: ' + e.message; el.appendChild(p);
    }

    main.replaceChildren(el);
}

// ── Redirect Manager view (Pro) ───────────────────────────────────
var _redirSortCol = 0;
var _redirSortAsc = true;
var _redirItems = [];

async function load_seo_redirects() {
    var __role = (window.__CMS && window.__CMS.nexusRole) || '';
    var __isStarter = __role === 'central' || __role === 'professional' || __role === 'enterprise' || __role === 'starter';
    var main = document.getElementById('adminMain');
    var el = document.createElement('div');

    var h = document.createElement('h2');
    h.textContent = 'Redirect Manager';
    if (!_seoIsPro) { h.appendChild(document.createTextNode(' ')); h.appendChild(seoProBadge()); }
    el.appendChild(h);

    if (!_seoIsPro) {
        seoUpgradeCta(el);
        main.replaceChildren(el);
        return;
    }

    // Create redirect form
    var form = document.createElement('div');
    form.style.cssText = 'display:flex;gap:8px;align-items:end;flex-wrap:wrap;margin-bottom:20px;padding:16px;background:var(--surface);border:1px solid var(--border);border-radius:8px;';

    function mkInput(id, label, placeholder, width) {
        var w = document.createElement('div');
        var lbl = document.createElement('label'); lbl.style.cssText = 'font-size:12px;color:var(--text-muted);display:block;margin-bottom:4px;'; lbl.textContent = label; w.appendChild(lbl);
        var input = document.createElement('input'); input.type = 'text'; input.className = 'admin-input'; input.id = id; input.placeholder = placeholder; input.style.width = width || '200px'; w.appendChild(input);
        return w;
    }

    form.appendChild(mkInput('redir-source', 'Source Path', '/old-page', '200px'));
    form.appendChild(mkInput('redir-target', 'Target URL', '/new-page', '200px'));

    var typeW = document.createElement('div');
    var typeLbl = document.createElement('label'); typeLbl.style.cssText = 'font-size:12px;color:var(--text-muted);display:block;margin-bottom:4px;'; typeLbl.textContent = 'Status'; typeW.appendChild(typeLbl);
    var typeSel = document.createElement('select'); typeSel.className = 'admin-input'; typeSel.id = 'redir-status';
    [['301', '301 Permanent'], ['302', '302 Temporary']].forEach(function(o) {
        var opt = document.createElement('option'); opt.value = o[0]; opt.textContent = o[1]; typeSel.appendChild(opt);
    });
    typeW.appendChild(typeSel);
    form.appendChild(typeW);

    var addBtn = document.createElement('button'); addBtn.className = 'btn btn-primary btn-sm'; addBtn.textContent = 'Add Redirect';
    addBtn.style.marginBottom = '2px';
    addBtn.onclick = async function() {
        var source = document.getElementById('redir-source').value.trim();
        var target = document.getElementById('redir-target').value.trim();
        var status = parseInt(document.getElementById('redir-status').value);
        if (!source || !target) { toast('Source and target are required'); return; }
        var r = await fetch('/api/modules/seo/redirects', {
            method: 'POST', headers: {'Content-Type':'application/json'},
            body: JSON.stringify({source: source, target: target, status_code: status, match_type: 'exact'})
        }).then(function(r) { return r.json(); });
        toast(r.ok ? 'Redirect created' : 'Error: ' + r.message);
        if (r.ok) load_seo_redirects();
    };
    form.appendChild(addBtn);
    el.appendChild(form);

    // Export/Import bar
    lqAddExportImportBar(el, function(format) {
        if (format === 'json') {
            lqExportJSON(_redirItems, 'seo-redirects.json');
        } else {
            lqExportCSV(_redirItems, ['redirect_id','source','target','status_code','is_active','hit_count'], 'seo-redirects.csv');
        }
    }, function() {
        lqImportJSON(async function(data) {
            var arr = Array.isArray(data) ? data : [data];
            var ok = 0;
            for (var i = 0; i < arr.length; i++) {
                try {
                    await fetch('/api/modules/seo/redirects', {
                        method: 'POST', headers: {'Content-Type':'application/json'},
                        body: JSON.stringify(arr[i])
                    });
                    ok++;
                } catch(e) {}
            }
            toast('Imported ' + ok + ' of ' + arr.length + ' redirects');
            load_seo_redirects();
        });
    });

    // Redirects table
    try {
        var r = await fetch('/api/modules/seo/redirects').then(function(r) { return r.json(); });
        _redirItems = r.data || [];

        var statsDiv = document.createElement('div'); statsDiv.className = 'stats';
        [['Total Redirects', _redirItems.length, false], ['Active', _redirItems.filter(function(x){return x.is_active;}).length, false]].forEach(function(s) {
            var card = document.createElement('div'); card.className = 'stat-card';
            var l = document.createElement('div'); l.className = 'label'; l.textContent = s[0]; card.appendChild(l);
            var v = document.createElement('div'); v.className = 'value' + (s[2] ? ' accent' : ''); v.textContent = s[1]; card.appendChild(v);
            statsDiv.appendChild(card);
        });
        el.appendChild(statsDiv);

        var tableContainer = document.createElement('div');
        tableContainer.id = 'redir-table-container';
        el.appendChild(tableContainer);
        renderRedirectTable(tableContainer);

    } catch (e) {
        var p = document.createElement('p'); p.style.color = 'var(--danger)'; p.textContent = 'Error: ' + e.message; el.appendChild(p);
    }

    main.replaceChildren(el);
}

function renderRedirectTable(container) {
    container.replaceChildren();
    var sorted = _redirItems.slice().sort(function(a, b) {
        var fields = ['source', 'target', 'status_code', 'hit_count', 'is_active'];
        var f = fields[_redirSortCol] || 'source';
        var av = a[f] != null ? a[f] : '';
        var bv = b[f] != null ? b[f] : '';
        if (typeof av === 'boolean') { av = av ? 1 : 0; bv = bv ? 1 : 0; }
        if (typeof av === 'number' && typeof bv === 'number') {
            return _redirSortAsc ? av - bv : bv - av;
        }
        return _redirSortAsc ? String(av).localeCompare(String(bv)) : String(bv).localeCompare(String(av));
    });

    var wrapper = document.createElement('div'); wrapper.className = 'content-table';
    var t = document.createElement('table');
    var hdr = document.createElement('tr');
    var colNames = ['Source', 'Target', 'Status', 'Hits', 'Active', ''];
    colNames.forEach(function(col, i) {
        var th = document.createElement('th');
        if (i < 5) {
            th.style.cssText = 'cursor:pointer;user-select:none;';
            var sortIdx = [0, 1, 2, 3, 4][i];
            th.textContent = col + (_redirSortCol === sortIdx ? (_redirSortAsc ? ' ▲' : ' ▼') : '');
            th.onclick = function() {
                if (_redirSortCol === sortIdx) { _redirSortAsc = !_redirSortAsc; }
                else { _redirSortCol = sortIdx; _redirSortAsc = true; }
                var c = document.getElementById('redir-table-container');
                if (c) renderRedirectTable(c);
            };
        } else {
            th.textContent = col;
        }
        hdr.appendChild(th);
    });
    t.appendChild(hdr);

    if (sorted.length === 0) {
        var tr = document.createElement('tr');
        var td = document.createElement('td'); td.colSpan = 6;
        td.style.cssText = 'text-align:center;padding:20px;color:var(--text-muted);';
        td.textContent = 'No redirects configured yet.';
        tr.appendChild(td); t.appendChild(tr);
    }

    sorted.forEach(function(item) {
        var tr = document.createElement('tr');

        // Source — color-coded
        var srcTd = document.createElement('td');
        srcTd.style.cssText = 'font-family:monospace;font-size:12px;color:' + (item.is_active ? 'var(--success)' : 'var(--text-muted)') + ';';
        srcTd.textContent = item.source;
        tr.appendChild(srcTd);

        // Target — styled as link
        var tgtTd = document.createElement('td');
        tgtTd.style.cssText = 'font-family:monospace;font-size:12px;color:var(--accent);';
        tgtTd.textContent = item.target;
        tr.appendChild(tgtTd);

        var stTd = document.createElement('td'); stTd.textContent = item.status_code; tr.appendChild(stTd);
        var hitTd = document.createElement('td'); hitTd.textContent = item.hit_count || 0; tr.appendChild(hitTd);

        // Toggle active
        var actTd = document.createElement('td');
        var toggleBtn = document.createElement('button');
        toggleBtn.className = 'btn btn-sm ' + (item.is_active ? 'btn-primary' : 'btn-ghost');
        toggleBtn.textContent = item.is_active ? 'Active' : 'Inactive';
        toggleBtn.onclick = async function() {
            toggleBtn.disabled = true;
            var r = await fetch('/api/modules/seo/redirects/' + encodeURIComponent(item.redirect_id), {
                method: 'PUT', headers: {'Content-Type':'application/json'},
                body: JSON.stringify({is_active: !item.is_active})
            }).then(function(r) { return r.json(); });
            toast(r.ok ? 'Redirect toggled' : r.message);
            if (r.ok) load_seo_redirects();
        };
        actTd.appendChild(toggleBtn);
        tr.appendChild(actTd);

        // Edit + Delete
        var actionsTd = document.createElement('td');
        actionsTd.style.whiteSpace = 'nowrap';
        var editBtn = document.createElement('button');
        editBtn.className = 'btn btn-sm btn-ghost';
        editBtn.textContent = 'Edit';
        editBtn.onclick = function(ev) {
            ev.stopPropagation();
            openRedirectEditor(item);
        };
        actionsTd.appendChild(editBtn);
        var delBtn = document.createElement('button');
        delBtn.className = 'btn btn-sm btn-danger';
        delBtn.style.marginLeft = '4px';
        delBtn.textContent = 'Delete';
        delBtn.onclick = async function() {
            if (!confirm('Delete redirect ' + item.source + '?')) return;
            delBtn.disabled = true;
            var r = await fetch('/api/modules/seo/redirects/' + encodeURIComponent(item.redirect_id), {
                method: 'DELETE'
            }).then(function(r) { return r.json(); });
            toast(r.ok ? 'Deleted' : r.message);
            if (r.ok) load_seo_redirects();
        };
        actionsTd.appendChild(delBtn);
        tr.appendChild(actionsTd);

        t.appendChild(tr);
    });

    wrapper.appendChild(t); container.appendChild(wrapper);
}

function openRedirectEditor(item) {
    openDrillDownModal(500, 'Edit Redirect', function(body) {
        function mkField(label, id, value, placeholder) {
            var w = document.createElement('div'); w.style.marginBottom = '12px';
            var lbl = document.createElement('label'); lbl.style.cssText = 'font-size:12px;color:var(--text-muted);display:block;margin-bottom:4px;'; lbl.textContent = label; w.appendChild(lbl);
            var input = document.createElement('input'); input.type = 'text'; input.className = 'admin-input'; input.id = id; input.value = value || ''; input.placeholder = placeholder || ''; w.appendChild(input);
            return w;
        }
        body.appendChild(mkField('Source Path', 'edit-redir-source', item.source, '/old-page'));
        body.appendChild(mkField('Target URL', 'edit-redir-target', item.target, '/new-page'));
        var statusW = document.createElement('div'); statusW.style.marginBottom = '12px';
        var statusLbl = document.createElement('label'); statusLbl.style.cssText = 'font-size:12px;color:var(--text-muted);display:block;margin-bottom:4px;'; statusLbl.textContent = 'Status Code'; statusW.appendChild(statusLbl);
        var statusSel = document.createElement('select'); statusSel.className = 'admin-input'; statusSel.id = 'edit-redir-status';
        [['301','301 Permanent'],['302','302 Temporary']].forEach(function(o) {
            var opt = document.createElement('option'); opt.value = o[0]; opt.textContent = o[1];
            if (String(item.status_code) === o[0]) opt.selected = true;
            statusSel.appendChild(opt);
        });
        statusW.appendChild(statusSel); body.appendChild(statusW);
        var btns = document.createElement('div'); btns.style.cssText = 'display:flex;gap:8px;margin-top:16px;';
        var saveBtn = document.createElement('button'); saveBtn.className = 'btn btn-primary'; saveBtn.textContent = 'Save Changes';
        saveBtn.onclick = async function() {
            saveBtn.disabled = true; saveBtn.textContent = 'Saving...';
            var payload = {
                source: document.getElementById('edit-redir-source').value.trim(),
                target: document.getElementById('edit-redir-target').value.trim(),
                status_code: parseInt(document.getElementById('edit-redir-status').value)
            };
            if (!payload.source || !payload.target) { toast('Source and target are required'); saveBtn.disabled = false; saveBtn.textContent = 'Save Changes'; return; }
            var r = await fetch('/api/modules/seo/redirects/' + encodeURIComponent(item.redirect_id), {
                method: 'PUT', headers: {'Content-Type':'application/json'}, body: JSON.stringify(payload)
            }).then(function(r) { return r.json(); });
            toast(r.ok ? 'Redirect updated' : r.message);
            if (r.ok) load_seo_redirects();
        };
        btns.appendChild(saveBtn);
        body.appendChild(btns);
    });
}

// ── Bulk Optimizer view (Pro) ──────────────────────────────────────
var _bulkSortCol = 4; // default sort by score
var _bulkSortAsc = true; // ascending (worst first)
var _bulkPages = [];

async function load_seo_bulk() {
    var __role = (window.__CMS && window.__CMS.nexusRole) || '';
    var __isStarter = __role === 'central' || __role === 'professional' || __role === 'enterprise' || __role === 'starter';
    var main = document.getElementById('adminMain');
    var el = document.createElement('div');

    var h = document.createElement('h2');
    h.textContent = 'AI Bulk Optimizer';
    if (!_seoIsPro) { h.appendChild(document.createTextNode(' ')); h.appendChild(seoProBadge()); }
    el.appendChild(h);

    if (!_seoIsPro) {
        seoUpgradeCta(el);
        main.replaceChildren(el);
        return;
    }

    var desc = document.createElement('p');
    desc.style.cssText = 'color:var(--text-muted);font-size:13px;margin-bottom:16px;';
    desc.textContent = 'Select up to 10 pages to auto-generate SEO titles, descriptions, and keywords using AI. Costs 1 credit per page.';
    el.appendChild(desc);

    // Load pages from export
    var r = await fetch('/api/modules/seo/export').then(function(r) { return r.json(); }).catch(function() { return {data:[]}; });
    _bulkPages = r.data || [];

    // Filter bar
    var filterBar = document.createElement('div');
    filterBar.style.cssText = 'display:flex;gap:8px;margin-bottom:12px;align-items:center;';
    var selAll = document.createElement('input'); selAll.type = 'checkbox'; selAll.id = 'bulk-sel-all';
    selAll.onchange = function() {
        var boxes = document.querySelectorAll('.bulk-page-cb');
        var checked = 0;
        boxes.forEach(function(cb) {
            if (selAll.checked && checked < 10) { cb.checked = true; checked++; }
            else cb.checked = false;
        });
    };
    filterBar.appendChild(selAll);
    var selLabel = document.createElement('label'); selLabel.htmlFor = 'bulk-sel-all';
    selLabel.style.cssText = 'font-size:12px;color:var(--text-muted);';
    selLabel.textContent = 'Select All (max 10)';
    filterBar.appendChild(selLabel);

    var onlyMissing = document.createElement('button'); onlyMissing.className = 'btn btn-ghost btn-sm';
    onlyMissing.textContent = 'Show Missing Meta Only';
    onlyMissing.onclick = function() {
        var rows = document.querySelectorAll('.bulk-page-row');
        rows.forEach(function(row) {
            var hasMeta = row.dataset.hasMeta === 'true';
            row.style.display = hasMeta ? 'none' : '';
        });
    };
    filterBar.appendChild(onlyMissing);

    var showAll = document.createElement('button'); showAll.className = 'btn btn-ghost btn-sm';
    showAll.textContent = 'Show All';
    showAll.onclick = function() {
        document.querySelectorAll('.bulk-page-row').forEach(function(row) { row.style.display = ''; });
    };
    filterBar.appendChild(showAll);
    el.appendChild(filterBar);

    // Pages checklist with sorting
    var tableContainer = document.createElement('div');
    tableContainer.id = 'bulk-table-container';
    el.appendChild(tableContainer);
    renderBulkTable(tableContainer);

    // Optimize button + progress (after table)
    var actionBar = document.createElement('div'); actionBar.style.cssText = 'margin-top:16px;display:flex;gap:12px;align-items:center;';
    var optimizeBtn = document.createElement('button'); optimizeBtn.className = 'btn btn-primary';
    optimizeBtn.textContent = 'Optimize Selected';
    var progressDiv = document.createElement('div'); progressDiv.id = 'bulk-progress';
    progressDiv.style.cssText = 'font-size:13px;color:var(--text-muted);';
    actionBar.appendChild(optimizeBtn);
    actionBar.appendChild(progressDiv);
    el.appendChild(actionBar);

    // Results container
    var resultsDiv = document.createElement('div'); resultsDiv.id = 'bulk-results'; resultsDiv.style.marginTop = '16px';
    el.appendChild(resultsDiv);

    optimizeBtn.onclick = async function() {
        var selected = [];
        document.querySelectorAll('.bulk-page-cb:checked').forEach(function(cb) { selected.push(cb.value); });
        if (selected.length === 0) { toast('Select at least one page'); return; }
        if (selected.length > 10) { toast('Maximum 10 pages at a time'); return; }

        // Capture before state
        var beforeState = {};
        _bulkPages.forEach(function(p) {
            if (selected.indexOf(p.content_id) !== -1) {
                beforeState[p.content_id] = {
                    slug: p.slug, page_title: p.page_title,
                    seo_title: p.seo_title || '', seo_description: p.seo_description || '',
                    focus_keyword: p.focus_keyword || '', seo_score: p.seo_score || 0
                };
            }
        });

        optimizeBtn.disabled = true; optimizeBtn.textContent = 'Optimizing...';
        progressDiv.textContent = 'Processing ' + selected.length + ' pages with AI...';

        try {
            var r = await fetch('/api/modules/seo/ai/bulk', {
                method: 'POST', headers: {'Content-Type':'application/json'},
                body: JSON.stringify({content_ids: selected})
            }).then(function(r) { return r.json(); });

            progressDiv.textContent = r.ok ? r.message : 'Error: ' + r.message;

            // Show results with before/after comparison
            var results = r.data ? r.data.results || [] : [];
            var errors = r.data ? r.data.errors || [] : [];
            var rDiv = document.getElementById('bulk-results');
            while (rDiv.firstChild) rDiv.removeChild(rDiv.firstChild);

            if (results.length > 0) {
                var rh = document.createElement('h3'); rh.textContent = 'Results — Before vs After'; rh.style.marginBottom = '12px'; rDiv.appendChild(rh);
                results.forEach(function(res) {
                    var before = beforeState[res.content_id] || {};
                    var card = document.createElement('div');
                    card.style.cssText = 'padding:16px;margin-bottom:12px;background:var(--surface);border:1px solid var(--border);border-radius:8px;';
                    // Slug header
                    var slugHeader = document.createElement('div');
                    slugHeader.style.cssText = 'font-family:monospace;font-size:12px;color:var(--accent);margin-bottom:4px;';
                    slugHeader.textContent = '/' + (before.slug || '');
                    card.appendChild(slugHeader);
                    var pageTitle = document.createElement('div');
                    pageTitle.style.cssText = 'font-weight:600;margin-bottom:10px;';
                    pageTitle.textContent = before.page_title || '';
                    card.appendChild(pageTitle);
                    // Comparison table
                    var cTable = document.createElement('table');
                    cTable.style.cssText = 'width:100%;font-size:12px;border-collapse:collapse;';
                    var cHead = document.createElement('tr');
                    ['Field', 'Before', 'After'].forEach(function(col) {
                        var th = document.createElement('th');
                        th.style.cssText = 'text-align:left;padding:4px 8px;border-bottom:1px solid var(--border);font-size:11px;color:var(--text-muted);';
                        th.textContent = col;
                        cHead.appendChild(th);
                    });
                    cTable.appendChild(cHead);
                    function addRow(field, oldVal, newVal) {
                        var tr = document.createElement('tr');
                        var changed = oldVal !== newVal;
                        var tdF = document.createElement('td'); tdF.style.cssText = 'padding:4px 8px;font-weight:500;white-space:nowrap;'; tdF.textContent = field; tr.appendChild(tdF);
                        var tdO = document.createElement('td'); tdO.style.cssText = 'padding:4px 8px;max-width:250px;overflow:hidden;text-overflow:ellipsis;white-space:nowrap;';
                        tdO.textContent = oldVal || '(empty)';
                        if (!oldVal) tdO.style.color = 'var(--text-muted)';
                        if (changed) tdO.style.cssText += 'text-decoration:line-through;opacity:0.6;';
                        tr.appendChild(tdO);
                        var tdN = document.createElement('td'); tdN.style.cssText = 'padding:4px 8px;max-width:250px;overflow:hidden;text-overflow:ellipsis;white-space:nowrap;';
                        tdN.textContent = newVal || '(empty)';
                        if (changed) tdN.style.color = '#22c55e';
                        tr.appendChild(tdN);
                        cTable.appendChild(tr);
                    }
                    addRow('Title', before.seo_title, res.title);
                    addRow('Description', before.seo_description, res.description);
                    addRow('Keyword', before.focus_keyword, res.focus_keyword);
                    card.appendChild(cTable);
                    // Score comparison
                    var scoreRow = document.createElement('div');
                    scoreRow.style.cssText = 'margin-top:8px;display:flex;align-items:center;gap:8px;font-size:12px;';
                    scoreRow.appendChild(document.createTextNode('Score: '));
                    scoreRow.appendChild(seoScoreBadge(before.seo_score || 0));
                    scoreRow.appendChild(document.createTextNode(' \u2192 '));
                    scoreRow.appendChild(seoScoreBadge(res.score));
                    card.appendChild(scoreRow);
                    rDiv.appendChild(card);
                });
            }
            if (errors.length > 0) {
                var eh = document.createElement('h3'); eh.textContent = 'Errors'; eh.style.cssText = 'margin:12px 0 8px 0;color:var(--danger);'; rDiv.appendChild(eh);
                errors.forEach(function(err) {
                    var p = document.createElement('p'); p.style.cssText = 'font-size:12px;color:var(--danger);margin:2px 0;'; p.textContent = err; rDiv.appendChild(p);
                });
            }
        } catch(e) {
            progressDiv.textContent = 'Error: ' + e.message;
        }
        optimizeBtn.disabled = false; optimizeBtn.textContent = 'Optimize Selected';
    };

    main.replaceChildren(el);
}

function renderBulkTable(container) {
    container.replaceChildren();
    var sorted = _bulkPages.slice().sort(function(a, b) {
        var fields = [null, 'slug', 'page_title', 'seo_title', 'seo_score'];
        var f = fields[_bulkSortCol];
        if (!f) return 0;
        var av = a[f] != null ? a[f] : '';
        var bv = b[f] != null ? b[f] : '';
        if (typeof av === 'number' && typeof bv === 'number') {
            return _bulkSortAsc ? av - bv : bv - av;
        }
        return _bulkSortAsc ? String(av).localeCompare(String(bv)) : String(bv).localeCompare(String(av));
    });

    var tableWrapper = document.createElement('div'); tableWrapper.className = 'content-table'; tableWrapper.style.cssText = 'max-height:400px;overflow-y:auto;';
    var t = document.createElement('table');
    var hdr = document.createElement('tr');
    ['', 'Slug', 'Page Title', 'SEO Title', 'Score', ''].forEach(function(col, i) {
        var th = document.createElement('th');
        if (i >= 1 && i <= 4) {
            th.style.cssText = 'cursor:pointer;user-select:none;';
            th.textContent = col + (_bulkSortCol === i ? (_bulkSortAsc ? ' ▲' : ' ▼') : '');
            th.onclick = function() {
                if (_bulkSortCol === i) { _bulkSortAsc = !_bulkSortAsc; }
                else { _bulkSortCol = i; _bulkSortAsc = i === 4; }
                var c = document.getElementById('bulk-table-container');
                if (c) renderBulkTable(c);
            };
        } else {
            th.textContent = col;
        }
        hdr.appendChild(th);
    });
    t.appendChild(hdr);

    sorted.forEach(function(page) {
        var tr = document.createElement('tr'); tr.className = 'bulk-page-row';
        tr.dataset.hasMeta = !!(page.seo_title) + '';
        var cbTd = document.createElement('td');
        var cb = document.createElement('input'); cb.type = 'checkbox'; cb.className = 'bulk-page-cb'; cb.value = page.content_id;
        cbTd.appendChild(cb); tr.appendChild(cbTd);
        var slugTd = document.createElement('td'); slugTd.style.cssText = 'font-family:monospace;font-size:12px;'; slugTd.textContent = '/' + page.slug; tr.appendChild(slugTd);
        var ptTd = document.createElement('td'); ptTd.style.fontSize = '13px'; ptTd.textContent = seoTruncate(page.page_title, 35); tr.appendChild(ptTd);
        var stTd = document.createElement('td'); stTd.style.fontSize = '13px';
        stTd.textContent = page.seo_title ? seoTruncate(page.seo_title, 30) : '(none)';
        if (!page.seo_title) stTd.style.color = 'var(--danger)';
        tr.appendChild(stTd);
        var scTd = document.createElement('td');
        if (page.seo_score > 0) scTd.appendChild(seoScoreBadge(page.seo_score));
        else { scTd.textContent = '-'; scTd.style.color = 'var(--text-muted)'; }
        tr.appendChild(scTd);

        // Edit button
        var editTd = document.createElement('td');
        var editBtn = document.createElement('button');
        editBtn.className = 'btn btn-ghost btn-sm';
        editBtn.textContent = 'Edit';
        editBtn.onclick = function() {
            openDashEditor({
                content_id: page.content_id, slug: page.slug, page_title: page.page_title,
                seo_title: page.seo_title, seo_description: page.seo_description,
                focus_keyword: page.focus_keyword, seo_score: page.seo_score
            });
        };
        editTd.appendChild(editBtn);
        tr.appendChild(editTd);

        t.appendChild(tr);
    });
    tableWrapper.appendChild(t); container.appendChild(tableWrapper);
}

// ── Sitemap Manager ──────────────────────────────────────────────
// Exclusion logic mirrors the server-side sitemap handler
var _sitemapExcludePrefixes = ['demo-', 'example-', 'partner-'];
var _sitemapExcludeExact = ['checkout', 'register', 'customer-login', 'my-account'];
function _sitemapIsExcluded(slug) {
    if (_sitemapExcludeExact.indexOf(slug) !== -1) return true;
    for (var i = 0; i < _sitemapExcludePrefixes.length; i++) {
        if (slug.indexOf(_sitemapExcludePrefixes[i]) === 0) return true;
    }
    return false;
}
var _smSort = 'slug';
var _smSortDir = 1;
var _smFilter = 'all';
var _smSearch = '';
var _smGoogleData = {};

async function load_seo_sitemap() {
    var container = document.getElementById('adminMain');
    while (container.firstChild) container.removeChild(container.firstChild);

    var heading = document.createElement('h2');
    heading.textContent = 'Sitemap Manager';
    container.appendChild(heading);

    var desc = document.createElement('p');
    desc.style.cssText = 'color:var(--text-muted);margin-bottom:20px;';
    desc.textContent = 'View and manage which pages appear in your XML sitemap. Click any page to edit its SEO fields.';
    container.appendChild(desc);

    // Filter bar
    var filterBar = document.createElement('div');
    filterBar.style.cssText = 'display:flex;gap:8px;flex-wrap:wrap;align-items:center;margin-bottom:16px;';
    var filters = [
        { id: 'all', label: 'All' },
        { id: 'in-sitemap', label: 'In Sitemap' },
        { id: 'excluded', label: 'Excluded' },
        { id: 'missing-meta', label: 'Missing SEO' },
        { id: 'low-score', label: 'Score < 40' },
        { id: 'no-keyword', label: 'No Keyword' },
    ];
    filters.forEach(function(f) {
        var btn = document.createElement('button');
        btn.className = 'btn btn-sm ' + (_smFilter === f.id ? 'btn-primary' : 'btn-ghost');
        btn.textContent = f.label;
        btn.onclick = function() { _smFilter = f.id; load_seo_sitemap(); };
        filterBar.appendChild(btn);
    });
    // Search
    var searchInput = document.createElement('input');
    searchInput.type = 'text';
    searchInput.placeholder = 'Search pages...';
    searchInput.value = _smSearch;
    searchInput.style.cssText = 'margin-left:auto;padding:6px 12px;background:var(--bg);border:1px solid var(--border);border-radius:6px;color:var(--text);font-size:13px;width:200px;';
    var searchTimer;
    searchInput.oninput = function() { clearTimeout(searchTimer); searchTimer = setTimeout(function() { _smSearch = searchInput.value.trim().toLowerCase(); load_seo_sitemap(); }, 300); };
    filterBar.appendChild(searchInput);
    container.appendChild(filterBar);

    // Fetch data
    try {
        var exportRes = await fetch('/api/modules/seo/export').then(function(r) { return r.json(); });
        if (!exportRes.ok) { container.appendChild(document.createTextNode('Failed to load data')); return; }

        // Fetch Google data
        try {
            var gscRes = await fetch('/api/modules/seo/google/gsc/pages').then(function(r) { return r.json(); });
            if (gscRes.ok && gscRes.data) {
                _smGoogleData = {};
                gscRes.data.forEach(function(g) {
                    var s = g.page.replace(/^https?:\/\/[^\/]+\//, '').replace(/\/$/, '') || 'home';
                    _smGoogleData[s] = g;
                });
            }
        } catch(e) {}

        var items = exportRes.data || [];

        // Enrich with sitemap status
        items.forEach(function(item) {
            var isStaticRoute =
                item.content_type === 'static_page' &&
                ['home', 'services', 'book-online', 'financing', 'customer-portal', 'service-areas', 'equipment', 'products'].indexOf(item.slug) !== -1;
            if (item.content_type === 'page' || item.content_type === 'post' || isStaticRoute) {
                item._inSitemap = !_sitemapIsExcluded(item.slug);
            } else {
                item._inSitemap = false;
            }
            var g = _smGoogleData[item.slug];
            item._clicks = g ? (g.clicks || 0) : 0;
            item._impressions = g ? (g.impressions || 0) : 0;
            item._ctr = g ? parseFloat(g.ctr || 0) : 0;
            item._position = g ? parseFloat(g.position || 0) : 0;
        });

        // Apply filter
        if (_smFilter === 'in-sitemap') items = items.filter(function(d) { return d._inSitemap; });
        else if (_smFilter === 'excluded') items = items.filter(function(d) { return !d._inSitemap; });
        else if (_smFilter === 'missing-meta') items = items.filter(function(d) { return !d.seo_title && !d.seo_description; });
        else if (_smFilter === 'low-score') items = items.filter(function(d) { return d.seo_score < 40; });
        else if (_smFilter === 'no-keyword') items = items.filter(function(d) { return !d.focus_keyword; });

        // Apply search
        if (_smSearch) {
            items = items.filter(function(d) {
                return (d.slug && d.slug.toLowerCase().indexOf(_smSearch) !== -1)
                    || (d.page_title && d.page_title.toLowerCase().indexOf(_smSearch) !== -1)
                    || (d.seo_title && d.seo_title.toLowerCase().indexOf(_smSearch) !== -1)
                    || (d.focus_keyword && d.focus_keyword.toLowerCase().indexOf(_smSearch) !== -1);
            });
        }

        // Sort
        items.sort(function(a, b) {
            var av = a[_smSort], bv = b[_smSort];
            if (typeof av === 'number' && typeof bv === 'number') return (av - bv) * _smSortDir;
            av = (av || '').toString().toLowerCase();
            bv = (bv || '').toString().toLowerCase();
            return av < bv ? -_smSortDir : av > bv ? _smSortDir : 0;
        });

        // Stats row
        var inSitemap = (exportRes.data || []).filter(function(d) {
            var isStaticRoute = d.content_type === 'static_page' && ['home', 'services', 'book-online', 'financing', 'customer-portal', 'service-areas', 'equipment', 'products'].indexOf(d.slug) !== -1;
            return !_sitemapIsExcluded(d.slug) && (d.content_type === 'page' || d.content_type === 'post' || isStaticRoute);
        }).length;
        var excluded = (exportRes.data || []).length - inSitemap;
        var avgScore = items.length ? Math.round(items.reduce(function(s, d) { return s + (d.seo_score || 0); }, 0) / items.length) : 0;
        var statsRow = document.createElement('div');
        statsRow.style.cssText = 'display:flex;gap:16px;margin-bottom:16px;';
        [
            { label: 'Total Pages', value: (exportRes.data || []).length },
            { label: 'In Sitemap', value: inSitemap },
            { label: 'Excluded', value: excluded },
            { label: 'Avg Score', value: avgScore },
            { label: 'Showing', value: items.length },
        ].forEach(function(s) {
            var card = document.createElement('div');
            card.style.cssText = 'background:var(--surface);border:1px solid var(--border);border-radius:8px;padding:12px 16px;text-align:center;';
            var val = document.createElement('div');
            val.style.cssText = 'font-size:20px;font-weight:700;';
            val.textContent = s.value;
            card.appendChild(val);
            var lbl = document.createElement('div');
            lbl.style.cssText = 'font-size:12px;color:var(--text-muted);';
            lbl.textContent = s.label;
            card.appendChild(lbl);
            statsRow.appendChild(card);
        });
        container.appendChild(statsRow);

        // Sitemap link
        var smLink = document.createElement('div');
        smLink.style.cssText = 'margin-bottom:16px;font-size:13px;';
        var a = document.createElement('a');
        a.href = '/sitemap.xml';
        a.target = '_blank';
        a.style.color = 'var(--accent)';
        a.textContent = 'View Live Sitemap XML';
        smLink.appendChild(a);
        container.appendChild(smLink);

        // Table
        var tableWrapper = document.createElement('div');
        tableWrapper.className = 'content-table';
        var t = document.createElement('table');
        var thead = document.createElement('thead');
        var headRow = document.createElement('tr');
        var cols = [
            { key: '_inSitemap', label: 'Sitemap' },
            { key: 'slug', label: 'Slug' },
            { key: 'seo_score', label: 'Score' },
            { key: 'seo_title', label: 'SEO Title' },
            { key: 'focus_keyword', label: 'Keyword' },
            { key: '_clicks', label: 'Clicks' },
            { key: '_position', label: 'Position' },
            { key: '', label: '' },
        ];
        cols.forEach(function(col) {
            var th = document.createElement('th');
            th.textContent = col.label;
            th.style.cursor = col.key ? 'pointer' : 'default';
            th.style.userSelect = 'none';
            if (col.key === _smSort) th.textContent += _smSortDir === 1 ? ' \u25B2' : ' \u25BC';
            if (col.key) {
                th.onclick = function() {
                    if (_smSort === col.key) _smSortDir *= -1;
                    else { _smSort = col.key; _smSortDir = 1; }
                    load_seo_sitemap();
                };
            }
            headRow.appendChild(th);
        });
        thead.appendChild(headRow);
        t.appendChild(thead);

        var tbody = document.createElement('tbody');
        items.forEach(function(item) {
            var tr = document.createElement('tr');
            tr.style.cursor = 'pointer';
            tr.onmouseover = function() { tr.style.background = 'rgba(96,165,250,0.05)'; };
            tr.onmouseout = function() { tr.style.background = ''; };

            // Sitemap status
            var tdSm = document.createElement('td');
            var smBadge = document.createElement('span');
            smBadge.className = 'status-badge';
            if (item._inSitemap) {
                smBadge.textContent = 'Yes';
                smBadge.style.cssText = 'background:rgba(34,197,94,0.15);color:#22c55e;';
            } else {
                smBadge.textContent = 'No';
                smBadge.style.cssText = 'background:rgba(239,68,68,0.15);color:#ef4444;';
            }
            tdSm.appendChild(smBadge);
            tr.appendChild(tdSm);

            // Slug
            var tdSlug = document.createElement('td');
            tdSlug.style.cssText = 'font-family:monospace;font-size:12px;max-width:200px;overflow:hidden;text-overflow:ellipsis;white-space:nowrap;';
            tdSlug.textContent = '/' + item.slug;
            tdSlug.title = '/' + item.slug;
            tr.appendChild(tdSlug);

            // Score
            var tdScore = document.createElement('td');
            tdScore.appendChild(seoScoreBadge(item.seo_score || 0));
            tr.appendChild(tdScore);

            // SEO Title
            var tdTitle = document.createElement('td');
            tdTitle.style.cssText = 'max-width:200px;overflow:hidden;text-overflow:ellipsis;white-space:nowrap;font-size:13px;';
            tdTitle.textContent = item.seo_title || '(not set)';
            if (!item.seo_title) tdTitle.style.color = 'var(--text-muted)';
            tdTitle.title = item.seo_title || '';
            tr.appendChild(tdTitle);

            // Keyword
            var tdKw = document.createElement('td');
            tdKw.style.fontSize = '13px';
            tdKw.textContent = item.focus_keyword || '-';
            if (!item.focus_keyword) tdKw.style.color = 'var(--text-muted)';
            tr.appendChild(tdKw);

            // Clicks
            var tdClicks = document.createElement('td');
            tdClicks.style.fontSize = '13px';
            tdClicks.textContent = item._clicks || '-';
            if (!item._clicks) tdClicks.style.color = 'var(--text-muted)';
            tr.appendChild(tdClicks);

            // Position
            var tdPos = document.createElement('td');
            tdPos.style.fontSize = '13px';
            if (item._position > 0) {
                tdPos.textContent = item._position.toFixed(1);
                if (item._position <= 10) tdPos.style.color = '#22c55e';
                else if (item._position <= 20) tdPos.style.color = '#f59e0b';
                else tdPos.style.color = '#ef4444';
            } else { tdPos.textContent = '-'; tdPos.style.color = 'var(--text-muted)'; }
            tr.appendChild(tdPos);

            // Edit button
            var tdEdit = document.createElement('td');
            tdEdit.style.textAlign = 'right';
            var editBtn = document.createElement('button');
            editBtn.className = 'btn btn-sm btn-ghost';
            editBtn.textContent = 'SEO Edit';
            editBtn.onclick = function(e) {
                e.stopPropagation();
                openSeoEditor(item.content_id, item.page_title, {
                    content_id: item.content_id, slug: item.slug, page_title: item.page_title,
                    seo_title: item.seo_title, seo_description: item.seo_description,
                    focus_keyword: item.focus_keyword, seo_score: item.seo_score
                });
            };
            tdEdit.appendChild(editBtn);
            tr.appendChild(tdEdit);

            // Row click opens editor too
            tr.onclick = function() {
                openSeoEditor(item.content_id, item.page_title, {
                    content_id: item.content_id, slug: item.slug, page_title: item.page_title,
                    seo_title: item.seo_title, seo_description: item.seo_description,
                    focus_keyword: item.focus_keyword, seo_score: item.seo_score
                });
            };

            tbody.appendChild(tr);
        });
        t.appendChild(tbody);
        tableWrapper.appendChild(t);
        container.appendChild(tableWrapper);

        // ── Duplicate/Near-duplicate Detection ──────────────────────
        var allItems = exportRes.data || [];
        var kwGroups = {};
        var titleGroups = {};
        allItems.forEach(function(item) {
            if (item.focus_keyword) {
                var kw = item.focus_keyword.toLowerCase().trim();
                if (!kwGroups[kw]) kwGroups[kw] = [];
                kwGroups[kw].push(item);
            }
            if (item.seo_title) {
                // Normalize title for near-duplicate detection: lowercase, strip common suffixes
                var normalized = item.seo_title.toLowerCase().replace(/\s*[\|\-\u2013\u2014].*$/, '').trim();
                // Also check first 30 chars for near-duplicates
                var shortTitle = normalized.substring(0, 30);
                if (!titleGroups[shortTitle]) titleGroups[shortTitle] = [];
                titleGroups[shortTitle].push(item);
            }
        });
        // Filter to only groups with 2+ items
        var kwDuplicates = {};
        Object.keys(kwGroups).forEach(function(k) { if (kwGroups[k].length >= 2) kwDuplicates[k] = kwGroups[k]; });
        var titleDuplicates = {};
        Object.keys(titleGroups).forEach(function(k) { if (titleGroups[k].length >= 2) titleDuplicates[k] = titleGroups[k]; });

        var dupCount = Object.keys(kwDuplicates).length + Object.keys(titleDuplicates).length;
        if (dupCount > 0) {
            var dupSection = document.createElement('div');
            dupSection.style.cssText = 'margin-top:24px;';
            var dupTitle = document.createElement('h3');
            dupTitle.style.cssText = 'margin-bottom:12px;display:flex;align-items:center;gap:8px;';
            dupTitle.textContent = 'Duplicate & Near-Duplicate Detection';
            var dupBadge = document.createElement('span');
            dupBadge.className = 'status-badge';
            dupBadge.style.cssText = 'background:rgba(245,158,11,0.15);color:#f59e0b;';
            dupBadge.textContent = dupCount + ' group' + (dupCount !== 1 ? 's' : '');
            dupTitle.appendChild(dupBadge);
            dupSection.appendChild(dupTitle);

            // Keyword duplicates
            if (Object.keys(kwDuplicates).length > 0) {
                var kwHeader = document.createElement('div');
                kwHeader.style.cssText = 'font-weight:600;font-size:13px;margin:12px 0 8px 0;color:#f59e0b;';
                kwHeader.textContent = 'Keyword Cannibalization (same focus keyword)';
                dupSection.appendChild(kwHeader);
                Object.keys(kwDuplicates).forEach(function(kw) {
                    var group = kwDuplicates[kw];
                    var gCard = document.createElement('div');
                    gCard.style.cssText = 'padding:12px;margin-bottom:8px;background:var(--surface);border:1px solid rgba(245,158,11,0.3);border-radius:8px;';
                    var gTitle = document.createElement('div');
                    gTitle.style.cssText = 'font-weight:600;margin-bottom:8px;font-size:13px;';
                    gTitle.textContent = 'Keyword: "' + kw + '" (' + group.length + ' pages)';
                    gCard.appendChild(gTitle);
                    group.forEach(function(p) {
                        var row = document.createElement('div');
                        row.style.cssText = 'display:flex;align-items:center;gap:8px;padding:4px 0;font-size:12px;border-bottom:1px solid var(--border);';
                        var slug = document.createElement('span');
                        slug.style.cssText = 'font-family:monospace;color:var(--accent);min-width:180px;';
                        slug.textContent = '/' + p.slug;
                        row.appendChild(slug);
                        var title = document.createElement('span');
                        title.style.cssText = 'flex:1;overflow:hidden;text-overflow:ellipsis;white-space:nowrap;';
                        title.textContent = p.seo_title || p.page_title || '(untitled)';
                        row.appendChild(title);
                        row.appendChild(seoScoreBadge(p.seo_score || 0));
                        var editBtn = document.createElement('button');
                        editBtn.className = 'btn btn-ghost btn-sm';
                        editBtn.style.fontSize = '11px';
                        editBtn.textContent = 'Edit';
                        editBtn.onclick = function(e) {
                            e.stopPropagation();
                            openSeoEditor(p.content_id, p.page_title, {
                                content_id: p.content_id, slug: p.slug, page_title: p.page_title,
                                seo_title: p.seo_title, seo_description: p.seo_description,
                                focus_keyword: p.focus_keyword, seo_score: p.seo_score,
                                content_type: p.content_type
                            });
                        };
                        row.appendChild(editBtn);
                        gCard.appendChild(row);
                    });
                    dupSection.appendChild(gCard);
                });
            }

            // Title near-duplicates
            if (Object.keys(titleDuplicates).length > 0) {
                var tHeader = document.createElement('div');
                tHeader.style.cssText = 'font-weight:600;font-size:13px;margin:16px 0 8px 0;color:#f59e0b;';
                tHeader.textContent = 'Near-Duplicate Titles (similar SEO titles)';
                dupSection.appendChild(tHeader);
                Object.keys(titleDuplicates).forEach(function(shortTitle) {
                    var group = titleDuplicates[shortTitle];
                    var gCard = document.createElement('div');
                    gCard.style.cssText = 'padding:12px;margin-bottom:8px;background:var(--surface);border:1px solid rgba(245,158,11,0.3);border-radius:8px;';
                    var gTitle = document.createElement('div');
                    gTitle.style.cssText = 'font-weight:600;margin-bottom:8px;font-size:13px;';
                    gTitle.textContent = 'Similar titles (' + group.length + ' pages)';
                    gCard.appendChild(gTitle);
                    group.forEach(function(p) {
                        var row = document.createElement('div');
                        row.style.cssText = 'display:flex;align-items:center;gap:8px;padding:4px 0;font-size:12px;border-bottom:1px solid var(--border);';
                        var slug = document.createElement('span');
                        slug.style.cssText = 'font-family:monospace;color:var(--accent);min-width:180px;';
                        slug.textContent = '/' + p.slug;
                        row.appendChild(slug);
                        var title = document.createElement('span');
                        title.style.cssText = 'flex:1;overflow:hidden;text-overflow:ellipsis;white-space:nowrap;';
                        title.textContent = p.seo_title || '(not set)';
                        row.appendChild(title);
                        row.appendChild(seoScoreBadge(p.seo_score || 0));
                        gCard.appendChild(row);
                    });
                    dupSection.appendChild(gCard);
                });
            }
            container.appendChild(dupSection);
        }

    } catch(e) {
        var err = document.createElement('p');
        err.style.color = '#ef4444';
        err.textContent = 'Error loading sitemap data: ' + e.message;
        container.appendChild(err);
    }
}

async function load_seo_link_checker() {
    var _role = (window.__CMS && window.__CMS.nexusRole) || '';
    var main = document.getElementById('adminMain');
    var el = document.createElement('div');

    var h = document.createElement('h2');
    h.textContent = 'Link Checker';
    el.appendChild(h);

    var desc = document.createElement('p');
    desc.style.cssText = 'color:var(--text-muted);margin-bottom:16px;';
    desc.textContent = 'Crawl your site to find broken links. Internal links are always checked; enable external checking to verify outbound links too.';
    el.appendChild(desc);

    var formCard = document.createElement('div');
    formCard.className = 'stat-card';
    formCard.style.cssText = 'max-width:600px;padding:20px;';

    var urlLabel = document.createElement('label');
    urlLabel.textContent = 'Base URL';
    urlLabel.style.cssText = 'display:block;font-weight:600;margin-bottom:4px;';
    formCard.appendChild(urlLabel);
    var urlInput = document.createElement('input');
    urlInput.className = 'admin-input';
    urlInput.type = 'url';
    urlInput.value = window.location.origin;
    urlInput.style.cssText = 'width:100%;margin-bottom:12px;';
    formCard.appendChild(urlInput);

    var maxRow = document.createElement('div');
    maxRow.style.cssText = 'display:flex;align-items:center;gap:12px;margin-bottom:12px;';
    var maxLabel = document.createElement('label');
    maxLabel.textContent = 'Max pages to crawl';
    maxLabel.style.fontWeight = '600';
    maxRow.appendChild(maxLabel);
    var maxInput = document.createElement('input');
    maxInput.className = 'admin-input';
    maxInput.type = 'number';
    maxInput.min = '1';
    maxInput.max = '200';
    maxInput.value = '50';
    maxInput.style.width = '80px';
    maxRow.appendChild(maxInput);
    formCard.appendChild(maxRow);

    var extRow = document.createElement('div');
    extRow.style.cssText = 'display:flex;align-items:center;gap:8px;margin-bottom:16px;';
    var extCheck = document.createElement('input');
    extCheck.type = 'checkbox';
    extCheck.id = 'lc-ext-check';
    extRow.appendChild(extCheck);
    var extLabel = document.createElement('label');
    extLabel.textContent = 'Check external links';
    extLabel.htmlFor = 'lc-ext-check';
    extRow.appendChild(extLabel);
    formCard.appendChild(extRow);

    var runBtn = document.createElement('button');
    runBtn.className = 'btn btn-primary';
    runBtn.textContent = 'Run Link Check';
    formCard.appendChild(runBtn);

    var statusEl = document.createElement('div');
    statusEl.style.cssText = 'margin-top:12px;display:none;';
    formCard.appendChild(statusEl);

    el.appendChild(formCard);

    var resultsArea = document.createElement('div');
    resultsArea.style.marginTop = '20px';
    el.appendChild(resultsArea);

    runBtn.onclick = async function() {
        runBtn.disabled = true;
        runBtn.textContent = 'Scanning...';
        statusEl.style.display = 'block';
        statusEl.textContent = '';
        resultsArea.textContent = '';

        if (!document.getElementById('seo-spin-style')) {
            var style = document.createElement('style');
            style.id = 'seo-spin-style';
            style.textContent = '@keyframes spin{to{transform:rotate(360deg)}}';
            document.head.appendChild(style);
        }
        var spinner = document.createElement('span');
        spinner.style.cssText = 'display:inline-block;width:14px;height:14px;border:2px solid var(--text-muted);border-top-color:var(--accent);border-radius:50%;animation:spin 0.6s linear infinite;margin-right:8px;vertical-align:middle;';
        statusEl.appendChild(spinner);
        var statusText = document.createElement('span');
        statusText.textContent = 'Crawling and checking links...';
        statusEl.appendChild(statusText);

        try {
            var payload = {
                base_url: urlInput.value,
                max_pages: parseInt(maxInput.value, 10) || 50,
                check_external: extCheck.checked
            };
            var r = await fetch('/api/modules/seo/link-check', {
                method: 'POST',
                headers: {'Content-Type': 'application/json'},
                body: JSON.stringify(payload)
            }).then(function(r) { return r.json(); });

            statusEl.style.display = 'none';
            runBtn.disabled = false;
            runBtn.textContent = 'Run Link Check';

            if (!r.ok) {
                toast('Link check failed: ' + r.message);
                return;
            }

            var d = r.data;
            toast(r.message);

            var statsDiv = document.createElement('div');
            statsDiv.className = 'stats';
            [
                ['Links Checked', d.checked_count],
                ['Broken Links', d.broken_count],
                ['Pages Crawled', d.pages_crawled]
            ].forEach(function(item) {
                var card = document.createElement('div');
                card.className = 'stat-card';
                var l = document.createElement('div');
                l.className = 'label';
                l.textContent = item[0];
                card.appendChild(l);
                var v = document.createElement('div');
                v.className = 'value';
                if (item[0] === 'Broken Links' && item[1] > 0) {
                    v.style.color = '#ef4444';
                }
                v.textContent = item[1];
                card.appendChild(v);
                statsDiv.appendChild(card);
            });
            resultsArea.appendChild(statsDiv);

            if (d.broken_links && d.broken_links.length > 0) {
                var exportBtn = document.createElement('button');
                exportBtn.className = 'btn btn-ghost btn-sm';
                exportBtn.textContent = 'Export CSV';
                exportBtn.style.marginBottom = '12px';
                exportBtn.onclick = function() {
                    var csv = 'Status,Source Page,Broken Link,Error\n';
                    d.broken_links.forEach(function(bl) {
                        csv += '"' + (bl.status_code || 'N/A') + '","' +
                            bl.source_page.replace(/"/g, '""') + '","' +
                            bl.target_url.replace(/"/g, '""') + '","' +
                            bl.error.replace(/"/g, '""') + '"\n';
                    });
                    var blob = new Blob([csv], {type: 'text/csv'});
                    var a = document.createElement('a');
                    a.href = URL.createObjectURL(blob);
                    a.download = 'broken-links-' + new Date().toISOString().slice(0, 10) + '.csv';
                    a.click();
                    URL.revokeObjectURL(a.href);
                    toast('CSV exported');
                };
                resultsArea.appendChild(exportBtn);

                var tableWrap = document.createElement('div');
                tableWrap.className = 'content-table';
                var table = document.createElement('table');
                var thead = document.createElement('tr');
                ['Status', 'Source Page', 'Broken Link', 'Error'].forEach(function(col) {
                    var th = document.createElement('th');
                    th.textContent = col;
                    thead.appendChild(th);
                });
                table.appendChild(thead);

                d.broken_links.forEach(function(bl) {
                    var tr = document.createElement('tr');

                    var tdStatus = document.createElement('td');
                    var badge = document.createElement('span');
                    badge.className = 'status-badge';
                    var code = bl.status_code;
                    if (code && code === 404) {
                        badge.style.cssText = 'background:rgba(239,68,68,0.15);color:#ef4444;';
                    } else if (code && code >= 500) {
                        badge.style.cssText = 'background:rgba(245,158,11,0.15);color:#f59e0b;';
                    } else {
                        badge.style.cssText = 'background:rgba(234,179,8,0.15);color:#eab308;';
                    }
                    badge.textContent = code ? String(code) : 'ERR';
                    tdStatus.appendChild(badge);
                    tr.appendChild(tdStatus);

                    var tdSource = document.createElement('td');
                    var srcLink = document.createElement('a');
                    srcLink.href = bl.source_page;
                    srcLink.target = '_blank';
                    srcLink.rel = 'noopener';
                    srcLink.textContent = bl.source_page.replace(urlInput.value, '') || '/';
                    srcLink.style.color = 'var(--accent)';
                    tdSource.appendChild(srcLink);
                    tr.appendChild(tdSource);

                    var tdTarget = document.createElement('td');
                    var tgtLink = document.createElement('a');
                    tgtLink.href = bl.target_url;
                    tgtLink.target = '_blank';
                    tgtLink.rel = 'noopener';
                    tgtLink.textContent = seoTruncate(bl.target_url, 60);
                    tgtLink.title = bl.target_url;
                    tgtLink.style.color = 'var(--accent)';
                    tdTarget.appendChild(tgtLink);
                    tr.appendChild(tdTarget);

                    var tdError = document.createElement('td');
                    tdError.textContent = bl.error;
                    tdError.style.cssText = 'font-size:12px;color:var(--text-muted);';
                    tr.appendChild(tdError);

                    table.appendChild(tr);
                });

                tableWrap.appendChild(table);
                resultsArea.appendChild(tableWrap);
            } else {
                var noIssues = document.createElement('div');
                noIssues.style.cssText = 'padding:24px;text-align:center;color:var(--text-muted);border:1px dashed var(--border);border-radius:8px;';
                noIssues.textContent = 'No broken links found.';
                resultsArea.appendChild(noIssues);
            }

        } catch(e) {
            statusEl.style.display = 'none';
            runBtn.disabled = false;
            runBtn.textContent = 'Run Link Check';
            toast('Link check error: ' + e.message);
        }
    };

    main.replaceChildren(el);
}

// ── Keyword Consistency Checklist ────────────────────────────────────

async function seoKeywordChecklist(contentId, container) {
    container.textContent = '';
    var r = await fetch('/api/modules/seo/keyword-check/' + encodeURIComponent(contentId))
        .then(function(r) { return r.json(); });
    if (!r.ok || !r.data) {
        var p = document.createElement('p');
        p.style.cssText = 'font-size:12px;color:var(--text-muted);';
        p.textContent = r.message || 'Set a focus keyword to see the checklist.';
        container.appendChild(p);
        return;
    }

    var d = r.data;
    var header = document.createElement('div');
    header.style.cssText = 'display:flex;align-items:center;gap:8px;margin-bottom:8px;';
    var label = document.createElement('strong');
    label.style.fontSize = '13px';
    label.textContent = 'Keyword: "' + d.focus_keyword + '" — ' + d.score + '/' + d.total;
    header.appendChild(label);
    container.appendChild(header);

    d.checks.forEach(function(check) {
        var row = document.createElement('div');
        row.style.cssText = 'display:flex;align-items:center;gap:6px;padding:4px 0;font-size:12px;';

        var icon = document.createElement('span');
        icon.textContent = check.passed ? '\u2705' : '\u274C';
        row.appendChild(icon);

        var text = document.createElement('span');
        text.textContent = check.label;
        if (!check.passed) text.style.color = 'var(--text-muted)';
        row.appendChild(text);

        if (!check.passed && check.suggestion) {
            var tip = document.createElement('span');
            tip.style.cssText = 'font-size:11px;color:var(--text-muted);margin-left:auto;';
            tip.textContent = check.suggestion;
            row.appendChild(tip);
        }

        container.appendChild(row);
    });
}

// ── Change URL Button ────────────────────────────────────────────────

function seoChangeUrlButton(contentId, currentSlug) {
    var btn = document.createElement('button');
    btn.className = 'btn btn-ghost btn-sm';
    btn.style.cssText = 'font-size:11px;padding:2px 8px;';
    btn.textContent = 'Change URL';
    btn.onclick = async function() {
        var newSlug = prompt('Enter new URL slug:', currentSlug);
        if (!newSlug || newSlug === currentSlug) return;

        // Phase 1: Pre-check
        var check = await fetch('/api/modules/seo/meta/' + encodeURIComponent(contentId) + '/slug-check?new_slug=' + encodeURIComponent(newSlug))
            .then(function(r) { return r.json(); });
        if (!check.ok) { showToast(check.message, 'error'); return; }
        if (check.data && !check.data.new_slug_available) { showToast('Slug "' + newSlug + '" is already in use.', 'error'); return; }
        if (check.data && check.data.ai_warning) {
            if (!confirm(check.data.ai_warning + '\n\nProceed with URL change?')) return;
        }

        // Phase 2: Execute
        var result = await fetch('/api/modules/seo/meta/' + encodeURIComponent(contentId) + '/slug', {
            method: 'PUT',
            headers: {'Content-Type': 'application/json'},
            body: JSON.stringify({new_slug: newSlug})
        }).then(function(r) { return r.json(); });

        if (result.ok) {
            showToast(result.message, 'success');
            load_seo_meta();
        } else {
            showToast('Error: ' + result.message, 'error');
        }
    };
    return btn;
}

// ── A/B Test Button ──────────────────────────────────────────────────

function seoAbTestButton(contentId, field, currentValue) {
    var btn = document.createElement('button');
    btn.className = 'btn btn-ghost btn-sm';
    btn.style.cssText = 'font-size:11px;padding:2px 8px;';
    btn.textContent = 'A/B Test';
    btn.onclick = async function() {
        var variant_b = prompt('Enter challenger value for ' + field + ':', '');
        if (!variant_b) return;
        var duration = prompt('Duration in days (default 14):', '14');
        var dur = parseInt(duration) || 14;

        var r = await fetch('/api/modules/seo/ab/create', {
            method: 'POST',
            headers: {'Content-Type': 'application/json'},
            body: JSON.stringify({
                content_id: contentId,
                field: field,
                variant_b: variant_b,
                duration_days: dur
            })
        }).then(function(r) { return r.json(); });

        if (r.ok) {
            showToast('A/B experiment started! ' + r.data.experiment_id, 'success');
        } else {
            showToast('Error: ' + r.message, 'error');
        }
    };
    return btn;
}
"##;
