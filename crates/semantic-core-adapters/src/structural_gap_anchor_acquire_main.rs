//! Targeted ordinary anchor acquisition from planner-generated meaning.

use std::{error::Error, fs, path::PathBuf};

use dockable_semantic_core::{
    DockableCore, PlanIntentIR, SemanticPlanArgumentIR, SemanticPlanEventIR, SemanticPlanGoalIR,
    SemanticPlanProjectionIR, SemanticPlanRelationIR, SemanticPlanRelationKindIR,
    SemanticPlanRoleIR, SEMANTIC_PLAN_GOAL_SCHEMA,
};
use semantic_core_adapters::{
    realize_document_response, ApprovedCompositionalResponseIR, ApprovedResponseStyleIR,
    ApprovedVerbosityIR, CanonicalResponseBridge, LanguageCodeIR, LanguageRegisterIR,
};
use serde::Serialize;
use sha2::{Digest, Sha256};

#[derive(Clone, Copy)]
struct Case {
    id: &'static str,
    event_count: usize,
    relation: SemanticPlanRelationKindIR,
    register: LanguageRegisterIR,
}

#[derive(Serialize)]
struct CandidateRow {
    meaning_family_id: String,
    split: &'static str,
    factor: &'static str,
    value: &'static str,
    baseline_value: &'static str,
    speech_act: String,
    approved_response_ir: ApprovedCompositionalResponseIR,
    surface: String,
    teacher_model: &'static str,
    prompt_sha256: String,
    source_kind: &'static str,
    ordinary_realization: bool,
    expression_rewrite_instruction_used: bool,
    planner_relation: String,
    planner_event_count: usize,
}

fn main() -> Result<(), Box<dyn Error>> {
    let output = std::env::var_os("BCORE_STRUCTURAL_GAP_ANCHOR_OUTPUT")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(
            r"D:\B_Core_validation\expression_rewrite_calibration_v1\structural_gap_anchor_candidates_ko_v1.json",
        ));
    let cases = cases();
    let core =
        DockableCore::load_embedded().map_err(|error| format!("CORE_LOAD_FAILED:{error:?}"))?;
    let bridge = CanonicalResponseBridge;
    let mut rows = Vec::with_capacity(cases.len());
    for (index, case) in cases.iter().enumerate() {
        let goal = goal_for_case(case)?;
        let bundle = core
            .generate_semantic_plan(&goal)
            .map_err(|error| format!("PLANNER_FAILED:{}:{error:?}", case.id))?;
        if !bundle.validate_against(&goal) {
            return Err(format!("PLANNER_REPLAY_INVALID:{}", case.id).into());
        }
        let approved = bridge.issue_plan_description_with_style(
            &goal,
            &bundle,
            LanguageCodeIR::Korean,
            ApprovedResponseStyleIR {
                register: case.register,
                verbosity: ApprovedVerbosityIR::Short,
            },
        )?;
        let ordinary = realize_document_response(&approved, LanguageCodeIR::Korean)?;
        if !ordinary.validate(&approved) {
            return Err(format!("ORDINARY_REALIZATION_INVERSE_INVALID:{}", case.id).into());
        }
        rows.push(CandidateRow {
            meaning_family_id: approved.semantic_sha256.clone(),
            split: split(index),
            factor: "STRUCTURAL_GAP_TARGETED_ANCHOR_ACQUISITION",
            value: "ORDINARY_PLANNER_REALIZATION",
            baseline_value: "ORDINARY_PLANNER_REALIZATION",
            speech_act: format!("{:?}", approved.speech_act).to_ascii_uppercase(),
            approved_response_ir: approved,
            surface: ordinary.markdown,
            teacher_model: "BCORE_ORDINARY_DOCUMENT_CODEC",
            prompt_sha256: sha256(case.id.as_bytes()),
            source_kind: "SYNTHETIC_WORLD_STATE_TO_BCORE_PLANNER_TO_REPLAY_VERIFIED_APPROVED_IR",
            ordinary_realization: true,
            expression_rewrite_instruction_used: false,
            planner_relation: format!("{:?}", case.relation).to_ascii_uppercase(),
            planner_event_count: case.event_count,
        });
    }
    let artifact = serde_json::json!({
        "schema": "BCORE.STRUCTURAL_GAP_TARGETED_ANCHOR_CANDIDATES.V1",
        "runtime_installation": "FORBIDDEN",
        "primitive_learning": "FORBIDDEN",
        "candidate_count": rows.len(),
        "ordinary_realization_only": true,
        "expression_rewrite_instruction_used": false,
        "rows": rows,
    });
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&output, serde_json::to_vec_pretty(&artifact)?)?;
    println!(
        "{}",
        serde_json::json!({"output": output, "candidates": artifact["candidate_count"]})
    );
    Ok(())
}

