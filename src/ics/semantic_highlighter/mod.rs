//! Advanced Semantic Highlighting for Agentic Context
//!
//! Multi-tier semantic analysis system with three performance tiers:
//! - **Tier 1 (Structural)**: Real-time pattern matching (<5ms)
//! - **Tier 2 (Relational)**: Incremental NLP analysis (<200ms)
//! - **Tier 3 (Analytical)**: Optional Claude API deep analysis (2s+, cached)
//!
//! ## Features
//!
//! ### Tier 1: Structural Layer (Local, Real-time)
//! - XML tag recognition and validation
//! - Constraint annotations (RFC 2119)
//! - Modality and epistemic markers
//! - Ambiguity detection
//! - Enhanced domain patterns (#file, @symbol, ?hole)
//!
//! ### Tier 2: Relational Layer (Local, Incremental)
//! - Entity recognition and tracking
//! - Coreference resolution
//! - Relationship extraction (SVO triples)
//! - Semantic role labeling
//! - Anaphora resolution
//!
//! ### Tier 3: Analytical Layer (Claude API, Optional)
//! - Discourse coherence analysis
//! - Contradiction detection
//! - Presupposition extraction
//! - Cross-reference validation
//!
//! ## Architecture
//!
//! ```text
//! ┌───────────────────────────────────────────┐
//! │  SemanticHighlightEngine                  │
//! │  - Coordinates all three tiers            │
//! │  - Manages caching and incremental updates│
//! └─────────────────┬─────────────────────────┘
//!                   │
//!    ┌──────────────┼──────────────┐
//!    │              │              │
//! ┌──▼───┐      ┌──▼───┐      ┌──▼───┐
//! │Tier 1│      │Tier 2│      │Tier 3│
//! │Local │      │Local │      │Claude│
//! │<5ms  │      │<200ms│      │ 2s+  │
//! └──────┘      └──────┘      └──────┘
//! ```
//!
//! ## Usage
//!
//! ```rust,no_run
//! use mnemosyne_core::ics::semantic_highlighter::SemanticHighlightEngine;
//!
//! // Create engine (Tier 3 optional - requires API key)
//! let mut engine = SemanticHighlightEngine::new(None);
//!
//! // Highlight a line
//! let line = engine.highlight_line("See #src/main.rs for the implementation");
//!
//! // Enable Tier 3 with LLM service
//! let engine_with_llm = SemanticHighlightEngine::new(Some(llm_service));
//! ```

pub mod engine;
pub mod settings;
pub mod cache;

pub mod tier1_structural;
pub mod tier2_relational;
pub mod tier3_analytical;
pub mod visualization;
pub mod utils;

// Re-exports
pub use engine::SemanticHighlightEngine;
pub use settings::{HighlightSettings, AnalyticalSettings};
pub use cache::SemanticCache;

// Core types
pub use visualization::{HighlightSpan, HighlightSource, AnnotationType};

/// Result type for semantic highlighting operations
pub type Result<T> = std::result::Result<T, SemanticError>;

/// Errors that can occur during semantic analysis
#[derive(Debug, thiserror::Error)]
pub enum SemanticError {
    #[error("Analysis failed: {0}")]
    AnalysisFailed(String),

    #[error("Cache error: {0}")]
    CacheError(String),

    #[error("LLM service error: {0}")]
    LlmError(String),

    #[error("Invalid input: {0}")]
    InvalidInput(String),
}
