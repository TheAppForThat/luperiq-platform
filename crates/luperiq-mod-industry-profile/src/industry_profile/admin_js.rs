//! Admin JavaScript for the Industry Profile module.
//!
//! View `industry-profiles`:
//! - Stats bar: total profiles, active count, by-category counts
//! - Profile list table: name, slug, category, services count, keywords count, active badge
//! - Click profile to open detail/edit panel with expandable sections
//! - Each section supports Add/Edit/Remove for individual items
//!
//! Security: Uses DOM methods (createElement/textContent) exclusively. No innerHTML
//! with user data. The only innerHTML usage is `main.textContent = ''` to clear
//! the container before repopulating with DOM nodes.

pub const INDUSTRY_PROFILES_ADMIN_JS: &str = r##"
// ── Industry Profiles view ───────────────────────────────────────────
async function load_industry_profiles() {
    const main = document.getElementById('adminMain');

    var _role = (window.__CMS && window.__CMS.nexusRole) || '';
    var _isPro = _role === 'central' || _role === 'professional' || _role === 'enterprise';
    var _isStarter = _isPro || _role === 'starter';

    const r = await fetch('/api/modules/industry-profile/profiles').then(r => r.json());
    const profiles = r.data || [];

    const el = document.createElement('div');

    // Pricing card
    var _pc = lqModulePricingCard({ name: 'Industry Profiles', monthly: 9, annual: 89, lifetime: 249, tier: 'starter', deps: [], slug: 'industry-profile' });
    if (_pc) el.appendChild(_pc);

    // ── Stats bar ─────────────────────────────────────────────────────
    const stats = document.createElement('div');
    stats.className = 'stats-bar';

    const activeCount = profiles.filter(p => p.active).length;
    const cats = {};
    profiles.forEach(p => { cats[p.category] = (cats[p.category] || 0) + 1; });

    const addStat = (label, value) => {
        const s = document.createElement('div');
        s.className = 'stat-card';
        const v = document.createElement('div');
        v.className = 'stat-value';
        v.textContent = value;
        const l = document.createElement('div');
        l.className = 'stat-label';
        l.textContent = label;
        s.appendChild(v);
        s.appendChild(l);
        stats.appendChild(s);
    };

    addStat('Total Profiles', profiles.length);
    addStat('Active', activeCount);
    Object.entries(cats).forEach(([k, v]) => addStat(k.replace('_', ' '), v));
    el.appendChild(stats);

    // ── Toolbar ────────────────────────────────────────────────────────
    const toolbar = document.createElement('div');
    toolbar.className = 'toolbar';
    const h = document.createElement('h2');
    h.textContent = 'Industry Profiles';
    toolbar.appendChild(h);

    const btnGroup = document.createElement('div');
    btnGroup.style.display = 'flex';
    btnGroup.style.gap = '8px';

    const seedBtn = document.createElement('button');
    seedBtn.className = 'btn';
    seedBtn.textContent = 'Seed Defaults';
    if (!_isStarter) { seedBtn.disabled = true; seedBtn.title = 'Upgrade to Starter'; }
    seedBtn.onclick = async () => {
        seedBtn.disabled = true;
        seedBtn.textContent = 'Seeding...';
        const sr = await fetch('/api/modules/industry-profile/seed', { method: 'POST' }).then(r => r.json());
        alert(sr.message);
        load_industry_profiles();
    };
    btnGroup.appendChild(seedBtn);

    const newBtn = document.createElement('button');
    newBtn.className = 'btn btn-primary';
    newBtn.textContent = '+ New Profile';
    if (!_isStarter) { newBtn.disabled = true; newBtn.title = 'Upgrade to Starter'; }
    newBtn.onclick = () => openProfileEditor(null);
    btnGroup.appendChild(newBtn);

    toolbar.appendChild(btnGroup);
    el.appendChild(toolbar);

    lqAddExportImportBar(el, function(format) {
        if (format === 'json') {
            lqExportJSON(profiles, 'industry-profiles.json');
        } else {
            lqExportCSV(profiles, ['slug','name','category','description','active'], 'industry-profiles.csv');
        }
    }, function() {
        lqImportJSON(async function(data) {
            var arr = Array.isArray(data) ? data : [data];
            var ok = 0;
            for (var i = 0; i < arr.length; i++) {
                try {
                    await fetch('/api/modules/industry-profile/profiles', { method: 'POST', headers: {'Content-Type':'application/json'}, body: JSON.stringify(arr[i]) });
                    ok++;
                } catch(e) {}
            }
            showToast('Imported ' + ok + ' of ' + arr.length + ' profiles', 'success');
            load_industry_profiles();
        });
    });

    // ── Table ──────────────────────────────────────────────────────────
    const tableWrap = document.createElement('div');
    tableWrap.className = 'content-table';
    const t = document.createElement('table');
    const hdr = document.createElement('tr');
    ['Name', 'Slug', 'Category', 'Services', 'Keywords', 'Active', ''].forEach(col => {
        const th = document.createElement('th');
        th.textContent = col;
        hdr.appendChild(th);
    });
    t.appendChild(hdr);

    if (profiles.length === 0) {
        const tr = document.createElement('tr');
        const td = document.createElement('td');
        td.colSpan = 7;
        td.style.cssText = 'text-align:center;padding:20px;color:var(--text-muted);';
        td.textContent = 'No industry profiles yet. Click "Seed Defaults" to create 8 starter profiles.';
        tr.appendChild(td);
        t.appendChild(tr);
    }

    profiles.sort((a, b) => a.name.localeCompare(b.name));
    profiles.forEach(p => {
        const tr = document.createElement('tr');
        tr.style.cursor = 'pointer';
        tr.onclick = () => openProfileDetail(p.slug);

        const tdName = document.createElement('td');
        const b = document.createElement('strong');
        b.textContent = p.name;
        tdName.appendChild(b);
        tr.appendChild(tdName);

        const tdSlug = document.createElement('td');
        const code = document.createElement('code');
        code.textContent = p.slug;
        tdSlug.appendChild(code);
        tr.appendChild(tdSlug);

        const tdCat = document.createElement('td');
        const badge = document.createElement('span');
        badge.className = 'badge';
        badge.textContent = p.category.replace('_', ' ');
        tdCat.appendChild(badge);
        tr.appendChild(tdCat);

        const tdSvc = document.createElement('td');
        tdSvc.textContent = (p.common_services || []).length;
        tr.appendChild(tdSvc);

        const tdKw = document.createElement('td');
        tdKw.textContent = (p.seo_keywords || []).length;
        tr.appendChild(tdKw);

        const tdActive = document.createElement('td');
        const ab = document.createElement('span');
        ab.className = 'badge ' + (p.active ? 'badge-success' : 'badge-muted');
        ab.textContent = p.active ? 'Active' : 'Inactive';
        tdActive.appendChild(ab);
        tr.appendChild(tdActive);

        const tdActions = document.createElement('td');
        const delBtn = document.createElement('button');
        delBtn.className = 'btn btn-sm btn-danger';
        delBtn.textContent = 'Delete';
        delBtn.onclick = async (e) => {
            e.stopPropagation();
            if (!confirm('Delete profile "' + p.name + '"?')) return;
            await fetch('/api/modules/industry-profile/profiles/' + encodeURIComponent(p.slug), { method: 'DELETE' });
            load_industry_profiles();
        };
        tdActions.appendChild(delBtn);
        tr.appendChild(tdActions);

        t.appendChild(tr);
    });

    tableWrap.appendChild(t);
    el.appendChild(tableWrap);

    while (main.firstChild) main.removeChild(main.firstChild);
    main.appendChild(el);
}

