# OpenViking Review: What Mnemosyne Can Borrow

> Review of [volcengine/OpenViking](https://github.com/volcengine/OpenViking) (v0.3.x, "context database for AI agents")
> against our fork of Mnemosyne. Generated from source inspection of both repos.

## ⚠️ License Constraint (read first)

OpenViking is **AGPLv3**; Mnemosyne is **MIT**. We **cannot copy code** from OpenViking
into Mnemosyne without relicensing the whole project (and AGPL's network clause would apply
to the daemon). Everything below is therefore a **concept/design borrow** — clean-room
reimplementation in Rust from their *documentation* (docs/en/concepts/*.md), not their code.
Ideas and algorithms are not copyrightable; their Python source is.

## What OpenViking Is

A context database where memories, resources, and skills live in one virtual filesystem
under `viking://` URIs. Agents browse context with `ls`/`tree`/`find` instead of querying an
opaque vector store. Core ideas:

1. **L0/L1/L2 tiered content** — every directory gets an abstract (~100 tok) and overview
   (~2k tok) sidecar; full content loads on demand.
2. **Hierarchical retrieval** — vector search locates the best *directory*, then recurses
   down with score propagation, preserving surrounding context.
3. **Session → memory pipeline** — on `session.commit()`, async extraction of preferences /
   agent experience into long-term memory, with LLM-mediated dedup decisions and a
   `memory_diff.json` audit log.
4. **Observable retrieval** — each query preserves its directory-browsing trajectory.

Their LoCoMo benchmark claims 80–83% memory accuracy vs 24–57% for agents' native memory,
with 34–91% token reduction — the mechanisms below are what buy that.

## Gap Analysis

| Capability | OpenViking | Mnemosyne today |
|---|---|---|
| Memory organization | Tree of directories w/ L0/L1 sidecars | Flat `MemoryNote` set per namespace |
| Retrieval | Intent analysis → hierarchical recursion → rerank | Single-query vector + FTS5 + graph expansion |
| Context packing | Token-budgeted breadth-first-then-depth tier assembler | Launcher top-10 by importance, fixed format |
| Query understanding | LLM planner → 0–5 typed queries (skip chit-chat) | Raw query string |
| Session → memory | Two-phase commit, async extraction, dedup decisions, audit diff | `sync()` stores a note; `consolidate()` is manual |
| Usage feedback | `session.used()` records which contexts helped | Implicit signals in evaluation/feedback_collector |
| Retrieval debuggability | Per-query trajectory | Event persistence, but no per-query path trace |
| Benchmarks | LoCoMo, LongMemEval, tau2-bench repro scripts | Custom EVALUATION.md |

## Recommended Borrows (prioritized)

### P0 — Tiered memory representation (L0/L1/L2)

Mnemosyne already has a `summary` field and CONTEXT_LOADING.md defines three *loading*
layers, but memory content itself is single-resolution. Adopt their model:

- **L0**: one-sentence abstract (≤256 chars) — embedded for vector search, used in
  prefetch/launcher context.
- **L1**: structured overview (≤4k chars): key points, when-to-use, links.
- **L2**: full note content, loaded only by `mnemosyne.context` / on demand.

Extend to **directory-level sidecars**: introduce a lightweight topic-tree over namespaces
(e.g. `project:mnemosyne/decisions/`, `/architecture/`, `/patterns/`) with generated L0/L1
for each directory, refreshed bottom-up when children change. This directly upgrades
`src/launcher/context.rs`: instead of "top 10 memories", inject L0 of the namespace root +
L1 of the 2–3 most relevant directories.

**Borrow the freshness metadata** from their OKF sidecar format: `total_entries`,
`sampled_entries`, `pending_child_changes`, plus **stable sampling** (deterministic sample
of children per refresh) so unchanged trees don't produce noisy LLM rewrites and churn.

### P0 — Hierarchical (directory-recursive) retrieval

Replace flat `recall` with their two-stage design:

1. Global vector search over directory L0s to pick starting directories (top-k).
2. Priority-queue recursion into children; blend score:
   `final = α·child_score + (1−α)·parent_score` (α configurable, default 1.0).
3. Convergence: stop when top-k unchanged for 3 rounds; cap parallel child searches.

This preserves *context around hits* (a hit in `decisions/` surfaces its siblings), which
flat per-note search structurally cannot do. Our graph links approximate this but are
LLM-generated and unstructured; the topic tree gives a deterministic backbone.

### P1 — Intent analyzer / typed query planner

Before retrieval, an LLM step (Haiku-class, we already have the LLM service) rewrites the
query into 0–5 typed queries: `{query, context_type, intent, priority}`. Style hints:
verb-first for tasks/skills, noun phrases for references, "user's X" for preferences.
**0 queries → skip retrieval entirely** (chit-chat) — cheap token win for the SessionStart
hook path.

### P1 — Session commit pipeline with dedup decisions + memory diff

Upgrade `MemoryManager::sync()` toward their commit model:

- **Two-phase**: sync write of the archive/summary, then *async* extraction (we have a
  tokio runtime; spawn background task, report via event bus).
- **LLM dedup decisions**: for each candidate memory, vector pre-filter similar existing
  notes, then one LLM call returning per-item `skip / create / merge / delete`. This makes
  `consolidate()` continuous instead of manual.
- **`memory_diff.json`-style audit**: record adds/updates/deletes (before/after) per
  extraction run. We already persist orchestration events — extend to memory mutations for
  rollback support. Complements the existing supersede audit trail.

### P1 — Token-budgeted context assembler

Port the *behavior* of their `context_assembler` (budget.py/tiers.py): place every candidate
at its default tier first, then deepen on leftover budget breadth-first; oversized tiers
**fall back to the previous tier instead of truncating mid-sentence**; keep a token ledger
per section. This makes `prefetch()` and the launcher context strictly better under the same
byte budget, and pairs naturally with L0/L1/L2.

### P2 — Retrieval trajectories (observability)

Record per query: the directories visited, scores at each level, convergence rounds, and
which node produced each final result. Expose via `mnemosyne recall --trace` and the TUI.
We have SSE event broadcasting — add a `retrieval.trajectory` event type. Big debugging win
when "the right memory didn't come back".

### P2 — Hotness blending in ranking

Their lifecycle score: `sigmoid(log1p(active_count)) × exponential_time_decay(updated_at)`
(7-day half-life), blended with similarity. Our evaluation system *learns* weights online —
add hotness as a 14th feature in `src/evaluation/feature_extractor.rs` rather than a
hardcoded blend, keeping the learned-weight architecture.

### P2 — `used()` explicit feedback

Add an MCP tool `mnemosyne.used(memory_ids)` so agents declare which recalled memories were
actually helpful. Feeds the feedback collector with a *stronger* signal than implicit
access/edit events and closes the loop with the online weight learner.

### P3 — `mnemosyne doctor`

Their `doctor` validates config, Python version, provider connectivity, disk space without a
running server. We have `health.rs` internals; surface a one-shot `mnemosyne doctor` CLI:
DB integrity, embedding provider reachability, API key resolution (secrets system), disk,
namespace detection. Great support/UX pattern.

### P3 — Standard benchmarks

Adopt their benchmark targets: run Mnemosyne against **LoCoMo** and **LongMemEval**
(datasets are public) with the same harness structure (benchmark/ dir: per-suite scripts +
result tables). Our EVALUATION.md is currently self-referential; external benchmarks make
claims like "sub-ms retrieval" comparable to the ecosystem.

## What NOT to Borrow

- **The virtual-filesystem-as-database architecture** (`ragfs`, AGFS, C++/Rust hybrid
  crates): a huge surface area we don't need; our LibSQL storage + MCP tools cover the use
  cases with far less complexity.
- **Multi-tenant server / OAuth / API keys**: OpenViking is a shared service; Mnemosyne is
  deliberately local-first and privacy-preserving.
- **Rerank-as-a-service dependency** (Volcengine doubao rerank): optional; our cross-encoder
  or LLM-rerank via existing DSPy modules is more portable.
- **Mooncake/Redis/Yuanrong cache backends**: irrelevant at our scale.

## Suggested Execution Order

1. Topic-tree schema + L0/L1 generation + freshness metadata (P0) — foundation for #2.
2. Hierarchical retrieval with score propagation + convergence (P0).
3. Token-budget assembler rewrite of `prefetch`/launcher context (P1).
4. Session-commit extraction pipeline + dedup decisions + memory diff audit (P1).
5. Intent analyzer / typed queries (P1).
6. Trajectories, hotness feature, `used()` tool (P2).
7. Doctor CLI, LoCoMo/LongMemEval harness (P3).

Items 1–2 are the highest-leverage: they're the core of OpenViking's measured gains
(context-preserving retrieval + tiered loading) and map cleanly onto our existing
storage/evolution/evaluation layers.

## References

- OpenViking docs: `docs/en/concepts/01-architecture.md` … `15-vikingbot.md` (in their repo)
- Key files studied: `openviking/retrieve/hierarchical_retriever.py`,
  `openviking/retrieve/context_assembler/{budget,tiers}.py`,
  `openviking/retrieve/memory_lifecycle.py`, `openviking/session/memory/*`
- Our counterparts: `src/memory_manager.rs`, `src/launcher/context.rs`,
  `src/evaluation/*`, `src/evolution/*`, `CONTEXT_LOADING.md`, `ARCHITECTURE.md`
