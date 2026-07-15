pub(crate) const GOOGLE_ADMIN_JS: &str = r##"
// ── Google integration helpers ─────────────────────────────────────

function googleMaxLookback() {
    var role = (window.__CMS && window.__CMS.nexusRole) || '';
    if (role === 'central' || role === 'professional' || role === 'enterprise') return 540;
    return 14;
}
function googleDateStr(daysAgo) {
    var d = new Date();
    d.setDate(d.getDate() - daysAgo);
    return d.toISOString().split('T')[0];
}
function googleMinDate() {
    return googleDateStr(googleMaxLookback());
}
function googleCard(content) {
    var c = document.createElement('div');
    c.style.cssText = 'padding:20px;background:var(--surface);border:1px solid var(--border);border-radius:8px;margin-bottom:16px;';
    if (content) c.appendChild(content);
    return c;
}
function googleRow(label, value) {
    var row = document.createElement('div');
    row.style.cssText = 'display:flex;align-items:center;gap:12px;margin-bottom:8px;';
    var lEl = document.createElement('span');
    lEl.style.cssText = 'color:var(--text-muted);font-size:13px;min-width:140px;';
    lEl.textContent = label;
    var vEl = document.createElement('span');
    vEl.style.cssText = 'font-size:13px;font-weight:500;';
    vEl.textContent = value || '—';
    row.appendChild(lEl);
    row.appendChild(vEl);
    return row;
}
function googleNotConnected(msg, btnLabel, btnView) {
    var wrap = document.createElement('div');
    wrap.style.cssText = 'text-align:center;padding:48px 24px;';
    var icon = document.createElement('div');
    icon.style.cssText = 'font-size:48px;margin-bottom:16px;';
    icon.textContent = '🔌';
    wrap.appendChild(icon);
    var p = document.createElement('p');
    p.style.cssText = 'color:var(--text-muted);margin-bottom:20px;';
    p.textContent = msg || 'Not connected to Google.';
    wrap.appendChild(p);
    var btn = document.createElement('button');
    btn.className = 'btn btn-primary';
    btn.textContent = btnLabel || 'Go to Settings';
    btn.onclick = function() { navigateTo(btnView || 'seo-google-settings'); };
    wrap.appendChild(btn);
    return wrap;
}
function googleStatCards(stats) {
    var row = document.createElement('div');
    row.className = 'stats';
    stats.forEach(function(s) {
        var card = document.createElement('div');
        card.className = 'stat-card';
        var lbl = document.createElement('div'); lbl.className = 'label'; lbl.textContent = s[0];
        var val = document.createElement('div'); val.className = 'value'; val.textContent = s[1] != null ? s[1] : '—';
        card.appendChild(lbl); card.appendChild(val);
        row.appendChild(card);
    });
    return row;
}
function googleTable(headers, rows) {
    var wrap = document.createElement('div');
    wrap.style.cssText = 'overflow-x:auto;';
    var tbl = document.createElement('table');
    tbl.className = 'content-table';
    var thead = document.createElement('thead');
    var hrow = document.createElement('tr');
    headers.forEach(function(h) {
        var th = document.createElement('th');
        th.textContent = h;
        hrow.appendChild(th);
    });
    thead.appendChild(hrow);
    tbl.appendChild(thead);
    var tbody = document.createElement('tbody');
    rows.forEach(function(r) {
        var tr = document.createElement('tr');
        r.forEach(function(cell) {
            var td = document.createElement('td');
            td.textContent = cell != null ? String(cell) : '—';
            tr.appendChild(td);
        });
        tbody.appendChild(tr);
    });
    tbl.appendChild(tbody);
    wrap.appendChild(tbl);
    return wrap;
}
function googleBadge(text, ok) {
    var b = document.createElement('span');
    b.className = 'status-badge';
    b.textContent = text;
    if (ok) { b.style.cssText = 'background:rgba(34,197,94,0.15);color:#22c55e;'; }
    else { b.style.cssText = 'background:rgba(239,68,68,0.15);color:#ef4444;'; }
    return b;
}
function googleFmt(n) {
    if (n == null || n === '') return '—';
    var num = parseFloat(n);
    if (isNaN(num)) return String(n);
    if (num >= 1000000) return (num/1000000).toFixed(1) + 'M';
    if (num >= 1000) return (num/1000).toFixed(1) + 'K';
    return String(Math.round(num));
}
function googlePct(n) {
    if (n == null || n === '') return '—';
    var num = parseFloat(n);
    if (isNaN(num)) return String(n);
    if (num > 1) return num.toFixed(1) + '%';
    return (num * 100).toFixed(1) + '%';
}

