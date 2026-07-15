//! Admin JavaScript for the SEO Page Generator module.
//!
//! Renders the admin UI with:
//! 1. Industry selector (if multiple industries are registered)
//! 2. Item selector (checkboxes from industry data + custom entries)
//! 3. Cities/areas text input (comma-separated)
//! 4. Business details (brand, phone, state)
//! 5. Page type selector (item hub, city hub, cross-product, category, category x city)
//! 6. Preview — shows planned pages with page counts before generating
//! 7. AI mode toggle with credit estimate
//! 8. Generate button with safety confirmation ("type GENERATE")
//! 9. Batch history
//!
//! Security: Uses DOM methods (createElement/textContent) exclusively — no innerHTML.

pub const ADMIN_JS: &str = r##"
// ── SEO Page Generator view ─────────────────────────────────────────
var pgCurrentIndustry = '';
var pgConfig = null;

async function load_page_generator() {
    const main = document.getElementById('adminMain');
    const el = document.createElement('div');

    // Title
    const toolbar = document.createElement('div');
    toolbar.className = 'toolbar';
    const h = document.createElement('h2');
    h.textContent = 'SEO Page Generator';
    toolbar.appendChild(h);
    el.appendChild(toolbar);

    const subtitle = document.createElement('p');
    subtitle.style.cssText = 'color:var(--text-muted);margin-bottom:24px;';
    subtitle.textContent = 'Generate hundreds of SEO-optimized pages from item \u00d7 city combinations';
    el.appendChild(subtitle);

    // ── Industry selector (if multiple) ──────────────────────────────
    var industries = [];
    try {
        var ir = await fetch('/api/modules/page-generator/industries');
        var ij = await ir.json();
        if (ij.ok && ij.data) industries = ij.data;
    } catch (e) {}

    if (industries.length > 1) {
        var indSection = document.createElement('div');
        indSection.className = 'pg-industry-select';
        var indLabel = document.createElement('span');
        indLabel.style.fontWeight = '600';
        indLabel.textContent = 'Industry:';
        indSection.appendChild(indLabel);

        var indSelect = document.createElement('select');
        indSelect.id = 'pg-industry';
        industries.forEach(function(ind) {
            var opt = document.createElement('option');
            opt.value = ind.slug;
            opt.textContent = ind.name;
            indSelect.appendChild(opt);
        });
        indSelect.addEventListener('change', async function() {
            pgCurrentIndustry = indSelect.value;
            // Reload config and items
            await pgReloadConfig();
            await pgReloadItems();
        });
        indSection.appendChild(indSelect);
        el.appendChild(indSection);

        pgCurrentIndustry = industries[0].slug;
    } else if (industries.length === 1) {
        pgCurrentIndustry = industries[0].slug;
    }

    // Fetch config for current industry
    try {
        var cr = await fetch('/api/modules/page-generator/config' + (pgCurrentIndustry ? '?industry=' + pgCurrentIndustry : ''));
        var cj = await cr.json();
        if (cj.ok && cj.data) pgConfig = cj.data;
    } catch (e) {}

    var itemSingular = pgConfig ? pgConfig.item_singular : 'item';
    var itemSingularCap = itemSingular.charAt(0).toUpperCase() + itemSingular.slice(1);
    var itemPlural = pgConfig ? pgConfig.item_plural : 'items';
    var itemPluralCap = itemPlural.charAt(0).toUpperCase() + itemPlural.slice(1);
    var industryName = pgConfig ? pgConfig.industry_name : 'Service';

    // ── Step 1: Select Items ─────────────────────────────────────────
    const itemSection = document.createElement('div');
    itemSection.className = 'pg-section';
    const itemH3 = document.createElement('h3');
    itemH3.id = 'pg-step1-heading';
    itemH3.textContent = '1. Select ' + itemPluralCap;
    itemSection.appendChild(itemH3);

    const itemGrid = document.createElement('div');
    itemGrid.className = 'pg-pest-grid';
    itemGrid.id = 'pg-pest-grid';

    // Fetch items
    var itemTypes = [];
    try {
        var r = await fetch('/api/modules/page-generator/items' + (pgCurrentIndustry ? '?industry=' + pgCurrentIndustry : ''));
        var json = await r.json();
        if (json.ok && json.data) itemTypes = json.data;
    } catch (e) {}

    // Select All / Select None buttons
    const selectRow = document.createElement('div');
    selectRow.style.cssText = 'display:flex;gap:8px;margin-bottom:8px;';
    const selectAllBtn = document.createElement('button');
    selectAllBtn.className = 'btn btn-ghost btn-sm';
    selectAllBtn.textContent = 'Select All';
    selectAllBtn.addEventListener('click', function() {
        itemGrid.querySelectorAll('input[type="checkbox"]').forEach(function(cb) {
            cb.checked = true;
            cb.closest('.pg-pest-item').classList.add('selected');
        });
    });
    selectRow.appendChild(selectAllBtn);
    const selectNoneBtn = document.createElement('button');
    selectNoneBtn.className = 'btn btn-ghost btn-sm';
    selectNoneBtn.textContent = 'Select None';
    selectNoneBtn.addEventListener('click', function() {
        itemGrid.querySelectorAll('input[type="checkbox"]').forEach(function(cb) {
            cb.checked = false;
            cb.closest('.pg-pest-item').classList.remove('selected');
        });
    });
    selectRow.appendChild(selectNoneBtn);
    itemSection.appendChild(selectRow);

    pgRenderItems(itemGrid, itemTypes);

    itemSection.appendChild(itemGrid);

    // Custom items
    const customLabel = document.createElement('div');
    customLabel.style.cssText = 'font-size:13px;font-weight:500;margin-bottom:4px;color:var(--text-muted);';
    customLabel.id = 'pg-custom-label';
    customLabel.textContent = 'Add custom ' + itemPlural + ':';
    itemSection.appendChild(customLabel);

    const customContainer = document.createElement('div');
    customContainer.id = 'pg-custom-pests';
    itemSection.appendChild(customContainer);

    const addCustomBtn = document.createElement('button');
    addCustomBtn.className = 'btn btn-ghost btn-sm';
    addCustomBtn.id = 'pg-add-custom-btn';
    addCustomBtn.textContent = '+ Add Custom ' + itemSingularCap;
    addCustomBtn.addEventListener('click', function() {
        var row = document.createElement('div');
        row.className = 'pg-custom-pest-row';
        var nameInput = document.createElement('input');
        nameInput.type = 'text';
        nameInput.placeholder = itemSingularCap + ' name';
        nameInput.style.flex = '1';
        nameInput.dataset.custom = '1';
        row.appendChild(nameInput);
        var catInput = document.createElement('input');
        catInput.type = 'text';
        catInput.placeholder = 'Category (optional)';
        catInput.style.width = '150px';
        catInput.dataset.customCat = '1';
        row.appendChild(catInput);
        var removeBtn = document.createElement('button');
        removeBtn.className = 'btn btn-ghost btn-sm';
        removeBtn.textContent = '\u00d7';
        removeBtn.addEventListener('click', function() { row.remove(); });
        row.appendChild(removeBtn);
        customContainer.appendChild(row);
    });
    itemSection.appendChild(addCustomBtn);
    el.appendChild(itemSection);

    // ── Step 2: Enter Cities ─────────────────────────────────────────
    const citySection = document.createElement('div');
    citySection.className = 'pg-section';
    const cityH3 = document.createElement('h3');
    cityH3.textContent = '2. Enter Cities / Service Areas';
    citySection.appendChild(cityH3);

    // Location profile picker (populated after the section is rendered)
    const profilePickerWrap = document.createElement('div');
    profilePickerWrap.id = 'pg-profile-picker';
    profilePickerWrap.style.cssText = 'margin-bottom:10px;display:none;';
    const profilePickerLabel = document.createElement('div');
    profilePickerLabel.style.cssText = 'font-size:0.85em;color:var(--text-muted);margin-bottom:6px;';
    profilePickerLabel.textContent = 'Location profiles (click to add):';
    profilePickerWrap.appendChild(profilePickerLabel);
    const profileChips = document.createElement('div');
    profileChips.id = 'pg-profile-chips';
    profileChips.style.cssText = 'display:flex;flex-wrap:wrap;gap:6px;margin-bottom:6px;';
    profilePickerWrap.appendChild(profileChips);
    const useAllBtn = document.createElement('button');
    useAllBtn.className = 'btn btn-ghost btn-sm';
    useAllBtn.textContent = 'Use all profile cities';
    useAllBtn.addEventListener('click', function() {
        var allCities = [];
        profileChips.querySelectorAll('[data-city]').forEach(function(chip) {
            allCities.push(chip.dataset.city);
        });
        document.getElementById('pg-cities').value = allCities.join(', ');
    });
    profilePickerWrap.appendChild(useAllBtn);
    citySection.appendChild(profilePickerWrap);

    const cityInput = document.createElement('textarea');
    cityInput.className = 'pg-cities-input';
    cityInput.id = 'pg-cities';
    cityInput.rows = 3;
    cityInput.placeholder = 'Dallas, Fort Worth, Arlington, Plano, Frisco, McKinney...';
    citySection.appendChild(cityInput);

    const cityHelp = document.createElement('div');
    cityHelp.className = 'pg-cities-help';
    cityHelp.textContent = 'Separate cities with commas. Each city creates a city hub page + cross-product pages with each selected ' + itemSingular + '. Location profiles add local intelligence to AI-generated pages.';
    citySection.appendChild(cityHelp);
    el.appendChild(citySection);

    // Async: fetch location profiles and populate picker
    fetch('/api/modules/location-profile/locations')
        .then(function(r) { return r.json(); })
        .then(function(data) {
            var profiles = data && Array.isArray(data.data)
                ? data.data : [];
            profiles = profiles.filter(function(p) { return p.active !== false; });
            if (profiles.length === 0) return;
            profiles.forEach(function(p) {
                var chip = document.createElement('button');
                chip.className = 'btn btn-ghost btn-sm';
                chip.style.cssText = 'font-size:0.82em;padding:2px 8px;';
                chip.dataset.city = p.city;
                chip.textContent = p.city + (p.state ? ', ' + p.state : '');
                chip.title = 'Add ' + p.city + ' to cities list';
                chip.addEventListener('click', function() {
                    var ta = document.getElementById('pg-cities');
                    var cur = ta.value.trim();
                    var cities = cur ? cur.split(',').map(function(c){return c.trim();}).filter(Boolean) : [];
                    if (!cities.includes(p.city)) {
                        cities.push(p.city);
                        ta.value = cities.join(', ');
                    }
                });
                profileChips.appendChild(chip);
            });
            profilePickerWrap.style.display = '';
        })
        .catch(function() { /* location-profile module may not be enabled */ });

    // ── Step 3: Business Details ─────────────────────────────────────
    const brandSection = document.createElement('div');
    brandSection.className = 'pg-section';
    const brandH3 = document.createElement('h3');
    brandH3.textContent = '3. Business Details';
    brandSection.appendChild(brandH3);

    const brandRow = document.createElement('div');
    brandRow.className = 'pg-brand-row';

    const brandInput = document.createElement('input');
    brandInput.type = 'text';
    brandInput.id = 'pg-brand';
    brandInput.placeholder = 'Business Name (e.g. General Pest Co)';
    brandRow.appendChild(brandInput);

    const phoneInput = document.createElement('input');
    phoneInput.type = 'tel';
    phoneInput.id = 'pg-phone';
    phoneInput.placeholder = 'Phone (e.g. 555-123-4567)';
    brandRow.appendChild(phoneInput);

    const stateInput = document.createElement('input');
    stateInput.type = 'text';
    stateInput.id = 'pg-state';
    stateInput.placeholder = 'State (e.g. TX)';
    stateInput.style.maxWidth = '80px';
    brandRow.appendChild(stateInput);

    brandSection.appendChild(brandRow);
    el.appendChild(brandSection);

    // ── Step 4: Page Types ───────────────────────────────────────────
    const typeSection = document.createElement('div');
    typeSection.className = 'pg-section';
    const typeH3 = document.createElement('h3');
    typeH3.textContent = '4. Page Types to Generate';
    typeSection.appendChild(typeH3);

    const typeOptions = document.createElement('div');
    typeOptions.className = 'pg-options';

    var pageTypeChoices = [
        { key: 'item_hub', label: itemSingularCap + ' Hub Pages', desc: 'One per ' + itemSingular, checked: true },
        { key: 'city_hub', label: 'City Hub Pages', desc: 'One per city', checked: true },
        { key: 'item_city', label: itemSingularCap + ' \u00d7 City', desc: 'Cross-product pages', checked: true },
        { key: 'category_hub', label: 'Category Hubs', desc: 'One per category', checked: false },
        { key: 'category_city', label: 'Category \u00d7 City', desc: 'Category cross-products', checked: false },
    ];

    pageTypeChoices.forEach(function(pt) {
        var chip = document.createElement('label');
        chip.className = 'pg-option-chip' + (pt.checked ? ' selected' : '');
        var cb = document.createElement('input');
        cb.type = 'checkbox';
        cb.checked = pt.checked;
        cb.dataset.pageType = pt.key;
        cb.addEventListener('change', function() {
            chip.classList.toggle('selected', cb.checked);
        });
        chip.appendChild(cb);
        var text = document.createElement('span');
        text.textContent = pt.label;
        chip.appendChild(text);
        typeOptions.appendChild(chip);
    });

    typeSection.appendChild(typeOptions);
    el.appendChild(typeSection);

    // ── Preview + Generate ──────────────────────────────────────────
    const actionSection = document.createElement('div');
    actionSection.className = 'pg-section';

    // AI toggle
    const aiToggle = document.createElement('div');
    aiToggle.className = 'pg-ai-toggle';
    aiToggle.id = 'pg-ai-toggle';
    aiToggle.style.display = 'none'; // hidden until AI check

    const aiLabel = document.createElement('label');
    aiLabel.className = 'module-toggle';
    const aiCb = document.createElement('input');
    aiCb.type = 'checkbox';
    aiCb.id = 'pg-ai-mode';
    const aiSlider = document.createElement('span');
    aiSlider.className = 'slider';
    aiLabel.appendChild(aiCb);
    aiLabel.appendChild(aiSlider);
    aiToggle.appendChild(aiLabel);

    const aiText = document.createElement('span');
    aiText.style.cssText = 'font-weight:600;font-size:15px;';
    aiText.textContent = 'AI-Generated Content';
    aiToggle.appendChild(aiText);

    const aiInfo = document.createElement('div');
    aiInfo.style.cssText = 'font-size:13px;color:var(--text-muted);width:100%;margin-top:4px;';
    aiInfo.id = 'pg-ai-info';
    aiToggle.appendChild(aiInfo);

    actionSection.appendChild(aiToggle);

    const aiCostDiv = document.createElement('div');
    aiCostDiv.className = 'pg-ai-cost';
    aiCostDiv.id = 'pg-ai-cost';
    aiCostDiv.style.display = 'none';
    actionSection.appendChild(aiCostDiv);

    // Check AI availability
    try {
        const aiResp = await fetch('/api/modules/page-generator/ai/status').then(function(r) { return r.json(); });
        if (aiResp.ok) {
            aiToggle.style.display = 'flex';
            aiInfo.textContent = 'Using ' + aiResp.data.provider + ' (' + aiResp.data.model + ') \u2014 generates unique content per page (costs credits)';
        }
    } catch (e) {}

    // Preview area
    const previewArea = document.createElement('div');
    previewArea.id = 'pg-preview-area';
    actionSection.appendChild(previewArea);

    // Buttons
    const btns = document.createElement('div');
    btns.style.cssText = 'display:flex;gap:8px;flex-wrap:wrap;align-items:center;';

    const previewBtn = document.createElement('button');
    previewBtn.className = 'btn btn-ghost';
    previewBtn.textContent = 'Preview Pages';
    previewBtn.addEventListener('click', async function() {
        var data = gatherPageGenData();
        if (!data) return;
        previewBtn.disabled = true;
        previewBtn.textContent = 'Planning...';
        try {
            var r = await fetch('/api/modules/page-generator/preview', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify(data),
            });
            var json = await r.json();
            if (!json.ok) {
                showToast(json.message, 'error');
                return;
            }
            renderPageGenPreview(json.data);

            // Update AI cost if toggled
            var aiCheck = document.getElementById('pg-ai-mode');
            if (aiCheck && aiCheck.checked) {
                var cd = document.getElementById('pg-ai-cost');
                cd.textContent = 'Estimated AI cost: ' + json.data.ai_credits_estimate + ' credits (' + json.data.total_pages + ' pages \u00d7 ' + (json.data.credits_per_page + json.data.credits_per_seo) + ' credits each)';
                cd.style.display = 'block';
            }
        } catch (e) {
            showToast('Network error', 'error');
        } finally {
            previewBtn.disabled = false;
            previewBtn.textContent = 'Preview Pages';
        }
    });
    btns.appendChild(previewBtn);

    const genBtn = document.createElement('button');
    genBtn.className = 'btn btn-primary';
    genBtn.textContent = 'Generate Pages';
    genBtn.addEventListener('click', function() {
        showGenerateConfirmation();
    });
    btns.appendChild(genBtn);

    actionSection.appendChild(btns);
    el.appendChild(actionSection);

    // ── Batch History ────────────────────────────────────────────────
    const histSection = document.createElement('div');
    histSection.className = 'pg-section pg-batch-list';
    const histH3 = document.createElement('h3');
    histH3.textContent = 'Generation History';
    histSection.appendChild(histH3);

    try {
        var r = await fetch('/api/modules/page-generator/batches');
        var json = await r.json();
        var batches = (json.ok && json.data) ? json.data : [];
        if (batches.length === 0) {
            var empty = document.createElement('p');
            empty.style.color = 'var(--text-muted)';
            empty.textContent = 'No pages generated yet.';
            histSection.appendChild(empty);
        } else {
            batches.forEach(function(b) {
                var item = document.createElement('div');
                item.className = 'pg-batch-item';

                var left = document.createElement('div');
                var mode = document.createElement('span');
                mode.className = 'pg-batch-mode ' + b.mode;
                mode.textContent = b.mode.toUpperCase();
                left.appendChild(mode);

                var info = document.createElement('span');
                info.style.marginLeft = '8px';
                var countLabel = (b.item_count || b.pest_count || 0) + ' items \u00d7 ' + b.city_count + ' cities';
                info.textContent = b.pages_created + ' pages (' + countLabel + ')';
                if (b.industry) {
                    var indBadge = document.createElement('span');
                    indBadge.style.cssText = 'font-size:10px;background:var(--surface);border:1px solid var(--border);border-radius:4px;padding:1px 6px;margin-left:6px;';
                    indBadge.textContent = b.industry;
                    info.appendChild(indBadge);
                }
                left.appendChild(info);
                item.appendChild(left);

                var right = document.createElement('span');
                right.style.color = 'var(--text-muted)';
                if (b.created_at) {
                    var d = new Date(b.created_at * 1000);
                    right.textContent = d.toLocaleDateString() + ' ' + d.toLocaleTimeString();
                }
                item.appendChild(right);

                histSection.appendChild(item);
            });
        }
    } catch (e) {}

    el.appendChild(histSection);
    main.replaceChildren(el);
}

