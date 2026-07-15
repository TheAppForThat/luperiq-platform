//! Visual Layout Builder JS for Theme Studio.
//!
//! Provides the drag-and-drop block builder system reused by header, footer,
//! popup, and page layout editors. All JS uses DOM methods only (createElement,
//! textContent, replaceChildren). No innerHTML.

/// Return the layout builder JavaScript as a string.
/// This is prepended to admin_js() by admin.rs.
pub fn builder_js() -> &'static str {
    r##"
/* ══════════════════════════════════════════════════════════════════════
   Theme Studio: Visual Layout Builder
   ══════════════════════════════════════════════════════════════════════ */

/* ── Block type definitions (categories + defaults) ─────────────────── */

var TS_BLOCK_CATEGORIES = [
    { name: 'Navigation', types: ['site_brand','nav','mega_nav','nav_toggle','cta_group','user_menu'] },
    { name: 'Content', types: ['heading','paragraph','image','button','link','spacer','divider','icon','alert_box','quote','code','video','countdown','progress_bar','announcement','rotating_text','cta_bar','custom_html','newsletter_signup','coupon_code','popup_close','wp_content'] },
    { name: 'Form', types: ['text_input','email_input','phone_input','number_input','date_input','textarea','select','radio','checkbox'] }
];

var TS_BLOCK_LABELS = {
    site_brand:'Site Brand', nav:'Nav', mega_nav:'Mega Nav', nav_toggle:'Nav Toggle',
    cta_group:'CTA Group', user_menu:'User Menu', heading:'Heading', paragraph:'Paragraph',
    image:'Image', button:'Button', link:'Link', spacer:'Spacer', divider:'Divider',
    icon:'Icon', alert_box:'Alert Box', quote:'Quote', code:'Code', video:'Video',
    countdown:'Countdown', progress_bar:'Progress Bar', announcement:'Announcement',
    rotating_text:'Rotating Text',
    cta_bar:'CTA Bar', custom_html:'Custom HTML', newsletter_signup:'Newsletter Signup',
    coupon_code:'Coupon Code', popup_close:'Popup Close', wp_content:'WP Content',
    text_input:'Text Input', email_input:'Email Input', phone_input:'Phone Input',
    number_input:'Number Input', date_input:'Date Input', textarea:'Textarea',
    select:'Select', radio:'Radio', checkbox:'Checkbox'
};

function tsNewBlock(type) {
    var common = { type: type, enabled: true, tone: 'Surface', align: '', bg_color: '', text_color: '', font_size: 0, padding: 0, border_radius: 0 };
    switch (type) {
        case 'site_brand': return Object.assign(common, { name: '', subtitle: '', url: '/', show_logo: true, show_name: true, show_subtitle: false });
        case 'nav': return Object.assign(common, { mode: 'Auto', menu_location: '' });
        case 'mega_nav': return Object.assign(common, { nav_style: 'classic', mode: 'Auto', menu_location: '', panel_columns: 'auto', trigger: 'Hover', panel_width: 'full', max_depth: 3, show_descriptions: true, panel_mode: 'Expanded', desc_color: '', desc_font_size: 0, module_name_color: '' });
        case 'nav_toggle': return common;
        case 'cta_group': return Object.assign(common, { buttons: [] });
        case 'user_menu': return common;
        case 'heading': return Object.assign(common, { text: '', level: 2 });
        case 'paragraph': return Object.assign(common, { text: '' });
        case 'image': return Object.assign(common, { path: '', alt: '', max_width: null });
        case 'button': return Object.assign(common, { label: '', url: '', style: 'Primary' });
        case 'link': return Object.assign(common, { label: '', url: '' });
        case 'spacer': return Object.assign(common, { height: 20 });
        case 'divider': return Object.assign(common, { variant: 'Line' });
        case 'icon': return Object.assign(common, { name: '' });
        case 'alert_box': return Object.assign(common, { text: '', variant: 'Info' });
        case 'quote': return Object.assign(common, { text: '', attribution: null });
        case 'code': return Object.assign(common, { text: '' });
        case 'video': return Object.assign(common, { url: '' });
        case 'countdown': return Object.assign(common, { target: '' });
        case 'progress_bar': return Object.assign(common, { value: 0, max: 100, label: null });
        case 'announcement': return Object.assign(common, { text: '', url: null });
        case 'rotating_text': return Object.assign(common, {
            line_one: 'Build [[rotate]] websites.',
            line_two: 'Block wasteful bots. Publish one verified source.',
            swap_token: '[[rotate]]',
            words: ['faster', 'verified', 'launch-ready'],
            interval_ms: 2400,
            font_family: '',
            font_weight: '700',
            line_height: '1.1',
            letter_spacing: '0.08em',
            rotate_color: '',
            min_word_width_ch: 0,
            font_size: 13
        });
        case 'cta_bar': return Object.assign(common, { heading: '', subheading: '', buttons: [] });
        case 'custom_html': return Object.assign(common, { html: '' });
        case 'newsletter_signup': return Object.assign(common, { placeholder: 'Your email address', button_text: 'Subscribe' });
        case 'coupon_code': return Object.assign(common, { code: '', label: null });
        case 'popup_close': return common;
        case 'wp_content': return common;
        case 'text_input': return Object.assign(common, { label: '', name: '', placeholder: '', required: false });
        case 'email_input': return Object.assign(common, { label: '', name: '', placeholder: '', required: false });
        case 'phone_input': return Object.assign(common, { label: '', name: '', placeholder: '', required: false });
        case 'number_input': return Object.assign(common, { label: '', name: '', min: null, max: null, step: null, required: false });
        case 'date_input': return Object.assign(common, { label: '', name: '', required: false });
        case 'textarea': return Object.assign(common, { label: '', name: '', placeholder: '', rows: 4, required: false });
        case 'select': return Object.assign(common, { label: '', name: '', options: [], required: false });
        case 'radio': return Object.assign(common, { label: '', name: '', options: [], required: false });
        case 'checkbox': return Object.assign(common, { label: '', name: '' });
        default: return common;
    }
}

/* ── Tier gating ──────────────────────────────────────────────────── */
var _tsRole = (window.__CMS && window.__CMS.nexusRole) || '';
var _tsIsPro = _tsRole === 'central' || _tsRole === 'professional' || _tsRole === 'enterprise' || _tsRole === '';
var TS_CREDIT_PER_USD = 0.05;

/* ── AI content generation for Theme Studio — delegates to shared window.liqAiGenerateBlock() ── */
function tsAiGenerateBlock(blockType, userPrompt, callback) {
    window.liqAiGenerateBlock('theme_studio_ai_block', blockType, userPrompt)
        .then(function(data) { callback(null, data); })
        .catch(function(err) { callback(err, null); });
}

