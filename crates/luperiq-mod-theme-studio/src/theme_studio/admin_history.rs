//! Revision history panel for the Design Studio.
//!
//! Shows timestamped snapshots with restore functionality.
//! Integrates with the studio shell's undo system so that restoring
//! a revision first pushes a snapshot for local undo.
//!
//! Security: All JS uses DOM methods only (createElement, textContent,
//! replaceChildren). No innerHTML, outerHTML, or insertAdjacentHTML.

/// Return JS that adds `TsStudio.showHistory()` for the revision history panel.
pub fn history_js() -> &'static str {
    HISTORY_JS
}

const HISTORY_JS: &str = r####"
/* ── TsStudio: Revision History Panel ──────────────────────────────── */

TsStudio.showHistory = async function() {
    if (!TsStudio._slug) return;

    var res = await tsApi('/revisions/profile/' + encodeURIComponent(TsStudio._slug));
    if (!res.ok) {
        if (typeof showToast === 'function') showToast('Could not load revisions', 'error');
        return;
    }

    var revisions = res.data || [];

    /* ── Build overlay ──────────────────────────────────────────────── */
    var overlay = document.createElement('div');
    overlay.className = 'ts-history-overlay';
    overlay.addEventListener('click', function(e) {
        if (e.target === overlay) overlay.remove();
    });

    var panel = document.createElement('div');
    panel.className = 'ts-history-panel';

    var title = document.createElement('h3');
    title.textContent = 'Revision History';
    panel.appendChild(title);

    var subtitle = document.createElement('p');
    subtitle.className = 'ts-history-subtitle';
    subtitle.textContent = 'Profile: ' + TsStudio._slug;
    panel.appendChild(subtitle);

    if (revisions.length === 0) {
        var empty = document.createElement('p');
        empty.className = 'ts-history-empty';
        empty.textContent = 'No revisions yet. Revisions are created each time you save.';
        panel.appendChild(empty);
    } else {
        var list = document.createElement('div');
        list.className = 'ts-history-list';

        revisions.forEach(function(rev) {
            var row = document.createElement('div');
            row.className = 'ts-history-row';

            var info = document.createElement('div');
            info.className = 'ts-history-info';

            var versionLabel = document.createElement('strong');
            versionLabel.textContent = 'v' + rev.version;
            info.appendChild(versionLabel);

            var dateLabel = document.createElement('span');
            dateLabel.className = 'ts-history-date';
            var ts = rev.created_at || 0;
            dateLabel.textContent = ts ? new Date(ts * 1000).toLocaleString() : 'unknown date';
            info.appendChild(dateLabel);

            row.appendChild(info);

            var restoreBtn = tsBtn('Restore', async function() {
                if (!confirm('Restore to version ' + rev.version + '? Current unsaved changes will be lost.')) return;

                /* Push local undo snapshot before restoring */
                TsStudio.pushSnapshot('Before restore to v' + rev.version);

                var rr = await tsApi(
                    '/revisions/profile/' + encodeURIComponent(TsStudio._slug) + '/restore/' + rev.version,
                    { method: 'POST' }
                );
                if (rr.ok) {
                    await TsStudio.loadProfile(TsStudio._slug);
                    TsStudio.switchTab(TsStudio._activeTab);
                    if (typeof showToast === 'function') showToast('Restored to version ' + rev.version, 'success');
                    overlay.remove();
                } else {
                    if (typeof showToast === 'function') showToast('Restore failed: ' + (rr.message || 'unknown error'), 'error');
                }
            });
            row.appendChild(restoreBtn);

            list.appendChild(row);
        });

        panel.appendChild(list);
    }

    /* Close button at bottom */
    var footer = document.createElement('div');
    footer.className = 'ts-history-footer';
    var closeBtn = tsBtnGhost('Close', function() { overlay.remove(); });
    footer.appendChild(closeBtn);
    panel.appendChild(footer);

    overlay.appendChild(panel);
    document.body.appendChild(overlay);
};
"####;