// ── Render items into the grid ───────────────────────────────────────
function pgRenderItems(grid, items) {
    while (grid.firstChild) grid.removeChild(grid.firstChild);
    items.forEach(function(it) {
        var item = document.createElement('label');
        item.className = 'pg-pest-item';
        var cb = document.createElement('input');
        cb.type = 'checkbox';
        cb.dataset.slug = it.slug;
        cb.addEventListener('change', function() {
            item.classList.toggle('selected', cb.checked);
        });
        item.appendChild(cb);
        var name = document.createElement('span');
        name.textContent = it.name;
        item.appendChild(name);
        var cat = document.createElement('span');
        cat.className = 'pg-pest-category';
        cat.textContent = it.category;
        item.appendChild(cat);
        grid.appendChild(item);
    });
}

// ── Reload config for selected industry ──────────────────────────────
async function pgReloadConfig() {
    try {
        var cr = await fetch('/api/modules/page-generator/config?industry=' + pgCurrentIndustry);
        var cj = await cr.json();
        if (cj.ok && cj.data) {
            pgConfig = cj.data;
            // Update dynamic labels
            var singular = pgConfig.item_singular;
            var singularCap = singular.charAt(0).toUpperCase() + singular.slice(1);
            var plural = pgConfig.item_plural;
            var pluralCap = plural.charAt(0).toUpperCase() + plural.slice(1);

            var h3 = document.getElementById('pg-step1-heading');
            if (h3) h3.textContent = '1. Select ' + pluralCap;

            var cl = document.getElementById('pg-custom-label');
            if (cl) cl.textContent = 'Add custom ' + plural + ':';

            var ab = document.getElementById('pg-add-custom-btn');
            if (ab) ab.textContent = '+ Add Custom ' + singularCap;
        }
    } catch (e) {}
}

