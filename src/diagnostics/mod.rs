//! Diagnostic and profiling utilities
//!
//! This module provides tools for diagnosing stability issues:
//! - Memory profiling and tracking
//! - Resource leak detection
//! - Performance monitoring

pub mod memory;

pub use memory::{
    global_memory_tracker, start_memory_monitoring, MemorySnapshot, MemoryStatus, MemoryTracker,
};
