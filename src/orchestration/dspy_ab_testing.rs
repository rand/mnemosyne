//! A/B Testing Framework for DSPy Module Deployment
//!
//! Provides controlled rollout of optimized DSPy modules with traffic splitting,
//! performance comparison, and gradual rollout capabilities.
//!
//! # Architecture
//!
//! ```text
//! Request → ABTestRouter → Select Version → DSpyModuleLoader → DSpyBridge
//!                ↓
//!         ABTestMetrics (track performance)
//! ```
//!
//! # Usage
//!
//! ```rust,no_run
//! use mnemosyne_core::orchestration::dspy_ab_testing::{ABTestRouter, ABTestConfig};
//! use mnemosyne_core::orchestration::dspy_module_loader::ModuleVersion;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Create A/B test configuration
//! let config = ABTestConfig::new(
//!     "reviewer",
//!     ModuleVersion::Baseline,
//!     ModuleVersion::Optimized("v1".to_string()),
//!     0.10, // 10% traffic to optimized
//! );
//!
//! // Create router with module loader
//! let router = ABTestRouter::new(module_loader, vec![config]).await?;
//!
//! // Route request (automatically selects version based on config)
//! let selected_version = router.route_request("reviewer").await?;
//! # Ok(())
//! # }
//! ```

use crate::error::{MnemosyneError, Result};
use crate::orchestration::dspy_module_loader::{DSpyModuleLoader, ModuleVersion};
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

/// A/B test configuration for a specific module
///
/// Defines traffic split between control (baseline) and treatment (optimized) versions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ABTestConfig {
    /// Module name (e.g., "reviewer", "optimizer")
    pub module_name: String,
    /// Control version (typically baseline)
    pub control_version: ModuleVersion,
    /// Treatment version (typically optimized)
    pub treatment_version: ModuleVersion,
    /// Percentage of traffic to treatment (0.0-1.0)
    /// e.g., 0.10 = 10% to treatment, 90% to control
    pub treatment_percentage: f64,
    /// Whether the test is currently active
    pub enabled: bool,
}

impl ABTestConfig {
    /// Create a new A/B test configuration
    ///
    /// # Arguments
    ///
    /// * `module_name` - Name of module to A/B test
    /// * `control_version` - Baseline version (control group)
    /// * `treatment_version` - Optimized version (treatment group)
    /// * `treatment_percentage` - Fraction of traffic to treatment (0.0-1.0)
    pub fn new(
        module_name: impl Into<String>,
        control_version: ModuleVersion,
        treatment_version: ModuleVersion,
        treatment_percentage: f64,
    ) -> Self {
        Self {
            module_name: module_name.into(),
            control_version,
            treatment_version,
            treatment_percentage: treatment_percentage.clamp(0.0, 1.0),
            enabled: true,
        }
    }

    /// Create a gradual rollout config (starts at low percentage)
    ///
    /// Common pattern: Start at 1%, gradually increase to 100%
    pub fn gradual_rollout(
        module_name: impl Into<String>,
        treatment_version: ModuleVersion,
    ) -> Self {
        Self::new(
            module_name,
            ModuleVersion::Baseline,
            treatment_version,
            0.01, // Start at 1%
        )
    }

    /// Update traffic split percentage
    ///
    /// Used for gradual rollout: 1% → 5% → 10% → 25% → 50% → 100%
    pub fn set_treatment_percentage(&mut self, percentage: f64) {
        self.treatment_percentage = percentage.clamp(0.0, 1.0);
        info!(
            "Updated A/B test for {} to {}% treatment",
            self.module_name,
            (self.treatment_percentage * 100.0).round()
        );
    }

    /// Disable the A/B test (reverts to control)
    pub fn disable(&mut self) {
        self.enabled = false;
        info!("Disabled A/B test for {}", self.module_name);
    }

    /// Enable the A/B test
    pub fn enable(&mut self) {
        self.enabled = true;
        info!("Enabled A/B test for {}", self.module_name);
    }
}

/// Rollback policy configuration
///
/// Defines thresholds and rules for automated rollback.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RollbackPolicy {
    /// Maximum acceptable error rate (0.0-1.0)
    /// If treatment error rate exceeds this, trigger rollback
    pub max_error_rate: f64,
    /// Maximum acceptable latency increase factor (e.g., 1.5 = 50% increase)
    /// If treatment latency exceeds control * factor, trigger rollback
    pub max_latency_factor: f64,
    /// Minimum samples required before rollback can trigger
    /// Prevents premature rollback on insufficient data
    pub min_samples: u64,
    /// Enable automated rollback
    pub auto_rollback_enabled: bool,
}

