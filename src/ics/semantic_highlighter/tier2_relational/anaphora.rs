//! Anaphora resolution
//!
//! Resolves anaphoric references (pronouns and demonstratives) to their antecedents:
//! - Pronouns: "John arrived. He was early." (he → John)
//! - Demonstratives: "The system failed. This caused issues." (this → failure)
//! - Relative pronouns: "The function which was called" (which → function)
//!
//! Uses distance-based heuristics and grammatical agreement.

use crate::ics::semantic_highlighter::{
    visualization::{Connection, ConnectionType},
    Result,
};
use once_cell::sync::Lazy;
use regex::Regex;
use std::ops::Range;

/// Anaphoric reference
#[derive(Debug, Clone, PartialEq)]
pub struct Anaphor {
    pub range: Range<usize>,
    pub text: String,
    pub anaphor_type: AnaphorType,
}

/// Type of anaphor
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnaphorType {
    /// Personal pronoun (he, she, it, they)
    PersonalPronoun,
    /// Possessive pronoun (his, her, its, their)
    PossessivePronoun,
    /// Demonstrative (this, that, these, those)
    Demonstrative,
    /// Relative pronoun (which, who, that)
    RelativePronoun,
}

/// Antecedent (what the anaphor refers to)
#[derive(Debug, Clone, PartialEq)]
pub struct Antecedent {
    pub range: Range<usize>,
    pub text: String,
    pub confidence: f32,
}

/// Anaphora resolution (anaphor → antecedent link)
#[derive(Debug, Clone)]
pub struct AnaphoraResolution {
    pub anaphor: Anaphor,
    pub antecedent: Antecedent,
}

/// Anaphora resolver
pub struct AnaphoraResolver {
    /// Maximum lookback distance (in characters)
    max_lookback: usize,
    /// Minimum confidence threshold
    threshold: f32,
}

/// Patterns for anaphora detection
struct AnaphoraPatterns {
    personal_pronouns: Regex,
    possessive_pronouns: Regex,
    demonstratives: Regex,
    relative_pronouns: Regex,
    potential_antecedents: Regex,
}

impl AnaphoraPatterns {
    fn new() -> Self {
        Self {
            personal_pronouns: Regex::new(r"\b(he|she|it|they|him|her|them)\b").unwrap(),
            possessive_pronouns: Regex::new(r"\b(his|her|its|their)\b").unwrap(),
            demonstratives: Regex::new(r"\b(this|that|these|those)\b").unwrap(),
            relative_pronouns: Regex::new(r"\b(which|who|whom|whose|that)\b").unwrap(),
            // Capitalized words or "the X" as potential antecedents
            potential_antecedents: Regex::new(
                r"\b(?:([A-Z][a-z]+(?:\s+[A-Z][a-z]+)*)|the\s+([a-z]+))\b",
            )
            .unwrap(),
        }
    }
}

static PATTERNS: Lazy<AnaphoraPatterns> = Lazy::new(AnaphoraPatterns::new);

impl AnaphoraResolver {
    pub fn new() -> Self {
        Self {
            max_lookback: 300, // 300 characters
            threshold: 0.5,
        }
    }

    /// Set maximum lookback distance (minimum 1 to prevent division by zero)
    pub fn with_max_lookback(mut self, distance: usize) -> Self {
        self.max_lookback = distance.max(1); // Ensure at least 1 to prevent NaN
        self
    }

    /// Set confidence threshold
    pub fn with_threshold(mut self, threshold: f32) -> Self {
        self.threshold = threshold.clamp(0.0, 1.0);
        self
    }

    /// Resolve anaphora in text
    pub fn resolve(&self, text: &str) -> Result<Vec<AnaphoraResolution>> {
        let mut resolutions = Vec::new();
        let text_lower = text.to_lowercase();

        // Detect anaphors
        let anaphors = self.detect_anaphors(&text_lower, text)?;

        // For each anaphor, find antecedent
        for anaphor in anaphors {
            if let Some(antecedent) = self.find_antecedent(&anaphor, text)? {
                if antecedent.confidence >= self.threshold {
                    resolutions.push(AnaphoraResolution {
                        anaphor,
                        antecedent,
                    });
                }
            }
        }

        Ok(resolutions)
    }

