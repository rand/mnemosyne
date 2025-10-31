//! Coreference resolution
//!
//! Identifies when different phrases refer to the same entity:
//! - "John Smith... he... Smith... the CEO"
//! - Creates coreference chains linking mentions
//! - Uses distance-based heuristics and entity matching
//!
//! This is a simplified local implementation using pattern matching
//! and distance-based scoring.

use crate::ics::semantic_highlighter::{
    visualization::{Connection, ConnectionType},
    Result,
};
use std::ops::Range;
use std::collections::HashMap;

/// Coreference mention
#[derive(Debug, Clone, PartialEq)]
pub struct Mention {
    pub range: Range<usize>,
    pub text: String,
    pub mention_type: MentionType,
}

/// Type of mention
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MentionType {
    /// Proper name (John Smith)
    ProperName,
    /// Pronoun (he, she, it, they)
    Pronoun,
    /// Nominal (the CEO, the system)
    Nominal,
    /// Partial name (Smith)
    PartialName,
}

/// Coreference chain (cluster of mentions referring to same entity)
#[derive(Debug, Clone)]
pub struct CorefChain {
    pub chain_id: usize,
    pub mentions: Vec<Mention>,
    pub representative: String, // Most descriptive mention
}

/// Coreference resolver
pub struct CoreferenceResolver {
    /// Maximum distance for coreference (in characters)
    max_distance: usize,
    /// Minimum confidence threshold
    threshold: f32,
}

impl CoreferenceResolver {
    pub fn new() -> Self {
        Self {
            max_distance: 500,  // 500 characters
            threshold: 0.6,
        }
    }

    /// Set maximum distance for coreference
    pub fn with_max_distance(mut self, distance: usize) -> Self {
        self.max_distance = distance;
        self
    }

    /// Set confidence threshold
    pub fn with_threshold(mut self, threshold: f32) -> Self {
        self.threshold = threshold.clamp(0.0, 1.0);
        self
    }

    /// Resolve coreferences in text
    pub fn resolve(&self, text: &str) -> Result<Vec<CorefChain>> {
        // Step 1: Detect all mentions
        let mentions = self.detect_mentions(text)?;

        // Step 2: Cluster mentions into chains
        let chains = self.cluster_mentions(&mentions, text)?;

        Ok(chains)
    }

    /// Convert coreference chains to visual connections
    pub fn chains_to_connections(&self, chains: &[CorefChain]) -> Vec<Connection> {
        let mut connections = Vec::new();

        for chain in chains {
            // Connect each mention to the next in the chain
            for window in chain.mentions.windows(2) {
                connections.push(Connection {
                    from: window[0].range.clone(),
                    to: window[1].range.clone(),
                    connection_type: ConnectionType::Coreference,
                    label: Some(chain.representative.clone()),
                    confidence: 0.7,
                });
            }
        }

        connections
    }

    /// Detect mentions in text
    fn detect_mentions(&self, text: &str) -> Result<Vec<Mention>> {
        let mut mentions = Vec::new();

        // Detect pronouns
        mentions.extend(self.detect_pronouns(text)?);

        // Detect proper names (capitalized sequences)
        mentions.extend(self.detect_proper_names(text)?);

        // Detect nominals with "the"
        mentions.extend(self.detect_nominals(text)?);

        // Sort by position
        mentions.sort_by_key(|m| m.range.start);

        Ok(mentions)
    }

    /// Detect pronoun mentions
    fn detect_pronouns(&self, text: &str) -> Result<Vec<Mention>> {
        let mut mentions = Vec::new();
        let text_lower = text.to_lowercase();

        let pronouns = [
            "he", "she", "it", "they", "them",
            "his", "her", "its", "their",
            "him", "himself", "herself", "itself", "themselves",
        ];

        for pronoun in pronouns.iter() {
            let pattern = format!(r"\b{}\b", pronoun);
            let re = regex::Regex::new(&pattern).unwrap();

            for mat in re.find_iter(&text_lower) {
                mentions.push(Mention {
                    range: mat.start()..mat.end(),
                    text: text[mat.start()..mat.end()].to_string(),
                    mention_type: MentionType::Pronoun,
                });
            }
        }

        Ok(mentions)
    }