// ── Profile detail view ───────────────────────────────────────────────
async function openProfileDetail(slug) {
    const main = document.getElementById('adminMain');
    const r = await fetch('/api/modules/industry-profile/profiles/' + encodeURIComponent(slug)).then(r => r.json());
    if (!r.ok) { alert(r.message); return; }
    const p = r.data;

    const el = document.createElement('div');

    // Back button
    const backBtn = document.createElement('button');
    backBtn.className = 'btn';
    backBtn.textContent = '\u2190 Back to Profiles';
    backBtn.onclick = () => load_industry_profiles();
    el.appendChild(backBtn);

    // Header
    const header = document.createElement('div');
    header.style.cssText = 'display:flex;align-items:center;gap:12px;margin:16px 0;';
    const h2 = document.createElement('h2');
    h2.textContent = p.name;
    header.appendChild(h2);
    const catBadge = document.createElement('span');
    catBadge.className = 'badge';
    catBadge.textContent = p.category.replace('_', ' ');
    header.appendChild(catBadge);
    const activeBadge = document.createElement('span');
    activeBadge.className = 'badge ' + (p.active ? 'badge-success' : 'badge-muted');
    activeBadge.textContent = p.active ? 'Active' : 'Inactive';
    header.appendChild(activeBadge);

    var __role = (window.__CMS && window.__CMS.nexusRole) || '';
    var __isPro = __role === 'central' || __role === 'professional' || __role === 'enterprise';
    var __isStarter = __isPro || __role === 'starter';

    const editBtn = document.createElement('button');
    editBtn.className = 'btn btn-primary';
    editBtn.textContent = 'Edit Profile';
    editBtn.style.marginLeft = 'auto';
    if (!__isStarter) { editBtn.disabled = true; editBtn.title = 'Upgrade to Starter to edit'; }
    editBtn.onclick = () => openProfileEditor(p);
    header.appendChild(editBtn);

    // AI Guidelines button
    if (typeof LiqAI !== 'undefined') {
        var _aiGuideBtn = LiqAI.button({
            label: 'AI Generate Guidelines',
            feature: 'industry_ai_guidelines',
            credits: 3,
            tier: 'free',
            getInput: function() {
                if (!p.name) { showToast('Profile has no name', 'error'); return ''; }
                return 'Industry: ' + p.name + '\nCategory: ' + p.category + '\nDescription: ' + p.description + '\nServices: ' + (p.common_services || []).map(function(s) { return s.name; }).join(', ') + '\nPain points: ' + (p.customer_pain_points || []).join(', ');
            },
            onResult: function(result) {
                if (Array.isArray(result)) {
                    showToast('Generated ' + result.length + ' content guidelines. Save to apply.', 'success');
                }
            },
        });
        if (_aiGuideBtn) { _aiGuideBtn.style.marginLeft = '8px'; header.appendChild(_aiGuideBtn); }
    }
    el.appendChild(header);

    const desc = document.createElement('p');
    desc.textContent = p.description;
    desc.style.color = 'var(--text-muted)';
    el.appendChild(desc);

    // Expandable sections
    const addSection = (title, content) => {
        const sec = document.createElement('div');
        sec.style.cssText = 'border:1px solid var(--border);border-radius:8px;margin-bottom:8px;';
        const toggle = document.createElement('div');
        toggle.style.cssText = 'padding:12px 16px;cursor:pointer;display:flex;justify-content:space-between;align-items:center;font-weight:600;';
        toggle.textContent = title;
        const arrow = document.createElement('span');
        arrow.textContent = '\u25B6';
        arrow.style.transition = 'transform 0.2s';
        toggle.appendChild(arrow);
        const body = document.createElement('div');
        body.style.cssText = 'padding:0 16px 12px;display:none;';
        body.appendChild(content);
        toggle.onclick = () => {
            const open = body.style.display !== 'none';
            body.style.display = open ? 'none' : 'block';
            arrow.style.transform = open ? '' : 'rotate(90deg)';
        };
        sec.appendChild(toggle);
        sec.appendChild(body);
        el.appendChild(sec);
    };

    // Terminology
    if (p.terminology && p.terminology.length > 0) {
        const tbl = document.createElement('table');
        tbl.style.width = '100%';
        const h = document.createElement('tr');
        ['Term', 'Definition', 'Usage Context'].forEach(c => { const th = document.createElement('th'); th.textContent = c; h.appendChild(th); });
        tbl.appendChild(h);
        p.terminology.forEach(t => {
            const tr = document.createElement('tr');
            [t.term, t.definition, t.usage_context].forEach(v => { const td = document.createElement('td'); td.textContent = v; tr.appendChild(td); });
            tbl.appendChild(tr);
        });
        addSection('Terminology (' + p.terminology.length + ')', tbl);
    }

    // Common Services
    if (p.common_services && p.common_services.length > 0) {
        const tbl = document.createElement('table');
        tbl.style.width = '100%';
        const h = document.createElement('tr');
        ['Service', 'Slug', 'Description', 'Price Range'].forEach(c => { const th = document.createElement('th'); th.textContent = c; h.appendChild(th); });
        tbl.appendChild(h);
        p.common_services.forEach(s => {
            const tr = document.createElement('tr');
            [s.name, s.slug, s.description, s.price_range].forEach(v => { const td = document.createElement('td'); td.textContent = v; tr.appendChild(td); });
            tbl.appendChild(tr);
        });
        addSection('Common Services (' + p.common_services.length + ')', tbl);
    }

    // Compliance
    if (p.compliance_requirements && p.compliance_requirements.length > 0) {
        const tbl = document.createElement('table');
        tbl.style.width = '100%';
        const h = document.createElement('tr');
        ['Requirement', 'Description', 'Required'].forEach(c => { const th = document.createElement('th'); th.textContent = c; h.appendChild(th); });
        tbl.appendChild(h);
        p.compliance_requirements.forEach(c => {
            const tr = document.createElement('tr');
            const tdN = document.createElement('td'); tdN.textContent = c.name; tr.appendChild(tdN);
            const tdD = document.createElement('td'); tdD.textContent = c.description; tr.appendChild(tdD);
            const tdR = document.createElement('td');
            const rb = document.createElement('span');
            rb.className = 'badge ' + (c.required ? 'badge-danger' : 'badge-muted');
            rb.textContent = c.required ? 'Required' : 'Optional';
            tdR.appendChild(rb);
            tr.appendChild(tdR);
            tbl.appendChild(tr);
        });
        addSection('Compliance Requirements (' + p.compliance_requirements.length + ')', tbl);
    }

    // SEO Keywords
    if (p.seo_keywords && p.seo_keywords.length > 0) {
        const wrap = document.createElement('div');
        const tbl = document.createElement('table');
        tbl.style.width = '100%';
        const h = document.createElement('tr');
        ['Keyword', 'Volume', 'Difficulty', 'Intent'].forEach(c => { const th = document.createElement('th'); th.textContent = c; h.appendChild(th); });
        tbl.appendChild(h);
        p.seo_keywords.forEach(k => {
            const tr = document.createElement('tr');
            const tdK = document.createElement('td'); tdK.textContent = k.keyword; tr.appendChild(tdK);
            const tdV = document.createElement('td'); tdV.textContent = k.search_volume != null ? k.search_volume.toLocaleString() : '-'; tr.appendChild(tdV);
            const tdD = document.createElement('td'); tdD.textContent = k.difficulty != null ? k.difficulty : '-'; tr.appendChild(tdD);
            const tdI = document.createElement('td');
            const ib = document.createElement('span');
            ib.className = 'badge';
            ib.textContent = k.intent;
            tdI.appendChild(ib);
            tr.appendChild(tdI);
            tbl.appendChild(tr);
        });
        wrap.appendChild(tbl);

        // CSV import form
        const impDiv = document.createElement('div');
        impDiv.style.cssText = 'margin-top:12px;padding-top:12px;border-top:1px solid var(--border);';
        const impLabel = document.createElement('strong');
        impLabel.textContent = 'Import SEO Keywords (CSV)';
        impDiv.appendChild(impLabel);
        const ta = document.createElement('textarea');
        ta.placeholder = 'keyword,search_volume,difficulty,intent\nhvac repair near me,12000,45,transactional';
        ta.style.cssText = 'width:100%;height:80px;margin:8px 0;font-family:monospace;font-size:12px;';
        impDiv.appendChild(ta);
        const impBtn = document.createElement('button');
        impBtn.className = 'btn btn-primary';
        impBtn.textContent = 'Import CSV';
        impBtn.onclick = async () => {
            const body = ta.value.trim();
            if (!body) return;
            const ir = await fetch('/api/modules/industry-profile/profiles/' + encodeURIComponent(p.slug) + '/import-seo', {
                method: 'POST',
                headers: { 'Content-Type': 'text/csv' },
                body: body,
            }).then(r => r.json());
            alert(ir.message);
            if (ir.ok) openProfileDetail(p.slug);
        };
        impDiv.appendChild(impBtn);
        wrap.appendChild(impDiv);

        addSection('SEO Keywords (' + p.seo_keywords.length + ')', wrap);
    }

    // Content Guidelines
    if (p.content_guidelines && p.content_guidelines.length > 0) {
        const wrap = document.createElement('div');
        p.content_guidelines.forEach(g => {
            const card = document.createElement('div');
            card.style.cssText = 'margin-bottom:12px;padding:12px;background:var(--bg-secondary);border-radius:6px;';
            const title = document.createElement('strong');
            title.textContent = g.page_type;
            card.appendChild(title);
            const wc = document.createElement('div');
            wc.style.color = 'var(--text-muted)';
            wc.textContent = g.word_count_min + ' - ' + g.word_count_max + ' words | ' + g.tone_notes;
            card.appendChild(wc);
            const secs = document.createElement('div');
            secs.style.marginTop = '6px';
            secs.textContent = 'Sections: ' + g.recommended_sections.join(', ');
            card.appendChild(secs);
            wrap.appendChild(card);
        });
        addSection('Content Guidelines (' + p.content_guidelines.length + ')', wrap);
    }

    // Seasonal Patterns
    if (p.seasonal_patterns && p.seasonal_patterns.length > 0) {
        const tbl = document.createElement('table');
        tbl.style.width = '100%';
        const h = document.createElement('tr');
        ['Months', 'Description', 'Demand'].forEach(c => { const th = document.createElement('th'); th.textContent = c; h.appendChild(th); });
        tbl.appendChild(h);
        const monthNames = ['', 'Jan','Feb','Mar','Apr','May','Jun','Jul','Aug','Sep','Oct','Nov','Dec'];
        p.seasonal_patterns.forEach(s => {
            const tr = document.createElement('tr');
            const tdM = document.createElement('td'); tdM.textContent = s.months.map(m => monthNames[m] || m).join(', '); tr.appendChild(tdM);
            const tdD = document.createElement('td'); tdD.textContent = s.description; tr.appendChild(tdD);
            const tdL = document.createElement('td');
            const lb = document.createElement('span');
            lb.className = 'badge ' + (s.demand_level === 'high' ? 'badge-danger' : s.demand_level === 'medium' ? 'badge-warning' : 'badge-muted');
            lb.textContent = s.demand_level;
            tdL.appendChild(lb);
            tr.appendChild(tdL);
            tbl.appendChild(tr);
        });
        addSection('Seasonal Patterns (' + p.seasonal_patterns.length + ')', tbl);
    }

    // Pricing Norms
    if (p.pricing_norms) {
        const tbl = document.createElement('table');
        tbl.style.width = '100%';
        const rows = [
            ['Hourly Rate', p.pricing_norms.hourly_rate_range],
            ['Service Call Fee', p.pricing_norms.service_call_fee_range],
            ['Emergency Markup', p.pricing_norms.emergency_markup_pct != null ? p.pricing_norms.emergency_markup_pct + '%' : 'N/A'],
            ['Weekend Markup', p.pricing_norms.weekend_markup_pct != null ? p.pricing_norms.weekend_markup_pct + '%' : 'N/A'],
        ];
        rows.forEach(([label, val]) => {
            const tr = document.createElement('tr');
            const tdL = document.createElement('td'); tdL.style.fontWeight = '600'; tdL.textContent = label; tr.appendChild(tdL);
            const tdV = document.createElement('td'); tdV.textContent = val; tr.appendChild(tdV);
            tbl.appendChild(tr);
        });
        addSection('Pricing Norms', tbl);
    }

    // Pain Points
    if (p.customer_pain_points && p.customer_pain_points.length > 0) {
        const ul = document.createElement('ul');
        p.customer_pain_points.forEach(pp => { const li = document.createElement('li'); li.textContent = pp; ul.appendChild(li); });
        addSection('Customer Pain Points (' + p.customer_pain_points.length + ')', ul);
    }

    // Trust Factors
    if (p.trust_factors && p.trust_factors.length > 0) {
        const ul = document.createElement('ul');
        p.trust_factors.forEach(tf => { const li = document.createElement('li'); li.textContent = tf; ul.appendChild(li); });
        addSection('Trust Factors (' + p.trust_factors.length + ')', ul);
    }

    // Equipment Categories
    if (p.equipment_categories && p.equipment_categories.length > 0) {
        const wrap = document.createElement('div');
        p.equipment_categories.forEach(cat => {
            const h4 = document.createElement('h4');
            h4.textContent = cat.name;
            h4.style.marginBottom = '4px';
            wrap.appendChild(h4);
            const tags = document.createElement('div');
            tags.style.cssText = 'display:flex;flex-wrap:wrap;gap:4px;margin-bottom:12px;';
            cat.items.forEach(item => {
                const tag = document.createElement('span');
                tag.className = 'badge';
                tag.textContent = item;
                tags.appendChild(tag);
            });
            wrap.appendChild(tags);
        });
        addSection('Equipment Categories', wrap);
    }

    // Material Categories
    if (p.material_categories && p.material_categories.length > 0) {
        const wrap = document.createElement('div');
        p.material_categories.forEach(cat => {
            const h4 = document.createElement('h4');
            h4.textContent = cat.name;
            h4.style.marginBottom = '4px';
            wrap.appendChild(h4);
            const tags = document.createElement('div');
            tags.style.cssText = 'display:flex;flex-wrap:wrap;gap:4px;margin-bottom:12px;';
            cat.items.forEach(item => {
                const tag = document.createElement('span');
                tag.className = 'badge';
                tag.textContent = item;
                tags.appendChild(tag);
            });
            wrap.appendChild(tags);
        });
        addSection('Material Categories', wrap);
    }

    // Schema.org types & competitor terms
    if ((p.schema_org_types && p.schema_org_types.length > 0) || (p.competitor_terms && p.competitor_terms.length > 0)) {
        const wrap = document.createElement('div');
        if (p.schema_org_types && p.schema_org_types.length > 0) {
            const h4 = document.createElement('h4'); h4.textContent = 'Schema.org Types'; wrap.appendChild(h4);
            const tags = document.createElement('div'); tags.style.cssText = 'display:flex;flex-wrap:wrap;gap:4px;margin-bottom:12px;';
            p.schema_org_types.forEach(t => { const tag = document.createElement('span'); tag.className = 'badge'; tag.textContent = t; tags.appendChild(tag); });
            wrap.appendChild(tags);
        }
        if (p.competitor_terms && p.competitor_terms.length > 0) {
            const h4 = document.createElement('h4'); h4.textContent = 'Competitor Terms'; wrap.appendChild(h4);
            const tags = document.createElement('div'); tags.style.cssText = 'display:flex;flex-wrap:wrap;gap:4px;';
            p.competitor_terms.forEach(t => { const tag = document.createElement('span'); tag.className = 'badge'; tag.textContent = t; tags.appendChild(tag); });
            wrap.appendChild(tags);
        }
        addSection('SEO Metadata', wrap);
    }

    while (main.firstChild) main.removeChild(main.firstChild);
    main.appendChild(el);
}