// ── View: Google Settings ──────────────────────────────────────────
async function load_seo_google_settings() {
    var main = document.getElementById('adminMain');
    var el = document.createElement('div');

    var h = document.createElement('h2'); h.textContent = 'Google Settings'; el.appendChild(h);
    var sub = document.createElement('p');
    sub.style.cssText = 'color:var(--text-muted);margin-bottom:20px;font-size:14px;';
    sub.textContent = 'Connect your Google account to enable GA4 Analytics, Search Console, and Google Ads.';
    el.appendChild(sub);

    // ── OAuth status card ──
    var statusCard = googleCard(null);
    var statusTitle = document.createElement('h3');
    statusTitle.style.cssText = 'margin-bottom:14px;font-size:15px;';
    statusTitle.textContent = 'Connection Status';
    statusCard.appendChild(statusTitle);

    var statusBody = document.createElement('div');
    statusCard.appendChild(statusBody);
    el.appendChild(statusCard);

    async function refreshStatus() {
        try {
            var r = await fetch('/api/modules/seo/google/status').then(function(r) { return r.json(); });
            statusBody.replaceChildren();
            var data = (r.data) || {};
            var row = document.createElement('div');
            row.style.cssText = 'display:flex;align-items:center;gap:16px;flex-wrap:wrap;';
            var badge = googleBadge(data.authenticated ? 'Connected' : 'Not Connected', !!data.authenticated);
            row.appendChild(badge);
            if (data.authenticated) {
                var modeSpan = document.createElement('span');
                modeSpan.style.cssText = 'font-size:12px;color:var(--text-muted);';
                modeSpan.textContent = 'Mode: ' + (data.oauth_mode || 'luperiq');
                row.appendChild(modeSpan);
                var disconnectBtn = document.createElement('button');
                disconnectBtn.className = 'btn btn-ghost btn-sm';
                disconnectBtn.style.cssText = 'margin-left:auto;color:#ef4444;';
                disconnectBtn.textContent = 'Disconnect';
                disconnectBtn.onclick = async function() {
                    if (!confirm('Disconnect Google account? This will remove all stored tokens.')) return;
                    try {
                        var dr = await fetch('/api/modules/seo/google/disconnect', { method: 'POST' }).then(function(r) { return r.json(); });
                        toast(dr.message || 'Disconnected');
                        refreshStatus();
                        loadPropertySelectors();
                    } catch(e) { toast('Error: ' + e.message); }
                };
                row.appendChild(disconnectBtn);
            } else {
                var connectBtn = document.createElement('button');
                connectBtn.className = 'btn btn-primary btn-sm';
                connectBtn.textContent = 'Connect via LuperIQ';
                connectBtn.onclick = async function() {
                    try {
                        var ar = await fetch('/api/modules/seo/google/auth-url?redirect_uri=' + encodeURIComponent(window.location.origin + '/admin?google_status=connected')).then(function(r) { return r.json(); });
                        if (ar.ok && ar.data && ar.data.url) {
                            window.open(ar.data.url, '_blank');
                        } else {
                            toast(ar.message || 'Could not get auth URL');
                        }
                    } catch(e) { toast('Error: ' + e.message); }
                };
                row.appendChild(connectBtn);
            }
            statusBody.appendChild(row);
        } catch(e) {
            statusBody.textContent = 'Failed to load status.';
        }
    }

    // ── Property selectors ──
    var selectorsCard = googleCard(null);
    var selectorsTitle = document.createElement('h3');
    selectorsTitle.style.cssText = 'margin-bottom:14px;font-size:15px;';
    selectorsTitle.textContent = 'Service Configuration';
    selectorsCard.appendChild(selectorsTitle);
    var selectorsBody = document.createElement('div');
    selectorsCard.appendChild(selectorsBody);
    el.appendChild(selectorsCard);

    async function loadPropertySelectors() {
        try {
            var cfg = await fetch('/api/modules/seo/google/config').then(function(r) { return r.json(); });
            var data = (cfg.ok && cfg.data) ? cfg.data : {};
            selectorsBody.replaceChildren();

            if (!data.authenticated) {
                var note = document.createElement('p');
                note.style.cssText = 'color:var(--text-muted);font-size:13px;';
                note.textContent = 'Connect your Google account above to configure services.';
                selectorsBody.appendChild(note);
                return;
            }

            // GA4
            var ga4Wrap = document.createElement('div');
            ga4Wrap.style.cssText = 'margin-bottom:20px;';
            var ga4Label = document.createElement('label');
            ga4Label.style.cssText = 'display:block;font-size:13px;font-weight:500;margin-bottom:6px;';
            ga4Label.textContent = 'GA4 Property';
            ga4Wrap.appendChild(ga4Label);
            if (data.ga4_property_id) {
                var cur4 = document.createElement('div');
                cur4.style.cssText = 'font-size:12px;color:var(--text-muted);margin-bottom:6px;';
                cur4.textContent = 'Current: ' + (data.ga4_property_display_name || data.ga4_property_id);
                ga4Wrap.appendChild(cur4);
            }
            var ga4Sel = document.createElement('select');
            ga4Sel.className = 'admin-input';
            ga4Sel.style.cssText = 'max-width:400px;';
            var ga4Loading = document.createElement('option'); ga4Loading.textContent = 'Loading properties...';
            ga4Sel.appendChild(ga4Loading);
            ga4Wrap.appendChild(ga4Sel);
            var ga4Btn = document.createElement('button');
            ga4Btn.className = 'btn btn-primary btn-sm';
            ga4Btn.style.cssText = 'margin-left:8px;';
            ga4Btn.textContent = 'Save';
            ga4Btn.onclick = async function() {
                var opt = ga4Sel.options[ga4Sel.selectedIndex];
                if (!opt || !opt.value) return;
                var d = JSON.parse(opt.dataset.payload || '{}');
                try {
                    var r = await fetch('/api/modules/seo/google/ga4/select', {
                        method: 'POST', headers: {'Content-Type':'application/json'},
                        body: JSON.stringify(d)
                    }).then(function(r) { return r.json(); });
                    toast(r.ok ? 'GA4 property saved' : (r.message || 'Error'));
                } catch(e) { toast('Error: ' + e.message); }
            };
            ga4Wrap.appendChild(ga4Btn);
            selectorsBody.appendChild(ga4Wrap);

            // Load GA4 properties async
            fetch('/api/modules/seo/google/ga4/properties').then(function(r) { return r.json(); }).then(function(pr) {
                ga4Sel.replaceChildren();
                var blank = document.createElement('option'); blank.textContent = '— Select property —'; blank.value = '';
                ga4Sel.appendChild(blank);
                var props = (pr.ok && pr.data && Array.isArray(pr.data.properties)) ? pr.data.properties : [];
                props.forEach(function(p) {
                    var opt = document.createElement('option');
                    opt.value = p.property_id || p.id || '';
                    opt.textContent = (p.property_display_name || p.display_name || p.property_id || '') + (p.measurement_id ? ' (' + p.measurement_id + ')' : '');
                    if (opt.value === data.ga4_property_id) opt.selected = true;
                    opt.dataset.payload = JSON.stringify({
                        property_id: p.property_id || p.id || '',
                        property_display_name: p.property_display_name || p.display_name || '',
                        account_display_name: p.account_display_name || '',
                        measurement_id: p.measurement_id || ''
                    });
                    ga4Sel.appendChild(opt);
                });
                if (props.length === 0) {
                    var empty = document.createElement('option'); empty.textContent = 'No properties found'; empty.value = '';
                    ga4Sel.appendChild(empty);
                }
            }).catch(function() {
                ga4Sel.replaceChildren();
                var err = document.createElement('option'); err.textContent = 'Error loading properties';
                ga4Sel.appendChild(err);
            });

            // GSC
            var gscWrap = document.createElement('div');
            gscWrap.style.cssText = 'margin-bottom:20px;';
            var gscLabel = document.createElement('label');
            gscLabel.style.cssText = 'display:block;font-size:13px;font-weight:500;margin-bottom:6px;';
            gscLabel.textContent = 'Search Console Site';
            gscWrap.appendChild(gscLabel);
            if (data.gsc_site_url) {
                var curGsc = document.createElement('div');
                curGsc.style.cssText = 'font-size:12px;color:var(--text-muted);margin-bottom:6px;';
                curGsc.textContent = 'Current: ' + data.gsc_site_url;
                gscWrap.appendChild(curGsc);
            }
            var gscSel = document.createElement('select');
            gscSel.className = 'admin-input';
            gscSel.style.cssText = 'max-width:400px;';
            var gscLoading = document.createElement('option'); gscLoading.textContent = 'Loading sites...';
            gscSel.appendChild(gscLoading);
            gscWrap.appendChild(gscSel);
            var gscBtn = document.createElement('button');
            gscBtn.className = 'btn btn-primary btn-sm';
            gscBtn.style.cssText = 'margin-left:8px;';
            gscBtn.textContent = 'Save';
            gscBtn.onclick = async function() {
                var opt = gscSel.options[gscSel.selectedIndex];
                if (!opt || !opt.value) return;
                try {
                    var r = await fetch('/api/modules/seo/google/gsc/select', {
                        method: 'POST', headers: {'Content-Type':'application/json'},
                        body: JSON.stringify({ site_url: opt.value, permission_level: opt.dataset.perm || '' })
                    }).then(function(r) { return r.json(); });
                    toast(r.ok ? 'Search Console site saved' : (r.message || 'Error'));
                } catch(e) { toast('Error: ' + e.message); }
            };
            gscWrap.appendChild(gscBtn);
            selectorsBody.appendChild(gscWrap);

            fetch('/api/modules/seo/google/gsc/sites').then(function(r) { return r.json(); }).then(function(sr) {
                gscSel.replaceChildren();
                var blank = document.createElement('option'); blank.textContent = '— Select site —'; blank.value = '';
                gscSel.appendChild(blank);
                var sites = (sr.ok && sr.data && Array.isArray(sr.data.sites)) ? sr.data.sites : [];
                sites.forEach(function(s) {
                    var opt = document.createElement('option');
                    opt.value = s.site_url || s.url || '';
                    opt.textContent = (s.site_url || s.url || '') + (s.permission_level ? ' [' + s.permission_level + ']' : '');
                    opt.dataset.perm = s.permission_level || '';
                    if (opt.value === data.gsc_site_url) opt.selected = true;
                    gscSel.appendChild(opt);
                });
                if (sites.length === 0) {
                    var empty = document.createElement('option'); empty.textContent = 'No sites found'; empty.value = '';
                    gscSel.appendChild(empty);
                }
            }).catch(function() {
                gscSel.replaceChildren();
                var err = document.createElement('option'); err.textContent = 'Error loading sites';
                gscSel.appendChild(err);
            });

            // Ads
            var adsWrap = document.createElement('div');
            adsWrap.style.cssText = 'margin-bottom:8px;';
            var adsLabel = document.createElement('label');
            adsLabel.style.cssText = 'display:block;font-size:13px;font-weight:500;margin-bottom:6px;';
            adsLabel.textContent = 'Google Ads Customer';
            adsWrap.appendChild(adsLabel);
            if (data.ads_customer_id) {
                var curAds = document.createElement('div');
                curAds.style.cssText = 'font-size:12px;color:var(--text-muted);margin-bottom:6px;';
                curAds.textContent = 'Current: ' + (data.ads_customer_display_name || data.ads_customer_id);
                adsWrap.appendChild(curAds);
            }
            var adsSel = document.createElement('select');
            adsSel.className = 'admin-input';
            adsSel.style.cssText = 'max-width:400px;';
            var adsLoading = document.createElement('option'); adsLoading.textContent = 'Loading customers...';
            adsSel.appendChild(adsLoading);
            adsWrap.appendChild(adsSel);
            var adsBtn = document.createElement('button');
            adsBtn.className = 'btn btn-primary btn-sm';
            adsBtn.style.cssText = 'margin-left:8px;';
            adsBtn.textContent = 'Save';
            adsBtn.onclick = async function() {
                var opt = adsSel.options[adsSel.selectedIndex];
                if (!opt || !opt.value) return;
                try {
                    var r = await fetch('/api/modules/seo/google/ads/select', {
                        method: 'POST', headers: {'Content-Type':'application/json'},
                        body: JSON.stringify({ customer_id: opt.value, customer_display_name: opt.dataset.name || '' })
                    }).then(function(r) { return r.json(); });
                    toast(r.ok ? 'Ads customer saved' : (r.message || 'Error'));
                } catch(e) { toast('Error: ' + e.message); }
            };
            adsWrap.appendChild(adsBtn);
            selectorsBody.appendChild(adsWrap);

            fetch('/api/modules/seo/google/ads/customers').then(function(r) { return r.json(); }).then(function(ar) {
                adsSel.replaceChildren();
                var blank = document.createElement('option'); blank.textContent = '— Select customer —'; blank.value = '';
                adsSel.appendChild(blank);
                if (ar.ok && ar.data && ar.data.optional) {
                    var note = document.createElement('option'); note.textContent = ar.data.reason || 'Ads not available'; note.value = '';
                    adsSel.appendChild(note);
                    return;
                }
                var custs = (ar.ok && ar.data && Array.isArray(ar.data.customers)) ? ar.data.customers : [];
                custs.forEach(function(c) {
                    var opt = document.createElement('option');
                    opt.value = c.customer_id || c.id || '';
                    opt.textContent = (c.display_name || c.customer_id || '');
                    opt.dataset.name = c.display_name || '';
                    if (opt.value === data.ads_customer_id) opt.selected = true;
                    adsSel.appendChild(opt);
                });
                if (custs.length === 0) {
                    var empty = document.createElement('option'); empty.textContent = 'No customers found'; empty.value = '';
                    adsSel.appendChild(empty);
                }
            }).catch(function() {
                adsSel.replaceChildren();
                var err = document.createElement('option'); err.textContent = 'Error loading customers';
                adsSel.appendChild(err);
            });
        } catch(e) {
            selectorsBody.textContent = 'Failed to load configuration.';
        }
    }

    // ── Advanced: Direct OAuth ──
    var advCard = googleCard(null);
    var advToggle = document.createElement('button');
    advToggle.className = 'btn btn-ghost btn-sm';
    advToggle.style.cssText = 'margin-bottom:12px;font-size:12px;';
    advToggle.textContent = 'Advanced: Direct OAuth ▸';
    var advBody = document.createElement('div');
    advBody.style.display = 'none';
    advToggle.onclick = function() {
        if (advBody.style.display === 'none') {
            advBody.style.display = 'block';
            advToggle.textContent = 'Advanced: Direct OAuth ▾';
        } else {
            advBody.style.display = 'none';
            advToggle.textContent = 'Advanced: Direct OAuth ▸';
        }
    };

    var cidLabel = document.createElement('label');
    cidLabel.style.cssText = 'display:block;font-size:13px;margin-bottom:4px;';
    cidLabel.textContent = 'Client ID';
    var cidInput = document.createElement('input');
    cidInput.type = 'text'; cidInput.className = 'admin-input';
    cidInput.placeholder = 'your-client-id.apps.googleusercontent.com';
    cidInput.style.cssText = 'max-width:420px;display:block;margin-bottom:12px;';
    advBody.appendChild(cidLabel); advBody.appendChild(cidInput);

    var csLabel = document.createElement('label');
    csLabel.style.cssText = 'display:block;font-size:13px;margin-bottom:4px;';
    csLabel.textContent = 'Client Secret';
    var csInput = document.createElement('input');
    csInput.type = 'password'; csInput.className = 'admin-input';
    csInput.placeholder = 'GOCSPX-...';
    csInput.style.cssText = 'max-width:420px;display:block;margin-bottom:12px;';
    advBody.appendChild(csLabel); advBody.appendChild(csInput);

    var advBtnRow = document.createElement('div');
    advBtnRow.style.cssText = 'display:flex;gap:8px;flex-wrap:wrap;';
    var saveCredsBtn = document.createElement('button');
    saveCredsBtn.className = 'btn btn-primary btn-sm';
    saveCredsBtn.textContent = 'Save Credentials';
    saveCredsBtn.onclick = async function() {
        try {
            var r = await fetch('/api/modules/seo/google/config', {
                method: 'PUT', headers: {'Content-Type':'application/json'},
                body: JSON.stringify({ direct_client_id: cidInput.value.trim(), direct_client_secret: csInput.value.trim() })
            }).then(function(r) { return r.json(); });
            toast(r.ok ? 'Credentials saved' : (r.message || 'Error'));
        } catch(e) { toast('Error: ' + e.message); }
    };
    var startDirectBtn = document.createElement('button');
    startDirectBtn.className = 'btn btn-ghost btn-sm';
    startDirectBtn.textContent = 'Start Direct OAuth';
    startDirectBtn.onclick = async function() {
        try {
            var ar = await fetch('/api/modules/seo/google/auth-url?redirect_uri=' + encodeURIComponent(window.location.origin + '/admin?google_status=connected')).then(function(r) { return r.json(); });
            if (ar.ok && ar.data && ar.data.url) {
                window.open(ar.data.url, '_blank');
            } else {
                toast(ar.message || 'Could not get auth URL');
            }
        } catch(e) { toast('Error: ' + e.message); }
    };
    advBtnRow.appendChild(saveCredsBtn);
    advBtnRow.appendChild(startDirectBtn);
    advBody.appendChild(advBtnRow);

    advCard.appendChild(advToggle);
    advCard.appendChild(advBody);
    el.appendChild(advCard);

    main.replaceChildren(el);
    await refreshStatus();
    await loadPropertySelectors();
}

