//! Inline JavaScript for the Surfer SEO admin panel (sheets, page mapping, AI queue).

pub(crate) const SURFER_ADMIN_JS: &str = r##"
// ── Surfer Sheets / Page Mapping / AI Queue admin views ──────────────────────
// Security: all rendering uses DOM methods (createElement/textContent), no innerHTML.

// ── Shared fetch helper ────────────────────────────────────────────────────────

async function surferFetch(url, opts) {
    try {
        var r = await fetch(url, opts);
        return await r.json();
    } catch(e) {
        return { ok: false, message: e.message };
    }
}

// ── Style constants ────────────────────────────────────────────────────────────

var _surferCardStyle    = 'border-radius:8px;background:#1a1a2e;border:1px solid #333;padding:16px;cursor:pointer;transition:border-color 0.15s;';
var _surferBtnStyle     = 'background:#6c5ce7;color:#fff;border:none;border-radius:6px;padding:8px 16px;cursor:pointer;font-size:13px;';
var _surferBtnSmStyle   = 'background:#6c5ce7;color:#fff;border:none;border-radius:6px;padding:4px 10px;cursor:pointer;font-size:12px;';
var _surferBtnGhostStyle = 'background:transparent;color:#a0aec0;border:1px solid #333;border-radius:6px;padding:8px 16px;cursor:pointer;font-size:13px;';
var _surferBtnDangerStyle = 'background:#e53e3e;color:#fff;border:none;border-radius:6px;padding:4px 10px;cursor:pointer;font-size:12px;';
var _surferBadgeStyle   = 'display:inline-block;padding:2px 8px;border-radius:4px;font-size:11px;background:#333;color:#e2e8f0;';
var _surferTableStyle   = 'width:100%;border-collapse:collapse;font-size:13px;';
var _surferThStyle      = 'padding:8px 12px;border-bottom:2px solid #333;text-align:left;color:#a0aec0;font-weight:600;font-size:12px;cursor:pointer;user-select:none;';
var _surferTdStyle      = 'padding:8px 12px;border-bottom:1px solid #333;color:#e2e8f0;vertical-align:middle;';
var _surferStatCardStyle = 'background:#1a1a2e;border:1px solid #333;border-radius:8px;padding:16px;flex:1;min-width:100px;';
var _surferInputStyle   = 'background:#0d0d1a;border:1px solid #333;border-radius:6px;padding:8px 12px;color:#e2e8f0;font-size:13px;width:100%;box-sizing:border-box;';
var _surferTabStyle     = 'background:transparent;border:none;border-bottom:2px solid transparent;padding:8px 16px;cursor:pointer;font-size:13px;color:#a0aec0;';
var _surferTabActiveStyle = 'background:transparent;border:none;border-bottom:2px solid #6c5ce7;padding:8px 16px;cursor:pointer;font-size:13px;color:#e2e8f0;font-weight:600;';

// ── Helper: stat card ─────────────────────────────────────────────────────────

function surferStatCard(label, value, color) {
    var card = document.createElement('div');
    card.style.cssText = _surferStatCardStyle;
    var l = document.createElement('div');
    l.style.cssText = 'font-size:11px;color:#a0aec0;margin-bottom:6px;text-transform:uppercase;font-weight:600;';
    l.textContent = label;
    card.appendChild(l);
    var v = document.createElement('div');
    v.style.cssText = 'font-size:24px;font-weight:700;color:' + (color || '#e2e8f0') + ';';
    v.textContent = String(value);
    card.appendChild(v);
    return card;
}

// ── Helper: empty state ──────────────────────────────────────────────────────

function surferEmptyState(msg) {
    var el = document.createElement('div');
    el.style.cssText = 'padding:32px;text-align:center;color:#a0aec0;font-size:14px;';
    el.textContent = msg;
    return el;
}

// ── Helper: section heading ──────────────────────────────────────────────────

function surferSectionHeading(text) {
    var h = document.createElement('h3');
    h.style.cssText = 'margin:24px 0 12px;font-size:15px;font-weight:700;color:#e2e8f0;border-bottom:1px solid #333;padding-bottom:8px;';
    h.textContent = text;
    return h;
}

// ════════════════════════════════════════════════════════════════════════════
// VIEW 1 — Surfer Sheets
// ════════════════════════════════════════════════════════════════════════════

var _surferSheetsFilter = '';
var _surferSheetsData   = [];

async function load_seo_surfer() {
    var main = document.getElementById('adminMain');
    if (!main) return;
    main.textContent = '';
    var role = (window.__CMS && window.__CMS.nexusRole) || '';

    if (role && role !== 'central') {
        var header = document.createElement('h2');
        header.style.cssText = 'margin:0 0 16px;font-size:1.4rem;color:#e2e8f0;';
        header.textContent = 'Surfer SEO Sheets';
        main.appendChild(header);

        var note = document.createElement('div');
        note.className = 'card';
        note.style.cssText = 'padding:16px;border:1px solid #333;border-radius:10px;background:#1a1a2e;max-width:900px;';
        var p1 = document.createElement('p');
        p1.textContent = 'Surfer sheets, page mappings, and the AI content queue are managed on Central so LuperIQ keeps the reference optimization layer inside the platform.';
        note.appendChild(p1);
        var p2 = document.createElement('p');
        p2.style.marginBottom = '0';
        p2.textContent = 'Customer sites can still use grounded content workflows, but the raw Surfer library is not editable here.';
        note.appendChild(p2);
        main.appendChild(note);
        return;
    }

    // ── Header row ──
    var header = document.createElement('div');
    header.style.cssText = 'display:flex;align-items:center;justify-content:space-between;margin-bottom:20px;flex-wrap:wrap;gap:12px;';
    var h2 = document.createElement('h2');
    h2.style.cssText = 'margin:0;font-size:1.4rem;color:#e2e8f0;';
    h2.textContent = 'Surfer SEO Sheets';
    header.appendChild(h2);

    var btnRow = document.createElement('div');
    btnRow.style.cssText = 'display:flex;gap:8px;flex-wrap:wrap;';

    var uploadBtn = document.createElement('button');
    uploadBtn.style.cssText = _surferBtnStyle;
    uploadBtn.textContent = 'Upload .txt';
    uploadBtn.onclick = function() { surferShowUploadPanel(main); };
    btnRow.appendChild(uploadBtn);

    var importBtn = document.createElement('button');
    importBtn.style.cssText = _surferBtnGhostStyle;
    importBtn.textContent = 'Import Directory';
    importBtn.onclick = async function() {
        importBtn.disabled = true;
        importBtn.textContent = 'Importing...';
        var r = await surferFetch('/api/modules/seo/surfer/import-dir', { method: 'POST' });
        importBtn.disabled = false;
        importBtn.textContent = 'Import Directory';
        if (typeof showToast === 'function') showToast(r.message || (r.ok ? 'Imported' : 'Import failed'), r.ok ? 'success' : 'error');
        else if (typeof toast === 'function') toast(r.message || (r.ok ? 'Imported' : 'Import failed'));
        if (r.ok) load_seo_surfer();
    };
    btnRow.appendChild(importBtn);

    header.appendChild(btnRow);
    main.appendChild(header);

    // ── Load data ──
    var r = await surferFetch('/api/modules/seo/surfer/sheets');
    _surferSheetsData = (r.ok && Array.isArray(r.data)) ? r.data : [];

    // ── Stats row ──
    var industries = {};
    _surferSheetsData.forEach(function(s) { industries[s.industry || 'general'] = true; });
    var statsRow = document.createElement('div');
    statsRow.style.cssText = 'display:flex;gap:12px;margin-bottom:20px;flex-wrap:wrap;';
    statsRow.appendChild(surferStatCard('Total Sheets', _surferSheetsData.length, '#6c5ce7'));
    statsRow.appendChild(surferStatCard('Industries', Object.keys(industries).length, '#00b5d8'));
    main.appendChild(statsRow);

    // ── Industry filter ──
    var filterBar = document.createElement('div');
    filterBar.style.cssText = 'display:flex;align-items:center;gap:8px;margin-bottom:16px;flex-wrap:wrap;';

    var filterLabel = document.createElement('span');
    filterLabel.style.cssText = 'font-size:12px;color:#a0aec0;';
    filterLabel.textContent = 'Industry:';
    filterBar.appendChild(filterLabel);

    var industrySelect = document.createElement('select');
    industrySelect.style.cssText = _surferInputStyle + 'width:auto;padding:6px 10px;';
    var allOpt = document.createElement('option');
    allOpt.value = '';
    allOpt.textContent = 'All Industries';
    industrySelect.appendChild(allOpt);
    Object.keys(industries).sort().forEach(function(ind) {
        var opt = document.createElement('option');
        opt.value = ind;
        opt.textContent = ind.charAt(0).toUpperCase() + ind.slice(1);
        if (ind === _surferSheetsFilter) opt.selected = true;
        industrySelect.appendChild(opt);
    });
    industrySelect.onchange = function() {
        _surferSheetsFilter = this.value;
        surferRenderSheetGrid(main, gridContainer);
    };
    industrySelect.value = _surferSheetsFilter;
    filterBar.appendChild(industrySelect);
    main.appendChild(filterBar);

    // ── Sheet grid ──
    var gridContainer = document.createElement('div');
    main.appendChild(gridContainer);
    surferRenderSheetGrid(main, gridContainer);
}

