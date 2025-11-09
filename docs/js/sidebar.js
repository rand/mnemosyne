// mnemosyne-specific sidebar comments
// Load this BEFORE sidebar-base.js

(function() {
    // Set theme key for mnemosyne
    window.THEME_KEY = 'mnemosyne-theme';

    // Detect which page we're on
    const path = window.location.pathname;
    const isWhitepaper = path.includes('whitepaper');

    if (isWhitepaper) {
        // Whitepaper page comments
        window.SIDEBAR_COMMENTS = {
            'abstract': '// Semantic memory: because context windows are finite',
            'table-of-contents': '// Your map through the semantic memory rabbit hole',
            '1-executive-summary': '// TL;DR: Sub-millisecond memory for LLM agents',
            '2-introduction': '// Context windows: 32K tokens, 10-15K useful reality',
            '3-the-challenge-context-loss-in-llm-systems': '// 5-13 minutes wasted per session on context',
            '4-mnemosyne-architecture': '// LibSQL + Hybrid Search + 4-Agent Framework',
            '5-workflows--integration': '// CLI + MCP + Automatic Hooks = Zero Config',
            '6-qualitative-comparison': '// MemGPT vs Mem0 vs LangChain: Fight!',
            '7-validation--evidence': '// 715 tests • 100% pass rate • v2.1.2 tagged',
            '8-conclusion': '// Context loss elimination via persistent memory',
            '9-references': '// Standing on the shoulders of giants'
        };

        window.SIDEBAR_SUBSECTIONS = {
            // Executive Summary
            '1.1 The Problem': '// Context evaporates, agents forget, coordination fails',
            '1.2 The Solution': '// Hybrid search + 4 agents + LLM evolution',
            '1.3 Key Capabilities': '// 0.88ms list • 1.61ms search • sub-millisecond',
            '1.4 Target Use Cases': '// Persistent context + Multi-agent + Autonomous',

            // Introduction
            '2.1 The Context Window Challenge': '// 32K tokens → 10-15K effective working memory',
            '2.2 Current Landscape': '// MemGPT (dead), Mem0 (graphs), LangChain (buffers)',
            '2.3 Mnemosyne\'s Position': '// Memory + Agents = Inseparable concerns',

            // The Challenge
            '3.1 Context Window Mathematics': '// 3K system + 10K history = 10-15K usable',
            '3.2 The Re-initialization Tax': '// 200-520 minutes wasted per 2-week sprint',

            // Architecture
            '4.1 Core Memory System': '// LibSQL: SQLite-compatible with native vectors',
            '4.1.1 Memory Model': '// 20+ fields: Identity, Content, Classification, Links',
            '4.1.2 Hybrid Search': '// FTS5 (20%) + Graph (10%) + Vectors (70% planned)',
            '4.2 Multi-Agent Orchestration': '// Ractor supervision for deadlock-free coordination',
            '4.2.1 Four-Agent Framework': '// Orchestrator, Optimizer, Reviewer, Executor',
            '4.3 Evolution System': '// LLM-guided consolidation, decay, archival',
            '4.3.1 Consolidation': '// Merge, Supersede, or KeepBoth via Claude Haiku',
            '4.3.2 Importance Recalibration': '// Recency decay + access boost + graph proximity',

            // Workflows & Integration
            '5.1 Developer Workflows': '// CLI remember/recall for explicit control',
            '5.1.1 Memory Capture': '// mnemosyne remember -i 9 -t "tags"',
            '5.1.2 Memory Recall': '// Hybrid search across keyword + graph space',
            '5.2 Claude Code Integration': '// MCP + Hooks + Real-time monitoring',
            '5.2.1 MCP Protocol Tools': '// 8 OODA-aligned tools for Claude Code',
            '5.2.2 Automatic Hooks': '// session-start, post-tool-use, pre-destructive',

            // Validation
            '7.1 Test Coverage': '// 715 tests: Unit, Integration, E2E, Specialized',
            '7.2 Performance Metrics': '// 0.88ms list • 1.61ms hybrid search • 2.25ms store',

            // Conclusion
            '8.1 Summary of Contributions': '// Persistent + Coordination + Evolution + Integration',
            '8.2 Impact on LLM Agent Systems': '// Context loss eliminated, coordination enabled'
        };

        window.SIDEBAR_DEFAULT = '// Semantic memory for LLM systems';
    } else {
        // Index page comments
        window.SIDEBAR_COMMENTS = {
            'abstract': '// Because context windows are a leaky abstraction',
            'the-challenge': '// The re-initialization tax: 3-9 hours per feature',
            'architecture': '// Hybrid search: FTS5 + graphs + vectors',
            'comparison-with-existing-systems': '// MemGPT is dead. Long live semantic memory.',
            'validation-evidence': '// 934 tests • 65% coverage • v2.2.0 tagged',
            'summary': '// Persistent memory: solved ✓'
        };

        window.SIDEBAR_SUBSECTIONS = {
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

        window.SIDEBAR_DEFAULT = '// Semantic memory for LLM systems';
    }
})();
