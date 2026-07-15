//! Presets sub-tab for the unified Design Studio.
//!
//! Provides: searchable preset library, create/edit presets,
//! dynamic data bindings, drag-to-page functionality.
//!
//! AI context: preset name, category, block count, target audience.
//!
//! Security: All JS uses DOM methods only (createElement, textContent,
//! replaceChildren). No innerHTML, outerHTML, or insertAdjacentHTML.

pub fn presets_js() -> &'static str {
    PRESETS_JS
}

const PRESETS_JS: &str = r####"
/* ── TsStudio.loadPresets: Presets sub-tab ──────────────────────────── */

TsStudio.loadPresets = async function() {
    var controls = document.getElementById('tsStudioControls');
    if (!controls) return;
    controls.replaceChildren();

    // ── Category filter ──
    var filterBar = document.createElement('div');
    filterBar.style.cssText = 'display:flex;gap:4px;flex-wrap:wrap;margin-bottom:12px;';
    var categories = ['all', 'marketing', 'layout', 'commerce', 'interactive', 'content'];
    var _activeCategory = 'all';

    categories.forEach(function(cat) {
        var btn = document.createElement('button');
        btn.className = 'ts-studio-btn' + (cat === 'all' ? ' ts-studio-btn-primary' : '');
        btn.textContent = cat.charAt(0).toUpperCase() + cat.slice(1);
        btn.dataset.cat = cat;
        btn.onclick = function() {
            _activeCategory = cat;
            filterBar.querySelectorAll('button').forEach(function(b) {
                b.className = 'ts-studio-btn' + (b.dataset.cat === cat ? ' ts-studio-btn-primary' : '');
            });
            renderPresetList();
        };
        filterBar.appendChild(btn);
    });

    var topCard = tsCard();
    topCard.appendChild(tsH2('Presets'));
    topCard.appendChild(filterBar);

    // Search
    var searchInput = tsInput('text', '', {});
    searchInput.placeholder = 'Search presets...';
    searchInput.style.cssText = 'width:100%;margin-bottom:8px;padding:4px 8px;border:1px solid var(--border);border-radius:4px;background:var(--bg);color:var(--text);';
    searchInput.addEventListener('input', function() { renderPresetList(); });
    topCard.appendChild(searchInput);

    // Create button
    var createBtn = tsBtn('+ New Preset', function() { showPresetEditor(null); });
    topCard.appendChild(createBtn);
    controls.appendChild(topCard);

    // Preset list area
    var listArea = document.createElement('div');
    listArea.id = 'tsPresetList';
    controls.appendChild(listArea);

    // Editor area (hidden until editing)
    var editorArea = document.createElement('div');
    editorArea.id = 'tsPresetEditor';
    editorArea.style.display = 'none';
    controls.appendChild(editorArea);

    // State
    var _presets = [];
    var _editingPreset = null;
    var _presetBlockEditor = null;

    // ── Load presets ──
    async function loadPresets() {
        var res = await tsApi('/presets');
        if (res.ok) _presets = res.data || [];
        renderPresetList();
    }

    function renderPresetList() {
        var container = document.getElementById('tsPresetList');
        if (!container) return;
        container.replaceChildren();

        var query = searchInput.value.toLowerCase();
        var filtered = _presets.filter(function(p) {
            if (_activeCategory !== 'all' && p.category !== _activeCategory) return false;
            if (query && p.name.toLowerCase().indexOf(query) < 0 && p.description.toLowerCase().indexOf(query) < 0) return false;
            return true;
        });

        if (filtered.length === 0) {
            container.appendChild(tsEmpty('No presets found. Create one to get started.'));
            return;
        }

        filtered.forEach(function(preset) {
            var card = tsCard();
            card.style.cssText = 'margin-bottom:8px;cursor:pointer;';
            card.draggable = true;
            card.dataset.presetId = preset.preset_id;

            // Drag support for dropping into page editor
            card.addEventListener('dragstart', function(e) {
                e.dataTransfer.setData('application/x-liq-preset', JSON.stringify(preset));
                e.dataTransfer.effectAllowed = 'copy';
            });

            var header = document.createElement('div');
            header.style.cssText = 'display:flex;align-items:center;justify-content:space-between;';
            var name = document.createElement('strong');
            name.textContent = preset.name;
            header.appendChild(name);
            var badge = document.createElement('span');
            badge.className = 'ts-badge';
            badge.textContent = preset.category;
            header.appendChild(badge);
            card.appendChild(header);

            if (preset.description) {
                var desc = document.createElement('p');
                desc.textContent = preset.description;
                desc.style.cssText = 'margin:4px 0 8px;font-size:12px;color:var(--text-muted);';
                card.appendChild(desc);
            }

            var blockCount = 0;
            try { blockCount = JSON.parse(preset.blocks_json || '[]').length; } catch(e) {}
            var meta = document.createElement('div');
            meta.style.cssText = 'font-size:11px;color:var(--text-muted);display:flex;gap:12px;';
            meta.textContent = blockCount + ' block' + (blockCount !== 1 ? 's' : '');
            if (preset.bindings && preset.bindings.length > 0) {
                meta.textContent += ' \u00b7 ' + preset.bindings.length + ' binding' + (preset.bindings.length !== 1 ? 's' : '');
            }
            card.appendChild(meta);

            var actions = document.createElement('div');
            actions.style.cssText = 'display:flex;gap:4px;margin-top:8px;';
            actions.appendChild(tsBtn('Edit', function(e) { e.stopPropagation(); showPresetEditor(preset); }));
            actions.appendChild(tsBtnDanger('Delete', async function(e) {
                e.stopPropagation();
                if (!confirm('Delete preset "' + preset.name + '"?')) return;
                await tsApi('/presets/' + preset.preset_id, { method: 'DELETE' });
                await loadPresets();
                if (typeof showToast === 'function') showToast('Preset deleted', 'success');
            }));
            card.appendChild(actions);

            container.appendChild(card);
        });
    }

    // ── Preset editor ──
    function showPresetEditor(preset) {
        var editor = document.getElementById('tsPresetEditor');
        if (!editor) return;
        editor.replaceChildren();
        editor.style.display = '';
        _editingPreset = preset;

        var edCard = tsCard();
        edCard.appendChild(tsH2(preset ? 'Edit Preset' : 'New Preset'));

        // Name
        var nameRow = document.createElement('div');
        nameRow.style.marginBottom = '8px';
        nameRow.appendChild(tsLabel('Name'));
        var nameInput = tsInput('text', preset ? preset.name : '', {});
        nameInput.id = 'tsPresetName';
        nameInput.style.width = '100%';
        nameRow.appendChild(nameInput);
        edCard.appendChild(nameRow);

        // Category
        var catRow = document.createElement('div');
        catRow.style.marginBottom = '8px';
        catRow.appendChild(tsLabel('Category'));
        var catSel = tsSelect(
            ['marketing', 'layout', 'commerce', 'interactive', 'content'],
            preset ? preset.category : 'content'
        );
        catSel.id = 'tsPresetCategory';
        catRow.appendChild(catSel);
        edCard.appendChild(catRow);

        // Description
        var descRow = document.createElement('div');
        descRow.style.marginBottom = '8px';
        descRow.appendChild(tsLabel('Description'));
        var descInput = tsInput('text', preset ? preset.description : '', {});
        descInput.id = 'tsPresetDesc';
        descInput.style.width = '100%';
        descRow.appendChild(descInput);
        edCard.appendChild(descRow);

        // Block editor
        var blockContainer = document.createElement('div');
        blockContainer.style.cssText = 'min-height:200px;border:1px solid var(--border);border-radius:6px;margin-bottom:12px;';
        edCard.appendChild(blockContainer);

        var blocks = [];
        if (preset) { try { blocks = JSON.parse(preset.blocks_json || '[]'); } catch(e) {} }
        if (typeof BlockEditor === 'function') {
            _presetBlockEditor = new BlockEditor(blockContainer, { blocks: blocks });
        } else {
            blockContainer.textContent = 'Block editor loading...';
        }

        // Dynamic bindings
        edCard.appendChild(tsH2('Dynamic Bindings'));
        var bindingsContainer = document.createElement('div');
        bindingsContainer.id = 'tsPresetBindings';
        edCard.appendChild(bindingsContainer);

        var currentBindings = (preset && preset.bindings) ? preset.bindings.slice() : [];
        function renderBindings() {
            bindingsContainer.replaceChildren();
            currentBindings.forEach(function(b, idx) {
                var row = document.createElement('div');
                row.style.cssText = 'display:flex;gap:4px;margin-bottom:4px;align-items:center;';
                var ph = tsInput('text', b.placeholder, {});
                ph.placeholder = '{{business.phone}}';
                ph.style.flex = '1';
                ph.onchange = function() { currentBindings[idx].placeholder = ph.value; };
                var src = tsInput('text', b.source, {});
                src.placeholder = 'site_config.phone';
                src.style.flex = '1';
                src.onchange = function() { currentBindings[idx].source = src.value; };
                var fb = tsInput('text', b.fallback, {});
                fb.placeholder = 'fallback';
                fb.style.flex = '1';
                fb.onchange = function() { currentBindings[idx].fallback = fb.value; };
                var delBtn = document.createElement('button');
                delBtn.textContent = '\u00d7';
                delBtn.style.cssText = 'background:none;border:none;cursor:pointer;color:var(--text-muted);font-size:16px;';
                delBtn.onclick = function() { currentBindings.splice(idx, 1); renderBindings(); };
                row.appendChild(ph);
                row.appendChild(src);
                row.appendChild(fb);
                row.appendChild(delBtn);
                bindingsContainer.appendChild(row);
            });
        }
        renderBindings();

        var addBindingBtn = tsBtnGhost('+ Add Binding', function() {
            currentBindings.push({ placeholder: '', source: '', fallback: '' });
            renderBindings();
        });
        edCard.appendChild(addBindingBtn);

        // Save / Cancel
        var btnRow = document.createElement('div');
        btnRow.style.cssText = 'display:flex;gap:8px;margin-top:12px;';
        btnRow.appendChild(tsBtn('Save Preset', async function() {
            var payload = {
                name: document.getElementById('tsPresetName').value,
                category: document.getElementById('tsPresetCategory').value,
                description: document.getElementById('tsPresetDesc').value,
                blocks_json: _presetBlockEditor ? JSON.stringify(_presetBlockEditor.getBlocks()) : '[]',
                bindings: currentBindings.filter(function(b) { return b.placeholder; })
            };
            var url, method;
            if (_editingPreset) {
                url = '/presets/' + _editingPreset.preset_id;
                method = 'PUT';
            } else {
                url = '/presets';
                method = 'POST';
            }
            var res = await tsApi(url, {
                method: method,
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify(payload)
            });
            if (res.ok) {
                if (typeof showToast === 'function') showToast('Preset saved', 'success');
                editor.style.display = 'none';
                await loadPresets();
            } else {
                if (typeof showToast === 'function') showToast('Save failed: ' + (res.message || ''), 'error');
            }
        }));
        btnRow.appendChild(tsBtnGhost('Cancel', function() {
            editor.style.display = 'none';
        }));
        edCard.appendChild(btnRow);

        editor.appendChild(edCard);
    }

    // ── AI context ──
    if (window.LiqAI && window.LiqAI.panel) {
        window.LiqAI.panel.register({
            featureKey: 'ts_presets',
            placeholder: "Describe a reusable section, e.g. 'Hero section for a bakery with seasonal specials and online ordering CTA'",
            creditCost: 3,
            properties: [
                { key: 'name', label: 'Preset Name', group: 'Preset', tip: 'Short name for this reusable section' },
                { key: 'category', label: 'Category', group: 'Preset', tip: 'marketing, layout, commerce, interactive, or content' },
                { key: 'description', label: 'Description', group: 'Preset', tip: 'What this preset is for and when to use it' },
                { key: 'blocks', label: 'Blocks', group: 'Content', tip: 'The block structure and content of this preset' }
            ],
            onResult: function(result, checkedKeys) {
                if (typeof result !== 'object') return;
                if (result.name && checkedKeys.indexOf('name') >= 0) {
                    var el = document.getElementById('tsPresetName');
                    if (el) el.value = result.name;
                }
                if (result.category && checkedKeys.indexOf('category') >= 0) {
                    var el = document.getElementById('tsPresetCategory');
                    if (el) el.value = result.category;
                }
                if (result.description && checkedKeys.indexOf('description') >= 0) {
                    var el = document.getElementById('tsPresetDesc');
                    if (el) el.value = result.description;
                }
                if (result.blocks && checkedKeys.indexOf('blocks') >= 0 && _presetBlockEditor) {
                    try {
                        var blocks = typeof result.blocks === 'string' ? JSON.parse(result.blocks) : result.blocks;
                        _presetBlockEditor.setBlocks(blocks);
                    } catch(e) {}
                }
                if (typeof showToast === 'function') showToast('AI preset generated', 'success');
            },
            captureState: function() { return {}; },
            restoreState: function() {}
        });
    }

    await loadPresets();
};
"####;
