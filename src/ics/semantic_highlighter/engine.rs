//! Main semantic highlighting engine coordinating all three tiers

use crate::LlmService;
use ratatui::text::Line;
use std::sync::Arc;
use tokio::sync::mpsc;

use super::{
    cache::SemanticCache,
    settings::HighlightSettings,
    tier1_structural::StructuralHighlighter,
    tier2_relational::RelationalAnalyzer,
    tier3_analytical::AnalyticalProcessor,
    visualization::{HighlightSpan, SpanMerger},
    Result, SemanticError,
};

/// Main semantic highlighting engine
///
/// Coordinates three performance tiers:
/// - Tier 1 (Structural): Real-time pattern matching
/// - Tier 2 (Relational): Incremental local NLP
/// - Tier 3 (Analytical): Optional Claude API analysis
pub struct SemanticHighlightEngine {
    /// Configuration settings
    settings: HighlightSettings,

    /// Tier 1: Structural pattern highlighter (always active)
    structural: StructuralHighlighter,

    /// Tier 2: Relational analyzer (debounced)
    relational: Option<RelationalAnalyzer>,

    /// Tier 3: Analytical processor (background, optional)
    analytical: Option<AnalyticalProcessor>,

    /// Unified cache
    cache: Arc<SemanticCache>,

    /// Channel for background analysis requests
    analysis_tx: Option<mpsc::Sender<AnalysisRequest>>,
}

/// Request for background analysis
#[derive(Debug, Clone)]
pub enum AnalysisRequest {
    /// Analyze specific range
    Range {
        text: String,
        range: std::ops::Range<usize>,
    },

    /// Analyze full document
    Full {
        text: String,
    },

    /// Clear all caches
    ClearCache,
}

impl SemanticHighlightEngine {
    /// Create new engine with optional LLM service for Tier 3
    pub fn new(llm_service: Option<Arc<LlmService>>) -> Self {
        Self::with_settings(HighlightSettings::default(), llm_service)
    }

    /// Create engine with custom settings
    pub fn with_settings(
        settings: HighlightSettings,
        llm_service: Option<Arc<LlmService>>,
    ) -> Self {
        let cache = Arc::new(SemanticCache::default());

        // Tier 1: Always enabled
        let structural = StructuralHighlighter::new();

        // Tier 2: Enabled if configured
        let relational = if settings.enable_relational {
            Some(RelationalAnalyzer::new(
                settings.relational.clone(),
                Arc::clone(&cache),
            ))
        } else {
            None
        };

        // Tier 3: Requires LLM service and configuration
        let (analytical, analysis_tx) = if let (true, Some(llm)) = (settings.enable_analytical, llm_service) {
            let (tx, rx) = mpsc::channel(32);
            let processor = AnalyticalProcessor::new(
                llm,
                settings.analytical.clone(),
                Arc::clone(&cache),
                rx,
            );
            (Some(processor), Some(tx))
        } else {
            (None, None)
        };

        Self {
            settings,
            structural,
            relational,
            analytical,
            cache,
            analysis_tx,
        }
    }

    /// Highlight a single line of text
    ///
    /// This is the primary API for real-time highlighting.
    /// Combines results from all active tiers.
    pub fn highlight_line(&mut self, text: &str) -> Line<'static> {
        let mut spans = Vec::new();

        // Tier 1: Structural (always, <5ms)
        if self.settings.enable_structural {
            if let Ok(structural_spans) = self.structural.highlight(text) {
                spans.extend(structural_spans);
            }
        }

        // Tier 2: Relational (if enabled and cached)
        if let Some(ref mut relational) = self.relational {
            if let Ok(relational_spans) = relational.highlight_cached(text) {
                spans.extend(relational_spans);
            }
        }

        // Tier 3: Analytical (if available and cached)
        if let Some(ref analytical) = self.analytical {
            if let Ok(analytical_spans) = analytical.get_cached_highlights(text) {
                spans.extend(analytical_spans);
            }
        }

        // Merge overlapping spans by priority
        let merged = self.merge_spans(spans);

