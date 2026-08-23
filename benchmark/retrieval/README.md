# Mnemosyne Retrieval Benchmarks

Quality evaluation harness for memory retrieval, structured after
OpenViking's benchmark layout (one directory per suite, reproducible scripts).

## Retrieval suite (`locomo_eval.py`)

Measures **Hit@k** and **MRR** for `mnemosyne recall`, comparing flat hybrid
search against `--hierarchical` topic-tree reranking.

### Dataset format

JSONL, one item per line:

```json
{"query": "how do we handle caching?", "relevant_ids": ["uuid-or-summary", ...]}
```

### Running

```bash
# Ingest memories first, then:
python3 benchmark/retrieval/locomo_eval.py \
    --dataset benchmark/retrieval/eval_set.jsonl \
    --namespace project:myapp \
    -k 5 \
    --mode compare
```

`--mode compare` runs both flat and hierarchical retrieval and prints both
rows so improvements from topic-tree reranking are directly visible.

### Notes

- Uses only the public CLI, so it works against any Mnemosyne deployment.
- `relevant_ids` accept either memory UUIDs or summary-text prefixes for
  hand-authored datasets.
- The name honors the LoCoMo long-conversation memory benchmark; any QA
  dataset in the format above works.
