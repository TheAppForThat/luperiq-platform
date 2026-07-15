//! Pages sub-tab for the unified Design Studio.
//!
//! Provides: searchable page dropdown, inline block editor,
//! SEO metadata panel, and live preview of the selected page.
//!
//! AI context: page title, meta description, focus keyword,
//! og:image, schema type.

pub fn pages_js() -> &'static str {
    PAGES_JS
}

const PAGES_JS: &str = r####"
/* ── Pages sub-tab ────────────────────────────────────────────────── */

TsStudio.loadPages = async function() {
    var controls = document.getElementById('tsStudioControls');
    if (!controls) return;
    controls.replaceChildren();

    // ── Search card ──
    var searchCard = tsCard();
    searchCard.appendChild(tsH2('Pages'));

    var searchInput = tsInput('text', '', {});
    searchInput.placeholder = 'Search pages\u2026';
    searchInput.style.cssText = 'width:100%;margin-bottom:8px;';
    searchCard.appendChild(searchInput);

    var pageList = document.createElement('div');
    pageList.className = 'ts-page-list';
    pageList.style.cssText = 'max-height:200px;overflow-y:auto;';
    searchCard.appendChild(pageList);
    controls.appendChild(searchCard);

    // ── Editor area (hidden until page selected) ──
    var editorCard = tsCard();
    editorCard.id = 'tsPageEditorCard';
    editorCard.style.display = 'none';
    controls.appendChild(editorCard);

    // ── SEO panel (hidden until page selected) ──
    var seoCard = tsCard();
    seoCard.id = 'tsPageSeoCard';
    seoCard.style.display = 'none';
    controls.appendChild(seoCard);

    // ── State ──
    var _selectedPageId = null;
    var _blockEditor = null;
    var _searchTimeout = null;

    // ── Search with debounce ──
    searchInput.addEventListener('input', function() {
        clearTimeout(_searchTimeout);
        _searchTimeout = setTimeout(function() {
            loadPageList(searchInput.value.trim());
        }, 300);
    });

    // ── Load page list ──
    async function loadPageList(query) {
        var gqlQuery;
        if (query) {
            gqlQuery = 'query { contentSearch(query: "' + query.replace(/"/g, '\\"') + '", contentType: "page", limit: 50) { items { contentId title slug status } total } }';
        } else {
            gqlQuery = 'query { contentList(contentType: "page", limit: 50, sortBy: "updated_at", sortDir: "desc") { items { contentId title slug status } total } }';
        }
        try {
            var res = await fetch('/api/graphql', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ query: gqlQuery })
            });
            var data = await res.json();
            var result = data.data ? (data.data.contentSearch || data.data.contentList) : null;
            var items = result ? (result.items || []) : [];
            renderPageList(items);
        } catch (e) {
            pageList.textContent = 'Error loading pages';
        }
    }

    function renderPageList(items) {
        pageList.replaceChildren();
        if (items.length === 0) {
            pageList.appendChild(tsEmpty('No pages found'));
            return;
        }
        items.forEach(function(item) {
            var row = document.createElement('div');
            row.style.cssText = 'display:flex;align-items:center;justify-content:space-between;padding:8px 10px;border-bottom:1px solid var(--border);cursor:pointer;font-size:13px;';
            row.onmouseover = function() { row.style.background = 'var(--bg)'; };
            row.onmouseout = function() { row.style.background = ''; };

            var title = document.createElement('span');
            title.textContent = item.title || '(untitled)';
            title.style.cssText = 'flex:1;overflow:hidden;text-overflow:ellipsis;white-space:nowrap;';
            row.appendChild(title);

            var slug = document.createElement('span');
            slug.textContent = '/' + item.slug;
            slug.style.cssText = 'color:var(--text-muted);font-size:11px;margin:0 8px;';
            row.appendChild(slug);

            var badge = document.createElement('span');
            badge.textContent = item.status;
            badge.className = 'ts-badge';
            badge.style.cssText = item.status === 'published' ? 'background:var(--accent);' : 'background:var(--text-muted);';
            row.appendChild(badge);

            row.onclick = function() { selectPage(item.contentId, item.slug); };
            pageList.appendChild(row);
        });
    }

    // ── Select and edit a page ──
    async function selectPage(contentId, slug) {
        _selectedPageId = contentId;

        // Navigate preview
        TsStudio.navigatePreview('/' + slug);

        // Fetch full content
        try {
            var res = await fetch('/api/graphql', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ query: 'query { content(contentId: "' + contentId + '") { contentId title slug bodyJson excerpt status } }' })
            });
            var data = await res.json();
            var page = data.data ? data.data.content : null;
            if (!page) { if (typeof showToast === 'function') showToast('Could not load page', 'error'); return; }

            renderPageEditor(page);
            await loadSeoData(contentId);
            registerPageAi(page);
        } catch (e) {
            if (typeof showToast === 'function') showToast('Error loading page', 'error');
        }
    }

    function renderPageEditor(page) {
        editorCard.replaceChildren();
        editorCard.style.display = '';

        editorCard.appendChild(tsH2('Edit Page'));

        // Title
        var titleRow = document.createElement('div');
        titleRow.style.cssText = 'margin-bottom:8px;';
        titleRow.appendChild(tsLabel('Title'));
        var titleInput = tsInput('text', page.title || '', {});
        titleInput.id = 'tsPageTitle';
        titleInput.style.width = '100%';
        titleRow.appendChild(titleInput);
        editorCard.appendChild(titleRow);

        // Slug
        var slugRow = document.createElement('div');
        slugRow.style.cssText = 'margin-bottom:8px;';
        slugRow.appendChild(tsLabel('Slug'));
        var slugInput = tsInput('text', page.slug || '', {});
        slugInput.id = 'tsPageSlug';
        slugInput.style.width = '100%';
        slugRow.appendChild(slugInput);
        editorCard.appendChild(slugRow);

        // Excerpt
        var excRow = document.createElement('div');
        excRow.style.cssText = 'margin-bottom:12px;';
        excRow.appendChild(tsLabel('Excerpt'));
        var excInput = tsInput('text', page.excerpt || '', {});
        excInput.id = 'tsPageExcerpt';
        excInput.style.width = '100%';
        excRow.appendChild(excInput);
        editorCard.appendChild(excRow);

        // Block editor
        var editorContainer = document.createElement('div');
        editorContainer.style.cssText = 'min-height:300px;border:1px solid var(--border);border-radius:6px;margin-bottom:12px;';
        editorCard.appendChild(editorContainer);

        var blocks = [];
        try { blocks = JSON.parse(page.bodyJson || '[]'); } catch(e) {}
        if (typeof BlockEditor === 'function') {
            _blockEditor = new BlockEditor(editorContainer, { blocks: blocks });
        } else {
            editorContainer.textContent = 'Block editor not available';
        }

        // Save button
        var saveBtn = tsBtn('Save Page', async function() {
            var title = document.getElementById('tsPageTitle').value;
            var sl = document.getElementById('tsPageSlug').value;
            var exc = document.getElementById('tsPageExcerpt').value;
            var bj = _blockEditor ? JSON.stringify(_blockEditor.getBlocks()) : page.bodyJson;

            var mutation = 'mutation { updateContent(contentId: "' + _selectedPageId + '", title: "' + title.replace(/"/g, '\\"') + '", slug: "' + sl.replace(/"/g, '\\"') + '", bodyJson: "' + bj.replace(/\\/g, '\\\\').replace(/"/g, '\\"') + '", excerpt: "' + exc.replace(/"/g, '\\"') + '") { contentId } }';
            try {
                var res = await fetch('/api/graphql', {
                    method: 'POST',
                    headers: { 'Content-Type': 'application/json' },
                    body: JSON.stringify({ query: mutation })
                });
                var data = await res.json();
                if (data.data && data.data.updateContent) {
                    if (typeof showToast === 'function') showToast('Page saved', 'success');
                    TsStudio.navigatePreview('/' + sl);
                } else {
                    if (typeof showToast === 'function') showToast('Save failed', 'error');
                }
            } catch(e) {
                if (typeof showToast === 'function') showToast('Save error: ' + e.message, 'error');
            }
        });
        editorCard.appendChild(saveBtn);
    }

    // ── SEO panel ──
    async function loadSeoData(contentId) {
        seoCard.replaceChildren();
        seoCard.style.display = '';

        var toggle = document.createElement('div');
        toggle.style.cssText = 'display:flex;align-items:center;justify-content:space-between;cursor:pointer;';
        toggle.appendChild(tsH2('SEO'));
        var arrow = document.createElement('span');
        arrow.textContent = '\u25BC';
        arrow.style.cssText = 'font-size:12px;color:var(--text-muted);';
        toggle.appendChild(arrow);
        seoCard.appendChild(toggle);

        var body = document.createElement('div');
        body.id = 'tsPageSeoBody';
        seoCard.appendChild(body);

        toggle.onclick = function() {
            body.style.display = body.style.display === 'none' ? '' : 'none';
            arrow.textContent = body.style.display === 'none' ? '\u25B6' : '\u25BC';
        };

        try {
            var res = await fetch('/api/modules/seo/meta/' + contentId);
            var data = await res.json();
            var seo = (data.ok && data.data) ? data.data : {};
            renderSeoFields(body, contentId, seo);
        } catch(e) {
            body.textContent = 'Could not load SEO data';
        }
    }

    function renderSeoFields(container, contentId, seo) {
        container.replaceChildren();

        // Score indicator
        var score = seo.seo_score || 0;
        var scoreColor = score >= 80 ? '#22c55e' : score >= 50 ? '#eab308' : '#ef4444';
        var scoreRow = document.createElement('div');
        scoreRow.style.cssText = 'display:flex;align-items:center;gap:8px;margin-bottom:12px;';
        var scoreDot = document.createElement('span');
        scoreDot.style.cssText = 'width:12px;height:12px;border-radius:50%;background:' + scoreColor;
        scoreRow.appendChild(scoreDot);
        var scoreLabel = document.createElement('span');
        scoreLabel.textContent = 'SEO Score: ' + score + '/100';
        scoreLabel.style.cssText = 'font-size:13px;font-weight:600;';
        scoreRow.appendChild(scoreLabel);
        container.appendChild(scoreRow);

        var fields = [
            { key: 'title', label: 'SEO Title', type: 'text', hint: '50\u201360 characters' },
            { key: 'description', label: 'Meta Description', type: 'text', hint: '150\u2013160 characters' },
            { key: 'focus_keyword', label: 'Focus Keyword', type: 'text', hint: 'Primary keyword' },
            { key: 'og_image', label: 'OG Image URL', type: 'text', hint: 'Social share image' },
            { key: 'canonical_url', label: 'Canonical URL', type: 'text', hint: 'Leave empty for auto' },
            { key: 'robots', label: 'Robots', type: 'text', hint: 'e.g. index, follow' }
        ];

        fields.forEach(function(f) {
            var row = document.createElement('div');
            row.style.cssText = 'margin-bottom:8px;';
            row.appendChild(tsLabel(f.label));
            var input = tsInput(f.type, seo[f.key] || '', {});
            input.id = 'tsSeo_' + f.key;
            input.placeholder = f.hint;
            input.style.width = '100%';
            row.appendChild(input);
            container.appendChild(row);
        });

        // Schema JSON (textarea)
        var schemaRow = document.createElement('div');
        schemaRow.style.cssText = 'margin-bottom:8px;';
        schemaRow.appendChild(tsLabel('Schema JSON-LD'));
        var schemaArea = document.createElement('textarea');
        schemaArea.id = 'tsSeo_schema_json';
        schemaArea.value = seo.schema_json || '';
        schemaArea.style.cssText = 'width:100%;min-height:80px;padding:6px 8px;border:1px solid var(--border);border-radius:4px;background:var(--bg);color:var(--text);font-size:12px;font-family:monospace;resize:vertical;box-sizing:border-box;';
        schemaRow.appendChild(schemaArea);
        container.appendChild(schemaRow);

        // Save SEO button
        var saveSeoBtn = tsBtn('Save SEO', async function() {
            var payload = {};
            fields.forEach(function(f) {
                var el = document.getElementById('tsSeo_' + f.key);
                if (el) payload[f.key] = el.value;
            });
            payload.schema_json = document.getElementById('tsSeo_schema_json').value;

            try {
                var res = await fetch('/api/modules/seo/meta/' + contentId, {
                    method: 'PUT',
                    headers: { 'Content-Type': 'application/json' },
                    body: JSON.stringify(payload)
                });
                var data = await res.json();
                if (data.ok) {
                    if (typeof showToast === 'function') showToast('SEO data saved', 'success');
                } else {
                    if (typeof showToast === 'function') showToast('SEO save failed', 'error');
                }
            } catch(e) {
                if (typeof showToast === 'function') showToast('SEO save error', 'error');
            }
        });
        container.appendChild(saveSeoBtn);
    }

    // ── AI context ──
    function registerPageAi(page) {
        if (window.LiqAI && window.LiqAI.panel) {
            window.LiqAI.panel.register({
                featureKey: 'ts_page_' + page.contentId,
                placeholder: "Describe what this page should convey, e.g. 'Optimize for local plumbing services in Fort Worth'",
                creditCost: 3,
                properties: [
                    { key: 'title', label: 'Page Title', group: 'Content', tip: 'The page heading and browser tab title' },
                    { key: 'excerpt', label: 'Excerpt', group: 'Content', tip: 'Brief summary shown in listings and previews' },
                    { key: 'seo_title', label: 'SEO Title', group: 'SEO', tip: 'Title shown in search results (50\u201360 chars)' },
                    { key: 'seo_description', label: 'Meta Description', group: 'SEO', tip: 'Description shown in search results (150\u2013160 chars)' },
                    { key: 'focus_keyword', label: 'Focus Keyword', group: 'SEO', tip: 'Primary keyword this page targets' },
                    { key: 'og_image', label: 'OG Image', group: 'SEO', tip: 'Image shown when page is shared on social media' },
                    { key: 'schema_type', label: 'Schema Type', group: 'SEO', tip: 'JSON-LD structured data type' }
                ],
                onResult: function(result, checkedKeys) {
                    if (typeof result !== 'object') return;
                    Object.keys(result).forEach(function(k) {
                        if (checkedKeys && checkedKeys.indexOf(k) < 0) return;
                        var el = document.getElementById('tsPage' + k.charAt(0).toUpperCase() + k.slice(1)) ||
                                 document.getElementById('tsSeo_' + k);
                        if (el) el.value = result[k];
                    });
                    if (typeof showToast === 'function') showToast('AI suggestions applied', 'success');
                },
                captureState: function() {
                    return {
                        title: (document.getElementById('tsPageTitle') || {}).value || '',
                        excerpt: (document.getElementById('tsPageExcerpt') || {}).value || '',
                        seo_title: (document.getElementById('tsSeo_title') || {}).value || '',
                        seo_description: (document.getElementById('tsSeo_description') || {}).value || '',
                        focus_keyword: (document.getElementById('tsSeo_focus_keyword') || {}).value || '',
                        og_image: (document.getElementById('tsSeo_og_image') || {}).value || '',
                        schema_type: (document.getElementById('tsSeo_schema_json') || {}).value || ''
                    };
                },
                restoreState: function(snap) {
                    if (!snap) return;
                    Object.keys(snap).forEach(function(k) {
                        var el = document.getElementById('tsPage' + k.charAt(0).toUpperCase() + k.slice(1)) ||
                                 document.getElementById('tsSeo_' + k);
                        if (el) el.value = snap[k];
                    });
                }
            });
        }
    }

    // On tab entry, unregister previous AI context if no page selected
    if (window.LiqAI && window.LiqAI.panel) {
        window.LiqAI.panel.unregister();
    }

    // Load initial page list
    await loadPageList('');
};
"####;
