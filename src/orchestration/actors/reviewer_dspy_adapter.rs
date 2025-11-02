//! ReviewerDSpyAdapter - Type-safe wrapper for Reviewer DSPy integration
//!
//! This adapter provides type-safe methods for calling the Reviewer DSPy module
//! from Rust code, wrapping the generic DSpyBridge interface with strongly-typed
//! method signatures that match the Reviewer's validation operations.
//!
//! ## Purpose
//!
//! - **Type Safety**: Convert between Rust types and JSON HashMap for DSPy calls
//! - **Error Handling**: Provide clear error messages for validation failures
//! - **Semantic Validation**: Four validation operations matching ReviewerModule signatures
//!
//! ## Available Operations
//!
//! 1. **extract_requirements**: Extract structured requirements from user intent
//! 2. **semantic_intent_check**: Validate intent satisfaction semantically
//! 3. **verify_completeness**: Check all requirements implemented
//! 4. **verify_correctness**: Validate logical correctness and bug-freedom
//!
//! ## Usage Example
//!
//! ```rust,no_run
//! use mnemosyne::orchestration::dspy_bridge::DSpyBridge;
//! use mnemosyne::orchestration::actors::reviewer_dspy_adapter::ReviewerDSpyAdapter;
//!
//! let bridge = DSpyBridge::new(dspy_service)?;
//! let adapter = ReviewerDSpyAdapter::new(bridge);
//!
//! // Extract requirements from intent
//! let requirements = adapter.extract_requirements(
//!     "Implement user authentication",
//!     Some("REST API with JWT tokens")
//! ).await?;
//!
//! // Validate intent satisfaction
//! let (passed, issues) = adapter.semantic_intent_check(
//!     "Implement user authentication",
//!     "Created auth.py with JWT support",
//!     vec![]
//! ).await?;
//! ```

use crate::{
    error::{MnemosyneError, Result},
    orchestration::dspy_bridge::DSpyBridge,
};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{debug, error, warn};

/// Type-safe adapter for Reviewer DSPy operations
///
/// Wraps DSpyBridge generic interface with strongly-typed methods
/// matching the ReviewerModule's signature.
pub struct ReviewerDSpyAdapter {
    bridge: Arc<DSpyBridge>,
}

impl ReviewerDSpyAdapter {
    /// Create new adapter from DSpyBridge
    pub fn new(bridge: Arc<DSpyBridge>) -> Self {
        Self { bridge }
    }

    /// Extract structured requirements from user intent
    ///
    /// # Arguments
    ///
    /// * `intent` - Original user intent/description
    /// * `context` - Optional additional context (work item phase, agent, scope)
    ///
    /// # Returns
    ///
    /// Vector of extracted requirement strings
    pub async fn extract_requirements(
        &self,
        intent: &str,
        context: Option<&str>,
    ) -> Result<Vec<String>> {
        debug!("DSPy: Extracting requirements from intent");

        let mut inputs = HashMap::new();
        inputs.insert("user_intent".to_string(), json!(intent));
        if let Some(ctx) = context {
            inputs.insert("context".to_string(), json!(ctx));
        }

        let outputs = self.bridge.call_agent_module("Reviewer", inputs).await?;

        // Extract requirements from output
        let requirements: Vec<String> = outputs
            .get("requirements")
            .ok_or_else(|| {
                error!("DSPy reviewer output missing 'requirements' field");
                MnemosyneError::Other("Missing requirements in DSPy output".to_string())
            })?
            .as_array()
            .ok_or_else(|| {
                error!("DSPy reviewer 'requirements' is not an array");
                MnemosyneError::Other("Invalid requirements format".to_string())
            })?
            .iter()
            .filter_map(|v| v.as_str().map(String::from))
            .collect();

        debug!("DSPy: Extracted {} requirements", requirements.len());
        Ok(requirements)
    }

    /// Validate intent satisfaction semantically
    ///
    /// # Arguments
    ///
    /// * `intent` - Original user intent
    /// * `implementation` - Implementation content (code, docs, etc.)
    /// * `execution_memories` - Execution context from memories (optional, for deeper analysis)
    ///
    /// # Returns
    ///
    /// Tuple of (passed: bool, issues: Vec<String>)
    pub async fn semantic_intent_check(
        &self,
        intent: &str,
        implementation: &str,
        execution_memories: Vec<Value>,
    ) -> Result<(bool, Vec<String>)> {
        debug!("DSPy: Checking semantic intent satisfaction");

        let mut inputs = HashMap::new();
        inputs.insert("user_intent".to_string(), json!(intent));
        inputs.insert("implementation".to_string(), json!(implementation));
        inputs.insert("execution_context".to_string(), json!(execution_memories));

        let outputs = self.bridge.call_agent_module("Reviewer", inputs).await?;

        // Extract validation results
        let intent_satisfied = outputs
            .get("intent_satisfied")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let issues: Vec<String> = outputs
            .get("issues")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();

        if !intent_satisfied {
            warn!(
                "DSPy: Intent satisfaction failed with {} issues",
                issues.len()
            );
        } else {
            debug!("DSPy: Intent satisfied");
        }

        Ok((intent_satisfied, issues))
    }

    /// Verify completeness - check all requirements implemented
    ///
    /// # Arguments
    ///
    /// * `requirements` - Extracted requirements to validate
    /// * `implementation` - Implementation content
    /// * `execution_memories` - Execution context from memories
    ///
    /// # Returns
    ///
    /// Tuple of (passed: bool, issues: Vec<String>)
    pub async fn verify_completeness(
        &self,
        requirements: &[String],
        implementation: &str,
        execution_memories: Vec<Value>,
    ) -> Result<(bool, Vec<String>)> {
        debug!("DSPy: Verifying completeness against {} requirements", requirements.len());

        let mut inputs = HashMap::new();
        inputs.insert("requirements".to_string(), json!(requirements));
        inputs.insert("implementation".to_string(), json!(implementation));
        inputs.insert("execution_context".to_string(), json!(execution_memories));

        let outputs = self.bridge.call_agent_module("Reviewer", inputs).await?;

        // Extract validation results
        let complete = outputs
            .get("complete")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let issues: Vec<String> = outputs
            .get("issues")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();

        if !complete {
            warn!("DSPy: Completeness check failed with {} issues", issues.len());
        } else {
            debug!("DSPy: Completeness verified");
        }

        Ok((complete, issues))
    }

    /// Verify correctness - validate logical soundness and bug-freedom
    ///
    /// # Arguments
    ///
    /// * `implementation` - Implementation content to validate
    /// * `execution_memories` - Execution context from memories
    ///
    /// # Returns
    ///
    /// Tuple of (passed: bool, issues: Vec<String>)
    pub async fn verify_correctness(
        &self,
        implementation: &str,
        execution_memories: Vec<Value>,
    ) -> Result<(bool, Vec<String>)> {
        debug!("DSPy: Verifying correctness");

        let mut inputs = HashMap::new();
        inputs.insert("implementation".to_string(), json!(implementation));
        inputs.insert("execution_context".to_string(), json!(execution_memories));

        let outputs = self.bridge.call_agent_module("Reviewer", inputs).await?;

        // Extract validation results
        let correct = outputs
            .get("correct")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let issues: Vec<String> = outputs
            .get("issues")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();

        if !correct {
            warn!("DSPy: Correctness check failed with {} issues", issues.len());
        } else {
            debug!("DSPy: Correctness verified");
        }

        Ok((correct, issues))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_adapter_construction() {
        // Test that adapter can be constructed (actual DSPy calls require Python runtime)
        // This is a placeholder for future integration tests
    }
}
