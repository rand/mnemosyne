//! Python bindings for configuration management.
//!
//! Provides secure access to API keys and configuration from Python.

use pyo3::prelude::*;
use crate::config::ConfigManager;
use crate::error::Result;

/// Python wrapper for ConfigManager.
///
/// Provides access to configuration including secure API key retrieval
/// from environment variables, age-encrypted config, or OS keychain.
#[pyclass(name = "PyConfigManager")]
pub struct PyConfigManager {
    manager: ConfigManager,
}

#[pymethods]
impl PyConfigManager {
    /// Create a new ConfigManager instance.
    ///
    /// This will initialize the config system with paths from the environment
    /// or use defaults (~/.config/mnemosyne or ~/Library/Application Support/mnemosyne).
    #[new]
    fn new() -> PyResult<Self> {
        let manager = ConfigManager::new()
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                format!("Failed to initialize ConfigManager: {}", e)
            ))?;

        Ok(Self { manager })
    }

    /// Get the Anthropic API key from secure storage.
    ///
    /// Tries in order:
    /// 1. ANTHROPIC_API_KEY environment variable
    /// 2. Age-encrypted secrets file (~/.config/mnemosyne/secrets.age)
    /// 3. OS keychain (if keyring-fallback feature enabled)
    ///
    /// Returns:
    ///     str: The API key
    ///
    /// Raises:
    ///     RuntimeError: If no API key is configured
    fn get_api_key(&self) -> PyResult<String> {
        self.manager.get_api_key()
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                format!("Failed to retrieve API key: {}", e)
            ))
    }

    /// Check if an API key is configured.
    ///
    /// Returns:
    ///     bool: True if API key is available
    fn has_api_key(&self) -> bool {
        self.manager.get_api_key().is_ok()
    }
}