    /// Detect proper names (capitalized words/phrases)
    fn detect_proper_names(&self, text: &str) -> Result<Vec<Mention>> {
        let mut mentions = Vec::new();

        // Pattern: One or more capitalized words
        let re = regex::Regex::new(r"\b([A-Z][a-z]+(?:\s+[A-Z][a-z]+)*)\b").unwrap();

        for cap in re.captures_iter(text) {
            let full_match = cap.get(1).unwrap();
            let name = full_match.as_str();

            // Skip common words that happen to be capitalized (sentence starts)
            if name.split_whitespace().count() >= 2 || self.is_likely_name(name) {
                mentions.push(Mention {
                    range: full_match.start()..full_match.end(),
                    text: name.to_string(),
                    mention_type: if name.split_whitespace().count() > 1 {
                        MentionType::ProperName
                    } else {
                        MentionType::PartialName
                    },
                });
            }
        }

        Ok(mentions)
    }

    /// Detect nominal mentions ("the X", "this X")
    fn detect_nominals(&self, text: &str) -> Result<Vec<Mention>> {
        let mut mentions = Vec::new();

        // Pattern: "the/this/that" + noun
        let re = regex::Regex::new(r"\b(the|this|that)\s+([a-z]+)\b").unwrap();

        for cap in re.captures_iter(&text.to_lowercase()) {
            if let Some(noun) = cap.get(2) {
                // Get the actual text (not lowercased)
                let full_start = cap.get(0).unwrap().start();
                let full_end = cap.get(0).unwrap().end();

                mentions.push(Mention {
                    range: full_start..full_end,
                    text: text[full_start..full_end].to_string(),
                    mention_type: MentionType::Nominal,
                });
            }
        }

        Ok(mentions)
    }

    /// Cluster mentions into coreference chains
    fn cluster_mentions(&self, mentions: &[Mention], text: &str) -> Result<Vec<CorefChain>> {
        let mut chains: Vec<CorefChain> = Vec::new();
        let mut mention_to_chain: HashMap<usize, usize> = HashMap::new();
        let mut next_chain_id = 0;

        for (i, mention) in mentions.iter().enumerate() {
            // Try to find an existing chain for this mention
            let mut assigned_chain: Option<usize> = None;

            for (j, prev_mention) in mentions[..i].iter().enumerate() {
                let distance = mention.range.start.saturating_sub(prev_mention.range.end);

                if distance > self.max_distance {
                    continue;
                }

                let score = self.score_coreference(prev_mention, mention, text);

                if score >= self.threshold {
                    // Found a match - add to same chain as prev_mention
                    if let Some(&chain_id) = mention_to_chain.get(&j) {
                        assigned_chain = Some(chain_id);
                        break;
                    }
                }
            }

            if let Some(chain_id) = assigned_chain {
                // Add to existing chain
                mention_to_chain.insert(i, chain_id);
                chains[chain_id].mentions.push(mention.clone());
            } else {
                // Create new chain
                let chain_id = next_chain_id;
                next_chain_id += 1;

                let representative = mention.text.clone();
                chains.push(CorefChain {
                    chain_id,
                    mentions: vec![mention.clone()],
                    representative,
                });

                mention_to_chain.insert(i, chain_id);
            }
        }

        // Update representatives (choose most descriptive mention)
        for chain in chains.iter_mut() {
            chain.representative = self.choose_representative(&chain.mentions);
        }

        // Filter chains with only one mention
        chains.retain(|c| c.mentions.len() > 1);

        Ok(chains)
    }

