/* ==========================================================================
   AMANICX UNIFIED THEME ENGINE
   ========================================================================== */

(function () {
  const STORAGE_KEY = 'amanicx_theme';

  function getSystemTheme() {
    return window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light';
  }

  function getSavedTheme() {
    try {
      return localStorage.getItem(STORAGE_KEY) || 'system';
    } catch (_) {
      return 'system';
    }
  }

  function applyTheme(mode) {
    const effective = mode === 'system' ? getSystemTheme() : mode;
    document.documentElement.setAttribute('data-theme', effective);
    document.documentElement.setAttribute('data-theme-setting', mode);

    // Update switcher UI if present
    document.querySelectorAll('.theme-btn').forEach(btn => {
      const btnMode = btn.getAttribute('data-mode');
      if (btnMode === mode) {
        btn.classList.add('active');
        btn.setAttribute('aria-pressed', 'true');
      } else {
        btn.classList.remove('active');
        btn.setAttribute('aria-pressed', 'false');
      }
    });

    // Notify listeners
    window.dispatchEvent(new CustomEvent('amanicx-theme-change', { detail: { mode, effective } }));
  }

  window.setAmanicxTheme = function (mode) {
    try {
      localStorage.setItem(STORAGE_KEY, mode);
    } catch (_) {}
    applyTheme(mode);
  };

  // Listen to system preference changes when in 'system' mode
  window.matchMedia('(prefers-color-scheme: dark)').addEventListener('change', () => {
    if (getSavedTheme() === 'system') {
      applyTheme('system');
    }
  });

  // Apply on DOM ready
  document.addEventListener('DOMContentLoaded', () => {
    applyTheme(getSavedTheme());

    // Bind all theme switcher buttons
    document.querySelectorAll('.theme-btn').forEach(btn => {
      btn.addEventListener('click', () => {
        const mode = btn.getAttribute('data-mode');
        if (mode) window.setAmanicxTheme(mode);
      });
    });
  });
})();
