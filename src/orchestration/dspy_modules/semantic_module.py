"""DSPy module for Tier 3 semantic analysis.

This module implements systematic prompt optimization for Tier 3 semantic
highlighting operations:

- **Discourse Analysis**: Segment text into discourse units with relations
- **Contradiction Detection**: Identify conflicting statements
- **Pragmatics Extraction**: Extract implied meanings and speech acts

# Architecture

The SemanticModule uses ChainOfThought for all analysis operations, enabling:
- Transparent reasoning about semantic structure
- Systematic optimization via teleprompters
- Batch processing for efficiency
- JSON-structured outputs for type safety

# Usage from Rust

```rust
// Via DSpySemanticBridge
let bridge = DSpySemanticBridge::new()?;

let segments = bridge.analyze_discourse("The system is distributed. This enables scaling.").await?;
// Returns: Vec<DiscourseSegment> with ranges and relations

let contradictions = bridge.detect_contradictions("Auth required. No auth needed.").await?;
// Returns: Vec<Contradiction> with conflicting statement pairs

let pragmatics = bridge.extract_pragmatics("Could you implement that?").await?;
// Returns: Vec<PragmaticElement> with speech acts and implied meanings
```

# Usage from Python

```python
from mnemosyne.orchestration.dspy_modules.semantic_module import SemanticModule

semantic = SemanticModule()

# Discourse analysis
result = semantic.analyze_discourse(
    text="The system is distributed. This enables horizontal scaling."
)
print(result.segments)  # [{'start': 0, 'end': 26, 'relation': 'Background'}, ...]

# Contradiction detection
result = semantic.detect_contradictions(
    text="Authentication is required. No authentication needed."
)
print(result.contradictions)  # [{'statement1': ..., 'statement2': ..., 'type': 'Direct'}]

# Pragmatics extraction
result = semantic.extract_pragmatics(
    text="Could you please implement authentication?"
)
print(result.elements)  # [{'type': 'IndirectSpeech', 'speech_act': 'Request', ...}]
```

# Optimization

This module can be optimized jointly with ReviewerModule using GEPA:

```python
from dspy.teleprompt import MIPROv2

# Combined metric for all three analyzers
def semantic_quality(example, pred, trace=None):
    discourse_score = evaluate_discourse_accuracy(example.discourse, pred.segments)
    contradiction_score = evaluate_contradiction_precision(example.contradictions, pred.contradictions)
    pragmatics_score = evaluate_pragmatics_recall(example.pragmatics, pred.elements)
    return (discourse_score + contradiction_score + pragmatics_score) / 3.0

# Optimize all three jointly
teleprompter = MIPROv2(metric=semantic_quality, num_candidates=10)
optimized_semantic = teleprompter.compile(SemanticModule(), trainset=examples)
```
"""

import dspy
from typing import Optional, List, Dict
import logging

logger = logging.getLogger(__name__)


# Discourse Analysis Signature
class AnalyzeDiscourse(dspy.Signature):
    """Analyze text discourse structure.

    Segment text into discourse units and identify relations between them.
    Discourse relations capture how information flows:
    - Elaboration: B expands on A
    - Contrast: B contrasts with A
    - Cause: A causes B
    - Sequence: B follows A temporally
    - Condition: B occurs if A
    - Background: A provides context for B
    - Summary: A summarizes previous discourse
    - Evaluation: A evaluates previous discourse
    """

    text: str = dspy.InputField(
        desc="Text to analyze for discourse structure"
    )

    segments = dspy.OutputField(
        desc="Discourse segments with fields: start (int), end (int), text (str), relation (str or null), related_to_start (int or null), related_to_end (int or null), confidence (float 0-1). Return as list[dict]."
    )
    coherence_score = dspy.OutputField(
        desc="Overall discourse coherence score (0-1) as float"
    )


# Contradiction Detection Signature
class DetectContradictions(dspy.Signature):
    """Detect contradictions in text.

    Identify pairs of statements that contradict each other.
    Contradiction types:
    - Direct: Explicit contradiction (X vs not-X)
    - Temporal: Contradictory timelines
    - Semantic: Contradictory implications
    - Implication: One implies negation of other
    """

    text: str = dspy.InputField(
        desc="Text to analyze for contradictions"
    )

    contradictions = dspy.OutputField(
        desc="Contradictions with fields: statement1_start (int), statement1_end (int), text1 (str), statement2_start (int), statement2_end (int), text2 (str), type (str: Direct/Temporal/Semantic/Implication), explanation (str), confidence (float 0-1). Return as list[dict]."
    )


# Pragmatics Extraction Signature
class ExtractPragmatics(dspy.Signature):
    """Extract pragmatic elements from text.

    Identify implied meanings, presuppositions, implicatures, and speech acts.
    Pragmatic types:
    - Presupposition: Assumed true by speaker
    - Implicature: Implied but not stated
    - SpeechAct: Performative utterance
    - IndirectSpeech: Indirect request/command

    Speech act types:
    - Assertion: Statement of fact
    - Question: Request for information
    - Command: Direct instruction
    - Promise: Commitment to action
    - Request: Indirect instruction
    - Wish: Desired state
    """

    text: str = dspy.InputField(
        desc="Text to analyze for pragmatic elements"
    )

    elements = dspy.OutputField(
        desc="Pragmatic elements with fields: start (int), end (int), text (str), type (str: Presupposition/Implicature/SpeechAct/IndirectSpeech), speech_act (str or null: Assertion/Question/Command/Promise/Request/Wish), explanation (str), implied_meaning (str or null), confidence (float 0-1). Return as list[dict]."
    )


