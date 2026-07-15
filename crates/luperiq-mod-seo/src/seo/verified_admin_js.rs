//! Inline JavaScript for the Verified Content admin panel (drift detection, hash comparison).

pub(crate) const VERIFIED_ADMIN_JS: &str = r##"
// ── Verified Content admin view ───────────────────────────────────────────────
// Security: all rendering uses DOM methods (createElement/textContent), no innerHTML.

// ── Shared fetch helper ────────────────────────────────────────────────────────

async function verifiedFetch(url, opts) {
    try {
        var r = await fetch(url, opts);
        return await r.json();
    } catch(e) {
        return { ok: false, message: e.message };
    }
}

// ── Helper: clear all children of an element (safe, no innerHTML) ─────────────

function vfClear(el) {
    while (el.firstChild) el.removeChild(el.firstChild);
}

// ── Style constants ────────────────────────────────────────────────────────────

var _vfBtnStyle      = 'background:#6c5ce7;color:#fff;border:none;border-radius:6px;padding:8px 16px;cursor:pointer;font-size:13px;';
var _vfBtnGreenStyle = 'background:#38a169;color:#fff;border:none;border-radius:6px;padding:4px 10px;cursor:pointer;font-size:12px;';
var _vfTableStyle    = 'width:100%;border-collapse:collapse;font-size:13px;';
var _vfThStyle       = 'padding:8px 12px;border-bottom:2px solid #333;text-align:left;color:#a0aec0;font-weight:600;font-size:12px;';
var _vfTdStyle       = 'padding:8px 12px;border-bottom:1px solid #333;color:#e2e8f0;vertical-align:middle;';
var _vfStatCardStyle = 'background:#1a1a2e;border:1px solid #333;border-radius:8px;padding:16px;flex:1;min-width:100px;';
var _vfLabelStyle    = 'font-size:11px;color:#a0aec0;margin-bottom:6px;text-transform:uppercase;font-weight:600;';
var _vfValueStyle    = 'font-size:24px;font-weight:700;';

// ── Helper: stat card ──────────────────────────────────────────────────────────

function vfStatCard(label, value, color) {
    var card = document.createElement('div');
    card.style.cssText = _vfStatCardStyle;
    var l = document.createElement('div');
    l.style.cssText = _vfLabelStyle;
    l.textContent = label;
    card.appendChild(l);
    var v = document.createElement('div');
    v.style.cssText = _vfValueStyle + 'color:' + (color || '#e2e8f0') + ';';
    v.textContent = String(value);
    card.appendChild(v);
    return card;
}

// ── Helper: drift badge ────────────────────────────────────────────────────────

function vfDriftBadge(status) {
    // status: 'clean' | 'drifted' | 'unverified'
    var badge = document.createElement('span');
    var styles = {
        clean:      'background:#2d6a4f;color:#b7e4c7;',
        drifted:    'background:#7b1d1d;color:#feb2b2;',
        unverified: 'background:#333;color:#a0aec0;'
    };
    badge.style.cssText = 'display:inline-block;padding:2px 8px;border-radius:4px;font-size:11px;font-weight:600;'
                          + (styles[status] || styles.unverified);
    badge.textContent = status.charAt(0).toUpperCase() + status.slice(1);
    return badge;
}

// ── Helper: format unix timestamp ─────────────────────────────────────────────

function vfFmtDate(ts) {
    if (!ts) return '\u2014';
    return new Date(ts * 1000).toLocaleDateString();
}

// ── Helper: truncate hash ──────────────────────────────────────────────────────

function vfShortHash(h) {
    if (!h) return '\u2014';
    return h.slice(0, 12) + '\u2026';
}

// ── Helper: labelled field row for detail panel ───────────────────────────────

function vfField(label, value) {
    var row = document.createElement('div');
    var lbl = document.createElement('div');
    lbl.style.cssText = 'font-size:11px;color:#a0aec0;font-weight:600;margin-bottom:2px;';
    lbl.textContent = label;
    var val = document.createElement('div');
    val.style.cssText = 'color:#e2e8f0;word-break:break-all;font-size:13px;';
    val.textContent = value;
    row.appendChild(lbl);
    row.appendChild(val);
    return row;
}

// ── Helper: slug chip ─────────────────────────────────────────────────────────

function vfSlugChip(text, bgColor, textColor) {
    var chip = document.createElement('span');
    chip.style.cssText = 'display:inline-block;padding:2px 8px;border-radius:4px;font-size:12px;margin:2px;'
                         + 'background:' + (bgColor || '#2d3748') + ';color:' + (textColor || '#a0aec0') + ';';
    chip.textContent = text;
    return chip;
}

