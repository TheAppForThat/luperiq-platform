
//! Admin CSS for the Content Library module.

pub(crate) const CONTENT_SOURCES_ADMIN_CSS: &str = r##"
/* ── Content Library ───────────────────────────────────────── */

.cl-header {
    display: flex; justify-content: space-between; align-items: center;
    margin-bottom: 1.5rem; flex-wrap: wrap; gap: 1rem;
}
.cl-header h2 { margin: 0; font-size: 1.4rem; }
.cl-actions { display: flex; gap: 0.5rem; flex-wrap: wrap; }

/* Topic cards grid */
.cl-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
    gap: 1rem;
}
.cl-card {
    background: var(--card-bg, #1e1e2e);
    border: 1px solid var(--border, #333);
    border-radius: 8px;
    padding: 1.25rem;
    cursor: pointer;
    transition: border-color 0.15s, box-shadow 0.15s;
}
.cl-card:hover {
    border-color: var(--accent, #6366f1);
    box-shadow: 0 2px 8px rgba(99,102,241,0.15);
}
.cl-card-title {
    font-size: 1.1rem; font-weight: 600; margin-bottom: 0.5rem;
}
.cl-card-meta {
    display: flex; gap: 0.75rem; font-size: 0.85rem;
    color: var(--text-muted, #888); flex-wrap: wrap;
}
.cl-type-icon {
    display: inline-flex; align-items: center; gap: 0.25rem;
    padding: 2px 6px; border-radius: 4px; font-size: 0.8rem;
    background: rgba(99,102,241,0.1); color: var(--accent, #6366f1);
}
.cl-conflict-badge {
    background: rgba(245,158,11,0.15); color: #f59e0b;
    padding: 2px 8px; border-radius: 4px; font-size: 0.8rem;
}

/* Source detail view */
.cl-detail-header {
    display: flex; align-items: center; gap: 0.75rem;
    margin-bottom: 1.5rem;
}
.cl-back-btn {
    background: none; border: none; color: var(--accent, #6366f1);
    cursor: pointer; font-size: 1.1rem; padding: 4px 8px;
}
.cl-source-list { display: flex; flex-direction: column; gap: 1rem; }
.cl-source-card {
    background: var(--card-bg, #1e1e2e);
    border: 1px solid var(--border, #333);
    border-radius: 8px; padding: 1rem;
}
.cl-source-card-header {
    display: flex; justify-content: space-between; align-items: center;
    margin-bottom: 0.75rem;
}
.cl-source-type-badge {
    font-size: 0.75rem; padding: 2px 8px; border-radius: 4px;
    background: rgba(99,102,241,0.1); color: var(--accent, #6366f1);
}
.cl-source-type-badge.luperiq {
    background: rgba(34,197,94,0.1); color: #22c55e;
}
.cl-facts-grid {
    display: grid; grid-template-columns: max-content 1fr;
    gap: 0.25rem 1rem; font-size: 0.9rem;
}
.cl-fact-key { color: var(--text-muted, #888); font-weight: 500; }
.cl-fact-value { color: var(--text, #e0e0e0); }
.cl-raw-preview {
    background: rgba(0,0,0,0.2); border-radius: 6px;
    padding: 0.75rem; font-size: 0.85rem; white-space: pre-wrap;
    max-height: 200px; overflow-y: auto; color: var(--text-muted, #888);
}

/* Upload zone */
.cl-upload-zone {
    border: 2px dashed var(--border, #444);
    border-radius: 8px; padding: 2rem; text-align: center;
    transition: border-color 0.15s, background 0.15s;
    cursor: pointer;
}
.cl-upload-zone.dragover {
    border-color: var(--accent, #6366f1);
    background: rgba(99,102,241,0.05);
}
.cl-upload-zone p { margin: 0.5rem 0; color: var(--text-muted, #888); }
.cl-guidelines {
    background: rgba(99,102,241,0.05); border-radius: 6px;
    padding: 1rem; font-size: 0.85rem; margin-top: 1rem;
    line-height: 1.5;
}
.cl-guidelines strong { color: var(--text, #e0e0e0); }
.cl-guidelines ul { margin: 0.5rem 0; padding-left: 1.25rem; }

/* Scrape form */
.cl-scrape-form {
    display: flex; gap: 0.5rem; align-items: center; margin-bottom: 1rem;
}
.cl-scrape-form input {
    flex: 1; padding: 0.5rem 0.75rem; border-radius: 6px;
    border: 1px solid var(--border, #333);
    background: var(--input-bg, #161622); color: var(--text, #e0e0e0);
}

/* Commission tiers */
.cl-tiers { display: grid; grid-template-columns: 1fr 1fr; gap: 1rem; margin: 1rem 0; }
.cl-tier-card {
    background: var(--card-bg, #1e1e2e);
    border: 2px solid var(--border, #333);
    border-radius: 8px; padding: 1.25rem; cursor: pointer;
    transition: border-color 0.15s;
}
.cl-tier-card:hover, .cl-tier-card.selected {
    border-color: var(--accent, #6366f1);
}
.cl-tier-card h4 { margin: 0 0 0.5rem; }
.cl-tier-price {
    font-size: 1.5rem; font-weight: 700;
    color: var(--accent, #6366f1); margin: 0.5rem 0;
}
.cl-tier-desc { font-size: 0.85rem; color: var(--text-muted, #888); }

/* Conflict banner */
.cl-conflict-banner {
    background: rgba(245,158,11,0.1); border: 1px solid rgba(245,158,11,0.3);
    border-radius: 6px; padding: 0.75rem 1rem; margin-bottom: 1rem;
    display: flex; align-items: center; gap: 0.5rem;
    color: #f59e0b; font-size: 0.9rem;
}
.cl-conflict-row {
    display: grid; grid-template-columns: 1fr 1fr; gap: 1rem;
    margin-bottom: 0.75rem; padding: 0.75rem;
    background: rgba(0,0,0,0.15); border-radius: 6px;
}
.cl-conflict-side { font-size: 0.85rem; }
.cl-conflict-side label {
    font-weight: 600; font-size: 0.75rem; text-transform: uppercase;
    color: var(--text-muted, #888); margin-bottom: 0.25rem; display: block;
}
.cl-conflict-actions { display: flex; gap: 0.5rem; margin-top: 0.75rem; }

/* Modal overlay */
.cl-modal-overlay {
    position: fixed; inset: 0; background: rgba(0,0,0,0.6);
    display: flex; align-items: center; justify-content: center;
    z-index: 1000;
}
.cl-modal {
    background: var(--card-bg, #1e1e2e);
    border: 1px solid var(--border, #333);
    border-radius: 12px; padding: 1.5rem;
    max-width: 600px; width: 90%; max-height: 80vh; overflow-y: auto;
}
.cl-modal h3 { margin: 0 0 1rem; }

/* Preview table for parsed facts */
.cl-preview-table {
    width: 100%; border-collapse: collapse; font-size: 0.85rem; margin: 1rem 0;
}
.cl-preview-table th {
    text-align: left; padding: 0.5rem; border-bottom: 1px solid var(--border, #333);
    color: var(--text-muted, #888); font-weight: 600;
}
.cl-preview-table td {
    padding: 0.5rem; border-bottom: 1px solid rgba(255,255,255,0.05);
}

/* Empty state */
.cl-empty {
    text-align: center; padding: 3rem 1rem;
    color: var(--text-muted, #888);
}
.cl-empty p { margin: 0.5rem 0; }

/* Responsive */
@media (max-width: 640px) {
    .cl-tiers { grid-template-columns: 1fr; }
    .cl-conflict-row { grid-template-columns: 1fr; }
}
"##;
