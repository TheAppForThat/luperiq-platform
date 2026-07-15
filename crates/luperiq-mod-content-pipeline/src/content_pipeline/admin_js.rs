//! Admin dashboard JavaScript for the Content Pipeline module.
//!
//! Four admin views:
//! 1. content-pipeline-generator — page type + target + quality, Generate + Generate Site
//! 2. content-pipeline-jobs — job list with status badges, review/edit/publish
//! 3. content-pipeline-templates — prompt template editor
//! 4. content-pipeline-seo — SEO guidelines browser + fact pack browser

/// Admin JavaScript for the Content Pipeline module.
pub const ADMIN_JS: &str = r#"
var _cpJobs = [];
var _cpTemplates = [];
var _cpGuidelines = [];
var _cpFactPacks = [];

// ── Content Pipeline: Generator View ─────────────────────────────────

function load_content_pipeline_generator() {
    var container = document.getElementById('adminMain');
    container.textContent = '';

    var _role = (window.__CMS && window.__CMS.nexusRole) || '';
    var _isPro = _role === 'central' || _role === 'professional' || _role === 'enterprise';
    var _isStarter = _isPro || _role === 'starter';

    // Pricing card
    var _pc = lqModulePricingCard({ name: 'Content Pipeline', monthly: 19, annual: 189, lifetime: 499, tier: 'professional', deps: ['Company Profile', 'Industry Profile', 'Location Profile'], slug: 'content-pipeline' });
    if (_pc) container.appendChild(_pc);

    var header = document.createElement('h2');
    header.textContent = 'Content Generator';
    if (!_isPro) {
        var _tierBadge = document.createElement('span');
        _tierBadge.className = 'status-badge status-published';
        _tierBadge.style.cssText = 'margin-left:8px;background:#f59e0b;color:#000;font-size:11px;';
        _tierBadge.textContent = 'PRO';
        header.appendChild(_tierBadge);
    }
    container.appendChild(header);

    var desc = document.createElement('p');
    desc.textContent = 'Generate AI-powered content using your Company Profile, Industry Profile, Location Profiles, SEO Guidelines, and Fact Packs.';
    container.appendChild(desc);

    // Form
    var form = document.createElement('div');
    form.className = 'cp-generator-form';

    // Page type
    var ptLabel = document.createElement('label');
    ptLabel.textContent = 'Page Type';
    form.appendChild(ptLabel);
    var ptSelect = document.createElement('select');
    ptSelect.id = 'cp-page-type';
    var pageTypes = ['homepage','about','service-page','equipment-page','area-page','blog-post'];
    for (var i = 0; i < pageTypes.length; i++) {
        var opt = document.createElement('option');
        opt.value = pageTypes[i];
        opt.textContent = pageTypes[i];
        ptSelect.appendChild(opt);
    }
    form.appendChild(ptSelect);

    // Target slug
    var tsLabel = document.createElement('label');
    tsLabel.textContent = 'Target Slug';
    form.appendChild(tsLabel);
    var tsInput = document.createElement('input');
    tsInput.type = 'text';
    tsInput.id = 'cp-target-slug';
    tsInput.placeholder = 'e.g. ac-repair, fort-worth-tx, german-cockroaches';
    form.appendChild(tsInput);

    // Quality
    var qLabel = document.createElement('label');
    qLabel.textContent = 'Quality';
    form.appendChild(qLabel);
    var qDiv = document.createElement('div');
    qDiv.className = 'cp-quality-options';
    var qdraft = document.createElement('label');
    var rdraft = document.createElement('input');
    rdraft.type = 'radio';
    rdraft.name = 'cp-quality';
    rdraft.value = 'quick_draft';
    rdraft.checked = true;
    qdraft.appendChild(rdraft);
    var draftText = document.createTextNode(' Quick Draft (local model)');
    qdraft.appendChild(draftText);
    qDiv.appendChild(qdraft);
    var qprem = document.createElement('label');
    var rprem = document.createElement('input');
    rprem.type = 'radio';
    rprem.name = 'cp-quality';
    rprem.value = 'premium';
    qprem.appendChild(rprem);
    var premText = document.createTextNode(' Premium (cloud model)');
    qprem.appendChild(premText);
    qDiv.appendChild(qprem);
    form.appendChild(qDiv);

    // Buttons
    var btnRow = document.createElement('div');
    btnRow.className = 'cp-btn-row';
    var genBtn = document.createElement('button');
    genBtn.className = 'btn btn-primary';
    genBtn.textContent = 'Generate Page';
    genBtn.addEventListener('click', function() { cpGenerateSingle(); });
    btnRow.appendChild(genBtn);
    var siteBtn = document.createElement('button');
    siteBtn.className = 'btn btn-secondary';
    siteBtn.textContent = 'Generate Entire Site';
    if (!_isPro) { siteBtn.disabled = true; siteBtn.title = 'Upgrade to Professional'; }
    siteBtn.addEventListener('click', function() { cpGenerateSite(); });
    btnRow.appendChild(siteBtn);

    // AI outline button
    if (typeof LiqAI !== 'undefined') {
        var _aiOutBtn = LiqAI.button({
            label: 'AI Generate Outline',
            feature: 'content_ai_outline',
            credits: 2,
            tier: 'free',
            getInput: function() {
                var pageType = document.getElementById('cp-page-type') ? document.getElementById('cp-page-type').value : '';
                var target = document.getElementById('cp-target-slug') ? document.getElementById('cp-target-slug').value : '';
                if (!pageType) { showToast('Select a page type first', 'error'); return ''; }
                return 'Page type: ' + pageType + '\nTarget: ' + (target || 'general');
            },
            onResult: function(result) {
                if (result && result.title) {
                    showToast('Outline generated: ' + result.title, 'success');
                }
            },
        });
        if (_aiOutBtn) btnRow.appendChild(_aiOutBtn);
    }
    form.appendChild(btnRow);

    container.appendChild(form);

    // Status area
    var statusDiv = document.createElement('div');
    statusDiv.id = 'cp-gen-status';
    container.appendChild(statusDiv);

    // Preview area
    var previewDiv = document.createElement('div');
    previewDiv.id = 'cp-gen-preview';
    previewDiv.className = 'cp-preview';
    container.appendChild(previewDiv);
}

