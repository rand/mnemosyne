-- Mnemosyne v2.1: Optimizer Evaluation System
-- Version: 2.1.0
-- Created: 2025-10-27
--
-- This migration adds an adaptive evaluation system for the Optimizer agent
-- to learn context relevance over time with privacy-preserving design.
--
-- Key Features:
-- - Highly contextual learning (task_type × work_phase × error_context)
-- - Multi-level aggregation (session → project → global)
-- - Privacy-first: hash-only storage, no raw content
-- - Hierarchical weight lookup with graceful fallback

-- ============================================================================
-- Context Evaluations - Rich Feedback Tracking
-- ============================================================================

CREATE TABLE IF NOT EXISTS context_evaluations (
    -- Identity
    id TEXT PRIMARY KEY NOT NULL,
    session_id TEXT NOT NULL,
    agent_role TEXT NOT NULL CHECK(agent_role IN (
        'orchestrator',
        'optimizer',
        'reviewer',
        'executor'
    )),
    namespace TEXT NOT NULL,

    -- What context was provided
    context_type TEXT NOT NULL CHECK(context_type IN (
        'skill',
        'memory',
        'file',
        'commit',
        'plan'
    )),
    context_id TEXT NOT NULL,  -- skill path, memory ID, file path, etc.

    -- Rich contextual metadata (highly contextual learning)
    task_hash TEXT NOT NULL,  -- SHA256 hash (16 chars) - PRIVACY: no raw description
    task_keywords TEXT,  -- JSON array: generic keywords only
    task_type TEXT CHECK(task_type IN (
        'feature',
        'bugfix',
        'refactor',
        'test',
        'documentation',
        'optimization',
        'exploration'
    )),
    work_phase TEXT CHECK(work_phase IN (
        'planning',
        'implementation',
        'debugging',
        'review',
        'testing',
        'documentation'
    )),
    file_types TEXT,  -- JSON array: ['.rs', '.py', '.md'] - generic patterns only
    error_context TEXT CHECK(error_context IN (
        'compilation',
        'runtime',
        'test_failure',
        'lint',
        'none'
    )),
    related_technologies TEXT,  -- JSON array: ['rust', 'tokio', 'postgres']

    -- Implicit feedback signals
    was_accessed INTEGER NOT NULL DEFAULT 0 CHECK(was_accessed IN (0, 1)),
    access_count INTEGER NOT NULL DEFAULT 0,
    time_to_first_access_ms INTEGER,
    total_time_accessed_ms INTEGER NOT NULL DEFAULT 0,

    -- Explicit feedback signals
    user_rating INTEGER CHECK(user_rating BETWEEN -1 AND 1),  -- -1=not useful, 0=neutral, 1=useful
    was_edited INTEGER NOT NULL DEFAULT 0 CHECK(was_edited IN (0, 1)),
    was_committed INTEGER NOT NULL DEFAULT 0 CHECK(was_committed IN (0, 1)),
    was_cited_in_response INTEGER NOT NULL DEFAULT 0 CHECK(was_cited_in_response IN (0, 1)),

    -- Outcome signals
    task_completed INTEGER NOT NULL DEFAULT 0 CHECK(task_completed IN (0, 1)),
    task_success_score REAL CHECK(task_success_score BETWEEN 0.0 AND 1.0),

    -- Timestamps (Unix epoch)
    context_provided_at INTEGER NOT NULL,
    evaluation_updated_at INTEGER NOT NULL,

    FOREIGN KEY (session_id) REFERENCES agent_sessions(id) ON DELETE CASCADE
);

-- Indexes for efficient queries
CREATE INDEX IF NOT EXISTS idx_eval_session ON context_evaluations(session_id);
CREATE INDEX IF NOT EXISTS idx_eval_type_id ON context_evaluations(context_type, context_id);
CREATE INDEX IF NOT EXISTS idx_eval_namespace ON context_evaluations(namespace, context_provided_at DESC);
CREATE INDEX IF NOT EXISTS idx_eval_context ON context_evaluations(work_phase, task_type, error_context);
CREATE INDEX IF NOT EXISTS idx_eval_accessed ON context_evaluations(was_accessed, context_provided_at DESC);