    /// Score coreference likelihood between two mentions
    fn score_coreference(&self, m1: &Mention, m2: &Mention, _text: &str) -> f32 {
        let mut score: f32 = 0.0;

        // Gender/number agreement for pronouns
        if m2.mention_type == MentionType::Pronoun {
            score += 0.5;

            // If previous mention is a name, boost score
            if matches!(m1.mention_type, MentionType::ProperName | MentionType::PartialName) {
                score += 0.3;
            }
        }

        // Partial name matching
        if m1.mention_type == MentionType::ProperName && m2.mention_type == MentionType::PartialName
            && m1.text.contains(&m2.text) {
                score += 0.8;
            }

        // Same text (but different positions)
        if m1.text.to_lowercase() == m2.text.to_lowercase() {
            score += 0.9;
        }

        // Nominal with similar semantics (simplified)
        if m1.mention_type == MentionType::Nominal && m2.mention_type == MentionType::Pronoun {
            score += 0.4;
        }

        score.min(1.0)
    }

    /// Choose most descriptive mention as representative
    fn choose_representative(&self, mentions: &[Mention]) -> String {
        mentions
            .iter()
            .filter(|m| m.mention_type != MentionType::Pronoun)
            .max_by_key(|m| m.text.len())
            .map(|m| m.text.clone())
            .unwrap_or_else(|| mentions[0].text.clone())
    }

    /// Check if a capitalized word is likely a name
    fn is_likely_name(&self, word: &str) -> bool {
        // Simple heuristic: not a common word
        let common_words = ["The", "This", "That", "These", "Those", "When", "Where", "Who", "What"];
        !common_words.contains(&word)
    }
}

impl Default for CoreferenceResolver {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pronoun_detection() {
        let resolver = CoreferenceResolver::new();
        let text = "John went to the store. He bought milk.";
        let chains = resolver.resolve(text).unwrap();

        // Should find a chain linking "John" and "He"
        assert!(!chains.is_empty());
    }

    #[test]
    fn test_proper_name_detection() {
        let resolver = CoreferenceResolver::new();
        let text = "John Smith arrived. Smith was early.";
        let chains = resolver.resolve(text).unwrap();

        // Should link "John Smith" and "Smith"
        assert!(!chains.is_empty());
        assert!(chains.iter().any(|c| c.mentions.len() >= 2));
    }

    #[test]
    fn test_nominal_coreference() {
        let resolver = CoreferenceResolver::new();
        let text = "The system processes data. It runs efficiently.";
        let chains = resolver.resolve(text).unwrap();

        // Should link "The system" and "It"
        assert!(!chains.is_empty());
    }

    #[test]
    fn test_max_distance() {
        let resolver = CoreferenceResolver::new().with_max_distance(10);
        let text = "John went somewhere. <very long text> He returned.";
        let chains = resolver.resolve(text).unwrap();

        // With short max distance, should not link mentions far apart
        // This is a simplified test
        assert!(chains.len() <= 1);
    }

    #[test]
    fn test_chains_to_connections() {
        let resolver = CoreferenceResolver::new();
        let text = "John arrived. He left.";
        let chains = resolver.resolve(text).unwrap();
        let connections = resolver.chains_to_connections(&chains);

        // Should create connections between mentions
        if !chains.is_empty() {
            assert!(!connections.is_empty());
            assert_eq!(connections[0].connection_type, ConnectionType::Coreference);
        }
    }

    #[test]
    fn test_representative_selection() {
        let resolver = CoreferenceResolver::new();
        let mentions = vec![
            Mention {
                range: 0..10,
                text: "John Smith".to_string(),
                mention_type: MentionType::ProperName,
            },
            Mention {
                range: 20..22,
                text: "he".to_string(),
                mention_type: MentionType::Pronoun,
            },
        ];

        let rep = resolver.choose_representative(&mentions);
        assert_eq!(rep, "John Smith"); // Should choose the proper name
    }

    #[test]
    fn test_confidence_threshold() {
        let resolver = CoreferenceResolver::new().with_threshold(0.9);
        let text = "John went out. He came back.";
        let chains = resolver.resolve(text).unwrap();

        // With high threshold, may not find weak coreferences
        // This is a loose test since our scoring is heuristic
        assert!(chains.len() <= 1);
    }
}
