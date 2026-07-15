//! Export / import of directory mini-site pages (the per-company tabs) in JSON and CSV.
//!
//! Shared by the **operator** data-studio editor and the **owner** my-listings editor so
//! both round-trip identically. The unit is `(company_id, DirectoryMiniSitePage)`.
//!
//! - **JSON** (lossless): `{ "companies": [ { "company_id", "pages": [ {page} ] } ] }`.
//!   A single-company export is just a one-entry `companies` array.
//! - **CSV** (flat index): one row per *block*; a page with no blocks emits one row with an
//!   empty `block_index` so empty tabs survive the round-trip. Columns:
//!   `company_id,page_slug,page_title,block_index,block_type,heading,subheading,body,cta_text,cta_url`.

use crate::store::{DirectoryMiniSiteBlock, DirectoryMiniSitePage, DirectoryStore};
use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;

const CSV_HEADER: &str =
    "company_id,page_slug,page_title,block_index,block_type,heading,subheading,body,cta_text,cta_url";

// ── JSON ────────────────────────────────────────────────────────────────────

/// Group `(company_id, page)` rows into the canonical export JSON value.
pub fn to_json(rows: &[(String, DirectoryMiniSitePage)]) -> serde_json::Value {
    let mut order: Vec<String> = Vec::new();
    let mut by_company: std::collections::HashMap<String, Vec<&DirectoryMiniSitePage>> =
        std::collections::HashMap::new();
    for (cid, page) in rows {
        by_company.entry(cid.clone()).or_default().push(page);
        if !order.contains(cid) {
            order.push(cid.clone());
        }
    }
    let companies: Vec<serde_json::Value> = order
        .into_iter()
        .map(|cid| {
            let pages = by_company.remove(&cid).unwrap_or_default();
            serde_json::json!({ "company_id": cid, "pages": pages })
        })
        .collect();
    serde_json::json!({ "companies": companies })
}

/// Parse the canonical export JSON back into `(company_id, page)` rows. Tolerant: accepts
/// either the `{companies:[...]}` envelope or a bare single-company `{company_id, pages}`.
pub fn from_json(text: &str) -> Result<Vec<(String, DirectoryMiniSitePage)>, String> {
    let v: serde_json::Value =
        serde_json::from_str(text).map_err(|e| format!("invalid JSON: {e}"))?;
    let companies: Vec<serde_json::Value> = if let Some(arr) = v.get("companies").and_then(|c| c.as_array()) {
        arr.clone()
    } else if v.get("company_id").is_some() {
        vec![v.clone()]
    } else {
        return Err("JSON must have a `companies` array or a `company_id` + `pages`".into());
    };
    let mut out = Vec::new();
    for c in companies {
        let cid = c
            .get("company_id")
            .and_then(|x| x.as_str())
            .ok_or("a company entry is missing `company_id`")?
            .to_string();
        let pages = c.get("pages").and_then(|p| p.as_array()).cloned().unwrap_or_default();
        for p in pages {
            let page: DirectoryMiniSitePage = serde_json::from_value(p)
                .map_err(|e| format!("bad page for {cid}: {e}"))?;
            out.push((cid.clone(), page));
        }
    }
    Ok(out)
}

// ── CSV ─────────────────────────────────────────────────────────────────────

pub fn to_csv(rows: &[(String, DirectoryMiniSitePage)]) -> String {
    let mut s = String::from(CSV_HEADER);
    s.push('\n');
    for (cid, page) in rows {
        if page.blocks.is_empty() {
            s.push_str(&csv_row(&[
                cid,
                &page.page_slug,
                page.page_title.as_deref().unwrap_or(""),
                "",
                "",
                "",
                "",
                "",
                "",
                "",
            ]));
        } else {
            for (i, b) in page.blocks.iter().enumerate() {
                s.push_str(&csv_row(&[
                    cid,
                    &page.page_slug,
                    page.page_title.as_deref().unwrap_or(""),
                    &i.to_string(),
                    &b.block_type,
                    b.heading.as_deref().unwrap_or(""),
                    b.subheading.as_deref().unwrap_or(""),
                    b.body.as_deref().unwrap_or(""),
                    b.cta_text.as_deref().unwrap_or(""),
                    b.cta_url.as_deref().unwrap_or(""),
                ]));
            }
        }
    }
    s
}

