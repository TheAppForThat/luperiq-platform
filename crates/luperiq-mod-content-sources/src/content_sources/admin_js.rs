
//! Admin JavaScript for the Content Library module.
//!
//! Provides:
//! - Dashboard view: topic cards with source counts and type icons
//! - Source detail view: per-topic source list with preview
//! - Upload flow: drag-and-drop file upload with parsed fact preview
//! - Scrape flow: URL-based content extraction via site-blueprint research
//! - Commission flow: AI Verified / Expert Reviewed tier selection
//! - Conflict display: inline conflict notification with resolution buttons
//!
//! Security: Uses DOM methods (createElement/textContent) exclusively. No innerHTML.

pub(crate) const CONTENT_SOURCES_ADMIN_JS: &str = r##"
// ── Content Library: Dashboard ────────────────────────────────────────

var _clSources = [];
var _clConflicts = [];
var _clPricing = {};

async function _clFetchSources() {
    try {
        var r = await fetch('/api/modules/content-sources/sources');
        var j = await r.json();
        if (j.ok && j.data) { _clSources = j.data; }
    } catch(e) { console.error('Failed to fetch sources:', e); }
}

async function _clFetchConflicts() {
    try {
        var r = await fetch('/api/modules/content-sources/conflicts');
        var j = await r.json();
        if (j.ok && j.data) { _clConflicts = j.data; }
    } catch(e) { console.error('Failed to fetch conflicts:', e); }
}

async function _clFetchPricing() {
    try {
        var r = await fetch('/api/modules/content-sources/pricing');
        var j = await r.json();
        if (j.ok && j.data) { _clPricing = j.data; }
    } catch(e) { console.error('Failed to fetch pricing:', e); }
}

function _clGroupByTopic(sources) {
    var groups = {};
    sources.forEach(function(s) {
        var key = s.industry_slug + '/' + s.topic_slug;
        if (!groups[key]) {
            groups[key] = { industry: s.industry_slug, topic: s.topic_slug, title: s.title, sources: [] };
        }
        groups[key].sources.push(s);
    });
    return Object.values(groups);
}

function _clSourceTypeLabel(t) {
    var map = {
        'luperiq_fact_sheet': 'LuperIQ',
        'customer_upload': 'Upload',
        'site_scrape': 'Scrape',
        'commissioned_ai_verified': 'AI Verified',
        'commissioned_expert_review': 'Expert'
    };
    return map[t] || t;
}

function _clRenderTopicCard(group) {
    var card = document.createElement('div');
    card.className = 'cl-card';

    var title = document.createElement('div');
    title.className = 'cl-card-title';
    title.textContent = group.title || group.topic;
    card.appendChild(title);

    var meta = document.createElement('div');
    meta.className = 'cl-card-meta';

    // Source count
    var countSpan = document.createElement('span');
    countSpan.textContent = group.sources.length + ' source' + (group.sources.length !== 1 ? 's' : '');
    meta.appendChild(countSpan);

    // Type icons
    var types = {};
    group.sources.forEach(function(s) { types[s.source_type] = true; });
    Object.keys(types).forEach(function(t) {
        var icon = document.createElement('span');
        icon.className = 'cl-type-icon';
        if (t === 'luperiq_fact_sheet') icon.className += ' luperiq';
        icon.textContent = _clSourceTypeLabel(t);
        meta.appendChild(icon);
    });

    card.appendChild(meta);

    // Conflict badge (rendered when _clConflicts is populated by Plan C)
    var groupConflicts = _clConflicts.filter(function(c) {
        return group.sources.some(function(s) { return s.source_id === c.source_id; })
            && c.resolution === 'pending';
    });
    if (groupConflicts.length > 0) {
        var fieldCount = 0;
        groupConflicts.forEach(function(c) { fieldCount += c.conflicting_fields.length; });
        var conflictBadge = document.createElement('span');
        conflictBadge.className = 'cl-conflict-badge';
        conflictBadge.textContent = fieldCount + ' conflict' + (fieldCount !== 1 ? 's' : '');
        meta.appendChild(conflictBadge);
    }

    card.addEventListener('click', function() {
        _clShowDetail(group);
    });

    return card;
}