function surferRenderSheetGrid(main, container) {
    container.textContent = '';
    var sheets = _surferSheetsFilter
        ? _surferSheetsData.filter(function(s) { return s.industry === _surferSheetsFilter; })
        : _surferSheetsData;

    if (sheets.length === 0) {
        container.appendChild(surferEmptyState('No sheets found. Upload a Surfer SEO .txt file or import from the surfer/ directory.'));
        return;
    }

    var grid = document.createElement('div');
    grid.style.cssText = 'display:grid;grid-template-columns:repeat(auto-fill,minmax(280px,1fr));gap:16px;';

    sheets.forEach(function(sheet) {
        var card = document.createElement('div');
        card.style.cssText = _surferCardStyle;
        card.onmouseenter = function() { this.style.borderColor = '#6c5ce7'; };
        card.onmouseleave = function() { this.style.borderColor = '#333'; };

        // Topic heading
        var topic = document.createElement('div');
        topic.style.cssText = 'font-weight:700;font-size:14px;color:#e2e8f0;margin-bottom:8px;line-height:1.3;';
        topic.textContent = sheet.topic || sheet.sheet_id;
        card.appendChild(topic);

        // Industry badge
        var badge = document.createElement('span');
        badge.style.cssText = _surferBadgeStyle + 'margin-bottom:10px;display:inline-block;background:#2d3748;color:#a0aec0;';
        badge.textContent = sheet.industry || 'general';
        card.appendChild(badge);
        card.appendChild(document.createElement('br'));

        // Counts
        var counts = document.createElement('div');
        counts.style.cssText = 'display:flex;gap:12px;margin:10px 0;font-size:12px;color:#a0aec0;';
        var termCount = document.createElement('span');
        termCount.textContent = (sheet.term_count || 0) + ' terms';
        counts.appendChild(termCount);
        var factCount = document.createElement('span');
        factCount.textContent = (sheet.fact_group_count || 0) + ' fact groups';
        counts.appendChild(factCount);
        card.appendChild(counts);

        // Source file
        if (sheet.source_file) {
            var src = document.createElement('div');
            src.style.cssText = 'font-size:11px;color:#718096;margin-bottom:10px;white-space:nowrap;overflow:hidden;text-overflow:ellipsis;';
            src.textContent = sheet.source_file;
            src.title = sheet.source_file;
            card.appendChild(src);
        }

        // Action row
        var actions = document.createElement('div');
        actions.style.cssText = 'display:flex;gap:8px;margin-top:12px;';

        var detailBtn = document.createElement('button');
        detailBtn.style.cssText = _surferBtnSmStyle;
        detailBtn.textContent = 'View Details';
        (function(sheetId) {
            detailBtn.onclick = function(e) { e.stopPropagation(); surferShowSheetDetail(sheetId); };
        })(sheet.sheet_id);
        actions.appendChild(detailBtn);

        var delBtn = document.createElement('button');
        delBtn.style.cssText = _surferBtnDangerStyle;
        delBtn.textContent = 'Delete';
        (function(sheetId, sheetTopic) {
            delBtn.onclick = async function(e) {
                e.stopPropagation();
                if (!confirm('Delete sheet "' + sheetTopic + '"?')) return;
                var r = await surferFetch('/api/modules/seo/surfer/sheets/' + encodeURIComponent(sheetId), { method: 'DELETE' });
                if (typeof showToast === 'function') showToast(r.message || (r.ok ? 'Deleted' : 'Delete failed'), r.ok ? 'success' : 'error');
                else if (typeof toast === 'function') toast(r.message || (r.ok ? 'Deleted' : 'Delete failed'));
                if (r.ok) load_seo_surfer();
            };
        })(sheet.sheet_id, sheet.topic);
        actions.appendChild(delBtn);

        card.appendChild(actions);
        card.onclick = (function(sheetId) { return function() { surferShowSheetDetail(sheetId); }; })(sheet.sheet_id);
        grid.appendChild(card);
    });

    container.appendChild(grid);
}