-- ============================================================================
-- Learned Relevance Weights - Multi-Dimensional Adaptive Weights
-- ============================================================================

CREATE TABLE IF NOT EXISTS learned_relevance_weights (
    -- Identity
    id TEXT PRIMARY KEY NOT NULL,

    -- Scope hierarchy (session < project < global)
    scope TEXT NOT NULL CHECK(scope IN ('session', 'project', 'global')),
    scope_id TEXT NOT NULL,  -- session_id, namespace, or 'global'

    -- What these weights apply to
    context_type TEXT NOT NULL CHECK(context_type IN (
        'skill',
        'memory',
        'file',
        'commit',
        'plan'
    )),
    agent_role TEXT NOT NULL CHECK(agent_role IN (
        'orchestrator',
        'optimizer',
        'reviewer',
        'executor'
    )),

    -- Contextual dimensions (multi-dimensional learning)
    work_phase TEXT CHECK(work_phase IN (
        'planning',
        'implementation',
        'debugging',
        'review',
        'testing',
        'documentation'
    )),
    task_type TEXT CHECK(task_type IN (
        'feature',
        'bugfix',
        'refactor',
        'test',
        'documentation',
        'optimization',
        'exploration'
    )),
    error_context TEXT CHECK(error_context IN (
        'compilation',
        'runtime',
        'test_failure',
        'lint',
        'none'
    )),

    -- Learned weights (JSON object)
    -- Example: {"keyword_match": 0.4, "recency": 0.3, "access_patterns": 0.2, "historical_success": 0.1}
    weights TEXT NOT NULL,

    -- Learning metadata
    sample_count INTEGER NOT NULL DEFAULT 0,  -- Number of evaluations that contributed
    last_updated_at INTEGER NOT NULL,
    confidence REAL NOT NULL DEFAULT 0.5 CHECK(confidence BETWEEN 0.0 AND 1.0),
    learning_rate REAL NOT NULL DEFAULT 0.1,  -- Alpha for exponential weighted average

    -- Performance metrics
    avg_precision REAL,
    avg_recall REAL,
    avg_f1_score REAL,

    -- Ensure unique weight sets per context
    UNIQUE(scope, scope_id, context_type, agent_role, work_phase, task_type, error_context)
);

-- Indexes for hierarchical weight lookup with fallback
CREATE INDEX IF NOT EXISTS idx_weights_scope ON learned_relevance_weights(scope, scope_id);
CREATE INDEX IF NOT EXISTS idx_weights_context ON learned_relevance_weights(context_type, agent_role);
CREATE INDEX IF NOT EXISTS idx_weights_dimensions ON learned_relevance_weights(work_phase, task_type, error_context);
CREATE INDEX IF NOT EXISTS idx_weights_specificity ON learned_relevance_weights(
    scope,
    context_type,
    agent_role,
    work_phase,
    task_type,
    error_context,
    sample_count DESC
);

-- ============================================================================
-- Relevance Features - Privacy-Preserving Feature Extraction
-- ============================================================================

CREATE TABLE IF NOT EXISTS relevance_features (
    -- Identity (one-to-one with context_evaluations)
    evaluation_id TEXT PRIMARY KEY NOT NULL,

    -- Statistical features (PRIVACY: no raw content, only computed metrics)
    keyword_overlap_score REAL NOT NULL,  -- Jaccard similarity
    semantic_similarity REAL,  -- Cosine similarity if embeddings available
    recency_days REAL NOT NULL,
    access_frequency REAL NOT NULL,
    last_used_days_ago REAL,

    -- Contextual features
    work_phase_match INTEGER NOT NULL DEFAULT 0 CHECK(work_phase_match IN (0, 1)),
    task_type_match INTEGER NOT NULL DEFAULT 0 CHECK(task_type_match IN (0, 1)),
    agent_role_affinity REAL,
    namespace_match INTEGER NOT NULL DEFAULT 0 CHECK(namespace_match IN (0, 1)),
    file_type_match INTEGER NOT NULL DEFAULT 0 CHECK(file_type_match IN (0, 1)),

    -- Historical features
    historical_success_rate REAL,  -- Success rate of this context in past
    co_occurrence_score REAL,  -- How often this context appears with others

    -- Ground truth (outcome) - used for learning
    was_useful INTEGER NOT NULL CHECK(was_useful IN (0, 1)),

    FOREIGN KEY (evaluation_id) REFERENCES context_evaluations(id) ON DELETE CASCADE
);

