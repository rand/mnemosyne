-- Version Check Cache
-- Stores cached version check results with 24-hour TTL

CREATE TABLE IF NOT EXISTS version_check_cache (
    tool TEXT PRIMARY KEY NOT NULL,
    latest_version TEXT NOT NULL,
    release_url TEXT NOT NULL,
    checked_at INTEGER NOT NULL,
    last_notified_version TEXT
);

-- Index for querying stale cache entries
CREATE INDEX IF NOT EXISTS idx_version_cache_checked_at ON version_check_cache(checked_at);
