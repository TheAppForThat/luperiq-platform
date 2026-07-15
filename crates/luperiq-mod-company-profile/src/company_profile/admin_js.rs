//! Admin JavaScript for the Company Profile module.
//!
//! View `company-profile`:
//! - If no profile exists, shows a setup wizard (questionnaire form)
//! - If profile exists, shows full profile editor with collapsible sections:
//!   Brand Identity, Company Story, Team, Trust Signals, Contact & Location,
//!   Social Links, Voice & Style
//! - Import panel: buttons for Google Business, Facebook, Website, Conversation
//! - Import history: list of CompanyImportJob entries with status badges
//!
//! Security: Uses DOM methods (createElement/textContent) exclusively. No innerHTML
//! with user data.

pub const COMPANY_PROFILE_ADMIN_JS: &str = r##"
// ── Company Profile view ────────────────────────────────────────────
async function load_company_profile() {
    const main = document.getElementById('adminMain');

    var _role = (window.__CMS && window.__CMS.nexusRole) || '';
    var _isPro = _role === 'central' || _role === 'professional' || _role === 'enterprise';
    var _isStarter = _isPro || _role === 'starter';

    const r = await fetch('/api/modules/company-profile/profile').then(r => r.json());

    if (!r.ok || !r.data) {
        // No profile yet — show setup wizard
        renderSetupWizard(main, _isStarter);
        return;
    }

    const profile = r.data;
    renderProfileEditor(main, profile, _isStarter, _isPro);
}

// ── Setup Wizard (Questionnaire) ─────────────────────────────────────
async function renderSetupWizard(main, _isStarter) {
    const el = document.createElement('div');

    const h = document.createElement('h2');
    h.textContent = 'Company Profile Setup';
    el.appendChild(h);

    const intro = document.createElement('p');
    intro.style.cssText = 'color:var(--text-muted);margin-bottom:20px;';
    intro.textContent = 'Answer the questions below to set up your business identity. You can import from external sources or use AI extraction later.';
    el.appendChild(intro);

    // Fetch questionnaire
    const qr = await fetch('/api/modules/company-profile/questionnaire').then(r => r.json());
    const questions = qr.data || [];

    const form = document.createElement('form');
    form.className = 'questionnaire-form';

    const inputs = {};

    questions.forEach(q => {
        const group = document.createElement('div');
        group.style.cssText = 'margin-bottom:16px;';

        const label = document.createElement('label');
        label.style.cssText = 'display:block;font-weight:600;margin-bottom:4px;';
        label.textContent = q.question;
        if (q.required) {
            const req = document.createElement('span');
            req.style.color = 'var(--error)';
            req.textContent = ' *';
            label.appendChild(req);
        }
        group.appendChild(label);

        let input;
        if (q.input_type === 'textarea') {
            input = document.createElement('textarea');
            input.rows = 3;
        } else if (q.input_type === 'select' && q.options) {
            input = document.createElement('select');
            q.options.forEach(opt => {
                const o = document.createElement('option');
                o.value = opt;
                o.textContent = opt.charAt(0).toUpperCase() + opt.slice(1);
                input.appendChild(o);
            });
        } else {
            input = document.createElement('input');
            input.type = 'text';
        }
        input.style.cssText = 'width:100%;padding:8px;border:1px solid var(--border);border-radius:4px;background:var(--bg-card);color:var(--text);';
        input.name = q.id;
        inputs[q.id] = input;
        group.appendChild(input);
        form.appendChild(group);
    });

    const btnRow = document.createElement('div');
    btnRow.style.cssText = 'display:flex;gap:8px;margin-top:20px;';

    const submitBtn = document.createElement('button');
    submitBtn.type = 'button';
    submitBtn.className = 'btn btn-primary';
    submitBtn.textContent = 'Create Profile';
    submitBtn.onclick = async () => {
        const answers = [];
        questions.forEach(q => {
            const val = inputs[q.id] ? inputs[q.id].value : '';
            if (val.trim()) answers.push({ id: q.id, value: val });
        });

        // Check required
        for (const q of questions) {
            if (q.required && (!inputs[q.id] || !inputs[q.id].value.trim())) {
                alert('Please fill in: ' + q.question);
                return;
            }
        }

        submitBtn.disabled = true;
        submitBtn.textContent = 'Creating...';

        const sr = await fetch('/api/modules/company-profile/questionnaire', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify(answers),
        }).then(r => r.json());

        if (sr.ok) {
            // Auto-apply the import immediately for the wizard flow
            if (sr.data && sr.data.import_id) {
                await fetch('/api/modules/company-profile/imports/' + sr.data.import_id + '/apply', { method: 'POST' });
            }
            load_company_profile();
        } else {
            alert(sr.message);
            submitBtn.disabled = false;
            submitBtn.textContent = 'Create Profile';
        }
    };
    btnRow.appendChild(submitBtn);
    form.appendChild(btnRow);
    el.appendChild(form);

    main.textContent = '';
    main.appendChild(el);
}

