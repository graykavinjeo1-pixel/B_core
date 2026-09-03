//! Frozen R38-RUN-0001 discourse-group revision diagnostic.
//!
//! This evaluator is frozen before the R38 product mechanism exists. It tests
//! stable group identity across explicit member addition/removal, action and
//! attributed-speaker groups, composite group provenance, and fail-closed
//! invalid updates through the public conversation API.

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

#[derive(Clone, Copy)]
struct ActionRevisionCase {
    id: &'static str,
    setup: &'static str,
    extra: Option<&'static str>,
    update: &'static str,
    query: &'static str,
    member_surface: &'static str,
    remove: bool,
    language: LanguageCodeIR,
    category: &'static str,
}

#[derive(Clone, Copy)]
struct SpeakerRevisionCase {
    id: &'static str,
    first: &'static str,
    second: &'static str,
    establish: &'static str,
    extra: Option<&'static str>,
    update: &'static str,
    query: &'static str,
    member_key: &'static str,
    remove: bool,
    language: LanguageCodeIR,
    category: &'static str,
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

fn ledger_ids(observed: &Value) -> Vec<String> {
    observed
        .pointer("/conversation_state/action_state_ledger/records")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|record| record["action_id"].as_str().map(str::to_string))
        .collect()
}

fn goal_id_for_subject(observed: &Value, surface: &str) -> Option<String> {
    observed
        .pointer("/conversation_state/action_state_ledger/records")
        .and_then(Value::as_array)?
        .iter()
        .find(|record| {
            record["subject"]
                .as_str()
                .is_some_and(|subject| subject.eq_ignore_ascii_case(surface))
        })?["goal_id"]
        .as_str()
        .map(str::to_string)
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

fn groups(observed: &Value) -> &[Value] {
    observed
        .pointer("/conversation_state/active_discourse_groups")
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or(&[])
}

fn member_keys(group: &Value) -> Vec<String> {
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

fn group_with_members<'a>(observed: &'a Value, members: &[String]) -> Option<&'a Value> {
    let expected = sorted(members.to_vec());
    groups(observed)
        .iter()
        .find(|group| member_keys(group) == expected)
}

fn group_by_id<'a>(observed: &'a Value, group_id: &str) -> Option<&'a Value> {
    groups(observed)
        .iter()
        .find(|group| group["group_id"] == group_id)
}

fn valid_group(group: &Value, expected: &[String], revision: u64, components: usize) -> bool {
    member_keys(group) == sorted(expected.to_vec())
        && group["revision"] == revision
        && group["membership_sha256"]
            .as_str()
            .is_some_and(|hash| hash.len() == 64)
        && group["component_group_ids"]
            .as_array()
            .is_some_and(|items| items.len() == components)
        && group["semantic_authority"] == false
        && group["external_execution_authorized"] == false
}

fn valid_update(observed: &Value, operation: &str, applied: bool) -> bool {
    observed
        .pointer("/discourse_group_update")
        .is_some_and(|update| {
            update["operation"] == operation
                && update["applied"] == applied
                && update["semantic_authority"] == false
                && update["external_action_executed"] == false
                && update["update_sha256"]
                    .as_str()
                    .is_some_and(|hash| hash.len() == 64)
                && if applied {
                    update["unresolved_terms"]
                        .as_array()
                        .is_some_and(Vec::is_empty)
                } else {
                    update["unresolved_terms"]
                        .as_array()
                        .is_some_and(|items| !items.is_empty())
                }
        })
}