impl RollbackPolicy {
    /// Create a default rollback policy
    ///
    /// Defaults: max_error_rate=0.10 (10%), max_latency_factor=1.5 (50% increase), min_samples=100
    pub fn default_policy() -> Self {
        Self {
            max_error_rate: 0.10,    // 10% error rate threshold
            max_latency_factor: 1.5, // 50% latency increase threshold
            min_samples: 100,
            auto_rollback_enabled: true,
        }
    }

    /// Create a conservative rollback policy (stricter thresholds)
    pub fn conservative() -> Self {
        Self {
            max_error_rate: 0.05,    // 5% error rate threshold
            max_latency_factor: 1.2, // 20% latency increase threshold
            min_samples: 200,
            auto_rollback_enabled: true,
        }
    }

    /// Create a permissive rollback policy (looser thresholds)
    pub fn permissive() -> Self {
        Self {
            max_error_rate: 0.20,    // 20% error rate threshold
            max_latency_factor: 2.0, // 100% latency increase threshold
            min_samples: 50,
            auto_rollback_enabled: true,
        }
    }
}

impl Default for RollbackPolicy {
    fn default() -> Self {
        Self::default_policy()
    }
}

/// Rollback event for history tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RollbackEvent {
    /// Module name
    pub module_name: String,
    /// Version rolled back from
    pub from_version: ModuleVersion,
    /// Version rolled back to
    pub to_version: ModuleVersion,
    /// Reason for rollback
    pub reason: String,
    /// Timestamp (Unix milliseconds)
    pub timestamp_ms: u64,
    /// Was this an automated rollback?
    pub automated: bool,
    /// Metrics snapshot at time of rollback
    pub metrics_snapshot: Option<String>,
}

impl RollbackEvent {
    pub fn new(
        module_name: impl Into<String>,
        from_version: ModuleVersion,
        to_version: ModuleVersion,
        reason: impl Into<String>,
        automated: bool,
    ) -> Self {
        let timestamp_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        Self {
            module_name: module_name.into(),
            from_version,
            to_version,
            reason: reason.into(),
            timestamp_ms,
            automated,
            metrics_snapshot: None,
        }
    }

    pub fn with_metrics(mut self, metrics: String) -> Self {
        self.metrics_snapshot = Some(metrics);
        self
    }
}

/// Performance metrics for a specific module version
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct VersionMetrics {
    /// Total number of requests
    pub request_count: u64,
    /// Total latency in milliseconds
    pub total_latency_ms: u64,
    /// Number of errors
    pub error_count: u64,
    /// Average latency in milliseconds
    pub avg_latency_ms: f64,
    /// Error rate (0.0-1.0)
    pub error_rate: f64,
}

impl VersionMetrics {
    /// Record a successful request with latency
    pub fn record_success(&mut self, latency: Duration) {
        self.request_count += 1;
        self.total_latency_ms += latency.as_millis() as u64;
        self.recalculate_averages();
    }

    /// Record a failed request with latency
    pub fn record_error(&mut self, latency: Duration) {
        self.request_count += 1;
        self.total_latency_ms += latency.as_millis() as u64;
        self.error_count += 1;
        self.recalculate_averages();
    }

    /// Recalculate average metrics
    fn recalculate_averages(&mut self) {
        if self.request_count > 0 {
            self.avg_latency_ms = self.total_latency_ms as f64 / self.request_count as f64;
            self.error_rate = self.error_count as f64 / self.request_count as f64;
        }
    }
}

/// Metrics tracker for A/B testing
///
/// Tracks performance comparison between control and treatment versions.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ABTestMetrics {
    /// Module name
    pub module_name: String,
    /// Control version metrics
    pub control_metrics: VersionMetrics,
    /// Treatment version metrics
    pub treatment_metrics: VersionMetrics,
    /// Test start time
    #[serde(skip)]
    pub start_time: Option<Instant>,
}

