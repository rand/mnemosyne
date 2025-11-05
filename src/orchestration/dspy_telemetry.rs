//! Telemetry and Observability for DSPy Operations
//!
//! Provides comprehensive telemetry, metrics, and cost tracking for DSPy module usage.
//! Integrates with A/B testing framework and module loader for production observability.
//!
//! # Architecture
//!
//! ```text
//! DSPy Call → TelemetryCollector → Record Event → Export Metrics
//!                  ↓                      ↓
//!          Track Performance      Cost Calculator
//! ```
//!
//! # Usage
//!
//! ```rust,no_run
//! use mnemosyne_core::orchestration::dspy_telemetry::{TelemetryCollector, DSpyEvent};
//! use mnemosyne_core::orchestration::dspy_module_loader::ModuleVersion;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Create telemetry collector
//! let telemetry = TelemetryCollector::new();
//!
//! // Record DSPy call
//! let event = DSpyEvent::request(
//!     "reviewer",
//!     ModuleVersion::Optimized("v1".to_string()),
//!     "validate_intent",
//! );
//! telemetry.record(event).await;
//!
//! // Export metrics for monitoring
//! let metrics = telemetry.export_prometheus().await?;
//! # Ok(())
//! # }
//! ```

use crate::error::Result;
use crate::orchestration::dspy_module_loader::ModuleVersion;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;
use tracing::{debug, info};

/// DSPy telemetry event
///
/// Captures all relevant information about a DSPy operation for observability.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DSpyEvent {
    /// Event ID (unique per request)
    pub event_id: String,
    /// Event type (request, response, error)
    pub event_type: EventType,
    /// Timestamp (Unix milliseconds)
    pub timestamp_ms: u64,
    /// Module name (e.g., "reviewer", "optimizer")
    pub module_name: String,
    /// Module version used
    pub module_version: ModuleVersion,
    /// Signature/method called (e.g., "validate_intent")
    pub signature: String,
    /// Latency in milliseconds (for response/error events)
    pub latency_ms: Option<u64>,
    /// Token usage (input + output)
    pub tokens: Option<TokenUsage>,
    /// API cost in USD
    pub cost_usd: Option<f64>,
    /// Error message (for error events)
    pub error: Option<String>,
    /// Additional metadata
    pub metadata: HashMap<String, String>,
}

/// Event type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EventType {
    /// Request initiated
    Request,
    /// Response received
    Response,
    /// Error occurred
    Error,
}

/// Token usage for LLM calls
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct TokenUsage {
    /// Input/prompt tokens
    pub input_tokens: u64,
    /// Output/completion tokens
    pub output_tokens: u64,
    /// Total tokens
    pub total_tokens: u64,
}

impl TokenUsage {
    pub fn new(input_tokens: u64, output_tokens: u64) -> Self {
        Self {
            input_tokens,
            output_tokens,
            total_tokens: input_tokens + output_tokens,
        }
    }
}

impl DSpyEvent {
    /// Create a request event
    pub fn request(
        module_name: impl Into<String>,
        module_version: ModuleVersion,
        signature: impl Into<String>,
    ) -> Self {
        let event_id = uuid::Uuid::new_v4().to_string();
        let timestamp_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        Self {
            event_id,
            event_type: EventType::Request,
            timestamp_ms,
            module_name: module_name.into(),
            module_version,
            signature: signature.into(),
            latency_ms: None,
            tokens: None,
            cost_usd: None,
            error: None,
            metadata: HashMap::new(),
        }
    }

    /// Create a response event
    pub fn response(
        request: &Self,
        latency_ms: u64,
        tokens: TokenUsage,
        cost_usd: f64,
    ) -> Self {
        let timestamp_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        Self {
            event_id: request.event_id.clone(),
            event_type: EventType::Response,
            timestamp_ms,
            module_name: request.module_name.clone(),
            module_version: request.module_version.clone(),
            signature: request.signature.clone(),
            latency_ms: Some(latency_ms),
            tokens: Some(tokens),
            cost_usd: Some(cost_usd),
            error: None,
            metadata: request.metadata.clone(),
        }
    }