// ── Detail panel ───────────────────────────────────────────────────────────────

function vfShowDetail(container, record, driftData) {
    vfClear(container);

    var panel = document.createElement('div');
    panel.style.cssText = 'background:#1a1a2e;border:1px solid #6c5ce7;border-radius:8px;padding:20px;margin-top:16px;';

    // Header row
    var header = document.createElement('div');
    header.style.cssText = 'display:flex;justify-content:space-between;align-items:center;margin-bottom:16px;';
    var titleEl = document.createElement('h3');
    titleEl.style.cssText = 'margin:0;color:#e2e8f0;font-size:16px;';
    titleEl.textContent = record.title_at_verify || record.content_id;
    var closeBtn = document.createElement('button');
    closeBtn.style.cssText = 'background:transparent;border:none;color:#a0aec0;font-size:18px;cursor:pointer;';
    closeBtn.textContent = '\u00d7';
    closeBtn.onclick = function() { vfClear(container); };
    header.appendChild(titleEl);
    header.appendChild(closeBtn);
    panel.appendChild(header);

    // Fields grid
    var grid = document.createElement('div');
    grid.style.cssText = 'display:grid;grid-template-columns:1fr 1fr;gap:12px;';
    grid.appendChild(vfField('Content ID', record.content_id));
    grid.appendChild(vfField('Verified By', record.verified_by || '\u2014'));
    grid.appendChild(vfField('Verified Hash', record.content_hash || '\u2014'));
    grid.appendChild(vfField('Current Hash', driftData ? (driftData.current_hash || '\u2014') : '\u2014'));
    grid.appendChild(vfField('First Verified', vfFmtDate(record.first_verified_at)));
    grid.appendChild(vfField('Last Verified', vfFmtDate(record.latest_verified_at)));
    grid.appendChild(vfField('Words (verified)', String(record.word_count_at_verify || 0)));
    if (driftData && driftData.word_count_change !== null && driftData.word_count_change !== undefined) {
        var wcc = driftData.word_count_change;
        grid.appendChild(vfField('Word Count Change', (wcc > 0 ? '+' : '') + String(wcc)));
    }
    panel.appendChild(grid);

    // Outgoing links
    if (record.internal_links_out && record.internal_links_out.length > 0) {
        var linksSection = document.createElement('div');
        linksSection.style.cssText = 'margin-top:16px;';
        var linksLabel = document.createElement('div');
        linksLabel.style.cssText = 'font-size:11px;color:#a0aec0;font-weight:600;margin-bottom:6px;text-transform:uppercase;';
        linksLabel.textContent = 'Outgoing Links (' + record.internal_links_out.length + ')';
        linksSection.appendChild(linksLabel);
        var chipRow = document.createElement('div');
        chipRow.style.cssText = 'display:flex;flex-wrap:wrap;';
        record.internal_links_out.forEach(function(slug) {
            chipRow.appendChild(vfSlugChip('/' + slug, '#2d3748', '#a0aec0'));
        });
        linksSection.appendChild(chipRow);
        panel.appendChild(linksSection);
    }

    // Links added (drift)
    if (driftData && driftData.links_added && driftData.links_added.length > 0) {
        var addedSection = document.createElement('div');
        addedSection.style.cssText = 'margin-top:12px;';
        var addedLabel = document.createElement('div');
        addedLabel.style.cssText = 'font-size:11px;color:#68d391;font-weight:600;margin-bottom:6px;text-transform:uppercase;';
        addedLabel.textContent = 'Links Added';
        addedSection.appendChild(addedLabel);
        var addedRow = document.createElement('div');
        addedRow.style.cssText = 'display:flex;flex-wrap:wrap;';
        driftData.links_added.forEach(function(slug) {
            addedRow.appendChild(vfSlugChip('+/' + slug, '#2d6a4f', '#b7e4c7'));
        });
        addedSection.appendChild(addedRow);
        panel.appendChild(addedSection);
    }

    // Links removed (drift)
    if (driftData && driftData.links_removed && driftData.links_removed.length > 0) {
        var removedSection = document.createElement('div');
        removedSection.style.cssText = 'margin-top:12px;';
        var removedLabel = document.createElement('div');
        removedLabel.style.cssText = 'font-size:11px;color:#fc8181;font-weight:600;margin-bottom:6px;text-transform:uppercase;';
        removedLabel.textContent = 'Links Removed';
        removedSection.appendChild(removedLabel);
        var removedRow = document.createElement('div');
        removedRow.style.cssText = 'display:flex;flex-wrap:wrap;';
        driftData.links_removed.forEach(function(slug) {
            removedRow.appendChild(vfSlugChip('-/' + slug, '#7b1d1d', '#feb2b2'));
        });
        removedSection.appendChild(removedRow);
        panel.appendChild(removedSection);
    }

    container.appendChild(panel);
}