async function load_content_library() {
    var main = document.getElementById('adminMain');
    if (!main) return;
    main.textContent = '';

    // Loading state
    var loadMsg = document.createElement('p');
    loadMsg.textContent = 'Loading content library...';
    loadMsg.style.cssText = 'color:var(--text-muted,#888);padding:2rem;text-align:center;';
    main.appendChild(loadMsg);

    await Promise.all([_clFetchSources(), _clFetchPricing(), _clFetchConflicts()]);
    main.textContent = '';

    // Header
    var header = document.createElement('div');
    header.className = 'cl-header';

    var h2 = document.createElement('h2');
    h2.textContent = 'Content Library';
    header.appendChild(h2);

    var actions = document.createElement('div');
    actions.className = 'cl-actions';

    var uploadBtn = document.createElement('button');
    uploadBtn.className = 'btn btn-primary btn-sm';
    uploadBtn.textContent = 'Upload Content';
    uploadBtn.addEventListener('click', function() { _clShowUploadModal(); });
    actions.appendChild(uploadBtn);

    var scrapeBtn = document.createElement('button');
    scrapeBtn.className = 'btn btn-secondary btn-sm';
    scrapeBtn.textContent = 'Scrape My Site';
    scrapeBtn.addEventListener('click', function() { _clShowScrapeModal(); });
    actions.appendChild(scrapeBtn);

    var commissionBtn = document.createElement('button');
    commissionBtn.className = 'btn btn-secondary btn-sm';
    commissionBtn.textContent = 'Commission Fact Sheet';
    commissionBtn.addEventListener('click', function() { _clShowCommissionModal(); });
    actions.appendChild(commissionBtn);

    header.appendChild(actions);
    main.appendChild(header);

    // Topic cards
    if (_clSources.length === 0) {
        var empty = document.createElement('div');
        empty.className = 'cl-empty';
        var ep = document.createElement('p');
        ep.textContent = 'No content sources yet.';
        empty.appendChild(ep);
        var ep2 = document.createElement('p');
        ep2.textContent = 'Upload your own content, scrape your existing site, or commission a new fact sheet.';
        empty.appendChild(ep2);
        main.appendChild(empty);
        return;
    }

    var groups = _clGroupByTopic(_clSources);
    var grid = document.createElement('div');
    grid.className = 'cl-grid';
    groups.forEach(function(g) {
        grid.appendChild(_clRenderTopicCard(g));
    });
    main.appendChild(grid);
}

// ── Content Library: Source Detail View ────────────────────────────────

function _clShowDetail(group) {
    var main = document.getElementById('adminMain');
    if (!main) return;
    main.textContent = '';

    // Header with back button
    var header = document.createElement('div');
    header.className = 'cl-detail-header';

    var backBtn = document.createElement('button');
    backBtn.className = 'cl-back-btn';
    backBtn.textContent = '\u2190 Back';
    backBtn.addEventListener('click', function() { load_content_library(); });
    header.appendChild(backBtn);

    var h2 = document.createElement('h2');
    h2.textContent = group.title || group.topic;
    h2.style.margin = '0';
    header.appendChild(h2);

    main.appendChild(header);

    // Conflict banner (rendered when conflicts exist — populated by Plan C)
    var groupConflicts = _clConflicts.filter(function(c) {
        return group.sources.some(function(s) { return s.source_id === c.source_id; });
    });
    _clRenderConflictBanner(groupConflicts, main);

    // Source list
    var list = document.createElement('div');
    list.className = 'cl-source-list';

    // V1 note: All sources for a topic are used for page generation by default.
    // Per-source enable/disable toggle is planned for a future iteration
    // (requires an additional field on ContentSource or a separate aggregate).

    group.sources.forEach(function(src) {
        var card = document.createElement('div');
        card.className = 'cl-source-card';

        // Card header
        var cardHeader = document.createElement('div');
        cardHeader.className = 'cl-source-card-header';

        var nameDiv = document.createElement('div');
        var nameStrong = document.createElement('strong');
        nameStrong.textContent = src.title || _clSourceTypeLabel(src.source_type);
        nameDiv.appendChild(nameStrong);
        cardHeader.appendChild(nameDiv);

        var badge = document.createElement('span');
        badge.className = 'cl-source-type-badge';
        if (src.source_type === 'luperiq_fact_sheet') badge.className += ' luperiq';
        badge.textContent = _clSourceTypeLabel(src.source_type);
        cardHeader.appendChild(badge);

        // Show validation status for commissioned/pending sources
        if (src.validation_status && src.validation_status !== 'not_applicable') {
            var statusBadge = document.createElement('span');
            statusBadge.className = 'status-badge';
            var statusLabels = {
                'pending': 'Pending Expert Review',
                'in_review': 'Generating...',
                'validated': 'Validated',
                'rejected': 'Rejected'
            };
            statusBadge.textContent = statusLabels[src.validation_status] || src.validation_status;
            if (src.validation_status === 'pending' || src.validation_status === 'in_review') {
                statusBadge.style.cssText = 'background:rgba(245,158,11,0.15);color:#f59e0b;font-size:0.75rem;padding:2px 8px;border-radius:4px;';
            }
            cardHeader.appendChild(statusBadge);
        }

        card.appendChild(cardHeader);

        // Facts display
        if (src.structured_facts && src.structured_facts.length > 0) {
            var isLuperiq = (src.source_type === 'luperiq_fact_sheet');
            var factsToShow = src.structured_facts;

            // LuperIQ fact sheets: show 10-line random sample
            if (isLuperiq && factsToShow.length > 10) {
                var shuffled = factsToShow.slice();
                for (var i = shuffled.length - 1; i > 0; i--) {
                    var j = Math.floor(Math.random() * (i + 1));
                    var tmp = shuffled[i]; shuffled[i] = shuffled[j]; shuffled[j] = tmp;
                }
                factsToShow = shuffled.slice(0, 10);

                var note = document.createElement('p');
                note.style.cssText = 'font-size:0.8rem;color:var(--text-muted,#888);margin:0 0 0.5rem;';
                note.textContent = 'Showing 10 of ' + src.structured_facts.length + ' verified facts (random sample)';
                card.appendChild(note);
            }

            var factsGrid = document.createElement('div');
            factsGrid.className = 'cl-facts-grid';
            factsToShow.forEach(function(f) {
                var keyEl = document.createElement('span');
                keyEl.className = 'cl-fact-key';
                keyEl.textContent = f.key;
                factsGrid.appendChild(keyEl);

                var valEl = document.createElement('span');
                valEl.className = 'cl-fact-value';
                valEl.textContent = f.value;
                factsGrid.appendChild(valEl);
            });
            card.appendChild(factsGrid);
        }

        // Raw content preview (customer sources only)
        if (src.raw_content && src.source_type !== 'luperiq_fact_sheet') {
            var rawDiv = document.createElement('div');
            rawDiv.className = 'cl-raw-preview';
            rawDiv.textContent = src.raw_content.length > 500
                ? src.raw_content.substring(0, 500) + '...'
                : src.raw_content;
            card.appendChild(rawDiv);
        }

        // Delete button (customer sources only)
        if (src.source_type !== 'luperiq_fact_sheet') {
            var delBtn = document.createElement('button');
            delBtn.className = 'btn btn-danger btn-sm';
            delBtn.textContent = 'Delete';
            delBtn.style.marginTop = '0.75rem';
            delBtn.addEventListener('click', function(e) {
                e.stopPropagation();
                _clDeleteSource(src.source_id, group);
            });
            card.appendChild(delBtn);
        }

        list.appendChild(card);
    });

    main.appendChild(list);
}

