// Mnemosyne Whitepaper Website - JavaScript Interactivity

(function() {
    'use strict';

    // ========================================
    // Smooth Scroll with Offset for Fixed Nav
    // ========================================
    function initSmoothScroll() {
        const navLinks = document.querySelectorAll('a[href^="#"]');
        const navHeight = document.querySelector('.navbar').offsetHeight;

        navLinks.forEach(link => {
            link.addEventListener('click', function(e) {
                const href = this.getAttribute('href');

                // Only handle internal links
                if (href === '#' || !href.startsWith('#')) return;

                e.preventDefault();

                const targetId = href.substring(1);
                const targetElement = document.getElementById(targetId);

                if (targetElement) {
                    const targetPosition = targetElement.offsetTop - navHeight - 20;

                    window.scrollTo({
                        top: targetPosition,
                        behavior: 'smooth'
                    });

                    // Update URL without jumping
                    history.pushState(null, null, href);
                }
            });
        });
    }

    // ========================================
    // Progress Bar
    // ========================================
    function initProgressBar() {
        const progressBar = document.getElementById('progress-bar');

        function updateProgressBar() {
            const windowHeight = window.innerHeight;
            const documentHeight = document.documentElement.scrollHeight;
            const scrollTop = window.pageYOffset || document.documentElement.scrollTop;

            const scrollPercentage = (scrollTop / (documentHeight - windowHeight)) * 100;

            progressBar.style.width = scrollPercentage + '%';
        }

        window.addEventListener('scroll', updateProgressBar);
        updateProgressBar(); // Initial call
    }

    // ========================================
    // Active Nav Link Highlighting
    // ========================================
    function initActiveNavHighlight() {
        const sections = document.querySelectorAll('section[id]');
        const navLinks = document.querySelectorAll('.nav-link');

        function highlightNavOnScroll() {
            const scrollPosition = window.pageYOffset;

            sections.forEach(section => {
                const sectionTop = section.offsetTop - 100;
                const sectionHeight = section.offsetHeight;
                const sectionId = section.getAttribute('id');

                if (scrollPosition >= sectionTop && scrollPosition < sectionTop + sectionHeight) {
                    navLinks.forEach(link => {
                        link.classList.remove('active');
                        if (link.getAttribute('href') === '#' + sectionId) {
                            link.classList.add('active');
                        }
                    });
                }
            });
        }

        window.addEventListener('scroll', highlightNavOnScroll);
        highlightNavOnScroll(); // Initial call
    }

    // ========================================
    // Intersection Observer for Fade-in Animations
    // ========================================
    function initScrollAnimations() {
        const animatedElements = document.querySelectorAll('.feature-card, .problem-card, .validation-card');

        const observerOptions = {
            threshold: 0.1,
            rootMargin: '0px 0px -50px 0px'
        };

        const observer = new IntersectionObserver(function(entries) {
            entries.forEach(entry => {
                if (entry.isIntersecting) {
                    entry.target.style.opacity = '1';
                    entry.target.style.transform = 'translateY(0)';
                }
            });
        }, observerOptions);

        animatedElements.forEach(element => {
            element.style.opacity = '0';
            element.style.transform = 'translateY(20px)';
            element.style.transition = 'opacity 0.6s ease, transform 0.6s ease';
            observer.observe(element);
        });
    }

    // ========================================
    // Mobile Navigation Toggle
    // ========================================
    function initMobileNav() {
        const navToggle = document.getElementById('nav-toggle');
        const navLinks = document.querySelector('.nav-links');

        if (navToggle) {
            navToggle.addEventListener('click', function() {
                navLinks.classList.toggle('mobile-active');
                this.classList.toggle('active');
            });

            // Close mobile nav when link is clicked
            const links = navLinks.querySelectorAll('.nav-link');
            links.forEach(link => {
                link.addEventListener('click', function() {
                    navLinks.classList.remove('mobile-active');
                    navToggle.classList.remove('active');
                });
            });
        }
    }

    // ========================================
    // Copy Code Buttons (for code snippets)
    // ========================================
    function initCopyButtons() {
        const codeBlocks = document.querySelectorAll('pre code');

        codeBlocks.forEach(codeBlock => {
            const pre = codeBlock.parentElement;

            // Create copy button
            const copyButton = document.createElement('button');
            copyButton.className = 'copy-button';
            copyButton.textContent = 'Copy';
            copyButton.setAttribute('aria-label', 'Copy code to clipboard');

            // Insert button
            pre.style.position = 'relative';
            pre.appendChild(copyButton);

            // Copy functionality
            copyButton.addEventListener('click', function() {
                const textToCopy = codeBlock.textContent;

                navigator.clipboard.writeText(textToCopy).then(() => {
                    copyButton.textContent = 'Copied!';
                    copyButton.classList.add('copied');

                    setTimeout(() => {
                        copyButton.textContent = 'Copy';
                        copyButton.classList.remove('copied');
                    }, 2000);
                }).catch(err => {
                    console.error('Failed to copy:', err);
                    copyButton.textContent = 'Failed';
                    setTimeout(() => {
                        copyButton.textContent = 'Copy';
                    }, 2000);
                });
            });
        });
    }

    // ========================================
    // Navbar Background on Scroll
    // ========================================
    function initNavbarScroll() {
        const navbar = document.querySelector('.navbar');
        let lastScrollTop = 0;

        window.addEventListener('scroll', function() {
            const scrollTop = window.pageYOffset || document.documentElement.scrollTop;

            // Add shadow when scrolled
            if (scrollTop > 50) {
                navbar.style.boxShadow = '0 2px 4px rgba(0,0,0,0.1)';
            } else {
                navbar.style.boxShadow = 'none';
            }

            lastScrollTop = scrollTop;
        });
    }

    // ========================================
    // External Link Icons
    // ========================================
    function initExternalLinks() {
        const externalLinks = document.querySelectorAll('a[target="_blank"]');

        externalLinks.forEach(link => {
            // Skip if it already has an icon
            if (!link.querySelector('.external-icon')) {
                const icon = document.createElement('span');
                icon.className = 'external-icon';
                icon.setAttribute('aria-hidden', 'true');
                icon.textContent = ' ↗';
                link.appendChild(icon);
            }
        });
    }

    // ========================================
    // Keyboard Navigation
    // ========================================
    function initKeyboardNav() {
        document.addEventListener('keydown', function(e) {
            // Escape key closes mobile nav
            if (e.key === 'Escape') {
                const navLinks = document.querySelector('.nav-links');
                const navToggle = document.getElementById('nav-toggle');
                if (navLinks && navLinks.classList.contains('mobile-active')) {
                    navLinks.classList.remove('mobile-active');
                    if (navToggle) navToggle.classList.remove('active');
                }
            }
        });
    }

    // ========================================
    // Lazy Load Images (if any are added)
    // ========================================
    function initLazyLoad() {
        if ('IntersectionObserver' in window) {
            const lazyImages = document.querySelectorAll('img[data-src]');

            const imageObserver = new IntersectionObserver(function(entries) {
                entries.forEach(entry => {
                    if (entry.isIntersecting) {
                        const img = entry.target;
                        img.src = img.dataset.src;
                        img.removeAttribute('data-src');
                        imageObserver.unobserve(img);
                    }
                });
            });

            lazyImages.forEach(img => imageObserver.observe(img));
        }
    }

    // ========================================
    // Performance: Reduce Motion for Users Who Prefer It
    // ========================================
    function initReducedMotion() {
        const prefersReducedMotion = window.matchMedia('(prefers-reduced-motion: reduce)').matches;

        if (prefersReducedMotion) {
            // Disable smooth scroll
            document.documentElement.style.scrollBehavior = 'auto';

            // Disable animations
            const style = document.createElement('style');
            style.textContent = `
                * {
                    animation-duration: 0.01ms !important;
                    animation-iteration-count: 1 !important;
                    transition-duration: 0.01ms !important;
                }
            `;
            document.head.appendChild(style);
        }
    }

    // ========================================
    // Back to Top Button (optional enhancement)
    // ========================================
    function initBackToTop() {
        // Create back-to-top button
        const backToTopButton = document.createElement('button');
        backToTopButton.className = 'back-to-top';
        backToTopButton.innerHTML = '↑';
        backToTopButton.setAttribute('aria-label', 'Scroll to top');
        document.body.appendChild(backToTopButton);

        // Show/hide button based on scroll
        window.addEventListener('scroll', function() {
            if (window.pageYOffset > 500) {
                backToTopButton.classList.add('visible');
            } else {
                backToTopButton.classList.remove('visible');
            }
        });

        // Scroll to top on click
        backToTopButton.addEventListener('click', function() {
            window.scrollTo({
                top: 0,
                behavior: 'smooth'
            });
        });
    }

    // ========================================
    // Mermaid Diagram Click to Expand (optional)
    // ========================================
    function initDiagramExpand() {
        const diagrams = document.querySelectorAll('.diagram-container');

        diagrams.forEach(container => {
            container.style.cursor = 'pointer';
            container.title = 'Click to expand';

            container.addEventListener('click', function() {
                this.classList.toggle('expanded');
            });
        });
    }

    // ========================================
    // Initialize All Features
    // ========================================
    function init() {
        // Core features
        initSmoothScroll();
        initProgressBar();
        initActiveNavHighlight();
        initScrollAnimations();
        initMobileNav();
        initNavbarScroll();
        initKeyboardNav();

        // Enhancement features
        initCopyButtons();
        initExternalLinks();
        initLazyLoad();
        initReducedMotion();
        initBackToTop();
        initDiagramExpand();

        // Log initialization (for debugging)
        console.log('Mnemosyne website initialized');
    }

    // Initialize when DOM is ready
    if (document.readyState === 'loading') {
        document.addEventListener('DOMContentLoaded', init);
    } else {
        init();
    }

})();

