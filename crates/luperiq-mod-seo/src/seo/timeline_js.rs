//! Inline JavaScript for the SEO Change Timeline admin panel.

pub(crate) const SEO_TIMELINE_JS: &str = r##"
// ── SEO Change Timeline ──────────────────────────────────────────────

function seoChangeTypeBadge(changeType) {
    var badge = document.createElement('span');
    badge.className = 'status-badge';
    var colors = {
        'SlugChange': 'background:rgba(59,130,246,0.15);color:#3b82f6',
        'KeywordChange': 'background:rgba(34,197,94,0.15);color:#22c55e',
        'TitleChange': 'background:rgba(245,158,11,0.15);color:#f59e0b',
        'DescriptionChange': 'background:rgba(245,158,11,0.15);color:#f59e0b',
        'SchemaChange': 'background:rgba(168,85,247,0.15);color:#a855f7',
        'AbVariantStart': 'background:rgba(168,85,247,0.15);color:#a855f7',
        'AbVariantEnd': 'background:rgba(168,85,247,0.15);color:#a855f7'
    };
    badge.style.cssText = colors[changeType] || 'background:rgba(100,116,139,0.15);color:#64748b';
    var labels = {
        'SlugChange': 'URL Change',
        'KeywordChange': 'Keyword',
        'TitleChange': 'Title',
        'DescriptionChange': 'Description',
        'SchemaChange': 'Schema',
        'AbVariantStart': 'A/B Start',
        'AbVariantEnd': 'A/B End'
    };
    badge.textContent = labels[changeType] || changeType;
    return badge;
}

async function renderSeoTimeline(container) {
    container.textContent = '';

    var header = document.createElement('div');
    header.style.cssText = 'display:flex;justify-content:space-between;align-items:center;margin-bottom:16px;';
    var h = document.createElement('h3'); h.textContent = 'SEO Change Timeline'; h.style.margin = '0';
    header.appendChild(h);
    container.appendChild(header);

    // ── Filter controls ──
    var filters = document.createElement('div');
    filters.style.cssText = 'display:flex;gap:8px;margin-bottom:12px;flex-wrap:wrap;align-items:center;';

    var typeSelect = document.createElement('select');
    typeSelect.className = 'admin-input';
    typeSelect.style.cssText = 'width:auto;font-size:12px;padding:4px 8px;';
    ['All Types','SlugChange','TitleChange','DescriptionChange','KeywordChange','SchemaChange','AbVariantStart','AbVariantEnd'].forEach(function(t) {
        var o = document.createElement('option'); o.value = t === 'All Types' ? '' : t; o.textContent = t; typeSelect.appendChild(o);
    });
    filters.appendChild(typeSelect);

    var filterBtn = document.createElement('button');
    filterBtn.className = 'btn btn-ghost btn-sm';
    filterBtn.textContent = 'Filter';
    filterBtn.style.fontSize = '12px';
    filterBtn.onclick = function() { renderSeoTimeline(container); };
    filters.appendChild(filterBtn);
    container.appendChild(filters);

    var params = 'limit=100';
    if (typeSelect.value) params += '&change_type=' + typeSelect.value;
    var r = await fetch('/api/modules/seo/timeline?' + params).then(function(r) { return r.json(); });
    if (!r.ok || !r.data) {
        var empty = document.createElement('p');
        empty.textContent = r.message || 'No changes recorded yet.';
        empty.style.color = 'var(--text-muted)';
        container.appendChild(empty);
        return;
    }

    var changes = Array.isArray(r.data) ? r.data : [];
    if (changes.length === 0) {
        var empty = document.createElement('p');
        empty.textContent = 'No SEO changes recorded yet. Changes are tracked automatically when you edit SEO meta.';
        empty.style.color = 'var(--text-muted)';
        container.appendChild(empty);
        return;
    }

    var list = document.createElement('div');
    list.style.cssText = 'display:flex;flex-direction:column;gap:8px;';

    changes.forEach(function(change) {
        var card = document.createElement('div');
        card.className = 'admin-card';
        card.style.cssText = 'padding:12px 16px;';

        var top = document.createElement('div');
        top.style.cssText = 'display:flex;align-items:center;gap:8px;margin-bottom:6px;';
        top.appendChild(seoChangeTypeBadge(change.change_type));

        var date = document.createElement('span');
        date.style.cssText = 'font-size:12px;color:var(--text-muted);';
        date.textContent = new Date(change.timestamp * 1000).toLocaleString();
        top.appendChild(date);

        var cid = document.createElement('span');
        cid.style.cssText = 'font-size:11px;color:var(--text-muted);margin-left:auto;';
        cid.textContent = change.content_id;
        top.appendChild(cid);

        card.appendChild(top);

        var diff = document.createElement('div');
        diff.style.cssText = 'font-size:13px;';

        var oldSpan = document.createElement('span');
        oldSpan.style.cssText = 'text-decoration:line-through;color:var(--text-muted);';
        oldSpan.textContent = truncateStr(change.old_value, 80);

        var arrow = document.createTextNode(' \u2192 ');

        var newSpan = document.createElement('span');
        newSpan.style.fontWeight = '500';
        newSpan.textContent = truncateStr(change.new_value, 80);

        diff.appendChild(oldSpan);
        diff.appendChild(arrow);
        diff.appendChild(newSpan);
        card.appendChild(diff);

        // Show before/after performance if available
        if (change.snapshot_before || change.snapshot_after) {
            var perf = document.createElement('div');
            perf.style.cssText = 'margin-top:8px;font-size:12px;color:var(--text-muted);display:flex;gap:16px;';
            if (change.snapshot_before) {
                var before = document.createElement('span');
                before.textContent = 'Before: ' + change.snapshot_before.impressions + ' imp, pos ' + (change.snapshot_before.avg_position || 0).toFixed(1);
                perf.appendChild(before);
            }
            if (change.snapshot_after) {
                var after = document.createElement('span');
                after.textContent = 'After: ' + change.snapshot_after.impressions + ' imp, pos ' + (change.snapshot_after.avg_position || 0).toFixed(1);
                perf.appendChild(after);
            }
            card.appendChild(perf);
        }

        list.appendChild(card);
    });

    container.appendChild(list);
}

function truncateStr(s, max) {
    if (!s) return '';
    return s.length > max ? s.substring(0, max) + '...' : s;
}

// ── Admin view registration ──────────────────────────────────────────

function load_seo_timeline() {
    var container = document.getElementById('adminMain') || document.getElementById('module-content');
    if (!container) return;
    while (container.firstChild) container.removeChild(container.firstChild);
    renderSeoTimeline(container);
}

if (typeof registerAdminView === 'function') {
    registerAdminView('seo-timeline', load_seo_timeline);
}
"##;