function cpGenerateSingle() {
    var pageType = document.getElementById('cp-page-type').value;
    var target = document.getElementById('cp-target-slug').value;
    var quality = document.querySelector('input[name="cp-quality"]:checked').value;
    var status = document.getElementById('cp-gen-status');
    var preview = document.getElementById('cp-gen-preview');
    status.textContent = 'Generating content...';
    preview.textContent = '';

    fetch('/api/modules/content-pipeline/generate', {
        method: 'POST',
        headers: {'Content-Type': 'application/json'},
        body: JSON.stringify({page_type: pageType, target_slug: target, quality_level: quality})
    })
    .then(function(r) { return r.json(); })
    .then(function(data) {
        if (data.error) {
            status.textContent = 'Error: ' + data.error;
        } else {
            status.textContent = 'Generated! Tokens: ' + (data.token_count || 0) + ' | Time: ' + (data.generation_time_ms || 0) + 'ms | Model: ' + (data.model_used || 'unknown');
            var pre = document.createElement('pre');
            pre.textContent = data.generated_content || data.html || '';
            preview.appendChild(pre);
        }
    })
    .catch(function(e) { status.textContent = 'Error: ' + e.message; });
}

function cpGenerateSite() {
    var quality = document.querySelector('input[name="cp-quality"]:checked').value;
    var status = document.getElementById('cp-gen-status');
    status.textContent = 'Starting full site generation... This may take several minutes.';

    fetch('/api/modules/content-pipeline/generate-site', {
        method: 'POST',
        headers: {'Content-Type': 'application/json'},
        body: JSON.stringify({quality_level: quality})
    })
    .then(function(r) { return r.json(); })
    .then(function(data) {
        if (data.error) {
            status.textContent = 'Error: ' + data.error;
        } else {
            status.textContent = 'Site generation started! ' + (data.job_ids ? data.job_ids.length : 0) + ' jobs queued. Check the Jobs view for progress.';
        }
    })
    .catch(function(e) { status.textContent = 'Error: ' + e.message; });
}

// ── Content Pipeline: Jobs View ──────────────────────────────────────