function tsShowAiPrompt(blockType, hooks) {
    if (typeof hooks === 'function') hooks = { onAccept: hooks };
    hooks = hooks || {};
    var overlay = document.createElement('div');
    overlay.className = 'ts-editor-overlay';
    overlay.style.zIndex = '10001';
    var panel = document.createElement('div');
    panel.className = 'ts-ai-panel';
    var accepted = false;
    var previewApplied = false;
    var latestResult = null;

    var header = document.createElement('div');
    header.className = 'ts-ai-header';
    header.textContent = '\u2728 AI Generate ' + (TS_BLOCK_LABELS[blockType] || blockType);
    panel.appendChild(header);

    var desc = document.createElement('div');
    desc.className = 'ts-ai-desc';
    desc.textContent = 'Describe what you want and AI will generate the content.';
    panel.appendChild(desc);

    var ta = document.createElement('textarea');
    ta.className = 'ts-ai-input';
    ta.rows = 4;
    ta.placeholder = 'e.g. "A bold heading for our hero section about premium service" or "Navigation with Home, Services, About, Contact links"';
    panel.appendChild(ta);

    var costNote = document.createElement('div');
    costNote.className = 'ts-ai-cost';
    var creditCost = 2;
    costNote.textContent = creditCost + ' AI credits (\u2248$' + (creditCost * TS_CREDIT_PER_USD).toFixed(2) + ') per generation';
    panel.appendChild(costNote);

    var previewNote = document.createElement('div');
    previewNote.className = 'ts-ai-desc';
    previewNote.style.display = 'none';
    panel.appendChild(previewNote);

    var feedback = document.createElement('div');
    panel.appendChild(feedback);

    function closePrompt(shouldRevert) {
        if (shouldRevert && !accepted && previewApplied && typeof hooks.onRevert === 'function') {
            hooks.onRevert();
        }
        if (overlay.parentNode) document.body.removeChild(overlay);
    }

    var btnRow = document.createElement('div');
    btnRow.className = 'ts-ai-btns';
    var cancelBtn = document.createElement('button');
    cancelBtn.className = 'ts-editor-cancel';
    cancelBtn.textContent = 'Cancel';
    cancelBtn.onclick = function() { closePrompt(true); };

    var genBtn = document.createElement('button');
    genBtn.className = 'ts-ai-gen-btn';
    genBtn.textContent = '\u2728 Generate';
    genBtn.onclick = function() {
        var prompt = ta.value.trim();
        if (!prompt) { ta.focus(); return; }
        genBtn.disabled = true;
        genBtn.textContent = 'Generating...';
        if (window.LiqAI && typeof window.LiqAI.generate === 'function') {
            window.LiqAI.generate({
                feature: 'theme_studio_ai_block',
                userInput: 'Block type: ' + blockType + '\nUser request: ' + prompt,
                container: feedback,
                onResult: function(result) {
                    latestResult = result;
                    if (typeof hooks.onPreview === 'function') {
                        hooks.onPreview(result);
                        previewApplied = true;
                        previewNote.textContent = 'Preview updated behind this panel. Keep it, retry it, or ask for help.';
                    } else {
                        previewApplied = false;
                        previewNote.textContent = 'Generated result is ready. Keep it to insert this block.';
                    }
                    previewNote.style.display = '';
                },
                onKeep: function() {
                    accepted = true;
                    if (!previewApplied) {
                        if (typeof hooks.onAccept === 'function') hooks.onAccept(latestResult);
                        else if (typeof hooks.onResult === 'function') hooks.onResult(latestResult);
                        if (typeof toast === 'function') toast('AI result kept.');
                        closePrompt(false);
                    } else if (typeof hooks.onKeep === 'function') {
                        hooks.onKeep(latestResult);
                    }
                    if (previewApplied && typeof toast === 'function') {
                        toast('AI result kept. Undo if you want the previous version back.');
                    }
                },
                onUndoKeep: typeof hooks.onRevert === 'function'
                    ? function() {
                        if (!previewApplied) return;
                        hooks.onRevert();
                        previewApplied = false;
                        previewNote.style.display = 'none';
                    }
                    : undefined,
                onDone: function() {
                    closePrompt(false);
                },
                onRetry: function() {
                    if (previewApplied && typeof hooks.onRevert === 'function') hooks.onRevert();
                    previewApplied = false;
                    previewNote.style.display = 'none';
                },
                onError: function(err) {
                    if (previewApplied && typeof hooks.onRevert === 'function') hooks.onRevert();
                    previewApplied = false;
                    previewNote.style.display = 'none';
                    if (typeof toast === 'function') toast('AI error: ' + err);
                }
            }).finally(function() {
                genBtn.disabled = false;
                genBtn.textContent = '\u2728 Generate';
            });
            return;
        }

        tsAiGenerateBlock(blockType, prompt, function(err, result, credits) {
            if (err) {
                genBtn.disabled = false;
                genBtn.textContent = '\u2728 Generate';
                if (typeof toast === 'function') toast('AI error: ' + err);
                return;
            }
            closePrompt(false);
            if (typeof toast === 'function') toast('AI generated! (' + (credits||2) + ' credits)');
            if (typeof hooks.onAccept === 'function') hooks.onAccept(result);
            else if (typeof hooks.onResult === 'function') hooks.onResult(result);
        });
    };
    btnRow.appendChild(cancelBtn);
    btnRow.appendChild(genBtn);
    panel.appendChild(btnRow);

    overlay.appendChild(panel);
    overlay.addEventListener('click', function(e) { if(e.target===overlay) closePrompt(true); });
    document.body.appendChild(overlay);
    ta.focus();
}

function tsShowAiBlockPicker(onPick) {
    var overlay = document.createElement('div');
    overlay.className = 'ts-editor-overlay';
    overlay.style.zIndex = '10001';
    var panel = document.createElement('div');
    panel.className = 'ts-ai-panel';

    var header = document.createElement('div');
    header.className = 'ts-ai-header';
    header.textContent = '\u2728 Choose Block Type for AI';
    panel.appendChild(header);

    var grid = document.createElement('div');
    grid.className = 'ts-ai-picker-grid';

    TS_BLOCK_CATEGORIES.forEach(function(cat) {
        cat.types.forEach(function(type) {
            var btn = document.createElement('button');
            btn.className = 'ts-ai-picker-btn';
            var lbl = document.createElement('span');
            lbl.textContent = TS_BLOCK_LABELS[type] || type;
            lbl.style.fontSize = '10px';
            btn.appendChild(lbl);
            btn.onclick = function() {
                document.body.removeChild(overlay);
                onPick(type);
            };
            grid.appendChild(btn);
        });
    });
    panel.appendChild(grid);

    var cancelBtn = document.createElement('button');
    cancelBtn.className = 'ts-editor-cancel';
    cancelBtn.style.cssText = 'margin:12px 16px;';
    cancelBtn.textContent = 'Cancel';
    cancelBtn.onclick = function() { document.body.removeChild(overlay); };
    panel.appendChild(cancelBtn);

    overlay.appendChild(panel);
    overlay.addEventListener('click', function(e) { if(e.target===overlay) document.body.removeChild(overlay); });
    document.body.appendChild(overlay);
}

/* ── Block Palette ──────────────────────────────────────────────────── */

function tsBlockPalette(onAiAdd) {
    var palette = document.createElement('div');
    palette.className = 'ts-palette';

    /* Upgrade banner at top for non-pro users */
    if (!_tsIsPro) {
        var upBanner = document.createElement('div');
        upBanner.className = 'ts-upgrade';
        var upTitle = document.createElement('div');
        upTitle.style.cssText = 'font-weight:600;color:#a78bfa;margin-bottom:4px;font-size:13px;';
        upTitle.textContent = 'Upgrade to Professional';
        upBanner.appendChild(upTitle);
        var upDesc = document.createElement('div');
        upDesc.style.cssText = 'font-size:11px;color:var(--text-muted);line-height:1.4;margin-bottom:8px;';
        upDesc.textContent = 'Unlock unlimited layouts, AI content generation, and advanced blocks.';
        upBanner.appendChild(upDesc);
        var upBtn = document.createElement('button');
        upBtn.className = 'ts-upgrade-btn';
        upBtn.textContent = 'Upgrade';
        upBtn.onclick = function() { if(typeof navigateTo==='function') navigateTo('store'); };
        upBanner.appendChild(upBtn);
        palette.appendChild(upBanner);
    }

    /* AI quick-add section */
    var aiSection = document.createElement('div');
    aiSection.className = 'ts-ai-section';
    var aiTitle = document.createElement('div');
    aiTitle.style.cssText = 'font-weight:600;color:#c4b5fd;font-size:12px;margin-bottom:4px;';
    aiTitle.textContent = '\u2728 AI Content';
    aiSection.appendChild(aiTitle);
    var aiBtn = document.createElement('button');
    aiBtn.className = 'ts-ai-quick-btn';
    aiBtn.textContent = '\u2728 Describe with AI';
    aiBtn.onclick = function() {
        tsShowAiBlockPicker(function(type) {
            tsShowAiPrompt(type, {
                onAccept: function(result) {
                    /* Merge AI result with common defaults */
                    var block = tsNewBlock(type);
                    if (typeof result === 'object') {
                        Object.keys(result).forEach(function(k) { block[k] = result[k]; });
                    }
                    if (typeof onAiAdd === 'function') onAiAdd(block);
                }
            });
        });
    };
    aiSection.appendChild(aiBtn);
    palette.appendChild(aiSection);

    TS_BLOCK_CATEGORIES.forEach(function(cat) {
        var catHeader = document.createElement('div');
        catHeader.className = 'ts-palette-cat';
        catHeader.textContent = cat.name;
        palette.appendChild(catHeader);

        var grid = document.createElement('div');
        grid.className = 'ts-palette-grid';

        cat.types.forEach(function(type) {
            var pill = document.createElement('div');
            pill.className = 'ts-palette-pill';
            pill.textContent = TS_BLOCK_LABELS[type] || type;
            pill.draggable = true;
            pill.addEventListener('dragstart', function(e) {
                e.dataTransfer.setData('application/ts-block', JSON.stringify({ source: 'palette', type: type }));
                e.dataTransfer.effectAllowed = 'copy';
                pill.classList.add('is-dragging');
            });
            pill.addEventListener('dragend', function() {
                pill.classList.remove('is-dragging');
            });
            grid.appendChild(pill);
        });
        palette.appendChild(grid);
    });

    return palette;
}

