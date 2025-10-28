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
//! # Privacy Design
//!
//! - No raw content stored (hash-only, max 16 chars)
//! - Only statistical features (keyword overlap scores, not keywords)
//! - All data local in `.mnemosyne/project.db`
//! - No network calls, no telemetry
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

pub mod feedback_collector;
pub mod feature_extractor;
pub mod relevance_scorer;

pub use feedback_collector::{FeedbackCollector, ProvidedContext, ContextEvaluation};
pub use feature_extractor::{FeatureExtractor, RelevanceFeatures};
pub use relevance_scorer::{RelevanceScorer, Scope, WeightSet};