function load_content_pipeline_jobs() {
    var container = document.getElementById('adminMain');
    container.textContent = '';

    var _role = (window.__CMS && window.__CMS.nexusRole) || '';
    var _isPro = _role === 'central' || _role === 'professional' || _role === 'enterprise';

    var header = document.createElement('h2');
    header.textContent = 'Content Jobs';
    if (!_isPro) {
        var _tierBadge = document.createElement('span');
        _tierBadge.className = 'status-badge status-published';
        _tierBadge.style.cssText = 'margin-left:8px;background:#f59e0b;color:#000;font-size:11px;';
        _tierBadge.textContent = 'PRO';
        header.appendChild(_tierBadge);
    }
    container.appendChild(header);

    var table = document.createElement('table');
    table.className = 'data-table';
    var thead = document.createElement('thead');
    var hrow = document.createElement('tr');
    var cols = ['ID','Page Type','Target','Quality','Status','Tokens','Time','Actions'];
    for (var i = 0; i < cols.length; i++) {
        var th = document.createElement('th');
        th.textContent = cols[i];
        hrow.appendChild(th);
    }
    thead.appendChild(hrow);
    table.appendChild(thead);
    var tbody = document.createElement('tbody');
    tbody.id = 'cp-jobs-body';
    table.appendChild(tbody);
    container.appendChild(table);

    lqAddExportImportBar(container, function(format) {
        if (format === 'json') {
            lqExportJSON(_cpJobs, 'content-pipeline-jobs.json');
        } else {
            lqExportCSV(_cpJobs, ['id','page_type','target_slug','quality_level','status','token_count','generation_time_ms','model_used','error_message'], 'content-pipeline-jobs.csv');
        }
    }, function() {
        lqImportJSON(async function(data) {
            var arr = Array.isArray(data) ? data : [data];
            var ok = 0;
            for (var i = 0; i < arr.length; i++) {
                try {
                    await fetch('/api/modules/content-pipeline/generate', { method: 'POST', headers: {'Content-Type':'application/json'}, body: JSON.stringify(arr[i]) });
                    ok++;
                } catch(e) {}
            }
            showToast('Imported ' + ok + ' of ' + arr.length + ' jobs', 'success');
            cpLoadJobs();
        });
    });

    var detailDiv = document.createElement('div');
    detailDiv.id = 'cp-job-detail';
    detailDiv.className = 'cp-job-detail';
    container.appendChild(detailDiv);

    cpLoadJobs();
}

function cpLoadJobs() {
    fetch('/api/modules/content-pipeline/jobs')
    .then(function(r) { return r.json(); })
    .then(function(data) {
        var tbody = document.getElementById('cp-jobs-body');
        if (!tbody) return;
        tbody.textContent = '';
        var jobs = data.jobs || data || [];
        if (!Array.isArray(jobs)) jobs = [];
        _cpJobs = jobs;
        for (var i = 0; i < jobs.length; i++) {
            var job = jobs[i];
            var tr = document.createElement('tr');
            var cells = [
                (job.id || '').substring(0, 8),
                job.page_type || '',
                job.target_slug || '',
                job.quality_level || '',
                job.status || '',
                String(job.token_count || 0),
                (job.generation_time_ms || 0) + 'ms'
            ];
            for (var c = 0; c < cells.length; c++) {
                var td = document.createElement('td');
                if (c === 4) {
                    var badge = document.createElement('span');
                    badge.className = 'status-badge status-' + (job.status || 'pending');
                    badge.textContent = cells[c];
                    td.appendChild(badge);
                } else {
                    td.textContent = cells[c];
                }
                tr.appendChild(td);
            }
            // Actions cell
            var actTd = document.createElement('td');
            var viewBtn = document.createElement('button');
            viewBtn.className = 'btn btn-sm';
            viewBtn.textContent = 'View';
            viewBtn.setAttribute('data-job-id', job.id);
            viewBtn.addEventListener('click', function() {
                cpViewJob(this.getAttribute('data-job-id'));
            });
            actTd.appendChild(viewBtn);
            if (job.status === 'review') {
                var pubBtn = document.createElement('button');
                pubBtn.className = 'btn btn-sm btn-primary';
                pubBtn.textContent = 'Publish';
                pubBtn.setAttribute('data-job-id', job.id);
                pubBtn.addEventListener('click', function() {
                    cpPublishJob(this.getAttribute('data-job-id'));
                });
                actTd.appendChild(pubBtn);
            }
            tr.appendChild(actTd);
            tbody.appendChild(tr);
        }
    });
}

