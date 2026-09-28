//! Live bridge from a replay-verified semantic plan to approved response IR.
//!
//! The planner's event graph remains the authority. This bridge only emits a
//! bounded plan-description response: it never reclassifies an event as a
//! completed world fact and never accepts surface text as input.

use dockable_semantic_core::{
    DeliberationEngine, DeliberationRequestIR, EvidenceIR, LiteralIR, SemanticPlanBundleIR,
    SemanticPlanArgumentIR, SemanticPlanEventIR, SemanticPlanGoalIR,
    SemanticPlanRelationKindIR, SemanticPlanRoleIR, DELIBERATION_REQUEST_SCHEMA,
};

use crate::approved_response::{
    ApprovedClaimBindingIR, ApprovedCompositionalClaimIR, ApprovedCompositionalResponseBuilder,
    ApprovedCompositionalResponseIR, ApprovedDiscourseRelationIR, ApprovedEventPhaseIR,
    ApprovedEventPredicateSenseIR, ApprovedEventRealizationClassIR, ApprovedEventRealizationIR,
    ApprovedLexicalNodeIR, ApprovedModalityIR, ApprovedOpenValueIR, ApprovedRelationTypeIR,
    ApprovedResponseBuilder, ApprovedResponseCandidateIR, ApprovedResponseStyleIR,
    ApprovedSemanticTypeIR, ApprovedSpeechActIR, ApprovedValueIR, ApprovedVerbosityIR,
    approved_event_predicate_spec,
};
use crate::language_knowledge::{LanguageCodeIR, LanguageRegisterIR};

const MAX_BRIDGED_EVENTS: usize = 32;

/// Emits a canonical response for the fact that B_Core has produced a
/// planned description of selected semantic events. It is deliberately
/// inapplicable to claims about external completion, truth, or execution.
#[derive(Debug, Clone, Copy, Default)]
pub struct CanonicalResponseBridge;

impl CanonicalResponseBridge {
    pub fn issue_plan_description(
        &self,
        semantic_goal: &SemanticPlanGoalIR,
        semantic_plan_bundle: &SemanticPlanBundleIR,
        language: LanguageCodeIR,
    ) -> Result<ApprovedCompositionalResponseIR, String> {
        self.issue_plan_description_with_style(
            semantic_goal,
            semantic_plan_bundle,
            language,
            ApprovedResponseStyleIR {
                register: LanguageRegisterIR::Neutral,
                verbosity: ApprovedVerbosityIR::Short,
            },
        )
    }

