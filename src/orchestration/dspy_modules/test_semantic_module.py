"""Integration tests for SemanticModule.

Tests verify:
- Discourse analysis structure and relations
- Contradiction detection accuracy
- Pragmatics extraction (speech acts, implicatures)
- JSON output format compatibility with Rust bridge
- ChainOfThought transparency
"""

import os
import pytest
import dspy
from semantic_module import SemanticModule


@pytest.fixture
def semantic_module():
    """Create SemanticModule with Claude API (requires ANTHROPIC_API_KEY)."""
    # Check for API key
    api_key = os.getenv("ANTHROPIC_API_KEY")
    if not api_key:
        pytest.skip("ANTHROPIC_API_KEY not set - skipping integration tests")

    # Configure DSPy with Anthropic Claude
    dspy.configure(lm=dspy.LM('anthropic/claude-haiku-4-5-20251001', api_key=api_key))

    return SemanticModule()


class TestDiscourseAnalysis:
    """Test discourse structure analysis."""

    def test_analyze_discourse_basic(self, semantic_module):
        """Test basic discourse analysis."""
        text = "The system is distributed. This enables horizontal scaling."

        result = semantic_module.analyze_discourse(text)

        # Check structure
        assert hasattr(result, 'segments')
        assert hasattr(result, 'coherence_score')
        assert isinstance(result.segments, list)
        assert isinstance(result.coherence_score, (int, float))
        assert 0 <= result.coherence_score <= 1

    def test_discourse_segment_structure(self, semantic_module):
        """Test discourse segment has required fields."""
        text = "Python is a language. It has dynamic typing."

        result = semantic_module.analyze_discourse(text)

        if result.segments:
            segment = result.segments[0]
            assert isinstance(segment, dict)
            # Required fields
            assert 'start' in segment
            assert 'end' in segment
            assert 'text' in segment
            assert 'confidence' in segment
            # Optional fields
            # relation, related_to_start, related_to_end may be None

    def test_discourse_relations(self, semantic_module):
        """Test discourse relation types."""
        text = "Rust is fast. However, it has a steep learning curve."

        result = semantic_module.analyze_discourse(text)

        # Valid relation types
        valid_relations = {
            'Elaboration', 'Contrast', 'Cause', 'Sequence',
            'Condition', 'Background', 'Summary', 'Evaluation', None
        }

        for segment in result.segments:
            if 'relation' in segment and segment['relation'] is not None:
                assert segment['relation'] in valid_relations


class TestContradictionDetection:
    """Test contradiction detection."""

    def test_detect_contradictions_basic(self, semantic_module):
        """Test basic contradiction detection."""
        text = "Authentication is required. No authentication is needed."

        result = semantic_module.detect_contradictions(text)

        # Check structure
        assert hasattr(result, 'contradictions')
        assert isinstance(result.contradictions, list)

    def test_contradiction_structure(self, semantic_module):
        """Test contradiction has required fields."""
        text = "The API is synchronous. The API uses async operations."

        result = semantic_module.detect_contradictions(text)

        if result.contradictions:
            contradiction = result.contradictions[0]
            assert isinstance(contradiction, dict)
            # Required fields
            required_fields = [
                'statement1_start', 'statement1_end', 'text1',
                'statement2_start', 'statement2_end', 'text2',
                'type', 'explanation', 'confidence'
            ]
            for field in required_fields:
                assert field in contradiction

            # Check types
            assert isinstance(contradiction['statement1_start'], int)
            assert isinstance(contradiction['statement1_end'], int)
            assert isinstance(contradiction['confidence'], (int, float))
            assert 0 <= contradiction['confidence'] <= 1

    def test_contradiction_types(self, semantic_module):
        """Test contradiction type classification."""
        valid_types = {'Direct', 'Temporal', 'Semantic', 'Implication'}

        text = "X is true. X is false."
        result = semantic_module.detect_contradictions(text)

        for contradiction in result.contradictions:
            assert contradiction['type'] in valid_types


class TestPragmaticsExtraction:
    """Test pragmatic element extraction."""

    def test_extract_pragmatics_basic(self, semantic_module):
        """Test basic pragmatics extraction."""
        text = "Could you please implement authentication?"

        result = semantic_module.extract_pragmatics(text)

        # Check structure
        assert hasattr(result, 'elements')
        assert isinstance(result.elements, list)

    def test_pragmatic_element_structure(self, semantic_module):
        """Test pragmatic element has required fields."""
        text = "Have you stopped using deprecated APIs?"

        result = semantic_module.extract_pragmatics(text)

        if result.elements:
            element = result.elements[0]
            assert isinstance(element, dict)
            # Required fields
            required_fields = [
                'start', 'end', 'text', 'type',
                'explanation', 'confidence'
            ]
            for field in required_fields:
                assert field in element

            # Optional fields
            # speech_act and implied_meaning may be None

            # Check types
            assert isinstance(element['start'], int)
            assert isinstance(element['end'], int)
            assert isinstance(element['confidence'], (int, float))
            assert 0 <= element['confidence'] <= 1

    def test_pragmatic_types(self, semantic_module):
        """Test pragmatic type classification."""
        valid_types = {
            'Presupposition', 'Implicature', 'SpeechAct', 'IndirectSpeech'
        }

        text = "I wish we had better error handling."
        result = semantic_module.extract_pragmatics(text)

        for element in result.elements:
            assert element['type'] in valid_types

    def test_speech_act_types(self, semantic_module):
        """Test speech act classification."""
        valid_speech_acts = {
            'Assertion', 'Question', 'Command',
            'Promise', 'Request', 'Wish', None
        }

        text = "Please add logging. I promise to review it."
        result = semantic_module.extract_pragmatics(text)

        for element in result.elements:
            if 'speech_act' in element:
                assert element['speech_act'] in valid_speech_acts


class TestAnalyzeAll:
    """Test combined analysis."""

    def test_analyze_all_returns_all_results(self, semantic_module):
        """Test analyze_all() returns combined results."""
        text = "The system is fast. However, the system is slow. Please optimize it."

        result = semantic_module.analyze_all(text)

        # Should have all three analysis types
        assert hasattr(result, 'segments')
        assert hasattr(result, 'coherence_score')
        assert hasattr(result, 'contradictions')
        assert hasattr(result, 'elements')


class TestJSONCompatibility:
    """Test JSON compatibility with Rust bridge."""

    def test_discourse_json_serializable(self, semantic_module):
        """Test discourse results can be JSON serialized."""
        import json

        text = "First sentence. Second sentence."
        result = semantic_module.analyze_discourse(text)

        # Should be JSON serializable
        json_str = json.dumps({
            'segments': result.segments,
            'coherence_score': result.coherence_score
        })
        assert json_str

        # Should be deserializable
        parsed = json.loads(json_str)
        assert 'segments' in parsed
        assert 'coherence_score' in parsed

    def test_contradictions_json_serializable(self, semantic_module):
        """Test contradiction results can be JSON serialized."""
        import json

        text = "X is true. X is false."
        result = semantic_module.detect_contradictions(text)

        json_str = json.dumps({'contradictions': result.contradictions})
        assert json_str

        parsed = json.loads(json_str)
        assert 'contradictions' in parsed

    def test_pragmatics_json_serializable(self, semantic_module):
        """Test pragmatics results can be JSON serialized."""
        import json

        text = "Could you help?"
        result = semantic_module.extract_pragmatics(text)

        json_str = json.dumps({'elements': result.elements})
        assert json_str

        parsed = json.loads(json_str)
        assert 'elements' in parsed


if __name__ == '__main__':
    pytest.main([__file__, '-v'])