/* ── Layout Canvas ──────────────────────────────────────────────────── */

function tsLayoutCanvas(rows, onChange) {
    var canvas = document.createElement('div');
    canvas.className = 'ts-canvas';

    function rebuild() {
        while (canvas.firstChild) canvas.removeChild(canvas.firstChild);
        rows.forEach(function(row, ri) {
            canvas.appendChild(buildRowEl(row, ri));
        });
        var addRowBtn = document.createElement('button');
        addRowBtn.className = 'ts-canvas-add-row';
        addRowBtn.textContent = '+ Add Row';
        addRowBtn.addEventListener('click', function() {
            rows.push({ columns: [{ blocks: [] }] });
            onChange(rows);
            rebuild();
        });
        canvas.appendChild(addRowBtn);
    }

    function buildRowEl(row, ri) {
        var rowEl = document.createElement('div');
        rowEl.className = 'ts-canvas-row';

        /* Row header */
        var rowHeader = document.createElement('div');
        rowHeader.className = 'ts-canvas-row-header';

        var rowLabel = document.createElement('span');
        rowLabel.className = 'ts-canvas-row-label';
        rowLabel.textContent = 'Row ' + (ri + 1);
        rowHeader.appendChild(rowLabel);

        /* Column layout selector */
        var colBtns = document.createElement('div');
        colBtns.className = 'ts-col-btns';
        [1,2,3,4].forEach(function(n) {
            var cb = document.createElement('button');
            cb.className = 'ts-col-btn' + ((row.columns || []).length === n ? ' is-active' : '');
            cb.textContent = n;
            cb.title = n + ' column' + (n > 1 ? 's' : '');
            cb.addEventListener('click', function() {
                var cols = row.columns || [];
                while (cols.length < n) cols.push({ blocks: [] });
                if (cols.length > n) {
                    /* Merge extra column blocks into last kept column */
                    var overflow = cols.splice(n);
                    overflow.forEach(function(c) {
                        (c.blocks || []).forEach(function(b) { cols[n - 1].blocks.push(b); });
                    });
                }
                row.columns = cols;
                onChange(rows);
                rebuild();
            });
            colBtns.appendChild(cb);
        });
        rowHeader.appendChild(colBtns);

        /* Move row up/down */
        if (ri > 0) {
            var upBtn = document.createElement('button');
            upBtn.className = 'ts-row-move';
            upBtn.textContent = '\u2191';
            upBtn.title = 'Move up';
            upBtn.addEventListener('click', function() {
                var tmp = rows[ri - 1];
                rows[ri - 1] = rows[ri];
                rows[ri] = tmp;
                onChange(rows);
                rebuild();
            });
            rowHeader.appendChild(upBtn);
        }
        if (ri < rows.length - 1) {
            var downBtn = document.createElement('button');
            downBtn.className = 'ts-row-move';
            downBtn.textContent = '\u2193';
            downBtn.title = 'Move down';
            downBtn.addEventListener('click', function() {
                var tmp = rows[ri + 1];
                rows[ri + 1] = rows[ri];
                rows[ri] = tmp;
                onChange(rows);
                rebuild();
            });
            rowHeader.appendChild(downBtn);
        }

        /* Delete row */
        var delRow = document.createElement('button');
        delRow.className = 'ts-row-del';
        delRow.textContent = '\u00D7';
        delRow.title = 'Delete row';
        delRow.addEventListener('click', function() {
            if (!confirm('Delete Row ' + (ri + 1) + '?')) return;
            rows.splice(ri, 1);
            onChange(rows);
            rebuild();
        });
        rowHeader.appendChild(delRow);

        rowEl.appendChild(rowHeader);

        /* Columns */
        var colsWrap = document.createElement('div');
        colsWrap.className = 'ts-canvas-cols';
        colsWrap.style.gridTemplateColumns = 'repeat(' + (row.columns || []).length + ', 1fr)';

        (row.columns || []).forEach(function(col, ci) {
            var colEl = document.createElement('div');
            colEl.className = 'ts-canvas-col';

            /* Drop zone */
            colEl.addEventListener('dragover', function(e) {
                e.preventDefault();
                e.dataTransfer.dropEffect = 'copy';
                colEl.classList.add('is-dragover');
            });
            colEl.addEventListener('dragleave', function() {
                colEl.classList.remove('is-dragover');
            });
            colEl.addEventListener('drop', function(e) {
                e.preventDefault();
                colEl.classList.remove('is-dragover');
                var raw = e.dataTransfer.getData('application/ts-block');
                if (!raw) return;
                try {
                    var data = JSON.parse(raw);
                    if (data.source === 'palette') {
                        col.blocks.push(tsNewBlock(data.type));
                    } else if (data.source === 'canvas') {
                        /* Move: remove from old position */
                        var srcCol = rows[data.ri] && rows[data.ri].columns && rows[data.ri].columns[data.ci];
                        if (srcCol) {
                            var moved = srcCol.blocks.splice(data.bi, 1)[0];
                            if (moved) col.blocks.push(moved);
                        }
                    }
                    onChange(rows);
                    rebuild();
                } catch(ex) { /* ignore bad data */ }
            });

            /* Render blocks */
            (col.blocks || []).forEach(function(block, bi) {
                colEl.appendChild(buildBlockEl(block, ri, ci, bi));
            });

            if ((col.blocks || []).length === 0) {
                var emptyHint = document.createElement('div');
                emptyHint.className = 'ts-canvas-empty';
                emptyHint.textContent = 'Drop blocks here';
                colEl.appendChild(emptyHint);
            }

            colsWrap.appendChild(colEl);
        });

        rowEl.appendChild(colsWrap);
        return rowEl;
    }

    function buildBlockEl(block, ri, ci, bi) {
        var el = document.createElement('div');
        el.className = 'ts-canvas-block' + (block.enabled === false ? ' is-disabled' : '');
        el.draggable = true;

        el.addEventListener('dragstart', function(e) {
            e.dataTransfer.setData('application/ts-block', JSON.stringify({ source: 'canvas', ri: ri, ci: ci, bi: bi }));
            e.dataTransfer.effectAllowed = 'move';
            el.classList.add('is-dragging');
        });
        el.addEventListener('dragend', function() {
            el.classList.remove('is-dragging');
        });

        /* Drag handle */
        var handle = document.createElement('span');
        handle.className = 'ts-block-handle';
        handle.textContent = '\u2630';
        el.appendChild(handle);

        /* Block label */
        var label = document.createElement('span');
        label.className = 'ts-block-label';
        label.textContent = TS_BLOCK_LABELS[block.type] || block.type || '?';
        el.appendChild(label);

        /* Edit button */
        var editBtn = document.createElement('button');
        editBtn.className = 'ts-block-edit';
        editBtn.textContent = '\u270E';
        editBtn.title = 'Edit';
        editBtn.addEventListener('click', function() {
            tsOpenBlockEditor(block, function(updated) {
                rows[ri].columns[ci].blocks[bi] = updated;
                onChange(rows);
                rebuild();
            });
        });
        el.appendChild(editBtn);

        /* Delete button */
        var delBtn = document.createElement('button');
        delBtn.className = 'ts-block-del';
        delBtn.textContent = '\u00D7';
        delBtn.title = 'Delete';
        delBtn.addEventListener('click', function() {
            rows[ri].columns[ci].blocks.splice(bi, 1);
            onChange(rows);
            rebuild();
        });
        el.appendChild(delBtn);

        return el;
    }

    canvas.tsAddBlock = function(block) {
        if (!rows.length) rows.push({ columns: [{ blocks: [] }] });
        var lastRow = rows[rows.length - 1];
        if (!lastRow.columns || !lastRow.columns.length) lastRow.columns = [{ blocks: [] }];
        if (!Array.isArray(lastRow.columns[0].blocks)) lastRow.columns[0].blocks = [];
        lastRow.columns[0].blocks.push(JSON.parse(JSON.stringify(block)));
        onChange(rows);
        rebuild();
    };

    rebuild();
    return canvas;
}

/* ── Block Property Editor (modal overlay) ──────────────────────────── */

