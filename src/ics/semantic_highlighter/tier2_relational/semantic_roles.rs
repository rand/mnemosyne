//! Semantic Role Labeling (SRL)
//!
//! Identifies semantic roles of phrases in sentences:
//! - Agent: Who/what performs the action
//! - Patient: Who/what receives the action
//! - Instrument: Tool/means used
//! - Location: Where the action occurs
//! - Time: When the action occurs
//! - Beneficiary: Who/what benefits
//!
//! Uses pattern-based heuristics for role assignment.

use crate::ics::semantic_highlighter::{
    visualization::{HighlightSpan, HighlightSource},
    Result,
};
use ratatui::style::{Color, Modifier, Style};
use regex::Regex;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::ops::Range;

/// Semantic role type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SemanticRole {
    /// Performer of action
    Agent,
    /// Receiver of action
    Patient,
    /// Tool or means
    Instrument,
    /// Location of action
    Location,
    /// Time of action
    Time,
    /// Beneficiary
    Beneficiary,
}

impl SemanticRole {
    fn color(&self) -> Color {
        match self {
            SemanticRole::Agent => Color::Rgb(255, 140, 0),      // Orange
            SemanticRole::Patient => Color::Rgb(135, 206, 250),  // Sky blue
            SemanticRole::Instrument => Color::Rgb(144, 238, 144), // Light green
            SemanticRole::Location => Color::Rgb(221, 160, 221),  // Plum
            SemanticRole::Time => Color::Rgb(255, 215, 0),        // Gold
            SemanticRole::Beneficiary => Color::Rgb(255, 182, 193), // Light pink
        }
    }

    fn _description(&self) -> &'static str {
        match self {
            SemanticRole::Agent => "Agent (who/what does)",
            SemanticRole::Patient => "Patient (receives action)",
            SemanticRole::Instrument => "Instrument (tool/means)",
            SemanticRole::Location => "Location (where)",
            SemanticRole::Time => "Time (when)",
            SemanticRole::Beneficiary => "Beneficiary (who benefits)",
        }
    }
}

/// Role assignment
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RoleAssignment {
    pub role: SemanticRole,
    pub range: Range<usize>,
    pub text: String,
    pub confidence: f32,
}

/// Semantic role labeler
pub struct SemanticRoleLabeler {
    /// Minimum confidence threshold
    threshold: f32,
}

/// Preposition patterns for role detection
struct RolePatterns {
    /// Agent markers (by)
    agent_prep: Regex,
    /// Instrument markers (with, using)
    instrument_prep: Regex,
    /// Location markers (in, at, on)
    location_prep: Regex,
    /// Time markers (during, when, at)
    time_prep: Regex,
    /// Beneficiary markers (for)
    beneficiary_prep: Regex,
}

impl RolePatterns {
    fn new() -> Self {
        Self {
            agent_prep: Regex::new(r"\bby\s+(\w+(?:\s+\w+)*)").unwrap(),
            instrument_prep: Regex::new(r"\b(?:with|using)\s+(\w+(?:\s+\w+)*)").unwrap(),
            location_prep: Regex::new(r"\b(?:in|at|on)\s+(\w+(?:\s+\w+)*)").unwrap(),
            time_prep: Regex::new(r"\b(?:during|when|at)\s+(\w+(?:\s+\w+)*)").unwrap(),
            beneficiary_prep: Regex::new(r"\bfor\s+(\w+(?:\s+\w+)*)").unwrap(),
        }
    }
}

static PATTERNS: Lazy<RolePatterns> = Lazy::new(RolePatterns::new);

impl SemanticRoleLabeler {
    pub fn new() -> Self {
        Self {
            threshold: 0.5,
        }
    }

    /// Set confidence threshold
    pub fn with_threshold(mut self, threshold: f32) -> Self {
        self.threshold = threshold.clamp(0.0, 1.0);
        self
    }