// ── Profile Editor ───────────────────────────────────────────────────
function companyProfileAreaLabel(profile) {
    const slug = String((profile && profile.industry_slug) || '').toLowerCase();
    if (slug === 'bakery') return 'Pickup / Catering Area Notes';
    if (slug === 'coffee' || slug === 'coffee-shop') return 'Pickup / Delivery Area Notes';
    if (slug === 'restaurant') return 'Pickup / Delivery Area Notes';
    if (slug === 'artisan-market') return 'Market / Pickup Area Notes';
    if (slug === 'app-publisher') return 'Audience / Platform Notes';
    if (slug === 'creator') return 'Audience / Collaboration Notes';
    if (slug === 'blog') return 'Reader / Topic Coverage Notes';
    if (slug === 'medical' || slug === 'medical-office' || slug === 'healthcare') return 'Practice Location Notes';
    if (slug === 'salon' || slug === 'salon-barbershop') return 'Appointment Location Notes';
    return 'Service Area Description';
}

function companyProfileContactSectionTitle(profile) {
    const slug = String((profile && profile.industry_slug) || '').toLowerCase();
    if (slug === 'creator') return 'Contact & Audience';
    if (slug === 'blog') return 'Contact & Publication';
    if (slug === 'app-publisher') return 'Contact & Product Reach';
    return 'Contact & Location';
}

function companyProfileSlugLabel(profile) {
    const slug = String((profile && profile.industry_slug) || '').toLowerCase();
    if (slug === 'creator') return 'Topic / Offer Slugs (comma-separated)';
    if (slug === 'blog') return 'Topic Slugs (comma-separated)';
    if (slug === 'app-publisher') return 'Product / Platform Slugs (comma-separated)';
    return 'Location Slugs (comma-separated)';
}

