//! DSPy Instrumentation and Telemetry Sampling
//!
//! Provides transparent instrumentation layer for DSPy module invocations with:
//! - 10% sampling for training data collection
//! - Production logging for sampled interactions
//! - Telemetry collection for all interactions
//! - Performance tracking (latency, tokens, cost)
//!
//! # Architecture
//!
//! ```text
//! Request → DSpyInstrumentation → DSpyBridge → Python DSPy → LLM
//!                ↓ (10% sampling)
//!         ProductionLogger → logs/dspy_production.jsonl
//!                ↓ (all requests)
//!         TelemetryCollector → Prometheus/JSON metrics
//! ```
//!
//! # Usage
//!
//! ```rust,no_run
//! use mnemosyne_core::orchestration::dspy_instrumentation::{DSpyInstrumentation, InstrumentationConfig};
//! use mnemosyne_core::orchestration::dspy_bridge::DSpyBridge;
//! use mnemosyne_core::orchestration::dspy_production_logger::{ProductionLogger, LogConfig};
//! use mnemosyne_core::orchestration::dspy_telemetry::TelemetryCollector;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let bridge = Arc::new(DSpyBridge::new()?);
//! let logger = Arc::new(ProductionLogger::new(LogConfig::default()).await?);
//! let telemetry = Arc::new(TelemetryCollector::new());
//!
//! let config = InstrumentationConfig {
//!     sampling_rate: 0.10, // 10%
//!     enabled: true,
//! };
//!
//! let instrumentation = DSpyInstrumentation::new(bridge, logger, telemetry, config);
//!
//! // Call module with instrumentation
//! let request_id = uuid::Uuid::new_v4().to_string();
//! let result = instrumentation.call_module_with_sampling(
//!     "reviewer",
//!     &ModuleVersion::Baseline,
//!     inputs,
//!     &request_id
//! ).await?;
//! # Ok(())
//! # }
//! ```

