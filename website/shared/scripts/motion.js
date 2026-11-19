/* ==========================================================================
   AMANICX MOTION & SCROLL CONTROLLER
   ========================================================================== */

document.addEventListener('DOMContentLoaded', () => {
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

  if (prefersReduced) {
    document.querySelectorAll('.reveal').forEach(el => el.classList.add('is-visible'));
    return;
  }

  // 2. Reveal on Scroll (Intersection Observer)
  const reveals = document.querySelectorAll('.reveal');
  if (reveals.length > 0 && 'IntersectionObserver' in window) {
    const observer = new IntersectionObserver((entries) => {
      entries.forEach(entry => {
        if (entry.isIntersecting) {
          entry.target.classList.add('is-visible');
          observer.unobserve(entry.target);
        }
      });
    }, { threshold: 0.15, rootMargin: '0px 0px -40px 0px' });

    reveals.forEach(el => observer.observe(el));
  } else {
    reveals.forEach(el => el.classList.add('is-visible'));
  }

  // 3. Nexora-inspired Scroll-Linked 3D Scrubbing on Story Stages
  const storyCards = document.querySelectorAll('.story-card-3d');
  if (storyCards.length > 0) {
    let ticking = false;

    function updateStoryTransforms() {
      storyCards.forEach(card => {
        const rect = card.getBoundingClientRect();
        const winH = window.innerHeight;
        // Calculate progress from 0 (enters bottom) to 1 (exits top)
        const progress = Math.min(1, Math.max(0, (winH - rect.top) / (winH + rect.height)));
        
        // Subtle tilt and lift based on scroll center
        const centerDist = (rect.top + rect.height / 2) - (winH / 2);
        const normDist = centerDist / (winH / 2); // -1 (top) to +1 (bottom)
        
        const rx = normDist * 6; // max 6 deg tilt
        const ry = (normDist * -2);
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
});
