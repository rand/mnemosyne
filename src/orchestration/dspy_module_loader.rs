//! DSPy Module Loader for Production Deployment
//!
//! Provides version management and loading of optimized DSPy modules.
//! Supports baseline and multiple optimized versions with configuration-driven selection.
//!
//! # Architecture
//!
//! ```text
//! DSpyModuleLoader → Load JSON modules → DSpyBridge → Python DSPy → LLM
//! ```
//!
//! # Usage
//!
//! ```rust,no_run
//! use mnemosyne_core::orchestration::dspy_module_loader::{DSpyModuleLoader, ModuleVersion};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Initialize loader with configuration
//! let loader = DSpyModuleLoader::new("src/orchestration/dspy_modules")?;
//!
//! // Load optimized module
//! loader.load_module("reviewer", ModuleVersion::Optimized("v1".to_string())).await?;
//!
//! // Get active version
//! let version = loader.get_active_version("reviewer").await?;
//! # Ok(())
//! # }
//! ```

use crate::error::{MnemosyneError, Result};
use crate::orchestration::dspy_bridge::DSpyBridge;
use pyo3::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

/// Module version identifier
///
/// Distinguishes between baseline (unoptimized) and optimized versions.
/// Optimized versions are labeled with version strings (e.g., "v1", "v2").
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ModuleVersion {
    /// Baseline unoptimized module
    Baseline,
    /// Optimized module with version label (e.g., "v1", "v2")
    Optimized(String),
}

impl std::fmt::Display for ModuleVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ModuleVersion::Baseline => write!(f, "baseline"),
            ModuleVersion::Optimized(v) => write!(f, "optimized_{}", v),
        }
    }
}

/// Module metadata from results JSON
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleMetadata {
    /// Module name (e.g., "reviewer", "optimizer")
    pub name: String,
    /// Version (e.g., "v1", "v2")
    pub version: ModuleVersion,
    /// File path where module JSON is stored
    pub file_path: PathBuf,
    /// Optional performance metrics
    pub metrics: Option<HashMap<String, f64>>,
}

/// DSPy Module Loader
///
/// Manages loading and versioning of DSPy modules for production deployment.
/// Thread-safe via Arc<RwLock<>> for concurrent access.
///
/// Supports:
/// - Loading baseline modules from Python code
/// - Loading optimized modules from JSON files
/// - Version tracking per module
/// - Configuration-driven module selection
pub struct DSpyModuleLoader {
    /// Base directory for DSPy modules (e.g., "src/orchestration/dspy_modules")
    base_dir: PathBuf,
    /// Active module versions per module name
    active_versions: Arc<RwLock<HashMap<String, ModuleVersion>>>,
    /// Loaded module metadata
    loaded_modules: Arc<RwLock<HashMap<String, ModuleMetadata>>>,
    /// DSPy bridge for Python interop
    bridge: Arc<DSpyBridge>,
}

impl DSpyModuleLoader {
    /// Create a new DSPy module loader
    ///
    /// # Arguments
    ///
    /// * `base_dir` - Base directory containing DSPy modules and results
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Base directory doesn't exist
    /// - DSPy bridge initialization fails
    pub fn new<P: AsRef<Path>>(base_dir: P) -> Result<Self> {
        let base_dir = base_dir.as_ref().to_path_buf();

        if !base_dir.exists() {
            return Err(MnemosyneError::Other(format!(
                "DSPy modules directory not found: {}",
                base_dir.display()
            )));
        }

        let bridge = Arc::new(DSpyBridge::new()?);

        info!(
            "DSPy module loader initialized with base dir: {}",
            base_dir.display()
        );

        Ok(Self {
            base_dir,
            active_versions: Arc::new(RwLock::new(HashMap::new())),
            loaded_modules: Arc::new(RwLock::new(HashMap::new())),
            bridge,
        })
    }

    /// Load a module with specified version
    ///
    /// For baseline: Uses Python module loading via DSpyBridge
    /// For optimized: Loads from JSON file in results/ directory
    ///
    /// # Arguments
    ///
    /// * `module_name` - Name of module (e.g., "reviewer", "optimizer")
    /// * `version` - Version to load (baseline or optimized)
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Module file not found
    /// - JSON parsing fails
    /// - Python module loading fails
    pub async fn load_module(
        &self,
        module_name: &str,
        version: ModuleVersion,
    ) -> Result<ModuleMetadata> {
        debug!(
            "Loading DSPy module '{}' with version: {}",
            module_name, version
        );

        match &version {
            ModuleVersion::Baseline => self.load_baseline_module(module_name).await,
            ModuleVersion::Optimized(v) => self.load_optimized_module(module_name, v).await,
        }
    }