// ── Reload items for selected industry ───────────────────────────────
async function pgReloadItems() {
    try {
        var r = await fetch('/api/modules/page-generator/items?industry=' + pgCurrentIndustry);
        var json = await r.json();
        var items = (json.ok && json.data) ? json.data : [];
        var grid = document.getElementById('pg-pest-grid');
        if (grid) pgRenderItems(grid, items);
    } catch (e) {}
}

// ── Gather form data ────────────────────────────────────────────────
function gatherPageGenData() {
    // Selected item slugs
    var itemSlugs = [];
    document.querySelectorAll('#pg-pest-grid input[type="checkbox"]:checked').forEach(function(cb) {
        if (cb.dataset.slug) itemSlugs.push(cb.dataset.slug);
    });

    // Custom items
    var customItems = [];
    document.querySelectorAll('#pg-custom-pests .pg-custom-pest-row').forEach(function(row) {
        var nameEl = row.querySelector('input[data-custom]');
        var catEl = row.querySelector('input[data-custom-cat]');
        if (nameEl && nameEl.value.trim()) {
            customItems.push({
                name: nameEl.value.trim(),
                category: catEl ? catEl.value.trim() : '',
            });
        }
    });

    var singularLabel = pgConfig ? pgConfig.item_singular : 'item';
    if (itemSlugs.length === 0 && customItems.length === 0) {
        showToast('Select at least one ' + singularLabel, 'error');
        return null;
    }

    // Cities
    var citiesRaw = document.getElementById('pg-cities').value.trim();
    if (!citiesRaw) {
        showToast('Enter at least one city or service area', 'error');
        return null;
    }
    var cities = citiesRaw.split(',').map(function(c) { return c.trim(); }).filter(function(c) { return c.length > 0; });
    if (cities.length === 0) {
        showToast('Enter at least one city', 'error');
        return null;
    }

    // Page types
    var pageTypes = [];
    document.querySelectorAll('.pg-options input[type="checkbox"]:checked').forEach(function(cb) {
        if (cb.dataset.pageType) pageTypes.push(cb.dataset.pageType);
    });

    var aiCb = document.getElementById('pg-ai-mode');
    var mode = (aiCb && aiCb.checked) ? 'ai' : 'template';

    return {
        item_slugs: itemSlugs,
        custom_items: customItems,
        cities: cities,
        state_abbr: document.getElementById('pg-state').value.trim(),
        brand: document.getElementById('pg-brand').value.trim(),
        phone: document.getElementById('pg-phone').value.trim(),
        page_types: pageTypes,
        mode: mode,
        industry: pgCurrentIndustry,
    };
}