// ── View: Google Overview ──────────────────────────────────────────
async function load_seo_google() {
    var main = document.getElementById('adminMain');
    var el = document.createElement('div');
    var h = document.createElement('h2'); h.textContent = 'Google Overview'; el.appendChild(h);
    main.replaceChildren(el);

    try {
        var cfg = await fetch('/api/modules/seo/google/config').then(function(r) { return r.json(); });
        var data = (cfg.ok && cfg.data) ? cfg.data : {};

        if (!data.authenticated) {
            el.appendChild(googleNotConnected(
                'Connect your Google account to view analytics, Search Console data, and Ads.',
                'Connect Google', 'seo-google-settings'
            ));
            return;
        }

        // Service status cards
        var servTitle = document.createElement('h3');
        servTitle.style.cssText = 'margin-bottom:12px;font-size:15px;';
        servTitle.textContent = 'Connected Services';
        el.appendChild(servTitle);

        var servRow = document.createElement('div');
        servRow.style.cssText = 'display:grid;grid-template-columns:repeat(auto-fit,minmax(200px,1fr));gap:12px;margin-bottom:24px;';

        [
            ['GA4 Analytics', data.ga4_property_id, data.ga4_property_display_name, 'seo-ga4'],
            ['Search Console', data.gsc_site_url, data.gsc_site_url, 'seo-gsc'],
            ['Google Ads', data.ads_customer_id, data.ads_customer_display_name, 'seo-ads'],
        ].forEach(function(item) {
            var card = document.createElement('div');
            card.style.cssText = 'padding:16px;background:var(--surface);border:1px solid var(--border);border-radius:8px;cursor:pointer;';
            card.onclick = function() { navigateTo(item[3]); };
            var nameEl = document.createElement('div');
            nameEl.style.cssText = 'font-weight:600;font-size:14px;margin-bottom:8px;';
            nameEl.textContent = item[0];
            card.appendChild(nameEl);
            var badge = googleBadge(item[1] ? 'Connected' : 'Not Set', !!item[1]);
            card.appendChild(badge);
            if (item[1]) {
                var detail = document.createElement('div');
                detail.style.cssText = 'font-size:12px;color:var(--text-muted);margin-top:6px;word-break:break-all;';
                detail.textContent = item[2] || item[1];
                card.appendChild(detail);
            }
            servRow.appendChild(card);
        });
        el.appendChild(servRow);

        // Date range selector
        var dateBar = document.createElement('div');
        dateBar.style.cssText = 'display:flex;align-items:center;gap:8px;flex-wrap:wrap;margin-bottom:20px;';
        var ovStartInput = document.createElement('input'); ovStartInput.type = 'date'; ovStartInput.className = 'admin-input'; ovStartInput.style.cssText = 'width:160px;'; ovStartInput.value = googleDateStr(7);
        var ovEndInput = document.createElement('input'); ovEndInput.type = 'date'; ovEndInput.className = 'admin-input'; ovEndInput.style.cssText = 'width:160px;'; ovEndInput.value = googleDateStr(0);
        var ovSep = document.createElement('span'); ovSep.textContent = 'to'; ovSep.style.cssText = 'color:var(--text-muted);';
        dateBar.appendChild(ovStartInput); dateBar.appendChild(ovSep); dateBar.appendChild(ovEndInput);
        var ovRefresh = document.createElement('button'); ovRefresh.className = 'btn btn-primary btn-sm'; ovRefresh.textContent = 'Refresh';
        dateBar.appendChild(ovRefresh);
        el.appendChild(dateBar);

        // Export buttons
        var ovExportBar = document.createElement('div'); ovExportBar.style.cssText = 'display:flex;gap:8px;margin-bottom:16px;';
        var _ovAllData = {};
        var ovExpJson = document.createElement('button'); ovExpJson.className = 'btn btn-ghost btn-sm'; ovExpJson.textContent = 'Export JSON';
        ovExpJson.onclick = function() { lqExportJSON(_ovAllData, 'google-overview.json'); };
        ovExportBar.appendChild(ovExpJson);
        var ovExpCsv = document.createElement('button'); ovExpCsv.className = 'btn btn-ghost btn-sm'; ovExpCsv.textContent = 'Export CSV';
        ovExpCsv.onclick = function() {
            var rows = [];
            if (_ovAllData.queries) _ovAllData.queries.forEach(function(q) { rows.push({type:'query', name:q.query||'', clicks:q.clicks||0, impressions:q.impressions||0, ctr:q.ctr||0, position:q.position||0}); });
            if (_ovAllData.pages) _ovAllData.pages.forEach(function(p) { rows.push({type:'page', name:p.page||p.page_path||'', clicks:p.clicks||0, impressions:p.impressions||0, ctr:p.ctr||0, position:p.position||0}); });
            if (rows.length > 0) lqExportCSV(rows, ['type','name','clicks','impressions','ctr','position'], 'google-overview.csv');
            else toast('No data to export');
        };
        ovExportBar.appendChild(ovExpCsv);
        el.appendChild(ovExportBar);

        // Section toggle state
        var _ovShowGA4 = true, _ovShowGSC = true;

        // Data sections container
        var statsBody = document.createElement('div');
        el.appendChild(statsBody);

        async function loadOverviewData() {
            var start = ovStartInput.value || googleDateStr(7);
            var end = ovEndInput.value || googleDateStr(0);
            statsBody.replaceChildren();
            var loadEl = document.createElement('p'); loadEl.style.cssText = 'color:var(--text-muted);'; loadEl.textContent = 'Loading...'; statsBody.appendChild(loadEl);

            var promises = [];
            if (data.ga4_property_id) {
                promises.push(fetch('/api/modules/seo/google/ga4/traffic?start_date=' + start + '&end_date=' + end + '&row_limit=1').then(function(r){return r.json();}).catch(function(){return null;}));
                promises.push(fetch('/api/modules/seo/google/ga4/pages?start_date=' + start + '&end_date=' + end + '&row_limit=5').then(function(r){return r.json();}).catch(function(){return null;}));
            } else { promises.push(Promise.resolve(null)); promises.push(Promise.resolve(null)); }
            if (data.gsc_site_url) {
                promises.push(fetch('/api/modules/seo/google/gsc/queries?start_date=' + start + '&end_date=' + end + '&row_limit=5').then(function(r){return r.json();}).catch(function(){return null;}));
                promises.push(fetch('/api/modules/seo/google/gsc/pages?start_date=' + start + '&end_date=' + end + '&row_limit=5').then(function(r){return r.json();}).catch(function(){return null;}));
            } else { promises.push(Promise.resolve(null)); promises.push(Promise.resolve(null)); }

            var results = await Promise.all(promises);
            statsBody.replaceChildren();
            _ovAllData = {};

            // GA4 Section
            var ga4Section = document.createElement('div'); ga4Section.id = 'ov-ga4-section';
            var ga4Toggle = document.createElement('div');
            ga4Toggle.style.cssText = 'display:flex;align-items:center;gap:8px;cursor:pointer;margin-bottom:12px;';
            var ga4Arrow = document.createElement('span'); ga4Arrow.textContent = _ovShowGA4 ? '▾' : '▸'; ga4Arrow.style.fontSize = '12px';
            var ga4Title = document.createElement('h3'); ga4Title.style.cssText = 'font-size:15px;margin:0;'; ga4Title.textContent = 'GA4 Analytics';
            ga4Toggle.appendChild(ga4Arrow); ga4Toggle.appendChild(ga4Title);
            ga4Toggle.onclick = function() { _ovShowGA4 = !_ovShowGA4; ga4Arrow.textContent = _ovShowGA4 ? '▾' : '▸'; ga4Body2.style.display = _ovShowGA4 ? '' : 'none'; };
            ga4Section.appendChild(ga4Toggle);
            var ga4Body2 = document.createElement('div');
            ga4Body2.style.display = _ovShowGA4 ? '' : 'none';

            var trafficData = results[0];
            if (trafficData && trafficData.ok && trafficData.data) {
                var td2 = trafficData.data;
                var totals = td2.summary || td2.totals || td2[0] || {};
                ga4Body2.appendChild(googleStatCards([
                    ['Total Users', googleFmt(totals.total_users || totals.users)],
                    ['Sessions', googleFmt(totals.sessions)],
                    ['Pageviews', googleFmt(totals.pageviews || totals.event_count || totals.events)],
                    ['Bounce Rate', googlePct(totals.bounce_rate)],
                ]));
            } else if (data.ga4_property_id) {
                var noTraffic = document.createElement('p'); noTraffic.style.cssText = 'color:var(--text-muted);font-size:13px;margin-bottom:12px;'; noTraffic.textContent = 'GA4 traffic data unavailable.'; ga4Body2.appendChild(noTraffic);
            }

            // GA4 top pages
            var ga4PagesR = results[1];
            var ga4Pages = (ga4PagesR && ga4PagesR.ok && ga4PagesR.data) ? (ga4PagesR.data.pages || ga4PagesR.data) : [];
            if (!Array.isArray(ga4Pages)) ga4Pages = [];
            _ovAllData.ga4Pages = ga4Pages;
            if (ga4Pages.length > 0) {
                var pgH = document.createElement('h4'); pgH.style.cssText = 'margin:12px 0 6px;font-size:13px;'; pgH.textContent = 'Top Pages'; ga4Body2.appendChild(pgH);
                var pgRows = ga4Pages.map(function(p) { return [p.page_path||p.path||'—', googleFmt(p.pageviews||p.screen_page_views||p.views)]; });
                ga4Body2.appendChild(googleTable(['Page', 'Views'], pgRows));
            }

            ga4Section.appendChild(ga4Body2);
            statsBody.appendChild(ga4Section);

            // GSC Section
            var gscSection = document.createElement('div'); gscSection.style.cssText = 'margin-top:20px;';
            var gscToggle = document.createElement('div');
            gscToggle.style.cssText = 'display:flex;align-items:center;gap:8px;cursor:pointer;margin-bottom:12px;';
            var gscArrow = document.createElement('span'); gscArrow.textContent = _ovShowGSC ? '▾' : '▸'; gscArrow.style.fontSize = '12px';
            var gscTitle = document.createElement('h3'); gscTitle.style.cssText = 'font-size:15px;margin:0;'; gscTitle.textContent = 'Search Console';
            gscToggle.appendChild(gscArrow); gscToggle.appendChild(gscTitle);
            gscToggle.onclick = function() { _ovShowGSC = !_ovShowGSC; gscArrow.textContent = _ovShowGSC ? '▾' : '▸'; gscBody2.style.display = _ovShowGSC ? '' : 'none'; };
            gscSection.appendChild(gscToggle);
            var gscBody2 = document.createElement('div');
            gscBody2.style.display = _ovShowGSC ? '' : 'none';

            var queryData = results[2];
            var qArr = (queryData && queryData.ok && queryData.data) ? (queryData.data.queries || queryData.data) : [];
            if (!Array.isArray(qArr)) qArr = [];
            _ovAllData.queries = qArr;
            if (qArr.length > 0) {
                var qTitle = document.createElement('h4'); qTitle.style.cssText = 'margin:0 0 6px;font-size:13px;'; qTitle.textContent = 'Top Queries';
                gscBody2.appendChild(qTitle);
                // Clickable query rows
                var qWrap = document.createElement('div'); qWrap.style.cssText = 'overflow-x:auto;';
                var qTbl = document.createElement('table'); qTbl.className = 'content-table';
                var qThead = document.createElement('thead'); var qHrow = document.createElement('tr');
                ['Query', 'Clicks', 'Impressions'].forEach(function(h) { var th = document.createElement('th'); th.textContent = h; qHrow.appendChild(th); });
                qThead.appendChild(qHrow); qTbl.appendChild(qThead);
                var qTbody = document.createElement('tbody');
                qArr.slice(0, 5).forEach(function(q) {
                    var tr = document.createElement('tr');
                    var qName = q.query || q.keys || '';
                    var qTd = document.createElement('td');
                    var qLink = document.createElement('span');
                    qLink.textContent = qName || '—';
                    qLink.style.cssText = 'cursor:pointer;color:var(--accent);';
                    qLink.onmouseenter = function() { qLink.style.textDecoration = 'underline'; };
                    qLink.onmouseleave = function() { qLink.style.textDecoration = 'none'; };
                    qLink.onclick = function() { openQueryDetailModal(qName, {start: start, end: end}); };
                    qTd.appendChild(qLink); tr.appendChild(qTd);
                    var cTd = document.createElement('td'); cTd.textContent = googleFmt(q.clicks); tr.appendChild(cTd);
                    var iTd = document.createElement('td'); iTd.textContent = googleFmt(q.impressions); tr.appendChild(iTd);
                    qTbody.appendChild(tr);
                });
                qTbl.appendChild(qTbody); qWrap.appendChild(qTbl); gscBody2.appendChild(qWrap);
            } else if (data.gsc_site_url) {
                var noQ = document.createElement('p'); noQ.style.cssText = 'color:var(--text-muted);font-size:13px;'; noQ.textContent = 'No query data available.'; gscBody2.appendChild(noQ);
            }

            // GSC top pages
            var pagesR = results[3];
            var pArr = (pagesR && pagesR.ok && pagesR.data) ? (pagesR.data.pages || pagesR.data) : [];
            if (!Array.isArray(pArr)) pArr = [];
            _ovAllData.pages = pArr;
            if (pArr.length > 0) {
                var pTitle = document.createElement('h4'); pTitle.style.cssText = 'margin:12px 0 6px;font-size:13px;'; pTitle.textContent = 'Top Pages';
                gscBody2.appendChild(pTitle);
                gscBody2.appendChild(googleClickablePageTable(pArr, {start: start, end: end}));
            }

            gscSection.appendChild(gscBody2);
            statsBody.appendChild(gscSection);
        }

        ovRefresh.onclick = loadOverviewData;
        await loadOverviewData();

        var settingsLink = document.createElement('button');
        settingsLink.className = 'btn btn-ghost btn-sm';
        settingsLink.style.cssText = 'margin-top:20px;';
        settingsLink.textContent = 'Manage Settings';
        settingsLink.onclick = function() { navigateTo('seo-google-settings'); };
        el.appendChild(settingsLink);

    } catch(e) {
        var errEl = document.createElement('p');
        errEl.style.cssText = 'color:#ef4444;';
        errEl.textContent = 'Error loading Google overview: ' + e.message;
        el.appendChild(errEl);
    }
}

