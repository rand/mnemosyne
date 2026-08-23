# Hierarchical Memory & Context Assembly

> Features inspired by the OpenViking context-database design (concept
> reimplementations in Rust; no upstream code was copied — OpenViking is
> AGPLv3, Mnemosyne is MIT).

## Overview

Mnemosyne memories are now organized into a deterministic **topic tree** with
**tiered summaries** (L0/L1/L2), retrieved through a **directory-recursive
hierarchical retriever** and packed into prompts by a **token-budgeted
context assembler**. Sessions can be **committed** through an extraction
pipeline that makes dedup decisions and writes a memory-diff audit log.

```
project:myapp/
├── decisions/
│   ├── caching          # leaf: one memory (L0 abstract + L1 overview + L2 content)
│   └── auth
├── patterns/
│   └── error-handling
└── insights/
    └── performance      # directories carry generated L0/L1 sidecars too
```

## Tiered content (L0/L1/L2)

Every memory exposes three resolutions (`src/hierarchy.rs`):

| Layer | Size limit | Used for |
|---|---|---|
| **L0 Abstract** | ≤256 chars | Vector-style quick relevance checks, launcher context |
| **L1 Overview** | ≤4000 chars | Reranking, navigation, planning |
| **L2 Detail** | full | On-demand loading via `mnemosyne.context` |

Topic paths derive deterministically from namespace + type + first tag:
`project:myapp/decisions/caching`. Directories aggregate child L0 bodies into
their own L0/L1 sidecars bottom-up, using **stable sampling** (max 32 direct
children, deterministically chosen so unchanged trees never produce noisy
rewrites) and carry **freshness metadata** (`total_entries`,
`sampled_entries`, `unsampled_entries`, `pending_child_changes`).

## Hierarchical retrieval

`HierarchicalRetriever` implements OpenViking's algorithm:

1. **Global search**: roots ranked by best descendant score; top-k seed a
   priority queue.
2. **Recursion**: pop best directory, score children,
   `final = α·own + (1−α)·parent_score` (α configurable, default 1.0).
3. **Convergence**: stop when the top-k set is unchanged for 3 rounds.
4. **Trajectory**: every step (global_search / recurse / collect) is recorded
   in a serializable `RetrievalTrajectory` for debugging.

### CLI

```bash
# Flat hybrid search (unchanged default)
mnemosyne recall --query "caching decisions" --format json

# Hierarchical rerank + trajectory trace + token-budgeted assembly
mnemosyne recall --query "caching decisions" \
    --hierarchical --trace --budget-tokens 2000 --format json
```

The JSON payload gains `trajectory` and `assembled_context` fields. With
`--hierarchical`, chit-chat queries skip retrieval entirely (intent analysis).

## Intent analysis

`src/intent.rs` plans typed queries before retrieval: verb-first rewrites for
skill intents, "user preferences" phrasing for memory intents, noun phrases
for resources — and **zero queries for chit-chat**, saving a full retrieval
round-trip on greetings/thanks. Fully heuristic (offline-safe); an LLM planner
can layer on later.

## Context assembler

`src/context_assembler.rs` packs candidates under a token budget:

- Every candidate enters at its shallowest tier first (**breadth-first**),
  then tiers deepen on leftover budget — coverage beats depth on narrow-band
  scores.
- Oversized tiers **fall back shallower instead of truncating mid-sentence**.
- A ledger reports exactly where the budget went.

## MCP tools

- **`mnemosyne.recall`** gains optional `hierarchical` (topic-tree rerank)
  and returns a retrieval `trajectory`.
- **`mnemosyne.used`** — agents report which recalled memories were actually
  helpful (access-count feedback feeding the relevance learner).
- **`mnemosyne.hierarchy`** — browse the topic tree (L0 abstracts, L1
  overviews, freshness) without loading any full content.

## Session commit pipeline

`src/session_extract.rs`:

1. **Sync phase**: `archive_messages()` writes the transcript to JSONL.
2. **Async phase**: heuristic candidate extraction (preferences, decisions),
   similarity pre-filtering, per-item dedup decisions
   (`skip ≥0.92`, `merge ≥0.72`, create otherwise; conservative delete policy
   for subsumed-and-richer replacements), and a **memory diff audit log**
   (`memory_diff_*.json`) recording every add/update/delete for rollback.

## Hotness ranking

`src/utils/hotness.rs`: `sigmoid(log1p(access_count)) × exponential recency
decay` (7-day half-life). Blended into hierarchical reranking as an optional
factor and available as a feature for the online relevance learner.

## Doctor extensions

`mnemosyne doctor` now also checks API-key resolution (env → keyring →
encrypted secrets) and data-directory writability.

## Benchmarks

`benchmark/retrieval/locomo_eval.py` measures Hit@k and MRR via the public CLI
with `--mode compare` (flat vs hierarchical).