// ── Render preview ──────────────────────────────────────────────────
function renderPageGenPreview(data) {
    var area = document.getElementById('pg-preview-area');
    while (area.firstChild) area.removeChild(area.firstChild);

    // Summary stats
    var summary = document.createElement('div');
    summary.className = 'pg-preview-summary';

    var totalStat = pgStat(data.total_pages, 'Total Pages');
    summary.appendChild(totalStat);

    var byKind = data.by_kind || {};
    Object.keys(byKind).forEach(function(kind) {
        summary.appendChild(pgStat(byKind[kind], kind));
    });

    if (data.ai_credits_estimate > 0) {
        var aiCb = document.getElementById('pg-ai-mode');
        if (aiCb && aiCb.checked) {
            summary.appendChild(pgStat(data.ai_credits_estimate, 'AI Credits'));
        }
    }

    area.appendChild(summary);

    // Page list
    var box = document.createElement('div');
    box.className = 'pg-preview-box';

    (data.pages || []).forEach(function(p) {
        var row = document.createElement('div');
        row.className = 'pg-page-row';

        var kindBadge = document.createElement('span');
        kindBadge.className = 'pg-page-kind pg-kind-' + p.kind;
        kindBadge.textContent = (p.kind || '').replace('_', ' ').toUpperCase();
        row.appendChild(kindBadge);

        var title = document.createElement('span');
        title.textContent = p.title;
        title.style.flex = '1';
        row.appendChild(title);

        var slug = document.createElement('span');
        slug.className = 'pg-page-slug';
        slug.textContent = '/' + p.slug;
        row.appendChild(slug);

        box.appendChild(row);
    });

    area.appendChild(box);
}