// ── View: GA4 Analytics ────────────────────────────────────────────
async function load_seo_ga4() {
    var main = document.getElementById('adminMain');
    var el = document.createElement('div');
    var h = document.createElement('h2'); h.textContent = 'GA4 Analytics'; el.appendChild(h);
    main.replaceChildren(el);

    try {
        var cfg = await fetch('/api/modules/seo/google/config').then(function(r) { return r.json(); });
        var data = (cfg.ok && cfg.data) ? cfg.data : {};

        if (!data.authenticated || !data.ga4_property_id) {
            el.appendChild(googleNotConnected(
                data.authenticated ? 'No GA4 property selected. Configure it in Settings.' : 'Connect your Google account in Settings first.',
                'Go to Settings', 'seo-google-settings'
            ));
            return;
        }

        var isPro = (function() {
            var role = (window.__CMS && window.__CMS.nexusRole) || '';
            return role === 'central' || role === 'professional' || role === 'enterprise';
        })();

        // Property name
        var propLine = document.createElement('p');
        propLine.style.cssText = 'color:var(--text-muted);font-size:13px;margin-bottom:16px;';
        propLine.textContent = 'Property: ' + (data.ga4_property_display_name || data.ga4_property_id);
        el.appendChild(propLine);

        // Date range bar
        var dateBar = document.createElement('div');
        dateBar.style.cssText = 'display:flex;align-items:center;gap:8px;flex-wrap:wrap;margin-bottom:20px;';
        var startInput = document.createElement('input');
        startInput.type = 'date'; startInput.className = 'admin-input'; startInput.style.cssText = 'width:160px;';
        startInput.value = googleDateStr(7);
        startInput.min = googleMinDate();
        var endInput = document.createElement('input');
        endInput.type = 'date'; endInput.className = 'admin-input'; endInput.style.cssText = 'width:160px;';
        endInput.value = googleDateStr(0);
        var sep = document.createElement('span'); sep.textContent = 'to'; sep.style.cssText = 'color:var(--text-muted);';
        dateBar.appendChild(startInput); dateBar.appendChild(sep); dateBar.appendChild(endInput);

        if (!isPro) {
            var tierHint = document.createElement('span');
            tierHint.style.cssText = 'font-size:11px;color:var(--text-muted);';
            tierHint.textContent = 'Upgrade to Professional for 18 months of history';
            dateBar.appendChild(tierHint);
        }

        var refreshBtn = document.createElement('button');
        refreshBtn.className = 'btn btn-primary btn-sm';
        refreshBtn.textContent = 'Refresh';
        dateBar.appendChild(refreshBtn);
        el.appendChild(dateBar);

        // Data sections
        var ga4Body = document.createElement('div');
        el.appendChild(ga4Body);

        async function loadGa4Data() {
            var start = startInput.value || googleDateStr(7);
            var end = endInput.value || googleDateStr(0);
            ga4Body.replaceChildren();

            var loadingEl = document.createElement('p');
            loadingEl.style.cssText = 'color:var(--text-muted);';
            loadingEl.textContent = 'Loading...';
            ga4Body.appendChild(loadingEl);

            try {
                var qs = '?start_date=' + start + '&end_date=' + end;
                var results = await Promise.all([
                    fetch('/api/modules/seo/google/ga4/traffic' + qs + '&row_limit=1').then(function(r) { return r.json(); }).catch(function() { return null; }),
                    fetch('/api/modules/seo/google/ga4/timeseries' + qs).then(function(r) { return r.json(); }).catch(function() { return null; }),
                    fetch('/api/modules/seo/google/ga4/sources' + qs + '&row_limit=10').then(function(r) { return r.json(); }).catch(function() { return null; }),
                    fetch('/api/modules/seo/google/ga4/pages' + qs + '&row_limit=10').then(function(r) { return r.json(); }).catch(function() { return null; }),
                ]);
                ga4Body.replaceChildren();

                // Traffic summary
                var trafficR = results[0];
                if (trafficR && trafficR.ok && trafficR.data) {
                    var td = trafficR.data;
                    var totals = td.summary || td.totals || td[0] || {};
                    var sc = googleStatCards([
                        ['Total Users', googleFmt(totals.total_users || totals.users)],
                        ['Sessions', googleFmt(totals.sessions)],
                        ['Pageviews', googleFmt(totals.pageviews || totals.event_count || totals.events)],
                        ['Bounce Rate', googlePct(totals.bounce_rate)],
                    ]);
                    ga4Body.appendChild(sc);
                }

                // Timeseries bar chart
                var tsR = results[1];
                var tsData = (tsR && tsR.ok && tsR.data) ? (tsR.data.data || tsR.data) : [];
                if (!Array.isArray(tsData)) tsData = [];
                if (tsData.length > 0) {
                    var tsTitle = document.createElement('h3');
                    tsTitle.style.cssText = 'margin:20px 0 12px;font-size:14px;';
                    tsTitle.textContent = 'Daily Sessions';
                    ga4Body.appendChild(tsTitle);

                    var chartWrap = document.createElement('div');
                    chartWrap.style.cssText = 'display:flex;align-items:flex-end;gap:3px;height:80px;background:var(--surface);border:1px solid var(--border);border-radius:8px;padding:8px;overflow-x:auto;';
                    var vals = tsData.map(function(d) { return parseFloat(d.sessions || d.value || 0) || 0; });
                    var maxVal = Math.max.apply(null, vals) || 1;
                    tsData.forEach(function(d, i) {
                        var bar = document.createElement('div');
                        var pct = Math.max(4, Math.round((vals[i] / maxVal) * 64));
                        bar.style.cssText = 'flex:1;min-width:6px;background:var(--accent);border-radius:2px 2px 0 0;';
                        bar.style.height = pct + 'px';
                        bar.title = (d.date || '') + ': ' + vals[i];
                        chartWrap.appendChild(bar);
                    });
                    ga4Body.appendChild(chartWrap);
                }

                // Sources table — sortable
                var srcR = results[2];
                var srcData = (srcR && srcR.ok && srcR.data) ? (srcR.data.sources || srcR.data) : [];
                if (!Array.isArray(srcData)) srcData = [];
                if (srcData.length > 0) {
                    var srcTitle = document.createElement('h3');
                    srcTitle.style.cssText = 'margin:20px 0 8px;font-size:14px;';
                    srcTitle.textContent = 'Traffic Sources';
                    ga4Body.appendChild(srcTitle);
                    var _ga4SrcSort = 1, _ga4SrcAsc = false;
                    function renderGA4Sources() {
                        var items = srcData.slice().sort(function(a, b) {
                            var keys = ['_name', '_users', '_sessions'];
                            if (_ga4SrcSort === 0) {
                                var an = a.channel || a.source || a.session_source || '';
                                var bn = b.channel || b.source || b.session_source || '';
                                return _ga4SrcAsc ? an.localeCompare(bn) : bn.localeCompare(an);
                            }
                            var av = _ga4SrcSort === 1 ? parseFloat(a.total_users || a.users || 0) : parseFloat(a.sessions || 0);
                            var bv = _ga4SrcSort === 1 ? parseFloat(b.total_users || b.users || 0) : parseFloat(b.sessions || 0);
                            return _ga4SrcAsc ? av - bv : bv - av;
                        });
                        var wrap = document.createElement('div'); wrap.style.cssText = 'overflow-x:auto;'; wrap.id = 'ga4-src-table';
                        var tbl = document.createElement('table'); tbl.className = 'content-table';
                        var thead = document.createElement('thead'); var hrow = document.createElement('tr');
                        ['Source', 'Users', 'Sessions'].forEach(function(h, i) {
                            var th = document.createElement('th');
                            th.style.cssText = 'cursor:pointer;user-select:none;';
                            th.textContent = h + (_ga4SrcSort === i ? (_ga4SrcAsc ? ' ▲' : ' ▼') : '');
                            th.onclick = function() {
                                if (_ga4SrcSort === i) _ga4SrcAsc = !_ga4SrcAsc; else { _ga4SrcSort = i; _ga4SrcAsc = false; }
                                var old = document.getElementById('ga4-src-table');
                                if (old) old.replaceWith(renderGA4Sources());
                            };
                            hrow.appendChild(th);
                        });
                        thead.appendChild(hrow); tbl.appendChild(thead);
                        var tbody = document.createElement('tbody');
                        items.forEach(function(s) {
                            var tr = document.createElement('tr');
                            [s.channel || s.source || s.session_source || '—', googleFmt(s.total_users || s.users), googleFmt(s.sessions)].forEach(function(v) {
                                var td = document.createElement('td'); td.textContent = v; tr.appendChild(td);
                            });
                            tbody.appendChild(tr);
                        });
                        tbl.appendChild(tbody); wrap.appendChild(tbl); return wrap;
                    }
                    ga4Body.appendChild(renderGA4Sources());
                }

                // Pages table — sortable + clickable
                var pagesR = results[3];
                var pagesData = (pagesR && pagesR.ok && pagesR.data) ? (pagesR.data.pages || pagesR.data) : [];
                if (!Array.isArray(pagesData)) pagesData = [];
                if (pagesData.length > 0) {
                    var pagesTitle = document.createElement('h3');
                    pagesTitle.style.cssText = 'margin:20px 0 8px;font-size:14px;';
                    pagesTitle.textContent = 'Top Pages';
                    ga4Body.appendChild(pagesTitle);
                    var _ga4PgSort = 1, _ga4PgAsc = false;
                    function renderGA4Pages() {
                        var items = pagesData.slice().sort(function(a, b) {
                            if (_ga4PgSort === 0) {
                                var an = a.page_path || a.path || '';
                                var bn = b.page_path || b.path || '';
                                return _ga4PgAsc ? an.localeCompare(bn) : bn.localeCompare(an);
                            }
                            var av, bv;
                            if (_ga4PgSort === 1) { av = parseFloat(a.pageviews || a.screen_page_views || a.views || 0); bv = parseFloat(b.pageviews || b.screen_page_views || b.views || 0); }
                            else { av = parseFloat(a.avg_duration || a.user_engagement_duration || 0); bv = parseFloat(b.avg_duration || b.user_engagement_duration || 0); }
                            return _ga4PgAsc ? av - bv : bv - av;
                        });
                        var wrap = document.createElement('div'); wrap.style.cssText = 'overflow-x:auto;'; wrap.id = 'ga4-pages-table';
                        var tbl = document.createElement('table'); tbl.className = 'content-table';
                        var thead = document.createElement('thead'); var hrow = document.createElement('tr');
                        ['Page Path', 'Views', 'Avg Time'].forEach(function(h, i) {
                            var th = document.createElement('th');
                            th.style.cssText = 'cursor:pointer;user-select:none;';
                            th.textContent = h + (_ga4PgSort === i ? (_ga4PgAsc ? ' ▲' : ' ▼') : '');
                            th.onclick = function() {
                                if (_ga4PgSort === i) _ga4PgAsc = !_ga4PgAsc; else { _ga4PgSort = i; _ga4PgAsc = false; }
                                var old = document.getElementById('ga4-pages-table');
                                if (old) old.replaceWith(renderGA4Pages());
                            };
                            hrow.appendChild(th);
                        });
                        thead.appendChild(hrow); tbl.appendChild(thead);
                        var tbody = document.createElement('tbody');
                        items.forEach(function(p) {
                            var tr = document.createElement('tr');
                            var pagePath = p.page_path || p.path || '—';
                            var views = googleFmt(p.pageviews || p.screen_page_views || p.views);
                            var avgTime = p.avg_duration != null ? Math.round(parseFloat(p.avg_duration || 0)) + 's' : (p.user_engagement_duration != null ? Math.round(parseFloat(p.user_engagement_duration || 0)) + 's' : '—');
                            // Page path — clickable to Page Detail Modal
                            var pathTd = document.createElement('td');
                            var pathLink = document.createElement('span');
                            pathLink.textContent = pagePath;
                            pathLink.style.cssText = 'cursor:pointer;color:var(--accent);word-break:break-all;';
                            pathLink.onmouseenter = function() { pathLink.style.textDecoration = 'underline'; };
                            pathLink.onmouseleave = function() { pathLink.style.textDecoration = 'none'; };
                            pathLink.onclick = function() { openPageDetailModal(pagePath, { start: startInput.value, end: endInput.value }); };
                            pathTd.appendChild(pathLink);
                            tr.appendChild(pathTd);
                            var viewsTd = document.createElement('td'); viewsTd.textContent = views; tr.appendChild(viewsTd);
                            var timeTd = document.createElement('td'); timeTd.textContent = avgTime; tr.appendChild(timeTd);
                            tbody.appendChild(tr);
                        });
                        tbl.appendChild(tbody); wrap.appendChild(tbl); return wrap;
                    }
                    ga4Body.appendChild(renderGA4Pages());
                }

            } catch(e) {
                ga4Body.replaceChildren();
                var errEl = document.createElement('p'); errEl.style.cssText = 'color:#ef4444;';
                errEl.textContent = 'Error loading GA4 data: ' + e.message;
                ga4Body.appendChild(errEl);
            }
        }

        refreshBtn.onclick = loadGa4Data;
        await loadGa4Data();

    } catch(e) {
        var errEl = document.createElement('p'); errEl.style.cssText = 'color:#ef4444;';
        errEl.textContent = 'Error: ' + e.message;
        el.appendChild(errEl);
    }
}

