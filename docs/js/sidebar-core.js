// Sidebar Core - Scroll Tracking and Navigation
// Monitors scroll position and highlights current section

(function() {
    'use strict';

    // Configuration
    const SCROLL_OFFSET = 100; // Pixels from top to consider "active"
    const DEBOUNCE_DELAY = 100; // Milliseconds to debounce scroll events

    // State
    let currentSection = null;
    let sections = [];
    let observer = null;

    /**
     * Debounce function to limit scroll event frequency
     * @param {Function} func - Function to debounce
     * @param {number} wait - Milliseconds to wait
     * @returns {Function} Debounced function
     */
    function debounce(func, wait) {
        let timeout;
        return function executedFunction(...args) {
            const later = () => {
                clearTimeout(timeout);
                func(...args);
            };
            clearTimeout(timeout);
            timeout = setTimeout(later, wait);
        };
    }

    /**
     * Get all section elements (h2 and h3 headings)
     * @returns {Array} Array of section objects with element and id
     */
    function getSections() {
        const headings = document.querySelectorAll('h2[id], h3[id]');
        return Array.from(headings).map(heading => ({
            id: heading.id,
            element: heading,
            level: parseInt(heading.tagName[1])
        }));
    }

    /**
     * Find the currently visible section based on scroll position
     * @returns {Object|null} Current section object or null
     */
    function findCurrentSection() {
        const scrollTop = window.scrollY || document.documentElement.scrollTop;
        const windowHeight = window.innerHeight;

        // Find the section closest to the top of the viewport
        for (let i = sections.length - 1; i >= 0; i--) {
            const section = sections[i];
            const rect = section.element.getBoundingClientRect();
            const elementTop = scrollTop + rect.top;

            if (elementTop <= scrollTop + SCROLL_OFFSET) {
                return section;
            }
        }

        // Default to first section if at top of page
        if (scrollTop < SCROLL_OFFSET && sections.length > 0) {
            return sections[0];
        }

        return null;
    }

    /**
     * Update sidebar highlighting and commentary
     * @param {Object|null} section - Current section object
     */
    function updateSidebar(section) {
        if (!section) return;

        // Update sidebar links highlighting
        const sidebarLinks = document.querySelectorAll('.sidebar a, .toc a');
        sidebarLinks.forEach(link => {
            const href = link.getAttribute('href');
            if (href && href.startsWith('#')) {
                const linkId = href.substring(1);
                if (linkId === section.id) {
                    link.classList.add('active');
                } else {
                    link.classList.remove('active');
                }
            }
        });

        // Update sidebar status message (if callback is registered)
        if (window.updateSidebarStatus && typeof window.updateSidebarStatus === 'function') {
            window.updateSidebarStatus(section.id, section.element);
        }

        // Dispatch custom event for extensions
        document.dispatchEvent(new CustomEvent('sectionChanged', {
            detail: {
                section: section.id,
                element: section.element,
                level: section.level
            }
        }));
    }

    /**
     * Handle scroll events
     */
    const handleScroll = debounce(() => {
        const newSection = findCurrentSection();

        if (newSection && newSection.id !== currentSection?.id) {
            currentSection = newSection;
            updateSidebar(newSection);
        }
    }, DEBOUNCE_DELAY);

    /**
     * Smooth scroll to section
     * @param {string} sectionId - ID of section to scroll to
     */
    function scrollToSection(sectionId) {
        const element = document.getElementById(sectionId);
        if (element) {
            const top = element.getBoundingClientRect().top + window.scrollY - SCROLL_OFFSET;
            window.scrollTo({
                top: top,
                behavior: 'smooth'
            });
        }
    }

    /**
     * Set up IntersectionObserver for more efficient scroll tracking
     */
    function setupIntersectionObserver() {
        const options = {
            root: null,
            rootMargin: `-${SCROLL_OFFSET}px 0px -70% 0px`,
            threshold: 0
        };

        observer = new IntersectionObserver((entries) => {
            entries.forEach(entry => {
                if (entry.isIntersecting) {
                    const section = sections.find(s => s.element === entry.target);
                    if (section && section.id !== currentSection?.id) {
                        currentSection = section;
                        updateSidebar(section);
                    }
                }
            });
        }, options);

        // Observe all section headings
        sections.forEach(section => {
            observer.observe(section.element);
        });
    }

    /**
     * Attach click handlers to sidebar/TOC links
     */
    function attachLinkHandlers() {
        const links = document.querySelectorAll('.sidebar a[href^="#"], .toc a[href^="#"]');

        links.forEach(link => {
            link.addEventListener('click', (e) => {
                const href = link.getAttribute('href');
                if (href && href.startsWith('#')) {
                    e.preventDefault();
                    const sectionId = href.substring(1);
                    scrollToSection(sectionId);

                    // Update URL hash without triggering scroll
                    if (history.pushState) {
                        history.pushState(null, null, href);
                    } else {
                        window.location.hash = href;
                    }
                }
            });
        });
    }

    /**
     * Initialize sidebar core functionality
     */
    function init() {
        sections = getSections();

        if (sections.length === 0) {
            return; // No sections to track
        }

        // Use IntersectionObserver if available, fallback to scroll events
        if ('IntersectionObserver' in window) {
            setupIntersectionObserver();
        } else {
            window.addEventListener('scroll', handleScroll, { passive: true });
            handleScroll(); // Initial call
        }

        // Attach link handlers
        attachLinkHandlers();

        // Handle initial hash in URL
        if (window.location.hash) {
            const sectionId = window.location.hash.substring(1);
            setTimeout(() => scrollToSection(sectionId), 100);
        } else {
            // Set initial section
            handleScroll();
        }
    }

    /**
     * Cleanup function
     */
    function cleanup() {
        if (observer) {
            observer.disconnect();
        }
        window.removeEventListener('scroll', handleScroll);
    }

    // Export API
    window.sidebarCore = {
        init: init,
        cleanup: cleanup,
        scrollToSection: scrollToSection,
        getCurrentSection: () => currentSection,
        getSections: () => sections
    };

    // Auto-initialize on DOMContentLoaded
    if (document.readyState === 'loading') {
        document.addEventListener('DOMContentLoaded', init);
    } else {
        init();
    }
})();