impl ABTestMetrics {
    /// Create new metrics tracker
    pub fn new(module_name: impl Into<String>) -> Self {
        Self {
            module_name: module_name.into(),
            control_metrics: VersionMetrics::default(),
            treatment_metrics: VersionMetrics::default(),
            start_time: Some(Instant::now()),
        }
    }

    /// Record a request for control version
    pub fn record_control(&mut self, latency: Duration, success: bool) {
        if success {
            self.control_metrics.record_success(latency);
        } else {
            self.control_metrics.record_error(latency);
        }
    }

    /// Record a request for treatment version
    pub fn record_treatment(&mut self, latency: Duration, success: bool) {
        if success {
            self.treatment_metrics.record_success(latency);
        } else {
            self.treatment_metrics.record_error(latency);
        }
    }

    /// Get performance improvement percentage (positive = treatment better)
    ///
    /// Compares average latency: (control - treatment) / control * 100
    pub fn latency_improvement_pct(&self) -> f64 {
        if self.control_metrics.avg_latency_ms == 0.0 {
            return 0.0;
        }
        ((self.control_metrics.avg_latency_ms - self.treatment_metrics.avg_latency_ms)
            / self.control_metrics.avg_latency_ms)
            * 100.0
    }

    /// Get error rate improvement percentage (positive = treatment better)
    ///
    /// Compares error rates: (control - treatment) / control * 100
    pub fn error_rate_improvement_pct(&self) -> f64 {
        if self.control_metrics.error_rate == 0.0 {
            return 0.0;
        }
        ((self.control_metrics.error_rate - self.treatment_metrics.error_rate)
            / self.control_metrics.error_rate)
            * 100.0
    }

    /// Check if treatment is statistically better than control
    ///
    /// Simple heuristic: treatment must have lower latency AND lower error rate
    /// with sufficient sample size (>100 requests each)
    pub fn is_treatment_better(&self) -> bool {
        let min_samples = 100;
        if self.control_metrics.request_count < min_samples
            || self.treatment_metrics.request_count < min_samples
        {
            return false; // Insufficient data
        }

        self.treatment_metrics.avg_latency_ms < self.control_metrics.avg_latency_ms
            && self.treatment_metrics.error_rate <= self.control_metrics.error_rate
    }
}

/// A/B test router for module version selection
///
/// Routes requests between control and treatment versions based on configuration.
/// Thread-safe and supports dynamic configuration updates.
pub struct ABTestRouter {
    /// Module loader for loading different versions
    module_loader: Arc<DSpyModuleLoader>,
    /// Active A/B test configurations per module
    configs: Arc<RwLock<HashMap<String, ABTestConfig>>>,
    /// Metrics tracker per module
    metrics: Arc<RwLock<HashMap<String, ABTestMetrics>>>,
    /// Random number generator for traffic splitting
    rng: Arc<RwLock<rand::rngs::ThreadRng>>,
}

impl ABTestRouter {
    /// Create a new A/B test router
    ///
    /// # Arguments
    ///
    /// * `module_loader` - Module loader for version management
    /// * `configs` - Initial A/B test configurations
    ///
    /// # Errors
    ///
    /// Returns error if module loading fails during initialization
    pub async fn new(
        module_loader: Arc<DSpyModuleLoader>,
        configs: Vec<ABTestConfig>,
    ) -> Result<Self> {
        let mut config_map = HashMap::new();
        let mut metrics_map = HashMap::new();

        // Initialize configs and metrics
        for config in configs {
            let module_name = config.module_name.clone();

            // Pre-load both versions
            module_loader
                .load_module(&module_name, config.control_version.clone())
                .await?;
            module_loader
                .load_module(&module_name, config.treatment_version.clone())
                .await?;

            metrics_map.insert(module_name.clone(), ABTestMetrics::new(&module_name));
            config_map.insert(module_name, config);
        }

        info!(
            "A/B test router initialized with {} tests",
            config_map.len()
        );

        Ok(Self {
            module_loader,
            configs: Arc::new(RwLock::new(config_map)),
            metrics: Arc::new(RwLock::new(metrics_map)),
            rng: Arc::new(RwLock::new(rand::thread_rng())),
        })
    }