pub fn from_csv(text: &str) -> Result<Vec<(String, DirectoryMiniSitePage)>, String> {
    let records = parse_csv(text);
    if records.is_empty() {
        return Ok(Vec::new());
    }
    // Skip a header row if present (first cell == "company_id").
    let start = if records[0].first().map(|c| c.as_str()) == Some("company_id") { 1 } else { 0 };
    // Preserve first-seen order of (company_id, page_slug); collect blocks with their index.
    let mut order: Vec<(String, String)> = Vec::new();
    let mut titles: std::collections::HashMap<(String, String), String> = std::collections::HashMap::new();
    let mut blocks: std::collections::HashMap<(String, String), Vec<(i64, DirectoryMiniSiteBlock)>> =
        std::collections::HashMap::new();
    for rec in &records[start..] {
        if rec.iter().all(|c| c.trim().is_empty()) {
            continue;
        }
        let g = |i: usize| rec.get(i).cloned().unwrap_or_default();
        let cid = g(0);
        let slug = g(1);
        if cid.is_empty() || slug.is_empty() {
            return Err("CSV row missing company_id or page_slug".into());
        }
        let key = (cid.clone(), slug.clone());
        if !order.contains(&key) {
            order.push(key.clone());
        }
        titles.entry(key.clone()).or_insert_with(|| g(2));
        let idx_raw = g(3);
        if !idx_raw.trim().is_empty() {
            let idx: i64 = idx_raw.trim().parse().unwrap_or(i64::MAX);
            let opt = |s: String| if s.is_empty() { None } else { Some(s) };
            let block = DirectoryMiniSiteBlock {
                block_type: { let t = g(4); if t.is_empty() { "content".into() } else { t } },
                heading: opt(g(5)),
                subheading: opt(g(6)),
                body: opt(g(7)),
                cta_text: opt(g(8)),
                cta_url: opt(g(9)),
                image_url: opt(g(10)),
            };
            blocks.entry(key).or_default().push((idx, block));
        }
    }
    let mut out = Vec::new();
    for key in order {
        let mut bl = blocks.remove(&key).unwrap_or_default();
        bl.sort_by_key(|(i, _)| *i);
        let title = titles.remove(&key).filter(|t| !t.is_empty());
        out.push((
            key.0,
            DirectoryMiniSitePage {
                page_slug: key.1,
                page_title: title,
                blocks: bl.into_iter().map(|(_, b)| b).collect(),
            },
        ));
    }
    Ok(out)
}

// ── Upgrade: directory mini-site → full website carry-over bundle ──────────────

/// Map a mini-site block to a Theme-Studio-style block envelope `{type, data}`. Type names
/// align with the Theme Studio library so provisioning seeds pages near 1:1; the generic
/// fields are carried in `data` under both the smart-block names (headline/description) and
/// their originals, so nothing is lost regardless of which the target block reads.
fn block_to_smart(b: &DirectoryMiniSiteBlock) -> serde_json::Value {
    // Map to Theme-Studio blocks that render *custom block data* (verified in smart_blocks.rs).
    // service-grid/trust-badges/testimonials/about-section are data-driven (pull site records),
    // so generic text goes to `cta-bar` (renders heading+description), hero→company-hero,
    // cta→cta-section, faq→faq-accordion (faqs[]), testimonial→testimonial-quote.
    match b.block_type.as_str() {
        "hero" => serde_json::json!({"type":"company-hero","data":{
            "headline": b.heading, "heading": b.heading, "title": b.heading,
            "subheading": b.subheading, "subhead": b.subheading,
            "description": b.body, "body": b.body,
            "cta_text": b.cta_text, "cta_url": b.cta_url,
        }}),
        "cta" => serde_json::json!({"type":"cta-section","data":{
            "headline": b.heading, "heading": b.heading, "title": b.heading,
            "subheading": b.subheading, "description": b.body, "body": b.body,
            "cta_text": b.cta_text, "cta_url": b.cta_url,
        }}),
        "faq" => serde_json::json!({"type":"faq-accordion","data":{
            "title": b.heading,
            "faqs": [{"question": b.heading, "answer": b.body}],
        }}),
        "testimonial" => serde_json::json!({"type":"testimonial-quote","data":{
            "quote": b.body, "attribution": b.heading, "tone": "default",
        }}),
        // content / services / about / trust / unknown → cta-bar (renders custom heading+body)
        _ => serde_json::json!({"type":"cta-bar","data":{
            "heading": b.heading, "title": b.heading, "headline": b.heading,
            "subheading": b.subheading, "subhead": b.subheading,
            "description": b.body, "body": b.body,
            "cta_text": b.cta_text, "cta_url": b.cta_url,
            "source_block_type": b.block_type,
        }}),
    }
}