function pgStat(num, label) {
    var stat = document.createElement('div');
    stat.className = 'pg-preview-stat';
    var n = document.createElement('div');
    n.className = 'num';
    n.textContent = num;
    stat.appendChild(n);
    var l = document.createElement('div');
    l.className = 'lbl';
    l.textContent = label;
    stat.appendChild(l);
    return stat;
}

// ── Generate confirmation modal ─────────────────────────────────────
async function showGenerateConfirmation() {
    var data = gatherPageGenData();
    if (!data) return;

    // Create overlay
    var overlay = document.createElement('div');
    overlay.className = 'bp-overlay';
    overlay.addEventListener('click', function(e) {
        if (e.target === overlay) overlay.remove();
    });

    var modal = document.createElement('div');
    modal.className = 'bp-modal';
    modal.style.maxWidth = '520px';

    var title = document.createElement('h3');
    title.textContent = 'Confirm Bulk Page Generation';
    title.style.marginBottom = '16px';
    modal.appendChild(title);

    // Calculate page count
    var itemCount = data.item_slugs.length + data.custom_items.length;
    var cityCount = data.cities.length;
    var estimate = 0;
    if (data.page_types.indexOf('item_hub') >= 0 || data.page_types.indexOf('pest_hub') >= 0 || data.page_types.length === 0) estimate += itemCount;
    if (data.page_types.indexOf('city_hub') >= 0 || data.page_types.length === 0) estimate += cityCount;
    if (data.page_types.indexOf('item_city') >= 0 || data.page_types.indexOf('pest_city') >= 0 || data.page_types.length === 0) estimate += itemCount * cityCount;

    var singularLabel = pgConfig ? pgConfig.item_singular : 'item';
    var pluralLabel = pgConfig ? pgConfig.item_plural : 'items';

    var info = document.createElement('p');
    info.style.cssText = 'margin-bottom:16px;font-size:14px;';
    info.textContent = 'This will create approximately ' + estimate + ' pages (' + itemCount + ' ' + pluralLabel + ' \u00d7 ' + cityCount + ' cities).';
    modal.appendChild(info);

    var creditsEstimate = estimate * 18;

    if (data.mode === 'ai') {
        var aiBox = document.createElement('div');
        aiBox.className = 'pg-ai-cost';

        // Credit cost line (price fetched from rates API below)
        var costLine = document.createElement('div');
        costLine.style.cssText = 'font-weight:600;font-size:14px;margin-bottom:6px;';
        costLine.textContent = 'AI cost: ' + creditsEstimate + ' credits';
        aiBox.appendChild(costLine);

        // Fetch actual credit pricing
        (async function() {
            try {
                var rr = await fetch('/api/modules/nexus/credits/rates');
                var rj = await rr.json();
                if (rj.ok && rj.data && rj.data.packs && rj.data.packs.length > 0) {
                    // Use the midrange pack price as reference
                    var pack = rj.data.packs[1] || rj.data.packs[0];
                    var perCredit = pack.per_credit || 0.009;
                    var dollarCost = (creditsEstimate * perCredit).toFixed(2);
                    costLine.textContent = 'AI cost: ' + creditsEstimate + ' credits (~$' + dollarCost + ' at ' + pack.name + ' rate)';
                }
            } catch (e) {}
        })();

        var detailLine = document.createElement('div');
        detailLine.style.cssText = 'font-size:12px;margin-bottom:8px;';
        detailLine.textContent = estimate + ' pages \u00d7 18 credits each (15 content + 3 SEO)';
        aiBox.appendChild(detailLine);

        // Fetch credit balance
        var balanceLine = document.createElement('div');
        balanceLine.style.cssText = 'font-size:13px;padding-top:8px;border-top:1px solid rgba(0,0,0,0.1);';
        balanceLine.textContent = 'Checking credit balance...';
        aiBox.appendChild(balanceLine);

        modal.appendChild(aiBox);

        // Async fetch balance
        (async function() {
            try {
                var r = await fetch('/api/modules/nexus/credits/balance', {
                    method: 'POST',
                    headers: { 'Content-Type': 'application/json' },
                    body: JSON.stringify({ license_key: '' }),
                });
                var json = await r.json();
                if (json.ok && json.data) {
                    var total = (json.data.credits_remaining || 0) + (json.data.bundle_credits_remaining || 0);
                    var after = total - creditsEstimate;
                    if (after >= 0) {
                        balanceLine.textContent = 'Your balance: ' + total + ' credits \u2014 ' + after + ' remaining after generation';
                        balanceLine.style.color = '#166534';
                    } else {
                        balanceLine.textContent = 'Your balance: ' + total + ' credits \u2014 need ' + (-after) + ' more';
                        balanceLine.style.color = '#dc2626';
                        // Add buy more button
                        var buyBtn = document.createElement('a');
                        buyBtn.href = '/admin#commerce';
                        buyBtn.target = '_blank';
                        buyBtn.className = 'btn btn-ghost btn-sm';
                        buyBtn.style.cssText = 'display:inline-block;margin-top:8px;font-size:12px;text-decoration:none;';
                        buyBtn.textContent = 'Buy More Credits';
                        aiBox.appendChild(buyBtn);
                    }
                } else {
                    balanceLine.textContent = 'Could not check balance (Central may be offline)';
                }
            } catch (e) {
                balanceLine.textContent = 'Could not check balance';
            }
        })();

        // Time estimate
        var timeLine = document.createElement('div');
        timeLine.style.cssText = 'font-size:13px;color:var(--text-muted);margin-top:12px;padding:8px 12px;background:rgba(59,130,246,0.06);border-radius:6px;';
        var estSeconds = estimate * 30;
        var estMin = Math.ceil(estSeconds / 60);
        timeLine.textContent = 'Estimated time: ~' + estMin + ' minute' + (estMin !== 1 ? 's' : '') + ' (' + estimate + ' pages \u00d7 ~30 seconds each). Please do not close this window.';
        modal.appendChild(timeLine);
    }

    var confirmLabel = document.createElement('p');
    confirmLabel.style.cssText = 'font-size:14px;font-weight:500;margin-bottom:8px;margin-top:16px;';
    confirmLabel.textContent = 'Type GENERATE to confirm:';
    modal.appendChild(confirmLabel);

    var confirmInput = document.createElement('input');
    confirmInput.type = 'text';
    confirmInput.className = 'pg-confirm-input';
    confirmInput.placeholder = 'GENERATE';
    modal.appendChild(confirmInput);

    var btnRow = document.createElement('div');
    btnRow.style.cssText = 'display:flex;gap:8px;margin-top:16px;';

    var goBtn = document.createElement('button');
    goBtn.className = 'btn btn-primary';
    goBtn.textContent = 'Generate';
    goBtn.addEventListener('click', async function() {
        if (confirmInput.value.trim() !== 'GENERATE') {
            showToast('Type GENERATE to confirm', 'error');
            return;
        }
        data.confirmation = 'GENERATE';
        data.max_credits = creditsEstimate;
        goBtn.disabled = true;
        confirmInput.disabled = true;

        if (data.mode === 'ai') {
            // Show progress with time estimate
            goBtn.textContent = 'Generating with AI... (0/' + estimate + ')';
            var startTime = Date.now();
            var progressInterval = setInterval(function() {
                var elapsed = Math.floor((Date.now() - startTime) / 1000);
                var estPage = Math.min(Math.floor(elapsed / 30), estimate);
                var remaining = Math.max(0, (estimate - estPage) * 30);
                var remMin = Math.floor(remaining / 60);
                var remSec = remaining % 60;
                var timeStr = remMin > 0 ? remMin + 'm ' + remSec + 's' : remSec + 's';
                goBtn.textContent = 'Generating with AI... (~' + timeStr + ' remaining)';
            }, 3000);
        } else {
            goBtn.textContent = 'Generating...';
            var progressInterval = null;
        }

        try {
            var r = await fetch('/api/modules/page-generator/generate', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify(data),
            });
            if (progressInterval) clearInterval(progressInterval);
            var json = await r.json();
            if (!json.ok) {
                showToast(json.message, 'error');
                goBtn.disabled = false;
                confirmInput.disabled = false;
                goBtn.textContent = 'Generate';
                return;
            }
            showToast(json.message, 'success');
            showGenerateResult(modal, json.data);
        } catch (e) {
            if (progressInterval) clearInterval(progressInterval);
            showToast('Network error', 'error');
            goBtn.disabled = false;
            confirmInput.disabled = false;
            goBtn.textContent = 'Generate';
        }
    });
    btnRow.appendChild(goBtn);

    var cancelBtn = document.createElement('button');
    cancelBtn.className = 'btn btn-ghost';
    cancelBtn.textContent = 'Cancel';
    cancelBtn.addEventListener('click', function() { overlay.remove(); });
    btnRow.appendChild(cancelBtn);

    modal.appendChild(btnRow);
    overlay.appendChild(modal);
    document.body.appendChild(overlay);
}