    /// Route a request to appropriate module version
    ///
    /// Uses configured traffic split to randomly select control or treatment.
    /// If no A/B test configured, returns current active version.
    ///
    /// # Returns
    ///
    /// Selected module version for this request
    pub async fn route_request(&self, module_name: &str) -> Result<ModuleVersion> {
        let configs = self.configs.read().await;

        if let Some(config) = configs.get(module_name) {
            if !config.enabled {
                debug!("A/B test disabled for {}, using control", module_name);
                return Ok(config.control_version.clone());
            }

            // Random selection based on treatment percentage
            let mut rng = self.rng.write().await;
            let random_value: f64 = rng.gen();

            let selected_version = if random_value < config.treatment_percentage {
                debug!(
                    "Routed {} to treatment ({}% rollout)",
                    module_name,
                    (config.treatment_percentage * 100.0).round()
                );
                config.treatment_version.clone()
            } else {
                debug!("Routed {} to control", module_name);
                config.control_version.clone()
            };

            Ok(selected_version)
        } else {
            // No A/B test configured, use current active version
            self.module_loader.get_active_version(module_name).await
        }
    }

    /// Record request metrics
    ///
    /// Should be called after each request to track performance.
    ///
    /// # Arguments
    ///
    /// * `module_name` - Name of module that handled request
    /// * `version` - Version that was used
    /// * `latency` - Request latency
    /// * `success` - Whether request succeeded
    pub async fn record_metrics(
        &self,
        module_name: &str,
        version: &ModuleVersion,
        latency: Duration,
        success: bool,
    ) {
        let mut metrics = self.metrics.write().await;
        let configs = self.configs.read().await;

        if let Some(config) = configs.get(module_name) {
            let module_metrics = metrics
                .entry(module_name.to_string())
                .or_insert_with(|| ABTestMetrics::new(module_name));

            if version == &config.control_version {
                module_metrics.record_control(latency, success);
            } else if version == &config.treatment_version {
                module_metrics.record_treatment(latency, success);
            }
        }
    }

    /// Get current metrics for a module
    pub async fn get_metrics(&self, module_name: &str) -> Option<ABTestMetrics> {
        let metrics = self.metrics.read().await;
        metrics.get(module_name).cloned()
    }

    /// Get all active A/B test metrics
    pub async fn get_all_metrics(&self) -> HashMap<String, ABTestMetrics> {
        let metrics = self.metrics.read().await;
        metrics.clone()
    }

    /// Update A/B test configuration
    ///
    /// Used for gradual rollout: incrementally increase treatment percentage
    pub async fn update_config(&self, module_name: &str, new_config: ABTestConfig) -> Result<()> {
        let mut configs = self.configs.write().await;

        if let Some(config) = configs.get_mut(module_name) {
            *config = new_config;
            info!(
                "Updated A/B test config for {}: {}% treatment",
                module_name,
                (config.treatment_percentage * 100.0).round()
            );
            Ok(())
        } else {
            Err(MnemosyneError::Other(format!(
                "No A/B test configured for module: {}",
                module_name
            )))
        }
    }

    /// Increase treatment percentage (gradual rollout)
    ///
    /// Common rollout schedule: 1% → 5% → 10% → 25% → 50% → 100%
    pub async fn increase_rollout(&self, module_name: &str, new_percentage: f64) -> Result<()> {
        let mut configs = self.configs.write().await;

        if let Some(config) = configs.get_mut(module_name) {
            let old_percentage = config.treatment_percentage;
            config.set_treatment_percentage(new_percentage);

            info!(
                "Increased rollout for {} from {}% to {}%",
                module_name,
                (old_percentage * 100.0).round(),
                (new_percentage * 100.0).round()
            );
            Ok(())
        } else {
            Err(MnemosyneError::Other(format!(
                "No A/B test configured for module: {}",
                module_name
            )))
        }
    }

    /// Disable A/B test and revert to control
    ///
    /// Used for emergency rollback if treatment has issues
    pub async fn disable_test(&self, module_name: &str) -> Result<()> {
        let mut configs = self.configs.write().await;

        if let Some(config) = configs.get_mut(module_name) {
            config.disable();
            warn!("Disabled A/B test for {}, reverted to control", module_name);
            Ok(())
        } else {
            Err(MnemosyneError::Other(format!(
                "No A/B test configured for module: {}",
                module_name
            )))
        }
    }