async function surferShowSheetDetail(sheetId) {
    var main = document.getElementById('adminMain');
    if (!main) return;
    main.textContent = '';

    var r = await surferFetch('/api/modules/seo/surfer/sheets/' + encodeURIComponent(sheetId));
    if (!r.ok || !r.data) {
        main.appendChild(surferEmptyState('Sheet not found: ' + sheetId));
        return;
    }
    var sheet = r.data;

    // Back button
    var backBtn = document.createElement('button');
    backBtn.style.cssText = _surferBtnGhostStyle + 'margin-bottom:16px;';
    backBtn.textContent = '\u2190 Back to Sheets';
    backBtn.onclick = function() { load_seo_surfer(); };
    main.appendChild(backBtn);

    // Heading
    var h2 = document.createElement('h2');
    h2.style.cssText = 'margin:0 0 4px;font-size:1.3rem;color:#e2e8f0;';
    h2.textContent = sheet.topic || sheet.sheet_id;
    main.appendChild(h2);

    var meta = document.createElement('div');
    meta.style.cssText = 'display:flex;gap:12px;margin-bottom:20px;font-size:12px;color:#a0aec0;';
    var indBadge = document.createElement('span');
    indBadge.style.cssText = _surferBadgeStyle;
    indBadge.textContent = sheet.industry || 'general';
    meta.appendChild(indBadge);
    if (sheet.source_date) {
        var dateEl = document.createElement('span');
        dateEl.textContent = 'Date: ' + sheet.source_date;
        meta.appendChild(dateEl);
    }
    if (sheet.source_file) {
        var fileEl = document.createElement('span');
        fileEl.textContent = 'File: ' + sheet.source_file;
        meta.appendChild(fileEl);
    }
    main.appendChild(meta);

    // Structure targets
    main.appendChild(surferSectionHeading('Content Structure Targets'));
    if (sheet.structure) {
        var structTable = document.createElement('table');
        structTable.style.cssText = _surferTableStyle;
        var thead = document.createElement('thead');
        var hrow = document.createElement('tr');
        ['Metric', 'Min', 'Max'].forEach(function(col) {
            var th = document.createElement('th');
            th.style.cssText = _surferThStyle;
            th.textContent = col;
            hrow.appendChild(th);
        });
        thead.appendChild(hrow);
        structTable.appendChild(thead);
        var tbody = document.createElement('tbody');
        var s = sheet.structure;
        [
            ['Words', s.words],
            ['Headings', s.headings],
            ['Paragraphs', s.paragraphs],
            ['Images', s.images],
            ['Characters', s.characters],
        ].forEach(function(entry) {
            var metric = entry[0], range = entry[1] || {};
            var row = document.createElement('tr');
            [metric, range.min != null ? range.min : '—', range.max != null ? range.max : '—'].forEach(function(val) {
                var td = document.createElement('td');
                td.style.cssText = _surferTdStyle;
                td.textContent = val;
                row.appendChild(td);
            });
            tbody.appendChild(row);
        });
        structTable.appendChild(tbody);
        main.appendChild(structTable);
    }

    // Terms
    main.appendChild(surferSectionHeading('Important Terms (' + (sheet.terms ? sheet.terms.length : 0) + ')'));
    if (sheet.terms && sheet.terms.length > 0) {
        var termTable = document.createElement('table');
        termTable.style.cssText = _surferTableStyle;
        var tthead = document.createElement('thead');
        var throw2 = document.createElement('tr');
        ['Term', 'Min', 'Max'].forEach(function(col) {
            var th = document.createElement('th');
            th.style.cssText = _surferThStyle;
            th.textContent = col;
            throw2.appendChild(th);
        });
        tthead.appendChild(throw2);
        termTable.appendChild(tthead);
        var ttbody = document.createElement('tbody');
        sheet.terms.forEach(function(term) {
            var row = document.createElement('tr');
            [term.term, term.min, term.max].forEach(function(val) {
                var td = document.createElement('td');
                td.style.cssText = _surferTdStyle;
                td.textContent = val != null ? val : '—';
                row.appendChild(td);
            });
            ttbody.appendChild(row);
        });
        termTable.appendChild(ttbody);
        main.appendChild(termTable);
    } else {
        main.appendChild(surferEmptyState('No terms defined in this sheet.'));
    }

    // Facts
    main.appendChild(surferSectionHeading('Facts to Include (' + (sheet.facts ? sheet.facts.length : 0) + ' groups)'));
    if (sheet.facts && sheet.facts.length > 0) {
        sheet.facts.forEach(function(group) {
            var groupHead = document.createElement('div');
            groupHead.style.cssText = 'font-weight:600;font-size:13px;color:#a0aec0;margin:12px 0 6px;';
            groupHead.textContent = group.group || 'General';
            main.appendChild(groupHead);
            if (group.items && group.items.length > 0) {
                var ul = document.createElement('ul');
                ul.style.cssText = 'margin:0 0 8px 16px;padding:0;list-style:disc;color:#e2e8f0;font-size:13px;';
                group.items.forEach(function(item) {
                    var li = document.createElement('li');
                    li.style.cssText = 'margin-bottom:4px;line-height:1.5;';
                    li.textContent = item;
                    ul.appendChild(li);
                });
                main.appendChild(ul);
            }
        });
    } else {
        main.appendChild(surferEmptyState('No facts defined in this sheet.'));
    }
}

