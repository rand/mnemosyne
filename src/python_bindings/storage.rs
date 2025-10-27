//! PyStorage - Direct Rust storage access from Python.
//!
//! Provides thread-safe, low-latency access to Mnemosyne's storage layer
//! via PyO3. Operations complete in <1ms vs 20-50ms for subprocess calls.

use pyo3::prelude::*;
use pyo3::types::PyDict;
use pyo3::Bound;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::storage::{libsql::{ConnectionMode, LibsqlStorage}, MemorySortOrder, StorageBackend};
use crate::types::{MemoryNote, MemoryId, Namespace};

/// Python wrapper for Mnemosyne storage.
///
/// Thread-safe via Arc<Mutex<LibsqlStorage>>. Async operations are converted
/// to sync via tokio runtime for Python compatibility.
#[pyclass]
pub struct PyStorage {
    inner: Arc<Mutex<LibsqlStorage>>,
    runtime: tokio::runtime::Runtime,
}

#[pymethods]
impl PyStorage {
    /// Create new storage instance.
    ///
    /// Args:
    ///     db_path: Path to LibSQL database (default: ~/.local/share/mnemosyne/mnemosyne.db)
    #[new]
    #[pyo3(signature = (db_path=None))]
    fn new(db_path: Option<String>) -> PyResult<Self> {
        let runtime = tokio::runtime::Runtime::new()
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;

        let db_path = db_path.unwrap_or_else(|| {
            let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
            format!("{}/.local/share/mnemosyne/mnemosyne.db", home)
        });

        let storage = runtime.block_on(async {
            LibsqlStorage::new(ConnectionMode::Local(db_path)).await
        }).map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;

        Ok(PyStorage {
            inner: Arc::new(Mutex::new(storage)),
            runtime,
        })
    }

    /// Store a memory (Python dict format).
    ///
    /// Args:
    ///     memory: Dictionary with keys: content, namespace, importance, memory_type, etc.
    ///
    /// Returns:
    ///     str: Memory ID (UUID)
    fn store(&self, memory: &Bound<'_, PyDict>) -> PyResult<String> {
        let note = self.dict_to_memory_note(memory)?;
        let id = note.id.clone();

        self.runtime.block_on(async {
            let storage = self.inner.lock().await;
            storage.store_memory(&note).await
        }).map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;

        Ok(id.to_string())
    }

    /// Retrieve a memory by ID.
    ///
    /// Args:
    ///     id: Memory ID (UUID string)
    ///
    /// Returns:
    ///     dict: Memory as Python dictionary, or None if not found
    fn get(&self, id: String) -> PyResult<Option<PyObject>> {
        let memory_id = MemoryId::from_string(&id)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))?;

        let result = self.runtime.block_on(async {
            let storage = self.inner.lock().await;
            storage.get_memory(memory_id).await
        });

        Python::with_gil(|py| {
            match result {
                Ok(note) => Ok(Some(self.memory_note_to_dict(py, &note)?)),
                Err(_) => Ok(None), // Not found or error
            }
        })
    }

    /// Search memories.
    ///
    /// Args:
    ///     query: Search query string
    ///     namespace: Optional namespace filter (e.g., "project:mnemosyne", "session:abc123")
    ///     limit: Maximum results (default: 10)
    ///
    /// Returns:
    ///     list[dict]: List of matching memories as dictionaries
    #[pyo3(signature = (query, namespace=None, limit=None))]
    fn search(&self, query: String, namespace: Option<String>, limit: Option<usize>) -> PyResult<Vec<PyObject>> {
        let ns = namespace.as_ref().map(|s| parse_namespace(s))
            .transpose()
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e))?;

        let results = self.runtime.block_on(async {
            let storage = self.inner.lock().await;
            storage.hybrid_search(&query, ns, limit.unwrap_or(10), true).await
        }).map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;

        Python::with_gil(|py| {
            results.iter()
                .map(|r| self.memory_note_to_dict(py, &r.memory))
                .collect::<PyResult<Vec<_>>>()
        })
    }

    /// List recent memories.
    ///
    /// Args:
    ///     namespace: Optional namespace filter
    ///     limit: Maximum results (default: 20)
    ///
    /// Returns:
    ///     list[dict]: List of recent memories
    #[pyo3(signature = (namespace=None, limit=None))]
    fn list_recent(&self, namespace: Option<String>, limit: Option<usize>) -> PyResult<Vec<PyObject>> {
        let ns = namespace.as_ref().map(|s| parse_namespace(s))
            .transpose()
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e))?;

        let results = self.runtime.block_on(async {
            let storage = self.inner.lock().await;
            storage.list_memories(ns, limit.unwrap_or(20), MemorySortOrder::Recent).await
        }).map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;

        Python::with_gil(|py| {
            results.iter()
                .map(|note| self.memory_note_to_dict(py, note))
                .collect::<PyResult<Vec<_>>>()
        })
    }

    /// Get context statistics.
    ///
    /// Returns:
    ///     dict: Statistics with keys: total_memories
    #[pyo3(signature = (namespace=None))]
    fn get_stats(&self, namespace: Option<String>) -> PyResult<PyObject> {
        let ns = namespace.as_ref().map(|s| parse_namespace(s))
            .transpose()
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e))?;

        let count = self.runtime.block_on(async {
            let storage = self.inner.lock().await;
            storage.count_memories(ns).await
        }).map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;

        Python::with_gil(|py| {
            let dict = PyDict::new_bound(py);
            dict.set_item("total_memories", count)?;
            Ok(dict.into())
        })
    }
}