    /// Convert resolutions to visual connections
    pub fn resolutions_to_connections(
        &self,
        resolutions: &[AnaphoraResolution],
    ) -> Vec<Connection> {
        resolutions
            .iter()
            .map(|res| Connection {
                from: res.anaphor.range.clone(),
                to: res.antecedent.range.clone(),
                connection_type: ConnectionType::Anaphora,
                label: Some(format!("{} → {}", res.anaphor.text, res.antecedent.text)),
                confidence: res.antecedent.confidence,
            })
            .collect()
    }

    /// Detect all anaphors in text
    fn detect_anaphors(&self, text_lower: &str, original: &str) -> Result<Vec<Anaphor>> {
        let mut anaphors = Vec::new();

        // Personal pronouns
        for mat in PATTERNS.personal_pronouns.find_iter(text_lower) {
            anaphors.push(Anaphor {
                range: mat.start()..mat.end(),
                text: original[mat.start()..mat.end()].to_string(),
                anaphor_type: AnaphorType::PersonalPronoun,
            });
        }

        // Possessive pronouns
        for mat in PATTERNS.possessive_pronouns.find_iter(text_lower) {
            anaphors.push(Anaphor {
                range: mat.start()..mat.end(),
                text: original[mat.start()..mat.end()].to_string(),
                anaphor_type: AnaphorType::PossessivePronoun,
            });
        }

        // Demonstratives
        for mat in PATTERNS.demonstratives.find_iter(text_lower) {
            anaphors.push(Anaphor {
                range: mat.start()..mat.end(),
                text: original[mat.start()..mat.end()].to_string(),
                anaphor_type: AnaphorType::Demonstrative,
            });
        }

        // Relative pronouns
        for mat in PATTERNS.relative_pronouns.find_iter(text_lower) {
            anaphors.push(Anaphor {
                range: mat.start()..mat.end(),
                text: original[mat.start()..mat.end()].to_string(),
                anaphor_type: AnaphorType::RelativePronoun,
            });
        }

        // Sort by position
        anaphors.sort_by_key(|a| a.range.start);

        Ok(anaphors)
    }

    /// Find antecedent for an anaphor
    fn find_antecedent(&self, anaphor: &Anaphor, text: &str) -> Result<Option<Antecedent>> {
        let lookback_start = anaphor.range.start.saturating_sub(self.max_lookback);
        let lookback_text = &text[lookback_start..anaphor.range.start];

        // Find potential antecedents in lookback window
        let mut candidates = Vec::new();

        for cap in PATTERNS.potential_antecedents.captures_iter(lookback_text) {
            if let Some(name) = cap.get(1).or_else(|| cap.get(2)) {
                let abs_start = lookback_start + name.start();
                let abs_end = lookback_start + name.end();

                candidates.push(Antecedent {
                    range: abs_start..abs_end,
                    text: text[abs_start..abs_end].to_string(),
                    confidence: 0.0, // Will be scored below
                });
            }
        }

        if candidates.is_empty() {
            return Ok(None);
        }

        // Score candidates
        for candidate in candidates.iter_mut() {
            candidate.confidence = self.score_antecedent(anaphor, candidate, text);
        }

        // Return best candidate (with NaN-safe comparison)
        Ok(candidates
            .into_iter()
            .max_by(|a, b| {
                a.confidence
                    .partial_cmp(&b.confidence)
                    .unwrap_or(std::cmp::Ordering::Equal)
            }))
    }

