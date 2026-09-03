use semantic_core_adapters::{
    CognitiveApi, ConversationInputModalityIR, ConversationTurnDispositionIR,
    ConversationTurnRequestIR, DeferredCommitmentStatusIR, LanguageCodeIR,
    CONVERSATION_STATE_SCHEMA, CONVERSATION_TURN_REQUEST_SCHEMA,
};
use serde::Serialize;
use serde_json::Value;

#[derive(Clone, Copy)]
pub struct Turn {
    pub text: &'static str,
    pub language: LanguageCodeIR,
}

pub struct Case {
    pub id: &'static str,
    pub category: &'static str,
    pub turns: &'static [Turn],
    pub expected_signature: Option<&'static str>,
    pub expected_guarded_programs: usize,
    pub expected_pending_commitments: usize,
    pub expected_clarification: bool,
    pub require_stable_rebinding: bool,
}

#[derive(Serialize)]
struct Row {
    id: String,
    category: String,
    state_schema: String,
    guarded_programs: usize,
    guarded_steps: usize,
    expression_signatures: Vec<String>,
    expression_hashes: Vec<String>,
    linked_expression_hashes: usize,
    pending_commitments: usize,
    activated_commitments: usize,
    clarification_required: bool,
    stable_rebinding: bool,
    authority_violation: bool,
    pass: bool,
}

#[derive(Serialize)]
struct Summary {
    schema: &'static str,
    suite: &'static str,
    total: usize,
    passed: usize,
    failed: usize,
    external_llm_calls: usize,
    local_teacher_calls: usize,
    network_calls: usize,
    recursive_source_mutations: usize,
    rows: Vec<Row>,
}

fn request(id: &str, turn: u64, spec: Turn) -> ConversationTurnRequestIR {
    ConversationTurnRequestIR {
        schema: CONVERSATION_TURN_REQUEST_SCHEMA.to_string(),
        conversation_id: id.to_string(),
        turn_index: turn,
        request_id: format!("{id}-{turn}"),
        modality: ConversationInputModalityIR::Text,
        raw_text: spec.text.to_string(),
        input_confidence_millis: 1_000,
        alternatives: Vec::new(),
        output_language: Some(spec.language),
        context_tags: Vec::new(),
        max_plan_steps: 16,
    }
}

fn expression_signature(expression: &Value) -> Option<String> {
    let operator = expression.get("operator")?.as_str()?;
    match operator {
        "ATOM" => Some(format!(
            "ATOM:{}",
            expression.get("canonical_predicate")?.as_str()?
        )),
        "NOT" => {
            let children = expression.get("children")?.as_array()?;
            (children.len() == 1)
                .then(|| expression_signature(&children[0]).map(|child| format!("NOT({child})")))?
        }
        "ALL" | "ANY" => {
            let children = expression.get("children")?.as_array()?;
            if children.len() < 2 {
                return None;
            }
            let signatures = children
                .iter()
                .map(expression_signature)
                .collect::<Option<Vec<_>>>()?;
            Some(format!("{operator}({})", signatures.join(",")))
        }
        _ => None,
    }
}

fn expression_has_authority(expression: &Value) -> bool {
    expression
        .get("semantic_authority")
        .and_then(Value::as_bool)
        .unwrap_or(true)
        || expression
            .get("external_execution_authorized")
            .and_then(Value::as_bool)
            .unwrap_or(true)
        || expression
            .get("children")
            .and_then(Value::as_array)
            .is_none_or(|children| children.iter().any(expression_has_authority))
}