function cpViewJob(jobId) {
    fetch('/api/modules/content-pipeline/jobs/' + jobId)
    .then(function(r) { return r.json(); })
    .then(function(job) {
        var detail = document.getElementById('cp-job-detail');
        if (!detail) return;
        detail.textContent = '';
        var h3 = document.createElement('h3');
        h3.textContent = 'Job: ' + (job.id || '').substring(0, 8) + ' (' + (job.status || '') + ')';
        detail.appendChild(h3);
        var meta = document.createElement('p');
        meta.textContent = 'Type: ' + (job.page_type || '') + ' | Target: ' + (job.target_slug || '') + ' | Model: ' + (job.model_used || '') + ' | Tokens: ' + (job.token_count || 0);
        detail.appendChild(meta);
        if (job.generated_content) {
            var pre = document.createElement('pre');
            pre.textContent = job.generated_content;
            pre.className = 'cp-content-preview';
            detail.appendChild(pre);
        }
        if (job.error_message) {
            var err = document.createElement('p');
            err.className = 'error-text';
            err.textContent = 'Error: ' + job.error_message;
            detail.appendChild(err);
        }
    });
}

function cpPublishJob(jobId) {
    var now = Math.floor(Date.now() / 1000);
    fetch('/api/modules/content-pipeline/jobs/' + jobId, {
        method: 'PUT',
        headers: {'Content-Type': 'application/json'},
        body: JSON.stringify({status: 'published', published_at: now})
    })
    .then(function(r) { return r.json(); })
    .then(function() { cpLoadJobs(); });
}

// ── Content Pipeline: Templates View ─────────────────────────────────

function load_content_pipeline_templates() {
    var container = document.getElementById('adminMain');
    container.textContent = '';

    var _role = (window.__CMS && window.__CMS.nexusRole) || '';
    var _isPro = _role === 'central' || _role === 'professional' || _role === 'enterprise';

    var header = document.createElement('h2');
    header.textContent = 'Content Templates';
    container.appendChild(header);

    // AI template button
    if (typeof LiqAI !== 'undefined') {
        var _aiRow = document.createElement('div');
        _aiRow.style.cssText = 'margin-bottom:12px;';
        var _aiTplBtn = LiqAI.button({
            label: 'AI Generate Template',
            feature: 'content_ai_template',
            credits: 3,
            tier: 'free',
            getInput: function() {
                var pageType = prompt('What page type is this template for? (e.g. homepage, service-page, area-page)');
                if (!pageType) return '';
                return 'Page type: ' + pageType;
            },
            onResult: function(result) {
                if (typeof result === 'string') {
                    showToast('Template generated. Create a new template and paste it in.', 'success');
                }
            },
        });
        if (_aiTplBtn) _aiRow.appendChild(_aiTplBtn);
        container.appendChild(_aiRow);
    }

    var desc = document.createElement('p');
    desc.textContent = 'Prompt templates control how AI generates content. Use Handlebars syntax: {{company.name}}, {{industry.name}}, {{location.city}}, {{seo.term_frequencies}}, {{facts}}';
    container.appendChild(desc);

    var table = document.createElement('table');
    table.className = 'data-table';
    var thead = document.createElement('thead');
    var hrow = document.createElement('tr');
    var cols = ['ID','Page Type','Industry','Sections','Active','Actions'];
    for (var i = 0; i < cols.length; i++) {
        var th = document.createElement('th');
        th.textContent = cols[i];
        hrow.appendChild(th);
    }
    thead.appendChild(hrow);
    table.appendChild(thead);
    var tbody = document.createElement('tbody');
    tbody.id = 'cp-tpl-body';
    table.appendChild(tbody);
    container.appendChild(table);

    lqAddExportImportBar(container, function(format) {
        if (format === 'json') {
            lqExportJSON(_cpTemplates, 'content-pipeline-templates.json');
        } else {
            lqExportCSV(_cpTemplates, ['id','page_type','industry_slug','prompt_template','active'], 'content-pipeline-templates.csv');
        }
    }, function() {
        lqImportJSON(async function(data) {
            var arr = Array.isArray(data) ? data : [data];
            var ok = 0;
            for (var i = 0; i < arr.length; i++) {
                try {
                    await fetch('/api/modules/content-pipeline/templates', { method: 'POST', headers: {'Content-Type':'application/json'}, body: JSON.stringify(arr[i]) });
                    ok++;
                } catch(e) {}
            }
            showToast('Imported ' + ok + ' of ' + arr.length + ' templates', 'success');
            cpLoadTemplates();
        });
    });

    var editorDiv = document.createElement('div');
    editorDiv.id = 'cp-tpl-editor';
    editorDiv.className = 'cp-tpl-editor';
    container.appendChild(editorDiv);

    cpLoadTemplates();
}