async function _clDeleteSource(sourceId, group) {
    if (!confirm('Delete this content source? This cannot be undone.')) return;
    try {
        var r = await fetch('/api/modules/content-sources/sources/' + encodeURIComponent(sourceId), {
            method: 'DELETE'
        });
        var j = await r.json();
        if (j.ok) {
            toast('Content source deleted');
            await _clFetchSources();
            // Refresh the detail view
            var updatedGroup = _clGroupByTopic(_clSources).find(function(g) {
                return g.industry === group.industry && g.topic === group.topic;
            });
            if (updatedGroup) { _clShowDetail(updatedGroup); }
            else { load_content_library(); }
        } else {
            toast('Error: ' + j.message);
        }
    } catch(e) { toast('Failed to delete source'); }
}

// ── Content Library: Upload Flow ──────────────────────────────────────

function _clShowUploadModal() {
    var overlay = document.createElement('div');
    overlay.className = 'cl-modal-overlay';
    overlay.addEventListener('click', function(e) {
        if (e.target === overlay) overlay.remove();
    });

    var modal = document.createElement('div');
    modal.className = 'cl-modal';

    var h3 = document.createElement('h3');
    h3.textContent = 'Upload Content';
    modal.appendChild(h3);

    // Industry + topic inputs
    var formGrid = document.createElement('div');
    formGrid.style.cssText = 'display:grid;grid-template-columns:1fr 1fr;gap:0.75rem;margin-bottom:1rem;';

    var indLabel = document.createElement('label');
    indLabel.textContent = 'Industry Slug';
    indLabel.style.cssText = 'font-size:0.85rem;color:var(--text-muted,#888);';
    var indInput = document.createElement('input');
    indInput.type = 'text';
    indInput.placeholder = 'e.g. pest-control';
    indInput.style.cssText = 'width:100%;padding:0.5rem;border-radius:6px;border:1px solid var(--border,#333);background:var(--input-bg,#161622);color:var(--text,#e0e0e0);box-sizing:border-box;';
    var indDiv = document.createElement('div');
    indDiv.appendChild(indLabel); indDiv.appendChild(indInput);
    formGrid.appendChild(indDiv);

    var topLabel = document.createElement('label');
    topLabel.textContent = 'Topic Slug';
    topLabel.style.cssText = 'font-size:0.85rem;color:var(--text-muted,#888);';
    var topInput = document.createElement('input');
    topInput.type = 'text';
    topInput.placeholder = 'e.g. termites';
    topInput.style.cssText = 'width:100%;padding:0.5rem;border-radius:6px;border:1px solid var(--border,#333);background:var(--input-bg,#161622);color:var(--text,#e0e0e0);box-sizing:border-box;';
    var topDiv = document.createElement('div');
    topDiv.appendChild(topLabel); topDiv.appendChild(topInput);
    formGrid.appendChild(topDiv);

    modal.appendChild(formGrid);

    var titleLabel = document.createElement('label');
    titleLabel.textContent = 'Title';
    titleLabel.style.cssText = 'font-size:0.85rem;color:var(--text-muted,#888);display:block;margin-bottom:0.25rem;';
    var titleInput = document.createElement('input');
    titleInput.type = 'text';
    titleInput.placeholder = 'e.g. Our Termite Treatment Process';
    titleInput.style.cssText = 'width:100%;padding:0.5rem;border-radius:6px;border:1px solid var(--border,#333);background:var(--input-bg,#161622);color:var(--text,#e0e0e0);margin-bottom:1rem;box-sizing:border-box;';
    modal.appendChild(titleLabel);
    modal.appendChild(titleInput);

    // Drop zone
    var zone = document.createElement('div');
    zone.className = 'cl-upload-zone';
    var zoneP = document.createElement('p');
    zoneP.textContent = 'Drag & drop a file here, or click to browse';
    zoneP.style.fontWeight = '600';
    zone.appendChild(zoneP);
    var zoneHint = document.createElement('p');
    zoneHint.textContent = 'Supported: .txt, .md, .csv';
    zone.appendChild(zoneHint);

    var fileInput = document.createElement('input');
    fileInput.type = 'file';
    fileInput.accept = '.txt,.md,.csv,.markdown';
    fileInput.style.display = 'none';

    zone.addEventListener('click', function() { fileInput.click(); });
    zone.addEventListener('dragover', function(e) {
        e.preventDefault(); zone.classList.add('dragover');
    });
    zone.addEventListener('dragleave', function() {
        zone.classList.remove('dragover');
    });
    zone.addEventListener('drop', function(e) {
        e.preventDefault(); zone.classList.remove('dragover');
        if (e.dataTransfer.files.length > 0) {
            _clHandleFile(e.dataTransfer.files[0], overlay, indInput, topInput, titleInput);
        }
    });
    fileInput.addEventListener('change', function() {
        if (fileInput.files.length > 0) {
            _clHandleFile(fileInput.files[0], overlay, indInput, topInput, titleInput);
        }
    });

    modal.appendChild(zone);
    modal.appendChild(fileInput);

    // Guidelines
    var guide = document.createElement('div');
    guide.className = 'cl-guidelines';
    var guideTitle = document.createElement('strong');
    guideTitle.textContent = 'Acceptable content sources:';
    guide.appendChild(guideTitle);

    var acceptList = document.createElement('ul');
    var accepts = [
        'Content you\'ve written about your business and expertise',
        'AI-generated content from tools you\'re licensed to use',
        'Research and data from SEO tools (Surfer, Ahrefs, SEMrush, etc.)',
        'Your existing website content, brochures, and marketing materials'
    ];
    accepts.forEach(function(t) {
        var li = document.createElement('li'); li.textContent = t; acceptList.appendChild(li);
    });
    guide.appendChild(acceptList);

    var rejectTitle = document.createElement('strong');
    rejectTitle.textContent = 'We can\'t accept:';
    guide.appendChild(rejectTitle);

    var rejectList = document.createElement('ul');
    var rejects = [
        'Content copied from other businesses\' websites',
        'Copyrighted material you don\'t have rights to',
        'Content that appears verbatim on other sites you don\'t own'
    ];
    rejects.forEach(function(t) {
        var li = document.createElement('li'); li.textContent = t; rejectList.appendChild(li);
    });
    guide.appendChild(rejectList);

    modal.appendChild(guide);
    overlay.appendChild(modal);
    document.body.appendChild(overlay);
}

