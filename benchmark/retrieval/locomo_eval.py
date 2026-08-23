#!/usr/bin/env python3
"""Retrieval quality evaluation for Mnemosyne recall.

Evaluates hierarchical vs flat retrieval against a labeled QA dataset,
computing Hit@k and MRR (Mean Reciprocal Rank).

Dataset format (JSONL, one evaluation item per line):

    {"query": "how do we handle caching?", "relevant_ids": ["<memory-uuid>", ...]}
    ...

`relevant_ids` are memory IDs that a correct answer should retrieve. IDs may
also be memory summaries — prefix matching against summary text is used as a
fallback so datasets can be authored without knowing internal UUIDs.

Usage:

    # Build eval set, ingest memories, then:
    python3 locomo_eval.py --db /path/to/mnemosyne.db --dataset eval.jsonl -k 5

    # Compare modes:
    python3 locomo_eval.py --dataset eval.jsonl --mode flat --k 5
    python3 locomo_eval.py --dataset eval.jsonl --mode hierarchical --k 5

Inspired by OpenViking's benchmark harness structure; uses only the public
`mnemosyne recall` CLI so it works against any deployment.
"""

import argparse
import json
import subprocess
import sys
from dataclasses import dataclass, field


@dataclass
class EvalItem:
    query: str
    relevant_ids: list = field(default_factory=list)


def load_dataset(path):
    items = []
    with open(path) as f:
        for line in f:
            line = line.strip()
            if not line or line.startswith("#"):
                continue
            obj = json.loads(line)
            items.append(EvalItem(query=obj["query"], relevant_ids=obj.get("relevant_ids", [])))
    return items


def run_recall(db_path, query, namespace, k, hierarchical):
    cmd = [
        "mnemosyne", "recall",
        "--query", query,
        "--limit", str(k),
        "--format", "json",
    ]
    if db_path:
        cmd += ["--db-path", db_path]
    if namespace:
        cmd += ["--namespace", namespace]
    if hierarchical:
        cmd += ["--hierarchical"]
    result = subprocess.run(cmd, capture_output=True, text=True)
    if result.returncode != 0:
        print(f"recall failed for {query!r}: {result.stderr}", file=sys.stderr)
        return []
    try:
        payload = json.loads(result.stdout)
        return payload.get("results", [])
    except json.JSONDecodeError:
        print(f"could not parse recall output for {query!r}", file=sys.stderr)
        return []


def is_relevant(result, relevant_ids):
    rid = result.get("id", "")
    if rid in relevant_ids:
        return True
    # Fallback: match by summary prefix (for hand-authored datasets)
    summary = result.get("summary", "").strip().lower()
    for target in relevant_ids:
        target_l = target.strip().lower()
        if len(target_l) > 12 and (target_l in summary or summary.startswith(target_l)):
            return True
    return False


def evaluate(items, db_path, namespace, k, hierarchical):
    hits = 0
    reciprocal_ranks = []
    for item in items:
        results = run_recall(db_path, item.query, namespace, k, hierarchical)
        rank = None
        for i, r in enumerate(results, start=1):
            if is_relevant(r, item.relevant_ids):
                rank = i
                break
        if rank is not None:
            hits += 1
            reciprocal_ranks.append(1.0 / rank)
        else:
            reciprocal_ranks.append(0.0)
    n = max(len(items), 1)
    return {
        "mode": "hierarchical" if hierarchical else "flat",
        "k": k,
        "queries": len(items),
        "hit_rate": hits / n,
        "mrr": sum(reciprocal_ranks) / n,
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--dataset", required=True, help="JSONL eval dataset")
    parser.add_argument("--db-path", default=None, help="mnemosyne DB path")
    parser.add_argument("--namespace", default=None, help="namespace filter")
    parser.add_argument("-k", type=int, default=5, help="cutoff rank")
    parser.add_argument("--mode", choices=["flat", "hierarchical", "compare"], default="flat")
    args = parser.parse_args()

    items = load_dataset(args.dataset)
    if not items:
        print("empty dataset", file=sys.stderr)
        sys.exit(1)

    if args.mode == "compare":
        rows = [
            evaluate(items, args.db_path, args.namespace, args.k, False),
            evaluate(items, args.db_path, args.namespace, args.k, True),
        ]
    else:
        rows = [evaluate(items, args.db_path, args.namespace, args.k, args.mode == "hierarchical")]

    print(f"{'mode':<15} {'k':>3} {'queries':>8} {'hit@k':>8} {'mrr':>8}")
    for r in rows:
        print(f"{r['mode']:<15} {r['k']:>3} {r['queries']:>8} {r['hit_rate']:>8.3f} {r['mrr']:>8.3f}")


if __name__ == "__main__":
    main()
