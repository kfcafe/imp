//! Host/workflow integration around the core message loop.
//!
//! This facade keeps existing agent call sites stable while separating
//! recipe/runtime support from workflow-graph compatibility.

mod recipe_runtime;
mod workflow_compat;

#[cfg(test)]
pub(super) use recipe_runtime::workflow_layer_may_override_finish;
pub(crate) use recipe_runtime::WorkflowRuntimeLayer;
pub(crate) use workflow_compat::orchestration_follow_up_text;
#[cfg(test)]
pub(super) use workflow_compat::workflow_run_status_from_result;