-- Indexes for feature analysis
CREATE INDEX IF NOT EXISTS idx_features_useful ON relevance_features(was_useful);
CREATE INDEX IF NOT EXISTS idx_features_similarity ON relevance_features(semantic_similarity DESC);
CREATE INDEX IF NOT EXISTS idx_features_success ON relevance_features(historical_success_rate DESC);

-- ============================================================================
-- Weight Update History - Audit Trail for Learning
-- ============================================================================

CREATE TABLE IF NOT EXISTS weight_update_history (
    id TEXT PRIMARY KEY NOT NULL,
    weight_id TEXT NOT NULL,

    -- What changed
    old_weights TEXT NOT NULL,  -- JSON
    new_weights TEXT NOT NULL,  -- JSON

    -- Why (what evaluation triggered this)
    triggering_evaluation_id TEXT,

    -- Learning metadata
    sample_count_at_update INTEGER NOT NULL,
    confidence_at_update REAL NOT NULL,
    learning_rate_used REAL NOT NULL,

    -- Timestamps
    updated_at INTEGER NOT NULL,

    FOREIGN KEY (weight_id) REFERENCES learned_relevance_weights(id) ON DELETE CASCADE,
    FOREIGN KEY (triggering_evaluation_id) REFERENCES context_evaluations(id) ON DELETE SET NULL
);

CREATE INDEX IF NOT EXISTS idx_weight_history ON weight_update_history(weight_id, updated_at DESC);

-- ============================================================================
-- Initialize Default Weights
-- ============================================================================

-- Global default weights for each agent role and context type
-- These serve as priors before any learning occurs

-- Optimizer + Skill
INSERT OR IGNORE INTO learned_relevance_weights (
    id, scope, scope_id, context_type, agent_role,
    work_phase, task_type, error_context,
    weights, sample_count, last_updated_at, confidence, learning_rate
) VALUES (
    'global_optimizer_skill_default',
    'global',
    'global',
    'skill',
    'optimizer',
    NULL, NULL, NULL,
    '{"keyword_match": 0.35, "recency": 0.15, "access_patterns": 0.25, "historical_success": 0.15, "file_type_match": 0.10}',
    0,
    unixepoch(),
    0.5,
    0.03  -- Slow learning rate for global
);

-- Optimizer + Memory
INSERT OR IGNORE INTO learned_relevance_weights (
    id, scope, scope_id, context_type, agent_role,
    work_phase, task_type, error_context,
    weights, sample_count, last_updated_at, confidence, learning_rate
) VALUES (
    'global_optimizer_memory_default',
    'global',
    'global',
    'memory',
    'optimizer',
    NULL, NULL, NULL,
    '{"keyword_match": 0.30, "recency": 0.25, "access_patterns": 0.20, "historical_success": 0.15, "semantic_similarity": 0.10}',
    0,
    unixepoch(),
    0.5,
    0.03
);

-- Executor + File
INSERT OR IGNORE INTO learned_relevance_weights (
    id, scope, scope_id, context_type, agent_role,
    work_phase, task_type, error_context,
    weights, sample_count, last_updated_at, confidence, learning_rate
) VALUES (
    'global_executor_file_default',
    'global',
    'global',
    'file',
    'executor',
    NULL, NULL, NULL,
    '{"keyword_match": 0.25, "recency": 0.30, "access_patterns": 0.25, "historical_success": 0.10, "file_type_match": 0.10}',
    0,
    unixepoch(),
    0.5,
    0.03
);