async function renderProfileEditor(main, profile, _isStarter, _isPro) {
    const el = document.createElement('div');

    // ── Pricing card ──────────────────────────────────────────────────
    var _pc = lqModulePricingCard({ name: 'Company Profile', monthly: 9, annual: 89, lifetime: 249, tier: 'starter', deps: [], slug: 'company-profile' });
    if (_pc) el.appendChild(_pc);

    // ── Header ────────────────────────────────────────────────────────
    const toolbar = document.createElement('div');
    toolbar.className = 'toolbar';
    const h = document.createElement('h2');
    h.textContent = 'Company Profile';
    if (!_isStarter) {
        var _tierBadge = document.createElement('span');
        _tierBadge.className = 'status-badge status-published';
        _tierBadge.style.cssText = 'margin-left:8px;background:#f59e0b;color:#000;font-size:11px;';
        _tierBadge.textContent = 'STARTER';
        h.appendChild(_tierBadge);
    }
    toolbar.appendChild(h);

    const btnGroup = document.createElement('div');
    btnGroup.style.cssText = 'display:flex;gap:8px;';

    const importBtn = document.createElement('button');
    importBtn.className = 'btn';
    importBtn.textContent = 'Import Data';
    if (!_isStarter) { importBtn.disabled = true; importBtn.title = 'Upgrade to Starter to import data'; }
    importBtn.onclick = () => showImportPanel(el);
    btnGroup.appendChild(importBtn);

    const saveBtn = document.createElement('button');
    saveBtn.className = 'btn btn-primary';
    saveBtn.textContent = 'Save Changes';
    if (!_isStarter) { saveBtn.disabled = true; saveBtn.title = 'Upgrade to Starter to edit'; }
    saveBtn.onclick = async () => {
        saveBtn.disabled = true;
        saveBtn.textContent = 'Saving...';
        const updated = collectProfileData(el, profile);
        const sr = await fetch('/api/modules/company-profile/profile', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify(updated),
        }).then(r => r.json());
        if (sr.ok) {
            load_company_profile();
        } else {
            alert(sr.message);
            saveBtn.disabled = false;
            saveBtn.textContent = 'Save Changes';
        }
    };
    btnGroup.appendChild(saveBtn);
    toolbar.appendChild(btnGroup);
    el.appendChild(toolbar);

    lqAddExportImportBar(el, function(format) {
        if (format === 'json') {
            lqExportJSON([profile], 'company-profile.json');
        } else {
            lqExportJSON([profile], 'company-profile.json');
        }
    }, null);

    // ── Sections ──────────────────────────────────────────────────────

    // Brand Identity
    const brandSection = createSection('Brand Identity', [
        fieldRow('Business Name', 'text', 'cp_name', profile.name, true),
        fieldRow('Legal Name', 'text', 'cp_legal_name', profile.legal_name || ''),
        fieldRow('Tagline', 'text', 'cp_tagline', profile.tagline),
        fieldRow('Industry Slug', 'text', 'cp_industry_slug', profile.industry_slug),
        fieldRow('Logo URL', 'text', 'cp_logo_url', profile.logo_url || ''),
        fieldRow('Favicon URL', 'text', 'cp_favicon_url', profile.favicon_url || ''),
        colorRow('Primary Color', 'cp_color_primary', profile.brand_colors?.primary || '#1a73e8'),
        colorRow('Secondary Color', 'cp_color_secondary', profile.brand_colors?.secondary || '#34a853'),
        colorRow('Accent Color', 'cp_color_accent', profile.brand_colors?.accent || '#ea4335'),
    ]);
    el.appendChild(brandSection);

    // Add upload buttons next to Logo URL and Favicon URL
    (function(container) {
        function addImageUploadBtn(inputId) {
            var inp = container.querySelector('#' + inputId);
            if (!inp) return;
            var btn = document.createElement('button');
            btn.type = 'button';
            btn.textContent = '\u2191 Upload';
            btn.style.cssText = 'margin-left:8px;padding:4px 10px;background:#334155;color:#e2e8f0;border:1px solid #475569;border-radius:6px;font-size:12px;cursor:pointer;vertical-align:middle;white-space:nowrap;';
            var fi = document.createElement('input');
            fi.type = 'file';
            fi.accept = 'image/png,image/jpeg,image/svg+xml,image/webp,image/gif';
            fi.style.display = 'none';
            fi.addEventListener('change', function() {
                var file = fi.files[0];
                if (!file) return;
                var origText = btn.textContent;
                btn.textContent = 'Uploading...';
                btn.disabled = true;
                var fd = new FormData();
                fd.append('file', file);
                var headers = {};
                try { headers = csrfHeaders(); } catch(e) {}
                fetch('/api/media/upload', { method: 'POST', body: fd, credentials: 'include', headers: headers })
                    .then(function(r) { return r.json(); })
                    .then(function(data) {
                        if (data.ok && data.data && data.data.url) {
                            inp.value = data.data.url;
                            btn.textContent = '\u2713 Uploaded';
                            setTimeout(function() { btn.textContent = origText; btn.disabled = false; }, 2500);
                        } else {
                            btn.textContent = '\u2717 Failed';
                            setTimeout(function() { btn.textContent = origText; btn.disabled = false; }, 2500);
                        }
                    })
                    .catch(function() {
                        btn.textContent = '\u2717 Error';
                        setTimeout(function() { btn.textContent = origText; btn.disabled = false; }, 2500);
                    });
            });
            btn.addEventListener('click', function() { fi.click(); });
            inp.parentNode.style.display = 'flex';
            inp.parentNode.style.alignItems = 'center';
            inp.parentNode.style.gap = '8px';
            inp.parentNode.appendChild(btn);
            inp.parentNode.appendChild(fi);
        }
        addImageUploadBtn('cp_logo_url');
        addImageUploadBtn('cp_favicon_url');
    })(el);

    // Company Story
    const storySection = createSection('Company Story', [
        textareaRow('Story / Mission', 'cp_story', profile.story, 5),
        textareaRow('Service Philosophy', 'cp_service_philosophy', profile.service_philosophy, 3),
        fieldRow('Years in Business', 'number', 'cp_years', profile.years_in_business || ''),
    ]);
    // AI Bio button
    if (typeof LiqAI !== 'undefined') {
        var _aiBioRow = document.createElement('div');
        _aiBioRow.style.cssText = 'padding:0 16px 12px;';
        var _aiBioBtn = LiqAI.button({
            label: 'AI Generate Bio',
            feature: 'company_ai_bio',
            credits: 2,
            tier: 'free',
            getInput: function() {
                var name = document.getElementById('cp_name') ? document.getElementById('cp_name').value : '';
                var tagline = document.getElementById('cp_tagline') ? document.getElementById('cp_tagline').value : '';
                var industry = document.getElementById('cp_industry_slug') ? document.getElementById('cp_industry_slug').value : '';
                var years = document.getElementById('cp_years') ? document.getElementById('cp_years').value : '';
                var philosophy = document.getElementById('cp_service_philosophy') ? document.getElementById('cp_service_philosophy').value : '';
                if (!name.trim()) { showToast('Add a business name first', 'error'); return ''; }
                return 'Business: ' + name + '\nTagline: ' + tagline + '\nIndustry: ' + industry + '\nYears: ' + years + '\nPhilosophy: ' + philosophy;
            },
            onResult: function(result) {
                if (typeof result === 'string') {
                    var ta = document.getElementById('cp_story');
                    if (ta) { ta.value = result; showToast('Bio generated by AI', 'success'); }
                }
            },
        });
        if (_aiBioBtn) _aiBioRow.appendChild(_aiBioBtn);
        storySection.querySelector('.section-body').appendChild(_aiBioRow);
    }
    el.appendChild(storySection);

    // Team
    const teamSection = createSection('Team', []);
    const teamContainer = document.createElement('div');
    teamContainer.id = 'cp_team_container';
    renderTeamMembers(teamContainer, profile.team_bios || []);
    teamSection.querySelector('.section-body').appendChild(teamContainer);

    const addTeamBtn = document.createElement('button');
    addTeamBtn.className = 'btn';
    addTeamBtn.textContent = '+ Add Team Member';
    addTeamBtn.style.marginTop = '8px';
    addTeamBtn.onclick = () => {
        const members = collectTeamMembers(teamContainer);
        members.push({ name: '', title: '', bio: '', photo_url: null });
        renderTeamMembers(teamContainer, members);
    };
    teamSection.querySelector('.section-body').appendChild(addTeamBtn);
    el.appendChild(teamSection);

    // Trust Signals
    const trustSection = createSection('Trust Signals', [
        textareaRow('Certifications (one per line)', 'cp_certifications', (profile.certifications || []).join('\n'), 3),
        textareaRow('License Numbers (one per line)', 'cp_license_numbers', (profile.license_numbers || []).join('\n'), 2),
        textareaRow('Unique Selling Points (one per line)', 'cp_usps', (profile.unique_selling_points || []).join('\n'), 3),
    ]);
    // AI USP button
    if (typeof LiqAI !== 'undefined') {
        var _aiUspRow = document.createElement('div');
        _aiUspRow.style.cssText = 'margin-bottom:12px;';
        var _aiUspBtn = LiqAI.button({
            label: 'AI Generate USPs',
            feature: 'company_ai_usp',
            credits: 2,
            tier: 'free',
            getInput: function() {
                var name = document.getElementById('cp_name') ? document.getElementById('cp_name').value : '';
                var industry = document.getElementById('cp_industry_slug') ? document.getElementById('cp_industry_slug').value : '';
                var certs = document.getElementById('cp_certifications') ? document.getElementById('cp_certifications').value : '';
                var story = document.getElementById('cp_story') ? document.getElementById('cp_story').value : '';
                if (!name.trim()) { showToast('Add a business name first', 'error'); return ''; }
                return 'Business: ' + name + '\nIndustry: ' + industry + '\nCertifications: ' + certs + '\nStory: ' + story;
            },
            onResult: function(result) {
                if (Array.isArray(result)) {
                    var ta = document.getElementById('cp_usps');
                    if (ta) { ta.value = result.join('\n'); showToast('USPs generated by AI', 'success'); }
                }
            },
        });
        if (_aiUspBtn) _aiUspRow.appendChild(_aiUspBtn);
        trustSection.querySelector('.section-body').appendChild(_aiUspRow);
    }

    const reviewContainer = document.createElement('div');
    reviewContainer.id = 'cp_reviews_container';
    renderReviewHighlights(reviewContainer, profile.review_highlights || []);
    trustSection.querySelector('.section-body').appendChild(reviewContainer);

    const addReviewBtn = document.createElement('button');
    addReviewBtn.className = 'btn';
    addReviewBtn.textContent = '+ Add Review Highlight';
    addReviewBtn.style.marginTop = '8px';
    addReviewBtn.onclick = () => {
        const reviews = collectReviews(reviewContainer);
        reviews.push({ source: 'google', rating: 5.0, text: '', author: '' });
        renderReviewHighlights(reviewContainer, reviews);
    };
    trustSection.querySelector('.section-body').appendChild(addReviewBtn);
    el.appendChild(trustSection);

    // Contact & Location
    const contactSection = createSection(companyProfileContactSectionTitle(profile), [
        fieldRow('Phone', 'text', 'cp_phone', profile.phone),
        fieldRow('Email', 'email', 'cp_email', profile.email),
        fieldRow('Address', 'text', 'cp_address', profile.address),
        fieldRow('City', 'text', 'cp_city', profile.city),
        fieldRow('State', 'text', 'cp_state', profile.state),
        fieldRow('ZIP', 'text', 'cp_zip', profile.zip),
        textareaRow(companyProfileAreaLabel(profile), 'cp_service_area', profile.service_area_description, 2),
        fieldRow(companyProfileSlugLabel(profile), 'text', 'cp_location_slugs', (profile.location_slugs || []).join(', ')),
    ]);
    el.appendChild(contactSection);

    // Social Links
    const socialSection = createSection('Social Links', [
        fieldRow('Google Business', 'url', 'cp_social_google', profile.social_links?.google_business || ''),
        fieldRow('Facebook', 'url', 'cp_social_facebook', profile.social_links?.facebook || ''),
        fieldRow('Instagram', 'url', 'cp_social_instagram', profile.social_links?.instagram || ''),
        fieldRow('Twitter / X', 'url', 'cp_social_twitter', profile.social_links?.twitter || ''),
        fieldRow('YouTube', 'url', 'cp_social_youtube', profile.social_links?.youtube || ''),
        fieldRow('LinkedIn', 'url', 'cp_social_linkedin', profile.social_links?.linkedin || ''),
        fieldRow('Yelp', 'url', 'cp_social_yelp', profile.social_links?.yelp || ''),
        fieldRow('Nextdoor', 'url', 'cp_social_nextdoor', profile.social_links?.nextdoor || ''),
    ]);
    el.appendChild(socialSection);

    // Voice & Style
    const voiceSection = createSection('Voice & Style', [
        selectRow('Tone', 'cp_tone', profile.tone || 'professional', ['professional', 'friendly', 'casual', 'authoritative', 'playful']),
        textareaRow('Voice Notes (one per line)', 'cp_voice_notes', (profile.voice_notes || []).join('\n'), 3),
        fieldRow('Owner Name', 'text', 'cp_owner_name', profile.owner_name || ''),
        fieldRow('Owner Title', 'text', 'cp_owner_title', profile.owner_title || ''),
    ]);
    el.appendChild(voiceSection);

    // ── Import History ────────────────────────────────────────────────
    const importsSection = createSection('Import History', []);
    const importsContainer = document.createElement('div');
    importsContainer.id = 'cp_imports_container';
    await renderImportHistory(importsContainer);
    importsSection.querySelector('.section-body').appendChild(importsContainer);
    el.appendChild(importsSection);

    main.textContent = '';
    main.appendChild(el);
}