    /// Score antecedent candidate for an anaphor
    fn score_antecedent(&self, anaphor: &Anaphor, candidate: &Antecedent, _text: &str) -> f32 {
        let mut score = 0.0;

        // Distance penalty (closer is better)
        let distance = anaphor.range.start - candidate.range.end;
        let distance_score = 1.0 - (distance as f32 / self.max_lookback as f32);
        score += distance_score * 0.5;

        // Type agreement
        match anaphor.anaphor_type {
            AnaphorType::PersonalPronoun => {
                // Prefer proper names as antecedents
                if candidate.text.chars().next().unwrap().is_uppercase() {
                    score += 0.4;
                }

                // Number agreement (simplified)
                let is_plural =
                    ["they", "them", "their"].contains(&anaphor.text.to_lowercase().as_str());
                let looks_plural = candidate.text.split_whitespace().count() > 1;

                if is_plural == looks_plural {
                    score += 0.2;
                }
            }

            AnaphorType::PossessivePronoun => {
                // Similar to personal pronouns
                if candidate.text.chars().next().unwrap().is_uppercase() {
                    score += 0.4;
                }
            }

            AnaphorType::Demonstrative => {
                // Demonstratives can refer to concepts or things
                score += 0.3;
            }

            AnaphorType::RelativePronoun => {
                // Relative pronouns usually refer to immediately preceding noun
                if distance < 20 {
                    score += 0.6;
                } else {
                    score += 0.2;
                }
            }
        }

        score.min(1.0)
    }
}

impl Default for AnaphoraResolver {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_personal_pronoun_resolution() {
        let resolver = AnaphoraResolver::new();
        let text = "John arrived early. He was prepared.";
        let resolutions = resolver.resolve(text).unwrap();

        assert!(!resolutions.is_empty());
        let res = &resolutions[0];
        assert_eq!(res.anaphor.text.to_lowercase(), "he");
        assert!(res.antecedent.text.contains("John"));
    }

    #[test]
    fn test_demonstrative_resolution() {
        let resolver = AnaphoraResolver::new();
        let text = "The system failed. This caused problems.";
        let resolutions = resolver.resolve(text).unwrap();

        // Should find "this" referring to something
        let has_this = resolutions
            .iter()
            .any(|r| r.anaphor.text.to_lowercase() == "this");
        assert!(has_this);
    }

    #[test]
    fn test_possessive_pronoun_resolution() {
        let resolver = AnaphoraResolver::new();
        let text = "Alice completed the task. Her work was excellent.";
        let resolutions = resolver.resolve(text).unwrap();

        let has_her = resolutions
            .iter()
            .any(|r| r.anaphor.text.to_lowercase() == "her");
        assert!(has_her);
    }

    #[test]
    fn test_max_lookback() {
        let resolver = AnaphoraResolver::new().with_max_lookback(10);
        let text = "John went to the store which is very far away. He bought milk.";
        let resolutions = resolver.resolve(text).unwrap();

        // With short lookback, may not link "He" to "John"
        // This is a simplified test
        assert!(resolutions.len() <= 2);
    }

    #[test]
    fn test_confidence_threshold() {
        let resolver = AnaphoraResolver::new().with_threshold(0.9);
        let text = "Something happened. It was unexpected.";
        let resolutions = resolver.resolve(text).unwrap();

        // With high threshold, only high-confidence resolutions
        for res in resolutions {
            assert!(res.antecedent.confidence >= 0.9);
        }
    }

    #[test]
    fn test_resolutions_to_connections() {
        let resolver = AnaphoraResolver::new();
        let text = "John arrived. He left.";
        let resolutions = resolver.resolve(text).unwrap();
        let connections = resolver.resolutions_to_connections(&resolutions);

        if !resolutions.is_empty() {
            assert!(!connections.is_empty());
            assert_eq!(connections[0].connection_type, ConnectionType::Anaphora);
        }
    }

    #[test]
    fn test_relative_pronoun() {
        let resolver = AnaphoraResolver::new();
        let text = "The function which processes data is efficient.";
        let resolutions = resolver.resolve(text).unwrap();

        let has_which = resolutions.iter().any(|r| {
            r.anaphor.text.to_lowercase() == "which"
                && r.anaphor.anaphor_type == AnaphorType::RelativePronoun
        });

        assert!(has_which);
    }

    #[test]
    fn test_plural_agreement() {
        let resolver = AnaphoraResolver::new();
        let text = "The developers completed the project. They celebrated.";
        let resolutions = resolver.resolve(text).unwrap();

        // Should prefer "developers" (plural) for "they"
        if let Some(res) = resolutions
            .iter()
            .find(|r| r.anaphor.text.to_lowercase() == "they")
        {
            assert!(res.antecedent.confidence > 0.5);
        }
    }
}