function surferShowUploadPanel(main) {
    // Modal overlay
    var overlay = document.createElement('div');
    overlay.style.cssText = 'position:fixed;inset:0;background:rgba(0,0,0,0.7);z-index:9998;display:flex;align-items:center;justify-content:center;';

    var panel = document.createElement('div');
    panel.style.cssText = 'background:#0d0d1a;border:1px solid #333;border-radius:12px;padding:24px;width:560px;max-width:95vw;max-height:80vh;overflow-y:auto;';

    var ph2 = document.createElement('h3');
    ph2.style.cssText = 'margin:0 0 16px;color:#e2e8f0;font-size:16px;';
    ph2.textContent = 'Upload Surfer SEO Sheet (.txt)';
    panel.appendChild(ph2);

    // Filename input
    var fnLabel = document.createElement('label');
    fnLabel.style.cssText = 'display:block;font-size:12px;color:#a0aec0;margin-bottom:4px;';
    fnLabel.textContent = 'Filename (e.g. surfer-pest-control-2026.txt)';
    panel.appendChild(fnLabel);
    var fnInput = document.createElement('input');
    fnInput.type = 'text';
    fnInput.style.cssText = _surferInputStyle + 'margin-bottom:12px;';
    fnInput.placeholder = 'surfer-guidelines-topic-date.txt';
    panel.appendChild(fnInput);

    // Content textarea
    var ctLabel = document.createElement('label');
    ctLabel.style.cssText = 'display:block;font-size:12px;color:#a0aec0;margin-bottom:4px;';
    ctLabel.textContent = 'Paste file content here';
    panel.appendChild(ctLabel);
    var ta = document.createElement('textarea');
    ta.style.cssText = _surferInputStyle + 'height:200px;resize:vertical;font-family:monospace;font-size:12px;margin-bottom:16px;';
    ta.placeholder = 'Paste the contents of your Surfer SEO .txt file...';
    panel.appendChild(ta);

    // Error display
    var errMsg = document.createElement('div');
    errMsg.style.cssText = 'color:#fc8181;font-size:12px;margin-bottom:8px;display:none;';
    panel.appendChild(errMsg);

    // Action buttons
    var btnRow = document.createElement('div');
    btnRow.style.cssText = 'display:flex;gap:8px;justify-content:flex-end;';

    var cancelBtn = document.createElement('button');
    cancelBtn.style.cssText = _surferBtnGhostStyle;
    cancelBtn.textContent = 'Cancel';
    cancelBtn.onclick = function() { document.body.removeChild(overlay); };
    btnRow.appendChild(cancelBtn);

    var submitBtn = document.createElement('button');
    submitBtn.style.cssText = _surferBtnStyle;
    submitBtn.textContent = 'Upload Sheet';
    submitBtn.onclick = async function() {
        errMsg.style.display = 'none';
        var filename = fnInput.value.trim();
        var content = ta.value.trim();
        if (!filename) { errMsg.textContent = 'Please enter a filename.'; errMsg.style.display = 'block'; return; }
        if (!content) { errMsg.textContent = 'Please paste the file content.'; errMsg.style.display = 'block'; return; }
        submitBtn.disabled = true;
        submitBtn.textContent = 'Uploading...';
        var r = await surferFetch('/api/modules/seo/surfer/sheets/upload', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ filename: filename, content: content }),
        });
        submitBtn.disabled = false;
        submitBtn.textContent = 'Upload Sheet';
        if (!r.ok) {
            errMsg.textContent = r.message || 'Upload failed';
            errMsg.style.display = 'block';
            return;
        }
        document.body.removeChild(overlay);
        if (typeof showToast === 'function') showToast('Sheet uploaded successfully', 'success');
        else if (typeof toast === 'function') toast('Sheet uploaded successfully');
        load_seo_surfer();
    };
    btnRow.appendChild(submitBtn);
    panel.appendChild(btnRow);

    overlay.appendChild(panel);
    overlay.onclick = function(e) { if (e.target === overlay) document.body.removeChild(overlay); };
    document.body.appendChild(overlay);
    fnInput.focus();
}

// ════════════════════════════════════════════════════════════════════════════
// VIEW 2 — Page Mapping
// ════════════════════════════════════════════════════════════════════════════

