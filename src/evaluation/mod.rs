//! Evaluation system for adaptive context relevance learning.
//!
//! This module implements an online learning system that tracks how useful
//! provided context (skills, memories, files) is and adapts relevance scoring
//! over time at session, project, and global levels.
//!
//! # Architecture
//!
//! - **FeedbackCollector**: Records implicit feedback signals (access, edits, commits)
//! - **FeatureExtractor**: Extracts privacy-preserving statistical features
//! - **RelevanceScorer**: Online learning algorithm with hierarchical weights
//!
//! # Privacy-First Design
//!
//! The evaluation system is designed with **privacy as a fundamental constraint**:
//!
//! ## Privacy Guarantees
//!
//! - ✅ **Local-Only Storage**: All data in `.mnemosyne/project.db` (gitignored)
//! - ✅ **Hashed Tasks**: SHA256 hash of task descriptions (max 16 chars stored)
//! - ✅ **Limited Keywords**: Max 10 generic keywords, no sensitive terms
//! - ✅ **Statistical Features**: Only computed metrics, never raw content
//! - ✅ **No Network Calls**: Uses existing Anthropic API calls, no separate requests
//! - ✅ **Graceful Degradation**: System works perfectly when disabled
//!
//! ## What IS Stored
//!
//! ```rust,ignore
//! // Evaluation record
//! ContextEvaluation {
//!     task_hash: "a3f8e9d1...",  // SHA256, 16 chars
//!     task_keywords: Some(vec!["rust", "async"]), // Max 10, generic
//!     was_accessed: true,
//!     access_count: 3,
//!     was_edited: false,
//! }
//!
//! // Statistical features
//! RelevanceFeatures {
//!     keyword_overlap_score: 0.75,  // Jaccard similarity
//!     recency_days: 7.2,
//!     access_frequency: 0.5,
//!     was_useful: true,
//! }
//! ```
//!
//! ## What IS NOT Stored
//!
//! - ✗ Raw task descriptions
//! - ✗ Full file contents
//! - ✗ Actual code snippets
//! - ✗ Sensitive variable names
//! - ✗ API keys or secrets
//! - ✗ Personal information
//!
//! **Privacy guarantee**: Only statistical features are persisted. No content
//! reconstruction possible.
//!
//! For complete privacy documentation, see:
//! - **Privacy Policy**: `docs/features/PRIVACY.md`
//! - **Technical Details**: `EVALUATION.md`
//!
//! # Hierarchical Learning
//!
//! Weights are learned at three levels with different learning rates:
//! - **Session**: Fast adaptation (α=0.3) for immediate context
//! - **Project**: Moderate adaptation (α=0.1) for project patterns
//! - **Global**: Slow adaptation (α=0.03) for universal patterns
//!
//! Weight lookup follows fallback hierarchy:
//! 1. Most specific (work_phase + task_type + error_context)
//! 2. Partial match (work_phase + task_type)
//! 3. Phase-only (work_phase)
//! 4. Generic (no constraints)
//!
//! # Usage
//!
//! ```rust,no_run
//! use mnemosyne_core::evaluation::{ContextType, FeedbackCollector, ProvidedContext};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let collector = FeedbackCollector::new(".mnemosyne/project.db".to_string());
//!
//! // Record context provided
//! let eval_id = collector.record_context_provided(ProvidedContext {
//!     session_id: "session-123".to_string(),
//!     agent_role: "optimizer".to_string(),
//!     namespace: "project:mnemosyne".to_string(),
//!     context_type: ContextType::Skill,
//!     context_id: "rust-async.md".to_string(),
//!     task_hash: "a3f8e9d1".to_string(),  // SHA256, 16 chars
//!     task_keywords: Some(vec!["rust".to_string(), "async".to_string()]),
//!     task_type: None,
//!     work_phase: None,
//!     file_types: None,
//!     error_context: None,
//!     related_technologies: None,
//! }).await?;
//!
//! // Record feedback signals
//! collector.record_context_accessed(&eval_id).await?;
//! collector.record_context_edited(&eval_id).await?;
//! collector.record_context_committed(&eval_id).await?;
//! # Ok(())
//! # }
//! ```

pub mod feature_extractor;
pub mod feedback_collector;
pub mod relevance_scorer;
pub mod schema;

pub use feature_extractor::{FeatureExtractor, RelevanceFeatures};
pub use feedback_collector::{ContextEvaluation, ContextType, FeedbackCollector, ProvidedContext};
pub use relevance_scorer::{RelevanceScorer, Scope, WeightSet};
pub use schema::init_evaluation_tables;
