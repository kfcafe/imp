//! Host/workflow integration around the core message loop.
//!
//! This facade keeps existing agent call sites stable while separating
//! recipe/runtime support from mana/work-graph compatibility.

mod mana_compat;
mod recipe_runtime;

pub(crate) use mana_compat::orchestration_follow_up_text;
pub(crate) use recipe_runtime::WorkflowRuntimeLayer;