function tsOpenBlockEditor(block, onSave) {
    /* Create overlay */
    var overlay = document.createElement('div');
    overlay.className = 'ts-editor-overlay';

    var panel = document.createElement('div');
    panel.className = 'ts-editor-panel';

    /* Header */
    var header = document.createElement('div');
    header.className = 'ts-editor-header';
    var title = document.createElement('h3');
    title.textContent = 'Edit: ' + (TS_BLOCK_LABELS[block.type] || block.type);
    header.appendChild(title);
    var closeBtn = document.createElement('button');
    closeBtn.className = 'ts-editor-close';
    closeBtn.textContent = '\u00D7';
    closeBtn.addEventListener('click', function() {
        document.body.removeChild(overlay);
    });
    header.appendChild(closeBtn);
    panel.appendChild(header);

    /* Body (scrollable) */
    var body = document.createElement('div');
    body.className = 'ts-editor-body';

    /* Clone the block so we edit a copy */
    var edited = JSON.parse(JSON.stringify(block));

    /* Common fields */
    var commonTitle = document.createElement('h4');
    commonTitle.textContent = 'Common';
    commonTitle.style.cssText = 'margin:0 0 8px;font-size:13px;color:var(--text-muted);';
    body.appendChild(commonTitle);

    body.appendChild(tsEditorCheckbox('Enabled', edited.enabled !== false, function(v) { edited.enabled = v; }));
    body.appendChild(tsEditorSelect('Tone', ['Surface','Accent','Primary','Muted'], edited.tone || 'Surface', function(v) { edited.tone = v; }));
    body.appendChild(tsEditorSelect('Align', ['','Left','Center','Right'], edited.align || '', function(v) { edited.align = v; }));
    body.appendChild(tsEditorColor('BG Color', edited.bg_color || '', function(v) { edited.bg_color = v; }));
    body.appendChild(tsEditorColor('Text Color', edited.text_color || '', function(v) { edited.text_color = v; }));
    body.appendChild(tsEditorNumber('Font Size', edited.font_size || 0, 0, 100, function(v) { edited.font_size = v; }));
    body.appendChild(tsEditorNumber('Padding', edited.padding || 0, 0, 100, function(v) { edited.padding = v; }));
    body.appendChild(tsEditorNumber('Border Radius', edited.border_radius || 0, 0, 100, function(v) { edited.border_radius = v; }));

    /* Type-specific fields */
    var typeTitle = document.createElement('h4');
    typeTitle.textContent = TS_BLOCK_LABELS[edited.type] || edited.type;
    typeTitle.style.cssText = 'margin:16px 0 8px;font-size:13px;color:var(--text-muted);border-top:1px solid var(--border);padding-top:12px;';
    body.appendChild(typeTitle);

    tsEditorTypeFields(edited, body);

    panel.appendChild(body);

    /* Footer with AI Fill / Save / Cancel */
    var footer = document.createElement('div');
    footer.className = 'ts-editor-footer';

    var aiEditBtn = document.createElement('button');
    aiEditBtn.className = 'ts-ai-edit-btn';
    aiEditBtn.textContent = '\u2728 AI Fill';
    aiEditBtn.title = 'Describe what you want and AI will fill this block';
    aiEditBtn.addEventListener('click', function() {
        var originalEdited = JSON.parse(JSON.stringify(edited));
        function rebuildEditedBody() {
            while (body.firstChild) body.removeChild(body.firstChild);
            body.appendChild(commonTitle);
            body.appendChild(tsEditorCheckbox('Enabled', edited.enabled !== false, function(v) { edited.enabled = v; }));
            body.appendChild(tsEditorSelect('Tone', ['Surface','Accent','Primary','Muted'], edited.tone || 'Surface', function(v) { edited.tone = v; }));
            body.appendChild(tsEditorSelect('Align', ['','Left','Center','Right'], edited.align || '', function(v) { edited.align = v; }));
            body.appendChild(tsEditorColor('BG Color', edited.bg_color || '', function(v) { edited.bg_color = v; }));
            body.appendChild(tsEditorColor('Text Color', edited.text_color || '', function(v) { edited.text_color = v; }));
            body.appendChild(tsEditorNumber('Font Size', edited.font_size || 0, 0, 100, function(v) { edited.font_size = v; }));
            body.appendChild(tsEditorNumber('Padding', edited.padding || 0, 0, 100, function(v) { edited.padding = v; }));
            body.appendChild(tsEditorNumber('Border Radius', edited.border_radius || 0, 0, 100, function(v) { edited.border_radius = v; }));
            var tt = document.createElement('h4');
            tt.textContent = TS_BLOCK_LABELS[edited.type] || edited.type;
            tt.style.cssText = 'margin:16px 0 8px;font-size:13px;color:var(--text-muted);border-top:1px solid var(--border);padding-top:12px;';
            body.appendChild(tt);
            tsEditorTypeFields(edited, body);
        }
        tsShowAiPrompt(edited.type, {
            onPreview: function(result) {
                if (typeof result === 'object') {
                    Object.keys(result).forEach(function(k) { edited[k] = result[k]; });
                }
                rebuildEditedBody();
            },
            onRevert: function() {
                Object.keys(edited).forEach(function(k) { delete edited[k]; });
                Object.keys(originalEdited).forEach(function(k) { edited[k] = originalEdited[k]; });
                rebuildEditedBody();
            }
        });
    });
    footer.appendChild(aiEditBtn);

    var saveBtn = document.createElement('button');
    saveBtn.className = 'ts-editor-save';
    saveBtn.textContent = 'Save';
    saveBtn.addEventListener('click', function() {
        onSave(edited);
        document.body.removeChild(overlay);
    });
    footer.appendChild(saveBtn);
    var cancelBtn = document.createElement('button');
    cancelBtn.className = 'ts-editor-cancel';
    cancelBtn.textContent = 'Cancel';
    cancelBtn.addEventListener('click', function() {
        document.body.removeChild(overlay);
    });
    footer.appendChild(cancelBtn);
    panel.appendChild(footer);

    overlay.appendChild(panel);
    document.body.appendChild(overlay);
}

/* ── Editor field helpers ───────────────────────────────────────────── */

function tsEditorRow(labelText, inputEl) {
    var row = document.createElement('div');
    row.className = 'ts-editor-row';
    var lbl = document.createElement('label');
    lbl.textContent = labelText;
    row.appendChild(lbl);
    row.appendChild(inputEl);
    return row;
}

function tsEditorText(label, value, onChange) {
    var inp = document.createElement('input');
    inp.type = 'text';
    inp.value = value || '';
    inp.className = 'ts-editor-input';
    inp.addEventListener('input', function() { onChange(inp.value); });
    return tsEditorRow(label, inp);
}

function tsEditorTextarea(label, value, onChange) {
    var ta = document.createElement('textarea');
    ta.value = value || '';
    ta.className = 'ts-editor-input';
    ta.rows = 4;
    ta.addEventListener('input', function() { onChange(ta.value); });
    return tsEditorRow(label, ta);
}

function tsEditorNumber(label, value, min, max, onChange) {
    var inp = document.createElement('input');
    inp.type = 'number';
    inp.value = value !== undefined && value !== null ? value : '';
    inp.className = 'ts-editor-input ts-editor-num';
    if (min !== undefined) inp.min = min;
    if (max !== undefined) inp.max = max;
    inp.addEventListener('input', function() {
        var v = inp.value === '' ? 0 : parseInt(inp.value, 10);
        onChange(isNaN(v) ? 0 : v);
    });
    return tsEditorRow(label, inp);
}

function tsEditorColor(label, value, onChange) {
    var wrap = document.createElement('div');
    wrap.style.cssText = 'display:flex;gap:6px;align-items:center;';
    var inp = document.createElement('input');
    inp.type = 'color';
    inp.value = value || '#000000';
    inp.className = 'ts-editor-color';
    inp.addEventListener('input', function() { onChange(inp.value); });
    var clear = document.createElement('button');
    clear.textContent = '\u00D7';
    clear.className = 'ts-editor-clear';
    clear.title = 'Clear';
    clear.addEventListener('click', function() {
        inp.value = '#000000';
        onChange('');
    });
    wrap.appendChild(inp);
    wrap.appendChild(clear);
    return tsEditorRow(label, wrap);
}

function tsEditorCheckbox(label, value, onChange) {
    var cb = document.createElement('input');
    cb.type = 'checkbox';
    cb.checked = !!value;
    cb.addEventListener('change', function() { onChange(cb.checked); });
    return tsEditorRow(label, cb);
}

