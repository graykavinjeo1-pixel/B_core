//! Product-agnostic semantic runtime.
//!
//! Research runners, blind data, reports, language aliases, and product adapters
//! deliberately live outside this crate.

pub mod dsl;
pub mod interface;
pub mod reasoning;
pub mod runtime;
pub mod state;
pub mod substrate;
pub mod task;

pub use interface::{
    Capability, CapabilityContract, CapabilityRequest, CapabilityResult, GoalIR, ResultIR,
    SemanticValue, CAPABILITY_CONTRACT_VERSION, CORE_ABI_VERSION, SEMANTIC_STATE_VERSION,
};
pub use runtime::{CoreError, DockableCore};
