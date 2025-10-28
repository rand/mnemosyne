//! Python bindings for Mnemosyne via PyO3.
//!
//! This module provides direct Rust ↔ Python interop for the multi-agent
//! orchestration system, enabling:
//! - Low-latency storage operations (<1ms vs 20-50ms subprocess)
//! - High-frequency context monitoring (10ms polling)
//! - Shared memory coordination between agents

mod storage;
mod memory;
mod coordination;
mod evaluation;

use pyo3::prelude::*;

/// PyO3 module initialization.
///
/// Exposes Rust types and functions to Python as the `mnemosyne_core` module.
#[pymodule]
fn mnemosyne_core(m: &Bound<'_, PyModule>) -> PyResult<()> {
    // Storage layer
    m.add_class::<storage::PyStorage>()?;

    // Memory types
    m.add_class::<memory::PyMemory>()?;
    m.add_class::<memory::PyMemoryId>()?;
    m.add_class::<memory::PyNamespace>()?;

    // Coordination primitives
    m.add_class::<coordination::PyCoordinator>()?;

    // Evaluation system
    m.add_class::<evaluation::PyFeedbackCollector>()?;
    m.add_class::<evaluation::PyFeatureExtractor>()?;
    m.add_class::<evaluation::PyRelevanceScorer>()?;

    Ok(())
}