// ── Profile editor (create / edit) ───────────────────────────────────
function openProfileEditor(existing) {
    const main = document.getElementById('adminMain');
    const isEdit = !!existing;
    const el = document.createElement('div');

    const backBtn = document.createElement('button');
    backBtn.className = 'btn';
    backBtn.textContent = '\u2190 Back';
    backBtn.onclick = () => isEdit ? openProfileDetail(existing.slug) : load_industry_profiles();
    el.appendChild(backBtn);

    const h2 = document.createElement('h2');
    h2.textContent = isEdit ? 'Edit: ' + existing.name : 'New Industry Profile';
    h2.style.margin = '16px 0';
    el.appendChild(h2);

    const form = document.createElement('div');
    form.className = 'form-grid';

    const addField = (label, name, value, type) => {
        const row = document.createElement('div');
        row.className = 'form-row';
        const lbl = document.createElement('label');
        lbl.textContent = label;
        row.appendChild(lbl);
        if (type === 'textarea') {
            const ta = document.createElement('textarea');
            ta.name = name;
            ta.value = value || '';
            ta.rows = 3;
            row.appendChild(ta);
        } else if (type === 'select') {
            const sel = document.createElement('select');
            sel.name = name;
            ['field_service', 'professional', 'retail', 'ecommerce', 'other'].forEach(opt => {
                const o = document.createElement('option');
                o.value = opt;
                o.textContent = opt.replace('_', ' ');
                if (opt === value) o.selected = true;
                sel.appendChild(o);
            });
            row.appendChild(sel);
        } else if (type === 'checkbox') {
            const cb = document.createElement('input');
            cb.type = 'checkbox';
            cb.name = name;
            cb.checked = value !== false;
            row.appendChild(cb);
        } else {
            const inp = document.createElement('input');
            inp.type = type || 'text';
            inp.name = name;
            inp.value = value || '';
            row.appendChild(inp);
        }
        form.appendChild(row);
    };

    addField('Name', 'name', existing?.name, 'text');
    addField('Slug', 'slug', existing?.slug, 'text');
    addField('Category', 'category', existing?.category || 'field_service', 'select');
    addField('Description', 'description', existing?.description, 'textarea');
    addField('Active', 'active', existing?.active, 'checkbox');

    // Pricing norms
    const pn = existing?.pricing_norms || {};
    addField('Hourly Rate Range', 'hourly_rate_range', pn.hourly_rate_range, 'text');
    addField('Service Call Fee Range', 'service_call_fee_range', pn.service_call_fee_range, 'text');
    addField('Emergency Markup %', 'emergency_markup_pct', pn.emergency_markup_pct, 'text');
    addField('Weekend Markup %', 'weekend_markup_pct', pn.weekend_markup_pct, 'text');

    // Simple lists (pain points, trust factors, competitor terms, schema.org)
    const addListField = (label, name, values) => {
        const row = document.createElement('div');
        row.className = 'form-row';
        const lbl = document.createElement('label');
        lbl.textContent = label + ' (one per line)';
        row.appendChild(lbl);
        const ta = document.createElement('textarea');
        ta.name = name;
        ta.value = (values || []).join('\n');
        ta.rows = 4;
        row.appendChild(ta);
        form.appendChild(row);
    };

    addListField('Customer Pain Points', 'customer_pain_points', existing?.customer_pain_points);
    addListField('Trust Factors', 'trust_factors', existing?.trust_factors);
    addListField('Competitor Terms', 'competitor_terms', existing?.competitor_terms);
    addListField('Schema.org Types', 'schema_org_types', existing?.schema_org_types);

    el.appendChild(form);

    const saveBtn = document.createElement('button');
    saveBtn.className = 'btn btn-primary';
    saveBtn.textContent = isEdit ? 'Save Changes' : 'Create Profile';
    saveBtn.style.marginTop = '16px';
    saveBtn.onclick = async () => {
        const fd = {};
        form.querySelectorAll('input, textarea, select').forEach(el => {
            if (el.type === 'checkbox') fd[el.name] = el.checked;
            else fd[el.name] = el.value;
        });

        const listVal = (name) => (fd[name] || '').split('\n').map(s => s.trim()).filter(Boolean);

        const body = {
            name: fd.name,
            slug: fd.slug,
            description: fd.description,
            category: fd.category,
            active: fd.active,
            pricing_norms: {
                hourly_rate_range: fd.hourly_rate_range || '',
                service_call_fee_range: fd.service_call_fee_range || '',
                emergency_markup_pct: fd.emergency_markup_pct ? parseFloat(fd.emergency_markup_pct) : null,
                weekend_markup_pct: fd.weekend_markup_pct ? parseFloat(fd.weekend_markup_pct) : null,
            },
            customer_pain_points: listVal('customer_pain_points'),
            trust_factors: listVal('trust_factors'),
            competitor_terms: listVal('competitor_terms'),
            schema_org_types: listVal('schema_org_types'),
            // Preserve existing complex sub-arrays when editing
            terminology: existing?.terminology || [],
            compliance_requirements: existing?.compliance_requirements || [],
            common_services: existing?.common_services || [],
            equipment_categories: existing?.equipment_categories || [],
            material_categories: existing?.material_categories || [],
            seo_keywords: existing?.seo_keywords || [],
            content_guidelines: existing?.content_guidelines || [],
            seasonal_patterns: existing?.seasonal_patterns || [],
        };

        const url = isEdit
            ? '/api/modules/industry-profile/profiles/' + encodeURIComponent(existing.slug)
            : '/api/modules/industry-profile/profiles';
        const method = isEdit ? 'PUT' : 'POST';

        saveBtn.disabled = true;
        const r = await fetch(url, {
            method,
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify(body),
        }).then(r => r.json());

        if (r.ok) {
            openProfileDetail(fd.slug || existing?.slug);
        } else {
            alert(r.message);
            saveBtn.disabled = false;
        }
    };
    el.appendChild(saveBtn);

    while (main.firstChild) main.removeChild(main.firstChild);
    main.appendChild(el);
}
"##;