async function _clHandleFile(file, overlay, indInput, topInput, titleInput) {
    var industry = indInput.value.trim();
    var topic = topInput.value.trim();
    var title = titleInput.value.trim();

    if (!industry || !topic) {
        toast('Please enter industry and topic slugs first');
        return;
    }
    if (!title) { title = file.name.replace(/\.[^.]+$/, ''); }

    // Upload via multipart
    var formData = new FormData();
    formData.append('file', file);
    formData.append('industry_slug', industry);
    formData.append('topic_slug', topic);
    formData.append('title', title);

    try {
        var r = await fetch('/api/modules/content-sources/upload', {
            method: 'POST',
            body: formData
        });
        var j = await r.json();
        if (j.ok) {
            var msg = 'Content uploaded and parsed';
            if (j.data && j.data.conflicts_detected > 0) {
                msg += ' (' + j.data.conflicts_detected + ' conflicts detected)';
            }
            toast(msg);
            overlay.remove();
            await _clFetchSources();
            load_content_library();
        } else {
            toast('Upload failed: ' + j.message);
        }
    } catch(e) {
        toast('Upload error: ' + e.message);
    }
}

// ── Content Library: Scrape Flow ──────────────────────────────────────

function _clShowScrapeModal() {
    var overlay = document.createElement('div');
    overlay.className = 'cl-modal-overlay';
    overlay.addEventListener('click', function(e) {
        if (e.target === overlay) overlay.remove();
    });

    var modal = document.createElement('div');
    modal.className = 'cl-modal';

    var h3 = document.createElement('h3');
    h3.textContent = 'Scrape Existing Site';
    modal.appendChild(h3);

    var desc = document.createElement('p');
    desc.style.cssText = 'color:var(--text-muted,#888);font-size:0.9rem;margin-bottom:1rem;';
    desc.textContent = 'Enter your website URL and we\'ll extract structured facts from your existing content.';
    modal.appendChild(desc);

    // URL input
    var urlLabel = document.createElement('label');
    urlLabel.textContent = 'Website URL';
    urlLabel.style.cssText = 'font-size:0.85rem;color:var(--text-muted,#888);display:block;margin-bottom:0.25rem;';
    var urlInput = document.createElement('input');
    urlInput.type = 'url';
    urlInput.placeholder = 'https://www.example.com';
    urlInput.style.cssText = 'width:100%;padding:0.5rem;border-radius:6px;border:1px solid var(--border,#333);background:var(--input-bg,#161622);color:var(--text,#e0e0e0);margin-bottom:1rem;box-sizing:border-box;';
    modal.appendChild(urlLabel);
    modal.appendChild(urlInput);

    // Industry + topic
    var formGrid = document.createElement('div');
    formGrid.style.cssText = 'display:grid;grid-template-columns:1fr 1fr;gap:0.75rem;margin-bottom:1rem;';

    var indInput = document.createElement('input');
    indInput.type = 'text'; indInput.placeholder = 'Industry slug';
    indInput.style.cssText = 'width:100%;padding:0.5rem;border-radius:6px;border:1px solid var(--border,#333);background:var(--input-bg,#161622);color:var(--text,#e0e0e0);box-sizing:border-box;';
    formGrid.appendChild(indInput);

    var topInput = document.createElement('input');
    topInput.type = 'text'; topInput.placeholder = 'Topic slug';
    topInput.style.cssText = 'width:100%;padding:0.5rem;border-radius:6px;border:1px solid var(--border,#333);background:var(--input-bg,#161622);color:var(--text,#e0e0e0);box-sizing:border-box;';
    formGrid.appendChild(topInput);

    modal.appendChild(formGrid);

    // Results area (hidden initially)
    var resultsDiv = document.createElement('div');
    resultsDiv.style.display = 'none';
    modal.appendChild(resultsDiv);

    // Scrape button
    var scrapeBtn = document.createElement('button');
    scrapeBtn.className = 'btn btn-primary';
    scrapeBtn.textContent = 'Extract Content';
    scrapeBtn.addEventListener('click', function() {
        _clRunScrape(urlInput.value.trim(), indInput.value.trim(), topInput.value.trim(), resultsDiv, overlay);
    });
    modal.appendChild(scrapeBtn);

    overlay.appendChild(modal);
    document.body.appendChild(overlay);
}