// ── Section helpers ──────────────────────────────────────────────────

function createSection(title, fields) {
    const section = document.createElement('div');
    section.style.cssText = 'margin-bottom:16px;border:1px solid var(--border);border-radius:8px;overflow:hidden;';

    const header = document.createElement('div');
    header.style.cssText = 'padding:12px 16px;background:var(--bg-card);cursor:pointer;display:flex;justify-content:space-between;align-items:center;border-bottom:1px solid var(--border);';
    const titleEl = document.createElement('strong');
    titleEl.textContent = title;
    header.appendChild(titleEl);

    const arrow = document.createElement('span');
    arrow.textContent = '\u25BC';
    arrow.style.cssText = 'transition:transform 0.2s;font-size:12px;';
    header.appendChild(arrow);

    const body = document.createElement('div');
    body.className = 'section-body';
    body.style.cssText = 'padding:16px;';

    header.onclick = () => {
        if (body.style.display === 'none') {
            body.style.display = 'block';
            arrow.style.transform = 'rotate(0deg)';
        } else {
            body.style.display = 'none';
            arrow.style.transform = 'rotate(-90deg)';
        }
    };

    fields.forEach(f => body.appendChild(f));
    section.appendChild(header);
    section.appendChild(body);
    return section;
}

function fieldRow(label, type, id, value, required) {
    const row = document.createElement('div');
    row.style.cssText = 'margin-bottom:12px;';
    const lbl = document.createElement('label');
    lbl.style.cssText = 'display:block;font-weight:500;margin-bottom:4px;font-size:13px;';
    lbl.textContent = label;
    row.appendChild(lbl);
    const inp = document.createElement('input');
    inp.type = type;
    inp.id = id;
    inp.value = value || '';
    if (required) inp.required = true;
    inp.style.cssText = 'width:100%;padding:8px;border:1px solid var(--border);border-radius:4px;background:var(--bg-card);color:var(--text);box-sizing:border-box;';
    row.appendChild(inp);
    return row;
}

