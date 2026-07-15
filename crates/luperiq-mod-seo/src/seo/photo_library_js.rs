//! Admin SPA loader for the SEO Photo Review view (Phase 7 / 2026-05-27).
//!
//! Rendered into the in-shell admin via `SeoModule::admin_js`. Mirrors the
//! standalone HTML page at `/admin/seo/photo-review` but lives inside the
//! existing admin layout so reviewers don't have to context-switch.

pub(crate) const PHOTO_REVIEW_ADMIN_JS: &str = r##"
async function load_seo_photo_review() {
    const main = document.getElementById('adminMain');
    if (!main) return;
    main.textContent = '';

    const wrap = document.createElement('div');
    const head = document.createElement('div');
    head.style.cssText = 'display:flex;align-items:center;justify-content:space-between;margin-bottom:14px';
    const h = document.createElement('h2');
    h.textContent = 'SEO Photo Review';
    head.appendChild(h);

    const tabs = document.createElement('div');
    tabs.style.cssText = 'display:flex;gap:8px';
    function makeTab(status, label) {
        const b = document.createElement('button');
        b.className = 'btn btn-ghost btn-sm';
        b.textContent = label;
        b.dataset.status = status;
        b.onclick = () => loadStatus(status, b);
        return b;
    }
    const tabPending  = makeTab('pending',  'Pending');
    const tabApproved = makeTab('approved', 'Approved');
    const tabRejected = makeTab('rejected', 'Rejected');
    tabs.appendChild(tabPending);
    tabs.appendChild(tabApproved);
    tabs.appendChild(tabRejected);
    head.appendChild(tabs);
    wrap.appendChild(head);

    const grid = document.createElement('div');
    grid.style.cssText = 'display:grid;grid-template-columns:repeat(auto-fill,minmax(280px,1fr));gap:14px';
    wrap.appendChild(grid);

    main.appendChild(wrap);

    function highlight(active) {
        [tabPending, tabApproved, tabRejected].forEach(b => {
            b.className = 'btn btn-' + (b === active ? 'primary' : 'ghost') + ' btn-sm';
        });
    }

    async function loadStatus(status, activeBtn) {
        highlight(activeBtn);
        grid.textContent = '';
        const r = await fetch('/api/modules/seo/photo-review?status=' + encodeURIComponent(status), {credentials:'include'})
            .then(r => r.json())
            .catch(() => ({ok:false}));
        if (!r.ok) {
            const e = document.createElement('div');
            e.className = 'muted';
            e.textContent = 'Failed to load. Do you have the tenant.seo.review capability?';
            grid.appendChild(e);
            return;
        }
        if (!r.items || r.items.length === 0) {
            const e = document.createElement('div');
            e.className = 'muted';
            e.style.cssText = 'grid-column:1/-1;padding:24px;text-align:center';
            e.textContent = 'No photos in this state.';
            grid.appendChild(e);
            return;
        }
        r.items.forEach(item => grid.appendChild(renderCard(item, status)));
    }

    function renderCard(item, status) {
        const card = document.createElement('div');
        card.className = 'card';
        card.style.cssText = 'border:1px solid #334155;border-radius:10px;padding:12px;background:#0f172a';

        const img = document.createElement('img');
        img.src = item.image_url;
        img.alt = item.notes || '';
        img.style.cssText = 'width:100%;aspect-ratio:1/1;object-fit:cover;border-radius:8px;background:#1e293b;margin-bottom:8px';
        card.appendChild(img);

        function row(label, value) {
            const r = document.createElement('div');
            r.style.cssText = 'font-size:12px;margin-bottom:4px';
            const l = document.createElement('span'); l.style.color='#94a3b8'; l.textContent = label + ': ';
            const v = document.createElement('span'); v.style.color='#e2e8f0'; v.textContent = value || '—';
            r.appendChild(l); r.appendChild(v);
            return r;
        }
        card.appendChild(row('Pest', item.pest_type));
        card.appendChild(row('ZIP',  item.location_zip));
        if (item.notes) card.appendChild(row('Notes', item.notes));
        if (item.caption) card.appendChild(row('Caption', item.caption));
        if (item.reject_reason) card.appendChild(row('Rejected', item.reject_reason));

        if (status === 'pending') {
            const actions = document.createElement('div');
            actions.style.cssText = 'display:flex;gap:8px;margin-top:10px';
            const ap = document.createElement('button');
            ap.className = 'btn btn-primary btn-sm';
            ap.textContent = 'Approve';
            ap.onclick = async () => {
                const caption = window.prompt('Optional caption for SEO use:', item.notes || '');
                const r = await fetch('/api/modules/seo/photo-review/' + encodeURIComponent(item.photo_id) + '/approve', {
                    method:'POST', credentials:'include',
                    headers:{'Content-Type':'application/json'},
                    body: JSON.stringify({caption: caption || null})
                }).then(r => r.json()).catch(() => ({ok:false}));
                if (r.ok) card.style.opacity = '0.4';
                else alert('Approve failed: ' + (r.error || ''));
            };
            const rj = document.createElement('button');
            rj.className = 'btn btn-ghost btn-sm';
            rj.textContent = 'Reject';
            rj.onclick = async () => {
                const reason = window.prompt('Reason for rejection (required):', '');
                if (!reason) return;
                const r = await fetch('/api/modules/seo/photo-review/' + encodeURIComponent(item.photo_id) + '/reject', {
                    method:'POST', credentials:'include',
                    headers:{'Content-Type':'application/json'},
                    body: JSON.stringify({reason: reason})
                }).then(r => r.json()).catch(() => ({ok:false}));
                if (r.ok) card.style.opacity = '0.4';
                else alert('Reject failed: ' + (r.error || ''));
            };
            actions.appendChild(ap);
            actions.appendChild(rj);
            card.appendChild(actions);
        }

        return card;
    }

    loadStatus('pending', tabPending);
}
"##;