    /// Create an error event
    pub fn error(request: &Self, latency_ms: u64, error: impl Into<String>) -> Self {
        let timestamp_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        Self {
            event_id: request.event_id.clone(),
            event_type: EventType::Error,
            timestamp_ms,
            module_name: request.module_name.clone(),
            module_version: request.module_version.clone(),
            signature: request.signature.clone(),
            latency_ms: Some(latency_ms),
            tokens: None,
            cost_usd: None,
            error: Some(error.into()),
            metadata: request.metadata.clone(),
        }
    }

    /// Add metadata to event
    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }
}

/// Cost calculator for LLM API calls
///
/// Uses pricing data from Anthropic API (as of 2025-01-01).
pub struct CostCalculator {
    /// Pricing per 1M input tokens (USD)
    input_price_per_1m: HashMap<String, f64>,
    /// Pricing per 1M output tokens (USD)
    output_price_per_1m: HashMap<String, f64>,
}

impl CostCalculator {
    /// Create a new cost calculator with default Anthropic pricing
    pub fn new() -> Self {
        let mut input_price_per_1m = HashMap::new();
        let mut output_price_per_1m = HashMap::new();

        // Claude Sonnet 4 pricing (as of 2025-01-01)
        input_price_per_1m.insert("claude-sonnet-4-20250514".to_string(), 3.0);
        output_price_per_1m.insert("claude-sonnet-4-20250514".to_string(), 15.0);

        // Claude Haiku 3.5 pricing
        input_price_per_1m.insert("claude-3-5-haiku-20241022".to_string(), 1.0);
        output_price_per_1m.insert("claude-3-5-haiku-20241022".to_string(), 5.0);

        Self {
            input_price_per_1m,
            output_price_per_1m,
        }
    }

    /// Calculate cost for a given model and token usage
    ///
    /// Returns cost in USD
    pub fn calculate_cost(&self, model: &str, tokens: &TokenUsage) -> f64 {
        let input_price = self
            .input_price_per_1m
            .get(model)
            .copied()
            .unwrap_or(3.0); // Default to Sonnet pricing
        let output_price = self
            .output_price_per_1m
            .get(model)
            .copied()
            .unwrap_or(15.0);

        let input_cost = (tokens.input_tokens as f64 / 1_000_000.0) * input_price;
        let output_cost = (tokens.output_tokens as f64 / 1_000_000.0) * output_price;

        input_cost + output_cost
    }

    /// Add custom pricing for a model
    pub fn add_model_pricing(&mut self, model: impl Into<String>, input_price: f64, output_price: f64) {
        let model = model.into();
        self.input_price_per_1m.insert(model.clone(), input_price);
        self.output_price_per_1m.insert(model, output_price);
    }
}

impl Default for CostCalculator {
    fn default() -> Self {
        Self::new()
    }
}

/// Aggregated metrics for a module version
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleMetrics {
    /// Module name
    pub module_name: String,
    /// Module version
    pub module_version: ModuleVersion,
    /// Total requests
    pub request_count: u64,
    /// Successful responses
    pub success_count: u64,
    /// Error count
    pub error_count: u64,
    /// Total latency (milliseconds)
    pub total_latency_ms: u64,
    /// Total tokens consumed
    pub total_tokens: u64,
    /// Total cost (USD)
    pub total_cost_usd: f64,
    /// Average latency (milliseconds)
    pub avg_latency_ms: f64,
    /// p50 latency (milliseconds)
    pub p50_latency_ms: f64,
    /// p95 latency (milliseconds)
    pub p95_latency_ms: f64,
    /// p99 latency (milliseconds)
    pub p99_latency_ms: f64,
    /// Error rate (0.0-1.0)
    pub error_rate: f64,
}

impl ModuleMetrics {
    pub fn new(module_name: String, module_version: ModuleVersion) -> Self {
        Self {
            module_name,
            module_version,
            request_count: 0,
            success_count: 0,
            error_count: 0,
            total_latency_ms: 0,
            total_tokens: 0,
            total_cost_usd: 0.0,
            avg_latency_ms: 0.0,
            p50_latency_ms: 0.0,
            p95_latency_ms: 0.0,
            p99_latency_ms: 0.0,
            error_rate: 0.0,
        }
    }