    /// Label semantic roles in text
    pub fn label(&self, text: &str) -> Result<Vec<RoleAssignment>> {
        let mut roles = Vec::new();

        // Detect agents (passive voice)
        roles.extend(self.detect_agents(text)?);

        // Detect instruments
        roles.extend(self.detect_instruments(text)?);

        // Detect locations
        roles.extend(self.detect_locations(text)?);

        // Detect times
        roles.extend(self.detect_times(text)?);

        // Detect beneficiaries
        roles.extend(self.detect_beneficiaries(text)?);

        // Detect patients and agents in active voice
        roles.extend(self.detect_active_voice_roles(text)?);

        // Filter by confidence
        roles.retain(|r| r.confidence >= self.threshold);

        Ok(roles)
    }

    /// Convert role assignments to highlight spans
    pub fn roles_to_spans(&self, roles: &[RoleAssignment]) -> Vec<HighlightSpan> {
        roles
            .iter()
            .map(|role| {
                let style = Style::default()
                    .fg(role.role.color())
                    .add_modifier(Modifier::UNDERLINED);

                HighlightSpan {
                    range: role.range.clone(),
                    style,
                    source: HighlightSource::Relational,
                    annotation: None,
                    confidence: role.confidence,
                    metadata: None,
                }
            })
            .collect()
    }

    /// Detect agents using "by" preposition
    fn detect_agents(&self, text: &str) -> Result<Vec<RoleAssignment>> {
        let mut roles = Vec::new();

        for cap in PATTERNS.agent_prep.captures_iter(text) {
            if let Some(agent_match) = cap.get(1) {
                roles.push(RoleAssignment {
                    role: SemanticRole::Agent,
                    range: agent_match.start()..agent_match.end(),
                    text: agent_match.as_str().to_string(),
                    confidence: 0.8,
                });
            }
        }

        Ok(roles)
    }

    /// Detect instruments using "with/using"
    fn detect_instruments(&self, text: &str) -> Result<Vec<RoleAssignment>> {
        let mut roles = Vec::new();

        for cap in PATTERNS.instrument_prep.captures_iter(text) {
            if let Some(instrument_match) = cap.get(1) {
                roles.push(RoleAssignment {
                    role: SemanticRole::Instrument,
                    range: instrument_match.start()..instrument_match.end(),
                    text: instrument_match.as_str().to_string(),
                    confidence: 0.7,
                });
            }
        }

        Ok(roles)
    }

    /// Detect locations using prepositions
    fn detect_locations(&self, text: &str) -> Result<Vec<RoleAssignment>> {
        let mut roles = Vec::new();

        for cap in PATTERNS.location_prep.captures_iter(text) {
            if let Some(loc_match) = cap.get(1) {
                roles.push(RoleAssignment {
                    role: SemanticRole::Location,
                    range: loc_match.start()..loc_match.end(),
                    text: loc_match.as_str().to_string(),
                    confidence: 0.6,
                });
            }
        }

        Ok(roles)
    }

    /// Detect temporal roles
    fn detect_times(&self, text: &str) -> Result<Vec<RoleAssignment>> {
        let mut roles = Vec::new();

        for cap in PATTERNS.time_prep.captures_iter(text) {
            if let Some(time_match) = cap.get(1) {
                roles.push(RoleAssignment {
                    role: SemanticRole::Time,
                    range: time_match.start()..time_match.end(),
                    text: time_match.as_str().to_string(),
                    confidence: 0.7,
                });
            }
        }

        Ok(roles)
    }

    /// Detect beneficiaries using "for"
    fn detect_beneficiaries(&self, text: &str) -> Result<Vec<RoleAssignment>> {
        let mut roles = Vec::new();

        for cap in PATTERNS.beneficiary_prep.captures_iter(text) {
            if let Some(benef_match) = cap.get(1) {
                roles.push(RoleAssignment {
                    role: SemanticRole::Beneficiary,
                    range: benef_match.start()..benef_match.end(),
                    text: benef_match.as_str().to_string(),
                    confidence: 0.65,
                });
            }
        }

        Ok(roles)
    }

