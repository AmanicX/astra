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

document.addEventListener('DOMContentLoaded', () => {
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

  // 5. Catalog Search & Filter
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
});
