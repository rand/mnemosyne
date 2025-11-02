//! Agent Actor Implementations
//!
//! Four agents coordinate through Ractor's actor model:
//! - **OrchestratorActor**: Central coordinator and work queue manager
//! - **OptimizerActor**: Context optimization and skill discovery
//! - **ReviewerActor**: Quality assurance with blocking gates
//! - **ExecutorActor**: Primary work execution with sub-agent spawning

pub mod executor;
pub mod optimizer;
pub mod orchestrator;
pub mod reviewer;
#[cfg(feature = "python")]
pub mod reviewer_dspy_adapter;

pub use executor::ExecutorActor;
pub use optimizer::OptimizerActor;
pub use orchestrator::OrchestratorActor;
pub use reviewer::ReviewerActor;
#[cfg(feature = "python")]
pub use reviewer_dspy_adapter::ReviewerDSpyAdapter;
