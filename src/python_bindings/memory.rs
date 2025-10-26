//! PyMemory - Python wrappers for Mnemosyne memory types.
//!
//! Exposes core types (MemoryId, Namespace, MemoryNote) to Python
//! with efficient serialization and zero-copy where possible.

use pyo3::prelude::*;
use crate::types::{MemoryId as RustMemoryId, Namespace as RustNamespace, MemoryNote as RustMemoryNote};

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
        let id = s.parse::<uuid::Uuid>()
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))?;
        Ok(PyMemoryId {
            inner: id.into(),
        })
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
            inner: RustNamespace::Project(name),
        }
    }

    /// Create Session namespace.
    #[staticmethod]
    fn session(id: String) -> Self {
        PyNamespace {
            inner: RustNamespace::Session(id),
        }
    }

    /// Parse namespace from string (e.g., "global", "project:mnemosyne", "session:abc123").
    #[staticmethod]
    fn parse(s: &str) -> PyResult<Self> {
        let ns = RustNamespace::parse(s)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))?;
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
            RustNamespace::Project(_) => "project".to_string(),
            RustNamespace::Session(_) => "session".to_string(),
        }
    }

    /// Get namespace name (for Project/Session, None for Global).
    #[getter]
    fn name(&self) -> Option<String> {
        match &self.inner {
            RustNamespace::Global => None,
            RustNamespace::Project(name) => Some(name.clone()),
            RustNamespace::Session(id) => Some(id.clone()),
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
    pub importance: i32,

    #[pyo3(get)]
    pub summary: Option<String>,

    #[pyo3(get)]
    pub keywords: Vec<String>,

    #[pyo3(get)]
    pub tags: Vec<String>,

    #[pyo3(get)]
    pub created_at: String,

    #[pyo3(get)]
    pub access_count: i32,
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
        importance: i32,
        summary: Option<String>,
        keywords: Option<Vec<String>>,
        tags: Option<Vec<String>>,
    ) -> Self {
        PyMemory {
            id,
            content,
            namespace,
            importance,
            summary,
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
        if let Some(ref summary) = self.summary {
            summary.clone()
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
