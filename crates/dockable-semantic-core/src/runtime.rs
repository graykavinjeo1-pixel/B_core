use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{
    experience::{
        ExperienceError, ExperienceIR, ExperienceInjectionReceiptIR, ExperienceMemory,
        ExperienceQueryIR, ExperienceSnapshotIR, RecalledExperienceIR,
    },
    interface::{
        Capability, CapabilityRequest, CapabilityResult, GoalIR, ResultIR,
        CAPABILITY_CONTRACT_VERSION, CORE_ABI_VERSION, SEMANTIC_STATE_VERSION,
    },
    planning::{PlanGoalIR, PlanIR, Planner, PlanningError},
    reasoning::{AdaptiveReasoner, ResourceBudget},
    state::{SemanticState, SparseIndex},
    swarm::{SwarmCore, SwarmDeliberationIR, SwarmDeliberationRequestIR, SwarmError},
    task::{Split, VisibleTask},
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CoreError {
    StateLoad,
    StateReceiptInvalid,
    IndexLoad,
    AbiMismatch,
    SemanticStateVersionMismatch,
    ConceptUnavailable,
    ExecutablePatternUnavailable,
    CapabilityIdMismatch,
    CapabilityContractMismatch,
    CapabilityInputTypeMismatch,
}

pub struct DockableCore {
    state: SemanticState,
    index: SparseIndex,
    reasoner: AdaptiveReasoner,
    experience_memory: ExperienceMemory,
    planner: Planner,
    swarm: SwarmCore,
}

impl DockableCore {
    pub fn load_embedded() -> Result<Self, CoreError> {
        let state = SemanticState::load_embedded().map_err(|_| CoreError::StateLoad)?;
        if state.semantic_state_version != SEMANTIC_STATE_VERSION || !state.validate_receipts() {
            return Err(CoreError::StateReceiptInvalid);
        }
        let index = SparseIndex::load_embedded(&state).map_err(|_| CoreError::IndexLoad)?;
        if index.len() != state.concepts.len() {
            return Err(CoreError::IndexLoad);
        }
        Ok(Self {
            state,
            index,
            reasoner: AdaptiveReasoner::default(),
            experience_memory: ExperienceMemory::default(),
            planner: Planner,
            swarm: SwarmCore,
        })
    }

    pub fn execute_goal(&self, goal: &GoalIR) -> Result<ResultIR, CoreError> {
        if goal.core_abi_version != CORE_ABI_VERSION {
            return Err(CoreError::AbiMismatch);
        }
        if goal.semantic_state_version != SEMANTIC_STATE_VERSION {
            return Err(CoreError::SemanticStateVersionMismatch);
        }
        if self.index.route(&goal.target_concept_id).is_none() {
            return Err(CoreError::ConceptUnavailable);
        }
        let pattern = self
            .state
            .pattern(&goal.target_concept_id)
            .ok_or(CoreError::ExecutablePatternUnavailable)?;
        let task = VisibleTask {
            task_id: goal.request_id.clone(),
            split: Split::DirectSemanticRequest,
            scalar_parameter: goal.scalar_parameter,
            demonstrations: goal.demonstrations.clone(),
            query_input: goal.query_input.clone(),
        };
        let mut solve = self.reasoner.semantic_pattern(
            &task,
            ResourceBudget::discovery(),
            &pattern.instructions,
            &pattern.concept_id,
        );
        let verified = solve.committed_output.is_some() && solve.derivation.validate_integrity();
        solve.seal_score(verified);
        let derivation_sha256 = format!(
            "{:x}",
            Sha256::digest(serde_json::to_vec(&solve.derivation).unwrap_or_default())
        );
        Ok(ResultIR {
            request_id: goal.request_id.clone(),
            target_concept_id: goal.target_concept_id.clone(),
            output: solve.committed_output,
            failure: solve.execution_error.map(|error| format!("{error:?}")),
            derivation_sha256,
            verified: solve.verified_after_commit,
            search_expansions: solve.metrics.search_expansions,
            peak_active_concepts: solve.metrics.peak_active_concepts,
            full_catalog_scans: solve.metrics.full_catalog_scans,
            routing_false_negatives: 0,
        })
    }

    pub fn execute_capability(
        &self,
        capability: &mut dyn Capability,
        request: CapabilityRequest,
    ) -> Result<CapabilityResult, CoreError> {
        let contract = capability.contract();
        if contract.core_abi_version != CORE_ABI_VERSION
            || contract.contract_version != CAPABILITY_CONTRACT_VERSION
        {
            return Err(CoreError::CapabilityContractMismatch);
        }
        if contract.capability_id != request.capability_id {
            return Err(CoreError::CapabilityIdMismatch);
        }
        if contract.input_type != request.input.semantic_type() {
            return Err(CoreError::CapabilityInputTypeMismatch);
        }
        let result = capability.execute(request);
        if result
            .output
            .as_ref()
            .is_some_and(|output| output.semantic_type() != contract.output_type)
        {
            return Err(CoreError::CapabilityContractMismatch);
        }
        Ok(result)
    }

    pub fn semantic_state(&self) -> &SemanticState {
        &self.state
    }

    pub fn sparse_index_len(&self) -> usize {
        self.index.len()
    }

    /// Adds one bounded, typed experience to the non-authoritative episodic
    /// memory used by planning. Semantic state and executable concepts remain
    /// unchanged until a separately verified promotion path accepts them.
    pub fn inject_experience(
        &mut self,
        experience: ExperienceIR,
    ) -> Result<ExperienceInjectionReceiptIR, ExperienceError> {
        self.experience_memory.inject(experience)
    }

    pub fn recall_experiences(
        &self,
        query: &ExperienceQueryIR,
    ) -> Result<Vec<RecalledExperienceIR>, ExperienceError> {
        self.experience_memory.recall(query)
    }

    pub fn generate_plan(&self, goal: &PlanGoalIR) -> Result<PlanIR, PlanningError> {
        self.planner.generate(goal, &self.experience_memory)
    }

    /// Runs a bounded internal panel. Worker roles are selected from typed
    /// quality criteria and never delegate authority to an external model.
    pub fn deliberate(
        &self,
        request: &SwarmDeliberationRequestIR,
    ) -> Result<SwarmDeliberationIR, SwarmError> {
        self.swarm.deliberate(request)
    }

    pub fn retained_experience_count(&self) -> usize {
        self.experience_memory.len()
    }

    pub fn export_experience_snapshot(&self) -> ExperienceSnapshotIR {
        self.experience_memory.export_snapshot()
    }

    pub fn import_experience_snapshot(
        &mut self,
        snapshot: &ExperienceSnapshotIR,
    ) -> Result<Vec<ExperienceInjectionReceiptIR>, ExperienceError> {
        self.experience_memory.import_snapshot(snapshot)
    }
}

#[cfg(test)]
mod tests {
    use super::DockableCore;
    use crate::{
        interface::{GoalIR, CORE_ABI_VERSION, SEMANTIC_STATE_VERSION},
        task::Demonstration,
    };

    #[test]
    fn direct_goal_ir_needs_no_language_or_research_artifact() {
        let core = DockableCore::load_embedded().expect("embedded core state");
        let goal = GoalIR {
            request_id: "CORE-TEST-001".to_string(),
            core_abi_version: CORE_ABI_VERSION,
            semantic_state_version: SEMANTIC_STATE_VERSION.to_string(),
            target_concept_id: "C000001".to_string(),
            scalar_parameter: 3,
            demonstrations: vec![Demonstration {
                input: vec![1, -2],
                observed_output: vec![4, 1],
            }],
            query_input: vec![2, 4, -1],
            constraints: vec!["CHECKED_ARITHMETIC".to_string()],
        };
        let result = core.execute_goal(&goal).expect("core result");
        assert_eq!(result.output, Some(vec![5, 7, 2]));
        assert!(result.verified);
        assert_eq!(result.full_catalog_scans, 0);
    }
}
