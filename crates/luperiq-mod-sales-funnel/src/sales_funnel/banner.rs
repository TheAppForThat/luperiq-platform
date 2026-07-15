//! Demo banner HTML generation — fixed-position banner for demo sites and
//! industry CTA modal with action cards.

/// Generate demo banner HTML for a demo site.
///
/// All parameters are passed in — nothing is hardcoded.
///
/// - `industry_name`: Human-readable industry name (e.g., "Pest Control")
/// - `accent_color`: CSS color for the accent (e.g., "#2563eb")
/// - `platform_url`: Full URL to the platform (e.g., "https://luperiq.com")
/// - `industry_page_slug`: Slug for the industry page (e.g., "pest-control")
pub fn demo_banner_html(
    industry_name: &str,
    accent_color: &str,
    platform_url: &str,
    industry_page_slug: &str,
) -> String {
    format!(
        r##"<div id="liq-demo-banner" style="
    position:fixed;bottom:0;left:0;right:0;z-index:99999;
    background:{accent};color:#fff;padding:10px 20px;
    font-family:-apple-system,BlinkMacSystemFont,'Segoe UI',Roboto,sans-serif;
    font-size:14px;display:flex;align-items:center;justify-content:center;gap:12px;
    box-shadow:0 -2px 12px rgba(0,0,0,0.15);
">
    <span>This is an example <strong>{industry}</strong> website &mdash; built with LuperIQ</span>
    <a href="{url}/{slug}" target="_blank" rel="noopener" style="
        display:inline-block;padding:6px 18px;border-radius:6px;
        background:#fff;color:{accent};font-weight:600;text-decoration:none;
        font-size:13px;white-space:nowrap;
    ">Build Yours</a>
    <button id="liq-demo-banner-dismiss" aria-label="Dismiss banner" style="
        background:none;border:none;color:#fff;font-size:20px;cursor:pointer;
        padding:0 4px;line-height:1;opacity:0.7;margin-left:4px;
    ">&times;</button>
</div>"##,
        accent = accent_color,
        industry = industry_name,
        url = platform_url,
        slug = industry_page_slug,
    )
}

/// JavaScript for the demo banner dismiss behavior.
///
/// Sets a cookie `liq_demo_banner_dismissed=1` valid for 7 days, then hides
/// the banner. On page load, checks for the cookie and hides if already set.
pub fn demo_banner_js() -> String {
    r##"(function(){
    function getCookie(n){var m=document.cookie.match(new RegExp('(?:^|;\\s*)'+n+'=([^;]*)'));return m?m[1]:null;}
    function setCookie(n,v,d){var e=new Date();e.setTime(e.getTime()+(d*864e5));document.cookie=n+'='+v+';expires='+e.toUTCString()+';path=/;SameSite=Lax';}
    if(getCookie('liq_demo_banner_dismissed')==='1'){
        var b=document.getElementById('liq-demo-banner');
        if(b)b.style.display='none';
        return;
    }
    document.addEventListener('click',function(e){
        if(e.target&&e.target.id==='liq-demo-banner-dismiss'){
            setCookie('liq_demo_banner_dismissed','1',7);
            var b=document.getElementById('liq-demo-banner');
            if(b)b.style.display='none';
        }
    });
})();"##
        .to_string()
}