    /// Enable A/B test
    pub async fn enable_test(&self, module_name: &str) -> Result<()> {
        let mut configs = self.configs.write().await;

        if let Some(config) = configs.get_mut(module_name) {
            config.enable();
            info!("Enabled A/B test for {}", module_name);
            Ok(())
        } else {
            Err(MnemosyneError::Other(format!(
                "No A/B test configured for module: {}",
                module_name
            )))
        }
    }

    /// Get reference to module loader
    pub fn module_loader(&self) -> &DSpyModuleLoader {
        &self.module_loader
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ab_config_creation() {
        let config = ABTestConfig::new(
            "reviewer",
            ModuleVersion::Baseline,
            ModuleVersion::Optimized("v1".to_string()),
            0.10,
        );

        assert_eq!(config.module_name, "reviewer");
        assert_eq!(config.treatment_percentage, 0.10);
        assert!(config.enabled);
    }

    #[test]
    fn test_ab_config_percentage_clamping() {
        let mut config = ABTestConfig::new(
            "test",
            ModuleVersion::Baseline,
            ModuleVersion::Optimized("v1".to_string()),
            1.5, // Should be clamped to 1.0
        );

        assert_eq!(config.treatment_percentage, 1.0);

        config.set_treatment_percentage(-0.5); // Should be clamped to 0.0
        assert_eq!(config.treatment_percentage, 0.0);
    }

    #[test]
    fn test_version_metrics() {
        let mut metrics = VersionMetrics::default();

        metrics.record_success(Duration::from_millis(100));
        metrics.record_success(Duration::from_millis(200));
        metrics.record_error(Duration::from_millis(150));

        assert_eq!(metrics.request_count, 3);
        assert_eq!(metrics.error_count, 1);
        assert_eq!(metrics.avg_latency_ms, 150.0);
        assert_eq!(metrics.error_rate, 1.0 / 3.0);
    }

    #[test]
    fn test_ab_metrics_improvement() {
        let mut metrics = ABTestMetrics::new("test");

        // Control: 100ms average, 10% errors
        metrics.record_control(Duration::from_millis(100), true);
        metrics.record_control(Duration::from_millis(100), true);
        metrics.record_control(Duration::from_millis(100), true);
        metrics.record_control(Duration::from_millis(100), true);
        metrics.record_control(Duration::from_millis(100), true);
        metrics.record_control(Duration::from_millis(100), true);
        metrics.record_control(Duration::from_millis(100), true);
        metrics.record_control(Duration::from_millis(100), true);
        metrics.record_control(Duration::from_millis(100), true);
        metrics.record_control(Duration::from_millis(100), false);

        // Treatment: 80ms average, 5% errors
        metrics.record_treatment(Duration::from_millis(80), true);
        metrics.record_treatment(Duration::from_millis(80), true);
        metrics.record_treatment(Duration::from_millis(80), true);
        metrics.record_treatment(Duration::from_millis(80), true);
        metrics.record_treatment(Duration::from_millis(80), true);
        metrics.record_treatment(Duration::from_millis(80), true);
        metrics.record_treatment(Duration::from_millis(80), true);
        metrics.record_treatment(Duration::from_millis(80), true);
        metrics.record_treatment(Duration::from_millis(80), true);
        metrics.record_treatment(Duration::from_millis(80), true);
        metrics.record_treatment(Duration::from_millis(80), true);
        metrics.record_treatment(Duration::from_millis(80), true);
        metrics.record_treatment(Duration::from_millis(80), true);
        metrics.record_treatment(Duration::from_millis(80), true);
        metrics.record_treatment(Duration::from_millis(80), true);
        metrics.record_treatment(Duration::from_millis(80), true);
        metrics.record_treatment(Duration::from_millis(80), true);
        metrics.record_treatment(Duration::from_millis(80), true);
        metrics.record_treatment(Duration::from_millis(80), true);
        metrics.record_treatment(Duration::from_millis(80), false);

        // 20% latency improvement
        assert_eq!(metrics.latency_improvement_pct(), 20.0);
    }

    #[test]
    fn test_gradual_rollout() {
        let config =
            ABTestConfig::gradual_rollout("reviewer", ModuleVersion::Optimized("v1".to_string()));

        assert_eq!(config.module_name, "reviewer");
        assert_eq!(config.treatment_percentage, 0.01); // Starts at 1%
        assert_eq!(config.control_version, ModuleVersion::Baseline);
    }
}
