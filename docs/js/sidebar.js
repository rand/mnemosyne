// Dynamic sidebar content based on scroll position
(function() {
    // Section-specific comments for mnemosyne (mix of technical + dry wit)
    const sectionComments = {
        'abstract': '// Because context windows are a leaky abstraction',
        'challenge': '// The re-initialization tax: 3-9 hours per feature',
        'architecture': '// Hybrid search: FTS5 + graphs + vectors',
        'comparison': '// MemGPT is dead. Long live semantic memory.',
        'validation': '// 934 tests • 65% coverage • v2.2.0 tagged',
        'conclusion': '// Persistent memory: solved ✓'
    };

    // Subsection commentary (detected via nearest h3)
    const subsectionComments = {
        'Context Window Mathematics': '// 32K tokens → 10-15K effective memory',
        'The Re-initialization Tax': '// $330-$870 wasted per feature sprint',
        'Multi-Agent Coordination Failures': '// Race conditions: the silent killer',
        'Core Memory System': '// 0.88ms list • 1.61ms search • sub-millisecond',
        'Hybrid Search Architecture': '// 70% vectors + 20% keywords + 10% graph',
        'Four-Agent Framework': '// Orchestrator → Optimizer → Reviewer → Executor',
        'Autonomous Evolution': '// LLM-guided memory consolidation & decay',
        'gRPC Remote Access': '// Remote memory: because localhost is so 2020',
        'Integrated Context Studio': '// VSCode extension for memory visualization',
        'Test Coverage': '// 934 tests across 12 test suites',
        'Performance Metrics': '// LibSQL: fast enough to make Redis jealous',
        'Production Readiness': '// Real-world integration with Claude Code',
        'Quality Gates': '// Clippy + integration tests + property tests',
        'Resources': '// Source code, docs, and the rabbit hole awaits'
    };

    function updateSidebarContent() {
        const sidebar = document.querySelector('.sidebar-tagline');
        if (!sidebar) return;

        // Get all sections and headings
        const sections = [...document.querySelectorAll('section[id]')];
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
            sidebar.textContent = '// Semantic memory for LLM systems';
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