function cpLoadTemplates() {
    fetch('/api/modules/content-pipeline/templates')
    .then(function(r) { return r.json(); })
    .then(function(data) {
        var tbody = document.getElementById('cp-tpl-body');
        if (!tbody) return;
        tbody.textContent = '';
        var templates = data.templates || data || [];
        if (!Array.isArray(templates)) templates = [];
        _cpTemplates = templates;
        for (var i = 0; i < templates.length; i++) {
            var tpl = templates[i];
            var tr = document.createElement('tr');
            var cells = [
                (tpl.id || '').substring(0, 12),
                tpl.page_type || '',
                tpl.industry_slug || '(universal)',
                String((tpl.section_prompts || []).length),
                tpl.active ? 'Yes' : 'No'
            ];
            for (var c = 0; c < cells.length; c++) {
                var td = document.createElement('td');
                td.textContent = cells[c];
                tr.appendChild(td);
            }
            var actTd = document.createElement('td');
            var editBtn = document.createElement('button');
            editBtn.className = 'btn btn-sm';
            editBtn.textContent = 'Edit';
            editBtn.setAttribute('data-tpl-id', tpl.id);
            editBtn.addEventListener('click', function() {
                cpEditTemplate(this.getAttribute('data-tpl-id'));
            });
            actTd.appendChild(editBtn);
            tr.appendChild(actTd);
            tbody.appendChild(tr);
        }
    });
}

function cpEditTemplate(tplId) {
    fetch('/api/modules/content-pipeline/templates')
    .then(function(r) { return r.json(); })
    .then(function(data) {
        var templates = data.templates || data || [];
        var tpl = null;
        for (var i = 0; i < templates.length; i++) {
            if (templates[i].id === tplId) { tpl = templates[i]; break; }
        }
        if (!tpl) return;
        var editor = document.getElementById('cp-tpl-editor');
        if (!editor) return;
        editor.textContent = '';

        var h3 = document.createElement('h3');
        h3.textContent = 'Edit Template: ' + tpl.page_type;
        editor.appendChild(h3);

        var sysLabel = document.createElement('label');
        sysLabel.textContent = 'System Prompt Template (Handlebars)';
        editor.appendChild(sysLabel);
        var sysArea = document.createElement('textarea');
        sysArea.id = 'cp-tpl-system';
        sysArea.rows = 12;
        sysArea.value = tpl.prompt_template || '';
        editor.appendChild(sysArea);

        var secLabel = document.createElement('label');
        secLabel.textContent = 'Section Prompts (one per text area)';
        editor.appendChild(secLabel);
        var secContainer = document.createElement('div');
        secContainer.id = 'cp-tpl-sections';
        var prompts = tpl.section_prompts || [];
        for (var i = 0; i < prompts.length; i++) {
            var area = document.createElement('textarea');
            area.className = 'cp-section-prompt';
            area.rows = 4;
            area.value = prompts[i];
            secContainer.appendChild(area);
        }
        editor.appendChild(secContainer);

        var saveBtn = document.createElement('button');
        saveBtn.className = 'btn btn-primary';
        saveBtn.textContent = 'Save Template';
        saveBtn.addEventListener('click', function() {
            var system = document.getElementById('cp-tpl-system').value;
            var areas = document.querySelectorAll('.cp-section-prompt');
            var secs = [];
            for (var j = 0; j < areas.length; j++) { secs.push(areas[j].value); }
            fetch('/api/modules/content-pipeline/templates/' + tplId, {
                method: 'PUT',
                headers: {'Content-Type': 'application/json'},
                body: JSON.stringify({prompt_template: system, section_prompts: secs})
            })
            .then(function(r) { return r.json(); })
            .then(function() { cpLoadTemplates(); });
        });
        editor.appendChild(saveBtn);
    });
}

// ── Content Pipeline: SEO Data View ──────────────────────────────────

