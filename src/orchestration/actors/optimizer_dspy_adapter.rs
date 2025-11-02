//! OptimizerDSpyAdapter - Type-safe wrapper for Optimizer DSPy operations
//!
//! Provides strongly-typed Rust interface to OptimizerModule Python DSPy signatures:
//! - Context consolidation (progressive summarization)
//! - Skills discovery (intelligent skill matching)
//! - Context budget optimization (resource allocation)
//!
//! All operations use async spawn_blocking for non-blocking Python GIL access.

use crate::error::{MnemosyneError, Result};
use crate::orchestration::dspy_bridge::DSpyBridge;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;

#[cfg(feature = "python")]
use pyo3::Python;

/// Type-safe adapter for Optimizer DSPy operations
pub struct OptimizerDSpyAdapter {
    bridge: Arc<DSpyBridge>,
}

impl OptimizerDSpyAdapter {
    /// Create new optimizer adapter wrapping generic DSPy bridge
    pub fn new(bridge: Arc<DSpyBridge>) -> Self {
        Self { bridge }
    }

    /// Consolidate work item context using DSPy
    ///
    /// Applies progressive consolidation strategy based on review attempt:
    /// - Attempt 1: Detailed feedback (preserve all context)
    /// - Attempts 2-3: Structured summary (key issues + patterns)
    /// - Attempt 4+: Compressed essentials (critical blockers only)
    ///
    /// # Arguments
    /// * `original_intent` - User's original work intent
    /// * `execution_summaries` - List of execution memory summaries
    /// * `review_feedback` - List of quality gate issues from review
    /// * `suggested_tests` - List of test improvements suggested
    /// * `review_attempt` - Review attempt number (1=first, 2=second, etc.)
    /// * `consolidation_mode` - Mode: "detailed"|"summary"|"compressed"
    ///
    /// # Returns
    /// ConsolidatedContext with:
    /// - consolidated_content: Markdown-formatted consolidated context
    /// - key_issues: List of critical issues to address
    /// - strategic_guidance: Recommendations for systematic fixes
    /// - estimated_tokens: Token count estimate
    #[cfg(feature = "python")]
    pub async fn consolidate_context(
        &self,
        original_intent: &str,
        execution_summaries: Vec<String>,
        review_feedback: Vec<String>,
        suggested_tests: Vec<String>,
        review_attempt: u32,
        consolidation_mode: &str,
    ) -> Result<ConsolidatedContext> {
        let mut inputs = HashMap::new();
        inputs.insert("original_intent".to_string(), json!(original_intent));
        inputs.insert("execution_summaries".to_string(), json!(execution_summaries));
        inputs.insert("review_feedback".to_string(), json!(review_feedback));
        inputs.insert("suggested_tests".to_string(), json!(suggested_tests));
        inputs.insert("review_attempt".to_string(), json!(review_attempt));
        inputs.insert(
            "consolidation_mode".to_string(),
            json!(consolidation_mode),
        );

        let outputs = self
            .bridge
            .call_agent_module("optimizer", inputs)
            .await?;

        // Parse outputs to ConsolidatedContext
        let consolidated_content: String = outputs
            .get("consolidated_content")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                MnemosyneError::Other("Missing consolidated_content in response".to_string())
            })?
            .to_string();

        let key_issues: Vec<String> = outputs
            .get("key_issues")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();

        let strategic_guidance: String = outputs
            .get("strategic_guidance")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                MnemosyneError::Other("Missing strategic_guidance in response".to_string())
            })?
            .to_string();

        let estimated_tokens: usize = outputs
            .get("estimated_tokens")
            .and_then(|v| v.as_str())
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);

        Ok(ConsolidatedContext {
            consolidated_content,
            key_issues,
            strategic_guidance,
            estimated_tokens,
        })
    }

    /// Discover relevant skills for a task using DSPy
    ///
    /// Analyzes task semantically to find best skill matches beyond simple
    /// keyword matching. Considers current context budget to recommend
    /// optimal skill count.
    ///
    /// # Arguments
    /// * `task_description` - Description of task to perform
    /// * `available_skills` - List of skill metadata (name, description, keywords)
    /// * `max_skills` - Maximum number of skills to return
    /// * `current_context_usage` - Current context usage (0.0-1.0)
    ///
    /// # Returns
    /// SkillDiscoveryResult with:
    /// - selected_skills: List of skill names to load
    /// - relevance_scores: Scores for each selected skill
    /// - reasoning: Explanation of selections
    #[cfg(feature = "python")]
    pub async fn discover_skills(
        &self,
        task_description: &str,
        available_skills: Vec<SkillMetadata>,
        max_skills: usize,
        current_context_usage: f32,
    ) -> Result<SkillDiscoveryResult> {
        // Convert SkillMetadata to JSON
        let skills_json: Vec<Value> = available_skills
            .iter()
            .map(|s| {
                json!({
                    "name": s.name,
                    "description": s.description,
                    "keywords": s.keywords,
                    "domains": s.domains,
                })
            })
            .collect();

        let mut inputs = HashMap::new();
        inputs.insert("task_description".to_string(), json!(task_description));
        inputs.insert("available_skills".to_string(), json!(skills_json));
        inputs.insert("max_skills".to_string(), json!(max_skills));
        inputs.insert(
            "current_context_usage".to_string(),
            json!(current_context_usage),
        );

        let outputs = self
            .bridge
            .call_agent_module("optimizer", inputs)
            .await?;

        // Parse outputs
        let selected_skills: Vec<String> = outputs
            .get("selected_skills")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();

        let relevance_scores: Vec<f32> = outputs
            .get("relevance_scores")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_f64().map(|f| f as f32))
                    .collect()
            })
            .unwrap_or_default();

        let reasoning: String = outputs
            .get("reasoning")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        Ok(SkillDiscoveryResult {
            selected_skills,
            relevance_scores,
            reasoning,
        })
    }

    /// Optimize context allocation to reach target percentage
    ///
    /// Analyzes current context usage and makes intelligent decisions about
    /// what to unload to reach target percentage while preserving critical
    /// resources for current work item.
    ///
    /// # Arguments
    /// * `current_usage` - Current usage breakdown by category
    /// * `loaded_resources` - Currently loaded resources
    /// * `target_pct` - Target context usage (0.0-1.0)
    /// * `work_priority` - Work item priority (0-10)
    ///
    /// # Returns
    /// OptimizationPlan with:
    /// - unload_skills: Skills to unload
    /// - unload_memory_ids: Memory IDs to unload
    /// - optimization_rationale: Explanation of decisions
    #[cfg(feature = "python")]
    pub async fn optimize_context_budget(
        &self,
        current_usage: ContextUsage,
        loaded_resources: LoadedResources,
        target_pct: f32,
        work_priority: u8,
    ) -> Result<OptimizationPlan> {
        // Convert inputs to JSON
        let usage_json = json!({
            "critical_pct": current_usage.critical_pct,
            "skills_pct": current_usage.skills_pct,
            "project_pct": current_usage.project_pct,
            "general_pct": current_usage.general_pct,
            "total_pct": current_usage.total_pct,
        });

        let resources_json = json!({
            "skill_names": loaded_resources.skill_names,
            "memory_ids": loaded_resources.memory_ids,
            "memory_summaries": loaded_resources.memory_summaries,
        });

        let mut inputs = HashMap::new();
        inputs.insert("current_usage".to_string(), usage_json);
        inputs.insert("loaded_resources".to_string(), resources_json);
        inputs.insert("target_pct".to_string(), json!(target_pct));
        inputs.insert("work_priority".to_string(), json!(work_priority));

        let outputs = self
            .bridge
            .call_agent_module("optimizer", inputs)
            .await?;

        // Parse outputs
        let unload_skills: Vec<String> = outputs
            .get("unload_skills")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();

        let unload_memory_ids: Vec<String> = outputs
            .get("unload_memory_ids")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();

        let optimization_rationale: String = outputs
            .get("optimization_rationale")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        Ok(OptimizationPlan {
            unload_skills,
            unload_memory_ids,
            optimization_rationale,
        })
    }
}