    /// Update metrics with new event
    pub fn update(&mut self, event: &DSpyEvent, latencies: &[u64]) {
        match event.event_type {
            EventType::Request => {
                self.request_count += 1;
            }
            EventType::Response => {
                self.success_count += 1;
                if let Some(latency) = event.latency_ms {
                    self.total_latency_ms += latency;
                }
                if let Some(tokens) = event.tokens {
                    self.total_tokens += tokens.total_tokens;
                }
                if let Some(cost) = event.cost_usd {
                    self.total_cost_usd += cost;
                }
            }
            EventType::Error => {
                self.error_count += 1;
                if let Some(latency) = event.latency_ms {
                    self.total_latency_ms += latency;
                }
            }
        }

        // Recalculate aggregates
        let total_completed = self.success_count + self.error_count;
        if total_completed > 0 {
            self.avg_latency_ms = self.total_latency_ms as f64 / total_completed as f64;
            self.error_rate = self.error_count as f64 / self.request_count as f64;

            // Calculate percentiles
            if !latencies.is_empty() {
                let mut sorted = latencies.to_vec();
                sorted.sort_unstable();
                let len = sorted.len();
                self.p50_latency_ms = sorted[len * 50 / 100] as f64;
                self.p95_latency_ms = sorted[len * 95 / 100] as f64;
                self.p99_latency_ms = sorted[len * 99 / 100] as f64;
            }
        }
    }
}

/// Telemetry collector
///
/// Thread-safe collector for DSPy telemetry events with metrics aggregation.
pub struct TelemetryCollector {
    /// Raw events
    events: Arc<RwLock<Vec<DSpyEvent>>>,
    /// Aggregated metrics per module version
    metrics: Arc<RwLock<HashMap<String, ModuleMetrics>>>,
    /// Latency samples for percentile calculation
    latencies: Arc<RwLock<HashMap<String, Vec<u64>>>>,
    /// Cost calculator
    cost_calculator: Arc<CostCalculator>,
}

impl TelemetryCollector {
    /// Create a new telemetry collector
    pub fn new() -> Self {
        Self {
            events: Arc::new(RwLock::new(Vec::new())),
            metrics: Arc::new(RwLock::new(HashMap::new())),
            latencies: Arc::new(RwLock::new(HashMap::new())),
            cost_calculator: Arc::new(CostCalculator::new()),
        }
    }

    /// Create a new telemetry collector with custom cost calculator
    pub fn with_cost_calculator(cost_calculator: CostCalculator) -> Self {
        Self {
            events: Arc::new(RwLock::new(Vec::new())),
            metrics: Arc::new(RwLock::new(HashMap::new())),
            latencies: Arc::new(RwLock::new(HashMap::new())),
            cost_calculator: Arc::new(cost_calculator),
        }
    }

    /// Record a telemetry event
    pub async fn record(&self, event: DSpyEvent) {
        debug!(
            "Recording DSPy event: {} {} {} {:?}",
            event.module_name, event.module_version, event.signature, event.event_type
        );

        let key = format!("{}:{}", event.module_name, event.module_version);

        // Store raw event
        {
            let mut events = self.events.write().await;
            events.push(event.clone());
        }

        // Update aggregated metrics
        {
            let mut metrics = self.metrics.write().await;
            let module_metrics = metrics
                .entry(key.clone())
                .or_insert_with(|| ModuleMetrics::new(event.module_name.clone(), event.module_version.clone()));

            // Get latencies for percentile calculation
            let latencies = self.latencies.read().await;
            let latency_samples = latencies.get(&key).map(|v| v.as_slice()).unwrap_or(&[]);

            module_metrics.update(&event, latency_samples);
        }

        // Store latency sample
        if let Some(latency) = event.latency_ms {
            let mut latencies = self.latencies.write().await;
            latencies.entry(key).or_default().push(latency);
        }
    }