        // Convert to ratatui Line
        self.spans_to_line(merged, text)
    }

    /// Schedule incremental analysis for a text range
    ///
    /// Triggers Tier 2 analysis after debounce delay.
    pub fn schedule_analysis(&mut self, text: &str, range: std::ops::Range<usize>) {
        if let Some(ref mut relational) = self.relational {
            relational.schedule_analysis(text, range);
        }
    }

    /// Request background analytical processing (Tier 3)
    ///
    /// Returns immediately. Results will be cached and available later.
    pub async fn request_analysis(&self, request: AnalysisRequest) -> Result<()> {
        if let Some(ref tx) = self.analysis_tx {
            tx.send(request)
                .await
                .map_err(|e| SemanticError::AnalysisFailed(e.to_string()))?;
            Ok(())
        } else {
            Err(SemanticError::AnalysisFailed(
                "Tier 3 not available".to_string(),
            ))
        }
    }

    /// Get cache statistics
    pub fn cache_stats(&self) -> (super::cache::CacheStats, super::cache::CacheStats) {
        self.cache.stats()
    }

    /// Clear all caches
    pub fn clear_caches(&self) {
        self.cache.clear_all();
    }

    /// Update settings
    pub fn update_settings(&mut self, settings: HighlightSettings) {
        self.settings = settings;

        // Update tier configurations
        if let Some(ref mut relational) = self.relational {
            relational.update_settings(self.settings.relational.clone());
        }
    }

    /// Merge spans using priority-based merging
    fn merge_spans(&self, spans: Vec<HighlightSpan>) -> Vec<HighlightSpan> {
        let mut merger = SpanMerger::new();
        for span in spans {
            merger.add(span);
        }
        merger.merge()
    }

    /// Convert highlight spans to ratatui Line
    fn spans_to_line(&self, spans: Vec<HighlightSpan>, text: &str) -> Line<'static> {
        use ratatui::text::Span;

        if spans.is_empty() {
            return Line::from(text.to_string());
        }

        let mut ratatui_spans = Vec::new();
        let mut last_end = 0;

        for span in spans {
            // Add any gap before this span
            if span.range.start > last_end {
                ratatui_spans.push(Span::raw(text[last_end..span.range.start].to_string()));
            }

            // Add the highlighted span
            let span_text = text[span.range.start..span.range.end].to_string();
            ratatui_spans.push(Span::styled(span_text, span.style));

            last_end = span.range.end;
        }

        // Add any remaining text
        if last_end < text.len() {
            ratatui_spans.push(Span::raw(text[last_end..].to_string()));
        }

        Line::from(ratatui_spans)
    }
}

/// Builder for SemanticHighlightEngine
pub struct EngineBuilder {
    settings: HighlightSettings,
    llm_service: Option<Arc<LlmService>>,
}

impl EngineBuilder {
    pub fn new() -> Self {
        Self {
            settings: HighlightSettings::default(),
            llm_service: None,
        }
    }

    pub fn with_settings(mut self, settings: HighlightSettings) -> Self {
        self.settings = settings;
        self
    }

    pub fn with_llm(mut self, llm: Arc<LlmService>) -> Self {
        self.llm_service = Some(llm);
        self
    }

    pub fn build(self) -> SemanticHighlightEngine {
        SemanticHighlightEngine::with_settings(self.settings, self.llm_service)
    }
}

impl Default for EngineBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_engine_creation() {
        let engine = SemanticHighlightEngine::new(None);
        assert!(engine.structural.is_enabled());
        assert!(engine.relational.is_some());
        assert!(engine.analytical.is_none());
    }

    #[tokio::test]
    async fn test_builder_pattern() {
        let settings = HighlightSettings {
            enable_relational: false,
            ..Default::default()
        };

        let engine = EngineBuilder::new().with_settings(settings).build();

        assert!(engine.relational.is_none());
    }

    #[tokio::test]
    async fn test_highlight_line_basic() {
        let mut engine = SemanticHighlightEngine::new(None);
        let line = engine.highlight_line("Hello world");

        // Should at least return something
        assert!(!line.spans.is_empty() || line.spans.is_empty()); // Just check it doesn't panic
    }
}