async function _clRunScrape(url, industry, topic, resultsDiv, overlay) {
    if (!url) { toast('Please enter a URL'); return; }
    if (!industry || !topic) { toast('Please enter industry and topic slugs'); return; }

    resultsDiv.textContent = '';
    var loading = document.createElement('p');
    loading.textContent = 'Scraping and extracting content... this may take a minute.';
    loading.style.cssText = 'color:var(--text-muted,#888);padding:1rem 0;';
    resultsDiv.appendChild(loading);
    resultsDiv.style.display = 'block';

    try {
        var r = await fetch('/api/modules/site-blueprint/research', {
            method: 'POST',
            headers: {'Content-Type': 'application/json'},
            body: JSON.stringify({ website_url: url })
        });
        var j = await r.json();

        resultsDiv.textContent = '';

        if (!j.ok) {
            var err = document.createElement('p');
            err.textContent = 'Scrape failed: ' + (j.error || j.message || 'Unknown error');
            err.style.color = '#ef4444';
            resultsDiv.appendChild(err);
            return;
        }

        // Convert research result to structured facts
        var facts = [];
        var data = j.data || j;
        if (data.business_name) facts.push({ key: 'Business Name', value: data.business_name, confidence: 'customer_stated' });
        if (data.tagline) facts.push({ key: 'Tagline', value: data.tagline, confidence: 'customer_stated' });
        if (data.description) facts.push({ key: 'Description', value: data.description, confidence: 'customer_stated' });
        if (data.phone) facts.push({ key: 'Phone', value: data.phone, confidence: 'customer_stated' });
        if (data.email) facts.push({ key: 'Email', value: data.email, confidence: 'customer_stated' });
        if (data.address) facts.push({ key: 'Address', value: data.address, confidence: 'customer_stated' });
        if (data.services && Array.isArray(data.services)) {
            data.services.forEach(function(svc, i) {
                facts.push({ key: 'Service ' + (i+1), value: (svc.name || '') + (svc.description ? ' — ' + svc.description : ''), confidence: 'customer_stated' });
            });
        }

        var rawContent = JSON.stringify(data, null, 2);

        // Show preview
        var previewH = document.createElement('h4');
        previewH.textContent = 'Extracted ' + facts.length + ' facts';
        resultsDiv.appendChild(previewH);

        if (facts.length > 0) {
            var table = document.createElement('table');
            table.className = 'cl-preview-table';
            var thead = document.createElement('thead');
            var headRow = document.createElement('tr');
            var th1 = document.createElement('th'); th1.textContent = 'Key'; headRow.appendChild(th1);
            var th2 = document.createElement('th'); th2.textContent = 'Value'; headRow.appendChild(th2);
            thead.appendChild(headRow);
            table.appendChild(thead);

            var tbody = document.createElement('tbody');
            facts.forEach(function(f) {
                var tr = document.createElement('tr');
                var td1 = document.createElement('td'); td1.textContent = f.key; tr.appendChild(td1);
                var td2 = document.createElement('td'); td2.textContent = f.value; tr.appendChild(td2);
                tbody.appendChild(tr);
            });
            table.appendChild(tbody);
            resultsDiv.appendChild(table);
        }

        // Save button
        var saveBtn = document.createElement('button');
        saveBtn.className = 'btn btn-primary';
        saveBtn.textContent = 'Save as Content Source';
        saveBtn.style.marginTop = '1rem';
        saveBtn.addEventListener('click', async function() {
            var now = Math.floor(Date.now() / 1000);
            var source = {
                source_id: 'scrape-' + Date.now() + '-' + Math.floor(Math.random() * 1000000),
                source_type: 'site_scrape',
                industry_slug: industry,
                topic_slug: topic,
                title: 'Scraped from ' + url,
                structured_facts: facts,
                raw_content: rawContent.substring(0, 2000),
                sharing_tier: 'never_share',
                sharing_discount_applied: false,
                validation_status: 'not_applicable',
                owner_license_key: '',
                created_at: now,
                updated_at: now,
                file_format: 'scrape',
                contributor_id: null,
                contributor_payout_status: 'not_applicable',
                quality_score: null,
                content_type_tag: 'fact_sheet',
                parent_source_id: null,
                transferable: false,
                credit_value: null
            };

            try {
                var r = await fetch('/api/modules/content-sources/sources', {
                    method: 'POST',
                    headers: {'Content-Type':'application/json'},
                    body: JSON.stringify(source)
                });
                var j = await r.json();
                if (j.ok) {
                    toast('Scraped content saved');
                    overlay.remove();
                    await _clFetchSources();
                    load_content_library();
                } else {
                    toast('Save failed: ' + j.message);
                }
            } catch(e) { toast('Save error: ' + e.message); }
        });
        resultsDiv.appendChild(saveBtn);

    } catch(e) {
        resultsDiv.textContent = '';
        var err = document.createElement('p');
        err.textContent = 'Scrape error: ' + e.message;
        err.style.color = '#ef4444';
        resultsDiv.appendChild(err);
    }
}