// ── View: Search Console ───────────────────────────────────────────
async function load_seo_gsc() {
    var main = document.getElementById('adminMain');
    var el = document.createElement('div');
    var h = document.createElement('h2'); h.textContent = 'Search Console'; el.appendChild(h);
    main.replaceChildren(el);

    try {
        var cfg = await fetch('/api/modules/seo/google/config').then(function(r) { return r.json(); });
        var data = (cfg.ok && cfg.data) ? cfg.data : {};

        if (!data.authenticated || !data.gsc_site_url) {
            el.appendChild(googleNotConnected(
                data.authenticated ? 'No Search Console site selected. Configure it in Settings.' : 'Connect your Google account in Settings first.',
                'Go to Settings', 'seo-google-settings'
            ));
            return;
        }

        var isPro = (function() {
            var role = (window.__CMS && window.__CMS.nexusRole) || '';
            return role === 'central' || role === 'professional' || role === 'enterprise';
        })();

        var siteLine = document.createElement('p');
        siteLine.style.cssText = 'color:var(--text-muted);font-size:13px;margin-bottom:16px;';
        siteLine.textContent = 'Site: ' + data.gsc_site_url;
        el.appendChild(siteLine);

        // Date range
        var dateBar = document.createElement('div');
        dateBar.style.cssText = 'display:flex;align-items:center;gap:8px;flex-wrap:wrap;margin-bottom:20px;';
        var startInput = document.createElement('input');
        startInput.type = 'date'; startInput.className = 'admin-input'; startInput.style.cssText = 'width:160px;';
        startInput.value = googleDateStr(28);
        startInput.min = googleMinDate();
        var endInput = document.createElement('input');
        endInput.type = 'date'; endInput.className = 'admin-input'; endInput.style.cssText = 'width:160px;';
        endInput.value = googleDateStr(0);
        var sep = document.createElement('span'); sep.textContent = 'to'; sep.style.cssText = 'color:var(--text-muted);';
        dateBar.appendChild(startInput); dateBar.appendChild(sep); dateBar.appendChild(endInput);

        if (!isPro) {
            var tierHint = document.createElement('span');
            tierHint.style.cssText = 'font-size:11px;color:var(--text-muted);';
            tierHint.textContent = 'Upgrade to Professional for 18 months of history';
            dateBar.appendChild(tierHint);
        }

        var refreshBtn = document.createElement('button');
        refreshBtn.className = 'btn btn-primary btn-sm';
        refreshBtn.textContent = 'Refresh';
        dateBar.appendChild(refreshBtn);
        el.appendChild(dateBar);

        var gscBody = document.createElement('div');
        el.appendChild(gscBody);

        // Sortable query table state
        var _gscSortCol = 1; // clicks
        var _gscSortAsc = false;
        var _gscQData = [];

        function renderQueryTable() {
            var sorted = _gscQData.slice().sort(function(a, b) {
                var fields = ['query', 'clicks', 'impressions', 'ctr', 'position'];
                var f = fields[_gscSortCol];
                var av = a[f] != null ? a[f] : '';
                var bv = b[f] != null ? b[f] : '';
                if (typeof av === 'number' && typeof bv === 'number') {
                    return _gscSortAsc ? av - bv : bv - av;
                }
                return _gscSortAsc ? String(av).localeCompare(String(bv)) : String(bv).localeCompare(String(av));
            });

            var wrap = document.createElement('div');
            wrap.style.cssText = 'overflow-x:auto;';
            var tbl = document.createElement('table');
            tbl.className = 'content-table';
            var thead = document.createElement('thead');
            var hrow = document.createElement('tr');
            var headers = ['Query', 'Clicks', 'Impressions', 'CTR', 'Avg Position'];
            headers.forEach(function(h, i) {
                var th = document.createElement('th');
                th.style.cssText = 'cursor:pointer;user-select:none;';
                th.textContent = h + (_gscSortCol === i ? (_gscSortAsc ? ' ▲' : ' ▼') : '');
                th.onclick = function() {
                    if (_gscSortCol === i) { _gscSortAsc = !_gscSortAsc; }
                    else { _gscSortCol = i; _gscSortAsc = false; }
                    var old = document.getElementById('gsc-query-table');
                    if (old) old.replaceWith(renderQueryTable());
                };
                hrow.appendChild(th);
            });
            thead.appendChild(hrow);
            tbl.appendChild(thead);
            var tbody = document.createElement('tbody');
            sorted.forEach(function(q) {
                var tr = document.createElement('tr');
                var vals = [q.query || '—', googleFmt(q.clicks), googleFmt(q.impressions), googlePct(q.ctr), q.position != null ? parseFloat(q.position).toFixed(1) : '—'];
                vals.forEach(function(val, ci) {
                    var td = document.createElement('td');
                    if (ci === 0 && q.query) {
                        var link = document.createElement('span');
                        link.textContent = val;
                        link.style.cssText = 'cursor:pointer;color:var(--accent);';
                        link.onmouseenter = function() { link.style.textDecoration = 'underline'; };
                        link.onmouseleave = function() { link.style.textDecoration = 'none'; };
                        link.onclick = function(ev) { ev.stopPropagation(); openQueryDetailModal(q.query, { start: startInput.value, end: endInput.value }); };
                        td.appendChild(link);
                    } else { td.textContent = val; }
                    tr.appendChild(td);
                });
                tbody.appendChild(tr);
            });
            tbl.appendChild(tbody);
            wrap.appendChild(tbl);
            wrap.id = 'gsc-query-table';
            return wrap;
        }

        var _gscBreakdownDim = 'device';

        async function loadGscData() {
            var start = startInput.value || googleDateStr(28);
            var end = endInput.value || googleDateStr(0);
            gscBody.replaceChildren();
            var loadEl = document.createElement('p'); loadEl.style.cssText = 'color:var(--text-muted);'; loadEl.textContent = 'Loading...';
            gscBody.appendChild(loadEl);

            try {
                var qs = '?start_date=' + start + '&end_date=' + end;
                var results = await Promise.all([
                    fetch('/api/modules/seo/google/gsc/queries' + qs + '&row_limit=25').then(function(r) { return r.json(); }).catch(function() { return null; }),
                    fetch('/api/modules/seo/google/gsc/pages' + qs + '&row_limit=15').then(function(r) { return r.json(); }).catch(function() { return null; }),
                    fetch('/api/modules/seo/google/gsc/breakdown' + qs + '&dimension=' + _gscBreakdownDim).then(function(r) { return r.json(); }).catch(function() { return null; }),
                ]);
                gscBody.replaceChildren();

                // Query table
                var qR = results[0];
                var qData = (qR && qR.ok && qR.data) ? (qR.data.queries || qR.data) : [];
                if (!Array.isArray(qData)) qData = [];
                if (qData.length > 0) {
                    _gscQData = qData;
                    var qTitle = document.createElement('h3'); qTitle.style.cssText = 'margin-bottom:8px;font-size:14px;'; qTitle.textContent = 'Query Performance';
                    gscBody.appendChild(qTitle);
                    gscBody.appendChild(renderQueryTable());
                }

                // Pages table
                var pR = results[1];
                var pData = (pR && pR.ok && pR.data) ? (pR.data.pages || pR.data) : [];
                if (!Array.isArray(pData)) pData = [];
                if (pData.length > 0) {
                    var pTitle = document.createElement('h3'); pTitle.style.cssText = 'margin:20px 0 8px;font-size:14px;'; pTitle.textContent = 'Page Performance';
                    gscBody.appendChild(pTitle);
                    gscBody.appendChild(googleClickablePageTable(pData, { start: startInput.value, end: endInput.value }));
                }

                // Breakdown section
                var bdTitle = document.createElement('h3'); bdTitle.style.cssText = 'margin:20px 0 8px;font-size:14px;'; bdTitle.textContent = 'Breakdown';
                gscBody.appendChild(bdTitle);
                var dimBar = document.createElement('div'); dimBar.style.cssText = 'display:flex;gap:8px;margin-bottom:12px;';
                ['device', 'country', 'searchType'].forEach(function(dim) {
                    var btn = document.createElement('button');
                    btn.className = 'btn btn-sm ' + (dim === _gscBreakdownDim ? 'btn-primary' : 'btn-ghost');
                    btn.textContent = dim === 'searchType' ? 'Search Type' : dim.charAt(0).toUpperCase() + dim.slice(1);
                    btn.onclick = function() { _gscBreakdownDim = dim; loadGscData(); };
                    dimBar.appendChild(btn);
                });
                gscBody.appendChild(dimBar);

                var bdR = results[2];
                var bdItems = (bdR && bdR.ok && bdR.data) ? (bdR.data.items || bdR.data) : [];
                if (!Array.isArray(bdItems)) bdItems = [];
                var bdRows = bdItems.map(function(b) {
                    var key = b.value || b[_gscBreakdownDim] || b.keys || Object.values(b)[0] || '—';
                    return [key, googleFmt(b.clicks), googleFmt(b.impressions), googlePct(b.ctr)];
                });
                if (bdRows.length > 0) {
                    var dimLabel = _gscBreakdownDim === 'searchType' ? 'Search Type' : _gscBreakdownDim.charAt(0).toUpperCase() + _gscBreakdownDim.slice(1);
                    gscBody.appendChild(googleTable([dimLabel, 'Clicks', 'Impressions', 'CTR'], bdRows));
                } else {
                    var noBreakdown = document.createElement('p'); noBreakdown.style.cssText = 'color:var(--text-muted);font-size:13px;'; noBreakdown.textContent = 'No breakdown data available.';
                    gscBody.appendChild(noBreakdown);
                }

            } catch(e) {
                gscBody.replaceChildren();
                var errEl = document.createElement('p'); errEl.style.cssText = 'color:#ef4444;'; errEl.textContent = 'Error: ' + e.message;
                gscBody.appendChild(errEl);
            }
        }

        refreshBtn.onclick = loadGscData;
        await loadGscData();

    } catch(e) {
        var errEl = document.createElement('p'); errEl.style.cssText = 'color:#ef4444;';
        errEl.textContent = 'Error: ' + e.message;
        el.appendChild(errEl);
    }
}

// ── View: Google Ads ───────────────────────────────────────────────
async function load_seo_ads() {
    var main = document.getElementById('adminMain');
    var el = document.createElement('div');
    var h = document.createElement('h2'); h.textContent = 'Google Ads'; el.appendChild(h);
    main.replaceChildren(el);

    try {
        var cfg = await fetch('/api/modules/seo/google/config').then(function(r) { return r.json(); });
        var data = (cfg.ok && cfg.data) ? cfg.data : {};

        if (!data.authenticated) {
            el.appendChild(googleNotConnected(
                'Connect your Google account in Settings to access Google Ads.',
                'Go to Settings', 'seo-google-settings'
            ));
            return;
        }

        if (!data.ads_customer_id) {
            // Try to load customers for selection
            var selTitle = document.createElement('h3');
            selTitle.style.cssText = 'margin-bottom:12px;font-size:15px;';
            selTitle.textContent = 'Select Ads Account';
            el.appendChild(selTitle);

            var selBody = document.createElement('div');
            el.appendChild(selBody);

            try {
                var ar = await fetch('/api/modules/seo/google/ads/customers').then(function(r) { return r.json(); });
                if (ar.ok && ar.data && ar.data.optional) {
                    var infoCard = googleCard(null);
                    var infoIcon = document.createElement('div'); infoIcon.textContent = 'ℹ️'; infoIcon.style.cssText = 'font-size:24px;margin-bottom:8px;';
                    infoCard.appendChild(infoIcon);
                    var infoMsg = document.createElement('p'); infoMsg.style.cssText = 'color:var(--text-muted);font-size:14px;';
                    infoMsg.textContent = ar.data.reason || 'Google Ads is not available for this account.';
                    infoCard.appendChild(infoMsg);
                    selBody.appendChild(infoCard);
                } else {
                    var custs = (ar.ok && ar.data && Array.isArray(ar.data.customers)) ? ar.data.customers : [];
                    if (custs.length === 0) {
                        var none = document.createElement('p'); none.style.cssText = 'color:var(--text-muted);font-size:13px;';
                        none.textContent = 'No Google Ads customers found on this account.';
                        selBody.appendChild(none);
                    } else {
                        custs.forEach(function(c) {
                            var card = document.createElement('div');
                            card.style.cssText = 'padding:14px 16px;background:var(--surface);border:1px solid var(--border);border-radius:8px;margin-bottom:8px;display:flex;align-items:center;justify-content:space-between;';
                            var info = document.createElement('div');
                            var name = document.createElement('div'); name.style.cssText = 'font-weight:500;font-size:14px;'; name.textContent = c.display_name || c.customer_id || '—';
                            var id = document.createElement('div'); id.style.cssText = 'font-size:12px;color:var(--text-muted);'; id.textContent = 'ID: ' + (c.customer_id || '—');
                            info.appendChild(name); info.appendChild(id);
                            card.appendChild(info);
                            var selBtn = document.createElement('button');
                            selBtn.className = 'btn btn-primary btn-sm';
                            selBtn.textContent = 'Select';
                            selBtn.onclick = (function(cust) {
                                return async function() {
                                    try {
                                        var r = await fetch('/api/modules/seo/google/ads/select', {
                                            method: 'POST', headers: {'Content-Type':'application/json'},
                                            body: JSON.stringify({ customer_id: cust.customer_id || '', customer_display_name: cust.display_name || '' })
                                        }).then(function(r) { return r.json(); });
                                        if (r.ok) { toast('Ads customer saved'); load_seo_ads(); }
                                        else { toast(r.message || 'Error'); }
                                    } catch(e) { toast('Error: ' + e.message); }
                                };
                            })(c);
                            card.appendChild(selBtn);
                            selBody.appendChild(card);
                        });
                    }
                }
            } catch(e) {
                var errEl = document.createElement('p'); errEl.style.cssText = 'color:#ef4444;'; errEl.textContent = 'Error loading customers: ' + e.message;
                selBody.appendChild(errEl);
            }
            return;
        }

        // Customer connected
        var custCard = googleCard(null);
        var custTitle = document.createElement('h3'); custTitle.style.cssText = 'margin-bottom:12px;font-size:15px;'; custTitle.textContent = 'Connected Account';
        custCard.appendChild(custTitle);
        custCard.appendChild(googleRow('Customer', data.ads_customer_display_name || data.ads_customer_id));
        custCard.appendChild(googleRow('Customer ID', data.ads_customer_id));
        var badge = googleBadge('Active', true);
        badge.style.cssText += 'display:inline-block;margin-top:8px;';
        custCard.appendChild(badge);

        var changeBtn = document.createElement('button');
        changeBtn.className = 'btn btn-ghost btn-sm';
        changeBtn.style.cssText = 'margin-top:12px;';
        changeBtn.textContent = 'Change Account';
        changeBtn.onclick = function() { navigateTo('seo-google-settings'); };
        custCard.appendChild(changeBtn);
        el.appendChild(custCard);

        var comingSoon = googleCard(null);
        var csMsg = document.createElement('p');
        csMsg.style.cssText = 'color:var(--text-muted);font-size:14px;text-align:center;padding:24px 0;';
        csMsg.textContent = 'Full Google Ads reporting dashboard coming soon. Campaign performance, spend, conversions, and keyword data will appear here.';
        comingSoon.appendChild(csMsg);
        el.appendChild(comingSoon);

    } catch(e) {
        var errEl = document.createElement('p'); errEl.style.cssText = 'color:#ef4444;';
        errEl.textContent = 'Error: ' + e.message;
        el.appendChild(errEl);
    }
}

// ── Drill-Down Modal Infrastructure ─────────────────────────────────

