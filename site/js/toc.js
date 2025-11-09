// Dynamic table of contents with scroll tracking
(function() {
    'use strict';

    // Generate TOC from page headings
    function generateTOC() {
        const main = document.querySelector('main');
        const headings = main.querySelectorAll('h2, h3');

        if (headings.length === 0) return;

        const tocContainer = document.querySelector('.sidebar-toc');
        if (!tocContainer) return;

        const tocList = document.createElement('ul');
        tocList.className = 'toc-list';

        headings.forEach(heading => {
            const id = heading.id || heading.textContent.toLowerCase().replace(/\s+/g, '-').replace(/[^\w-]/g, '');
            if (!heading.id) heading.id = id;

            const li = document.createElement('li');
            li.className = `toc-item toc-${heading.tagName.toLowerCase()}`;

            const a = document.createElement('a');
            a.href = `#${id}`;
            a.textContent = heading.textContent;
            a.className = 'toc-link';

            li.appendChild(a);
            tocList.appendChild(li);
        });

        tocContainer.appendChild(tocList);
    }

    // Track scroll position and highlight current section
    function trackScroll() {
        const headings = document.querySelectorAll('main h2, main h3');
        const tocLinks = document.querySelectorAll('.toc-link');

        if (headings.length === 0 || tocLinks.length === 0) return;

        let current = '';

        headings.forEach(heading => {
            const rect = heading.getBoundingClientRect();
            if (rect.top <= 100) {
                current = heading.id;
            }
        });

        tocLinks.forEach(link => {
            link.classList.remove('active');
            if (link.getAttribute('href') === `#${current}`) {
                link.classList.add('active');
            }
        });
    }

    // Initialize
    if (document.readyState === 'loading') {
        document.addEventListener('DOMContentLoaded', () => {
            generateTOC();
            trackScroll();
        });
    } else {
        generateTOC();
        trackScroll();
    }

    // Track scroll
    window.addEventListener('scroll', trackScroll, { passive: true });

    // Smooth scroll for TOC links
    document.addEventListener('click', (e) => {
        if (e.target.classList.contains('toc-link')) {
            e.preventDefault();
            const target = document.querySelector(e.target.getAttribute('href'));
            if (target) {
                target.scrollIntoView({ behavior: 'smooth', block: 'start' });
            }
        }
    });
})();
