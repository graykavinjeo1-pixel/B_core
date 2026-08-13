//! Product-agnostic semantic runtime.
//!
//! Research runners, blind data, reports, language aliases, and product adapters
//! deliberately live outside this crate.

pub mod dsl;
pub mod experience;
pub mod interface;
pub mod planning;
pub mod reasoning;
pub mod runtime;
pub mod state;
pub mod substrate;
pub mod task;

pub use experience::{
    ExperienceError, ExperienceIR, ExperienceInjectionReceiptIR, ExperienceOutcomeIR,
    ExperienceQueryIR, ExperienceSnapshotIR, RecalledExperienceIR, EXPERIENCE_SCHEMA,
    EXPERIENCE_SNAPSHOT_SCHEMA,
};
pub use interface::{
    Capability, CapabilityContract, CapabilityRequest, CapabilityResult, GoalIR, ResultIR,
    SemanticValue, CAPABILITY_CONTRACT_VERSION, CORE_ABI_VERSION, SEMANTIC_STATE_VERSION,
};
pub use planning::{
    PlanGoalIR, PlanIR, PlanIntentIR, PlanOperationIR, PlanStepIR, PlanningError, PLAN_GOAL_SCHEMA,
    PLAN_SCHEMA,
};
pub use runtime::{CoreError, DockableCore};