function openDrillDownModal(width, title, contentBuilder) {
    var overlay = document.createElement('div');
    overlay.style.cssText = 'position:fixed;top:0;left:0;right:0;bottom:0;z-index:9999;background:rgba(0,0,0,0.6);display:flex;align-items:center;justify-content:center;opacity:0;transition:opacity 0.2s;';
    var modal = document.createElement('div');
    modal.style.cssText = 'background:var(--bg);border:1px solid var(--border);border-radius:12px;width:96vw;max-width:' + width + 'px;max-height:90vh;overflow-y:auto;display:flex;flex-direction:column;';

    // Header
    var header = document.createElement('div');
    header.style.cssText = 'display:flex;align-items:center;justify-content:space-between;padding:16px 20px;border-bottom:1px solid var(--border);flex-shrink:0;';
    var titleEl = document.createElement('h3');
    titleEl.style.cssText = 'font-size:16px;margin:0;white-space:nowrap;overflow:hidden;text-overflow:ellipsis;max-width:calc(100% - 40px);';
    titleEl.textContent = title;
    header.appendChild(titleEl);
    var closeBtn = document.createElement('button');
    closeBtn.style.cssText = 'background:none;border:none;color:var(--text-muted);font-size:20px;cursor:pointer;padding:0 4px;line-height:1;';
    closeBtn.textContent = 'X';
    closeBtn.onclick = closeModal;
    header.appendChild(closeBtn);
    modal.appendChild(header);

    // Body
    var body = document.createElement('div');
    body.style.cssText = 'padding:20px;overflow-y:auto;flex:1;';
    modal.appendChild(body);
    overlay.appendChild(modal);
    document.body.appendChild(overlay);

    function closeModal() {
        overlay.style.opacity = '0';
        setTimeout(function() { if (overlay.parentNode) overlay.parentNode.removeChild(overlay); }, 200);
    }
    overlay.onclick = function(e) { if (e.target === overlay) closeModal(); };
    function escHandler(e) { if (e.key === 'Escape') { closeModal(); document.removeEventListener('keydown', escHandler); } }
    document.addEventListener('keydown', escHandler);

    // Animate in
    requestAnimationFrame(function() { overlay.style.opacity = '1'; });

    // Build content
    try { contentBuilder(body, closeModal); } catch(e) {
        var err = document.createElement('p'); err.style.cssText = 'color:#ef4444;'; err.textContent = 'Error: ' + e.message; body.appendChild(err);
    }
    return { overlay: overlay, modal: modal, body: body, close: closeModal };
}

// ── Tab Infrastructure ──────────────────────────────────────────────

function renderTabs(container, tabs, defaultTab) {
    container.replaceChildren();
    var tabBar = document.createElement('div');
    tabBar.style.cssText = 'display:flex;gap:4px;border-bottom:1px solid var(--border);margin-bottom:16px;';
    var tabBody = document.createElement('div');
    var activeId = defaultTab || (tabs[0] && tabs[0].id);

    function switchTab(id) {
        activeId = id;
        // Update buttons
        var btns = tabBar.children;
        for (var i = 0; i < btns.length; i++) {
            btns[i].style.borderBottom = (tabs[i].id === id) ? '2px solid var(--accent)' : '2px solid transparent';
            btns[i].style.color = (tabs[i].id === id) ? 'var(--text)' : 'var(--text-muted)';
        }
        // Render body
        tabBody.replaceChildren();
        for (var j = 0; j < tabs.length; j++) {
            if (tabs[j].id === id) {
                try { tabs[j].render(tabBody); } catch(e) {
                    var err = document.createElement('p'); err.style.cssText = 'color:#ef4444;'; err.textContent = 'Error: ' + e.message; tabBody.appendChild(err);
                }
                break;
            }
        }
    }

    tabs.forEach(function(tab) {
        var btn = document.createElement('button');
        btn.style.cssText = 'background:none;border:none;padding:8px 16px;font-size:13px;cursor:pointer;border-bottom:2px solid transparent;transition:color 0.15s;';
        btn.textContent = tab.label;
        btn.onclick = function() { switchTab(tab.id); };
        tabBar.appendChild(btn);
    });

    container.appendChild(tabBar);
    container.appendChild(tabBody);
    switchTab(activeId);
}

// ── Clickable Page Table ────────────────────────────────────────────

function googleClickablePageTable(pData, dateRange) {
    var _pgTblSort = 1, _pgTblAsc = false;
    var tableId = 'gsc-page-tbl-' + Math.random().toString(36).slice(2,8);
    function render() {
        var sorted = pData.slice().sort(function(a, b) {
            var fields = ['_page', 'clicks', 'impressions', 'ctr', 'position'];
            if (_pgTblSort === 0) {
                var an = a.page || a.keys || '';
                var bn = b.page || b.keys || '';
                return _pgTblAsc ? an.localeCompare(bn) : bn.localeCompare(an);
            }
            var f = fields[_pgTblSort];
            var av = parseFloat(a[f] || 0);
            var bv = parseFloat(b[f] || 0);
            return _pgTblAsc ? av - bv : bv - av;
        });
        var wrap = document.createElement('div');
        wrap.style.cssText = 'overflow-x:auto;';
        wrap.id = tableId;
        var tbl = document.createElement('table');
        tbl.className = 'content-table';
        var thead = document.createElement('thead');
        var hrow = document.createElement('tr');
        ['Page URL', 'Clicks', 'Impressions', 'CTR', 'Position'].forEach(function(h, i) {
            var th = document.createElement('th');
            th.style.cssText = 'cursor:pointer;user-select:none;';
            th.textContent = h + (_pgTblSort === i ? (_pgTblAsc ? ' ▲' : ' ▼') : '');
            th.onclick = function() {
                if (_pgTblSort === i) _pgTblAsc = !_pgTblAsc; else { _pgTblSort = i; _pgTblAsc = false; }
                var old = document.getElementById(tableId);
                if (old) old.replaceWith(render());
            };
            hrow.appendChild(th);
        });
        thead.appendChild(hrow);
        tbl.appendChild(thead);
        var tbody = document.createElement('tbody');
        sorted.forEach(function(p) {
            var tr = document.createElement('tr');
            var pageUrl = p.page || p.keys || '';
            var vals = [pageUrl || '---', googleFmt(p.clicks), googleFmt(p.impressions), googlePct(p.ctr), p.position != null ? parseFloat(p.position).toFixed(1) : '---'];
            vals.forEach(function(val, ci) {
                var td = document.createElement('td');
                if (ci === 0 && pageUrl) {
                    var link = document.createElement('span');
                    link.textContent = val;
                    link.style.cssText = 'cursor:pointer;color:var(--accent);word-break:break-all;';
                    link.onmouseenter = function() { link.style.textDecoration = 'underline'; };
                    link.onmouseleave = function() { link.style.textDecoration = 'none'; };
                    link.onclick = function(ev) { ev.stopPropagation(); openPageDetailModal(pageUrl, dateRange); };
                    td.appendChild(link);
                } else { td.textContent = val; }
                tr.appendChild(td);
            });
            tbody.appendChild(tr);
        });
        tbl.appendChild(tbody);
        wrap.appendChild(tbl);
        return wrap;
    }
    return render();
}

// ── KPI Card Row Helper ─────────────────────────────────────────────

function drilldownKpiCards(items) {
    var row = document.createElement('div');
    row.style.cssText = 'display:grid;grid-template-columns:repeat(auto-fit,minmax(140px,1fr));gap:12px;margin-bottom:20px;';
    items.forEach(function(item) {
        var card = document.createElement('div');
        card.style.cssText = 'padding:14px 16px;background:var(--surface);border:1px solid var(--border);border-radius:8px;';
        var lbl = document.createElement('div');
        lbl.style.cssText = 'font-size:11px;color:var(--text-muted);text-transform:uppercase;letter-spacing:0.5px;margin-bottom:6px;';
        lbl.textContent = item[0];
        var val = document.createElement('div');
        val.style.cssText = 'font-size:22px;font-weight:700;';
        val.textContent = item[1] != null ? item[1] : '---';
        card.appendChild(lbl);
        card.appendChild(val);
        if (item[2]) {
            var sub = document.createElement('div');
            sub.style.cssText = 'font-size:11px;color:var(--text-muted);margin-top:4px;';
            sub.textContent = item[2];
            card.appendChild(sub);
        }
        row.appendChild(card);
    });
    return row;
}

// ── Insight Cards Helper ────────────────────────────────────────────

function renderInsightCards(container, insights) {
    if (!insights || !Array.isArray(insights) || insights.length === 0) {
        var none = document.createElement('p');
        none.style.cssText = 'color:var(--text-muted);font-size:13px;';
        none.textContent = 'No insights available.';
        container.appendChild(none);
        return;
    }
    insights.forEach(function(ins) {
        var card = document.createElement('div');
        card.style.cssText = 'padding:14px 16px;background:var(--surface);border:1px solid var(--border);border-radius:8px;margin-bottom:10px;';
        var top = document.createElement('div');
        top.style.cssText = 'display:flex;align-items:center;gap:8px;margin-bottom:6px;';
        if (ins.severity) {
            var badge = document.createElement('span');
            var sevColors = { critical: '#ef4444', warning: '#f59e0b', info: '#3b82f6', success: '#22c55e' };
            var col = sevColors[ins.severity] || sevColors.info;
            badge.style.cssText = 'font-size:10px;padding:2px 8px;border-radius:10px;font-weight:600;text-transform:uppercase;background:' + col + '22;color:' + col + ';';
            badge.textContent = ins.severity;
            top.appendChild(badge);
        }
        if (ins.title) {
            var t = document.createElement('span');
            t.style.cssText = 'font-weight:600;font-size:14px;';
            t.textContent = ins.title;
            top.appendChild(t);
        }
        card.appendChild(top);
        if (ins.message || ins.description) {
            var msg = document.createElement('p');
            msg.style.cssText = 'font-size:13px;color:var(--text-muted);margin:0;line-height:1.5;';
            msg.textContent = ins.message || ins.description;
            card.appendChild(msg);
        }
        if (ins.recommendation) {
            var rec = document.createElement('p');
            rec.style.cssText = 'font-size:12px;color:var(--accent);margin:6px 0 0;';
            rec.textContent = 'Tip: ' + ins.recommendation;
            card.appendChild(rec);
        }
        container.appendChild(card);
    });
}

// ── Loading Spinner Helper ──────────────────────────────────────────

function drilldownLoading(container, msg) {
    var el = document.createElement('p');
    el.style.cssText = 'color:var(--text-muted);font-size:13px;padding:20px 0;';
    el.textContent = msg || 'Loading...';
    container.appendChild(el);
    return el;
}

// ── Expected CTR by Position ────────────────────────────────────────

function expectedCtr(pos) {
    var rates = [0.396, 0.187, 0.108, 0.075, 0.053, 0.038, 0.028, 0.021, 0.016, 0.013];
    var idx = Math.round(pos) - 1;
    if (idx < 0) idx = 0;
    if (idx >= rates.length) return 0.005;
    return rates[idx];
}

function renderMiniBarChart(container, title, points, valueKey, colorVar) {
    if (!Array.isArray(points) || points.length === 0) return;
    var chartTitle = document.createElement('h4');
    chartTitle.style.cssText = 'margin:0 0 10px;font-size:13px;';
    chartTitle.textContent = title;
    container.appendChild(chartTitle);

    var wrap = document.createElement('div');
    wrap.style.cssText = 'display:flex;align-items:flex-end;gap:3px;height:90px;background:var(--surface);border:1px solid var(--border);border-radius:8px;padding:8px;overflow-x:auto;';
    var vals = points.map(function(point) { return parseFloat(point[valueKey] || 0) || 0; });
    var maxVal = Math.max.apply(null, vals) || 1;

    points.forEach(function(point, idx) {
        var bar = document.createElement('div');
        var pct = Math.max(4, Math.round((vals[idx] / maxVal) * 72));
        bar.style.cssText = 'flex:1;min-width:8px;border-radius:3px 3px 0 0;background:' + (colorVar || 'var(--accent)') + ';';
        bar.style.height = pct + 'px';
        bar.title = (point.date || '') + ': ' + vals[idx];
        wrap.appendChild(bar);
    });
    container.appendChild(wrap);
}

function renderTrendTable(container, title, points) {
    if (!Array.isArray(points) || points.length === 0) return;
    var heading = document.createElement('h4');
    heading.style.cssText = 'margin:16px 0 8px;font-size:13px;';
    heading.textContent = title;
    container.appendChild(heading);

    var rows = points.map(function(point) {
        return [
            point.date || '—',
            googleFmt(point.clicks || 0),
            googleFmt(point.impressions || 0),
            googlePct(point.ctr || 0),
            point.position != null ? parseFloat(point.position).toFixed(1) : '---'
        ];
    });
    container.appendChild(googleTable(['Date', 'Clicks', 'Impressions', 'CTR', 'Position'], rows));
}

// ── Query Detail Modal ──────────────────────────────────────────────