function tsEditorSelect(label, options, selected, onChange) {
    var sel = document.createElement('select');
    sel.className = 'ts-editor-input';
    options.forEach(function(o) {
        var opt = document.createElement('option');
        opt.value = o;
        opt.textContent = o || '(None)';
        if (o === selected) opt.selected = true;
        sel.appendChild(opt);
    });
    sel.addEventListener('change', function() { onChange(sel.value); });
    return tsEditorRow(label, sel);
}

/* ── CTA Button array editor ───────────────────────────────────────── */

function tsEditorButtons(label, buttons, onChange) {
    var wrap = document.createElement('div');
    wrap.style.cssText = 'margin-bottom:8px;';
    var lbl = document.createElement('label');
    lbl.textContent = label;
    lbl.style.cssText = 'display:block;font-size:12px;color:var(--text-muted);margin-bottom:4px;';
    wrap.appendChild(lbl);

    var list = document.createElement('div');

    function rebuildList() {
        while (list.firstChild) list.removeChild(list.firstChild);
        buttons.forEach(function(btn, i) {
            var item = document.createElement('div');
            item.style.cssText = 'display:flex;gap:4px;margin-bottom:4px;align-items:center;';

            var lblInp = document.createElement('input');
            lblInp.type = 'text';
            lblInp.value = btn.label || '';
            lblInp.placeholder = 'Label';
            lblInp.className = 'ts-editor-input';
            lblInp.style.flex = '1';
            lblInp.addEventListener('input', function() { btn.label = lblInp.value; onChange(buttons); });

            var urlInp = document.createElement('input');
            urlInp.type = 'text';
            urlInp.value = btn.url || '';
            urlInp.placeholder = 'URL';
            urlInp.className = 'ts-editor-input';
            urlInp.style.flex = '1';
            urlInp.addEventListener('input', function() { btn.url = urlInp.value; onChange(buttons); });

            var styleSel = document.createElement('select');
            styleSel.className = 'ts-editor-input';
            ['Primary','Outline','Ghost'].forEach(function(s) {
                var o = document.createElement('option');
                o.value = s; o.textContent = s;
                if ((btn.style || 'Primary') === s) o.selected = true;
                styleSel.appendChild(o);
            });
            styleSel.addEventListener('change', function() { btn.style = styleSel.value; onChange(buttons); });

            var del = document.createElement('button');
            del.textContent = '\u00D7';
            del.className = 'ts-editor-clear';
            del.addEventListener('click', function() { buttons.splice(i, 1); onChange(buttons); rebuildList(); });

            item.appendChild(lblInp);
            item.appendChild(urlInp);
            item.appendChild(styleSel);
            item.appendChild(del);
            list.appendChild(item);
        });
    }

    rebuildList();
    wrap.appendChild(list);

    var addBtn = document.createElement('button');
    addBtn.textContent = '+ Add Button';
    addBtn.className = 'ts-editor-add';
    addBtn.addEventListener('click', function() {
        buttons.push({ label: '', url: '', style: 'Primary' });
        onChange(buttons);
        rebuildList();
    });
    wrap.appendChild(addBtn);

    return wrap;
}

/* ── String array editor (for select/radio options) ─────────────────── */

function tsEditorStringArray(label, arr, onChange) {
    var wrap = document.createElement('div');
    wrap.style.cssText = 'margin-bottom:8px;';
    var lbl = document.createElement('label');
    lbl.textContent = label;
    lbl.style.cssText = 'display:block;font-size:12px;color:var(--text-muted);margin-bottom:4px;';
    wrap.appendChild(lbl);

    var list = document.createElement('div');

    function rebuildList() {
        while (list.firstChild) list.removeChild(list.firstChild);
        arr.forEach(function(val, i) {
            var item = document.createElement('div');
            item.style.cssText = 'display:flex;gap:4px;margin-bottom:4px;align-items:center;';
            var inp = document.createElement('input');
            inp.type = 'text';
            inp.value = val;
            inp.className = 'ts-editor-input';
            inp.style.flex = '1';
            inp.addEventListener('input', function() { arr[i] = inp.value; onChange(arr); });
            var del = document.createElement('button');
            del.textContent = '\u00D7';
            del.className = 'ts-editor-clear';
            del.addEventListener('click', function() { arr.splice(i, 1); onChange(arr); rebuildList(); });
            item.appendChild(inp);
            item.appendChild(del);
            list.appendChild(item);
        });
    }

    rebuildList();
    wrap.appendChild(list);

    var addBtn = document.createElement('button');
    addBtn.textContent = '+ Add Option';
    addBtn.className = 'ts-editor-add';
    addBtn.addEventListener('click', function() { arr.push(''); onChange(arr); rebuildList(); });
    wrap.appendChild(addBtn);

    return wrap;
}

/* ── Type-specific field generators ─────────────────────────────────── */