// ── Content Library: Commission Flow ──────────────────────────────────

function _clShowCommissionModal() {
    var overlay = document.createElement('div');
    overlay.className = 'cl-modal-overlay';
    overlay.addEventListener('click', function(e) {
        if (e.target === overlay) overlay.remove();
    });

    var modal = document.createElement('div');
    modal.className = 'cl-modal';

    var h3 = document.createElement('h3');
    h3.textContent = 'Commission a Fact Sheet';
    modal.appendChild(h3);

    var desc = document.createElement('p');
    desc.style.cssText = 'color:var(--text-muted,#888);font-size:0.9rem;margin-bottom:1rem;';
    desc.textContent = 'Choose a quality tier for your commissioned fact sheet. AI Verified is fast and automated; Expert Reviewed includes human review.';
    modal.appendChild(desc);

    // Industry + topic
    var formGrid = document.createElement('div');
    formGrid.style.cssText = 'display:grid;grid-template-columns:1fr 1fr;gap:0.75rem;margin-bottom:1rem;';

    var indInput = document.createElement('input');
    indInput.type = 'text'; indInput.placeholder = 'Industry slug';
    indInput.style.cssText = 'width:100%;padding:0.5rem;border-radius:6px;border:1px solid var(--border,#333);background:var(--input-bg,#161622);color:var(--text,#e0e0e0);box-sizing:border-box;';
    formGrid.appendChild(indInput);

    var topInput = document.createElement('input');
    topInput.type = 'text'; topInput.placeholder = 'Topic slug';
    topInput.style.cssText = 'width:100%;padding:0.5rem;border-radius:6px;border:1px solid var(--border,#333);background:var(--input-bg,#161622);color:var(--text,#e0e0e0);box-sizing:border-box;';
    formGrid.appendChild(topInput);

    modal.appendChild(formGrid);

    // Tier cards
    var tiers = document.createElement('div');
    tiers.className = 'cl-tiers';
    var selectedTier = { value: '' };

    var aiCost = _clPricing.credits_ai_verified || 50;
    var expertCost = _clPricing.credits_expert_reviewed || 200;

    // AI Verified card
    var aiCard = document.createElement('div');
    aiCard.className = 'cl-tier-card';
    var aiH4 = document.createElement('h4'); aiH4.textContent = 'AI Verified'; aiCard.appendChild(aiH4);
    var aiPrice = document.createElement('div');
    aiPrice.className = 'cl-tier-price';
    aiPrice.textContent = aiCost + ' credits';
    aiCard.appendChild(aiPrice);
    var aiDesc = document.createElement('div');
    aiDesc.className = 'cl-tier-desc';
    aiDesc.textContent = 'Automated AI generation with cross-check verification. Ready in minutes.';
    aiCard.appendChild(aiDesc);
    aiCard.addEventListener('click', function() {
        selectedTier.value = 'ai_verified';
        aiCard.classList.add('selected');
        expertCard.classList.remove('selected');
    });
    tiers.appendChild(aiCard);

    // Expert Reviewed card
    var expertCard = document.createElement('div');
    expertCard.className = 'cl-tier-card';
    var exH4 = document.createElement('h4'); exH4.textContent = 'Expert Reviewed'; expertCard.appendChild(exH4);
    var exPrice = document.createElement('div');
    exPrice.className = 'cl-tier-price';
    exPrice.textContent = expertCost + ' credits';
    expertCard.appendChild(exPrice);
    var exDesc = document.createElement('div');
    exDesc.className = 'cl-tier-desc';
    exDesc.textContent = 'AI generation plus human expert review by the LuperIQ team. 24\u201348 hours.';
    expertCard.appendChild(exDesc);
    expertCard.addEventListener('click', function() {
        selectedTier.value = 'expert_reviewed';
        expertCard.classList.add('selected');
        aiCard.classList.remove('selected');
    });
    tiers.appendChild(expertCard);

    modal.appendChild(tiers);

    // Commission button
    var commBtn = document.createElement('button');
    commBtn.className = 'btn btn-primary';
    commBtn.textContent = 'Commission Fact Sheet';
    commBtn.addEventListener('click', function() {
        _clRunCommission(indInput.value.trim(), topInput.value.trim(), selectedTier.value, overlay);
    });
    modal.appendChild(commBtn);

    overlay.appendChild(modal);
    document.body.appendChild(overlay);
}