fn no_action_projection(observed: &Value) -> bool {
    observed["grounded_response"].is_null()
        && observed
            .pointer("/action_state_analysis/target_action_ids")
            .and_then(Value::as_array)
            .is_some_and(Vec::is_empty)
        && observed
            .pointer("/action_state_analysis/detected_reports")
            .and_then(Value::as_array)
            .is_none_or(Vec::is_empty)
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

fn action_revision(case: ActionRevisionCase) -> Row {
    let mut api = CognitiveApi::new_embedded().expect("embedded core");
    let setup = api
        .process_conversation_turn(&request(case.id, 1, case.setup, case.language))
        .expect("action setup");
    let setup_value = value(&setup);
    let mut original_members = ledger_ids(&setup_value);
    let original_group = group_with_members(&setup_value, &original_members);
    let original_id = original_group
        .and_then(|group| group["group_id"].as_str())
        .unwrap_or_default()
        .to_string();
    let mut turn = 2_u64;
    let mut latest = setup_value;
    if let Some(extra) = case.extra {
        let response = api
            .process_conversation_turn(&request(case.id, turn, extra, case.language))
            .expect("extra action");
        latest = value(&response);
        turn += 1;
    }
    let member_id = goal_id_for_subject(&latest, case.member_surface).unwrap_or_default();
    if case.remove {
        original_members.retain(|goal_id| goal_id != &member_id);
    } else {
        original_members.push(member_id.clone());
    }
    let expected = sorted(original_members);
    let ledger_before = ledger_ids(&latest);
    let update = api
        .process_conversation_turn(&request(case.id, turn, case.update, case.language))
        .expect("group revision");
    let update_value = value(&update);
    turn += 1;
    let revised = group_by_id(&update_value, &original_id);
    let query = api
        .process_conversation_turn(&request(case.id, turn, case.query, case.language))
        .expect("revised group query");
    let query_value = value(&query);
    Row {
        id: case.id.to_string(),
        category: case.category.to_string(),
        pass: !original_id.is_empty()
            && !member_id.is_empty()
            && revised.is_some_and(|group| valid_group(group, &expected, 2, 0))
            && valid_update(
                &update_value,
                if case.remove {
                    "REMOVE_MEMBER"
                } else {
                    "ADD_MEMBER"
                },
                true,
            )
            && no_action_projection(&update_value)
            && ledger_ids(&update_value) == ledger_before
            && targets(&query_value) == expected
            && binding_size(&query_value, "PLURAL_EVENT_REFERENCE", expected.len())
            && query.disposition == ConversationTurnDispositionIR::Grounded
            && safe(&update)
            && safe(&query),
        trace: vec![update_value.to_string(), query_value.to_string()],
    }
}

fn speaker_revision(case: SpeakerRevisionCase) -> Row {
    let mut api = CognitiveApi::new_embedded().expect("embedded core");
    api.process_conversation_turn(&request(case.id, 1, case.first, case.language))
        .expect("first speaker");
    api.process_conversation_turn(&request(case.id, 2, case.second, case.language))
        .expect("second speaker");
    let established = api
        .process_conversation_turn(&request(case.id, 3, case.establish, case.language))
        .expect("speaker group establishment");
    let established_value = value(&established);
    let original_group = groups(&established_value)
        .iter()
        .find(|group| group["kind"] == "ATTRIBUTED_PROPOSITION");
    let original_id = original_group
        .and_then(|group| group["group_id"].as_str())
        .unwrap_or_default()
        .to_string();
    let mut expected = original_group.map(member_keys).unwrap_or_default();
    let mut turn = 4_u64;
    if let Some(extra) = case.extra {
        api.process_conversation_turn(&request(case.id, turn, extra, case.language))
            .expect("extra speaker");
        turn += 1;
    }
    if case.remove {
        expected.retain(|member| member != case.member_key);
    } else {
        expected.push(case.member_key.to_string());
    }
    let expected = sorted(expected);
    let update = api
        .process_conversation_turn(&request(case.id, turn, case.update, case.language))
        .expect("speaker group revision");
    let update_value = value(&update);
    turn += 1;
    let revised = group_by_id(&update_value, &original_id);
    let query = api
        .process_conversation_turn(&request(case.id, turn, case.query, case.language))
        .expect("revised speaker group query");
    let query_value = value(&query);
    let resolved = query
        .reference_resolution
        .resolved_semantic_text
        .to_lowercase();
    Row {
        id: case.id.to_string(),
        category: case.category.to_string(),
        pass: !original_id.is_empty()
            && revised.is_some_and(|group| valid_group(group, &expected, 2, 0))
            && valid_update(
                &update_value,
                if case.remove {
                    "REMOVE_MEMBER"
                } else {
                    "ADD_MEMBER"
                },
                true,
            )
            && no_action_projection(&update_value)
            && binding_size(&query_value, "PLURAL_PROPOSITION_REFERENCE", expected.len())
            && expected.iter().all(|member| resolved.contains(member))
            && query.disposition == ConversationTurnDispositionIR::Grounded
            && safe(&update)
            && safe(&query),
        trace: vec![update_value.to_string(), query_value.to_string()],
    }
}

fn action_merge(
    id: &str,
    first: &str,
    second: &str,
    merge: &str,
    query: &str,
    language: LanguageCodeIR,
) -> Row {
    let mut api = CognitiveApi::new_embedded().expect("embedded core");
    let first_response = api
        .process_conversation_turn(&request(id, 1, first, language))
        .expect("first action group");
    let first_value = value(&first_response);
    let first_members = ledger_ids(&first_value);
    let first_id = group_with_members(&first_value, &first_members)
        .and_then(|group| group["group_id"].as_str())
        .unwrap_or_default()
        .to_string();
    let second_response = api
        .process_conversation_turn(&request(id, 2, second, language))
        .expect("second action group");
    let second_value = value(&second_response);
    let all_ids = ledger_ids(&second_value);
    let second_members = all_ids
        .iter()
        .filter(|goal_id| !first_members.contains(goal_id))
        .cloned()
        .collect::<Vec<_>>();
    let second_id = group_with_members(&second_value, &second_members)
        .and_then(|group| group["group_id"].as_str())
        .unwrap_or_default()
        .to_string();
    let merged = api
        .process_conversation_turn(&request(id, 3, merge, language))
        .expect("action group merge");
    let merged_value = value(&merged);
    let expected = sorted(all_ids);
    let composite = groups(&merged_value).iter().find(|group| {
        valid_group(group, &expected, 1, 2)
            && group["component_group_ids"]
                .as_array()
                .is_some_and(|items| {
                    items.iter().any(|item| item == &first_id)
                        && items.iter().any(|item| item == &second_id)
                })
    });
    let queried = api
        .process_conversation_turn(&request(id, 4, query, language))
        .expect("composite action query");
    let query_value = value(&queried);
    Row {
        id: id.to_string(),
        category: "action_group_merge".to_string(),
        pass: !first_id.is_empty()
            && !second_id.is_empty()
            && composite.is_some()
            && valid_update(&merged_value, "MERGE_GROUPS", true)
            && no_action_projection(&merged_value)
            && targets(&query_value) == expected
            && binding_size(&query_value, "PLURAL_EVENT_REFERENCE", 4)
            && merged.disposition == ConversationTurnDispositionIR::Grounded
            && queried.disposition == ConversationTurnDispositionIR::Grounded
            && safe(&merged)
            && safe(&queried),
        trace: vec![merged_value.to_string(), query_value.to_string()],
    }
}

#[allow(clippy::too_many_arguments)]
fn speaker_merge(
    id: &str,
    first: [&str; 2],
    establish_first: &str,
    second: [&str; 2],
    establish_second: &str,
    merge: &str,
    query: &str,
    expected: &[&str],
    language: LanguageCodeIR,
) -> Row {
    let mut api = CognitiveApi::new_embedded().expect("embedded core");
    api.process_conversation_turn(&request(id, 1, first[0], language))
        .expect("speaker one");
    api.process_conversation_turn(&request(id, 2, first[1], language))
        .expect("speaker two");
    api.process_conversation_turn(&request(id, 3, establish_first, language))
        .expect("first group");
    api.process_conversation_turn(&request(id, 4, second[0], language))
        .expect("speaker three");
    api.process_conversation_turn(&request(id, 5, second[1], language))
        .expect("speaker four");
    let established = api
        .process_conversation_turn(&request(id, 6, establish_second, language))
        .expect("second group");
    let established_value = value(&established);
    let source_groups = groups(&established_value)
        .iter()
        .filter(|group| group["kind"] == "ATTRIBUTED_PROPOSITION")
        .filter_map(|group| group["group_id"].as_str().map(str::to_string))
        .collect::<Vec<_>>();
    let merged = api
        .process_conversation_turn(&request(id, 7, merge, language))
        .expect("speaker group merge");
    let merged_value = value(&merged);
    let expected_keys = sorted(expected.iter().map(|item| (*item).to_string()).collect());
    let composite = groups(&merged_value).iter().find(|group| {
        valid_group(group, &expected_keys, 1, 2) && group["kind"] == "ATTRIBUTED_PROPOSITION"
    });
    let queried = api
        .process_conversation_turn(&request(id, 8, query, language))
        .expect("composite speaker query");
    let query_value = value(&queried);
    let resolved = queried
        .reference_resolution
        .resolved_semantic_text
        .to_lowercase();
    Row {
        id: id.to_string(),
        category: "speaker_group_merge".to_string(),
        pass: source_groups.len() == 2
            && composite.is_some()
            && valid_update(&merged_value, "MERGE_GROUPS", true)
            && no_action_projection(&merged_value)
            && binding_size(&query_value, "PLURAL_PROPOSITION_REFERENCE", 4)
            && expected_keys.iter().all(|member| resolved.contains(member))
            && merged.disposition == ConversationTurnDispositionIR::Grounded
            && queried.disposition == ConversationTurnDispositionIR::Grounded
            && safe(&merged)
            && safe(&queried),
        trace: vec![merged_value.to_string(), query_value.to_string()],
    }
}

fn invalid_update(id: &str, setup: &[&str], update: &str, language: LanguageCodeIR) -> Row {
    let mut api = CognitiveApi::new_embedded().expect("embedded core");
    let mut before = Value::Null;
    for (index, text) in setup.iter().enumerate() {
        before = value(
            &api.process_conversation_turn(&request(
                id,
                u64::try_from(index + 1).expect("bounded turn"),
                text,
                language,
            ))
            .expect("invalid setup"),
        );
    }
    let before_groups = groups(&before)
        .iter()
        .map(|group| {
            (
                group["group_id"].clone(),
                group["member_keys"].clone(),
                group["revision"].clone(),
                group["membership_sha256"].clone(),
            )
        })
        .collect::<Vec<_>>();
    let ledger_before = ledger_ids(&before);
    let response = api
        .process_conversation_turn(&request(
            id,
            u64::try_from(setup.len() + 1).expect("bounded turn"),
            update,
            language,
        ))
        .expect("invalid update response");
    let observed = value(&response);
    let after_groups = groups(&observed)
        .iter()
        .map(|group| {
            (
                group["group_id"].clone(),
                group["member_keys"].clone(),
                group["revision"].clone(),
                group["membership_sha256"].clone(),
            )
        })
        .collect::<Vec<_>>();
    Row {
        id: id.to_string(),
        category: "invalid_or_ambiguous_revision_fails_closed".to_string(),
        pass: valid_update(&observed, "UNRESOLVED", false)
            && before_groups == after_groups
            && ledger_before == ledger_ids(&observed)
            && no_action_projection(&observed)
            && response.disposition == ConversationTurnDispositionIR::ClarificationRequired
            && safe(&response),
        trace: vec![observed.to_string()],
    }
}

fn main() {
    let mut rows = vec![
        action_revision(ActionRevisionCase {
            id: "R38_AADD_EN_1",
            setup: "inspect cache and repair queue",
            extra: Some("analyze worker"),
            update: "Add the worker task to that task group.",
            query: "Show the status of that task group.",
            member_surface: "worker",
            remove: false,
            language: LanguageCodeIR::English,
            category: "action_member_addition",
        }),
        action_revision(ActionRevisionCase {
            id: "R38_AADD_EN_2",
            setup: "repair server and inspect log",
            extra: Some("analyze backup"),
            update: "Include the backup action in the current task group.",
            query: "List the state of the current task group.",
            member_surface: "backup",
            remove: false,
            language: LanguageCodeIR::English,
            category: "action_member_addition",
        }),
        action_revision(ActionRevisionCase {
            id: "R38_AADD_KO_1",
            setup: "캐시를 확인하고 큐를 수리해",
            extra: Some("워커를 분석해"),
            update: "그 작업 묶음에 워커 작업을 추가해",
            query: "그 작업 묶음의 현황을 알려줘",
            member_surface: "워커",
            remove: false,
            language: LanguageCodeIR::Korean,
            category: "action_member_addition",
        }),
        action_revision(ActionRevisionCase {
            id: "R38_AADD_KO_2",
            setup: "서버를 수리하고 로그를 확인해",
            extra: Some("백업을 분석해"),
            update: "현재 작업 묶음에 백업을 포함해",
            query: "현재 작업 묶음의 상태를 보여줘",
            member_surface: "백업",
            remove: false,
            language: LanguageCodeIR::Korean,
            category: "action_member_addition",
        }),
        action_revision(ActionRevisionCase {
            id: "R38_AREM_EN_1",
            setup: "inspect cache, repair queue, and analyze worker",
            extra: None,
            update: "Remove the queue task from that task group.",
            query: "Show the status of that task group.",
            member_surface: "queue",
            remove: true,
            language: LanguageCodeIR::English,
            category: "action_member_removal",
        }),
        action_revision(ActionRevisionCase {
            id: "R38_AREM_EN_2",
            setup: "repair server, inspect log, and analyze backup",
            extra: None,
            update: "Leave the log action out of the current task group.",
            query: "List the state of the current task group.",
            member_surface: "log",
            remove: true,
            language: LanguageCodeIR::English,
            category: "action_member_removal",
        }),
        action_revision(ActionRevisionCase {
            id: "R38_AREM_KO_1",
            setup: "캐시를 확인하고 큐를 수리한 뒤 워커를 분석해",
            extra: None,
            update: "그 작업 묶음에서 큐 작업을 빼",
            query: "그 작업 묶음의 현황을 알려줘",
            member_surface: "큐",
            remove: true,
            language: LanguageCodeIR::Korean,
            category: "action_member_removal",
        }),
        action_revision(ActionRevisionCase {
            id: "R38_AREM_KO_2",
            setup: "서버를 수리하고 로그를 확인한 뒤 백업을 분석해",
            extra: None,
            update: "현재 작업 묶음에서 로그를 제외해",
            query: "현재 작업 묶음의 상태를 보여줘",
            member_surface: "로그",
            remove: true,
            language: LanguageCodeIR::Korean,
            category: "action_member_removal",
        }),
        speaker_revision(SpeakerRevisionCase {
            id: "R38_SADD_EN_1",
            first: "Alice says that the cache is stale.",
            second: "Bob says that the queue is empty.",
            establish: "What did Alice and Bob say?",
            extra: Some("Carol says that the worker is ready."),
            update: "Add Carol to that speaker group.",
            query: "What did that speaker group say?",
            member_key: "carol",
            remove: false,
            language: LanguageCodeIR::English,
            category: "speaker_member_addition",
        }),
        speaker_revision(SpeakerRevisionCase {
            id: "R38_SADD_EN_2",
            first: "Mina says that the server is slow.",
            second: "Jin says that the log is clean.",
            establish: "What did Mina and Jin say?",
            extra: Some("Nora says that the backup exists."),
            update: "Include Nora in the current speaker group.",
            query: "Summarize what the current speaker group said.",
            member_key: "nora",
            remove: false,
            language: LanguageCodeIR::English,
            category: "speaker_member_addition",
        }),
        speaker_revision(SpeakerRevisionCase {
            id: "R38_SADD_KO_1",
            first: "민수는 캐시가 오래됐다고 말했다.",
            second: "지수는 큐가 비었다고 말했다.",
            establish: "민수와 지수는 뭐라고 말했어?",
            extra: Some("철수는 워커가 준비됐다고 말했다."),
            update: "그 화자 묶음에 철수를 추가해",
            query: "그 화자 묶음은 뭐라고 말했어?",
            member_key: "철수",
            remove: false,
            language: LanguageCodeIR::Korean,
            category: "speaker_member_addition",
        }),
        speaker_revision(SpeakerRevisionCase {
            id: "R38_SADD_KO_2",
            first: "영희는 서버가 느리다고 말했다.",
            second: "수진은 로그가 깨끗하다고 말했다.",
            establish: "영희와 수진은 뭐라고 말했어?",
            extra: Some("동수는 백업이 있다고 말했다."),
            update: "현재 화자 묶음에 동수를 포함해",
            query: "현재 화자 묶음이 한 말을 요약해",
            member_key: "동수",
            remove: false,
            language: LanguageCodeIR::Korean,
            category: "speaker_member_addition",
        }),
    ];

    // Removal cases build a three-member group through a preceding typed add,
    // then exercise a distinct removal turn without changing the frozen oracle.
    for (id, turns, update, query, removed, language) in [
        (
            "R38_SREM_EN_1",
            [
                "Alice says that the cache is stale.",
                "Bob says that the queue is empty.",
                "What did Alice and Bob say?",
                "Carol says that the worker is ready.",
                "Add Carol to that speaker group.",
            ],
            "Remove Bob from that speaker group.",
            "What did that speaker group say?",
            "bob",
            LanguageCodeIR::English,
        ),
        (
            "R38_SREM_EN_2",
            [
                "Mina says that the server is slow.",
                "Jin says that the log is clean.",
                "What did Mina and Jin say?",
                "Nora says that the backup exists.",
                "Include Nora in the current speaker group.",
            ],
            "Leave Jin out of the current speaker group.",
            "Summarize what the current speaker group said.",
            "jin",
            LanguageCodeIR::English,
        ),
        (
            "R38_SREM_KO_1",
            [
                "민수는 캐시가 오래됐다고 말했다.",
                "지수는 큐가 비었다고 말했다.",
                "민수와 지수는 뭐라고 말했어?",
                "철수는 워커가 준비됐다고 말했다.",
                "그 화자 묶음에 철수를 추가해",
            ],
            "그 화자 묶음에서 지수를 빼",
            "그 화자 묶음은 뭐라고 말했어?",
            "지수",
            LanguageCodeIR::Korean,
        ),
        (
            "R38_SREM_KO_2",
            [
                "영희는 서버가 느리다고 말했다.",
                "수진은 로그가 깨끗하다고 말했다.",
                "영희와 수진은 뭐라고 말했어?",
                "동수는 백업이 있다고 말했다.",
                "현재 화자 묶음에 동수를 포함해",
            ],
            "현재 화자 묶음에서 수진을 제외해",
            "현재 화자 묶음이 한 말을 요약해",
            "수진",
            LanguageCodeIR::Korean,
        ),
    ] {
        let mut api = CognitiveApi::new_embedded().expect("embedded core");
        let mut latest = Value::Null;
        for (index, text) in turns.iter().enumerate() {
            latest = value(
                &api.process_conversation_turn(&request(
                    id,
                    u64::try_from(index + 1).unwrap(),
                    text,
                    language,
                ))
                .expect("speaker removal setup"),
            );
        }
        let before_group = groups(&latest).iter().find(|group| {
            group["kind"] == "ATTRIBUTED_PROPOSITION" && member_keys(group).len() == 3
        });
        let group_id = before_group
            .and_then(|group| group["group_id"].as_str())
            .unwrap_or_default()
            .to_string();
        let mut expected = before_group.map(member_keys).unwrap_or_default();
        expected.retain(|member| member != removed);
        let update_response = api
            .process_conversation_turn(&request(id, 6, update, language))
            .expect("speaker removal");
        let update_value = value(&update_response);
        let query_response = api
            .process_conversation_turn(&request(id, 7, query, language))
            .expect("speaker removal query");
        let query_value = value(&query_response);
        let resolved = query_response
            .reference_resolution
            .resolved_semantic_text
            .to_lowercase();
        rows.push(Row {
            id: id.to_string(),
            category: "speaker_member_removal".to_string(),
            pass: !group_id.is_empty()
                && group_by_id(&update_value, &group_id)
                    .is_some_and(|group| valid_group(group, &expected, 3, 0))
                && valid_update(&update_value, "REMOVE_MEMBER", true)
                && no_action_projection(&update_value)
                && binding_size(&query_value, "PLURAL_PROPOSITION_REFERENCE", 2)
                && expected.iter().all(|member| resolved.contains(member))
                && !resolved.contains(removed)
                && safe(&update_response)
                && safe(&query_response),
            trace: vec![update_value.to_string(), query_value.to_string()],
        });
    }

    rows.extend([
        action_merge(
            "R38_AMERGE_EN_1",
            "inspect cache and repair queue",
            "analyze worker and inspect backup",
            "Combine the first task pair and the second task pair into one group.",
            "Show the status of that combined task group.",
            LanguageCodeIR::English,
        ),
        action_merge(
            "R38_AMERGE_EN_2",
            "repair server and inspect log",
            "analyze cache and repair queue",
            "Merge the earlier task group with the later task group.",
            "List the state of the merged task group.",
            LanguageCodeIR::English,
        ),
        action_merge(
            "R38_AMERGE_KO_1",
            "캐시를 확인하고 큐를 수리해",
            "워커를 분석하고 백업을 확인해",
            "첫 번째 작업 묶음과 두 번째 작업 묶음을 하나로 합쳐",
            "합친 작업 묶음의 현황을 알려줘",
            LanguageCodeIR::Korean,
        ),
        action_merge(
            "R38_AMERGE_KO_2",
            "서버를 수리하고 로그를 확인해",
            "캐시를 분석하고 큐를 수리해",
            "앞 작업 묶음과 뒤 작업 묶음을 병합해",
            "병합한 작업 묶음의 상태를 보여줘",
            LanguageCodeIR::Korean,
        ),
        speaker_merge(
            "R38_SMERGE_EN_1",
            [
                "Alice says the cache is stale.",
                "Bob says the queue is empty.",
            ],
            "What did Alice and Bob say?",
            [
                "Carol says the worker is ready.",
                "Dave says the backup exists.",
            ],
            "What did Carol and Dave say?",
            "Combine the first speaker group and the second speaker group.",
            "What did that combined speaker group say?",
            &["alice", "bob", "carol", "dave"],
            LanguageCodeIR::English,
        ),
        speaker_merge(
            "R38_SMERGE_EN_2",
            [
                "Mina says the server is slow.",
                "Jin says the log is clean.",
            ],
            "What did Mina and Jin say?",
            [
                "Nora says the cache is valid.",
                "Owen says the backup exists.",
            ],
            "What did Nora and Owen say?",
            "Merge the earlier speaker group with the later speaker group.",
            "Summarize what the merged speaker group said.",
            &["mina", "jin", "nora", "owen"],
            LanguageCodeIR::English,
        ),
        speaker_merge(
            "R38_SMERGE_KO_1",
            [
                "민수는 캐시가 오래됐다고 말했다.",
                "지수는 큐가 비었다고 말했다.",
            ],
            "민수와 지수는 뭐라고 말했어?",
            [
                "철수는 워커가 준비됐다고 말했다.",
                "영희는 백업이 있다고 말했다.",
            ],
            "철수와 영희는 뭐라고 말했어?",
            "첫 번째 화자 묶음과 두 번째 화자 묶음을 합쳐",
            "합친 화자 묶음은 뭐라고 말했어?",
            &["민수", "지수", "철수", "영희"],
            LanguageCodeIR::Korean,
        ),
        speaker_merge(
            "R38_SMERGE_KO_2",
            [
                "수진은 서버가 느리다고 말했다.",
                "동수는 로그가 깨끗하다고 말했다.",
            ],
            "수진과 동수는 뭐라고 말했어?",
            [
                "하나는 캐시가 유효하다고 말했다.",
                "미나는 백업이 있다고 말했다.",
            ],
            "하나와 미나는 뭐라고 말했어?",
            "앞 화자 묶음과 뒤 화자 묶음을 병합해",
            "병합한 화자 묶음이 한 말을 요약해",
            &["수진", "동수", "하나", "미나"],
            LanguageCodeIR::Korean,
        ),
        invalid_update(
            "R38_BAD_EN_1",
            &["inspect cache and repair queue"],
            "Add the database task to that task group.",
            LanguageCodeIR::English,
        ),
        invalid_update(
            "R38_BAD_EN_2",
            &["inspect cache and repair queue"],
            "Remove the worker task from that task group.",
            LanguageCodeIR::English,
        ),
        invalid_update(
            "R38_BAD_KO_1",
            &["캐시를 확인하고 큐를 수리해"],
            "그 작업 묶음에서 큐를 빼",
            LanguageCodeIR::Korean,
        ),
        invalid_update(
            "R38_BAD_KO_2",
            &[
                "캐시를 확인하고 큐를 수리해",
                "워커를 분석하고 백업을 확인해",
                "서버를 수리하고 로그를 확인해",
            ],
            "작업 묶음들을 하나로 합쳐",
            LanguageCodeIR::Korean,
        ),
    ]);

    rows.sort_by(|left, right| left.id.cmp(&right.id));
    let passed = rows.iter().filter(|row| row.pass).count();
    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "suite": "R38-RUN-0001",
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
        .expect("summary json")
    );
}
