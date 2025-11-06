// Clean technical documentation interactions

(function() {
    'use strict';

    // Playful loading messages from mnemosyne
    const LOADING_MESSAGES = [
        "Reticulating splines",
        "Wrangling squirrels",
        "Traversing latent space",
        "Pondering the ineffable",
        "Calibrating flux capacitors",
        "Consulting the oracle",
        "Warming up neurons",
        "Aligning chakras",
        "Defragmenting memories",
        "Untangling quantum states",
        "Initializing agent substrate",
        "Harmonizing vector embeddings",
        "Bootstrapping semantic networks",
        "Activating memory traces",
        "Synchronizing thought streams",
        "Priming knowledge graphs",
        "Energizing cognitive pathways",
        "Indexing conceptual spaces",
        "Weaving context threads",
        "Awakening neural ensembles",
        "Crystallizing insights",
        "Tuning attention mechanisms"
    ];

    // Nerd font glyphs (Font Awesome icons)
    const GLYPHS = [
        "\uf0eb", // lightbulb
        "\uf135", // rocket
        "\uf0e7", // bolt
        "\uf005", // star
        "\uf021", // sync
        "\uf013", // gear
        "\uf5dc", // brain
        "\uf0c1"  // link
    ];

    // Rotate status message and glyph
    function rotateSidebarStatus() {
        const messageEl = document.querySelector('.status-message');
        const glyphEl = document.querySelector('.status-glyph::before');

        if (messageEl) {
            const randomMessage = LOADING_MESSAGES[Math.floor(Math.random() * LOADING_MESSAGES.length)];
            messageEl.textContent = randomMessage;
        }

        if (glyphEl) {
            const randomGlyph = GLYPHS[Math.floor(Math.random() * GLYPHS.length)];
            // Update glyph content via CSS
            const style = document.createElement('style');
            style.textContent = `.status-glyph::before { content: "${randomGlyph}"; }`;
            document.head.appendChild(style);

            // Remove old style after animation
            setTimeout(() => style.remove(), 100);
        }
    }

    // Rotate logo glyph
    function rotateLogoGlyph() {
        const glyphEl = document.querySelector('.logo-glyph');
        if (glyphEl) {
            const randomGlyph = GLYPHS[Math.floor(Math.random() * GLYPHS.length)];
            const style = document.createElement('style');
            style.textContent = `.logo-glyph::before { content: "${randomGlyph}"; }`;
            document.head.appendChild(style);
            setTimeout(() => style.remove(), 100);
        }
    }

    // Smooth scroll for anchor links
    function initSmoothScroll() {
        const links = document.querySelectorAll('a[href^="#"]');

        links.forEach(link => {
            link.addEventListener('click', function(e) {
                const href = this.getAttribute('href');
                if (href === '#') return;

                e.preventDefault();
                const target = document.querySelector(href);

                if (target) {
                    const navHeight = document.querySelector('.navbar').offsetHeight;
                    const targetPosition = target.offsetTop - navHeight - 20;

                    window.scrollTo({
                        top: targetPosition,
                        behavior: 'smooth'
                    });

                    history.pushState(null, null, href);
                }
            });
        });
    }

    // Initialize when DOM is ready
    if (document.readyState === 'loading') {
        document.addEventListener('DOMContentLoaded', function() {
            initSmoothScroll();

            // Set initial glyphs
            rotateLogoGlyph();
            rotateSidebarStatus();

            // Rotate sidebar every 5 seconds
            setInterval(rotateSidebarStatus, 5000);

            // Rotate logo glyph every 8 seconds
            setInterval(rotateLogoGlyph, 8000);
        });
    } else {
        initSmoothScroll();
        rotateLogoGlyph();
        rotateSidebarStatus();
        setInterval(rotateSidebarStatus, 5000);
        setInterval(rotateLogoGlyph, 8000);
    }

})();
