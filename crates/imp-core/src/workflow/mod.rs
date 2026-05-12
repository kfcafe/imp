//! Workflow-first runtime data model.
//!
//! The workflow module starts as a lightweight contract model that can wrap
//! existing imp runs without changing agent behavior. Later workflow runtime
//! features (policy, verification, evidence, child runs) build on these types.

mod contract;

pub use contract::*;
