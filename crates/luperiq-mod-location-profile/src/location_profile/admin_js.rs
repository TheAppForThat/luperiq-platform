//! Admin JavaScript for the Location Profile module.
//!
//! View `location-profiles`:
//! - Stats bar: total locations, active count
//! - Location list table: city/state, slug, population, competitors count, keywords count, active badge
//! - Click location to open detail/edit panel
//! - Detail shows: Demographics, Weather, Keywords, Competitors, Regulations, Neighborhoods
//! - Import buttons for Census and Competitor data (JSON paste modals)
//! - Form editor for basic fields
//!
//! Security: Uses DOM methods (createElement/textContent) exclusively. No innerHTML
//! with user data.

pub const LOCATION_PROFILES_ADMIN_JS: &str = r##"
// ── Location Profiles view ──────────────────────────────────────────
async function load_location_profiles() {
    const main = document.getElementById('adminMain');

    var _role = (window.__CMS && window.__CMS.nexusRole) || '';
    var _isPro = _role === 'central' || _role === 'professional' || _role === 'enterprise';
    var _isStarter = _isPro || _role === 'starter';

    const r = await fetch('/api/modules/location-profile/locations').then(r => r.json());
    var locations = r.data || [];

    const el = document.createElement('div');

    // Pricing card
    var _pc = lqModulePricingCard({ name: 'Location Profiles', monthly: 9, annual: 89, lifetime: 249, tier: 'starter', deps: [], slug: 'location-profile' });
    if (_pc) el.appendChild(_pc);

    // Free tier limit: max 3
    var _allLocations = locations;
    if (!_isStarter && locations.length > 3) {
        locations = locations.slice(0, 3);
    }

    // ── Stats bar ─────────────────────────────────────────────────────
    const stats = document.createElement('div');
    stats.className = 'stats-bar';

    const activeCount = locations.filter(l => l.active).length;

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

    addStat('Total Locations', locations.length);
    addStat('Active', activeCount);
    el.appendChild(stats);

    // ── Toolbar ────────────────────────────────────────────────────────
    const toolbar = document.createElement('div');
    toolbar.className = 'toolbar';
    const h = document.createElement('h2');
    h.textContent = 'Location Profiles';
    toolbar.appendChild(h);

    const newBtn = document.createElement('button');
    newBtn.className = 'btn btn-primary';
    newBtn.textContent = '+ New Location';
    if (!_isStarter) { newBtn.disabled = true; newBtn.title = 'Upgrade to Starter'; }
    newBtn.onclick = () => openLocationEditor(null);
    toolbar.appendChild(newBtn);
    el.appendChild(toolbar);

    // Free tier gate message
    if (!_isStarter && _allLocations.length > 3) {
        var _gate = document.createElement('div');
        _gate.style.cssText = 'background:var(--surface);border:1px solid var(--border);border-radius:8px;padding:12px 16px;margin-bottom:12px;display:flex;align-items:center;justify-content:space-between;';
        var _gateText = document.createElement('span');
        _gateText.textContent = 'Showing 3 of ' + _allLocations.length + ' locations \u2014 upgrade for full access';
        _gate.appendChild(_gateText);
        var _upBtn = document.createElement('button');
        _upBtn.className = 'btn btn-primary btn-sm';
        _upBtn.textContent = 'Upgrade';
        _upBtn.onclick = function() { navigateTo('store'); };
        _gate.appendChild(_upBtn);
        el.appendChild(_gate);
    }

    lqAddExportImportBar(el, function(format) {
        if (format === 'json') {
            lqExportJSON(locations, 'location-profiles.json');
        } else {
            lqExportCSV(locations, ['city','state','slug','population','local_competitors','local_keywords','active','county','area_description'], 'location-profiles.csv');
        }
    }, function() {
        lqImportJSON(async function(data) {
            var arr = Array.isArray(data) ? data : [data];
            var ok = 0;
            for (var i = 0; i < arr.length; i++) {
                try {
                    await fetch('/api/modules/location-profile/locations', { method: 'POST', headers: {'Content-Type':'application/json'}, body: JSON.stringify(arr[i]) });
                    ok++;
                } catch(e) {}
            }
            showToast('Imported ' + ok + ' of ' + arr.length + ' locations', 'success');
            load_location_profiles();
        });
    });

    // ── Table ──────────────────────────────────────────────────────────
    const tableWrap = document.createElement('div');
    tableWrap.className = 'content-table';
    const t = document.createElement('table');
    const hdr = document.createElement('tr');
    ['City / State', 'Slug', 'Population', 'Competitors', 'Keywords', 'Active', ''].forEach(col => {
        const th = document.createElement('th');
        th.textContent = col;
        hdr.appendChild(th);
    });
    t.appendChild(hdr);

    if (locations.length === 0) {
        const tr = document.createElement('tr');
        const td = document.createElement('td');
        td.colSpan = 7;
        td.style.cssText = 'text-align:center;padding:20px;color:var(--text-muted);';
        td.textContent = 'No location profiles yet. Click "+ New Location" to add your first service area.';
        tr.appendChild(td);
        t.appendChild(tr);
    }

    locations.sort((a, b) => (a.city + a.state).localeCompare(b.city + b.state));
    locations.forEach(l => {
        const tr = document.createElement('tr');
        tr.style.cursor = 'pointer';
        tr.onclick = () => openLocationDetail(l.slug);

        const tdCity = document.createElement('td');
        const b = document.createElement('strong');
        b.textContent = l.city + ', ' + l.state;
        tdCity.appendChild(b);
        tr.appendChild(tdCity);

        const tdSlug = document.createElement('td');
        const code = document.createElement('code');
        code.textContent = l.slug;
        tdSlug.appendChild(code);
        tr.appendChild(tdSlug);

        const tdPop = document.createElement('td');
        tdPop.textContent = l.population != null ? l.population.toLocaleString() : '-';
        tr.appendChild(tdPop);

        const tdComp = document.createElement('td');
        tdComp.textContent = (l.local_competitors || []).length;
        tr.appendChild(tdComp);

        const tdKw = document.createElement('td');
        tdKw.textContent = (l.local_keywords || []).length;
        tr.appendChild(tdKw);

        const tdActive = document.createElement('td');
        const ab = document.createElement('span');
        ab.className = 'badge ' + (l.active ? 'badge-success' : 'badge-muted');
        ab.textContent = l.active ? 'Active' : 'Inactive';
        tdActive.appendChild(ab);
        tr.appendChild(tdActive);

        const tdActions = document.createElement('td');
        const delBtn = document.createElement('button');
        delBtn.className = 'btn btn-sm btn-danger';
        delBtn.textContent = 'Delete';
        delBtn.onclick = async (e) => {
            e.stopPropagation();
            if (!confirm('Delete location "' + l.city + ', ' + l.state + '"?')) return;
            await fetch('/api/modules/location-profile/locations/' + encodeURIComponent(l.slug), { method: 'DELETE' });
            load_location_profiles();
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

// ── Location detail view ─────────────────────────────────────────────
async function openLocationDetail(slug) {
    const main = document.getElementById('adminMain');
    const r = await fetch('/api/modules/location-profile/locations/' + encodeURIComponent(slug)).then(r => r.json());
    if (!r.ok) { alert(r.message); return; }
    const l = r.data;

    const el = document.createElement('div');

    // Back button
    const backBtn = document.createElement('button');
    backBtn.className = 'btn';
    backBtn.textContent = '\u2190 Back to Locations';
    backBtn.onclick = () => load_location_profiles();
    el.appendChild(backBtn);

    // Header
    const header = document.createElement('div');
    header.style.cssText = 'display:flex;align-items:center;gap:12px;margin:16px 0;';
    const h2 = document.createElement('h2');
    h2.textContent = l.city + ', ' + l.state;
    header.appendChild(h2);
    if (l.county) {
        const countyBadge = document.createElement('span');
        countyBadge.className = 'badge';
        countyBadge.textContent = l.county + ' County';
        header.appendChild(countyBadge);
    }
    const activeBadge = document.createElement('span');
    activeBadge.className = 'badge ' + (l.active ? 'badge-success' : 'badge-muted');
    activeBadge.textContent = l.active ? 'Active' : 'Inactive';
    header.appendChild(activeBadge);

    // Re-use the role variables computed in load_location_profiles() scope.
    // These are passed as arguments: isPro, isStarter (see call site).
    // NOTE: Until openLocationDetail() is refactored to accept (slug, isPro, isStarter),
    // we re-read from window.__CMS here. Consolidation tracked in review notes.
    var __role = (window.__CMS && window.__CMS.nexusRole) || '';
    var __isPro = __role === 'central' || __role === 'professional' || __role === 'enterprise';
    var __isStarter = __isPro || __role === 'starter';

    const editBtn = document.createElement('button');
    editBtn.className = 'btn btn-primary';
    editBtn.textContent = 'Edit Location';
    editBtn.style.marginLeft = 'auto';
    if (!__isStarter) { editBtn.disabled = true; editBtn.title = 'Upgrade to Starter to edit'; }
    editBtn.onclick = () => openLocationEditor(l);
    header.appendChild(editBtn);

    // AI Description button
    if (typeof LiqAI !== 'undefined') {
        var _aiDescBtn = LiqAI.button({
            label: 'AI Generate Description',
            feature: 'location_ai_description',
            credits: 2,
            tier: 'free',
            getInput: function() {
                if (!l.city) { showToast('Location has no city', 'error'); return ''; }
                var neighborhoods = (l.neighborhoods || []).join(', ');
                var keywords = (l.local_keywords || []).map(function(k) { return k.keyword; }).join(', ');
                return 'City: ' + l.city + ', ' + l.state + '\nCounty: ' + (l.county || '') + '\nPopulation: ' + (l.population || 'unknown') + '\nMedian Income: ' + (l.median_income || 'unknown') + '\nClimate Zone: ' + (l.climate_zone || 'unknown') + '\nNeighborhoods: ' + neighborhoods + '\nLocal Keywords: ' + keywords;
            },
            onResult: function(result) {
                if (typeof result === 'string') {
                    showToast('Area description generated. Edit the location to apply it.', 'success');
                }
            },
        });
        if (_aiDescBtn) { _aiDescBtn.style.marginLeft = '8px'; header.appendChild(_aiDescBtn); }
    }
    el.appendChild(header);

    if (l.area_description) {
        const desc = document.createElement('p');
        desc.textContent = l.area_description;
        desc.style.color = 'var(--text-muted)';
        el.appendChild(desc);
    }

    // Expandable sections helper
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

    // ── Demographics section ────────────────────────────────────────
    const demoTbl = document.createElement('table');
    demoTbl.style.width = '100%';
    const demoRows = [
        ['Population', l.population != null ? l.population.toLocaleString() : 'N/A'],
        ['Median Income', l.median_income != null ? '$' + l.median_income.toLocaleString() : 'N/A'],
        ['Housing Units', l.housing_units != null ? l.housing_units.toLocaleString() : 'N/A'],
        ['Owner Occupied', l.owner_occupied_pct != null ? l.owner_occupied_pct + '%' : 'N/A'],
        ['Median Home Age', l.median_home_age != null ? l.median_home_age + ' years' : 'N/A'],
        ['Cost of Living Index', l.cost_of_living_index != null ? l.cost_of_living_index.toString() : 'N/A'],
        ['Climate Zone', l.climate_zone || 'N/A'],
        ['Metro Area', l.metro_area || 'N/A'],
        ['ZIP Codes', (l.zip_codes || []).join(', ') || 'N/A'],
    ];
    demoRows.forEach(([label, val]) => {
        const tr = document.createElement('tr');
        const tdL = document.createElement('td');
        tdL.style.fontWeight = '600';
        tdL.style.width = '200px';
        tdL.textContent = label;
        tr.appendChild(tdL);
        const tdV = document.createElement('td');
        tdV.textContent = val;
        tr.appendChild(tdV);
        demoTbl.appendChild(tr);
    });

    // Census import button
    const censusWrap = document.createElement('div');
    censusWrap.appendChild(demoTbl);
    const censusDiv = document.createElement('div');
    censusDiv.style.cssText = 'margin-top:12px;padding-top:12px;border-top:1px solid var(--border);';
    const censusLabel = document.createElement('strong');
    censusLabel.textContent = 'Import Census Data (JSON)';
    censusDiv.appendChild(censusLabel);
    const censusTa = document.createElement('textarea');
    censusTa.placeholder = '{\n  "population": 1000000,\n  "median_income": 65000,\n  "housing_units": 400000,\n  "owner_occupied_pct": 55.0,\n  "median_home_age": 25,\n  "cost_of_living_index": 95.0\n}';
    censusTa.style.cssText = 'width:100%;height:120px;margin:8px 0;font-family:monospace;font-size:12px;';
    censusDiv.appendChild(censusTa);
    const censusBtn = document.createElement('button');
    censusBtn.className = 'btn btn-primary';
    censusBtn.textContent = 'Import Census';
    censusBtn.onclick = async () => {
        const body = censusTa.value.trim();
        if (!body) return;
        try { JSON.parse(body); } catch (e) { alert('Invalid JSON: ' + e.message); return; }
        const ir = await fetch('/api/modules/location-profile/locations/' + encodeURIComponent(l.slug) + '/import-census', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: body,
        }).then(r => r.json());
        alert(ir.message);
        if (ir.ok) openLocationDetail(l.slug);
    };
    censusDiv.appendChild(censusBtn);
    censusWrap.appendChild(censusDiv);
    addSection('Demographics', censusWrap);

    // ── Weather section ─────────────────────────────────────────────
    if (l.weather_patterns && l.weather_patterns.length > 0) {
        const tbl = document.createElement('table');
        tbl.style.width = '100%';
        const h = document.createElement('tr');
        ['Season', 'High (\u00B0F)', 'Low (\u00B0F)', 'Description'].forEach(c => {
            const th = document.createElement('th');
            th.textContent = c;
            h.appendChild(th);
        });
        tbl.appendChild(h);
        l.weather_patterns.forEach(w => {
            const tr = document.createElement('tr');
            const tdS = document.createElement('td');
            tdS.textContent = w.season.charAt(0).toUpperCase() + w.season.slice(1);
            tdS.style.fontWeight = '600';
            tr.appendChild(tdS);
            const tdH = document.createElement('td');
            tdH.textContent = w.avg_high_f != null ? w.avg_high_f : '-';
            tr.appendChild(tdH);
            const tdL = document.createElement('td');
            tdL.textContent = w.avg_low_f != null ? w.avg_low_f : '-';
            tr.appendChild(tdL);
            const tdD = document.createElement('td');
            tdD.textContent = w.description;
            tr.appendChild(tdD);
            tbl.appendChild(tr);
        });
        addSection('Weather Patterns (' + l.weather_patterns.length + ')', tbl);
    }

    // ── Local Keywords section ──────────────────────────────────────
    if (l.local_keywords && l.local_keywords.length > 0) {
        const tbl = document.createElement('table');
        tbl.style.width = '100%';
        const h = document.createElement('tr');
        ['Keyword', 'Search Volume', 'Geo Modifier'].forEach(c => {
            const th = document.createElement('th');
            th.textContent = c;
            h.appendChild(th);
        });
        tbl.appendChild(h);
        l.local_keywords.forEach(k => {
            const tr = document.createElement('tr');
            const tdK = document.createElement('td');
            tdK.textContent = k.keyword;
            tr.appendChild(tdK);
            const tdV = document.createElement('td');
            tdV.textContent = k.search_volume != null ? k.search_volume.toLocaleString() : '-';
            tr.appendChild(tdV);
            const tdG = document.createElement('td');
            const gb = document.createElement('code');
            gb.textContent = k.geo_modifier;
            tdG.appendChild(gb);
            tr.appendChild(tdG);
            tbl.appendChild(tr);
        });
        addSection('Local Keywords (' + l.local_keywords.length + ')', tbl);
    }

    // ── Competitors section ─────────────────────────────────────────
    const compWrap = document.createElement('div');
    if (l.local_competitors && l.local_competitors.length > 0) {
        const tbl = document.createElement('table');
        tbl.style.width = '100%';
        const h = document.createElement('tr');
        ['Name', 'Website', 'Rating', 'Reviews', 'Specialties'].forEach(c => {
            const th = document.createElement('th');
            th.textContent = c;
            h.appendChild(th);
        });
        tbl.appendChild(h);
        l.local_competitors.forEach(c => {
            const tr = document.createElement('tr');
            const tdN = document.createElement('td');
            tdN.style.fontWeight = '600';
            tdN.textContent = c.name;
            tr.appendChild(tdN);
            const tdW = document.createElement('td');
            if (c.website) {
                const a = document.createElement('a');
                a.href = c.website;
                a.target = '_blank';
                a.textContent = c.website.replace(/^https?:\/\//, '').replace(/\/$/, '');
                tdW.appendChild(a);
            } else {
                tdW.textContent = '-';
            }
            tr.appendChild(tdW);
            const tdR = document.createElement('td');
            tdR.textContent = c.rating != null ? c.rating.toFixed(1) : '-';
            tr.appendChild(tdR);
            const tdRc = document.createElement('td');
            tdRc.textContent = c.review_count != null ? c.review_count.toLocaleString() : '-';
            tr.appendChild(tdRc);
            const tdSp = document.createElement('td');
            const tags = document.createElement('div');
            tags.style.cssText = 'display:flex;flex-wrap:wrap;gap:4px;';
            (c.specialties || []).forEach(sp => {
                const tag = document.createElement('span');
                tag.className = 'badge';
                tag.textContent = sp;
                tags.appendChild(tag);
            });
            tdSp.appendChild(tags);
            tr.appendChild(tdSp);
            tbl.appendChild(tr);
        });
        compWrap.appendChild(tbl);
    } else {
        const empty = document.createElement('p');
        empty.style.color = 'var(--text-muted)';
        empty.textContent = 'No competitors added yet.';
        compWrap.appendChild(empty);
    }

    // Competitor import
    const compDiv = document.createElement('div');
    compDiv.style.cssText = 'margin-top:12px;padding-top:12px;border-top:1px solid var(--border);';
    const compLabel = document.createElement('strong');
    compLabel.textContent = 'Import Competitors (JSON array)';
    compDiv.appendChild(compLabel);
    const compTa = document.createElement('textarea');
    compTa.placeholder = '[\n  {\n    "name": "Acme HVAC",\n    "website": "https://acmehvac.com",\n    "rating": 4.5,\n    "review_count": 127,\n    "specialties": ["AC repair", "furnace installation"]\n  }\n]';
    compTa.style.cssText = 'width:100%;height:120px;margin:8px 0;font-family:monospace;font-size:12px;';
    compDiv.appendChild(compTa);
    const compBtn = document.createElement('button');
    compBtn.className = 'btn btn-primary';
    compBtn.textContent = 'Import Competitors';
    compBtn.onclick = async () => {
        const body = compTa.value.trim();
        if (!body) return;
        try { JSON.parse(body); } catch (e) { alert('Invalid JSON: ' + e.message); return; }
        const ir = await fetch('/api/modules/location-profile/locations/' + encodeURIComponent(l.slug) + '/import-competitors', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: body,
        }).then(r => r.json());
        alert(ir.message);
        if (ir.ok) openLocationDetail(l.slug);
    };
    compDiv.appendChild(compBtn);
    compWrap.appendChild(compDiv);
    addSection('Competitors (' + (l.local_competitors || []).length + ')', compWrap);

    // ── Regulations section ─────────────────────────────────────────
    if (l.local_regulations && l.local_regulations.length > 0) {
        const tbl = document.createElement('table');
        tbl.style.width = '100%';
        const h = document.createElement('tr');
        ['Regulation', 'Description', 'Authority'].forEach(c => {
            const th = document.createElement('th');
            th.textContent = c;
            h.appendChild(th);
        });
        tbl.appendChild(h);
        l.local_regulations.forEach(r => {
            const tr = document.createElement('tr');
            const tdN = document.createElement('td');
            tdN.style.fontWeight = '600';
            tdN.textContent = r.name;
            tr.appendChild(tdN);
            const tdD = document.createElement('td');
            tdD.textContent = r.description;
            tr.appendChild(tdD);
            const tdA = document.createElement('td');
            const ab = document.createElement('span');
            ab.className = 'badge';
            ab.textContent = r.authority;
            tdA.appendChild(ab);
            tr.appendChild(tdA);
            tbl.appendChild(tr);
        });
        addSection('Regulations (' + l.local_regulations.length + ')', tbl);
    }

    // ── Neighborhoods section ───────────────────────────────────────
    if (l.neighborhoods && l.neighborhoods.length > 0) {
        const wrap = document.createElement('div');
        wrap.style.cssText = 'display:flex;flex-wrap:wrap;gap:6px;';
        l.neighborhoods.forEach(n => {
            const tag = document.createElement('span');
            tag.className = 'badge';
            tag.textContent = n;
            wrap.appendChild(tag);
        });
        addSection('Neighborhoods (' + l.neighborhoods.length + ')', wrap);
    }

    while (main.firstChild) main.removeChild(main.firstChild);
    main.appendChild(el);
}

// ── Location editor (create / edit) ──────────────────────────────────
function openLocationEditor(existing) {
    const main = document.getElementById('adminMain');
    const isEdit = !!existing;
    const el = document.createElement('div');

    const backBtn = document.createElement('button');
    backBtn.className = 'btn';
    backBtn.textContent = '\u2190 Back';
    backBtn.onclick = () => isEdit ? openLocationDetail(existing.slug) : load_location_profiles();
    el.appendChild(backBtn);

    const h2 = document.createElement('h2');
    h2.textContent = isEdit ? 'Edit: ' + existing.city + ', ' + existing.state : 'New Location Profile';
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
            ta.rows = 4;
            row.appendChild(ta);
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

    addField('City', 'city', existing?.city, 'text');
    addField('State', 'state', existing?.state, 'text');
    addField('Slug', 'slug', existing?.slug, 'text');
    addField('County', 'county', existing?.county, 'text');
    addField('Metro Area', 'metro_area', existing?.metro_area, 'text');
    addField('Climate Zone', 'climate_zone', existing?.climate_zone, 'text');
    addField('Area Description', 'area_description', existing?.area_description, 'textarea');
    addField('Active', 'active', existing?.active, 'checkbox');

    // Simple list fields
    const addListField = (label, name, values) => {
        const row = document.createElement('div');
        row.className = 'form-row';
        const lbl = document.createElement('label');
        lbl.textContent = label + ' (one per line)';
        row.appendChild(lbl);
        const ta = document.createElement('textarea');
        ta.name = name;
        ta.value = (values || []).join('\n');
        ta.rows = 3;
        row.appendChild(ta);
        form.appendChild(row);
    };

    addListField('ZIP Codes', 'zip_codes', existing?.zip_codes);
    addListField('Neighborhoods', 'neighborhoods', existing?.neighborhoods);

    el.appendChild(form);

    const saveBtn = document.createElement('button');
    saveBtn.className = 'btn btn-primary';
    saveBtn.textContent = isEdit ? 'Save Changes' : 'Create Location';
    saveBtn.style.marginTop = '16px';
    saveBtn.onclick = async () => {
        const fd = {};
        form.querySelectorAll('input, textarea').forEach(el => {
            if (el.type === 'checkbox') fd[el.name] = el.checked;
            else fd[el.name] = el.value;
        });

        const listVal = (name) => (fd[name] || '').split('\n').map(s => s.trim()).filter(Boolean);

        const body = {
            city: fd.city,
            state: fd.state,
            slug: fd.slug,
            county: fd.county || null,
            metro_area: fd.metro_area || null,
            climate_zone: fd.climate_zone || null,
            area_description: fd.area_description || '',
            active: fd.active,
            zip_codes: listVal('zip_codes'),
            neighborhoods: listVal('neighborhoods'),
            // Preserve existing complex sub-arrays when editing
            weather_patterns: existing?.weather_patterns || [],
            local_keywords: existing?.local_keywords || [],
            local_competitors: existing?.local_competitors || [],
            local_regulations: existing?.local_regulations || [],
        };

        const url = isEdit
            ? '/api/modules/location-profile/locations/' + encodeURIComponent(existing.slug)
            : '/api/modules/location-profile/locations';
        const method = isEdit ? 'PUT' : 'POST';

        saveBtn.disabled = true;
        const r = await fetch(url, {
            method,
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify(body),
        }).then(r => r.json());

        if (r.ok) {
            openLocationDetail(fd.slug || existing?.slug);
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
