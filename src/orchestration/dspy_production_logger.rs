//! Production Logger for DSPy Operations
//!
//! Captures DSPy interactions in a structured format suitable for:
//! - Training data collection for future optimization runs
//! - Performance analysis and debugging
//! - Compliance and audit trails
//! - Cost tracking and budgeting
//!
//! # Architecture
//!
//! ```text
//! ProductionLogger → Capture interactions → Write to sink
//!                 ↓
//!            TelemetryCollector (observability)
//! ```
//!
//! # Usage
//!
//! ```rust,no_run
//! use mnemosyne_core::orchestration::dspy_production_logger::{ProductionLogger, LogConfig, LogSink};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Initialize logger with file output
//! let config = LogConfig {
//!     sink: LogSink::File("logs/dspy_production.jsonl".into()),
//!     buffer_size: 100,
//!     enable_telemetry: true,
//! };
//! let logger = ProductionLogger::new(config).await?;
//!
//! // Log interaction
//! let interaction = InteractionLog {
//!     module_name: "reviewer".to_string(),
//!     module_version: ModuleVersion::Optimized("v1".to_string()),
//!     signature: "validate_intent".to_string(),
//!     input: serde_json::json!({"spec": "..."}),
//!     output: serde_json::json!({"satisfied": true}),
//!     timestamp_ms: 1704067200000,
//!     latency_ms: 1250,
//!     tokens: TokenUsage { input_tokens: 450, output_tokens: 120 },
//!     cost_usd: 0.0042,
//!     model: "claude-sonnet-4-20250514".to_string(),
//!     success: true,
//!     error: None,
//! };
//! logger.log_interaction(interaction).await?;
//!
//! // Export training data in DSPy format
//! logger.export_training_data("training_data.jsonl").await?;
//! # Ok(())
//! # }
//! ```

use crate::error::{MnemosyneError, Result};
use crate::orchestration::dspy_module_loader::ModuleVersion;
use crate::orchestration::dspy_telemetry::{DSpyEvent, TelemetryCollector, TokenUsage};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::fs::{File, OpenOptions};
use tokio::io::{AsyncWriteExt, BufWriter};
use tokio::sync::{Mutex, RwLock};
use tracing::{debug, error, info, warn};

/// Log sink destination
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LogSink {
    /// Write to file (JSON Lines format)
    File(PathBuf),
    /// Write to stdout
    Stdout,
    /// Write to database (table name)
    Database(String),
    /// Multiple sinks
    Multiple(Vec<LogSink>),
}

/// Production logger configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogConfig {
    /// Output destination
    pub sink: LogSink,
    /// Buffer size before flushing (number of interactions)
    pub buffer_size: usize,
    /// Enable telemetry collection
    pub enable_telemetry: bool,
}

impl Default for LogConfig {
    fn default() -> Self {
        Self {
            sink: LogSink::File("logs/dspy_production.jsonl".into()),
            buffer_size: 100,
            enable_telemetry: true,
        }
    }
}

/// DSPy interaction log entry
///
/// Captures complete information about a single DSPy module invocation.
/// Format is compatible with DSPy training data requirements.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InteractionLog {
    /// Module name (e.g., "reviewer", "optimizer")
    pub module_name: String,
    /// Module version
    pub module_version: ModuleVersion,
    /// Signature name (e.g., "validate_intent", "extract_requirements")
    pub signature: String,
    /// Input data (JSON)
    pub input: Value,
    /// Output data (JSON)
    pub output: Value,
    /// Timestamp (milliseconds since epoch)
    pub timestamp_ms: u64,
    /// Latency in milliseconds
    pub latency_ms: u64,
    /// Token usage
    pub tokens: TokenUsage,
    /// Cost in USD
    pub cost_usd: f64,
    /// Model used
    pub model: String,
    /// Whether invocation succeeded
    pub success: bool,
    /// Error message if failed
    pub error: Option<String>,
    /// Additional metadata
    #[serde(default)]
    pub metadata: HashMap<String, String>,
}