-- ============================================================================
-- Configuration and Metadata
-- ============================================================================

-- Evaluation system configuration
CREATE TABLE IF NOT EXISTS evaluation_config (
    key TEXT PRIMARY KEY NOT NULL,
    value TEXT NOT NULL,
    description TEXT,
    updated_at INTEGER NOT NULL
);

INSERT OR IGNORE INTO evaluation_config (key, value, description, updated_at) VALUES
    ('enabled', 'true', 'Enable evaluation system globally', unixepoch()),
    ('privacy_mode', 'standard', 'Privacy mode: standard or strict', unixepoch()),
    ('retention_days', '90', 'Days to retain raw evaluations before aggregation', unixepoch()),
    ('min_samples_for_confidence', '20', 'Minimum samples needed for high confidence', unixepoch()),
    ('session_learning_rate', '0.3', 'Learning rate for session-level weights', unixepoch()),
    ('project_learning_rate', '0.1', 'Learning rate for project-level weights', unixepoch()),
    ('global_learning_rate', '0.03', 'Learning rate for global-level weights', unixepoch());

-- Update metadata
UPDATE metadata SET value = '9' WHERE key = 'schema_version';
INSERT OR IGNORE INTO metadata (key, value) VALUES ('evaluation_system_enabled', 'true');
INSERT OR IGNORE INTO metadata (key, value) VALUES ('evaluation_system_version', '2.1.0');

-- ============================================================================
-- Views for Analysis and Debugging
-- ============================================================================

-- Context usage summary by type and phase
CREATE VIEW IF NOT EXISTS context_usage_stats AS
SELECT
    context_type,
    work_phase,
    task_type,
    COUNT(*) as total_provided,
    SUM(was_accessed) as total_accessed,
    CAST(SUM(was_accessed) AS REAL) / COUNT(*) as access_rate,
    AVG(CASE WHEN was_accessed = 1 THEN time_to_first_access_ms END) as avg_time_to_access_ms,
    SUM(was_edited) as total_edited,
    SUM(was_committed) as total_committed
FROM context_evaluations
GROUP BY context_type, work_phase, task_type
ORDER BY access_rate DESC;

-- Weight evolution by scope
CREATE VIEW IF NOT EXISTS weight_evolution AS
SELECT
    w.scope,
    w.context_type,
    w.agent_role,
    w.work_phase,
    w.task_type,
    w.sample_count,
    w.confidence,
    w.avg_precision,
    w.avg_recall,
    w.avg_f1_score,
    datetime(w.last_updated_at, 'unixepoch') as last_updated
FROM learned_relevance_weights w
WHERE w.sample_count > 0
ORDER BY w.scope, w.sample_count DESC;

-- Most useful contexts (high precision)
CREATE VIEW IF NOT EXISTS most_useful_contexts AS
SELECT
    ce.context_type,
    ce.context_id,
    ce.work_phase,
    ce.task_type,
    COUNT(*) as times_provided,
    SUM(ce.was_accessed) as times_accessed,
    CAST(SUM(ce.was_accessed) AS REAL) / COUNT(*) as precision,
    AVG(ce.task_success_score) as avg_success_score
FROM context_evaluations ce
GROUP BY ce.context_type, ce.context_id, ce.work_phase, ce.task_type
HAVING times_provided >= 3
ORDER BY precision DESC, times_provided DESC
LIMIT 100;

-- Learning progress by scope
CREATE VIEW IF NOT EXISTS learning_progress AS
SELECT
    scope,
    context_type,
    agent_role,
    COUNT(*) as weight_sets,
    AVG(sample_count) as avg_samples,
    AVG(confidence) as avg_confidence,
    AVG(avg_f1_score) as avg_f1
FROM learned_relevance_weights
WHERE sample_count > 0
GROUP BY scope, context_type, agent_role
ORDER BY scope, avg_f1 DESC;
