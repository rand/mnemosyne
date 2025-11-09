// Theme toggle functionality - Simple two-state light/dark toggle
(function() {
    // Theme key should be project-specific when used
    // Override THEME_KEY in project-specific wrapper if needed
    const THEME_KEY = window.THEME_KEY || 'docs-theme';

    // Get saved theme, default to light
    function getSavedTheme() {
        return localStorage.getItem(THEME_KEY) || 'light';
    }

    // Save theme to localStorage
    function saveTheme(theme) {
        localStorage.setItem(THEME_KEY, theme);
    }

    // Apply theme to body
    function applyTheme(theme) {
        document.body.classList.remove('light-theme', 'dark-theme');
        document.body.classList.add(theme + '-theme');
    }

    // Toggle between light and dark
    function toggleTheme() {
        const currentTheme = getSavedTheme();
        const newTheme = currentTheme === 'light' ? 'dark' : 'light';

        saveTheme(newTheme);
        applyTheme(newTheme);
    }

    // Initialize theme on page load
    function initTheme() {
        const savedTheme = getSavedTheme();
        applyTheme(savedTheme);

        // Add event listener to theme toggle button
        const toggleButton = document.querySelector('.theme-toggle');
        if (toggleButton) {
            toggleButton.addEventListener('click', toggleTheme);
        }
    }

    // Run on DOMContentLoaded
    if (document.readyState === 'loading') {
        document.addEventListener('DOMContentLoaded', initTheme);
    } else {
        initTheme();
    }
})();
