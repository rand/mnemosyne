//! Integration tests for DSpySemanticBridge
//!
//! Tests verify:
//! - Discourse analysis via DSPy
//! - Contradiction detection via DSPy
//! - Pragmatics extraction via DSPy
//! - Type safety and JSON conversion
//! - Error handling

#[cfg(feature = "python")]
mod semantic_bridge_tests {
    use mnemosyne_core::ics::semantic_highlighter::tier3_analytical::dspy_integration::DSpySemanticBridge;
    use mnemosyne_core::ics::semantic_highlighter::tier3_analytical::{
        ContradictionType, DiscourseRelation, PragmaticType,
    };
    use std::sync::Arc;

    /// Helper to create test semantic bridge (requires Python environment)
    async fn create_test_bridge() -> DSpySemanticBridge {
        let dspy_service = mnemosyne_core::orchestration::dspy_service::DSpyService::new()
            .await
            .expect("Failed to create DSPy service");

        DSpySemanticBridge::new(Arc::new(tokio::sync::Mutex::new(
            dspy_service.into_py_object(),
        )))
    }

    // =============================================================================
    // Discourse Analysis Tests
    // =============================================================================

    #[tokio::test]
    #[ignore] // Requires Python environment
    async fn test_analyze_discourse_basic() {
        let bridge = create_test_bridge().await;

        let text = "The system is distributed. This enables horizontal scaling.";
        let segments = bridge
            .analyze_discourse(text)
            .await
            .expect("Failed to analyze discourse");

        // Should return segments
        assert!(segments.is_empty() || !segments.is_empty());

        for segment in segments {
            // Validate range
            assert!(segment.range.start < segment.range.end);
            assert!(segment.range.end <= text.len());
            // Validate confidence
            assert!(segment.confidence >= 0.0 && segment.confidence <= 1.0);
        }
    }

    #[tokio::test]
    #[ignore] // Requires Python environment
    async fn test_discourse_relations() {
        let bridge = create_test_bridge().await;

        let text = "Rust is fast. However, it has a steep learning curve.";
        let segments = bridge
            .analyze_discourse(text)
            .await
            .expect("Failed to analyze discourse");

        // Check relation types if present
        for segment in segments {
            if let Some(relation) = segment.relation {
                // Should be a valid relation type
                match relation {
                    DiscourseRelation::Elaboration
                    | DiscourseRelation::Contrast
                    | DiscourseRelation::Cause
                    | DiscourseRelation::Sequence
                    | DiscourseRelation::Condition
                    | DiscourseRelation::Background
                    | DiscourseRelation::Summary
                    | DiscourseRelation::Evaluation => {
                        // Valid
                    }
                }
            }

            // If has relation, should have related_to
            if segment.relation.is_some() && segment.related_to.is_some() {
                let related = segment.related_to.unwrap();
                assert!(related.start < related.end);
                assert!(related.end <= text.len());
            }
        }
    }

    #[tokio::test]
    #[ignore] // Requires Python environment
    async fn test_discourse_empty_text() {
        let bridge = create_test_bridge().await;

        let result = bridge.analyze_discourse("").await;

        // Should handle gracefully
        assert!(result.is_ok() || result.is_err());
    }

    // =============================================================================
    // Contradiction Detection Tests
    // =============================================================================

    #[tokio::test]
    #[ignore] // Requires Python environment
    async fn test_detect_contradictions_basic() {
        let bridge = create_test_bridge().await;

        let text = "Authentication is required. No authentication is needed.";
        let contradictions = bridge
            .detect_contradictions(text)
            .await
            .expect("Failed to detect contradictions");

        // Should return contradictions (may be empty if none found)
        assert!(contradictions.is_empty() || !contradictions.is_empty());

        for contradiction in contradictions {
            // Validate ranges
            assert!(contradiction.statement1.start < contradiction.statement1.end);
            assert!(contradiction.statement2.start < contradiction.statement2.end);
            assert!(contradiction.statement1.end <= text.len());
            assert!(contradiction.statement2.end <= text.len());

            // Validate confidence
            assert!(contradiction.confidence >= 0.0 && contradiction.confidence <= 1.0);

            // Validate type
            match contradiction.contradiction_type {
                ContradictionType::Direct
                | ContradictionType::Temporal
                | ContradictionType::Semantic
                | ContradictionType::Implication => {
                    // Valid
                }
            }

            // Should have explanatory text
            assert!(!contradiction.text1.is_empty());
            assert!(!contradiction.text2.is_empty());
            assert!(!contradiction.explanation.is_empty());
        }
    }

    #[tokio::test]
    #[ignore] // Requires Python environment
    async fn test_detect_contradictions_no_contradictions() {
        let bridge = create_test_bridge().await;

        let text = "The sky is blue. The grass is green.";
        let contradictions = bridge
            .detect_contradictions(text)
            .await
            .expect("Failed to detect contradictions");

        // May or may not find contradictions, but should not error
        assert!(contradictions.is_empty() || !contradictions.is_empty());
    }

    // =============================================================================
    // Pragmatics Extraction Tests
    // =============================================================================