function textareaRow(label, id, value, rows) {
    const row = document.createElement('div');
    row.style.cssText = 'margin-bottom:12px;';
    const lbl = document.createElement('label');
    lbl.style.cssText = 'display:block;font-weight:500;margin-bottom:4px;font-size:13px;';
    lbl.textContent = label;
    row.appendChild(lbl);
    const ta = document.createElement('textarea');
    ta.id = id;
    ta.rows = rows || 3;
    ta.value = value || '';
    ta.style.cssText = 'width:100%;padding:8px;border:1px solid var(--border);border-radius:4px;background:var(--bg-card);color:var(--text);box-sizing:border-box;font-family:inherit;';
    row.appendChild(ta);
    return row;
}

function selectRow(label, id, value, options) {
    const row = document.createElement('div');
    row.style.cssText = 'margin-bottom:12px;';
    const lbl = document.createElement('label');
    lbl.style.cssText = 'display:block;font-weight:500;margin-bottom:4px;font-size:13px;';
    lbl.textContent = label;
    row.appendChild(lbl);
    const sel = document.createElement('select');
    sel.id = id;
    sel.style.cssText = 'width:100%;padding:8px;border:1px solid var(--border);border-radius:4px;background:var(--bg-card);color:var(--text);';
    options.forEach(opt => {
        const o = document.createElement('option');
        o.value = opt;
        o.textContent = opt.charAt(0).toUpperCase() + opt.slice(1);
        if (opt === value) o.selected = true;
        sel.appendChild(o);
    });
    row.appendChild(sel);
    return row;
}

function colorRow(label, id, value) {
    const row = document.createElement('div');
    row.style.cssText = 'margin-bottom:12px;display:flex;align-items:center;gap:8px;';
    const lbl = document.createElement('label');
    lbl.style.cssText = 'font-weight:500;font-size:13px;min-width:140px;';
    lbl.textContent = label;
    row.appendChild(lbl);
    const colorInp = document.createElement('input');
    colorInp.type = 'color';
    colorInp.id = id;
    colorInp.value = value || '#000000';
    colorInp.style.cssText = 'width:40px;height:32px;border:1px solid var(--border);border-radius:4px;cursor:pointer;';
    row.appendChild(colorInp);
    const hexInp = document.createElement('input');
    hexInp.type = 'text';
    hexInp.value = value || '#000000';
    hexInp.style.cssText = 'width:100px;padding:6px;border:1px solid var(--border);border-radius:4px;background:var(--bg-card);color:var(--text);font-family:monospace;';
    hexInp.oninput = () => { colorInp.value = hexInp.value; };
    colorInp.oninput = () => { hexInp.value = colorInp.value; };
    hexInp.id = id + '_hex';
    row.appendChild(hexInp);
    return row;
}

