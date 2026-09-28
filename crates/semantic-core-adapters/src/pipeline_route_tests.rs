use super::*;

#[test]
fn question_followups_keep_the_requested_information_and_user_choice() {
    use crate::world_dialogue::WorldClarificationFollowupKindIR as K;
    for (language, question, reply, kind) in [
        (
            LanguageCodeIR::Korean,
            "서윤이 쉬는 게 좋겠네?",
            "잘 모르겠어.",
            K::UnknownAnswer,
        ),
        (
            LanguageCodeIR::English,
            "Should I rest?",
            "I don't know.",
            K::UnknownAnswer,
        ),
        (
            LanguageCodeIR::Korean,
            "서윤이 쉬는 게 좋겠네?",
            "말하고 싶지 않아.",
            K::DeclinedAnswer,
        ),
        (
            LanguageCodeIR::English,
            "Should I rest?",
            "I'd rather not say.",
            K::DeclinedAnswer,
        ),
        (
            LanguageCodeIR::Korean,
            "서윤이 쉬는 게 좋겠네?",
            "무슨 뜻이야?",
            K::Restatement,
        ),
        (
            LanguageCodeIR::English,
            "Should I rest?",
            "What do you mean?",
            K::Restatement,
        ),
        (
            LanguageCodeIR::Korean,
            "서윤이 쉬는 게 좋겠네?",
            "왜 물었어?",
            K::ReasonRequest,
        ),
        (
            LanguageCodeIR::English,
            "Should I rest?",
            "Why did you ask?",
            K::ReasonRequest,
        ),
    ] {
        let mut api = CognitiveApi::new_embedded().unwrap();
        api.process_conversation_turn(&request("QUESTION-REPLY", 1, question, language))
            .unwrap();
        let followups = if matches!(kind, K::UnknownAnswer | K::DeclinedAnswer) {
            vec![reply, "Why?", "Why did you ask?"]
        } else {
            vec![reply, "Yes."]
        };
        for (index, text) in followups.into_iter().enumerate() {
            let q = request("QUESTION-REPLY", index as u64 + 2, text, language);
            crate::generative_language::GENERATION_INVOCATIONS.with(|c| c.set(0));
            crate::natural_realization::FINAL_REALIZATION_CHECKS.with(|c| c.set(0));
            let response = api
                .process_conversation_turn(&q)
                .unwrap_or_else(|e| panic!("{question}/{reply}/{text}: {e:?}"));
            assert_eq!(
                crate::generative_language::GENERATION_INVOCATIONS.with(|c| c.get()),
                1
            );
            assert_eq!(
                crate::natural_realization::FINAL_REALIZATION_CHECKS.with(|c| c.get()),
                1
            );
            let inquiry = response
                .discourse_answer
                .as_ref()
                .and_then(|a| a.decision_inquiry.as_ref())
                .unwrap_or_else(|| panic!("{reply}/{text}: {}", response.output.text));
            assert!(response.validate_against(&q), "{}", response.output.text);
            assert!(response
                .conversation_state
                .action_state_ledger
                .records
                .is_empty());
            if index == 0 || matches!(kind, K::UnknownAnswer | K::DeclinedAnswer) {
                assert!(inquiry.clarification_reply.is_some());
                let mut tampered = inquiry.clone();
                tampered.clarification_reply.as_mut().unwrap().question_turn = 0;
                assert!(!tampered.validate());
                let mut tampered = inquiry.clone();
                let receipt = tampered.clarification_reply.as_mut().unwrap();
                receipt.explains_question = !receipt.explains_question;
                assert!(!tampered.validate());
                if inquiry.explanation_of.is_some() {
                    let mut tampered = inquiry.clone();
                    tampered.explanation_of = None;
                    assert!(!tampered.validate());
                }
                assert!(response
                    .conversation_state
                    .dialogue_world
                    .premises
                    .is_empty());
                assert!(inquiry.assessment.is_none());
            } else {
                assert!(inquiry.resumption.is_some() && inquiry.assessment.is_some());
            }
            if matches!(kind, K::UnknownAnswer | K::DeclinedAnswer) {
                assert!(response
                    .conversation_state
                    .dialogue_world
                    .last_query
                    .is_none());
                assert!(!response.output.text.ends_with('?'));
                assert_eq!(
                    inquiry
                        .clarification_reply
                        .as_ref()
                        .unwrap()
                        .explains_question,
                    index == 2
                );
            }
        }
    }
}

#[test]
fn nonfactual_replies_cannot_create_states_or_renew_closed_questions() {
    for text in [
        "왜 물었지만",
        "왜 민수는 물었어?",
        "I don't know why",
        "He won't answer",
        "민수는 말하고 싶지 않아",
        "무슨 뜻이야? 취소해",
    ] {
        assert!(
            crate::world_dialogue::clarification_followup(text).is_none(),
            "{text}"
        );
    }
    for turns in [
        vec!["Should I rest?", "I don't know.", "Yes."],
        vec!["Should I rest?", "I'd rather not say.", "No."],
        vec!["Should I rest?", "Never mind.", "What do you mean?"],
        vec!["Should I rest?", "What is entropy?", "Why did you ask?"],
        vec![
            "Should I rest?",
            "I don't know.",
            "Why?",
            "Why?",
            "Why did you ask?",
        ],
    ] {
        let mut api = CognitiveApi::new_embedded().unwrap();
        for (index, text) in turns.iter().enumerate() {
            let q = request(
                "REPLY-CLOSED",
                index as u64 + 1,
                text,
                LanguageCodeIR::English,
            );
            let r = api.process_conversation_turn(&q).unwrap();
            assert!(r.validate_against(&q));
            assert!(r.conversation_state.dialogue_world.premises.is_empty());
            assert!(r.conversation_state.action_state_ledger.records.is_empty());
            if index + 1 == turns.len() {
                assert!(
                    r.discourse_answer
                        .as_ref()
                        .and_then(|a| a.decision_inquiry.as_ref())
                        .is_none(),
                    "{turns:?}: {}",
                    r.output.text
                );
            }
        }
    }
}

#[test]
fn a_missing_state_is_asked_as_a_grounded_polar_question() {
    for (language, question, reply, value) in [
        (LanguageCodeIR::English, "Should I rest?", "Yes.", true),
        (LanguageCodeIR::English, "Should I rest?", "No.", false),
        (LanguageCodeIR::Korean, "지우가 쉬는 게 좋겠네?", "응", true),
        (
            LanguageCodeIR::Korean,
            "지우가 쉬는 게 좋겠네?",
            "그래",
            true,
        ),
        (
            LanguageCodeIR::Korean,
            "지우가 쉬는 게 좋겠네?",
            "맞아요",
            true,
        ),
        (
            LanguageCodeIR::Korean,
            "지우가 쉬는 게 좋겠네?",
            "그렇습니다",
            true,
        ),
        (LanguageCodeIR::English, "Should I rest?", "Yep!", true),
        (LanguageCodeIR::English, "Should I rest?", "아니요", false),
        (
            LanguageCodeIR::Korean,
            "지우가 쉬는 게 좋겠네?",
            "아니",
            false,
        ),
    ] {
        let mut api = CognitiveApi::new_embedded().unwrap();
        let q = request("POLAR-GAP", 1, question, language);
        let first = api.process_conversation_turn(&q).unwrap();
        assert!(
            first
                .output
                .text
                .contains(if language == LanguageCodeIR::Korean {
                    "피곤"
                } else {
                    "tired"
                }),
            "{}",
            first.output.text
        );
        assert!(first.output.text.ends_with('?'));
        assert!(first.conversation_state.dialogue_world.premises.is_empty());
        assert!(first.conversation_state.dialogue_world.last_query.is_some());
        assert!(first.validate_against(&q));
        for (i, text) in [reply, "Why do you think that?"].into_iter().enumerate() {
            let q = request("POLAR-GAP", i as u64 + 2, text, language);
            crate::generative_language::GENERATION_INVOCATIONS.with(|c| c.set(0));
            crate::natural_realization::FINAL_REALIZATION_CHECKS.with(|c| c.set(0));
            let r = api
                .process_conversation_turn(&q)
                .unwrap_or_else(|e| panic!("{question}/{text}: {e:?}"));
            assert_eq!(
                crate::generative_language::GENERATION_INVOCATIONS.with(|c| c.get()),
                1
            );
            assert_eq!(
                crate::natural_realization::FINAL_REALIZATION_CHECKS.with(|c| c.get()),
                1
            );
            let inquiry = r
                .discourse_answer
                .as_ref()
                .and_then(|a| a.decision_inquiry.as_ref())
                .unwrap_or_else(|| panic!("{question}/{text}: {}", r.output.text));
            assert!(inquiry.resumption.is_some());
            assert_eq!(inquiry.assessment.is_some(), value);
            let premise = r.conversation_state.dialogue_world.premises.last().unwrap();
            assert_eq!(premise.source_text, reply);
            assert_eq!(premise.value, value);
            assert!(premise.answer_binding.is_some());
            assert!(r.conversation_state.action_state_ledger.records.is_empty());
            assert!(r.validate_against(&q));
        }
    }
}

#[test]
fn polar_replies_require_a_live_question_not_acknowledgment_or_permission() {
    for reply in [
        "그래?",
        "그래서",
        "맞겠지",
        "알겠어",
        "좋아",
        "okay",
        "네 아니요",
    ] {
        assert_eq!(
            crate::conversation::confirmation_polarity(reply),
            None,
            "{reply}"
        );
    }
    for turns in [
        vec!["그래"],
        vec!["Should I rest?", "Never mind.", "그래"],
        vec!["Should I rest?", "What is entropy?", "맞아"],
        vec!["Should I rest?", "Thanks.", "Thanks.", "Thanks.", "그래"],
        vec!["Should I rest?", "그래?"],
        vec!["Should I rest?", "알겠어"],
        vec!["Should I rest?", "네 아니요"],
        vec!["Proceed?", "그래"],
        vec!["Should I rest?", "Yes.", "No."],
    ] {
        let mut api = CognitiveApi::new_embedded().unwrap();
        let mut before = Vec::new();
        for (i, text) in turns.iter().enumerate() {
            let q = request(
                "POLAR-BOUNDARY",
                i as u64 + 1,
                text,
                LanguageCodeIR::English,
            );
            let r = api.process_conversation_turn(&q).unwrap();
            assert!(r.validate_against(&q));
            if i + 1 == turns.len() {
                assert_eq!(
                    r.conversation_state.dialogue_world.premises, before,
                    "{turns:?}"
                );
                assert!(
                    r.discourse_answer
                        .as_ref()
                        .and_then(|a| a.decision_inquiry.as_ref())
                        .and_then(|d| d.resumption.as_ref())
                        .is_none(),
                    "{turns:?}"
                );
            }
            before = r.conversation_state.dialogue_world.premises.clone();
            assert!(r.conversation_state.action_state_ledger.records.is_empty());
        }
    }
}

#[test]
fn a_relevant_clarification_resumes_the_pending_semantic_question() {
    for (language, question, clarification, succeeds) in [
        (
            LanguageCodeIR::English,
            "Should I rest?",
            "I am tired.",
            true,
        ),
        (
            LanguageCodeIR::Korean,
            "지우가 쉬는 게 좋겠네?",
            "지우는 피곤해.",
            true,
        ),
        (
            LanguageCodeIR::English,
            "Should I rest?",
            "나는 피곤하지 않아.",
            false,
        ),
    ] {
        let mut api = CognitiveApi::new_embedded().unwrap();
        api.process_conversation_turn(&request("RESUME", 1, question, language))
            .unwrap();
        let mut retained = None;
        for (i, text) in [clarification, "Why do you think that?"]
            .into_iter()
            .enumerate()
        {
            let q = request("RESUME", i as u64 + 2, text, language);
            crate::generative_language::GENERATION_INVOCATIONS.with(|c| c.set(0));
            crate::natural_realization::FINAL_REALIZATION_CHECKS.with(|c| c.set(0));
            let r = api
                .process_conversation_turn(&q)
                .unwrap_or_else(|e| panic!("{question}/{text}: {e:?}"));
            assert_eq!(
                crate::generative_language::GENERATION_INVOCATIONS.with(|c| c.get()),
                1
            );
            assert_eq!(
                crate::natural_realization::FINAL_REALIZATION_CHECKS.with(|c| c.get()),
                1
            );
            let inquiry = r
                .discourse_answer
                .as_ref()
                .and_then(|a| a.decision_inquiry.as_ref())
                .unwrap_or_else(|| panic!("{text}: {}", r.output.text));
            let resumption = inquiry.resumption.as_ref().unwrap();
            assert_eq!(resumption.original_source, question);
            assert_eq!(resumption.question_turn, 1);
            assert_eq!(resumption.update.turn, 2);
            assert_eq!(resumption.update.source_text, clarification);
            assert_eq!(inquiry.assessment.is_some(), succeeds);
            assert_eq!(inquiry.knowledge_gap.is_some(), !succeeds);
            assert!(r.validate_against(&q));
            assert!(
                r.grounded_response.is_none()
                    && r.conversation_state.action_state_ledger.records.is_empty()
            );
            assert!(r
                .conversation_state
                .dialogue_world
                .premises
                .iter()
                .any(|p| p.source_text == clarification && p.introduced_turn == 2));
            if let Some(prior) = &retained {
                assert_eq!(resumption, prior);
            }
            retained = Some(resumption.clone());
            let mut forged = inquiry.clone();
            forged.resumption = None;
            assert!(!forged.validate());
            let mut forged = inquiry.clone();
            let receipt = forged.resumption.as_mut().unwrap();
            receipt.question_turn = 0;
            receipt.prior_gap.evaluated_turn = 0;
            assert!(!forged.validate());
            let mut forged = inquiry.clone();
            forged.resumption.as_mut().unwrap().update.source_text = "I am ready.".into();
            assert!(!forged.validate());
        }
    }
}

#[test]
fn unrelated_topics_cancellation_and_missing_evidence_do_not_resume_a_question() {
    for turns in [
        vec!["Should I rest?", "지우는 피곤해."],
        vec!["Should I rest?", "I am ready."],
        vec!["Should I rest?", "Never mind.", "I am tired."],
        vec!["Should I rest?", "What is entropy?", "I am tired."],
        vec!["Should I rest?", "\"I am tired.\""],
        vec!["Should I sleep?", "I am tired."],
    ] {
        let mut api = CognitiveApi::new_embedded().unwrap();
        for (i, text) in turns.iter().enumerate() {
            let q = request(
                "RESUME-BOUNDARY",
                i as u64 + 1,
                text,
                LanguageCodeIR::English,
            );
            let r = api.process_conversation_turn(&q).unwrap();
            assert!(r.validate_against(&q));
            assert!(
                r.discourse_answer
                    .as_ref()
                    .and_then(|a| a.decision_inquiry.as_ref())
                    .and_then(|i| i.resumption.as_ref())
                    .is_none(),
                "{turns:?}: {}",
                r.output.text
            );
        }
    }
}

#[test]
fn resumed_and_reasked_questions_use_identical_core_evidence() {
    let mut results = Vec::new();
    for turns in [
        vec!["Should I rest?", "I am tired."],
        vec!["Hello.", "I am tired.", "Should I rest?"],
    ] {
        let mut api = CognitiveApi::new_embedded().unwrap();
        for (i, text) in turns.iter().enumerate() {
            let q = request(
                "RESUME-EQUIVALENCE",
                i as u64 + 1,
                text,
                LanguageCodeIR::English,
            );
            let r = api.process_conversation_turn(&q).unwrap();
            if i + 1 == turns.len() {
                let assessment = r
                    .discourse_answer
                    .as_ref()
                    .unwrap()
                    .decision_inquiry
                    .as_ref()
                    .unwrap()
                    .assessment
                    .as_ref()
                    .unwrap();
                results.push((assessment.request.clone(), assessment.derivation.clone()));
                assert!(r.validate_against(&q));
            }
        }
    }
    assert_eq!(results[0], results[1]);
}

#[test]
fn decision_gaps_retain_actual_failure_reason_instead_of_requesting_benefits() {
    use crate::world_dialogue::ActionBenefitGapReasonIR as G;
    for (observations, query, expected, asks) in [
        (vec![], "Should I rest?", G::CurrentState, true),
        (vec!["나는 피곤해."], "쉬는 게 좋겠네?", G::Actor, true),
        (
            vec!["I am tired."],
            "Should I sleep?",
            G::EffectKnowledge,
            false,
        ),
        (
            vec!["I am tired."],
            "Should I read the report?",
            G::ActionRoles,
            false,
        ),
        (
            vec!["I am tired."],
            "Should I not rest?",
            G::NegatedActionEffect,
            false,
        ),
        (
            vec!["나는 피곤하지 않아."],
            "Should I rest?",
            G::InapplicableState,
            false,
        ),
        (
            vec!["나는 피곤해.", "나는 피곤하지 않아."],
            "Should I rest?",
            G::ConflictingState,
            true,
        ),
    ] {
        for language in [LanguageCodeIR::Korean, LanguageCodeIR::English] {
            let mut api = CognitiveApi::new_embedded().unwrap();
            for (i, text) in observations.iter().enumerate() {
                api.process_conversation_turn(&request("GAPS", i as u64 + 1, text, language))
                    .unwrap();
            }
            let mut original = None;
            for (i, text) in [query, "Why do you think that?"].into_iter().enumerate() {
                let q = request(
                    "GAPS",
                    observations.len() as u64 + i as u64 + 1,
                    text,
                    language,
                );
                crate::generative_language::GENERATION_INVOCATIONS.with(|c| c.set(0));
                crate::natural_realization::FINAL_REALIZATION_CHECKS.with(|c| c.set(0));
                let r = api
                    .process_conversation_turn(&q)
                    .unwrap_or_else(|e| panic!("{query}/{text}/{language:?}: {e:?}"));
                assert_eq!(
                    crate::generative_language::GENERATION_INVOCATIONS.with(|c| c.get()),
                    1
                );
                assert_eq!(
                    crate::natural_realization::FINAL_REALIZATION_CHECKS.with(|c| c.get()),
                    1
                );
                let inquiry = r
                    .discourse_answer
                    .as_ref()
                    .unwrap()
                    .decision_inquiry
                    .as_ref()
                    .unwrap();
                let gap = inquiry.knowledge_gap.as_ref().unwrap();
                assert_eq!(gap.reason, expected, "{query}: {}", r.output.text);
                assert!(inquiry.assessment.is_none());
                assert!(
                    !r.output.text.contains("expected benefit")
                        && !r.output.text.contains("기대하는 이득")
                );
                assert_eq!(r.output.text.ends_with('?'), asks && i == 0);
                assert!(
                    r.conversation_state.action_state_ledger.records.is_empty()
                        && r.grounded_response.is_none()
                );
                assert!(r.validate_against(&q));
                if let Some(prior) = &original {
                    assert_eq!(gap, prior);
                }
                original = Some(gap.clone());
                let mut forged = inquiry.clone();
                forged.knowledge_gap.as_mut().unwrap().reason = G::InvalidContext;
                assert!(!forged.validate());
            }
        }
    }
}

#[test]
fn decision_gap_and_answer_change_only_when_their_evidence_changes() {
    use crate::world_dialogue::ActionBenefitGapReasonIR as G;
    let mut states = Vec::new();
    for formality in [0, 900] {
        let mut api = CognitiveApi::new_embedded().unwrap();
        api.set_dialogue_personality(crate::affective_field::DialoguePersonalityIR {
            formality_millis: formality,
            ..Default::default()
        })
        .unwrap();
        let mut gaps = Vec::new();
        for (i, text) in [
            "Should I rest?",
            "나는 피곤해.",
            "Should I rest?",
            "아니, 나는 피곤하지 않아.",
            "Should I rest?",
            "Why do you think that?",
        ]
        .into_iter()
        .enumerate()
        {
            let q = request("GAP-REVISION", i as u64 + 1, text, LanguageCodeIR::Korean);
            let r = api.process_conversation_turn(&q).unwrap();
            assert!(r.validate_against(&q));
            let inquiry = r
                .discourse_answer
                .as_ref()
                .and_then(|a| a.decision_inquiry.as_ref());
            if i == 2 {
                let inquiry = inquiry.unwrap();
                assert!(inquiry.assessment.is_some() && inquiry.knowledge_gap.is_none());
            }
            if matches!(i, 0 | 4 | 5) {
                let inquiry = inquiry.unwrap();
                let gap = inquiry.knowledge_gap.as_ref().unwrap();
                assert_eq!(
                    gap.reason,
                    if i == 0 {
                        G::CurrentState
                    } else {
                        G::InapplicableState
                    }
                );
                assert!(inquiry.assessment.is_none());
                gaps.push(gap.clone());
            }
            assert!(r.conversation_state.action_state_ledger.records.is_empty());
        }
        assert_eq!(gaps[1], gaps[2]);
        states.push(gaps);
    }
    assert_eq!(states[0], states[1]);
}

#[test]
fn deliberative_modal_preserves_roles_without_asserting_or_executing() {
    use crate::proposition_content::ContentSlotIR as R;
    for action in ["rest", "read the report", "print the document", "not rest"] {
        let source = format!("Should I {action}?");
        let inquiry = crate::utterance_intent::decision_inquiry(&source).unwrap();
        assert!(inquiry.validate());
        let proposed = inquiry.proposed_action.as_ref().unwrap();
        let evaluative =
            crate::utterance_intent::decision_inquiry(&format!("Would it be good to {action}?"))
                .unwrap()
                .proposed_action
                .unwrap();
        assert_eq!(proposed.predicate_entry_ids, evaluative.predicate_entry_ids);
        assert_eq!(proposed.negated, evaluative.negated);
        let mut expected_roles = evaluative.event.roles;
        expected_roles.insert(R::Agent, "I".into());
        assert_eq!(proposed.event.roles, expected_roles);
        assert!(inquiry.assessment.is_none());
        let mut forged = inquiry.clone();
        forged
            .proposed_action
            .as_mut()
            .unwrap()
            .event
            .roles
            .insert(R::Agent, "someone else".into());
        assert!(!forged.validate());
    }
    for source in [
        "Did I rest?",
        "I should rest.",
        "Should I have rested?",
        "Should I resting?",
        "Should I read and print the report?",
        "Should I rest if it rains?",
        "Should I rest because I am tired?",
        "\"Should I rest?\"",
        "Should you rest?",
        "Rest now.",
        "Should I 쉬는?",
        "Should I rest",
    ] {
        assert!(
            crate::utterance_intent::decision_inquiry(source).is_none(),
            "{source}"
        );
    }
}

#[test]
fn deliberative_modal_reaches_the_same_core_without_changing_world_facts() {
    let mut receipts = Vec::new();
    for query in [
        "Then would it be good to rest?",
        "Should I rest?",
        "Then, SHOULD I REST?",
    ] {
        let mut api = CognitiveApi::new_embedded().unwrap();
        let previous = api
            .process_conversation_turn(&request("MODAL", 1, "I am tired.", LanguageCodeIR::English))
            .unwrap();
        for (i, text) in [query, "Why do you think that?"].into_iter().enumerate() {
            let q = request("MODAL", i as u64 + 2, text, LanguageCodeIR::English);
            crate::generative_language::GENERATION_INVOCATIONS.with(|c| c.set(0));
            crate::natural_realization::FINAL_REALIZATION_CHECKS.with(|c| c.set(0));
            let r = api.process_conversation_turn(&q).unwrap();
            assert_eq!(
                crate::generative_language::GENERATION_INVOCATIONS.with(|c| c.get()),
                1
            );
            assert_eq!(
                crate::natural_realization::FINAL_REALIZATION_CHECKS.with(|c| c.get()),
                1
            );
            let a = r
                .discourse_answer
                .as_ref()
                .unwrap()
                .decision_inquiry
                .as_ref()
                .and_then(|a| a.assessment.as_ref())
                .unwrap_or_else(|| panic!("{text}: {}", r.output.text));
            receipts.push((a.request.clone(), a.derivation.clone()));
            assert!(r.output.text.contains("may help"));
            if i == 1 {
                assert!(r.output.text.contains("purpose"));
            }
            assert_eq!(
                r.conversation_state.dialogue_world.premises,
                previous.conversation_state.dialogue_world.premises
            );
            assert!(r.conversation_state.action_state_ledger.records.is_empty());
            assert!(r.grounded_response.is_none());
            assert!(r.validate_against(&q));
        }
    }
    assert!(receipts.windows(2).all(|p| p[0] == p[1]));
}

#[test]
fn synonymous_proposals_share_a_core_derivation_not_copied_semantics() {
    let mut receipts = Vec::new();
    for (language, query, why) in [
        (
            LanguageCodeIR::Korean,
            "그럼 쉬는 게 좋겠네?",
            "왜 그렇게 생각해?",
        ),
        (
            LanguageCodeIR::Korean,
            "그럼 휴식하는 게 좋겠네?",
            "왜 그렇게 생각해?",
        ),
        (
            LanguageCodeIR::English,
            "Then would it be good to rest?",
            "Why do you think that?",
        ),
    ] {
        let mut api = CognitiveApi::new_embedded().unwrap();
        api.process_conversation_turn(&request("SYNONYM-BENEFIT", 1, "나는 피곤해.", language))
            .unwrap();
        for (i, text) in [query, why].into_iter().enumerate() {
            let q = request("SYNONYM-BENEFIT", i as u64 + 2, text, language);
            crate::generative_language::GENERATION_INVOCATIONS.with(|c| c.set(0));
            crate::natural_realization::FINAL_REALIZATION_CHECKS.with(|c| c.set(0));
            let r = api.process_conversation_turn(&q).unwrap();
            assert_eq!(
                crate::generative_language::GENERATION_INVOCATIONS.with(|c| c.get()),
                1
            );
            assert_eq!(
                crate::natural_realization::FINAL_REALIZATION_CHECKS.with(|c| c.get()),
                1
            );
            let a = r
                .discourse_answer
                .as_ref()
                .unwrap()
                .decision_inquiry
                .as_ref()
                .and_then(|i| i.assessment.as_ref())
                .unwrap_or_else(|| panic!("{text}: {}", r.output.text));
            receipts.push((a.request.clone(), a.derivation.clone()));
            assert!(
                r.grounded_response.is_none()
                    && r.conversation_state.action_state_ledger.records.is_empty()
            );
            assert!(r.validate_against(&q));
        }
    }
    assert!(receipts.windows(2).all(|p| p[0] == p[1]));
}

#[test]
fn grounded_possible_benefit_answers_instead_of_reasking_for_known_input() {
    for (lang, turns) in [
        (
            LanguageCodeIR::Korean,
            [
                "지우는 피곤해.",
                "지우가 쉬는 게 좋겠네?",
                "왜 그렇게 생각해?",
            ],
        ),
        (
            LanguageCodeIR::Korean,
            ["나는 피곤해.", "그럼 쉬는 게 좋겠네?", "왜?"],
        ),
        (
            LanguageCodeIR::English,
            [
                "I am tired.",
                "Then would it be good to rest?",
                "Why do you think that?",
            ],
        ),
    ] {
        let mut api = CognitiveApi::new_embedded().unwrap();
        let mut prior_state = None;
        for (i, text) in turns.into_iter().enumerate() {
            let q = request("BENEFIT", i as u64 + 1, text, lang);
            crate::generative_language::GENERATION_INVOCATIONS.with(|c| c.set(0));
            crate::natural_realization::FINAL_REALIZATION_CHECKS.with(|c| c.set(0));
            let r = api.process_conversation_turn(&q).unwrap();
            assert_eq!(
                crate::generative_language::GENERATION_INVOCATIONS.with(|c| c.get()),
                1
            );
            assert_eq!(
                crate::natural_realization::FINAL_REALIZATION_CHECKS.with(|c| c.get()),
                1
            );
            if i > 0 {
                let answer = r.discourse_answer.as_ref().unwrap();
                let assessment = answer
                    .decision_inquiry
                    .as_ref()
                    .and_then(|x| x.assessment.as_ref())
                    .unwrap_or_else(|| panic!("{text}: {}", r.output.text));
                assert_eq!(
                    assessment.derivation.disposition,
                    dockable_semantic_core::DeliberationDispositionIR::GoalReachable
                );
                assert!(assessment
                    .derivation
                    .selected_plan
                    .as_ref()
                    .unwrap()
                    .mechanism_ids
                    .contains(&"ABP_0001".into()));
                assert_eq!(assessment.derivation.external_action_execution_events, 0);
                assert!(
                    r.output.text.contains(if lang == LanguageCodeIR::Korean {
                        "도움이 될 수"
                    } else {
                        "may help"
                    }),
                    "{}",
                    r.output.text
                );
                assert!(
                    !r.output.text.contains("알려줄래")
                        && !r.output.text.contains("could you tell")
                );
                if i == 2 {
                    assert!(
                        r.output.text.contains(if lang == LanguageCodeIR::Korean {
                            "위한 행동"
                        } else {
                            "purpose"
                        }),
                        "{}",
                        r.output.text
                    );
                }
                assert!(!answer.dialogue_truth_established && answer.claims.is_empty());
                let prev: &ConversationStateIR = prior_state.as_ref().unwrap();
                assert_eq!(
                    prev.epistemic_ledger.records,
                    r.conversation_state.epistemic_ledger.records
                );
                assert_eq!(
                    prev.dialogue_world.premises,
                    r.conversation_state.dialogue_world.premises
                );
                let mut ablated = assessment.request.clone();
                ablated.mechanisms.clear();
                assert_ne!(
                    dockable_semantic_core::DeliberationEngine
                        .deliberate(&ablated)
                        .unwrap()
                        .disposition,
                    dockable_semantic_core::DeliberationDispositionIR::GoalReachable
                );
                let mut forged = answer.decision_inquiry.clone().unwrap();
                forged
                    .assessment
                    .as_mut()
                    .unwrap()
                    .request
                    .mechanisms
                    .clear();
                assert!(!forged.validate());
            }
            assert!(r.grounded_response.is_none());
            assert!(r.conversation_state.action_state_ledger.records.is_empty());
            assert!(r.validate_against(&q));
            prior_state = Some(r.conversation_state);
        }
    }
}

#[test]
fn possible_benefit_requires_matching_actor_polarity_and_nonconflicting_premise() {
    for (observations, query) in [
        (vec!["지우는 피곤하지 않아."], "지우가 쉬는 게 좋겠네?"),
        (
            vec!["지우는 피곤해.", "지우는 피곤하지 않아."],
            "지우가 쉬는 게 좋겠네?",
        ),
        (vec!["지우는 피곤해."], "민수가 쉬는 게 좋겠네?"),
        (vec!["지우는 피곤해."], "지우가 안 쉬는 게 좋겠네?"),
        (vec!["지우는 피곤해."], "지우가 회사를 쉬는 게 좋겠네?"),
        (vec!["지우는 피곤해."], "그럼 쉬는 게 좋겠네?"),
    ] {
        let mut api = CognitiveApi::new_embedded().unwrap();
        let turn = observations.len() as u64 + 1;
        for (i, text) in observations.into_iter().enumerate() {
            api.process_conversation_turn(&request(
                "BENEFIT-BOUNDARY",
                i as u64 + 1,
                text,
                LanguageCodeIR::Korean,
            ))
            .unwrap();
        }
        let q = request("BENEFIT-BOUNDARY", turn, query, LanguageCodeIR::Korean);
        let r = api.process_conversation_turn(&q).unwrap();
        assert!(
            r.discourse_answer
                .as_ref()
                .and_then(|a| a.decision_inquiry.as_ref())
                .and_then(|a| a.assessment.as_ref())
                .is_none(),
            "{query}: {}",
            r.output.text
        );
        assert!(r.validate_against(&q));
    }
}

#[test]
fn benefit_register_does_not_change_core_judgement_and_correction_retires_it() {
    let mut results = Vec::new();
    for formal in [0, 900] {
        let mut api = CognitiveApi::new_embedded().unwrap();
        api.set_dialogue_personality(crate::affective_field::DialoguePersonalityIR {
            formality_millis: formal,
            ..Default::default()
        })
        .unwrap();
        for (i, text) in [
            "나는 피곤해.",
            "그럼 쉬는 게 좋겠네?",
            "아니, 나는 피곤하지 않아.",
            "왜 그렇게 생각해?",
        ]
        .into_iter()
        .enumerate()
        {
            let q = request(
                "BENEFIT-CORRECTION",
                i as u64 + 1,
                text,
                LanguageCodeIR::Korean,
            );
            let r = api.process_conversation_turn(&q).unwrap();
            assert!(r.validate_against(&q));
            if i == 1 {
                let a = r
                    .discourse_answer
                    .as_ref()
                    .unwrap()
                    .decision_inquiry
                    .as_ref()
                    .unwrap()
                    .assessment
                    .clone()
                    .unwrap();
                results.push((r.output.text.clone(), a));
            }
            if i == 3 {
                assert!(r
                    .discourse_answer
                    .as_ref()
                    .and_then(|a| a.decision_inquiry.as_ref())
                    .and_then(|i| i.assessment.as_ref())
                    .is_none());
            }
        }
    }
    assert_ne!(results[0].0, results[1].0);
    assert_eq!(results[0].1, results[1].1);
}

#[test]
fn proposal_roles_survive_question_and_reason_without_becoming_observations() {
    use crate::proposition_content::ContentSlotIR as R;
    for (language, question, reason, object, place, negative) in [
        (
            LanguageCodeIR::Korean,
            "지우가 도서관에서 기록을 읽는 게 좋겠네?",
            "왜 그렇게 생각해?",
            "기록",
            "도서관",
            false,
        ),
        (
            LanguageCodeIR::Korean,
            "지우가 도서관에서 기록을 안 읽는 게 좋겠네?",
            "왜 그렇게 생각해?",
            "기록",
            "도서관",
            true,
        ),
        (
            LanguageCodeIR::English,
            "Then Would it be good to read DeltaLedger in WestRoom?",
            "Why do you ask?",
            "DeltaLedger",
            "WestRoom",
            false,
        ),
        (
            LanguageCodeIR::English,
            "Would it be good to Not read ZetaNotes in EastHall?",
            "Why do you ask?",
            "ZetaNotes",
            "EastHall",
            true,
        ),
    ] {
        let mut api = CognitiveApi::new_embedded().unwrap();
        let mut previous_action = None;
        for (i, text) in [question, reason].into_iter().enumerate() {
            let q = request("PROPOSAL-ROLES", i as u64 + 1, text, language);
            crate::generative_language::GENERATION_INVOCATIONS.with(|c| c.set(0));
            crate::natural_realization::FINAL_REALIZATION_CHECKS.with(|c| c.set(0));
            let r = api.process_conversation_turn(&q).unwrap();
            assert_eq!(
                crate::generative_language::GENERATION_INVOCATIONS.with(|c| c.get()),
                1
            );
            assert_eq!(
                crate::natural_realization::FINAL_REALIZATION_CHECKS.with(|c| c.get()),
                1
            );
            let answer = r.discourse_answer.as_ref().unwrap();
            let inquiry = answer
                .decision_inquiry
                .as_ref()
                .unwrap_or_else(|| panic!("{text}: {}", r.output.text));
            let action = inquiry.proposed_action.as_ref().unwrap();
            assert_eq!(
                action.event.roles.get(&R::Theme).map(String::as_str),
                Some(object)
            );
            assert_eq!(
                action.event.roles.get(&R::Location).map(String::as_str),
                Some(place)
            );
            assert_eq!(action.event.negated, negative);
            assert_eq!(
                action.event.roles.get(&R::Agent).map(String::as_str),
                if language == LanguageCodeIR::Korean {
                    Some("지우")
                } else {
                    None
                }
            );
            assert!(
                r.output.text.contains(object) && r.output.text.contains(place),
                "{}",
                r.output.text
            );
            if let Some(prior) = &previous_action {
                assert_eq!(action, prior);
            }
            previous_action = Some(action.clone());
            assert!(r.conversation_state.epistemic_ledger.records.is_empty());
            assert!(r.conversation_state.action_state_ledger.records.is_empty());
            assert!(answer.claims.is_empty() && !answer.dialogue_truth_established);
            assert!(
                r.grounded_response.is_none()
                    && !r.language_cortex_integration.external_action_executed
            );
            assert!(r.validate_against(&q));
            let mut forged = inquiry.clone();
            forged
                .proposed_action
                .as_mut()
                .unwrap()
                .event
                .roles
                .insert(R::Agent, "SomeoneElse".into());
            assert!(!forged.validate());
        }
    }
}

#[test]
fn proposal_valency_keeps_sense_alternatives_and_missing_actor_explicit() {
    use crate::proposition_content::ContentSlotIR as R;
    let compile = |text| {
        crate::utterance_intent::decision_inquiry(text)
            .unwrap()
            .proposed_action
            .unwrap()
    };
    let bare = compile("쉬는 게 좋겠네?");
    let object = compile("회사를 쉬는 게 좋겠네?");
    for (action, transitive) in [(&bare, false), (&object, true)] {
        let rest = action
            .frame_candidates
            .iter()
            .find(|f| f.entry_id == "71280" && f.sense_id == "1")
            .unwrap();
        let absence = action
            .frame_candidates
            .iter()
            .find(|f| f.entry_id == "71280" && f.sense_id == "5")
            .unwrap();
        assert!(rest.pattern_understood && absence.pattern_understood);
        assert!(rest.missing_roles.contains(&R::Agent));
        assert_eq!(rest.incompatible_roles.contains(&R::Theme), transitive);
        assert_eq!(absence.missing_roles.contains(&R::Theme), !transitive);
        assert!(!action.event.roles.contains_key(&R::Agent));
    }
    let english = compile("Would it be good to rest?");
    for candidate in &english.frame_candidates {
        let entry = crate::lexical_knowledge_pack::builtin_pack()
            .entry(&candidate.entry_id)
            .unwrap();
        let sense = entry
            .senses
            .iter()
            .find(|s| s.source_sense_id == candidate.sense_id)
            .unwrap();
        assert!(sense
            .english
            .split(';')
            .any(|a| a.trim().eq_ignore_ascii_case("rest")));
    }
    assert_eq!(
        bare.frame_candidates
            .iter()
            .find(|f| f.entry_id == "71280" && f.sense_id == "1"),
        english
            .frame_candidates
            .iter()
            .find(|f| f.entry_id == "71280" && f.sense_id == "1")
    );
    for source in [
        "안 기록을 읽는 게 좋겠네?", // negator cannot jump across an argument
        "Would it be good to read not DeltaLedger?",
        "Would it be good to read DeltaLedger in?",
        "Would it be good to reading DeltaLedger?",
        "Would it be good to read DeltaLedger and delete Archive?",
        "지우가 기록을 읽는 게 좋겠네? 파일을 지워.",
    ] {
        assert!(
            crate::utterance_intent::decision_inquiry(source).is_none(),
            "{source}"
        );
    }
    // The non-finite entry point must not broaden the observation parser.
    assert!(crate::proposition_content::described_event("rest", false).is_none());
    assert!(crate::proposition_content::described_event("not read DeltaLedger", false).is_none());
    assert!(
        crate::proposition_content::described_event("Do not read DeltaLedger", false).is_none()
    );
}

#[test]
fn decision_question_reason_uses_retained_act_not_world_cause_or_reply_text() {
    for (language, turns) in [
        (
            LanguageCodeIR::Korean,
            vec![
                "지우는 피곤해.",
                "그럼 쉬는 게 좋겠네?",
                "왜 그렇게 생각해?",
            ],
        ),
        (
            LanguageCodeIR::Korean,
            vec!["지우는 피곤해.", "그럼 쉬는 게 좋겠네?", "고마워.", "왜?"],
        ),
        (
            LanguageCodeIR::Korean,
            vec!["나 피곤해.", "그럼 어떻게 하지?", "왜 물어?"],
        ),
        (
            LanguageCodeIR::English,
            vec![
                "The visitor is tired.",
                "Would it be good to rest?",
                "Why do you ask?",
            ],
        ),
        (
            LanguageCodeIR::English,
            vec![
                "The visitor is tired.",
                "Would it be good to rest?",
                "Why do you think that?",
            ],
        ),
    ] {
        for formal in [false, true] {
            let mut api = CognitiveApi::new_embedded().unwrap();
            api.set_dialogue_personality(crate::affective_field::DialoguePersonalityIR {
                formality_millis: if formal { 800 } else { 0 },
                ..Default::default()
            })
            .unwrap();
            let mut before = None;
            for (i, text) in turns.iter().enumerate() {
                let q = request("INQUIRY-REASON", i as u64 + 1, text, language);
                crate::generative_language::GENERATION_INVOCATIONS.with(|c| c.set(0));
                crate::natural_realization::FINAL_REALIZATION_CHECKS.with(|c| c.set(0));
                let r = api.process_conversation_turn(&q).unwrap();
                assert_eq!(
                    crate::generative_language::GENERATION_INVOCATIONS.with(|c| c.get()),
                    1
                );
                assert_eq!(
                    crate::natural_realization::FINAL_REALIZATION_CHECKS.with(|c| c.get()),
                    1
                );
                if i + 1 == turns.len() {
                    let a = r.discourse_answer.as_ref().unwrap();
                    let inquiry = a
                        .decision_inquiry
                        .as_ref()
                        .unwrap_or_else(|| panic!("{text}: {}", r.output.text));
                    let origin = inquiry.explanation_of.as_ref().unwrap();
                    assert_eq!(origin.source_text, turns[1]);
                    assert_eq!(origin.asked_turn, 2);
                    assert!(a.world_reasoning.is_none());
                    assert!(a.claims.is_empty());
                    assert!(!a.dialogue_truth_established);
                    assert!(
                        r.output
                            .text
                            .contains(if language == LanguageCodeIR::Korean {
                                "필요해서"
                            } else {
                                "I asked because"
                            }),
                        "{}",
                        r.output.text
                    );
                    let previous: &ConversationStateIR = before.as_ref().unwrap();
                    assert_eq!(
                        r.conversation_state.epistemic_ledger.records,
                        previous.epistemic_ledger.records
                    );
                    assert_eq!(r.conversation_state.dialogue_world, previous.dialogue_world);
                    let mut forged = r.clone();
                    forged
                        .discourse_answer
                        .as_mut()
                        .unwrap()
                        .decision_inquiry
                        .as_mut()
                        .unwrap()
                        .missing_input = crate::utterance_intent::DecisionInputIR::Deadline;
                    assert!(!forged.validate_against(&q));
                }
                assert!(r.grounded_response.is_none());
                assert!(!r.language_cortex_integration.external_action_executed);
                assert!(r.validate_against(&q));
                before = Some(r.conversation_state);
            }
        }
    }
}

#[test]
fn decision_reason_reference_expires_and_never_consumes_a_new_target() {
    for intervening in [
        vec!["Switch to the database."],
        vec!["Thanks.", "Thanks.", "Thanks."],
    ] {
        let mut api = CognitiveApi::new_embedded().unwrap();
        let mut turns = vec!["Would it be good to rest?"];
        turns.extend(intervening);
        turns.push("Why do you ask?");
        for (i, text) in turns.iter().enumerate() {
            let q = request(
                "EXPIRED-DECISION",
                i as u64 + 1,
                text,
                LanguageCodeIR::English,
            );
            let r = api.process_conversation_turn(&q).unwrap();
            if i + 1 == turns.len() {
                assert!(r
                    .discourse_answer
                    .as_ref()
                    .and_then(|a| a.decision_inquiry.as_ref())
                    .is_none());
            }
            assert!(r.validate_against(&q));
        }
    }
    let prior = crate::utterance_intent::decision_inquiry("Would it be good to rest?").unwrap();
    for text in [
        "Why is Mira tired?",
        "Why did she ask?",
        "\"Why do you ask?\"",
        "Why do you ask? Delete the file.",
    ] {
        assert!(
            crate::utterance_intent::DecisionInquiryIR::explain_from(text, &prior, 1, 2).is_none()
        );
    }
}

#[test]
fn action_evaluation_preserves_target_without_execution_or_invented_benefit() {
    for (language, observation, question, target, negative) in [
        (
            LanguageCodeIR::Korean,
            "지우는 피곤해.",
            "그럼 쉬는 게 좋겠네?",
            "쉬는 것",
            false,
        ),
        (
            LanguageCodeIR::Korean,
            "지우는 피곤해.",
            "그럼 안 쉬는 게 좋겠네?",
            "안 쉬는 것",
            true,
        ),
        (
            LanguageCodeIR::Korean,
            "지우는 피곤해.",
            "기록을 읽는 게 좋겠네?",
            "기록을 읽는 것",
            false,
        ),
        (
            LanguageCodeIR::Korean,
            "지우는 피곤해.",
            "파일을 삭제하는 게 좋겠네?",
            "파일을 삭제하는 것",
            false,
        ),
        (
            LanguageCodeIR::English,
            "The visitor is tired.",
            "Would it be good to rest?",
            "rest",
            false,
        ),
        (
            LanguageCodeIR::English,
            "The visitor is tired.",
            "Would it be good to not rest?",
            "not rest",
            true,
        ),
    ] {
        let mut api = CognitiveApi::new_embedded().unwrap();
        let first = api
            .process_conversation_turn(&request("PROPOSAL", 1, observation, language))
            .unwrap();
        let q = request("PROPOSAL", 2, question, language);
        crate::generative_language::GENERATION_INVOCATIONS.with(|c| c.set(0));
        crate::natural_realization::FINAL_REALIZATION_CHECKS.with(|c| c.set(0));
        let r = api.process_conversation_turn(&q).unwrap();
        assert_eq!(
            crate::generative_language::GENERATION_INVOCATIONS.with(|c| c.get()),
            1
        );
        assert_eq!(
            crate::natural_realization::FINAL_REALIZATION_CHECKS.with(|c| c.get()),
            1
        );
        let answer = r.discourse_answer.as_ref().unwrap();
        let inquiry = answer
            .decision_inquiry
            .as_ref()
            .unwrap_or_else(|| panic!("{question}: {}", r.output.text));
        let action = inquiry.proposed_action.as_ref().unwrap();
        assert_eq!(action.surface, target);
        assert_eq!(action.negated, negative);
        assert!(!action.predicate_entry_ids.is_empty());
        assert!(r.output.text.contains(target), "{}", r.output.text);
        assert!(!answer.dialogue_truth_established);
        assert!(answer.claims.is_empty());
        assert!(r.grounded_response.is_none());
        assert!(!r.language_cortex_integration.external_action_executed);
        assert_eq!(
            r.conversation_state.epistemic_ledger.records,
            first.conversation_state.epistemic_ledger.records
        );
        assert!(r.validate_against(&q));
        let mut forged = r.clone();
        forged
            .discourse_answer
            .as_mut()
            .unwrap()
            .decision_inquiry
            .as_mut()
            .unwrap()
            .proposed_action
            .as_mut()
            .unwrap()
            .negated = !negative;
        assert!(!forged.validate_against(&q));
    }
    for source in [
        "쉬는 게 좋겠네.",
        "쉰 게 좋겠네?",
        "못 쉬는 게 좋겠네?",
        "쉬는 게 안 좋겠네?",
        "\"쉬는 게 좋겠네?\"",
        "파일을 삭제하고 쉬는 게 좋겠네?",
        "허구동작하는 게 좋겠네?",
    ] {
        assert!(
            crate::utterance_intent::decision_inquiry(source).is_none(),
            "{source}"
        );
    }
}

#[test]
fn multi_role_answer_shares_attribution_before_realization_without_changing_evidence() {
    for formal in [false, true] {
        let mut api = CognitiveApi::new_embedded().unwrap();
        api.set_dialogue_personality(crate::affective_field::DialoguePersonalityIR {
            formality_millis: if formal { 800 } else { 0 },
            ..Default::default()
        })
        .unwrap();
        let observation = request(
            "ROLE-COORDINATION",
            1,
            "하린이 도서관에서 책을 읽었어.",
            LanguageCodeIR::Korean,
        );
        let first = api.process_conversation_turn(&observation).unwrap();
        let question = request(
            "ROLE-COORDINATION",
            2,
            "누가 어디서 읽었어?",
            LanguageCodeIR::Korean,
        );
        crate::generative_language::GENERATION_INVOCATIONS.with(|c| c.set(0));
        crate::natural_realization::FINAL_REALIZATION_CHECKS.with(|c| c.set(0));
        let r = api.process_conversation_turn(&question).unwrap();
        assert_eq!(
            crate::generative_language::GENERATION_INVOCATIONS.with(|c| c.get()),
            1
        );
        assert_eq!(
            crate::natural_realization::FINAL_REALIZATION_CHECKS.with(|c| c.get()),
            1
        );
        assert_eq!(
            r.output.text.matches("따르면").count(),
            1,
            "{}",
            r.output.text
        );
        assert!(r.output.text.contains("이고,"), "{}", r.output.text);
        assert!(r
            .output
            .text
            .ends_with(if formal { "입니다." } else { "이야." }));
        let answer = r.discourse_answer.as_ref().unwrap();
        assert_eq!(
            answer
                .content_projection
                .as_ref()
                .unwrap()
                .all_bindings()
                .count(),
            2
        );
        assert!(!answer.dialogue_truth_established);
        assert_eq!(
            r.conversation_state.epistemic_ledger.records,
            first.conversation_state.epistemic_ledger.records
        );
        assert!(r.grounded_response.is_none());
        assert!(r.validate_against(&question));
    }
}

#[test]
fn source_event_meaning_outvotes_action_noun_cues_without_a_planning_frame() {
    for (statement, question, followup, person, place) in [
        (
            "The report was printed at the office by Keira.",
            "Who printed the report?",
            "Where did she print the report?",
            "keira",
            "office",
        ),
        (
            "Keira printed a report at the office.",
            "Who printed the report?",
            "Where did she print the report?",
            "keira",
            "office",
        ),
        (
            "The photo was printed by Nara in the studio.",
            "Who printed the photo?",
            "Where did she print the photo?",
            "nara",
            "studio",
        ),
    ] {
        assert!(
            crate::proposition_content::is_event_report(statement),
            "{statement}"
        );
        let mut api = CognitiveApi::new_embedded().unwrap();
        let mut records = None;
        for (i, text) in [statement, question, followup].into_iter().enumerate() {
            let input = request(
                "EVENT-ASSERTION",
                i as u64 + 1,
                text,
                LanguageCodeIR::English,
            );
            crate::generative_language::GENERATION_INVOCATIONS.with(|c| c.set(0));
            crate::natural_realization::FINAL_REALIZATION_CHECKS.with(|c| c.set(0));
            let response = api.process_conversation_turn(&input).unwrap();
            assert_eq!(
                crate::generative_language::GENERATION_INVOCATIONS.with(|c| c.get()),
                1
            );
            assert_eq!(
                crate::natural_realization::FINAL_REALIZATION_CHECKS.with(|c| c.get()),
                1
            );
            if i == 0 {
                assert!(
                    response.conversation_contract.assertion_only,
                    "{statement}: {:?}",
                    response.conversation_contract
                );
                assert!(!response
                    .conversation_state
                    .epistemic_ledger
                    .records
                    .is_empty());
                records = Some(response.conversation_state.epistemic_ledger.records.clone());
            } else {
                assert_eq!(
                    &response.conversation_state.epistemic_ledger.records,
                    records.as_ref().unwrap()
                );
                let answer = response.discourse_answer.as_ref().unwrap();
                let projection = answer
                    .content_projection
                    .as_ref()
                    .unwrap_or_else(|| panic!("{text}: {}", response.output.text));
                assert_eq!(
                    projection.binding.value.to_lowercase(),
                    if i == 1 { person } else { place }
                );
                assert!(!answer.dialogue_truth_established);
            }
            assert!(response.grounded_response.is_none());
            assert!(
                !response
                    .language_cortex_integration
                    .external_action_executed
            );
            assert!(response.validate_against(&input));
        }
    }
}

#[test]
fn event_assertion_does_not_consume_questions_commands_or_unknown_participles() {
    // Do-support in a factual negation and a same-spelled proper name remain
    // legal. This is a structural boundary, not an initial-word blacklist.
    for text in ["Keira did not print the report.", "Do printed a report."] {
        assert!(crate::proposition_content::is_event_report(text), "{text}");
    }
    for text in [
        "Print the report.",
        "Do not print the report.",
        "Who printed the report?",
        "The report was qzorbed by Keira.",
    ] {
        assert!(!crate::proposition_content::is_event_report(text), "{text}");
        let mut api = CognitiveApi::new_embedded().unwrap();
        let input = request("NOT-AN-EVENT-REPORT", 1, text, LanguageCodeIR::English);
        let response = api.process_conversation_turn(&input).unwrap();
        assert!(
            !response
                .conversation_contract
                .evidence
                .iter()
                .any(|e| e == "SOURCE_BOUND_EVENT_REPORT"),
            "{text}"
        );
        assert!(
            !response
                .language_cortex_integration
                .external_action_executed
        );
        assert!(response.validate_against(&input));
    }
}

#[test]
fn unanswered_questions_and_social_turns_do_not_expire_event_context() {
    for (turns, language, expected) in [
        (
            vec![
                "Sora opened a window in the studio.",
                "Who repaired a file?",
                "Where did she open the window?",
            ],
            LanguageCodeIR::English,
            "studio",
        ),
        (
            vec![
                "Nora wrote a letter in the garden.",
                "Thanks!",
                "Where did she write the letter?",
            ],
            LanguageCodeIR::English,
            "garden",
        ),
        (
            vec![
                "Sora opened a window in the studio.",
                "Who opened the window?",
                "Who repaired a file?",
                "Where did she open the window?",
            ],
            LanguageCodeIR::English,
            "studio",
        ),
        (
            vec![
                "하린은 도서관에서 책을 읽었어.",
                "누가 파일을 수리했어?",
                "그녀는 어디서 책을 읽었어?",
            ],
            LanguageCodeIR::Korean,
            "도서관",
        ),
    ] {
        let mut api = CognitiveApi::new_embedded().unwrap();
        let mut records = None;
        for (i, text) in turns.iter().enumerate() {
            let input = request("RETAIN-EVENT", i as u64 + 1, text, language);
            crate::generative_language::GENERATION_INVOCATIONS.with(|c| c.set(0));
            crate::natural_realization::FINAL_REALIZATION_CHECKS.with(|c| c.set(0));
            let response = api.process_conversation_turn(&input).unwrap();
            assert_eq!(
                crate::generative_language::GENERATION_INVOCATIONS.with(|c| c.get()),
                1
            );
            assert_eq!(
                crate::natural_realization::FINAL_REALIZATION_CHECKS.with(|c| c.get()),
                1
            );
            if i == 0 {
                records = Some(response.conversation_state.epistemic_ledger.records.clone());
            } else {
                assert_eq!(
                    &response.conversation_state.epistemic_ledger.records,
                    records.as_ref().unwrap(),
                    "{text}"
                );
            }
            if i == turns.len() - 1 {
                let answer = response.discourse_answer.as_ref().unwrap();
                let projection = answer
                    .content_projection
                    .as_ref()
                    .unwrap_or_else(|| panic!("{text}: {}", response.output.text));
                assert_eq!(projection.binding.value, expected);
                assert!(!projection.reference_bindings.is_empty());
                assert!(!answer.dialogue_truth_established);
            }
            assert!(response.grounded_response.is_none());
            assert!(
                !response
                    .language_cortex_integration
                    .external_action_executed
            );
            assert!(response.validate_against(&input));
        }
    }
}

#[test]
fn event_context_retention_respects_topic_new_observation_and_age_boundaries() {
    let mut expired = vec!["Sora opened a window in the studio."];
    expired.extend(std::iter::repeat_n("Thanks!", 17));
    expired.push("Where did she open the window?");
    for turns in [
        vec![
            "Sora opened a window in the studio.",
            "Switch to the database topic.",
            "Where did she open the window?",
        ],
        vec![
            "Sora opened a window in the studio.",
            "Who opened the window?",
            "Switch to the database topic.",
            "Where did she open the window?",
        ],
        vec![
            "Sora opened a window in the studio.",
            "Nara wrote a letter in the park.",
            "Where did she open the window?",
        ],
        expired,
    ] {
        let mut api = CognitiveApi::new_embedded().unwrap();
        for (i, text) in turns.iter().enumerate() {
            let input = request(
                "EVENT-BOUNDARY",
                i as u64 + 1,
                text,
                LanguageCodeIR::English,
            );
            let response = api.process_conversation_turn(&input).unwrap();
            if i == turns.len() - 1 {
                assert!(
                    response
                        .discourse_answer
                        .as_ref()
                        .is_none_or(|a| a.content_projection.is_none()),
                    "{turns:?}: {}",
                    response.output.text
                );
                assert!(response.grounded_response.is_none());
                assert!(
                    !response
                        .language_cortex_integration
                        .external_action_executed
                );
            }
            assert!(response.validate_against(&input));
        }
    }
}

#[test]
fn question_variables_and_passive_adjuncts_cannot_compete_with_a_person_reference() {
    for (statement, question, followup, person, location) in [
        (
            "The window was opened by Sora in the studio.",
            "Who opened the window?",
            "Where did she open the window?",
            "sora",
            "studio",
        ),
        (
            "The file was repaired in the library by Mina.",
            "Who repaired the file?",
            "Where did he repair the file?",
            "mina",
            "library",
        ),
        (
            "Lior cleaned a gate in the workshop.",
            "Who cleaned the gate?",
            "Where did she clean the gate?",
            "lior",
            "workshop",
        ),
    ] {
        let mut api = CognitiveApi::new_embedded().unwrap();
        let mut records = None;
        for (index, text) in [statement, question, followup].into_iter().enumerate() {
            let input = request(
                "REFERENCE-OWNER",
                index as u64 + 1,
                text,
                LanguageCodeIR::English,
            );
            crate::generative_language::GENERATION_INVOCATIONS.with(|c| c.set(0));
            crate::natural_realization::FINAL_REALIZATION_CHECKS.with(|c| c.set(0));
            let response = api.process_conversation_turn(&input).unwrap();
            assert_eq!(
                crate::generative_language::GENERATION_INVOCATIONS.with(|c| c.get()),
                1
            );
            assert_eq!(
                crate::natural_realization::FINAL_REALIZATION_CHECKS.with(|c| c.get()),
                1
            );
            let people = response
                .conversation_state
                .active_typed_entities
                .iter()
                .filter(|n| n.kind == crate::typed_coreference::TypedEntityKindIR::Person)
                .map(|n| n.normalized_label.as_str())
                .collect::<Vec<_>>();
            assert_eq!(people, vec![person], "{text}");
            if index == 0 {
                records = Some(response.conversation_state.epistemic_ledger.records.clone());
            } else {
                assert_eq!(
                    &response.conversation_state.epistemic_ledger.records,
                    records.as_ref().unwrap()
                );
                let answer = response.discourse_answer.as_ref().unwrap();
                let value = &answer
                    .content_projection
                    .as_ref()
                    .unwrap_or_else(|| {
                        panic!(
                            "{text}: {} reference={:?}",
                            response.output.text, response.reference_resolution
                        )
                    })
                    .binding
                    .value;
                assert_eq!(
                    value.to_lowercase(),
                    if index == 1 { person } else { location }
                );
                assert!(!answer.dialogue_truth_established);
            }
            assert!(
                !response
                    .language_cortex_integration
                    .external_action_executed
            );
            assert!(response.grounded_response.is_none());
            assert!(response.validate_against(&input));
        }
    }
}

#[test]
fn stress_licensed_inflections_reach_event_memory_and_followup_answers() {
    for (statement, question, value) in [
        (
            "The window was opened by Sora in the studio.",
            "Who opened the window?",
            "sora",
        ),
        (
            "Rhea admitted a visitor in the lobby.",
            "Where was the visitor admitted?",
            "lobby",
        ),
        (
            "Eren visited a museum yesterday.",
            "Who visited the museum?",
            "eren",
        ),
    ] {
        let mut api = CognitiveApi::new_embedded().unwrap();
        let first = api
            .process_conversation_turn(&request(
                "STRESS-MEMORY",
                1,
                statement,
                LanguageCodeIR::English,
            ))
            .unwrap();
        let records = first.conversation_state.epistemic_ledger.records;
        let input = request("STRESS-MEMORY", 2, question, LanguageCodeIR::English);
        crate::generative_language::GENERATION_INVOCATIONS.with(|c| c.set(0));
        crate::natural_realization::FINAL_REALIZATION_CHECKS.with(|c| c.set(0));
        let response = api.process_conversation_turn(&input).unwrap();
        assert_eq!(
            crate::generative_language::GENERATION_INVOCATIONS.with(|c| c.get()),
            1
        );
        assert_eq!(
            crate::natural_realization::FINAL_REALIZATION_CHECKS.with(|c| c.get()),
            1
        );
        let answer = response.discourse_answer.as_ref().unwrap();
        let projection = answer
            .content_projection
            .as_ref()
            .unwrap_or_else(|| panic!("{statement} -> {question}: {}", response.output.text));
        assert_eq!(projection.binding.value.to_lowercase(), value);
        assert_eq!(
            response.conversation_state.epistemic_ledger.records,
            records
        );
        assert!(!answer.dialogue_truth_established);
        assert!(response.grounded_response.is_none());
        assert!(
            !response
                .language_cortex_integration
                .external_action_executed
        );
        assert!(response.validate_against(&input));
    }
}

#[test]
fn event_memory_is_queryable_across_active_and_passive_voice() {
    for (statement, question, value) in [
        (
            "Mina repaired a file in the library.",
            "Where was the file repaired?",
            "library",
        ),
        (
            "The file was repaired by Mina in the library.",
            "Where did Mina repair the file?",
            "library",
        ),
        (
            "Lior cleaned a gate in the workshop.",
            "Where was the gate cleaned?",
            "workshop",
        ),
        (
            "The gate was cleaned by Lior in the workshop.",
            "Who cleaned the gate?",
            "lior",
        ),
        (
            "Nara wrote a letter in the park.",
            "Where was the letter written?",
            "park",
        ),
        (
            "The letter was written by Nara in the park.",
            "Who wrote the letter?",
            "nara",
        ),
        (
            "Mina did not repair a file in the library.",
            "Where was the file not repaired?",
            "library",
        ),
    ] {
        let mut api = CognitiveApi::new_embedded().unwrap();
        let first = api
            .process_conversation_turn(&request(
                "VOICE-MEMORY",
                1,
                statement,
                LanguageCodeIR::English,
            ))
            .unwrap();
        let records = first.conversation_state.epistemic_ledger.records;
        assert!(!records.is_empty(), "{statement}");
        let input = request("VOICE-MEMORY", 2, question, LanguageCodeIR::English);
        crate::generative_language::GENERATION_INVOCATIONS.with(|c| c.set(0));
        crate::natural_realization::FINAL_REALIZATION_CHECKS.with(|c| c.set(0));
        let response = api.process_conversation_turn(&input).unwrap();
        assert_eq!(
            crate::generative_language::GENERATION_INVOCATIONS.with(|c| c.get()),
            1
        );
        assert_eq!(
            crate::natural_realization::FINAL_REALIZATION_CHECKS.with(|c| c.get()),
            1
        );
        let answer = response
            .discourse_answer
            .as_ref()
            .expect("event role answer");
        let projection = answer
            .content_projection
            .as_ref()
            .unwrap_or_else(|| panic!("{statement} -> {question}: {}", response.output.text));
        assert_eq!(projection.binding.value.to_lowercase(), value);
        assert_eq!(
            response.conversation_state.epistemic_ledger.records,
            records
        );
        assert!(!answer.dialogue_truth_established);
        assert!(response.grounded_response.is_none());
        assert!(
            !response
                .language_cortex_integration
                .external_action_executed
        );
        assert!(response.validate_against(&input));
    }
}

#[test]
fn passive_queries_do_not_erase_negation_or_invent_other_events() {
    for malformed in [
        "Where was the file wrote?",
        "Where was the file repair?",
        "Where was the file qzorbed?",
        "Where was the file repaired to?",
        "Where was the file repaired by?",
    ] {
        assert!(
            crate::proposition_content::described_event(malformed, true).is_none(),
            "{malformed}"
        );
    }
    // A retained preposition for the SAME requested gap is supported by the
    // role grammar; it must not invent a filled location or change the event.
    let plain =
        crate::proposition_content::described_event("Where was the file repaired?", true).unwrap();
    let stranded =
        crate::proposition_content::described_event("Where was the file repaired in?", true)
            .unwrap();
    assert_eq!(plain.roles, stranded.roles);
    assert_eq!(plain.lexical_entry_ids, stranded.lexical_entry_ids);
    let mut api = CognitiveApi::new_embedded().unwrap();
    api.process_conversation_turn(&request(
        "VOICE-COUNTER",
        1,
        "Mina did not repair a file in the library.",
        LanguageCodeIR::English,
    ))
    .unwrap();
    for (i, question) in [
        "Where was the file repaired?",
        "Where was the gate repaired?",
        "Where was the file cleaned?",
    ]
    .iter()
    .enumerate()
    {
        let input = request(
            "VOICE-COUNTER",
            i as u64 + 2,
            question,
            LanguageCodeIR::English,
        );
        let response = api.process_conversation_turn(&input).unwrap();
        assert!(
            response
                .discourse_answer
                .as_ref()
                .is_none_or(|a| a.content_projection.is_none()),
            "{question}: {}",
            response.output.text
        );
        assert!(!response.output.text.contains("library"));
        assert!(response.grounded_response.is_none());
        assert!(response.validate_against(&input));
    }
}

#[test]
fn relation_queries_keep_their_meaning_when_status_words_have_no_answer() {
    for (text, language) in [
        (
            "How did Mira know that the file failed?",
            LanguageCodeIR::English,
        ),
        ("When was the archive repaired?", LanguageCodeIR::English),
        ("Who verified the report?", LanguageCodeIR::English),
        ("Where was the file repaired?", LanguageCodeIR::English),
        ("Why did the worker fail?", LanguageCodeIR::English),
        ("어떻게 수리했어?", LanguageCodeIR::Korean),
        ("언제 수리했어?", LanguageCodeIR::Korean),
        ("누가 검증했어?", LanguageCodeIR::Korean),
    ] {
        let mut api = CognitiveApi::new_embedded().unwrap();
        let input = request("RELATION-STATUS", 1, text, language);
        crate::generative_language::GENERATION_INVOCATIONS.with(|c| c.set(0));
        crate::natural_realization::FINAL_REALIZATION_CHECKS.with(|c| c.set(0));
        let response = api.process_conversation_turn(&input).unwrap();
        assert_eq!(
            crate::generative_language::GENERATION_INVOCATIONS.with(|c| c.get()),
            1
        );
        assert_eq!(
            crate::natural_realization::FINAL_REALIZATION_CHECKS.with(|c| c.get()),
            1
        );
        assert_eq!(
            response.natural_realization.response_act,
            NaturalResponseActIR::DiscourseAnswer,
            "{text}: {}",
            response.output.text
        );
        let answer = response
            .discourse_answer
            .as_ref()
            .expect("typed relation answer or gap");
        assert!(!answer.dialogue_truth_established);
        assert!(answer.evidence.is_empty());
        assert!(response.grounded_response.is_none());
        assert!(response
            .conversation_state
            .action_state_ledger
            .records
            .is_empty());
        assert!(
            !response
                .language_cortex_integration
                .external_action_executed
        );
        assert!(!response
            .output
            .text
            .contains("host-verified execution receipt"));
        assert!(!response.output.text.contains("호스트 검증 영수증"));
        assert!(response.validate_against(&input));
    }
}

#[test]
fn outcome_queries_do_not_inherit_relational_question_priority() {
    for (setup, question, language, baseline_clarification) in [
        (
            "Inspect the file.",
            "What was the verified result?",
            LanguageCodeIR::English,
            false,
        ),
        (
            "Inspect the queue.",
            "Did it succeed or fail?",
            LanguageCodeIR::English,
            false,
        ),
        (
            "파일을 조사해.",
            "그래서 성공한 거야, 실패한 거야?",
            LanguageCodeIR::Korean,
            // The pre-repair native executable already asks for a target here.
            // Preserve that required-clarification path, not a successful-answer claim.
            true,
        ),
    ] {
        let mut api = CognitiveApi::new_embedded().unwrap();
        api.process_conversation_turn(&request("OUTCOME-CONTROL", 1, setup, language))
            .unwrap();
        let input = request("OUTCOME-CONTROL", 2, question, language);
        let response = api.process_conversation_turn(&input).unwrap();
        assert!(
            if baseline_clarification {
                response.natural_realization.response_act
                    == NaturalResponseActIR::ClarificationRequest
            } else {
                matches!(
                    response.natural_realization.response_act,
                    NaturalResponseActIR::PlanResultStatus | NaturalResponseActIR::ResultAbsence
                )
            },
            "{question}: {}",
            response.output.text
        );
        assert!(response.grounded_response.is_none());
        assert!(
            !response
                .language_cortex_integration
                .external_action_executed
        );
        assert!(response.validate_against(&input));
    }
}

#[test]
fn optional_recorded_plan_does_not_reserve_inline_space_in_every_answer() {
    use crate::discourse_qa::{DiscourseAnswerIR, PlanMethodAnswerIR};
    assert_eq!(
        std::mem::size_of::<Option<Box<PlanMethodAnswerIR>>>(),
        std::mem::size_of::<usize>()
    );
    eprintln!(
        "ANSWER_LAYOUT_BYTES={} OPTIONAL_PLAN_INLINE_BYTES={} OPTIONAL_PLAN_BOX_BYTES={}",
        std::mem::size_of::<DiscourseAnswerIR>(),
        std::mem::size_of::<Option<PlanMethodAnswerIR>>(),
        std::mem::size_of::<Option<Box<PlanMethodAnswerIR>>>()
    );
}

#[test]
fn nonpast_method_gaps_are_not_claims_of_an_occurred_event() {
    for (text, language) in [
        ("어떻게 조사해?", LanguageCodeIR::Korean),
        ("어떻게 저장해?", LanguageCodeIR::Korean),
        ("How does Mira inspect the file?", LanguageCodeIR::English),
        ("How do you repair the archive?", LanguageCodeIR::English),
    ] {
        let mut api = CognitiveApi::new_embedded().unwrap();
        let input = request("METHOD-UNKNOWN", 1, text, language);
        crate::generative_language::GENERATION_INVOCATIONS.with(|c| c.set(0));
        crate::natural_realization::FINAL_REALIZATION_CHECKS.with(|c| c.set(0));
        let response = api.process_conversation_turn(&input).unwrap();
        assert_eq!(
            crate::generative_language::GENERATION_INVOCATIONS.with(|c| c.get()),
            1
        );
        assert_eq!(
            crate::natural_realization::FINAL_REALIZATION_CHECKS.with(|c| c.get()),
            1
        );
        let answer = response.discourse_answer.as_ref().expect("information gap");
        assert_eq!(
            answer.disposition,
            crate::discourse_qa::DiscourseAnswerDispositionIR::NoMatchingRecord,
            "{text}: {}",
            response.output.text
        );
        assert!(answer.query.presuppositions.is_empty());
        assert_eq!(
            answer.unknown_content_slot(),
            Some(crate::proposition_content::ContentSlotIR::Manner)
        );
        assert!(
            response
                .output
                .text
                .contains(if language == LanguageCodeIR::Korean {
                    "방법은 아직 모르겠어"
                } else {
                    "don't yet know the method"
                }),
            "{}",
            response.output.text
        );
        assert!(response.grounded_response.is_none());
        assert!(response
            .conversation_state
            .action_state_ledger
            .records
            .is_empty());
        assert!(
            !response
                .language_cortex_integration
                .external_action_executed
        );
        assert!(response.validate_against(&input));
    }
}

#[test]
fn missing_relation_is_expressed_without_changing_its_epistemic_status() {
    let qa = crate::discourse_qa::DiscourseQaEngine;
    for (text, expected) in [
        ("왜 조사해?", "이유"),
        ("언제 조사해?", "시점"),
        ("어떻게 조사해?", "방법"),
    ] {
        let answer = qa.unanswered(text, LanguageCodeIR::Korean);
        let generated = crate::generative_language::generate_discourse_answer_from_knowledge(
            LanguageCodeIR::Korean,
            &answer,
            &["TEST:ABSENT_ANSWER".into()],
        )
        .unwrap();
        assert!(generated.morphology.realized_text.contains(expected));
        assert!(generated.morphology.realized_text.ends_with("모르겠어."));
        assert!(generated.validate());
        assert_eq!(
            answer.disposition,
            crate::discourse_qa::DiscourseAnswerDispositionIR::NoMatchingRecord
        );
    }
    for text in [
        "How did Mira inspect the file?",
        "어떻게 조사했어?",
        "How did Mira know that the file failed?",
    ] {
        let parsed = qa.parse(text, None).expect("past or factive query");
        assert_eq!(
            parsed.kind,
            crate::discourse_qa::DiscourseQueryKindIR::PresuppositionCheck,
            "{text}"
        );
        assert!(!parsed.presuppositions.is_empty());
    }
}

#[test]
fn method_questions_read_the_recorded_plan_without_reissuing_its_action() {
    for (setup, question, language) in [
        (
            "파일을 조사해.",
            "어떻게 조사하는데?",
            LanguageCodeIR::Korean,
        ),
        ("로그를 조사해.", "어떻게 조사해?", LanguageCodeIR::Korean),
        (
            "Inspect the cache.",
            "How do you inspect the cache?",
            LanguageCodeIR::English,
        ),
        (
            "Inspect the buffer.",
            "How do you inspect the buffer?",
            LanguageCodeIR::English,
        ),
    ] {
        let mut api = CognitiveApi::new_embedded().unwrap();
        let first = api
            .process_conversation_turn(&request("METHOD-READ", 1, setup, language))
            .unwrap();
        let initial = first.grounded_response.as_ref().expect("initial plan");
        let input = request("METHOD-READ", 2, question, language);
        crate::generative_language::GENERATION_INVOCATIONS.with(|c| c.set(0));
        crate::natural_realization::FINAL_REALIZATION_CHECKS.with(|c| c.set(0));
        let response = api.process_conversation_turn(&input).unwrap();
        assert_eq!(
            crate::generative_language::GENERATION_INVOCATIONS.with(|c| c.get()),
            1
        );
        assert_eq!(
            crate::natural_realization::FINAL_REALIZATION_CHECKS.with(|c| c.get()),
            1
        );
        let method = response
            .discourse_answer
            .as_ref()
            .and_then(|a| a.plan_method.as_ref())
            .unwrap_or_else(|| panic!("{question}: {}", response.output.text));
        assert_eq!(method.recorded.bundle, initial.semantic_plan_bundle);
        assert!(response.grounded_response.is_none());
        assert_eq!(
            response.conversation_state.active_goals.len(),
            first.conversation_state.active_goals.len()
        );
        assert_eq!(
            response
                .conversation_state
                .action_state_ledger
                .records
                .len(),
            first.conversation_state.action_state_ledger.records.len()
        );
        assert!(
            !response
                .language_cortex_integration
                .external_action_executed
        );
        assert!(response
            .natural_realization
            .generation_traces
            .iter()
            .all(|t| t
                .speech_intent
                .intents
                .iter()
                .all(|i| i.intent
                    == crate::generative_language::GenerationSpeechIntentIR::DescribePlan)));
        assert!(response.validate_against(&input));
        let mut tampered = response.clone();
        tampered
            .discourse_answer
            .as_mut()
            .unwrap()
            .plan_method
            .as_mut()
            .unwrap()
            .recorded
            .bundle
            .plans[0]
            .steps[0]
            .target = "unrelated target".into();
        assert!(!tampered.validate_against(&input));
        assert_eq!(
            response
                .output
                .text
                .matches(if language == LanguageCodeIR::Korean {
                    "계획은"
                } else {
                    "The plan is"
                })
                .count(),
            1
        );
        assert!(response
            .output
            .text
            .contains(if language == LanguageCodeIR::Korean {
                "여러 가설"
            } else {
                "alternative hypotheses"
            }));
        assert!(response
            .output
            .text
            .contains(if language == LanguageCodeIR::Korean {
                "검사하고"
            } else {
                "then test"
            }));
    }
}

#[test]
fn recorded_method_uses_personality_register_without_changing_its_meaning() {
    let mut outputs = Vec::new();
    let mut meanings = Vec::new();
    for formality in [0, 1000] {
        let mut api = CognitiveApi::new_embedded().unwrap();
        api.set_dialogue_personality(crate::affective_field::DialoguePersonalityIR {
            formality_millis: formality,
            ..Default::default()
        })
        .unwrap();
        api.process_conversation_turn(&request(
            "METHOD-STYLE",
            1,
            "로그를 조사해.",
            LanguageCodeIR::Korean,
        ))
        .unwrap();
        let response = api
            .process_conversation_turn(&request(
                "METHOD-STYLE",
                2,
                "어떻게 조사해?",
                LanguageCodeIR::Korean,
            ))
            .unwrap();
        outputs.push(response.output.text);
        meanings.push(
            response.natural_realization.generation_traces[0]
                .meaning
                .clone(),
        );
    }
    assert_eq!(meanings[0], meanings[1]);
    assert_ne!(outputs[0], outputs[1]);
    assert!(outputs[0].ends_with("거야."));
    assert!(outputs[1].ends_with("것입니다."));
}

#[test]
fn method_read_does_not_supply_past_execution_or_guess_a_different_target() {
    for (setup, question, language) in [
        ("파일을 조사해.", "어떻게 조사했어?", LanguageCodeIR::Korean),
        (
            "파일을 조사해.",
            "어떻게 서버를 조사해?",
            LanguageCodeIR::Korean,
        ),
        (
            "Inspect the file.",
            "How does Mira inspect the file?",
            LanguageCodeIR::English,
        ),
    ] {
        let mut api = CognitiveApi::new_embedded().unwrap();
        api.process_conversation_turn(&request("METHOD-CONTROL", 1, setup, language))
            .unwrap();
        let response = api
            .process_conversation_turn(&request("METHOD-CONTROL", 2, question, language))
            .unwrap();
        assert!(response
            .discourse_answer
            .as_ref()
            .is_none_or(|a| a.plan_method.is_none()));
        assert!(
            !response
                .language_cortex_integration
                .external_action_executed
        );
    }
}

#[test]
fn method_summary_retains_action_validation_and_conversation_identity() {
    let mut api = CognitiveApi::new_embedded().unwrap();
    api.process_conversation_turn(&request(
        "METHOD-SAVE",
        1,
        "자료를 저장해.",
        LanguageCodeIR::Korean,
    ))
    .unwrap();
    let input = request("METHOD-SAVE", 2, "어떻게 저장해?", LanguageCodeIR::Korean);
    let response = api.process_conversation_turn(&input).unwrap();
    assert!(
        response
            .output
            .text
            .contains("후보를 검증하고, 선택한 조치를 적용하고"),
        "{}",
        response.output.text
    );
    assert!(response.validate_against(&input));
    let mut forged = response.clone();
    forged
        .discourse_answer
        .as_mut()
        .unwrap()
        .plan_method
        .as_mut()
        .unwrap()
        .recorded
        .conversation_id = "ANOTHER-CONVERSATION".into();
    assert!(!forged.validate_against(&input));
}

#[test]
fn inflected_reference_gap_cannot_veto_an_existing_explicit_theme_binding() {
    let mut api = CognitiveApi::new_embedded().unwrap();
    api.process_conversation_turn(&request(
        "INFLECTED-BOUND-THEME",
        1,
        "로그를 어떻게 삭제하는지 알아.",
        LanguageCodeIR::Korean,
    ))
    .unwrap();
    let input = request(
        "INFLECTED-BOUND-THEME",
        2,
        "그것을 조사해.",
        LanguageCodeIR::Korean,
    );
    let result = api.process_conversation_turn(&input).unwrap();
    assert!(result
        .native_language_circuit
        .reference_bindings
        .iter()
        .any(|binding| binding.kind
            == crate::native_language_circuit::NativeReferenceKindIR::ExplicitPriorTheme));
    assert!(result
        .reference_resolution
        .ambiguous_reference_surfaces
        .is_empty());
    assert_eq!(
        result
            .grounded_response
            .as_ref()
            .expect("bound target")
            .understanding
            .subject,
        "로그"
    );
    assert_eq!(result.conversation_state.active_typed_entities.len(), 1);
    assert!(!result.language_cortex_integration.external_action_executed);
    assert!(result.validate_against(&input));
}

#[test]
fn korean_reference_mentions_keep_one_object_across_three_turns() {
    let mut api = CognitiveApi::new_embedded().unwrap();
    let initial = api
        .process_conversation_turn(&request(
            "PRONOUN-IDENTITY",
            1,
            "파일을 어떻게 삭제하는지 알아.",
            LanguageCodeIR::Korean,
        ))
        .unwrap();
    let entity_id = initial.conversation_state.active_typed_entities[0]
        .entity_id
        .clone();
    for (offset, text) in ["그걸 조사해.", "그것을 다시 조사해."]
        .into_iter()
        .enumerate()
    {
        let input = request(
            "PRONOUN-IDENTITY",
            offset as u64 + 2,
            text,
            LanguageCodeIR::Korean,
        );
        crate::generative_language::GENERATION_INVOCATIONS.with(|c| c.set(0));
        crate::natural_realization::FINAL_REALIZATION_CHECKS.with(|c| c.set(0));
        let result = api.process_conversation_turn(&input).unwrap();
        assert_eq!(
            crate::generative_language::GENERATION_INVOCATIONS.with(|c| c.get()),
            1
        );
        assert_eq!(
            crate::natural_realization::FINAL_REALIZATION_CHECKS.with(|c| c.get()),
            1
        );
        let entities = &result.conversation_state.active_typed_entities;
        assert_eq!(entities.len(), 1, "{text}: {entities:?}");
        assert_eq!(entities[0].entity_id, entity_id);
        assert_eq!(entities[0].normalized_label, "파일");
        assert_eq!(
            result
                .grounded_response
                .as_ref()
                .expect("bound plan")
                .understanding
                .subject,
            "파일"
        );
        assert!(!result.language_cortex_integration.external_action_executed);
        assert!(result.validate_against(&input));
    }
}

#[test]
fn unresolved_pronouns_cannot_create_their_own_future_antecedent() {
    let mut api = CognitiveApi::new_embedded().unwrap();
    for (i, text) in ["그걸 조사해.", "그것을 조사해.", "그걸 다시 조사해."]
        .into_iter()
        .enumerate()
    {
        let input = request(
            "PRONOUN-NO-ANTECEDENT",
            i as u64 + 1,
            text,
            LanguageCodeIR::Korean,
        );
        let result = api.process_conversation_turn(&input).unwrap();
        assert!(
            result.grounded_response.is_none(),
            "{text}: {} native={:?} refs={:?}",
            result.output.text,
            result.native_language_circuit,
            result.reference_resolution
        );
        assert!(result.conversation_state.active_typed_entities.is_empty());
        assert!(api
            .native_dialogue_memory
            .get("PRONOUN-NO-ANTECEDENT")
            .is_none_or(|memory| {
                memory.active_goals.is_empty() && memory.active_entities.is_empty()
            }));
        assert!(result
            .conversation_state
            .action_state_ledger
            .records
            .is_empty());
        assert!(result.validate_against(&input));
    }
}

#[test]
fn information_content_mentions_remain_usable_without_matrix_pseudo_entities() {
    for (source, expected) in [
        ("I know how to delete the file.", "file"),
        ("I know why the file was deleted.", "file"),
        ("We remember how to inspect the cache.", "cache"),
        ("I understand how to modify the queue.", "queue"),
    ] {
        let mut api = CognitiveApi::new_embedded().unwrap();
        let input = request(
            "CONTENT-NOMINAL-CONTINUITY",
            1,
            source,
            LanguageCodeIR::English,
        );
        let initial = api.process_conversation_turn(&input).unwrap();
        assert!(initial.grounded_response.is_none(), "{source}");
        assert!(initial
            .conversation_state
            .action_state_ledger
            .records
            .is_empty());
        assert!(initial
            .conversation_state
            .dialogue_world
            .premises
            .is_empty());
        let entities = &initial.conversation_state.active_typed_entities;
        assert!(
            entities
                .iter()
                .all(|entity| entity.normalized_label == expected),
            "{source}: {entities:?}"
        );
        // Some existing native predicates have no compositional frame entry.
        // Assert retained, source-grounded noun evidence and the actual next
        // turn outcome, not a fictitious guarantee of identical storage paths.
        assert!(
            entities
                .iter()
                .any(|entity| entity.normalized_label == expected)
                || initial
                    .native_language_circuit
                    .entities
                    .iter()
                    .any(|entity| {
                        entity.surface == expected && entity.start_byte < entity.end_byte
                    }),
            "{source}"
        );
        assert!(initial.validate_against(&input));
        let next = request(
            "CONTENT-NOMINAL-CONTINUITY",
            2,
            "Inspect it.",
            LanguageCodeIR::English,
        );
        crate::generative_language::GENERATION_INVOCATIONS.with(|c| c.set(0));
        crate::natural_realization::FINAL_REALIZATION_CHECKS.with(|c| c.set(0));
        let result = api.process_conversation_turn(&next).unwrap();
        assert_eq!(
            crate::generative_language::GENERATION_INVOCATIONS.with(|c| c.get()),
            1
        );
        assert_eq!(
            crate::natural_realization::FINAL_REALIZATION_CHECKS.with(|c| c.get()),
            1
        );
        assert_eq!(
            result
                .grounded_response
                .as_ref()
                .expect("grounded nominal")
                .understanding
                .subject
                .to_lowercase(),
            expected
        );
        assert!(result
            .reference_resolution
            .ambiguous_reference_surfaces
            .is_empty());
        assert!(!result.language_cortex_integration.external_action_executed);
        assert!(result.validate_against(&next));
    }
}

#[test]
fn content_role_boundaries_preserve_overt_agents_and_real_ambiguity() {
    let analysis = crate::compositional_semantics::CompositionalSemanticAnalyzer
        .analyze("I know why Mira deleted the file.");
    assert!(analysis.semantic_role_graph.role_edges.iter().any(|edge| {
        edge.role == crate::semantic_roles::SemanticRoleKindIR::Agent
            && analysis.semantic_role_graph.nodes.iter().any(|node| {
                node.node_id == edge.argument_node_id && node.normalized_label == "mira"
            })
    }));
    let mut api = CognitiveApi::new_embedded().unwrap();
    let initial = api
        .process_conversation_turn(&request(
            "CONTENT-REAL-AMBIGUITY",
            1,
            "I know how to compare the cache with the queue.",
            LanguageCodeIR::English,
        ))
        .unwrap();
    assert!(initial.grounded_response.is_none());
    assert!(initial.conversation_state.active_typed_entities.len() >= 2);
    let next = request(
        "CONTENT-REAL-AMBIGUITY",
        2,
        "Inspect it.",
        LanguageCodeIR::English,
    );
    let result = api.process_conversation_turn(&next).unwrap();
    assert_eq!(
        result.disposition,
        ConversationTurnDispositionIR::ClarificationRequired
    );
    assert!(result.grounded_response.is_none());
    assert!(result
        .conversation_state
        .action_state_ledger
        .records
        .is_empty());
    assert!(result.validate_against(&next));
}

#[test]
fn acknowledgement_reference_dependencies_preserve_unresolved_content() {
    for text in [
        "I don't know why it happened.",
        "I know why it failed.",
        "We understand how it works.",
        "Why it happened is unclear.",
        "I never knew where it disappeared.",
    ] {
        let mut api = CognitiveApi::new_embedded().unwrap();
        let input = request("CONTENT-REFERENCE-DEMAND", 1, text, LanguageCodeIR::English);
        crate::generative_language::GENERATION_INVOCATIONS.with(|c| c.set(0));
        crate::natural_realization::FINAL_REALIZATION_CHECKS.with(|c| c.set(0));
        let result = api.process_conversation_turn(&input).unwrap();
        assert_eq!(
            crate::generative_language::GENERATION_INVOCATIONS.with(|c| c.get()),
            1
        );
        assert_eq!(
            crate::natural_realization::FINAL_REALIZATION_CHECKS.with(|c| c.get()),
            1
        );
        assert_eq!(
            result.natural_realization.response_act,
            NaturalResponseActIR::InformAcknowledgement,
            "{text}: {}",
            result.output.text
        );
        assert_eq!(result.reference_resolution.resolved_reference_count, 0);
        assert_eq!(
            result.reference_resolution.resolved_semantic_text,
            result.reference_resolution.original_semantic_text
        );
        assert!(result.reference_resolution.used_referent_ids.is_empty());
        assert!(result.conversation_state.dialogue_world.premises.is_empty());
        assert!(result
            .conversation_state
            .action_state_ledger
            .records
            .is_empty());
        assert!(result.grounded_response.is_none());
        if !result
            .reference_resolution
            .ambiguous_reference_surfaces
            .is_empty()
        {
            assert!(result
                .reference_resolution
                .can_defer_content_references_for_acknowledgement());
            assert!(result.conversation_state.unresolved_reference_count > 0);
        }
        if text == "I don't know why it happened." {
            assert_eq!(
                result.reference_resolution.ambiguous_reference_surfaces,
                vec!["it"]
            );
        }
        assert!(result.validate_against(&input));

        // Acknowledging an unresolved statement must not manufacture an entity
        // for a later command. The action requires a binding and still stops.
        let followup = request(
            "CONTENT-REFERENCE-DEMAND",
            2,
            "Delete it.",
            LanguageCodeIR::English,
        );
        let next = api.process_conversation_turn(&followup).unwrap();
        assert_eq!(
            next.disposition,
            ConversationTurnDispositionIR::ClarificationRequired,
            "{text}: {}",
            next.output.text
        );
        assert!(next.grounded_response.is_none());
        assert!(next
            .conversation_state
            .action_state_ledger
            .records
            .is_empty());
        assert!(next.validate_against(&followup));
    }
}

#[test]
fn content_reference_deferral_checks_every_original_occurrence_and_matrix_force() {
    for (text, surfaces, expected) in [
        ("I know why it failed.", vec!["it"], true),
        ("We remember where they went.", vec!["they"], true),
        ("  I know why IT failed.  ", vec!["it"], true),
        ("She knows why it failed.", vec!["she", "it"], false),
        ("It knows how it works.", vec!["it"], false),
        ("I know why it failed.", vec!["CURRENT_TASK"], false),
        ("I know why it failed.", vec!["its"], false),
        ("Do you know why it failed?", vec!["it"], false),
        ("I know why it failed. Delete it.", vec!["it"], false),
        ("I know why it failed and delete it.", vec!["it"], false),
        ("I know why it failed.", vec![], false),
    ] {
        let reference = ReferenceResolutionIR {
            original_semantic_text: text.into(),
            resolved_semantic_text: text.into(),
            resolved_reference_count: 0,
            used_referent_ids: vec![],
            ambiguous_reference_surfaces: surfaces.into_iter().map(str::to_string).collect(),
            topic_anchored_resolution: None,
            discourse_bindings: vec![],
            resolution_graph: crate::reference_resolution_graph::build_reference_resolution_graph(
                text,
                text,
                &[],
                &[],
                &[],
            ),
        };
        assert_eq!(
            reference.can_defer_content_references_for_acknowledgement(),
            expected,
            "{text}"
        );
    }
}

#[test]
fn embedded_information_statements_do_not_become_questions_or_action_goals() {
    for (text, language) in [
        ("왜 그런지는 아직 몰라.", LanguageCodeIR::Korean),
        ("나는 누가 왔는지는 기억해.", LanguageCodeIR::Korean),
        ("서버가 왜 실패했는지 몰라.", LanguageCodeIR::Korean),
        ("어디로 갔는지 이미 알아.", LanguageCodeIR::Korean),
        ("I don't know why it happened.", LanguageCodeIR::English),
        ("I know how to delete the file.", LanguageCodeIR::English),
        (
            "We remember who changed the account.",
            LanguageCodeIR::English,
        ),
        ("What happened is unclear.", LanguageCodeIR::English),
        (
            "When the gate failed is still unknown.",
            LanguageCodeIR::English,
        ),
    ] {
        let parsed = crate::grammatical_scope::embedded_information_statement(text).expect(text);
        assert!(!parsed.content.is_empty() && !parsed.matrix.is_empty());
        assert!(
            !crate::conversation_contract::is_interrogative(text),
            "{text}"
        );
        assert!(
            !crate::proposition_content::is_event_question(text),
            "{text}"
        );
        assert!(
            crate::discourse_qa::DiscourseQaEngine
                .parse(text, None)
                .is_none(),
            "{text}"
        );
        let mut api = CognitiveApi::new_embedded().unwrap();
        let q = request("MATRIX-INFORMATION-STATEMENT", 1, text, language);
        crate::generative_language::GENERATION_INVOCATIONS.with(|c| c.set(0));
        crate::natural_realization::FINAL_REALIZATION_CHECKS.with(|c| c.set(0));
        let r = api.process_conversation_turn(&q).unwrap();
        assert_eq!(
            crate::generative_language::GENERATION_INVOCATIONS.with(|c| c.get()),
            1
        );
        assert_eq!(
            crate::natural_realization::FINAL_REALIZATION_CHECKS.with(|c| c.get()),
            1
        );
        assert!(
            !r.conversation_contract.information_requested,
            "{text}: {:?}",
            r.conversation_contract
        );
        assert!(
            r.conversation_contract.assertion_only,
            "{text}: {:?}",
            r.conversation_contract
        );
        assert!(r.grounded_response.is_none(), "{text}");
        assert!(
            r.native_language_circuit.selected_live_goals.is_empty(),
            "{text}: {:?}",
            r.native_language_circuit.selected_live_goals
        );
        assert_eq!(
            r.pragmatic_interpretation.speech_act,
            SpeechActIR::Inform,
            "{text}"
        );
        assert!(r.pragmatic_interpretation.inferred_goal.is_none(), "{text}");
        assert!(r.conversation_state.action_state_ledger.records.is_empty());
        assert!(
            r.conversation_state.dialogue_world.premises.is_empty(),
            "{text}"
        );
        assert!(r.validate_against(&q), "{text}");
    }
}

#[test]
fn matrix_statement_scope_does_not_hide_questions_requests_or_coordinates() {
    for text in [
        "왜 그런지 알아?",
        "누가 왔어?",
        "왜 그랬는지 알려줘.",
        "누가 왔는지는 기억해.", // imperative/assertion ambiguity: abstain
        "Do you know why it happened?",
        "Tell me how to delete the file.",
        "I want to know why it happened.",
        "What is unclear?",
        "I know how it works and delete the log.",
        "왜 그런지는 몰라. 파일을 삭제해.",
        "\"I know why it failed\"",
    ] {
        assert!(
            crate::grammatical_scope::embedded_information_statement(text).is_none(),
            "{text}"
        );
    }
    for text in [
        "왜 그런지 알아?",
        "누가 왔어?",
        "Do you know why it happened?",
        "What is unclear?",
    ] {
        assert!(
            crate::conversation_contract::is_interrogative(text),
            "{text}"
        );
    }
}

#[test]
fn formal_policy_reaches_social_receipt_and_source_constituents() {
    use crate::affective_field::DialoguePersonalityIR;
    let mut api = CognitiveApi::new_embedded().unwrap();
    api.set_dialogue_personality(DialoguePersonalityIR {
        formality_millis: 800,
        warmth_millis: 800,
        playfulness_millis: 800,
        ..Default::default()
    })
    .unwrap();
    for (index, (text, required)) in [
        ("안녕", "안녕하세요!"),
        ("노아는 행복해.", "행복"),
        ("노아의 상태는 뭐야?", "말씀에 따르면,"),
        ("고마워", "천만에요."),
    ]
    .into_iter()
    .enumerate()
    {
        let q = request(
            "REGISTER-CONSTITUENTS",
            index as u64 + 1,
            text,
            LanguageCodeIR::Korean,
        );
        crate::generative_language::GENERATION_INVOCATIONS.with(|c| c.set(0));
        crate::natural_realization::FINAL_REALIZATION_CHECKS.with(|c| c.set(0));
        let response = api.process_conversation_turn(&q).unwrap();
        assert_eq!(
            crate::generative_language::GENERATION_INVOCATIONS.with(|c| c.get()),
            1
        );
        assert_eq!(
            crate::natural_realization::FINAL_REALIZATION_CHECKS.with(|c| c.get()),
            1
        );
        assert!(
            response.output.text.contains(required),
            "{}",
            response.output.text
        );
        for incompatible in ["안녕!", "알겠어,", "네 말에 따르면,", "ㅎㅎ"] {
            assert!(
                !response.output.text.contains(incompatible),
                "{}",
                response.output.text
            );
        }
        assert!(response.validate_against(&q));
        assert!(
            !response
                .language_cortex_integration
                .external_action_executed
        );
        assert!(response
            .natural_realization
            .generation_traces
            .iter()
            .all(|g| {
                g.context.register == crate::language_knowledge::LanguageRegisterIR::Formal
                    && g.verification.unsupported_claims == 0
            }));
        if index == 2 {
            assert_eq!(
                response
                    .discourse_answer
                    .as_ref()
                    .unwrap()
                    .content_projection
                    .as_ref()
                    .unwrap()
                    .binding
                    .value,
                "행복해"
            );
        }
        if index == 1 {
            assert!(
                response.output.text.ends_with("군요.")
                    || response.output.text.ends_with("합니다."),
                "{}",
                response.output.text
            );
        }
    }
}

#[test]
fn response_cannot_claim_a_formal_policy_over_an_informal_generation() {
    let mut api = CognitiveApi::new_embedded().unwrap();
    let q = request("REGISTER-BOUNDARY", 1, "안녕", LanguageCodeIR::Korean);
    let response = api.process_conversation_turn(&q).unwrap();
    assert!(response.validate_against(&q));
    let mut inconsistent = response;
    inconsistent.dialogue_personality.formality_millis = 800;
    inconsistent.affective_policy = inconsistent
        .affective_field
        .policy_with_personality(&inconsistent.dialogue_personality);
    assert!(inconsistent.affective_policy.formal);
    assert!(!inconsistent.validate_against(&q));
}

#[test]
fn personality_changes_expression_not_the_selected_answer_or_authority() {
    use crate::affective_field::DialoguePersonalityIR as Personality;
    let profiles = [
        Personality::default(),
        Personality {
            warmth_millis: 800,
            ..Default::default()
        },
        Personality {
            playfulness_millis: 800,
            ..Default::default()
        },
        Personality {
            formality_millis: 800,
            ..Default::default()
        },
    ];
    for (language, greeting, observation, question, unknown, plan) in [
        (
            LanguageCodeIR::Korean,
            "안녕",
            "지우는 피곤해.",
            "지우는 어떤 상태야?",
            "다온의 상태는 뭐야?",
            "기록을 읽어.",
        ),
        (
            LanguageCodeIR::English,
            "Hello",
            "The visitor is tired.",
            "How is the visitor?",
            "How is the mechanic?",
            "Read the log.",
        ),
    ] {
        let mut baseline: Option<Vec<ConversationTurnResponseIR>> = None;
        let mut greeting_surfaces = std::collections::BTreeSet::new();
        let mut answer_surfaces = std::collections::BTreeSet::new();
        for profile in profiles {
            let mut api = CognitiveApi::new_embedded().unwrap();
            let configured = api.execute_command(CognitiveApiCommandIR::SetDialoguePersonality {
                personality: profile,
            });
            assert!(configured.ok);
            assert_eq!(
                configured.payload,
                Some(CognitiveApiPayloadIR::DialoguePersonalityUpdated(profile))
            );
            // Invalid configuration cannot replace an established profile.
            assert_eq!(
                api.set_dialogue_personality(Personality {
                    warmth_millis: 1_001,
                    ..profile
                }),
                Err(CognitiveApiError::InvalidRequest)
            );
            let mut responses = Vec::new();
            for (index, text) in [greeting, observation, question, unknown, plan]
                .into_iter()
                .enumerate()
            {
                let q = request("PERSONALITY-SEMANTICS", index as u64 + 1, text, language);
                crate::generative_language::GENERATION_INVOCATIONS.with(|c| c.set(0));
                crate::natural_realization::FINAL_REALIZATION_CHECKS.with(|c| c.set(0));
                let r = api.process_conversation_turn(&q).unwrap();
                assert_eq!(
                    crate::generative_language::GENERATION_INVOCATIONS.with(|c| c.get()),
                    1
                );
                assert_eq!(
                    crate::natural_realization::FINAL_REALIZATION_CHECKS.with(|c| c.get()),
                    1
                );
                assert_eq!(r.dialogue_personality, profile);
                assert!(r.validate_against(&q));
                assert!(!r.language_cortex_integration.external_action_executed);
                if index == 0 {
                    greeting_surfaces.insert(r.output.text.clone());
                }
                if index == 2 {
                    answer_surfaces.insert(r.output.text.clone());
                }
                if let Some(base) = baseline.as_ref() {
                    let b = &base[index];
                    assert_eq!(r.discourse_answer, b.discourse_answer, "{text}");
                    assert_eq!(r.plan_result_boundary, b.plan_result_boundary, "{text}");
                    assert_eq!(
                        r.natural_realization.response_act,
                        b.natural_realization.response_act
                    );
                    let g = &r.natural_realization.generation_traces[0];
                    let bg = &b.natural_realization.generation_traces[0];
                    assert_eq!(g.meaning, bg.meaning, "{text}");
                    assert_eq!(g.speech_intent, bg.speech_intent, "{text}");
                    assert_eq!(g.verification, bg.verification, "{text}");
                }
                if profile.warmth_millis != 0 {
                    let mut tampered = r.clone();
                    tampered.dialogue_personality = Personality::default();
                    assert!(!tampered.validate_against(&q));
                }
                responses.push(r);
            }
            if baseline.is_none() {
                baseline = Some(responses);
            }
        }
        // Actual distinct public surfaces, not merely a changed style flag.
        assert!(greeting_surfaces.len() >= 3, "{greeting_surfaces:?}");
        if language == LanguageCodeIR::Korean {
            assert!(answer_surfaces.len() >= 2, "{answer_surfaces:?}");
        }
    }
}

#[test]
fn configured_korean_dialect_changes_only_the_public_expression_layer() {
    use crate::affective_field::{DialoguePersonalityIR as Personality, KoreanDialectIR as D};
    let q = request("DIALECT-PUBLIC", 1, "안녕", LanguageCodeIR::Korean);
    let mut baseline_api = CognitiveApi::new_embedded().unwrap();
    let baseline = baseline_api.process_conversation_turn(&q).unwrap();
    for (dialect, expected) in [
        (D::Gyeongsang, "반갑데이! 무엇을 도와줄까예?"),
        (D::Chungcheong, "반가워유! 무엇을 도와줄까유?"),
    ] {
        let mut api = CognitiveApi::new_embedded().unwrap();
        api.set_dialogue_personality(Personality {
            korean_dialect: dialect,
            ..Default::default()
        })
        .unwrap();
        let response = api.process_conversation_turn(&q).unwrap();
        assert_eq!(response.output.text, expected);
        assert_eq!(response.dialogue_personality.korean_dialect, dialect);
        assert_eq!(response.affective_policy.korean_dialect, dialect);
        assert_eq!(
            response.natural_realization.generation_traces[0].korean_dialect,
            dialect
        );
        assert_eq!(
            response.natural_realization.response_act,
            baseline.natural_realization.response_act
        );
        assert_eq!(
            response.natural_realization.generation_traces[0].meaning,
            baseline.natural_realization.generation_traces[0].meaning
        );
        assert_eq!(
            response.natural_realization.generation_traces[0].speech_intent,
            baseline.natural_realization.generation_traces[0].speech_intent
        );
        assert!(response.validate_against(&q));

        let mut forged = response.clone();
        forged.dialogue_personality.korean_dialect = D::Standard;
        assert!(!forged.validate_against(&q));
    }

    let english = request("DIALECT-EN", 1, "Hello", LanguageCodeIR::English);
    let mut api = CognitiveApi::new_embedded().unwrap();
    api.set_dialogue_personality(Personality {
        korean_dialect: D::Gyeongsang,
        ..Default::default()
    })
    .unwrap();
    let response = api.process_conversation_turn(&english).unwrap();
    assert!(response
        .natural_realization
        .generation_traces
        .iter()
        .all(|trace| trace.korean_dialect == D::Standard));
    assert!(response.validate_against(&english));
}

#[test]
fn multi_stem_answer_gap_does_not_surface_internal_retrieval_terms() {
    let mut api = CognitiveApi::new_embedded().unwrap();
    api.process_conversation_turn(&request(
        "STANDARD-GAP-SURFACE",
        1,
        "계속할 만한 건 맞아. 단, 처리 시간이 실제로 줄어든다는 조건에서.",
        LanguageCodeIR::Korean,
    ))
    .unwrap();
    let query = request(
        "STANDARD-GAP-SURFACE",
        2,
        "시간이 그대로여도 계속하라는 말이었나?",
        LanguageCodeIR::Korean,
    );
    let response = api.process_conversation_turn(&query).unwrap();
    assert_eq!(
        response.output.text,
        "그 질문에 대해서는 조건에 맞는 대화 기록을 찾지 못했어. 없는 출처나 내용을 추측해서 채우지 않을게."
    );
    assert!(!response.output.text.contains("계속하 그대로여 시간"));
    assert!(response.validate_against(&query));
    let trace = &response.natural_realization.generation_traces[0];
    assert!(trace
        .meaning
        .nodes
        .iter()
        .all(|node| node.concept_id != "C_RUNTIME_GAP_TOPIC"));
    assert!(trace.validate());
}

#[test]
fn social_backchannel_preserves_event_focus_for_a_relative_role_followup() {
    let mut api = CognitiveApi::new_embedded().unwrap();
    let inputs = [
        "다희는 연구실에서 문서를 읽었어.",
        "다희는 뭘 읽었어?",
        "응 고맙다, 이제 좀 알겠네.",
        "읽은 장소도 알려줄래?",
    ];
    let mut responses = Vec::new();
    for (index, text) in inputs.into_iter().enumerate() {
        let query = request(
            "STANDARD-EVENT-FOCUS",
            index as u64 + 1,
            text,
            LanguageCodeIR::Korean,
        );
        let response = api.process_conversation_turn(&query).unwrap();
        assert!(response.validate_against(&query), "{text}");
        responses.push(response);
    }
    let social = &responses[2];
    assert_eq!(
        social.disposition,
        crate::conversation::ConversationTurnDispositionIR::BackchannelOnly
    );
    assert_eq!(social.conversation_state.epistemic_ledger.records.len(), 1);
    assert_eq!(
        social.conversation_state.epistemic_ledger.records[0].status,
        crate::epistemic::BeliefRecordStatusIR::Active
    );
    assert!(social.conversation_state.answer_focus.is_some());
    let followup = &responses[3];
    assert_eq!(followup.output.text, "연구실이야.");
    assert_eq!(
        followup
            .discourse_answer
            .as_ref()
            .and_then(|answer| answer.content_projection.as_ref())
            .map(|projection| projection.binding.value.as_str()),
        Some("연구실")
    );
}

#[test]
fn explicit_topic_return_resolves_a_directional_relative_role_question() {
    let mut api = CognitiveApi::new_embedded().unwrap();
    let inputs = [
        "정호는 수빈에게 신문을 빌려줬어.",
        "아, 그리고 나 오늘 좀 피곤해.",
        "아까 신문 얘기로 돌아가서, 빌려 간 사람 누구였지?",
    ];
    let mut final_response = None;
    for (index, text) in inputs.into_iter().enumerate() {
        let query = request(
            "STANDARD-TOPIC-RETURN",
            index as u64 + 1,
            text,
            LanguageCodeIR::Korean,
        );
        let response = api.process_conversation_turn(&query).unwrap();
        assert!(response.validate_against(&query), "{text}");
        final_response = Some(response);
    }
    let response = final_response.unwrap();
    assert_eq!(response.output.text, "수빈이야.");
    let projection = response
        .discourse_answer
        .as_ref()
        .and_then(|answer| answer.content_projection.as_ref())
        .unwrap();
    assert_eq!(projection.binding.value, "수빈");
    assert!(projection.event_perspective.is_some());
}

#[test]
fn current_urgency_suppresses_playful_personality_before_expression() {
    use crate::affective_field::DialoguePersonalityIR as Personality;
    for (language, urgent, greeting) in [
        (LanguageCodeIR::Korean, "급해 빨리!", "안녕"),
        (LanguageCodeIR::English, "urgent quickly!", "Hello"),
    ] {
        let mut api = CognitiveApi::new_embedded().unwrap();
        api.set_dialogue_personality(Personality {
            playfulness_millis: 800,
            ..Default::default()
        })
        .unwrap();
        api.process_conversation_turn(&request("PERSONALITY-URGENCY", 1, urgent, language))
            .unwrap();
        let q = request("PERSONALITY-URGENCY", 2, greeting, language);
        let r = api.process_conversation_turn(&q).unwrap();
        assert_eq!(r.dialogue_personality.playfulness_millis, 800);
        assert_eq!(r.affective_policy.playfulness_millis, 0);
        assert!(
            r.natural_realization
                .generation_traces
                .iter()
                .all(|g| g.context.emotion
                    != crate::generative_language::GenerationEmotionIR::Playful)
        );
        assert!(r.validate_against(&q));
    }
}

#[test]
fn missing_property_preserves_query_until_observation_supplies_the_answer() {
    for (query, observation, owner, language) in [
        (
            "민서의 상태는 무엇인가요?",
            "민서는 피곤해.",
            "민서",
            LanguageCodeIR::Korean,
        ),
        (
            "지후의 친구는 어떤 상태야",
            "지후의 친구는 행복해.",
            "지후의 친구",
            LanguageCodeIR::Korean,
        ),
        (
            "What is the condition of the mechanic?",
            "The mechanic is tired.",
            "the mechanic",
            LanguageCodeIR::English,
        ),
        ("How is Qv?", "Qv is calm.", "Qv", LanguageCodeIR::English),
    ] {
        let mut api = CognitiveApi::new_embedded().unwrap();
        for (index, text) in [query, observation, query].into_iter().enumerate() {
            let q = request("QUERY-SURVIVES-GAP", index as u64 + 1, text, language);
            crate::generative_language::GENERATION_INVOCATIONS.with(|c| c.set(0));
            crate::natural_realization::FINAL_REALIZATION_CHECKS.with(|c| c.set(0));
            let r = api.process_conversation_turn(&q).unwrap();
            assert_eq!(
                crate::generative_language::GENERATION_INVOCATIONS.with(|c| c.get()),
                1
            );
            assert_eq!(
                crate::natural_realization::FINAL_REALIZATION_CHECKS.with(|c| c.get()),
                1
            );
            assert!(r.validate_against(&q));
            assert!(r.grounded_response.is_none());
            assert!(r.conversation_state.action_state_ledger.records.is_empty());
            if index == 1 {
                continue;
            }
            let answer = r.discourse_answer.as_ref().unwrap();
            let understood = answer.described_query.as_ref().unwrap();
            assert!(
                understood.roles[&crate::proposition_content::ContentSlotIR::Theme]
                    .eq_ignore_ascii_case(owner)
            );
            if index == 0 {
                assert_eq!(
                    answer.disposition,
                    crate::discourse_qa::DiscourseAnswerDispositionIR::NoMatchingRecord
                );
                assert!(r.conversation_state.epistemic_ledger.records.is_empty());
                assert!(
                    r.output.text.to_lowercase().contains(&owner.to_lowercase()),
                    "{}",
                    r.output.text
                );
                assert!(
                    r.output.text.contains("모르") || r.output.text.contains("don't yet know"),
                    "{}",
                    r.output.text
                );
                assert!(!r.output.text.contains("기록") && !r.output.text.contains("record"));
                let mut forged = answer.clone();
                forged.described_query.as_mut().unwrap().roles.insert(
                    crate::proposition_content::ContentSlotIR::Theme,
                    "another-owner".into(),
                );
                assert!(!forged.validate());
                let mut omitted = answer.clone();
                omitted.described_query = None;
                assert!(!omitted.validate());
                let mut noisy_index = answer.clone();
                noisy_index.query.topic_terms = vec!["irrelevant".into()];
                let original =
                    crate::generative_language::generate_discourse_answer_from_knowledge(
                        language,
                        answer,
                        &[],
                    )
                    .unwrap();
                let noisy = crate::generative_language::generate_discourse_answer_from_knowledge(
                    language,
                    &noisy_index,
                    &[],
                )
                .unwrap();
                assert_eq!(
                    original.morphology.realized_text,
                    noisy.morphology.realized_text
                );
            } else {
                assert!(answer.content_projection.is_some());
                assert!(answer.missing_property_owner().is_none());
                assert!(
                    !r.output.text.contains("모르") && !r.output.text.contains("don't yet know")
                );
            }
        }
    }
}

#[test]
fn open_property_questions_bind_memory_once_without_teaching_answers() {
    for (source, questions, language) in [
        (
            "수빈은 피곤해.",
            ["수빈은 어때?", "수빈은 어떤 상태야", "수빈의 상태는 어때?"],
            LanguageCodeIR::Korean,
        ),
        (
            "연우는 답답해.",
            [
                "연우는 어때?",
                "연우는 어떤 상태인가요?",
                "연우의 상태가 무엇인가요?",
            ],
            LanguageCodeIR::Korean,
        ),
        (
            "The visitor is happy.",
            [
                "How is the visitor?",
                "What is the state of the visitor?",
                "How is the condition of the visitor?",
            ],
            LanguageCodeIR::English,
        ),
        (
            "The teacher is not worried.",
            [
                "How is the teacher?",
                "What is the condition of the teacher?",
                "What is the state of the teacher?",
            ],
            LanguageCodeIR::English,
        ),
    ] {
        let mut api = CognitiveApi::new_embedded().unwrap();
        let mut original_binding = None;
        let mut record_count = 0;
        for (index, text) in std::iter::once(source).chain(questions).enumerate() {
            let q = request("OPEN-PROPERTY", index as u64 + 1, text, language);
            crate::generative_language::GENERATION_INVOCATIONS.with(|c| c.set(0));
            crate::natural_realization::FINAL_REALIZATION_CHECKS.with(|c| c.set(0));
            let r = api.process_conversation_turn(&q).unwrap();
            assert_eq!(
                crate::generative_language::GENERATION_INVOCATIONS.with(|c| c.get()),
                1
            );
            assert_eq!(
                crate::natural_realization::FINAL_REALIZATION_CHECKS.with(|c| c.get()),
                1
            );
            assert!(r.validate_against(&q));
            assert!(r.grounded_response.is_none());
            assert!(r.conversation_state.action_state_ledger.records.is_empty());
            if index == 0 {
                record_count = r.conversation_state.epistemic_ledger.records.len();
                continue;
            }
            assert_eq!(
                r.conversation_state.epistemic_ledger.records.len(),
                record_count
            );
            let p = r
                .discourse_answer
                .as_ref()
                .and_then(|a| a.content_projection.as_ref())
                .unwrap_or_else(|| panic!("{text}: {}", r.output.text));
            assert_eq!(
                p.binding.slot,
                crate::proposition_content::ContentSlotIR::Property
            );
            let evidence = (p.belief_id.clone(), p.binding.clone());
            if let Some(original) = &original_binding {
                assert_eq!(&evidence, original);
            } else {
                original_binding = Some(evidence);
            }
        }
        let mut empty = CognitiveApi::new_embedded().unwrap();
        let q = request("NO-PROPERTY-MEMORY", 1, questions[2], language);
        let r = empty.process_conversation_turn(&q).unwrap();
        assert!(r.validate_against(&q));
        assert!(r
            .discourse_answer
            .as_ref()
            .is_none_or(|a| a.content_projection.is_none()));
    }
}

#[test]
fn nominal_reason_question_reaches_the_same_world_decision_once() {
    for (source, direct, nominal, language) in [
        (
            "수빈은 피곤해.",
            "왜 수빈은 피곤해?",
            "수빈이 피곤한 이유가 뭐야?",
            LanguageCodeIR::Korean,
        ),
        (
            "도윤은 한가하지 않아.",
            "왜 도윤은 한가하지 않아?",
            "도윤이 한가하지 않은 원인은 무엇인가요?",
            LanguageCodeIR::Korean,
        ),
        (
            "nora is tired.",
            "Why is nora tired?",
            "What is the reason that nora is tired?",
            LanguageCodeIR::English,
        ),
        (
            "수빈은 피곤해.",
            "왜 수빈은 피곤하지 않아?",
            "수빈이 피곤하지 않은 이유가 뭐야?",
            LanguageCodeIR::Korean,
        ),
    ] {
        let mut api = CognitiveApi::new_embedded().unwrap();
        let mut decision = None;
        for (index, text) in [source, direct, nominal].into_iter().enumerate() {
            let q = request("NOMINAL-REASON", index as u64 + 1, text, language);
            crate::generative_language::GENERATION_INVOCATIONS.with(|c| c.set(0));
            crate::natural_realization::FINAL_REALIZATION_CHECKS.with(|c| c.set(0));
            let r = api.process_conversation_turn(&q).unwrap();
            assert_eq!(
                crate::generative_language::GENERATION_INVOCATIONS.with(|c| c.get()),
                1
            );
            assert_eq!(
                crate::natural_realization::FINAL_REALIZATION_CHECKS.with(|c| c.get()),
                1
            );
            assert!(r.validate_against(&q));
            assert!(r.grounded_response.is_none());
            assert!(r.conversation_state.action_state_ledger.records.is_empty());
            assert_eq!(r.conversation_state.dialogue_world.premises.len(), 1);
            if index == 0 {
                continue;
            }
            let world = r
                .discourse_answer
                .as_ref()
                .unwrap()
                .world_reasoning
                .as_ref()
                .unwrap_or_else(|| panic!("{text}: {}", r.output.text));
            assert!(world.query.explain);
            assert_eq!(
                world.utterance_plan.moves[0].purpose,
                if world.decision.verdict == crate::world_dialogue::WorldVerdictIR::Refuted {
                    crate::world_dialogue::WorldMovePurposeIR::Conclusion
                } else {
                    crate::world_dialogue::WorldMovePurposeIR::CauseUnknown
                }
            );
            if let Some(prior) = &decision {
                assert_eq!(&world.decision, prior);
            } else {
                decision = Some(world.decision.clone());
            }
        }
    }
}

#[test]
fn interpreted_reference_does_not_teach_its_own_nominal_form() {
    for (source, name, expected_forms, expected_answer) in [
        (
            "HanSol이 학생이 행복하다고 말했다.",
            "HanSol",
            1,
            "HanSol이야.",
        ),
        ("Qv는 학생이 행복하다고 말했다.", "Qv", 1, "Qv야."),
        ("NeoVex said the visitor is tired.", "NeoVex", 0, "NeoVex."),
    ] {
        let mut api = CognitiveApi::new_embedded().unwrap();
        let mut stored = None;
        for (index, text) in [
            source,
            "그가 학생이 피곤하다고 말했다.",
            "누가 그렇게 말했어?",
            "그가 학생이 불안하다고 말했다.",
            "누가 그렇게 말했어?",
        ]
        .into_iter()
        .enumerate()
        {
            let input = request(
                "REFERENCE-OBSERVATION",
                index as u64 + 1,
                text,
                LanguageCodeIR::Korean,
            );
            crate::generative_language::GENERATION_INVOCATIONS.with(|c| c.set(0));
            crate::natural_realization::FINAL_REALIZATION_CHECKS.with(|c| c.set(0));
            let r = api.process_conversation_turn(&input).unwrap();
            assert_eq!(
                crate::generative_language::GENERATION_INVOCATIONS.with(|c| c.get()),
                1
            );
            assert_eq!(
                crate::natural_realization::FINAL_REALIZATION_CHECKS.with(|c| c.get()),
                1
            );
            assert!(r.validate_against(&input));
            let entity = r
                .conversation_state
                .active_typed_entities
                .iter()
                .find(|e| e.canonical_surface == name)
                .unwrap();
            assert_eq!(entity.korean_nominal_forms.len(), expected_forms);
            if let Some(before) = &stored {
                assert_eq!(&entity.korean_nominal_forms, before);
            } else {
                stored = Some(entity.korean_nominal_forms.clone());
            }
            assert!(r.grounded_response.is_none());
            assert!(r.conversation_state.action_state_ledger.records.is_empty());
            if index == 2 || index == 4 {
                assert_eq!(r.output.text, expected_answer);
            }
        }
    }
}

#[test]
fn nominal_form_memory_controls_korean_endings_without_semantic_or_generation_retry() {
    for (name, particle, ending) in [
        ("HanSol", "이", "이야."),
        ("Mira", "가", "야."),
        ("Qx", "은", "이야."),
    ] {
        let mut api = CognitiveApi::new_embedded().unwrap();
        let source = format!("{name}{particle} 학생이 행복하다고 말했다.");
        for (index, text) in [
            source.as_str(),
            "누가 그렇게 말했어?",
            "고마워.",
            "누가 그렇게 말했어?",
        ]
        .into_iter()
        .enumerate()
        {
            let input = request(
                "NOMINAL-FORM",
                index as u64 + 1,
                text,
                LanguageCodeIR::Korean,
            );
            crate::generative_language::GENERATION_INVOCATIONS.with(|c| c.set(0));
            crate::natural_realization::FINAL_REALIZATION_CHECKS.with(|c| c.set(0));
            let r = api.process_conversation_turn(&input).unwrap();
            assert_eq!(
                crate::generative_language::GENERATION_INVOCATIONS.with(|c| c.get()),
                1
            );
            assert_eq!(
                crate::natural_realization::FINAL_REALIZATION_CHECKS.with(|c| c.get()),
                1
            );
            assert!(r.validate_against(&input));
            assert!(r.grounded_response.is_none());
            assert!(r.conversation_state.action_state_ledger.records.is_empty());
            if index == 1 || index == 3 {
                assert_eq!(r.output.text, format!("{name}{ending}"));
                let a = r.discourse_answer.as_ref().unwrap();
                assert_eq!(a.korean_nominal_forms.len(), 1);
                assert!(a.validate_response_part_memory(&r.conversation_state));
                let mut forged = a.clone();
                forged.korean_nominal_forms[0].observed_form =
                    format!("{name}{}", if particle == "가" { "이" } else { "가" });
                assert!(!forged.validate_response_part_memory(&r.conversation_state));
            }
        }
    }
}

#[test]
fn unknown_and_conflicting_foreign_nominal_forms_do_not_choose_vowel_by_default() {
    let mut api = CognitiveApi::new_embedded().unwrap();
    for (index, text) in [
        "Qz said the visitor is tired.",
        "누가 그렇게 말했어?",
        "Qz가 학생이 행복하다고 말했다.",
        "누가 그렇게 말했어?",
        "Qz은 학생이 행복하다고 말했다.",
        "누가 그렇게 말했어?",
    ]
    .into_iter()
    .enumerate()
    {
        let input = request(
            "UNKNOWN-CODA",
            index as u64 + 1,
            text,
            LanguageCodeIR::Korean,
        );
        let r = api.process_conversation_turn(&input).unwrap();
        assert!(r.validate_against(&input));
        if index % 2 == 1 {
            assert_eq!(r.output.text, if index == 3 { "Qz야." } else { "Qz." });
            if index == 5 {
                let mut forged = r.discourse_answer.as_ref().unwrap().clone();
                assert_eq!(forged.korean_nominal_forms.len(), 2);
                forged.korean_nominal_forms.pop();
                assert!(!forged.validate_response_part_memory(&r.conversation_state));
            }
        }
    }
}

#[test]
fn observed_source_spelling_survives_memory_and_single_pass_realization() {
    let mut api = CognitiveApi::new_embedded().unwrap();
    for (index, (text, expected)) in [
        (
            "McKay said the visitor is tired.",
            "According to McKay, the visitor is tired.",
        ),
        (
            "UNESCO said the visitor is tired.",
            "According to UNESCO, the visitor is tired.",
        ),
        ("Who said that?", "McKay and UNESCO."),
        (
            "Thanks.",
            "You're welcome. Tell me if you need anything else.",
        ),
        ("Who said it?", "McKay and UNESCO."),
        (
            "eBay reported the client is worried.",
            "According to eBay, the client is worried.",
        ),
        ("Who said that?", "eBay."),
    ]
    .into_iter()
    .enumerate()
    {
        let input = request(
            "OBSERVED-SOURCE",
            index as u64 + 1,
            text,
            LanguageCodeIR::English,
        );
        crate::generative_language::GENERATION_INVOCATIONS.with(|c| c.set(0));
        crate::natural_realization::FINAL_REALIZATION_CHECKS.with(|c| c.set(0));
        let response = api.process_conversation_turn(&input).unwrap();
        assert_eq!(
            crate::generative_language::GENERATION_INVOCATIONS.with(|c| c.get()),
            1
        );
        assert_eq!(
            crate::natural_realization::FINAL_REALIZATION_CHECKS.with(|c| c.get()),
            1
        );
        assert_eq!(response.output.text, expected, "{text}");
        assert!(response.validate_against(&input));
        assert!(response.grounded_response.is_none());
        assert!(response
            .conversation_state
            .action_state_ledger
            .records
            .is_empty());
    }
}

#[test]
fn source_then_content_ellipsis_reaches_one_attributed_clause_once() {
    for (source, who, content_queries, expected, formal_expected, language) in [
        (
            "관리인이 간호사가 피곤하다고 말했다.",
            "누가 그렇게 말했어?",
            vec!["뭐라고?", "무엇이라고요?", "뭐라고"],
            "관리인에 따르면, 간호사가 피곤하다.",
            "관리인에 따르면, 간호사가 피곤합니다.",
            LanguageCodeIR::Korean,
        ),
        (
            "The gardener reported the visitor is tired.",
            "Who said that?",
            vec![
                "Said what?",
                "What was reported?",
                "What did the gardener say?",
                "said what",
            ],
            "According to the gardener, the visitor is tired.",
            "According to the gardener, the visitor is tired.",
            LanguageCodeIR::English,
        ),
    ] {
        let mut api = CognitiveApi::new_embedded().unwrap();
        let turns = std::iter::once(source)
            .chain(std::iter::once(who))
            .chain(content_queries);
        for (index, text) in turns.enumerate() {
            let input = request("SPEECH-CONTENT", index as u64 + 1, text, language);
            crate::generative_language::GENERATION_INVOCATIONS.with(|c| c.set(0));
            crate::natural_realization::FINAL_REALIZATION_CHECKS.with(|c| c.set(0));
            let response = api.process_conversation_turn(&input).unwrap();
            assert_eq!(
                crate::generative_language::GENERATION_INVOCATIONS.with(|c| c.get()),
                1
            );
            assert_eq!(
                crate::natural_realization::FINAL_REALIZATION_CHECKS.with(|c| c.get()),
                1
            );
            assert!(response.validate_against(&input));
            assert!(response.grounded_response.is_none());
            assert!(response
                .conversation_state
                .action_state_ledger
                .records
                .is_empty());
            if index >= 2 {
                assert_eq!(
                    response.output.text,
                    if response.affective_policy.formal {
                        formal_expected
                    } else {
                        expected
                    }
                );
                let answer = response.discourse_answer.as_ref().unwrap();
                assert_eq!(
                    answer.query.kind,
                    crate::discourse_qa::DiscourseQueryKindIR::SourceContent
                );
                assert!(!answer.dialogue_truth_established);
            }
        }
    }
}

#[test]
fn shared_proposition_survives_source_coordination_and_social_bridge_once() {
    for (turns, language) in [
        (
            vec![
                "관리인이 간호사가 피곤하다고 말했다.",
                "기자가 간호사가 피곤하다고 말했다.",
                "누가 간호사가 피곤하다고 말했어?",
                "고마워.",
                "누가 그렇게 말했어?",
            ],
            LanguageCodeIR::Korean,
        ),
        (
            vec![
                "Alice said the visitor is tired.",
                "Bob said the visitor is tired.",
                "Who said the visitor is tired?",
                "Thanks.",
                "Who said that?",
            ],
            LanguageCodeIR::English,
        ),
    ] {
        let mut api = CognitiveApi::new_embedded().unwrap();
        let mut prior_sources = None;
        for (index, text) in turns.iter().enumerate() {
            let input = request("SHARED-PROPOSITION", index as u64 + 1, text, language);
            crate::generative_language::GENERATION_INVOCATIONS.with(|c| c.set(0));
            crate::natural_realization::FINAL_REALIZATION_CHECKS.with(|c| c.set(0));
            let response = api.process_conversation_turn(&input).unwrap();
            assert_eq!(
                crate::generative_language::GENERATION_INVOCATIONS.with(|c| c.get()),
                1
            );
            assert_eq!(
                crate::natural_realization::FINAL_REALIZATION_CHECKS.with(|c| c.get()),
                1
            );
            assert!(response.validate_against(&input));
            assert!(response
                .conversation_state
                .action_state_ledger
                .records
                .is_empty());
            if index == 2 || index == 4 {
                let answer = response.discourse_answer.as_ref().unwrap();
                let sources = answer
                    .evidence
                    .iter()
                    .map(|e| e.source_actor.clone())
                    .collect::<std::collections::BTreeSet<_>>();
                assert_eq!(sources.len(), 2, "{text}: {}", response.output.text);
                if index == 2 {
                    prior_sources = Some(sources);
                } else {
                    assert_eq!(Some(sources), prior_sources);
                }
                let focus = response.conversation_state.answer_focus.as_ref().unwrap();
                assert_eq!(
                    focus.shared_proposition.as_ref().unwrap().belief_ids.len(),
                    2
                );
                assert!(focus.proposition_belief_id.is_none());
                assert!(!answer.dialogue_truth_established);
            }
        }
    }
}

#[test]
fn source_followup_binds_proposition_not_executable_goal_once() {
    for (source, question, followup, actor, language) in [
        (
            "관리인이 간호사가 피곤하다고 말했다.",
            "누가 피곤해?",
            "누가 그렇게 말했어?",
            "관리인",
            LanguageCodeIR::Korean,
        ),
        (
            "기자가 학생이 불안하다고 말했다.",
            "누가 불안해?",
            "누가 그렇게 말했어?",
            "기자",
            LanguageCodeIR::Korean,
        ),
        (
            "The librarian said the visitor is exhausted.",
            "Who is exhausted?",
            "Who said that?",
            "the librarian",
            LanguageCodeIR::English,
        ),
        (
            "The courier said the client is worried.",
            "Who is worried?",
            "Who said it?",
            "the courier",
            LanguageCodeIR::English,
        ),
    ] {
        let mut api = CognitiveApi::new_embedded().unwrap();
        let mut focused_id = None;
        for (index, text) in [source, question, followup].iter().enumerate() {
            let input = request("SOURCE-QUERY-OWNER", index as u64 + 1, text, language);
            crate::generative_language::GENERATION_INVOCATIONS.with(|c| c.set(0));
            crate::natural_realization::FINAL_REALIZATION_CHECKS.with(|c| c.set(0));
            let response = api.process_conversation_turn(&input).unwrap();
            assert_eq!(
                crate::generative_language::GENERATION_INVOCATIONS.with(|c| c.get()),
                1
            );
            assert_eq!(
                crate::natural_realization::FINAL_REALIZATION_CHECKS.with(|c| c.get()),
                1
            );
            assert!(response.validate_against(&input));
            assert!(response.grounded_response.is_none());
            assert!(response
                .conversation_state
                .action_state_ledger
                .records
                .is_empty());
            if index == 1 {
                focused_id = response
                    .discourse_answer
                    .as_ref()
                    .unwrap()
                    .focused_proposition();
            }
            if index == 2 {
                let answer = response
                    .discourse_answer
                    .as_ref()
                    .expect("source query owner");
                assert_eq!(
                    answer.query.kind,
                    crate::discourse_qa::DiscourseQueryKindIR::PropositionSources
                );
                assert_eq!(answer.evidence.len(), 1, "{}", response.output.text);
                assert_eq!(answer.evidence[0].source_actor.to_lowercase(), actor);
                assert!(source.contains(&answer.evidence[0].source_actor));
                let expected = if language == LanguageCodeIR::Korean {
                    format!(
                        "{actor}{}",
                        if actor.ends_with('인') {
                            "이야."
                        } else {
                            "야."
                        }
                    )
                } else {
                    let mut chars = actor.chars();
                    format!(
                        "{}{}.",
                        chars.next().unwrap().to_uppercase(),
                        chars.as_str()
                    )
                };
                assert_eq!(response.output.text, expected);
                assert_eq!(Some(&answer.evidence[0].belief_id), focused_id.as_ref());
                assert_eq!(
                    answer
                        .contextual_target
                        .as_ref()
                        .unwrap_or_else(|| panic!(
                            "{text}: query={:?} reference={:?} output={}",
                            answer.query, response.reference_resolution, response.output.text
                        ))
                        .belief_id,
                    answer.evidence[0].belief_id
                );
                assert!(!answer.dialogue_truth_established);
                assert!(!answer.external_execution_authorized);
                assert!(!response
                    .reference_resolution
                    .ambiguous_reference_surfaces
                    .iter()
                    .any(|s| s == "ELLIPTICAL_ACTION"));
            }
        }
    }
}

#[test]
fn attributed_state_source_and_bearer_remain_distinct_through_realization_and_queries() {
    for (source, actor, bearer, question, property_question, property, language) in [
        (
            "The courier said the client is worried.",
            "the courier",
            "the client",
            "Who is worried?",
            "How is the client?",
            "is worried",
            LanguageCodeIR::English,
        ),
        (
            "The teacher reported the student is tired.",
            "the teacher",
            "the student",
            "Who is tired?",
            "How is the student?",
            "is tired",
            LanguageCodeIR::English,
        ),
        (
            "배달원이 학생이 불안하다고 말했다.",
            "배달원",
            "학생",
            "누가 불안해?",
            "학생은 어때?",
            "불안하다",
            LanguageCodeIR::Korean,
        ),
        (
            "기자가 학생이 불안하지 않다고 말했다.",
            "기자",
            "학생",
            "누가 불안하지 않아?",
            "학생은 어때?",
            "불안하지 않다",
            LanguageCodeIR::Korean,
        ),
        (
            "선생님이 학생이 피곤하다고 말했다.",
            "선생님",
            "학생",
            "누가 피곤해?",
            "학생은 어때?",
            "피곤하다",
            LanguageCodeIR::Korean,
        ),
    ] {
        let mut api = CognitiveApi::new_embedded().unwrap();
        for (turn, text) in [(1, source), (2, question), (3, property_question)] {
            let q = request("ATTRIBUTED-STATE", turn, text, language);
            crate::generative_language::GENERATION_INVOCATIONS.with(|n| n.set(0));
            crate::natural_realization::FINAL_REALIZATION_CHECKS.with(|n| n.set(0));
            let r = api.process_conversation_turn(&q).unwrap();
            assert_eq!(
                crate::generative_language::GENERATION_INVOCATIONS.with(|n| n.get()),
                1,
                "{text}: {}",
                r.output.text
            );
            assert_eq!(
                crate::natural_realization::FINAL_REALIZATION_CHECKS.with(|n| n.get()),
                1
            );
            assert!(r.validate_against(&q));
            assert_ne!(
                r.natural_realization.response_act,
                NaturalResponseActIR::AffectSupport
            );
            assert!(
                r.output.text.to_lowercase().contains(actor),
                "{text}: {}",
                r.output.text
            );
            if turn == 1 {
                let record = &r.conversation_state.epistemic_ledger.records[0];
                assert_eq!(record.source_actor.to_lowercase(), actor);
                assert!(source.contains(&record.source_actor));
                assert_eq!(
                    record.content.events[0].roles
                        [&crate::proposition_content::ContentSlotIR::Theme],
                    bearer
                );
                assert!(!record.dialogue_truth_established);
                assert!(!record.external_execution_authorized);
            } else {
                let p = r
                    .discourse_answer
                    .as_ref()
                    .and_then(|a| a.content_projection.as_ref())
                    .unwrap_or_else(|| panic!("{text}: {}", r.output.text));
                assert_eq!(p.source_actor.to_lowercase(), actor);
                assert!(source.contains(&p.source_actor));
                assert_eq!(p.binding.value, if turn == 2 { bearer } else { property });
                if turn == 2 {
                    let mut forged = p.clone();
                    forged.binding.value = actor.into();
                    assert!(!forged.validate());
                }
            }
            assert!(r.conversation_state.action_state_ledger.records.is_empty());
            println!("ATTRIBUTED-STATE {text} => {}", r.output.text);
        }
    }
}

#[test]
fn reported_affect_scope_does_not_endorse_denials_or_erase_matrix_affect() {
    use crate::affective_field::{expressed_affect, ExpressedAffectIR};
    assert_eq!(
        expressed_affect("The courier said the client is worried, but I feel angry."),
        Some(ExpressedAffectIR::Angry)
    );
    for source in [
        "The courier denied the client is worried.",
        "The courier did not say the client is worried.",
    ] {
        let mut api = CognitiveApi::new_embedded().unwrap();
        let q = request("DENIED-STATE", 1, source, LanguageCodeIR::English);
        let r = api.process_conversation_turn(&q).unwrap();
        assert!(r.validate_against(&q));
        assert_ne!(
            r.natural_realization.response_act,
            NaturalResponseActIR::AffectSupport
        );
        assert!(
            !r.output.text.contains("the client is worried"),
            "{}",
            r.output.text
        );
        let q = request(
            "DENIED-STATE",
            2,
            "Who is worried?",
            LanguageCodeIR::English,
        );
        let r = api.process_conversation_turn(&q).unwrap();
        assert!(r.validate_against(&q));
        assert!(
            r.discourse_answer
                .as_ref()
                .is_none_or(|a| a.content_projection.is_none()),
            "{}",
            r.output.text
        );
    }
}

#[test]
fn attributed_state_meaning_owns_response_instead_of_affect_keyword() {
    for (source, question, original_bearer, property, output_clause, language) in [
        (
            "The pupil is tired.",
            "How is the pupil?",
            "The pupil",
            "is tired",
            "the pupil is tired",
            LanguageCodeIR::English,
        ),
        (
            "The pupil is angry.",
            "How is the pupil?",
            "The pupil",
            "is angry",
            "the pupil is angry",
            LanguageCodeIR::English,
        ),
        (
            "The teacher is worried.",
            "How is the teacher?",
            "The teacher",
            "is worried",
            "the teacher is worried",
            LanguageCodeIR::English,
        ),
        (
            "I am tired.",
            "How am I?",
            "I",
            "am tired",
            "you are tired",
            LanguageCodeIR::English,
        ),
        (
            "I was not worried.",
            "How am I?",
            "I",
            "was not worried",
            "you were not worried",
            LanguageCodeIR::English,
        ),
        (
            "나는 피곤해.",
            "나는 어때?",
            "나",
            "피곤해",
            // Korean permits omission of the already-established addressee.
            "피곤",
            LanguageCodeIR::Korean,
        ),
    ] {
        let mut api = CognitiveApi::new_embedded().unwrap();
        let q = request("STATE-OWNER", 1, source, language);
        crate::generative_language::GENERATION_INVOCATIONS.with(|n| n.set(0));
        crate::natural_realization::FINAL_REALIZATION_CHECKS.with(|n| n.set(0));
        let r = api.process_conversation_turn(&q).unwrap();
        assert_eq!(
            crate::generative_language::GENERATION_INVOCATIONS.with(|n| n.get()),
            1,
            "{source}"
        );
        assert_eq!(
            crate::natural_realization::FINAL_REALIZATION_CHECKS.with(|n| n.get()),
            1,
            "{source}"
        );
        assert!(
            r.natural_realization.response_act == NaturalResponseActIR::InformAcknowledgement
                || r.natural_realization.response_act == NaturalResponseActIR::DiscourseAnswer
                    && r.discourse_answer
                        .as_ref()
                        .is_some_and(|a| a.world_memory_update.is_some()),
            "{source}: {}",
            r.output.text
        );
        assert!(
            r.output.text.contains(output_clause),
            "{source}: {}",
            r.output.text
        );
        let event = &r.conversation_state.epistemic_ledger.records[0]
            .content
            .events[0];
        assert_eq!(
            event.roles[&crate::proposition_content::ContentSlotIR::Theme],
            original_bearer
        );
        assert_eq!(event.predicate_surface, property);
        assert!(r.conversation_state.action_state_ledger.records.is_empty());
        assert!(r.validate_against(&q));
        let follow = request("STATE-OWNER", 2, question, language);
        let answer = api.process_conversation_turn(&follow).unwrap();
        assert!(answer.validate_against(&follow));
        let projection = answer
            .discourse_answer
            .as_ref()
            .and_then(|a| a.content_projection.as_ref())
            .unwrap_or_else(|| panic!("{question}: {}", answer.output.text));
        assert_eq!(projection.binding.value, property);
        assert!(
            answer.output.text.contains(output_clause),
            "{question}: {}",
            answer.output.text
        );
        println!(
            "STATE-OWNER {source} => {} / {question} => {}",
            r.output.text, answer.output.text
        );
    }
}

#[test]
fn state_typed_reference_uses_definition_evidence_before_single_realization() {
    for (source, question, expected, language) in [
        (
            "창고가 어두워.",
            "거긴 어때?",
            "어두워",
            LanguageCodeIR::Korean,
        ),
        (
            "학생이 피곤해.",
            "그 사람은 어때?",
            "피곤해",
            LanguageCodeIR::Korean,
        ),
        (
            "The warehouse is quiet.",
            "How is that place?",
            "is quiet",
            LanguageCodeIR::English,
        ),
        // The explicit person reference disambiguates the nominal senses.
        (
            "교사가 조용해.",
            "그 사람은 어때?",
            "조용해",
            LanguageCodeIR::Korean,
        ),
    ] {
        let mut api = CognitiveApi::new_embedded().unwrap();
        api.process_conversation_turn(&request("STATE-TYPE", 1, source, language))
            .unwrap();
        let q = request("STATE-TYPE", 2, question, language);
        crate::generative_language::GENERATION_INVOCATIONS.with(|n| n.set(0));
        crate::natural_realization::FINAL_REALIZATION_CHECKS.with(|n| n.set(0));
        let r = api.process_conversation_turn(&q).unwrap();
        assert_eq!(
            crate::generative_language::GENERATION_INVOCATIONS.with(|n| n.get()),
            1
        );
        assert_eq!(
            crate::natural_realization::FINAL_REALIZATION_CHECKS.with(|n| n.get()),
            1
        );
        let p = r
            .discourse_answer
            .as_ref()
            .and_then(|a| a.content_projection.as_ref())
            .unwrap_or_else(|| panic!("{source} / {question} => {}", r.output.text));
        assert_eq!(p.binding.value, expected);
        assert!(p.reference_bindings[0].nominal_type_evidence.is_some());
        assert!(r.validate_against(&q));
        let mut forged = p.clone();
        forged.reference_bindings[0]
            .nominal_type_evidence
            .as_mut()
            .unwrap()
            .definition_heads
            .clear();
        assert!(!forged.matches_question(question));
        println!("STATE-TYPE {source} / {question} => {}", r.output.text);
    }
    for (source, question) in [
        ("가방이 무거워.", "거긴 어때?"),
        ("시멘트가 무거워.", "거긴 어때?"),
        ("상자가 무거워.", "그 사람은 어때?"),
    ] {
        let mut api = CognitiveApi::new_embedded().unwrap();
        api.process_conversation_turn(&request("TYPE-UNKNOWN", 1, source, LanguageCodeIR::Korean))
            .unwrap();
        let q = request("TYPE-UNKNOWN", 2, question, LanguageCodeIR::Korean);
        let r = api.process_conversation_turn(&q).unwrap();
        assert!(
            r.discourse_answer
                .as_ref()
                .is_none_or(|a| a.content_projection.is_none()),
            "{source} / {question} => {}",
            r.output.text
        );
        assert!(r.validate_against(&q));
    }
}

#[test]
fn absent_human_sense_is_a_lexical_gap_not_permission_to_invent_a_type() {
    // This is an abstention-boundary regression, NOT a natural-conversation
    // success case. The current pack gives pupil only its eye-anatomy sense.
    let evidence = crate::lexical_knowledge_pack::builtin_pack().nominal_referent_evidence_for(
        "pupil",
        crate::lexical_knowledge_pack::NominalReferentKindIR::Person,
    );
    assert!(evidence.is_none());
    let mut api = CognitiveApi::new_embedded().unwrap();
    api.process_conversation_turn(&request(
        "LEXICAL-GAP",
        1,
        "The pupil is tired.",
        LanguageCodeIR::English,
    ))
    .unwrap();
    let q = request(
        "LEXICAL-GAP",
        2,
        "How is that person?",
        LanguageCodeIR::English,
    );
    let r = api.process_conversation_turn(&q).unwrap();
    assert!(r.validate_against(&q));
    assert!(r
        .discourse_answer
        .as_ref()
        .is_none_or(|a| a.content_projection.is_none()));
}

#[test]
fn state_reference_queries_use_bearer_center_and_preserve_social_continuity() {
    for (source, social, question, expected, language) in [
        (
            "상자는 무겁지 않아.",
            "고마워.",
            "그것은 어때?",
            "무겁지 않아",
            LanguageCodeIR::Korean,
        ),
        (
            "상자는 무겁지 않아.",
            "고마워.",
            "그건 어땠지?",
            "무겁지 않아",
            LanguageCodeIR::Korean,
        ),
        (
            "The box is not heavy.",
            "Thanks.",
            "How is it?",
            "is not heavy",
            LanguageCodeIR::English,
        ),
    ] {
        let mut api = CognitiveApi::new_embedded().unwrap();
        let first = api
            .process_conversation_turn(&request("STATE-REF", 1, source, language))
            .unwrap();
        let center = first.conversation_state.discourse_focus.current().unwrap();
        assert!(
            center.surface == "상자" || center.surface.eq_ignore_ascii_case("The box"),
            "{center:?}"
        );
        api.process_conversation_turn(&request("STATE-REF", 2, social, language))
            .unwrap();
        let q = request("STATE-REF", 3, question, language);
        crate::generative_language::GENERATION_INVOCATIONS.with(|n| n.set(0));
        crate::natural_realization::FINAL_REALIZATION_CHECKS.with(|n| n.set(0));
        let r = api.process_conversation_turn(&q).unwrap();
        assert_eq!(
            crate::generative_language::GENERATION_INVOCATIONS.with(|n| n.get()),
            1
        );
        assert_eq!(
            crate::natural_realization::FINAL_REALIZATION_CHECKS.with(|n| n.get()),
            1
        );
        assert!(r.validate_against(&q));
        println!("STATE-REF {question} => {}", r.output.text);
        let p = r
            .discourse_answer
            .as_ref()
            .and_then(|a| a.content_projection.as_ref())
            .unwrap_or_else(|| panic!("{}", r.output.text));
        assert_eq!(p.binding.value, expected);
        assert_eq!(p.reference_bindings.len(), 1);
        let mut forged = p.clone();
        forged.reference_bindings[0].value = "INVENTED".into();
        assert!(!forged.matches_question(question));
        assert!(r.conversation_state.action_state_ledger.records.is_empty());
    }
}

#[test]
fn state_reference_binding_preserves_ambiguity_and_topic_changes() {
    let mut api = CognitiveApi::new_embedded().unwrap();
    api.process_conversation_turn(&request(
        "STATE-AMBIGUITY",
        1,
        "The room is quiet. The box is heavy.",
        LanguageCodeIR::English,
    ))
    .unwrap();
    api.process_conversation_turn(&request(
        "STATE-AMBIGUITY",
        2,
        "Thanks.",
        LanguageCodeIR::English,
    ))
    .unwrap();
    let q = request("STATE-AMBIGUITY", 3, "How is it?", LanguageCodeIR::English);
    let ambiguous = api.process_conversation_turn(&q).unwrap();
    assert!(ambiguous.validate_against(&q));
    println!("STATE-AMBIGUITY {}", ambiguous.output.text);
    assert!(ambiguous
        .discourse_answer
        .as_ref()
        .is_some_and(|a| a.reference_gap.is_some()));
    assert!(ambiguous
        .discourse_answer
        .as_ref()
        .unwrap()
        .content_projection
        .is_none());
    let resolved = api
        .process_conversation_turn(&request(
            "STATE-AMBIGUITY",
            4,
            "The room",
            LanguageCodeIR::English,
        ))
        .unwrap();
    println!("STATE-SELECT {}", resolved.output.text);
    assert_eq!(
        resolved
            .discourse_answer
            .as_ref()
            .and_then(|a| a.content_projection.as_ref())
            .map(|p| p.binding.value.as_str()),
        Some("is quiet")
    );

    let mut api = CognitiveApi::new_embedded().unwrap();
    for (index, text) in [
        "The box is heavy.",
        "What is heavy?",
        "The bag is light.",
        "Thanks.",
    ]
    .iter()
    .enumerate()
    {
        api.process_conversation_turn(&request(
            "STATE-SHIFT",
            index as u64 + 1,
            text,
            LanguageCodeIR::English,
        ))
        .unwrap();
    }
    let current = api
        .process_conversation_turn(&request(
            "STATE-SHIFT",
            5,
            "How is it?",
            LanguageCodeIR::English,
        ))
        .unwrap();
    assert_eq!(
        current
            .discourse_answer
            .as_ref()
            .and_then(|a| a.content_projection.as_ref())
            .map(|p| p.binding.value.as_str()),
        Some("is light"),
        "{}",
        current.output.text
    );
    api.process_conversation_turn(&request(
        "STATE-SHIFT",
        6,
        "Let's talk about the Beryl queue.",
        LanguageCodeIR::English,
    ))
    .unwrap();
    let shifted = api
        .process_conversation_turn(&request(
            "STATE-SHIFT",
            7,
            "How is it?",
            LanguageCodeIR::English,
        ))
        .unwrap();
    assert!(
        shifted
            .discourse_answer
            .as_ref()
            .is_none_or(|a| a.content_projection.is_none()),
        "{}",
        shifted.output.text
    );
    assert!(!shifted.language_cortex_integration.external_action_executed);
}

#[test]
fn state_acknowledgement_assembles_grammar_from_meaning_once() {
    for (source, expected, language) in [
        ("통로가 좁아.", "통로가 좁구나.", LanguageCodeIR::Korean),
        (
            "방이 조용하지 않아.",
            "방이 조용하지 않구나.",
            LanguageCodeIR::Korean,
        ),
        ("방이 조용했어.", "방이 조용했구나.", LanguageCodeIR::Korean),
        (
            "The room was not quiet.",
            "was not quiet",
            LanguageCodeIR::English,
        ),
    ] {
        let mut api = CognitiveApi::new_embedded().unwrap();
        let q = request("STATE-ACK", 1, source, language);
        crate::generative_language::GENERATION_INVOCATIONS.with(|n| n.set(0));
        crate::natural_realization::FINAL_REALIZATION_CHECKS.with(|n| n.set(0));
        let r = api.process_conversation_turn(&q).unwrap();
        assert_eq!(
            crate::generative_language::GENERATION_INVOCATIONS.with(|n| n.get()),
            1
        );
        assert_eq!(
            crate::natural_realization::FINAL_REALIZATION_CHECKS.with(|n| n.get()),
            1
        );
        assert!(r.validate_against(&q));
        println!("STATE-ACK {source} => {}", r.output.text);
        println!(
            "STATE-ACK-CONTEXT {:?} assertion={} records={:?}",
            r.natural_realization.response_act,
            r.conversation_contract.assertion_only,
            r.conversation_state
                .epistemic_ledger
                .records
                .iter()
                .map(|record| (
                    &record.source_actor,
                    &record.proposition_surface,
                    &record.status,
                    &record.signature.modal_world,
                    record
                        .content
                        .events
                        .iter()
                        .map(|event| (
                            event.kind,
                            crate::generative_language::EventSummaryIR {
                                omitted_roles: vec![],
                                belief_id: record.belief_id.clone(),
                                source_actor: record.source_actor.clone(),
                                source_proposition: record.proposition_surface.clone(),
                                context_sources: record.content.context_sources.clone(),
                                event: event.clone(),
                            }
                            .can_acknowledge(language)
                        ))
                        .collect::<Vec<_>>()
                ))
                .collect::<Vec<_>>()
        );
        assert!(r.output.text.contains(expected), "{}", r.output.text);
        assert!(r.conversation_state.action_state_ledger.records.is_empty());
        assert!(!r.language_cortex_integration.external_action_executed);
    }
}

#[test]
fn state_property_queries_preserve_source_predicate_and_polarity() {
    use crate::proposition_content::{described_event, requested_content_slots, ContentSlotIR};
    for (source, question, expected, language) in [
        (
            "통로가 좁아.",
            "통로가 어땠지?",
            "좁아",
            LanguageCodeIR::Korean,
        ),
        (
            "방이 조용하지 않아.",
            "방이 어때?",
            "조용하지 않아",
            LanguageCodeIR::Korean,
        ),
        (
            "The room is quiet.",
            "How is the room?",
            "is quiet",
            LanguageCodeIR::English,
        ),
        (
            "The room was not quiet.",
            "How was the room?",
            "was not quiet",
            LanguageCodeIR::English,
        ),
    ] {
        assert_eq!(
            requested_content_slots(question),
            [ContentSlotIR::Property],
            "{question}"
        );
        let event = described_event(source, false).unwrap();
        assert_eq!(event.predicate_surface, expected);
        let mut api = CognitiveApi::new_embedded().unwrap();
        api.process_conversation_turn(&request("STATE-PROPERTY", 1, source, language))
            .unwrap();
        let q = request("STATE-PROPERTY", 2, question, language);
        crate::generative_language::GENERATION_INVOCATIONS.with(|n| n.set(0));
        crate::natural_realization::FINAL_REALIZATION_CHECKS.with(|n| n.set(0));
        let r = api.process_conversation_turn(&q).unwrap();
        assert_eq!(
            crate::generative_language::GENERATION_INVOCATIONS.with(|n| n.get()),
            1
        );
        assert_eq!(
            crate::natural_realization::FINAL_REALIZATION_CHECKS.with(|n| n.get()),
            1
        );
        assert!(r.validate_against(&q));
        println!("STATE-PROPERTY {question} => {}", r.output.text);
        assert_eq!(
            r.discourse_answer
                .as_ref()
                .and_then(|a| a.content_projection.as_ref())
                .map(|p| p.binding.value.as_str()),
            Some(expected),
            "{}",
            r.output.text
        );
        assert!(r.output.text.contains(expected));
        assert!(!r.output.text.contains("좁아야"));
        assert!(r.conversation_state.action_state_ledger.records.is_empty());
        assert!(!r.language_cortex_integration.external_action_executed);
    }
}

#[test]
fn state_descriptions_preserve_polarity_and_do_not_absorb_scope() {
    use crate::proposition_content::{
        described_event, ContentSlotIR, DescriptionKindIR, PropositionContentIR,
    };
    for source in [
        "조용해.",
        "방이 조용해서",
        "방이 조용하면",
        "방이 조용하다고",
        "방이 조용한",
        "방을 조용하게 해줘.",
        "민수가 ‘방이 조용해’라고 말했다.",
        "If the room is quiet.",
        "Mina says the room is quiet.",
    ] {
        assert!(
            !PropositionContentIR::compile(source)
                .events
                .iter()
                .any(|e| e.kind == DescriptionKindIR::State),
            "{source}"
        );
    }
    let negative = described_event("방이 조용하지 않아.", false).unwrap();
    let positive_query = described_event("뭐가 조용해?", true).unwrap();
    let negative_query = described_event("뭐가 조용하지 않아?", true).unwrap();
    assert!(negative.negated);
    assert!(!negative.matches(&positive_query, ContentSlotIR::Theme));
    assert!(negative.matches(&negative_query, ContentSlotIR::Theme));
    let mut api = CognitiveApi::new_embedded().unwrap();
    api.process_conversation_turn(&request(
        "STATE-POLARITY",
        1,
        "방이 조용하지 않아.",
        LanguageCodeIR::Korean,
    ))
    .unwrap();
    let q = request(
        "STATE-POLARITY",
        2,
        "뭐가 조용하지 않아?",
        LanguageCodeIR::Korean,
    );
    let r = api.process_conversation_turn(&q).unwrap();
    assert!(r.validate_against(&q));
    assert_eq!(
        r.discourse_answer
            .as_ref()
            .and_then(|a| a.content_projection.as_ref())
            .map(|p| p.binding.value.as_str()),
        Some("방")
    );
    assert!(!r.language_cortex_integration.external_action_executed);
    let mut api = CognitiveApi::new_embedded().unwrap();
    api.process_conversation_turn(&request(
        "STATE-ANIMACY",
        1,
        "The parcel is heavy.",
        LanguageCodeIR::English,
    ))
    .unwrap();
    let q = request("STATE-ANIMACY", 2, "Who is heavy?", LanguageCodeIR::English);
    let r = api.process_conversation_turn(&q).unwrap();
    assert!(r.validate_against(&q));
    assert!(r
        .discourse_answer
        .as_ref()
        .is_none_or(|a| a.content_projection.is_none()));
}

#[test]
fn state_bearer_recall_uses_lexical_identity_and_source_evidence() {
    for (statement, question, bearer, language) in [
        (
            "분석의 결과가 이상해.",
            "뭐가 이상하다고 했지?",
            "분석의 결과",
            LanguageCodeIR::Korean,
        ),
        (
            "수리비가 비싸.",
            "무엇이 비싸?",
            "수리비",
            LanguageCodeIR::Korean,
        ),
        (
            "방이 조용해.",
            "뭐가 조용하다고 했지?",
            "방",
            LanguageCodeIR::Korean,
        ),
        (
            "The room is quiet.",
            "What is quiet?",
            "The room",
            LanguageCodeIR::English,
        ),
    ] {
        use crate::proposition_content::{ContentSlotIR, DescriptionKindIR, PropositionContentIR};
        let content = PropositionContentIR::compile(statement);
        assert_eq!(content.events.len(), 1, "{statement}: {content:#?}");
        assert_eq!(content.events[0].kind, DescriptionKindIR::State);
        assert_eq!(
            content.events[0].roles.get(&ContentSlotIR::Theme).unwrap(),
            bearer
        );
        let mut forged = content.clone();
        forged.events[0].kind = DescriptionKindIR::Event;
        assert!(!forged.validate_source(statement));
        let mut api = CognitiveApi::new_embedded().unwrap();
        let first = api
            .process_conversation_turn(&request("STATE-RECALL", 1, statement, language))
            .unwrap();
        assert!(first.grounded_response.is_none());
        let q = request("STATE-RECALL", 2, question, language);
        crate::generative_language::GENERATION_INVOCATIONS.with(|n| n.set(0));
        crate::natural_realization::FINAL_REALIZATION_CHECKS.with(|n| n.set(0));
        let r = api.process_conversation_turn(&q).unwrap();
        assert_eq!(
            crate::generative_language::GENERATION_INVOCATIONS.with(|n| n.get()),
            1
        );
        assert_eq!(
            crate::natural_realization::FINAL_REALIZATION_CHECKS.with(|n| n.get()),
            1
        );
        assert!(r.validate_against(&q));
        println!("STATE-RECALL {statement} / {question} => {}", r.output.text);
        assert_eq!(
            r.discourse_answer
                .as_ref()
                .and_then(|a| a.content_projection.as_ref())
                .map(|p| p.binding.value.as_str()),
            Some(bearer),
            "{question} => {}",
            r.output.text
        );
        assert!(r.conversation_state.action_state_ledger.records.is_empty());
        assert!(!r.language_cortex_integration.external_action_executed);
    }
}

#[test]
fn problem_content_does_not_manufacture_a_missing_request() {
    for (text, language) in [
        ("분석의 결과가 이상해.", LanguageCodeIR::Korean),
        ("분석의 결과가 좋아.", LanguageCodeIR::Korean),
        (
            "The Alder cache is acting up again.",
            LanguageCodeIR::English,
        ),
        ("The Alder cache seems wrong.", LanguageCodeIR::English),
        ("The Alder cache seems fine.", LanguageCodeIR::English),
    ] {
        let mut api = CognitiveApi::new_embedded().unwrap();
        let q = request("REPORT-NOT-REQUEST", 1, text, language);
        crate::generative_language::GENERATION_INVOCATIONS.with(|n| n.set(0));
        crate::natural_realization::FINAL_REALIZATION_CHECKS.with(|n| n.set(0));
        let r = api.process_conversation_turn(&q).unwrap();
        assert_eq!(
            crate::generative_language::GENERATION_INVOCATIONS.with(|n| n.get()),
            1
        );
        assert_eq!(
            crate::natural_realization::FINAL_REALIZATION_CHECKS.with(|n| n.get()),
            1
        );
        assert!(r.validate_against(&q));
        assert!(r.native_language_circuit.unresolved.is_empty(), "{text}");
        assert!(
            r.native_language_circuit.selected_live_goals.is_empty(),
            "{text}"
        );
        assert!(
            !r.conversation_contract.independent_action_requested,
            "{text}"
        );
        assert!(
            r.conversation_state.action_state_ledger.records.is_empty(),
            "{text}"
        );
        assert!(!r.language_cortex_integration.external_action_executed);
    }
    let ambiguous = crate::native_language_circuit::NativeLanguageCircuit.analyze("Inspect it.");
    assert!(!ambiguous.unresolved.is_empty());
    assert_eq!(
        ambiguous.response_goal,
        crate::native_language_circuit::NativeResponseGoalIR::AskClarification
    );
}

#[test]
fn affect_response_selection_shares_quote_and_negation_scope() {
    for (text, language) in [
        ("난 별로 걱정되지 않아.", LanguageCodeIR::Korean),
        ("전혀 답답하지 않아.", LanguageCodeIR::Korean),
        ("I am not very worried.", LanguageCodeIR::English),
        ("I am not frustrated.", LanguageCodeIR::English),
        ("민수가 ‘답답해’라고 말했다.", LanguageCodeIR::Korean),
    ] {
        let mut api = CognitiveApi::new_embedded().unwrap();
        let q = request("AFFECT-SCOPE", 1, text, language);
        let r = api.process_conversation_turn(&q).unwrap();
        assert!(r.validate_against(&q));
        assert_ne!(
            r.natural_realization.response_act,
            NaturalResponseActIR::AffectSupport,
            "{text}: {}",
            r.output.text
        );
    }
    use crate::affective_field::{expressed_affect, ExpressedAffectIR};
    assert_eq!(
        expressed_affect("설명이 없어서 답답해."),
        Some(ExpressedAffectIR::Frustrated)
    );
    assert_eq!(
        expressed_affect("걱정되지는 않아. 하지만 답답해."),
        Some(ExpressedAffectIR::Frustrated)
    );
}

#[test]
fn nominal_state_reports_do_not_become_native_action_requests() {
    for text in [
        "설명이 복잡해서 어려워.",
        "분석이 복잡해.",
        "수정은 어려워.",
        "설명서가 복잡해.",
    ] {
        let mut api = CognitiveApi::new_embedded().unwrap();
        let q = request("NOMINAL-STATE", 1, text, LanguageCodeIR::Korean);
        crate::generative_language::GENERATION_INVOCATIONS.with(|n| n.set(0));
        crate::natural_realization::FINAL_REALIZATION_CHECKS.with(|n| n.set(0));
        let r = api.process_conversation_turn(&q).unwrap();
        println!("NOMINAL-STATE {text} => {}", r.output.text);
        assert_eq!(
            crate::generative_language::GENERATION_INVOCATIONS.with(|n| n.get()),
            1
        );
        assert_eq!(
            crate::natural_realization::FINAL_REALIZATION_CHECKS.with(|n| n.get()),
            1
        );
        assert!(r.validate_against(&q));
        assert!(!r.conversation_contract.information_requested, "{text}");
        assert!(
            !r.conversation_contract.independent_action_requested,
            "{text}"
        );
        assert!(
            r.native_language_circuit.selected_live_goals.is_empty(),
            "{text}"
        );
        assert!(
            !r.conversation_state.epistemic_ledger.records.is_empty(),
            "{text}"
        );
    }
}

#[test]
fn affect_only_response_has_no_unsupported_failure_story() {
    for text in ["방이 더워서 답답해.", "오늘 좀 걱정돼.", "속상해."] {
        let mut api = CognitiveApi::new_embedded().unwrap();
        let q = request("AFFECT-STATE", 1, text, LanguageCodeIR::Korean);
        let r = api.process_conversation_turn(&q).unwrap();
        assert!(r.validate_against(&q));
        assert!(!r.output.text.contains("실패"));
        assert!(!r.output.text.contains("반복"));
        assert!(!r.output.text.contains("확인해 보자"));
    }
}

#[test]
fn nominal_report_memory_and_explicit_request_remain_distinct() {
    let mut api = CognitiveApi::new_embedded().unwrap();
    api.process_conversation_turn(&request(
        "NOMINAL-MEMORY",
        1,
        "설명이 복잡해서 어려워.",
        LanguageCodeIR::Korean,
    ))
    .unwrap();
    let q = request(
        "NOMINAL-MEMORY",
        2,
        "내가 왜 어렵다고 했지?",
        LanguageCodeIR::Korean,
    );
    let r = api.process_conversation_turn(&q).unwrap();
    assert!(r.validate_against(&q));
    let p = r
        .discourse_answer
        .as_ref()
        .and_then(|a| a.content_projection.as_ref())
        .unwrap();
    assert_eq!(p.binding.value, "설명이 복잡해서");
    for text in ["결과를 설명해줘.", "설명은 해줘."] {
        let mut api = CognitiveApi::new_embedded().unwrap();
        let q = request("EXPLICIT-REQUEST", 1, text, LanguageCodeIR::Korean);
        let r = api.process_conversation_turn(&q).unwrap();
        assert!(r.validate_against(&q));
        assert!(r.conversation_contract.information_requested, "{text}");
    }
}

#[test]
fn stated_property_reason_recall_uses_lexical_identity_and_source_memory() {
    for (source, question, cause) in [
        (
            "오늘 일이 많아서 좀 피곤해.",
            "내가 왜 피곤하다고 했지?",
            "오늘 일이 많아서",
        ),
        (
            "방이 좁아서 불편해.",
            "내가 왜 불편하다고 했지?",
            "방이 좁아서",
        ),
        (
            "짐이 무거워서 힘들어.",
            "제가 왜 힘들다고 말했죠?",
            "짐이 무거워서",
        ),
    ] {
        let mut api = CognitiveApi::new_embedded().unwrap();
        api.process_conversation_turn(&request(
            "STATIVE-RECALL",
            1,
            source,
            LanguageCodeIR::Korean,
        ))
        .unwrap();
        api.process_conversation_turn(&request(
            "STATIVE-RECALL",
            2,
            "조언보다는 내 얘기를 들어줬으면 해.",
            LanguageCodeIR::Korean,
        ))
        .unwrap();
        let q = request("STATIVE-RECALL", 3, question, LanguageCodeIR::Korean);
        crate::generative_language::GENERATION_INVOCATIONS.with(|n| n.set(0));
        crate::natural_realization::FINAL_REALIZATION_CHECKS.with(|n| n.set(0));
        let r = api.process_conversation_turn(&q).unwrap();
        println!("STATIVE-RECALL {source} / {question} => {}", r.output.text);
        assert_eq!(
            crate::generative_language::GENERATION_INVOCATIONS.with(|n| n.get()),
            1
        );
        assert_eq!(
            crate::natural_realization::FINAL_REALIZATION_CHECKS.with(|n| n.get()),
            1
        );
        assert!(r.validate_against(&q));
        let a = r.discourse_answer.as_ref().unwrap();
        let p = a
            .content_projection
            .as_ref()
            .expect("source-bound cause projection");
        assert_eq!(p.binding.value, cause);
        assert_eq!(p.source_actor, "DIALOGUE_USER");
        assert!(!a.dialogue_truth_established);
        assert!(!a.external_execution_authorized);
    }
    for source in ["도서관에 가서 책을 읽었어.", "일이 많으면 피곤해."] {
        let c = crate::proposition_content::PropositionContentIR::compile(source);
        println!("CAUSAL-CONTROL {source}: {:?}", c.bindings);
        // Action sequence and condition never become a stated causal binding.
        assert!(!c
            .bindings
            .iter()
            .any(|b| b.slot == crate::proposition_content::ContentSlotIR::Cause));
    }
}

#[test]
fn recalled_reason_does_not_borrow_old_or_other_property_causes() {
    for latest in ["오늘은 피곤해.", "방이 좁아서 불편해."] {
        let mut api = CognitiveApi::new_embedded().unwrap();
        let prior = if latest.contains("오늘") {
            "일이 많아서 피곤해."
        } else {
            "안녕."
        };
        api.process_conversation_turn(&request("CAUSE-GAP", 1, prior, LanguageCodeIR::Korean))
            .unwrap();
        api.process_conversation_turn(&request("CAUSE-GAP", 2, latest, LanguageCodeIR::Korean))
            .unwrap();
        let q = request(
            "CAUSE-GAP",
            3,
            "내가 왜 피곤하다고 했지?",
            LanguageCodeIR::Korean,
        );
        let r = api.process_conversation_turn(&q).unwrap();
        assert!(r.validate_against(&q));
        println!(
            "CAUSE-GAP {latest} => {}\nrecords={:?}\nanswer={:?}",
            r.output.text, r.conversation_state.epistemic_ledger.records, r.discourse_answer
        );
        assert!(r
            .discourse_answer
            .as_ref()
            .is_none_or(|a| a.content_projection.is_none()));
        assert!(!r.output.text.contains("일이 많아서"));
    }
}

#[test]
fn desiderative_complement_reaches_dialogue_preference_without_fake_condition() {
    for text in [
        "조언보다는 내 얘기를 들어줬으면 해.",
        "해결책보다 내 이야기를 들어주었으면 해요.",
        "내 얘기를 들어줬으면 좋겠어.",
    ] {
        let mut api = CognitiveApi::new_embedded().unwrap();
        let input = request("WISH-SCOPE", 1, text, LanguageCodeIR::Korean);
        crate::generative_language::GENERATION_INVOCATIONS.with(|n| n.set(0));
        crate::natural_realization::FINAL_REALIZATION_CHECKS.with(|n| n.set(0));
        let r = api.process_conversation_turn(&input).unwrap();
        println!("WISH-SCOPE {text} => {}", r.output.text);
        assert_eq!(
            crate::generative_language::GENERATION_INVOCATIONS.with(|n| n.get()),
            1
        );
        assert_eq!(
            crate::natural_realization::FINAL_REALIZATION_CHECKS.with(|n| n.get()),
            1
        );
        assert!(r.validate_against(&input));
        assert_ne!(
            r.natural_realization.response_act,
            NaturalResponseActIR::ConditionalGuard
        );
        assert!(r.grounded_response.is_none());
        assert!(!r.language_cortex_integration.external_action_executed);
        let p = crate::proposition_content::interaction_preference(text).unwrap();
        assert_eq!(
            p.desired,
            crate::proposition_content::InteractionModeIR::Listening
        );
        assert!(p.owns_response());
    }
}

#[test]
fn recalled_desiderative_clause_uses_a_quotative_ending_instead_of_a_noun_copula() {
    for (source, question) in [
        (
            "오늘은 조언보다 내 얘기를 들어주는 사람이 있었으면 좋겠네.",
            "내가 지금 바라는 게 뭐야?",
        ),
        (
            "해결책보다 내 이야기를 들어주었으면 해요.",
            "내가 원하는 건 뭐야?",
        ),
    ] {
        let mut api = CognitiveApi::new_embedded().unwrap();
        api.process_conversation_turn(&request(
            "WISH-RECALL-REALIZATION",
            1,
            source,
            LanguageCodeIR::Korean,
        ))
        .unwrap();
        let input = request(
            "WISH-RECALL-REALIZATION",
            2,
            question,
            LanguageCodeIR::Korean,
        );
        let response = api.process_conversation_turn(&input).unwrap();
        assert!(response.validate_against(&input));
        assert!(
            response.output.text.ends_with("’라는 거야.")
                || response.output.text.ends_with("’라는 것입니다."),
            "{}",
            response.output.text
        );
        assert!(!response.output.text.ends_with("’야."));
        if source.ends_with("해요.") {
            assert!(
                response
                    .output
                    .text
                    .starts_with("말씀하신 내용에 따르면, 원하시는 것은"),
                "{}",
                response.output.text
            );
        } else {
            assert!(
                response
                    .output
                    .text
                    .starts_with("네 말에 따르면, 네가 원하는 건"),
                "{}",
                response.output.text
            );
        }
        assert!(response
            .natural_realization
            .generation_traces
            .iter()
            .flat_map(|trace| &trace.meaning.nodes)
            .any(|node| node.concept_id == "C_CONTENT_EMBEDDED_CLAUSE"));
    }
}

#[test]
fn preference_choice_answers_the_question_focus_without_requoting_the_source() {
    for (source, question, expected) in [
        (
            "오늘은 조언보다 내 얘기를 들어주는 사람이 있었으면 좋겠네.",
            "내가 지금 바라는 건 해결책일까, 들어주는 걸까?",
            "해결책보다는 네 이야기를 들어주는 쪽이야.",
        ),
        (
            "해결책보다 제 이야기를 들어주었으면 해요.",
            "제가 원하는 건 해결책일까요, 들어주는 걸까요?",
            "해결책보다는 말씀을 들어드리는 쪽입니다.",
        ),
        (
            "조언은 필요 없어, 그냥 대화하고 싶어.",
            "내가 원하는 건 해결책일까, 대화일까?",
            "해결책보다는 같이 이야기하는 쪽이야.",
        ),
    ] {
        let mut api = CognitiveApi::new_embedded().unwrap();
        api.process_conversation_turn(&request(
            "PREFERENCE-CHOICE-FOCUS",
            1,
            source,
            LanguageCodeIR::Korean,
        ))
        .unwrap();
        let input = request(
            "PREFERENCE-CHOICE-FOCUS",
            2,
            question,
            LanguageCodeIR::Korean,
        );
        let response = api.process_conversation_turn(&input).unwrap();
        assert!(response.validate_against(&input), "{response:#?}");
        assert_eq!(response.output.text, expected);
        assert!(!response.output.text.contains('‘'));
        let trace = &response.natural_realization.generation_traces[0];
        assert!(trace
            .meaning
            .nodes
            .iter()
            .any(|node| node.concept_id == "C_INTERACTION_PREFERENCE_ANSWER"));
    }
}

#[test]
fn reported_request_target_recalls_the_typed_response_preference() {
    let mut api = CognitiveApi::new_embedded().unwrap();
    api.process_conversation_turn(&request(
        "RESPONSE-PREFERENCE-RECALL",
        1,
        "내가 원하는 건 짧은 대답이야. 길게 계획을 늘어놓으면 더 헷갈려.",
        LanguageCodeIR::Korean,
    ))
    .unwrap();
    let input = request(
        "RESPONSE-PREFERENCE-RECALL",
        2,
        "그러니까 지금은 뭘 줄여 달라는 거야?",
        LanguageCodeIR::Korean,
    );
    let response = api.process_conversation_turn(&input).unwrap();
    assert!(response.validate_against(&input));
    assert_eq!(response.output.text, "답변 길이야.");
    assert!(!response.output.text.contains("아직 모르겠어"));
    let projection = response
        .discourse_answer
        .as_ref()
        .and_then(|answer| answer.content_projection.as_ref())
        .expect("typed response-preference projection");
    assert_eq!(
        projection.binding.slot,
        crate::proposition_content::ContentSlotIR::Intention
    );
    assert_eq!(projection.binding.value, "짧은 대답");
    assert_eq!(projection.binding.grammar_evidence, "DESIDERATIVE_NOMINAL");
}

#[test]
fn multiple_answer_roles_compose_one_grounded_event_clause() {
    for (language, source, question, expected, omitted) in [
        (
            LanguageCodeIR::Korean,
            "규리가 어제 강당에서 일기를 안 읽었어.",
            "누가 뭘 안 읽었어?",
            "규리가 일기를 안 읽었어",
            ["어제", "강당"],
        ),
        (
            LanguageCodeIR::English,
            "Galen did not read a note in the studio yesterday.",
            "Who did not read what?",
            "Galen did not read a note",
            ["studio", "yesterday"],
        ),
        (
            LanguageCodeIR::Korean,
            "서윤이가 어제 도서관에서 우산을 잃어버렸어.",
            "누가 뭘 잃어버렸지?",
            "서윤이가 우산을 잃어버렸어",
            ["어제", "도서관"],
        ),
        (
            LanguageCodeIR::Korean,
            "태오가 오늘 강의실에서 편지를 썼어.",
            "누가 뭘 썼지?",
            "태오가 편지를 썼어",
            ["오늘", "강의실"],
        ),
        (
            LanguageCodeIR::English,
            "Mina read a book in the library yesterday.",
            "Who read what?",
            "Mina read a book",
            ["library", "yesterday"],
        ),
        (
            LanguageCodeIR::English,
            "Nora wrote a letter at the station today.",
            "Who wrote what?",
            "Nora wrote a letter",
            ["station", "today"],
        ),
    ] {
        let mut api = CognitiveApi::new_embedded().unwrap();
        api.process_conversation_turn(&request("ROLE-CLAUSE", 1, source, language))
            .unwrap();
        let input = request("ROLE-CLAUSE", 2, question, language);
        crate::generative_language::GENERATION_INVOCATIONS.with(|n| n.set(0));
        crate::natural_realization::FINAL_REALIZATION_CHECKS.with(|n| n.set(0));
        let r = api.process_conversation_turn(&input).unwrap();
        println!("ROLE-CLAUSE {question} => {}", r.output.text);
        assert_eq!(
            crate::generative_language::GENERATION_INVOCATIONS.with(|n| n.get()),
            1
        );
        assert_eq!(
            crate::natural_realization::FINAL_REALIZATION_CHECKS.with(|n| n.get()),
            1
        );
        assert!(r.validate_against(&input));
        assert!(r.output.text.contains(expected), "{}", r.output.text);
        for value in omitted {
            assert!(!r.output.text.contains(value));
        }
        assert_eq!(r.natural_realization.generation_traces.len(), 1);
        let trace = &r.natural_realization.generation_traces[0];
        assert_eq!(
            trace
                .meaning
                .nodes
                .iter()
                .filter(|n| n.concept_id.starts_with("C_EVENT_RECAP_"))
                .count(),
            1
        );
        assert!(!trace
            .meaning
            .nodes
            .iter()
            .any(|n| n.concept_id.starts_with("C_RECAP_ROLE_Location")
                || n.concept_id.starts_with("C_RECAP_ROLE_Time")));
        assert!(!r.language_cortex_integration.external_action_executed);
    }
}

#[test]
fn recollection_morphology_reaches_memory_instead_of_recommendation() {
    for (statement, question, value) in [
        (
            "서윤이가 어제 도서관에서 우산을 잃어버렸어.",
            "뭘 잃어버렸지?",
            "우산",
        ),
        ("나래가 어제 시장에서 사과를 샀어.", "뭘 샀죠?", "사과"),
        ("태오가 어제 교실에서 편지를 썼어.", "뭘 썼지?", "편지"),
    ] {
        let mut api = CognitiveApi::new_embedded().unwrap();
        api.process_conversation_turn(&request(
            "RECOLLECTION",
            1,
            statement,
            LanguageCodeIR::Korean,
        ))
        .unwrap();
        let input = request("RECOLLECTION", 2, question, LanguageCodeIR::Korean);
        crate::generative_language::GENERATION_INVOCATIONS.with(|n| n.set(0));
        crate::natural_realization::FINAL_REALIZATION_CHECKS.with(|n| n.set(0));
        let r = api.process_conversation_turn(&input).unwrap();
        println!("RECOLLECTION {question} => {}", r.output.text);
        let a = r.discourse_answer.as_ref().unwrap();
        assert!(a.decision_inquiry.is_none());
        assert_eq!(
            a.content_projection.as_ref().expect(question).binding.value,
            value
        );
        assert!(r.output.text.contains(value));
        assert!(r.grounded_response.is_none());
        assert!(!r.language_cortex_integration.external_action_executed);
        assert_eq!(
            crate::generative_language::GENERATION_INVOCATIONS.with(|n| n.get()),
            1
        );
        assert_eq!(
            crate::natural_realization::FINAL_REALIZATION_CHECKS.with(|n| n.get()),
            1
        );
        // Check the actual request path before the test independently replays
        // response validation; that replay is not a second runtime check.
        assert!(r.validate_against(&input));
        let follow = request("RECOLLECTION", 3, "어디서?", LanguageCodeIR::Korean);
        let r = api.process_conversation_turn(&follow).unwrap();
        assert!(r.validate_against(&follow));
        assert!(r
            .discourse_answer
            .as_ref()
            .unwrap()
            .content_projection
            .is_some());
    }
}

#[test]
fn recent_dialogue_act_bridges_social_turns_but_not_cancellation() {
    for (language, turns) in [
        (
            LanguageCodeIR::Korean,
            ["피곤해?", "말하기 싫어", "고마워", "왜?", "취소", "왜?"],
        ),
        (
            LanguageCodeIR::English,
            [
                "Tired?",
                "I'm not sure",
                "Thanks!",
                "Why?",
                "Cancel",
                "Why?",
            ],
        ),
    ] {
        let mut api = CognitiveApi::new_embedded().unwrap();
        for (i, text) in turns.into_iter().enumerate() {
            let input = request("ACT-BRIDGE", i as u64 + 1, text, language);
            let r = api
                .process_conversation_turn(&input)
                .unwrap_or_else(|e| panic!("{text}: {e:?}"));
            assert!(r.validate_against(&input));
            if i == 3 {
                assert_eq!(
                    r.conversation_state
                        .answer_focus
                        .as_ref()
                        .unwrap()
                        .clarification_act
                        .as_ref()
                        .unwrap()
                        .kind,
                    crate::world_dialogue::WorldClarificationActKindIR::ExplainChoice
                );
            }
            if i >= 4 {
                assert!(r
                    .conversation_state
                    .dialogue_world
                    .pending_reference
                    .is_none());
                assert!(r
                    .conversation_state
                    .answer_focus
                    .as_ref()
                    .is_none_or(|f| f.clarification_act.is_none()));
            }
        }
    }
}

#[test]
fn recent_dialogue_act_owns_reason_and_restatement_without_answer_cache() {
    use crate::world_dialogue::WorldClarificationActKindIR::*;
    for (language, turns) in [
        (
            LanguageCodeIR::Korean,
            [
                "피곤해?",
                "다시 말해줘",
                "모르겠어",
                "왜?",
                "다시 말해줘",
                "왜 물어?",
                "하린",
            ],
        ),
        (
            LanguageCodeIR::English,
            [
                "Tired?",
                "Repeat that.",
                "I'd rather not say",
                "Why?",
                "Repeat that.",
                "Why do you ask?",
                "Qevran",
            ],
        ),
    ] {
        let mut api = CognitiveApi::new_embedded().unwrap();
        let expected = [
            RequestIdentity,
            RequestIdentity,
            AllowAbstention,
            ExplainChoice,
            ExplainChoice,
            ExplainRequirement,
        ];
        for (i, text) in turns.into_iter().enumerate() {
            let input = request("ACT-FOCUS", i as u64 + 1, text, language);
            crate::generative_language::GENERATION_INVOCATIONS.with(|n| n.set(0));
            crate::natural_realization::FINAL_REALIZATION_CHECKS.with(|n| n.set(0));
            let r = api
                .process_conversation_turn(&input)
                .unwrap_or_else(|e| panic!("{text}: {e:?}"));
            println!("ACT-FOCUS {text} => {}", r.output.text);
            assert_eq!(
                crate::generative_language::GENERATION_INVOCATIONS.with(|n| n.get()),
                1
            );
            assert_eq!(
                crate::natural_realization::FINAL_REALIZATION_CHECKS.with(|n| n.get()),
                1
            );
            assert!(r.validate_against(&input));
            assert!(r.conversation_state.dialogue_world.premises.is_empty());
            assert!(!r.language_cortex_integration.external_action_executed);
            if i < expected.len() {
                let c = r
                    .discourse_answer
                    .as_ref()
                    .unwrap()
                    .world_clarification
                    .as_ref()
                    .expect(text);
                assert_eq!(c.response_act().kind, expected[i], "{text}");
                assert_eq!(
                    r.conversation_state
                        .answer_focus
                        .as_ref()
                        .unwrap()
                        .clarification_act,
                    Some(c.response_act())
                );
                let replay: ConversationTurnResponseIR =
                    serde_json::from_str(&serde_json::to_string(&r).unwrap()).unwrap();
                assert!(replay.validate_against(&input));
                let mut forged = c.clone();
                if let Some(anchor) = forged.followup.as_mut().and_then(|f| f.anchor.as_mut()) {
                    anchor.gap_identity = "other gap".into();
                    assert!(!forged.validate());
                }
                if i == 3 || i == 4 {
                    assert!(r.natural_realization.generation_traces[0]
                        .meaning
                        .nodes
                        .iter()
                        .any(|n| n.concept_id == "C_WORLD_CLAUSE_REFERENCE_RESPONSE_CHOICE"));
                }
            } else {
                assert!(r
                    .conversation_state
                    .dialogue_world
                    .pending_reference
                    .is_none());
                assert!(r.conversation_state.answer_focus.is_none());
                assert!(r
                    .discourse_answer
                    .as_ref()
                    .unwrap()
                    .world_reasoning
                    .is_some());
            }
        }
    }
}

#[test]
fn clarification_abstention_preserves_meaning_and_does_not_repeat_the_question() {
    use crate::world_dialogue::WorldClarificationFollowupKindIR::{DeclinedAnswer, UnknownAnswer};
    for (language, question, replies, name) in [
        (
            LanguageCodeIR::Korean,
            "피곤해?",
            [
                ("나도 잘 모르겠어", UnknownAnswer),
                ("말하기 싫어", DeclinedAnswer),
            ],
            "하린",
        ),
        (
            LanguageCodeIR::English,
            "Tired?",
            [
                ("I don't know", UnknownAnswer),
                ("I'd rather not say", DeclinedAnswer),
            ],
            "Qevran",
        ),
    ] {
        let mut api = CognitiveApi::new_embedded().unwrap();
        let first = api
            .process_conversation_turn(&request("ABSTENTION", 1, question, language))
            .unwrap();
        let before = first.conversation_state.dialogue_world.clone();
        for (i, (text, kind)) in replies.into_iter().enumerate() {
            let input = request("ABSTENTION", i as u64 + 2, text, language);
            crate::generative_language::GENERATION_INVOCATIONS.with(|n| n.set(0));
            crate::natural_realization::FINAL_REALIZATION_CHECKS.with(|n| n.set(0));
            let response = api.process_conversation_turn(&input).unwrap();
            println!("ABSTENTION {text} => {}", response.output.text);
            assert_eq!(
                crate::generative_language::GENERATION_INVOCATIONS.with(|n| n.get()),
                1
            );
            assert_eq!(
                crate::natural_realization::FINAL_REALIZATION_CHECKS.with(|n| n.get()),
                1
            );
            assert!(response.validate_against(&input));
            assert_eq!(response.conversation_state.dialogue_world, before);
            let c = response
                .discourse_answer
                .as_ref()
                .unwrap()
                .world_clarification
                .as_ref()
                .unwrap();
            assert_eq!(c.followup.as_ref().unwrap().kind, kind);
            assert!(c.memory.premises.is_empty());
            assert!(!response.output.text.contains('?'));
            assert!(response.grounded_response.is_none());
            let mut forged = c.clone();
            forged.followup.as_mut().unwrap().kind = if kind == UnknownAnswer {
                DeclinedAnswer
            } else {
                UnknownAnswer
            };
            assert!(!forged.validate());
            let restored: crate::world_dialogue::WorldClarificationIR =
                serde_json::from_str(&serde_json::to_string(c).unwrap()).unwrap();
            assert!(restored.validate());
        }
        let resumed = api
            .process_conversation_turn(&request("ABSTENTION", 4, name, language))
            .unwrap();
        let world = resumed
            .discourse_answer
            .as_ref()
            .unwrap()
            .world_reasoning
            .as_ref()
            .unwrap();
        assert_eq!(world.query.target.0.entity, name.to_lowercase());
        assert_eq!(
            world.decision.verdict,
            crate::world_dialogue::WorldVerdictIR::Unknown
        );
        assert!(world.memory.premises.is_empty());
        assert!(before
            .prepare("cancel", 4)
            .unwrap()
            .memory
            .pending_reference
            .is_none());
    }
}

#[test]
fn open_identity_slots_accept_nominal_replies_without_control_or_truth_leakage() {
    for (language, question, name) in [
        (LanguageCodeIR::Korean, "피곤해?", "민수"),
        (LanguageCodeIR::Korean, "피곤해?", "장미"),
        (LanguageCodeIR::English, "Tired?", "Lyra"),
        (LanguageCodeIR::English, "Tired?", "Qevran"),
    ] {
        let mut api = CognitiveApi::new_embedded().unwrap();
        let original = api
            .process_conversation_turn(&request("BARE-IDENTITY", 1, question, language))
            .unwrap();
        let pending = original.conversation_state.dialogue_world;
        let argument = pending
            .pending_reference
            .as_ref()
            .unwrap()
            .argument
            .as_ref()
            .unwrap();
        for control in [
            "cancel",
            "stop",
            "취소해",
            "모르겠어",
            "몰라",
            "고마워",
            "알겠어",
            "음",
            "yes",
            "아니야",
            "취소",
            "철회",
            "중지",
            "중단",
            "시작",
            "진행",
        ] {
            let rejected = pending.prepare(control, 2).unwrap();
            assert!(rejected.query.is_none(), "{control}");
            assert!(rejected.memory.premises.is_empty(), "{control}");
        }
        let input = request("BARE-IDENTITY", 2, name, language);
        crate::generative_language::GENERATION_INVOCATIONS.with(|n| n.set(0));
        crate::natural_realization::FINAL_REALIZATION_CHECKS.with(|n| n.set(0));
        let output = api.process_conversation_turn(&input).unwrap();
        println!("BARE_IDENTITY {name} => {}", output.output.text);
        assert_eq!(
            crate::generative_language::GENERATION_INVOCATIONS.with(|n| n.get()),
            1
        );
        assert_eq!(
            crate::natural_realization::FINAL_REALIZATION_CHECKS.with(|n| n.get()),
            1
        );
        assert!(output.validate_against(&input));
        assert!(output.conversation_state.dialogue_world.premises.is_empty());
        assert!(output
            .conversation_state
            .dialogue_world
            .pending_reference
            .is_none());
        let world = output
            .discourse_answer
            .as_ref()
            .unwrap()
            .world_reasoning
            .as_ref()
            .unwrap();
        assert_eq!(world.query.target.0.entity, name.to_lowercase());
        assert_eq!(world.query.target.0.property, argument.property);
        assert_eq!(world.query.target.1, argument.positive);
        assert_eq!(
            world.decision.verdict,
            crate::world_dialogue::WorldVerdictIR::Unknown
        );
        let restored: crate::world_dialogue::DialogueWorldIR = serde_json::from_str(
            &serde_json::to_string(&output.conversation_state.dialogue_world).unwrap(),
        )
        .unwrap();
        assert!(restored.validate(2));
        let quoted = pending.prepare("\"stop\"", 2).unwrap();
        assert_eq!(quoted.query.unwrap().target.0.entity, "stop");
        assert!(quoted.memory.premises.is_empty());
    }
}

#[test]
fn pending_clarification_reason_keeps_original_query_and_single_realization() {
    for (language, question, reasons, reply) in [
        (
            LanguageCodeIR::Korean,
            "피곤해?",
            ["왜 물어?", "왜 그걸 묻는 거야?"],
            "나야",
        ),
        (
            LanguageCodeIR::English,
            "Tired?",
            ["Why do you ask?", "Why are you asking?"],
            "me",
        ),
    ] {
        let mut api = CognitiveApi::new_embedded().unwrap();
        let first = api
            .process_conversation_turn(&request("CLARIFICATION-REASON", 1, question, language))
            .unwrap();
        let gap = first
            .conversation_state
            .dialogue_world
            .pending_reference
            .clone();
        for (i, text) in reasons.into_iter().chain([reply]).enumerate() {
            let input = request("CLARIFICATION-REASON", i as u64 + 2, text, language);
            crate::generative_language::GENERATION_INVOCATIONS.with(|n| n.set(0));
            crate::natural_realization::FINAL_REALIZATION_CHECKS.with(|n| n.set(0));
            let output = api.process_conversation_turn(&input).unwrap();
            println!("CLARIFICATION_REASON {text} => {}", output.output.text);
            assert_eq!(
                crate::generative_language::GENERATION_INVOCATIONS.with(|n| n.get()),
                1
            );
            assert_eq!(
                crate::natural_realization::FINAL_REALIZATION_CHECKS.with(|n| n.get()),
                1
            );
            assert!(output.validate_against(&input));
            assert!(output.conversation_state.dialogue_world.premises.is_empty());
            let answer = output.discourse_answer.as_ref().unwrap();
            if i < 2 {
                assert_eq!(
                    output.conversation_state.dialogue_world.pending_reference,
                    gap
                );
                assert!(output
                    .conversation_state
                    .dialogue_world
                    .last_query
                    .is_none());
                let c = answer.world_clarification.as_ref().unwrap();
                assert_eq!(c.response_source(), text);
                let mut forged = c.clone();
                forged.followup.as_mut().unwrap().source_text = "Why is alpha tired?".into();
                assert!(!forged.validate());
                forged = c.clone();
                forged.followup.as_mut().unwrap().turn = 1;
                assert!(!forged.validate());
            } else {
                assert!(output
                    .conversation_state
                    .dialogue_world
                    .pending_reference
                    .is_none());
                assert_eq!(
                    answer
                        .world_reasoning
                        .as_ref()
                        .unwrap()
                        .query
                        .target
                        .0
                        .entity,
                    "__user__"
                );
            }
        }
    }
}

#[test]
fn explanation_followup_keeps_world_meaning_and_current_question_ownership() {
    for (language, texts) in [
        (
            LanguageCodeIR::Korean,
            [
                "나 피곤해.",
                "그럼 어떻게 하지?",
                "왜?",
                "설명해줘.",
                "피곤해?",
            ],
        ),
        (
            LanguageCodeIR::English,
            [
                "I am tired.",
                "What should I do?",
                "Why?",
                "Please explain that.",
                "Tired?",
            ],
        ),
    ] {
        let mut api = CognitiveApi::new_embedded().unwrap();
        for (i, text) in texts.iter().enumerate() {
            let input = request("EXPLANATION-FOCUS", i as u64 + 1, text, language);
            crate::generative_language::GENERATION_INVOCATIONS.with(|n| n.set(0));
            crate::natural_realization::FINAL_REALIZATION_CHECKS.with(|n| n.set(0));
            let response = api.process_conversation_turn(&input).unwrap();
            println!("EXPLANATION_FOCUS {text} => {}", response.output.text);
            assert_eq!(
                crate::generative_language::GENERATION_INVOCATIONS.with(|n| n.get()),
                1
            );
            assert_eq!(
                crate::natural_realization::FINAL_REALIZATION_CHECKS.with(|n| n.get()),
                1
            );
            assert!(response.validate_against(&input));
            assert!(response.grounded_response.is_none());
            if i >= 2 {
                let world = response
                    .discourse_answer
                    .as_ref()
                    .unwrap()
                    .world_reasoning
                    .as_ref()
                    .expect("focused world question");
                assert_eq!(world.query.target.0.entity, "__user__");
                assert_eq!(
                    world.query.target.0.property,
                    crate::world_dialogue::WorldPropertyIR::Registered("W_USER_900001".into())
                );
                assert_eq!(world.query.explain, i != 4);
            }
        }
    }
}

#[test]
fn unanswered_new_content_never_inherits_previous_operation_as_topic() {
    for (language, setup, question) in [
        (LanguageCodeIR::Korean, "설명해줘.", "피곤해?"),
        (LanguageCodeIR::Korean, "설명해줘.", "답답해?"),
        (LanguageCodeIR::English, "Please explain.", "Tired?"),
        (
            LanguageCodeIR::English,
            "Please explain.",
            "What is entropy?",
        ),
        (
            LanguageCodeIR::English,
            "Please explain.",
            "Too verbose. What is entropy?",
        ),
    ] {
        let mut api = CognitiveApi::new_embedded().unwrap();
        api.process_conversation_turn(&request("NEW-QUESTION", 1, setup, language))
            .unwrap();
        let input = request("NEW-QUESTION", 2, question, language);
        let response = api.process_conversation_turn(&input).unwrap();
        println!("NEW_TOPIC {question} => {}", response.output.text);
        assert!(response.validate_against(&input));
        let answer = response.discourse_answer.as_ref().unwrap();
        if let Some(clarification) = &answer.world_clarification {
            assert_eq!(
                answer.disposition,
                crate::discourse_qa::DiscourseAnswerDispositionIR::AmbiguousQuery
            );
            assert!(clarification.gap.argument.is_some());
        } else {
            assert_eq!(
                answer.disposition,
                crate::discourse_qa::DiscourseAnswerDispositionIR::NoMatchingRecord
            );
            assert!(!answer.query.topic_terms.is_empty());
        }
        assert!(!answer
            .query
            .topic_terms
            .iter()
            .any(|t| t == "explain" || t == "설명"));
        assert!(
            answer.world_reasoning.is_none(),
            "do not guess an omitted experiencer"
        );
    }
}

#[test]
fn structured_generation_invocation_audit() {
    for (name, language, setup, question) in [
        (
            "RECAP",
            LanguageCodeIR::English,
            vec!["Nico read a letter in the studio."],
            "Please summarize that.",
        ),
        (
            "INFORMATION_GAP",
            LanguageCodeIR::Korean,
            vec![],
            "설명해줘.",
        ),
        (
            "DECISION_GAP",
            LanguageCodeIR::Korean,
            vec!["나 피곤해."],
            "그럼 어떻게 하지?",
        ),
        ("WORLD_UPDATE", LanguageCodeIR::Korean, vec![], "나 피곤해."),
        (
            "WORLD_REASON",
            LanguageCodeIR::English,
            vec!["I am tired."],
            "Tired?",
        ),
    ] {
        let mut api = CognitiveApi::new_embedded().unwrap();
        for (i, text) in setup.iter().enumerate() {
            api.process_conversation_turn(&request(
                "STRUCTURED-AUDIT",
                i as u64 + 1,
                text,
                language,
            ))
            .unwrap();
        }
        let input = request(
            "STRUCTURED-AUDIT",
            setup.len() as u64 + 1,
            question,
            language,
        );
        crate::generative_language::GENERATION_INVOCATIONS.with(|n| n.set(0));
        crate::natural_realization::FINAL_REALIZATION_CHECKS.with(|n| n.set(0));
        let response = api.process_conversation_turn(&input).unwrap();
        let calls = crate::generative_language::GENERATION_INVOCATIONS.with(|n| n.get());
        println!(
            "STRUCTURED_GENERATION_AUDIT {name} calls={calls} output={}",
            response.output.text
        );
        assert_eq!(calls, 1, "{name}");
        assert_eq!(
            crate::natural_realization::FINAL_REALIZATION_CHECKS.with(|n| n.get()),
            1
        );
        assert!(response.validate_against(&input));
        let answer = response.discourse_answer.as_ref().unwrap();
        let decoded: crate::discourse_qa::DiscourseAnswerIR =
            serde_json::from_str(&serde_json::to_string(answer).unwrap()).unwrap();
        crate::generative_language::GENERATION_INVOCATIONS.with(|n| n.set(0));
        assert!(decoded.validate());
        assert_eq!(
            crate::generative_language::GENERATION_INVOCATIONS.with(|n| n.get()),
            0
        );
        let mut forged = decoded.clone();
        forged.realized_text.push_str(" invented");
        assert!(!forged.validate());
        let mut forged = decoded;
        forged.query.original_text.push_str(" invented");
        assert!(!forged.validate());
        assert!(
            match name {
                "RECAP" => answer.event_summary.is_some(),
                "INFORMATION_GAP" =>
                    answer.query.kind
                        == crate::discourse_qa::DiscourseQueryKindIR::MissingExplanationTarget,
                "DECISION_GAP" => answer.decision_inquiry.is_some(),
                "WORLD_UPDATE" => answer.world_memory_update.is_some(),
                _ => answer.world_reasoning.is_some(),
            },
            "{name}"
        );
    }
}

#[test]
fn open_referent_clarification_then_identity_then_truth_are_separate_turns() {
    for (language, texts) in [
        (
            LanguageCodeIR::Korean,
            ["피곤해?", "나야", "나 피곤해.", "피곤해?"],
        ),
        (
            LanguageCodeIR::English,
            ["Tired?", "me", "I am tired.", "Tired?"],
        ),
    ] {
        let mut api = CognitiveApi::new_embedded().unwrap();
        for (i, text) in texts.iter().enumerate() {
            let input = request("OPEN-REFERENT", i as u64 + 1, text, language);
            crate::generative_language::GENERATION_INVOCATIONS.with(|n| n.set(0));
            crate::natural_realization::FINAL_REALIZATION_CHECKS.with(|n| n.set(0));
            let output = api.process_conversation_turn(&input).unwrap();
            println!("OPEN_REFERENT {text} => {}", output.output.text);
            assert_eq!(
                crate::generative_language::GENERATION_INVOCATIONS.with(|n| n.get()),
                1
            );
            assert_eq!(
                crate::natural_realization::FINAL_REALIZATION_CHECKS.with(|n| n.get()),
                1
            );
            assert!(output.validate_against(&input));
            assert!(output.grounded_response.is_none());
            let answer = output.discourse_answer.as_ref().unwrap();
            if i == 0 {
                assert!(answer
                    .world_clarification
                    .as_ref()
                    .unwrap()
                    .gap
                    .argument
                    .is_some());
                assert!(output
                    .conversation_state
                    .dialogue_world
                    .last_query
                    .is_none());
            } else if i == 2 {
                assert!(answer.world_memory_update.is_some());
            } else {
                let world = answer.world_reasoning.as_ref().unwrap();
                assert_eq!(world.query.target.0.entity, "__user__");
                assert_eq!(
                    world.decision.verdict,
                    if i == 1 {
                        crate::world_dialogue::WorldVerdictIR::Unknown
                    } else {
                        crate::world_dialogue::WorldVerdictIR::Supported
                    }
                );
            }
            assert_eq!(
                output.conversation_state.dialogue_world.premises.len(),
                usize::from(i >= 2)
            );
            assert!(!output.output.text.contains("__open_argument_probe__"));
        }
    }
}

#[test]
fn discourse_generation_invocation_audit() {
    for (name, question) in [
        ("FOCUSED", "누가 지도를 읽었어?"),
        (
            "SELECTIVE_BATCH",
            "누가 지도를 읽었는지 자세히 알려주고 장소는 말하지 마.",
        ),
        (
            "TWO_ANSWERS",
            "누가 지도를 읽었는지 알려주고 언제 읽었는지 알려줘.",
        ),
    ] {
        let mut api = CognitiveApi::new_embedded().unwrap();
        api.process_conversation_turn(&request(
            "GENERATION-AUDIT",
            1,
            "어제 노아는 공원에서 지도를 읽었어.",
            LanguageCodeIR::Korean,
        ))
        .unwrap();
        crate::generative_language::GENERATION_INVOCATIONS.with(|n| n.set(0));
        crate::natural_realization::FINAL_REALIZATION_CHECKS.with(|n| n.set(0));
        let input = request("GENERATION-AUDIT", 2, question, LanguageCodeIR::Korean);
        let response = api.process_conversation_turn(&input).unwrap();
        let calls = crate::generative_language::GENERATION_INVOCATIONS.with(|n| n.get());
        assert_eq!(calls, if name == "TWO_ANSWERS" { 2 } else { 1 }, "{name}");
        assert_eq!(
            crate::natural_realization::FINAL_REALIZATION_CHECKS.with(|n| n.get()),
            1
        );
        println!(
            "GENERATION_AUDIT {name} calls={calls} output={}",
            response.output.text
        );
        assert!(response.validate_against(&input));
        let answer = response.discourse_answer.as_ref().unwrap();
        let serialized = serde_json::to_string(answer).unwrap();
        let decoded: crate::discourse_qa::DiscourseAnswerIR =
            serde_json::from_str(&serialized).unwrap();
        crate::generative_language::GENERATION_INVOCATIONS.with(|n| n.set(0));
        assert!(decoded.validate());
        assert_eq!(
            crate::generative_language::GENERATION_INVOCATIONS.with(|n| n.get()),
            0
        );
        if !answer.response_parts.is_empty() {
            let mut forged = answer.clone();
            forged.response_parts[0].realized_text = "UNSUPPORTED_CALLER_PROSE".into();
            forged.realized_text = forged
                .response_parts
                .iter()
                .map(|p| p.realized_text.as_str())
                .collect::<Vec<_>>()
                .join(" ");
            assert!(!forged.validate());
            assert_ne!(answer.realized_text, response.output.text);
        }
    }
}

#[test]
fn selective_english_batch_preserves_multiple_answer_status() {
    let mut api = CognitiveApi::new_embedded().unwrap();
    for (i, report) in [
        "Nessa read a letter in the studio yesterday.",
        "Fenn read a letter in the library today.",
    ]
    .iter()
    .enumerate()
    {
        api.process_conversation_turn(&request(
            "SELECTIVE-EN",
            i as u64 + 1,
            report,
            LanguageCodeIR::English,
        ))
        .unwrap();
    }
    let input = request(
        "SELECTIVE-EN",
        3,
        "Tell me who read a letter in detail and do not tell me the location.",
        LanguageCodeIR::English,
    );
    crate::natural_realization::FINAL_REALIZATION_CHECKS.with(|n| n.set(0));
    let response = api.process_conversation_turn(&input).unwrap();
    assert_eq!(
        crate::natural_realization::FINAL_REALIZATION_CHECKS.with(|n| n.get()),
        1
    );
    assert!(response.validate_against(&input));
    assert_eq!(
        response.discourse_answer.as_ref().unwrap().disposition,
        crate::discourse_qa::DiscourseAnswerDispositionIR::MultipleDialogueRecords
    );
    for fragment in [
        "Nessa read a letter",
        "Fenn read a letter",
        "yesterday",
        "today",
    ] {
        assert!(
            response.output.text.contains(fragment),
            "{}",
            response.output.text
        );
    }
    for omitted in ["studio", "library"] {
        assert!(!response.output.text.contains(omitted));
    }
    assert!(
        !response
            .language_cortex_integration
            .external_action_executed
    );
}

#[test]
fn mixed_response_request_selects_permitted_detail_before_realization() {
    let mut api = CognitiveApi::new_embedded().unwrap();
    api.process_conversation_turn(&request(
        "DETAIL-SCOPE",
        1,
        "여울은 공원에서 지도를 읽었어.",
        LanguageCodeIR::Korean,
    ))
    .unwrap();
    let input = request(
        "DETAIL-SCOPE",
        2,
        "누가 지도를 읽었는지 자세히 알려주고 장소는 말하지 마.",
        LanguageCodeIR::Korean,
    );
    crate::natural_realization::FINAL_REALIZATION_CHECKS.with(|n| n.set(0));
    let response = api.process_conversation_turn(&input).unwrap();
    assert_eq!(
        crate::natural_realization::FINAL_REALIZATION_CHECKS.with(|n| n.get()),
        1
    );
    assert!(response.validate_against(&input));
    assert!(!response.conversation_contract.suppresses_answer());
    let answer = response.discourse_answer.as_ref().unwrap();
    assert_eq!(answer.response_parts.len(), 1);
    let projection = answer.response_parts[0]
        .content_projection
        .as_ref()
        .unwrap();
    assert_eq!(
        projection.elaboration_omitted_roles,
        [crate::proposition_content::ContentSlotIR::Location]
    );
    assert!(
        response.output.text.contains("여울이 지도를 읽었어"),
        "{}",
        response.output.text
    );
    let mut forged = answer.clone();
    forged.response_parts[0]
        .content_projection
        .as_mut()
        .unwrap()
        .elaboration_omitted_roles
        .clear();
    assert!(!forged.validate());
    assert!(
        !response.output.text.contains("공원"),
        "{}",
        response.output.text
    );
    assert!(
        !response
            .language_cortex_integration
            .external_action_executed
    );
}

#[test]
fn detailed_answers_plan_source_events_before_single_realization() {
    for (language, reports, question, fragments) in [
        (
            LanguageCodeIR::English,
            vec![
                "Vera wrote a letter yesterday.",
                "Oren wrote a letter today.",
            ],
            "Tell me who wrote a letter in detail.",
            vec![
                "Vera wrote a letter",
                "yesterday",
                "Oren wrote a letter",
                "today",
            ],
        ),
        (
            LanguageCodeIR::Korean,
            vec![
                "하린은 교실에서 책을 읽었어.",
                "도윤은 공원에서 책을 읽었어.",
            ],
            "누가 책을 읽었는지 자세히 알려줘.",
            vec!["하린", "교실에서", "도윤", "공원에서", "책을 읽었어"],
        ),
        (
            LanguageCodeIR::English,
            vec!["Lena borrowed a camera from Ezra."],
            "Tell me who lent a camera to Lena in detail.",
            vec!["Lena borrowed a camera from Ezra"],
        ),
    ] {
        let mut api = CognitiveApi::new_embedded().unwrap();
        for (i, report) in reports.iter().enumerate() {
            api.process_conversation_turn(&request("DETAIL-PLAN", i as u64 + 1, report, language))
                .unwrap();
        }
        let input = request("DETAIL-PLAN", reports.len() as u64 + 1, question, language);
        crate::natural_realization::FINAL_REALIZATION_CHECKS.with(|n| n.set(0));
        let response = api.process_conversation_turn(&input).unwrap();
        assert_eq!(
            crate::natural_realization::FINAL_REALIZATION_CHECKS.with(|n| n.get()),
            1
        );
        assert!(response.validate_against(&input));
        for fragment in fragments {
            assert!(
                response.output.text.contains(fragment),
                "{}: {}",
                question,
                response.output.text
            );
        }
        assert_eq!(response.natural_realization.generation_traces.len(), 1);
        let answer = response.discourse_answer.as_ref().unwrap();
        let projection = answer.content_projection.as_ref().unwrap();
        assert_eq!(projection.all_projections().count(), reports.len());
        assert_eq!(answer.claims.len(), reports.len() * 2);
        assert!(projection
            .all_projections()
            .all(|p| p.elaboration_event.is_some()));
        assert!(!answer.dialogue_truth_established);
        assert!(
            !response
                .language_cortex_integration
                .external_action_executed
        );
        for mutate_role in [false, true] {
            let mut forged = answer.clone();
            let event = forged
                .content_projection
                .as_mut()
                .unwrap()
                .elaboration_event
                .as_mut()
                .unwrap();
            if mutate_role {
                event.roles.insert(
                    crate::proposition_content::ContentSlotIR::Agent,
                    "Invented".into(),
                );
            } else {
                event.predicate_surface = "invented".into();
            }
            // Changing the claim along with the event still cannot alter the source.
            forged.claims[reports.len()].value = serde_json::to_string(event).unwrap();
            assert!(!forged.validate());
        }
        let mut missing_proof = answer.clone();
        missing_proof.claims.pop();
        assert!(!missing_proof.validate());
        let mut unrequested = answer.clone();
        unrequested
            .question_request
            .as_mut()
            .unwrap()
            .response_manner = None;
        assert!(!unrequested.validate());
        if reports.len() > 1 {
            let mut partial = answer.clone();
            partial.content_projection.as_mut().unwrap().co_answers[0].elaboration_event = None;
            partial.claims.pop();
            assert!(!partial.validate());
        }
    }
}

#[test]
fn event_perspective_answers_use_source_roles_and_one_realization_check() {
    for (language, report, questions) in [
        (
            LanguageCodeIR::Korean,
            "다원은 소율에게 우산을 빌려줬어.",
            vec![
                ("누가 다원에게서 우산을 빌렸어?", "소율"),
                ("그 사람은 누구에게서 우산을 빌렸어?", "다원"),
                ("소율은 누구에게서 우산을 빌렸어?", "다원"),
            ],
        ),
        (
            LanguageCodeIR::English,
            "Lena borrowed a camera from Ezra.",
            vec![
                ("Who lent a camera to Lena?", "Ezra"),
                ("To whom did Ezra lend a camera?", "Lena"),
            ],
        ),
    ] {
        let mut api = CognitiveApi::new_embedded().unwrap();
        let observed = api
            .process_conversation_turn(&request("PERSPECTIVE", 1, report, language))
            .unwrap();
        for (index, (question, value)) in questions.iter().enumerate() {
            let input = request("PERSPECTIVE", index as u64 + 2, question, language);
            crate::natural_realization::FINAL_REALIZATION_CHECKS.with(|n| n.set(0));
            let answer = api.process_conversation_turn(&input).unwrap();
            assert_eq!(
                crate::natural_realization::FINAL_REALIZATION_CHECKS.with(|n| n.get()),
                1
            );
            let projection = answer
                .discourse_answer
                .as_ref()
                .and_then(|a| a.content_projection.as_ref())
                .unwrap_or_else(|| panic!("{question}: {}", answer.output.text));
            assert_eq!(&projection.binding.value, value);
            assert!(projection.event_perspective.is_some());
            if language == LanguageCodeIR::English {
                use crate::proposition_content::ContentSlotIR;
                let expected_case = match projection.binding.slot {
                    ContentSlotIR::Recipient => Some("To "),
                    ContentSlotIR::Source => Some("From "),
                    _ => None,
                };
                if let Some(case) = expected_case {
                    assert!(
                        answer.output.text.starts_with(case),
                        "{}",
                        answer.output.text
                    );
                }
            }
            assert!(answer.grounded_response.is_none());
            assert_eq!(
                answer.conversation_state.active_goals,
                observed.conversation_state.active_goals
            );
            assert!(answer.validate_against(&input));
            assert_eq!(
                answer.conversation_state.epistemic_ledger.records[0].content,
                observed.conversation_state.epistemic_ledger.records[0].content
            );
        }
    }
}

#[test]
fn event_reference_clarification_returns_to_the_original_question() {
    for (language, reports, question, choice, expected) in [
        (
            LanguageCodeIR::Korean,
            [
                "서윤은 도서관에서 책을 읽었어.",
                "지환은 도서관에서 신문을 읽었어.",
            ],
            "아까 도서관 이야기로 돌아가서, 누가 그것을 읽었어?",
            "신문이야",
            "지환",
        ),
        (
            LanguageCodeIR::English,
            [
                "Mira read the letter at the garden.",
                "Noel read the newspaper at the garden.",
            ],
            "Back to the garden, who read it?",
            "the letter",
            "Mira",
        ),
    ] {
        let mut api = CognitiveApi::new_embedded().unwrap();
        for (index, text) in reports.iter().enumerate() {
            api.process_conversation_turn(&request(
                "REFERENCE-DIALOGUE",
                index as u64 + 1,
                text,
                language,
            ))
            .unwrap();
        }
        let input = request("REFERENCE-DIALOGUE", 3, question, language);
        crate::natural_realization::FINAL_REALIZATION_CHECKS.with(|n| n.set(0));
        let clarification = api.process_conversation_turn(&input).unwrap();
        assert_eq!(
            crate::natural_realization::FINAL_REALIZATION_CHECKS.with(|n| n.get()),
            1
        );
        let gap = clarification
            .discourse_answer
            .as_ref()
            .and_then(|a| a.reference_gap.as_ref())
            .unwrap_or_else(|| panic!("{}", clarification.output.text));
        assert!(gap.live_in(&clarification.conversation_state));
        for (value, _) in gap.choices().unwrap() {
            assert!(clarification.output.text.contains(&value));
        }
        assert!(clarification
            .conversation_state
            .pending_question
            .as_ref()
            .is_some_and(|q| q.reference_gap.is_some()));
        assert!(clarification.validate_against(&input));
        let followup = request("REFERENCE-DIALOGUE", 4, choice, language);
        crate::natural_realization::FINAL_REALIZATION_CHECKS.with(|n| n.set(0));
        let answer = api.process_conversation_turn(&followup).unwrap();
        assert_eq!(
            crate::natural_realization::FINAL_REALIZATION_CHECKS.with(|n| n.get()),
            1
        );
        assert_eq!(
            answer
                .discourse_answer
                .as_ref()
                .and_then(|a| a.content_projection.as_ref())
                .map(|p| p.binding.value.as_str()),
            Some(expected),
            "{}",
            answer.output.text
        );
        assert!(answer.grounded_response.is_none());
        assert!(answer.conversation_state.pending_question.is_none());
        assert_eq!(
            answer.conversation_state.active_goals,
            clarification.conversation_state.active_goals
        );
        let mut stale = clarification.conversation_state.clone();
        stale.epistemic_ledger.records[0].status =
            crate::epistemic::BeliefRecordStatusIR::Retracted;
        assert!(!gap.live_in(&stale));
        assert!(answer.validate_against(&followup));
    }
}

#[test]
fn open_event_questions_coordinate_source_bound_answers_once() {
    for (language, reports, question, expected) in [
        (
            LanguageCodeIR::Korean,
            [
                "다원은 책을 읽었어.",
                "소율은 책을 읽었어.",
                "은재는 책을 읽었어.",
            ],
            "누가 책을 읽었어?",
            vec!["다원", "소율", "은재"],
        ),
        (
            LanguageCodeIR::English,
            [
                "Mira read the letter.",
                "Noel read the letter.",
                "Lena read the letter.",
            ],
            "Back to the letter, who read it?",
            vec!["Mira", "Noel", "Lena"],
        ),
        (
            LanguageCodeIR::Korean,
            [
                "다원은 다락에서 편지를 읽었어.",
                "소율은 공원에서 편지를 읽었어.",
                "은재는 도서관에서 편지를 읽었어.",
            ],
            "어디에서 편지를 읽었어?",
            vec!["다락", "공원", "도서관"],
        ),
        (
            LanguageCodeIR::English,
            [
                "Mira read a letter at the garden.",
                "Mira read a letter at the station.",
                "Mira read a letter at the library.",
            ],
            "What did Mira read?",
            vec!["letter"],
        ),
    ] {
        let mut api = CognitiveApi::new_embedded().unwrap();
        for (index, report) in reports.iter().enumerate() {
            api.process_conversation_turn(&request(
                "ANSWER-SET",
                index as u64 + 1,
                report,
                language,
            ))
            .unwrap();
        }
        let input = request("ANSWER-SET", 4, question, language);
        crate::natural_realization::FINAL_REALIZATION_CHECKS.with(|n| n.set(0));
        crate::generative_language::MORPHOLOGY_PASSES.with(|n| n.set(0));
        let response = api.process_conversation_turn(&input).unwrap();
        assert_eq!(
            crate::natural_realization::FINAL_REALIZATION_CHECKS.with(|n| n.get()),
            1
        );
        assert_eq!(
            crate::generative_language::MORPHOLOGY_PASSES.with(|n| n.get()),
            2 * response.natural_realization.generation_traces.len()
        );
        let answer = response.discourse_answer.as_ref().expect(question);
        let p = answer
            .content_projection
            .as_ref()
            .unwrap_or_else(|| panic!("{question}: {}", response.output.text));
        assert_eq!(
            p.all_projections()
                .map(|p| p.binding.value.to_lowercase())
                .collect::<std::collections::BTreeSet<_>>(),
            expected.iter().map(|v| v.to_lowercase()).collect(),
            "{question}"
        );
        assert_eq!(p.co_answers.len(), 2);
        assert_eq!(answer.evidence.len(), 3);
        assert!(answer.focused_event().is_none());
        assert!(!answer.dialogue_truth_established);
        assert!(response.grounded_response.is_none());
        assert!(response.validate_against(&input));
        for value in &expected {
            assert!(
                response.output.text.contains(value),
                "{}",
                response.output.text
            );
        }
        let value_nodes = response
            .natural_realization
            .generation_traces
            .iter()
            .flat_map(|g| &g.meaning.nodes)
            .filter(|n| n.node_id.starts_with("CONTENT_SET_VALUE_"))
            .count();
        assert_eq!(value_nodes, expected.len());
        let mut forged = answer.clone();
        forged.content_projection.as_mut().unwrap().co_answers[0]
            .binding
            .value = "UNATTESTED".into();
        assert!(!forged.validate());
    }
}

#[test]
fn named_topic_questions_do_not_guess_missing_or_ambiguous_memory() {
    use crate::discourse_qa::DiscourseAnswerDispositionIR;
    for (language, first, second, question, disposition) in [
        (
            LanguageCodeIR::Korean,
            "서윤은 도서관에서 책을 읽었어.",
            "지환은 도서관에서 신문을 읽었어.",
            "아까 도서관 이야기로 돌아가서, 누가 그것을 읽었어?",
            DiscourseAnswerDispositionIR::AmbiguousQuery,
        ),
        (
            LanguageCodeIR::English,
            "Mira read the letter at the garden.",
            "Noel read the newspaper at the garden.",
            "Back to the garden, who read it?",
            DiscourseAnswerDispositionIR::AmbiguousQuery,
        ),
        (
            LanguageCodeIR::English,
            "Mira read the letter at the garden.",
            "Noel read the newspaper at the station.",
            "Back to the magazine, who read it?",
            DiscourseAnswerDispositionIR::NoMatchingRecord,
        ),
        (
            LanguageCodeIR::Korean,
            "서윤은 책을 읽었어.",
            "지환은 신문을 읽었어.",
            "아까 편지 이야기로 돌아가서, 누가 읽었어?",
            DiscourseAnswerDispositionIR::NoMatchingRecord,
        ),
    ] {
        let mut api = CognitiveApi::new_embedded().unwrap();
        for (i, text) in [first, second].into_iter().enumerate() {
            api.process_conversation_turn(&request("TOPIC-GAP", i as u64 + 1, text, language))
                .unwrap();
        }
        let input = request("TOPIC-GAP", 3, question, language);
        let response = api.process_conversation_turn(&input).unwrap();
        let answer = response.discourse_answer.as_ref().expect(question);
        assert_eq!(
            answer.disposition, disposition,
            "{question}: {}",
            response.output.text
        );
        assert!(answer.content_projection.is_none());
        assert!(response.grounded_response.is_none());
        assert!(response.validate_against(&input));
    }
}

#[test]
fn named_topic_questions_answer_from_memory_without_transition_monologue() {
    for (language, first, second, question, expected, wrong_topic) in [
        (
            LanguageCodeIR::Korean,
            "서윤은 도서관에서 책을 읽었어.",
            "지환은 사무실에서 신문을 읽었어.",
            "아까 책 이야기로 돌아가서, 누가 읽었어?",
            "서윤",
            "아까 신문 이야기로 돌아가서, 누가 읽었어?",
        ),
        (
            LanguageCodeIR::English,
            "Mira read the letter at the garden.",
            "Noel read the newspaper at the station.",
            "Back to the letter, where did Mira read it?",
            "garden",
            "Back to the newspaper, where did Mira read it?",
        ),
        (
            LanguageCodeIR::Korean,
            "다원은 다락에서 편지를 읽었어.",
            "소율은 공원에서 책을 읽었어.",
            "아까 다락 이야기로 돌아가서, 누가 읽었어?",
            "다원",
            "아까 공원 이야기로 돌아가서, 누가 읽었어?",
        ),
    ] {
        let mut api = CognitiveApi::new_embedded().unwrap();
        for (i, text) in [first, second].into_iter().enumerate() {
            api.process_conversation_turn(&request("TOPIC-QUESTION", i as u64 + 1, text, language))
                .unwrap();
        }
        let input = request("TOPIC-QUESTION", 3, question, language);
        crate::natural_realization::FINAL_REALIZATION_CHECKS.with(|n| n.set(0));
        let response = api.process_conversation_turn(&input).unwrap();
        assert_eq!(
            crate::natural_realization::FINAL_REALIZATION_CHECKS.with(|n| n.get()),
            1
        );
        let projection = response
            .discourse_answer
            .as_ref()
            .and_then(|a| a.content_projection.as_ref())
            .unwrap_or_else(|| panic!("{question}: {}", response.output.text));
        assert_eq!(projection.binding.value, expected);
        assert!(projection.matches_question(question));
        assert!(!projection.matches_question(wrong_topic));
        assert!(response.grounded_response.is_none());
        assert!(response
            .natural_realization
            .response_plan
            .moves
            .iter()
            .all(|m| m.response_act != NaturalResponseActIR::TopicTransition));
        assert!(response.validate_against(&input));
    }
}

#[test]
fn ordinary_information_receipt_does_not_turn_into_veracity_lecture() {
    for (language, source) in [
        (LanguageCodeIR::Korean, "아, 그리고 나 오늘 좀 피곤해."),
        (
            LanguageCodeIR::Korean,
            "음… 오늘은 머리가 복잡해서 말이 잘 안 나오네.",
        ),
        (LanguageCodeIR::Korean, "응, 실행하라는 뜻은 아니었어."),
        (LanguageCodeIR::English, "I had a long day."),
    ] {
        let mut api = CognitiveApi::new_embedded().unwrap();
        let input = request("RECEIPT-NOT-CERTIFICATE", 1, source, language);
        crate::generative_language::MORPHOLOGY_PASSES.with(|count| count.set(0));
        crate::natural_realization::FINAL_REALIZATION_CHECKS.with(|count| count.set(0));
        let response = api.process_conversation_turn(&input).unwrap();
        assert_eq!(
            response.natural_realization.response_act,
            NaturalResponseActIR::InformAcknowledgement,
            "{source}"
        );
        assert_eq!(
            crate::natural_realization::FINAL_REALIZATION_CHECKS.with(|count| count.get()),
            1
        );
        assert_eq!(
            crate::generative_language::MORPHOLOGY_PASSES.with(|count| count.get()),
            2 * response.natural_realization.generation_traces.len()
        );
        assert!(response
            .natural_realization
            .generation_traces
            .iter()
            .all(|trace| trace
                .meaning
                .nodes
                .iter()
                .all(|node| node.concept_id == "C_ACKNOWLEDGE")));
        assert!(!response
            .conversation_state
            .epistemic_ledger
            .records
            .is_empty());
        assert!(response
            .conversation_state
            .epistemic_ledger
            .records
            .iter()
            .all(|record| !record.dialogue_truth_established
                && !record.external_execution_authorized));
        assert!(response.grounded_response.is_none());
        assert!(response.validate_against(&input));
    }
}

#[test]
fn multi_observation_correction_retains_event_for_followup_answers() {
    for (language, source, correction, question, expected) in [
        (
            LanguageCodeIR::Korean,
            "유림은 공원에서 편지를 읽었어.",
            "장소를 잘못 말했네. 공원이 아니고 집이야.",
            "유림이 편지를 읽은 곳이 어디라고?",
            "집",
        ),
        (
            LanguageCodeIR::Korean,
            "다원은 다락에서 신문을 읽었어.",
            "물건을 잘못 말했네. 신문이 아니고 편지야.",
            "다원은 무엇을 읽었어?",
            "편지",
        ),
        (
            LanguageCodeIR::English,
            "Mira read the letter at the harbor.",
            "It was the garden, not the harbor.",
            "Where did Mira read the letter?",
            "garden",
        ),
    ] {
        let mut api = CognitiveApi::new_embedded().unwrap();
        api.process_conversation_turn(&request("ROLE-REVISION", 1, source, language))
            .unwrap();
        api.process_conversation_turn(&request("ROLE-REVISION", 2, correction, language))
            .unwrap();
        let input = request("ROLE-REVISION", 3, question, language);
        crate::natural_realization::FINAL_REALIZATION_CHECKS.with(|count| count.set(0));
        let response = api.process_conversation_turn(&input).unwrap();
        assert_eq!(
            crate::natural_realization::FINAL_REALIZATION_CHECKS.with(|count| count.get()),
            1
        );
        let projection = response
            .discourse_answer
            .as_ref()
            .and_then(|a| a.content_projection.as_ref())
            .unwrap_or_else(|| panic!("{question}: {}", response.output.text));
        assert_eq!(projection.binding.value, expected);
        assert_eq!(projection.context_sources.len(), 1);
        assert!(response.grounded_response.is_none());
        assert!(response.validate_against(&input));
    }
}

#[test]
fn execution_capability_questions_read_the_conversation_boundary_not_action_history() {
    for (language, question, predicate) in [
        (LanguageCodeIR::Korean, "자료를 읽을 수 있나요?", "READ"),
        (LanguageCodeIR::Korean, "문서를 열 수 있나요?", "OPEN"),
        (LanguageCodeIR::Korean, "기록을 저장할 수 있나요?", "SAVE"),
        (
            LanguageCodeIR::English,
            "Can the system read the document?",
            "READ",
        ),
        (
            LanguageCodeIR::English,
            "Can the system open the file?",
            "OPEN",
        ),
    ] {
        let mut api = CognitiveApi::new_embedded().unwrap();
        let input = request("CAPABILITY-BOUNDARY", 1, question, language);
        crate::generative_language::MORPHOLOGY_PASSES.with(|p| p.set(0));
        crate::natural_realization::FINAL_REALIZATION_CHECKS.with(|p| p.set(0));
        let response = api.process_conversation_turn(&input).unwrap();
        let query = response
            .action_state_analysis
            .execution_capability_question
            .as_ref()
            .unwrap_or_else(|| {
                panic!(
                    "{question}: {:?}; {}",
                    response.pragmatic_interpretation.compositional_analysis, response.output.text
                )
            });
        assert_eq!(query.canonical_predicate, predicate);
        assert!(!response.action_state_analysis.query_requested);
        assert!(response
            .conversation_state
            .action_state_ledger
            .records
            .is_empty());
        assert!(response.grounded_response.is_none());
        assert_eq!(
            response.natural_realization.response_act,
            NaturalResponseActIR::ActionState,
            "{question}: {}",
            response.output.text
        );
        assert!(response
            .natural_realization
            .generation_traces
            .iter()
            .flat_map(|t| &t.meaning.nodes)
            .any(|n| n.concept_id == "C_CONVERSATION_EXTERNAL_EXECUTION_UNAVAILABLE"));
        assert_eq!(
            crate::generative_language::MORPHOLOGY_PASSES.with(|p| p.get()),
            2 * response.natural_realization.generation_traces.len()
        );
        assert_eq!(
            crate::natural_realization::FINAL_REALIZATION_CHECKS.with(|p| p.get()),
            1
        );
        assert!(
            !response
                .language_cortex_integration
                .external_action_executed
        );
        assert!(response.validate_against(&input));
        let meaning = response.natural_realization.generation_traces[0]
            .meaning
            .semantic_sha256
            .clone();
        let mut with_history = CognitiveApi::new_embedded().unwrap();
        let first = with_history
            .process_conversation_turn(&request(
                "CAPABILITY-HISTORY",
                1,
                if language == LanguageCodeIR::Korean {
                    "사본을 저장해."
                } else {
                    "Save the archive."
                },
                language,
            ))
            .unwrap();
        let input = request("CAPABILITY-HISTORY", 2, question, language);
        let response = with_history.process_conversation_turn(&input).unwrap();
        assert_eq!(
            response.conversation_state.action_state_ledger,
            first.conversation_state.action_state_ledger
        );
        assert_eq!(
            response.natural_realization.generation_traces[0]
                .meaning
                .semantic_sha256,
            meaning,
            "an unrelated plan must not supply capability evidence: {question}"
        );
    }
}

#[test]
fn capability_boundary_does_not_claim_other_actors_or_replace_selected_requests() {
    for (language, source, request_expected) in [
        (
            LanguageCodeIR::Korean,
            "민수가 자료를 읽을 수 있나요?",
            false,
        ),
        (
            LanguageCodeIR::Korean,
            "민수는 자료를 읽을 수 있나요?",
            false,
        ),
        (
            LanguageCodeIR::English,
            "Can Mina read the document?",
            false,
        ),
        (
            LanguageCodeIR::English,
            "Can you read the document for me?",
            true,
        ),
        (LanguageCodeIR::Korean, "자료를 읽어 줄 수 있나요?", true),
        (LanguageCodeIR::Korean, "자료를 읽어주실 수 있나요?", true),
        (
            LanguageCodeIR::Korean,
            "민수가 자료를 읽어줄 수 있나요?",
            false,
        ),
    ] {
        let mut api = CognitiveApi::new_embedded().unwrap();
        let input = request("CAPABILITY-SCOPE", 1, source, language);
        let response = api.process_conversation_turn(&input).unwrap();
        assert!(
            response
                .action_state_analysis
                .execution_capability_question
                .is_none(),
            "{source}"
        );
        if request_expected {
            assert!(
                response.grounded_response.is_some(),
                "selected request lost: {source}: {}",
                response.output.text
            );
        } else {
            assert!(
                response.grounded_response.is_none(),
                "third-party ability became a task: {source}"
            );
        }
        assert!(
            !response
                .language_cortex_integration
                .external_action_executed
        );
        assert!(response.validate_against(&input));
    }
}

#[test]
fn recipient_before_question_complement_belongs_to_matrix_not_inner_event() {
    use crate::compositional_semantics::{CompositionalSemanticAnalyzer, FrameMoodIR};
    use crate::semantic_roles::SemanticRoleKindIR;
    for (source, outer) in [
        ("민수에게 누가 책을 읽었는지 말해줘.", true),
        ("민수에게 누가 책을 읽었는지 알려주세요.", true),
        ("누가 민수에게 책을 읽었는지 말해줘.", false),
    ] {
        let analysis = CompositionalSemanticAnalyzer.analyze(source);
        let owner = analysis
            .frames
            .iter()
            .find(|frame| {
                analysis
                    .semantic_role_graph
                    .arguments_for_frame(&frame.frame_id)
                    .iter()
                    .any(|(role, node)| {
                        *role == SemanticRoleKindIR::Recipient && node.normalized_label == "민수"
                    })
            })
            .expect(source);
        assert_eq!(
            owner.mood != FrameMoodIR::ContentComplement,
            outer,
            "{source}"
        );
        assert_eq!(
            crate::proposition_content::question_request(source).is_none(),
            outer,
            "{source}"
        );
    }
}

#[test]
fn past_event_questions_bind_existing_actions_without_creating_execution() {
    for (language, action, question, predicate) in [
        (
            LanguageCodeIR::Korean,
            "자료를 읽어.",
            "실제로 읽었나요?",
            "READ",
        ),
        (LanguageCodeIR::Korean, "문서를 열어.", "열었나요?", "OPEN"),
        (
            LanguageCodeIR::Korean,
            "사본을 저장해.",
            "저장했나요?",
            "SAVE",
        ),
        (
            LanguageCodeIR::English,
            "Read the document.",
            "Did you read it?",
            "READ",
        ),
        (
            LanguageCodeIR::English,
            "Open the file.",
            "Did you open it?",
            "OPEN",
        ),
        (
            LanguageCodeIR::Korean,
            "자료를 읽어.",
            "왜 읽었나요?",
            "READ",
        ),
        (
            LanguageCodeIR::Korean,
            "문서를 열어.",
            "언제 열었나요?",
            "OPEN",
        ),
        (
            LanguageCodeIR::Korean,
            "사본을 저장해.",
            "어떻게 저장했나요?",
            "SAVE",
        ),
        (
            LanguageCodeIR::English,
            "Read the document.",
            "Why did you read it?",
            "READ",
        ),
        (
            LanguageCodeIR::English,
            "Open the file.",
            "When did you open it?",
            "OPEN",
        ),
        (
            LanguageCodeIR::English,
            "Save the copy.",
            "How did you save it?",
            "SAVE",
        ),
    ] {
        let mut api = CognitiveApi::new_embedded().unwrap();
        let first = api
            .process_conversation_turn(&request("PAST-EVENT", 1, action, language))
            .unwrap();
        let input = request("PAST-EVENT", 2, question, language);
        crate::generative_language::MORPHOLOGY_PASSES.with(|passes| passes.set(0));
        crate::natural_realization::FINAL_REALIZATION_CHECKS.with(|checks| checks.set(0));
        let response = api.process_conversation_turn(&input).unwrap();
        assert!(
            response.action_state_analysis.query_requested,
            "{question}: {:?}; {:?}",
            response
                .pragmatic_interpretation
                .compositional_analysis
                .frames,
            response
                .pragmatic_interpretation
                .compositional_analysis
                .semantic_role_graph
        );
        assert_eq!(
            response
                .action_state_analysis
                .event_question_frame_ids
                .len(),
            1,
            "{question}"
        );
        assert_eq!(
            response.action_state_analysis.target_action_ids,
            vec![first.conversation_state.action_state_ledger.records[0]
                .action_id
                .clone()]
        );
        assert_eq!(
            response.conversation_state.action_state_ledger.records[0].canonical_predicate,
            predicate
        );
        assert_eq!(
            response.conversation_state.action_state_ledger,
            first.conversation_state.action_state_ledger
        );
        assert!(response.grounded_response.is_none());
        if !response
            .action_state_analysis
            .event_question_content_slots
            .is_empty()
        {
            assert_eq!(
                response.plan_result_boundary.query_focus,
                PlanResultQueryFocusIR::UnverifiedEventPremise
            );
            let claims = response
                .natural_realization
                .generation_traces
                .iter()
                .flat_map(|trace| &trace.meaning.nodes)
                .filter(|node| {
                    node.kind == crate::generative_language::GenerationMeaningNodeKindIR::Quality
                })
                .map(|node| node.concept_id.as_str())
                .collect::<Vec<_>>();
            assert_eq!(claims, vec!["C_LIFECYCLE_NO_EXECUTION_OR_RESULT"],
                "an unknown event premise must not produce a new plan or invented cause: {question}");
        }
        assert!(
            response.discourse_answer.is_none(),
            "{question}: {:?}",
            response.discourse_answer
        );
        assert_eq!(
            crate::generative_language::MORPHOLOGY_PASSES.with(|passes| passes.get()),
            2 * response.natural_realization.generation_traces.len()
        );
        assert_eq!(
            crate::natural_realization::FINAL_REALIZATION_CHECKS.with(|checks| checks.get()),
            1
        );
        assert!(
            !response
                .language_cortex_integration
                .external_action_executed
        );
        assert!(response.validate_against(&input));
    }
}

#[test]
fn event_identity_resolution_survives_answer_domain_selection() {
    for (language, actions, questions, requires_clarification) in [
        (
            LanguageCodeIR::Korean,
            "책을 읽고 문서를 읽어.",
            ["읽었나요?", "왜 읽었나요?"],
            true,
        ),
        (
            LanguageCodeIR::English,
            "Read the book and read the document.",
            ["Did you read it?", "Why did you read it?"],
            false,
        ),
    ] {
        for question in questions {
            let mut api = CognitiveApi::new_embedded().unwrap();
            let first = api
                .process_conversation_turn(&request("EVENT-IDENTITY", 1, actions, language))
                .unwrap();
            let input = request("EVENT-IDENTITY", 2, question, language);
            crate::generative_language::MORPHOLOGY_PASSES.with(|p| p.set(0));
            crate::natural_realization::FINAL_REALIZATION_CHECKS.with(|p| p.set(0));
            let response = api.process_conversation_turn(&input).unwrap();
            let unresolved = !response
                .action_state_analysis
                .unresolved_ambiguities
                .is_empty()
                || !response
                    .reference_resolution
                    .ambiguous_reference_surfaces
                    .is_empty();
            assert_eq!(unresolved, requires_clarification, "{question}");
            if requires_clarification {
                assert!(response.plan_result_boundary.selected_action_ids.is_empty());
                assert_eq!(
                    response.natural_realization.response_act,
                    NaturalResponseActIR::ClarificationRequest,
                    "{question}: {}",
                    response.output.text
                );
                assert!(!response
                    .natural_realization
                    .generation_traces
                    .iter()
                    .flat_map(|t| &t.meaning.nodes)
                    .any(|n| n.concept_id.starts_with("C_LIFECYCLE_")));
            } else {
                // Existing discourse-focus resolution binds English `it` to
                // document. This is a preservation control, not evidence that
                // every multi-target pronoun is linguistically unambiguous.
                assert!(!response.reference_resolution.discourse_bindings.is_empty());
                assert_eq!(response.action_state_analysis.target_action_ids.len(), 1);
                let bound = response
                    .conversation_state
                    .action_state_ledger
                    .record(&response.action_state_analysis.target_action_ids[0])
                    .unwrap();
                assert_eq!(bound.subject, "document");
                assert_ne!(
                    response.natural_realization.response_act,
                    NaturalResponseActIR::ClarificationRequest
                );
            }
            assert_eq!(
                response.conversation_state.action_state_ledger,
                first.conversation_state.action_state_ledger
            );
            assert!(response.grounded_response.is_none());
            assert_eq!(
                crate::generative_language::MORPHOLOGY_PASSES.with(|p| p.get()),
                2 * response.natural_realization.generation_traces.len()
            );
            assert_eq!(
                crate::natural_realization::FINAL_REALIZATION_CHECKS.with(|p| p.get()),
                1
            );
            assert!(response.validate_against(&input));
        }
    }
}

#[test]
fn prohibited_response_values_reach_memory_and_pre_generation_choice() {
    for (language, positive, negative, action) in [
        (
            LanguageCodeIR::Korean,
            "답변은 상세하게 해줘.",
            "답변은 상세하게 하지 마.",
            "문서를 읽어.",
        ),
        (
            LanguageCodeIR::English,
            "Keep the response detailed.",
            "Do not keep the response detailed.",
            "Read the document.",
        ),
    ] {
        let mut api = CognitiveApi::new_embedded().unwrap();
        for (index, text) in [positive, negative, action].iter().enumerate() {
            let input = request("PROHIBITED-PREFERENCE", index as u64 + 1, text, language);
            let response = api.process_conversation_turn(&input).unwrap();
            assert!(response.validate_against(&input));
            if index == 1 {
                assert_eq!(
                    response.natural_realization.response_act,
                    NaturalResponseActIR::InformAcknowledgement
                );
                assert!(response.grounded_response.is_none());
                assert!(response
                    .conversation_state
                    .dialogue_directive_ledger
                    .active()
                    .any(|d| d.value_key == "DETAILED" && d.prohibited));
                assert!(!response
                    .conversation_state
                    .dialogue_directive_ledger
                    .active()
                    .any(|d| d.value_key == "CONCISE" && !d.prohibited));
            }
            if index == 2 {
                assert!(!response
                    .natural_realization
                    .generation_traces
                    .iter()
                    .flat_map(|t| &t.meaning.nodes)
                    .any(|n| n.concept_id == "C_OBSERVE_CURRENT_STATE"));
            }
            assert!(
                !response
                    .language_cortex_integration
                    .external_action_executed
            );
        }
    }
}

#[test]
fn incompatible_response_constraints_ask_without_overwriting_memory() {
    let mut api = CognitiveApi::new_embedded().unwrap();
    let first = request(
        "CONSTRAINT-CHOICE",
        1,
        "Don't keep the response brief.",
        LanguageCodeIR::English,
    );
    let first_response = api.process_conversation_turn(&first).unwrap();
    assert!(first_response
        .conversation_state
        .dialogue_directive_ledger
        .active()
        .any(|d| d.value_key == "CONCISE" && d.prohibited));
    let input = request(
        "CONSTRAINT-CHOICE",
        2,
        "Do not make the response detailed.",
        LanguageCodeIR::English,
    );
    crate::generative_language::MORPHOLOGY_PASSES.with(|n| n.set(0));
    crate::natural_realization::FINAL_REALIZATION_CHECKS.with(|n| n.set(0));
    let response = api.process_conversation_turn(&input).unwrap();
    assert_eq!(
        response.natural_realization.response_act,
        NaturalResponseActIR::ClarificationRequest
    );
    assert!(response
        .natural_realization
        .generation_traces
        .iter()
        .flat_map(|t| &t.meaning.nodes)
        .any(|n| n.concept_id == "C_NAME_TARGET"));
    assert!(!response
        .natural_realization
        .generation_traces
        .iter()
        .flat_map(|t| &t.meaning.nodes)
        .any(|n| n.concept_id == "C_CLARIFY_COMPETING_REQUEST"));
    assert_eq!(
        response.conversation_state.dialogue_directive_ledger,
        first_response.conversation_state.dialogue_directive_ledger
    );
    assert_eq!(
        crate::generative_language::MORPHOLOGY_PASSES.with(|n| n.get()),
        2 * response.natural_realization.generation_traces.len()
    );
    assert_eq!(
        crate::natural_realization::FINAL_REALIZATION_CHECKS.with(|n| n.get()),
        1
    );
    assert!(response.validate_against(&input));
    assert!(
        !response
            .language_cortex_integration
            .external_action_executed
    );
}

#[test]
fn turn_realization_builds_once_then_checks_once_across_surface_policies() {
    use crate::generative_language::MORPHOLOGY_PASSES;
    use crate::natural_realization::FINAL_REALIZATION_CHECKS;
    for (language, turns) in [
        (
            LanguageCodeIR::Korean,
            vec![
                "안녕하세요.",
                "대답은 상세하게 해주세요.",
                "문서를 읽어 주세요.",
                "민수가 안내서를 읽었어.",
                "누가 읽었어?",
                "ㅋㅋ 고마워",
                "그거 취소해.",
            ],
        ),
        (
            LanguageCodeIR::English,
            vec![
                "Hello!",
                "Please keep your answers brief.",
                "Read the document.",
                "Mina read the handbook.",
                "Who read it?",
                "Thanks haha!",
                "Cancel that.",
            ],
        ),
    ] {
        let mut api = CognitiveApi::new_embedded().unwrap();
        for (index, text) in turns.iter().enumerate() {
            let input = request("SINGLE-SURFACE", index as u64 + 1, text, language);
            MORPHOLOGY_PASSES.with(|passes| passes.set(0));
            FINAL_REALIZATION_CHECKS.with(|checks| checks.set(0));
            let response = api.process_conversation_turn(&input).unwrap();
            let traces = &response.natural_realization.generation_traces;
            assert!(!traces.is_empty(), "{text}");
            // One construction plus one independent replay at the final
            // completed-output check. No tone-driven construction in between.
            assert_eq!(
                MORPHOLOGY_PASSES.with(|passes| passes.get()),
                2 * traces.len(),
                "{text}"
            );
            assert_eq!(
                FINAL_REALIZATION_CHECKS.with(|checks| checks.get()),
                1,
                "{text}"
            );
            assert!(
                !response
                    .language_cortex_integration
                    .external_action_executed
            );
            assert!(response.validate_against(&input));
        }
    }
}

#[test]
fn lexical_morphology_reaches_preference_memory_and_question_manner() {
    use crate::proposition_content::{content_request, ResponseMannerIR};
    for form in ["자세하게", "상세하게"] {
        let mut api = CognitiveApi::new_embedded().unwrap();
        let text = format!("대답은 조금 {form} 해주세요.");
        let input = request("MORPHOLOGY-PREFERENCE", 1, &text, LanguageCodeIR::Korean);
        let response = api.process_conversation_turn(&input).unwrap();
        assert!(response.grounded_response.is_none());
        assert_eq!(response.natural_realization.generation_traces.len(), 1);
        assert!(response.natural_realization.generation_traces[0]
            .meaning
            .nodes
            .iter()
            .all(|node| node.concept_id == "C_ACKNOWLEDGE"));
        assert!(response
            .conversation_state
            .dialogue_directive_ledger
            .active()
            .any(|d| d.value_key == "DETAILED"));
        assert!(response.validate_against(&input));
        let task = request(
            "MORPHOLOGY-PREFERENCE",
            2,
            "Read the transcript.",
            LanguageCodeIR::English,
        );
        let response = api.process_conversation_turn(&task).unwrap();
        assert!(response
            .natural_realization
            .generation_traces
            .iter()
            .flat_map(|g| &g.meaning.nodes)
            .any(|node| node.concept_id == "C_OBSERVE_CURRENT_STATE"));
        assert!(response.validate_against(&task));
        assert!(
            !response
                .language_cortex_integration
                .external_action_executed
        );

        let query = format!("왜 그랬는지 좀 {form} 말해줘.");
        let parsed = content_request(&query)
            .expect("same morphological knowledge in content-request parsing");
        assert_eq!(parsed.response_manner, Some(ResponseMannerIR::Detailed));
        assert!(parsed.validate());
    }
}

#[test]
fn plan_set_has_one_status_boundary_without_repeating_scaffolding() {
    for (language, text) in [
        (LanguageCodeIR::Korean, "기록을 열고 파일을 저장해."),
        (
            LanguageCodeIR::English,
            "Read the transcript and delete the draft.",
        ),
    ] {
        let mut api = CognitiveApi::new_embedded().unwrap();
        let input = request("PLAN-SET-CONTENT", 1, text, language);
        crate::natural_realization::FINAL_REALIZATION_CHECKS.with(|checks| checks.set(0));
        let response = api.process_conversation_turn(&input).unwrap();
        assert_eq!(
            crate::natural_realization::FINAL_REALIZATION_CHECKS.with(|checks| checks.get()),
            1
        );
        let traces = &response.natural_realization.generation_traces;
        let concepts = traces
            .iter()
            .flat_map(|trace| &trace.meaning.nodes)
            .map(|node| node.concept_id.as_str())
            .collect::<Vec<_>>();
        assert_eq!(
            traces
                .iter()
                .filter(|trace| trace.meaning.nodes.iter().any(|n| n.node_id == "E_ACTION"))
                .count(),
            2
        );
        assert_eq!(
            concepts
                .iter()
                .filter(|id| **id == "C_LIFECYCLE_NO_EXECUTION_OR_RESULT")
                .count(),
            1
        );
        assert!(!concepts.iter().any(|id| matches!(
            *id,
            "C_ACKNOWLEDGE"
                | "C_OBSERVE_CURRENT_STATE"
                | "C_VERIFY_RESULT"
                | "C_PLANNED_STATE"
                | "C_EXECUTED_OCCURRENCE"
        )));
        assert_eq!(response.natural_realization.sentences.len(), 3);
        assert!(response.validate_against(&input));
        let action_traces = traces
            .iter()
            .filter(|trace| {
                trace
                    .meaning
                    .nodes
                    .iter()
                    .any(|node| node.node_id == "E_ACTION")
            })
            .cloned()
            .collect::<Vec<_>>();
        let mut truncated = response.natural_realization.coverage.clone();
        for obligation in &mut truncated.obligations {
            obligation
                .supporting_generation_trace_sha256s
                .retain(|hash| {
                    action_traces
                        .iter()
                        .any(|trace| &trace.generation_sha256 == hash)
                });
        }
        truncated.coverage_sha256 =
            crate::natural_realization::natural_realization_coverage_sha256(&truncated);
        assert!(!truncated.validate_against(
            &response.natural_realization.response_plan,
            &action_traces,
            Some(&response.grounded_response.as_ref().unwrap().semantic_goal)
        ));
        assert!(
            !response
                .language_cortex_integration
                .external_action_executed
        );
    }
}

#[test]
fn connective_action_meanings_and_targets_survive_to_output() {
    for (verb, predicate, root) in [("열고", "OPEN", "여는"), ("옮기고", "MOVE", "이동하는")]
    {
        let text = format!("기록을 {verb} 파일을 저장해.");
        let mut api = CognitiveApi::new_embedded().unwrap();
        let input = request("CONNECTIVE-MEANING", 1, &text, LanguageCodeIR::Korean);
        let response = api.process_conversation_turn(&input).unwrap();
        let goal = &response.grounded_response.as_ref().unwrap().semantic_goal;
        let selected = goal
            .selected_live_event_ids
            .iter()
            .map(|id| {
                let event = goal
                    .events
                    .iter()
                    .find(|event| &event.event_id == id)
                    .unwrap();
                let subjects = event
                    .goal_subject_argument_ids
                    .iter()
                    .map(|id| {
                        goal.arguments
                            .iter()
                            .find(|arg| &arg.argument_id == id)
                            .unwrap()
                            .grounded_label
                            .as_str()
                    })
                    .collect::<Vec<_>>();
                (event.predicate_concept_id.as_str(), subjects)
            })
            .collect::<Vec<_>>();
        assert_eq!(
            selected,
            vec![(predicate, vec!["기록"]), ("SAVE", vec!["파일"])],
            "{text}"
        );
        assert!(
            response.output.text.contains(root),
            "{}",
            response.output.text
        );
        assert!(
            response.output.text.contains("파일을 저장하는"),
            "{}",
            response.output.text
        );
        assert!(response.validate_against(&input));
        assert!(
            !response
                .language_cortex_integration
                .external_action_executed
        );
    }
}

#[test]
fn valid_language_trace_cannot_substitute_a_different_action() {
    let mut api = CognitiveApi::new_embedded().unwrap();
    let input = request(
        "ACTION-TRACE-BOUNDARY",
        1,
        "Save the document.",
        LanguageCodeIR::English,
    );
    let response = api.process_conversation_turn(&input).unwrap();
    let original = &response.natural_realization.generation_traces[0];
    let event_ref = original
        .meaning
        .nodes
        .iter()
        .flat_map(|node| &node.grounding_refs)
        .find(|reference| reference.starts_with("SEMANTIC_PLAN_EVENT:"))
        .unwrap();
    let substituted = crate::generative_language::generate_plan_preview_with_predicate(
        LanguageCodeIR::English,
        "the document",
        dockable_semantic_core::PlanIntentIR::Execute,
        event_ref,
        None,
        crate::generative_language::PlanPreviewContentIR::Compact,
        Some("DELETE"),
    )
    .unwrap();
    assert!(substituted.validate());
    let mut coverage = response.natural_realization.coverage.clone();
    for obligation in &mut coverage.obligations {
        for hash in &mut obligation.supporting_generation_trace_sha256s {
            if *hash == original.generation_sha256 {
                *hash = substituted.generation_sha256.clone();
            }
        }
    }
    coverage.coverage_sha256 =
        crate::natural_realization::natural_realization_coverage_sha256(&coverage);
    assert!(!coverage.validate_against(
        &response.natural_realization.response_plan,
        &[substituted],
        Some(&response.grounded_response.as_ref().unwrap().semantic_goal)
    ));
}

#[test]
fn a_valid_promise_cannot_replace_a_plan_description() {
    use crate::generative_language::{
        ExpressionNodeStore, GenerationSpeechIntentIR, GenerativeLanguageCortex,
        GenerativeLanguageRequestIR,
    };
    for (language, source) in [
        (LanguageCodeIR::Korean, "기록을 저장해."),
        (LanguageCodeIR::English, "Save the record."),
    ] {
        let mut api = CognitiveApi::new_embedded().unwrap();
        let input = request("PLAN-NOT-PROMISE", 1, source, language);
        crate::natural_realization::FINAL_REALIZATION_CHECKS.with(|count| count.set(0));
        let response = api.process_conversation_turn(&input).unwrap();
        assert_eq!(
            crate::natural_realization::FINAL_REALIZATION_CHECKS.with(|count| count.get()),
            1
        );
        let original = &response.natural_realization.generation_traces[0];
        assert_eq!(
            original.context.default_speech_intent,
            GenerationSpeechIntentIR::DescribePlan
        );
        let mut expressions = ExpressionNodeStore::default();
        for selection in &original.expression_selection.selections {
            expressions.inject(selection.expression.clone()).unwrap();
        }
        let mut context = original.context.clone();
        context.default_speech_intent = GenerationSpeechIntentIR::CommitFutureAction;
        let promise = GenerativeLanguageCortex
            .generate(GenerativeLanguageRequestIR {
                meaning: original.meaning.clone(),
                context,
                expressions: &expressions,
            })
            .unwrap();
        // The sentence can be grammatically valid while its speech act is
        // unsupported by this plan-only source. Rehashing cannot authorize it.
        assert!(promise.validate());
        assert_eq!(promise.meaning, original.meaning);
        let mut coverage = response.natural_realization.coverage.clone();
        for obligation in &mut coverage.obligations {
            for hash in &mut obligation.supporting_generation_trace_sha256s {
                if *hash == original.generation_sha256 {
                    *hash = promise.generation_sha256.clone();
                }
            }
        }
        coverage.coverage_sha256 =
            crate::natural_realization::natural_realization_coverage_sha256(&coverage);
        assert!(!coverage.validate_against(
            &response.natural_realization.response_plan,
            &[promise],
            Some(&response.grounded_response.as_ref().unwrap().semantic_goal),
        ));
        assert!(response.validate_against(&input));
    }
}

#[test]
fn action_identity_reaches_bilingual_expression_without_intent_collapse() {
    for (predicate, korean, english, ko_root, en_root) in [
        (
            "SAVE",
            "문서를 저장해.",
            "Save the document.",
            "저장하는",
            "save",
        ),
        ("READ", "문서를 읽어.", "Read the document.", "읽는", "read"),
        ("OPEN", "문서를 열어.", "Open the document.", "여는", "open"),
        (
            "TRANSFORM",
            "문서를 변환해.",
            "Transform the document.",
            "변환하는",
            "transform",
        ),
        (
            "DELETE",
            "문서를 삭제해.",
            "Delete the document.",
            "삭제하는",
            "delete",
        ),
        (
            "DEPLOY",
            "문서를 배포해.",
            "Deploy the document.",
            "배포하는",
            "deploy",
        ),
        (
            "UPDATE",
            "문서를 갱신해.",
            "Update the document.",
            "갱신하는",
            "update",
        ),
        (
            "MOVE",
            "문서를 옮겨.",
            "Move the document.",
            "이동하는",
            "move",
        ),
    ] {
        for (text, root, language) in [
            (korean, ko_root, LanguageCodeIR::Korean),
            (english, en_root, LanguageCodeIR::English),
        ] {
            let mut api = CognitiveApi::new_embedded().unwrap();
            let input = request("PREDICATE-PRESERVED", 1, text, language);
            let response = api.process_conversation_turn(&input).unwrap();
            assert!(response
                .natural_realization
                .generation_traces
                .iter()
                .flat_map(|trace| &trace.speech_intent.intents)
                .filter(|intent| intent.event_node_id == "E_ACTION")
                .all(|intent| intent.intent
                    == crate::generative_language::GenerationSpeechIntentIR::DescribePlan));
            let plan = response
                .grounded_response
                .as_ref()
                .expect("typed action plan");
            assert!(
                plan.semantic_goal
                    .events
                    .iter()
                    .any(|event| event.predicate_concept_id == predicate
                        && plan
                            .semantic_goal
                            .selected_live_event_ids
                            .contains(&event.event_id)),
                "{text}"
            );
            let action_nodes = response
                .natural_realization
                .generation_traces
                .iter()
                .flat_map(|g| &g.meaning.nodes)
                .filter(|node| node.node_id == "E_ACTION")
                .collect::<Vec<_>>();
            assert!(
                action_nodes
                    .iter()
                    .any(|n| n.concept_id == format!("C_{predicate}")),
                "{text}: {action_nodes:?}"
            );
            assert!(
                response.output.text.contains(root),
                "{text}: {}",
                response.output.text
            );
            assert!(
                !response.output.text.contains("수행할게")
                    && !response.output.text.contains("will perform")
            );
            assert!(response.validate_against(&input));
            assert!(
                !response
                    .language_cortex_integration
                    .external_action_executed
            );
        }
    }
}

#[test]
fn clause_owned_task_arguments_survive_other_clause_entities() {
    for actor in ["Nerissa", "Tobias", "Yuna"] {
        for (verb, target) in [
            ("inspect", "parser"),
            ("delete", "cache"),
            ("repair", "scanner"),
        ] {
            for query in [
                format!("Tell me where {actor} read the report and {verb} the {target}."),
                format!("{verb} the {target} and tell me when {actor} read the report."),
            ] {
                let native = NativeLanguageCircuit.analyze(&query);
                assert_eq!(native.selected_live_goals.len(), 1, "{query}: {native:?}");
                let goal = &native.selected_live_goals[0];
                assert!(
                    crate::native_language_circuit::subjects_share_context_concept(
                        &goal.subject,
                        target
                    ),
                    "{query}: {goal:?}"
                );
                assert!(
                    !goal.subject.to_lowercase().contains(&actor.to_lowercase()),
                    "{query}: {goal:?}"
                );
                assert!(native.validate_for_source(&query));
            }
        }
    }
}

#[test]
fn task_target_is_not_replaced_by_embedded_question_actor() {
    for query in [
        "Tell me where Elspeth read the letter and inspect the parser.",
        "Delete the cache and tell me when Gareth read the newspaper.",
    ] {
        let mut api = CognitiveApi::new_embedded().unwrap();
        let input = request("OWNED-TARGET", 1, query, LanguageCodeIR::English);
        let response = api.process_conversation_turn(&input).unwrap();
        let plan = response.grounded_response.as_ref().expect("work retained");
        for id in &plan.semantic_goal.selected_live_event_ids {
            let event = plan
                .semantic_goal
                .events
                .iter()
                .find(|event| event.event_id == *id)
                .unwrap();
            for arg_id in &event.goal_subject_argument_ids {
                let argument = plan
                    .semantic_goal
                    .arguments
                    .iter()
                    .find(|a| a.argument_id == *arg_id)
                    .unwrap();
                assert!(!argument.grounded_label.to_lowercase().contains("elspeth"));
                assert!(!argument.grounded_label.to_lowercase().contains("gareth"));
            }
        }
        assert!(response.validate_against(&input));
    }
}

#[test]
fn information_and_work_keep_separate_obligations() {
    for (fact, query, expected, language) in [
        (
            "Isolde read the report in the solarium.",
            "Tell me who read the report and save the document.",
            "Isolde",
            LanguageCodeIR::English,
        ),
        (
            "서겸은 어제 신문을 읽었어.",
            "누가 신문을 읽었는지 알려주고 문서를 저장해.",
            "서겸",
            LanguageCodeIR::Korean,
        ),
        (
            "Lucian read the letter in the greenhouse.",
            "Save the document and tell me who read the letter.",
            "Lucian",
            LanguageCodeIR::English,
        ),
    ] {
        let mut api = CognitiveApi::new_embedded().unwrap();
        api.process_conversation_turn(&request("MIXED-EFFECTS", 1, fact, language))
            .unwrap();
        let input = request("MIXED-EFFECTS", 2, query, language);
        let result = api.process_conversation_turn(&input);
        assert!(result.is_ok(), "{query}: {result:?}");
        let response = result.unwrap();
        assert!(
            response
                .discourse_answer
                .as_ref()
                .is_some_and(|a| !a.response_parts.is_empty()),
            "{query}: {} {:?}",
            response.output.text,
            response.conversation_contract
        );
        assert!(
            response.output.text.contains(expected),
            "{query}: {}",
            response.output.text
        );
        let plan = response
            .grounded_response
            .as_ref()
            .expect("independent work retained");
        assert_eq!(
            plan.semantic_goal.selected_live_event_ids.len(),
            1,
            "{query}: {:?}",
            plan.semantic_goal
        );
        assert_eq!(
            response
                .natural_realization
                .response_plan
                .moves
                .iter()
                .filter(|m| m.response_act == NaturalResponseActIR::DiscourseAnswer)
                .count(),
            1
        );
        assert!(response.validate_against(&input));
        assert!(
            !response
                .language_cortex_integration
                .external_action_executed
        );
        assert_eq!(
            response
                .conversation_state
                .active_goals
                .iter()
                .filter(|g| g.introduced_turn == 2)
                .count(),
            1
        );
        assert!(response
            .grounded_realization
            .claims
            .iter()
            .any(|c| c.kind == crate::grounded_realization::GroundedClaimKindIR::PlanStatus));
        let mut missing_answer = response.clone();
        missing_answer.discourse_answer = None;
        assert!(!mixed_response_obligations_preserved(&missing_answer));
        let mut leaked_task = response.clone();
        let info_id = leaked_task
            .pragmatic_interpretation
            .language_center
            .events
            .iter()
            .find(|e| {
                !plan
                    .semantic_goal
                    .selected_live_event_ids
                    .contains(&e.event_id)
            })
            .unwrap()
            .event_id
            .clone();
        leaked_task
            .grounded_response
            .as_mut()
            .unwrap()
            .semantic_goal
            .selected_live_event_ids
            .push(info_id);
        assert!(!mixed_response_obligations_preserved(&leaked_task));
    }
}

#[test]
fn mixed_partition_does_not_drop_unowned_or_prohibited_clauses() {
    for text in [
        "Tell me who read the report and do not save the document.",
        "If Mira reads the report, save the document.",
        "She said: \"Tell me who read the report and save the document.\"",
        "Tell me who read the report and save the document and flerm the quux.",
    ] {
        assert!(
            crate::discourse_qa::mixed_response_task_frames(text).is_none(),
            "{text}"
        );
    }
    for verb in ["save", "delete", "inspect"] {
        for text in [
            format!("Tell me who read the report and {verb} the document."),
            format!("{verb} the document and tell me who read the report."),
        ] {
            assert_eq!(
                crate::discourse_qa::mixed_response_task_frames(&text)
                    .unwrap()
                    .len(),
                1,
                "{text}"
            );
            assert!(
                !crate::discourse_qa::is_response_operation_batch(&text),
                "mixed turn is not answer-only: {text}"
            );
        }
    }
}

#[test]
fn completed_realization_has_one_check_owner_across_integration_layers() {
    use crate::natural_realization::FINAL_REALIZATION_CHECKS;
    for (text, language) in [
        ("안녕!", LanguageCodeIR::Korean),
        ("Inspect the parser.", LanguageCodeIR::English),
        ("lamp is active.", LanguageCodeIR::English),
        ("Is lamp active?", LanguageCodeIR::English),
        ("누가 신문을 읽었는지 알려주지 마.", LanguageCodeIR::Korean),
    ] {
        let mut api = CognitiveApi::new_embedded().unwrap();
        let input = request("ONE-OUTPUT-CHECK", 1, text, language);
        FINAL_REALIZATION_CHECKS.with(|checks| checks.set(0));
        let response = api.process_conversation_turn(&input).unwrap();
        assert_eq!(
            FINAL_REALIZATION_CHECKS.with(|checks| checks.get()),
            1,
            "{text}"
        );
        // External validation starts a fresh trust boundary, also with one owner.
        FINAL_REALIZATION_CHECKS.with(|checks| checks.set(0));
        assert!(response.validate_against(&input));
        assert_eq!(
            FINAL_REALIZATION_CHECKS.with(|checks| checks.get()),
            1,
            "{text}"
        );
        let check =
            crate::natural_realization::NaturalRealizationCheck::new(&response.natural_realization);
        let mut forged = response.clone();
        forged
            .natural_realization
            .realized_text
            .push_str(" invented");
        assert!(!check.accepts(&forged.natural_realization));
        forged.natural_realization.realization_sha256 =
            crate::natural_realization::natural_realization_sha256(&forged.natural_realization);
        assert!(!check.accepts(&forged.natural_realization));
        assert!(!forged.validate_against(&input));
    }
}

#[test]
fn coordinated_positive_and_negative_requests_keep_local_polarity_and_conflict() {
    for negative_first in [false, true] {
        let mut api = CognitiveApi::new_embedded().unwrap();
        let before = api
            .process_conversation_turn(&request(
                "LOCAL-NEGATION",
                1,
                "Aurelia read a book in the gazebo.",
                LanguageCodeIR::English,
            ))
            .unwrap();
        let positive = "tell me where Aurelia read the book";
        let negative = "do not tell me where Aurelia read the book";
        let text = if negative_first {
            format!("{negative} and {positive}.")
        } else {
            format!("{positive} and {negative}.")
        };
        let input = request("LOCAL-NEGATION", 2, &text, LanguageCodeIR::English);
        let response = api.process_conversation_turn(&input).unwrap();
        assert!(response.validate_against(&input));
        assert!(
            !response.output.text.contains("gazebo"),
            "{}",
            response.output.text
        );
        assert!(response.grounded_response.is_none());
        let answer = response.discourse_answer.as_ref().unwrap();
        assert_eq!(answer.response_parts.len(), 1, "{answer:?}");
        assert!(
            answer.response_parts[0]
                .response_constraint_conflict
                .is_some(),
            "{answer:?}"
        );
        assert_eq!(
            before.conversation_state.epistemic_ledger,
            response.conversation_state.epistemic_ledger
        );
        let frames = &response
            .pragmatic_interpretation
            .compositional_analysis
            .frames;
        let matrix = frames
            .iter()
            .filter(|f| f.canonical_predicate == "COMMUNICATE")
            .collect::<Vec<_>>();
        assert_eq!(matrix.len(), 2);
        for (index, frame) in matrix.into_iter().enumerate() {
            let is_negative = if index == 0 {
                negative_first
            } else {
                !negative_first
            };
            assert_eq!(
                frame.polarity == crate::compositional_semantics::FramePolarityIR::Negative,
                is_negative
            );
        }
    }
}

#[test]
fn coordinated_request_validation_replays_parent_context_not_truncated_fragment() {
    let mut api = CognitiveApi::new_embedded().unwrap();
    api.process_conversation_turn(&request(
        "CONTEXT-REPLAY",
        1,
        "다예는 어제 잡지를 읽었어.",
        LanguageCodeIR::Korean,
    ))
    .unwrap();
    let input = request(
        "CONTEXT-REPLAY",
        2,
        "누가 잡지를 읽었는지 알려주고 언제 다예는 잡지를 읽었는지 알려줘.",
        LanguageCodeIR::Korean,
    );
    let response = api.process_conversation_turn(&input).unwrap();
    assert!(response.validate_against(&input));
    let answer = response.discourse_answer.as_ref().unwrap();
    assert_eq!(answer.response_parts.len(), 2, "{answer:?}");
    assert!(
        answer
            .response_parts
            .iter()
            .all(|p| p.content_projection.is_some()),
        "{answer:?}"
    );
    assert!(response.output.text.contains("다예"));
    assert!(response.output.text.contains("어제"));
    assert!(response.grounded_response.is_none());
    let mut forged = response.clone();
    forged.discourse_answer.as_mut().unwrap().response_parts[0]
        .question_request
        .as_mut()
        .unwrap()
        .clause_context
        .as_mut()
        .unwrap()
        .frame_id = answer.response_parts[1]
        .question_request
        .as_ref()
        .unwrap()
        .clause_context
        .as_ref()
        .unwrap()
        .frame_id
        .clone();
    assert!(!forged.validate_against(&input));
    let mut forged = response.clone();
    forged.discourse_answer.as_mut().unwrap().response_parts[0]
        .question_request
        .as_mut()
        .unwrap()
        .clause_context = None;
    assert!(!forged.validate_against(&input));
}

#[test]
fn coordinated_question_ownership_survives_contract_memory_and_answering() {
    for connector in ["and", "then", "."] {
        let mut api = CognitiveApi::new_embedded().unwrap();
        let before = api
            .process_conversation_turn(&request(
                "OWNED-QUESTIONS",
                1,
                "Rowan read the letter in the conservatory.",
                LanguageCodeIR::English,
            ))
            .unwrap();
        let text =
            format!("Tell me who read the letter {connector} tell me where Rowan read the letter.");
        let input = request("OWNED-QUESTIONS", 2, &text, LanguageCodeIR::English);
        let response = api.process_conversation_turn(&input).unwrap();
        assert!(response.validate_against(&input));
        assert!(
            response.conversation_contract.answer_only(),
            "{text}: {:?}",
            response.conversation_contract
        );
        assert!(response.grounded_response.is_none());
        let answer = response.discourse_answer.as_ref().unwrap();
        assert_eq!(answer.response_parts.len(), 2, "{text}: {answer:?}");
        assert!(
            answer
                .response_parts
                .iter()
                .all(|part| part.question_request.is_some() && part.content_projection.is_some()),
            "{text}: {answer:?}"
        );
        assert!(response.output.text.to_lowercase().contains("rowan"));
        assert!(response.output.text.contains("conservatory"));
        assert_eq!(
            before.conversation_state.epistemic_ledger,
            response.conversation_state.epistemic_ledger
        );
    }
}

#[test]
fn mixed_response_operations_conflict_cannot_disclose_forbidden_role() {
    let mut api = CognitiveApi::new_embedded().unwrap();
    let before = api
        .process_conversation_turn(&request(
            "MIXED-CONFLICT",
            1,
            "Soren read a book in the cellar.",
            LanguageCodeIR::English,
        ))
        .unwrap();
    let input = request(
        "MIXED-CONFLICT",
        2,
        "Do not tell me where Soren read the book. Tell me where Soren read the book.",
        LanguageCodeIR::English,
    );
    let response = api.process_conversation_turn(&input).unwrap();
    assert!(response.validate_against(&input));
    assert!(
        !response.output.text.contains("cellar"),
        "{}",
        response.output.text
    );
    assert!(response.grounded_response.is_none());
    let answer = response.discourse_answer.as_ref().unwrap();
    assert_eq!(answer.response_parts.len(), 1, "{answer:?}");
    assert_eq!(
        answer.response_parts[0].disposition,
        crate::discourse_qa::DiscourseAnswerDispositionIR::AmbiguousQuery
    );
    assert!(answer.evidence.is_empty());
    let mut forged = response.clone();
    forged.discourse_answer.as_mut().unwrap().response_parts[0]
        .response_constraint_conflict
        .as_mut()
        .unwrap()
        .source_text = "Tell me where Soren read the book.".into();
    assert!(!forged.validate_against(&input));
    assert_eq!(
        before.conversation_state.epistemic_ledger,
        response.conversation_state.epistemic_ledger
    );
}

#[test]
fn mixed_response_operations_preserve_multiple_questions_and_reject_unparsed_tail() {
    assert!(crate::discourse_qa::is_response_operation_batch(
        "그 사건을 요약하고 원인도 설명해줘."
    ));
    let mut api = CognitiveApi::new_embedded().unwrap();
    api.process_conversation_turn(&request(
        "MIXED-QUESTIONS",
        1,
        "Rhea read a book in the library.",
        LanguageCodeIR::English,
    ))
    .unwrap();
    let input = request(
        "MIXED-QUESTIONS",
        2,
        "Tell me who read the book. Tell me where Rhea read the book.",
        LanguageCodeIR::English,
    );
    let response = api.process_conversation_turn(&input).unwrap();
    assert!(response.validate_against(&input));
    assert!(response.output.text.to_lowercase().contains("rhea"));
    assert!(response.output.text.contains("library"));
    assert_eq!(
        response
            .discourse_answer
            .as_ref()
            .unwrap()
            .response_parts
            .len(),
        2
    );
    for text in [
        "Tell me who read the book. Tell me where Rhea read the book. Zorbulate the foobar.",
        "Mina said \"do not tell me who read\". Tell me where Rhea read the book.",
        "If you can, tell me who read the book. Tell me where Rhea read the book.",
    ] {
        assert!(
            !crate::discourse_qa::is_response_operation_batch(text),
            "{text}"
        );
    }
}

#[test]
fn mixed_response_operations_keep_positive_question_and_negative_role() {
    for (statement, question, expected, forbidden, lang) in [
        (
            "Orin read a letter in the kitchen.",
            "Do not tell me where Orin read the letter. Tell me who read it.",
            "orin",
            "kitchen",
            LanguageCodeIR::English,
        ),
        (
            "해림은 창고에서 편지를 읽었어.",
            "해림은 어디서 편지를 읽었는지 알려주지 마. 누가 편지를 읽었는지 알려줘.",
            "해림",
            "창고",
            LanguageCodeIR::Korean,
        ),
    ] {
        let mut api = CognitiveApi::new_embedded().unwrap();
        api.process_conversation_turn(&request("MIXED-RESPONSE", 1, statement, lang))
            .unwrap();
        let input = request("MIXED-RESPONSE", 2, question, lang);
        let response = api.process_conversation_turn(&input).unwrap();
        assert!(response.validate_against(&input));
        assert!(
            response.output.text.to_lowercase().contains(expected),
            "{}; {:?}; {:?}",
            response.output.text,
            response.conversation_contract,
            response.pragmatic_interpretation.compositional_analysis
        );
        assert!(!response.output.text.to_lowercase().contains(forbidden));
        assert!(response.grounded_response.is_none());
        assert_eq!(
            response
                .discourse_answer
                .as_ref()
                .unwrap()
                .response_parts
                .len(),
            1
        );
    }
}

#[test]
fn discourse_request_prefix_preserves_question_ownership_and_memory() {
    for (statement, ban, question, prefixes, expected, lang) in [
        (
            "도겸은 정원에서 신문을 읽었어.",
            "누가 신문을 읽었는지 알려주지 마.",
            "누가 신문을 읽었는지 알려 주세요.",
            vec!["이제 ", "그러면 이제 "],
            "도겸",
            LanguageCodeIR::Korean,
        ),
        (
            "Nolan read a book in the attic.",
            "Do not tell me where Nolan read the book.",
            "tell me where Nolan read the book.",
            vec!["Now ", "Well, now "],
            "attic",
            LanguageCodeIR::English,
        ),
    ] {
        for prefix in prefixes {
            let mut api = CognitiveApi::new_embedded().unwrap();
            api.process_conversation_turn(&request("DISCOURSE-REQUEST", 1, statement, lang))
                .unwrap();
            api.process_conversation_turn(&request("DISCOURSE-REQUEST", 2, ban, lang))
                .unwrap();
            let text = format!("{prefix}{question}");
            let input = request("DISCOURSE-REQUEST", 3, &text, lang);
            let response = api.process_conversation_turn(&input).unwrap();
            assert!(response.validate_against(&input));
            let answer = response.discourse_answer.as_ref().unwrap();
            let envelope = answer
                .question_request
                .as_ref()
                .unwrap_or_else(|| panic!("{text}: {answer:?}"));
            assert!(envelope.validate());
            assert!(envelope.request_lead_in.is_some());
            assert_eq!(
                envelope.question_text,
                crate::proposition_content::question_request(question)
                    .unwrap()
                    .question_text
            );
            assert!(answer.content_projection.is_some(), "{text}: {answer:?}");
            assert!(
                response.output.text.to_lowercase().contains(expected),
                "{text}: {}",
                response.output.text
            );
            assert!(response.grounded_response.is_none());
            let mut forged = response.clone();
            forged
                .discourse_answer
                .as_mut()
                .unwrap()
                .question_request
                .as_mut()
                .unwrap()
                .request_lead_in = Some("invented".into());
            assert!(!forged.validate_against(&input));
        }
    }
}

#[test]
fn response_prohibition_cannot_supersede_event_memory() {
    for (statement, ban, question, expected, lang) in [
        (
            "은솔은 어제 편지를 읽었어.",
            "이제 누가 편지를 읽었는지 알려주지 마.",
            "이제 누가 편지를 읽었는지 알려줘.",
            "은솔",
            LanguageCodeIR::Korean,
        ),
        (
            "Dorian read the paper yesterday.",
            "Now do not tell me who read the paper.",
            "Now tell me who read the paper.",
            "dorian",
            LanguageCodeIR::English,
        ),
    ] {
        let mut api = CognitiveApi::new_embedded().unwrap();
        let initial = api
            .process_conversation_turn(&request("MEMORY-ADMISSION", 1, statement, lang))
            .unwrap();
        let stopped = api
            .process_conversation_turn(&request("MEMORY-ADMISSION", 2, ban, lang))
            .unwrap();
        assert!(stopped.conversation_contract.suppresses_answer());
        assert_eq!(
            initial.conversation_state.epistemic_ledger,
            stopped.conversation_state.epistemic_ledger
        );
        let input = request("MEMORY-ADMISSION", 3, question, lang);
        let response = api.process_conversation_turn(&input).unwrap();
        assert!(response.validate_against(&input));
        assert!(
            response.output.text.to_lowercase().contains(expected),
            "{}",
            response.output.text
        );
        assert!(response
            .discourse_answer
            .as_ref()
            .unwrap()
            .content_projection
            .is_some());
    }
}

#[test]
fn discourse_request_prefix_never_strips_complement_or_negative_scope() {
    for text in [
        "Now do not tell me who read the letter.",
        "If you can, now tell me who read the letter.",
        "Mina said now tell me who read the letter.",
        "이제 누가 책을 읽었는지 알려주지 마.",
        "누가 책을 읽었는지 이제 알려줘.",
    ] {
        assert!(
            crate::proposition_content::question_request(text).is_none(),
            "{text}"
        );
    }
}

#[test]
fn event_query_entities_cannot_supply_attribution_predicates() {
    for name in ["다원", "지원", "말리", "알린"] {
        let mut api = CognitiveApi::new_embedded().unwrap();
        api.process_conversation_turn(&request(
            "EVENT-OWNER",
            1,
            &format!("{name}은 안내서를 읽었어."),
            LanguageCodeIR::Korean,
        ))
        .unwrap();
        let input = request(
            "EVENT-OWNER",
            2,
            &format!("{name}은 무엇을 읽었는지 말해 주세요."),
            LanguageCodeIR::Korean,
        );
        let response = api.process_conversation_turn(&input).unwrap();
        assert!(response.validate_against(&input));
        let answer = response.discourse_answer.as_ref().unwrap();
        assert!(answer.content_projection.is_some(), "{name}: {answer:?}");
        assert!(
            response.output.text.contains("안내서"),
            "{}",
            response.output.text
        );
        assert!(!response.output.text.contains("DIALOGUE_USER"));
    }
}

#[test]
fn response_prohibition_closes_lookup_and_final_output_paths() {
    for (statement, prohibited, secret, language) in [
        (
            "예율은 잡지를 읽었어.",
            "누가 잡지를 읽었는지 알려주지 마세요.",
            "예율",
            LanguageCodeIR::Korean,
        ),
        (
            "태린은 서재에서 책을 읽었어.",
            "태린은 어디서 책을 읽었는지 말하지 마세요.",
            "서재",
            LanguageCodeIR::Korean,
        ),
        (
            "Bren read a letter in the library.",
            "Do not tell me where Bren read the letter.",
            "library",
            LanguageCodeIR::English,
        ),
        (
            "Vera read a book.",
            "Don't tell me who read the book.",
            "Vera",
            LanguageCodeIR::English,
        ),
        (
            "Harlan read a letter.",
            "Now do not tell me who read the letter.",
            "Harlan",
            LanguageCodeIR::English,
        ),
    ] {
        let mut api = CognitiveApi::new_embedded().unwrap();
        api.process_conversation_turn(&request("RESPONSE-PROHIBITION", 1, statement, language))
            .unwrap();
        let input = request("RESPONSE-PROHIBITION", 2, prohibited, language);
        let response = api
            .process_conversation_turn(&input)
            .unwrap_or_else(|e| panic!("{prohibited}: {e:?}"));
        assert!(
            response.conversation_contract.suppresses_answer(),
            "{prohibited}: {:?}; {:?}",
            response.conversation_contract,
            response.pragmatic_interpretation.compositional_analysis
        );
        assert!(response.validate_against(&input));
        assert!(response.discourse_answer.is_none());
        assert!(response.grounded_response.is_none());
        assert!(
            !response
                .output
                .text
                .to_lowercase()
                .contains(&secret.to_lowercase()),
            "{}",
            response.output.text
        );
        assert!(!response.output.text.contains("DIALOGUE_USER"));
        assert_eq!(
            response
                .natural_realization
                .response_arbitration
                .selected_source,
            NaturalResponseSourceIR::ResponseProhibition
        );
        assert!(
            !response
                .language_cortex_integration
                .external_action_executed
        );
        let mut forged = response.clone();
        forged.discourse_answer =
            Some(crate::discourse_qa::DiscourseQaEngine.unanswered("Who read?", language));
        assert!(!forged.validate_against(&input));
    }
}

#[test]
fn response_prohibition_is_not_ordinary_negation_or_a_quoted_command() {
    for text in [
        "Why did Mina not explain the cause?",
        "왜 원인을 설명하지 않았어?",
        "Tell me who did not read the letter.",
        "If you do not tell me who read it, stop.",
        "Do not explain the cause. Tell me who read the letter.",
        "Mina said \"do not tell me who read the letter\".",
    ] {
        let analysis = crate::compositional_semantics::CompositionalSemanticAnalyzer.analyze(text);
        assert!(
            crate::conversation_contract::response_prohibition(text, &analysis).is_empty(),
            "{text}"
        );
    }
    let route = LanguagePipelineRoutingIR::from_candidates([
        Some(LanguagePipelineSignalIR::ResponseProhibited),
        Some(LanguagePipelineSignalIR::InformationRequest),
        Some(LanguagePipelineSignalIR::NormalizedGrounded),
        Some(LanguagePipelineSignalIR::DeicticQueryReferenceSafe),
        Some(LanguagePipelineSignalIR::ReferencesFullyResolved),
    ]);
    assert!(!route.allows_discourse_qa(false, false));
    assert!(!route.allows_temporal_qa());
    assert!(!route.allows_dialogue_relation_qa(false));
    assert!(!PlanProjectionDecisionIR::from_routing(&route).allows_plan());
}

#[test]
fn benefactive_request_morphology_preserves_inner_question_across_register_and_spacing() {
    let mut baseline = None;
    for ending in [
        "알려줘",
        "알려 줘",
        "알려주세요",
        "알려 주세요",
        "알려주십시오",
        "알려 주십시오",
        "알려줄래요",
        "알려 줄래요",
        "말해줘",
        "말해 주세요",
    ] {
        for punctuation in [".", "?"] {
            let question = format!("누가 잡지를 읽었는지 {ending}{punctuation}");
            let mut api = CognitiveApi::new_embedded().unwrap();
            api.process_conversation_turn(&request(
                "REQUEST-MORPHOLOGY",
                1,
                "채림은 잡지를 읽었어.",
                LanguageCodeIR::Korean,
            ))
            .unwrap();
            let input = request("REQUEST-MORPHOLOGY", 2, &question, LanguageCodeIR::Korean);
            let response = api
                .process_conversation_turn(&input)
                .unwrap_or_else(|e| panic!("{question}: {e:?}"));
            assert!(response.validate_against(&input));
            assert!(
                response.conversation_contract.answer_only(),
                "{question}: {:?}",
                response.conversation_contract
            );
            let answer = response
                .discourse_answer
                .as_ref()
                .unwrap_or_else(|| panic!("{question}: {}", response.output.text));
            assert!(
                answer.question_request.is_some(),
                "{question}: {}",
                response.output.text
            );
            assert!(
                answer.content_projection.is_some(),
                "{question}: {}",
                response.output.text
            );
            assert!(response.output.text.contains("채림"));
            assert!(!response.output.text.contains("DIALOGUE_USER"));
            assert!(response.grounded_response.is_none());
            if crate::compositional_semantics::formal_korean_benefactive(ending) {
                assert!(
                    response.output.text.ends_with("입니다."),
                    "{question}: {}",
                    response.output.text
                );
            }
            if let Some((claims, evidence)) = &baseline {
                assert_eq!(&answer.claims, claims);
                assert_eq!(&answer.evidence, evidence);
            } else {
                baseline = Some((answer.claims.clone(), answer.evidence.clone()));
            }
        }
    }
}

#[test]
fn request_ending_does_not_authorize_quoted_negated_or_conditional_content() {
    for text in ["민수가 선물을 줘.", "그가 알려주셨다.", "알려주지 마세요."] {
        assert!(
            !crate::compositional_semantics::is_korean_benefactive_request(text),
            "{text}"
        );
    }
    for text in [
        "누가 책을 읽었는지 알려주지 마세요.",
        "민수에게 누가 책을 읽었는지 알려주세요.",
        "그가 ‘누가 책을 읽었는지 알려주세요’라고 말했어.",
        "누가 책을 읽었는지 알려주시면 그만둘게.",
        "누가 책을 읽었는지 알려주세요. 파일을 삭제해줘.",
    ] {
        assert!(
            crate::proposition_content::question_request(text).is_none(),
            "{text}"
        );
    }
    for tail in [
        "주지 마세요",
        "주시면",
        "주셨다",
        "주 세요",
        "해주세요라고",
        "해 주세요 그리고 삭제해",
    ] {
        assert!(
            !crate::compositional_semantics::korean_request_tail(tail),
            "{tail}"
        );
    }
}

#[test]
fn inner_question_owns_gap_realization_even_if_lookup_hints_change() {
    for (statement, question, expected, forbidden, language) in [
        (
            "Avel wrote a message yesterday.",
            "Tell me who wrote the map.",
            "who wrote the map",
            "me who",
            LanguageCodeIR::English,
        ),
        (
            "한별은 도서관에서 책을 읽었어.",
            "누가 편지를 읽었는지 말해줘.",
            "누가 편지를",
            "말해줘",
            LanguageCodeIR::Korean,
        ),
    ] {
        let mut api = CognitiveApi::new_embedded().unwrap();
        api.process_conversation_turn(&request("GAP-OWNER", 1, statement, language))
            .unwrap();
        let input = request("GAP-OWNER", 2, question, language);
        let response = api.process_conversation_turn(&input).unwrap();
        assert!(response.validate_against(&input));
        assert!(
            response.output.text.contains(expected),
            "{}",
            response.output.text
        );
        assert!(!response.output.text.contains(forbidden));
        assert!(response.grounded_response.is_none());
        let mut answer = response.discourse_answer.unwrap();
        assert!(answer.question_request.is_some());
        assert!(answer.evidence.is_empty());
        let baseline = crate::generative_language::generate_discourse_answer_from_knowledge(
            language,
            &answer,
            &[],
        )
        .unwrap();
        answer.query.topic_terms = vec!["OUTER_REQUEST_MUST_NOT_REPLACE_INNER_QUERY".into()];
        let changed = crate::generative_language::generate_discourse_answer_from_knowledge(
            language,
            &answer,
            &[],
        )
        .unwrap();
        assert_eq!(
            baseline.morphology.realized_text,
            changed.morphology.realized_text
        );
    }
}

#[test]
fn focused_answer_ellipsis_preserves_claims_and_source() {
    for (statement, short, detailed, expected, language) in [
        (
            "Tarin wrote a letter yesterday.",
            "Tell me who wrote the letter.",
            "Tell me who wrote the letter in detail.",
            "tarin",
            LanguageCodeIR::English,
        ),
        (
            "소연은 교실에서 책을 읽었어.",
            "소연은 어디서 책을 읽었는지 알려줘.",
            "소연은 어디서 책을 읽었는지 자세히 알려줘.",
            "교실",
            LanguageCodeIR::Korean,
        ),
    ] {
        let mut baseline = None;
        for (index, text) in [short, detailed].into_iter().enumerate() {
            let mut api = CognitiveApi::new_embedded().unwrap();
            api.process_conversation_turn(&request("FOCUSED-ANSWER", 1, statement, language))
                .unwrap();
            let input = request("FOCUSED-ANSWER", 2, text, language);
            let response = api.process_conversation_turn(&input).unwrap();
            assert!(response.validate_against(&input));
            assert!(
                response.output.text.to_lowercase().contains(expected),
                "{}",
                response.output.text
            );
            let answer = response.discourse_answer.as_ref().unwrap();
            let generated = crate::generative_language::generate_discourse_answer_from_knowledge(
                language,
                answer,
                &[],
            )
            .unwrap();
            if index == 0 {
                assert!(!response.output.text.contains("the actor"));
                assert!(!response.output.text.contains("장소는"));
                assert!(!response
                    .output
                    .text
                    .contains(if language == LanguageCodeIR::Korean {
                        "네 말로는"
                    } else {
                        "from what you told me"
                    }));
                assert!(generated
                    .meaning
                    .nodes
                    .iter()
                    .any(|n| n.concept_id.starts_with("C_CONTENT_RECALL_")));
                assert!(generated.meaning.nodes.iter().any(|n| n
                    .grounding_refs
                    .iter()
                    .any(|r| r.starts_with("DIALOGUE_BELIEF_ID:"))));
                baseline = Some((
                    answer.claims.clone(),
                    answer.evidence.clone(),
                    generated.morphology.realized_text,
                ));
            } else {
                let (claims, evidence, surface) = baseline.as_ref().unwrap();
                assert_eq!(&answer.claims[..claims.len()], claims);
                assert_eq!(answer.claims.len(), claims.len() + 1);
                assert!(answer
                    .content_projection
                    .as_ref()
                    .unwrap()
                    .elaboration_event
                    .is_some());
                assert!(!response.output.text.contains("the actor is"));
                assert!(!response.output.text.contains("장소는"));
                assert!(generated
                    .meaning
                    .nodes
                    .iter()
                    .any(|n| n.concept_id.starts_with("C_EVENT_RECAP_")));
                assert_eq!(&answer.evidence, evidence);
                assert_ne!(&generated.morphology.realized_text, surface);
            }
            assert!(!answer.dialogue_truth_established);
            assert!(
                !response
                    .language_cortex_integration
                    .external_action_executed
            );
        }
    }
}

#[test]
fn focused_english_answers_keep_source_determiners_and_case_markers() {
    for (statement, question, expected) in [
        (
            "Liora wrote a note in the studio.",
            "Where did Liora write a note?",
            "in the studio.",
        ),
        (
            "Iven read a catalogue.",
            "What did Iven read?",
            "a catalogue.",
        ),
        (
            "Mina lent a book to Jin.",
            "To whom did Mina lend a book?",
            "to jin.",
        ),
    ] {
        let mut api = CognitiveApi::new_embedded().unwrap();
        api.process_conversation_turn(&request(
            "FOCUS-PHRASE",
            1,
            statement,
            LanguageCodeIR::English,
        ))
        .unwrap();
        let input = request("FOCUS-PHRASE", 2, question, LanguageCodeIR::English);
        let response = api.process_conversation_turn(&input).unwrap();
        assert!(response.validate_against(&input));
        assert!(
            response.output.text.to_lowercase().starts_with(expected),
            "{}",
            response.output.text
        );
        assert!(!response.output.text.contains("from what you told me"));
        let answer = response.discourse_answer.unwrap();
        assert!(answer.validate());
        assert!(!answer.dialogue_truth_established);
        assert!(answer.content_projection.is_some());
    }
}

#[test]
fn question_envelope_reuses_direct_role_queries_and_evidence() {
    for (statement, direct, wrapped, value, language) in [
        (
            "Tarin wrote a letter yesterday.",
            "Who wrote that letter?",
            "Could you tell me who wrote that letter?",
            "tarin",
            LanguageCodeIR::English,
        ),
        (
            "Nora read a book in the library.",
            "Where did Nora read a book?",
            "Tell me where Nora read a book.",
            "library",
            LanguageCodeIR::English,
        ),
        (
            "Mina read a book yesterday.",
            "When did Mina read a book?",
            "Please tell me when Mina read a book.",
            "yesterday",
            LanguageCodeIR::English,
        ),
        (
            "Briar read a manual.",
            "What did Briar read?",
            "Tell me what Briar read.",
            "manual",
            LanguageCodeIR::English,
        ),
        (
            "소연은 도서관에서 책을 읽었어.",
            "누가 책을 읽었어?",
            "누가 책을 읽었는지 말해줘.",
            "소연",
            LanguageCodeIR::Korean,
        ),
        (
            "해온은 어제 편지를 읽었어.",
            "언제 해온은 편지를 읽었어?",
            "언제 해온은 편지를 읽었는지 알려줘.",
            "어제",
            LanguageCodeIR::Korean,
        ),
    ] {
        let envelope = crate::proposition_content::question_request(wrapped).unwrap_or_else(|| {
            panic!(
                "{wrapped}: {:?}",
                crate::compositional_semantics::CompositionalSemanticAnalyzer.analyze(wrapped)
            )
        });
        assert!(envelope.validate());
        let mut baseline = None;
        for question in [direct, wrapped] {
            let mut api = CognitiveApi::new_embedded().unwrap();
            api.process_conversation_turn(&request("QUESTION-ENVELOPE", 1, statement, language))
                .unwrap();
            let input = request("QUESTION-ENVELOPE", 2, question, language);
            let response = api
                .process_conversation_turn(&input)
                .unwrap_or_else(|e| panic!("{question}: {e:?}"));
            assert!(response.validate_against(&input));
            assert!(response.conversation_contract.answer_only());
            assert!(response.grounded_response.is_none());
            assert!(
                !response
                    .language_cortex_integration
                    .external_action_executed
            );
            let answer = response.discourse_answer.as_ref().expect("role answer");
            let projection = answer
                .content_projection
                .as_ref()
                .unwrap_or_else(|| panic!("{question}: {}", response.output.text));
            assert!(
                projection.binding.value.to_lowercase().contains(value),
                "{question}: {}",
                response.output.text
            );
            if let Some((evidence, claims)) = &baseline {
                assert_eq!(&answer.evidence, evidence);
                assert_eq!(&answer.claims, claims);
                let receipt = answer.question_request.as_ref().expect("request receipt");
                assert_eq!(receipt.question_text, envelope.question_text);
                assert_eq!(receipt.response_manner, envelope.response_manner);
                assert_eq!(
                    receipt.source_text,
                    response.normalization.semantic_surface_text
                );
                let mut removed = response.clone();
                removed.discourse_answer.as_mut().unwrap().question_request = None;
                assert!(!removed.validate_against(&input));
                let mut forged = answer.clone();
                forged.question_request.as_mut().unwrap().question_text =
                    "Who wrote a different letter?".into();
                assert!(!forged.validate());
            } else {
                baseline = Some((answer.evidence.clone(), answer.claims.clone()));
            }
        }
    }
}

#[test]
fn question_envelope_cannot_erase_scope_recipient_or_unconsumed_content() {
    for text in [
        "Tell Rowan who wrote that letter.",
        "Do not tell me who wrote that letter.",
        "I told you who wrote that letter.",
        "If you tell me who wrote that letter, stop.",
        "Tell me who wrote that letter and delete the file.",
        "Tell me \"who wrote that letter\".",
        "민수에게 누가 책을 읽었는지 말해줘.",
        "누가 책을 읽었는지 말하지 마.",
    ] {
        assert!(
            crate::proposition_content::question_request(text).is_none(),
            "{text}"
        );
    }
}

#[test]
fn direct_contextual_cause_uses_the_same_relation_as_embedded_request() {
    let mut baseline = None;
    for text in ["왜 그랬어?", "왜 그랬는지 간단히 알려줘."] {
        let mut api = CognitiveApi::new_embedded().unwrap();
        api.process_conversation_turn(&request(
            "DIRECT-CAUSE",
            1,
            "휴대폰이 고장났기 때문에 나율은 안내서를 읽었어.",
            LanguageCodeIR::Korean,
        ))
        .unwrap();
        let input = request("DIRECT-CAUSE", 2, text, LanguageCodeIR::Korean);
        let response = api.process_conversation_turn(&input).unwrap();
        assert!(
            response.output.text.contains("휴대폰"),
            "{text}: {}",
            response.output.text
        );
        assert!(response.validate_against(&input));
        let answer = response.discourse_answer.unwrap();
        if let Some(evidence) = &baseline {
            assert_eq!(&answer.evidence, evidence);
        } else {
            baseline = Some(answer.evidence);
        }
    }
}

#[test]
fn embedded_information_is_answer_content_not_a_communication_job() {
    use crate::proposition_content::{content_request, ResponseMannerIR};
    for (statement, question, expected, language) in [
        (
            "컴퓨터가 멈췄기 때문에 채온은 안내서를 읽었어.",
            "왜 그랬는지 좀 자세히 말해줘.",
            "컴퓨터",
            LanguageCodeIR::Korean,
        ),
        (
            "Briar read a workbook because the phone was flickering.",
            "Please explain that reason in more detail.",
            "phone",
            LanguageCodeIR::English,
        ),
        (
            "Mina read a book because the screen was damaged.",
            "Tell me why that happened in more detail.",
            "screen",
            LanguageCodeIR::English,
        ),
        (
            "The cache failed because the disk was full.",
            "Tell me why the cache failed.",
            "disk",
            LanguageCodeIR::English,
        ),
    ] {
        let parsed = content_request(question).unwrap_or_else(|| {
            panic!(
                "{question}: {:?}",
                crate::compositional_semantics::CompositionalSemanticAnalyzer.analyze(question)
            )
        });
        assert!(parsed.validate());
        if question.contains("detail") || question.contains("자세히") {
            assert_eq!(parsed.response_manner, Some(ResponseMannerIR::Detailed));
        }
        let mut api = CognitiveApi::new_embedded().unwrap();
        api.process_conversation_turn(&request("EMBEDDED-INFORMATION", 1, statement, language))
            .unwrap();
        let input = request("EMBEDDED-INFORMATION", 2, question, language);
        let response = api.process_conversation_turn(&input).unwrap();
        assert!(
            response.conversation_contract.answer_only(),
            "{question}: {:?}",
            response.conversation_contract
        );
        assert!(
            response.output.text.contains(expected),
            "{question}: {}",
            response.output.text
        );
        assert!(response
            .discourse_answer
            .as_ref()
            .is_some_and(|a| a.content_projection.is_some()));
        assert!(response.grounded_response.is_none());
        assert!(response
            .conversation_state
            .action_state_ledger
            .records
            .is_empty());
        assert!(response.validate_against(&input));
    }
}

#[test]
fn content_request_does_not_erase_recipient_scope_or_external_task() {
    use crate::proposition_content::content_request;
    for text in [
        "Tell Alice why that happened.",
        "민수에게 왜 그랬는지 말해줘.",
        "Do not tell me why that happened.",
        "I told you why that happened.",
        "If you tell me why that happened, stop.",
        "Tell me why that happened and delete the file.",
        "Tell me \"why that happened\".",
        "Tell me why that happened, then send a report.",
        "Tell me why this happened not in more detail.",
    ] {
        assert!(content_request(text).is_none(), "{text}");
    }
    let mut api = CognitiveApi::new_embedded().unwrap();
    let input = request(
        "RECIPIENT-SCOPE",
        1,
        "Tell Alice why that happened.",
        LanguageCodeIR::English,
    );
    let response = api.process_conversation_turn(&input).unwrap();
    assert!(!response.conversation_contract.answer_only());
    assert!(
        !response
            .language_cortex_integration
            .external_action_executed
    );
    assert!(response.validate_against(&input));
}

#[test]
fn content_request_preserves_target_and_manner_as_independent_axes() {
    use crate::proposition_content::{content_request, ContentSlotIR, ResponseMannerIR};
    for (text, target, manner) in [
        (
            "Please explain this reason in detail.",
            None,
            Some(ResponseMannerIR::Detailed),
        ),
        (
            "그 원인을 자세히 설명해줘.",
            None,
            Some(ResponseMannerIR::Detailed),
        ),
        (
            "새 모터 진동의 원인을 설명해줘.",
            Some("새 모터 진동"),
            None,
        ),
        (
            "그 공장 정전의 원인을 짧게 설명해줘.",
            Some("그 공장 정전"),
            Some(ResponseMannerIR::Concise),
        ),
        (
            "Briefly explain the cause of the elevator outage.",
            Some("the elevator outage"),
            Some(ResponseMannerIR::Concise),
        ),
    ] {
        let parsed = content_request(text).unwrap_or_else(|| panic!("{text}"));
        assert!(parsed.validate());
        assert_eq!(parsed.slot, ContentSlotIR::Cause);
        assert_eq!(parsed.target_surface.as_deref(), target);
        assert_eq!(parsed.response_manner, manner);
        let mut forged = parsed.clone();
        forged.target_surface = Some("another target".into());
        assert!(!forged.validate());
    }
    for text in [
        "Do not explain its cause in detail.",
        "Explain its cause, then delete the file.",
        "If you explain the cause in detail, stop.",
        "Explain \"the cause of the outage\".",
        "Explain the cause of.",
        "Explain its cause not in detail.",
    ] {
        assert!(content_request(text).is_none(), "{text}");
    }
}

#[test]
fn relational_manner_does_not_change_retrieved_evidence() {
    for (statement, plain, variants, language) in [
        (
            "Corin read a manual because the screen was flickering.",
            "Explain this reason.",
            vec![
                "Please explain this reason in detail.",
                "Briefly explain this reason.",
            ],
            LanguageCodeIR::English,
        ),
        (
            "인터넷이 끊겼기 때문에 라온은 책을 읽었어.",
            "그 원인을 설명해줘.",
            vec!["그 원인을 자세히 설명해줘.", "짧게 그 원인을 설명해줘."],
            LanguageCodeIR::Korean,
        ),
    ] {
        let mut baseline = None;
        for question in std::iter::once(plain).chain(variants) {
            let mut api = CognitiveApi::new_embedded().unwrap();
            api.process_conversation_turn(&request("MANNER-IDENTITY", 1, statement, language))
                .unwrap();
            let input = request("MANNER-IDENTITY", 2, question, language);
            let response = api.process_conversation_turn(&input).unwrap();
            let answer = response.discourse_answer.as_ref().expect("content answer");
            assert!(
                answer.content_projection.is_some(),
                "{question}: {}",
                response.output.text
            );
            if let Some(evidence) = baseline.as_ref() {
                assert_eq!(&answer.evidence, evidence);
            } else {
                baseline = Some(answer.evidence.clone());
            }
            assert!(response.validate_against(&input));
            assert!(response.grounded_response.is_none());
            let mut removed = answer.clone();
            removed.content_request = None;
            assert!(!removed.validate());
        }
    }
}

#[test]
fn explicit_relational_gap_cannot_be_relabeled_by_prior_subject() {
    for (statement, question, expected, forbidden, language) in [
        (
            "글씨가 흐렸기 때문에 도윤은 설명서를 읽었어.",
            "새 모터 진동의 원인을 설명해줘.",
            "새 모터 진동의 원인",
            "읽었어",
            LanguageCodeIR::Korean,
        ),
        (
            "비가 그쳤기 때문에 유란은 편지를 읽었어.",
            "그 공장 정전의 원인을 설명해줘.",
            "그 공장 정전의 원인",
            "읽었어",
            LanguageCodeIR::Korean,
        ),
        (
            "The elevator door opened because the motor stopped.",
            "Please explain the cause of the elevator outage.",
            "elevator outage",
            "motor stopped",
            LanguageCodeIR::English,
        ),
    ] {
        let mut api = CognitiveApi::new_embedded().unwrap();
        api.process_conversation_turn(&request("GAP-TARGET", 1, statement, language))
            .unwrap();
        let input = request("GAP-TARGET", 2, question, language);
        let response = api.process_conversation_turn(&input).unwrap();
        let answer = response.discourse_answer.as_ref().expect("gap answer");
        assert!(answer.evidence.is_empty());
        assert!(
            response.output.text.contains(expected),
            "{}",
            response.output.text
        );
        assert!(
            !response.output.text.contains(forbidden),
            "{}",
            response.output.text
        );
        assert!(response.validate_against(&input));
        let mut changed = answer.clone();
        changed.query.topic_terms = vec!["an unrelated topic".into()];
        // Gap surface depends on the source-bound request, not mutable lookup terms.
        let realized = crate::generative_language::generate_discourse_answer_from_knowledge(
            language,
            &changed,
            &[],
        )
        .unwrap();
        assert!(realized.morphology.realized_text.contains(expected));
    }
}

#[test]
fn contextual_relation_binds_memory_before_searching_for_a_cause() {
    use crate::proposition_content::{contextual_content_slot, ContentSlotIR};
    for question in [
        "Please explain the cause.",
        "Explain its cause.",
        "그 원인을 설명해줘.",
    ] {
        assert_eq!(
            contextual_content_slot(question),
            Some(ContentSlotIR::Cause),
            "{question}: {:?}",
            crate::compositional_semantics::CompositionalSemanticAnalyzer.analyze(question)
        );
    }
    for question in [
        "Explain the cause of the flood.",
        "창고 정전의 원인을 설명해줘.",
        "Do not explain its cause.",
        "Explain why the server failed.",
        "If you explain its cause, stop.",
        "Explain \"its cause\".",
    ] {
        assert_eq!(contextual_content_slot(question), None, "{question}");
    }
    for (statement, question, value, language) in [
        (
            "Vera read a brochure because the screen was damaged.",
            "Please explain the cause and summarize this event.",
            "screen",
            LanguageCodeIR::English,
        ),
        (
            "Elian moved the cart because the aisle was narrow.",
            "Do not summarize the event. Explain its cause.",
            "aisle",
            LanguageCodeIR::English,
        ),
        (
            "화면이 고장났기 때문에 윤서는 설명서를 읽었어.",
            "그 원인을 설명해줘.",
            "화면",
            LanguageCodeIR::Korean,
        ),
    ] {
        let mut api = CognitiveApi::new_embedded().unwrap();
        api.process_conversation_turn(&request("CONTEXT-CAUSE", 1, statement, language))
            .unwrap();
        let input = request("CONTEXT-CAUSE", 2, question, language);
        let response = api.process_conversation_turn(&input).unwrap();
        assert!(
            response.output.text.contains(value),
            "{question}: {}; answer={:?}; records={:?}",
            response.output.text,
            response.discourse_answer,
            response.conversation_state.epistemic_ledger.records
        );
        assert!(response.validate_against(&input));
        assert!(response.grounded_response.is_none());
        let answer = response
            .discourse_answer
            .as_ref()
            .expect("attributed response");
        let leaf = std::iter::once(answer)
            .chain(&answer.response_parts)
            .find(|a| a.contextual_target.is_some())
            .expect("target receipt");
        assert!(leaf.validate());
        let mut forged = leaf.clone();
        forged.contextual_target.as_mut().unwrap().belief_id = "wrong-record".into();
        assert!(!forged.validate());
        let mut ablated = response.conversation_state.clone();
        ablated
            .epistemic_ledger
            .records
            .retain(|r| r.belief_id != leaf.contextual_target.as_ref().unwrap().belief_id);
        assert!(!leaf.validate_response_part_memory(&ablated));
    }
}

#[test]
fn explicit_genitive_target_is_not_dropped_by_contextual_binding() {
    let mut api = CognitiveApi::new_embedded().unwrap();
    api.process_conversation_turn(&request(
        "EXPLICIT-CAUSE",
        1,
        "비가 왔기 때문에 세라는 안내서를 읽었어.",
        LanguageCodeIR::Korean,
    ))
    .unwrap();
    let input = request(
        "EXPLICIT-CAUSE",
        2,
        "창고 정전의 원인을 설명해줘.",
        LanguageCodeIR::Korean,
    );
    let response = api.process_conversation_turn(&input).unwrap();
    let answer = response.discourse_answer.as_ref().expect("evidence gap");
    assert!(answer.evidence.is_empty(), "{}", response.output.text);
    assert!(answer.contextual_target.is_none());
    assert!(response.validate_against(&input));
}

#[test]
fn contextual_relation_does_not_use_an_older_cause_for_a_new_event() {
    use crate::discourse_qa::{DiscourseAnswerDispositionIR, DiscourseQaEngine};
    let mut api = CognitiveApi::new_embedded().unwrap();
    for (index, text) in [
        "Nora read a booklet because the printer was broken.",
        "Explain its cause.",
        "Lena moved a crate.",
    ]
    .iter()
    .enumerate()
    {
        api.process_conversation_turn(&request(
            "CONTEXT-NEW",
            index as u64 + 1,
            text,
            LanguageCodeIR::English,
        ))
        .unwrap();
    }
    let input = request(
        "CONTEXT-NEW",
        4,
        "Explain its cause.",
        LanguageCodeIR::English,
    );
    let response = api.process_conversation_turn(&input).unwrap();
    let answer = response
        .discourse_answer
        .as_ref()
        .expect("explicit evidence gap");
    assert_ne!(
        answer.disposition,
        DiscourseAnswerDispositionIR::AnsweredFromDialogueRecords
    );
    assert!(answer.evidence.is_empty());
    assert_eq!(
        answer
            .contextual_target
            .as_ref()
            .expect("new target, absent relation")
            .introduced_turn,
        3
    );
    assert!(!response.output.text.contains("printer"));
    assert!(response.validate_against(&input));
    let no_context = DiscourseQaEngine.answer("Explain its cause.", None, LanguageCodeIR::English);
    assert!(no_context.is_none_or(|a| a.evidence.is_empty()));
}

#[test]
fn contextual_reference_requires_live_unambiguous_attributed_memory() {
    use crate::discourse_qa::DiscourseQaEngine;
    use crate::epistemic::BeliefRecordStatusIR;
    let mut api = CognitiveApi::new_embedded().unwrap();
    for (i, text) in [
        "Nora read a booklet because the printer was broken.",
        "Who read it?",
    ]
    .iter()
    .enumerate()
    {
        api.process_conversation_turn(&request(
            "CONTEXT-FOCUS",
            i as u64 + 1,
            text,
            LanguageCodeIR::English,
        ))
        .unwrap();
    }
    let input = request(
        "CONTEXT-FOCUS",
        3,
        "Please explain its cause.",
        LanguageCodeIR::English,
    );
    let response = api.process_conversation_turn(&input).unwrap();
    assert!(
        response.output.text.contains("printer"),
        "{}; answer={:?}; focus={:?}; records={:?}",
        response.output.text,
        response.discourse_answer,
        response.conversation_state.answer_focus,
        response.conversation_state.epistemic_ledger.records
    );
    assert!(response.validate_against(&input));
    let target = response
        .discourse_answer
        .as_ref()
        .unwrap()
        .contextual_target
        .as_ref()
        .unwrap();
    let mut state = response.conversation_state.clone();
    for mode in ["retracted", "hypothetical", "ambiguous"] {
        let mut changed = state.clone();
        if mode == "ambiguous" {
            changed.answer_focus = None;
            changed.completed_turns = 1;
            let mut extra = changed
                .epistemic_ledger
                .records
                .iter()
                .find(|r| r.belief_id == target.belief_id)
                .unwrap()
                .clone();
            extra.belief_id = "OTHER-SOURCE".into();
            extra.source_actor = "OTHER".into();
            changed.epistemic_ledger.records.push(extra);
        } else {
            let r = changed
                .epistemic_ledger
                .records
                .iter_mut()
                .find(|r| r.belief_id == target.belief_id)
                .unwrap();
            if mode == "retracted" {
                r.status = BeliefRecordStatusIR::Retracted;
            } else {
                r.signature.modal_world = crate::modality::ModalWorldIR::Hypothetical;
            }
        }
        let answer = DiscourseQaEngine
            .answer(
                "Explain its cause.",
                Some(&changed),
                LanguageCodeIR::English,
            )
            .unwrap();
        assert!(answer.evidence.is_empty(), "{mode}: {answer:?}");
    }
    state.answer_focus = None;
    let answer = DiscourseQaEngine.answer(
        "Explain the cause of the flood.",
        Some(&state),
        LanguageCodeIR::English,
    );
    assert!(answer.is_none_or(|a| a.contextual_target.is_none() && a.evidence.is_empty()));
}

#[test]
fn korean_response_operations_and_unanswered_parts_are_retained() {
    let mut api = CognitiveApi::new_embedded().unwrap();
    api.process_conversation_turn(&request(
        "KO-PARTS",
        1,
        "인쇄기가 고장났기 때문에 민수는 안내서를 읽었어.",
        LanguageCodeIR::Korean,
    ))
    .unwrap();
    let input = request(
        "KO-PARTS",
        2,
        "그 일을 요약해줘. 이유를 설명해줘.",
        LanguageCodeIR::Korean,
    );
    let response = api.process_conversation_turn(&input).unwrap();
    let answer = response.discourse_answer.as_ref().unwrap_or_else(|| {
        panic!(
            "{}; contract={:?}; frames={:?}; clauses={:?}",
            response.output.text,
            response.conversation_contract,
            response
                .pragmatic_interpretation
                .compositional_analysis
                .frames,
            response
                .pragmatic_interpretation
                .compositional_analysis
                .clause_graph
        )
    });
    assert_eq!(answer.response_parts.len(), 2, "{}", response.output.text);
    assert!(
        response.output.text.contains("안내서"),
        "{}",
        response.output.text
    );
    assert!(
        response.output.text.contains("인쇄기"),
        "{}",
        response.output.text
    );
    assert!(response.validate_against(&input));

    let mut empty = CognitiveApi::new_embedded().unwrap();
    let input = request(
        "PARTS-GAP",
        1,
        "Could you summarize the event and explain its cause?",
        LanguageCodeIR::English,
    );
    let response = empty.process_conversation_turn(&input).unwrap();
    let answer = response.discourse_answer.as_ref().expect("gap response");
    assert_eq!(answer.response_parts.len(), 2, "{}", response.output.text);
    assert!(answer.response_parts.iter().all(|p| p.evidence.is_empty()));
    assert_ne!(
        answer.disposition,
        crate::discourse_qa::DiscourseAnswerDispositionIR::AnsweredFromDialogueRecords
    );
    assert!(response.grounded_response.is_none());
    assert!(response.validate_against(&input));
}

#[test]
fn excluded_matrix_content_does_not_override_acknowledgement() {
    for text in [
        "Do not explain why Jori deleted the note; just acknowledge my message.",
        "Do not summarize the incident; acknowledge my request.",
        "Please acknowledge my message.",
    ] {
        let mut api = CognitiveApi::new_embedded().unwrap();
        let input = request("ACK-REQUEST", 1, text, LanguageCodeIR::English);
        let response = api.process_conversation_turn(&input).unwrap();
        assert!(
            response.conversation_contract.interaction_only(),
            "{text}: {:?}",
            response.conversation_contract
        );
        assert!(
            response.discourse_answer.is_none(),
            "{text}: {}",
            response.output.text
        );
        assert!(response.grounded_response.is_none());
        assert!(response.output.text.len() < 60, "{}", response.output.text);
        assert!(response.validate_against(&input));
    }
}

#[test]
fn response_operations_reach_output_with_separate_evidence() {
    let mut api = CognitiveApi::new_embedded().unwrap();
    api.process_conversation_turn(&request(
        "RESPONSE-PARTS",
        1,
        "Nora read a booklet because the printer was broken.",
        LanguageCodeIR::English,
    ))
    .unwrap();
    let input = request(
        "RESPONSE-PARTS",
        2,
        "Could you summarize the event and explain why Nora read the booklet?",
        LanguageCodeIR::English,
    );
    let response = api.process_conversation_turn(&input).unwrap();
    let answer = response.discourse_answer.as_ref().expect("typed answer");
    assert_eq!(answer.response_parts.len(), 2, "{}", response.output.text);
    assert!(answer.response_parts.iter().all(|a| a.validate()));
    assert!(
        response.output.text.contains("read a booklet"),
        "{}; summary={:?}; records={:?}",
        response.output.text,
        answer.response_parts[0],
        response.conversation_state.epistemic_ledger.records
    );
    assert!(
        response.output.text.contains("printer was broken"),
        "{}",
        response.output.text
    );
    assert!(response.grounded_response.is_none());
    assert!(response.validate_against(&input));
    let follow = request("RESPONSE-PARTS", 3, "Who read it?", LanguageCodeIR::English);
    let followed = api.process_conversation_turn(&follow).unwrap();
    assert!(
        followed.output.text.to_lowercase().contains("nora"),
        "{}",
        followed.output.text
    );
    assert!(followed.validate_against(&follow));
    let exclusion = request(
        "RESPONSE-PARTS",
        4,
        "Do not explain the cause. Summarize the event.",
        LanguageCodeIR::English,
    );
    let excluded = api.process_conversation_turn(&exclusion).unwrap();
    let parts = &excluded
        .discourse_answer
        .as_ref()
        .expect("positive operation survives exclusion")
        .response_parts;
    assert_eq!(parts.len(), 1, "{}", excluded.output.text);
    assert!(parts[0].event_summary.is_some(), "{}", excluded.output.text);
    assert!(!excluded.output.text.contains("printer"));
    assert!(excluded.validate_against(&exclusion));
    let mut missing = answer.clone();
    missing.response_parts.pop();
    assert!(!missing.validate());
    let mut swapped = answer.clone();
    swapped.response_parts.reverse();
    assert!(!swapped.validate());
    let mut unsupported = answer.clone();
    unsupported.response_parts[0].realized_text = "The work is complete.".into();
    assert!(!unsupported.validate());
}

#[test]
fn content_complement_authority_survives_full_pipeline() {
    use crate::compositional_semantics::FrameMoodIR;
    for (language, raw) in [
        (
            LanguageCodeIR::English,
            "Could you explain whether Mara deleted the archive?",
        ),
        (
            LanguageCodeIR::English,
            "Please explain how Ivo deployed the service.",
        ),
        (
            LanguageCodeIR::English,
            "Could you explain why Mara did not delete the archive?",
        ),
        (
            LanguageCodeIR::English,
            "Could you explain why Ivo explained the result?",
        ),
        (LanguageCodeIR::Korean, "파일을 삭제했는지 설명해줘."),
        (LanguageCodeIR::Korean, "문서를 수정했는지 설명해줘."),
    ] {
        let mut api = CognitiveApi::new_embedded().unwrap();
        let input = request("CONTENT-AUTHORITY", 1, raw, language);
        let response = api.process_conversation_turn(&input).unwrap();
        let analysis = &response.pragmatic_interpretation.compositional_analysis;
        assert!(
            analysis
                .frames
                .iter()
                .any(|f| f.mood == FrameMoodIR::ContentComplement),
            "{raw}: {:?}",
            analysis.frames
        );
        for frame in analysis
            .frames
            .iter()
            .filter(|f| f.mood == FrameMoodIR::ContentComplement)
        {
            assert!(!frame.external_execution_authorized, "{raw}: {frame:?}");
            assert!(
                analysis
                    .candidates
                    .iter()
                    .filter(|c| c.source_frame_id == frame.frame_id)
                    .all(|c| !c.external_execution_authorized),
                "{raw}"
            );
            assert!(
                response
                    .conversation_contract
                    .request_effects
                    .iter()
                    .all(|e| e.source_id != frame.frame_id),
                "{raw}"
            );
        }
        assert!(
            !response.conversation_contract.independent_action_requested,
            "{raw}: {:?}",
            response.conversation_contract
        );
        assert!(
            response.grounded_response.is_none(),
            "{raw}: {}",
            response.output.text
        );
        assert!(response.validate_against(&input), "{raw}");
    }
}

#[test]
fn matrix_request_does_not_authorize_its_content_predicates() {
    use crate::compositional_semantics::{CompositionalSemanticAnalyzer, FrameMoodIR};
    for head in ["explain", "investigate"] {
        for embedded in [
            "the curtain moved",
            "Mara deleted the file",
            "Ivo deployed the service",
        ] {
            for marker in ["why", "how", "whether"] {
                let raw = format!("Could you {head} {marker} {embedded}?");
                let a = CompositionalSemanticAnalyzer.analyze(&raw);
                assert!(
                    a.frames
                        .iter()
                        .any(|f| f.mood == FrameMoodIR::ContentComplement),
                    "{raw}: {:?}",
                    a.frames
                );
                assert!(a
                    .frames
                    .iter()
                    .filter(|f| f.mood == FrameMoodIR::ContentComplement)
                    .all(|f| !f.external_execution_authorized));
                assert!(a.grammatical_scope_graph.edges.iter().any(|e| e.kind
                    == crate::grammatical_scope::GrammaticalScopeEdgeKindIR::ContentComplement));
                assert!(a.grammatical_scope_graph.validate());
            }
        }
    }
    // Unknown predicates remain an unparsed matrix argument, not a fabricated
    // executable frame. This checks authority, not vocabulary coverage.
    for raw in [
        "Could you explain why the file changed?",
        "Could you explain why the window opened?",
    ] {
        let a = CompositionalSemanticAnalyzer.analyze(raw);
        assert!(a.frames.iter().all(|f| f.intent_hint
            == dockable_semantic_core::PlanIntentIR::Explain
            || !f.external_execution_authorized));
    }
    for raw in ["파일을 삭제했는지 설명해줘.", "문서를 수정했는지 확인해줘."]
    {
        let a = CompositionalSemanticAnalyzer.analyze(raw);
        assert!(
            a.frames
                .iter()
                .any(|f| f.mood == FrameMoodIR::ContentComplement),
            "{raw}: {:?}",
            a.frames
        );
    }
    let mut api = CognitiveApi::new_embedded().unwrap();
    api.process_conversation_turn(&request(
        "MATRIX-CONTENT",
        1,
        "The curtain moved because the window opened.",
        LanguageCodeIR::English,
    ))
    .unwrap();
    let input = request(
        "MATRIX-CONTENT",
        2,
        "Could you briefly explain why the curtain moved?",
        LanguageCodeIR::English,
    );
    let response = api.process_conversation_turn(&input).unwrap();
    assert!(
        response.conversation_contract.answer_only(),
        "{:?}",
        response.conversation_contract
    );
    assert!(
        response.output.text.contains("window opened"),
        "{}",
        response.output.text
    );
    assert!(response.grounded_response.is_none());
    assert!(response
        .pragmatic_interpretation
        .compositional_analysis
        .frames
        .iter()
        .filter(|f| f.mood == FrameMoodIR::ContentComplement)
        .all(|f| !f.external_execution_authorized));
    assert!(response.validate_against(&input));
}

#[test]
fn complete_conversation_preference_keeps_excluded_operations_inactive() {
    for (language, text) in [
        (
            LanguageCodeIR::Korean,
            "정리해달라는 건 아니고, 그냥 잠깐 같이 얘기하자.",
        ),
        (
            LanguageCodeIR::English,
            "I don't want a solution. Just chat with me.",
        ),
    ] {
        let mut api = CognitiveApi::new_embedded().unwrap();
        let input = request("CONTENT-EXCLUSION", 1, text, language);
        let response = api.process_conversation_turn(&input).unwrap();
        assert!(
            response.conversation_contract.interaction_only(),
            "{text}: {:?}",
            response.conversation_contract
        );
        assert!(!response.conversation_contract.independent_action_requested);
        assert!(
            response.grounded_response.is_none(),
            "{}",
            response.output.text
        );
        assert!(!response.output.text.contains("unresolved_subject"));
        assert!(response.validate_against(&input));
    }
}

#[test]
fn response_manner_does_not_replace_content_operation() {
    use crate::proposition_content::{InteractionModeIR, ResponseMannerIR};
    for (language, source, question, fragment, mode) in [
        (
            LanguageCodeIR::English,
            "Iris sent an envelope to Pavel yesterday.",
            "Could you briefly summarize what happened?",
            "sent an envelope to pavel",
            InteractionModeIR::Summary,
        ),
        (
            LanguageCodeIR::English,
            "The kiln cooled because the fuel valve closed.",
            "Could you briefly explain why the kiln cooled?",
            "fuel valve closed",
            InteractionModeIR::Explanation,
        ),
        (
            LanguageCodeIR::Korean,
            "소연은 마당에서 잡지를 읽었어.",
            "방금 그 일을 간단히 요약해줘.",
            "소연",
            InteractionModeIR::Summary,
        ),
    ] {
        let mut api = CognitiveApi::new_embedded().unwrap();
        api.process_conversation_turn(&request("MANNER-CONTENT", 1, source, language))
            .unwrap();
        let input = request("MANNER-CONTENT", 2, question, language);
        let response = api.process_conversation_turn(&input).unwrap();
        let preference = response
            .conversation_contract
            .interaction_preference
            .as_ref()
            .unwrap_or_else(|| {
                panic!(
                    "{question}: {} {:?}",
                    response.output.text, response.conversation_contract
                )
            });
        assert_eq!(preference.desired, mode);
        assert_eq!(preference.response_manner, Some(ResponseMannerIR::Concise));
        assert!(response.conversation_contract.answer_only());
        assert_eq!(
            response.natural_realization.response_act,
            NaturalResponseActIR::DiscourseAnswer,
            "{question}: {}",
            response.output.text
        );
        assert!(
            response.output.text.to_lowercase().contains(fragment),
            "{}",
            response.output.text
        );
        assert!(response.validate_against(&input));
        assert!(response.grounded_response.is_none());
        assert!(
            !response
                .language_cortex_integration
                .external_action_executed
        );
    }
}

#[test]
fn nominal_request_carrier_preserves_operation_identity() {
    for noun in ["요약", "설명", "삭제", "수정"] {
        for carrier in ["부탁해", "부탁할게", "부탁드려요", "부탁드립니다"] {
            let raw = format!("자료 {noun} {carrier}.");
            let parsed =
                crate::compositional_semantics::CompositionalSemanticAnalyzer.analyze(&raw);
            assert!(
                parsed
                    .frames
                    .iter()
                    .any(|f| f.external_execution_authorized),
                "{raw}: {:?}",
                parsed.frames
            );
        }
    }
    for raw in [
        "자료 요약 부탁하지 않아.",
        "자료 요약 부탁하면 알려줘.",
        "자료 요약 부탁했다고 말했어.",
    ] {
        let parsed = crate::compositional_semantics::CompositionalSemanticAnalyzer.analyze(raw);
        assert!(
            !parsed
                .frames
                .iter()
                .any(|f| f.canonical_predicate == "SUMMARIZE" && f.external_execution_authorized),
            "{raw}"
        );
    }
    let mut api = CognitiveApi::new_embedded().unwrap();
    api.process_conversation_turn(&request(
        "NOMINAL-CONTENT",
        1,
        "태린은 식당에서 신문을 읽었어.",
        LanguageCodeIR::Korean,
    ))
    .unwrap();
    let input = request(
        "NOMINAL-CONTENT",
        2,
        "그 일 요약 부탁드려요.",
        LanguageCodeIR::Korean,
    );
    let response = api.process_conversation_turn(&input).unwrap();
    assert!(
        response
            .discourse_answer
            .as_ref()
            .is_some_and(|a| a.event_summary.is_some()),
        "{}",
        response.output.text
    );
    assert!(
        response.output.text.contains("신문을 읽었습니다"),
        "{}",
        response.output.text
    );
    assert!(response.validate_against(&input));
    assert!(response.grounded_response.is_none());
}

#[test]
fn recap_operation_resolves_its_event_and_does_not_replay_a_role_answer() {
    for (source, question, followup, expected) in [
        (
            "Lena lent a map to Oskar.",
            "To whom did Lena lend a map?",
            "Please summarize that.",
            "lent a map to oskar",
        ),
        (
            "Dario did not read the letter in the cabin.",
            "Who did not read the letter?",
            "Will you summarize that event?",
            "did not read the letter in the cabin",
        ),
    ] {
        let mut api = CognitiveApi::new_embedded().unwrap();
        for (i, text) in [source, question].into_iter().enumerate() {
            api.process_conversation_turn(&request(
                "OPERATION-REFERENCE",
                i as u64 + 1,
                text,
                LanguageCodeIR::English,
            ))
            .unwrap();
        }
        let input = request("OPERATION-REFERENCE", 3, followup, LanguageCodeIR::English);
        let response = api.process_conversation_turn(&input).unwrap();
        assert!(
            response.output.text.to_lowercase().contains(expected),
            "{}",
            response.output.text
        );
        let answer = response.discourse_answer.as_ref().unwrap();
        assert!(answer.event_summary.is_some(), "{answer:?}");
        assert!(answer.reformulated_request.is_none());
        assert!(answer.content_projection.is_none());
        assert!(response.validate_against(&input));
        assert!(response.grounded_response.is_none());
    }
}

#[test]
fn affect_policy_changes_the_public_response_plan_not_semantic_obligations() {
    use crate::affective_field::AffectiveFieldIR;
    let mut neutral_api = CognitiveApi::new_embedded().unwrap();
    let mut urgent_api = CognitiveApi::new_embedded().unwrap();
    let input = request(
        "BREVITY",
        1,
        "I'm upset. Investigate the cache.",
        LanguageCodeIR::English,
    );
    urgent_api.affective_memory.insert(
        "BREVITY".into(),
        AffectiveFieldIR::observe(None, "urgent urgent urgent", None),
    );
    let neutral = neutral_api.process_conversation_turn(&input).unwrap();
    let urgent = urgent_api.process_conversation_turn(&input).unwrap();
    assert_eq!(
        neutral.natural_realization.response_act,
        urgent.natural_realization.response_act
    );
    assert!(urgent.affective_policy.brevity_millis > 150);
    assert!(
        neutral.natural_realization.response_plan.moves.len()
            > urgent.natural_realization.response_plan.moves.len(),
        "neutral={} urgent={}",
        neutral.output.text,
        urgent.output.text
    );
    assert_eq!(neutral.request_semantics, urgent.request_semantics);
    assert!(urgent.validate_against(&input));
    assert!(urgent.natural_realization.coverage.orphan_generation_traces == 0);
    let neutral_traces = &neutral.natural_realization.generation_traces;
    let urgent_traces = &urgent.natural_realization.generation_traces;
    let removed_auxiliary = neutral_traces.len() - urgent_traces.len();
    assert_eq!(
        neutral_traces[removed_auxiliary..]
            .iter()
            .map(|trace| &trace.meaning)
            .collect::<Vec<_>>(),
        urgent_traces
            .iter()
            .map(|trace| &trace.meaning)
            .collect::<Vec<_>>()
    );
    println!(
        "BREVITY_NEUTRAL={}\nBREVITY_URGENT={}",
        neutral.output.text, urgent.output.text
    );
}

#[test]
fn playful_social_input_reaches_public_output_with_no_authority() {
    for (text, language, marker) in [
        ("ㅋㅋ 고마워", LanguageCodeIR::Korean, "ㅎㅎ"),
        ("haha thanks", LanguageCodeIR::English, "Heh,"),
    ] {
        let mut api = CognitiveApi::new_embedded().unwrap();
        let input = request("PLAYFUL-API", 1, text, language);
        let response = api.process_conversation_turn(&input).unwrap();
        assert!(
            response.output.text.contains(marker),
            "{}",
            response.output.text
        );
        assert!(response.validate_against(&input));
        assert!(response.grounded_response.is_none());
        assert!(response
            .conversation_state
            .action_state_ledger
            .records
            .is_empty());
        println!(
            "PLAYFUL_INPUT={text}\nPLAYFUL_OUTPUT={}",
            response.output.text
        );
    }
}

#[test]
fn reformulation_requeries_evidence_and_preserves_answer_slot() {
    for (id, language, report, question, followup) in [
        (
            "REFORM-KO",
            LanguageCodeIR::Korean,
            "민수가 보고서를 수정했어.",
            "누가 수정했어?",
            "핵심만 다시 설명해.",
        ),
        (
            "REFORM-EN",
            LanguageCodeIR::English,
            "Mina said that Lumen failed because DeltaWorker stopped.",
            "Why did Lumen fail?",
            "Explain that again briefly.",
        ),
    ] {
        let mut api = CognitiveApi::new_embedded().unwrap();
        api.process_conversation_turn(&request(id, 1, report, language))
            .unwrap();
        let first = api
            .process_conversation_turn(&request(id, 2, question, language))
            .unwrap();
        let prior = first.discourse_answer.as_ref().unwrap();
        assert!(prior.content_projection.is_some());
        let next_request = request(id, 3, followup, language);
        let next = api.process_conversation_turn(&next_request).unwrap();
        let answer = next.discourse_answer.as_ref().unwrap();
        assert_eq!(
            next.natural_realization.response_act,
            NaturalResponseActIR::DiscourseAnswer,
            "{}",
            next.output.text
        );
        assert!(answer.reformulated_request.is_some(), "{answer:#?}");
        assert_eq!(prior.content_projection, answer.content_projection);
        assert_eq!(prior.claims, answer.claims);
        assert!(next.validate_against(&next_request));
        assert!(next
            .conversation_state
            .action_state_ledger
            .records
            .is_empty());
        println!(
            "REFORMULATION_INPUT={followup}\nREFORMULATION_OUTPUT={}",
            next.output.text
        );

        // Adversarial state ablation: removing current semantic evidence must
        // remove the answer even though the question focus still exists.
        let mut state = next.conversation_state.clone();
        state.epistemic_ledger.records.clear();
        let missing = crate::discourse_qa::DiscourseQaEngine
            .reformulate(followup, Some(&state), language)
            .unwrap();
        assert!(missing.content_projection.is_none());
        assert!(!missing
            .claims
            .iter()
            .any(|claim| claim.value == prior.claims[0].value));
        let mut expired = next.conversation_state.clone();
        expired.completed_turns += 4;
        assert!(crate::discourse_qa::DiscourseQaEngine
            .reformulate(followup, Some(&expired), language)
            .is_none());
        assert!(crate::discourse_qa::DiscourseQaEngine
            .reformulate(followup, None, language)
            .is_none());
        assert!(crate::discourse_qa::DiscourseQaEngine
            .reformulate(
                "Explain Beryl again.",
                Some(&next.conversation_state),
                language
            )
            .is_none());

        api.process_conversation_turn(&request(id, 4, "Investigate Beryl queue.", language))
            .unwrap();
        assert!(api
            .conversation_memory
            .state(id)
            .unwrap()
            .answer_focus
            .is_none());
    }
}

#[test]
fn reformulation_grammar_rejects_new_topics_quotes_negation_and_mixed_actions() {
    use crate::discourse_qa::is_answer_reformulation as accepts;
    for text in [
        "다시 설명해.",
        "그걸 짧게 다시 말해.",
        "Please rephrase your previous answer.",
        "Explain it again in detail.",
    ] {
        assert!(accepts(text), "{text}");
    }
    for text in [
        "Explain Beryl again.",
        "Do not repeat that.",
        "다시 설명하지 마.",
        "Explain it again and delete the file.",
        "\"Explain it again\"",
        "누가 수정했어?",
    ] {
        assert!(!accepts(text), "{text}");
    }
}

#[test]
fn playful_morphology_is_selected_before_generation_and_meaning_preserving() {
    use crate::affective_field::AffectiveRealizationPolicyIR;
    use crate::generative_language::{
        generate_dialogue_response_from_knowledge, GenerationDialogueResponseKindIR,
        GenerationSettings,
    };
    for language in [LanguageCodeIR::Korean, LanguageCodeIR::English] {
        for kind in [
            GenerationDialogueResponseKindIR::Greeting,
            GenerationDialogueResponseKindIR::Gratitude,
        ] {
            let neutral = generate_dialogue_response_from_knowledge(language, kind).unwrap();
            let policy = AffectiveRealizationPolicyIR {
                playfulness_millis: 800,
                ..Default::default()
            };
            let playful = generate_dialogue_response_from_knowledge(
                GenerationSettings::with_policy(language, &policy),
                kind,
            )
            .unwrap();
            assert!(playful.validate());
            assert_eq!(neutral.meaning, playful.meaning);
            assert_eq!(neutral.speech_intent, playful.speech_intent);
            assert_eq!(neutral.syntax_plan, playful.syntax_plan);
            assert_ne!(
                neutral.morphology.realized_text,
                playful.morphology.realized_text
            );
            assert_eq!(
                playful
                    .morphology
                    .tokens
                    .iter()
                    .filter(|token| token.grammar_rule_id.as_deref()
                        == Some("GRAMMAR_SOCIAL_PLAYFUL_MARKER"))
                    .count(),
                1
            );
            let urgent = generate_dialogue_response_from_knowledge(
                GenerationSettings::with_policy(
                    language,
                    &AffectiveRealizationPolicyIR {
                        urgency_millis: 800,
                        ..policy
                    },
                ),
                kind,
            )
            .unwrap();
            assert!(!urgent
                .morphology
                .tokens
                .iter()
                .any(|token| token.grammar_rule_id.as_deref()
                    == Some("GRAMMAR_SOCIAL_PLAYFUL_MARKER")));
            let neutral_again = generate_dialogue_response_from_knowledge(
                GenerationSettings::with_policy(language, &Default::default()),
                kind,
            )
            .unwrap();
            assert_eq!(neutral.morphology, neutral_again.morphology);
        }
    }
}

/// Exhaust the declared finite route dimensions, not all possible sentences.
/// Each row supplies competing module signals, including a misleading native
/// plan candidate. This tests the actual central projection/arbitration code.
#[test]
fn finite_category_route_matrix_544_cells() {
    let categories = [
        "question",
        "explanation",
        "command",
        "correction",
        "cancellation",
        "disagreement",
        "agreement",
        "condition",
        "hypothesis",
        "past_result",
        "followup",
        "topic_transition",
        "affect",
        "social",
        "ambiguous_reference",
        "multiple_goals",
        "fragment",
    ];
    let mut cells = 0;
    for category in categories {
        for flags in 0..32 {
            let multi_turn = flags & 1 != 0;
            let explicit = flags & 2 != 0;
            let negative = flags & 4 != 0;
            let emotional = flags & 8 != 0;
            let known_reference = flags & 16 != 0;
            let information = matches!(
                category,
                "question" | "explanation" | "past_result" | "followup"
            );
            let non_action = matches!(
                category,
                "disagreement" | "agreement" | "hypothesis" | "affect" | "social"
            );
            let ambiguous = category == "ambiguous_reference" || (!explicit && !known_reference);
            use LanguagePipelineSignalIR as S;
            let routing = LanguagePipelineRoutingIR::from_candidates([
                Some(S::NormalizedGrounded),
                Some(S::GroundedDisposition),
                Some(S::SemanticGoalAvailable),
                Some(S::NativeGoalOwnsTurn),
                information.then_some(S::InformationRequest),
                non_action.then_some(S::AssertionOnly),
                explicit.then_some(S::ExplicitSelectedRequest),
                (multi_turn && category == "past_result").then_some(S::PlanResultOwnsTurn),
                negative.then_some(S::InteractionBoundaryOwnsTurn),
                emotional.then_some(S::AffectOnly),
                ambiguous.then_some(S::AmbiguousInput),
                (!ambiguous).then_some(S::ReferencesFullyResolved),
            ]);
            let plan = PlanProjectionDecisionIR::from_routing(&routing);
            if information || non_action || negative || ambiguous {
                assert!(
                    !plan.allows_plan(),
                    "category={category} flags={flags} blockers={:?}",
                    plan.blockers
                );
            }
            let mut candidates = vec![NaturalResponseCandidateIR::new(
                NaturalResponseSourceIR::NativePlan,
                NaturalResponseActIR::PlanPreview,
                "competing plan",
            )];
            if information {
                candidates.push(NaturalResponseCandidateIR::new(
                    NaturalResponseSourceIR::InformationAnswer,
                    NaturalResponseActIR::DiscourseAnswer,
                    "answer obligation",
                ));
            }
            if ambiguous {
                candidates.push(NaturalResponseCandidateIR::new(
                    NaturalResponseSourceIR::Clarification,
                    NaturalResponseActIR::ClarificationRequest,
                    "unresolved reference",
                ));
            }
            let arbitration = arbitrate_natural_response(candidates.clone());
            candidates.reverse();
            assert_eq!(
                arbitration.selected_act,
                arbitrate_natural_response(candidates).selected_act
            );
            if ambiguous {
                assert_eq!(
                    arbitration.selected_act,
                    NaturalResponseActIR::ClarificationRequest
                );
            } else if information {
                assert_eq!(
                    arbitration.selected_act,
                    NaturalResponseActIR::DiscourseAnswer
                );
            }
            cells += 1;
        }
    }
    assert_eq!(cells, 544);
}

fn request(id: &str, turn: u64, text: &str, language: LanguageCodeIR) -> ConversationTurnRequestIR {
    ConversationTurnRequestIR {
        schema: crate::conversation::CONVERSATION_TURN_REQUEST_SCHEMA.into(),
        conversation_id: id.into(),
        request_id: format!("{id}-{turn}"),
        turn_index: turn,
        raw_text: text.into(),
        modality: crate::conversation::ConversationInputModalityIR::Text,
        input_confidence_millis: 1000,
        alternatives: vec![],
        output_language: Some(language),
        context_tags: vec![],
        max_plan_steps: 16,
    }
}

#[test]
fn dialogue_role_slots_are_answered_without_reissuing_the_reported_action() {
    for (index, (actor, object)) in [("민수", "보고서"), ("유나", "파일"), ("지민", "설정")]
        .iter()
        .enumerate()
    {
        let mut api = CognitiveApi::new_embedded().unwrap();
        let id = format!("ROLE-{index}");
        let first = api
            .process_conversation_turn(&request(
                &id,
                1,
                &format!("{actor}가 {object}를 수정했어."),
                LanguageCodeIR::Korean,
            ))
            .unwrap();
        assert!(
            first.grounded_response.is_none(),
            "report became a plan: {}",
            first.output.text
        );
        for (turn, text, expected) in [(2, "누가 수정했어?", *actor), (3, "뭘 수정했어?", *object)]
        {
            let response = api
                .process_conversation_turn(&request(&id, turn, text, LanguageCodeIR::Korean))
                .unwrap();
            assert!(
                response.grounded_response.is_none(),
                "{text}: {:?}",
                response.conversation_contract
            );
            let projection = response
                .discourse_answer
                .as_ref()
                .and_then(|answer| answer.content_projection.as_ref());
            assert_eq!(
                projection.map(|projection| projection.binding.value.as_str()),
                Some(expected),
                "{}",
                response.output.text
            );
            assert!(response.output.text.contains(expected));
            assert!(response
                .conversation_state
                .action_state_ledger
                .records
                .is_empty());
        }
    }
}

#[test]
fn causal_slots_are_retrieved_from_reported_content_without_truth_promotion() {
    for (index, (statement, question, expected, language)) in [
        (
            "Mina says the Lumen cache is stale because the worker stopped.",
            "Why is the Lumen cache stale?",
            "the worker stopped",
            LanguageCodeIR::English,
        ),
        (
            "Joon says the Orion task failed because the disk filled.",
            "Why did the Orion task fail?",
            "the disk filled",
            LanguageCodeIR::English,
        ),
        (
            "서버 중단 때문에 요청이 실패했어.",
            "왜 실패했어?",
            "서버 중단 때문에",
            LanguageCodeIR::Korean,
        ),
    ]
    .into_iter()
    .enumerate()
    {
        let mut api = CognitiveApi::new_embedded().unwrap();
        let id = format!("CAUSE-{index}");
        api.process_conversation_turn(&request(&id, 1, statement, language))
            .unwrap();
        let response = api
            .process_conversation_turn(&request(&id, 2, question, language))
            .unwrap();
        let answer = response
            .discourse_answer
            .as_ref()
            .expect("typed content answer");
        assert_eq!(
            answer
                .content_projection
                .as_ref()
                .map(|p| p.binding.value.as_str()),
            Some(expected),
            "{}",
            response.output.text
        );
        assert!(!answer.dialogue_truth_established);
        assert!(!response.action_state_analysis.has_language_reports());
    }
}

#[test]
fn unsupported_explanations_are_gaps_not_promises_and_queries_do_not_create_actions() {
    for (id, language, turns) in [
        (
            "GAP-KO",
            LanguageCodeIR::Korean,
            vec![
                "캐시가 뭔지 설명해.",
                "왜 필요한데?",
                "계획 말고 지금 설명해.",
            ],
        ),
        (
            "GAP-EN",
            LanguageCodeIR::English,
            vec![
                "Explain what a cache is.",
                "Why is it useful?",
                "Do not tell me your plan. Answer the question.",
            ],
        ),
    ] {
        let mut api = CognitiveApi::new_embedded().unwrap();
        for (index, text) in turns.into_iter().enumerate() {
            let response = api
                .process_conversation_turn(&request(id, index as u64 + 1, text, language))
                .unwrap();
            assert!(
                response.grounded_response.is_none(),
                "{text}: {:?}",
                response.conversation_contract
            );
            assert_ne!(
                response.natural_realization.response_act,
                NaturalResponseActIR::PlanPreview
            );
            assert!(!response
                .reference_resolution
                .resolved_semantic_text
                .contains("why is why"));
            assert!(response
                .conversation_state
                .action_state_ledger
                .records
                .is_empty());
        }
    }
}

#[test]
fn affect_changes_realization_but_cannot_change_meaning_or_authority() {
    use crate::affective_field::AffectiveFieldIR;
    let mut api = CognitiveApi::new_embedded().unwrap();
    api.process_conversation_turn(&request(
        "AFFECT-BOUNDARY",
        1,
        "민수가 보고서를 수정했어.",
        LanguageCodeIR::Korean,
    ))
    .unwrap();
    let query = request(
        "AFFECT-BOUNDARY",
        2,
        "누가 수정했어?",
        LanguageCodeIR::Korean,
    );
    let response = api.process_conversation_turn(&query).unwrap();
    use crate::generative_language::{
        generate_discourse_answer_from_knowledge, GenerationSettings,
    };
    let answer = response.discourse_answer.as_ref().unwrap();
    let refs = vec!["TEST:AFFECT".to_string()];
    let neutral =
        generate_discourse_answer_from_knowledge(LanguageCodeIR::Korean, answer, &refs).unwrap();
    let field = AffectiveFieldIR::observe(None, "알려 주세요.", None);
    let formal = generate_discourse_answer_from_knowledge(
        GenerationSettings::with_policy(LanguageCodeIR::Korean, &field.policy()),
        answer,
        &refs,
    )
    .unwrap();
    assert!(neutral.validate() && formal.validate());
    assert_ne!(
        neutral.morphology.realized_text,
        formal.morphology.realized_text
    );
    assert_eq!(neutral.meaning, formal.meaning);
    assert_eq!(neutral.speech_intent, formal.speech_intent);
    assert_eq!(
        neutral.verification.covered_meaning_node_ids,
        formal.verification.covered_meaning_node_ids
    );
    assert!(!formal.language_can_execute && !formal.semantic_authority);
    let playful_fact = generate_discourse_answer_from_knowledge(
        GenerationSettings::with_policy(
            LanguageCodeIR::Korean,
            &crate::affective_field::AffectiveRealizationPolicyIR {
                playfulness_millis: 1000,
                ..Default::default()
            },
        ),
        answer,
        &refs,
    )
    .unwrap();
    assert_eq!(neutral.morphology, playful_fact.morphology);
    let mut forged_morphology = neutral.clone();
    forged_morphology.morphology.tokens[0].surface = "unsupported grammar claim".into();
    forged_morphology.generation_sha256 =
        crate::generative_language::generative_language_sha256(&forged_morphology);
    assert!(!forged_morphology.validate());
    let mut forged = response.clone();
    forged.conversation_contract.independent_action_requested = true;
    assert!(!forged.validate_against(&query));
    let mut forged_answer = response.discourse_answer.clone().unwrap();
    forged_answer.claims[0].value = "a fabricated actor".into();
    assert!(!forged_answer.validate());
}

#[test]
fn content_compilation_keeps_case_and_does_not_guess_ambiguous_causality() {
    use crate::proposition_content::{ContentSlotIR, PropositionContentIR};
    let text = "The Q17 cache failed because DeltaWorker stopped.";
    let content = PropositionContentIR::compile(text);
    assert!(content.validate_source(text));
    assert!(content
        .bindings
        .iter()
        .any(|binding| binding.slot == ContentSlotIR::Cause
            && binding.value == "DeltaWorker stopped."));
    let mut forged = content;
    forged.bindings[0].value = "invented".into();
    assert!(!forged.validate_source(text));
    let ambiguous = PropositionContentIR::compile("파일을 열어서 내용을 수정했어.");
    assert!(!ambiguous
        .bindings
        .iter()
        .any(|binding| binding.slot == ContentSlotIR::Cause));
}

#[test]
fn affect_evidence_has_bounds_decay_negation_and_no_invented_timing() {
    use crate::affective_field::{AffectAxisIR as A, AffectiveFieldIR as F};
    let positive = F::observe(None, "happy", None);
    let negative = F::observe(None, "not happy", None);
    assert!(positive.value(A::Valence) > 0 && negative.value(A::Valence) < 0);
    assert!(F::observe(None, "\"urgent fuck!!!\"", None)
        .observations
        .is_empty());
    let timed = F::observe(None, "", Some(4000));
    assert!(timed.axes.is_empty());
    assert_eq!(timed.response_interval_ms, Some(4000));
    let punctuation = F::observe(None, "!!!", None);
    assert!(punctuation.value(A::Arousal) > 0);
    assert_eq!(punctuation.value(A::Confrontation), 0);
    let mut accumulated = F::observe(None, "urgent!", None);
    for _ in 0..50 {
        accumulated = F::observe(Some(&accumulated), "urgent urgent!!!", None);
    }
    assert!(accumulated.validate());
    assert!(
        F::observe(Some(&accumulated), "ordinary input", None).value(A::Urgency)
            < accumulated.value(A::Urgency)
    );
}

/// End-to-end category smoke coverage is separate from the 544 typed-signal
/// combinations. Passing these checks says nothing about arbitrary paraphrases.
#[test]
fn seventeen_categories_reach_a_valid_committed_response() {
    let cases: [(&str, &[&str]); 17] = [
        ("question", &["민수가 보고서를 수정했어.", "누가 수정했어?"]),
        (
            "explanation",
            &[
                "Mina says the cache failed because the disk filled.",
                "Why did the cache fail?",
            ],
        ),
        ("command", &["Inspect the Aster cache."]),
        (
            "correction",
            &[
                "Inspect the Aster cache.",
                "No, do not inspect it; explain why it failed.",
            ],
        ),
        ("cancellation", &["Run the build.", "Cancel it."]),
        (
            "disagreement",
            &["Mina says the cache is stale.", "I disagree."],
        ),
        ("agreement", &["Mina says the cache is stale.", "I agree."]),
        ("condition", &["If the tests pass, deploy the bundle."]),
        ("hypothesis", &["Suppose the cache failed."]),
        (
            "past_result",
            &["Inspect the Aster cache.", "Has it been executed?"],
        ),
        (
            "followup",
            &[
                "민수가 보고서를 수정했어.",
                "누가 수정했어?",
                "뭘 수정했어?",
            ],
        ),
        (
            "topic_transition",
            &[
                "Inspect the Aster cache.",
                "Let's talk about the Beryl queue.",
            ],
        ),
        ("affect", &["답답해."]),
        ("social", &["고마워."]),
        ("ambiguous_reference", &["Inspect it."]),
        (
            "multiple_goals",
            &["Inspect the Aster cache, then repair the Beryl queue."],
        ),
        ("fragment", &["음... 저기..."]),
    ];
    for (category, turns) in cases {
        let mut api = CognitiveApi::new_embedded().unwrap();
        for (index, text) in turns.iter().enumerate() {
            let input = request(category, index as u64 + 1, text, LanguageCodeIR::English);
            let response = api
                .process_conversation_turn(&input)
                .unwrap_or_else(|error| panic!("category={category} turn={index}: {error:?}"));
            assert!(response.validate_against(&input));
            if matches!(
                category,
                "hypothesis" | "affect" | "social" | "ambiguous_reference" | "fragment"
            ) {
                assert!(
                    response.grounded_response.is_none(),
                    "category={category}: {}",
                    response.output.text
                );
                assert!(response
                    .conversation_state
                    .action_state_ledger
                    .records
                    .is_empty());
            }
            assert_eq!(response.natural_realization.stage_overwrite_count, 0);
            if response.conversation_contract.answer_only() {
                assert!(response.grounded_response.is_none());
                assert_ne!(
                    response.natural_realization.response_act,
                    NaturalResponseActIR::PlanPreview
                );
            }
            assert!(
                !response
                    .language_cortex_integration
                    .external_action_executed
            );
            assert_eq!(response.language_cortex_integration.external_llm_calls, 0);
        }
    }
}

#[test]
fn cause_questions_without_causal_knowledge_do_not_become_execution_status_queries() {
    for (id, statement, question) in [
        (
            "CAUSE-GAP-KO",
            "서버가 멈춰서 요청이 실패했어.",
            "왜 실패했어?",
        ),
        (
            "CAUSE-GAP-EN",
            "The cache failed.",
            "Why did the cache fail?",
        ),
    ] {
        let mut api = CognitiveApi::new_embedded().unwrap();
        let report = api
            .process_conversation_turn(&request(id, 1, statement, LanguageCodeIR::English))
            .unwrap();
        assert!(report.grounded_response.is_none());
        let response = api
            .process_conversation_turn(&request(id, 2, question, LanguageCodeIR::English))
            .unwrap();
        assert_eq!(
            response.natural_realization.response_act,
            NaturalResponseActIR::DiscourseAnswer
        );
        assert_eq!(
            response.discourse_answer.as_ref().unwrap().disposition,
            crate::discourse_qa::DiscourseAnswerDispositionIR::NoMatchingRecord
        );
        assert!(response
            .conversation_state
            .action_state_ledger
            .records
            .is_empty());
    }
}