    /// Get metrics for a specific module version
    pub async fn get_metrics(&self, module_name: &str, version: &ModuleVersion) -> Option<ModuleMetrics> {
        let key = format!("{}:{}", module_name, version);
        let metrics = self.metrics.read().await;
        metrics.get(&key).cloned()
    }

    /// Get all metrics
    pub async fn get_all_metrics(&self) -> HashMap<String, ModuleMetrics> {
        let metrics = self.metrics.read().await;
        metrics.clone()
    }

    /// Export metrics in Prometheus format
    ///
    /// Returns a string containing Prometheus-compatible metrics.
    pub async fn export_prometheus(&self) -> Result<String> {
        let metrics = self.metrics.read().await;
        let mut output = String::new();

        // Request count
        output.push_str("# HELP dspy_requests_total Total number of DSPy requests\n");
        output.push_str("# TYPE dspy_requests_total counter\n");
        for m in metrics.values() {
            output.push_str(&format!(
                "dspy_requests_total{{module=\"{}\",version=\"{}\"}} {}\n",
                m.module_name, m.module_version, m.request_count
            ));
        }

        // Success count
        output.push_str("# HELP dspy_success_total Total number of successful DSPy responses\n");
        output.push_str("# TYPE dspy_success_total counter\n");
        for m in metrics.values() {
            output.push_str(&format!(
                "dspy_success_total{{module=\"{}\",version=\"{}\"}} {}\n",
                m.module_name, m.module_version, m.success_count
            ));
        }

        // Error count
        output.push_str("# HELP dspy_errors_total Total number of DSPy errors\n");
        output.push_str("# TYPE dspy_errors_total counter\n");
        for m in metrics.values() {
            output.push_str(&format!(
                "dspy_errors_total{{module=\"{}\",version=\"{}\"}} {}\n",
                m.module_name, m.module_version, m.error_count
            ));
        }

        // Latency metrics
        output.push_str("# HELP dspy_latency_ms_avg Average latency in milliseconds\n");
        output.push_str("# TYPE dspy_latency_ms_avg gauge\n");
        for m in metrics.values() {
            output.push_str(&format!(
                "dspy_latency_ms_avg{{module=\"{}\",version=\"{}\"}} {}\n",
                m.module_name, m.module_version, m.avg_latency_ms
            ));
        }

        output.push_str("# HELP dspy_latency_ms_p95 p95 latency in milliseconds\n");
        output.push_str("# TYPE dspy_latency_ms_p95 gauge\n");
        for m in metrics.values() {
            output.push_str(&format!(
                "dspy_latency_ms_p95{{module=\"{}\",version=\"{}\"}} {}\n",
                m.module_name, m.module_version, m.p95_latency_ms
            ));
        }

        // Cost metrics
        output.push_str("# HELP dspy_cost_usd_total Total cost in USD\n");
        output.push_str("# TYPE dspy_cost_usd_total counter\n");
        for m in metrics.values() {
            output.push_str(&format!(
                "dspy_cost_usd_total{{module=\"{}\",version=\"{}\"}} {}\n",
                m.module_name, m.module_version, m.total_cost_usd
            ));
        }

        // Token metrics
        output.push_str("# HELP dspy_tokens_total Total tokens consumed\n");
        output.push_str("# TYPE dspy_tokens_total counter\n");
        for m in metrics.values() {
            output.push_str(&format!(
                "dspy_tokens_total{{module=\"{}\",version=\"{}\"}} {}\n",
                m.module_name, m.module_version, m.total_tokens
            ));
        }

        // Error rate
        output.push_str("# HELP dspy_error_rate Error rate (0.0-1.0)\n");
        output.push_str("# TYPE dspy_error_rate gauge\n");
        for m in metrics.values() {
            output.push_str(&format!(
                "dspy_error_rate{{module=\"{}\",version=\"{}\"}} {}\n",
                m.module_name, m.module_version, m.error_rate
            ));
        }

        Ok(output)
    }