// ── Team member rendering ────────────────────────────────────────────
function renderTeamMembers(container, members) {
    container.textContent = '';
    members.forEach((m, idx) => {
        const card = document.createElement('div');
        card.style.cssText = 'border:1px solid var(--border);border-radius:6px;padding:12px;margin-bottom:8px;';

        const nameRow = document.createElement('div');
        nameRow.style.cssText = 'display:flex;gap:8px;margin-bottom:8px;';

        const nameInp = document.createElement('input');
        nameInp.type = 'text';
        nameInp.placeholder = 'Name';
        nameInp.value = m.name || '';
        nameInp.className = 'team-name';
        nameInp.style.cssText = 'flex:1;padding:6px;border:1px solid var(--border);border-radius:4px;background:var(--bg-card);color:var(--text);';

        const titleInp = document.createElement('input');
        titleInp.type = 'text';
        titleInp.placeholder = 'Title';
        titleInp.value = m.title || '';
        titleInp.className = 'team-title';
        titleInp.style.cssText = 'flex:1;padding:6px;border:1px solid var(--border);border-radius:4px;background:var(--bg-card);color:var(--text);';

        const removeBtn = document.createElement('button');
        removeBtn.className = 'btn';
        removeBtn.textContent = 'Remove';
        removeBtn.style.cssText = 'color:var(--error);';
        removeBtn.onclick = () => {
            const all = collectTeamMembers(container);
            all.splice(idx, 1);
            renderTeamMembers(container, all);
        };

        nameRow.appendChild(nameInp);
        nameRow.appendChild(titleInp);
        nameRow.appendChild(removeBtn);
        card.appendChild(nameRow);

        const bioTa = document.createElement('textarea');
        bioTa.placeholder = 'Short bio...';
        bioTa.value = m.bio || '';
        bioTa.rows = 2;
        bioTa.className = 'team-bio';
        bioTa.style.cssText = 'width:100%;padding:6px;border:1px solid var(--border);border-radius:4px;background:var(--bg-card);color:var(--text);box-sizing:border-box;margin-bottom:4px;';
        card.appendChild(bioTa);

        const photoInp = document.createElement('input');
        photoInp.type = 'text';
        photoInp.placeholder = 'Photo URL (optional)';
        photoInp.value = m.photo_url || '';
        photoInp.className = 'team-photo';
        photoInp.style.cssText = 'width:100%;padding:6px;border:1px solid var(--border);border-radius:4px;background:var(--bg-card);color:var(--text);box-sizing:border-box;';
        card.appendChild(photoInp);

        container.appendChild(card);
    });
}

function collectTeamMembers(container) {
    const cards = container.children;
    const members = [];
    for (let i = 0; i < cards.length; i++) {
        const card = cards[i];
        const name = card.querySelector('.team-name');
        const title = card.querySelector('.team-title');
        const bio = card.querySelector('.team-bio');
        const photo = card.querySelector('.team-photo');
        if (name) {
            members.push({
                name: name.value,
                title: title ? title.value : '',
                bio: bio ? bio.value : '',
                photo_url: photo && photo.value ? photo.value : null,
            });
        }
    }
    return members;
}

// ── Review highlights rendering ──────────────────────────────────────
function renderReviewHighlights(container, reviews) {
    container.textContent = '';
    const label = document.createElement('label');
    label.style.cssText = 'display:block;font-weight:500;margin-bottom:8px;font-size:13px;margin-top:12px;';
    label.textContent = 'Review Highlights';
    container.appendChild(label);

    reviews.forEach((r, idx) => {
        const card = document.createElement('div');
        card.style.cssText = 'border:1px solid var(--border);border-radius:6px;padding:12px;margin-bottom:8px;';

        const topRow = document.createElement('div');
        topRow.style.cssText = 'display:flex;gap:8px;margin-bottom:8px;';

        const sourceInp = document.createElement('select');
        sourceInp.className = 'review-source';
        sourceInp.style.cssText = 'padding:6px;border:1px solid var(--border);border-radius:4px;background:var(--bg-card);color:var(--text);';
        ['google', 'yelp', 'facebook', 'nextdoor', 'other'].forEach(s => {
            const o = document.createElement('option');
            o.value = s;
            o.textContent = s.charAt(0).toUpperCase() + s.slice(1);
            if (s === r.source) o.selected = true;
            sourceInp.appendChild(o);
        });

        const ratingInp = document.createElement('input');
        ratingInp.type = 'number';
        ratingInp.min = '1';
        ratingInp.max = '5';
        ratingInp.step = '0.1';
        ratingInp.value = r.rating || 5;
        ratingInp.className = 'review-rating';
        ratingInp.style.cssText = 'width:70px;padding:6px;border:1px solid var(--border);border-radius:4px;background:var(--bg-card);color:var(--text);';

        const authorInp = document.createElement('input');
        authorInp.type = 'text';
        authorInp.placeholder = 'Reviewer name';
        authorInp.value = r.author || '';
        authorInp.className = 'review-author';
        authorInp.style.cssText = 'flex:1;padding:6px;border:1px solid var(--border);border-radius:4px;background:var(--bg-card);color:var(--text);';

        const removeBtn = document.createElement('button');
        removeBtn.className = 'btn';
        removeBtn.textContent = 'Remove';
        removeBtn.style.cssText = 'color:var(--error);';
        removeBtn.onclick = () => {
            const all = collectReviews(container);
            all.splice(idx, 1);
            renderReviewHighlights(container, all);
        };

        topRow.appendChild(sourceInp);
        topRow.appendChild(ratingInp);
        topRow.appendChild(authorInp);
        topRow.appendChild(removeBtn);
        card.appendChild(topRow);

        const textTa = document.createElement('textarea');
        textTa.placeholder = 'Review text...';
        textTa.value = r.text || '';
        textTa.rows = 2;
        textTa.className = 'review-text';
        textTa.style.cssText = 'width:100%;padding:6px;border:1px solid var(--border);border-radius:4px;background:var(--bg-card);color:var(--text);box-sizing:border-box;';
        card.appendChild(textTa);

        container.appendChild(card);
    });
}

function collectReviews(container) {
    const reviews = [];
    const cards = container.querySelectorAll('div[style*="border-radius:6px"]');
    cards.forEach(card => {
        const source = card.querySelector('.review-source');
        const rating = card.querySelector('.review-rating');
        const author = card.querySelector('.review-author');
        const text = card.querySelector('.review-text');
        if (source) {
            reviews.push({
                source: source.value,
                rating: parseFloat(rating?.value || '5'),
                text: text?.value || '',
                author: author?.value || '',
            });
        }
    });
    return reviews;
}

