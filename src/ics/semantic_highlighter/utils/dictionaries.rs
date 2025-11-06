//! Dictionary-based word lists for semantic analysis
//!
//! Provides curated word lists for entity recognition, modality detection,
//! and other semantic analysis tasks.

use once_cell::sync::Lazy;
use std::collections::HashSet;

/// Modality and hedging dictionaries
pub struct ModalityDictionaries;

impl ModalityDictionaries {
    /// High certainty markers
    pub fn certain_markers() -> &'static HashSet<&'static str> {
        static SET: Lazy<HashSet<&'static str>> = Lazy::new(|| {
            [
                "definitely",
                "certainly",
                "clearly",
                "obviously",
                "undoubtedly",
                "unquestionably",
                "absolutely",
                "surely",
                "always",
                "never",
                "must",
                "will",
                "cannot",
                "impossible",
                "guaranteed",
                "proven",
                "confirmed",
                "established",
                "evident",
                "indisputable",
            ]
            .iter()
            .copied()
            .collect()
        });
        &SET
    }

    /// Probable/likely markers
    pub fn probable_markers() -> &'static HashSet<&'static str> {
        static SET: Lazy<HashSet<&'static str>> = Lazy::new(|| {
            [
                "probably",
                "likely",
                "presumably",
                "apparently",
                "seemingly",
                "should",
                "would",
                "expected",
                "anticipated",
                "plausible",
                "reasonable",
                "typical",
                "usually",
                "generally",
                "normally",
                "often",
                "frequently",
                "tends to",
                "inclined to",
            ]
            .iter()
            .copied()
            .collect()
        });
        &SET
    }

    /// Uncertain/hedging markers
    pub fn uncertain_markers() -> &'static HashSet<&'static str> {
        static SET: Lazy<HashSet<&'static str>> = Lazy::new(|| {
            [
                "maybe",
                "perhaps",
                "possibly",
                "conceivably",
                "potentially",
                "might",
                "may",
                "could",
                "uncertain",
                "unclear",
                "ambiguous",
                "questionable",
                "doubtful",
                "speculative",
                "hypothetical",
                "unknown",
                "unsure",
                "unconfirmed",
                "debatable",
                "tentative",
                "approximately",
                "roughly",
                "about",
                "around",
                "nearly",
            ]
            .iter()
            .copied()
            .collect()
        });
        &SET
    }

    /// Conditional markers
    pub fn conditional_markers() -> &'static HashSet<&'static str> {
        static SET: Lazy<HashSet<&'static str>> = Lazy::new(|| {
            [
                "if",
                "unless",
                "provided",
                "assuming",
                "suppose",
                "given",
                "when",
                "whenever",
                "in case",
                "should",
                "were",
                "had",
                "assuming that",
                "on condition",
            ]
            .iter()
            .copied()
            .collect()
        });
        &SET
    }
}

/// Entity recognition dictionaries
pub struct EntityDictionaries;

impl EntityDictionaries {
    /// Common person titles and honorifics
    pub fn person_titles() -> &'static HashSet<&'static str> {
        static SET: Lazy<HashSet<&'static str>> = Lazy::new(|| {
            [
                "mr",
                "mrs",
                "ms",
                "miss",
                "dr",
                "prof",
                "professor",
                "sir",
                "madam",
                "lord",
                "lady",
                "captain",
                "colonel",
                "general",
                "admiral",
                "president",
                "senator",
                "governor",
                "judge",
                "justice",
                "reverend",
                "father",
                "brother",
                "sister",
                "rabbi",
                "imam",
                "king",
                "queen",
                "prince",
                "princess",
                "duke",
                "duchess",
                "count",
                "countess",
            ]
            .iter()
            .copied()
            .collect()
        });
        &SET
    }

    /// Organization suffixes and indicators
    pub fn organization_indicators() -> &'static HashSet<&'static str> {
        static SET: Lazy<HashSet<&'static str>> = Lazy::new(|| {
            [
                "inc",
                "corp",
                "corporation",
                "company",
                "co",
                "ltd",
                "limited",
                "llc",
                "llp",
                "plc",
                "gmbh",
                "sa",
                "ag",
                "group",
                "holdings",
                "industries",
                "enterprises",
                "foundation",
                "institute",
                "association",
                "organization",
                "university",
                "college",
                "school",
                "academy",
                "hospital",
                "clinic",
                "laboratory",
                "center",
                "department",
                "ministry",
                "agency",
                "bureau",
                "commission",
                "committee",
                "council",
            ]
            .iter()
            .copied()
            .collect()
        });
        &SET
    }

    /// Location type indicators
    pub fn location_indicators() -> &'static HashSet<&'static str> {
        static SET: Lazy<HashSet<&'static str>> = Lazy::new(|| {
            [
                "city",
                "town",
                "village",
                "county",
                "state",
                "province",
                "region",
                "district",
                "territory",
                "country",
                "nation",
                "continent",
                "island",
                "peninsula",
                "mountain",
                "river",
                "lake",
                "ocean",
                "sea",
                "bay",
                "gulf",
                "strait",
                "street",
                "avenue",
                "road",
                "boulevard",
                "lane",
                "drive",
                "building",
                "tower",
                "center",
                "square",
                "park",
                "garden",
            ]
            .iter()
            .copied()
            .collect()
        });
        &SET
    }

    /// Temporal indicators
    pub fn temporal_indicators() -> &'static HashSet<&'static str> {
        static SET: Lazy<HashSet<&'static str>> = Lazy::new(|| {
            [
                "monday",
                "tuesday",
                "wednesday",
                "thursday",
                "friday",
                "saturday",
                "sunday",
                "january",
                "february",
                "march",
                "april",
                "may",
                "june",
                "july",
                "august",
                "september",
                "october",
                "november",
                "december",
                "morning",
                "afternoon",
                "evening",
                "night",
                "yesterday",
                "today",
                "tomorrow",
                "week",
                "month",
                "year",
                "decade",
                "century",
                "era",
                "spring",
                "summer",
                "autumn",
                "fall",
                "winter",
            ]
            .iter()
            .copied()
            .collect()
        });
        &SET
    }

    /// Common technical/abstract concepts
    pub fn concept_indicators() -> &'static HashSet<&'static str> {
        static SET: Lazy<HashSet<&'static str>> = Lazy::new(|| {
            [
                "algorithm",
                "architecture",
                "framework",
                "pattern",
                "paradigm",
                "methodology",
                "approach",
                "strategy",
                "technique",
                "method",
                "process",
                "procedure",
                "protocol",
                "standard",
                "specification",
                "interface",
                "abstraction",
                "implementation",
                "design",
                "model",
                "theory",
                "principle",
                "concept",
                "notion",
                "idea",
                "system",
                "structure",
                "component",
                "module",
                "service",
            ]
            .iter()
            .copied()
            .collect()
        });
        &SET
    }
}