class SemanticModule(dspy.Module):
    """DSPy module for Tier 3 semantic analysis.

    Implements three analytical operations:
    1. Discourse analysis: Segment and relate discourse units
    2. Contradiction detection: Find conflicting statements
    3. Pragmatics extraction: Identify implied meanings

    All operations use ChainOfThought for transparency and optimization.
    """

    def __init__(self):
        """Initialize Semantic module with ChainOfThought for all operations."""
        super().__init__()

        # Three analytical operations
        self.discourse = dspy.ChainOfThought(AnalyzeDiscourse)
        self.contradictions = dspy.ChainOfThought(DetectContradictions)
        self.pragmatics = dspy.ChainOfThought(ExtractPragmatics)

        logger.info("SemanticModule initialized with ChainOfThought")

    def _parse_json_list(self, text) -> list:
        """Parse DSPy's JSON-formatted string output into Python list.

        DSPy returns JSON strings like:
        '[{"key": "value"}, {"key": "value"}]'

        This converts to actual Python list of dicts.
        """
        if isinstance(text, list):
            return text  # Already a list

        if not isinstance(text, str):
            return []

        # Try to parse as JSON
        import json
        try:
            parsed = json.loads(text)
            if isinstance(parsed, list):
                return parsed
            return [parsed]  # Single dict wrapped in list
        except (json.JSONDecodeError, ValueError):
            logger.warning(f"Failed to parse JSON list: {text[:100]}")
            return []

    def _parse_float(self, value) -> float:
        """Parse DSPy's float output which may be string."""
        if isinstance(value, float):
            return value
        if isinstance(value, int):
            return float(value)
        if isinstance(value, str):
            try:
                return float(value.strip())
            except ValueError:
                return 0.0
        return 0.0

    def forward(self, text: str, operation: str = "all"):
        """Main forward pass - performs semantic analysis.

        Args:
            text: Text to analyze
            operation: Which operation to perform ("discourse", "contradictions", "pragmatics", "all")

        Returns:
            dspy.Prediction with requested analysis results
        """
        if operation == "discourse":
            return self.analyze_discourse(text)
        elif operation == "contradictions":
            return self.detect_contradictions(text)
        elif operation == "pragmatics":
            return self.extract_pragmatics(text)
        elif operation == "all":
            return self.analyze_all(text)
        else:
            raise ValueError(f"Unknown operation: {operation}")

    def analyze_discourse(self, text: str) -> dspy.Prediction:
        """Analyze discourse structure of text.

        Args:
            text: Text to analyze

        Returns:
            Prediction with:
                - segments: List[Dict] of discourse segments
                - coherence_score: float overall coherence
        """
        logger.debug(f"Analyzing discourse structure for {len(text)} chars")

        result = self.discourse(text=text)

        # Parse DSPy's string outputs into proper types
        segments = self._parse_json_list(result.segments)
        coherence_score = self._parse_float(result.coherence_score)

        logger.info(f"Found {len(segments)} discourse segments")
        return dspy.Prediction(
            segments=segments,
            coherence_score=coherence_score,
            **{k: v for k, v in result.items() if k not in ['segments', 'coherence_score']}
        )

    def detect_contradictions(self, text: str) -> dspy.Prediction:
        """Detect contradictions in text.

        Args:
            text: Text to analyze

        Returns:
            Prediction with:
                - contradictions: List[Dict] of contradictions
        """
        logger.debug(f"Detecting contradictions in {len(text)} chars")

        result = self.contradictions(text=text)

        # Parse DSPy's JSON string output into proper list
        contradictions = self._parse_json_list(result.contradictions)

        logger.info(f"Found {len(contradictions)} contradictions")
        return dspy.Prediction(
            contradictions=contradictions,
            **{k: v for k, v in result.items() if k != 'contradictions'}
        )

    def extract_pragmatics(self, text: str) -> dspy.Prediction:
        """Extract pragmatic elements from text.

        Args:
            text: Text to analyze

        Returns:
            Prediction with:
                - elements: List[Dict] of pragmatic elements
        """
        logger.debug(f"Extracting pragmatics from {len(text)} chars")

        result = self.pragmatics(text=text)

        # Parse DSPy's JSON string output into proper list
        elements = self._parse_json_list(result.elements)

        logger.info(f"Found {len(elements)} pragmatic elements")
        return dspy.Prediction(
            elements=elements,
            **{k: v for k, v in result.items() if k != 'elements'}
        )

    def analyze_all(self, text: str) -> dspy.Prediction:
        """Perform all three analyses on text.

        Args:
            text: Text to analyze

        Returns:
            Prediction with all analysis results combined
        """
        logger.info("Performing complete semantic analysis")

        discourse_result = self.analyze_discourse(text)
        contradiction_result = self.detect_contradictions(text)
        pragmatics_result = self.extract_pragmatics(text)

        return dspy.Prediction(
            segments=discourse_result.segments,
            coherence_score=discourse_result.coherence_score,
            contradictions=contradiction_result.contradictions,
            elements=pragmatics_result.elements,
        )