function openQueryDetailModal(query, dateRange) {
    var dr = dateRange || {};
    var start = dr.start || googleDateStr(28);
    var end = dr.end || googleDateStr(0);

    openDrillDownModal(1050, 'Query: ' + query, function(body) {
        var loader = drilldownLoading(body, 'Fetching query data...');

        fetch('/api/modules/seo/google/gsc/queries?query_contains=' + encodeURIComponent(query) + '&start_date=' + start + '&end_date=' + end + '&row_limit=1')
            .then(function(r) { return r.json(); })
            .then(function(res) {
                body.removeChild(loader);
                var qArr = (res.ok && res.data) ? (res.data.queries || res.data) : [];
                if (!Array.isArray(qArr)) qArr = [res.data];
                var qData = qArr[0] || {};

                var clicks = parseFloat(qData.clicks || 0);
                var impr = parseFloat(qData.impressions || 0);
                var ctr = parseFloat(qData.ctr || 0);
                var pos = parseFloat(qData.position || 0);
                // Normalize CTR (some APIs return 0-1, some 0-100)
                var ctrPct = ctr > 1 ? ctr : ctr * 100;
                var expCtr = expectedCtr(pos) * 100;

                renderTabs(body, [
                    {
                        id: 'overview',
                        label: 'Overview',
                        render: function(tb) {
                            tb.appendChild(drilldownKpiCards([
                                ['Clicks', googleFmt(clicks)],
                                ['Impressions', googleFmt(impr)],
                                ['CTR', ctrPct.toFixed(1) + '%'],
                                ['Avg Position', pos > 0 ? pos.toFixed(1) : '---']
                            ]));
                            // CTR analysis
                            if (pos > 0) {
                                var analysis = document.createElement('div');
                                analysis.style.cssText = 'padding:14px 16px;background:var(--surface);border:1px solid var(--border);border-radius:8px;margin-top:8px;';
                                var aText = document.createElement('p');
                                aText.style.cssText = 'font-size:13px;line-height:1.6;margin:0;';
                                var verdict = ctrPct >= expCtr ? 'above' : 'below';
                                aText.textContent = 'Your CTR is ' + ctrPct.toFixed(1) + '% — expected for position ' + Math.round(pos) + ' is ~' + expCtr.toFixed(1) + '%. You are ' + verdict + ' the benchmark.';
                                analysis.appendChild(aText);
                                if (ctrPct < expCtr) {
                                    var tip = document.createElement('p');
                                    tip.style.cssText = 'font-size:12px;color:var(--accent);margin:8px 0 0;';
                                    tip.textContent = 'Tip: Improve title tag and meta description to increase CTR.';
                                    analysis.appendChild(tip);
                                }
                                tb.appendChild(analysis);
                            }
                        }
                    },
                    {
                        id: 'pages',
                        label: 'Ranking Pages',
                        render: function(tb) {
                            var pLoader = drilldownLoading(tb, 'Loading pages...');
                            fetch('/api/modules/seo/google/gsc/query-pages?start_date=' + start + '&end_date=' + end + '&query=' + encodeURIComponent(query))
                                .then(function(r) { return r.json(); })
                                .then(function(pr) {
                                    tb.removeChild(pLoader);
                                    var pArr = (pr.ok && pr.data) ? (pr.data.pages || pr.data) : [];
                                    if (!Array.isArray(pArr)) pArr = [];
                                    if (pArr.length === 0) {
                                        var none = document.createElement('p');
                                        none.style.cssText = 'color:var(--text-muted);font-size:13px;';
                                        none.textContent = 'No page data found for this query.';
                                        tb.appendChild(none);
                                        return;
                                    }
                                    tb.appendChild(googleClickablePageTable(pArr, { start: start, end: end }));
                                })
                                .catch(function(e) {
                                    tb.removeChild(pLoader);
                                    var err = document.createElement('p'); err.style.cssText = 'color:#ef4444;'; err.textContent = 'Error: ' + e.message; tb.appendChild(err);
                                });
                        }
                    },
                    {
                        id: 'trend',
                        label: 'Trend',
                        render: function(tb) {
                            var tLoader = drilldownLoading(tb, 'Loading trend data...');
                            fetch('/api/modules/seo/google/gsc/query-timeseries?start_date=' + start + '&end_date=' + end + '&query=' + encodeURIComponent(query))
                                .then(function(r) { return r.json(); })
                                .then(function(tr) {
                                    tb.removeChild(tLoader);
                                    var pts = (tr.ok && tr.data) ? (tr.data.data || tr.data) : [];
                                    if (!Array.isArray(pts)) pts = [];
                                    if (pts.length === 0) {
                                        var none = document.createElement('p');
                                        none.style.cssText = 'color:var(--text-muted);font-size:13px;';
                                        none.textContent = 'No daily trend data found for this query.';
                                        tb.appendChild(none);
                                        return;
                                    }
                                    var latest = pts[pts.length - 1] || {};
                                    tb.appendChild(drilldownKpiCards([
                                        ['Data Points', googleFmt(pts.length)],
                                        ['Latest Clicks', googleFmt(latest.clicks || 0)],
                                        ['Latest Impressions', googleFmt(latest.impressions || 0)],
                                        ['Latest Position', latest.position != null ? parseFloat(latest.position).toFixed(1) : '---']
                                    ]));
                                    renderMiniBarChart(tb, 'Daily Impressions', pts, 'impressions', 'var(--accent)');
                                    renderTrendTable(tb, 'Recent Daily Metrics', pts.slice(-14).reverse());
                                })
                                .catch(function(e) {
                                    tb.removeChild(tLoader);
                                    var err = document.createElement('p');
                                    err.style.cssText = 'color:#ef4444;';
                                    err.textContent = 'Error: ' + e.message;
                                    tb.appendChild(err);
                                });
                        }
                    },
                    {
                        id: 'insights',
                        label: 'Insights',
                        render: function(tb) {
                            var iLoader = drilldownLoading(tb, 'Generating insights...');
                            fetch('/api/modules/seo/insights', {
                                method: 'POST',
                                headers: { 'Content-Type': 'application/json' },
                                body: JSON.stringify({ queries: [{ query: query, clicks: clicks, impressions: impr, ctr: ctr, position: pos }] })
                            }).then(function(r) { return r.json(); })
                            .then(function(ir) {
                                tb.removeChild(iLoader);
                                var insights = (ir.ok && ir.data) ? (ir.data.insights || ir.data) : [];
                                if (!Array.isArray(insights)) insights = [];
                                if (insights.length === 0) {
                                    // Provide default analysis
                                    var defaultInsights = [];
                                    if (pos > 0 && pos <= 3 && ctrPct < expCtr) {
                                        defaultInsights.push({ severity: 'warning', title: 'Low CTR for Top Position', message: 'This query ranks in position ' + Math.round(pos) + ' but CTR is below expected. Consider improving the title tag and meta description.', recommendation: 'Use the query naturally in your title tag. Add a compelling call-to-action in the meta description.' });
                                    }
                                    if (pos > 10) {
                                        defaultInsights.push({ severity: 'info', title: 'Page 2+ Ranking', message: 'This query ranks on page 2 or lower (position ' + Math.round(pos) + '). Content improvements could push it to page 1.', recommendation: 'Review content depth, add relevant headers, and build topical authority.' });
                                    }
                                    if (impr > 1000 && clicks < 10) {
                                        defaultInsights.push({ severity: 'warning', title: 'High Impressions, Low Clicks', message: 'This query gets ' + googleFmt(impr) + ' impressions but only ' + googleFmt(clicks) + ' clicks. The snippet may not be compelling.', recommendation: 'Rewrite meta title and description to be more enticing.' });
                                    }
                                    if (defaultInsights.length === 0) {
                                        defaultInsights.push({ severity: 'success', title: 'Query Performing Well', message: 'No major issues detected for this query.' });
                                    }
                                    renderInsightCards(tb, defaultInsights);
                                } else {
                                    renderInsightCards(tb, insights);
                                }
                            })
                            .catch(function() {
                                tb.removeChild(iLoader);
                                // Fallback: local analysis
                                var fallback = [];
                                if (ctrPct > 0 && ctrPct < expCtr) {
                                    fallback.push({ severity: 'warning', title: 'Below-Average CTR', message: 'CTR of ' + ctrPct.toFixed(1) + '% is below the ~' + expCtr.toFixed(1) + '% expected for position ' + Math.round(pos) + '.', recommendation: 'Improve title tag and meta description.' });
                                } else {
                                    fallback.push({ severity: 'info', title: 'Query Overview', message: 'Clicks: ' + googleFmt(clicks) + ', Impressions: ' + googleFmt(impr) + ', Position: ' + (pos > 0 ? pos.toFixed(1) : 'N/A') });
                                }
                                renderInsightCards(tb, fallback);
                            });
                        }
                    }
                ], 'overview');
            })
            .catch(function(e) {
                body.removeChild(loader);
                var err = document.createElement('p'); err.style.cssText = 'color:#ef4444;'; err.textContent = 'Failed to load query data: ' + e.message; body.appendChild(err);
            });
    });
}

// ── Page Detail Modal ───────────────────────────────────────────────