async function load_seo_mapping() {
    var main = document.getElementById('adminMain');
    if (!main) return;
    main.textContent = '';

    // Header
    var header = document.createElement('div');
    header.style.cssText = 'display:flex;align-items:center;justify-content:space-between;margin-bottom:20px;flex-wrap:wrap;gap:12px;';
    var h2 = document.createElement('h2');
    h2.style.cssText = 'margin:0;font-size:1.4rem;color:#e2e8f0;';
    h2.textContent = 'Page Mapping';
    header.appendChild(h2);

    var autoMapBtn = document.createElement('button');
    autoMapBtn.style.cssText = _surferBtnStyle;
    autoMapBtn.textContent = 'Auto-Map All';
    autoMapBtn.onclick = async function() {
        autoMapBtn.disabled = true;
        autoMapBtn.textContent = 'Mapping...';
        var r = await surferFetch('/api/modules/seo/surfer/auto-map', { method: 'POST' });
        autoMapBtn.disabled = false;
        autoMapBtn.textContent = 'Auto-Map All';
        if (typeof showToast === 'function') showToast(r.message || (r.ok ? 'Auto-map complete' : 'Failed'), r.ok ? 'success' : 'error');
        else if (typeof toast === 'function') toast(r.message || (r.ok ? 'Auto-map complete' : 'Failed'));
        if (r.ok) load_seo_mapping();
    };
    header.appendChild(autoMapBtn);
    main.appendChild(header);

    // Load data in parallel
    var loadingMsg = document.createElement('p');
    loadingMsg.style.cssText = 'color:#a0aec0;font-size:13px;';
    loadingMsg.textContent = 'Loading page data...';
    main.appendChild(loadingMsg);

    var [unmappedR, sheetsR] = await Promise.all([
        surferFetch('/api/modules/seo/surfer/map/unmapped'),
        surferFetch('/api/modules/seo/surfer/sheets'),
    ]);

    main.removeChild(loadingMsg);

    var unmapped = (unmappedR.ok && Array.isArray(unmappedR.data)) ? unmappedR.data : [];
    var sheets = (sheetsR.ok && Array.isArray(sheetsR.data)) ? sheetsR.data : [];

    // We also need mapped pages — load all sheets' maps indirectly via a separate endpoint.
    // For now: gather content items that have a mapping by calling list_queue + cross-referencing.
    // The mapped list uses a different approach: we load the queue items which have content_ids.
    // Actually, let's get the mapped list from the queue endpoint for a quick overview.
    var queueR = await surferFetch('/api/modules/seo/surfer/queue');
    var queueItems = (queueR.ok && Array.isArray(queueR.data)) ? queueR.data : [];
    var mappedContentIds = {};
    queueItems.forEach(function(qi) {
        if (qi.surfer_score && qi.surfer_score > 0) mappedContentIds[qi.content_id] = qi;
    });

    // Stats
    var mappedCount = Object.keys(mappedContentIds).length;
    var statsRow = document.createElement('div');
    statsRow.style.cssText = 'display:flex;gap:12px;margin-bottom:20px;flex-wrap:wrap;';
    statsRow.appendChild(surferStatCard('Total Published', unmapped.length + mappedCount));
    statsRow.appendChild(surferStatCard('Mapped', mappedCount, '#48bb78'));
    statsRow.appendChild(surferStatCard('Unmapped', unmapped.length, '#fc8181'));
    main.appendChild(statsRow);

    // ── Unmapped Pages ──
    main.appendChild(surferSectionHeading('Unmapped Pages (' + unmapped.length + ')'));

    if (unmapped.length === 0) {
        main.appendChild(surferEmptyState('All pages are mapped to Surfer sheets.'));
    } else {
        var unmappedTable = document.createElement('table');
        unmappedTable.style.cssText = _surferTableStyle;
        var uthead = document.createElement('thead');
        var utr = document.createElement('tr');
        ['Slug', 'Title', 'Suggested Sheet', 'Action'].forEach(function(col) {
            var th = document.createElement('th');
            th.style.cssText = _surferThStyle;
            th.textContent = col;
            utr.appendChild(th);
        });
        uthead.appendChild(utr);
        unmappedTable.appendChild(uthead);
        var utbody = document.createElement('tbody');

        unmapped.forEach(function(page) {
            var row = document.createElement('tr');

            var slugTd = document.createElement('td');
            slugTd.style.cssText = _surferTdStyle + 'font-family:monospace;font-size:12px;';
            slugTd.textContent = page.slug;
            row.appendChild(slugTd);

            var titleTd = document.createElement('td');
            titleTd.style.cssText = _surferTdStyle;
            titleTd.textContent = page.title || '—';
            row.appendChild(titleTd);

            // Suggestion cell — lazy loaded
            var suggTd = document.createElement('td');
            suggTd.style.cssText = _surferTdStyle + 'color:#a0aec0;font-size:12px;';
            suggTd.textContent = 'Loading...';
            row.appendChild(suggTd);

            var actionTd = document.createElement('td');
            actionTd.style.cssText = _surferTdStyle;
            row.appendChild(actionTd);

            // Load suggestions for this page
            (function(contentId, slug, std, atd) {
                surferFetch('/api/modules/seo/surfer/map/suggest/' + encodeURIComponent(contentId)).then(function(sr) {
                    std.textContent = '';
                    if (sr.ok && sr.data && sr.data.length > 0) {
                        var topSugg = sr.data[0];
                        var badge = document.createElement('span');
                        badge.style.cssText = _surferBadgeStyle + 'background:#2d3748;color:#e2e8f0;';
                        badge.textContent = topSugg.topic || topSugg.sheet_id;
                        std.appendChild(badge);
                        if (topSugg.confidence != null) {
                            var conf = document.createElement('span');
                            conf.style.cssText = 'font-size:11px;color:#a0aec0;margin-left:6px;';
                            conf.textContent = Math.round(topSugg.confidence * 100) + '%';
                            std.appendChild(conf);
                        }

                        var mapBtn = document.createElement('button');
                        mapBtn.style.cssText = _surferBtnSmStyle;
                        mapBtn.textContent = 'Map';
                        mapBtn.onclick = async function() {
                            mapBtn.disabled = true;
                            var mr = await surferFetch('/api/modules/seo/surfer/map/' + encodeURIComponent(contentId), {
                                method: 'PUT',
                                headers: { 'Content-Type': 'application/json' },
                                body: JSON.stringify({ sheet_ids: [topSugg.sheet_id], primary_sheet_id: topSugg.sheet_id }),
                            });
                            if (typeof showToast === 'function') showToast(mr.message || (mr.ok ? 'Mapped' : 'Failed'), mr.ok ? 'success' : 'error');
                            else if (typeof toast === 'function') toast(mr.message || (mr.ok ? 'Mapped' : 'Failed'));
                            if (mr.ok) load_seo_mapping();
                        };
                        atd.appendChild(mapBtn);
                    } else {
                        std.textContent = 'No suggestion';
                    }
                });
            })(page.content_id, page.slug, suggTd, actionTd);

            utbody.appendChild(row);
        });

        unmappedTable.appendChild(utbody);
        main.appendChild(unmappedTable);
    }

    // ── Mapped Pages ──
    main.appendChild(surferSectionHeading('Mapped Pages (' + mappedCount + ')'));

    if (mappedCount === 0) {
        main.appendChild(surferEmptyState('No pages have been mapped yet. Use Auto-Map All or map individual pages above.'));
    } else {
        var mappedTable = document.createElement('table');
        mappedTable.style.cssText = _surferTableStyle;
        var mthead = document.createElement('thead');
        var mtr = document.createElement('tr');
        ['Slug', 'Primary Sheet', 'Surfer Score', 'Action'].forEach(function(col) {
            var th = document.createElement('th');
            th.style.cssText = _surferThStyle;
            th.textContent = col;
            mtr.appendChild(th);
        });
        mthead.appendChild(mtr);
        mappedTable.appendChild(mthead);
        var mtbody = document.createElement('tbody');

        Object.values(mappedContentIds).forEach(function(qi) {
            var row = document.createElement('tr');

            var slugTd = document.createElement('td');
            slugTd.style.cssText = _surferTdStyle + 'font-family:monospace;font-size:12px;';
            slugTd.textContent = qi.slug || qi.content_id;
            row.appendChild(slugTd);

            var sheetTd = document.createElement('td');
            sheetTd.style.cssText = _surferTdStyle;
            sheetTd.textContent = qi.content_id;
            row.appendChild(sheetTd);

            var scoreTd = document.createElement('td');
            scoreTd.style.cssText = _surferTdStyle;
            var scoreColor = qi.surfer_score >= 70 ? '#48bb78' : qi.surfer_score >= 40 ? '#f6ad55' : '#fc8181';
            var scoreBadge = document.createElement('span');
            scoreBadge.style.cssText = _surferBadgeStyle + 'background:transparent;color:' + scoreColor + ';font-weight:700;font-size:14px;';
            scoreBadge.textContent = qi.surfer_score || 0;
            scoreTd.appendChild(scoreBadge);
            row.appendChild(scoreTd);

            var editTd = document.createElement('td');
            editTd.style.cssText = _surferTdStyle;
            var editBtn = document.createElement('button');
            editBtn.style.cssText = _surferBtnSmStyle + 'background:transparent;color:#6c5ce7;border:1px solid #6c5ce7;';
            editBtn.textContent = 'Edit Mapping';
            (function(contentId) {
                editBtn.onclick = function() { surferShowEditMappingModal(contentId, sheets, function() { load_seo_mapping(); }); };
            })(qi.content_id);
            editTd.appendChild(editBtn);
            row.appendChild(editTd);

            mtbody.appendChild(row);
        });

        mappedTable.appendChild(mtbody);
        main.appendChild(mappedTable);
    }
}