pub fn emit(suite: &'static str, cases: &[Case]) {
    let mut rows = Vec::new();
    for case in cases {
        let mut api = CognitiveApi::new_embedded().expect("embedded core");
        let mut last_disposition = ConversationTurnDispositionIR::Grounded;
        let mut turn_failed = false;
        for (index, turn) in case.turns.iter().copied().enumerate() {
            match api.process_conversation_turn(&request(
                case.id,
                u64::try_from(index + 1).expect("bounded turn"),
                turn,
            )) {
                Ok(response) => last_disposition = response.disposition,
                Err(_) => {
                    turn_failed = true;
                    break;
                }
            }
        }
        let Some(state) = api.conversation_state(case.id) else {
            rows.push(Row {
                id: case.id.to_string(),
                category: case.category.to_string(),
                state_schema: "MISSING".to_string(),
                guarded_programs: 0,
                guarded_steps: 0,
                expression_signatures: Vec::new(),
                expression_hashes: Vec::new(),
                linked_expression_hashes: 0,
                pending_commitments: 0,
                activated_commitments: 0,
                clarification_required: false,
                stable_rebinding: false,
                authority_violation: false,
                pass: false,
            });
            continue;
        };
        let value = serde_json::to_value(state).expect("state json");
        let programs = value
            .get("active_discourse_programs")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        let mut guarded_programs = 0;
        let mut guarded_steps = 0;
        let mut expression_signatures = Vec::new();
        let mut expression_hashes = Vec::new();
        let mut linked_expression_hashes = 0;
        let mut authority_violation = false;
        let mut schemas_valid = state.schema == CONVERSATION_STATE_SCHEMA;
        for program in &programs {
            schemas_valid &= program.get("schema").and_then(Value::as_str)
                == Some("B_CORE_DISCOURSE_PROGRAM_IR_4");
            let program_guard_count = program
                .get("guarded_step_count")
                .and_then(Value::as_u64)
                .unwrap_or(0) as usize;
            if program_guard_count > 0 {
                guarded_programs += 1;
            }
            for step in program
                .get("steps")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
            {
                let Some(guard) = step.get("guard").filter(|guard| !guard.is_null()) else {
                    continue;
                };
                guarded_steps += 1;
                schemas_valid &= guard.get("schema").and_then(Value::as_str)
                    == Some("B_CORE_DISCOURSE_PROGRAM_GUARD_IR_3");
                let Some(expression) = guard.get("condition_expression") else {
                    continue;
                };
                schemas_valid &= expression.get("schema").and_then(Value::as_str)
                    == Some("B_CORE_GUARD_CONDITION_EXPRESSION_IR_1");
                if let Some(signature) = expression_signature(expression) {
                    expression_signatures.push(signature);
                }
                if let Some(hash) = guard
                    .get("condition_expression_sha256")
                    .and_then(Value::as_str)
                {
                    expression_hashes.push(hash.to_string());
                    if hash.len() == 64
                        && state.deferred_action_commitments.iter().any(|commitment| {
                            guard.get("deferred_commitment_id").and_then(Value::as_str)
                                == Some(commitment.commitment_id.as_str())
                                && guard.get("condition_sha256").and_then(Value::as_str)
                                    == Some(commitment.condition_sha256.as_str())
                        })
                    {
                        linked_expression_hashes += 1;
                    }
                }
                authority_violation |= expression_has_authority(expression)
                    || guard
                        .get("semantic_authority")
                        .and_then(Value::as_bool)
                        .unwrap_or(true)
                    || guard
                        .get("external_execution_authorized")
                        .and_then(Value::as_bool)
                        .unwrap_or(true);
            }
        }
        let pending_commitments = state
            .deferred_action_commitments
            .iter()
            .filter(|commitment| commitment.status == DeferredCommitmentStatusIR::ConditionPending)
            .count();
        let activated_commitments = state
            .deferred_action_commitments
            .iter()
            .filter(|commitment| commitment.status == DeferredCommitmentStatusIR::Activated)
            .count();
        let clarification_required =
            last_disposition == ConversationTurnDispositionIR::ClarificationRequired;
        let stable_rebinding = !case.require_stable_rebinding
            || (expression_hashes.len() >= 2
                && expression_hashes.windows(2).all(|pair| pair[0] == pair[1]));
        let expected_signature_matches = match case.expected_signature {
            Some(expected) => {
                expression_signatures.len() == guarded_steps
                    && expression_signatures
                        .iter()
                        .all(|signature| signature == expected)
            }
            None => expression_signatures.is_empty() && guarded_steps == 0,
        };
        let pass = !turn_failed
            && schemas_valid
            && guarded_programs == case.expected_guarded_programs
            && expected_signature_matches
            && expression_hashes.len() == guarded_steps
            && linked_expression_hashes == guarded_steps
            && pending_commitments == case.expected_pending_commitments
            && activated_commitments == 0
            && clarification_required == case.expected_clarification
            && stable_rebinding
            && !authority_violation;
        rows.push(Row {
            id: case.id.to_string(),
            category: case.category.to_string(),
            state_schema: state.schema.clone(),
            guarded_programs,
            guarded_steps,
            expression_signatures,
            expression_hashes,
            linked_expression_hashes,
            pending_commitments,
            activated_commitments,
            clarification_required,
            stable_rebinding,
            authority_violation,
            pass,
        });
    }
    let passed = rows.iter().filter(|row| row.pass).count();
    let total = rows.len();
    println!(
        "{}",
        serde_json::to_string(&Summary {
            schema: "B_CORE_R49_COMPOUND_GUARD_EXPRESSION_CANARY_1",
            suite,
            total,
            passed,
            failed: total - passed,
            external_llm_calls: 0,
            local_teacher_calls: 0,
            network_calls: 0,
            recursive_source_mutations: 0,
            rows,
        })
        .expect("summary json")
    );
    if passed != total {
        std::process::exit(1);
    }
}
