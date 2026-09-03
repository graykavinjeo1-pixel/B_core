//! Frozen R39-RUN-0001 topic/group restoration diagnostic.
//!
//! Frozen before the R39 product mechanism exists. The oracle requires topic
//! state to anchor an exact discourse-group identity and to refresh its live
//! revision after suspension without acquiring semantic or execution authority.

use semantic_core_adapters::{
    CognitiveApi, ConversationInputModalityIR, ConversationTurnDispositionIR,
    ConversationTurnRequestIR, LanguageCodeIR, CONVERSATION_TURN_REQUEST_SCHEMA,
};
use serde::Serialize;
use serde_json::{json, Value};

#[derive(Debug, Serialize)]
struct Row {
    id: String,
    category: String,
    pass: bool,
    trace: Vec<String>,
}

fn request(id: &str, turn: u64, text: &str, language: LanguageCodeIR) -> ConversationTurnRequestIR {
    ConversationTurnRequestIR {
        schema: CONVERSATION_TURN_REQUEST_SCHEMA.to_string(),
        conversation_id: id.to_string(),
        turn_index: turn,
        request_id: format!("{id}-{turn}"),
        modality: ConversationInputModalityIR::Text,
        raw_text: text.to_string(),
        input_confidence_millis: 1_000,
        alternatives: Vec::new(),
        output_language: Some(language),
        context_tags: Vec::new(),
        max_plan_steps: 16,
    }
}

fn run(
    api: &mut CognitiveApi,
    id: &str,
    turn: u64,
    text: &str,
    language: LanguageCodeIR,
) -> semantic_core_adapters::ConversationTurnResponseIR {
    api.process_conversation_turn(&request(id, turn, text, language))
        .expect("R39 turn")
}

fn value(response: &semantic_core_adapters::ConversationTurnResponseIR) -> Value {
    serde_json::to_value(response).expect("response json")
}

fn safe(response: &semantic_core_adapters::ConversationTurnResponseIR) -> bool {
    response.grounded_realization.validate()
        && response.grounded_realization.realized_text == response.output.text
        && response.grounded_realization.unsupported_claims == 0
        && !response.grounded_realization.semantic_authority
        && !response.grounded_realization.external_action_executed
}

fn sorted(mut values: Vec<String>) -> Vec<String> {
    values.sort();
    values.dedup();
    values
}

fn groups(observed: &Value) -> &[Value] {
    observed
        .pointer("/conversation_state/active_discourse_groups")
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or(&[])
}

fn group_members(group: &Value) -> Vec<String> {
    sorted(
        group["member_keys"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .map(str::to_string)
            .collect(),
    )
}

fn ledger_ids(observed: &Value) -> Vec<String> {
    observed
        .pointer("/conversation_state/action_state_ledger/records")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|record| record["goal_id"].as_str().map(str::to_string))
        .collect()
}

fn group_by_members<'a>(observed: &'a Value, expected: &[String]) -> Option<&'a Value> {
    let expected = sorted(expected.to_vec());
    groups(observed)
        .iter()
        .find(|group| group_members(group) == expected)
}

fn group_by_id<'a>(observed: &'a Value, group_id: &str) -> Option<&'a Value> {
    groups(observed)
        .iter()
        .find(|group| group["group_id"] == group_id)
}

fn active_topic(observed: &Value) -> Option<&Value> {
    observed
        .pointer("/conversation_state/active_topics/0")
        .filter(|topic| topic["explicitly_activated"] == true)
}

fn topic_anchors_group(observed: &Value, group: &Value) -> bool {
    let Some(topic) = active_topic(observed) else {
        return false;
    };
    let expected_kind = match group["kind"].as_str() {
        Some("ACTION") => "ACTION_GROUP",
        Some("ATTRIBUTED_PROPOSITION") => "ATTRIBUTED_PROPOSITION_GROUP",
        _ => return false,
    };
    topic["anchor_kind"] == expected_kind
        && topic["anchor_group_id"] == group["group_id"]
        && topic["anchor_group_revision"] == group["revision"]
        && topic["anchor_membership_sha256"] == group["membership_sha256"]
        && topic["topic_sha256"]
            .as_str()
            .is_some_and(|hash| hash.len() == 64)
        && topic["semantic_authority"] == false
        && topic["external_execution_authorized"] == false
}