function surferShowEditMappingModal(contentId, sheets, onSaved) {
    var overlay = document.createElement('div');
    overlay.style.cssText = 'position:fixed;inset:0;background:rgba(0,0,0,0.7);z-index:9998;display:flex;align-items:center;justify-content:center;';

    var panel = document.createElement('div');
    panel.style.cssText = 'background:#0d0d1a;border:1px solid #333;border-radius:12px;padding:24px;width:480px;max-width:95vw;max-height:80vh;overflow-y:auto;';

    var ph2 = document.createElement('h3');
    ph2.style.cssText = 'margin:0 0 16px;color:#e2e8f0;font-size:16px;';
    ph2.textContent = 'Edit Mapping';
    panel.appendChild(ph2);

    var desc = document.createElement('p');
    desc.style.cssText = 'font-size:12px;color:#a0aec0;margin:0 0 12px;font-family:monospace;';
    desc.textContent = 'Page: ' + contentId;
    panel.appendChild(desc);

    // Primary sheet select
    var primaryLabel = document.createElement('label');
    primaryLabel.style.cssText = 'display:block;font-size:12px;color:#a0aec0;margin-bottom:4px;';
    primaryLabel.textContent = 'Primary Sheet';
    panel.appendChild(primaryLabel);
    var primarySelect = document.createElement('select');
    primarySelect.style.cssText = _surferInputStyle + 'margin-bottom:16px;';
    var emptyOpt = document.createElement('option');
    emptyOpt.value = '';
    emptyOpt.textContent = '— Select primary sheet —';
    primarySelect.appendChild(emptyOpt);
    sheets.forEach(function(s) {
        var opt = document.createElement('option');
        opt.value = s.sheet_id;
        opt.textContent = s.topic + ' (' + s.industry + ')';
        primarySelect.appendChild(opt);
    });
    panel.appendChild(primarySelect);

    var errMsg = document.createElement('div');
    errMsg.style.cssText = 'color:#fc8181;font-size:12px;margin-bottom:8px;display:none;';
    panel.appendChild(errMsg);

    var btnRow = document.createElement('div');
    btnRow.style.cssText = 'display:flex;gap:8px;justify-content:flex-end;';
    var cancelBtn = document.createElement('button');
    cancelBtn.style.cssText = _surferBtnGhostStyle;
    cancelBtn.textContent = 'Cancel';
    cancelBtn.onclick = function() { document.body.removeChild(overlay); };
    btnRow.appendChild(cancelBtn);
    var saveBtn = document.createElement('button');
    saveBtn.style.cssText = _surferBtnStyle;
    saveBtn.textContent = 'Save';
    saveBtn.onclick = async function() {
        errMsg.style.display = 'none';
        var primaryId = primarySelect.value;
        if (!primaryId) { errMsg.textContent = 'Please select a primary sheet.'; errMsg.style.display = 'block'; return; }
        saveBtn.disabled = true;
        var r = await surferFetch('/api/modules/seo/surfer/map/' + encodeURIComponent(contentId), {
            method: 'PUT',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ sheet_ids: [primaryId], primary_sheet_id: primaryId }),
        });
        saveBtn.disabled = false;
        if (!r.ok) { errMsg.textContent = r.message || 'Save failed'; errMsg.style.display = 'block'; return; }
        document.body.removeChild(overlay);
        if (typeof showToast === 'function') showToast('Mapping saved', 'success');
        else if (typeof toast === 'function') toast('Mapping saved');
        if (onSaved) onSaved();
    };
    btnRow.appendChild(saveBtn);
    panel.appendChild(btnRow);

    overlay.appendChild(panel);
    overlay.onclick = function(e) { if (e.target === overlay) document.body.removeChild(overlay); };
    document.body.appendChild(overlay);
}

// ════════════════════════════════════════════════════════════════════════════
// VIEW 3 — AI Queue
// ════════════════════════════════════════════════════════════════════════════

var _surferQueueData       = [];
var _surferQueueSort       = { col: 'priority', dir: 'asc' };
var _surferQueuePhaseFilter = 'all';

async function load_seo_queue() {
    var main = document.getElementById('adminMain');
    if (!main) return;
    main.textContent = '';

    // Header
    var header = document.createElement('div');
    header.style.cssText = 'display:flex;align-items:center;justify-content:space-between;margin-bottom:20px;flex-wrap:wrap;gap:12px;';
    var h2 = document.createElement('h2');
    h2.style.cssText = 'margin:0;font-size:1.4rem;color:#e2e8f0;';
    h2.textContent = 'AI Content Queue';
    header.appendChild(h2);

    var headerBtns = document.createElement('div');
    headerBtns.style.cssText = 'display:flex;gap:8px;flex-wrap:wrap;';

    var genBtn = document.createElement('button');
    genBtn.style.cssText = _surferBtnStyle;
    genBtn.textContent = 'Generate Queue';
    genBtn.onclick = async function() {
        genBtn.disabled = true;
        genBtn.textContent = 'Generating...';
        var r = await surferFetch('/api/modules/seo/surfer/queue/generate', { method: 'POST' });
        genBtn.disabled = false;
        genBtn.textContent = 'Generate Queue';
        if (typeof showToast === 'function') showToast(r.message || (r.ok ? 'Queue generated' : 'Failed'), r.ok ? 'success' : 'error');
        else if (typeof toast === 'function') toast(r.message || (r.ok ? 'Queue generated' : 'Failed'));
        if (r.ok) load_seo_queue();
    };
    headerBtns.appendChild(genBtn);

    var exportBtn = document.createElement('button');
    exportBtn.style.cssText = _surferBtnGhostStyle;
    exportBtn.textContent = 'Export JSON';
    exportBtn.onclick = function() {
        var dataStr = JSON.stringify(_surferQueueData, null, 2);
        var blob = new Blob([dataStr], { type: 'application/json' });
        var a = document.createElement('a');
        a.href = URL.createObjectURL(blob);
        a.download = 'seo-queue-' + new Date().toISOString().slice(0, 10) + '.json';
        a.click();
        URL.revokeObjectURL(a.href);
    };
    headerBtns.appendChild(exportBtn);

    header.appendChild(headerBtns);
    main.appendChild(header);

    // Load data
    var loadMsg = document.createElement('p');
    loadMsg.style.cssText = 'color:#a0aec0;font-size:13px;';
    loadMsg.textContent = 'Loading queue...';
    main.appendChild(loadMsg);

    var r = await surferFetch('/api/modules/seo/surfer/queue');
    _surferQueueData = (r.ok && Array.isArray(r.data)) ? r.data : [];
    main.removeChild(loadMsg);

    // ── Stats cards ──
    var phases = { Pending: 0, ContentAiInProgress: 0, ContentAiDone: 0, ReviewAiDone: 0, Published: 0, Error: 0 };
    var needsHuman = 0;
    _surferQueueData.forEach(function(qi) {
        var p = (typeof qi.phase === 'string') ? qi.phase : (qi.phase && qi.phase.Pending !== undefined ? 'Pending' : JSON.stringify(qi.phase));
        // Phase might be an enum string like "Pending" or {"Pending": null}
        var phaseStr = surferQueuePhaseLabel(qi.phase);
        if (phases[phaseStr] !== undefined) phases[phaseStr]++;
        else if (phaseStr === 'ContentAiInProgress') phases['ContentAiInProgress']++;
        if (qi.needs_human_review) needsHuman++;
    });

    var statsGrid = document.createElement('div');
    statsGrid.style.cssText = 'display:grid;grid-template-columns:repeat(auto-fill,minmax(120px,1fr));gap:10px;margin-bottom:20px;';

    var statDefs = [
        ['Total', _surferQueueData.length, '#e2e8f0'],
        ['Pending', phases.Pending, '#a0aec0'],
        ['AI In Progress', phases.ContentAiInProgress, '#f6ad55'],
        ['AI Done', phases.ContentAiDone, '#48bb78'],
        ['Published', phases.Published, '#6c5ce7'],
        ['Errors', phases.Error, '#fc8181'],
        ['Needs Review', needsHuman, '#f6e05e'],
    ];
    statDefs.forEach(function(def) {
        statsGrid.appendChild(surferStatCard(def[0], def[1], def[2]));
    });
    main.appendChild(statsGrid);

    // ── Phase filter tabs ──
    var tabBar = document.createElement('div');
    tabBar.style.cssText = 'display:flex;border-bottom:1px solid #333;margin-bottom:16px;flex-wrap:wrap;';
    var tabDefs = ['all', 'Pending', 'ContentAiInProgress', 'ContentAiDone', 'Published', 'Error'];
    var tabLabels = { all: 'All', Pending: 'Pending', ContentAiInProgress: 'In Progress', ContentAiDone: 'AI Done', Published: 'Published', Error: 'Errors' };
    var tabEls = {};

    tabDefs.forEach(function(tab) {
        var btn = document.createElement('button');
        btn.style.cssText = tab === _surferQueuePhaseFilter ? _surferTabActiveStyle : _surferTabStyle;
        btn.textContent = tabLabels[tab] || tab;
        btn.onclick = function() {
            _surferQueuePhaseFilter = tab;
            tabDefs.forEach(function(t) {
                if (tabEls[t]) tabEls[t].style.cssText = t === tab ? _surferTabActiveStyle : _surferTabStyle;
            });
            surferRenderQueueTable(tableContainer);
        };
        tabEls[tab] = btn;
        tabBar.appendChild(btn);
    });
    main.appendChild(tabBar);

    // ── Table ──
    var tableContainer = document.createElement('div');
    main.appendChild(tableContainer);
    surferRenderQueueTable(tableContainer);
}

