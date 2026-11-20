/* ==========================================================================
   AMANICX CLIENT CONTROLLER: PLATFORMS, TABS, SEARCH, COPY
   ========================================================================== */

// 1. Platform Detection
window.detectPlatform = function () {
  const ua = navigator.userAgent || '';
  const plat = navigator.platform || '';

  if (/Android/i.test(ua)) return 'android';
  if (/iPhone|iPad|iPod/i.test(ua)) return 'ios';
  if (/Mac/i.test(plat) || /Macintosh/i.test(ua)) return 'macos';
  if (/Win/i.test(plat) || /Windows/i.test(ua)) return 'windows';
  if (/Arch/i.test(ua)) return 'arch';
  if (/Linux/i.test(plat) || /Linux/i.test(ua)) return 'linux';
  return 'unknown';
};

function initClient() {
  if (window.__amanicx_client_initialized) return;
  window.__amanicx_client_initialized = true;

  // 2. Setup Copy Buttons
  document.querySelectorAll('.copy-btn').forEach(btn => {
    btn.addEventListener('click', async () => {
      const targetId = btn.getAttribute('data-copy-target');
      const textToCopy = btn.getAttribute('data-copy-text') || 
        (targetId ? document.getElementById(targetId)?.innerText : btn.parentElement?.querySelector('code')?.innerText);

      if (!textToCopy) return;

      try {
        await navigator.clipboard.writeText(textToCopy.trim());
        const originalHtml = btn.innerHTML;
        btn.classList.add('copied');
        btn.innerHTML = `<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5"><polyline points="20 6 9 17 4 12"/></svg>`;
        setTimeout(() => {
          btn.innerHTML = originalHtml;
          btn.classList.remove('copied');
        }, 2200);
      } catch (err) {
        console.error('Clipboard copy error:', err);
      }
    });
  });

  // 3. Tab Switching
  document.querySelectorAll('[data-tabs-group]').forEach(group => {
    const groupId = group.getAttribute('data-tabs-group');
    const buttons = group.querySelectorAll('.tab-btn');
    const panes = document.querySelectorAll(`[data-tab-content="${groupId}"]`);

    buttons.forEach(btn => {
      btn.addEventListener('click', () => {
        const targetTab = btn.getAttribute('data-tab');

        buttons.forEach(b => b.classList.toggle('active', b === btn));
        panes.forEach(pane => {
          const isMatch = pane.getAttribute('data-tab-pane') === targetTab;
          pane.style.display = isMatch ? 'block' : 'none';
          if (isMatch) pane.classList.add('reveal', 'is-visible');
        });
      });
    });
  });

  // 4. Automatic OS Highlighting on Releases & Download Hubs
  const detected = window.detectPlatform();
  const platformTab = document.querySelector(`.tab-btn[data-platform="${detected}"]`) ||
                      (detected === 'arch' ? document.querySelector('.tab-btn[data-platform="linux"]') : null);
  if (platformTab) {
    platformTab.click();
    const highlightNotice = document.getElementById('detected-os-notice');
    if (highlightNotice) {
      highlightNotice.innerText = `Detected ${detected.toUpperCase()} - recommended package selected.`;
      highlightNotice.style.display = 'inline-flex';
    }
  }

  // 5. Expandable Pill Dropdown Menus
  document.querySelectorAll('.pill-dropdown').forEach(dropdown => {
    const trigger = dropdown.querySelector('.pill-dropdown-trigger');
    const menu = dropdown.querySelector('.pill-dropdown-menu');
    const items = dropdown.querySelectorAll('.pill-dropdown-item');
    const hiddenInput = dropdown.querySelector('input[type="hidden"]');
    const label = dropdown.querySelector('.selected-label');

    if (!trigger || !menu || !hiddenInput) return;

    trigger.addEventListener('click', (e) => {
      e.stopPropagation();
      const isOpen = dropdown.classList.contains('open');
      // Close any other open dropdowns
      document.querySelectorAll('.pill-dropdown.open').forEach(d => {
        if (d !== dropdown) d.classList.remove('open');
      });
      dropdown.classList.toggle('open', !isOpen);
      trigger.setAttribute('aria-expanded', !isOpen ? 'true' : 'false');
    });

    items.forEach(item => {
      item.addEventListener('click', (e) => {
        e.stopPropagation();
        const val = item.getAttribute('data-value') || 'all';
        items.forEach(i => i.classList.remove('active'));
        item.classList.add('active');
        if (label) label.textContent = item.textContent.trim();
        hiddenInput.value = val;
        dropdown.classList.remove('open');
        trigger.setAttribute('aria-expanded', 'false');

        // Dispatch change event to update filtered catalog
        hiddenInput.dispatchEvent(new Event('change', { bubbles: true }));
      });
    });
  });

  // Close dropdowns on outside click or Escape
  document.addEventListener('click', () => {
    document.querySelectorAll('.pill-dropdown.open').forEach(d => {
      d.classList.remove('open');
      d.querySelector('.pill-dropdown-trigger')?.setAttribute('aria-expanded', 'false');
    });
  });

  document.addEventListener('keydown', (e) => {
    if (e.key === 'Escape') {
      document.querySelectorAll('.pill-dropdown.open').forEach(d => {
        d.classList.remove('open');
        d.querySelector('.pill-dropdown-trigger')?.setAttribute('aria-expanded', 'false');
      });
    }
  });

  // 6. Catalog Search & Filter
  const catalogSearchInput = document.getElementById('catalog-search');
  const catalogFilterCategory = document.getElementById('filter-category');
  const catalogFilterPlatform = document.getElementById('filter-platform');
  const catalogItems = document.querySelectorAll('.catalog-item');
  const emptyState = document.getElementById('catalog-empty-state');
  const countBadge = document.getElementById('catalog-count');

  if (catalogSearchInput || catalogFilterCategory || catalogFilterPlatform) {
    function filterCatalog() {
      const q = (catalogSearchInput?.value || '').toLowerCase().trim();
      const cat = (catalogFilterCategory?.value || 'all').toLowerCase();
      const plat = (catalogFilterPlatform?.value || 'all').toLowerCase();

      let visibleCount = 0;

      catalogItems.forEach(item => {
        const title = (item.getAttribute('data-name') || '').toLowerCase();
        const desc = (item.getAttribute('data-desc') || '').toLowerCase();
        const tags = (item.getAttribute('data-tags') || '').toLowerCase();
        const itemCat = (item.getAttribute('data-category') || '').toLowerCase();
        const itemPlat = (item.getAttribute('data-platforms') || '').toLowerCase();

        const matchesQuery = !q || title.includes(q) || desc.includes(q) || tags.includes(q);
        const matchesCat = cat === 'all' || itemCat.includes(cat);
        const matchesPlat = plat === 'all' || itemPlat.includes(plat);

        if (matchesQuery && matchesCat && matchesPlat) {
          item.style.display = '';
          visibleCount++;
        } else {
          item.style.display = 'none';
        }
      });

      if (emptyState) emptyState.style.display = visibleCount === 0 ? 'block' : 'none';
      if (countBadge) countBadge.innerText = `${visibleCount} ${visibleCount === 1 ? 'Project' : 'Projects'}`;
    }

    if (catalogSearchInput) catalogSearchInput.addEventListener('input', filterCatalog);
    if (catalogFilterCategory) catalogFilterCategory.addEventListener('change', filterCatalog);
    if (catalogFilterPlatform) catalogFilterPlatform.addEventListener('change', filterCatalog);
  }

  // 7. Mobile Navigation Drawer Controller
  const navToggleBtn = document.querySelector('.nav-toggle-btn');
  const mobileDrawer = document.getElementById('mobile-nav-drawer');

  if (navToggleBtn && mobileDrawer) {
    function openMobileDrawer() {
      navToggleBtn.setAttribute('aria-expanded', 'true');
      mobileDrawer.removeAttribute('hidden');
      // Trigger reflow to start CSS transition
      void mobileDrawer.offsetWidth;
      mobileDrawer.classList.add('is-open');
      document.body.classList.add('menu-open');

      // Focus first link or close button inside drawer
      const focusable = mobileDrawer.querySelector('button, [href], input, select, textarea, [tabindex]:not([tabindex="-1"])');
      if (focusable) focusable.focus();
    }

    function closeMobileDrawer() {
      navToggleBtn.setAttribute('aria-expanded', 'false');
      mobileDrawer.classList.remove('is-open');
      document.body.classList.remove('menu-open');
      setTimeout(() => {
        if (!mobileDrawer.classList.contains('is-open')) {
          mobileDrawer.setAttribute('hidden', '');
        }
      }, 300);
      navToggleBtn.focus();
    }

    navToggleBtn.addEventListener('click', () => {
      const isExpanded = navToggleBtn.getAttribute('aria-expanded') === 'true';
      if (isExpanded) {
        closeMobileDrawer();
      } else {
        openMobileDrawer();
      }
    });

    mobileDrawer.querySelectorAll('[data-action="close-menu"]').forEach(el => {
      el.addEventListener('click', closeMobileDrawer);
    });

    // Close when navigating via links in drawer
    mobileDrawer.querySelectorAll('.mobile-nav-link').forEach(link => {
      link.addEventListener('click', () => {
        closeMobileDrawer();
      });
    });

    // Handle Escape key and focus trapping inside drawer
    document.addEventListener('keydown', (e) => {
      if (!mobileDrawer.classList.contains('is-open')) return;

      if (e.key === 'Escape') {
        e.preventDefault();
        closeMobileDrawer();
        return;
      }

      if (e.key === 'Tab') {
        const focusables = Array.from(
          mobileDrawer.querySelectorAll('button:not([disabled]), [href], input:not([disabled]), [tabindex]:not([tabindex="-1"])')
        );
        if (focusables.length === 0) return;

        const first = focusables[0];
        const last = focusables[focusables.length - 1];

        if (e.shiftKey) {
          if (document.activeElement === first) {
            e.preventDefault();
            last.focus();
          }
        } else {
          if (document.activeElement === last) {
            e.preventDefault();
            first.focus();
          }
        }
      }
    });
  }
}

if (document.readyState === 'loading') {
  document.addEventListener('DOMContentLoaded', initClient);
} else {
  initClient();
}