function tsEditorTypeFields(block, body) {
    switch (block.type) {
        case 'site_brand':
            body.appendChild(tsEditorText('Name', block.name, function(v) { block.name = v; }));
            body.appendChild(tsEditorText('Subtitle', block.subtitle, function(v) { block.subtitle = v; }));
            body.appendChild(tsEditorText('URL', block.url, function(v) { block.url = v; }));
            body.appendChild(tsEditorCheckbox('Show Logo', block.show_logo, function(v) { block.show_logo = v; }));
            body.appendChild(tsEditorCheckbox('Show Name', block.show_name, function(v) { block.show_name = v; }));
            body.appendChild(tsEditorCheckbox('Show Subtitle', block.show_subtitle, function(v) { block.show_subtitle = v; }));
            break;
        case 'nav':
            body.appendChild(tsEditorSelect('Mode', ['Auto','Menu','Inherit'], block.mode || 'Auto', function(v) { block.mode = v; }));
            body.appendChild(tsEditorText('Menu Location', block.menu_location, function(v) { block.menu_location = v; }));
            break;
        case 'mega_nav':
            body.appendChild(tsEditorSelect('Navigation Style', ['classic','simple_bar','two_row','full_width_mega','card_grid_mega','side_drawer','command_palette'], block.nav_style || 'classic', function(v) { block.nav_style = v; }));
            body.appendChild(tsEditorSelect('Mode', ['Auto','Menu','Inherit'], block.mode || 'Auto', function(v) { block.mode = v; }));
            body.appendChild(tsEditorText('Menu Location', block.menu_location, function(v) { block.menu_location = v; }));
            body.appendChild(tsEditorText('Panel Columns', block.panel_columns, function(v) { block.panel_columns = v; }));
            body.appendChild(tsEditorSelect('Trigger', ['Hover','Click'], block.trigger || 'Hover', function(v) { block.trigger = v; }));
            body.appendChild(tsEditorText('Panel Width', block.panel_width, function(v) { block.panel_width = v; }));
            body.appendChild(tsEditorNumber('Max Depth', block.max_depth, 1, 6, function(v) { block.max_depth = v; }));
            body.appendChild(tsEditorCheckbox('Show Descriptions', block.show_descriptions, function(v) { block.show_descriptions = v; }));
            body.appendChild(tsEditorSelect('Panel Mode', ['Expanded','Tabbed'], block.panel_mode || 'Expanded', function(v) { block.panel_mode = v; }));
            body.appendChild(tsEditorColor('Desc Color', block.desc_color, function(v) { block.desc_color = v; }));
            body.appendChild(tsEditorNumber('Desc Font Size', block.desc_font_size, 0, 60, function(v) { block.desc_font_size = v; }));
            body.appendChild(tsEditorColor('Module Name Color', block.module_name_color, function(v) { block.module_name_color = v; }));
            break;
        case 'cta_group':
            body.appendChild(tsEditorButtons('Buttons', block.buttons || [], function(v) { block.buttons = v; }));
            break;
        case 'heading':
            body.appendChild(tsEditorText('Text', block.text, function(v) { block.text = v; }));
            body.appendChild(tsEditorNumber('Level (1-6)', block.level, 1, 6, function(v) { block.level = v; }));
            break;
        case 'paragraph':
            body.appendChild(tsEditorTextarea('Text', block.text, function(v) { block.text = v; }));
            break;
        case 'image':
            body.appendChild(tsEditorText('Path', block.path, function(v) { block.path = v; }));
            body.appendChild(tsEditorText('Alt Text', block.alt, function(v) { block.alt = v; }));
            body.appendChild(tsEditorNumber('Max Width', block.max_width || 0, 0, 2000, function(v) { block.max_width = v || null; }));
            break;
        case 'button':
            body.appendChild(tsEditorText('Label', block.label, function(v) { block.label = v; }));
            body.appendChild(tsEditorText('URL', block.url, function(v) { block.url = v; }));
            body.appendChild(tsEditorSelect('Style', ['Primary','Outline','Ghost'], block.style || 'Primary', function(v) { block.style = v; }));
            break;
        case 'link':
            body.appendChild(tsEditorText('Label', block.label, function(v) { block.label = v; }));
            body.appendChild(tsEditorText('URL', block.url, function(v) { block.url = v; }));
            break;
        case 'spacer':
            body.appendChild(tsEditorNumber('Height', block.height, 0, 500, function(v) { block.height = v; }));
            break;
        case 'divider':
            body.appendChild(tsEditorSelect('Variant', ['Line','Dashed','Dotted'], block.variant || 'Line', function(v) { block.variant = v; }));
            break;
        case 'icon':
            body.appendChild(tsEditorText('Icon Name', block.name, function(v) { block.name = v; }));
            break;
        case 'alert_box':
            body.appendChild(tsEditorText('Text', block.text, function(v) { block.text = v; }));
            body.appendChild(tsEditorSelect('Variant', ['Info','Success','Warning','Error'], block.variant || 'Info', function(v) { block.variant = v; }));
            break;
        case 'quote':
            body.appendChild(tsEditorTextarea('Text', block.text, function(v) { block.text = v; }));
            body.appendChild(tsEditorText('Attribution', block.attribution || '', function(v) { block.attribution = v || null; }));
            break;
        case 'code':
            body.appendChild(tsEditorTextarea('Code', block.text, function(v) { block.text = v; }));
            break;
        case 'video':
            body.appendChild(tsEditorText('URL', block.url, function(v) { block.url = v; }));
            break;
        case 'countdown':
            body.appendChild(tsEditorText('Target (ISO date)', block.target, function(v) { block.target = v; }));
            break;
        case 'progress_bar':
            body.appendChild(tsEditorNumber('Value', block.value, 0, 10000, function(v) { block.value = v; }));
            body.appendChild(tsEditorNumber('Max', block.max, 1, 10000, function(v) { block.max = v; }));
            body.appendChild(tsEditorText('Label', block.label || '', function(v) { block.label = v || null; }));
            break;
        case 'announcement':
            body.appendChild(tsEditorText('Text', block.text, function(v) { block.text = v; }));
            body.appendChild(tsEditorText('URL', block.url || '', function(v) { block.url = v || null; }));
            break;
        case 'rotating_text':
            body.appendChild(tsEditorText('Line One', block.line_one, function(v) { block.line_one = v; }));
            body.appendChild(tsEditorText('Line Two', block.line_two, function(v) { block.line_two = v; }));
            body.appendChild(tsEditorText('Swap Token', block.swap_token || '[[rotate]]', function(v) { block.swap_token = v || '[[rotate]]'; }));
            body.appendChild(tsEditorStringArray('Rotating Words', block.words || [], function(v) { block.words = v; }));
            body.appendChild(tsEditorNumber('Interval (ms)', block.interval_ms || 2400, 1200, 20000, function(v) { block.interval_ms = v; }));
            body.appendChild(tsEditorText('Font Family', block.font_family || '', function(v) { block.font_family = v; }));
            body.appendChild(tsEditorText('Font Weight', block.font_weight || '', function(v) { block.font_weight = v; }));
            body.appendChild(tsEditorText('Line Height', block.line_height || '', function(v) { block.line_height = v; }));
            body.appendChild(tsEditorText('Letter Spacing', block.letter_spacing || '', function(v) { block.letter_spacing = v; }));
            body.appendChild(tsEditorColor('Swap Word Color', block.rotate_color || '', function(v) { block.rotate_color = v; }));
            body.appendChild(tsEditorNumber('Min Word Width (ch)', block.min_word_width_ch || 0, 0, 24, function(v) { block.min_word_width_ch = v; }));
            break;
        case 'cta_bar':
            body.appendChild(tsEditorText('Heading', block.heading, function(v) { block.heading = v; }));
            body.appendChild(tsEditorText('Subheading', block.subheading, function(v) { block.subheading = v; }));
            body.appendChild(tsEditorButtons('Buttons', block.buttons || [], function(v) { block.buttons = v; }));
            break;
        case 'custom_html':
            body.appendChild(tsEditorTextarea('HTML', block.html, function(v) { block.html = v; }));
            break;
        case 'newsletter_signup':
            body.appendChild(tsEditorText('Placeholder', block.placeholder, function(v) { block.placeholder = v; }));
            body.appendChild(tsEditorText('Button Text', block.button_text, function(v) { block.button_text = v; }));
            break;
        case 'coupon_code':
            body.appendChild(tsEditorText('Code', block.code, function(v) { block.code = v; }));
            body.appendChild(tsEditorText('Label', block.label || '', function(v) { block.label = v || null; }));
            break;
        case 'text_input': case 'email_input': case 'phone_input':
            body.appendChild(tsEditorText('Label', block.label, function(v) { block.label = v; }));
            body.appendChild(tsEditorText('Name', block.name, function(v) { block.name = v; }));
            body.appendChild(tsEditorText('Placeholder', block.placeholder, function(v) { block.placeholder = v; }));
            body.appendChild(tsEditorCheckbox('Required', block.required, function(v) { block.required = v; }));
            break;
        case 'number_input':
            body.appendChild(tsEditorText('Label', block.label, function(v) { block.label = v; }));
            body.appendChild(tsEditorText('Name', block.name, function(v) { block.name = v; }));
            body.appendChild(tsEditorNumber('Min', block.min, undefined, undefined, function(v) { block.min = v || null; }));
            body.appendChild(tsEditorNumber('Max', block.max, undefined, undefined, function(v) { block.max = v || null; }));
            body.appendChild(tsEditorNumber('Step', block.step, undefined, undefined, function(v) { block.step = v || null; }));
            body.appendChild(tsEditorCheckbox('Required', block.required, function(v) { block.required = v; }));
            break;
        case 'date_input':
            body.appendChild(tsEditorText('Label', block.label, function(v) { block.label = v; }));
            body.appendChild(tsEditorText('Name', block.name, function(v) { block.name = v; }));
            body.appendChild(tsEditorCheckbox('Required', block.required, function(v) { block.required = v; }));
            break;
        case 'textarea':
            body.appendChild(tsEditorText('Label', block.label, function(v) { block.label = v; }));
            body.appendChild(tsEditorText('Name', block.name, function(v) { block.name = v; }));
            body.appendChild(tsEditorText('Placeholder', block.placeholder, function(v) { block.placeholder = v; }));
            body.appendChild(tsEditorNumber('Rows', block.rows, 1, 20, function(v) { block.rows = v; }));
            body.appendChild(tsEditorCheckbox('Required', block.required, function(v) { block.required = v; }));
            break;
        case 'select':
            body.appendChild(tsEditorText('Label', block.label, function(v) { block.label = v; }));
            body.appendChild(tsEditorText('Name', block.name, function(v) { block.name = v; }));
            body.appendChild(tsEditorStringArray('Options', block.options || [], function(v) { block.options = v; }));
            body.appendChild(tsEditorCheckbox('Required', block.required, function(v) { block.required = v; }));
            break;
        case 'radio':
            body.appendChild(tsEditorText('Label', block.label, function(v) { block.label = v; }));
            body.appendChild(tsEditorText('Name', block.name, function(v) { block.name = v; }));
            body.appendChild(tsEditorStringArray('Options', block.options || [], function(v) { block.options = v; }));
            body.appendChild(tsEditorCheckbox('Required', block.required, function(v) { block.required = v; }));
            break;
        case 'checkbox':
            body.appendChild(tsEditorText('Label', block.label, function(v) { block.label = v; }));
            body.appendChild(tsEditorText('Name', block.name, function(v) { block.name = v; }));
            break;
        /* nav_toggle, user_menu, popup_close, wp_content have no type-specific fields */
    }
}

/* ── Template Picker ────────────────────────────────────────────────── */