// ========================================
// Additional CSS for JavaScript Features
// ========================================
const additionalStyles = `
    /* Mobile nav active state */
    @media (max-width: 768px) {
        .nav-links.mobile-active {
            display: flex;
            flex-direction: column;
            position: absolute;
            top: 100%;
            left: 0;
            right: 0;
            background-color: white;
            border-top: 1px solid var(--color-border);
            padding: var(--space-4);
            box-shadow: var(--shadow-lg);
        }

        .nav-toggle.active span:nth-child(1) {
            transform: rotate(45deg) translate(5px, 5px);
        }

        .nav-toggle.active span:nth-child(2) {
            opacity: 0;
        }

        .nav-toggle.active span:nth-child(3) {
            transform: rotate(-45deg) translate(7px, -7px);
        }
    }

    /* Active nav link */
    .nav-link.active {
        color: var(--color-primary);
        position: relative;
    }

    .nav-link.active::after {
        content: '';
        position: absolute;
        bottom: -8px;
        left: 0;
        right: 0;
        height: 2px;
        background-color: var(--color-primary);
    }

    /* Copy button */
    .copy-button {
        position: absolute;
        top: 8px;
        right: 8px;
        padding: 4px 12px;
        background-color: var(--color-primary);
        color: white;
        border: none;
        border-radius: 4px;
        font-size: 12px;
        cursor: pointer;
        transition: background-color 0.2s;
    }

    .copy-button:hover {
        background-color: var(--color-primary-dark);
    }

    .copy-button.copied {
        background-color: var(--color-secondary);
    }

    /* Back to top button */
    .back-to-top {
        position: fixed;
        bottom: 32px;
        right: 32px;
        width: 48px;
        height: 48px;
        background-color: var(--color-primary);
        color: white;
        border: none;
        border-radius: 50%;
        font-size: 24px;
        cursor: pointer;
        opacity: 0;
        visibility: hidden;
        transition: opacity 0.3s, visibility 0.3s, background-color 0.2s;
        z-index: 999;
        box-shadow: var(--shadow-lg);
    }

    .back-to-top.visible {
        opacity: 1;
        visibility: visible;
    }

    .back-to-top:hover {
        background-color: var(--color-primary-dark);
    }

    /* External link icon */
    .external-icon {
        font-size: 0.75em;
        opacity: 0.7;
    }

    /* Expanded diagram */
    .diagram-container.expanded {
        position: fixed;
        top: 50%;
        left: 50%;
        transform: translate(-50%, -50%);
        max-width: 90vw;
        max-height: 90vh;
        z-index: 1000;
        overflow: auto;
        box-shadow: var(--shadow-xl);
    }

    .diagram-container.expanded::before {
        content: '';
        position: fixed;
        top: 0;
        left: 0;
        right: 0;
        bottom: 0;
        background-color: rgba(0, 0, 0, 0.7);
        z-index: -1;
    }
`;

// Inject additional styles
const styleEl = document.createElement('style');
styleEl.textContent = additionalStyles;
document.head.appendChild(styleEl);
