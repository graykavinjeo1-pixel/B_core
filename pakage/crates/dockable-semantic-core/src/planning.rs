use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::experience::{
    ExperienceMemory, ExperienceOutcomeIR, ExperienceQueryIR, RecalledExperienceIR,
};

pub const PLAN_GOAL_SCHEMA: &str = "B_CORE_PLAN_GOAL_IR_1";
pub const PLAN_SCHEMA: &str = "B_CORE_PLAN_IR_1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PlanIntentIR {
    Plan,
    Investigate,
    Repair,
    Create,
    Learn,
    Explain,
    Communicate,
    Execute,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlanGoalIR {
    pub schema: String,
    pub goal_id: String,
    pub intent: PlanIntentIR,
    pub subject: String,
    pub constraints: Vec<String>,
    pub desired_outcomes: Vec<String>,
    pub context_tags: Vec<String>,
    pub max_steps: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PlanOperationIR {
    ObserveCurrentState,
    RecallRelevantExperience,
    SurfaceAssumptions,
    DerivePostconditions,
    ModelKnowledgeGap,
    GenerateCandidates,
    GenerateCompetingHypotheses,
    ConstructCausalModel,
    PredictConsequences,
    SimulateCounterfactuals,
    SelectInformationGainAction,
    RunDiagnostic,
    ValidateCandidates,
    ApplySelectedAction,
    VerifyOutcome,
    ReplanFromObservation,
    CalibrateConfidence,
    GeneralizeLesson,
    StoreSuccessfulExperience,
    SynthesizeExplanation,
    CommunicateResult,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlanStepIR {
    pub step_id: String,
    pub operation: PlanOperationIR,
    pub target: String,
    pub dependencies: Vec<String>,
    pub preconditions: Vec<String>,
    pub expected_postconditions: Vec<String>,
    pub supporting_experience_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlanIR {
    pub schema: String,
    pub goal_id: String,
    pub intent: PlanIntentIR,
    pub steps: Vec<PlanStepIR>,
    pub recalled_experiences: Vec<RecalledExperienceIR>,
    pub terminal_verification_step_id: String,
    pub plan_sha256: String,
    pub structurally_validated: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PlanningError {
    InvalidSchema,
    InvalidGoal,
    BudgetTooSmall,
    BudgetTooLarge,
    ExperienceQueryFailed,
    InvalidPlanGraph,
}

#[derive(Debug, Default)]
pub struct Planner;

impl Planner {
    pub fn generate(
        &self,
        goal: &PlanGoalIR,
        experience_memory: &ExperienceMemory,
    ) -> Result<PlanIR, PlanningError> {
        validate_goal(goal)?;
        let recalled_experiences = if experience_memory.is_empty() {
            Vec::new()
        } else {
            experience_memory
                .recall(&ExperienceQueryIR {
                    semantic_tags: goal.context_tags.clone(),
                    text_terms: subject_terms(&goal.subject),
                    preferred_outcome: Some(ExperienceOutcomeIR::Successful),
                    max_results: 4,
                })
                .map_err(|_| PlanningError::ExperienceQueryFailed)?
        };
        let successful_experience_ids = recalled_experiences
            .iter()
            .filter(|recalled| recalled.experience.outcome == ExperienceOutcomeIR::Successful)
            .map(|recalled| recalled.experience.experience_id.clone())
            .collect::<Vec<_>>();
        let operations = operations_for_intent(goal.intent);
        if operations.len() > goal.max_steps {
            return Err(PlanningError::BudgetTooSmall);
        }
        let mut steps = Vec::with_capacity(operations.len());
        for (index, operation) in operations.iter().copied().enumerate() {
            let step_id = format!("STEP-{:02}", index + 1);
            let dependencies = if index > 0 {
                vec![format!("STEP-{index:02}")]
            } else {
                Vec::new()
            };
            steps.push(PlanStepIR {
                step_id,
                operation,
                target: goal.subject.clone(),
                dependencies,
                preconditions: preconditions(operation, goal),
                expected_postconditions: postconditions(operation, goal),
                supporting_experience_ids: if matches!(
                    operation,
                    PlanOperationIR::GenerateCandidates
                        | PlanOperationIR::GenerateCompetingHypotheses
                        | PlanOperationIR::ConstructCausalModel
                        | PlanOperationIR::PredictConsequences
                        | PlanOperationIR::SimulateCounterfactuals
                ) {
                    successful_experience_ids.clone()
                } else if operation == PlanOperationIR::RecallRelevantExperience {
                    recalled_experiences
                        .iter()
                        .map(|recalled| recalled.experience.experience_id.clone())
                        .collect()
                } else {
                    Vec::new()
                },
            });
        }
        let terminal_verification_step_id = steps
            .iter()
            .rev()
            .find(|step| step.operation == PlanOperationIR::VerifyOutcome)
            .or_else(|| {
                steps
                    .iter()
                    .rev()
                    .find(|step| step.operation == PlanOperationIR::CommunicateResult)
            })
            .map(|step| step.step_id.clone())
            .ok_or(PlanningError::InvalidPlanGraph)?;
        validate_plan_graph(&steps)?;
        let plan_sha256 = plan_identity(goal, &steps, &recalled_experiences);
        Ok(PlanIR {
            schema: PLAN_SCHEMA.to_string(),
            goal_id: goal.goal_id.clone(),
            intent: goal.intent,
            steps,
            recalled_experiences,
            terminal_verification_step_id,
            plan_sha256,
            structurally_validated: true,
        })
    }
}

fn validate_goal(goal: &PlanGoalIR) -> Result<(), PlanningError> {
    if goal.schema != PLAN_GOAL_SCHEMA {
        return Err(PlanningError::InvalidSchema);
    }
    if goal.goal_id.trim().is_empty()
        || goal.goal_id.len() > 128
        || goal.subject.trim().is_empty()
        || goal.subject.len() > 64 * 1024
        || goal.desired_outcomes.is_empty()
        || goal.desired_outcomes.len() > 64
        || goal.constraints.len() > 64
        || goal.context_tags.len() > 64
        || goal
            .desired_outcomes
            .iter()
            .chain(&goal.constraints)
            .any(|value| value.trim().is_empty() || value.len() > 4_096)
        || goal
            .context_tags
            .iter()
            .any(|tag| tag.trim().is_empty() || tag.len() > 128)
    {
        return Err(PlanningError::InvalidGoal);
    }
    if goal.max_steps < 5 {
        return Err(PlanningError::BudgetTooSmall);
    }
    if goal.max_steps > 32 {
        return Err(PlanningError::BudgetTooLarge);
    }
    Ok(())
}

fn operations_for_intent(intent: PlanIntentIR) -> &'static [PlanOperationIR] {
    use PlanOperationIR::*;
    match intent {
        PlanIntentIR::Repair => &[
            ObserveCurrentState,
            RecallRelevantExperience,
            DerivePostconditions,
            SurfaceAssumptions,
            GenerateCompetingHypotheses,
            SimulateCounterfactuals,
            SelectInformationGainAction,
            ValidateCandidates,
            ApplySelectedAction,
            VerifyOutcome,
            ReplanFromObservation,
            StoreSuccessfulExperience,
        ],
        PlanIntentIR::Create | PlanIntentIR::Execute => &[
            ObserveCurrentState,
            RecallRelevantExperience,
            DerivePostconditions,
            SurfaceAssumptions,
            GenerateCompetingHypotheses,
            ConstructCausalModel,
            SimulateCounterfactuals,
            SelectInformationGainAction,
            ValidateCandidates,
            ApplySelectedAction,
            VerifyOutcome,
            CommunicateResult,
        ],
        PlanIntentIR::Investigate => &[
            ObserveCurrentState,
            RecallRelevantExperience,
            SurfaceAssumptions,
            ModelKnowledgeGap,
            GenerateCompetingHypotheses,
            ConstructCausalModel,
            SimulateCounterfactuals,
            SelectInformationGainAction,
            RunDiagnostic,
            VerifyOutcome,
            CalibrateConfidence,
            CommunicateResult,
        ],
        PlanIntentIR::Learn => &[
            ObserveCurrentState,
            RecallRelevantExperience,
            SurfaceAssumptions,
            ModelKnowledgeGap,
            GenerateCompetingHypotheses,
            ConstructCausalModel,
            SimulateCounterfactuals,
            SelectInformationGainAction,
            RunDiagnostic,
            GeneralizeLesson,
            ValidateCandidates,
            StoreSuccessfulExperience,
            VerifyOutcome,
            CalibrateConfidence,
        ],
        PlanIntentIR::Explain | PlanIntentIR::Communicate => &[
            ObserveCurrentState,
            RecallRelevantExperience,
            SurfaceAssumptions,
            ModelKnowledgeGap,
            GenerateCompetingHypotheses,
            ConstructCausalModel,
            SimulateCounterfactuals,
            SynthesizeExplanation,
            VerifyOutcome,
            CalibrateConfidence,
            CommunicateResult,
        ],
        PlanIntentIR::Plan => &[
            ObserveCurrentState,
            RecallRelevantExperience,
            DerivePostconditions,
            SurfaceAssumptions,
            GenerateCompetingHypotheses,
            ConstructCausalModel,
            SimulateCounterfactuals,
            SelectInformationGainAction,
            ValidateCandidates,
            VerifyOutcome,
            CalibrateConfidence,
            CommunicateResult,
        ],
    }
}

fn preconditions(operation: PlanOperationIR, goal: &PlanGoalIR) -> Vec<String> {
    match operation {
        PlanOperationIR::GenerateCompetingHypotheses => vec![
            "observations remain separate from assumptions".to_string(),
            "at least one falsifier is defined for each actionable hypothesis".to_string(),
        ],
        PlanOperationIR::SelectInformationGainAction => vec![
            "candidate diagnostics are read-only or reversibly bounded".to_string(),
            "expected information gain exceeds diagnostic cost".to_string(),
        ],
        PlanOperationIR::ApplySelectedAction => vec![
            "candidate validated".to_string(),
            "rollback or recovery path available".to_string(),
            "action lies inside the explicit authority envelope".to_string(),
        ],
        PlanOperationIR::ReplanFromObservation => {
            vec!["post-action observation is available".to_string()]
        }
        PlanOperationIR::StoreSuccessfulExperience => {
            vec!["outcome independently verified".to_string()]
        }
        PlanOperationIR::CommunicateResult => {
            vec!["claims bound to observed evidence".to_string()]
        }
        _ => goal.constraints.clone(),
    }
}

fn postconditions(operation: PlanOperationIR, goal: &PlanGoalIR) -> Vec<String> {
    match operation {
        PlanOperationIR::SurfaceAssumptions => {
            vec!["unstated assumptions are represented as falsifiable propositions".to_string()]
        }
        PlanOperationIR::DerivePostconditions => goal.desired_outcomes.clone(),
        PlanOperationIR::GenerateCompetingHypotheses => vec![
            "multiple causally distinct explanations are retained until evidence separates them"
                .to_string(),
        ],
        PlanOperationIR::ConstructCausalModel => vec![
            "prerequisite, effect, observation, cost, risk, and authority are typed".to_string(),
        ],
        PlanOperationIR::SimulateCounterfactuals => vec![
            "candidate actions have predicted goal, conflict, cost, and risk deltas".to_string(),
        ],
        PlanOperationIR::SelectInformationGainAction => vec![
            "uncertainty-reducing diagnostic is preferred when intervention is not justified"
                .to_string(),
        ],
        PlanOperationIR::VerifyOutcome => goal
            .desired_outcomes
            .iter()
            .map(|outcome| format!("verified:{outcome}"))
            .collect(),
        PlanOperationIR::StoreSuccessfulExperience => {
            vec!["successful method is callable knowledge".to_string()]
        }
        PlanOperationIR::ReplanFromObservation => {
            vec!["failed predictions update the next bounded plan".to_string()]
        }
        PlanOperationIR::CalibrateConfidence => {
            vec!["confidence reflects supporting, opposing, and unresolved evidence".to_string()]
        }
        _ => vec![format!("{:?} completed", operation)],
    }
}

fn validate_plan_graph(steps: &[PlanStepIR]) -> Result<(), PlanningError> {
    let mut observed = BTreeSet::new();
    for step in steps {
        if step.step_id.is_empty()
            || !observed.insert(step.step_id.clone())
            || step
                .dependencies
                .iter()
                .any(|dependency| !observed.contains(dependency))
        {
            return Err(PlanningError::InvalidPlanGraph);
        }
    }
    Ok(())
}

fn subject_terms(subject: &str) -> Vec<String> {
    subject
        .split(|character: char| !character.is_alphanumeric() && character != '_')
        .map(str::to_lowercase)
        .filter(|term| term.chars().count() >= 2)
        .take(32)
        .collect()
}

fn plan_identity(
    goal: &PlanGoalIR,
    steps: &[PlanStepIR],
    experiences: &[RecalledExperienceIR],
) -> String {
    format!(
        "{:X}",
        Sha256::digest(serde_json::to_vec(&(goal, steps, experiences)).unwrap_or_default())
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::experience::{ExperienceIR, EXPERIENCE_SCHEMA};

    #[test]
    fn repair_plan_is_a_valid_dag_and_reuses_successful_experience() {
        let mut memory = ExperienceMemory::default();
        memory
            .inject(ExperienceIR {
                schema: EXPERIENCE_SCHEMA.to_string(),
                experience_id: "EXP-PATH-1".to_string(),
                situation: "Rust path handling failed".to_string(),
                action: "use a predecessor-bound literal path".to_string(),
                outcome: ExperienceOutcomeIR::Successful,
                outcome_description: "path behavior restored".to_string(),
                semantic_tags: vec!["rust".to_string(), "path".to_string()],
                evidence: vec!["tests passed".to_string()],
                confidence_millis: 900,
                source_language: Some("en".to_string()),
            })
            .unwrap();
        let plan = Planner
            .generate(
                &PlanGoalIR {
                    schema: PLAN_GOAL_SCHEMA.to_string(),
                    goal_id: "GOAL-1".to_string(),
                    intent: PlanIntentIR::Repair,
                    subject: "repair Rust path handling".to_string(),
                    constraints: vec!["preserve public behavior".to_string()],
                    desired_outcomes: vec!["path tests pass".to_string()],
                    context_tags: vec!["rust".to_string(), "path".to_string()],
                    max_steps: 12,
                },
                &memory,
            )
            .unwrap();
        assert!(plan.structurally_validated);
        assert_eq!(plan.recalled_experiences.len(), 1);
        assert!(plan.steps.iter().any(|step| {
            step.operation == PlanOperationIR::GenerateCompetingHypotheses
                && step
                    .supporting_experience_ids
                    .contains(&"EXP-PATH-1".to_string())
        }));
    }

    #[test]
    fn failed_experience_is_diagnostic_context_not_solution_authority() {
        let mut memory = ExperienceMemory::default();
        memory
            .inject(ExperienceIR {
                schema: EXPERIENCE_SCHEMA.to_string(),
                experience_id: "EXP-FAILED-1".to_string(),
                situation: "path repair attempt".to_string(),
                action: "replace every separator".to_string(),
                outcome: ExperienceOutcomeIR::Failed,
                outcome_description: "unrelated paths regressed".to_string(),
                semantic_tags: vec!["repair".to_string(), "path".to_string()],
                evidence: vec!["regression failed".to_string()],
                confidence_millis: 900,
                source_language: Some("en".to_string()),
            })
            .unwrap();
        let plan = Planner
            .generate(
                &PlanGoalIR {
                    schema: PLAN_GOAL_SCHEMA.to_string(),
                    goal_id: "GOAL-FAILED-MEMORY".to_string(),
                    intent: PlanIntentIR::Repair,
                    subject: "repair path behavior".to_string(),
                    constraints: vec!["preserve unrelated paths".to_string()],
                    desired_outcomes: vec!["path behavior restored".to_string()],
                    context_tags: vec!["repair".to_string(), "path".to_string()],
                    max_steps: 12,
                },
                &memory,
            )
            .unwrap();
        assert_eq!(plan.recalled_experiences.len(), 1);
        assert!(plan.steps.iter().any(|step| {
            step.operation == PlanOperationIR::RecallRelevantExperience
                && step
                    .supporting_experience_ids
                    .contains(&"EXP-FAILED-1".to_string())
        }));
        assert!(plan.steps.iter().all(|step| {
            !matches!(
                step.operation,
                PlanOperationIR::GenerateCandidates
                    | PlanOperationIR::GenerateCompetingHypotheses
                    | PlanOperationIR::ConstructCausalModel
                    | PlanOperationIR::PredictConsequences
                    | PlanOperationIR::SimulateCounterfactuals
            ) || !step
                .supporting_experience_ids
                .contains(&"EXP-FAILED-1".to_string())
        }));
    }
}