fn transition_is_grounded(observed: &Value, kind: &str, group: &Value) -> bool {
    let Some(transition) = observed.pointer("/topic_transition") else {
        return false;
    };
    let Some(topic) = active_topic(observed) else {
        return false;
    };
    transition["schema"] == "B_CORE_TOPIC_TRANSITION_IR_1"
        && transition["kind"] == kind
        && transition["applied"] == true
        && transition["anchor_group_id"] == group["group_id"]
        && transition["anchor_group_revision"] == group["revision"]
        && transition["anchor_membership_sha256"] == group["membership_sha256"]
        && transition["unresolved_terms"]
            .as_array()
            .is_some_and(Vec::is_empty)
        && transition["transition_sha256"]
            .as_str()
            .is_some_and(|hash| hash.len() == 64)
        && transition["semantic_authority"] == false
        && transition["external_action_executed"] == false
        && observed
            .pointer("/grounded_realization/claims")
            .and_then(Value::as_array)
            .is_some_and(|claims| {
                claims.iter().any(|claim| {
                    claim["kind"] == "DISCOURSE_TOPIC_TRANSITION"
                        && claim["evidence_refs"].as_array().is_some_and(|refs| {
                            refs.iter().any(|item| item == &topic["topic_sha256"])
                                && refs.iter().any(|item| item == &group["membership_sha256"])
                        })
                        && claim["semantic_authority"] == false
                        && claim["external_action_executed"] == false
                })
            })
}

fn no_action_projection(observed: &Value) -> bool {
    observed["grounded_response"].is_null()
        && observed
            .pointer("/action_state_analysis/target_action_ids")
            .and_then(Value::as_array)
            .is_some_and(Vec::is_empty)
        && observed["discourse_group_update"].is_null()
}

fn targets(observed: &Value) -> Vec<String> {
    sorted(
        observed
            .pointer("/action_state_analysis/target_action_ids")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .map(str::to_string)
            .collect(),
    )
}

fn binding_size(observed: &Value, kind: &str, size: usize) -> bool {
    observed
        .pointer("/reference_resolution/discourse_bindings")
        .and_then(Value::as_array)
        .is_some_and(|bindings| {
            bindings.iter().any(|binding| {
                binding["kind"] == kind
                    && binding["referent_ids"]
                        .as_array()
                        .is_some_and(|ids| ids.len() == size)
            })
        })
}

fn action_anchor(id: &str, setup: &str, activate: &str, language: LanguageCodeIR) -> Row {
    let mut api = CognitiveApi::new_embedded().expect("embedded core");
    let setup_response = run(&mut api, id, 1, setup, language);
    let setup_value = value(&setup_response);
    let members = ledger_ids(&setup_value);
    let group_id = group_by_members(&setup_value, &members)
        .and_then(|group| group["group_id"].as_str())
        .unwrap_or_default()
        .to_string();
    let ledger_before = ledger_ids(&setup_value);
    let activated = run(&mut api, id, 2, activate, language);
    let observed = value(&activated);
    let group = group_by_id(&observed, &group_id);
    Row {
        id: id.to_string(),
        category: "group_topic_activation".to_string(),
        pass: !group_id.is_empty()
            && group.is_some_and(|group| {
                topic_anchors_group(&observed, group)
                    && transition_is_grounded(&observed, "ACTIVATE_GROUP", group)
            })
            && ledger_ids(&observed) == ledger_before
            && no_action_projection(&observed)
            && activated.disposition == ConversationTurnDispositionIR::Grounded
            && safe(&activated),
        trace: vec![observed.to_string()],
    }
}