/// Build the full carry-over bundle for a company: business profile + every mini-site tab
/// converted to a Theme-Studio page (`body_json` array of smart blocks). This is exactly the
/// artifact a provisioning step consumes (e.g. `ensure_seeded_content(content_type="page",
/// slug, title, body_json)`) to seed a new tenant website. Returns `None` if the company
/// doesn't exist.
pub fn upgrade_bundle(
    store: &crate::store::DirectoryStore,
    company_id: &str,
) -> Option<serde_json::Value> {
    let co = store.company_by_id(company_id)?;
    let pages: Vec<serde_json::Value> = store
        .mini_site_pages_for(company_id)
        .into_iter()
        .map(|p| {
            let blocks: Vec<serde_json::Value> = p.blocks.iter().map(block_to_smart).collect();
            serde_json::json!({
                "slug": p.page_slug,
                "title": p.page_title,
                "content_type": "page",
                "body_json": serde_json::to_string(&blocks).unwrap_or_else(|_| "[]".into()),
            })
        })
        .collect();
    Some(serde_json::json!({
        "schema": "luperiq.site-bundle.v1",
        "source": { "kind": "directory-listing", "company_id": company_id },
        "business": {
            "name": co.dba.clone().unwrap_or_else(|| co.entity_name.clone()),
            "legal_name": co.entity_name,
            "phone": co.phone,
            "email": co.email,
            "website": co.website,
            "address": co.address,
            "city": co.city,
            "state": co.state_abbr,
            "license_number": co.pest_license_num,
            "license_type": co.pest_license_type,
            "categories": co.pest_categories_decoded,
        },
        "pages": pages,
        "carried_pages": pages_count_hint(store, company_id),
    }))
}

fn pages_count_hint(store: &crate::store::DirectoryStore, company_id: &str) -> usize {
    store.mini_site_pages_for(company_id).len()
}

