//! Language-neutral thought graph assembled from immutable language-module contributions.
//!
//! Surface Cortex modules contribute evidence. They do not mutate this graph and
//! they never receive semantic or execution authority from it.

use std::collections::{BTreeMap, BTreeSet};

use dockable_semantic_core::{
    PlanIntentIR, SemanticPlanArgumentIR, SemanticPlanEventIR, SemanticPlanGoalIR,
    SemanticPlanProjectionIR, SemanticPlanRelationIR, SemanticPlanRelationKindIR,
    SemanticPlanRoleIR, SEMANTIC_PLAN_GOAL_SCHEMA,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::clause_graph::{ClauseFunctionIR, ClauseRelationKindIR};
use crate::compositional_semantics::{
    CompositionalAnalysisIR, FrameMoodIR, FramePolarityIR, PredicateFrameIR,
};
use crate::language_knowledge::LanguageCodeIR;
use crate::native_language_circuit::{
    subjects_share_context_concept, NativeEventScopeIR, NativeTurnIR,
};
use crate::pragmatic_intent::{PragmaticGoalProjectionIR, PragmaticIntentGraphIR};
use crate::pragmatics::{
    CommitmentActivationIR, IllocutionaryCommitmentGraphIR, IllocutionaryForceIR,
    InferredPragmaticGoalIR,
};
use crate::semantic_roles::SemanticRoleKindIR;

pub const LANGUAGE_CENTER_SCHEMA: &str = "B_CORE_LANGUAGE_CENTER_IR_2";
pub const LANGUAGE_CENTER_GOAL_PROJECTION_SCHEMA: &str =
    "B_CORE_LANGUAGE_CENTER_GOAL_PROJECTION_IR_1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum LanguageCenterSourceIR {
    NativeCircuit,
    CompositionalFrame,
    ClauseGraph,
    SemanticRoleGraph,
    GrammaticalScopeGraph,
    ModalScopeGraph,
    PragmaticIntentGraph,
    IllocutionaryCommitmentGraph,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum LanguageCenterProjectionIR {
    Prohibited,
    Conditional,
    Reported,
    Suppressed,
    LiveRequest,
    Advisory,
    Inquiry,
    Descriptive,
    Unresolved,
}

/// Identifies an immutable input whose proposal participates in the one-shot
/// GoalIR materialization.  These sources never receive a mutable reference to
/// the compositional analysis.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum LanguageCenterGoalDecisionSourceIR {
    LanguageCenterEvent,
    PragmaticIntentGraph,
    UtteranceIntentGraph,
    IllocutionaryCommitmentGraph,
    ContinuationGate,
    CentralMaterializer,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum LanguageCenterGoalEffectIR {
    SelectLiveGoal,
    RetainNonExecutable,
    SuppressGoal,
    DeferGoal,
    BlockGoal,
    RequireClarification,
    SynthesizeResponseGoal,
    PreserveConstraint,
    MaterializeOnce,
}

/// A proposal is retained even when a higher-precedence proposal wins.  This
/// makes conflict resolution inspectable instead of letting a later module
/// silently overwrite an earlier module's result.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LanguageCenterGoalDecisionIR {
    pub decision_id: String,
    pub source: LanguageCenterGoalDecisionSourceIR,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub event_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub frame_id: Option<String>,
    pub effect: LanguageCenterGoalEffectIR,
    pub precedence: u16,
    pub evidence_refs: Vec<String>,
    pub semantic_authority: bool,
    pub external_execution_authorized: bool,
}

/// Audit record for the single GoalIR projection boundary.  The materialized
/// analysis itself remains in `PragmaticInterpretationIR`; this record binds
/// the immutable inputs to that output without duplicating it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LanguageCenterGoalProjectionIR {
    pub schema: String,
    pub source_language_center_sha256: String,
    pub source_composition_sha256: String,
    pub source_pragmatic_intent_sha256: String,
    pub source_illocution_sha256: String,
    pub materialized_composition_sha256: String,
    pub decisions: Vec<LanguageCenterGoalDecisionIR>,
    pub retained_conflict_ids: Vec<String>,
    pub central_materialization_count: u8,
    pub module_outputs_immutable: bool,
    pub semantic_authority: bool,
    pub language_can_execute: bool,
    pub projection_sha256: String,
}

impl LanguageCenterGoalProjectionIR {
    pub(crate) fn seal(
        center: &LanguageCenterIR,
        source_composition: &CompositionalAnalysisIR,
        pragmatic_intent: &PragmaticIntentGraphIR,
        illocution: &IllocutionaryCommitmentGraphIR,
        materialized_composition: &CompositionalAnalysisIR,
        decisions: Vec<LanguageCenterGoalDecisionIR>,
    ) -> Self {
        let mut projection = Self {
            schema: LANGUAGE_CENTER_GOAL_PROJECTION_SCHEMA.to_string(),
            source_language_center_sha256: center.graph_sha256.clone(),
            source_composition_sha256: compositional_analysis_sha256(source_composition),
            source_pragmatic_intent_sha256: json_sha256(pragmatic_intent),
            source_illocution_sha256: json_sha256(illocution),
            materialized_composition_sha256: compositional_analysis_sha256(
                materialized_composition,
            ),
            decisions,
            retained_conflict_ids: center
                .conflicts
                .iter()
                .map(|conflict| conflict.conflict_id.clone())
                .collect(),
            central_materialization_count: 1,
            module_outputs_immutable: true,
            semantic_authority: false,
            language_can_execute: false,
            projection_sha256: String::new(),
        };
        projection.projection_sha256 = language_center_goal_projection_sha256(&projection);
        projection
    }