// ── Main render: seo-verified ──────────────────────────────────────────────────

async function renderVerifiedView(container) {
    vfClear(container);

    // ── Header ────────────────────────────────────────────────────────────────

    var topBar = document.createElement('div');
    topBar.style.cssText = 'display:flex;justify-content:space-between;align-items:center;margin-bottom:20px;';

    var heading = document.createElement('h2');
    heading.style.cssText = 'margin:0;color:#e2e8f0;font-size:18px;font-weight:600;';
    heading.textContent = 'Verified Content';
    topBar.appendChild(heading);

    var checkAllBtn = document.createElement('button');
    checkAllBtn.style.cssText = _vfBtnStyle;
    checkAllBtn.textContent = 'Check All Drift';
    topBar.appendChild(checkAllBtn);
    container.appendChild(topBar);

    // ── Stat cards ────────────────────────────────────────────────────────────

    var statsRow = document.createElement('div');
    statsRow.style.cssText = 'display:flex;gap:12px;margin-bottom:20px;flex-wrap:wrap;';
    var totalCard = vfStatCard('Total Verified', '\u2026', '#e2e8f0');
    var cleanCard = vfStatCard('Clean', '\u2026', '#68d391');
    var driftCard = vfStatCard('Drifted', '\u2026', '#fc8181');
    statsRow.appendChild(totalCard);
    statsRow.appendChild(cleanCard);
    statsRow.appendChild(driftCard);
    container.appendChild(statsRow);

    // ── Detail panel placeholder ──────────────────────────────────────────────

    var detailContainer = document.createElement('div');
    container.appendChild(detailContainer);

    // ── Table ─────────────────────────────────────────────────────────────────

    var tableWrap = document.createElement('div');
    tableWrap.style.cssText = 'background:#1a1a2e;border:1px solid #333;border-radius:8px;overflow:hidden;';
    container.appendChild(tableWrap);

    var table = document.createElement('table');
    table.style.cssText = _vfTableStyle;

    var thead = document.createElement('thead');
    var headRow = document.createElement('tr');
    ['Page', 'Verified', 'Hash', 'Words', 'Links Out', 'Status', 'Actions'].forEach(function(col) {
        var th = document.createElement('th');
        th.style.cssText = _vfThStyle;
        th.textContent = col;
        headRow.appendChild(th);
    });
    thead.appendChild(headRow);
    table.appendChild(thead);

    var tbody = document.createElement('tbody');
    table.appendChild(tbody);
    tableWrap.appendChild(table);

    // ── State ─────────────────────────────────────────────────────────────────

    var _driftCache  = {};  // content_id -> drift report
    var _recordCache = {};  // content_id -> record
    var _rows        = {};  // content_id -> tr element

    // ── Build a single table row ──────────────────────────────────────────────

    function buildRow(rec) {
        var drift  = _driftCache[rec.content_id];
        var status = !drift ? 'unverified' : (drift.has_drifted ? 'drifted' : 'clean');

        var tr = document.createElement('tr');
        tr.style.cssText = 'cursor:pointer;transition:background 0.1s;';
        tr.onmouseover = function() { tr.style.background = 'rgba(255,255,255,0.04)'; };
        tr.onmouseout  = function() { tr.style.background = ''; };

        // Title / slug
        var tdTitle = document.createElement('td');
        tdTitle.style.cssText = _vfTdStyle;
        var nameSpan = document.createElement('div');
        nameSpan.style.cssText = 'font-weight:500;color:#e2e8f0;';
        nameSpan.textContent = rec.title_at_verify || rec.content_id;
        tdTitle.appendChild(nameSpan);
        var idSpan = document.createElement('div');
        idSpan.style.cssText = 'font-size:11px;color:#718096;margin-top:2px;';
        idSpan.textContent = rec.content_id;
        tdTitle.appendChild(idSpan);
        tr.appendChild(tdTitle);

        // Verified date
        var tdDate = document.createElement('td');
        tdDate.style.cssText = _vfTdStyle + 'color:#a0aec0;';
        tdDate.textContent = vfFmtDate(rec.latest_verified_at);
        tr.appendChild(tdDate);

        // Hash
        var tdHash = document.createElement('td');
        tdHash.style.cssText = _vfTdStyle + 'font-family:monospace;color:#718096;font-size:11px;';
        tdHash.textContent = vfShortHash(rec.content_hash);
        tr.appendChild(tdHash);

        // Word count
        var tdWc = document.createElement('td');
        tdWc.style.cssText = _vfTdStyle + 'color:#a0aec0;';
        tdWc.textContent = String(rec.word_count_at_verify || 0);
        tr.appendChild(tdWc);

        // Links out count
        var tdLinks = document.createElement('td');
        tdLinks.style.cssText = _vfTdStyle + 'color:#a0aec0;';
        tdLinks.textContent = String(rec.links_out_count || 0);
        tr.appendChild(tdLinks);

        // Status badge
        var tdStatus = document.createElement('td');
        tdStatus.style.cssText = _vfTdStyle;
        tdStatus.appendChild(vfDriftBadge(status));
        tr.appendChild(tdStatus);

        // Actions
        var tdActions = document.createElement('td');
        tdActions.style.cssText = _vfTdStyle;
        var verifyBtn = document.createElement('button');
        verifyBtn.style.cssText = _vfBtnGreenStyle + 'margin-right:6px;';
        verifyBtn.textContent = 'Verify';
        verifyBtn.onclick = function(e) {
            e.stopPropagation();
            verifyBtn.disabled = true;
            verifyBtn.textContent = '\u2026';
            verifiedFetch('/api/modules/seo/verified/' + encodeURIComponent(rec.content_id) + '/verify', { method: 'POST' })
                .then(function(res) {
                    verifyBtn.disabled = false;
                    verifyBtn.textContent = 'Verify';
                    if (res.ok) {
                        renderVerifiedView(container);
                    } else {
                        alert('Verify failed: ' + (res.message || 'unknown error'));
                    }
                });
        };
        tdActions.appendChild(verifyBtn);
        tr.appendChild(tdActions);

        // Click row -> detail
        tr.onclick = function() {
            vfShowDetail(detailContainer, rec, _driftCache[rec.content_id] || null);
        };

        _rows[rec.content_id] = tr;
        return tr;
    }

    // ── Update stat counters ───────────────────────────────────────────────────

    function updateStats(records) {
        var drifted = 0;
        var clean   = 0;
        records.forEach(function(rec) {
            var d = _driftCache[rec.content_id];
            if (d) { if (d.has_drifted) drifted++; else clean++; }
        });
        totalCard.querySelector('div:last-child').textContent = String(records.length);
        cleanCard.querySelector('div:last-child').textContent = String(clean);
        driftCard.querySelector('div:last-child').textContent = String(drifted);
    }

    // ── Load verified pages ───────────────────────────────────────────────────

    var res = await verifiedFetch('/api/modules/seo/verified');
    if (!res.ok || !Array.isArray(res.data)) {
        var emptyRow = document.createElement('tr');
        var emptyCell = document.createElement('td');
        emptyCell.colSpan = 7;
        emptyCell.style.cssText = 'padding:32px;text-align:center;color:#a0aec0;font-size:14px;';
        emptyCell.textContent = res.message || 'No verified pages yet.';
        emptyRow.appendChild(emptyCell);
        tbody.appendChild(emptyRow);
        updateStats([]);
        return;
    }

    var records = res.data;
    records.forEach(function(rec) {
        _recordCache[rec.content_id] = rec;
        tbody.appendChild(buildRow(rec));
    });
    updateStats(records);

    // ── Check All Drift ────────────────────────────────────────────────────────

    checkAllBtn.onclick = async function() {
        checkAllBtn.disabled = true;
        checkAllBtn.textContent = 'Checking\u2026';
        var bulkRes = await verifiedFetch('/api/modules/seo/verified/bulk-drift', { method: 'POST' });
        checkAllBtn.disabled = false;
        checkAllBtn.textContent = 'Check All Drift';
        if (!bulkRes.ok || !bulkRes.data) {
            alert('Drift check failed: ' + (bulkRes.message || 'unknown error'));
            return;
        }
        var rpts = bulkRes.data.reports || [];
        rpts.forEach(function(rpt) {
            _driftCache[rpt.content_id] = rpt;
            var rec = _recordCache[rpt.content_id];
            if (rec) {
                var oldRow = _rows[rpt.content_id];
                if (oldRow) {
                    var newRow = buildRow(rec);
                    oldRow.parentNode.replaceChild(newRow, oldRow);
                }
            }
        });
        updateStats(records);
    };
}

// ── Hook into admin shell ─────────────────────────────────────────────────────

(function() {
    if (typeof window.__verifiedViewRegistered === 'undefined') {
        window.__verifiedViewRegistered = true;
        document.addEventListener('admin:view:seo-verified', function(e) {
            var container = e.detail && e.detail.container;
            if (container) renderVerifiedView(container);
        });
    }
})();
"##;