fn speaker_anchor(
    id: &str,
    statements: [&str; 2],
    establish: &str,
    activate: &str,
    language: LanguageCodeIR,
) -> Row {
    let mut api = CognitiveApi::new_embedded().expect("embedded core");
    run(&mut api, id, 1, statements[0], language);
    run(&mut api, id, 2, statements[1], language);
    let established = value(&run(&mut api, id, 3, establish, language));
    let group_id = groups(&established)
        .iter()
        .find(|group| group["kind"] == "ATTRIBUTED_PROPOSITION")
        .and_then(|group| group["group_id"].as_str())
        .unwrap_or_default()
        .to_string();
    let activated = run(&mut api, id, 4, activate, language);
    let observed = value(&activated);
    let group = group_by_id(&observed, &group_id);
    Row {
        id: id.to_string(),
        category: "group_topic_activation".to_string(),
        pass: !group_id.is_empty()
            && group.is_some_and(|group| {
                topic_anchors_group(&observed, group)
                    && transition_is_grounded(&observed, "ACTIVATE_GROUP", group)
            })
            && no_action_projection(&observed)
            && activated.disposition == ConversationTurnDispositionIR::Grounded
            && safe(&activated),
        trace: vec![observed.to_string()],
    }
}

#[allow(clippy::too_many_arguments)]
fn action_restore(
    id: &str,
    setup: &str,
    activate: &str,
    switch: &str,
    restore: &str,
    query: &str,
    language: LanguageCodeIR,
    category: &str,
) -> Row {
    let mut api = CognitiveApi::new_embedded().expect("embedded core");
    let setup_value = value(&run(&mut api, id, 1, setup, language));
    let expected = sorted(ledger_ids(&setup_value));
    let group_id = group_by_members(&setup_value, &expected)
        .and_then(|group| group["group_id"].as_str())
        .unwrap_or_default()
        .to_string();
    run(&mut api, id, 2, activate, language);
    run(&mut api, id, 3, switch, language);
    let restored = run(&mut api, id, 4, restore, language);
    let restored_value = value(&restored);
    let group = group_by_id(&restored_value, &group_id);
    let queried = run(&mut api, id, 5, query, language);
    let query_value = value(&queried);
    Row {
        id: id.to_string(),
        category: category.to_string(),
        pass: group.is_some_and(|group| {
            topic_anchors_group(&restored_value, group)
                && transition_is_grounded(&restored_value, "RETURN_PREVIOUS", group)
        }) && targets(&query_value) == expected
            && binding_size(&query_value, "PLURAL_EVENT_REFERENCE", expected.len())
            && safe(&restored)
            && safe(&queried),
        trace: vec![restored_value.to_string(), query_value.to_string()],
    }
}

#[allow(clippy::too_many_arguments)]
fn revised_action_restore(
    id: &str,
    setup: &str,
    extra: &str,
    activate: &str,
    switch: &str,
    update: &str,
    restore: &str,
    query: &str,
    language: LanguageCodeIR,
) -> Row {
    let mut api = CognitiveApi::new_embedded().expect("embedded core");
    let setup_value = value(&run(&mut api, id, 1, setup, language));
    let original = ledger_ids(&setup_value);
    let group_id = group_by_members(&setup_value, &original)
        .and_then(|group| group["group_id"].as_str())
        .unwrap_or_default()
        .to_string();
    let extra_value = value(&run(&mut api, id, 2, extra, language));
    let mut expected = ledger_ids(&extra_value);
    expected = sorted(expected);
    run(&mut api, id, 3, activate, language);
    run(&mut api, id, 4, switch, language);
    let updated = value(&run(&mut api, id, 5, update, language));
    let live_group = group_by_id(&updated, &group_id)
        .cloned()
        .unwrap_or(Value::Null);
    let restored = run(&mut api, id, 6, restore, language);
    let restored_value = value(&restored);
    let queried = run(&mut api, id, 7, query, language);
    let query_value = value(&queried);
    Row {
        id: id.to_string(),
        category: "suspended_action_group_revision_refresh".to_string(),
        pass: live_group["revision"] == 2
            && topic_anchors_group(&restored_value, &live_group)
            && transition_is_grounded(&restored_value, "RETURN_PREVIOUS", &live_group)
            && targets(&query_value) == expected
            && binding_size(&query_value, "PLURAL_EVENT_REFERENCE", expected.len())
            && safe(&restored)
            && safe(&queried),
        trace: vec![
            updated.to_string(),
            restored_value.to_string(),
            query_value.to_string(),
        ],
    }
}