// =============================================================================
// Type definitions
// =============================================================================

/// Consolidated context result from DSPy
#[derive(Debug, Clone)]
pub struct ConsolidatedContext {
    /// Markdown-formatted consolidated context
    pub consolidated_content: String,
    /// List of critical issues to address
    pub key_issues: Vec<String>,
    /// Strategic recommendations for systematic fixes
    pub strategic_guidance: String,
    /// Estimated token count for consolidated content
    pub estimated_tokens: usize,
}

/// Skill metadata for discovery
#[derive(Debug, Clone)]
pub struct SkillMetadata {
    /// Skill name
    pub name: String,
    /// Skill description
    pub description: String,
    /// Associated keywords
    pub keywords: Vec<String>,
    /// Skill domains
    pub domains: Vec<String>,
}

/// Skill discovery result from DSPy
#[derive(Debug, Clone)]
pub struct SkillDiscoveryResult {
    /// Selected skill names, ordered by relevance
    pub selected_skills: Vec<String>,
    /// Relevance scores (0.0-1.0) for each skill
    pub relevance_scores: Vec<f32>,
    /// Explanation of selections
    pub reasoning: String,
}

/// Current context usage breakdown
#[derive(Debug, Clone)]
pub struct ContextUsage {
    /// Critical context percentage (system prompts, CLAUDE.md)
    pub critical_pct: f32,
    /// Skills percentage
    pub skills_pct: f32,
    /// Project context percentage (memories)
    pub project_pct: f32,
    /// General overhead percentage
    pub general_pct: f32,
    /// Total usage percentage
    pub total_pct: f32,
}

/// Currently loaded resources
#[derive(Debug, Clone)]
pub struct LoadedResources {
    /// Loaded skill names
    pub skill_names: Vec<String>,
    /// Loaded memory IDs
    pub memory_ids: Vec<String>,
    /// Memory summaries
    pub memory_summaries: Vec<String>,
}

/// Context optimization plan from DSPy
#[derive(Debug, Clone)]
pub struct OptimizationPlan {
    /// Skills to unload, ordered by removal priority
    pub unload_skills: Vec<String>,
    /// Memory IDs to unload, ordered by removal priority
    pub unload_memory_ids: Vec<String>,
    /// Explanation of optimization decisions
    pub optimization_rationale: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_adapter_is_send_sync() {
        fn assert_send<T: Send>() {}
        fn assert_sync<T: Sync>() {}

        assert_send::<OptimizerDSpyAdapter>();
        assert_sync::<OptimizerDSpyAdapter>();
    }
}
