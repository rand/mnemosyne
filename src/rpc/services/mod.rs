//! gRPC service implementations

pub mod health;
pub mod memory;

pub use health::HealthServiceImpl;
pub use memory::MemoryServiceImpl;