#[allow(clippy::too_many_arguments)]
fn revised_speaker_restore(
    id: &str,
    turns: [&str; 4],
    activate: &str,
    switch: &str,
    update: &str,
    restore: &str,
    query: &str,
    expected_sources: &[&str],
    language: LanguageCodeIR,
) -> Row {
    let mut api = CognitiveApi::new_embedded().expect("embedded core");
    for (index, text) in turns.iter().enumerate() {
        run(
            &mut api,
            id,
            u64::try_from(index + 1).unwrap(),
            text,
            language,
        );
    }
    let established = value(&run(&mut api, id, 5, activate, language));
    let group_id = active_topic(&established)
        .and_then(|topic| topic["anchor_group_id"].as_str())
        .unwrap_or_default()
        .to_string();
    run(&mut api, id, 6, switch, language);
    let updated = value(&run(&mut api, id, 7, update, language));
    let live_group = group_by_id(&updated, &group_id)
        .cloned()
        .unwrap_or(Value::Null);
    let restored = run(&mut api, id, 8, restore, language);
    let restored_value = value(&restored);
    let queried = run(&mut api, id, 9, query, language);
    let query_value = value(&queried);
    let resolved = queried
        .reference_resolution
        .resolved_semantic_text
        .to_lowercase();
    Row {
        id: id.to_string(),
        category: "suspended_speaker_group_revision_refresh".to_string(),
        pass: live_group["revision"] == 2
            && topic_anchors_group(&restored_value, &live_group)
            && transition_is_grounded(&restored_value, "RETURN_PREVIOUS", &live_group)
            && binding_size(
                &query_value,
                "PLURAL_PROPOSITION_REFERENCE",
                expected_sources.len(),
            )
            && expected_sources
                .iter()
                .all(|source| resolved.contains(&source.to_lowercase()))
            && safe(&restored)
            && safe(&queried),
        trace: vec![
            updated.to_string(),
            restored_value.to_string(),
            query_value.to_string(),
        ],
    }
}

#[allow(clippy::too_many_arguments)]
fn composite_restore(
    id: &str,
    first: &str,
    second: &str,
    merge: &str,
    activate: &str,
    switch: &str,
    restore: &str,
    query: &str,
    language: LanguageCodeIR,
) -> Row {
    let mut api = CognitiveApi::new_embedded().expect("embedded core");
    run(&mut api, id, 1, first, language);
    let second_value = value(&run(&mut api, id, 2, second, language));
    let expected = sorted(ledger_ids(&second_value));
    let merged = value(&run(&mut api, id, 3, merge, language));
    let composite_id = groups(&merged)
        .iter()
        .find(|group| {
            group["component_group_ids"]
                .as_array()
                .is_some_and(|items| items.len() == 2)
        })
        .and_then(|group| group["group_id"].as_str())
        .unwrap_or_default()
        .to_string();
    run(&mut api, id, 4, activate, language);
    run(&mut api, id, 5, switch, language);
    let restored = run(&mut api, id, 6, restore, language);
    let restored_value = value(&restored);
    let composite = group_by_id(&restored_value, &composite_id);
    let queried = run(&mut api, id, 7, query, language);
    let query_value = value(&queried);
    Row {
        id: id.to_string(),
        category: "composite_group_topic_restoration".to_string(),
        pass: composite.is_some_and(|group| {
            topic_anchors_group(&restored_value, group)
                && transition_is_grounded(&restored_value, "RETURN_PREVIOUS", group)
        }) && targets(&query_value) == expected
            && binding_size(&query_value, "PLURAL_EVENT_REFERENCE", expected.len())
            && safe(&restored)
            && safe(&queried),
        trace: vec![restored_value.to_string(), query_value.to_string()],
    }
}

