//! Product-agnostic semantic runtime.
//!
//! Research runners, blind data, reports, language aliases, and product adapters
//! deliberately live outside this crate.

pub mod deliberation;
pub mod dsl;
pub mod experience;
pub mod interface;
pub mod mechanism_memory;
pub mod planning;
pub mod reasoning;
pub mod runtime;
pub mod state;
pub mod substrate;
pub mod swarm;
pub mod task;

pub use deliberation::{
    ActionAuthorityIR, AuthorityEnvelopeIR, BeliefIR, BeliefStatusIR, CausalMechanismIR,
    CounterfactualIR, DeliberationDispositionIR, DeliberationEngine, DeliberationError,
    DeliberationIR, DeliberationPlanIR, DeliberationRequestIR, DeliberationRevisionIR,
    DeliberationRevisionRequestIR, EvidenceIR, GroundedSelfModelIR, HypothesisIR, LiteralIR,
    MechanismKindIR, AUTHORITY_ENVELOPE_SCHEMA, DELIBERATION_REQUEST_SCHEMA,
    DELIBERATION_REVISION_REQUEST_SCHEMA, DELIBERATION_REVISION_SCHEMA, DELIBERATION_SCHEMA,
};
pub use experience::{
    ExperienceError, ExperienceIR, ExperienceInjectionReceiptIR, ExperienceOutcomeIR,
    ExperienceQueryIR, ExperienceSnapshotIR, RecalledExperienceIR, EXPERIENCE_SCHEMA,
    EXPERIENCE_SNAPSHOT_SCHEMA,
};
pub use interface::{
    Capability, CapabilityContract, CapabilityRequest, CapabilityResult, GoalIR, ResultIR,
    SemanticValue, CAPABILITY_CONTRACT_VERSION, CORE_ABI_VERSION, SEMANTIC_STATE_VERSION,
};
pub use mechanism_memory::{
    KnowledgeGroundedDeliberationIR, MechanismKnowledgeIR, MechanismKnowledgeInjectionReceiptIR,
    MechanismMemory, MechanismMemoryError, MechanismMemorySnapshotIR, MechanismQueryIR,
    RecalledMechanismIR, DEFAULT_MECHANISM_MEMORY_CAPACITY, KNOWLEDGE_GROUNDED_DELIBERATION_SCHEMA,
    MECHANISM_KNOWLEDGE_SCHEMA, MECHANISM_MEMORY_SNAPSHOT_SCHEMA,
};
pub use planning::{
    PlanGoalIR, PlanIR, PlanIntentIR, PlanOperationIR, PlanStepIR, PlanningError, PLAN_GOAL_SCHEMA,
    PLAN_SCHEMA,
};
pub use runtime::{CoreError, DockableCore};
pub use swarm::{
    AssessmentVerdictIR, CriterionDecisionIR, DeliberationFactIR, ExpertContributionIR,
    ExpertWorkerIR, ExpertWorkerRoleIR, PeerMessageIR, PeerReviewDispositionIR, QualityCriterionIR,
    SwarmCore, SwarmDeliberationIR, SwarmDeliberationRequestIR, SwarmError,
    SWARM_DELIBERATION_REQUEST_SCHEMA, SWARM_DELIBERATION_SCHEMA,
};
