#!/usr/bin/env python3
"""Interactive tool for expanding training data for DSPy modules.

Provides templates and guidance for creating new training examples
for each ReviewerModule signature.

Usage:
    python expand_training_data.py --signature validate_completeness --count 30
    python expand_training_data.py --signature all --count 10
"""

import json
import argparse
import sys
from pathlib import Path
from datetime import datetime
from typing import Dict, List, Any

# Define signature schemas
SIGNATURE_SCHEMAS = {
    "extract_requirements": {
        "inputs": ["user_intent", "context"],
        "outputs": ["requirements", "priorities"],
        "description": "Extract structured requirements from user intent and context"
    },
    "validate_intent": {
        "inputs": ["user_intent", "work_item", "implementation", "requirements"],
        "outputs": ["intent_satisfied", "explanation", "missing_aspects"],
        "description": "Validate if implementation satisfies user intent"
    },
    "validate_completeness": {
        "inputs": ["implementation", "requirements"],
        "outputs": ["is_complete", "missing_requirements", "explanation"],
        "description": "Check if implementation covers all requirements"
    },
    "validate_correctness": {
        "inputs": ["implementation", "code_sample", "work_item", "test_results"],
        "outputs": ["is_correct", "issues", "explanation"],
        "description": "Validate implementation correctness"
    },
    "generate_guidance": {
        "inputs": ["work_item", "implementation", "issues"],
        "outputs": ["guidance", "priority", "rationale"],
        "description": "Generate improvement guidance for implementation issues"
    }
}

# Example templates for each signature
EXAMPLE_TEMPLATES = {
    "validate_completeness": {
        "template_positive": {
            "inputs": {
                "implementation": "Implemented {feature} with {components}. Files: {files}. Tests: {tests}.",
                "requirements": [
                    "{requirement_1}",
                    "{requirement_2}",
                    "{requirement_3}"
                ]
            },
            "outputs": {
                "is_complete": True,
                "missing_requirements": [],
                "explanation": "Implementation addresses all {n} requirements comprehensively. {details}"
            },
            "metadata": {
                "source": "synthetic",
                "difficulty": "easy|medium|hard",
                "category": "{category}",
                "notes": "{notes}"
            }
        },
        "template_negative": {
            "inputs": {
                "implementation": "Implemented {feature} with {components}. Files: {files}. Tests: {tests}.",
                "requirements": [
                    "{requirement_1}",
                    "{requirement_2}",
                    "{requirement_3}",
                    "{requirement_4}",
                    "{requirement_5}"
                ]
            },
            "outputs": {
                "is_complete": False,
                "missing_requirements": [
                    "{missing_1} - {reason_1}",
                    "{missing_2} - {reason_2}"
                ],
                "explanation": "Implementation covers {n} of {total} requirements. Missing: {summary}. Impact: {impact}"
            },
            "metadata": {
                "source": "synthetic",
                "difficulty": "easy|medium|hard",
                "category": "{category}",
                "notes": "{notes}"
            }
        },
        "categories": [
            "authentication", "authorization", "api", "database", "caching",
            "testing", "monitoring", "deployment", "security", "performance",
            "ui", "error-handling", "logging", "concurrency", "validation"
        ]
    },
    "validate_correctness": {
        "template_positive": {
            "inputs": {
                "implementation": "{description}",
                "code_sample": "{code}",
                "work_item": "{work_item}",
                "test_results": "All {n} tests passing. Coverage: {coverage}%."
            },
            "outputs": {
                "is_correct": True,
                "issues": [],
                "explanation": "Implementation is correct. {positive_aspects}"
            },
            "metadata": {
                "source": "synthetic",
                "difficulty": "easy|medium|hard",
                "category": "{category}",
                "notes": "{notes}"
            }
        },
        "template_negative": {
            "inputs": {
                "implementation": "{description}",
                "code_sample": "{code}",
                "work_item": "{work_item}",
                "test_results": "{n} tests failing: {failures}"
            },
            "outputs": {
                "is_correct": False,
                "issues": [
                    "{issue_1}",
                    "{issue_2}"
                ],
                "explanation": "Implementation has issues: {summary}"
            },
            "metadata": {
                "source": "synthetic",
                "difficulty": "easy|medium|hard",
                "category": "{category}",
                "notes": "{notes}"
            }
        },
        "categories": ["logic-error", "edge-case", "race-condition", "memory-leak",
                      "security-vulnerability", "performance-issue", "api-contract"]
    },
    "generate_guidance": {
        "template": {
            "inputs": {
                "work_item": "{description}",
                "implementation": "{implementation}",
                "issues": [
                    "{issue_1}",
                    "{issue_2}"
                ]
            },
            "outputs": {
                "guidance": "{specific_actionable_guidance}",
                "priority": "high|medium|low",
                "rationale": "{why_this_matters}"
            },
            "metadata": {
                "source": "synthetic",
                "difficulty": "easy|medium|hard",
                "category": "{category}",
                "notes": "{notes}"
            }
        },
        "categories": ["refactoring", "bugfix", "optimization", "security", "testing"]
    }
}

def load_existing_data(signature: str) -> List[Dict[str, Any]]:
    """Load existing training data for a signature."""
    data_dir = Path(__file__).parent / "training_data"
    json_path = data_dir / f"{signature}.json"

    if not json_path.exists():
        return []

    with open(json_path) as f:
        return json.load(f)