/// DSPy training data format
///
/// Simplified format for DSPy optimization:
/// {"input": {...}, "output": {...}, "metadata": {...}}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingDataEntry {
    /// Signature name
    pub signature: String,
    /// Input fields
    pub input: Value,
    /// Expected output
    pub output: Value,
    /// Optional metadata
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, String>>,
}

impl From<&InteractionLog> for TrainingDataEntry {
    fn from(log: &InteractionLog) -> Self {
        let mut metadata = log.metadata.clone();
        metadata.insert("module_name".to_string(), log.module_name.clone());
        metadata.insert(
            "module_version".to_string(),
            log.module_version.to_string(),
        );
        metadata.insert("timestamp_ms".to_string(), log.timestamp_ms.to_string());
        metadata.insert("latency_ms".to_string(), log.latency_ms.to_string());
        metadata.insert("model".to_string(), log.model.clone());

        Self {
            signature: log.signature.clone(),
            input: log.input.clone(),
            output: log.output.clone(),
            metadata: Some(metadata),
        }
    }
}

/// File writer with buffering
struct FileWriter {
    writer: BufWriter<File>,
    buffer: Vec<InteractionLog>,
    buffer_size: usize,
}

impl FileWriter {
    async fn new(path: &Path, buffer_size: usize) -> Result<Self> {
        // Create parent directories if needed
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await.map_err(|e| {
                MnemosyneError::Other(format!("Failed to create log directory: {}", e))
            })?;
        }

        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .await
            .map_err(|e| MnemosyneError::Other(format!("Failed to open log file: {}", e)))?;

        let writer = BufWriter::new(file);

        Ok(Self {
            writer,
            buffer: Vec::with_capacity(buffer_size),
            buffer_size,
        })
    }

    async fn write(&mut self, log: InteractionLog) -> Result<()> {
        self.buffer.push(log);

        if self.buffer.len() >= self.buffer_size {
            self.flush().await?;
        }

        Ok(())
    }

    async fn flush(&mut self) -> Result<()> {
        if self.buffer.is_empty() {
            return Ok(());
        }

        for log in &self.buffer {
            let json = serde_json::to_string(log).map_err(|e| {
                MnemosyneError::SerializationError(format!("Failed to serialize log entry: {}", e))
            })?;

            self.writer
                .write_all(json.as_bytes())
                .await
                .map_err(|e| MnemosyneError::Other(format!("Failed to write log: {}", e)))?;

            self.writer
                .write_all(b"\n")
                .await
                .map_err(|e| MnemosyneError::Other(format!("Failed to write newline: {}", e)))?;
        }

        self.writer
            .flush()
            .await
            .map_err(|e| MnemosyneError::Other(format!("Failed to flush buffer: {}", e)))?;

        self.buffer.clear();
        Ok(())
    }
}

/// Production logger for DSPy operations
///
/// Thread-safe logger that captures DSPy interactions in a structured format.
/// Supports multiple output sinks and automatic integration with telemetry.
///
/// # Performance
///
/// - Async, non-blocking operation
/// - Configurable buffering (default: 100 interactions)
/// - Automatic flushing on buffer full or manual flush
/// - Zero-copy when possible
///
/// # Thread Safety
///
/// All operations are thread-safe via `Arc<Mutex<>>` for writers and `Arc<RwLock<>>` for config.
pub struct ProductionLogger {
    config: Arc<RwLock<LogConfig>>,
    file_writer: Arc<Mutex<Option<FileWriter>>>,
    telemetry: Option<Arc<TelemetryCollector>>,
    stats: Arc<RwLock<LoggerStats>>,
}

/// Logger statistics
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct LoggerStats {
    /// Total interactions logged
    pub total_logged: u64,
    /// Total successful interactions
    pub total_success: u64,
    /// Total failed interactions
    pub total_errors: u64,
    /// Total tokens consumed
    pub total_tokens: u64,
    /// Total cost (USD)
    pub total_cost_usd: f64,
}

