//! Configuration and settings for semantic highlighting

use serde::{Deserialize, Serialize};

/// Main highlighting settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HighlightSettings {
    /// Enable Tier 1 (structural patterns)
    pub enable_structural: bool,

    /// Enable Tier 2 (relational analysis)
    pub enable_relational: bool,

    /// Enable Tier 3 (analytical with Claude API)
    pub enable_analytical: bool,

    /// Relational analysis settings
    pub relational: RelationalSettings,

    /// Analytical analysis settings
    pub analytical: AnalyticalSettings,

    /// Visual settings
    pub visual: VisualSettings,
}

impl Default for HighlightSettings {
    fn default() -> Self {
        Self {
            enable_structural: true,
            enable_relational: true,
            enable_analytical: true, // Will be auto-disabled if no API key
            relational: RelationalSettings::default(),
            analytical: AnalyticalSettings::default(),
            visual: VisualSettings::default(),
        }
    }
}

/// Settings for Tier 2 relational analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelationalSettings {
    /// Debounce delay in milliseconds
    pub debounce_ms: u64,

    /// Enable entity recognition
    pub enable_entities: bool,

    /// Enable coreference resolution
    pub enable_coreference: bool,

    /// Enable relationship extraction
    pub enable_relationships: bool,

    /// Enable semantic role labeling
    pub enable_srl: bool,

    /// Enable anaphora resolution
    pub enable_anaphora: bool,

    /// Maximum distance for coreference (in sentences)
    pub max_coref_distance: usize,

    /// Minimum confidence for entity recognition (0.0-1.0)
    pub min_entity_confidence: f32,
}

impl Default for RelationalSettings {
    fn default() -> Self {
        Self {
            debounce_ms: 200,
            enable_entities: true,
            enable_coreference: true,
            enable_relationships: true,
            enable_srl: true,
            enable_anaphora: true,
            max_coref_distance: 3,
            min_entity_confidence: 0.6,
        }
    }
}

/// Settings for Tier 3 analytical analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalyticalSettings {
    /// Debounce delay in milliseconds
    pub debounce_ms: u64,

    /// Enable discourse analysis
    pub enable_discourse: bool,

    /// Enable contradiction detection
    pub enable_contradictions: bool,

    /// Enable presupposition extraction
    pub enable_presuppositions: bool,

    /// Enable cross-reference validation
    pub enable_cross_reference: bool,

    /// Maximum API calls per minute
    pub max_api_calls_per_minute: u32,

    /// Auto-analyze on idle
    pub auto_analyze_on_idle: bool,

    /// Cache analysis results
    pub cache_results: bool,

    /// Cache TTL in seconds
    pub cache_ttl_seconds: u64,
}

impl Default for AnalyticalSettings {
    fn default() -> Self {
        Self {
            debounce_ms: 2000,
            enable_discourse: true,
            enable_contradictions: true,
            enable_presuppositions: true,
            enable_cross_reference: true,
            max_api_calls_per_minute: 10,
            auto_analyze_on_idle: false,
            cache_results: true,
            cache_ttl_seconds: 3600, // 1 hour
        }
    }
}

/// Visual presentation settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VisualSettings {
    /// Show confidence scores in hovers
    pub show_confidence: bool,

    /// Show inline icons (⚠, ⓘ, 🔗, etc.)
    pub show_inline_icons: bool,

    /// Show connection lines between related spans
    pub show_connections: bool,

    /// Maximum connection lines to show
    pub max_connections: usize,

    /// Highlight intensity (0.0-1.0)
    pub highlight_intensity: f32,
}

impl Default for VisualSettings {
    fn default() -> Self {
        Self {
            show_confidence: false,
            show_inline_icons: true,
            show_connections: true,
            max_connections: 50,
            highlight_intensity: 0.8,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_settings() {
        let settings = HighlightSettings::default();
        assert!(settings.enable_structural);
        assert!(settings.enable_relational);
        assert_eq!(settings.relational.debounce_ms, 200);
    }

    #[test]
    fn test_serialization() {
        let settings = HighlightSettings::default();
        let json = serde_json::to_string(&settings).unwrap();
        let deserialized: HighlightSettings = serde_json::from_str(&json).unwrap();
        assert_eq!(settings.enable_structural, deserialized.enable_structural);
    }
}