async function _clRunCommission(industry, topic, tier, overlay) {
    if (!industry || !topic) { toast('Please enter industry and topic slugs'); return; }
    if (!tier) { toast('Please select a tier'); return; }

    var creditCost = tier === 'ai_verified'
        ? (_clPricing.credits_ai_verified || 50)
        : (_clPricing.credits_expert_reviewed || 200);

    // Commission via server-side endpoint (handles credit deduction internally)
    try {
        var r = await fetch('/api/modules/content-sources/commission', {
            method: 'POST',
            headers: {'Content-Type':'application/json'},
            body: JSON.stringify({
                industry_slug: industry,
                topic_slug: topic,
                tier: tier
            })
        });
        var j = await r.json();
        if (j.ok) {
            toast('Fact sheet commissioned (' + creditCost + ' credits). ' +
                  (tier === 'ai_verified' ? 'Generating...' : 'Pending expert review.'));
            overlay.remove();
            await _clFetchSources();
            load_content_library();
        } else {
            toast('Commission failed: ' + j.message);
        }
    } catch(e) { toast('Commission error: ' + e.message); }
}

// ── Content Library: Conflict Display ─────────────────────────────────

function _clRenderConflictBanner(conflicts, container) {
    if (!conflicts || conflicts.length === 0) return;

    var pending = conflicts.filter(function(c) { return c.resolution === 'pending'; });
    if (pending.length === 0) return;

    var banner = document.createElement('div');
    banner.className = 'cl-conflict-banner';

    var icon = document.createElement('span');
    icon.textContent = '\u26A0';
    banner.appendChild(icon);

    var text = document.createElement('span');
    var fieldCount = 0;
    pending.forEach(function(c) { fieldCount += c.conflicting_fields.length; });
    text.textContent = fieldCount + ' item' + (fieldCount !== 1 ? 's' : '') + ' differ from verified data';
    banner.appendChild(text);

    var viewBtn = document.createElement('button');
    viewBtn.className = 'btn btn-sm';
    viewBtn.textContent = 'Review';
    viewBtn.style.cssText = 'margin-left:auto;background:rgba(245,158,11,0.2);color:#f59e0b;border:none;padding:4px 12px;border-radius:4px;cursor:pointer;';
    viewBtn.addEventListener('click', function() {
        _clShowConflictDetail(pending, container);
    });
    banner.appendChild(viewBtn);

    container.insertBefore(banner, container.firstChild);
}

