// Theme Core - Light/Dark Mode Toggle with Picture Element Support
// Provides structure without imposing identity (no project-specific defaults)

(function() {
    'use strict';

    // Sites can override THEME_KEY by setting window.THEME_KEY before loading this script
    const THEME_KEY = window.THEME_KEY || 'docs-theme';

    /**
     * Get saved theme from localStorage
     * @returns {string} 'light' or 'dark'
     */
    function getSavedTheme() {
        const saved = localStorage.getItem(THEME_KEY);
        if (saved === 'light' || saved === 'dark') {
            return saved;
        }

        // Default to system preference
        if (window.matchMedia && window.matchMedia('(prefers-color-scheme: dark)').matches) {
            return 'dark';
        }

        return 'light';
    }

    /**
     * Save theme to localStorage
     * @param {string} theme - 'light' or 'dark'
     */
    function saveTheme(theme) {
        localStorage.setItem(THEME_KEY, theme);
    }

    /**
     * Update picture elements based on theme
     * Switches between light/dark diagram sources
     * @param {string} theme - 'light' or 'dark'
     */
    function updateDiagrams(theme) {
        const pictures = document.querySelectorAll('picture');

        pictures.forEach(picture => {
            const sources = picture.querySelectorAll('source');

            sources.forEach(source => {
                const media = source.getAttribute('media');

                if (media && media.includes('prefers-color-scheme')) {
                    const isDarkSource = media.includes('dark');

                    // Enable source that matches current theme
                    if (theme === 'dark') {
                        source.disabled = !isDarkSource;
                    } else {
                        source.disabled = isDarkSource;
                    }
                }
            });

            // Force picture to re-evaluate sources
            const img = picture.querySelector('img');
            if (img) {
                // Trigger reload by briefly changing src
                const currentSrc = img.src;
                img.src = '';
                img.src = currentSrc;
            }
        });
    }

    /**
     * Apply theme to document body
     * @param {string} theme - 'light' or 'dark'
     */
    function applyTheme(theme) {
        // Remove existing theme classes
        document.body.classList.remove('light-theme', 'dark-theme');

        // Add new theme class
        document.body.classList.add(theme + '-theme');

        // Update diagrams
        updateDiagrams(theme);

        // Dispatch custom event for extensions
        document.dispatchEvent(new CustomEvent('themeChanged', {
            detail: { theme }
        }));
    }

    /**
     * Toggle between light and dark themes
     */
    function toggleTheme() {
        const currentTheme = getSavedTheme();
        const newTheme = currentTheme === 'light' ? 'dark' : 'light';

        saveTheme(newTheme);
        applyTheme(newTheme);
    }

    /**
     * Initialize theme on page load
     */
    function initTheme() {
        const savedTheme = getSavedTheme();
        applyTheme(savedTheme);

        // Attach event listener to theme toggle button
        const toggleButton = document.querySelector('.theme-toggle');
        if (toggleButton) {
            toggleButton.addEventListener('click', toggleTheme);
        }

        // Listen for system theme changes (if user hasn't set preference)
        if (window.matchMedia) {
            const darkModeQuery = window.matchMedia('(prefers-color-scheme: dark)');

            darkModeQuery.addEventListener('change', (e) => {
                // Only auto-switch if user hasn't explicitly set a preference
                const hasExplicitPreference = localStorage.getItem(THEME_KEY);
                if (!hasExplicitPreference) {
                    applyTheme(e.matches ? 'dark' : 'light');
                }
            });
        }
    }

    // Export API for programmatic access
    window.themeCore = {
        get: getSavedTheme,
        set: function(theme) {
            if (theme !== 'light' && theme !== 'dark') {
                console.warn('Invalid theme:', theme, '- must be "light" or "dark"');
                return;
            }
            saveTheme(theme);
            applyTheme(theme);
        },
        toggle: toggleTheme,
        apply: applyTheme
    };

    // Initialize on DOMContentLoaded
    if (document.readyState === 'loading') {
        document.addEventListener('DOMContentLoaded', initTheme);
    } else {
        initTheme();
    }
})();