    /// Style is a realization constraint only. It cannot modify approved
    /// claims, the world-state receipt, event roles, or event phase.
    pub fn issue_plan_description_with_style(
        &self,
        semantic_goal: &SemanticPlanGoalIR,
        semantic_plan_bundle: &SemanticPlanBundleIR,
        language: LanguageCodeIR,
        style: ApprovedResponseStyleIR,
    ) -> Result<ApprovedCompositionalResponseIR, String> {
        if !semantic_goal.validate()
            || !semantic_plan_bundle.validate_against(semantic_goal)
            || semantic_goal.selected_live_event_ids.is_empty()
            || semantic_goal.selected_live_event_ids.len() > MAX_BRIDGED_EVENTS
            || !matches!(language, LanguageCodeIR::Korean | LanguageCodeIR::English)
        {
            return Err("INVALID_CANONICAL_RESPONSE_BRIDGE_SOURCE".into());
        }

        let state_label = match language {
            LanguageCodeIR::Korean => "계획됨",
            LanguageCodeIR::English => "planned",
            _ => unreachable!("language checked above"),
        };
        let mut bindings = Vec::new();
        let mut compositional = Vec::new();
        let mut event_realizations = Vec::new();
        for (index, event_id) in semantic_goal.selected_live_event_ids.iter().enumerate() {
            let event = semantic_goal
                .events
                .iter()
                .find(|event| &event.event_id == event_id)
                .ok_or("SELECTED_EVENT_MISSING")?;
            if let Some(typed) = planned_typed_event_claims(event, &semantic_goal.arguments) {
                for (role_index, (binding, claim)) in typed.claims.into_iter().enumerate() {
                    bindings.push(ApprovedClaimBindingIR {
                        proposition_id: format!("CANONICAL_PLAN_EVENT:{index}:ROLE:{role_index}"),
                        ..binding
                    });
                    compositional.push(ApprovedCompositionalClaimIR {
                        proposition_id: format!("CANONICAL_PLAN_EVENT:{index}:ROLE:{role_index}"),
                        ..claim
});
                }
                event_realizations.push(ApprovedEventRealizationIR {
                    subject_node_id: event.event_id.clone(),
                    class: typed.class,
                    predicate_sense: Some(typed.predicate_sense),
                    phase: Some(ApprovedEventPhaseIR::Planned),
                    perspective: typed.perspective,
                    voice: None,
                    information_structure: None,
                });
            } else {
                let surface_subject = event
                    .goal_subject_argument_ids
                    .iter()
                    .find_map(|argument_id| {
                        semantic_goal
                            .arguments
                            .iter()
                            .find(|argument| &argument.argument_id == argument_id)
                    })
                    .filter(|argument| !argument.grounded_label.trim().is_empty())
                    .map(|argument| argument.grounded_label.clone())
                    .ok_or("SELECTED_EVENT_HAS_NO_GROUNDED_SUBJECT")?;
                let proposition_id = format!("CANONICAL_PLAN_EVENT:{index}");
                bindings.push(ApprovedClaimBindingIR {
                    proposition_id: proposition_id.clone(),
                    subject: event.event_id.clone(),
                    property: "status".into(),
                    value: ApprovedValueIR::Symbol("planned".into()),
                    polarity: true,
                });
                compositional.push(ApprovedCompositionalClaimIR {
                    proposition_id,
                    subject: ApprovedLexicalNodeIR {
                        node_id: event.event_id.clone(),
                        semantic_type: ApprovedSemanticTypeIR::Event,
                        canonical_lexical_label: surface_subject,
                    },
                    relation: ApprovedRelationTypeIR::Status,
                    value: ApprovedOpenValueIR::Lexical(ApprovedLexicalNodeIR {
                        node_id: "planned".into(),
                        semantic_type: ApprovedSemanticTypeIR::State,
                        canonical_lexical_label: state_label.into(),
                    }),
                    polarity: true,
                    modality: ApprovedModalityIR::Asserted,
    status_frame: None,
});
                event_realizations.push(ApprovedEventRealizationIR {
                    subject_node_id: event.event_id.clone(),
                    class: ApprovedEventRealizationClassIR::ScheduledProcess,
                    predicate_sense: None,
                    phase: Some(ApprovedEventPhaseIR::Planned),
                    perspective: None,
                    voice: None,
                    information_structure: None,
                });
            }
        }

        // A fresh deliberation receipt establishes that every emitted claim
        // is supported by the selected semantic-plan snapshot. No mechanism
        // is supplied and no external action is authorized or executed.
        let evidence = bindings
            .iter()
            .enumerate()
            .map(|(index, binding)| EvidenceIR {
                evidence_id: format!("CANONICAL_PLAN_EVIDENCE:{index}"),
                literal: LiteralIR {
                    proposition_id: binding.proposition_id.clone(),
                    value: binding.polarity,
                },
                reliability_millis: 1_000,
                source_ref: format!("SEMANTIC_PLAN:{}", semantic_goal.semantic_sha256),
            })
            .collect::<Vec<_>>();
        let goals = bindings
            .iter()
            .map(|binding| LiteralIR {
                proposition_id: binding.proposition_id.clone(),
                value: binding.polarity,
            })
            .collect::<Vec<_>>();
        let request = DeliberationRequestIR {
            schema: DELIBERATION_REQUEST_SCHEMA.into(),
            request_id: format!(
                "CANONICAL_RESPONSE:{}",
                &semantic_goal.semantic_sha256[..24]
            ),
            subject: "approved_plan_description".into(),
            evidence,
            mechanisms: Vec::new(),
            goals,
            authority_envelope: Default::default(),
            immutable_constraints: Vec::new(),
            max_depth: 1,
            beam_width: 1,
            max_hypotheses: 1,
            max_counterfactuals: 1,
        };
        let deliberation = DeliberationEngine
            .deliberate(&request)
            .map_err(|_| "CANONICAL_RESPONSE_DELIBERATION_FAILED")?;
        let discourse_relation = discourse_relation_for_goal(semantic_goal);
        let approved = ApprovedResponseBuilder
            .approve(ApprovedResponseCandidateIR {
                world_state_sha256: semantic_goal.semantic_sha256.clone(),
                request,
                deliberation,
                speech_act: ApprovedSpeechActIR::Inform,
                claims: bindings,
                discourse_relation,
                style,
            })
            .map_err(|error| format!("CANONICAL_RESPONSE_APPROVAL_FAILED:{error}"))?;
        ApprovedCompositionalResponseBuilder
            .enrich_with_event_realizations(&approved, compositional, event_realizations)
            .map_err(|error| format!("CANONICAL_RESPONSE_ENRICH_FAILED:{error}"))
    }
}