function load_content_pipeline_seo() {
    var container = document.getElementById('adminMain');
    container.textContent = '';
    var role = (window.__CMS && window.__CMS.nexusRole) || '';

    var header = document.createElement('h2');
    header.textContent = 'SEO Data';
    container.appendChild(header);

    if (role && role !== 'central') {
        var note = document.createElement('div');
        note.className = 'card';
        note.style.cssText = 'padding:16px;border:1px solid var(--border,#334155);border-radius:10px;background:var(--surface,#1e293b);max-width:900px;';
        var p1 = document.createElement('p');
        p1.textContent = 'Truth sheets, Surfer-style SEO guides, and fact packs are managed on Central so the proprietary grounding layer stays inside LuperIQ.';
        note.appendChild(p1);
        var p2 = document.createElement('p');
        p2.style.marginBottom = '0';
        p2.textContent = 'Client sites can still generate grounded content, but the raw reference library is not editable here.';
        note.appendChild(p2);
        container.appendChild(note);
        return;
    }

    // Tab bar
    var tabs = document.createElement('div');
    tabs.className = 'cp-tabs';
    var guideTab = document.createElement('button');
    guideTab.className = 'cp-tab active';
    guideTab.textContent = 'SEO Guidelines';
    guideTab.addEventListener('click', function() {
        document.getElementById('cp-seo-guidelines-panel').style.display = 'block';
        document.getElementById('cp-fact-packs-panel').style.display = 'none';
        guideTab.className = 'cp-tab active';
        fpTab.className = 'cp-tab';
    });
    tabs.appendChild(guideTab);
    var fpTab = document.createElement('button');
    fpTab.className = 'cp-tab';
    fpTab.textContent = 'Fact Packs';
    fpTab.addEventListener('click', function() {
        document.getElementById('cp-seo-guidelines-panel').style.display = 'none';
        document.getElementById('cp-fact-packs-panel').style.display = 'block';
        fpTab.className = 'cp-tab active';
        guideTab.className = 'cp-tab';
    });
    tabs.appendChild(fpTab);
    container.appendChild(tabs);

    // Guidelines panel
    var guidePanel = document.createElement('div');
    guidePanel.id = 'cp-seo-guidelines-panel';

    var importBtn = document.createElement('button');
    importBtn.className = 'btn btn-secondary';
    importBtn.textContent = 'Import Guideline';
    importBtn.addEventListener('click', function() { cpShowGuidelineImport(); });
    guidePanel.appendChild(importBtn);

    lqAddExportImportBar(guidePanel, function(format) {
        if (format === 'json') {
            lqExportJSON(_cpGuidelines, 'seo-guidelines.json');
        } else {
            lqExportCSV(_cpGuidelines, ['id','scope','scope_type','industry_slugs','active'], 'seo-guidelines.csv');
        }
    }, function() {
        lqImportJSON(async function(data) {
            var arr = Array.isArray(data) ? data : [data];
            var ok = 0;
            for (var i = 0; i < arr.length; i++) {
                try {
                    await fetch('/api/modules/content-pipeline/seo-guidelines', { method: 'POST', headers: {'Content-Type':'application/json'}, body: JSON.stringify(arr[i]) });
                    ok++;
                } catch(e) {}
            }
            showToast('Imported ' + ok + ' of ' + arr.length + ' guidelines', 'success');
            cpLoadGuidelines();
        });
    });

    var guideTable = document.createElement('table');
    guideTable.className = 'data-table';
    var gthead = document.createElement('thead');
    var ghrow = document.createElement('tr');
    var gcols = ['ID','Scope','Type','Industries','Terms','Facts','Active'];
    for (var i = 0; i < gcols.length; i++) {
        var th = document.createElement('th');
        th.textContent = gcols[i];
        ghrow.appendChild(th);
    }
    gthead.appendChild(ghrow);
    guideTable.appendChild(gthead);
    var gtbody = document.createElement('tbody');
    gtbody.id = 'cp-guide-body';
    guideTable.appendChild(gtbody);
    guidePanel.appendChild(guideTable);
    container.appendChild(guidePanel);

    // Fact Packs panel
    var fpPanel = document.createElement('div');
    fpPanel.id = 'cp-fact-packs-panel';
    fpPanel.style.display = 'none';

    var fpImportBtn = document.createElement('button');
    fpImportBtn.className = 'btn btn-secondary';
    fpImportBtn.textContent = 'Import Fact Pack';
    fpImportBtn.addEventListener('click', function() { cpShowFactPackImport(); });
    fpPanel.appendChild(fpImportBtn);

    lqAddExportImportBar(fpPanel, function(format) {
        if (format === 'json') {
            lqExportJSON(_cpFactPacks, 'fact-packs.json');
        } else {
            lqExportCSV(_cpFactPacks, ['id','subject_slug','subject_type','title','industry_slugs','active'], 'fact-packs.csv');
        }
    }, function() {
        lqImportJSON(async function(data) {
            var arr = Array.isArray(data) ? data : [data];
            var ok = 0;
            for (var i = 0; i < arr.length; i++) {
                try {
                    await fetch('/api/modules/content-pipeline/fact-packs', { method: 'POST', headers: {'Content-Type':'application/json'}, body: JSON.stringify(arr[i]) });
                    ok++;
                } catch(e) {}
            }
            showToast('Imported ' + ok + ' of ' + arr.length + ' fact packs', 'success');
            cpLoadFactPacks();
        });
    });

    var fpTable = document.createElement('table');
    fpTable.className = 'data-table';
    var fthead = document.createElement('thead');
    var fhrow = document.createElement('tr');
    var fcols = ['ID','Subject','Type','Title','Industries','Sources','Active'];
    for (var i = 0; i < fcols.length; i++) {
        var th = document.createElement('th');
        th.textContent = fcols[i];
        fhrow.appendChild(th);
    }
    fthead.appendChild(fhrow);
    fpTable.appendChild(fthead);
    var ftbody = document.createElement('tbody');
    ftbody.id = 'cp-fp-body';
    fpTable.appendChild(ftbody);
    fpPanel.appendChild(fpTable);
    container.appendChild(fpPanel);

    // Import modal area
    var modalArea = document.createElement('div');
    modalArea.id = 'cp-seo-modal';
    container.appendChild(modalArea);

    cpLoadGuidelines();
    cpLoadFactPacks();
}