use crate::error::{MnemosyneError, Result};
use crate::orchestration::dspy_bridge::DSpyBridge;
use crate::orchestration::dspy_module_loader::ModuleVersion;
use crate::orchestration::dspy_production_logger::{InteractionLog, ProductionLogger};
use crate::orchestration::dspy_telemetry::{
    CostCalculator, DSpyEvent, TelemetryCollector, TokenUsage,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::sync::Arc;
use std::time::{Instant, SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

/// Instrumentation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstrumentationConfig {
    /// Sampling rate for production logging (0.0-1.0)
    /// Default: 0.10 (10%)
    pub sampling_rate: f64,
    /// Whether instrumentation is enabled
    pub enabled: bool,
}

impl Default for InstrumentationConfig {
    fn default() -> Self {
        Self {
            sampling_rate: 0.10, // 10% sampling
            enabled: true,
        }
    }
}

/// DSPy instrumentation layer
///
/// Wraps DSPy module invocations with:
/// - Timing and performance tracking
/// - Hash-based sampling for training data collection
/// - Production logging (sampled calls only)
/// - Telemetry collection (all calls)
///
/// # Thread Safety
///
/// All operations are thread-safe via Arc<> wrappers for shared components.
pub struct DSpyInstrumentation {
    /// DSPy bridge for module invocations
    bridge: Arc<DSpyBridge>,
    /// Production logger for sampled interactions
    logger: Arc<ProductionLogger>,
    /// Telemetry collector for metrics
    telemetry: Arc<TelemetryCollector>,
    /// Configuration
    config: Arc<RwLock<InstrumentationConfig>>,
    /// Cost calculator for pricing
    cost_calculator: Arc<CostCalculator>,
}

impl DSpyInstrumentation {
    /// Create a new DSPy instrumentation layer
    ///
    /// # Arguments
    ///
    /// * `bridge` - DSPy bridge for module invocations
    /// * `logger` - Production logger for sampled interactions
    /// * `telemetry` - Telemetry collector for metrics
    /// * `config` - Instrumentation configuration
    pub fn new(
        bridge: Arc<DSpyBridge>,
        logger: Arc<ProductionLogger>,
        telemetry: Arc<TelemetryCollector>,
        config: InstrumentationConfig,
    ) -> Self {
        info!(
            "Initializing DSPy instrumentation with {}% sampling",
            config.sampling_rate * 100.0
        );

        Self {
            bridge,
            logger,
            telemetry,
            config: Arc::new(RwLock::new(config)),
            cost_calculator: Arc::new(CostCalculator::new()),
        }
    }

    /// Call DSPy module with instrumentation and sampling
    ///
    /// Wraps the DSPy bridge call with:
    /// - Performance timing
    /// - Error handling
    /// - Hash-based sampling decision
    /// - Production logging (if sampled)
    /// - Telemetry recording (always)
    ///
    /// # Arguments
    ///
    /// * `module_name` - Name of module (e.g., "reviewer", "optimizer")
    /// * `version` - Module version to use
    /// * `inputs` - Input parameters as JSON values
    /// * `request_id` - Unique request identifier for sampling
    ///
    /// # Returns
    ///
    /// HashMap of output field names to JSON values
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Module call fails
    /// - Logging fails (non-fatal, logged as warning)
    /// - Telemetry fails (non-fatal, logged as warning)
    pub async fn call_module_with_sampling(
        &self,
        module_name: &str,
        version: &ModuleVersion,
        inputs: HashMap<String, Value>,
        request_id: &str,
    ) -> Result<HashMap<String, Value>> {
        let config = self.config.read().await;
        let enabled = config.enabled;
        let should_sample = self.should_sample(request_id, config.sampling_rate);
        drop(config);

        if !enabled {
            // Instrumentation disabled, call directly
            return self.bridge.call_agent_module(module_name, inputs).await;
        }

        debug!(
            "Instrumented call to {} {} (request_id={}, sampled={})",
            module_name, version, request_id, should_sample
        );

        // Create request event
        let mut request_event = DSpyEvent::request(module_name, version.clone(), module_name);
        request_event = request_event.with_metadata("request_id".to_string(), request_id.to_string());

        // Record request event
        self.telemetry.record(request_event.clone()).await;

        // Start timing
        let start = Instant::now();
        let start_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        // Call module
        let result = self
            .bridge
            .call_agent_module(module_name, inputs.clone())
            .await;

        // End timing
        let latency = start.elapsed();
        let latency_ms = latency.as_millis() as u64;

        // Process result
        match result {
            Ok(outputs) => {
                // TODO: Extract token usage from outputs if available
                // For now, estimate based on input/output size
                let tokens = self.estimate_token_usage(&inputs, &outputs);
                let cost_usd = self
                    .cost_calculator
                    .calculate_cost("claude-haiku-4-5-20251001", &tokens);

                // Create response event
                let response_event =
                    DSpyEvent::response(&request_event, latency_ms, tokens, cost_usd);
                self.telemetry.record(response_event).await;

                // Log interaction if sampled
                if should_sample {
                    self.log_interaction(
                        module_name,
                        version,
                        &inputs,
                        &outputs,
                        start_ms,
                        latency_ms,
                        tokens,
                        cost_usd,
                        true,
                        None,
                    )
                    .await;
                }

                Ok(outputs)
            }
            Err(e) => {
                // Create error event
                let error_event = DSpyEvent::error(&request_event, latency_ms, e.to_string());
                self.telemetry.record(error_event).await;

                // Log error interaction if sampled
                if should_sample {
                    let empty_outputs = HashMap::new();
                    let tokens = TokenUsage::new(0, 0); // No tokens on error
                    self.log_interaction(
                        module_name,
                        version,
                        &inputs,
                        &empty_outputs,
                        start_ms,
                        latency_ms,
                        tokens,
                        0.0,
                        false,
                        Some(e.to_string()),
                    )
                    .await;
                }

                Err(e)
            }
        }
    }

    /// Determine if request should be sampled using hash-based selection
    ///
    /// Uses deterministic hashing to ensure consistent sampling for same request_id.
    /// This provides uniform distribution and reproducibility.
    ///
    /// # Arguments
    ///
    /// * `request_id` - Unique request identifier
    /// * `sampling_rate` - Target sampling rate (0.0-1.0)
    ///
    /// # Returns
    ///
    /// `true` if request should be sampled, `false` otherwise
    fn should_sample(&self, request_id: &str, sampling_rate: f64) -> bool {
        if sampling_rate <= 0.0 {
            return false;
        }
        if sampling_rate >= 1.0 {
            return true;
        }

        // Hash the request ID
        let mut hasher = DefaultHasher::new();
        request_id.hash(&mut hasher);
        let hash = hasher.finish();

        // Use hash to determine sampling (uniform distribution)
        let normalized = (hash % 10000) as f64 / 10000.0; // 0.0-1.0
        normalized < sampling_rate
    }

    /// Log interaction to production logger
    ///
    /// Async, non-blocking logging. Errors are logged as warnings but don't
    /// fail the request.
    async fn log_interaction(
        &self,
        module_name: &str,
        version: &ModuleVersion,
        inputs: &HashMap<String, Value>,
        outputs: &HashMap<String, Value>,
        timestamp_ms: u64,
        latency_ms: u64,
        tokens: TokenUsage,
        cost_usd: f64,
        success: bool,
        error: Option<String>,
    ) {
        let log = InteractionLog {
            module_name: module_name.to_string(),
            module_version: version.clone(),
            signature: module_name.to_string(), // Use module name as signature
            input: serde_json::to_value(inputs).unwrap_or(Value::Null),
            output: serde_json::to_value(outputs).unwrap_or(Value::Null),
            timestamp_ms,
            latency_ms,
            tokens,
            cost_usd,
            model: "claude-haiku-4-5-20251001".to_string(),
            success,
            error,
            metadata: HashMap::new(),
        };

        if let Err(e) = self.logger.log_interaction(log).await {
            warn!("Failed to log interaction: {}", e);
        }
    }

    /// Estimate token usage from input/output sizes
    ///
    /// Rough estimate: 1 token ≈ 4 characters
    /// This is a placeholder until we can extract actual token usage from LLM responses.
    fn estimate_token_usage(
        &self,
        inputs: &HashMap<String, Value>,
        outputs: &HashMap<String, Value>,
    ) -> TokenUsage {
        let input_str = serde_json::to_string(inputs).unwrap_or_default();
        let output_str = serde_json::to_string(outputs).unwrap_or_default();

        let input_tokens = (input_str.len() / 4) as u64;
        let output_tokens = (output_str.len() / 4) as u64;

        TokenUsage::new(input_tokens, output_tokens)
    }

    /// Update instrumentation configuration
    ///
    /// Allows dynamic configuration updates without restarting.
    pub async fn update_config(&self, config: InstrumentationConfig) {
        let mut current = self.config.write().await;
        *current = config.clone();
        info!(
            "Updated instrumentation config: enabled={}, sampling={}%",
            config.enabled,
            config.sampling_rate * 100.0
        );
    }

    /// Get current configuration
    pub async fn get_config(&self) -> InstrumentationConfig {
        self.config.read().await.clone()
    }

    /// Manually flush production logger
    ///
    /// Forces all buffered logs to be written to disk.
    pub async fn flush(&self) -> Result<()> {
        self.logger.flush().await
    }

    /// Get reference to telemetry collector
    pub fn telemetry(&self) -> &Arc<TelemetryCollector> {
        &self.telemetry
    }

    /// Get reference to production logger
    pub fn logger(&self) -> &Arc<ProductionLogger> {
        &self.logger
    }

    /// Convenience method matching DSpyBridge API for drop-in replacement
    ///
    /// Calls `call_module_with_sampling` with default version (Baseline)
    /// and auto-generated request_id. Use this method when migrating
    /// existing code that uses DSpyBridge.
    ///
    /// # Arguments
    ///
    /// * `module_name` - Name of module (e.g., "reviewer", "optimizer")
    /// * `inputs` - Input parameters as JSON values
    ///
    /// # Returns
    ///
    /// HashMap of output field names to JSON values
    pub async fn call_agent_module(
        &self,
        module_name: &str,
        inputs: HashMap<String, Value>,
    ) -> Result<HashMap<String, Value>> {
        // Generate request ID from inputs hash for consistent sampling
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        let input_json = serde_json::to_string(&inputs).unwrap_or_default();
        input_json.hash(&mut hasher);
        let request_id = format!("{:x}", hasher.finish());

        // Use Baseline version by default
        let version = ModuleVersion::Baseline;

        self.call_module_with_sampling(module_name, &version, inputs, &request_id)
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sampling_determinism() {
        let config = InstrumentationConfig {
            sampling_rate: 0.10,
            enabled: true,
        };

        // Create mock instrumentation for testing sampling logic
        let bridge = Arc::new(DSpyBridge::new().unwrap());
        let logger = tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(async {
                ProductionLogger::new(crate::orchestration::dspy_production_logger::LogConfig {
                    sink: crate::orchestration::dspy_production_logger::LogSink::Stdout,
                    buffer_size: 1,
                    enable_telemetry: false,
                })
                .await
                .unwrap()
            });
        let telemetry = Arc::new(TelemetryCollector::new());

        let inst = DSpyInstrumentation::new(bridge, Arc::new(logger), telemetry, config);

        // Same request_id should always give same sampling decision
        let request_id = "test-request-123";
        let sample1 = inst.should_sample(request_id, 0.10);
        let sample2 = inst.should_sample(request_id, 0.10);
        assert_eq!(sample1, sample2);
    }

    #[test]
    fn test_sampling_rate_accuracy() {
        let config = InstrumentationConfig {
            sampling_rate: 0.10,
            enabled: true,
        };

        let bridge = Arc::new(DSpyBridge::new().unwrap());
        let logger = tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(async {
                ProductionLogger::new(crate::orchestration::dspy_production_logger::LogConfig {
                    sink: crate::orchestration::dspy_production_logger::LogSink::Stdout,
                    buffer_size: 1,
                    enable_telemetry: false,
                })
                .await
                .unwrap()
            });
        let telemetry = Arc::new(TelemetryCollector::new());

        let inst = DSpyInstrumentation::new(bridge, Arc::new(logger), telemetry, config);

        // Test sampling rate over 1000 requests
        let mut sampled_count = 0;
        for i in 0..1000 {
            let request_id = format!("request-{}", i);
            if inst.should_sample(&request_id, 0.10) {
                sampled_count += 1;
            }
        }

        // Should be approximately 10% (±2% tolerance for 1000 samples)
        let sampling_rate = sampled_count as f64 / 1000.0;
        assert!(
            sampling_rate >= 0.08 && sampling_rate <= 0.12,
            "Sampling rate {} outside expected range [0.08, 0.12]",
            sampling_rate
        );
    }

    #[test]
    fn test_sampling_edge_cases() {
        let bridge = Arc::new(DSpyBridge::new().unwrap());
        let logger = tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(async {
                ProductionLogger::new(crate::orchestration::dspy_production_logger::LogConfig {
                    sink: crate::orchestration::dspy_production_logger::LogSink::Stdout,
                    buffer_size: 1,
                    enable_telemetry: false,
                })
                .await
                .unwrap()
            });
        let telemetry = Arc::new(TelemetryCollector::new());

        let config = InstrumentationConfig {
            sampling_rate: 0.0,
            enabled: true,
        };
        let inst = DSpyInstrumentation::new(bridge.clone(), Arc::new(logger), telemetry.clone(), config);

        // 0% sampling should never sample
        assert!(!inst.should_sample("test", 0.0));

        // 100% sampling should always sample
        assert!(inst.should_sample("test", 1.0));
        assert!(inst.should_sample("test", 1.5)); // Clamped to 1.0
    }

    #[test]
    fn test_token_estimation() {
        let bridge = Arc::new(DSpyBridge::new().unwrap());
        let logger = tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(async {
                ProductionLogger::new(crate::orchestration::dspy_production_logger::LogConfig {
                    sink: crate::orchestration::dspy_production_logger::LogSink::Stdout,
                    buffer_size: 1,
                    enable_telemetry: false,
                })
                .await
                .unwrap()
            });
        let telemetry = Arc::new(TelemetryCollector::new());

        let config = InstrumentationConfig::default();
        let inst = DSpyInstrumentation::new(bridge, Arc::new(logger), telemetry, config);

        let mut inputs = HashMap::new();
        inputs.insert("test".to_string(), Value::String("a".repeat(400)));

        let mut outputs = HashMap::new();
        outputs.insert("result".to_string(), Value::String("b".repeat(800)));

        let tokens = inst.estimate_token_usage(&inputs, &outputs);

        // 400 chars ≈ 100 tokens, 800 chars ≈ 200 tokens (rough estimate)
        assert!(tokens.input_tokens >= 90 && tokens.input_tokens <= 110);
        assert!(tokens.output_tokens >= 190 && tokens.output_tokens <= 210);
        assert_eq!(tokens.total_tokens, tokens.input_tokens + tokens.output_tokens);
    }
}
