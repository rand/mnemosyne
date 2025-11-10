// TOC Core - Auto-generate Table of Contents from Headings
// Creates navigation from h2/h3 headings with scroll tracking

(function() {
    'use strict';

    // Configuration
    const DEFAULT_CONTAINER_SELECTOR = '.toc-container';
    const HEADING_SELECTOR = 'h2[id], h3[id]';
    const MAX_DEPTH = 3; // h2 and h3 only

    /**
     * Generate slug from heading text if id is missing
     * @param {string} text - Heading text
     * @returns {string} Slug
     */
    function generateSlug(text) {
        return text
            .toLowerCase()
            .replace(/[^\w\s-]/g, '')
            .replace(/\s+/g, '-')
            .replace(/-+/g, '-')
            .trim();
    }

    /**
     * Get heading level from tag name
     * @param {HTMLElement} heading - Heading element
     * @returns {number} Level (2 or 3)
     */
    function getHeadingLevel(heading) {
        return parseInt(heading.tagName[1]);
    }

    /**
     * Extract headings from content
     * @param {HTMLElement|null} container - Container to search within (null = entire document)
     * @returns {Array} Array of heading objects
     */
    function extractHeadings(container = null) {
        const root = container || document.body;
        const headingElements = root.querySelectorAll(HEADING_SELECTOR);

        return Array.from(headingElements).map((heading, index) => {
            const level = getHeadingLevel(heading);
            const text = heading.textContent.trim();
            let id = heading.id;

            // Generate id if missing
            if (!id) {
                id = generateSlug(text);
                heading.id = id;
            }

            return {
                id,
                text,
                level,
                element: heading,
                index
            };
        });
    }

    /**
     * Build nested TOC structure
     * @param {Array} headings - Flat array of headings
     * @returns {Array} Nested array of TOC items
     */
    function buildTOCStructure(headings) {
        const toc = [];
        const stack = [{ level: 1, children: toc }];

        headings.forEach(heading => {
            const item = {
                id: heading.id,
                text: heading.text,
                level: heading.level,
                children: []
            };

            // Find parent level
            while (stack.length > 1 && stack[stack.length - 1].level >= heading.level) {
                stack.pop();
            }

            // Add to parent's children
            stack[stack.length - 1].children.push(item);

            // Push to stack for potential children
            stack.push(item);
        });

        return toc;
    }

    /**
     * Render TOC HTML from structure
     * @param {Array} items - TOC items
     * @param {number} depth - Current depth (for nesting)
     * @returns {string} HTML string
     */
    function renderTOC(items, depth = 0) {
        if (items.length === 0) return '';

        const listClass = depth === 0 ? 'toc-list' : 'toc-sublist';
        let html = `<ul class="${listClass}">`;

        items.forEach(item => {
            const itemClass = `toc-item toc-level-${item.level}`;
            html += `<li class="${itemClass}">`;
            html += `<a href="#${item.id}" class="toc-link">${item.text}</a>`;

            if (item.children.length > 0) {
                html += renderTOC(item.children, depth + 1);
            }

            html += '</li>';
        });

        html += '</ul>';
        return html;
    }

    /**
     * Generate and insert TOC
     * @param {Object} options - Configuration options
     * @returns {boolean} Success status
     */
    function generateTOC(options = {}) {
        const {
            containerSelector = DEFAULT_CONTAINER_SELECTOR,
            contentSelector = null,
            title = 'Table of Contents',
            includeTitle = true,
            collapsible = false
        } = options;

        // Find TOC container
        const tocContainer = document.querySelector(containerSelector);
        if (!tocContainer) {
            console.warn(`TOC container not found: ${containerSelector}`);
            return false;
        }

        // Find content container
        const contentContainer = contentSelector
            ? document.querySelector(contentSelector)
            : null;

        // Extract headings
        const headings = extractHeadings(contentContainer);
        if (headings.length === 0) {
            console.warn('No headings found for TOC generation');
            tocContainer.innerHTML = '<p class="toc-empty">No sections found</p>';
            return false;
        }

        // Build TOC structure
        const tocStructure = buildTOCStructure(headings);

        // Render HTML
        let html = '<nav class="toc" aria-label="Table of Contents">';

        if (includeTitle) {
            html += `<h2 class="toc-title">${title}</h2>`;
        }

        if (collapsible) {
            html += '<button class="toc-toggle" aria-label="Toggle table of contents">▼</button>';
        }

        html += renderTOC(tocStructure);
        html += '</nav>';

        // Insert into container
        tocContainer.innerHTML = html;

        // Attach collapse handler if needed
        if (collapsible) {
            attachCollapseHandler(tocContainer);
        }

        return true;
    }

    /**
     * Attach collapse/expand handler to TOC
     * @param {HTMLElement} container - TOC container
     */
    function attachCollapseHandler(container) {
        const toggleButton = container.querySelector('.toc-toggle');
        const tocList = container.querySelector('.toc-list');

        if (!toggleButton || !tocList) return;

        toggleButton.addEventListener('click', () => {
            const isCollapsed = tocList.classList.toggle('collapsed');
            toggleButton.textContent = isCollapsed ? '▶' : '▼';
            toggleButton.setAttribute('aria-expanded', !isCollapsed);
        });
    }

    /**
     * Update TOC active state based on current section
     * @param {string} sectionId - Current section ID
     */
    function updateActiveState(sectionId) {
        const tocLinks = document.querySelectorAll('.toc-link');

        tocLinks.forEach(link => {
            const href = link.getAttribute('href');
            if (href === `#${sectionId}`) {
                link.classList.add('active');
            } else {
                link.classList.remove('active');
            }
        });
    }

    /**
     * Initialize TOC with default options
     */
    function init(options = {}) {
        const success = generateTOC(options);

        if (success) {
            // Listen for section changes from sidebar-core
            document.addEventListener('sectionChanged', (e) => {
                updateActiveState(e.detail.section);
            });
        }

        return success;
    }

    // Export API
    window.tocCore = {
        init: init,
        generate: generateTOC,
        updateActive: updateActiveState,
        extractHeadings: extractHeadings
    };

    // Auto-initialize if .toc-container exists
    if (document.readyState === 'loading') {
        document.addEventListener('DOMContentLoaded', () => {
            if (document.querySelector(DEFAULT_CONTAINER_SELECTOR)) {
                init();
            }
        });
    } else {
        if (document.querySelector(DEFAULT_CONTAINER_SELECTOR)) {
            init();
        }
    }
})();