function surferQueuePhaseLabel(phase) {
    if (typeof phase === 'string') return phase;
    if (phase === null || phase === undefined) return 'Pending';
    if (typeof phase === 'object') {
        var keys = Object.keys(phase);
        if (keys.length > 0) return keys[0];
    }
    return String(phase);
}

function surferRenderQueueTable(container) {
    container.textContent = '';

    var phaseFilter = _surferQueuePhaseFilter;
    var filtered = phaseFilter === 'all'
        ? _surferQueueData
        : _surferQueueData.filter(function(qi) { return surferQueuePhaseLabel(qi.phase) === phaseFilter; });

    if (filtered.length === 0) {
        container.appendChild(surferEmptyState('No queue items match this filter. Click "Generate Queue" to populate it.'));
        return;
    }

    // Sort
    var sorted = filtered.slice().sort(function(a, b) {
        var col = _surferQueueSort.col;
        var va = a[col], vb = b[col];
        if (va == null) va = typeof vb === 'number' ? 999 : '';
        if (vb == null) vb = typeof va === 'number' ? 999 : '';
        if (typeof va === 'number' && typeof vb === 'number') {
            return _surferQueueSort.dir === 'asc' ? va - vb : vb - va;
        }
        var sa = String(va).toLowerCase(), sb = String(vb).toLowerCase();
        if (sa < sb) return _surferQueueSort.dir === 'asc' ? -1 : 1;
        if (sa > sb) return _surferQueueSort.dir === 'asc' ? 1 : -1;
        return 0;
    });

    var cols = [
        { key: 'slug',               label: 'Slug' },
        { key: 'page_type',          label: 'Type' },
        { key: 'priority',           label: 'Priority' },
        { key: 'surfer_score',       label: 'Surfer' },
        { key: 'seo_score',          label: 'SEO' },
        { key: 'word_count',         label: 'Words' },
        { key: 'phase',              label: 'Phase' },
        { key: 'needs_human_review', label: 'Review?' },
    ];

    var table = document.createElement('table');
    table.style.cssText = _surferTableStyle;

    // Thead
    var thead = document.createElement('thead');
    var hrow = document.createElement('tr');
    cols.forEach(function(c) {
        var th = document.createElement('th');
        var isActive = _surferQueueSort.col === c.key;
        th.style.cssText = _surferThStyle + (isActive ? 'color:#6c5ce7;' : '');
        th.textContent = c.label + (isActive ? (_surferQueueSort.dir === 'asc' ? ' \u25B2' : ' \u25BC') : '');
        (function(key) {
            th.onclick = function() {
                if (_surferQueueSort.col === key) {
                    _surferQueueSort.dir = _surferQueueSort.dir === 'asc' ? 'desc' : 'asc';
                } else {
                    _surferQueueSort.col = key;
                    _surferQueueSort.dir = 'asc';
                }
                surferRenderQueueTable(container);
            };
        })(c.key);
        hrow.appendChild(th);
    });
    var actionTh = document.createElement('th');
    actionTh.style.cssText = _surferThStyle;
    actionTh.textContent = 'Actions';
    hrow.appendChild(actionTh);
    thead.appendChild(hrow);
    table.appendChild(thead);

    // Tbody
    var tbody = document.createElement('tbody');
    sorted.forEach(function(qi) {
        var row = document.createElement('tr');
        row.style.cssText = 'cursor:pointer;';
        row.onmouseenter = function() { this.style.background = '#1a1a2e'; };
        row.onmouseleave = function() { this.style.background = ''; };

        // slug
        var slugTd = document.createElement('td');
        slugTd.style.cssText = _surferTdStyle + 'font-family:monospace;font-size:12px;max-width:220px;overflow:hidden;text-overflow:ellipsis;white-space:nowrap;';
        slugTd.textContent = qi.slug || qi.content_id;
        slugTd.title = qi.slug || qi.content_id;
        row.appendChild(slugTd);

        // page_type
        var typeTd = document.createElement('td');
        typeTd.style.cssText = _surferTdStyle;
        var typeBadge = document.createElement('span');
        typeBadge.style.cssText = _surferBadgeStyle;
        typeBadge.textContent = qi.page_type || '—';
        typeTd.appendChild(typeBadge);
        row.appendChild(typeTd);

        // priority
        var priTd = document.createElement('td');
        priTd.style.cssText = _surferTdStyle;
        var priColor = qi.priority === 1 ? '#fc8181' : qi.priority === 2 ? '#f6ad55' : '#a0aec0';
        var priBadge = document.createElement('span');
        priBadge.style.cssText = 'font-weight:700;color:' + priColor + ';';
        priBadge.textContent = qi.priority != null ? qi.priority : '—';
        priTd.appendChild(priBadge);
        row.appendChild(priTd);

        // surfer_score
        var surfTd = document.createElement('td');
        surfTd.style.cssText = _surferTdStyle;
        if (qi.surfer_score != null) {
            var sc = qi.surfer_score;
            var scColor = sc >= 70 ? '#48bb78' : sc >= 40 ? '#f6ad55' : '#fc8181';
            var scSpan = document.createElement('span');
            scSpan.style.cssText = 'font-weight:600;color:' + scColor + ';';
            scSpan.textContent = sc;
            surfTd.appendChild(scSpan);
        } else {
            surfTd.textContent = '—';
        }
        row.appendChild(surfTd);

        // seo_score
        var seoTd = document.createElement('td');
        seoTd.style.cssText = _surferTdStyle;
        if (qi.seo_score != null) {
            var ss = qi.seo_score;
            var ssColor = ss >= 70 ? '#48bb78' : ss >= 40 ? '#f6ad55' : '#fc8181';
            var ssSpan = document.createElement('span');
            ssSpan.style.cssText = 'font-weight:600;color:' + ssColor + ';';
            ssSpan.textContent = ss;
            seoTd.appendChild(ssSpan);
        } else {
            seoTd.textContent = '—';
        }
        row.appendChild(seoTd);

        // word_count
        var wcTd = document.createElement('td');
        wcTd.style.cssText = _surferTdStyle;
        wcTd.textContent = qi.word_count != null ? qi.word_count.toLocaleString() : '—';
        row.appendChild(wcTd);

        // phase
        var phaseTd = document.createElement('td');
        phaseTd.style.cssText = _surferTdStyle;
        var phaseStr = surferQueuePhaseLabel(qi.phase);
        var phaseColors = {
            Pending: '#a0aec0', ContentAiInProgress: '#f6ad55', ContentAiDone: '#48bb78',
            ReviewAiInProgress: '#f6ad55', ReviewAiDone: '#68d391', Published: '#6c5ce7', Error: '#fc8181'
        };
        var phaseBadge = document.createElement('span');
        phaseBadge.style.cssText = _surferBadgeStyle + 'color:' + (phaseColors[phaseStr] || '#a0aec0') + ';background:transparent;';
        phaseBadge.textContent = phaseStr.replace(/([A-Z])/g, ' $1').trim();
        phaseTd.appendChild(phaseBadge);
        row.appendChild(phaseTd);

        // needs_human_review
        var reviewTd = document.createElement('td');
        reviewTd.style.cssText = _surferTdStyle;
        if (qi.needs_human_review) {
            var reviewBadge = document.createElement('span');
            reviewBadge.style.cssText = _surferBadgeStyle + 'background:rgba(246,224,94,0.15);color:#f6e05e;';
            reviewBadge.textContent = 'Review';
            reviewTd.appendChild(reviewBadge);
        } else {
            reviewTd.textContent = '—';
        }
        row.appendChild(reviewTd);

        // Actions
        var actTd = document.createElement('td');
        actTd.style.cssText = _surferTdStyle;
        var actRow = document.createElement('div');
        actRow.style.cssText = 'display:flex;gap:6px;';

        var viewBtn = document.createElement('button');
        viewBtn.style.cssText = _surferBtnSmStyle + 'background:transparent;color:#6c5ce7;border:1px solid #6c5ce7;';
        viewBtn.textContent = 'Intelligence';
        (function(contentId) {
            viewBtn.onclick = function(e) { e.stopPropagation(); surferShowIntelligence(contentId); };
        })(qi.content_id);
        actRow.appendChild(viewBtn);

        if (qi.needs_human_review) {
            var approveBtn = document.createElement('button');
            approveBtn.style.cssText = _surferBtnSmStyle + 'background:#48bb78;';
            approveBtn.textContent = 'Approve';
            (function(contentId, tableRef) {
                approveBtn.onclick = async function(e) {
                    e.stopPropagation();
                    approveBtn.disabled = true;
                    var ar = await surferFetch('/api/modules/seo/surfer/queue/' + encodeURIComponent(contentId) + '/approve', {
                        method: 'POST',
                    });
                    approveBtn.disabled = false;
                    if (typeof showToast === 'function') showToast(ar.message || (ar.ok ? 'Approved' : 'Failed'), ar.ok ? 'success' : 'error');
                    else if (typeof toast === 'function') toast(ar.message || (ar.ok ? 'Approved' : 'Failed'));
                    if (ar.ok) {
                        // Update local data and re-render
                        _surferQueueData.forEach(function(item) {
                            if (item.content_id === contentId) item.needs_human_review = false;
                        });
                        surferRenderQueueTable(tableRef);
                    }
                };
            })(qi.content_id, container);
            actRow.appendChild(approveBtn);
        }

        actTd.appendChild(actRow);
        row.appendChild(actTd);

        // Click row → show intelligence
        row.onclick = (function(contentId) {
            return function() { surferShowIntelligence(contentId); };
        })(qi.content_id);

        tbody.appendChild(row);
    });

    table.appendChild(tbody);
    container.appendChild(table);
}

