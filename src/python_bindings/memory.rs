//! PyMemory - Python wrappers for Mnemosyne memory types.
//!
//! Exposes core types (MemoryId, Namespace, MemoryNote) to Python
//! with efficient serialization and zero-copy where possible.

use crate::types::{
    MemoryId as RustMemoryId, MemoryNote as RustMemoryNote, Namespace as RustNamespace,
};
use pyo3::prelude::*;

/// Python wrapper for MemoryId.
#[pyclass]
#[derive(Clone)]
pub struct PyMemoryId {
    inner: RustMemoryId,
}

#[pymethods]
impl PyMemoryId {
    /// Create new random MemoryId.
    #[new]
    fn new() -> Self {
        PyMemoryId {
            inner: RustMemoryId::new(),
        }
    }

    /// Parse MemoryId from string.
    #[staticmethod]
    fn parse(s: &str) -> PyResult<Self> {
        let id = RustMemoryId::from_string(s)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))?;
        Ok(PyMemoryId { inner: id })
    }

    /// Convert to string.
    fn __str__(&self) -> String {
        self.inner.to_string()
    }

    fn __repr__(&self) -> String {
        format!("MemoryId('{}')", self.inner)
    }
}

/// Python wrapper for Namespace.
#[pyclass]
#[derive(Clone)]
pub struct PyNamespace {
    inner: RustNamespace,
}

#[pymethods]
impl PyNamespace {
    /// Create Global namespace.
    #[staticmethod]
    fn global() -> Self {
        PyNamespace {
            inner: RustNamespace::Global,
        }
    }

    /// Create Project namespace.
    #[staticmethod]
    fn project(name: String) -> Self {
        PyNamespace {
            inner: RustNamespace::Project { name },
        }
    }

    /// Create Session namespace.
    #[staticmethod]
    fn session(project: String, session_id: String) -> Self {
        PyNamespace {
            inner: RustNamespace::Session {
                project,
                session_id,
            },
        }
    }

    /// Parse namespace from string (e.g., "global", "project:mnemosyne", "session:mnem:abc123").
    #[staticmethod]
    fn parse(s: &str) -> PyResult<Self> {
        use crate::python_bindings::storage::parse_namespace;
        let ns =
            parse_namespace(s).map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e))?;
        Ok(PyNamespace { inner: ns })
    }

    /// Convert to string.
    fn __str__(&self) -> String {
        self.inner.to_string()
    }

    fn __repr__(&self) -> String {
        format!("Namespace('{}')", self.inner)
    }

    /// Get namespace kind ("global", "project", or "session").
    #[getter]
    fn kind(&self) -> String {
        match &self.inner {
            RustNamespace::Global => "global".to_string(),
            RustNamespace::Project { .. } => "project".to_string(),
            RustNamespace::Session { .. } => "session".to_string(),
        }
    }

    /// Get namespace name (for Project/Session, None for Global).
    #[getter]
    fn name(&self) -> Option<String> {
        match &self.inner {
            RustNamespace::Global => None,
            RustNamespace::Project { name } => Some(name.clone()),
            RustNamespace::Session { session_id, .. } => Some(session_id.clone()),
        }
    }
}

/// Python wrapper for MemoryNote.
///
/// Lightweight wrapper - most access goes through PyStorage.
/// This is primarily for type safety and IDE autocomplete.
#[pyclass]
#[derive(Clone)]
pub struct PyMemory {
    #[pyo3(get)]
    pub id: String,

    #[pyo3(get)]
    pub content: String,

    #[pyo3(get)]
    pub namespace: String,

    #[pyo3(get)]
    pub importance: u8,

    #[pyo3(get)]
    pub summary: String,

    #[pyo3(get)]
    pub keywords: Vec<String>,

    #[pyo3(get)]
    pub tags: Vec<String>,

    #[pyo3(get)]
    pub created_at: String,

    #[pyo3(get)]
    pub access_count: u32,
}

#[pymethods]
impl PyMemory {
    /// Create new PyMemory from components.
    #[new]
    #[pyo3(signature = (id, content, namespace, importance, summary=None, keywords=None, tags=None))]
    fn new(
        id: String,
        content: String,
        namespace: String,
        importance: u8,
        summary: Option<String>,
        keywords: Option<Vec<String>>,
        tags: Option<Vec<String>>,
    ) -> Self {
        PyMemory {
            id,
            content,
            namespace,
            importance,
            summary: summary.unwrap_or_default(),
            keywords: keywords.unwrap_or_default(),
            tags: tags.unwrap_or_default(),
            created_at: chrono::Utc::now().to_rfc3339(),
            access_count: 0,
        }
    }

    fn __repr__(&self) -> String {
        format!(
            "Memory(id='{}', namespace='{}', importance={})",
            &self.id[..8], // First 8 chars of UUID
            self.namespace,
            self.importance
        )
    }

    fn __str__(&self) -> String {
        if !self.summary.is_empty() {
            self.summary.clone()
        } else {
            self.content.chars().take(100).collect::<String>()
        }
    }
}

// Conversion helpers
impl From<&RustMemoryNote> for PyMemory {
    fn from(note: &RustMemoryNote) -> Self {
        PyMemory {
            id: note.id.to_string(),
            content: note.content.clone(),
            namespace: note.namespace.to_string(),
            importance: note.importance,
            summary: note.summary.clone(),
            keywords: note.keywords.clone(),
            tags: note.tags.clone(),
            created_at: note.created_at.to_rfc3339(),
            access_count: note.access_count,
        }
    }
}