/// Discourse relation markers
pub struct DiscourseMarkers;

impl DiscourseMarkers {
    /// Elaboration/explanation markers
    pub fn elaboration_markers() -> &'static HashSet<&'static str> {
        static SET: Lazy<HashSet<&'static str>> = Lazy::new(|| {
            [
                "specifically",
                "namely",
                "particularly",
                "especially",
                "for example",
                "for instance",
                "such as",
                "including",
                "like",
                "e.g.",
                "i.e.",
                "that is",
                "in other words",
                "to clarify",
                "to elaborate",
                "in fact",
                "actually",
            ]
            .iter()
            .copied()
            .collect()
        });
        &SET
    }

    /// Contrast/concession markers
    pub fn contrast_markers() -> &'static HashSet<&'static str> {
        static SET: Lazy<HashSet<&'static str>> = Lazy::new(|| {
            [
                "but",
                "however",
                "although",
                "though",
                "yet",
                "nevertheless",
                "nonetheless",
                "despite",
                "in spite of",
                "instead",
                "rather",
                "whereas",
                "while",
                "conversely",
                "on the other hand",
                "in contrast",
                "by contrast",
            ]
            .iter()
            .copied()
            .collect()
        });
        &SET
    }

    /// Causal/result markers
    pub fn causal_markers() -> &'static HashSet<&'static str> {
        static SET: Lazy<HashSet<&'static str>> = Lazy::new(|| {
            [
                "because",
                "since",
                "as",
                "therefore",
                "thus",
                "hence",
                "consequently",
                "so",
                "then",
                "as a result",
                "due to",
                "owing to",
                "thanks to",
                "leads to",
                "causes",
                "results in",
                "triggers",
                "produces",
            ]
            .iter()
            .copied()
            .collect()
        });
        &SET
    }

    /// Temporal sequence markers
    pub fn temporal_markers() -> &'static HashSet<&'static str> {
        static SET: Lazy<HashSet<&'static str>> = Lazy::new(|| {
            [
                "before",
                "after",
                "when",
                "while",
                "during",
                "since",
                "until",
                "then",
                "next",
                "finally",
                "previously",
                "subsequently",
                "meanwhile",
                "simultaneously",
                "first",
                "second",
                "third",
                "last",
                "initially",
            ]
            .iter()
            .copied()
            .collect()
        });
        &SET
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_modality_markers() {
        assert!(ModalityDictionaries::certain_markers().contains("definitely"));
        assert!(ModalityDictionaries::probable_markers().contains("probably"));
        assert!(ModalityDictionaries::uncertain_markers().contains("maybe"));
        assert!(ModalityDictionaries::conditional_markers().contains("if"));
    }

    #[test]
    fn test_entity_indicators() {
        assert!(EntityDictionaries::person_titles().contains("dr"));
        assert!(EntityDictionaries::organization_indicators().contains("inc"));
        assert!(EntityDictionaries::location_indicators().contains("city"));
        assert!(EntityDictionaries::temporal_indicators().contains("monday"));
        assert!(EntityDictionaries::concept_indicators().contains("algorithm"));
    }

    #[test]
    fn test_discourse_markers() {
        assert!(DiscourseMarkers::elaboration_markers().contains("specifically"));
        assert!(DiscourseMarkers::contrast_markers().contains("however"));
        assert!(DiscourseMarkers::causal_markers().contains("because"));
        assert!(DiscourseMarkers::temporal_markers().contains("before"));
    }
}