function openPageDetailModal(pageUrl, dateRange) {
    var dr = dateRange || {};
    var start = dr.start || googleDateStr(28);
    var end = dr.end || googleDateStr(0);
    var displayUrl = pageUrl.length > 70 ? pageUrl.substring(0, 67) + '...' : pageUrl;

    openDrillDownModal(1080, 'Page: ' + displayUrl, function(body) {
        var loader = drilldownLoading(body, 'Fetching page data...');

        var qs = '?start_date=' + start + '&end_date=' + end;
        Promise.all([
            fetch('/api/modules/seo/google/gsc/pages' + qs + '&page_contains=' + encodeURIComponent(pageUrl) + '&row_limit=1').then(function(r) { return r.json(); }).catch(function() { return null; }),
            fetch('/api/modules/seo/google/ga4/pages' + qs + '&page_path=' + encodeURIComponent(pageUrl) + '&row_limit=1').then(function(r) { return r.json(); }).catch(function() { return null; }),
            fetch('/api/modules/seo/google/gsc/queries' + qs + '&page_contains=' + encodeURIComponent(pageUrl) + '&row_limit=20').then(function(r) { return r.json(); }).catch(function() { return null; })
        ]).then(function(results) {
            body.removeChild(loader);

            var gscR = results[0];
            var gscArr = (gscR && gscR.ok && gscR.data) ? (gscR.data.pages || gscR.data) : [];
            if (!Array.isArray(gscArr)) gscArr = [gscR && gscR.data];
            var gscData = gscArr[0] || {};

            var ga4R = results[1];
            var ga4Arr = (ga4R && ga4R.ok && ga4R.data) ? (ga4R.data.pages || ga4R.data) : [];
            if (!Array.isArray(ga4Arr)) ga4Arr = [ga4R && ga4R.data];
            var ga4Data = ga4Arr[0] || {};

            var qR = results[2];
            var qArr = (qR && qR.ok && qR.data) ? (qR.data.queries || qR.data) : [];
            if (!Array.isArray(qArr)) qArr = [];

            var gscClicks = parseFloat(gscData.clicks || 0);
            var gscImpr = parseFloat(gscData.impressions || 0);
            var gscCtr = parseFloat(gscData.ctr || 0);
            var gscPos = parseFloat(gscData.position || 0);
            var gscCtrPct = gscCtr > 1 ? gscCtr : gscCtr * 100;

            var ga4Sessions = ga4Data.sessions || ga4Data.session_count || 0;
            var ga4Views = ga4Data.pageviews || ga4Data.screen_page_views || ga4Data.views || 0;
            var ga4Bounce = ga4Data.bounce_rate || 0;

            renderTabs(body, [
                {
                    id: 'overview',
                    label: 'Overview',
                    render: function(tb) {
                        var gscLabel = document.createElement('h4');
                        gscLabel.style.cssText = 'font-size:12px;color:var(--text-muted);text-transform:uppercase;letter-spacing:0.5px;margin-bottom:10px;';
                        gscLabel.textContent = 'Search Console';
                        tb.appendChild(gscLabel);
                        tb.appendChild(drilldownKpiCards([
                            ['Clicks', googleFmt(gscClicks)],
                            ['Impressions', googleFmt(gscImpr)],
                            ['CTR', gscCtrPct.toFixed(1) + '%'],
                            ['Avg Position', gscPos > 0 ? gscPos.toFixed(1) : '---']
                        ]));
                        var ga4Label = document.createElement('h4');
                        ga4Label.style.cssText = 'font-size:12px;color:var(--text-muted);text-transform:uppercase;letter-spacing:0.5px;margin:16px 0 10px;';
                        ga4Label.textContent = 'GA4 Analytics';
                        tb.appendChild(ga4Label);
                        tb.appendChild(drilldownKpiCards([
                            ['Sessions', googleFmt(ga4Sessions)],
                            ['Pageviews', googleFmt(ga4Views)],
                            ['Bounce Rate', googlePct(ga4Bounce)]
                        ]));
                    }
                },
                {
                    id: 'trend',
                    label: 'Trend',
                    render: function(tb) {
                        var trendLoader = drilldownLoading(tb, 'Loading page trend data...');
                        fetch('/api/modules/seo/google/gsc/page-timeseries?start_date=' + start + '&end_date=' + end + '&page=' + encodeURIComponent(pageUrl))
                            .then(function(r) { return r.json(); })
                            .then(function(tr) {
                                tb.removeChild(trendLoader);
                                var pts = (tr.ok && tr.data) ? (tr.data.data || tr.data) : [];
                                if (!Array.isArray(pts)) pts = [];
                                if (pts.length === 0) {
                                    var none = document.createElement('p');
                                    none.style.cssText = 'color:var(--text-muted);font-size:13px;';
                                    none.textContent = 'No daily trend data found for this page.';
                                    tb.appendChild(none);
                                    return;
                                }
                                var latest = pts[pts.length - 1] || {};
                                tb.appendChild(drilldownKpiCards([
                                    ['Data Points', googleFmt(pts.length)],
                                    ['Latest Clicks', googleFmt(latest.clicks || 0)],
                                    ['Latest Impressions', googleFmt(latest.impressions || 0)],
                                    ['Latest Position', latest.position != null ? parseFloat(latest.position).toFixed(1) : '---']
                                ]));
                                renderMiniBarChart(tb, 'Daily Impressions', pts, 'impressions', 'var(--accent)');
                                renderTrendTable(tb, 'Recent Daily Metrics', pts.slice(-14).reverse());
                            })
                            .catch(function(e) {
                                tb.removeChild(trendLoader);
                                var err = document.createElement('p');
                                err.style.cssText = 'color:#ef4444;';
                                err.textContent = 'Error: ' + e.message;
                                tb.appendChild(err);
                            });
                    }
                },
                {
                    id: 'seo-health',
                    label: 'SEO Health',
                    render: function(tb) {
                        // Try to find a contentId from the page URL
                        var pathParts = pageUrl.replace(/^https?:\/\/[^\/]+/, '').replace(/^\/+/, '').replace(/\/+$/, '');
                        var slug = pathParts.split('/').pop() || pathParts;
                        var healthLoader = drilldownLoading(tb, 'Checking SEO health...');
                        fetch('/api/modules/seo/score/' + encodeURIComponent(slug))
                            .then(function(r) { return r.json(); })
                            .then(function(sr) {
                                tb.removeChild(healthLoader);
                                if (sr.ok && sr.data && sr.data.score != null) {
                                    var score = parseInt(sr.data.score) || 0;
                                    // Score ring using conic-gradient
                                    var ringWrap = document.createElement('div');
                                    ringWrap.style.cssText = 'display:flex;align-items:center;gap:24px;margin-bottom:20px;';
                                    var ring = document.createElement('div');
                                    var scoreColor = score >= 80 ? '#22c55e' : score >= 50 ? '#f59e0b' : '#ef4444';
                                    ring.style.cssText = 'width:80px;height:80px;border-radius:50%;display:flex;align-items:center;justify-content:center;background:conic-gradient(' + scoreColor + ' ' + (score * 3.6) + 'deg, var(--border) 0deg);flex-shrink:0;';
                                    var inner = document.createElement('div');
                                    inner.style.cssText = 'width:60px;height:60px;border-radius:50%;background:var(--bg);display:flex;align-items:center;justify-content:center;font-size:20px;font-weight:700;';
                                    inner.textContent = score;
                                    ring.appendChild(inner);
                                    ringWrap.appendChild(ring);
                                    var scoreLabel = document.createElement('div');
                                    var gradeText = score >= 80 ? 'Good' : score >= 50 ? 'Needs Work' : 'Poor';
                                    var slabel = document.createElement('div');
                                    slabel.style.cssText = 'font-size:18px;font-weight:600;';
                                    slabel.textContent = 'SEO Score: ' + gradeText;
                                    scoreLabel.appendChild(slabel);
                                    var sdetail = document.createElement('div');
                                    sdetail.style.cssText = 'font-size:13px;color:var(--text-muted);margin-top:4px;';
                                    sdetail.textContent = score + ' out of 100';
                                    scoreLabel.appendChild(sdetail);
                                    ringWrap.appendChild(scoreLabel);
                                    tb.appendChild(ringWrap);

                                    // Issues
                                    var issues = sr.data.issues || [];
                                    if (issues.length > 0) {
                                        var issTitle = document.createElement('h4');
                                        issTitle.style.cssText = 'font-size:14px;margin-bottom:10px;';
                                        issTitle.textContent = 'Issues (' + issues.length + ')';
                                        tb.appendChild(issTitle);
                                        issues.forEach(function(iss) {
                                            var item = document.createElement('div');
                                            item.style.cssText = 'padding:10px 14px;background:var(--surface);border:1px solid var(--border);border-radius:6px;margin-bottom:6px;font-size:13px;';
                                            item.textContent = (iss.message || iss.description || iss);
                                            tb.appendChild(item);
                                        });
                                    }
                                } else {
                                    var noScore = document.createElement('div');
                                    noScore.style.cssText = 'padding:20px;background:var(--surface);border:1px solid var(--border);border-radius:8px;';
                                    var nsMsg = document.createElement('p');
                                    nsMsg.style.cssText = 'color:var(--text-muted);font-size:13px;margin:0;';
                                    nsMsg.textContent = 'No SEO score available for this page. Scores are generated when pages are analyzed by the SEO module.';
                                    noScore.appendChild(nsMsg);
                                    tb.appendChild(noScore);
                                }
                            })
                            .catch(function() {
                                tb.removeChild(healthLoader);
                                var fallback = document.createElement('p');
                                fallback.style.cssText = 'color:var(--text-muted);font-size:13px;';
                                fallback.textContent = 'SEO health data not available for this page.';
                                tb.appendChild(fallback);
                            });
                    }
                },
                {
                    id: 'queries',
                    label: 'Queries (' + qArr.length + ')',
                    render: function(tb) {
                        if (qArr.length === 0) {
                            var none = document.createElement('p');
                            none.style.cssText = 'color:var(--text-muted);font-size:13px;';
                            none.textContent = 'No query data for this page.';
                            tb.appendChild(none);
                            return;
                        }
                        // Build clickable query table
                        var wrap = document.createElement('div');
                        wrap.style.cssText = 'overflow-x:auto;';
                        var tbl = document.createElement('table');
                        tbl.className = 'content-table';
                        var thead = document.createElement('thead');
                        var hrow = document.createElement('tr');
                        ['Query', 'Clicks', 'Impressions', 'CTR', 'Position'].forEach(function(h) {
                            var th = document.createElement('th'); th.textContent = h; hrow.appendChild(th);
                        });
                        thead.appendChild(hrow);
                        tbl.appendChild(thead);
                        var tbody = document.createElement('tbody');
                        qArr.forEach(function(q) {
                            var tr = document.createElement('tr');
                            var qName = q.query || q.keys || '';
                            var vals = [qName || '---', googleFmt(q.clicks), googleFmt(q.impressions), googlePct(q.ctr), q.position != null ? parseFloat(q.position).toFixed(1) : '---'];
                            vals.forEach(function(val, ci) {
                                var td = document.createElement('td');
                                if (ci === 0 && qName) {
                                    var link = document.createElement('span');
                                    link.textContent = val;
                                    link.style.cssText = 'cursor:pointer;color:var(--accent);';
                                    link.onmouseenter = function() { link.style.textDecoration = 'underline'; };
                                    link.onmouseleave = function() { link.style.textDecoration = 'none'; };
                                    link.onclick = function(ev) { ev.stopPropagation(); openQueryDetailModal(qName, { start: start, end: end }); };
                                    td.appendChild(link);
                                } else { td.textContent = val; }
                                tr.appendChild(td);
                            });
                            tbody.appendChild(tr);
                        });
                        tbl.appendChild(tbody);
                        wrap.appendChild(tbl);
                        tb.appendChild(wrap);
                    }
                },
                {
                    id: 'content',
                    label: 'Content',
                    render: function(tb) {
                        // SERP preview
                        var serpTitle = document.createElement('h4');
                        serpTitle.style.cssText = 'font-size:13px;color:var(--text-muted);text-transform:uppercase;letter-spacing:0.5px;margin-bottom:12px;';
                        serpTitle.textContent = 'SERP Preview';
                        tb.appendChild(serpTitle);

                        var serpCard = document.createElement('div');
                        serpCard.style.cssText = 'padding:16px 20px;background:var(--surface);border:1px solid var(--border);border-radius:8px;margin-bottom:20px;max-width:600px;';

                        var serpUrl = document.createElement('div');
                        serpUrl.style.cssText = 'font-size:12px;color:#22c55e;margin-bottom:4px;word-break:break-all;';
                        serpUrl.textContent = pageUrl;
                        serpCard.appendChild(serpUrl);

                        var serpH = document.createElement('div');
                        serpH.style.cssText = 'font-size:18px;color:#8ab4f8;margin-bottom:4px;line-height:1.3;';
                        // Extract page title from URL as placeholder
                        var urlSlug = pageUrl.replace(/^https?:\/\/[^\/]+\/?/, '').replace(/[-_]/g, ' ').replace(/\/$/, '') || 'Home';
                        serpH.textContent = urlSlug.split('/').pop().replace(/^\w/, function(c) { return c.toUpperCase(); }) || 'Page';
                        serpCard.appendChild(serpH);

                        var serpDesc = document.createElement('div');
                        serpDesc.style.cssText = 'font-size:13px;color:#bdc1c6;line-height:1.4;';
                        serpDesc.textContent = 'Meta description not available in current data. Visit the SEO module to edit page metadata.';
                        serpCard.appendChild(serpDesc);
                        tb.appendChild(serpCard);

                        // Metadata
                        var metaTitle = document.createElement('h4');
                        metaTitle.style.cssText = 'font-size:13px;color:var(--text-muted);text-transform:uppercase;letter-spacing:0.5px;margin-bottom:10px;';
                        metaTitle.textContent = 'Page Metadata';
                        tb.appendChild(metaTitle);

                        var metaCard = googleCard(null);
                        metaCard.appendChild(googleRow('Full URL', pageUrl));
                        metaCard.appendChild(googleRow('GSC Clicks', googleFmt(gscClicks)));
                        metaCard.appendChild(googleRow('GSC Impressions', googleFmt(gscImpr)));
                        metaCard.appendChild(googleRow('Avg Position', gscPos > 0 ? gscPos.toFixed(1) : '---'));
                        metaCard.appendChild(googleRow('GA4 Sessions', googleFmt(ga4Sessions)));
                        metaCard.appendChild(googleRow('GA4 Pageviews', googleFmt(ga4Views)));
                        metaCard.appendChild(googleRow('Ranking Queries', String(qArr.length)));
                        tb.appendChild(metaCard);
                    }
                },
                {
                    id: 'ai-insights',
                    label: 'AI Insights',
                    render: function(tb) {
                        var iLoader = drilldownLoading(tb, 'Generating AI insights...');
                        fetch('/api/modules/seo/insights', {
                            method: 'POST',
                            headers: { 'Content-Type': 'application/json' },
                            body: JSON.stringify({
                                pages: [{ url: pageUrl, clicks: gscClicks, impressions: gscImpr, ctr: gscCtr, position: gscPos, sessions: ga4Sessions, pageviews: ga4Views, bounce_rate: ga4Bounce }],
                                queries: qArr.slice(0, 10).map(function(q) { return { query: q.query, clicks: q.clicks, impressions: q.impressions, ctr: q.ctr, position: q.position }; })
                            })
                        }).then(function(r) { return r.json(); })
                        .then(function(ir) {
                            tb.removeChild(iLoader);
                            var insights = (ir.ok && ir.data) ? (ir.data.insights || ir.data) : [];
                            if (!Array.isArray(insights)) insights = [];
                            if (insights.length === 0) {
                                // Provide local fallback insights
                                var fallbackIns = [];
                                if (gscImpr > 500 && gscClicks < 5) {
                                    fallbackIns.push({ severity: 'warning', title: 'Low Click-Through', message: 'This page gets ' + googleFmt(gscImpr) + ' impressions but very few clicks. The search snippet may not be compelling.', recommendation: 'Rewrite the title tag and meta description to better match search intent.' });
                                }
                                if (gscPos > 0 && gscPos <= 5 && gscCtrPct < expectedCtr(gscPos) * 100) {
                                    fallbackIns.push({ severity: 'warning', title: 'Below-Average CTR', message: 'Ranking in position ' + gscPos.toFixed(1) + ' but CTR is only ' + gscCtrPct.toFixed(1) + '%.', recommendation: 'Add structured data (FAQ, HowTo) to win featured snippets.' });
                                }
                                if (parseFloat(ga4Bounce) > 0.7 || parseFloat(ga4Bounce) > 70) {
                                    fallbackIns.push({ severity: 'info', title: 'High Bounce Rate', message: 'Bounce rate indicates visitors may not be finding what they expect.', recommendation: 'Ensure content matches search intent. Add internal links and clear CTAs.' });
                                }
                                if (qArr.length >= 10) {
                                    fallbackIns.push({ severity: 'success', title: 'Strong Query Coverage', message: 'This page ranks for ' + qArr.length + '+ queries, indicating good topical relevance.' });
                                }
                                if (fallbackIns.length === 0) {
                                    fallbackIns.push({ severity: 'info', title: 'Page Analysis', message: 'Clicks: ' + googleFmt(gscClicks) + ', Position: ' + (gscPos > 0 ? gscPos.toFixed(1) : 'N/A') + ', Queries: ' + qArr.length, recommendation: 'Continue monitoring performance and optimizing content.' });
                                }
                                renderInsightCards(tb, fallbackIns);
                            } else {
                                renderInsightCards(tb, insights);
                            }
                        })
                        .catch(function() {
                            tb.removeChild(iLoader);
                            var fallback = [];
                            fallback.push({ severity: 'info', title: 'Page Performance Summary', message: 'GSC: ' + googleFmt(gscClicks) + ' clicks, ' + googleFmt(gscImpr) + ' impressions, position ' + (gscPos > 0 ? gscPos.toFixed(1) : 'N/A') + '. GA4: ' + googleFmt(ga4Sessions) + ' sessions.', recommendation: 'Use the SEO module to optimize title, meta, and content.' });
                            renderInsightCards(tb, fallback);
                        });
                    }
                }
            ], 'overview');
        }).catch(function(e) {
            body.removeChild(loader);
            var err = document.createElement('p'); err.style.cssText = 'color:#ef4444;'; err.textContent = 'Failed to load page data: ' + e.message; body.appendChild(err);
        });
    });
}
"##;