function tsTemplatePicker(layoutType, currentRows, onApply) {
    var wrap = document.createElement('div');
    wrap.className = 'ts-templates';

    var header = document.createElement('div');
    header.style.cssText = 'display:flex;gap:8px;margin-bottom:12px;';

    var saveBtn = document.createElement('button');
    saveBtn.className = 'ts-editor-save';
    saveBtn.style.fontSize = '12px';
    saveBtn.textContent = 'Save as Template';
    saveBtn.addEventListener('click', async function() {
        var name = prompt('Template name:');
        if (!name) return;
        var r = await tsApi('/templates/' + layoutType, {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ id: name.toLowerCase().replace(/[^a-z0-9]+/g, '-'), rows: currentRows })
        });
        if (typeof showToast === 'function') showToast(r.ok ? 'Template saved' : (r.message || 'Error'), r.ok ? 'success' : 'error');
        if (r.ok) loadTemplates();
    });
    header.appendChild(saveBtn);

    var loadBtn = document.createElement('button');
    loadBtn.className = 'ts-editor-cancel';
    loadBtn.style.fontSize = '12px';
    loadBtn.textContent = 'Browse Templates';
    loadBtn.addEventListener('click', function() {
        templateList.style.display = templateList.style.display === 'none' ? 'block' : 'none';
        if (templateList.style.display !== 'none') loadTemplates();
    });
    header.appendChild(loadBtn);

    wrap.appendChild(header);

    var templateList = document.createElement('div');
    templateList.style.display = 'none';
    templateList.className = 'ts-template-list';

    async function loadTemplates() {
        while (templateList.firstChild) templateList.removeChild(templateList.firstChild);
        var res = await tsApi('/templates/' + layoutType);
        var templates = (res.ok && res.data) ? res.data : [];
        if (templates.length === 0) {
            var empty = document.createElement('div');
            empty.className = 'ts-empty';
            empty.style.padding = '12px';
            empty.textContent = 'No saved templates for ' + layoutType + '.';
            templateList.appendChild(empty);
            return;
        }
        templates.forEach(function(tpl) {
            var card = document.createElement('div');
            card.className = 'ts-template-card';

            var info = document.createElement('div');
            var nameEl = document.createElement('strong');
            nameEl.textContent = tpl.id;
            info.appendChild(nameEl);
            var summary = document.createElement('span');
            summary.style.cssText = 'margin-left:8px;font-size:12px;color:var(--text-muted);';
            var blockCount = 0;
            (tpl.rows || []).forEach(function(r) {
                (r.columns || []).forEach(function(c) { blockCount += (c.blocks || []).length; });
            });
            summary.textContent = (tpl.rows || []).length + ' rows, ' + blockCount + ' blocks';
            info.appendChild(summary);
            card.appendChild(info);

            var btns = document.createElement('div');
            btns.style.cssText = 'display:flex;gap:4px;';
            var applyBtn = document.createElement('button');
            applyBtn.className = 'ts-editor-save';
            applyBtn.style.fontSize = '11px';
            applyBtn.textContent = 'Apply';
            applyBtn.addEventListener('click', function() {
                if (!confirm('Replace current layout with template "' + tpl.id + '"?')) return;
                onApply(JSON.parse(JSON.stringify(tpl.rows || [])));
                templateList.style.display = 'none';
            });
            btns.appendChild(applyBtn);

            var delBtn = document.createElement('button');
            delBtn.className = 'ts-editor-clear';
            delBtn.textContent = '\u00D7';
            delBtn.addEventListener('click', async function() {
                if (!confirm('Delete template "' + tpl.id + '"?')) return;
                var r = await tsApi('/templates/' + layoutType + '/' + encodeURIComponent(tpl.id), { method: 'DELETE' });
                if (typeof showToast === 'function') showToast(r.ok ? 'Deleted' : (r.message || 'Error'), r.ok ? 'success' : 'error');
                if (r.ok) loadTemplates();
            });
            btns.appendChild(delBtn);

            card.appendChild(btns);
            templateList.appendChild(card);
        });
    }

    wrap.appendChild(templateList);
    return wrap;
}
"##
}

/// Return CSS for the layout builder UI components.
pub fn builder_css() -> &'static str {
    r##"
