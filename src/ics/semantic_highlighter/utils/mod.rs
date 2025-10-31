//! Utility modules for semantic highlighting

pub mod patterns;
pub mod dictionaries;

pub use patterns::CommonPatterns;
pub use dictionaries::{EntityDictionaries, ModalityDictionaries};