    /// Export metrics as JSON
    pub async fn export_json(&self) -> Result<String> {
        let metrics = self.get_all_metrics().await;
        serde_json::to_string_pretty(&metrics)
            .map_err(|e| crate::error::MnemosyneError::Other(format!("JSON export failed: {}", e)))
    }

    /// Clear all collected data
    ///
    /// Useful for resetting metrics after exporting to external systems.
    pub async fn clear(&self) {
        let mut events = self.events.write().await;
        events.clear();

        let mut metrics = self.metrics.write().await;
        metrics.clear();

        let mut latencies = self.latencies.write().await;
        latencies.clear();

        info!("Telemetry data cleared");
    }

    /// Get cost calculator reference
    pub fn cost_calculator(&self) -> &CostCalculator {
        &self.cost_calculator
    }
}

impl Default for TelemetryCollector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_usage() {
        let tokens = TokenUsage::new(1000, 500);
        assert_eq!(tokens.input_tokens, 1000);
        assert_eq!(tokens.output_tokens, 500);
        assert_eq!(tokens.total_tokens, 1500);
    }

    #[test]
    fn test_cost_calculator() {
        let calc = CostCalculator::new();
        let tokens = TokenUsage::new(1_000_000, 1_000_000);

        // Sonnet pricing: $3/1M input + $15/1M output
        let cost = calc.calculate_cost("claude-sonnet-4-20250514", &tokens);
        assert!((cost - 18.0).abs() < 0.001); // $3 + $15 = $18

        // Haiku pricing: $1/1M input + $5/1M output
        let cost = calc.calculate_cost("claude-3-5-haiku-20241022", &tokens);
        assert!((cost - 6.0).abs() < 0.001); // $1 + $5 = $6
    }

    #[test]
    fn test_cost_calculator_small_usage() {
        let calc = CostCalculator::new();
        let tokens = TokenUsage::new(10_000, 5_000); // 10k input, 5k output

        // Sonnet: 0.03 + 0.075 = 0.105
        let cost = calc.calculate_cost("claude-sonnet-4-20250514", &tokens);
        assert!((cost - 0.105).abs() < 0.001);
    }

    #[tokio::test]
    async fn test_telemetry_collector() {
        let collector = TelemetryCollector::new();

        // Create request event
        let request = DSpyEvent::request(
            "reviewer",
            ModuleVersion::Optimized("v1".to_string()),
            "validate_intent",
        );

        // Record request
        collector.record(request.clone()).await;

        // Create response event
        let tokens = TokenUsage::new(1000, 500);
        let cost = collector.cost_calculator().calculate_cost("claude-sonnet-4-20250514", &tokens);
        let response = DSpyEvent::response(&request, 250, tokens, cost);

        // Record response
        collector.record(response).await;

        // Check metrics
        let metrics = collector
            .get_metrics("reviewer", &ModuleVersion::Optimized("v1".to_string()))
            .await
            .unwrap();

        assert_eq!(metrics.request_count, 1);
        assert_eq!(metrics.success_count, 1);
        assert_eq!(metrics.error_count, 0);
        assert_eq!(metrics.avg_latency_ms, 250.0);
        assert_eq!(metrics.total_tokens, 1500);
        assert!(metrics.total_cost_usd > 0.0);
    }

    #[tokio::test]
    async fn test_prometheus_export() {
        let collector = TelemetryCollector::new();

        let request = DSpyEvent::request(
            "reviewer",
            ModuleVersion::Baseline,
            "validate_intent",
        );
        collector.record(request.clone()).await;

        let tokens = TokenUsage::new(1000, 500);
        let cost = collector.cost_calculator().calculate_cost("claude-sonnet-4-20250514", &tokens);
        let response = DSpyEvent::response(&request, 250, tokens, cost);
        collector.record(response).await;

        let prometheus = collector.export_prometheus().await.unwrap();

        assert!(prometheus.contains("dspy_requests_total"));
        assert!(prometheus.contains("dspy_latency_ms_avg"));
        assert!(prometheus.contains("dspy_cost_usd_total"));
        assert!(prometheus.contains("module=\"reviewer\""));
    }
}