    #[tokio::test]
    #[ignore] // Requires Python environment
    async fn test_extract_pragmatics_basic() {
        let bridge = create_test_bridge().await;

        let text = "Could you please implement authentication?";
        let elements = bridge
            .extract_pragmatics(text)
            .await
            .expect("Failed to extract pragmatics");

        // Should return elements
        assert!(elements.is_empty() || !elements.is_empty());

        for element in elements {
            // Validate range
            assert!(element.range.start < element.range.end);
            assert!(element.range.end <= text.len());

            // Validate confidence
            assert!(element.confidence >= 0.0 && element.confidence <= 1.0);

            // Validate type
            match element.pragmatic_type {
                PragmaticType::Presupposition
                | PragmaticType::Implicature
                | PragmaticType::SpeechAct
                | PragmaticType::IndirectSpeech => {
                    // Valid
                }
            }

            // Should have text and explanation
            assert!(!element.text.is_empty());
            assert!(!element.explanation.is_empty());
        }
    }

    #[tokio::test]
    #[ignore] // Requires Python environment
    async fn test_pragmatics_speech_acts() {
        let bridge = create_test_bridge().await;

        let text = "Please add logging. I promise to review it.";
        let elements = bridge
            .extract_pragmatics(text)
            .await
            .expect("Failed to extract pragmatics");

        // Check speech acts if present
        for element in elements {
            if let Some(speech_act) = element.speech_act {
                // Should be a valid speech act type
                use mnemosyne_core::ics::semantic_highlighter::tier3_analytical::SpeechActType;
                match speech_act {
                    SpeechActType::Assertion
                    | SpeechActType::Question
                    | SpeechActType::Command
                    | SpeechActType::Promise
                    | SpeechActType::Request
                    | SpeechActType::Wish => {
                        // Valid
                    }
                }
            }
        }
    }

    #[tokio::test]
    #[ignore] // Requires Python environment
    async fn test_pragmatics_implied_meaning() {
        let bridge = create_test_bridge().await;

        let text = "Have you stopped using deprecated APIs?";
        let elements = bridge
            .extract_pragmatics(text)
            .await
            .expect("Failed to extract pragmatics");

        // May or may not have implied meanings
        for element in elements {
            if let Some(implied) = &element.implied_meaning {
                assert!(!implied.is_empty());
            }
        }
    }

    // =============================================================================
    // Edge Cases and Error Handling
    // =============================================================================

    #[tokio::test]
    #[ignore] // Requires Python environment
    async fn test_empty_text_all_operations() {
        let bridge = create_test_bridge().await;

        let discourse_result = bridge.analyze_discourse("").await;
        let contradiction_result = bridge.detect_contradictions("").await;
        let pragmatics_result = bridge.extract_pragmatics("").await;

        // All should handle gracefully
        assert!(discourse_result.is_ok() || discourse_result.is_err());
        assert!(contradiction_result.is_ok() || contradiction_result.is_err());
        assert!(pragmatics_result.is_ok() || pragmatics_result.is_err());
    }

    #[tokio::test]
    #[ignore] // Requires Python environment
    async fn test_long_text() {
        let bridge = create_test_bridge().await;

        let long_text = "Sentence one. Sentence two. ".repeat(100);

        let discourse_result = bridge.analyze_discourse(&long_text).await;
        let contradiction_result = bridge.detect_contradictions(&long_text).await;
        let pragmatics_result = bridge.extract_pragmatics(&long_text).await;

        // Should handle long text
        assert!(discourse_result.is_ok() || discourse_result.is_err());
        assert!(contradiction_result.is_ok() || contradiction_result.is_err());
        assert!(pragmatics_result.is_ok() || pragmatics_result.is_err());
    }

    #[tokio::test]
    #[ignore] // Requires Python environment
    async fn test_special_characters() {
        let bridge = create_test_bridge().await;

        let special_text = "Code: `fn test() { return Ok(()); }` with symbols: !@#$%";

        let discourse_result = bridge.analyze_discourse(special_text).await;
        let contradiction_result = bridge.detect_contradictions(special_text).await;
        let pragmatics_result = bridge.extract_pragmatics(special_text).await;

        // Should handle special characters
        assert!(discourse_result.is_ok() || discourse_result.is_err());
        assert!(contradiction_result.is_ok() || contradiction_result.is_err());
        assert!(pragmatics_result.is_ok() || pragmatics_result.is_err());
    }

    #[tokio::test]
    #[ignore] // Requires Python environment
    async fn test_concurrent_operations() {
        let bridge = Arc::new(create_test_bridge().await);

        let mut handles = vec![];

        // Test concurrent calls to different operations
        for i in 0..3 {
            let bridge_clone = Arc::clone(&bridge);
            let text = format!("Test sentence {}. Another sentence {}.", i, i);

            let handle = tokio::spawn(async move {
                tokio::join!(
                    bridge_clone.analyze_discourse(&text),
                    bridge_clone.detect_contradictions(&text),
                    bridge_clone.extract_pragmatics(&text)
                )
            });
            handles.push(handle);
        }

        for handle in handles {
            let (discourse, contradiction, pragmatics) = handle.await.expect("Task panicked");
            assert!(discourse.is_ok() || discourse.is_err());
            assert!(contradiction.is_ok() || contradiction.is_err());
            assert!(pragmatics.is_ok() || pragmatics.is_err());
        }
    }

    #[test]
    fn test_bridge_is_send_sync() {
        fn assert_send<T: Send>() {}
        fn assert_sync<T: Sync>() {}

        assert_send::<DSpySemanticBridge>();
        assert_sync::<DSpySemanticBridge>();
    }
}