/// JavaScript for the industry CTA modal.
///
/// Opens a modal with 4 action cards. Closes on backdrop click or Escape key.
/// All URLs are constructed from data attributes on the trigger element:
/// - `data-platform-url` — base platform URL
/// - `data-industry-slug` — slug for the industry page
/// - `data-industry-name` — human-readable industry name
pub fn cta_modal_js() -> String {
    r##"(function(){
    var _ctaOverlay=null;

    function _ctaOpen(platformUrl, industrySlug, industryName) {
        if(_ctaOverlay) return;

        _ctaOverlay=document.createElement('div');
        _ctaOverlay.id='liq-cta-overlay';
        _ctaOverlay.style.cssText='position:fixed;inset:0;z-index:100000;background:rgba(0,0,0,0.5);display:flex;align-items:center;justify-content:center;';

        var modal=document.createElement('div');
        modal.style.cssText='background:#fff;border-radius:12px;padding:32px;max-width:560px;width:90%;box-shadow:0 8px 32px rgba(0,0,0,0.2);position:relative;';

        var closeBtn=document.createElement('button');
        closeBtn.textContent='\u00d7';
        closeBtn.style.cssText='position:absolute;top:12px;right:16px;background:none;border:none;font-size:24px;cursor:pointer;color:#666;line-height:1;';
        closeBtn.onclick=_ctaClose;
        modal.appendChild(closeBtn);

        var title=document.createElement('h2');
        title.style.cssText='margin:0 0 8px;font-size:22px;color:#1a1a2e;';
        title.textContent='Get Your '+industryName+' Website';
        modal.appendChild(title);

        var subtitle=document.createElement('p');
        subtitle.style.cssText='margin:0 0 24px;color:#666;font-size:14px;';
        subtitle.textContent='Start your own site free for 7 days. No card needed.';
        modal.appendChild(subtitle);

        var grid=document.createElement('div');
        grid.style.cssText='display:grid;grid-template-columns:1fr 1fr;gap:12px;';

        var cards=[
            {label:'Learn More',desc:'See all features',href:platformUrl+'/'+industrySlug,icon:'\ud83d\udcda'},
            {label:'See Example',desc:'Browse the demo site',href:platformUrl+'/demo/'+industrySlug,icon:'\ud83d\udd0d'},
            {label:'Start 7-Day Free Trial',desc:'Every feature unlocked for 7 days. No card, no commitment.',href:platformUrl+'/start-free?industry='+industrySlug,icon:'\ud83d\ude80'},
            {label:'See Pricing',desc:'Community $9, Creator $19, Business $49 per month. Lifetime 50% off now.',href:platformUrl+'/pricing?industry='+industrySlug,icon:'\ud83d\udcb3'}
        ];
        cards.forEach(function(c){
            var a=document.createElement('a');
            a.href=c.href;
            a.target='_blank';
            a.rel='noopener';
            a.style.cssText='display:block;padding:16px;border:1px solid #e5e7eb;border-radius:10px;text-decoration:none;color:#1a1a2e;text-align:center;transition:border-color 0.2s,box-shadow 0.2s;';
            a.onmouseenter=function(){a.style.borderColor='#2563eb';a.style.boxShadow='0 2px 8px rgba(37,99,235,0.15)';};
            a.onmouseleave=function(){a.style.borderColor='#e5e7eb';a.style.boxShadow='none';};
            var icon=document.createElement('div');
            icon.style.cssText='font-size:28px;margin-bottom:6px;';
            icon.textContent=c.icon;
            a.appendChild(icon);
            var lbl=document.createElement('div');
            lbl.style.cssText='font-weight:600;font-size:14px;';
            lbl.textContent=c.label;
            a.appendChild(lbl);
            var desc=document.createElement('div');
            desc.style.cssText='font-size:12px;color:#888;margin-top:2px;';
            desc.textContent=c.desc;
            a.appendChild(desc);
            grid.appendChild(a);
        });
        modal.appendChild(grid);
        _ctaOverlay.appendChild(modal);
        document.body.appendChild(_ctaOverlay);

        _ctaOverlay.addEventListener('click',function(e){
            if(e.target===_ctaOverlay) _ctaClose();
        });
        document.addEventListener('keydown',_ctaEsc);
    }

    function _ctaClose(){
        if(_ctaOverlay){_ctaOverlay.remove();_ctaOverlay=null;}
        document.removeEventListener('keydown',_ctaEsc);
    }

    function _ctaEsc(e){if(e.key==='Escape')_ctaClose();}

    // Expose globally for trigger elements
    window.liqCtaOpen=_ctaOpen;

    // Auto-bind elements with class "liq-cta-trigger"
    document.addEventListener('click',function(e){
        var trigger=e.target.closest('.liq-cta-trigger');
        if(!trigger)return;
        e.preventDefault();
        var url=trigger.getAttribute('data-platform-url')||'';
        var slug=trigger.getAttribute('data-industry-slug')||'';
        var name=trigger.getAttribute('data-industry-name')||'';
        _ctaOpen(url,slug,name);
    });
})();"##
        .to_string()
}