    pub fn validate_against(
        &self,
        center: &LanguageCenterIR,
        materialized_composition: &CompositionalAnalysisIR,
    ) -> bool {
        let event_ids = center
            .events
            .iter()
            .map(|event| event.event_id.as_str())
            .collect::<BTreeSet<_>>();
        let frame_ids = materialized_composition
            .frames
            .iter()
            .map(|frame| frame.frame_id.as_str())
            .collect::<BTreeSet<_>>();
        let decision_ids = self
            .decisions
            .iter()
            .map(|decision| decision.decision_id.as_str())
            .collect::<BTreeSet<_>>();
        self.schema == LANGUAGE_CENTER_GOAL_PROJECTION_SCHEMA
            && self.source_language_center_sha256 == center.graph_sha256
            && self.materialized_composition_sha256
                == compositional_analysis_sha256(materialized_composition)
            && self.source_composition_sha256.len() == 64
            && self.source_pragmatic_intent_sha256.len() == 64
            && self.source_illocution_sha256.len() == 64
            && self.central_materialization_count == 1
            && self.module_outputs_immutable
            && !self.semantic_authority
            && !self.language_can_execute
            && !self.decisions.is_empty()
            && decision_ids.len() == self.decisions.len()
            && self.decisions.iter().all(|decision| {
                !decision.decision_id.is_empty()
                    && decision.precedence <= 1_000
                    && !decision.evidence_refs.is_empty()
                    && !decision.semantic_authority
                    && !decision.external_execution_authorized
                    && decision
                        .event_id
                        .as_deref()
                        .is_none_or(|event_id| event_ids.contains(event_id))
                    && decision
                        .frame_id
                        .as_deref()
                        .is_none_or(|frame_id| frame_ids.contains(frame_id))
            })
            && self.retained_conflict_ids
                == center
                    .conflicts
                    .iter()
                    .map(|conflict| conflict.conflict_id.clone())
                    .collect::<Vec<_>>()
            && self.projection_sha256.len() == 64
            && self.projection_sha256 == language_center_goal_projection_sha256(self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LanguageCenterContributionIR {
    pub contribution_id: String,
    pub source: LanguageCenterSourceIR,
    pub event_id: String,
    pub projection: LanguageCenterProjectionIR,
    pub evidence_ref: String,
    pub semantic_authority: bool,
    pub language_can_execute: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LanguageCenterArgumentIR {
    pub argument_id: String,
    pub role: SemanticRoleKindIR,
    pub semantic_keys: Vec<String>,
    pub phenotype_surface: String,
    pub source_node_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LanguageCenterEventIR {
    pub event_id: String,
    pub source_frame_id: String,
    pub canonical_predicate: String,
    pub intent: PlanIntentIR,
    pub arguments: Vec<LanguageCenterArgumentIR>,
    /// Explicit planner target bindings. This prevents a later compatibility
    /// projection from guessing the goal subject from a flattened sentence or
    /// from whichever semantic role happens to sort first.
    pub goal_subject_argument_ids: Vec<String>,
    pub projection: LanguageCenterProjectionIR,
    pub contribution_ids: Vec<String>,
    pub user_request_present: bool,
    pub external_execution_authorized: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LanguageCenterRelationIR {
    pub relation_id: String,
    pub source_event_id: String,
    pub target_event_id: String,
    pub relation: ClauseRelationKindIR,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LanguageCenterConflictIR {
    pub conflict_id: String,
    pub event_id: String,
    pub projections: Vec<LanguageCenterProjectionIR>,
    pub contribution_ids: Vec<String>,
    pub fail_closed_projection: LanguageCenterProjectionIR,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LanguageCenterIR {
    pub schema: String,
    pub input_phenotype: LanguageCodeIR,
    pub events: Vec<LanguageCenterEventIR>,
    pub relations: Vec<LanguageCenterRelationIR>,
    pub contributions: Vec<LanguageCenterContributionIR>,
    pub conflicts: Vec<LanguageCenterConflictIR>,
    pub unresolved: Vec<String>,
    pub projected_goal_event_ids: Vec<String>,
    pub semantic_sha256: String,
    pub graph_sha256: String,
    pub semantic_authority: bool,
    pub language_can_execute: bool,
}

impl Default for LanguageCenterIR {
    fn default() -> Self {
        let mut center = Self {
            schema: LANGUAGE_CENTER_SCHEMA.to_string(),
            input_phenotype: LanguageCodeIR::Unknown,
            events: Vec::new(),
            relations: Vec::new(),
            contributions: Vec::new(),
            conflicts: Vec::new(),
            unresolved: Vec::new(),
            projected_goal_event_ids: Vec::new(),
            semantic_sha256: String::new(),
            graph_sha256: String::new(),
            semantic_authority: false,
            language_can_execute: false,
        };
        center.seal();
        center
    }
}

impl LanguageCenterIR {
    fn seal(&mut self) {
        self.semantic_sha256 = language_center_semantic_sha256(self);
        self.graph_sha256 = language_center_graph_sha256(self);
    }

    pub fn validate(&self) -> bool {
        let event_ids = self
            .events
            .iter()
            .map(|event| event.event_id.as_str())
            .collect::<BTreeSet<_>>();
        let contribution_ids = self
            .contributions
            .iter()
            .map(|item| item.contribution_id.as_str())
            .collect::<BTreeSet<_>>();
        self.schema == LANGUAGE_CENTER_SCHEMA
            && !self.semantic_authority
            && !self.language_can_execute
            && event_ids.len() == self.events.len()
            && contribution_ids.len() == self.contributions.len()
            && self.events.iter().all(|event| {
                !event.event_id.is_empty()
                    && !event.source_frame_id.is_empty()
                    && !event.canonical_predicate.is_empty()
                    && !event.external_execution_authorized
                    && event
                        .contribution_ids
                        .iter()
                        .all(|id| contribution_ids.contains(id.as_str()))
                    && event.arguments.iter().all(|argument| {
                        !argument.argument_id.is_empty()
                            && !argument.semantic_keys.is_empty()
                            && !argument.phenotype_surface.trim().is_empty()
                    })
                    && event.goal_subject_argument_ids.iter().all(|subject_id| {
                        event
                            .arguments
                            .iter()
                            .any(|argument| &argument.argument_id == subject_id)
                    })
            })
            && self.contributions.iter().all(|item| {
                event_ids.contains(item.event_id.as_str())
                    && !item.evidence_ref.trim().is_empty()
                    && !item.semantic_authority
                    && !item.language_can_execute
            })
            && self.relations.iter().all(|relation| {
                event_ids.contains(relation.source_event_id.as_str())
                    && event_ids.contains(relation.target_event_id.as_str())
                    && relation.source_event_id != relation.target_event_id
            })
            && self
                .projected_goal_event_ids
                .iter()
                .all(|id| event_ids.contains(id.as_str()))
            && self.conflicts.iter().all(|conflict| {
                event_ids.contains(conflict.event_id.as_str())
                    && conflict
                        .contribution_ids
                        .iter()
                        .all(|id| contribution_ids.contains(id.as_str()))
                    && conflict.projections.len() > 1
            })
            && self.semantic_sha256.len() == 64
            && self.graph_sha256.len() == 64
            && self.semantic_sha256 == language_center_semantic_sha256(self)
            && self.graph_sha256 == language_center_graph_sha256(self)
    }

    /// Projects the complete language-neutral event graph into the semantic
    /// planner boundary. Unlike `LanguageUnderstandingIR`, this does not join
    /// subjects or encode scope and relations as strings.
    pub fn to_semantic_plan_goal(
        &self,
        goal_id: &str,
        context_semantic_ids: &[String],
        max_steps_per_event: usize,
        materialized_composition: &CompositionalAnalysisIR,
        native: Option<&NativeTurnIR>,
        inferred_goal: Option<&InferredPragmaticGoalIR>,
    ) -> Option<SemanticPlanGoalIR> {
        if !self.validate() {
            return None;
        }
        let mut context_semantics = context_semantic_ids
            .iter()
            .filter(|concept| !concept.trim().is_empty())
            .cloned()
            .collect::<Vec<_>>();
        context_semantics.sort();
        context_semantics.dedup();
        let mut arguments: Vec<SemanticPlanArgumentIR> = self
            .events
            .iter()
            .flat_map(|event| &event.arguments)
            .map(|argument| SemanticPlanArgumentIR {
                argument_id: argument.argument_id.clone(),
                role: semantic_plan_role(argument.role),
                concept_ids: argument.semantic_keys.clone(),
                grounded_label: argument.phenotype_surface.clone(),
            })
            .collect::<Vec<_>>();
        let selected_frame_ids = materialized_composition
            .selected_candidates()
            .into_iter()
            .map(|candidate| candidate.source_frame_id.as_str())
            .collect::<BTreeSet<_>>();
        let mut events: Vec<SemanticPlanEventIR> = self
            .events
            .iter()
            .map(|event| {
                let centrally_selected =
                    selected_frame_ids.contains(event.source_frame_id.as_str());
                SemanticPlanEventIR {
                    event_id: event.event_id.clone(),
                    predicate_concept_id: event.canonical_predicate.clone(),
                    intent: event.intent,
                    argument_ids: event
                        .arguments
                        .iter()
                        .map(|argument| argument.argument_id.clone())
                        .collect(),
                    goal_subject_argument_ids: event.goal_subject_argument_ids.clone(),
                    projection: if centrally_selected {
                        SemanticPlanProjectionIR::LiveRequest
                    } else {
                        semantic_plan_projection(event.projection)
                    },
                    user_request_present: event.user_request_present || centrally_selected,
                    external_execution_authorized: false,
                }
            })
            .collect();
        let relations = self
            .relations
            .iter()
            .map(|relation| SemanticPlanRelationIR {
                relation_id: relation.relation_id.clone(),
                source_event_id: relation.source_event_id.clone(),
                target_event_id: relation.target_event_id.clone(),
                relation: semantic_plan_relation(relation.relation),
            })
            .collect();
        // The one-shot central materializer is authoritative for which
        // compositional candidates survived all module proposals.  The raw
        // Language Center retains every conflict for audit, but a lower-level
        // Suppressed/Reported proposal must not erase that final selection.
        let native_owns_goal_selection = native.is_some_and(|native| {
            !native.selected_live_goals.is_empty() && native.unresolved.is_empty()
        });
        let mut selected_live_event_ids = if native_owns_goal_selection {
            Vec::new()
        } else {
            self.events
                .iter()
                .filter(|event| selected_frame_ids.contains(event.source_frame_id.as_str()))
                .map(|event| event.event_id.clone())
                .collect::<Vec<_>>()
        };
        if let Some(native) = native {
            for native_goal in &native.selected_live_goals {
                let represented_event_id = semantic_event_for_goal(
                    &events,
                    &arguments,
                    &selected_live_event_ids,
                    native_goal.intent,
                    &native_goal.subject,
                );
                if let Some(event_id) = represented_event_id {
                    if !selected_live_event_ids.contains(&event_id) {
                        selected_live_event_ids.push(event_id.clone());
                    }
                    if let Some(event) = events.iter_mut().find(|event| event.event_id == event_id)
                    {
                        event.projection = SemanticPlanProjectionIR::LiveRequest;
                        event.user_request_present = true;
                    }
                } else {
                    append_supplemental_semantic_goal(
                        &mut events,
                        &mut arguments,
                        &mut selected_live_event_ids,
                        "NATIVE",
                        &native_goal.goal_id,
                        &native_goal.canonical_predicate,
                        native_goal.intent,
                        &native_goal.subject,
                        &native_goal.subject_concepts,
                    );
                }
            }
        }
        // Once the native circuit has selected a complete live-goal set, peer
        // analyzers remain evidence producers. They may enrich an aligned
        // event but cannot append another selected event after arbitration.
        // Without a native selection, the pragmatic inference path remains a
        // bounded fill-only source.
        if native_owns_goal_selection {
            let selected = selected_live_event_ids.iter().collect::<BTreeSet<_>>();
            for event in &mut events {
                if selected.contains(&event.event_id) {
                    event.projection = SemanticPlanProjectionIR::LiveRequest;
                    event.user_request_present = true;
                } else if let Some(source_event) = self
                    .events
                    .iter()
                    .find(|source_event| source_event.event_id == event.event_id)
                {
                    event.projection = semantic_plan_projection(source_event.projection);
                    event.user_request_present = source_event.user_request_present;
                }
            }
        } else if let Some(inferred) = inferred_goal {
            let represented_event_id = events
                .iter()
                .find(|event| {
                    semantic_event_represents_goal(
                        event,
                        &arguments,
                        inferred.intent,
                        &inferred.subject,
                    )
                })
                .map(|event| event.event_id.clone());
            if let Some(event_id) = represented_event_id {
                if !selected_live_event_ids.contains(&event_id) {
                    selected_live_event_ids.push(event_id.clone());
                }
                if let Some(event) = events.iter_mut().find(|event| event.event_id == event_id) {
                    event.projection = SemanticPlanProjectionIR::LiveRequest;
                    event.user_request_present = true;
                }
            } else {
                // A context-restored pragmatic goal is a resolution of an
                // underspecified surface argument, not an additional action.
                // Remove same-intent surface candidates from the selected set
                // before materializing the resolved goal, while retaining
                // their source events in the graph for audit.
                let displaced = selected_live_event_ids
                    .iter()
                    .filter(|event_id| {
                        events.iter().any(|event| {
                            &event.event_id == *event_id && event.intent == inferred.intent
                        })
                    })
                    .cloned()
                    .collect::<BTreeSet<_>>();
                selected_live_event_ids.retain(|event_id| !displaced.contains(event_id));
                for event in &mut events {
                    if displaced.contains(&event.event_id) {
                        if let Some(source_event) = self
                            .events
                            .iter()
                            .find(|source_event| source_event.event_id == event.event_id)
                        {
                            event.projection = semantic_plan_projection(source_event.projection);
                            event.user_request_present = source_event.user_request_present;
                        }
                    }
                }
                append_supplemental_semantic_goal(
                    &mut events,
                    &mut arguments,
                    &mut selected_live_event_ids,
                    "PRAGMATIC",
                    "INFERRED-GOAL",
                    &format!("{:?}", inferred.intent).to_uppercase(),
                    inferred.intent,
                    &inferred.subject,
                    &[],
                );
            }
        }
        selected_live_event_ids.sort_by_key(|event_id| {
            events
                .iter()
                .position(|event| &event.event_id == event_id)
                .unwrap_or(usize::MAX)
        });
        selected_live_event_ids.dedup();
        if selected_live_event_ids.is_empty() {
            return None;
        }
        let mut goal = SemanticPlanGoalIR {
            schema: SEMANTIC_PLAN_GOAL_SCHEMA.to_string(),
            goal_id: goal_id.to_string(),
            events,
            arguments,
            relations,
            selected_live_event_ids,
            context_semantic_ids: context_semantics,
            source_semantic_sha256: self.semantic_sha256.clone(),
            max_steps_per_event,
            semantic_authority: false,
            language_can_execute: false,
            semantic_sha256: String::new(),
        };
        goal.seal();
        goal.validate().then_some(goal)
    }
}

fn semantic_event_represents_goal(
    event: &SemanticPlanEventIR,
    arguments: &[SemanticPlanArgumentIR],
    intent: PlanIntentIR,
    subject: &str,
) -> bool {
    event.intent == intent
        && event.goal_subject_argument_ids.iter().any(|argument_id| {
            arguments.iter().any(|argument| {
                &argument.argument_id == argument_id
                    && subjects_share_context_concept(&argument.grounded_label, subject)
            })
        })
}

fn semantic_event_for_goal(
    events: &[SemanticPlanEventIR],
    arguments: &[SemanticPlanArgumentIR],
    already_selected: &[String],
    intent: PlanIntentIR,
    subject: &str,
) -> Option<String> {
    let unclaimed = |event: &&SemanticPlanEventIR| {
        event.intent == intent && !already_selected.contains(&event.event_id)
    };
    events
        .iter()
        .filter(unclaimed)
        .find(|event| {
            event.goal_subject_argument_ids.iter().any(|argument_id| {
                arguments.iter().any(|argument| {
                    &argument.argument_id == argument_id
                        && argument.grounded_label.eq_ignore_ascii_case(subject)
                })
            })
        })
        .or_else(|| {
            events
                .iter()
                .filter(unclaimed)
                .find(|event| semantic_event_represents_goal(event, arguments, intent, subject))
        })
        .map(|event| event.event_id.clone())
}

#[allow(clippy::too_many_arguments)]
fn append_supplemental_semantic_goal(
    events: &mut Vec<SemanticPlanEventIR>,
    arguments: &mut Vec<SemanticPlanArgumentIR>,
    selected_live_event_ids: &mut Vec<String>,
    source: &str,
    source_id: &str,
    predicate: &str,
    intent: PlanIntentIR,
    subject: &str,
    concept_ids: &[String],
) {
    if subject.trim().is_empty() {
        return;
    }
    let suffix = events.len() + 1;
    let event_id = format!("SEMANTIC-SUPPLEMENT-{source}-{suffix:03}");
    let argument_id = format!("{event_id}-SUBJECT");
    let mut concepts = concept_ids.to_vec();
    concepts.extend(phenotype_semantic_keys(LanguageCodeIR::Unknown, subject));
    concepts.sort();
    concepts.dedup();
    if concepts.is_empty() {
        concepts.push(format!("DISCOURSE_ENTITY:{source_id}"));
    }
    arguments.push(SemanticPlanArgumentIR {
        argument_id: argument_id.clone(),
        role: SemanticPlanRoleIR::Topic,
        concept_ids: concepts,
        grounded_label: subject.to_string(),
    });
    events.push(SemanticPlanEventIR {
        event_id: event_id.clone(),
        predicate_concept_id: predicate.to_string(),
        intent,
        argument_ids: vec![argument_id.clone()],
        goal_subject_argument_ids: vec![argument_id],
        projection: SemanticPlanProjectionIR::LiveRequest,
        user_request_present: true,
        external_execution_authorized: false,
    });
    selected_live_event_ids.push(event_id);
}

fn semantic_plan_role(role: SemanticRoleKindIR) -> SemanticPlanRoleIR {
    match role {
        SemanticRoleKindIR::Agent => SemanticPlanRoleIR::Agent,
        SemanticRoleKindIR::Topic => SemanticPlanRoleIR::Topic,
        SemanticRoleKindIR::Theme => SemanticPlanRoleIR::Theme,
        SemanticRoleKindIR::CoTheme => SemanticPlanRoleIR::CoTheme,
        SemanticRoleKindIR::Patient => SemanticPlanRoleIR::Patient,
        SemanticRoleKindIR::Experiencer => SemanticPlanRoleIR::Experiencer,
        SemanticRoleKindIR::Recipient => SemanticPlanRoleIR::Recipient,
        SemanticRoleKindIR::Source => SemanticPlanRoleIR::Source,
        SemanticRoleKindIR::Destination => SemanticPlanRoleIR::Destination,
        SemanticRoleKindIR::Instrument => SemanticPlanRoleIR::Instrument,
        SemanticRoleKindIR::Location => SemanticPlanRoleIR::Location,
        SemanticRoleKindIR::Result => SemanticPlanRoleIR::Result,
        SemanticRoleKindIR::ComparisonPeer => SemanticPlanRoleIR::ComparisonPeer,
        SemanticRoleKindIR::PriorResult => SemanticPlanRoleIR::PriorResult,
    }
}

fn semantic_plan_projection(projection: LanguageCenterProjectionIR) -> SemanticPlanProjectionIR {
    match projection {
        LanguageCenterProjectionIR::Prohibited => SemanticPlanProjectionIR::Prohibited,
        LanguageCenterProjectionIR::Conditional => SemanticPlanProjectionIR::Conditional,
        LanguageCenterProjectionIR::Reported => SemanticPlanProjectionIR::Reported,
        LanguageCenterProjectionIR::Suppressed => SemanticPlanProjectionIR::Suppressed,
        LanguageCenterProjectionIR::LiveRequest => SemanticPlanProjectionIR::LiveRequest,
        LanguageCenterProjectionIR::Advisory => SemanticPlanProjectionIR::Advisory,
        LanguageCenterProjectionIR::Inquiry => SemanticPlanProjectionIR::Inquiry,
        LanguageCenterProjectionIR::Descriptive => SemanticPlanProjectionIR::Descriptive,
        LanguageCenterProjectionIR::Unresolved => SemanticPlanProjectionIR::Unresolved,
    }
}

fn semantic_plan_relation(relation: ClauseRelationKindIR) -> SemanticPlanRelationKindIR {
    match relation {
        ClauseRelationKindIR::Coordination => SemanticPlanRelationKindIR::Coordination,
        ClauseRelationKindIR::Sequence => SemanticPlanRelationKindIR::Sequence,
        ClauseRelationKindIR::Condition => SemanticPlanRelationKindIR::Condition,
        ClauseRelationKindIR::Cause => SemanticPlanRelationKindIR::Cause,
        ClauseRelationKindIR::Purpose => SemanticPlanRelationKindIR::Purpose,
        ClauseRelationKindIR::Contrast => SemanticPlanRelationKindIR::Contrast,
        ClauseRelationKindIR::TemporalBefore => SemanticPlanRelationKindIR::TemporalBefore,
    }
}

pub struct LanguageCenterSources<'a> {
    pub phenotype: LanguageCodeIR,
    pub native: Option<&'a NativeTurnIR>,
    pub composition: &'a CompositionalAnalysisIR,
    pub pragmatic_intent: &'a PragmaticIntentGraphIR,
    pub illocution: &'a IllocutionaryCommitmentGraphIR,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct LanguageCenterPipeline;

impl LanguageCenterPipeline {
    pub fn build(&self, sources: LanguageCenterSources<'_>) -> LanguageCenterIR {
        let mut contributions = Vec::new();
        let frame_to_event = sources
            .composition
            .frames
            .iter()
            .enumerate()
            .map(|(index, frame)| {
                (
                    frame.frame_id.clone(),
                    format!("LANGUAGE-CENTER-EVENT-{:03}", index + 1),
                )
            })
            .collect::<BTreeMap<_, _>>();

        for frame in &sources.composition.frames {
            let event_id = frame_to_event
                .get(&frame.frame_id)
                .expect("frame event allocated");
            push_contribution(
                &mut contributions,
                LanguageCenterSourceIR::CompositionalFrame,
                event_id,
                frame_projection(frame),
                format!("FRAME:{}", frame.frame_id),
            );
            if let Some(clause) = sources
                .composition
                .clause_graph
                .node_for_frame(&frame.frame_id)
            {
                let projection = match clause.function {
                    ClauseFunctionIR::Condition | ClauseFunctionIR::Temporal => {
                        LanguageCenterProjectionIR::Conditional
                    }
                    ClauseFunctionIR::Cause
                    | ClauseFunctionIR::Purpose
                    | ClauseFunctionIR::Concession => LanguageCenterProjectionIR::Descriptive,
                    ClauseFunctionIR::Main | ClauseFunctionIR::Coordinate => {
                        LanguageCenterProjectionIR::LiveRequest
                    }
                };
                push_contribution(
                    &mut contributions,
                    LanguageCenterSourceIR::ClauseGraph,
                    event_id,
                    projection,
                    format!("CLAUSE:{}:{:?}", clause.clause_id, clause.function),
                );
            }
            if sources
                .composition
                .grammatical_scope_graph
                .nodes
                .iter()
                .any(|node| {
                    node.reference_id.as_deref() == Some(frame.frame_id.as_str())
                        && matches!(
                            node.kind,
                            crate::grammatical_scope::GrammaticalScopeNodeKindIR::Negation
                                | crate::grammatical_scope::GrammaticalScopeNodeKindIR::Restriction
                        )
                })
            {
                push_contribution(
                    &mut contributions,
                    LanguageCenterSourceIR::GrammaticalScopeGraph,
                    event_id,
                    LanguageCenterProjectionIR::Prohibited,
                    format!("GRAMMATICAL_SCOPE:{}", frame.frame_id),
                );
            }
        }

        if let Some(graph) = &sources.pragmatic_intent.composition {
            for node in &graph.nodes {
                let Some(event_id) = frame_to_event.get(&node.source_frame_id) else {
                    continue;
                };
                let conditional = graph.context_scopes.iter().any(|scope| {
                    scope.target_node_id == node.node_id
                        && scope.kind
                            == crate::pragmatic_intent::PragmaticIntentRelationKindIR::Conditions
                });
                let projection = if conditional {
                    LanguageCenterProjectionIR::Conditional
                } else {
                    match node.projection {
                        PragmaticGoalProjectionIR::AuthorizedRequest => {
                            LanguageCenterProjectionIR::LiveRequest
                        }
                        PragmaticGoalProjectionIR::AdvisoryOnly => {
                            LanguageCenterProjectionIR::Advisory
                        }
                        PragmaticGoalProjectionIR::Suppressed => {
                            LanguageCenterProjectionIR::Suppressed
                        }
                    }
                };
                push_contribution(
                    &mut contributions,
                    LanguageCenterSourceIR::PragmaticIntentGraph,
                    event_id,
                    projection,
                    format!("PRAGMATIC_NODE:{}", node.node_id),
                );
            }
        }

        for commitment in &sources.illocution.commitments {
            let candidates = matching_commitment_events(
                commitment,
                &sources.composition.frames,
                &frame_to_event,
            );
            for event_id in candidates {
                let projection = match (commitment.force, commitment.activation) {
                    (
                        IllocutionaryForceIR::IndirectActionRequest,
                        CommitmentActivationIR::Immediate,
                    ) => LanguageCenterProjectionIR::LiveRequest,
                    (IllocutionaryForceIR::DeferredConditionalRequest, _) => {
                        LanguageCenterProjectionIR::Conditional
                    }
                    (IllocutionaryForceIR::ReportedCommitment, _) => {
                        LanguageCenterProjectionIR::Reported
                    }
                    (
                        IllocutionaryForceIR::GoalWithdrawal
                        | IllocutionaryForceIR::OutcomeClaimConstraint,
                        _,
                    ) => LanguageCenterProjectionIR::Suppressed,
                    _ => LanguageCenterProjectionIR::Descriptive,
                };
                push_contribution(
                    &mut contributions,
                    LanguageCenterSourceIR::IllocutionaryCommitmentGraph,
                    &event_id,
                    projection,
                    format!("ILLOCUTION:{}", commitment.commitment_id),
                );
            }
        }

        let mut native_goal_subjects = BTreeMap::<String, (String, Vec<String>, String)>::new();
        if let Some(native) = sources.native {
            // Native and compositional analyzers use different local IDs.
            // Reconcile them by semantic predicate and source position while
            // consuming each compositional frame at most once.  A first-match
            // lookup silently merged repeated predicates into one event and
            // was therefore another order-dependent overwrite path.
            let mut matched_frame_ids = BTreeSet::new();
            for native_event in &native.events {
                let Some(frame) = sources
                    .composition
                    .frames
                    .iter()
                    .filter(|frame| {
                        frame.canonical_predicate == native_event.canonical_predicate
                            && frame_to_event.contains_key(&frame.frame_id)
                            && !matched_frame_ids.contains(&frame.frame_id)
                    })
                    .min_by_key(|frame| frame.source_start_byte.abs_diff(native_event.start_byte))
                else {
                    continue;
                };
                matched_frame_ids.insert(frame.frame_id.clone());
                let event_id = frame_to_event
                    .get(&frame.frame_id)
                    .expect("matched native frame has an event");
                let selected = native
                    .selected_live_goals
                    .iter()
                    .any(|goal| goal.source_event_id == native_event.event_id);
                if let Some(goal) = native
                    .selected_live_goals
                    .iter()
                    .find(|goal| goal.source_event_id == native_event.event_id)
                {
                    let mut semantic_keys = goal.subject_concepts.clone();
                    semantic_keys.extend(phenotype_semantic_keys(sources.phenotype, &goal.subject));
                    semantic_keys.sort();
                    semantic_keys.dedup();
                    native_goal_subjects.insert(
                        frame.frame_id.clone(),
                        (goal.subject.clone(), semantic_keys, goal.goal_id.clone()),
                    );
                }
                let projection = match native_event.scope {
                    NativeEventScopeIR::Live if selected => LanguageCenterProjectionIR::LiveRequest,
                    NativeEventScopeIR::Live => LanguageCenterProjectionIR::Unresolved,
                    NativeEventScopeIR::Conditional => LanguageCenterProjectionIR::Conditional,
                    NativeEventScopeIR::Prohibited => LanguageCenterProjectionIR::Prohibited,
                    NativeEventScopeIR::Reported => LanguageCenterProjectionIR::Reported,
                    NativeEventScopeIR::Possible => LanguageCenterProjectionIR::Advisory,
                };
                push_contribution(
                    &mut contributions,
                    LanguageCenterSourceIR::NativeCircuit,
                    event_id,
                    projection,
                    format!("NATIVE_EVENT:{}", native_event.event_id),
                );
            }
        }

        // A prohibited predicate may omit its object while the replacement
        // request supplies one unambiguous discourse-grounded subject.  Keep
        // this as a typed ellipsis binding at the Language Center boundary.
        // Individual analyzers remain immutable evidence producers, and no
        // downstream planner or realizer has to guess a target from surface
        // text. Multiple distinct subjects deliberately leave the gap open.
        let mut contextual_subjects = native_goal_subjects
            .values()
            .map(|(surface, semantic_keys, source_node_id)| {
                (
                    semantic_keys.clone(),
                    (
                        surface.clone(),
                        semantic_keys.clone(),
                        source_node_id.clone(),
                    ),
                )
            })
            .collect::<BTreeMap<_, _>>();
        let unambiguous_native_subject_for_ellipsis = (contextual_subjects.len() == 1).then(|| {
            contextual_subjects
                .pop_first()
                .expect("one contextual subject")
                .1
        });

        let mut conflicts = Vec::new();
        let mut events = sources
            .composition
            .frames
            .iter()
            .map(|frame| {
                let event_id = frame_to_event
                    .get(&frame.frame_id)
                    .expect("frame event allocated")
                    .clone();
                let event_contributions = contributions
                    .iter()
                    .filter(|item| item.event_id == event_id)
                    .collect::<Vec<_>>();
                let projection = resolve_projection(&event_contributions);
                let projections = event_contributions
                    .iter()
                    .map(|item| item.projection)
                    .collect::<BTreeSet<_>>();
                if incompatible_projections(&projections) {
                    conflicts.push(LanguageCenterConflictIR {
                        conflict_id: format!("LANGUAGE-CENTER-CONFLICT-{:03}", conflicts.len() + 1),
                        event_id: event_id.clone(),
                        projections: projections.iter().copied().collect(),
                        contribution_ids: event_contributions
                            .iter()
                            .map(|item| item.contribution_id.clone())
                            .collect(),
                        fail_closed_projection: projection,
                    });
                }
                let mut arguments =
                    arguments_for_frame(sources.phenotype, frame, sources.composition, &event_id);
                let goal_subject = native_goal_subjects
                    .get(&frame.frame_id)
                    .cloned()
                    .or_else(|| {
                        sources
                            .composition
                            .selected_candidates()
                            .into_iter()
                            .find(|candidate| candidate.source_frame_id == frame.frame_id)
                            .map(|candidate| {
                                (
                                    candidate.subject.clone(),
                                    phenotype_semantic_keys(sources.phenotype, &candidate.subject),
                                    candidate.candidate_id.clone(),
                                )
                            })
                    })
                    .or_else(|| {
                        (!frame.theme.trim().is_empty()).then(|| {
                            (
                                frame.theme.clone(),
                                phenotype_semantic_keys(sources.phenotype, &frame.theme),
                                format!("{}:FRAME_THEME", frame.frame_id),
                            )
                        })
                    })
                    .or_else(|| {
                        (projection == LanguageCenterProjectionIR::Prohibited
                            && frame.theme.trim().is_empty())
                        .then(|| {
                            unambiguous_native_subject_for_ellipsis.as_ref().map(
                                |(surface, semantic_keys, source_node_id)| {
                                    (
                                        surface.clone(),
                                        semantic_keys.clone(),
                                        format!("{source_node_id}:OMITTED_PROHIBITED_ARGUMENT"),
                                    )
                                },
                            )
                        })
                        .flatten()
                    });
                let goal_subject_argument_ids = goal_subject
                    .and_then(|(surface, semantic_keys, source_node_id)| {
                        ensure_goal_subject_argument(
                            &mut arguments,
                            &event_id,
                            &surface,
                            semantic_keys,
                            &source_node_id,
                        )
                    })
                    .into_iter()
                    .collect();
                LanguageCenterEventIR {
                    event_id,
                    source_frame_id: frame.frame_id.clone(),
                    canonical_predicate: frame.canonical_predicate.clone(),
                    intent: frame.intent_hint,
                    arguments,
                    goal_subject_argument_ids,
                    projection,
                    contribution_ids: event_contributions
                        .iter()
                        .map(|item| item.contribution_id.clone())
                        .collect(),
                    user_request_present: event_contributions
                        .iter()
                        .any(|item| item.projection == LanguageCenterProjectionIR::LiveRequest),
                    external_execution_authorized: false,
                }
            })
            .collect::<Vec<_>>();
        events.sort_by(|left, right| left.event_id.cmp(&right.event_id));

        let mut relations = sources
            .composition
            .clause_graph
            .edges
            .iter()
            .filter_map(|edge| {
                let source_frame = sources
                    .composition
                    .clause_graph
                    .nodes
                    .iter()
                    .find(|node| node.clause_id == edge.source_clause_id)?
                    .anchor_frame_id
                    .as_str();
                let target_frame = sources
                    .composition
                    .clause_graph
                    .nodes
                    .iter()
                    .find(|node| node.clause_id == edge.target_clause_id)?
                    .anchor_frame_id
                    .as_str();
                Some(LanguageCenterRelationIR {
                    relation_id: String::new(),
                    source_event_id: frame_to_event.get(source_frame)?.clone(),
                    target_event_id: frame_to_event.get(target_frame)?.clone(),
                    relation: edge.relation,
                })
            })
            .collect::<Vec<_>>();
        if let Some(goal_graph) = &sources.composition.goal_graph {
            relations.extend(goal_graph.edges.iter().filter_map(|edge| {
                let source_candidate_id = goal_graph
                    .nodes
                    .iter()
                    .find(|node| node.node_id == edge.source_node_id)?
                    .candidate_id
                    .as_str();
                let target_candidate_id = goal_graph
                    .nodes
                    .iter()
                    .find(|node| node.node_id == edge.target_node_id)?
                    .candidate_id
                    .as_str();
                let source_frame_id = sources
                    .composition
                    .candidates
                    .iter()
                    .find(|candidate| candidate.candidate_id == source_candidate_id)?
                    .source_frame_id
                    .as_str();
                let target_frame_id = sources
                    .composition
                    .candidates
                    .iter()
                    .find(|candidate| candidate.candidate_id == target_candidate_id)?
                    .source_frame_id
                    .as_str();
                Some(LanguageCenterRelationIR {
                    relation_id: String::new(),
                    source_event_id: frame_to_event.get(source_frame_id)?.clone(),
                    target_event_id: frame_to_event.get(target_frame_id)?.clone(),
                    relation: match edge.relation {
                        crate::compositional_semantics::GoalGraphRelationKindIR::Sequence => {
                            ClauseRelationKindIR::Sequence
                        }
                        crate::compositional_semantics::GoalGraphRelationKindIR::Coordination => {
                            ClauseRelationKindIR::Coordination
                        }
                    },
                })
            }));
        }
        relations.sort_by(|left, right| {
            left.source_event_id
                .cmp(&right.source_event_id)
                .then_with(|| left.target_event_id.cmp(&right.target_event_id))
                .then_with(|| left.relation.cmp(&right.relation))
        });
        relations.dedup_by(|left, right| {
            left.source_event_id == right.source_event_id
                && left.target_event_id == right.target_event_id
                && left.relation == right.relation
        });
        for (index, relation) in relations.iter_mut().enumerate() {
            relation.relation_id = format!("LANGUAGE-CENTER-RELATION-{:03}", index + 1);
        }

        let projected_goal_event_ids = events
            .iter()
            .filter(|event| {
                event.projection == LanguageCenterProjectionIR::LiveRequest
                    && !event.goal_subject_argument_ids.is_empty()
            })
            .map(|event| event.event_id.clone())
            .collect::<Vec<_>>();
        let mut unresolved = sources
            .composition
            .unresolved_competitions
            .iter()
            .chain(sources.pragmatic_intent.unresolved_ambiguities.iter())
            .chain(
                sources
                    .native
                    .into_iter()
                    .flat_map(|native| native.unresolved.iter()),
            )
            .cloned()
            .collect::<Vec<_>>();
        unresolved.sort();
        unresolved.dedup();
        unresolved.extend(
            events
                .iter()
                .filter(|event| {
                    event.projection == LanguageCenterProjectionIR::LiveRequest
                        && event.goal_subject_argument_ids.is_empty()
                })
                .map(|event| format!("UNBOUND_LIVE_EVENT:{}", event.event_id)),
        );
        unresolved.sort();
        unresolved.dedup();
        let mut center = LanguageCenterIR {
            schema: LANGUAGE_CENTER_SCHEMA.to_string(),
            input_phenotype: sources.phenotype,
            events,
            relations,
            contributions,
            conflicts,
            unresolved,
            projected_goal_event_ids,
            semantic_sha256: String::new(),
            graph_sha256: String::new(),
            semantic_authority: false,
            language_can_execute: false,
        };
        center.seal();
        debug_assert!(center.validate());
        center
    }
}

fn push_contribution(
    contributions: &mut Vec<LanguageCenterContributionIR>,
    source: LanguageCenterSourceIR,
    event_id: &str,
    projection: LanguageCenterProjectionIR,
    evidence_ref: String,
) {
    contributions.push(LanguageCenterContributionIR {
        contribution_id: format!(
            "LANGUAGE-CENTER-CONTRIBUTION-{:03}",
            contributions.len() + 1
        ),
        source,
        event_id: event_id.to_string(),
        projection,
        evidence_ref,
        semantic_authority: false,
        language_can_execute: false,
    });
}

fn frame_projection(frame: &PredicateFrameIR) -> LanguageCenterProjectionIR {
    if frame.polarity == FramePolarityIR::Negative {
        return LanguageCenterProjectionIR::Prohibited;
    }
    match frame.mood {
        FrameMoodIR::Conditional | FrameMoodIR::Counterfactual => {
            LanguageCenterProjectionIR::Conditional
        }
        FrameMoodIR::Reported | FrameMoodIR::RelativeClause => LanguageCenterProjectionIR::Reported,
        FrameMoodIR::Imperative => LanguageCenterProjectionIR::LiveRequest,
        FrameMoodIR::Interrogative => LanguageCenterProjectionIR::Inquiry,
        FrameMoodIR::Declarative => LanguageCenterProjectionIR::Descriptive,
    }
}

fn matching_commitment_events(
    commitment: &crate::pragmatics::IllocutionaryCommitmentIR,
    frames: &[PredicateFrameIR],
    frame_to_event: &BTreeMap<String, String>,
) -> Vec<String> {
    let surface = commitment.proposition_surface.to_lowercase();
    let mut matches = frames
        .iter()
        .filter(|frame| {
            surface.contains(&frame.predicate_surface.to_lowercase())
                || surface.contains(&frame.canonical_predicate.to_lowercase())
        })
        .filter_map(|frame| frame_to_event.get(&frame.frame_id).cloned())
        .collect::<Vec<_>>();
    if matches.is_empty() && frames.len() == 1 {
        matches.extend(
            frames
                .first()
                .and_then(|frame| frame_to_event.get(&frame.frame_id))
                .cloned(),
        );
    }
    matches.sort();
    matches.dedup();
    matches
}

fn arguments_for_frame(
    phenotype: LanguageCodeIR,
    frame: &PredicateFrameIR,
    composition: &CompositionalAnalysisIR,
    event_id: &str,
) -> Vec<LanguageCenterArgumentIR> {
    let mut arguments = composition
        .semantic_role_graph
        .arguments_for_frame(&frame.frame_id)
        .into_iter()
        .filter(|(_, node)| {
            matches!(
                node.kind,
                crate::semantic_roles::SemanticNodeKindIR::Entity
                    | crate::semantic_roles::SemanticNodeKindIR::ImplicitAgent
            )
        })
        .map(|(role, node)| {
            let semantic_keys =
                if node.kind == crate::semantic_roles::SemanticNodeKindIR::ImplicitAgent {
                    vec!["C_DIALOGUE_ASSISTANT".to_string()]
                } else {
                    phenotype_semantic_keys(phenotype, &node.normalized_label)
                };
            LanguageCenterArgumentIR {
                argument_id: String::new(),
                role,
                semantic_keys,
                phenotype_surface: node.surface.clone(),
                source_node_id: node.node_id.clone(),
            }
        })
        .filter(|argument| !argument.semantic_keys.is_empty())
        .collect::<Vec<_>>();
    if arguments.is_empty() && !frame.theme.trim().is_empty() {
        let semantic_keys = phenotype_semantic_keys(phenotype, &frame.theme);
        if !semantic_keys.is_empty() {
            arguments.push(LanguageCenterArgumentIR {
                argument_id: String::new(),
                role: SemanticRoleKindIR::Theme,
                semantic_keys,
                phenotype_surface: frame.theme.clone(),
                source_node_id: format!("{event_id}:THEME_FALLBACK"),
            });
        }
    }
    arguments.sort_by(|left, right| {
        left.role
            .cmp(&right.role)
            .then_with(|| left.semantic_keys.cmp(&right.semantic_keys))
    });
    arguments.dedup_by(|left, right| {
        left.role == right.role && left.semantic_keys == right.semantic_keys
    });
    for (index, argument) in arguments.iter_mut().enumerate() {
        argument.argument_id = format!("{event_id}-ARG-{:03}", index + 1);
    }
    arguments
}

fn ensure_goal_subject_argument(
    arguments: &mut Vec<LanguageCenterArgumentIR>,
    event_id: &str,
    surface: &str,
    mut semantic_keys: Vec<String>,
    source_node_id: &str,
) -> Option<String> {
    if surface.trim().is_empty() {
        return None;
    }
    semantic_keys.retain(|key| !key.trim().is_empty());
    semantic_keys.sort();
    semantic_keys.dedup();
    if semantic_keys.is_empty() {
        return None;
    }
    if let Some(argument) = arguments.iter().find(|argument| {
        argument.phenotype_surface.eq_ignore_ascii_case(surface)
            || argument.semantic_keys == semantic_keys
    }) {
        return Some(argument.argument_id.clone());
    }
    let argument_id = format!("{event_id}-GOAL-SUBJECT-{:03}", arguments.len() + 1);
    arguments.push(LanguageCenterArgumentIR {
        argument_id: argument_id.clone(),
        role: SemanticRoleKindIR::Topic,
        semantic_keys,
        phenotype_surface: surface.to_string(),
        source_node_id: source_node_id.to_string(),
    });
    Some(argument_id)
}

fn phenotype_semantic_keys(_phenotype: LanguageCodeIR, surface: &str) -> Vec<String> {
    let mut keys = surface
        .split_whitespace()
        .filter_map(|raw| {
            let token = strip_korean_particle(
                raw.trim_matches(|character: char| {
                    character.is_ascii_punctuation() || matches!(character, '“' | '”' | '‘' | '’')
                })
                .to_lowercase()
                .as_str(),
            )
            .to_string();
            if token.is_empty()
                || matches!(
                    token.as_str(),
                    "the" | "a" | "an" | "only" | "just" | "now" | "지금" | "바로" | "좀"
                )
            {
                return None;
            }
            let concept = match token.as_str() {
                "it" | "that" | "one" | "그거" | "그것" | "그걸" => "C_DISCOURSE_REFERENCE",
                "former" | "latter" | "전자" | "후자" => "C_ORDERED_DISCOURSE_REFERENCE",
                "prior_result" => "C_PRIOR_RESULT",
                "cache" | "캐시" => "C_OBJECT_CACHE",
                "log" | "로그" => "C_OBJECT_LOG",
                "queue" | "큐" => "C_OBJECT_QUEUE",
                "worker" | "워커" => "C_OBJECT_WORKER",
                "service" | "서비스" => "C_OBJECT_SERVICE",
                "report" | "보고서" => "C_OBJECT_REPORT",
                "migration" | "마이그레이션" => "C_PROCESS_MIGRATION",
                "index" | "인덱스" => "C_OBJECT_INDEX",
                "result" | "결과" => "C_RESULT",
                "cause" | "원인" | "why" => "C_CAUSE",
                "you" | "너" | "당신" => "C_DIALOGUE_ASSISTANT",
                _ => return Some(format!("NAME:{token}")),
            };
            Some(concept.to_string())
        })
        .collect::<Vec<_>>();
    keys.sort();
    keys.dedup();
    keys
}

fn strip_korean_particle(token: &str) -> &str {
    for suffix in [
        "에서는",
        "에서",
        "에게",
        "으로",
        "와",
        "과",
        "를",
        "을",
        "은",
        "는",
        "이",
        "가",
        "에",
        "도",
        "만",
    ] {
        if let Some(stem) = token.strip_suffix(suffix) {
            if !stem.is_empty() {
                return stem;
            }
        }
    }
    token
}

fn resolve_projection(
    contributions: &[&LanguageCenterContributionIR],
) -> LanguageCenterProjectionIR {
    let projections = contributions
        .iter()
        .map(|item| item.projection)
        .collect::<BTreeSet<_>>();
    [
        LanguageCenterProjectionIR::Prohibited,
        LanguageCenterProjectionIR::Conditional,
        LanguageCenterProjectionIR::Reported,
        LanguageCenterProjectionIR::Suppressed,
        LanguageCenterProjectionIR::LiveRequest,
        LanguageCenterProjectionIR::Advisory,
        LanguageCenterProjectionIR::Inquiry,
        LanguageCenterProjectionIR::Descriptive,
    ]
    .into_iter()
    .find(|projection| projections.contains(projection))
    .unwrap_or(LanguageCenterProjectionIR::Unresolved)
}

fn incompatible_projections(projections: &BTreeSet<LanguageCenterProjectionIR>) -> bool {
    let live = projections.contains(&LanguageCenterProjectionIR::LiveRequest);
    live && projections.iter().any(|projection| {
        matches!(
            projection,
            LanguageCenterProjectionIR::Prohibited
                | LanguageCenterProjectionIR::Reported
                | LanguageCenterProjectionIR::Suppressed
        )
    })
}

pub fn language_center_semantic_sha256(center: &LanguageCenterIR) -> String {
    let events = center
        .events
        .iter()
        .map(|event| {
            (
                &event.event_id,
                &event.canonical_predicate,
                event.intent,
                event
                    .arguments
                    .iter()
                    .filter(|argument| argument.role != SemanticRoleKindIR::Agent)
                    .map(|argument| (argument.role, &argument.semantic_keys))
                    .fold(
                        BTreeMap::<Vec<String>, SemanticRoleKindIR>::new(),
                        |mut normalized, (role, keys)| {
                            normalized
                                .entry(keys.clone())
                                .and_modify(|known| {
                                    if semantic_role_priority(role) < semantic_role_priority(*known)
                                    {
                                        *known = role;
                                    }
                                })
                                .or_insert(role);
                            normalized
                        },
                    )
                    .into_iter()
                    .map(|(keys, role)| (role, keys))
                    .collect::<Vec<_>>(),
                event
                    .goal_subject_argument_ids
                    .iter()
                    .filter_map(|subject_id| {
                        event
                            .arguments
                            .iter()
                            .find(|argument| &argument.argument_id == subject_id)
                    })
                    .map(|argument| &argument.semantic_keys)
                    .collect::<Vec<_>>(),
                event.projection,
            )
        })
        .collect::<Vec<_>>();
    let relations = center
        .relations
        .iter()
        .map(|relation| {
            (
                &relation.source_event_id,
                &relation.target_event_id,
                relation.relation,
            )
        })
        .collect::<Vec<_>>();
    let bytes = serde_json::to_vec(&(events, relations)).expect("language center semantics");
    format!("{:x}", Sha256::digest(bytes))
}

fn semantic_role_priority(role: SemanticRoleKindIR) -> u8 {
    match role {
        SemanticRoleKindIR::Theme | SemanticRoleKindIR::Patient => 0,
        SemanticRoleKindIR::Result | SemanticRoleKindIR::Destination => 1,
        SemanticRoleKindIR::Topic | SemanticRoleKindIR::Experiencer => 2,
        SemanticRoleKindIR::Recipient
        | SemanticRoleKindIR::Source
        | SemanticRoleKindIR::Instrument
        | SemanticRoleKindIR::Location
        | SemanticRoleKindIR::ComparisonPeer
        | SemanticRoleKindIR::PriorResult
        | SemanticRoleKindIR::CoTheme => 3,
        SemanticRoleKindIR::Agent => 4,
    }
}

pub fn language_center_graph_sha256(center: &LanguageCenterIR) -> String {
    let bytes = serde_json::to_vec(&(
        &center.schema,
        center.input_phenotype,
        &center.events,
        &center.relations,
        &center.contributions,
        &center.conflicts,
        &center.unresolved,
        &center.projected_goal_event_ids,
        &center.semantic_sha256,
        center.semantic_authority,
        center.language_can_execute,
    ))
    .expect("language center graph");
    format!("{:x}", Sha256::digest(bytes))
}

pub fn compositional_analysis_sha256(analysis: &CompositionalAnalysisIR) -> String {
    json_sha256(analysis)
}

pub fn language_center_goal_projection_sha256(
    projection: &LanguageCenterGoalProjectionIR,
) -> String {
    let mut canonical = projection.clone();
    canonical.projection_sha256.clear();
    json_sha256(&canonical)
}

fn json_sha256(value: &impl Serialize) -> String {
    let bytes = serde_json::to_vec(value).expect("language center hash input");
    format!("{:x}", Sha256::digest(bytes))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compositional_semantics::CompositionalSemanticAnalyzer;
    use crate::pragmatics::{PragmaticContextIR, PragmaticReasoner};

    #[test]
    fn korean_and_english_cortex_forms_converge_on_one_language_center_semantics() {
        let context = PragmaticContextIR::default();
        let english =
            PragmaticReasoner.interpret("Could you take a look at the Aster cache?", &context);
        let korean = PragmaticReasoner.interpret("Aster 캐시 좀 봐줄래?", &context);
        assert!(english.language_center.validate());
        assert!(korean.language_center.validate());
        assert_eq!(
            english.language_center.semantic_sha256, korean.language_center.semantic_sha256,
            "English={:#?}\nKorean={:#?}",
            english.language_center, korean.language_center
        );
        assert_ne!(
            english.language_center.graph_sha256,
            korean.language_center.graph_sha256
        );
    }

    #[test]
    fn contribution_resolution_preserves_conflicts_and_fails_closed() {
        let event_id = "EVENT";
        let contributions = [
            LanguageCenterContributionIR {
                contribution_id: "C1".to_string(),
                source: LanguageCenterSourceIR::CompositionalFrame,
                event_id: event_id.to_string(),
                projection: LanguageCenterProjectionIR::LiveRequest,
                evidence_ref: "FRAME:1".to_string(),
                semantic_authority: false,
                language_can_execute: false,
            },
            LanguageCenterContributionIR {
                contribution_id: "C2".to_string(),
                source: LanguageCenterSourceIR::PragmaticIntentGraph,
                event_id: event_id.to_string(),
                projection: LanguageCenterProjectionIR::Prohibited,
                evidence_ref: "PRAGMATIC:1".to_string(),
                semantic_authority: false,
                language_can_execute: false,
            },
        ];
        let refs = contributions.iter().collect::<Vec<_>>();
        assert_eq!(
            resolve_projection(&refs),
            LanguageCenterProjectionIR::Prohibited
        );
        assert!(incompatible_projections(
            &refs.iter().map(|item| item.projection).collect()
        ));
        assert_eq!(
            contributions.len(),
            2,
            "neither contribution was overwritten"
        );
    }

    #[test]
    fn goal_projection_binds_immutable_module_outputs_to_one_materialization() {
        let text = "Could you inspect the Aster log for me?";
        let result = PragmaticReasoner.interpret(text, &PragmaticContextIR::default());
        let projection = result
            .language_center_goal_projection
            .as_ref()
            .expect("central goal projection");
        let base = CompositionalSemanticAnalyzer.analyze(text);

        assert!(
            projection.validate_against(&result.language_center, &result.compositional_analysis)
        );
        assert_eq!(projection.source_composition_sha256, json_sha256(&base));
        assert_eq!(
            projection.source_pragmatic_intent_sha256,
            json_sha256(&result.pragmatic_intent_graph)
        );
        assert_eq!(
            projection.source_illocution_sha256,
            json_sha256(&result.illocutionary_commitments)
        );
        assert_eq!(projection.central_materialization_count, 1);
        assert!(projection.module_outputs_immutable);
        for source in [
            LanguageCenterGoalDecisionSourceIR::LanguageCenterEvent,
            LanguageCenterGoalDecisionSourceIR::PragmaticIntentGraph,
            LanguageCenterGoalDecisionSourceIR::UtteranceIntentGraph,
            LanguageCenterGoalDecisionSourceIR::IllocutionaryCommitmentGraph,
            LanguageCenterGoalDecisionSourceIR::CentralMaterializer,
        ] {
            assert!(
                projection
                    .decisions
                    .iter()
                    .any(|decision| decision.source == source),
                "missing retained source {source:?}: {projection:#?}"
            );
        }
    }
}