// ── Show generation result ──────────────────────────────────────────
function showGenerateResult(modal, data) {
    while (modal.firstChild) modal.removeChild(modal.firstChild);

    var result = document.createElement('div');
    result.className = 'pg-result';

    var icon = document.createElement('div');
    icon.className = 'pg-result-icon';
    icon.textContent = '\u2705';
    result.appendChild(icon);

    var title = document.createElement('h3');
    title.textContent = 'Pages Generated!';
    title.style.marginBottom = '8px';
    result.appendChild(title);

    var sub = document.createElement('p');
    sub.style.color = 'var(--text-muted)';
    sub.textContent = data.pages_created + ' pages created with ' + data.seo_entries_created + ' SEO entries';
    result.appendChild(sub);

    if (data.mode === 'ai' && data.ai_tokens_used > 0) {
        var aiInfo = document.createElement('p');
        aiInfo.style.cssText = 'font-size:13px;color:var(--text-muted);margin-top:8px;';
        aiInfo.textContent = 'AI: ' + data.ai_tokens_used + ' tokens, ' + data.ai_credits_charged + ' credits';
        result.appendChild(aiInfo);
    }

    if (data.errors && data.errors.length > 0) {
        var errDiv = document.createElement('div');
        errDiv.className = 'bp-errors';
        errDiv.style.textAlign = 'left';
        errDiv.style.marginTop = '16px';
        var errH4 = document.createElement('h4');
        errH4.textContent = data.errors.length + ' warnings';
        errDiv.appendChild(errH4);
        var errUl = document.createElement('ul');
        data.errors.forEach(function(e) {
            var li = document.createElement('li');
            li.textContent = e;
            errUl.appendChild(li);
        });
        errDiv.appendChild(errUl);
        result.appendChild(errDiv);
    }

    // Sharing offer (shown when customer content sources were used)
    if (data.sharing_offer && data.sharing_offer.eligible) {
        var offer = data.sharing_offer;
        var offerDiv = document.createElement('div');
        offerDiv.style.cssText = 'margin-top:24px;text-align:left;border-top:1px solid var(--border,#333);padding-top:16px;';

        var offerH4 = document.createElement('h4');
        offerH4.textContent = 'Share Your Content?';
        offerH4.style.marginBottom = '8px';
        offerDiv.appendChild(offerH4);

        var offerDesc = document.createElement('p');
        offerDesc.style.cssText = 'font-size:13px;color:var(--text-muted);margin-bottom:16px;';
        offerDesc.textContent = 'Your business content helped generate these pages. You can share it to earn credit refunds.';
        offerDiv.appendChild(offerDesc);

        var optionsDiv = document.createElement('div');
        optionsDiv.style.cssText = 'display:flex;flex-direction:column;gap:8px;';

        // Trusted Source option
        var trustedBtn = document.createElement('button');
        trustedBtn.className = 'btn btn-primary';
        trustedBtn.style.cssText = 'text-align:left;padding:12px 16px;';
        var trustedRefund = Math.floor(offer.credits_used * offer.refund_trusted_source_pct / 100);
        trustedBtn.textContent = 'Become a Trusted Source — get ' + trustedRefund + ' credits back (' + offer.refund_trusted_source_pct + '%)';
        trustedBtn.addEventListener('click', function() {
            _pgSharingChoice(offer.source_ids, 'share_as_trusted_source', offerDiv);
        });
        optionsDiv.appendChild(trustedBtn);

        // Anonymized option
        var anonBtn = document.createElement('button');
        anonBtn.className = 'btn btn-secondary';
        anonBtn.style.cssText = 'text-align:left;padding:12px 16px;';
        var anonRefund = Math.floor(offer.credits_used * offer.refund_anonymized_pct / 100);
        anonBtn.textContent = 'Share anonymously — get ' + anonRefund + ' credits back (' + offer.refund_anonymized_pct + '%)';
        anonBtn.addEventListener('click', function() {
            _pgSharingChoice(offer.source_ids, 'share_anonymized', offerDiv);
        });
        optionsDiv.appendChild(anonBtn);

        // No thanks option
        var noBtn = document.createElement('button');
        noBtn.className = 'btn';
        noBtn.style.cssText = 'text-align:left;padding:12px 16px;background:transparent;border:1px solid var(--border,#333);color:var(--text-muted);';
        noBtn.textContent = 'No thanks — keep my content private';
        noBtn.addEventListener('click', function() {
            _pgSharingChoice(offer.source_ids, 'never_share', offerDiv);
        });
        optionsDiv.appendChild(noBtn);

        offerDiv.appendChild(optionsDiv);
        result.appendChild(offerDiv);
    }

    var doneBtn = document.createElement('button');
    doneBtn.className = 'btn btn-primary';
    doneBtn.textContent = 'Done';
    doneBtn.style.marginTop = '24px';
    doneBtn.addEventListener('click', function() {
        var overlay = modal.parentElement;
        if (overlay) overlay.remove();
        load_page_generator();
    });
    result.appendChild(doneBtn);

    modal.appendChild(result);
}

async function _pgSharingChoice(sourceIds, tier, offerDiv) {
    var promises = sourceIds.map(function(sid) {
        return fetch('/api/modules/content-sources/sources/' + encodeURIComponent(sid) + '/sharing', {
            method: 'PUT',
            headers: {'Content-Type':'application/json'},
            body: JSON.stringify({ sharing_tier: tier })
        }).then(function(r) { return r.json(); });
    });

    try {
        var results = await Promise.all(promises);
        var firstOk = results.find(function(r) { return r.ok; });
        if (firstOk) {
            showToast(firstOk.message, 'success');
        }
    } catch(e) {
        showToast('Failed to save sharing preference', 'error');
    }

    // Replace offer with confirmation
    offerDiv.textContent = '';
    var conf = document.createElement('p');
    conf.style.cssText = 'color:var(--text-muted);font-size:13px;padding:8px 0;';
    if (tier === 'never_share') {
        conf.textContent = 'Your content will remain private.';
    } else {
        conf.textContent = 'Your content is being reviewed. Credits will be applied within 24 hours.';
    }
    offerDiv.appendChild(conf);
}
"##;