/* ── Upgrade Banner + AI Section ─────────────────────────────────────── */
.ts-upgrade {
    margin-bottom: 10px; padding: 10px 12px; background: linear-gradient(135deg, rgba(139,92,246,0.08), rgba(139,92,246,0.15));
    border: 1px solid rgba(139,92,246,0.3); border-radius: 8px;
}
.ts-upgrade-btn {
    background: #8b5cf6; color: #fff; border: none; border-radius: 6px;
    padding: 6px 14px; font-size: 11px; font-weight: 600; cursor: pointer; width: 100%;
}
.ts-upgrade-btn:hover { background: #7c3aed; }
.ts-ai-section {
    margin-bottom: 10px; padding: 10px 12px;
    background: linear-gradient(135deg, rgba(99,102,241,0.08), rgba(139,92,246,0.15));
    border: 1px solid rgba(99,102,241,0.25); border-radius: 8px;
}
.ts-ai-quick-btn {
    background: linear-gradient(135deg, #7c3aed, #6d28d9); color: #fff;
    border: none; border-radius: 6px; padding: 6px 14px; font-size: 11px;
    font-weight: 600; cursor: pointer; width: 100%;
}
.ts-ai-quick-btn:hover { background: linear-gradient(135deg, #8b5cf6, #7c3aed); }
.ts-ai-panel {
    background: var(--surface, #1e293b); border-radius: 12px; width: 480px;
    max-width: 90vw; box-shadow: 0 20px 60px rgba(0,0,0,0.4); overflow: hidden;
}
.ts-ai-header {
    padding: 14px 18px; font-size: 15px; font-weight: 700; color: var(--text, #e2e8f0);
    border-bottom: 1px solid var(--border);
}
.ts-ai-desc { padding: 10px 18px 0; font-size: 12px; color: var(--text-muted); }
.ts-ai-input {
    display: block; width: calc(100% - 36px); margin: 10px 18px;
    background: var(--bg); color: var(--text); border: 1px solid var(--border);
    border-radius: 6px; padding: 8px 10px; font-size: 13px; font-family: inherit;
    resize: vertical; min-height: 70px;
}
.ts-ai-input:focus { border-color: #7c3aed; outline: none; }
.ts-ai-cost { padding: 0 18px; font-size: 11px; color: var(--text-muted); }
.ts-ai-btns {
    display: flex; justify-content: flex-end; gap: 8px; padding: 10px 18px;
    border-top: 1px solid var(--border); margin-top: 10px;
}
.ts-ai-gen-btn {
    background: var(--accent); color: var(--accent-ink);
    border: none; border-radius: 6px; padding: 6px 18px; font-size: 13px;
    font-weight: 600; cursor: pointer;
}
.ts-ai-gen-btn:hover { opacity: 0.9; }
.ts-ai-gen-btn:disabled { opacity: 0.5; cursor: default; }
.ts-ai-edit-btn {
    background: var(--accent); color: var(--accent-ink);
    border: none; border-radius: 5px; padding: 6px 14px; font-size: 12px;
    font-weight: 600; cursor: pointer; margin-right: auto;
}
.ts-ai-edit-btn:hover { opacity: 0.9; }
.ts-ai-picker-grid {
    display: grid; grid-template-columns: repeat(4, 1fr); gap: 4px; padding: 10px 14px;
}
.ts-ai-picker-btn {
    display: flex; align-items: center; justify-content: center;
    background: var(--bg); color: var(--text-muted); border: 1px solid var(--border);
    border-radius: 6px; padding: 8px 4px; font-size: 11px; cursor: pointer;
}
.ts-ai-picker-btn:hover { border-color: #7c3aed; color: var(--text); }

/* ── Palette ────────────────────────────────────────────────────────── */
.ts-palette { border: 1px solid var(--border); border-radius: 8px; padding: 12px; background: var(--surface); max-height: 420px; overflow-y: auto; }
.ts-palette-cat { font-size: 11px; font-weight: 600; color: var(--text-muted); text-transform: uppercase; letter-spacing: 0.5px; margin: 8px 0 4px; }
.ts-palette-cat:first-child { margin-top: 0; }
.ts-palette-grid { display: flex; flex-wrap: wrap; gap: 4px; }
.ts-palette-pill { font-size: 12px; padding: 3px 10px; border-radius: 12px; background: var(--bg); border: 1px solid var(--border); cursor: grab; user-select: none; white-space: nowrap; }
.ts-palette-pill:hover { border-color: var(--accent); }
.ts-palette-pill.is-dragging { opacity: 0.4; }

/* ── Canvas ─────────────────────────────────────────────────────────── */
.ts-canvas { flex: 1; min-height: 200px; }
.ts-canvas-add-row { width: 100%; padding: 10px; margin-top: 8px; border: 2px dashed var(--border); border-radius: 8px; background: transparent; color: var(--text-muted); cursor: pointer; font-size: 13px; }
.ts-canvas-add-row:hover { border-color: var(--accent); color: var(--text); }
.ts-canvas-row { border: 1px solid var(--border); border-radius: 8px; margin-bottom: 8px; background: var(--surface); }
.ts-canvas-row-header { display: flex; align-items: center; gap: 8px; padding: 6px 10px; border-bottom: 1px solid var(--border); background: var(--bg); border-radius: 8px 8px 0 0; }
.ts-canvas-row-label { font-size: 12px; font-weight: 600; color: var(--text-muted); }
.ts-col-btns { display: flex; gap: 2px; margin-left: auto; }
.ts-col-btn { width: 24px; height: 24px; border: 1px solid var(--border); border-radius: 4px; background: transparent; color: var(--text-muted); cursor: pointer; font-size: 11px; font-weight: 600; }
.ts-col-btn.is-active { background: var(--accent); color: white; border-color: var(--accent); }
.ts-col-btn:hover { border-color: var(--accent); }
.ts-row-move { width: 24px; height: 24px; border: 1px solid var(--border); border-radius: 4px; background: transparent; color: var(--text-muted); cursor: pointer; font-size: 14px; line-height: 1; }
.ts-row-move:hover { border-color: var(--accent); color: var(--text); }
.ts-row-del { width: 24px; height: 24px; border: 1px solid var(--border); border-radius: 4px; background: transparent; color: var(--danger, #ef4444); cursor: pointer; font-size: 16px; line-height: 1; }
.ts-row-del:hover { background: var(--danger, #ef4444); color: white; }

/* ── Columns ────────────────────────────────────────────────────────── */
.ts-canvas-cols { display: grid; gap: 8px; padding: 8px; }
.ts-canvas-col { min-height: 60px; border: 1px dashed var(--border); border-radius: 6px; padding: 6px; transition: border-color 0.15s, background 0.15s; }
.ts-canvas-col.is-dragover { border-color: var(--accent); background: rgba(34,197,94,0.06); }
.ts-canvas-empty { text-align: center; padding: 16px 8px; color: var(--text-muted); font-size: 12px; opacity: 0.6; }

/* ── Blocks ─────────────────────────────────────────────────────────── */
.ts-canvas-block { display: flex; align-items: center; gap: 6px; padding: 5px 8px; border: 1px solid var(--border); border-radius: 4px; margin-bottom: 4px; background: var(--bg); cursor: grab; font-size: 12px; user-select: none; }
.ts-canvas-block.is-disabled { opacity: 0.45; }
.ts-canvas-block.is-dragging { opacity: 0.3; }
.ts-canvas-block:hover { border-color: var(--accent); }
.ts-block-handle { color: var(--text-muted); font-size: 14px; cursor: grab; }
.ts-block-label { flex: 1; font-weight: 500; }
.ts-block-edit, .ts-block-del { border: none; background: transparent; cursor: pointer; font-size: 14px; padding: 2px 4px; border-radius: 3px; }
.ts-block-edit { color: var(--text-muted); }
.ts-block-edit:hover { background: var(--border); color: var(--text); }
.ts-block-del { color: var(--danger, #ef4444); }
.ts-block-del:hover { background: var(--danger, #ef4444); color: white; }

/* ── Block Editor Modal ─────────────────────────────────────────────── */
.ts-editor-overlay { position: fixed; inset: 0; background: rgba(0,0,0,0.5); z-index: 10000; display: flex; justify-content: center; align-items: flex-start; padding: 40px 20px; }
.ts-editor-panel { background: var(--surface, #fff); border-radius: 12px; width: 480px; max-height: calc(100vh - 80px); display: flex; flex-direction: column; box-shadow: 0 20px 60px rgba(0,0,0,0.3); }
.ts-editor-header { display: flex; justify-content: space-between; align-items: center; padding: 14px 18px; border-bottom: 1px solid var(--border); }
.ts-editor-header h3 { margin: 0; font-size: 15px; }
.ts-editor-close { border: none; background: transparent; font-size: 20px; cursor: pointer; color: var(--text-muted); padding: 4px; }
.ts-editor-close:hover { color: var(--text); }
.ts-editor-body { padding: 16px 18px; overflow-y: auto; flex: 1; }
.ts-editor-footer { display: flex; gap: 8px; padding: 12px 18px; border-top: 1px solid var(--border); justify-content: flex-end; }
.ts-editor-row { display: flex; align-items: center; gap: 10px; margin-bottom: 8px; }
.ts-editor-row label { min-width: 100px; font-size: 12px; color: var(--text-muted); flex-shrink: 0; }
.ts-editor-input { padding: 4px 8px; border: 1px solid var(--border); border-radius: 4px; background: var(--bg); color: var(--text); font-size: 13px; flex: 1; min-width: 0; }
.ts-editor-input:focus { border-color: var(--accent); outline: none; }
.ts-editor-num { width: 80px; flex: 0 0 80px; }
.ts-editor-color { width: 36px; height: 28px; border: 1px solid var(--border); border-radius: 4px; cursor: pointer; padding: 0; }
.ts-editor-clear { border: 1px solid var(--border); background: transparent; border-radius: 4px; cursor: pointer; font-size: 14px; color: var(--danger, #ef4444); width: 24px; height: 24px; display: flex; align-items: center; justify-content: center; flex-shrink: 0; }
.ts-editor-clear:hover { background: var(--danger, #ef4444); color: white; }
.ts-editor-save { padding: 6px 16px; border-radius: 6px; border: none; background: var(--accent); color: white; cursor: pointer; font-size: 13px; font-weight: 500; }
.ts-editor-save:hover { filter: brightness(1.1); }
.ts-editor-cancel { padding: 6px 16px; border-radius: 6px; border: 1px solid var(--border); background: transparent; color: var(--text); cursor: pointer; font-size: 13px; }
.ts-editor-cancel:hover { background: var(--bg); }
.ts-editor-add { padding: 3px 10px; border-radius: 4px; border: 1px dashed var(--border); background: transparent; color: var(--text-muted); cursor: pointer; font-size: 11px; margin-top: 4px; }
.ts-editor-add:hover { border-color: var(--accent); color: var(--text); }

/* ── Template Picker ────────────────────────────────────────────────── */
.ts-templates { margin-bottom: 12px; }
.ts-template-list { border: 1px solid var(--border); border-radius: 8px; padding: 8px; background: var(--bg); max-height: 260px; overflow-y: auto; }
.ts-template-card { display: flex; justify-content: space-between; align-items: center; padding: 8px 10px; border: 1px solid var(--border); border-radius: 6px; margin-bottom: 4px; background: var(--surface); }
.ts-template-card:last-child { margin-bottom: 0; }

/* ── Layout builder layout (palette + canvas side-by-side) ──────────── */
.ts-builder-wrap { display: flex; gap: 16px; }
.ts-builder-wrap .ts-palette { width: 240px; flex-shrink: 0; }
.ts-builder-wrap .ts-canvas { flex: 1; }
@media (max-width: 860px) {
    .ts-builder-wrap { flex-direction: column; }
    .ts-builder-wrap .ts-palette { width: 100%; max-height: 200px; }
}

/* ── Responsive token overrides ─────────────────────────────────────── */
.ts-breakpoint-tabs { display: flex; gap: 0; border-bottom: 2px solid var(--border); margin-bottom: 16px; }
.ts-breakpoint-tab { padding: 8px 16px; cursor: pointer; border-bottom: 2px solid transparent; margin-bottom: -2px; color: var(--text-muted); font-size: 13px; background: transparent; border-left: none; border-right: none; border-top: none; }
.ts-breakpoint-tab.is-active { border-bottom-color: var(--accent); color: var(--text); font-weight: 600; }
.ts-override-row { display: flex; align-items: center; gap: 8px; }
.ts-override-clear { border: none; background: transparent; color: var(--danger, #ef4444); cursor: pointer; font-size: 14px; padding: 2px 4px; }
.ts-override-clear:hover { color: var(--text); }

/* ── Page Studio ────────────────────────────────────────────────────── */
.ts-rules-table { width: 100%; border-collapse: collapse; font-size: 13px; }
.ts-rules-table th { text-align: left; padding: 6px 8px; border-bottom: 2px solid var(--border); font-size: 12px; color: var(--text-muted); }
.ts-rules-table td { padding: 6px 8px; border-bottom: 1px solid var(--border); }
.ts-rules-table select, .ts-rules-table input { padding: 3px 6px; border: 1px solid var(--border); border-radius: 4px; background: var(--bg); color: var(--text); font-size: 12px; }
"##
}