// Helper function to parse namespace from string
pub(crate) fn parse_namespace(s: &str) -> Result<Namespace, String> {
    if s == "global" {
        return Ok(Namespace::Global);
    }

    let parts: Vec<&str> = s.split(':').collect();
    match parts.as_slice() {
        ["project", name] => Ok(Namespace::Project { name: name.to_string() }),
        ["session", project, session_id] => Ok(Namespace::Session {
            project: project.to_string(),
            session_id: session_id.to_string()
        }),
        _ => Err(format!("Invalid namespace format: {}", s)),
    }
}

// Helper methods for type conversion
impl PyStorage {
    fn dict_to_memory_note(&self, dict: &Bound<'_, PyDict>) -> PyResult<MemoryNote> {
        let content: String = dict.get_item("content")?
            .ok_or_else(|| PyErr::new::<pyo3::exceptions::PyKeyError, _>("Missing 'content' key"))?
            .extract()?;

        let namespace_str: String = dict.get_item("namespace")?
            .ok_or_else(|| PyErr::new::<pyo3::exceptions::PyKeyError, _>("Missing 'namespace' key"))?
            .extract()?;

        let namespace = parse_namespace(&namespace_str)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e))?;

        let importance: u8 = dict.get_item("importance")?
            .ok_or_else(|| PyErr::new::<pyo3::exceptions::PyKeyError, _>("Missing 'importance' key"))?
            .extract()?;

        let now = chrono::Utc::now();

        // Create MemoryNote with all required fields
        let note = MemoryNote {
            id: MemoryId::new(),
            namespace,
            created_at: now,
            updated_at: now,
            content,
            summary: dict.get_item("summary").ok().flatten().and_then(|v| v.extract().ok()).unwrap_or_default(),
            keywords: dict.get_item("keywords").ok().flatten().and_then(|v| v.extract().ok()).unwrap_or_default(),
            tags: dict.get_item("tags").ok().flatten().and_then(|v| v.extract().ok()).unwrap_or_default(),
            context: dict.get_item("context").ok().flatten().and_then(|v| v.extract().ok()).unwrap_or_default(),
            memory_type: crate::types::MemoryType::Insight, // Default, could parse from dict
            importance,
            confidence: dict.get_item("confidence").ok().flatten().and_then(|v| v.extract().ok()).unwrap_or(1.0),
            links: vec![],
            related_files: dict.get_item("related_files").ok().flatten().and_then(|v| v.extract().ok()).unwrap_or_default(),
            related_entities: dict.get_item("related_entities").ok().flatten().and_then(|v| v.extract().ok()).unwrap_or_default(),
            access_count: 0,
            last_accessed_at: now,
            expires_at: None,
            is_archived: false,
            superseded_by: None,
            embedding: None,
            embedding_model: String::new(),
        };

        Ok(note)
    }

    fn memory_note_to_dict(&self, py: Python, note: &MemoryNote) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("id", note.id.to_string())?;
        dict.set_item("content", &note.content)?;
        dict.set_item("namespace", note.namespace.to_string())?;
        dict.set_item("importance", note.importance)?;
        dict.set_item("created_at", note.created_at.to_rfc3339())?;
        dict.set_item("access_count", note.access_count)?;
        dict.set_item("summary", &note.summary)?;
        dict.set_item("keywords", note.keywords.clone())?;
        dict.set_item("tags", note.tags.clone())?;

        Ok(dict.into())
    }
}