/// A canonical planner predicate can enter the event codec only through this
/// closed semantic registry.  It never inspects Korean labels or infers a
/// verb from a surface string.
fn planned_event_profile(
    predicate_concept_id: &str,
) -> Option<(ApprovedEventRealizationClassIR, ApprovedEventPredicateSenseIR)> {
    let predicate = predicate_concept_id.strip_prefix("C_").unwrap_or(predicate_concept_id);
    let profile = match predicate {
        "MOVE" => (ApprovedEventRealizationClassIR::Motion, ApprovedEventPredicateSenseIR::Move),
        "TRANSFER" => (ApprovedEventRealizationClassIR::Transfer, ApprovedEventPredicateSenseIR::Transfer),
        "DELIVER" => (ApprovedEventRealizationClassIR::Transfer, ApprovedEventPredicateSenseIR::Deliver),
        "SEND" => (ApprovedEventRealizationClassIR::Transfer, ApprovedEventPredicateSenseIR::Send),
        "GIVE" => (ApprovedEventRealizationClassIR::Transfer, ApprovedEventPredicateSenseIR::Give),
        "APPLY" => (ApprovedEventRealizationClassIR::Transfer, ApprovedEventPredicateSenseIR::Apply),
        "ATTACH" => (ApprovedEventRealizationClassIR::StateChange, ApprovedEventPredicateSenseIR::Attach),
        "UPLOAD" => (ApprovedEventRealizationClassIR::Transfer, ApprovedEventPredicateSenseIR::Upload),
        "CREATE" => (ApprovedEventRealizationClassIR::Creation, ApprovedEventPredicateSenseIR::Create),
        "WRITE" => (ApprovedEventRealizationClassIR::Creation, ApprovedEventPredicateSenseIR::Write),
        "INVESTIGATE" | "INSPECT" => (ApprovedEventRealizationClassIR::Inspection, ApprovedEventPredicateSenseIR::Inspect),
        "CHANGE" => (ApprovedEventRealizationClassIR::StateChange, ApprovedEventPredicateSenseIR::Change),
        "ACTIVATE" => (ApprovedEventRealizationClassIR::StateChange, ApprovedEventPredicateSenseIR::Activate),
        "DEACTIVATE" => (ApprovedEventRealizationClassIR::StateChange, ApprovedEventPredicateSenseIR::Deactivate),
        "RESET" => (ApprovedEventRealizationClassIR::StateChange, ApprovedEventPredicateSenseIR::Reset),
        "EXPAND" => (ApprovedEventRealizationClassIR::StateChange, ApprovedEventPredicateSenseIR::Expand),
        "START" => (ApprovedEventRealizationClassIR::StateChange, ApprovedEventPredicateSenseIR::Start),
        "COMPLETE" => (ApprovedEventRealizationClassIR::StateChange, ApprovedEventPredicateSenseIR::Complete),
        "OPEN" => (ApprovedEventRealizationClassIR::StateChange, ApprovedEventPredicateSenseIR::Open),
        "CLOSE" => (ApprovedEventRealizationClassIR::StateChange, ApprovedEventPredicateSenseIR::Close),
        _ => return None,
    };
    approved_event_predicate_spec(profile.0, profile.1).map(|_| profile)
}