    /// Detect active voice roles (simplified heuristic)
    fn detect_active_voice_roles(&self, text: &str) -> Result<Vec<RoleAssignment>> {
        let mut roles = Vec::new();

        // Very simple: subject before verb is agent, object after verb is patient
        // This is a placeholder - would be better with proper parsing
        let action_verbs = ["calls", "uses", "creates", "sends", "processes"];

        for verb in action_verbs.iter() {
            if let Some(verb_pos) = text.find(verb) {
                // Subject (agent) is typically the word before the verb
                let before = &text[..verb_pos];
                if let Some(last_word_start) = before.rfind(char::is_whitespace) {
                    let agent_start = last_word_start + 1;
                    roles.push(RoleAssignment {
                        role: SemanticRole::Agent,
                        range: agent_start..verb_pos.saturating_sub(1),
                        text: before[agent_start..].trim().to_string(),
                        confidence: 0.5,
                    });
                }

                // Object (patient) is typically the word after the verb
                let after = &text[verb_pos + verb.len()..];
                if let Some(next_word_end) = after.find(char::is_whitespace) {
                    let patient_start = verb_pos + verb.len() + 1;
                    roles.push(RoleAssignment {
                        role: SemanticRole::Patient,
                        range: patient_start..patient_start + next_word_end,
                        text: after[..next_word_end].trim().to_string(),
                        confidence: 0.5,
                    });
                }
            }
        }

        Ok(roles)
    }
}

impl Default for SemanticRoleLabeler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_detection() {
        let labeler = SemanticRoleLabeler::new();
        let text = "The data was processed by the server";
        let roles = labeler.label(text).unwrap();

        let agents: Vec<_> = roles.iter()
            .filter(|r| r.role == SemanticRole::Agent)
            .collect();

        assert!(!agents.is_empty());
        assert!(agents.iter().any(|r| r.text.contains("server")));
    }

    #[test]
    fn test_instrument_detection() {
        let labeler = SemanticRoleLabeler::new();
        let text = "The task is done using a specialized tool";
        let roles = labeler.label(text).unwrap();

        let instruments: Vec<_> = roles.iter()
            .filter(|r| r.role == SemanticRole::Instrument)
            .collect();

        assert!(!instruments.is_empty());
    }

    #[test]
    fn test_location_detection() {
        let labeler = SemanticRoleLabeler::new();
        let text = "The meeting occurs in the conference room";
        let roles = labeler.label(text).unwrap();

        let locations: Vec<_> = roles.iter()
            .filter(|r| r.role == SemanticRole::Location)
            .collect();

        assert!(!locations.is_empty());
    }

    #[test]
    fn test_time_detection() {
        let labeler = SemanticRoleLabeler::new();
        let text = "The process runs during initialization";
        let roles = labeler.label(text).unwrap();

        let times: Vec<_> = roles.iter()
            .filter(|r| r.role == SemanticRole::Time)
            .collect();

        assert!(!times.is_empty());
    }

    #[test]
    fn test_beneficiary_detection() {
        let labeler = SemanticRoleLabeler::new();
        let text = "The system creates reports for the users";
        let roles = labeler.label(text).unwrap();

        let beneficiaries: Vec<_> = roles.iter()
            .filter(|r| r.role == SemanticRole::Beneficiary)
            .collect();

        assert!(!beneficiaries.is_empty());
    }

    #[test]
    fn test_confidence_threshold() {
        let labeler = SemanticRoleLabeler::new().with_threshold(0.7);
        let text = "The data was processed by the server with a tool";
        let roles = labeler.label(text).unwrap();

        for role in roles {
            assert!(role.confidence >= 0.7);
        }
    }

    #[test]
    fn test_roles_to_spans() {
        let labeler = SemanticRoleLabeler::new();
        let text = "The data was processed by the server";
        let roles = labeler.label(text).unwrap();
        let spans = labeler.roles_to_spans(&roles);

        assert_eq!(spans.len(), roles.len());
        for span in spans {
            assert_eq!(span.source, HighlightSource::Relational);
        }
    }

    #[test]
    fn test_multiple_roles() {
        let labeler = SemanticRoleLabeler::new();
        let text = "The system processes data with tools for users during runtime";
        let roles = labeler.label(text).unwrap();

        // Should detect multiple different role types
        let role_types: std::collections::HashSet<_> = roles.iter()
            .map(|r| r.role)
            .collect();

        assert!(role_types.len() >= 2);
    }
}