// ── Collect all profile data from editor ─────────────────────────────
function collectProfileData(container, original) {
    const val = id => {
        const el = document.getElementById(id);
        return el ? el.value : '';
    };

    const lines = id => {
        const v = val(id);
        return v ? v.split('\n').map(l => l.trim()).filter(l => l) : [];
    };

    const teamContainer = container.querySelector('#cp_team_container');
    const reviewContainer = container.querySelector('#cp_reviews_container');

    return {
        name: val('cp_name'),
        legal_name: val('cp_legal_name') || null,
        industry_slug: val('cp_industry_slug'),
        tagline: val('cp_tagline'),
        story: val('cp_story'),
        tone: val('cp_tone'),
        voice_notes: lines('cp_voice_notes'),
        brand_colors: {
            primary: val('cp_color_primary'),
            secondary: val('cp_color_secondary'),
            accent: val('cp_color_accent'),
        },
        logo_url: val('cp_logo_url') || null,
        favicon_url: val('cp_favicon_url') || null,
        owner_name: val('cp_owner_name') || null,
        owner_title: val('cp_owner_title') || null,
        team_bios: teamContainer ? collectTeamMembers(teamContainer) : (original.team_bios || []),
        certifications: lines('cp_certifications'),
        license_numbers: lines('cp_license_numbers'),
        years_in_business: parseInt(val('cp_years')) || null,
        service_philosophy: val('cp_service_philosophy'),
        unique_selling_points: lines('cp_usps'),
        review_highlights: reviewContainer ? collectReviews(reviewContainer) : (original.review_highlights || []),
        social_links: {
            google_business: val('cp_social_google') || null,
            facebook: val('cp_social_facebook') || null,
            instagram: val('cp_social_instagram') || null,
            twitter: val('cp_social_twitter') || null,
            youtube: val('cp_social_youtube') || null,
            linkedin: val('cp_social_linkedin') || null,
            yelp: val('cp_social_yelp') || null,
            nextdoor: val('cp_social_nextdoor') || null,
        },
        phone: val('cp_phone'),
        email: val('cp_email'),
        address: val('cp_address'),
        city: val('cp_city'),
        state: val('cp_state'),
        zip: val('cp_zip'),
        service_area_description: val('cp_service_area'),
        location_slugs: val('cp_location_slugs').split(',').map(s => s.trim()).filter(s => s),
    };
}

// ── Import Panel ─────────────────────────────────────────────────────
function showImportPanel(parentEl) {
    // Remove existing panel if present
    const existing = document.getElementById('cp_import_panel');
    if (existing) { existing.remove(); return; }

    const panel = document.createElement('div');
    panel.id = 'cp_import_panel';
    panel.style.cssText = 'border:2px solid var(--accent);border-radius:8px;padding:16px;margin:12px 0;background:var(--bg-card);';

    const title = document.createElement('h3');
    title.textContent = 'Import Data';
    title.style.marginTop = '0';
    panel.appendChild(title);

    const desc = document.createElement('p');
    desc.style.color = 'var(--text-muted)';
    desc.textContent = 'Import business data from external sources. Extracted data will be shown for review before applying to your profile.';
    panel.appendChild(desc);

    const btnGrid = document.createElement('div');
    btnGrid.style.cssText = 'display:grid;grid-template-columns:repeat(auto-fill,minmax(200px,1fr));gap:8px;margin-bottom:16px;';

    // Google Business import
    const gbBtn = createImportButton('Google Business', 'Enter your Google Business Profile URL:', '/api/modules/company-profile/import/google');
    btnGrid.appendChild(gbBtn);

    // Facebook import
    const fbBtn = createImportButton('Facebook Page', 'Enter your Facebook Business page URL:', '/api/modules/company-profile/import/facebook');
    btnGrid.appendChild(fbBtn);

    // Website import
    const webBtn = createImportButton('Website', 'Enter your business website URL:', '/api/modules/company-profile/import/website');
    btnGrid.appendChild(webBtn);

    panel.appendChild(btnGrid);

    // AI Conversation extraction
    const aiTitle = document.createElement('h4');
    aiTitle.textContent = 'AI Conversation Extraction';
    panel.appendChild(aiTitle);

    const aiDesc = document.createElement('p');
    aiDesc.style.cssText = 'font-size:13px;color:var(--text-muted);';
    aiDesc.textContent = 'Paste a transcript of a conversation with the business owner, and AI will extract profile information automatically.';
    panel.appendChild(aiDesc);

    const aiTa = document.createElement('textarea');
    aiTa.rows = 6;
    aiTa.placeholder = 'Paste conversation text here...';
    aiTa.style.cssText = 'width:100%;padding:8px;border:1px solid var(--border);border-radius:4px;background:var(--bg);color:var(--text);box-sizing:border-box;margin-bottom:8px;';
    panel.appendChild(aiTa);

    const aiBtn = document.createElement('button');
    aiBtn.className = 'btn btn-primary';
    aiBtn.textContent = 'Extract with AI';
    aiBtn.onclick = async () => {
        const text = aiTa.value.trim();
        if (!text) { alert('Please paste conversation text first.'); return; }
        aiBtn.disabled = true;
        aiBtn.textContent = 'Extracting...';
        const r = await fetch('/api/modules/company-profile/extract-conversation', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ transcript: text }),
        }).then(r => r.json());
        aiBtn.disabled = false;
        aiBtn.textContent = 'Extract with AI';
        if (r.ok) {
            alert('Data extracted! Check the Import History section to review and apply.');
            const ic = document.getElementById('cp_imports_container');
            if (ic) await renderImportHistory(ic);
        } else {
            alert('Error: ' + r.message);
        }
    };
    panel.appendChild(aiBtn);

    // Close button
    const closeBtn = document.createElement('button');
    closeBtn.className = 'btn';
    closeBtn.textContent = 'Close';
    closeBtn.style.cssText = 'margin-left:8px;';
    closeBtn.onclick = () => panel.remove();
    panel.appendChild(closeBtn);

    // Insert after toolbar
    const toolbar = parentEl.querySelector('.toolbar');
    if (toolbar && toolbar.nextSibling) {
        parentEl.insertBefore(panel, toolbar.nextSibling);
    } else {
        parentEl.appendChild(panel);
    }
}