function _clShowConflictDetail(conflicts, container) {
    // Find or create detail section
    var existing = container.querySelector('.cl-conflict-detail');
    if (existing) { existing.remove(); return; }

    var detail = document.createElement('div');
    detail.className = 'cl-conflict-detail';
    detail.style.marginBottom = '1rem';

    conflicts.forEach(function(conflict) {
        conflict.conflicting_fields.forEach(function(field) {
            var row = document.createElement('div');
            row.className = 'cl-conflict-row';

            var leftSide = document.createElement('div');
            leftSide.className = 'cl-conflict-side';
            var leftLabel = document.createElement('label');
            leftLabel.textContent = 'Verified Data';
            leftSide.appendChild(leftLabel);
            var leftVal = document.createElement('p');
            leftVal.textContent = field.field_name + ': ' + field.luperiq_value_summary;
            leftSide.appendChild(leftVal);
            row.appendChild(leftSide);

            var rightSide = document.createElement('div');
            rightSide.className = 'cl-conflict-side';
            var rightLabel = document.createElement('label');
            rightLabel.textContent = 'Your Content';
            rightSide.appendChild(rightLabel);
            var rightVal = document.createElement('p');
            rightVal.textContent = field.field_name + ': ' + field.customer_value;
            rightSide.appendChild(rightVal);
            row.appendChild(rightSide);

            detail.appendChild(row);
        });

        var actions = document.createElement('div');
        actions.className = 'cl-conflict-actions';

        var proceedBtn = document.createElement('button');
        proceedBtn.className = 'btn btn-sm btn-primary';
        proceedBtn.textContent = 'Proceed with my content';
        proceedBtn.addEventListener('click', function() {
            _clResolveConflict(conflict.conflict_id, 'customer_proceeded', container);
        });
        actions.appendChild(proceedBtn);

        var deferBtn = document.createElement('button');
        deferBtn.className = 'btn btn-sm btn-secondary';
        deferBtn.textContent = 'Use LuperIQ\u2019s data';
        deferBtn.addEventListener('click', function() {
            _clResolveConflict(conflict.conflict_id, 'customer_deferred', container);
        });
        actions.appendChild(deferBtn);

        detail.appendChild(actions);
    });

    // Insert after the banner
    var banner = container.querySelector('.cl-conflict-banner');
    if (banner && banner.nextSibling) {
        container.insertBefore(detail, banner.nextSibling);
    } else {
        container.appendChild(detail);
    }
}

async function _clResolveConflict(conflictId, resolution, container) {
    try {
        var r = await fetch('/api/modules/content-sources/conflicts/' + encodeURIComponent(conflictId), {
            method: 'PUT',
            headers: {'Content-Type':'application/json'},
            body: JSON.stringify({ resolution: resolution, customer_notes: '' })
        });
        var j = await r.json();
        if (j.ok) {
            toast('Conflict resolved');
            await _clFetchConflicts();
            var banner = container.querySelector('.cl-conflict-banner');
            var detail = container.querySelector('.cl-conflict-detail');
            if (banner) banner.remove();
            if (detail) detail.remove();
        } else {
            toast('Error: ' + j.message);
        }
    } catch(e) { toast('Failed to resolve conflict'); }
}
"##;
