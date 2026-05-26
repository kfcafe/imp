//! Host/workflow integration around the core message loop.
//!
//! This facade keeps existing agent call sites stable while separating
//! recipe/runtime support from mana/work-graph compatibility.

mod mana_compat;
mod recipe_runtime;

#[cfg(test)]
pub(super) use mana_compat::mana_run_status_from_result;
pub(crate) use mana_compat::orchestration_follow_up_text;
#[cfg(test)]
pub(super) use recipe_runtime::workflow_layer_may_override_finish;
pub(crate) use recipe_runtime::WorkflowRuntimeLayer;
