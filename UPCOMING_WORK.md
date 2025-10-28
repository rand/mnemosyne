# Upcoming Work for Mnemosyne

**Last Updated**: October 27, 2025
**Current Version**: v2.0.0
**Status**: Planning future enhancements

---

## v2.1 - Performance & UX Enhancements (Q4 2025)

### High Priority
- [ ] **Batch Vector Operations**: Optimize embedding generation for bulk imports
- [ ] **Vector Index Tuning**: Benchmark and optimize sqlite-vec search performance
- [ ] **Evolution Job Scheduler**: Implement cron-like job scheduling with configurable intervals
- [ ] **Memory Consolidation UI**: Interactive tool for reviewing and merging duplicate memories
- [ ] **Search Result Ranking**: Fine-tune hybrid search weights based on user feedback

### Medium Priority
- [ ] **Export Enhancements**: Add JSON, CSV export formats (currently Markdown only)
- [ ] **Import Tool**: Bulk import from Markdown/JSON with auto-enrichment
- [ ] **Graph Visualization**: Web UI for exploring memory relationships
- [ ] **Performance Benchmarks**: Formal benchmark suite for 10k, 100k, 1M memories
- [ ] **Metrics Dashboard**: Track memory stats, evolution jobs, search performance

### Low Priority
- [ ] **Multi-User Support**: User isolation for shared deployments
- [ ] **Alternative LLM Providers**: Support for OpenAI, Gemini, local models
- [ ] **Mobile Companion**: iOS/Android app for quick memory capture
- [ ] **Browser Extension**: Capture web research directly to Mnemosyne

---

## v3.0 - Advanced Intelligence (Q1 2026)

### Research & Development
- [ ] **Semantic Clustering**: Auto-detect memory clusters and relationships
- [ ] **Temporal Reasoning**: Time-aware memory retrieval and pattern detection
- [ ] **Cross-Project Learning**: Global knowledge graph across all projects
- [ ] **Active Learning**: Memory system suggests what to remember/consolidate
- [ ] **Federated Memory**: Sync memories across multiple machines/teams

### Infrastructure
- [ ] **Cloud Backend**: Turso integration for remote storage
- [ ] **Collaboration**: Multi-agent memory sharing and conflict resolution
- [ ] **Versioning**: Git-like memory versioning with diff/merge capabilities
- [ ] **Backup/Restore**: Automated backup strategies with point-in-time recovery

---

## Technical Debt & Maintenance

### Code Quality
- [ ] **Reduce Warnings**: Fix 23 unused import/variable warnings in lib and tests
- [ ] **Error Handling**: Standardize error types across modules
- [ ] **Documentation**: Add rustdoc comments to all public APIs
- [ ] **Integration Tests**: Add more end-to-end workflow tests
- [ ] **Performance Tests**: Enable and tune `test_search_performance_10k_vectors`

### Dependencies
- [ ] **Dependency Audit**: Review and update all Cargo dependencies
- [ ] **Security Scan**: Run cargo-audit regularly
- [ ] **Version Pinning**: Lock critical dependencies for reproducibility

### Documentation
- [ ] **API Documentation**: Generate and publish rustdoc to docs.rs
- [ ] **Architecture Diagrams**: Update with v2.0 implementation details
- [ ] **Deployment Guide**: Production deployment best practices
- [ ] **Troubleshooting Guide**: Common issues and solutions
- [ ] **Contributing Guide**: Onboarding for external contributors

---

## Community & Ecosystem

### Community Building
- [ ] **GitHub Discussions**: Set up for Q&A and feature requests
- [ ] **Blog Posts**: Technical deep-dives on architecture decisions
- [ ] **Video Tutorials**: YouTube series for common workflows
- [ ] **Example Projects**: Showcase repositories demonstrating Mnemosyne usage

### Integrations
- [ ] **VS Code Extension**: Native VS Code integration (alternative to Claude Code)
- [ ] **Neovim Plugin**: Lua plugin for Neovim integration
- [ ] **Obsidian Plugin**: Sync with Obsidian knowledge base
- [ ] **Raycast Extension**: Quick memory access from macOS Raycast

---

## Research Ideas (Exploratory)

- **Differential Privacy**: Memory encryption with privacy guarantees
- **Memory Compression**: LLM-based memory summarization for long-term storage
- **Prompt Engineering**: Auto-optimize prompts based on memory retrieval patterns
- **Context Windows**: Smart context window management for ultra-long sessions
- **Memory Pruning**: ML-based prediction of obsolete memories
- **Emotional Context**: Capture developer sentiment (frustration, breakthroughs)
- **Code Co-evolution**: Track how memories relate to code changes over time

---

## How to Contribute

See individual issues in the GitHub tracker for detailed specifications and implementation plans. Each item above will be tracked as a separate issue with:

- Detailed requirements
- Acceptance criteria
- Test plan
- Breaking change analysis
- Migration guide (if applicable)

**Want to work on something?** Check the GitHub issues labeled `help-wanted` or `good-first-issue`.

---

## Prioritization Framework

**Priorities are determined by**:
1. **User Impact**: Does this solve a painful problem?
2. **Technical Risk**: How complex/risky is the implementation?
3. **Dependencies**: What must be done first?
4. **Effort**: How much work is required?

**Decision Matrix**:
- High Impact + Low Risk + Low Effort = **Do First**
- High Impact + High Risk = **Research & Prototype**
- Low Impact + High Effort = **Defer or Cut**

---

## Version History

- **v2.0.0** (Oct 2025): Vector search, RBAC, evolution system
- **v1.0.0** (Oct 2025): Initial release with MCP integration
