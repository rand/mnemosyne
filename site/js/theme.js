// Theme toggle functionality with SVG diagram switching
(function() {
    'use strict';

    const THEME_KEY = 'theme-preference';

    // Get current theme
    function getCurrentTheme() {
        return localStorage.getItem(THEME_KEY) || 'light';
    }

    // Save theme preference
    function saveTheme(theme) {
        localStorage.setItem(THEME_KEY, theme);
    }

    // Update SVG diagrams based on theme
    function updateDiagrams(theme) {
        const pictures = document.querySelectorAll('picture');
        pictures.forEach(picture => {
            const sources = picture.querySelectorAll('source');
            const img = picture.querySelector('img');

            sources.forEach(source => {
                const media = source.getAttribute('media');
                // Hide/show sources based on theme
                if (theme === 'dark' && media === '(prefers-color-scheme: dark)') {
                    source.disabled = false;
                } else if (theme === 'light' && media === '(prefers-color-scheme: dark)') {
                    source.disabled = true;
                }
            });

            // Update img src directly for theme
            if (img) {
                const srcPath = img.src || img.getAttribute('src');
                if (srcPath) {
                    const basePath = srcPath.replace(/-light\.svg$/, '.svg').replace(/-dark\.svg$/, '.svg');
                    const newSrc = basePath.replace(/\.svg$/, theme === 'dark' ? '-dark.svg' : '-light.svg');

                    // Check if themed version exists, otherwise use base
                    const testSrc = newSrc.includes('-light.svg') || newSrc.includes('-dark.svg') ? newSrc : srcPath;
                    img.src = testSrc;
                }
            }
        });
    }

    // Apply theme to document
    function applyTheme(theme) {
        document.body.classList.remove('light-theme', 'dark-theme');
        document.body.classList.add(`${theme}-theme`);
        updateDiagrams(theme);
    }

    // Toggle theme
    function toggleTheme() {
        const current = getCurrentTheme();
        const newTheme = current === 'light' ? 'dark' : 'light';
        saveTheme(newTheme);
        applyTheme(newTheme);
    }

    // Initialize theme
    function initTheme() {
        const savedTheme = getCurrentTheme();
        applyTheme(savedTheme);

        // Add toggle listener
        const toggleBtn = document.querySelector('.theme-toggle');
        if (toggleBtn) {
            toggleBtn.addEventListener('click', toggleTheme);
        }
    }

    // Run on load
    if (document.readyState === 'loading') {
        document.addEventListener('DOMContentLoaded', initTheme);
    } else {
        initTheme();
    }
})();
