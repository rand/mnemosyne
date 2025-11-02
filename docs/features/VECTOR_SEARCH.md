# Vector Search with Local Embeddings

## Overview

Mnemosyne now supports **local embedding generation** and **vector similarity search** using the fastembed library with ONNX Runtime. This enables semantic search without requiring external API calls (beyond Anthropic for LLM features).

## Architecture

### Components

1. **LocalEmbeddingService** (`src/embeddings/local.rs`)
   - Generates embeddings using locally-run ONNX models
   - Thread-safe with `Arc<Mutex<TextEmbedding>>`
   - Async-friendly via `tokio::spawn_blocking`
   - Auto-downloads models on first use

2. **Storage Integration** (`src/storage/libsql.rs`)
   - Stores embeddings in `memory_vectors` table (sqlite-vec)
   - Auto-generates embeddings on `store_memory()`
   - Vector similarity search with `vec_distance_cosine`

3. **Hybrid Search** (`src/storage/libsql.rs`)
   - Combines 5 scoring signals with configurable weights
   - Graceful degradation when embeddings unavailable

4. **CLI Tools** (`src/main.rs`)
   - `mnemosyne embed` - Generate embeddings for memories
   - `mnemosyne models` - Manage embedding models

## Configuration

### EmbeddingConfig

```rust
use mnemosyne_core::EmbeddingConfig;

let config = EmbeddingConfig {
    enabled: true,
    model: "nomic-embed-text-v1.5".to_string(),
    device: "cpu".to_string(),
    batch_size: 32,
    cache_dir: PathBuf::from("~/.cache/mnemosyne/models"),
    show_download_progress: true,
};
```

**Supported Models:**
- `nomic-embed-text-v1.5` (768 dims, **recommended**)
- `nomic-embed-text-v1` (768 dims)
- `all-MiniLM-L6-v2` (384 dims)
- `all-MiniLM-L12-v2` (384 dims)
- `bge-small-en-v1.5` (384 dims)
- `bge-base-en-v1.5` (768 dims)
- `bge-large-en-v1.5` (1024 dims)

### SearchConfig

```rust
use mnemosyne_core::SearchConfig;

let config = SearchConfig {
    // Scoring weights (sum to 1.0)
    vector_weight: 0.35,      // Vector similarity (primary)
    keyword_weight: 0.30,     // Keyword matching (secondary)
    graph_weight: 0.20,       // Graph connections
    importance_weight: 0.10,  // Importance score
    recency_weight: 0.05,     // Recency bias

    // Feature flags
    enable_vector_search: true,
    enable_graph_expansion: true,
    max_graph_depth: 2,
};
```

## Usage

### Programmatic API

```rust
use mnemosyne_core::{
    LibsqlStorage, LocalEmbeddingService, EmbeddingConfig,
    ConnectionMode, SearchConfig
};
use std::sync::Arc;

// 1. Initialize embedding service
let embedding_config = EmbeddingConfig::default();
let embedding_service = Arc::new(
    LocalEmbeddingService::new(embedding_config).await?
);

// 2. Initialize storage with embedding service
let mut storage = LibsqlStorage::new(
    ConnectionMode::Local("mnemosyne.db".into())
).await?;
storage.set_embedding_service(embedding_service.clone());

// 3. Configure search weights (optional)
let search_config = SearchConfig::default();
storage.set_search_config(search_config);

// 4. Store memories (embeddings auto-generated)
storage.store_memory(&memory).await?;

// 5. Search (hybrid: keyword + vector + graph)
let results = storage.hybrid_search(
    "database decisions",
    Some(namespace),
    10,  // max_results
    true // expand_graph
).await?;
```

### CLI Usage

```bash
# Generate embeddings for all memories
mnemosyne embed --all --progress

# Generate embeddings for specific namespace
mnemosyne embed --namespace "project:myapp" --progress

# Generate embedding for single memory
mnemosyne embed --memory-id "uuid-here"

# List available models
mnemosyne models list

# Check cache info
mnemosyne models info

# Clear model cache
mnemosyne models clear
```

## How Hybrid Search Works

### Scoring Formula

```
final_score =
    vector_weight      * vector_similarity +
    keyword_weight     * keyword_match_score +
    graph_weight       * graph_connection_score +
    importance_weight  * (importance / 10.0) +
    recency_weight     * exp(-age_days / 30.0)
```

**Default Weights (sum to 1.0):**
- Vector: 35% - Semantic similarity via embeddings
- Keyword: 30% - FTS5 full-text search
- Graph: 20% - Link traversal depth
- Importance: 10% - User-assigned importance (1-10)
- Recency: 5% - Exponential decay over 30 days

### Search Flow

1. **Keyword Search** - FTS5 full-text search on content
2. **Vector Search** - Cosine similarity on embeddings (if available)
3. **Graph Expansion** - Traverse links from seed results (if enabled)
4. **Score Combination** - Weighted sum of all signals
5. **Ranking** - Sort by final score, return top N

### Graceful Degradation

Vector search automatically degrades gracefully:
- ✅ Skips if no embedding service configured
- ✅ Skips if query is empty
- ✅ Continues if individual embeddings fail
- ✅ Falls back to keyword-only search when needed

