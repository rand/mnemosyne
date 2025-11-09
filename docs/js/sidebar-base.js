// Core sidebar logic - project-specific comments loaded from overrides
// Each project should load their override file BEFORE this base file

(function() {
    // Projects define these via override files:
    // - window.SIDEBAR_COMMENTS (section comments mapping)
    // - window.SIDEBAR_SUBSECTIONS (subsection comments mapping)
    // - window.SIDEBAR_DEFAULT (fallback message)

    function getSectionComments() {
        return window.SIDEBAR_COMMENTS || {};
    }

    function getSubsectionComments() {
        return window.SIDEBAR_SUBSECTIONS || {};
    }

    function getDefaultMessage() {
        return window.SIDEBAR_DEFAULT || '// Documentation';
    }

    function updateSidebarContent() {
        const sidebar = document.querySelector('.sidebar-tagline');
        if (!sidebar) return;

        const sectionComments = getSectionComments();
        const subsectionComments = getSubsectionComments();

        // Get all sections and headings
        const sections = [...document.querySelectorAll('section[id], h2[id]')];
        const headings = [...document.querySelectorAll('h2, h3')];

        // Account for navbar height
        const navbarHeight = 80;
        const scrollPosition = window.scrollY + navbarHeight + 50;

        // Find current section
        let currentSection = null;
        for (let i = sections.length - 1; i >= 0; i--) {
            if (scrollPosition >= sections[i].offsetTop) {
                currentSection = sections[i].id;
                break;
            }
        }

        // Find nearest h3 for more granular commentary
        let nearestH3 = null;
        let minDistance = Infinity;

        for (const heading of headings) {
            if (heading.tagName === 'H3') {
                const distance = Math.abs(scrollPosition - heading.offsetTop);
                if (distance < minDistance && scrollPosition >= heading.offsetTop - 100) {
                    minDistance = distance;
                    nearestH3 = heading.textContent.trim();
                }
            }
        }

        // Prioritize subsection commentary if we're close to an h3
        if (nearestH3 && subsectionComments[nearestH3] && minDistance < 300) {
            sidebar.textContent = subsectionComments[nearestH3];
        } else if (currentSection && sectionComments[currentSection]) {
            sidebar.textContent = sectionComments[currentSection];
        } else {
            sidebar.textContent = getDefaultMessage();
        }
    }

    // Initialize on page load
    function init() {
        updateSidebarContent();

        // Update on scroll with throttling
        let ticking = false;
        window.addEventListener('scroll', function() {
            if (!ticking) {
                window.requestAnimationFrame(function() {
                    updateSidebarContent();
                    ticking = false;
                });
                ticking = true;
            }
        });
    }

    // Run on DOMContentLoaded
    if (document.readyState === 'loading') {
        document.addEventListener('DOMContentLoaded', init);
    } else {
        init();
    }
})();