impl ProductionLogger {
    /// Create a new production logger
    ///
    /// Initializes output sinks and optionally telemetry collector.
    ///
    /// # Errors
    ///
    /// Returns error if sink initialization fails.
    pub async fn new(config: LogConfig) -> Result<Self> {
        info!("Initializing production logger with config: {:?}", config);

        // Initialize file writer if needed
        // Extract file path from sink (handles Multiple case)
        let file_path = match &config.sink {
            LogSink::File(path) => Some(path.clone()),
            LogSink::Multiple(sinks) => {
                // Find first file sink
                sinks.iter().find_map(|sink| {
                    if let LogSink::File(path) = sink {
                        Some(path.clone())
                    } else {
                        None
                    }
                })
            }
            _ => None,
        };

        let file_writer = if let Some(path) = file_path {
            Some(FileWriter::new(&path, config.buffer_size).await?)
        } else {
            None
        };

        // Initialize telemetry if enabled
        let telemetry = if config.enable_telemetry {
            Some(Arc::new(TelemetryCollector::new()))
        } else {
            None
        };

        Ok(Self {
            config: Arc::new(RwLock::new(config)),
            file_writer: Arc::new(Mutex::new(file_writer)),
            telemetry,
            stats: Arc::new(RwLock::new(LoggerStats::default())),
        })
    }