No errors, just reduced functionality.

## Performance

### Model Sizes

| Model                     | Dimensions | Download Size | Speed    |
|---------------------------|------------|---------------|----------|
| nomic-embed-text-v1.5     | 768        | ~140 MB       | Fast     |
| nomic-embed-text-v1       | 768        | ~140 MB       | Fast     |
| all-MiniLM-L6-v2          | 384        | ~80 MB        | Faster   |
| bge-base-en-v1.5          | 768        | ~140 MB       | Fast     |
| bge-large-en-v1.5         | 1024       | ~400 MB       | Slower   |

### Embedding Generation Speed

- Single text: ~50-100ms (after model load)
- Batch (32 texts): ~500ms-1s
- Model load time: ~1-2s (first use only)

### Search Performance

- Keyword search: <10ms (FTS5 is fast)
- Vector search: ~20-50ms (100-1000 embeddings)
- Hybrid search: ~50-100ms total

## Migration

### Database Schema

The `010_update_vector_dimensions.sql` migration updates the vector table:

```sql
-- Drop old table (1536 dims for Voyage AI)
DROP TABLE IF EXISTS memory_vectors;

-- Create new table (768 dims for local models)
CREATE VIRTUAL TABLE IF NOT EXISTS memory_vectors USING vec0(
    memory_id TEXT PRIMARY KEY,
    embedding FLOAT[768]
);
```

**Important:** Existing embeddings are lost and need regeneration:

```bash
# After migration, regenerate all embeddings
mnemosyne embed --all --progress
```

### From Remote to Local Embeddings

If migrating from Voyage AI remote embeddings:

1. **Update dependencies** - Already done (fastembed 5.2)
2. **Run migration** - `010_update_vector_dimensions.sql`
3. **Initialize service** - Use `LocalEmbeddingService::new()`
4. **Regenerate embeddings** - `mnemosyne embed --all`

The system will automatically:
- Use local embeddings for new memories
- Fall back gracefully if embeddings unavailable
- Work without any remote API calls

## Model Cache

### Global Cache Location

Models are cached globally and shared across all projects:

- **Linux/macOS**: `~/.cache/mnemosyne/models/`
- **Windows**: `%LOCALAPPDATA%\mnemosyne\models\`

### Cache Management

```bash
# View cache info
mnemosyne models info

# Output:
# Model cache directory: /home/user/.cache/mnemosyne/models/
#
# Cached models:
#   - models--nomic-ai--nomic-embed-text-v1.5
#
# Total cache size: 147456000 bytes

# Clear cache (free up ~140MB per model)
mnemosyne models clear
```

Models re-download automatically on next use.

## Integration Tests

Run integration tests to verify embeddings work:

```bash
# Run with --test-threads=1 (fastembed concurrency limitation)
cargo test --lib embeddings::local::tests --release -- --test-threads=1

# Tests:
# - test_embed_single_text .......... ok (validates 768 dims)
# - test_embed_batch ................ ok (validates batch processing)
# - test_semantic_similarity ........ ok (validates cosine similarity)
```

## Troubleshooting

### Model Download Fails

**Symptom:** "Failed to load model: Failed to retrieve onnx/model.onnx"

**Solution:**
1. Check internet connection
2. Check disk space (~150MB per model)
3. Verify cache directory writable:
   ```bash
   ls -la ~/.cache/mnemosyne/models/
   ```
4. Clear cache and retry:
   ```bash
   mnemosyne models clear --yes
   ```

### Vector Search Returns No Results

**Symptom:** Hybrid search only returns keyword matches

**Possible Causes:**
1. No embeddings generated yet
   ```bash
   mnemosyne embed --all --progress
   ```

2. Embedding service not initialized
   ```rust
   storage.set_embedding_service(service);
   ```

3. Vector search disabled in config
   ```rust
   config.enable_vector_search = true;
   ```

### Test Failures with Concurrency Errors

**Symptom:** Tests fail with "Failed to retrieve onnx/model.onnx"

**Solution:** Run with `--test-threads=1`:
```bash
cargo test --test-threads=1
```

Fastembed has concurrency limitations during model loading.

## Future Enhancements

Potential improvements (not yet implemented):

1. **GPU Support** - CUDA/Metal acceleration
2. **Quantized Models** - Smaller, faster models
3. **Custom Models** - Load user-provided ONNX models
4. **Incremental Updates** - Update embeddings on edit
5. **Multi-Modal** - Image/audio embeddings
6. **Cross-Encoder Reranking** - Two-stage retrieval

## References

- **fastembed**: https://github.com/Anush008/fastembed-rs
- **sqlite-vec**: https://github.com/asg017/sqlite-vec
- **nomic-embed**: https://huggingface.co/nomic-ai/nomic-embed-text-v1.5
- **ONNX Runtime**: https://onnxruntime.ai/

## Summary

Mnemosyne now has a complete local embedding solution:

✅ No external API calls for embeddings
✅ Global model cache shared across projects
✅ Auto-embedding on memory storage
✅ Hybrid search combining 5 signals
✅ Graceful degradation throughout
✅ CLI tools for management
✅ Well-tested and documented

The system is production-ready for local-first semantic search.