fn cases() -> Vec<Case> {
    use LanguageRegisterIR::{Formal, Neutral};
    use SemanticPlanRelationKindIR::{Cause, Condition, Contrast, Sequence};
    [
        ("TWO-SEQUENCE-FORMAL", 2, Sequence, Formal),
        ("TWO-CAUSE-FORMAL", 2, Cause, Formal),
        ("TWO-CONDITION-FORMAL", 2, Condition, Formal),
        ("TWO-CONTRAST-FORMAL", 2, Contrast, Formal),
        ("TWO-SEQUENCE-POLITE", 2, Sequence, Neutral),
        ("TWO-CAUSE-POLITE", 2, Cause, Neutral),
        ("TWO-CONDITION-POLITE", 2, Condition, Neutral),
        ("TWO-CONTRAST-POLITE", 2, Contrast, Neutral),
        ("THREE-SEQUENCE-FORMAL", 3, Sequence, Formal),
        ("THREE-CAUSE-FORMAL", 3, Cause, Formal),
        ("THREE-CONDITION-FORMAL", 3, Condition, Formal),
        ("THREE-CONTRAST-FORMAL", 3, Contrast, Formal),
        ("THREE-SEQUENCE-POLITE", 3, Sequence, Neutral),
        ("THREE-CAUSE-POLITE", 3, Cause, Neutral),
        ("THREE-CONDITION-POLITE", 3, Condition, Neutral),
        ("THREE-CONTRAST-POLITE", 3, Contrast, Neutral),
    ]
    .into_iter()
    .map(|(id, event_count, relation, register)| Case {
        id,
        event_count,
        relation,
        register,
    })
    .collect()
}

fn goal_for_case(case: &Case) -> Result<SemanticPlanGoalIR, String> {
    let labels = ["안전 점검", "장비 정비", "운영 보고"];
    let intents = [
        PlanIntentIR::Investigate,
        PlanIntentIR::Repair,
        PlanIntentIR::Explain,
    ];
    let mut arguments = Vec::new();
    let mut events = Vec::new();
    let mut selected_live_event_ids = Vec::new();
    for index in 0..case.event_count {
        let argument_id = format!("ARG-{}-{index}", case.id);
        let event_id = format!("EVENT-{}-{index}", case.id);
        arguments.push(SemanticPlanArgumentIR {
            argument_id: argument_id.clone(),
            role: SemanticPlanRoleIR::Theme,
            concept_ids: vec![format!("C_STRUCTURAL_ANCHOR_{}_{index}", case.id)],
            grounded_label: format!("{} {}차", labels[index], index + 1),
        });
        events.push(SemanticPlanEventIR {
            event_id: event_id.clone(),
            predicate_concept_id: format!("PREDICATE_{:?}_{index}", intents[index]),
            intent: intents[index],
            argument_ids: vec![argument_id.clone()],
            goal_subject_argument_ids: vec![argument_id],
            projection: SemanticPlanProjectionIR::LiveRequest,
            user_request_present: true,
            external_execution_authorized: false,
        });
        selected_live_event_ids.push(event_id);
    }
    let relations = (1..case.event_count)
        .map(|index| SemanticPlanRelationIR {
            relation_id: format!("REL-{}-{index}", case.id),
            source_event_id: selected_live_event_ids[index - 1].clone(),
            target_event_id: selected_live_event_ids[index].clone(),
            relation: case.relation,
        })
        .collect::<Vec<_>>();
    let source = serde_json::json!({
        "schema": "BCORE.STRUCTURAL_GAP_TARGETED_WORLD_STATE.V1",
        "case_id": case.id,
        "event_count": case.event_count,
        "relation": format!("{:?}", case.relation),
        "register": format!("{:?}", case.register),
    });
    let mut goal = SemanticPlanGoalIR {
        schema: SEMANTIC_PLAN_GOAL_SCHEMA.into(),
        goal_id: format!("GOAL-{}", case.id),
        events,
        arguments,
        relations,
        selected_live_event_ids,
        context_semantic_ids: vec![format!("STRUCTURAL_GAP:{}", case.id)],
        source_semantic_sha256: sha256(&serde_json::to_vec(&source).expect("source serializes")),
        max_steps_per_event: 16,
        semantic_authority: false,
        language_can_execute: false,
        semantic_sha256: String::new(),
    };
    goal.seal();
    goal.validate()
        .then_some(goal)
        .ok_or_else(|| "TARGETED_ANCHOR_GOAL_INVALID".into())
}

fn split(index: usize) -> &'static str {
    match index % 8 {
        0 => "BLIND",
        1 => "VALIDATION",
        _ => "TRAIN",
    }
}

fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
