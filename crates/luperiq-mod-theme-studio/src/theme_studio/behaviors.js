// behaviors.js — compiled into binary via include_str!()
// Served at /api/modules/theme-studio/behaviors.js
// Implements all BlockBehavior enum variants.
// SECURITY: No eval, no Function constructor, no dynamic import, no network requests.
// Each behavior function receives only its block's DOM element.
(function() {
    'use strict';

    var behaviors = {

        // ── Accordion ───────────────────────────────────────────────
        // Expects: .accordion-item > .accordion-trigger + .accordion-panel
        // Data attrs: data-multi="true" allows multiple panels open
        accordion: function(el) {
            var triggers = el.querySelectorAll('.accordion-trigger, summary');
            var multi = el.dataset.multi === 'true';
            triggers.forEach(function(btn) {
                // For <details>/<summary> elements, let the browser handle it
                if (btn.tagName === 'SUMMARY') return;
                btn.addEventListener('click', function() {
                    var item = btn.closest('.accordion-item');
                    if (!item) return;
                    if (!multi) {
                        el.querySelectorAll('.accordion-item.is-open').forEach(function(other) {
                            if (other !== item) other.classList.remove('is-open');
                        });
                    }
                    item.classList.toggle('is-open');
                });
            });
        },

        // ── Tabs ────────────────────────────────────────────────────
        // Expects: .tab-btn[data-tab="N"] + .tab-panel[data-tab="N"]
        tabs: function(el) {
            var buttons = el.querySelectorAll('.tab-btn, [data-tab-trigger]');
            var panels = el.querySelectorAll('.tab-panel, [data-tab-panel]');
            buttons.forEach(function(btn) {
                btn.addEventListener('click', function() {
                    var tabId = btn.dataset.tab || btn.dataset.tabTrigger;
                    buttons.forEach(function(b) { b.classList.remove('active'); });
                    panels.forEach(function(p) { p.classList.remove('active'); });
                    btn.classList.add('active');
                    var target = el.querySelector('.tab-panel[data-tab="' + tabId + '"], [data-tab-panel="' + tabId + '"]');
                    if (target) target.classList.add('active');
                });
            });
        },

        // ── Carousel ────────────────────────────────────────────────
        // Expects: .carousel-track > .carousel-slide, optional .carousel-prev/.carousel-next
        // Data attrs: data-autoplay="5000" (ms), data-loop="true"
        carousel: function(el) {
            var track = el.querySelector('.carousel-track');
            var slides = el.querySelectorAll('.carousel-slide');
            if (!track || slides.length === 0) return;
            var current = 0;
            var total = slides.length;
            var autoplay = parseInt(el.dataset.autoplay, 10) || 0;
            var loop = el.dataset.loop !== 'false';

            function goTo(idx) {
                if (loop) {
                    idx = ((idx % total) + total) % total;
                } else {
                    idx = Math.max(0, Math.min(idx, total - 1));
                }
                current = idx;
                track.style.transform = 'translateX(-' + (current * 100) + '%)';
                slides.forEach(function(s, i) {
                    s.classList.toggle('active', i === current);
                });
            }

            var prev = el.querySelector('.carousel-prev');
            var next = el.querySelector('.carousel-next');
            if (prev) prev.addEventListener('click', function() { goTo(current - 1); });
            if (next) next.addEventListener('click', function() { goTo(current + 1); });

            if (autoplay > 0) {
                setInterval(function() { goTo(current + 1); }, autoplay);
            }
            goTo(0);
        },

        // ── Lightbox ────────────────────────────────────────────────
        // Expects: img or a[href] elements inside the block
        lightbox: function(el) {
            var images = el.querySelectorAll('img, a[data-lightbox]');
            images.forEach(function(img) {
                img.style.cursor = 'zoom-in';
                img.addEventListener('click', function(e) {
                    e.preventDefault();
                    var src = img.tagName === 'A' ? img.href : img.src;
                    var overlay = document.createElement('div');
                    overlay.style.cssText = 'position:fixed;inset:0;background:rgba(0,0,0,0.9);z-index:99999;display:flex;align-items:center;justify-content:center;cursor:zoom-out;';
                    var fullImg = document.createElement('img');
                    fullImg.src = src;
                    fullImg.style.cssText = 'max-width:90vw;max-height:90vh;object-fit:contain;border-radius:4px;';
                    overlay.appendChild(fullImg);
                    overlay.addEventListener('click', function() { overlay.remove(); });
                    document.addEventListener('keydown', function handler(ev) {
                        if (ev.key === 'Escape') { overlay.remove(); document.removeEventListener('keydown', handler); }
                    });
                    document.body.appendChild(overlay);
                });
            });
        },

        // ── Counter ─────────────────────────────────────────────────
        // Expects: element with data-target="1000" (the number to count to)
        // Data attrs: data-duration="2000" (ms)
        counter: function(el) {
            var target = parseInt(el.dataset.target, 10) || 0;
            var duration = parseInt(el.dataset.duration, 10) || 2000;
            var display = el.querySelector('.counter-value') || el;
            var started = false;

            function animate() {
                if (started) return;
                started = true;
                var start = 0;
                var startTime = performance.now();
                function step(now) {
                    var progress = Math.min((now - startTime) / duration, 1);
                    var current = Math.floor(progress * target);
                    display.textContent = current.toLocaleString();
                    if (progress < 1) requestAnimationFrame(step);
                    else display.textContent = target.toLocaleString();
                }
                requestAnimationFrame(step);
            }

            // Start when visible
            if ('IntersectionObserver' in window) {
                var observer = new IntersectionObserver(function(entries) {
                    if (entries[0].isIntersecting) { animate(); observer.disconnect(); }
                }, { threshold: 0.5 });
                observer.observe(el);
            } else {
                animate();
            }
        },

        // ── Toggle ──────────────────────────────────────────────────
        // Generic show/hide. Expects: .toggle-trigger + .toggle-content
        toggle: function(el) {
            var trigger = el.querySelector('.toggle-trigger');
            var content = el.querySelector('.toggle-content');
            if (!trigger || !content) return;
            trigger.addEventListener('click', function() {
                el.classList.toggle('is-open');
            });
        },

        // ── Dropdown ────────────────────────────────────────────────
        // Expects: .dropdown-trigger + .dropdown-menu
        dropdown: function(el) {
            var trigger = el.querySelector('.dropdown-trigger');
            var menu = el.querySelector('.dropdown-menu');
            if (!trigger || !menu) return;
            trigger.addEventListener('click', function(e) {
                e.stopPropagation();
                el.classList.toggle('is-open');
            });
            document.addEventListener('click', function() {
                el.classList.remove('is-open');
            });
        },

        // ── Copy Code ───────────────────────────────────────────────
        // Expects: pre or code element inside the block
        copy_code: function(el) {
            var code = el.querySelector('pre, code');
            if (!code) return;
            var btn = document.createElement('button');
            btn.textContent = 'Copy';
            btn.style.cssText = 'position:absolute;top:8px;right:8px;padding:4px 12px;border-radius:4px;border:1px solid rgba(255,255,255,0.2);background:rgba(0,0,0,0.3);color:#fff;cursor:pointer;font-size:12px;';
            el.style.position = 'relative';
            el.appendChild(btn);
            btn.addEventListener('click', function() {
                var text = code.textContent;
                if (navigator.clipboard) {
                    navigator.clipboard.writeText(text).then(function() {
                        btn.textContent = 'Copied!';
                        setTimeout(function() { btn.textContent = 'Copy'; }, 2000);
                    });
                }
            });
        },

        // ── Before/After ────────────────────────────────────────────
        // Image comparison slider. Expects: .ba-before img + .ba-after img
        before_after: function(el) {
            var before = el.querySelector('.ba-before, .before-after-before');
            var after = el.querySelector('.ba-after, .before-after-after');
            if (!before || !after) return;
            el.style.position = 'relative';
            el.style.overflow = 'hidden';
            el.style.cursor = 'col-resize';
            after.style.position = 'absolute';
            after.style.top = '0';
            after.style.left = '0';
            after.style.width = '100%';
            after.style.height = '100%';
            after.style.clipPath = 'inset(0 50% 0 0)';

            var dragging = false;
            function updatePosition(x) {
                var rect = el.getBoundingClientRect();
                var pct = Math.max(0, Math.min(100, ((x - rect.left) / rect.width) * 100));
                after.style.clipPath = 'inset(0 ' + (100 - pct) + '% 0 0)';
            }
            el.addEventListener('mousedown', function() { dragging = true; });
            document.addEventListener('mouseup', function() { dragging = false; });
            el.addEventListener('mousemove', function(e) { if (dragging) updatePosition(e.clientX); });
            el.addEventListener('touchmove', function(e) {
                if (e.touches.length) updatePosition(e.touches[0].clientX);
            }, { passive: true });
        },

        // ── Form Validation ─────────────────────────────────────────
        // Client-side validation for form blocks
        form_validation: function(el) {
            var form = el.querySelector('form') || el;
            form.addEventListener('submit', function(e) {
                var valid = true;
                form.querySelectorAll('[required]').forEach(function(field) {
                    if (!field.value.trim()) {
                        valid = false;
                        field.style.borderColor = '#ef4444';
                        field.addEventListener('input', function handler() {
                            if (field.value.trim()) {
                                field.style.borderColor = '';
                                field.removeEventListener('input', handler);
                            }
                        });
                    }
                });
                if (!valid) e.preventDefault();
            });
        },

        // ── Lazy Load ───────────────────────────────────────────────
        // Defer loading images until visible
        lazy_load: function(el) {
            var images = el.querySelectorAll('img[data-src]');
            if ('IntersectionObserver' in window) {
                var observer = new IntersectionObserver(function(entries) {
                    entries.forEach(function(entry) {
                        if (entry.isIntersecting) {
                            var img = entry.target;
                            img.src = img.dataset.src;
                            img.removeAttribute('data-src');
                            observer.unobserve(img);
                        }
                    });
                }, { rootMargin: '200px' });
                images.forEach(function(img) { observer.observe(img); });
            } else {
                images.forEach(function(img) { img.src = img.dataset.src; });
            }
        },

        // ── Smooth Scroll ───────────────────────────────────────────
        // Smooth-scroll anchor links within the block
        smooth_scroll: function(el) {
            el.querySelectorAll('a[href^="#"]').forEach(function(link) {
                link.addEventListener('click', function(e) {
                    var target = document.querySelector(link.getAttribute('href'));
                    if (target) {
                        e.preventDefault();
                        target.scrollIntoView({ behavior: 'smooth', block: 'start' });
                    }
                });
            });
        }
    };

    // ── Init ────────────────────────────────────────────────────────
    function init() {
        document.querySelectorAll('[data-behavior]').forEach(function(el) {
            var name = el.dataset.behavior;
            if (name && behaviors[name]) {
                try { behaviors[name](el); }
                catch(e) { /* silently skip broken behavior */ }
            }
        });
    }

    if (document.readyState === 'loading') {
        document.addEventListener('DOMContentLoaded', init);
    } else {
        init();
    }
})();