fn approved_role(role: SemanticPlanRoleIR) -> Option<(ApprovedRelationTypeIR, &'static str)> {
    match role {
        SemanticPlanRoleIR::Agent => Some((ApprovedRelationTypeIR::Agent, "agent")),
        SemanticPlanRoleIR::Theme => Some((ApprovedRelationTypeIR::Theme, "theme")),
        SemanticPlanRoleIR::Patient => Some((ApprovedRelationTypeIR::Patient, "patient")),
        SemanticPlanRoleIR::Source => Some((ApprovedRelationTypeIR::Source, "source")),
        SemanticPlanRoleIR::Destination => {
            Some((ApprovedRelationTypeIR::Destination, "destination"))
        }
        SemanticPlanRoleIR::Target => Some((ApprovedRelationTypeIR::Target, "target")),
        SemanticPlanRoleIR::Instrument => Some((ApprovedRelationTypeIR::Instrument, "instrument")),
        SemanticPlanRoleIR::Location => Some((ApprovedRelationTypeIR::Location, "location")),
        _ => None,
    }
}

/// The plan role, rather than a surface noun, supplies only the broad lexical
/// realization type needed by the document codec.  This is deliberately
/// conservative: unmapped roles never enter a typed event frame.
fn lexical_type_for_plan_role(role: SemanticPlanRoleIR) -> ApprovedSemanticTypeIR {
    match role {
        SemanticPlanRoleIR::Agent => ApprovedSemanticTypeIR::Organization,
        SemanticPlanRoleIR::Source | SemanticPlanRoleIR::Destination | SemanticPlanRoleIR::Target | SemanticPlanRoleIR::Location => {
            ApprovedSemanticTypeIR::Location
        }
        _ => ApprovedSemanticTypeIR::Concept,
    }
}

struct PlannedTypedEventClaims {
    class: ApprovedEventRealizationClassIR,
    predicate_sense: ApprovedEventPredicateSenseIR,
    perspective: Option<crate::approved_response::ApprovedEventPerspectiveIR>,
    claims: Vec<(ApprovedClaimBindingIR, ApprovedCompositionalClaimIR)>,
}

fn planned_typed_event_claims(
    event: &SemanticPlanEventIR,
    arguments: &[SemanticPlanArgumentIR],
) -> Option<PlannedTypedEventClaims> {
    let (class, predicate_sense) = planned_event_profile(&event.predicate_concept_id)?;
    let spec = approved_event_predicate_spec(class, predicate_sense)?;
    let mut claims = Vec::new();
    for argument_id in &event.argument_ids {
        let argument = arguments
            .iter()
            .find(|argument| &argument.argument_id == argument_id)?;
        let (relation, property) = approved_role(argument.role)?;
        if !spec.allows_role(relation) || claims.iter().any(|(_, claim): &(ApprovedClaimBindingIR, ApprovedCompositionalClaimIR)| claim.relation == relation) {
            return None;
        }
        let value = ApprovedOpenValueIR::Lexical(ApprovedLexicalNodeIR {
            node_id: argument.argument_id.clone(),
            semantic_type: lexical_type_for_plan_role(argument.role),
            canonical_lexical_label: argument.grounded_label.clone(),
        });
        claims.push((
            ApprovedClaimBindingIR {
                proposition_id: String::new(),
                subject: event.event_id.clone(),
                property: property.into(),
                value: ApprovedValueIR::Symbol(argument.argument_id.clone()),
                polarity: true,
            },
            ApprovedCompositionalClaimIR {
                proposition_id: String::new(),
                subject: ApprovedLexicalNodeIR {
                    node_id: event.event_id.clone(),
                    semantic_type: ApprovedSemanticTypeIR::Event,
                    canonical_lexical_label: event.predicate_concept_id.clone(),
                },
                relation,
                value,
                polarity: true,
                modality: ApprovedModalityIR::Asserted,
    status_frame: None,
},
        ));
    }
    if !spec.required_roles.iter().all(|role| {
        claims
            .iter()
            .any(|(_, claim)| claim.relation == *role)
    }) || !claims
        .iter()
        .any(|(_, claim)| claim.relation == ApprovedRelationTypeIR::Agent)
    {
        return None;
    }
    Some(PlannedTypedEventClaims {
        class,
        predicate_sense,
        perspective: Some(crate::approved_response::ApprovedEventPerspectiveIR {
            voice: crate::approved_response::ApprovedEventVoiceIR::Active,
            focus: crate::approved_response::ApprovedEventFocusIR::Agent,
        }),
        claims,
    })
}

