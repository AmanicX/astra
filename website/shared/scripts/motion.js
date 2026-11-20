/* ==========================================================================
   AMANICX MOTION & SCROLL CONTROLLER
   ========================================================================== */

function initMotion() {
  if (window.__amanicx_motion_initialized) return;
  window.__amanicx_motion_initialized = true;

  const prefersReduced = window.matchMedia('(prefers-reduced-motion: reduce)').matches;

  // 1. Scroll Progress Bar
  const progressBar = document.getElementById('scroll-progress');
  if (progressBar) {
    window.addEventListener('scroll', () => {
      const h = document.documentElement;
      const st = h.scrollTop || document.body.scrollTop;
      const sh = h.scrollHeight || document.body.scrollHeight;
      const percent = (st / (sh - h.clientHeight)) * 100;
      progressBar.style.width = Math.min(100, Math.max(0, percent)) + '%';
    }, { passive: true });
  }

  // 2. Reveal on Scroll (Intersection Observer with Progressive Enhancement)
  const reveals = document.querySelectorAll('.reveal');

  if (reveals.length > 0 && !prefersReduced && 'IntersectionObserver' in window) {
    const winH = window.innerHeight || document.documentElement.clientHeight || 900;

    // Immediately mark initial viewport elements visible so above-the-fold never flashes invisible
    reveals.forEach(el => {
      const rect = el.getBoundingClientRect();
      if (rect.top <= winH) {
        el.classList.add('is-visible', 'no-delay');
      }
    });

    // Enable scroll-motion styles for subsequent elements
    requestAnimationFrame(() => {
      document.documentElement.classList.add('has-motion');
    });

    const observer = new IntersectionObserver((entries) => {
      entries.forEach(entry => {
        if (entry.isIntersecting) {
          entry.target.classList.add('is-visible');
          observer.unobserve(entry.target);
        }
      });
    }, { threshold: 0.02, rootMargin: '100px 0px 100px 0px' });

    reveals.forEach(el => {
      if (!el.classList.contains('is-visible')) {
        observer.observe(el);
      }
    });
  } else {
    reveals.forEach(el => el.classList.add('is-visible'));
  }

  // 3. Nexora-inspired Scroll-Linked 3D Scrubbing on Story Stages
  const storyCards = document.querySelectorAll('.story-card-3d');
  if (storyCards.length > 0) {
    let ticking = false;

    function updateStoryTransforms() {
      if (window.innerWidth <= 860) {
        storyCards.forEach(card => {
          card.style.removeProperty('--rx');
          card.style.removeProperty('--ry');
          card.style.removeProperty('--ty');
        });
        ticking = false;
        return;
      }
      storyCards.forEach(card => {
        const rect = card.getBoundingClientRect();
        const winH = window.innerHeight;
        const normDist = ((rect.top + rect.height / 2) - (winH / 2)) / (winH / 2);
        const rx = normDist * 6;
        const ry = normDist * -2;
        const ty = normDist * -12;
        card.style.setProperty('--rx', `${rx.toFixed(2)}deg`);
        card.style.setProperty('--ry', `${ry.toFixed(2)}deg`);
        card.style.setProperty('--ty', `${ty.toFixed(2)}px`);
      });
      ticking = false;
    }

    window.addEventListener('scroll', () => {
      if (!ticking) {
        window.requestAnimationFrame(updateStoryTransforms);
        ticking = true;
      }
    }, { passive: true });
    updateStoryTransforms();
  }
}

if (document.readyState === 'loading') {
  document.addEventListener('DOMContentLoaded', initMotion);
} else {
  initMotion();
}