function cpLoadGuidelines() {
    fetch('/api/modules/content-pipeline/seo-guidelines')
    .then(function(r) { return r.json(); })
    .then(function(data) {
        var tbody = document.getElementById('cp-guide-body');
        if (!tbody) return;
        tbody.textContent = '';
        var guides = data.guidelines || data || [];
        if (!Array.isArray(guides)) guides = [];
        _cpGuidelines = guides;
        for (var i = 0; i < guides.length; i++) {
            var g = guides[i];
            var tr = document.createElement('tr');
            var cells = [
                (g.id || '').substring(0, 8),
                g.scope || '',
                g.scope_type || '',
                (g.industry_slugs || []).join(', ') || '(all)',
                String((g.term_frequencies || []).length),
                String((g.fact_groups || []).length),
                g.active ? 'Yes' : 'No'
            ];
            for (var c = 0; c < cells.length; c++) {
                var td = document.createElement('td');
                td.textContent = cells[c];
                tr.appendChild(td);
            }
            tbody.appendChild(tr);
        }
    });
}

function cpLoadFactPacks() {
    fetch('/api/modules/content-pipeline/fact-packs')
    .then(function(r) { return r.json(); })
    .then(function(data) {
        var tbody = document.getElementById('cp-fp-body');
        if (!tbody) return;
        tbody.textContent = '';
        var packs = data.fact_packs || data || [];
        if (!Array.isArray(packs)) packs = [];
        _cpFactPacks = packs;
        for (var i = 0; i < packs.length; i++) {
            var f = packs[i];
            var tr = document.createElement('tr');
            var cells = [
                (f.id || '').substring(0, 8),
                f.subject_slug || '',
                f.subject_type || '',
                f.title || '',
                (f.industry_slugs || []).join(', ') || '(all)',
                String((f.sources || []).length),
                f.active ? 'Yes' : 'No'
            ];
            for (var c = 0; c < cells.length; c++) {
                var td = document.createElement('td');
                td.textContent = cells[c];
                tr.appendChild(td);
            }
            tbody.appendChild(tr);
        }
    });
}

function cpShowGuidelineImport() {
    var modal = document.getElementById('cp-seo-modal');
    if (!modal) return;
    modal.textContent = '';

    var h3 = document.createElement('h3');
    h3.textContent = 'Import SEO Guideline';
    modal.appendChild(h3);

    var desc = document.createElement('p');
    desc.textContent = 'Paste a JSON guideline object. Example fields: scope, scope_type, industry_slugs, content_structure, term_frequencies, fact_groups.';
    modal.appendChild(desc);

    var area = document.createElement('textarea');
    area.id = 'cp-guide-import-json';
    area.rows = 12;
    area.placeholder = '{"scope": "topic:german-cockroaches", "scope_type": "topic", "industry_slugs": ["pest-control"], ...}';
    modal.appendChild(area);

    var btn = document.createElement('button');
    btn.className = 'btn btn-primary';
    btn.textContent = 'Import';
    btn.addEventListener('click', function() {
        var jsonStr = document.getElementById('cp-guide-import-json').value;
        try {
            var obj = JSON.parse(jsonStr);
            fetch('/api/modules/content-pipeline/seo-guidelines', {
                method: 'POST',
                headers: {'Content-Type': 'application/json'},
                body: JSON.stringify(obj)
            })
            .then(function(r) { return r.json(); })
            .then(function(result) {
                if (result.error) { alert('Error: ' + result.error); }
                else { modal.textContent = ''; cpLoadGuidelines(); }
            });
        } catch(e) { alert('Invalid JSON: ' + e.message); }
    });
    modal.appendChild(btn);
}