def save_training_data(signature: str, data: List[Dict[str, Any]]):
    """Save training data for a signature."""
    data_dir = Path(__file__).parent / "training_data"
    json_path = data_dir / f"{signature}.json"

    # Backup existing file
    if json_path.exists():
        backup_path = data_dir / f"{signature}.json.backup.{datetime.now().strftime('%Y%m%d_%H%M%S')}"
        json_path.rename(backup_path)
        print(f"Backed up existing data to: {backup_path}")

    with open(json_path, 'w') as f:
        json.dump(data, f, indent=2)

    print(f"Saved {len(data)} examples to: {json_path}")

def print_template(signature: str, template_type: str = None):
    """Print example template for a signature."""
    if signature not in EXAMPLE_TEMPLATES:
        print(f"No template available for signature: {signature}")
        print("Available templates:", list(EXAMPLE_TEMPLATES.keys()))
        return

    schema = SIGNATURE_SCHEMAS[signature]
    templates = EXAMPLE_TEMPLATES[signature]

    print(f"\n{'='*70}")
    print(f"Signature: {signature}")
    print(f"Description: {schema['description']}")
    print(f"Inputs: {', '.join(schema['inputs'])}")
    print(f"Outputs: {', '.join(schema['outputs'])}")
    print(f"{'='*70}\n")

    if "categories" in templates:
        print(f"Suggested categories: {', '.join(templates['categories'])}\n")

    # Print templates
    for key, template in templates.items():
        if key == "categories":
            continue
        if template_type and template_type not in key:
            continue

        print(f"--- {key.upper()} ---")
        print(json.dumps(template, indent=2))
        print()

def interactive_example_creation(signature: str) -> Dict[str, Any]:
    """Interactively create a new training example."""
    schema = SIGNATURE_SCHEMAS[signature]

    print(f"\nCreating new example for: {signature}")
    print(f"Description: {schema['description']}\n")

    example = {"inputs": {}, "outputs": {}, "metadata": {}}

    # Collect inputs
    print("=== INPUTS ===")
    for field in schema['inputs']:
        if field == "requirements" or field == "issues":
            print(f"{field} (JSON list): ", end='')
            value = input()
            try:
                example["inputs"][field] = json.loads(value)
            except:
                # Try splitting by newline or comma
                example["inputs"][field] = [v.strip() for v in value.split('\n') if v.strip()]
        else:
            print(f"{field}: ", end='')
            example["inputs"][field] = input()

    # Collect outputs
    print("\n=== OUTPUTS ===")
    for field in schema['outputs']:
        if field in ["requirements", "missing_requirements", "issues", "missing_aspects", "priorities"]:
            print(f"{field} (JSON list): ", end='')
            value = input()
            try:
                example["outputs"][field] = json.loads(value)
            except:
                example["outputs"][field] = [v.strip() for v in value.split('\n') if v.strip()]
        elif field in ["is_complete", "is_correct", "intent_satisfied"]:
            print(f"{field} (true/false): ", end='')
            value = input().lower()
            example["outputs"][field] = value in ["true", "yes", "1"]
        else:
            print(f"{field}: ", end='')
            example["outputs"][field] = input()

    # Collect metadata
    print("\n=== METADATA ===")
    print("difficulty (easy/medium/hard): ", end='')
    example["metadata"]["difficulty"] = input() or "medium"

    print("category: ", end='')
    example["metadata"]["category"] = input() or "general"

    print("notes: ", end='')
    example["metadata"]["notes"] = input() or ""

    example["metadata"]["source"] = "synthetic"

    return example

def main():
    parser = argparse.ArgumentParser(
        description="Expand training data for DSPy signatures"
    )
    parser.add_argument(
        "--signature",
        type=str,
        required=True,
        choices=list(SIGNATURE_SCHEMAS.keys()) + ["all"],
        help="Signature to expand training data for"
    )
    parser.add_argument(
        "--count",
        type=int,
        default=30,
        help="Number of examples to generate (default: 30)"
    )
    parser.add_argument(
        "--interactive",
        action="store_true",
        help="Interactive mode for creating examples"
    )
    parser.add_argument(
        "--show-template",
        action="store_true",
        help="Show example template and exit"
    )
    parser.add_argument(
        "--template-type",
        choices=["positive", "negative"],
        help="Filter templates by type"
    )

    args = parser.parse_args()

    if args.show_template:
        if args.signature == "all":
            for sig in EXAMPLE_TEMPLATES.keys():
                print_template(sig, args.template_type)
        else:
            print_template(args.signature, args.template_type)
        return 0

    if args.signature == "all":
        print("Error: Cannot use --signature all without --show-template")
        return 1

    # Load existing data
    existing_data = load_existing_data(args.signature)
    print(f"Loaded {len(existing_data)} existing examples")

    if args.interactive:
        new_examples = []
        for i in range(args.count):
            print(f"\n{'='*70}")
            print(f"Example {i+1}/{args.count}")
            print(f"{'='*70}")

            example = interactive_example_creation(args.signature)
            new_examples.append(example)

            print("\n--- Generated Example ---")
            print(json.dumps(example, indent=2))

            print("\nAdd this example? (y/n): ", end='')
            if input().lower() != 'y':
                print("Skipped")
                continue

            if i < args.count - 1:
                print("\nContinue to next example? (y/n): ", end='')
                if input().lower() != 'y':
                    break

        # Save expanded data
        combined_data = existing_data + new_examples
        save_training_data(args.signature, combined_data)
        print(f"\nAdded {len(new_examples)} new examples")
        print(f"Total: {len(combined_data)} examples")

    else:
        # Show templates and guidance
        print_template(args.signature, args.template_type)
        print(f"\nTo create {args.count} new examples, use --interactive flag")
        print("Or manually edit the training_data/{}.json file\n".format(args.signature))

    return 0

if __name__ == "__main__":
    sys.exit(main())
