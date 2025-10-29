//! Configuration for ICS
//!
//! Configuration options for the Integrated Context Studio

use serde::{Deserialize, Serialize};

/// ICS configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IcsConfig {
    /// Tab width in spaces
    pub tab_width: usize,

    /// Enable semantic analysis
    pub enable_semantic: bool,

    /// Analysis debounce time in milliseconds
    pub analysis_debounce_ms: u64,

    /// Maximum file size for analysis (bytes)
    pub max_file_size: usize,

    /// Theme name
    pub theme: String,
}

impl Default for IcsConfig {
    fn default() -> Self {
        Self {
            tab_width: 4,
            enable_semantic: true,
            analysis_debounce_ms: 500,
            max_file_size: 1024 * 1024, // 1MB
            theme: "default".to_string(),
        }
    }
}