/// Preserve a homogeneous selected-event relation from the semantic plan.
/// Mixed or absent relations deliberately fall back to a plain statement.
fn discourse_relation_for_goal(goal: &SemanticPlanGoalIR) -> ApprovedDiscourseRelationIR {
    let selected = goal
        .selected_live_event_ids
        .iter()
        .map(String::as_str)
        .collect::<std::collections::BTreeSet<_>>();
    let relations = goal
        .relations
        .iter()
        .filter(|relation| {
            selected.contains(relation.source_event_id.as_str())
                && selected.contains(relation.target_event_id.as_str())
        })
        .map(|relation| relation.relation)
        .collect::<std::collections::BTreeSet<_>>();
    if relations.len() != 1 {
        return ApprovedDiscourseRelationIR::Statement;
    }
    match relations.into_iter().next() {
        Some(SemanticPlanRelationKindIR::Cause) => ApprovedDiscourseRelationIR::Cause,
        Some(SemanticPlanRelationKindIR::Condition) => ApprovedDiscourseRelationIR::Condition,
        Some(SemanticPlanRelationKindIR::Contrast) => ApprovedDiscourseRelationIR::Comparison,
        Some(SemanticPlanRelationKindIR::Sequence | SemanticPlanRelationKindIR::TemporalBefore) => {
            ApprovedDiscourseRelationIR::Temporal
        }
        Some(SemanticPlanRelationKindIR::Coordination | SemanticPlanRelationKindIR::Purpose)
        | None => ApprovedDiscourseRelationIR::Statement,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dockable_semantic_core::{
        DockableCore, PlanIntentIR, SemanticPlanArgumentIR, SemanticPlanEventIR,
        SemanticPlanProjectionIR, SemanticPlanRelationIR, SemanticPlanRoleIR,
        SEMANTIC_PLAN_GOAL_SCHEMA,
    };
    use crate::cognitive::{CognitiveApi, NaturalLanguageRequestIR, NATURAL_LANGUAGE_REQUEST_SCHEMA};

    fn goal_with_relations(relations: Vec<SemanticPlanRelationKindIR>) -> SemanticPlanGoalIR {
        let mut goal = SemanticPlanGoalIR {
            schema: SEMANTIC_PLAN_GOAL_SCHEMA.into(),
            goal_id: "BRIDGE-RELATION-TEST".into(),
            events: (0..3)
                .map(|index| SemanticPlanEventIR {
                    event_id: format!("EVENT-{index}"),
                    predicate_concept_id: format!("PREDICATE-{index}"),
                    intent: PlanIntentIR::Plan,
                    argument_ids: vec![format!("ARG-{index}")],
                    goal_subject_argument_ids: vec![format!("ARG-{index}")],
                    projection: SemanticPlanProjectionIR::LiveRequest,
                    user_request_present: true,
                    external_execution_authorized: false,
                })
                .collect(),
            arguments: (0..3)
                .map(|index| SemanticPlanArgumentIR {
                    argument_id: format!("ARG-{index}"),
                    role: SemanticPlanRoleIR::Theme,
                    concept_ids: vec![format!("CONCEPT-{index}")],
                    grounded_label: format!("항목 {index}"),
                })
                .collect(),
            relations: relations
                .into_iter()
                .enumerate()
                .map(|(index, relation)| SemanticPlanRelationIR {
                    relation_id: format!("REL-{index}"),
                    source_event_id: format!("EVENT-{index}"),
                    target_event_id: format!("EVENT-{}", index + 1),
                    relation,
                })
                .collect(),
            selected_live_event_ids: vec!["EVENT-0".into(), "EVENT-1".into(), "EVENT-2".into()],
            context_semantic_ids: vec!["BRIDGE-RELATION".into()],
            source_semantic_sha256: "a".repeat(64),
            max_steps_per_event: 16,
            semantic_authority: false,
            language_can_execute: false,
            semantic_sha256: String::new(),
        };
        goal.seal();
        goal
    }

    #[test]
    fn preserves_homogeneous_planner_relation_without_surface_inference() {
        let goal = goal_with_relations(vec![
            SemanticPlanRelationKindIR::Cause,
            SemanticPlanRelationKindIR::Cause,
        ]);
        assert!(goal.validate());
        assert_eq!(
            discourse_relation_for_goal(&goal),
            ApprovedDiscourseRelationIR::Cause
        );
    }

    #[test]
    fn mixed_planner_relations_fail_closed_to_statement() {
        let goal = goal_with_relations(vec![
            SemanticPlanRelationKindIR::Cause,
            SemanticPlanRelationKindIR::Condition,
        ]);
        assert!(goal.validate());
        assert_eq!(
            discourse_relation_for_goal(&goal),
            ApprovedDiscourseRelationIR::Statement
        );
    }

    #[test]
    fn bridge_preserves_a_registered_event_predicate_and_roles_for_planned_realization() {
        let mut goal = SemanticPlanGoalIR {
            schema: SEMANTIC_PLAN_GOAL_SCHEMA.into(),
            goal_id: "BRIDGE-TYPED-UPLOAD".into(),
            events: vec![SemanticPlanEventIR {
                event_id: "UPLOAD-EVENT".into(),
                predicate_concept_id: "UPLOAD".into(),
                intent: PlanIntentIR::Plan,
                argument_ids: vec!["UPLOAD-AGENT".into(), "UPLOAD-THEME".into(), "UPLOAD-DEST".into()],
                goal_subject_argument_ids: vec!["UPLOAD-THEME".into()],
                projection: SemanticPlanProjectionIR::LiveRequest,
                user_request_present: true,
                external_execution_authorized: false,
            }],
            arguments: vec![
                SemanticPlanArgumentIR {
                    argument_id: "UPLOAD-AGENT".into(),
                    role: SemanticPlanRoleIR::Agent,
                    concept_ids: vec!["C_TEAM_OPERATIONS".into()],
                    grounded_label: "운영팀".into(),
                },
                SemanticPlanArgumentIR {
                    argument_id: "UPLOAD-THEME".into(),
                    role: SemanticPlanRoleIR::Theme,
                    concept_ids: vec!["C_REPORT".into()],
                    grounded_label: "보고서".into(),
                },
                SemanticPlanArgumentIR {
                    argument_id: "UPLOAD-DEST".into(),
                    role: SemanticPlanRoleIR::Destination,
                    concept_ids: vec!["C_ARCHIVE_SERVER".into()],
                    grounded_label: "서버".into(),
                },
            ],
            relations: Vec::new(),
            selected_live_event_ids: vec!["UPLOAD-EVENT".into()],
            context_semantic_ids: vec!["C_UPLOAD_TEST".into()],
            source_semantic_sha256: "a".repeat(64),
            max_steps_per_event: 16,
            semantic_authority: false,
            language_can_execute: false,
            semantic_sha256: String::new(),
        };
        goal.seal();
        assert!(goal.validate());
        let core = DockableCore::load_embedded().expect("embedded core");
        let bundle = core.generate_semantic_plan(&goal).expect("typed plan");
        let approved = CanonicalResponseBridge
            .issue_plan_description(&goal, &bundle, LanguageCodeIR::Korean)
            .expect("typed bridge");
        assert!(approved.validate());
        assert_eq!(approved.event_realizations.len(), 1);
        assert_eq!(
            approved.event_realizations[0].predicate_sense,
            Some(ApprovedEventPredicateSenseIR::Upload)
        );
        let output = crate::document_response::realize_document_response(
            &approved,
            LanguageCodeIR::Korean,
        )
        .expect("typed Korean realization");
        assert_eq!(output.markdown, "운영팀이 보고서를 서버로 업로드할 거예요.");
        assert!(output.validate(&approved));
    }

    #[test]
    fn unregistered_or_role_incomplete_planner_predicates_stay_in_safe_status_fallback() {
        let mut goal = goal_with_relations(Vec::new());
        goal.events.truncate(1);
        goal.arguments.truncate(1);
        goal.selected_live_event_ids = vec!["EVENT-0".into()];
        goal.events[0].predicate_concept_id = "UPLOAD".into();
        goal.events[0].argument_ids = vec!["ARG-0".into()];
        goal.events[0].goal_subject_argument_ids = vec!["ARG-0".into()];
        goal.seal();
        assert!(goal.validate());
        let core = DockableCore::load_embedded().expect("embedded core");
        let bundle = core.generate_semantic_plan(&goal).expect("fallback plan");
        let approved = CanonicalResponseBridge
            .issue_plan_description(&goal, &bundle, LanguageCodeIR::Korean)
            .expect("fallback bridge");
        assert_eq!(approved.event_realizations.len(), 1);
        assert_eq!(approved.event_realizations[0].predicate_sense, None);
        assert_eq!(approved.claims[0].relation, ApprovedRelationTypeIR::Status);
    }

    #[test]
    fn korean_upload_request_preserves_predicate_and_valency_to_document_surface() {
        let mut api = CognitiveApi::new_embedded().expect("embedded cognitive api");
        let response = api
            .process(&NaturalLanguageRequestIR {
                schema: NATURAL_LANGUAGE_REQUEST_SCHEMA.into(),
                request_id: "UPLOAD-ROUTE-REGRESSION".into(),
                text: "운영팀이 보고서를 서버로 업로드해.".into(),
                output_language: Some(LanguageCodeIR::Korean),
                context_tags: Vec::new(),
                max_plan_steps: 12,
            })
            .expect("Korean upload request");
        let approved = response
            .approved_plan_response
            .as_deref()
            .expect("approved plan response");
        assert_eq!(approved.event_realizations[0].predicate_sense, Some(ApprovedEventPredicateSenseIR::Upload));
        assert_eq!(
            approved
                .claims
                .iter()
                .map(|claim| claim.relation)
                .collect::<Vec<_>>(),
            vec![
                ApprovedRelationTypeIR::Agent,
                ApprovedRelationTypeIR::Theme,
                ApprovedRelationTypeIR::Destination,
            ]
        );
        let document = crate::document_response::realize_document_response(
            approved,
            LanguageCodeIR::Korean,
        )
        .expect("typed upload document");
        assert_eq!(document.markdown, "운영팀이 보고서를 서버로 업로드할 거예요.");
        assert!(document.validate(approved));
        assert!(response.validate());
    }

    #[test]
    fn korean_typed_event_requests_reach_natural_document_surfaces() {
        let cases = [
            (
                "보존팀이 보호제를 균열 부위에 도포해.",
                ApprovedEventPredicateSenseIR::Apply,
                "보존팀이 보호제를 균열 부위에 바를 거예요.",
                Some(ApprovedRelationTypeIR::Target),
            ),
            (
                "작업자가 표본 라벨을 보관 상자에 부착해.",
                ApprovedEventPredicateSenseIR::Attach,
                "작업자가 표본 라벨을 보관 상자에 붙일 거예요.",
                Some(ApprovedRelationTypeIR::Target),
            ),
            (
                "관리자가 수집 범위를 해안선까지 확대해.",
                ApprovedEventPredicateSenseIR::Expand,
                "관리자가 수집 범위를 해안선까지 확대할 거예요.",
                Some(ApprovedRelationTypeIR::Target),
            ),
            (
                "현장팀이 점검을 마쳐.",
                ApprovedEventPredicateSenseIR::Complete,
                "현장팀이 점검을 마칠 거예요.",
                None,
            ),
            (
                "관리자가 보안문을 닫아.",
                ApprovedEventPredicateSenseIR::Close,
                "관리자가 보안문을 닫을 거예요.",
                None,
            ),
        ];
        for (index, (text, sense, expected_surface, required_role)) in cases.iter().enumerate() {
            let mut api = CognitiveApi::new_embedded().expect("embedded cognitive api");
            let response = api
                .process(&NaturalLanguageRequestIR {
                    schema: NATURAL_LANGUAGE_REQUEST_SCHEMA.into(),
                    request_id: format!("TYPED-EVENT-ROUTE-{index}"),
                    text: (*text).into(),
                    output_language: Some(LanguageCodeIR::Korean),
                    context_tags: Vec::new(),
                    max_plan_steps: 12,
                })
                .expect("typed Korean request");
            let approved = response
                .approved_plan_response
                .as_deref()
                .expect("approved plan response");
            assert_eq!(approved.event_realizations[0].predicate_sense, Some(*sense));
            if let Some(required_role) = required_role {
                assert!(approved.claims.iter().any(|claim| claim.relation == *required_role));
            }
            let document = crate::document_response::realize_document_response(
                approved,
                LanguageCodeIR::Korean,
            )
            .expect("typed Korean document");
            assert_eq!(&document.markdown, expected_surface);
            assert!(document.validate(approved));
            assert!(response.validate());
        }
    }
}