/// Build an axum download `Response` of the upgrade bundle for `company_id`.
pub fn upgrade_bundle_response(store: &crate::store::DirectoryStore, company_id: &str) -> Response {
    match upgrade_bundle(store, company_id) {
        Some(b) => {
            let body = serde_json::to_string_pretty(&b).unwrap_or_else(|_| "{}".into());
            (
                StatusCode::OK,
                [
                    (header::CONTENT_TYPE, "application/json".to_string()),
                    (
                        header::CONTENT_DISPOSITION,
                        format!("attachment; filename=\"site-bundle-{company_id}.json\""),
                    ),
                ],
                body,
            )
                .into_response()
        }
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

// ── axum-facing helpers (shared by owner + operator) ──────────────────────────

/// Build a download `Response` of the given page rows in `fmt` ("json" | "csv").
pub fn export_response(rows: &[(String, DirectoryMiniSitePage)], fmt: &str, label: &str) -> Response {
    if fmt == "csv" {
        let body = to_csv(rows);
        (
            StatusCode::OK,
            [
                (header::CONTENT_TYPE, "text/csv; charset=utf-8".to_string()),
                (
                    header::CONTENT_DISPOSITION,
                    format!("attachment; filename=\"minisite-{label}.csv\""),
                ),
            ],
            body,
        )
            .into_response()
    } else {
        let body = serde_json::to_string_pretty(&to_json(rows)).unwrap_or_else(|_| "{}".into());
        (
            StatusCode::OK,
            [
                (header::CONTENT_TYPE, "application/json".to_string()),
                (
                    header::CONTENT_DISPOSITION,
                    format!("attachment; filename=\"minisite-{label}.json\""),
                ),
            ],
            body,
        )
            .into_response()
    }
}

/// Parse `body` as `fmt` and upsert every page. When `force_company_id` is `Some`, every
/// imported row is written to that company (the owner-scope guard); when `None`, the rows'
/// own `company_id`s are used (operator/site-wide). Returns `{imported}` or `{error}`.
pub fn import_apply(
    store: &DirectoryStore,
    body: &str,
    fmt: &str,
    force_company_id: Option<&str>,
) -> Response {
    let parsed = if fmt == "csv" { from_csv(body) } else { from_json(body) };
    let rows = match parsed {
        Ok(r) => r,
        Err(e) => {
            return (StatusCode::BAD_REQUEST, Json(serde_json::json!({ "error": e }))).into_response()
        }
    };
    let mut imported = 0usize;
    let mut errors: Vec<String> = Vec::new();
    for (cid, page) in rows {
        let target = force_company_id.unwrap_or(cid.as_str());
        match store.upsert_mini_site_page(target, &page.page_slug, page.page_title.as_deref(), &page.blocks) {
            Ok(()) => imported += 1,
            Err(e) => errors.push(format!("{}/{}: {}", target, page.page_slug, e)),
        }
    }
    Json(serde_json::json!({ "ok": errors.is_empty(), "imported": imported, "errors": errors }))
        .into_response()
}

// ── shared editor UI ─────────────────────────────────────────────────────────

fn esc(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

/// A self-contained mini-site page editor (HTML + inline JS), parameterized by `api_base`.
/// Both editors expose the same sub-paths under their base: `GET {base}/pages.json`,
/// `POST {base}/pages`, `POST {base}/pages/{slug}/delete`, `GET {base}/export?format=`,
/// `POST {base}/import`. `back_url` is the "done" link; `who` labels the surface.
pub fn editor_html(api_base: &str, company_id: &str, company_name: &str, back_url: &str, who: &str) -> String {
    let base = esc(api_base);
    let cid = esc(company_id);
    let name = esc(company_name);
    let back = esc(back_url);
    let who = esc(who);
    format!(r###"<!doctype html><html lang="en"><head><meta charset="utf-8">
<meta name="viewport" content="width=device-width,initial-scale=1">
<title>Mini-site editor — {name}</title>
<style>
:root{{--b:#e2e6ee;--a:#e8752a;--ink:#1a2236;--mut:#64748b}}
*{{box-sizing:border-box}}body{{font:15px/1.5 system-ui,Arial,sans-serif;color:var(--ink);max-width:920px;margin:0 auto;padding:18px;background:#f5f7fb}}
h1{{font-size:20px;margin:.2em 0}}.sub{{color:var(--mut);font-size:13px;margin-bottom:14px}}
.bar{{display:flex;gap:8px;flex-wrap:wrap;align-items:center;margin:12px 0}}
button,.btn{{font:inherit;border:1px solid var(--b);background:#fff;border-radius:8px;padding:7px 13px;cursor:pointer}}
button.primary{{background:var(--a);color:#fff;border-color:var(--a);font-weight:600}}
button.danger{{color:#b42318;border-color:#f3c4bd}}
.page{{background:#fff;border:1px solid var(--b);border-radius:12px;padding:14px;margin:12px 0}}
.page>.hd{{display:flex;gap:8px;flex-wrap:wrap;align-items:center}}
input,select,textarea{{font:inherit;width:100%;padding:7px 9px;border:1px solid var(--b);border-radius:7px}}
textarea{{min-height:70px;resize:vertical}}
.block{{border:1px dashed var(--b);border-radius:9px;padding:10px;margin:10px 0;background:#fafbfe}}
.row{{display:grid;grid-template-columns:1fr 1fr;gap:8px;margin:6px 0}}
label{{font-size:12px;color:var(--mut);display:block;margin-bottom:2px}}
.slugw{{max-width:240px}}.titlew{{flex:1;min-width:200px}}
#msg{{position:fixed;top:12px;right:12px;background:#0b1020;color:#cfe2ff;padding:9px 14px;border-radius:9px;opacity:0;transition:.2s;z-index:9}}
#msg.show{{opacity:1}}.muted{{color:var(--mut);font-size:12px}}
</style></head><body>
<div id="msg"></div>
<h1>Mini-site editor — {name}</h1>
<div class="sub">{who} · company <code>{cid}</code> · <a href="{back}">← done</a></div>
<div class="bar">
  <button class="primary" onclick="addPage()">+ Add tab</button>
  <a class="btn" href="{base}/export?format=json" download>Export JSON</a>
  <a class="btn" href="{base}/export?format=csv" download>Export CSV</a>
  <label class="btn" style="position:relative">Import…<input type="file" accept=".json,.csv" style="position:absolute;inset:0;opacity:0;cursor:pointer" onchange="importFile(this)"></label>
  <button class="btn" style="margin-left:auto;background:#0f2a44;color:#fff;border-color:#0f2a44;font-weight:600" onclick="upgradeSite()">⬆ Upgrade to full website</button>
</div>
<div class="muted" style="margin:-4px 0 6px">Tabs render at <code>/&lt;state&gt;/&lt;city&gt;/&lt;company&gt;/&lt;slug&gt;</code> · Upgrade carries your tabs + business info into a full website.</div>
<div id="pages"></div>
<script>
const BASE={base_js}, CID={cid_js}, BIZ_NAME={name_js};
const BT=["hero","content","services","trust","about","faq","testimonial","cta"];
let pages=[];
function toast(t){{const m=document.getElementById('msg');m.textContent=t;m.classList.add('show');setTimeout(()=>m.classList.remove('show'),1800);}}
function el(t,a={{}},...k){{const e=document.createElement(t);for(const x in a)x=='html'?e.innerHTML=a[x]:e.setAttribute(x,a[x]);k.flat().forEach(c=>e.append(c));return e;}}
async function load(){{const r=await fetch(BASE+'/pages.json',{{credentials:'same-origin'}});pages=r.ok?(await r.json()).pages||[]:[];render();}}
function addPage(){{pages.push({{page_slug:'',page_title:'',blocks:[]}});render();}}
function addBlock(pi){{pages[pi].blocks.push({{block_type:'content'}});render();}}
function render(){{
 const root=document.getElementById('pages');root.innerHTML='';
 if(!pages.length)root.append(el('p',{{class:'muted'}},'No tabs yet — add one.'));
 pages.forEach((p,pi)=>{{
  const card=el('div',{{class:'page'}});
  const slug=el('input',{{class:'',placeholder:'slug (e.g. services)',value:p.page_slug||''}});slug.oninput=e=>p.page_slug=e.target.value;
  const title=el('input',{{placeholder:'Tab title (e.g. Our Services)',value:p.page_title||''}});title.oninput=e=>p.page_title=e.target.value;
  const hd=el('div',{{class:'hd'}},el('div',{{class:'slugw'}},el('label',{{}},'Slug'),slug),el('div',{{class:'titlew'}},el('label',{{}},'Title'),title));
  card.append(hd);
  p.blocks.forEach((b,bi)=>{{
   const bw=el('div',{{class:'block'}});
   const ty=el('select');BT.forEach(t=>{{const o=el('option',{{value:t}},t);if(b.block_type==t)o.setAttribute('selected','');ty.append(o);}});ty.value=b.block_type||'content';ty.onchange=e=>b.block_type=e.target.value;
   const mk=(k,ph,ta)=>{{const i=ta?el('textarea',{{placeholder:ph}}):el('input',{{placeholder:ph,value:b[k]||''}});if(ta)i.value=b[k]||'';i.oninput=e=>b[k]=e.target.value;return el('div',{{}},el('label',{{}},ph),i);}};
   bw.append(el('div',{{class:'row'}},el('div',{{}},el('label',{{}},'Block type'),ty),mk('heading','Heading')));
   bw.append(el('div',{{class:'row'}},mk('subheading','Subheading'),mk('cta_text','Button text')));
   bw.append(mk('body','Body text',true));
   bw.append(el('div',{{class:'row'}},mk('cta_url','Button URL'),el('div',{{style:'display:flex;align-items:flex-end'}},el('button',{{class:'danger',onclick:''}},'Remove block'))));
   bw.querySelector('button.danger').onclick=()=>{{p.blocks.splice(bi,1);render();}};
   card.append(bw);
  }});
  const foot=el('div',{{class:'bar'}},
    el('button',{{}},'+ Block'),
    el('button',{{class:'primary'}},'Save tab'),
    el('button',{{class:'danger'}},'Delete tab'));
  foot.children[0].onclick=()=>addBlock(pi);
  foot.children[1].onclick=()=>savePage(pi);
  foot.children[2].onclick=()=>delPage(pi);
  card.append(foot);root.append(card);
 }});
}}
async function savePage(pi){{const p=pages[pi];if(!(p.page_slug||'').trim())return toast('Slug required');
 const r=await fetch(BASE+'/pages',{{method:'POST',credentials:'same-origin',headers:{{'content-type':'application/json'}},body:JSON.stringify(p)}});
 toast(r.ok?'Saved':'Save failed');if(r.ok)load();}}
async function delPage(pi){{const p=pages[pi];if(!(p.page_slug||'').trim()){{pages.splice(pi,1);return render();}}
 if(!confirm('Delete tab "'+p.page_slug+'"?'))return;
 const r=await fetch(BASE+'/pages/'+encodeURIComponent(p.page_slug)+'/delete',{{method:'POST',credentials:'same-origin'}});
 toast(r.ok?'Deleted':'Delete failed');if(r.ok)load();}}
async function importFile(inp){{const f=inp.files[0];if(!f)return;const text=await f.text();const fmt=f.name.toLowerCase().endsWith('.csv')?'csv':'json';
 const r=await fetch(BASE+'/import?format='+fmt,{{method:'POST',credentials:'same-origin',headers:{{'content-type':'text/plain'}},body:text}});
 const j=await r.json().catch(()=>({{}}));toast(r.ok?('Imported '+(j.imported||0)+' tab(s)'):('Import failed: '+(j.error||r.status)));inp.value='';if(r.ok)load();}}
async function upgradeSite(){{
  if(!confirm('Upgrade this listing to a full AI-built website? We’ll carry over your real business info, services, and photos.'))return;
  /* Record the upgrade request server-side (best-effort; the bundle stays
     available at BASE+"/upgrade-bundle" if the operator wants the raw artifact). */
  try{{await fetch(BASE+'/upgrade',{{method:'POST',credentials:'same-origin'}});}}catch(_){{}}
  /* Route into the AI Builder carrying the directory company id (cid). The
     builder threads cid onto the lead so business research seeds from our REAL
     crawled data (name/phone/services/photos + grounded copy) instead of AI
     fabrication. We also cheap-prefill the business name + pest-control industry. */
  const p=new URLSearchParams();
  if(CID)p.set('cid',CID);
  if(BIZ_NAME)p.set('business_name',BIZ_NAME);
  p.set('industry','pest-control');
  window.location.href='/ai-builder?'+p.toString();}}
load();
</script></body></html>"###,
        base = base, cid = cid, name = name, back = back, who = who,
        base_js = serde_json::to_string(api_base).unwrap_or_else(|_| "\"\"".into()),
        cid_js = serde_json::to_string(company_id).unwrap_or_else(|_| "\"\"".into()),
        name_js = serde_json::to_string(company_name).unwrap_or_else(|_| "\"\"".into()),
    )
}

// ── minimal RFC-4180 CSV reader/writer ───────────────────────────────────────

fn csv_escape(field: &str) -> String {
    if field.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", field.replace('"', "\"\""))
    } else {
        field.to_string()
    }
}

fn csv_row(fields: &[&str]) -> String {
    let mut line: String = fields.iter().map(|f| csv_escape(f)).collect::<Vec<_>>().join(",");
    line.push('\n');
    line
}

/// Parse CSV into rows of fields. Handles quoted fields with embedded commas, quotes
/// (doubled) and newlines.
fn parse_csv(text: &str) -> Vec<Vec<String>> {
    let mut rows: Vec<Vec<String>> = Vec::new();
    let mut field = String::new();
    let mut row: Vec<String> = Vec::new();
    let mut in_quotes = false;
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if in_quotes {
            if c == '"' {
                if chars.peek() == Some(&'"') {
                    field.push('"');
                    chars.next();
                } else {
                    in_quotes = false;
                }
            } else {
                field.push(c);
            }
        } else {
            match c {
                '"' => in_quotes = true,
                ',' => {
                    row.push(std::mem::take(&mut field));
                }
                '\r' => {}
                '\n' => {
                    row.push(std::mem::take(&mut field));
                    rows.push(std::mem::take(&mut row));
                }
                _ => field.push(c),
            }
        }
    }
    if !field.is_empty() || !row.is_empty() {
        row.push(field);
        rows.push(row);
    }
    rows
}