    /// Load baseline (unoptimized) module from Python
    async fn load_baseline_module(&self, module_name: &str) -> Result<ModuleMetadata> {
        // Baseline modules are loaded dynamically by Python
        // No need to load from JSON
        let metadata = ModuleMetadata {
            name: module_name.to_string(),
            version: ModuleVersion::Baseline,
            file_path: PathBuf::new(), // No file path for baseline
            metrics: None,
        };

        // Update active version
        let mut active = self.active_versions.write().await;
        active.insert(module_name.to_string(), ModuleVersion::Baseline);

        // Store metadata
        let mut loaded = self.loaded_modules.write().await;
        loaded.insert(module_name.to_string(), metadata.clone());

        info!("Loaded baseline module: {}", module_name);
        Ok(metadata)
    }

    /// Load optimized module from JSON file
    async fn load_optimized_module(
        &self,
        module_name: &str,
        version: &str,
    ) -> Result<ModuleMetadata> {
        // Construct path to optimized module JSON
        // Format: results/optimized_{module_name}_{version}.json
        let file_name = format!("optimized_{}_{}.json", module_name, version);
        let file_path = self.base_dir.join("results").join(&file_name);

        if !file_path.exists() {
            return Err(MnemosyneError::Other(format!(
                "Optimized module file not found: {}",
                file_path.display()
            )));
        }

        // Load and parse JSON
        let json_content = tokio::fs::read_to_string(&file_path)
            .await
            .map_err(|e| MnemosyneError::Other(format!("Failed to read module file: {}", e)))?;

        // Load module into Python via DSpyBridge
        self.load_module_from_json(module_name, &json_content)
            .await?;

        // Try to load associated metrics from results file
        let metrics = self.load_module_metrics(module_name, version).await;

        let metadata = ModuleMetadata {
            name: module_name.to_string(),
            version: ModuleVersion::Optimized(version.to_string()),
            file_path: file_path.clone(),
            metrics,
        };

        // Update active version
        let mut active = self.active_versions.write().await;
        active.insert(
            module_name.to_string(),
            ModuleVersion::Optimized(version.to_string()),
        );

        // Store metadata
        let mut loaded = self.loaded_modules.write().await;
        loaded.insert(module_name.to_string(), metadata.clone());

        info!(
            "Loaded optimized module: {} {} from {}",
            module_name,
            version,
            file_path.display()
        );
        Ok(metadata)
    }

    /// Load module from JSON string into Python
    async fn load_module_from_json(&self, module_name: &str, json_content: &str) -> Result<()> {
        let module_name = module_name.to_string();
        let json_content = json_content.to_string();
        let _bridge = self.bridge.clone();

        tokio::task::spawn_blocking(move || {
            Python::with_gil(|py| {
                // Import dspy
                let dspy = py.import_bound("dspy").map_err(|e| {
                    error!("Failed to import dspy: {}", e);
                    MnemosyneError::Other(format!("dspy import failed: {}", e))
                })?;

                // Parse JSON
                let json_module = py
                    .import_bound("json")
                    .map_err(|e| MnemosyneError::Other(format!("json import failed: {}", e)))?;
                let parsed = json_module
                    .call_method1("loads", (&json_content,))
                    .map_err(|e| MnemosyneError::Other(format!("JSON parsing failed: {}", e)))?;

                // Load module using dspy.load()
                // Note: This loads the module state into the current Python environment
                // The DSpyBridge.get_agent_module() will then return this loaded module
                let _loaded_module = dspy.call_method1("load", (parsed,)).map_err(|e| {
                    error!("Failed to load DSPy module '{}': {}", module_name, e);
                    MnemosyneError::Other(format!("DSPy module load failed: {}", e))
                })?;

                debug!("Successfully loaded optimized module into Python");
                Ok::<(), MnemosyneError>(())
            })
        })
        .await
        .map_err(|e| MnemosyneError::Other(format!("Async execution failed: {}", e)))??;

        Ok(())
    }

    /// Load module performance metrics from results JSON
    async fn load_module_metrics(
        &self,
        module_name: &str,
        version: &str,
    ) -> Option<HashMap<String, f64>> {
        // Format: results/optimized_{module_name}_{version}.results.json
        let file_name = format!("optimized_{}_{}.results.json", module_name, version);
        let file_path = self.base_dir.join("results").join(&file_name);

        if !file_path.exists() {
            warn!(
                "Metrics file not found for {} {}: {}",
                module_name,
                version,
                file_path.display()
            );
            return None;
        }

        match tokio::fs::read_to_string(&file_path).await {
            Ok(content) => match serde_json::from_str::<HashMap<String, f64>>(&content) {
                Ok(metrics) => {
                    debug!(
                        "Loaded metrics for {} {}: {:?}",
                        module_name, version, metrics
                    );
                    Some(metrics)
                }
                Err(e) => {
                    warn!("Failed to parse metrics file: {}", e);
                    None
                }
            },
            Err(e) => {
                warn!("Failed to read metrics file: {}", e);
                None
            }
        }
    }