function cpShowFactPackImport() {
    var modal = document.getElementById('cp-seo-modal');
    if (!modal) return;
    modal.textContent = '';

    var h3 = document.createElement('h3');
    h3.textContent = 'Import Fact Pack';
    modal.appendChild(h3);

    var desc = document.createElement('p');
    desc.textContent = 'Paste a JSON fact pack object. Example fields: subject_slug, subject_type, title, industry_slugs, data, sources.';
    modal.appendChild(desc);

    var area = document.createElement('textarea');
    area.id = 'cp-fp-import-json';
    area.rows = 12;
    area.placeholder = '{"subject_slug": "german-cockroach", "subject_type": "pest", "title": "German Cockroach Facts", ...}';
    modal.appendChild(area);

    var btn = document.createElement('button');
    btn.className = 'btn btn-primary';
    btn.textContent = 'Import';
    btn.addEventListener('click', function() {
        var jsonStr = document.getElementById('cp-fp-import-json').value;
        try {
            var obj = JSON.parse(jsonStr);
            fetch('/api/modules/content-pipeline/fact-packs', {
                method: 'POST',
                headers: {'Content-Type': 'application/json'},
                body: JSON.stringify(obj)
            })
            .then(function(r) { return r.json(); })
            .then(function(result) {
                if (result.error) { alert('Error: ' + result.error); }
                else { modal.textContent = ''; cpLoadFactPacks(); }
            });
        } catch(e) { alert('Invalid JSON: ' + e.message); }
    });
    modal.appendChild(btn);
}
"#;

/// Admin CSS for the Content Pipeline module.
pub const ADMIN_CSS: &str = r#"
.cp-generator-form { display: flex; flex-direction: column; gap: 10px; max-width: 600px; margin-bottom: 20px; }
.cp-generator-form label { font-weight: 600; margin-top: 8px; }
.cp-generator-form select,
.cp-generator-form input { padding: 8px; border: 1px solid var(--border); border-radius: 4px; }
.cp-quality-options { display: flex; gap: 20px; }
.cp-quality-options label { font-weight: normal; display: flex; align-items: center; gap: 6px; }
.cp-btn-row { display: flex; gap: 10px; margin-top: 12px; }
.cp-preview { margin-top: 20px; }
.cp-preview pre { background: var(--bg-secondary, #1a1a2e); padding: 16px; border-radius: 4px; overflow-x: auto; white-space: pre-wrap; max-height: 500px; overflow-y: auto; }
.cp-job-detail { margin-top: 20px; }
.cp-content-preview { background: var(--bg-secondary, #1a1a2e); padding: 16px; border-radius: 4px; overflow-x: auto; white-space: pre-wrap; max-height: 400px; overflow-y: auto; }
.cp-tpl-editor { margin-top: 20px; }
.cp-tpl-editor textarea { width: 100%; font-family: monospace; padding: 8px; border: 1px solid var(--border); border-radius: 4px; background: var(--bg-secondary, #1a1a2e); color: inherit; }
.cp-section-prompt { margin-bottom: 8px; }
.cp-tabs { display: flex; gap: 0; margin-bottom: 16px; }
.cp-tab { padding: 8px 20px; border: 1px solid var(--border); background: transparent; color: inherit; cursor: pointer; }
.cp-tab.active { background: var(--accent, #6c63ff); color: #fff; }
.status-badge { padding: 2px 8px; border-radius: 10px; font-size: 0.85em; }
.status-pending { background: #555; color: #fff; }
.status-generating { background: #e67e22; color: #fff; }
.status-review { background: #3498db; color: #fff; }
.status-published { background: #2ecc71; color: #fff; }
.status-failed { background: #e74c3c; color: #fff; }
.error-text { color: #e74c3c; }
#cp-seo-modal { margin-top: 20px; }
#cp-seo-modal textarea { width: 100%; font-family: monospace; padding: 8px; border: 1px solid var(--border); border-radius: 4px; background: var(--bg-secondary, #1a1a2e); color: inherit; }
"#;