function createImportButton(label, prompt, endpoint) {
    const btn = document.createElement('button');
    btn.className = 'btn';
    btn.textContent = 'Import from ' + label;
    btn.onclick = async () => {
        const url = window.prompt(prompt);
        if (!url) return;
        btn.disabled = true;
        btn.textContent = 'Importing...';
        const r = await fetch(endpoint, {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ url: url }),
        }).then(r => r.json());
        btn.disabled = false;
        btn.textContent = 'Import from ' + label;
        if (r.ok) {
            alert('Data extracted! Check the Import History section to review and apply.');
            const ic = document.getElementById('cp_imports_container');
            if (ic) await renderImportHistory(ic);
        } else {
            alert('Error: ' + r.message);
        }
    };
    return btn;
}

// ── Import History ───────────────────────────────────────────────────
async function renderImportHistory(container) {
    container.textContent = '';
    const r = await fetch('/api/modules/company-profile/imports').then(r => r.json());
    const imports = r.data || [];

    if (imports.length === 0) {
        const empty = document.createElement('p');
        empty.style.color = 'var(--text-muted)';
        empty.textContent = 'No import jobs yet.';
        container.appendChild(empty);
        return;
    }

    const table = document.createElement('table');
    table.style.cssText = 'width:100%;border-collapse:collapse;';
    const hdr = document.createElement('tr');
    ['Source', 'Status', 'Created', 'Actions'].forEach(col => {
        const th = document.createElement('th');
        th.textContent = col;
        th.style.cssText = 'text-align:left;padding:8px;border-bottom:1px solid var(--border);';
        hdr.appendChild(th);
    });
    table.appendChild(hdr);

    imports.forEach(imp => {
        const tr = document.createElement('tr');

        const srcTd = document.createElement('td');
        srcTd.style.padding = '8px';
        srcTd.textContent = imp.source + (imp.source_url ? ' (' + imp.source_url.substring(0, 40) + '...)' : '');
        tr.appendChild(srcTd);

        const statusTd = document.createElement('td');
        statusTd.style.padding = '8px';
        const badge = document.createElement('span');
        badge.style.cssText = 'padding:2px 8px;border-radius:12px;font-size:12px;font-weight:600;';
        const colors = { pending: '#888', extracting: '#e8a800', review: '#1a73e8', applied: '#34a853', failed: '#ea4335' };
        badge.style.background = (colors[imp.status] || '#888') + '22';
        badge.style.color = colors[imp.status] || '#888';
        badge.textContent = imp.status;
        statusTd.appendChild(badge);
        tr.appendChild(statusTd);

        const dateTd = document.createElement('td');
        dateTd.style.padding = '8px';
        dateTd.textContent = imp.created_at ? new Date(imp.created_at * 1000).toLocaleString() : '';
        tr.appendChild(dateTd);

        const actionTd = document.createElement('td');
        actionTd.style.padding = '8px';
        if (imp.status === 'review') {
            const previewBtn = document.createElement('button');
            previewBtn.className = 'btn';
            previewBtn.textContent = 'Preview';
            previewBtn.style.cssText = 'margin-right:4px;font-size:12px;padding:4px 8px;';
            previewBtn.onclick = () => {
                alert('Extracted data:\n\n' + JSON.stringify(imp.extracted_data, null, 2));
            };
            actionTd.appendChild(previewBtn);

            const applyBtn = document.createElement('button');
            applyBtn.className = 'btn btn-primary';
            applyBtn.textContent = 'Apply';
            applyBtn.style.cssText = 'font-size:12px;padding:4px 8px;';
            applyBtn.onclick = async () => {
                if (!confirm('Apply this import data to your company profile? Existing fields will be overwritten.')) return;
                applyBtn.disabled = true;
                const ar = await fetch('/api/modules/company-profile/imports/' + imp.id + '/apply', { method: 'POST' }).then(r => r.json());
                if (ar.ok) {
                    load_company_profile();
                } else {
                    alert(ar.message);
                    applyBtn.disabled = false;
                }
            };
            actionTd.appendChild(applyBtn);
        } else if (imp.status === 'failed' && imp.error) {
            const errSpan = document.createElement('span');
            errSpan.style.cssText = 'color:var(--error);font-size:12px;';
            errSpan.textContent = imp.error.substring(0, 60);
            actionTd.appendChild(errSpan);
        }
        tr.appendChild(actionTd);

        table.appendChild(tr);
    });

    container.appendChild(table);
}
"##;