#[allow(clippy::too_many_arguments)]
fn overlapping_exact_anchor(
    id: &str,
    first: &str,
    second: &str,
    activate: &str,
    select_oldest: bool,
    switch: &str,
    restore: &str,
    query: &str,
    language: LanguageCodeIR,
) -> Row {
    let mut api = CognitiveApi::new_embedded().expect("embedded core");
    let first_value = value(&run(&mut api, id, 1, first, language));
    let first_members = sorted(ledger_ids(&first_value));
    let first_id = group_by_members(&first_value, &first_members)
        .and_then(|group| group["group_id"].as_str())
        .unwrap_or_default()
        .to_string();
    let second_value = value(&run(&mut api, id, 2, second, language));
    let all = ledger_ids(&second_value);
    let second_members = sorted(
        all.into_iter()
            .filter(|goal_id| !first_members.contains(goal_id))
            .collect(),
    );
    let second_id = group_by_members(&second_value, &second_members)
        .and_then(|group| group["group_id"].as_str())
        .unwrap_or_default()
        .to_string();
    let (expected_id, expected_members) = if select_oldest {
        (first_id, first_members)
    } else {
        (second_id, second_members)
    };
    run(&mut api, id, 3, activate, language);
    run(&mut api, id, 4, switch, language);
    let restored = run(&mut api, id, 5, restore, language);
    let restored_value = value(&restored);
    let group = group_by_id(&restored_value, &expected_id);
    let queried = run(&mut api, id, 6, query, language);
    let query_value = value(&queried);
    Row {
        id: id.to_string(),
        category: "exact_anchor_overlapping_groups".to_string(),
        pass: group.is_some_and(|group| topic_anchors_group(&restored_value, group))
            && targets(&query_value) == expected_members
            && binding_size(
                &query_value,
                "PLURAL_EVENT_REFERENCE",
                expected_members.len(),
            )
            && safe(&restored)
            && safe(&queried),
        trace: vec![restored_value.to_string(), query_value.to_string()],
    }
}

fn invalid_topic_request(
    id: &str,
    setup: &[&str],
    surface: &str,
    quoted: bool,
    language: LanguageCodeIR,
) -> Row {
    let mut api = CognitiveApi::new_embedded().expect("embedded core");
    let mut latest = Value::Null;
    for (index, text) in setup.iter().enumerate() {
        latest = value(&run(
            &mut api,
            id,
            u64::try_from(index + 1).unwrap(),
            text,
            language,
        ));
    }
    let groups_before = groups(&latest).to_vec();
    let ledger_before = ledger_ids(&latest);
    let response = run(
        &mut api,
        id,
        u64::try_from(setup.len() + 1).unwrap(),
        surface,
        language,
    );
    let observed = value(&response);
    let transition = observed.pointer("/topic_transition");
    let transition_ok = if quoted {
        transition.is_none()
            && response.disposition != ConversationTurnDispositionIR::ClarificationRequired
    } else {
        transition.is_some_and(|transition| {
            transition["kind"] == "UNRESOLVED"
                && transition["applied"] == false
                && transition["unresolved_terms"]
                    .as_array()
                    .is_some_and(|terms| !terms.is_empty())
                && transition["semantic_authority"] == false
                && transition["external_action_executed"] == false
        }) && response.disposition == ConversationTurnDispositionIR::ClarificationRequired
    };
    Row {
        id: id.to_string(),
        category: "invalid_or_quoted_topic_restore_fails_closed".to_string(),
        pass: transition_ok
            && groups_before == groups(&observed)
            && ledger_before == ledger_ids(&observed)
            && safe(&response),
        trace: vec![observed.to_string()],
    }
}