async function surferShowIntelligence(contentId) {
    var main = document.getElementById('adminMain');
    if (!main) return;

    // Modal overlay
    var overlay = document.createElement('div');
    overlay.style.cssText = 'position:fixed;inset:0;background:rgba(0,0,0,0.75);z-index:9998;display:flex;align-items:center;justify-content:center;padding:16px;';

    var panel = document.createElement('div');
    panel.style.cssText = 'background:#0d0d1a;border:1px solid #333;border-radius:12px;padding:24px;width:700px;max-width:95vw;max-height:85vh;overflow-y:auto;';

    var panelHeader = document.createElement('div');
    panelHeader.style.cssText = 'display:flex;align-items:center;justify-content:space-between;margin-bottom:16px;';
    var panelH = document.createElement('h3');
    panelH.style.cssText = 'margin:0;color:#e2e8f0;font-size:16px;';
    panelH.textContent = 'Page Intelligence';
    panelHeader.appendChild(panelH);
    var closeBtn = document.createElement('button');
    closeBtn.style.cssText = 'background:transparent;border:none;color:#a0aec0;font-size:20px;cursor:pointer;padding:4px 8px;';
    closeBtn.textContent = '\u00D7';
    closeBtn.onclick = function() { document.body.removeChild(overlay); };
    panelHeader.appendChild(closeBtn);
    panel.appendChild(panelHeader);

    var loading = document.createElement('p');
    loading.style.cssText = 'color:#a0aec0;font-size:13px;';
    loading.textContent = 'Loading intelligence data...';
    panel.appendChild(loading);

    overlay.appendChild(panel);
    overlay.onclick = function(e) { if (e.target === overlay) document.body.removeChild(overlay); };
    document.body.appendChild(overlay);

    var r = await surferFetch('/api/modules/seo/intelligence/' + encodeURIComponent(contentId));
    panel.removeChild(loading);

    if (!r.ok || !r.data) {
        var errEl = document.createElement('p');
        errEl.style.cssText = 'color:#fc8181;font-size:13px;';
        errEl.textContent = r.message || 'No intelligence data available for this page.';
        panel.appendChild(errEl);
        return;
    }

    var pre = document.createElement('pre');
    pre.style.cssText = 'background:#000;border:1px solid #333;border-radius:6px;padding:16px;font-size:12px;color:#a0d8ef;overflow-x:auto;white-space:pre-wrap;word-break:break-word;max-height:500px;overflow-y:auto;';
    pre.textContent = JSON.stringify(r.data, null, 2);
    panel.appendChild(pre);
}

// ── Admin view registrations ────────────────────────────────────────────────

if (typeof window !== 'undefined') {
    window.moduleViews = window.moduleViews || {};
    window.moduleViews['seo-surfer']  = load_seo_surfer;
    window.moduleViews['seo-mapping'] = load_seo_mapping;
    window.moduleViews['seo-queue']   = load_seo_queue;
}

if (typeof registerAdminView === 'function') {
    registerAdminView('seo-surfer',  load_seo_surfer);
    registerAdminView('seo-mapping', load_seo_mapping);
    registerAdminView('seo-queue',   load_seo_queue);
}
"##;