    /// Log a DSPy interaction
    ///
    /// Writes to configured sinks and optionally sends to telemetry collector.
    ///
    /// # Errors
    ///
    /// Returns error if writing to sink fails.
    pub async fn log_interaction(&self, log: InteractionLog) -> Result<()> {
        debug!(
            "Logging interaction: {} {} {} ({}ms)",
            log.module_name, log.module_version, log.signature, log.latency_ms
        );

        // Update statistics
        let mut stats = self.stats.write().await;
        stats.total_logged += 1;
        if log.success {
            stats.total_success += 1;
        } else {
            stats.total_errors += 1;
        }
        stats.total_tokens += (log.tokens.input_tokens + log.tokens.output_tokens) as u64;
        stats.total_cost_usd += log.cost_usd;
        drop(stats);

        // Send to telemetry if enabled
        if let Some(telemetry) = &self.telemetry {
            // Create request event
            let mut event = DSpyEvent::request(
                log.module_name.clone(),
                log.module_version.clone(),
                log.signature.clone(),
            );

            // Convert to error event if needed
            if !log.success {
                event = DSpyEvent::error(
                    &event,
                    log.latency_ms,
                    log.error.clone().unwrap_or_default(),
                );
            }

            telemetry.record(event).await;
        }

        // Write to sink
        let config = self.config.read().await;
        match &config.sink {
            LogSink::File(_) => {
                let mut writer = self.file_writer.lock().await;
                if let Some(writer) = writer.as_mut() {
                    writer.write(log).await?;
                } else {
                    error!("File writer not initialized");
                }
            }
            LogSink::Stdout => {
                let json = serde_json::to_string(&log).map_err(|e| {
                    MnemosyneError::SerializationError(format!("Failed to serialize log entry: {}", e))
                })?;
                println!("{}", json);
            }
            LogSink::Database(table) => {
                warn!("Database sink '{}' not yet implemented", table);
            }
            LogSink::Multiple(sinks) => {
                // Handle each sink directly to avoid recursion
                for sink in sinks {
                    match sink {
                        LogSink::File(_) => {
                            // File sink is handled by file_writer
                            let mut writer = self.file_writer.lock().await;
                            if let Some(writer) = writer.as_mut() {
                                writer.write(log.clone()).await?;
                            }
                        }
                        LogSink::Stdout => {
                            let json = serde_json::to_string(&log).map_err(|e| {
                                MnemosyneError::SerializationError(format!("Failed to serialize log entry: {}", e))
                            })?;
                            println!("{}", json);
                        }
                        LogSink::Database(table) => {
                            warn!("Database sink '{}' not yet implemented", table);
                        }
                        LogSink::Multiple(_) => {
                            warn!("Nested Multiple sinks not supported");
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Manually flush buffered logs
    ///
    /// Forces all buffered logs to be written to sinks.
    ///
    /// # Errors
    ///
    /// Returns error if flushing fails.
    pub async fn flush(&self) -> Result<()> {
        debug!("Flushing production logger");

        let mut writer = self.file_writer.lock().await;
        if let Some(writer) = writer.as_mut() {
            writer.flush().await?;
        }

        Ok(())
    }

    /// Export training data in DSPy-compatible format
    ///
    /// Reads all logged interactions and converts to DSPy training data format.
    /// Output is JSON Lines format with one example per line.
    ///
    /// # Arguments
    ///
    /// * `output_path` - Output file path for training data
    /// * `filter` - Optional filter function to select specific interactions
    ///
    /// # Errors
    ///
    /// Returns error if reading logs or writing training data fails.
    pub async fn export_training_data<P: AsRef<Path>>(
        &self,
        output_path: P,
        filter: Option<Box<dyn Fn(&InteractionLog) -> bool + Send>>,
    ) -> Result<usize> {
        info!(
            "Exporting training data to: {}",
            output_path.as_ref().display()
        );

        // First flush any pending logs
        self.flush().await?;

        let config = self.config.read().await;

        // Get source file path
        let source_path = match &config.sink {
            LogSink::File(path) => path.clone(),
            _ => {
                return Err(MnemosyneError::Other(
                    "Can only export training data from file sink".to_string(),
                ))
            }
        };

        // Read all logs
        let content = tokio::fs::read_to_string(&source_path)
            .await
            .map_err(|e| MnemosyneError::Other(format!("Failed to read log file: {}", e)))?;

        let mut training_entries = Vec::new();

        for line in content.lines() {
            if line.trim().is_empty() {
                continue;
            }

            let log: InteractionLog = serde_json::from_str(line).map_err(|e| {
                MnemosyneError::SerializationError(format!("Failed to parse log entry: {}", e))
            })?;

            // Apply filter if provided
            if let Some(ref f) = filter {
                if !f(&log) {
                    continue;
                }
            }

            // Only include successful interactions in training data
            if log.success {
                let entry = TrainingDataEntry::from(&log);
                training_entries.push(entry);
            }
        }

        // Write training data
        let output_file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(output_path.as_ref())
            .await
            .map_err(|e| MnemosyneError::Other(format!("Failed to create output file: {}", e)))?;

        let mut writer = BufWriter::new(output_file);

        for entry in &training_entries {
            let json = serde_json::to_string(entry).map_err(|e| {
                MnemosyneError::SerializationError(format!("Failed to serialize training entry: {}", e))
            })?;

            writer
                .write_all(json.as_bytes())
                .await
                .map_err(|e| MnemosyneError::Other(format!("Failed to write training data: {}", e)))?;

            writer
                .write_all(b"\n")
                .await
                .map_err(|e| MnemosyneError::Other(format!("Failed to write newline: {}", e)))?;
        }

        writer
            .flush()
            .await
            .map_err(|e| MnemosyneError::Other(format!("Failed to flush output: {}", e)))?;

        let count = training_entries.len();
        info!("Exported {} training examples", count);

        Ok(count)
    }

    /// Get logger statistics
    pub async fn stats(&self) -> LoggerStats {
        self.stats.read().await.clone()
    }

    /// Reset statistics
    pub async fn reset_stats(&self) {
        let mut stats = self.stats.write().await;
        *stats = LoggerStats::default();
    }

    /// Get reference to telemetry collector
    pub fn telemetry(&self) -> Option<&Arc<TelemetryCollector>> {
        self.telemetry.as_ref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[tokio::test]
    async fn test_file_logging() {
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path().to_path_buf();

        let config = LogConfig {
            sink: LogSink::File(path.clone()),
            buffer_size: 2,
            enable_telemetry: false,
        };

        let logger = ProductionLogger::new(config).await.unwrap();

        let log = InteractionLog {
            module_name: "reviewer".to_string(),
            module_version: ModuleVersion::Baseline,
            signature: "validate_intent".to_string(),
            input: serde_json::json!({"spec": "test"}),
            output: serde_json::json!({"satisfied": true}),
            timestamp_ms: 1704067200000,
            latency_ms: 1250,
            tokens: TokenUsage {
                input_tokens: 450,
                output_tokens: 120,
                total_tokens: 570,
            },
            cost_usd: 0.0042,
            model: "claude-sonnet-4".to_string(),
            success: true,
            error: None,
            metadata: HashMap::new(),
        };

        logger.log_interaction(log.clone()).await.unwrap();
        logger.log_interaction(log).await.unwrap();

        logger.flush().await.unwrap();

        let content = tokio::fs::read_to_string(&path).await.unwrap();
        let lines: Vec<&str> = content.lines().collect();
        assert_eq!(lines.len(), 2);
    }

    #[tokio::test]
    async fn test_training_data_export() {
        let temp_log = NamedTempFile::new().unwrap();
        let log_path = temp_log.path().to_path_buf();

        let temp_training = NamedTempFile::new().unwrap();
        let training_path = temp_training.path().to_path_buf();

        let config = LogConfig {
            sink: LogSink::File(log_path.clone()),
            buffer_size: 1,
            enable_telemetry: false,
        };

        let logger = ProductionLogger::new(config).await.unwrap();

        let log = InteractionLog {
            module_name: "reviewer".to_string(),
            module_version: ModuleVersion::Optimized("v1".to_string()),
            signature: "validate_intent".to_string(),
            input: serde_json::json!({"spec": "test"}),
            output: serde_json::json!({"satisfied": true}),
            timestamp_ms: 1704067200000,
            latency_ms: 1250,
            tokens: TokenUsage {
                input_tokens: 450,
                output_tokens: 120,
                total_tokens: 570,
            },
            cost_usd: 0.0042,
            model: "claude-sonnet-4".to_string(),
            success: true,
            error: None,
            metadata: HashMap::new(),
        };

        logger.log_interaction(log).await.unwrap();

        let count = logger.export_training_data(&training_path, None).await.unwrap();
        assert_eq!(count, 1);

        let content = tokio::fs::read_to_string(&training_path).await.unwrap();
        let lines: Vec<&str> = content.lines().collect();
        assert_eq!(lines.len(), 1);

        let entry: TrainingDataEntry = serde_json::from_str(lines[0]).unwrap();
        assert_eq!(entry.signature, "validate_intent");
    }

    #[tokio::test]
    async fn test_statistics() {
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path().to_path_buf();

        let config = LogConfig {
            sink: LogSink::File(path),
            buffer_size: 100,
            enable_telemetry: false,
        };

        let logger = ProductionLogger::new(config).await.unwrap();

        let log = InteractionLog {
            module_name: "reviewer".to_string(),
            module_version: ModuleVersion::Baseline,
            signature: "validate_intent".to_string(),
            input: serde_json::json!({"spec": "test"}),
            output: serde_json::json!({"satisfied": true}),
            timestamp_ms: 1704067200000,
            latency_ms: 1250,
            tokens: TokenUsage {
                input_tokens: 450,
                output_tokens: 120,
                total_tokens: 570,
            },
            cost_usd: 0.0042,
            model: "claude-sonnet-4".to_string(),
            success: true,
            error: None,
            metadata: HashMap::new(),
        };

        logger.log_interaction(log.clone()).await.unwrap();
        logger.log_interaction(log).await.unwrap();

        let stats = logger.stats().await;
        assert_eq!(stats.total_logged, 2);
        assert_eq!(stats.total_success, 2);
        assert_eq!(stats.total_errors, 0);
        assert_eq!(stats.total_tokens, (450 + 120) * 2);
        assert!((stats.total_cost_usd - 0.0084).abs() < 0.0001);
    }
}