fn main() {
    let mut rows = Vec::new();

    for (id, setup, activate, language) in [
        (
            "R39_AANCH_EN_1",
            "inspect cache and repair queue",
            "Make that task group the current topic.",
            LanguageCodeIR::English,
        ),
        (
            "R39_AANCH_KO_1",
            "캐시를 확인하고 큐를 수리해",
            "그 작업 묶음을 현재 주제로 두자",
            LanguageCodeIR::Korean,
        ),
    ] {
        rows.push(action_anchor(id, setup, activate, language));
    }

    for (id, statements, establish, activate, language) in [
        (
            "R39_SANCH_EN_1",
            [
                "Alice says the cache is stale.",
                "Bob says the queue is empty.",
            ],
            "What did Alice and Bob say?",
            "Make that speaker group the current topic.",
            LanguageCodeIR::English,
        ),
        (
            "R39_SANCH_KO_1",
            [
                "민수는 캐시가 오래됐다고 말했다.",
                "지수는 큐가 비었다고 말했다.",
            ],
            "민수와 지수는 뭐라고 말했어?",
            "그 화자 묶음을 현재 주제로 두자",
            LanguageCodeIR::Korean,
        ),
    ] {
        rows.push(speaker_anchor(
            id, statements, establish, activate, language,
        ));
    }

    for (id, setup, activate, switch, restore, query, language) in [
        (
            "R39_AREST_EN_1",
            "inspect cache and repair queue",
            "Make that task group the current topic.",
            "Back to the backup topic.",
            "Return to the previous topic.",
            "How is that task group doing?",
            LanguageCodeIR::English,
        ),
        (
            "R39_AREST_EN_2",
            "repair server and analyze log",
            "Keep the current task group as our topic.",
            "Go back to the worker topic.",
            "Back to the earlier topic.",
            "Show that task group's status.",
            LanguageCodeIR::English,
        ),
        (
            "R39_AREST_KO_1",
            "캐시를 확인하고 큐를 수리해",
            "그 작업 묶음을 현재 주제로 두자",
            "백업 주제로 돌아가자",
            "이전 주제로 돌아가자",
            "그 작업 묶음의 현황을 알려줘",
            LanguageCodeIR::Korean,
        ),
        (
            "R39_AREST_KO_2",
            "서버를 수리하고 로그를 분석해",
            "현재 작업 묶음을 화제로 삼자",
            "워커 이야기로 돌아가자",
            "아까 주제로 돌아가자",
            "그 작업 묶음의 상태를 보여줘",
            LanguageCodeIR::Korean,
        ),
    ] {
        rows.push(action_restore(
            id,
            setup,
            activate,
            switch,
            restore,
            query,
            language,
            "action_group_topic_restoration",
        ));
    }

    for (id, setup, extra, activate, switch, update, restore, query, language) in [
        (
            "R39_AREV_EN_1",
            "inspect cache and repair queue",
            "analyze worker",
            "Make the earlier task group the current topic.",
            "Back to the backup topic.",
            "Add worker to that earlier task group.",
            "Return to the previous topic.",
            "How is that task group doing?",
            LanguageCodeIR::English,
        ),
        (
            "R39_AREV_EN_2",
            "repair server and inspect log",
            "analyze backup",
            "Make the earlier task group our topic.",
            "Back to the cache topic.",
            "Include backup in that earlier task group.",
            "Back to the earlier topic.",
            "Show that task group's status.",
            LanguageCodeIR::English,
        ),
        (
            "R39_AREV_KO_1",
            "캐시를 확인하고 큐를 수리해",
            "워커를 분석해",
            "앞 작업 묶음을 현재 주제로 두자",
            "백업 주제로 돌아가자",
            "앞 작업 묶음에 워커를 추가해",
            "이전 주제로 돌아가자",
            "그 작업 묶음의 현황을 알려줘",
            LanguageCodeIR::Korean,
        ),
        (
            "R39_AREV_KO_2",
            "서버를 수리하고 로그를 확인해",
            "백업을 분석해",
            "앞 작업 묶음을 화제로 삼자",
            "캐시 주제로 돌아가자",
            "앞 작업 묶음에 백업을 포함해",
            "아까 주제로 돌아가자",
            "그 작업 묶음의 상태를 보여줘",
            LanguageCodeIR::Korean,
        ),
    ] {
        rows.push(revised_action_restore(
            id, setup, extra, activate, switch, update, restore, query, language,
        ));
    }

    rows.push(revised_speaker_restore(
        "R39_SREV_EN_1",
        [
            "Alice says the cache is stale.",
            "Bob says the queue is empty.",
            "What did Alice and Bob say?",
            "Carol says the worker is ready.",
        ],
        "Make that earlier speaker group the topic.",
        "Back to the backup topic.",
        "Add Carol to that earlier speaker group.",
        "Return to the previous topic.",
        "What did that speaker group say?",
        &["alice", "bob", "carol"],
        LanguageCodeIR::English,
    ));
    rows.push(revised_speaker_restore(
        "R39_SREV_EN_2",
        [
            "Mina says the server is slow.",
            "Jin says the log is clean.",
            "What did Mina and Jin say?",
            "Nora says the backup exists.",
        ],
        "Make that earlier speaker group our topic.",
        "Back to the cache topic.",
        "Include Nora in that earlier speaker group.",
        "Back to the earlier topic.",
        "Summarize what that speaker group said.",
        &["mina", "jin", "nora"],
        LanguageCodeIR::English,
    ));
    rows.push(revised_speaker_restore(
        "R39_SREV_KO_1",
        [
            "민수는 캐시가 오래됐다고 말했다.",
            "지수는 큐가 비었다고 말했다.",
            "민수와 지수는 뭐라고 말했어?",
            "철수는 워커가 준비됐다고 말했다.",
        ],
        "앞 화자 묶음을 현재 주제로 두자",
        "백업 주제로 돌아가자",
        "앞 화자 묶음에 철수를 추가해",
        "이전 주제로 돌아가자",
        "그 화자 묶음은 뭐라고 말했어?",
        &["민수", "지수", "철수"],
        LanguageCodeIR::Korean,
    ));
    rows.push(revised_speaker_restore(
        "R39_SREV_KO_2",
        [
            "영희는 서버가 느리다고 말했다.",
            "수진은 로그가 깨끗하다고 말했다.",
            "영희와 수진은 뭐라고 말했어?",
            "동수는 백업이 있다고 말했다.",
        ],
        "앞 화자 묶음을 화제로 삼자",
        "캐시 주제로 돌아가자",
        "앞 화자 묶음에 동수를 포함해",
        "아까 주제로 돌아가자",
        "그 화자 묶음이 한 말을 요약해",
        &["영희", "수진", "동수"],
        LanguageCodeIR::Korean,
    ));

    for (id, first, second, merge, activate, switch, restore, query, language) in [
        (
            "R39_COMP_EN_1",
            "inspect cache and repair queue",
            "analyze worker and repair server",
            "Combine the earlier and later task groups.",
            "Make the combined task group the topic.",
            "Back to the backup topic.",
            "Return to the previous topic.",
            "How is that task group doing?",
            LanguageCodeIR::English,
        ),
        (
            "R39_COMP_EN_2",
            "repair backup and inspect log",
            "analyze cache and repair worker",
            "Merge the first and second task groups.",
            "Keep the merged task group as our topic.",
            "Back to the server topic.",
            "Back to the earlier topic.",
            "Show that task group's status.",
            LanguageCodeIR::English,
        ),
        (
            "R39_COMP_KO_1",
            "캐시를 확인하고 큐를 수리해",
            "워커를 분석하고 서버를 수리해",
            "앞과 뒤 작업 묶음을 합쳐",
            "합친 작업 묶음을 주제로 두자",
            "백업 주제로 돌아가자",
            "이전 주제로 돌아가자",
            "그 작업 묶음의 현황을 알려줘",
            LanguageCodeIR::Korean,
        ),
        (
            "R39_COMP_KO_2",
            "백업을 수리하고 로그를 확인해",
            "캐시를 분석하고 워커를 수리해",
            "첫 번째와 두 번째 작업 묶음을 병합해",
            "병합한 작업 묶음을 화제로 삼자",
            "서버 주제로 돌아가자",
            "아까 주제로 돌아가자",
            "그 작업 묶음의 상태를 보여줘",
            LanguageCodeIR::Korean,
        ),
    ] {
        rows.push(composite_restore(
            id, first, second, merge, activate, switch, restore, query, language,
        ));
    }

    for (id, first, second, activate, oldest, switch, restore, query, language) in [
        (
            "R39_EXACT_EN_1",
            "inspect cache and repair queue",
            "analyze cache and repair worker",
            "Make the first task group the topic.",
            true,
            "Back to the backup topic.",
            "Return to the previous topic.",
            "How is that task group doing?",
            LanguageCodeIR::English,
        ),
        (
            "R39_EXACT_EN_2",
            "repair server and inspect log",
            "analyze server and repair backup",
            "Make the second task group the topic.",
            false,
            "Back to the cache topic.",
            "Back to the earlier topic.",
            "Show that task group's status.",
            LanguageCodeIR::English,
        ),
        (
            "R39_EXACT_KO_1",
            "캐시를 확인하고 큐를 수리해",
            "캐시를 분석하고 워커를 수리해",
            "첫 번째 작업 묶음을 주제로 두자",
            true,
            "백업 주제로 돌아가자",
            "이전 주제로 돌아가자",
            "그 작업 묶음의 현황을 알려줘",
            LanguageCodeIR::Korean,
        ),
        (
            "R39_EXACT_KO_2",
            "서버를 수리하고 로그를 확인해",
            "서버를 분석하고 백업을 수리해",
            "두 번째 작업 묶음을 화제로 삼자",
            false,
            "캐시 주제로 돌아가자",
            "아까 주제로 돌아가자",
            "그 작업 묶음의 상태를 보여줘",
            LanguageCodeIR::Korean,
        ),
    ] {
        rows.push(overlapping_exact_anchor(
            id, first, second, activate, oldest, switch, restore, query, language,
        ));
    }

    rows.push(invalid_topic_request(
        "R39_SAFE_EN_1",
        &[],
        "Make that task group the topic.",
        false,
        LanguageCodeIR::English,
    ));
    rows.push(invalid_topic_request(
        "R39_SAFE_EN_2",
        &[
            "inspect cache and repair queue",
            "analyze worker and repair server",
        ],
        "Make that task group the topic.",
        false,
        LanguageCodeIR::English,
    ));
    rows.push(invalid_topic_request(
        "R39_SAFE_KO_1",
        &["캐시를 확인하고 큐를 수리해"],
        "‘그 작업 묶음을 주제로 두자’라는 문장을 설명해",
        true,
        LanguageCodeIR::Korean,
    ));
    rows.push(invalid_topic_request(
        "R39_SAFE_KO_2",
        &[
            "민수는 캐시가 오래됐다고 말했다.",
            "지수는 큐가 비었다고 말했다.",
            "민수와 지수는 뭐라고 말했어?",
            "영희는 서버가 느리다고 말했다.",
            "수진은 로그가 깨끗하다고 말했다.",
            "영희와 수진은 뭐라고 말했어?",
        ],
        "그 화자 묶음을 주제로 두자",
        false,
        LanguageCodeIR::Korean,
    ));

    let passed = rows.iter().filter(|row| row.pass).count();
    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "suite": "R39-RUN-0001",
            "frozen_before_product_changes": true,
            "total": rows.len(),
            "passed": passed,
            "failed": rows.len() - passed,
            "external_llm_calls": 0,
            "local_teacher_calls": 0,
            "network_calls": 0,
            "recursive_source_mutations": 0,
            "rows": rows,
        }))
        .expect("suite json")
    );
}
