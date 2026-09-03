//! Frozen R38-TRANSFER-0001 held-out discourse-group revision suite.
//!
//! Do not semantically execute this binary until R38-RUN-0001 passes.

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
        .expect("held-out turn")
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

fn members(group: &Value) -> Vec<String> {
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

fn group_by_members<'a>(observed: &'a Value, expected: &[String]) -> Option<&'a Value> {
    let expected = sorted(expected.to_vec());
    groups(observed)
        .iter()
        .find(|group| members(group) == expected)
}

fn group_by_id<'a>(observed: &'a Value, group_id: &str) -> Option<&'a Value> {
    groups(observed)
        .iter()
        .find(|group| group["group_id"] == group_id)
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

fn goal_for_subject(observed: &Value, subject: &str) -> Option<String> {
    observed
        .pointer("/conversation_state/action_state_ledger/records")
        .and_then(Value::as_array)?
        .iter()
        .find(|record| {
            record["subject"]
                .as_str()
                .is_some_and(|surface| surface.eq_ignore_ascii_case(subject))
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

fn valid_group(group: &Value, expected: &[String], revision: u64, components: usize) -> bool {
    members(group) == sorted(expected.to_vec())
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

fn update_applied(observed: &Value, operation: &str) -> bool {
    observed
        .pointer("/discourse_group_update")
        .is_some_and(|update| {
            update["operation"] == operation
                && update["applied"] == true
                && update["unresolved_terms"]
                    .as_array()
                    .is_some_and(Vec::is_empty)
                && update["semantic_authority"] == false
                && update["external_action_executed"] == false
                && update["update_sha256"]
                    .as_str()
                    .is_some_and(|hash| hash.len() == 64)
        })
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

#[allow(clippy::too_many_arguments)]
fn reversible(
    id: &str,
    setup: &str,
    extra: &str,
    add: &str,
    remove: &str,
    query: &str,
    member_surface: &str,
    language: LanguageCodeIR,
) -> Row {
    let mut api = CognitiveApi::new_embedded().expect("embedded core");
    let setup_response = run(&mut api, id, 1, setup, language);
    let setup_value = value(&setup_response);
    let expected = sorted(ledger_ids(&setup_value));
    let group_id = group_by_members(&setup_value, &expected)
        .and_then(|group| group["group_id"].as_str())
        .unwrap_or_default()
        .to_string();
    let extra_response = run(&mut api, id, 2, extra, language);
    let extra_value = value(&extra_response);
    let member = goal_for_subject(&extra_value, member_surface).unwrap_or_default();
    let added = run(&mut api, id, 3, add, language);
    let added_value = value(&added);
    let removed = run(&mut api, id, 4, remove, language);
    let removed_value = value(&removed);
    let queried = run(&mut api, id, 5, query, language);
    let query_value = value(&queried);
    Row {
        id: id.to_string(),
        category: "heldout_stable_identity_reversible_revision".to_string(),
        pass: !group_id.is_empty()
            && !member.is_empty()
            && update_applied(&added_value, "ADD_MEMBER")
            && update_applied(&removed_value, "REMOVE_MEMBER")
            && group_by_id(&removed_value, &group_id)
                .is_some_and(|group| valid_group(group, &expected, 3, 0))
            && targets(&query_value) == expected
            && binding_size(&query_value, "PLURAL_EVENT_REFERENCE", 2)
            && safe(&added)
            && safe(&removed)
            && safe(&queried),
        trace: vec![
            added_value.to_string(),
            removed_value.to_string(),
            query_value.to_string(),
        ],
    }
}

#[allow(clippy::too_many_arguments)]
fn overlapping_speaker_merge(
    id: &str,
    statements: [&str; 3],
    establish_first: &str,
    establish_second: &str,
    merge: &str,
    query: &str,
    expected: &[&str],
    language: LanguageCodeIR,
) -> Row {
    let mut api = CognitiveApi::new_embedded().expect("embedded core");
    for (index, statement) in statements.iter().enumerate() {
        run(
            &mut api,
            id,
            u64::try_from(index + 1).expect("bounded turn"),
            statement,
            language,
        );
    }
    run(&mut api, id, 4, establish_first, language);
    let established = run(&mut api, id, 5, establish_second, language);
    let established_value = value(&established);
    let parent_ids = groups(&established_value)
        .iter()
        .filter(|group| group["kind"] == "ATTRIBUTED_PROPOSITION")
        .filter_map(|group| group["group_id"].as_str())
        .map(str::to_string)
        .collect::<Vec<_>>();
    let merged = run(&mut api, id, 6, merge, language);
    let merged_value = value(&merged);
    let expected = sorted(expected.iter().map(|item| (*item).to_string()).collect());
    let composite = group_by_members(&merged_value, &expected);
    let queried = run(&mut api, id, 7, query, language);
    let query_value = value(&queried);
    let resolved = queried
        .reference_resolution
        .resolved_semantic_text
        .to_lowercase();
    Row {
        id: id.to_string(),
        category: "heldout_overlapping_composite_deduplication".to_string(),
        pass: parent_ids.len() == 2
            && composite.is_some_and(|group| valid_group(group, &expected, 1, 2))
            && update_applied(&merged_value, "MERGE_GROUPS")
            && binding_size(&query_value, "PLURAL_PROPOSITION_REFERENCE", 3)
            && expected.iter().all(|member| resolved.contains(member))
            && safe(&merged)
            && safe(&queried),
        trace: vec![merged_value.to_string(), query_value.to_string()],
    }
}

#[allow(clippy::too_many_arguments)]
fn cross_language_action(
    id: &str,
    setup: &str,
    extra: &str,
    update: &str,
    query: &str,
    member_surface: &str,
    setup_language: LanguageCodeIR,
    update_language: LanguageCodeIR,
) -> Row {
    let mut api = CognitiveApi::new_embedded().expect("embedded core");
    let setup_response = run(&mut api, id, 1, setup, setup_language);
    let setup_value = value(&setup_response);
    let original = ledger_ids(&setup_value);
    let group_id = group_by_members(&setup_value, &original)
        .and_then(|group| group["group_id"].as_str())
        .unwrap_or_default()
        .to_string();
    let extra_response = run(&mut api, id, 2, extra, setup_language);
    let extra_value = value(&extra_response);
    let member = goal_for_subject(&extra_value, member_surface).unwrap_or_default();
    let expected = sorted(original.into_iter().chain([member]).collect());
    let updated = run(&mut api, id, 3, update, update_language);
    let update_value = value(&updated);
    let queried = run(&mut api, id, 4, query, setup_language);
    let query_value = value(&queried);
    Row {
        id: id.to_string(),
        category: "heldout_cross_language_revision".to_string(),
        pass: group_by_id(&update_value, &group_id)
            .is_some_and(|group| valid_group(group, &expected, 2, 0))
            && update_applied(&update_value, "ADD_MEMBER")
            && targets(&query_value) == expected
            && binding_size(&query_value, "PLURAL_EVENT_REFERENCE", 3)
            && safe(&updated)
            && safe(&queried),
        trace: vec![update_value.to_string(), query_value.to_string()],
    }
}

#[allow(clippy::too_many_arguments)]
fn delayed_action(
    id: &str,
    setup: &str,
    extra: &str,
    update: &str,
    query: &str,
    pauses: &[&str],
    member_surface: &str,
    language: LanguageCodeIR,
) -> Row {
    let mut api = CognitiveApi::new_embedded().expect("embedded core");
    let setup_response = run(&mut api, id, 1, setup, language);
    let setup_value = value(&setup_response);
    let original = ledger_ids(&setup_value);
    let group_id = group_by_members(&setup_value, &original)
        .and_then(|group| group["group_id"].as_str())
        .unwrap_or_default()
        .to_string();
    let extra_response = run(&mut api, id, 2, extra, language);
    let extra_value = value(&extra_response);
    let member = goal_for_subject(&extra_value, member_surface).unwrap_or_default();
    let expected = sorted(original.into_iter().chain([member]).collect());
    let mut turn = 3_u64;
    for pause in pauses {
        run(&mut api, id, turn, pause, language);
        turn += 1;
    }
    let updated = run(&mut api, id, turn, update, language);
    let update_value = value(&updated);
    turn += 1;
    for pause in pauses.iter().rev() {
        run(&mut api, id, turn, pause, language);
        turn += 1;
    }
    let queried = run(&mut api, id, turn, query, language);
    let query_value = value(&queried);
    Row {
        id: id.to_string(),
        category: "heldout_long_horizon_revision".to_string(),
        pass: group_by_id(&update_value, &group_id)
            .is_some_and(|group| valid_group(group, &expected, 2, 0))
            && update_applied(&update_value, "ADD_MEMBER")
            && targets(&query_value) == expected
            && binding_size(&query_value, "PLURAL_EVENT_REFERENCE", 3)
            && safe(&updated)
            && safe(&queried),
        trace: vec![update_value.to_string(), query_value.to_string()],
    }
}

fn quoted_non_authority(id: &str, setup: &[&str], quoted: &str, language: LanguageCodeIR) -> Row {
    let mut api = CognitiveApi::new_embedded().expect("embedded core");
    let mut before = Value::Null;
    for (index, text) in setup.iter().enumerate() {
        before = value(&run(
            &mut api,
            id,
            u64::try_from(index + 1).expect("bounded turn"),
            text,
            language,
        ));
    }
    let group_snapshot = groups(&before)
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
    let ledger_snapshot = ledger_ids(&before);
    let response = run(
        &mut api,
        id,
        u64::try_from(setup.len() + 1).expect("bounded turn"),
        quoted,
        language,
    );
    let observed = value(&response);
    let after = groups(&observed)
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
        category: "heldout_quoted_revision_non_authority".to_string(),
        pass: observed["discourse_group_update"].is_null()
            && group_snapshot == after
            && ledger_snapshot == ledger_ids(&observed)
            && response.disposition != ConversationTurnDispositionIR::ClarificationRequired
            && safe(&response),
        trace: vec![observed.to_string()],
    }
}

fn main() {
    const EN_PAUSES: &[&str] = &["okay", "thanks", "hmm", "wait", "yep"];
    const KO_PAUSES: &[&str] = &["응", "고마워", "음", "잠깐", "알겠어"];
    let mut rows = vec![
        reversible(
            "R38X_REV_EN_1",
            "inspect cache and repair queue",
            "analyze worker",
            "Put the worker task into that task group.",
            "Take the worker task back out of that task group.",
            "How is that task group doing?",
            "worker",
            LanguageCodeIR::English,
        ),
        reversible(
            "R38X_REV_EN_2",
            "repair server and inspect log",
            "analyze backup",
            "Bring the backup action into the current task group.",
            "Drop the backup action from the current task group.",
            "Give me the current task group's status.",
            "backup",
            LanguageCodeIR::English,
        ),
        reversible(
            "R38X_REV_KO_1",
            "캐시를 확인하고 큐를 수리해",
            "워커를 분석해",
            "그 작업 묶음에 워커도 넣어",
            "그 작업 묶음에서 워커를 다시 빼",
            "그 작업 묶음은 어디까지 됐어?",
            "워커",
            LanguageCodeIR::Korean,
        ),
        reversible(
            "R38X_REV_KO_2",
            "서버를 수리하고 로그를 확인해",
            "백업을 분석해",
            "현재 작업 묶음에 백업도 넣어줘",
            "현재 작업 묶음에서 백업은 다시 제외해",
            "현재 작업 묶음의 진척을 알려줘",
            "백업",
            LanguageCodeIR::Korean,
        ),
        overlapping_speaker_merge(
            "R38X_DEDUP_EN_1",
            [
                "Alice says the cache is stale.",
                "Bob says the queue is empty.",
                "Carol says the worker is ready.",
            ],
            "What did Alice and Bob say?",
            "What did Bob and Carol say?",
            "Combine the first and second speaker groups.",
            "What did the combined speaker group say?",
            &["alice", "bob", "carol"],
            LanguageCodeIR::English,
        ),
        overlapping_speaker_merge(
            "R38X_DEDUP_EN_2",
            [
                "Mina says the server is slow.",
                "Jin says the log is clean.",
                "Nora says the backup exists.",
            ],
            "What did Mina and Jin say?",
            "What did Jin and Nora say?",
            "Merge the earlier and later speaker groups.",
            "Summarize the merged speaker group.",
            &["mina", "jin", "nora"],
            LanguageCodeIR::English,
        ),
        overlapping_speaker_merge(
            "R38X_DEDUP_KO_1",
            [
                "민수는 캐시가 오래됐다고 말했다.",
                "지수는 큐가 비었다고 말했다.",
                "철수는 워커가 준비됐다고 말했다.",
            ],
            "민수와 지수는 뭐라고 말했어?",
            "지수와 철수는 뭐라고 말했어?",
            "첫 번째와 두 번째 화자 묶음을 합쳐",
            "합친 화자 묶음은 뭐라고 말했어?",
            &["민수", "지수", "철수"],
            LanguageCodeIR::Korean,
        ),
        overlapping_speaker_merge(
            "R38X_DEDUP_KO_2",
            [
                "영희는 서버가 느리다고 말했다.",
                "수진은 로그가 깨끗하다고 말했다.",
                "동수는 백업이 있다고 말했다.",
            ],
            "영희와 수진은 뭐라고 말했어?",
            "수진과 동수는 뭐라고 말했어?",
            "앞과 뒤 화자 묶음을 병합해",
            "병합한 화자 묶음이 한 말을 요약해",
            &["영희", "수진", "동수"],
            LanguageCodeIR::Korean,
        ),
        cross_language_action(
            "R38X_CROSS_EN_KO_1",
            "inspect cache and repair queue",
            "analyze worker",
            "그 작업 묶음에 worker 작업을 추가해",
            "Show the status of that task group.",
            "worker",
            LanguageCodeIR::English,
            LanguageCodeIR::Korean,
        ),
        cross_language_action(
            "R38X_CROSS_EN_KO_2",
            "repair server and inspect log",
            "analyze backup",
            "현재 작업 묶음에 backup을 포함해",
            "List the current task group's state.",
            "backup",
            LanguageCodeIR::English,
            LanguageCodeIR::Korean,
        ),
        cross_language_action(
            "R38X_CROSS_KO_EN_1",
            "캐시를 확인하고 큐를 수리해",
            "워커를 분석해",
            "Add the 워커 task to that task group.",
            "그 작업 묶음의 현황을 알려줘",
            "워커",
            LanguageCodeIR::Korean,
            LanguageCodeIR::English,
        ),
        cross_language_action(
            "R38X_CROSS_KO_EN_2",
            "서버를 수리하고 로그를 확인해",
            "백업을 분석해",
            "Include the 백업 action in the current task group.",
            "현재 작업 묶음의 상태를 보여줘",
            "백업",
            LanguageCodeIR::Korean,
            LanguageCodeIR::English,
        ),
        delayed_action(
            "R38X_DELAY_EN_1",
            "inspect cache and repair queue",
            "analyze worker",
            "Add worker to that earlier task group.",
            "How is that task group doing?",
            EN_PAUSES,
            "worker",
            LanguageCodeIR::English,
        ),
        delayed_action(
            "R38X_DELAY_EN_2",
            "repair server and inspect log",
            "analyze backup",
            "Include backup in the current task group.",
            "Catch me up on that task group.",
            EN_PAUSES,
            "backup",
            LanguageCodeIR::English,
        ),
        delayed_action(
            "R38X_DELAY_KO_1",
            "캐시를 확인하고 큐를 수리해",
            "워커를 분석해",
            "아까 그 작업 묶음에 워커를 추가해",
            "그 작업 묶음은 어디까지 됐어?",
            KO_PAUSES,
            "워커",
            LanguageCodeIR::Korean,
        ),
        delayed_action(
            "R38X_DELAY_KO_2",
            "서버를 수리하고 로그를 확인해",
            "백업을 분석해",
            "현재 작업 묶음에 백업을 포함해",
            "그 작업 묶음의 진척을 알려줘",
            KO_PAUSES,
            "백업",
            LanguageCodeIR::Korean,
        ),
        quoted_non_authority(
            "R38X_QUOTE_EN_1",
            &["inspect cache and repair queue", "analyze worker"],
            "The sentence ‘add worker to that task group’ is only an example.",
            LanguageCodeIR::English,
        ),
        quoted_non_authority(
            "R38X_QUOTE_EN_2",
            &[
                "Alice says the cache is stale.",
                "Bob says the queue is empty.",
                "What did Alice and Bob say?",
            ],
            "Do not act on the quote ‘remove Bob from that speaker group’. Explain its grammar.",
            LanguageCodeIR::English,
        ),
        quoted_non_authority(
            "R38X_QUOTE_KO_1",
            &["캐시를 확인하고 큐를 수리해", "워커를 분석해"],
            "‘그 작업 묶음에 워커를 추가해’라는 문장을 예로 든 것뿐이야",
            LanguageCodeIR::Korean,
        ),
        quoted_non_authority(
            "R38X_QUOTE_KO_2",
            &[
                "민수는 캐시가 오래됐다고 말했다.",
                "지수는 큐가 비었다고 말했다.",
                "민수와 지수는 뭐라고 말했어?",
            ],
            "‘그 화자 묶음에서 지수를 빼’라는 인용문의 문법만 설명해",
            LanguageCodeIR::Korean,
        ),
    ];
    rows.sort_by(|left, right| left.id.cmp(&right.id));
    let passed = rows.iter().filter(|row| row.pass).count();
    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "suite": "R38-TRANSFER-0001",
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