    /// Get active version for a module
    ///
    /// Returns the currently loaded version for the specified module.
    /// Returns baseline if no version has been explicitly loaded.
    pub async fn get_active_version(&self, module_name: &str) -> Result<ModuleVersion> {
        let active = self.active_versions.read().await;
        Ok(active
            .get(module_name)
            .cloned()
            .unwrap_or(ModuleVersion::Baseline))
    }

    /// Get metadata for a loaded module
    pub async fn get_module_metadata(&self, module_name: &str) -> Option<ModuleMetadata> {
        let loaded = self.loaded_modules.read().await;
        loaded.get(module_name).cloned()
    }

    /// List all available optimized modules in results directory
    pub async fn list_available_modules(&self) -> Result<Vec<ModuleMetadata>> {
        let results_dir = self.base_dir.join("results");

        if !results_dir.exists() {
            return Ok(Vec::new());
        }

        let mut modules = Vec::new();
        let mut entries = tokio::fs::read_dir(&results_dir).await.map_err(|e| {
            MnemosyneError::Other(format!("Failed to read results directory: {}", e))
        })?;

        while let Some(entry) = entries
            .next_entry()
            .await
            .map_err(|e| MnemosyneError::Other(format!("Failed to read directory entry: {}", e)))?
        {
            let path = entry.path();
            if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
                // Match pattern: optimized_{module_name}_{version}.json
                if file_name.starts_with("optimized_")
                    && file_name.ends_with(".json")
                    && !file_name.contains(".results")
                {
                    if let Some((module_name, version)) = Self::parse_module_filename(file_name) {
                        let metrics = self.load_module_metrics(&module_name, &version).await;
                        modules.push(ModuleMetadata {
                            name: module_name,
                            version: ModuleVersion::Optimized(version),
                            file_path: path,
                            metrics,
                        });
                    }
                }
            }
        }

        debug!("Found {} available optimized modules", modules.len());
        Ok(modules)
    }

    /// Parse module filename to extract module name and version
    fn parse_module_filename(filename: &str) -> Option<(String, String)> {
        // Format: optimized_{module_name}_{version}.json
        let without_prefix = filename.strip_prefix("optimized_")?;
        let without_suffix = without_prefix.strip_suffix(".json")?;

        // Find last underscore to separate module_name from version
        let last_underscore = without_suffix.rfind('_')?;
        let module_name = without_suffix[..last_underscore].to_string();
        let version = without_suffix[last_underscore + 1..].to_string();

        Some((module_name, version))
    }

    /// Get reference to underlying DSpyBridge
    ///
    /// Allows direct access to bridge for calling modules
    pub fn bridge(&self) -> &DSpyBridge {
        &self.bridge
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_module_version_display() {
        assert_eq!(ModuleVersion::Baseline.to_string(), "baseline");
        assert_eq!(
            ModuleVersion::Optimized("v1".to_string()).to_string(),
            "optimized_v1"
        );
        assert_eq!(
            ModuleVersion::Optimized("v2".to_string()).to_string(),
            "optimized_v2"
        );
    }

    #[test]
    fn test_parse_module_filename() {
        assert_eq!(
            DSpyModuleLoader::parse_module_filename("optimized_reviewer_v1.json"),
            Some(("reviewer".to_string(), "v1".to_string()))
        );
        assert_eq!(
            DSpyModuleLoader::parse_module_filename("optimized_optimizer_v2.json"),
            Some(("optimizer".to_string(), "v2".to_string()))
        );
        assert_eq!(
            DSpyModuleLoader::parse_module_filename("optimized_semantic_v1.json"),
            Some(("semantic".to_string(), "v1".to_string()))
        );
        assert_eq!(
            DSpyModuleLoader::parse_module_filename("invalid_format.json"),
            None
        );
        assert_eq!(
            DSpyModuleLoader::parse_module_filename("optimized_reviewer_v1.results.json"),
            None
        );
    }

    #[test]
    fn test_module_version_equality() {
        assert_eq!(ModuleVersion::Baseline, ModuleVersion::Baseline);
        assert_eq!(
            ModuleVersion::Optimized("v1".to_string()),
            ModuleVersion::Optimized("v1".to_string())
        );
        assert_ne!(
            ModuleVersion::Baseline,
            ModuleVersion::Optimized("v1".to_string())
        );
        assert_ne!(
            ModuleVersion::Optimized("v1".to_string()),
            ModuleVersion::Optimized("v2".to_string())
        );
    }
}
