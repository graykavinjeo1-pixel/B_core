//! Clause construction from a sealed core decision, never from the input string.
//! State predicates, object identifiers, polarity and epistemic force are separate
//! nodes. The same clause grammar is used for every object and property.

use super::*;
use crate::world_dialogue::{WorldAtomIR, WorldMemoryUpdateIR, WorldReasoningIR};
use crate::world_vocabulary::{english_third_person, WorldLexicalGrammarIR, WorldVocabularyIR};

pub(crate) fn generate_decision_inquiry(
    settings: impl Into<GenerationSettings>,
    inquiry: &crate::utterance_intent::DecisionInquiryIR,
) -> Result<GenerativeLanguageIR, String> {
    let settings = settings.into();
    let language = settings.language;
    use crate::utterance_intent::DecisionInputIR;
    if !inquiry.validate() {
        return Err("INVALID_DECISION_INQUIRY".into());
    }
    if let Some(assessment) = &inquiry.assessment {
        return generate_action_benefit(settings, assessment, inquiry.explanation_of.is_some());
    }
    if let Some(reply) = inquiry
        .clarification_reply
        .as_ref()
        .filter(|r| r.abstention.is_some() && !r.explains_question)
    {
        return generate_optional_clarification_response(settings, reply);
    }
    if let Some(selection) = &inquiry.choice_selection {
        return generate_decision_choice_selection(settings, selection);
    }
    let option_evidence_gap = crate::utterance_intent::decision_choice_needs_option_evidence(inquiry);
    // A declarative reply can supply the requested decision context even
    // though it is not another question.  Acknowledge only that this
    // source-bound context will govern the still-open decision; do not turn
    // it into an asserted world fact or fabricate a recommendation.
    if !option_evidence_gap
        && (!inquiry.context_evidence.is_empty() || !inquiry.inline_context.is_empty())
        && inquiry.explanation_of.is_none()
        && inquiry.knowledge_gap.is_none()
        && inquiry.resumption.is_none()
    {
        return generate_decision_context_received(settings);
    }
    if inquiry.explanation_of.is_none() {
        if let Some((gap, query)) = inquiry.knowledge_gap.as_ref().and_then(|g| {
            g.state_question(inquiry.proposed_action.as_ref()?, inquiry.continues_context)
                .map(|q| (g, q))
        }) {
            let world = crate::world_dialogue::deliberate_world(&gap.basis, &query)?;
            return generate_world_decision(settings, &world);
        }
    }
    use crate::world_dialogue::ActionBenefitGapReasonIR as G;
    let gap = inquiry.knowledge_gap.as_ref().map(|g| g.reason);
    let ask_gap = gap.is_some_and(|g| {
        matches!(
            g,
            G::Actor | G::CurrentState | G::ConflictingState | G::RecentState
        )
    });
    let (ko, en) = if option_evidence_gap {
        (
            "각 선택지의 조건 정보",
            "information about each option's conditions",
        )
    } else if let Some(reason) = gap {
        match reason {
            G::Actor => ("누가 행동할지", "who would act"),
            G::CurrentState => ("지금 어떤 상태인지", "what the current state is"),
            G::ConflictingState => (
                "어느 상태 정보가 맞는지",
                "which state information is correct",
            ),
            G::RecentState => ("지금도 같은 상태인지", "whether that state still holds"),
            G::NegatedActionEffect => (
                "행동하지 않을 때의 효과에 관한 근거",
                "evidence about the effects of not acting",
            ),
            G::ActionRoles => (
                "이 행동 구조에 적용할 판단 근거",
                "reasoning support for this action structure",
            ),
            G::EffectKnowledge => (
                "이 행동의 효과에 관한 근거",
                "evidence about this action's effects",
            ),
            G::InapplicableState => (
                "현재 조건에 적용할 효과 근거",
                "effect evidence applicable to these conditions",
            ),
            G::AmbiguousEffect => (
                "가능한 효과들을 구분할 근거",
                "evidence distinguishing the possible effects",
            ),
            G::InvalidContext => ("유효한 문맥 정보", "valid context information"),
        }
    } else {
        match inquiry.missing_input {
            DecisionInputIR::DesiredOutcome => ("원하는 결과", "desired outcome"),
            DecisionInputIR::Priority => ("우선순위를 정할 기준", "a priority criterion"),
            DecisionInputIR::Deadline => ("기한", "deadline"),
            DecisionInputIR::Constraints => ("제약 조건", "constraints"),
            DecisionInputIR::Preference => ("선호하는 조건", "preferences"),
            DecisionInputIR::ExpectedBenefit => ("기대하는 이득", "expected benefit"),
        }
    };
    let korean = language == LanguageCodeIR::Korean;
    let mut store = ExpressionNodeStore::default();
    let mut nodes = Vec::new();
    for (id, concept, root, kind, pos) in [
        (
            "D",
            if gap.is_some() {
                if ask_gap {
                    if inquiry.explanation_of.is_some() {
                        "C_WORLD_CLAUSE_GAP_ASK_REASON"
                    } else {
                        "C_WORLD_CLAUSE_GAP_ASK"
                    }
                } else if inquiry.explanation_of.is_some() {
                    "C_WORLD_CLAUSE_GAP_LIMIT_REASON"
                } else {
                    "C_WORLD_CLAUSE_GAP_LIMIT"
                }
            } else if inquiry.explanation_of.is_some() {
                "C_WORLD_CLAUSE_DECISION_REASON"
            } else {
                "C_WORLD_CLAUSE_DECISION_GAP"
            }
            .into(),
            if gap.is_some() && !ask_gap {
                if korean {
                    "부족하다"
                } else {
                    "lack"
                }
            } else if inquiry.explanation_of.is_some() {
                if korean {
                    "필요하다"
                } else {
                    "require"
                }
            } else if korean {
                "알려주다"
            } else {
                "tell"
            },
            GenerationMeaningNodeKindIR::Event,
            ExpressionPartOfSpeechIR::Verb,
        ),
        (
            "I",
            if option_evidence_gap {
                "C_DECISION_OPTION_EVIDENCE".to_string()
            } else {
                gap.map_or_else(
                    || format!("C_DECISION_INPUT_{:?}", inquiry.missing_input),
                    |g| format!("C_DECISION_KNOWLEDGE_GAP_{g:?}"),
                )
            },
            if korean { ko } else { en },
            GenerationMeaningNodeKindIR::Entity,
            ExpressionPartOfSpeechIR::Noun,
        ),
    ] {
        store.attach_alias(
            &format!("EXPR.DECISION.{id}"),
            language,
            &concept,
            root,
            pos,
            "RUNTIME_REFERENT_SURFACE:DECISION_INPUT",
        )?;
        nodes.push(GenerationMeaningNodeIR {
            node_id: id.into(),
            concept_id: concept,
            kind,
            grounding_refs: vec![if option_evidence_gap {
                "DECISION_OPTION_EVIDENCE_GAP:SOURCE_BOUND".to_string()
            } else {
                gap.map_or_else(
                    || format!("DECISION_INPUT:{:?}", inquiry.missing_input),
                    |g| format!("REPLAYED_DECISION_GAP:{g:?}"),
                )
            }],
        });
    }
    let mut edges = vec![meaning_edge(
        "DI",
        "D",
        "I",
        GenerationMeaningRelationIR::Theme,
    )];
    if let Some(action) = &inquiry.proposed_action {
        let concept = format!(
            "C_PROPOSED_ACTION:{:x}",
            Sha256::digest(
                serde_json::to_vec(&(
                    &action.predicate_entry_ids,
                    action.negated,
                    &action.event.roles,
                ))
                .map_err(|_| "INVALID_PROPOSAL_ROLES")?
            )
        );
        store.attach_alias(
            "EXPR.DECISION.ACTION",
            language,
            &concept,
            // This is a source-language mention of the user's proposal, not a
            // newly asserted event. Preserve articles/prepositions and focus
            // markers that the current role graph does not yet fully encode.
            &action.surface,
            ExpressionPartOfSpeechIR::Noun,
            "RUNTIME_REFERENT_SURFACE:PROPOSED_ACTION",
        )?;
        nodes.push(GenerationMeaningNodeIR {
            node_id: "A".into(),
            concept_id: concept,
            kind: GenerationMeaningNodeKindIR::Entity,
            grounding_refs: std::iter::once(format!(
                "PROPOSAL_SOURCE_SHA256:{:x}",
                Sha256::digest(inquiry.source_text.as_bytes())
            ))
            .chain(
                action
                    .event
                    .roles
                    .iter()
                    .map(|(role, value)| format!("PROPOSAL_ROLE:{role:?}:{value}")),
            )
            .collect(),
        });
        edges.push(meaning_edge(
            "DA",
            "D",
            "A",
            GenerationMeaningRelationIR::Goal,
        ));
    }
    settings.generate(GenerativeLanguageRequestIR {
        meaning: GenerationMeaningGraphIR::new(nodes, edges),
        context: GenerationContextIR {
            language,
            register: LanguageRegisterIR::Informal,
            tense: GenerationTenseIR::Present,
            emotion: GenerationEmotionIR::Neutral,
            urgency_millis: 0,
            default_speech_intent: if inquiry.explanation_of.is_some()
                || (gap.is_some() && !ask_gap)
            {
                GenerationSpeechIntentIR::Inform
            } else {
                GenerationSpeechIntentIR::Ask
            },
        },
        expressions: &store,
    })
}

fn generate_decision_context_received(
    settings: GenerationSettings,
) -> Result<GenerativeLanguageIR, String> {
    let language = settings.language;
    let korean = language == LanguageCodeIR::Korean;
    let mut store = ExpressionNodeStore::default();
    store.attach_alias(
        "EXPR.DECISION.CONTEXT",
        language,
        "C_DECISION_CONTEXT_EVIDENCE",
        if korean {
            "말해 준 조건"
        } else {
            "the conditions you provided"
        },
        ExpressionPartOfSpeechIR::Noun,
        "RUNTIME_REFERENT_SURFACE:DECISION_CONTEXT_EVIDENCE",
    )?;
    store.attach_alias(
        "EXPR.DECISION.CONTEXT.BIND",
        language,
        "C_WORLD_CLAUSE_DECISION_CONTEXT_BOUND",
        if korean { "기준으로 보다" } else { "use as a basis" },
        ExpressionPartOfSpeechIR::Verb,
        "RUNTIME_REFERENT_SURFACE:DECISION_CONTEXT_BINDING",
    )?;
    let nodes = vec![
        GenerationMeaningNodeIR {
            node_id: "D".into(),
            concept_id: "C_WORLD_CLAUSE_DECISION_CONTEXT_BOUND".into(),
            kind: GenerationMeaningNodeKindIR::Event,
            grounding_refs: vec!["DECISION_CONTEXT:SOURCE_BOUND".into()],
        },
        GenerationMeaningNodeIR {
            node_id: "I".into(),
            concept_id: "C_DECISION_CONTEXT_EVIDENCE".into(),
            kind: GenerationMeaningNodeKindIR::Entity,
            grounding_refs: vec!["DECISION_CONTEXT:SOURCE_BOUND".into()],
        },
    ];
    settings.generate(GenerativeLanguageRequestIR {
        meaning: GenerationMeaningGraphIR::new(
            nodes,
            vec![meaning_edge(
                "DI",
                "D",
                "I",
                GenerationMeaningRelationIR::Theme,
            )],
        ),
        context: GenerationContextIR {
            language,
            register: LanguageRegisterIR::Informal,
            tense: GenerationTenseIR::Present,
            emotion: GenerationEmotionIR::Neutral,
            urgency_millis: 0,
            default_speech_intent: GenerationSpeechIntentIR::Inform,
        },
        expressions: &store,
    })
}

/// Realize a selection that has already passed the source-bound choice
/// contract. The selected text is a quoted user option, not a newly asserted
/// property of that option.
fn generate_decision_choice_selection(
    settings: GenerationSettings,
    selection: &crate::utterance_intent::DecisionChoiceSelectionIR,
) -> Result<GenerativeLanguageIR, String> {
    let language = settings.language;
    let korean = language == LanguageCodeIR::Korean;
    let selected = selection
        .options
        .get(selection.selected_option_index)
        .ok_or("INVALID_DECISION_CHOICE_SELECTION")?;
    let mut store = ExpressionNodeStore::default();
    store.attach_alias(
        "EXPR.DECISION.CHOICE",
        language,
        "C_DECISION_CHOICE_SELECTED",
        &selected.source_text,
        ExpressionPartOfSpeechIR::Noun,
        "RUNTIME_REFERENT_SURFACE:DECISION_CHOICE_OPTION",
    )?;
    store.attach_alias(
        "EXPR.DECISION.CHOICE.FIT",
        language,
        "C_WORLD_CLAUSE_DECISION_CHOICE",
        if korean { "맞다" } else { "fit" },
        ExpressionPartOfSpeechIR::Verb,
        "RUNTIME_REFERENT_SURFACE:DECISION_CHOICE_SELECTION",
    )?;
    let nodes = vec![
        GenerationMeaningNodeIR {
            node_id: "D".into(),
            concept_id: "C_WORLD_CLAUSE_DECISION_CHOICE".into(),
            kind: GenerationMeaningNodeKindIR::Event,
            grounding_refs: vec![format!(
                "DECISION_QUESTION_SHA256:{}",
                selection.question_source_sha256
            )],
        },
        GenerationMeaningNodeIR {
            node_id: "O".into(),
            concept_id: "C_DECISION_CHOICE_SELECTED".into(),
            kind: GenerationMeaningNodeKindIR::Entity,
            grounding_refs: std::iter::once(format!(
                "DECISION_OPTION_SHA256:{}",
                selected.source_sha256
            ))
            .chain(
                selection
                    .matching_features
                    .iter()
                    .map(|feature| format!("DECISION_MATCHED_FEATURE:{feature}")),
            )
            .collect(),
        },
    ];
    settings.generate(GenerativeLanguageRequestIR {
        meaning: GenerationMeaningGraphIR::new(
            nodes,
            vec![meaning_edge(
                "DO",
                "D",
                "O",
                GenerationMeaningRelationIR::Theme,
            )],
        ),
        context: GenerationContextIR {
            language,
            register: LanguageRegisterIR::Informal,
            tense: GenerationTenseIR::Present,
            emotion: GenerationEmotionIR::Neutral,
            urgency_millis: 0,
            default_speech_intent: GenerationSpeechIntentIR::Inform,
        },
        expressions: &store,
    })
}

fn generate_optional_clarification_response(
    settings: GenerationSettings,
    reply: &crate::utterance_intent::DecisionClarificationReplyIR,
) -> Result<GenerativeLanguageIR, String> {
    let language = settings.language;
    let ko = language == LanguageCodeIR::Korean;
    let explain =
        reply.kind == crate::world_dialogue::WorldClarificationFollowupKindIR::ReasonRequest;
    let mut store = ExpressionNodeStore::default();
    let mut nodes = Vec::new();
    for (id, concept, root, kind, pos) in [
        (
            "R",
            if explain {
                "C_WORLD_CLAUSE_REFERENCE_RESPONSE_CHOICE"
            } else {
                "C_WORLD_CLAUSE_REFERENCE_OPTIONAL_RESPONSE"
            },
            if explain {
                if ko {
                    "선택"
                } else {
                    "choice"
                }
            } else if ko {
                "답하다"
            } else {
                "answer"
            },
            GenerationMeaningNodeKindIR::Event,
            if explain {
                ExpressionPartOfSpeechIR::Noun
            } else {
                ExpressionPartOfSpeechIR::Verb
            },
        ),
        (
            "A",
            if explain {
                "C_DIALOGUE_RESPONSE_DECISION"
            } else {
                "C_DIALOGUE_CLARIFICATION"
            },
            if explain {
                if ko {
                    "대답 여부"
                } else {
                    "whether to answer"
                }
            } else if ko {
                "그 질문"
            } else {
                "that question"
            },
            GenerationMeaningNodeKindIR::Entity,
            ExpressionPartOfSpeechIR::Noun,
        ),
    ] {
        store.attach_alias(
            &format!("EXPR.REPLY.{id}"),
            language,
            concept,
            root,
            pos,
            "RUNTIME_REFERENT_SURFACE:CLARIFICATION_RESPONSE",
        )?;
        nodes.push(GenerationMeaningNodeIR {
            node_id: id.into(),
            concept_id: concept.into(),
            kind,
            grounding_refs: vec![
                format!("QUESTION_REPLY:{:?}:{}", reply.kind, reply.turn),
                format!("QUESTION_ORIGIN:{}", reply.question_turn),
            ],
        });
    }
    settings.generate(GenerativeLanguageRequestIR {
        meaning: GenerationMeaningGraphIR::new(
            nodes,
            vec![meaning_edge(
                "RA",
                "R",
                "A",
                GenerationMeaningRelationIR::Theme,
            )],
        ),
        context: GenerationContextIR {
            language,
            register: LanguageRegisterIR::Informal,
            tense: GenerationTenseIR::Present,
            emotion: GenerationEmotionIR::Neutral,
            urgency_millis: 0,
            default_speech_intent: GenerationSpeechIntentIR::Inform,
        },
        expressions: &store,
    })
}

fn generate_action_benefit(
    settings: GenerationSettings,
    assessment: &crate::world_dialogue::ActionBenefitAssessmentIR,
    explain: bool,
) -> Result<GenerativeLanguageIR, String> {
    let p = assessment
        .primitive()
        .ok_or("MISSING_ACTION_BENEFIT_PRIMITIVE")?;
    let expression = assessment
        .expression()
        .ok_or("MISSING_ACTION_BENEFIT_EXPRESSION")?;
    let language = settings.language;
    let ko = language == LanguageCodeIR::Korean;
    let mut store = ExpressionNodeStore::default();
    let actor = world_entity_root(&assessment.actor, language);
    let mut nodes = Vec::new();
    for (id, concept, root, kind, pos) in [
        (
            "B",
            if explain {
                "C_WORLD_CLAUSE_BENEFIT_BASIS"
            } else {
                "C_WORLD_CLAUSE_POSSIBLE_BENEFIT"
            },
            if ko { "돕다" } else { "help" },
            GenerationMeaningNodeKindIR::Event,
            ExpressionPartOfSpeechIR::Verb,
        ),
        (
            "E",
            p.possible_effect_id.as_str(),
            if ko {
                &expression.effect_ko
            } else {
                &expression.effect_en
            },
            GenerationMeaningNodeKindIR::Entity,
            ExpressionPartOfSpeechIR::Noun,
        ),
        (
            "A",
            p.id.as_str(),
            if ko {
                &expression.action_ko
            } else {
                &expression.action_en
            },
            GenerationMeaningNodeKindIR::Entity,
            ExpressionPartOfSpeechIR::Noun,
        ),
        (
            "P",
            assessment.actor.as_str(),
            actor.as_str(),
            GenerationMeaningNodeKindIR::Entity,
            ExpressionPartOfSpeechIR::Noun,
        ),
    ] {
        store.attach_alias(
            &format!("EXPR.BENEFIT.{id}"),
            language,
            concept,
            root,
            pos,
            "RUNTIME_REFERENT_SURFACE:CONDITIONAL_BENEFIT",
        )?;
        nodes.push(GenerationMeaningNodeIR {
            node_id: id.into(),
            concept_id: concept.into(),
            kind,
            grounding_refs: vec![
                format!(
                    "CORE_DERIVATION:{}",
                    assessment.derivation.deliberation_sha256
                ),
                p.source_ref.clone(),
                "CONDITIONAL_INTERPRETATION_POSSIBLE_EFFECT_NOT_EXECUTED".into(),
            ],
        });
    }
    settings.generate(GenerativeLanguageRequestIR {
        meaning: GenerationMeaningGraphIR::new(
            nodes,
            vec![
                meaning_edge("BE", "B", "E", GenerationMeaningRelationIR::Theme),
                meaning_edge("BA", "B", "A", GenerationMeaningRelationIR::Goal),
                meaning_edge("BP", "B", "P", GenerationMeaningRelationIR::Agent),
            ],
        ),
        context: GenerationContextIR {
            language,
            register: LanguageRegisterIR::Informal,
            tense: GenerationTenseIR::Present,
            emotion: GenerationEmotionIR::Neutral,
            urgency_millis: 0,
            default_speech_intent: GenerationSpeechIntentIR::Inform,
        },
        expressions: &store,
    })
}

pub(crate) fn generate_world_clarification(
    settings: impl Into<GenerationSettings>,
    c: &crate::world_dialogue::WorldClarificationIR,
) -> Result<GenerativeLanguageIR, String> {
    let settings = settings.into();
    let language = settings.language;
    if !c.validate() {
        return Err("INVALID_REFERENCE_GAP".into());
    }
    let candidates = c.gap.candidates();
    let mut store = ExpressionNodeStore::default();
    let mut nodes = Vec::new();
    let open_argument = c.gap.argument.as_ref();
    use crate::world_dialogue::WorldClarificationActKindIR;
    let act = c.response_act();
    let followup = act.kind != WorldClarificationActKindIR::RequestIdentity;
    let abstention = act.kind == WorldClarificationActKindIR::AllowAbstention;
    let choice = act.kind == WorldClarificationActKindIR::ExplainChoice;
    let mut parts = vec![
        (
            "R".to_string(),
            if choice {
                "C_WORLD_CLAUSE_REFERENCE_RESPONSE_CHOICE"
            } else if abstention {
                "C_WORLD_CLAUSE_REFERENCE_OPTIONAL_RESPONSE"
            } else if followup {
                "C_WORLD_CLAUSE_REFERENCE_REQUIREMENT"
            } else {
                "C_WORLD_CLAUSE_REFERENCE"
            }
            .to_string(),
            if choice && language == LanguageCodeIR::Korean {
                "선택"
            } else if choice {
                "choice"
            } else if abstention && language == LanguageCodeIR::Korean {
                "답하다"
            } else if abstention {
                "answer"
            } else if followup && language == LanguageCodeIR::Korean {
                "식별하다"
            } else if followup {
                "identify"
            } else if language == LanguageCodeIR::Korean {
                "말하다"
            } else {
                "mean"
            }
            .to_string(),
            GenerationMeaningNodeKindIR::Event,
            if choice {
                ExpressionPartOfSpeechIR::Noun
            } else {
                ExpressionPartOfSpeechIR::Verb
            },
        ),
        (
            "A".into(),
            if choice {
                "C_DIALOGUE_RESPONSE_DECISION".into()
            } else if abstention {
                "C_DIALOGUE_CLARIFICATION".into()
            } else if followup {
                "C_DIALOGUE_REFERENT".into()
            } else {
                open_argument.map_or_else(
                    || format!("C_ENTITY_{}", candidates[0]),
                    |gap| format!("C_WORLD_OPEN_{:?}", gap.missing_role),
                )
            },
            if choice {
                if language == LanguageCodeIR::Korean {
                    "대답 여부".into()
                } else {
                    "whether to answer".into()
                }
            } else if abstention {
                if language == LanguageCodeIR::Korean {
                    "그 질문".into()
                } else {
                    "that question".into()
                }
            } else if followup {
                if language == LanguageCodeIR::Korean {
                    "대상".into()
                } else {
                    "the intended referent".into()
                }
            } else {
                open_argument.map_or_else(
                    || world_entity_root(&candidates[0], language),
                    |_| {
                        if language == LanguageCodeIR::Korean {
                            "대상".into()
                        } else {
                            "person or thing".into()
                        }
                    },
                )
            },
            GenerationMeaningNodeKindIR::Entity,
            ExpressionPartOfSpeechIR::Noun,
        ),
    ];
    if open_argument.is_none() && !followup {
        parts.push((
            "B".into(),
            format!("C_ENTITY_{}", candidates[1]),
            world_entity_root(&candidates[1], language),
            GenerationMeaningNodeKindIR::Entity,
            ExpressionPartOfSpeechIR::Noun,
        ));
    }
    for (id, concept, root, kind, pos) in parts {
        store.attach_alias(
            &format!("EXPR.REF.{id}"),
            language,
            &concept,
            &root,
            pos,
            "RUNTIME_REFERENT_SURFACE:WORLD_REFERENCE",
        )?;
        let mut grounding_refs = vec![format!("WORLD_REFERENCE_GAP:{}", c.gap.source_sha256)];
        if let Some(r) = &c.followup {
            grounding_refs.push(format!("CLARIFICATION_FOLLOWUP:{:?}:{}", r.kind, r.turn));
        }
        nodes.push(GenerationMeaningNodeIR {
            node_id: id,
            concept_id: concept,
            kind,
            grounding_refs,
        });
    }
    let mut edges = vec![meaning_edge(
        "RA",
        "R",
        "A",
        GenerationMeaningRelationIR::Theme,
    )];
    if open_argument.is_none() && !followup {
        edges.push(meaning_edge(
            "RB",
            "R",
            "B",
            GenerationMeaningRelationIR::Goal,
        ));
    }
    settings.generate(GenerativeLanguageRequestIR {
        meaning: GenerationMeaningGraphIR::new(nodes, edges),
        context: GenerationContextIR {
            language,
            register: LanguageRegisterIR::Informal,
            tense: GenerationTenseIR::Present,
            emotion: GenerationEmotionIR::Neutral,
            urgency_millis: 0,
            default_speech_intent: if followup {
                GenerationSpeechIntentIR::Inform
            } else {
                GenerationSpeechIntentIR::Ask
            },
        },
        expressions: &store,
    })
}

pub(crate) fn generate_world_decision(
    settings: impl Into<GenerationSettings>,
    world: &WorldReasoningIR,
) -> Result<GenerativeLanguageIR, String> {
    generate_world_clauses(
        settings,
        world_decision_clauses(world)?,
        &world.memory.vocabulary,
    )
}

type WorldClause = (WorldAtomIR, bool, &'static str, Vec<String>);

fn world_decision_clauses(world: &WorldReasoningIR) -> Result<Vec<WorldClause>, String> {
    if !world.validate() {
        return Err("INVALID_WORLD_DECISION".into());
    }
    world
        .utterance_plan
        .moves
        .iter()
        .map(|item| {
            let literal = &item.proposition;
            let atom = world
                .atoms
                .get(&literal.proposition_id)
                .ok_or("MISSING_WORLD_ATOM")?
                .clone();
            let mut refs = vec![
                format!("WORLD_DECISION:{}", world.semantic_decision_sha256),
                literal.proposition_id.clone(),
            ];
            refs.extend(
                item.evidence_refs
                    .iter()
                    .map(|r| format!("WORLD_EVIDENCE:{r}")),
            );
            Ok((atom, literal.value, item.purpose.mode(), refs))
        })
        .collect::<Result<Vec<_>, String>>()
}

pub(crate) fn generate_world_memory_update(
    settings: impl Into<GenerationSettings>,
    update: &WorldMemoryUpdateIR,
) -> Result<GenerativeLanguageIR, String> {
    generate_world_clauses(
        settings,
        world_update_clauses(update)?,
        &update.memory.vocabulary,
    )
}

fn world_update_clauses(update: &WorldMemoryUpdateIR) -> Result<Vec<WorldClause>, String> {
    if !update.validate() {
        return Err("INVALID_WORLD_MEMORY_UPDATE".into());
    }
    let mut clauses = Vec::new();
    let refs = vec![format!("WORLD_MEMORY_TURN:{}", update.turn)];
    let premises = update
        .memory
        .premises
        .iter()
        .filter(|p| p.introduced_turn == update.turn)
        .collect::<Vec<_>>();
    if !premises.is_empty() {
        for p in premises {
            clauses.push((p.atom.clone(), p.value, "REMEMBER", refs.clone()));
        }
    } else {
        let rule = update
            .memory
            .implications
            .iter()
            .find(|r| r.introduced_turn == update.turn)
            .ok_or("MISSING_WORLD_MEMORY_UPDATE")?;
        for (index, (atom, value)) in rule.prerequisites.iter().enumerate() {
            let mode = match (index == 0, index + 1 == rule.prerequisites.len()) {
                (true, true) => "IF",
                (true, false) => "IF_AND",
                (false, true) => "AND_IF",
                (false, false) => "AND",
            };
            clauses.push((atom.clone(), *value, mode, refs.clone()));
        }
        clauses.push((rule.effect.0.clone(), rule.effect.1, "THEN", refs));
    }
    Ok(clauses)
}

// Preflight checks capabilities and bounds without constructing a sentence.
// Clause selection is shared with final realization; it is not a second policy.
fn world_clauses_available(
    language: LanguageCodeIR,
    clauses: &[WorldClause],
    vocabulary: &WorldVocabularyIR,
) -> bool {
    matches!(language, LanguageCodeIR::Korean | LanguageCodeIR::English)
        && !clauses.is_empty()
        && clauses.len() <= 30
        && clauses.iter().all(|(atom, _, _, _)| {
            !matches!(
                atom.property,
                crate::world_dialogue::WorldPropertyIR::Registered(_)
            ) || vocabulary.expression(&atom.property, language).is_some()
        })
}

pub(crate) fn world_decision_language_available(
    language: LanguageCodeIR,
    world: &WorldReasoningIR,
) -> bool {
    world_decision_clauses(world)
        .is_ok_and(|clauses| world_clauses_available(language, &clauses, &world.memory.vocabulary))
}

pub(crate) fn world_update_language_available(
    language: LanguageCodeIR,
    update: &WorldMemoryUpdateIR,
) -> bool {
    world_update_clauses(update)
        .is_ok_and(|clauses| world_clauses_available(language, &clauses, &update.memory.vocabulary))
}

fn generate_world_clauses(
    settings: impl Into<GenerationSettings>,
    clauses: Vec<WorldClause>,
    vocabulary: &WorldVocabularyIR,
) -> Result<GenerativeLanguageIR, String> {
    let settings = settings.into();
    let language = settings.language;
    // The underlying proof depth is bounded. Do not silently truncate its explanation.
    if clauses.len() > 30 {
        return Err("WORLD_EXPLANATION_BOUND".into());
    }
    if !world_clauses_available(language, &clauses, vocabulary) {
        return Err("WORLD_EXPRESSION_UNAVAILABLE".into());
    }
    let mut store = ExpressionNodeStore::default();
    let mut nodes = Vec::new();
    let mut edges = Vec::new();
    for (index, (atom, value, mode, refs)) in clauses.into_iter().enumerate() {
        let event = format!("W{index}");
        let subject = format!("S{index}");
        let property = format!("P{index}");
        let predicate_concept = format!("C_WORLD_CLAUSE_{mode}");
        let lexeme = vocabulary.expression(&atom.property, language);
        let (property_id, root) = match &atom.property {
            crate::world_dialogue::WorldPropertyIR::Registered(id) => (
                format!("C_PROPERTY_{id}"),
                lexeme.ok_or("WORLD_EXPRESSION_UNAVAILABLE")?.root.clone(),
            ),
            other => (
                format!("C_PROPERTY_{other:?}"),
                other.expression(language == LanguageCodeIR::Korean).into(),
            ),
        };
        let mut parts = vec![
            (
                event.clone(),
                predicate_concept,
                if language == LanguageCodeIR::Korean {
                    "이다"
                } else {
                    "is"
                }
                .to_string(),
                GenerationMeaningNodeKindIR::Event,
                ExpressionPartOfSpeechIR::Verb,
            ),
            (
                subject.clone(),
                format!("C_ENTITY_{}", atom.entity),
                world_entity_root(&atom.entity, language),
                GenerationMeaningNodeKindIR::Entity,
                ExpressionPartOfSpeechIR::Noun,
            ),
            (
                property.clone(),
                property_id,
                root,
                GenerationMeaningNodeKindIR::Quality,
                ExpressionPartOfSpeechIR::Adjective,
            ),
        ];
        edges.push(meaning_edge(
            &format!("SUBJECT{index}"),
            &event,
            &subject,
            GenerationMeaningRelationIR::Theme,
        ));
        edges.push(meaning_edge(
            &format!("PROPERTY{index}"),
            &event,
            &property,
            GenerationMeaningRelationIR::Property,
        ));
        if let Some(object) = &atom.object {
            let id = format!("O{index}");
            parts.push((
                id.clone(),
                format!("C_ENTITY_{object}"),
                world_entity_root(object, language),
                GenerationMeaningNodeKindIR::Entity,
                ExpressionPartOfSpeechIR::Noun,
            ));
            edges.push(meaning_edge(
                &format!("OBJECT{index}"),
                &event,
                &id,
                GenerationMeaningRelationIR::Goal,
            ));
        }
        if !value {
            let negation = format!("N{index}");
            parts.push((
                negation.clone(),
                "C_WORLD_NEGATION".into(),
                if language == LanguageCodeIR::Korean {
                    "아니다"
                } else {
                    "not"
                }
                .into(),
                GenerationMeaningNodeKindIR::Quality,
                ExpressionPartOfSpeechIR::Adjective,
            ));
            edges.push(meaning_edge(
                &format!("NEGATION{index}"),
                &event,
                &negation,
                GenerationMeaningRelationIR::Negates,
            ));
        }
        if index > 0 {
            edges.push(meaning_edge(
                &format!("ORDER{index}"),
                &format!("W{}", index - 1),
                &event,
                GenerationMeaningRelationIR::Sequence,
            ));
        }
        for (id, concept, surface, kind, pos) in parts {
            // One expression per event constituent keeps provenance unambiguous.
            store.attach_alias(
                &format!("EXPR.WORLD.{id}"),
                language,
                &concept,
                &surface,
                pos,
                "RUNTIME_REFERENT_SURFACE:WORLD_ATOM",
            )?;
            if id == property {
                if let Some(lexeme) = lexeme {
                    let expression = store
                        .entries
                        .get_mut(&format!("EXPR.WORLD.{id}"))
                        .ok_or("MISSING_WORLD_EXPRESSION")?;
                    expression.morphology = match lexeme.grammar {
                        WorldLexicalGrammarIR::Copular => expression.morphology,
                        WorldLexicalGrammarIR::KoreanHadaState
                        | WorldLexicalGrammarIR::KoreanHadaExperiencer => {
                            ExpressionMorphologyClassIR::KoreanHada
                        }
                        WorldLexicalGrammarIR::EnglishRegularVerb => {
                            ExpressionMorphologyClassIR::EnglishRegularRelation
                        }
                        WorldLexicalGrammarIR::KoreanHadaLocative => {
                            ExpressionMorphologyClassIR::KoreanHadaLocative
                        }
                        WorldLexicalGrammarIR::KoreanHadaAccusative => {
                            ExpressionMorphologyClassIR::KoreanHadaAccusative
                        }
                    };
                }
            }
            nodes.push(GenerationMeaningNodeIR {
                node_id: id,
                concept_id: concept,
                kind,
                grounding_refs: refs.clone(),
            });
        }
    }
    settings.generate(GenerativeLanguageRequestIR {
        meaning: GenerationMeaningGraphIR::new(nodes, edges),
        context: GenerationContextIR {
            language,
            register: LanguageRegisterIR::Informal,
            tense: GenerationTenseIR::Present,
            emotion: GenerationEmotionIR::Neutral,
            urgency_millis: 0,
            default_speech_intent: GenerationSpeechIntentIR::Inform,
        },
        expressions: &store,
    })
}

pub(super) fn realize_world_clause(
    clause: &SyntaxClauseIR,
    context: &GenerationContextIR,
    selected: &BTreeMap<(&str, &str), &ExpressionSelectionIR>,
    predicate: &ExpressionSelectionIR,
) -> Vec<MorphologicalTokenIR> {
    let mut output = Vec::new();
    let Some(subject) = constituent_selection(clause, SyntaxConstituentRoleIR::Theme, selected)
    else {
        return output;
    };
    if predicate.expression.concept_id == "C_WORLD_CLAUSE_BENEFIT_BASIS" {
        let Some(action) = constituent_selection(clause, SyntaxConstituentRoleIR::Goal, selected)
        else {
            return output;
        };
        let Some(actor) = constituent_selection(clause, SyntaxConstituentRoleIR::Agent, selected)
        else {
            return output;
        };
        if context.language == LanguageCodeIR::Korean {
            push_expression_token(
                &mut output,
                action,
                world_korean_topic(&action.expression.lexical_root),
            );
            push_expression_token(
                &mut output,
                subject,
                world_korean_object(&subject.expression.lexical_root),
            );
            push_grammar_token(
                &mut output,
                "위한 행동이니까, 그 뜻이라면 말해 준 상태에서",
                "KO.ACTION_PURPOSE_AND_CONDITIONAL_PREMISE",
                &clause.event_node_id,
            );
            push_expression_token(
                &mut output,
                actor,
                format!("{}에게", actor.expression.lexical_root),
            );
            push_expression_token(
                &mut output,
                predicate,
                if context.register == LanguageRegisterIR::Formal {
                    "도움이 될 수 있다고 판단했습니다."
                } else {
                    "도움이 될 수 있다고 본 거야."
                }
                .into(),
            );
        } else {
            push_grammar_token(
                &mut output,
                "If you mean",
                "EN.CONDITIONAL_SENSE",
                &clause.event_node_id,
            );
            push_expression_token(
                &mut output,
                action,
                format!("{},", action.expression.lexical_root),
            );
            push_grammar_token(
                &mut output,
                "its purpose is",
                "EN.ACTION_PURPOSE",
                &clause.event_node_id,
            );
            push_expression_token(
                &mut output,
                subject,
                format!("{}.", subject.expression.lexical_root),
            );
            push_grammar_token(
                &mut output,
                "Given the state you described, I thought it may",
                "EN.JUDGEMENT_BASIS",
                &clause.event_node_id,
            );
            push_expression_token(&mut output, predicate, "help".into());
            push_expression_token(
                &mut output,
                actor,
                format!("{}.", actor.expression.lexical_root),
            );
        }
        return output;
    }
    if predicate.expression.concept_id == "C_WORLD_CLAUSE_POSSIBLE_BENEFIT" {
        let Some(action) = constituent_selection(clause, SyntaxConstituentRoleIR::Goal, selected)
        else {
            return output;
        };
        let Some(actor) = constituent_selection(clause, SyntaxConstituentRoleIR::Agent, selected)
        else {
            return output;
        };
        if context.language == LanguageCodeIR::Korean {
            push_expression_token(
                &mut output,
                action,
                world_korean_object(&action.expression.lexical_root),
            );
            push_grammar_token(
                &mut output,
                "뜻한다면, 말해 준 상태에서는",
                "KO.CONDITIONAL_SENSE_AND_PREMISE",
                &clause.event_node_id,
            );
            push_expression_token(
                &mut output,
                actor,
                format!("{}의", actor.expression.lexical_root),
            );
            push_expression_token(
                &mut output,
                subject,
                format!("{}에", subject.expression.lexical_root),
            );
            push_expression_token(
                &mut output,
                predicate,
                if context.register == LanguageRegisterIR::Formal {
                    "도움이 될 수 있습니다."
                } else {
                    "도움이 될 수 있어."
                }
                .into(),
            );
        } else {
            push_grammar_token(
                &mut output,
                "If you mean",
                "EN.CONDITIONAL_SENSE",
                &clause.event_node_id,
            );
            push_expression_token(
                &mut output,
                action,
                format!("{},", action.expression.lexical_root),
            );
            push_grammar_token(
                &mut output,
                "it may",
                "EN.POSSIBLE_EFFECT",
                &clause.event_node_id,
            );
            push_expression_token(&mut output, predicate, "help".into());
            push_expression_token(&mut output, actor, actor.expression.lexical_root.clone());
            push_grammar_token(
                &mut output,
                "with",
                "EN.BENEFIT_RELATION",
                &clause.event_node_id,
            );
            push_expression_token(
                &mut output,
                subject,
                format!("{},", subject.expression.lexical_root),
            );
            push_grammar_token(
                &mut output,
                "given the state you described.",
                "EN.CONDITIONAL_PREMISE",
                &clause.event_node_id,
            );
        }
        return output;
    }
    if matches!(
        predicate.expression.concept_id.as_str(),
        "C_WORLD_CLAUSE_GAP_ASK"
            | "C_WORLD_CLAUSE_GAP_ASK_REASON"
            | "C_WORLD_CLAUSE_GAP_LIMIT"
            | "C_WORLD_CLAUSE_GAP_LIMIT_REASON"
    ) {
        let ko = context.language == LanguageCodeIR::Korean;
        let formal = context.register == LanguageRegisterIR::Formal;
        let asking = predicate.expression.concept_id.contains("GAP_ASK");
        let why = predicate.expression.concept_id.ends_with("_REASON");
        if let Some(action) = constituent_selection(clause, SyntaxConstituentRoleIR::Goal, selected)
        {
            push_expression_token(
                &mut output,
                action,
                if ko {
                    format!("‘{}’에 관해서는,", action.expression.lexical_root)
                } else {
                    format!("Regarding ‘{}’,", action.expression.lexical_root)
                },
            );
        }
        if !ko {
            push_grammar_token(
                &mut output,
                match (asking, why) {
                    (true, false) => "could you tell me",
                    (true, true) => "I asked because I need to know",
                    (false, false) => "I don't yet have enough",
                    (false, true) => "I cannot judge it yet because I need more",
                },
                "EN.EPISTEMIC_GAP",
                &clause.event_node_id,
            );
        }
        let root = &subject.expression.lexical_root;
        push_expression_token(
            &mut output,
            subject,
            if ko {
                if asking {
                    if why {
                        format!("{root}에 관한 정보가")
                    } else {
                        root.clone()
                    }
                } else {
                    world_korean_subject(root, true)
                }
            } else {
                format!("{root}{}", if asking && !why { "?" } else { "." })
            },
        );
        if ko {
            push_grammar_token(
                &mut output,
                match (asking, why, formal) {
                    (true, false, false) => "알려줄래?",
                    (true, false, true) => "알려주시겠어요?",
                    (true, true, false) => "필요해서 물었어.",
                    (true, true, true) => "필요해서 여쭤봤습니다.",
                    (false, false, false) => "아직 부족해.",
                    (false, false, true) => "아직 부족합니다.",
                    (false, true, false) => "부족해서 아직 판단하기 어려워.",
                    (false, true, true) => "부족해서 아직 판단하기 어렵습니다.",
                },
                "KO.EPISTEMIC_GAP",
                &clause.event_node_id,
            );
        }
        return output;
    }
    if predicate.expression.concept_id == "C_WORLD_CLAUSE_DECISION_REASON" {
        let action = constituent_selection(clause, SyntaxConstituentRoleIR::Goal, selected);
        if context.language == LanguageCodeIR::Korean {
            if let Some(action) = action {
                push_expression_token(
                    &mut output,
                    action,
                    world_korean_object(&action.expression.lexical_root),
                );
            }
            push_grammar_token(
                &mut output,
                "판단하려면",
                "KO.DECISION.REQUIREMENT",
                &clause.event_node_id,
            );
            push_expression_token(
                &mut output,
                subject,
                world_korean_subject(&subject.expression.lexical_root, true),
            );
            push_expression_token(&mut output, predicate, "필요해서".into());
            push_grammar_token(
                &mut output,
                if context.register == LanguageRegisterIR::Formal {
                    "여쭤봤습니다."
                } else {
                    "물었어."
                },
                "KO.PRIOR_QUESTION.REASON",
                &clause.event_node_id,
            );
        } else {
            push_grammar_token(
                &mut output,
                "I asked because",
                "EN.PRIOR_QUESTION.REASON",
                &clause.event_node_id,
            );
            push_grammar_token(
                &mut output,
                "a decision",
                "EN.DECISION.SUBJECT",
                &clause.event_node_id,
            );
            if let Some(action) = action {
                push_grammar_token(
                    &mut output,
                    "on the proposal to",
                    "EN.DECISION.TARGET",
                    &action.meaning_node_id,
                );
                push_expression_token(&mut output, action, action.expression.lexical_root.clone());
            }
            push_expression_token(&mut output, predicate, "requires".into());
            push_grammar_token(
                &mut output,
                "knowing the",
                "EN.DECISION.MISSING_INPUT",
                &clause.event_node_id,
            );
            push_expression_token(
                &mut output,
                subject,
                format!("{}.", subject.expression.lexical_root),
            );
        }
        return output;
    }
    if predicate.expression.concept_id == "C_WORLD_CLAUSE_DECISION_GAP" {
        let root = &subject.expression.lexical_root;
        if let Some(action) = constituent_selection(clause, SyntaxConstituentRoleIR::Goal, selected)
        {
            if context.language == LanguageCodeIR::Korean {
                let particle = korean_direction_particle(&action.expression.lexical_root);
                push_expression_token(
                    &mut output,
                    action,
                    format!("{}{particle}", action.expression.lexical_root),
                );
            } else {
                push_grammar_token(
                    &mut output,
                    "For the proposal to",
                    "EN.PROPOSED_ACTION.NOMINAL_INFINITIVE",
                    &action.meaning_node_id,
                );
                push_expression_token(
                    &mut output,
                    action,
                    format!("{},", action.expression.lexical_root),
                );
            }
        }
        if context.language == LanguageCodeIR::Korean {
            push_expression_token(&mut output, subject, world_korean_object(root));
            let stem = predicate
                .expression
                .lexical_root
                .strip_suffix('다')
                .unwrap_or(&predicate.expression.lexical_root);
            push_expression_token(
                &mut output,
                predicate,
                if context.register == LanguageRegisterIR::Formal {
                    format!("{stem}시겠어요?")
                } else {
                    format!(
                        "{}래?",
                        korean_future_commitment(stem).trim_end_matches('게')
                    )
                },
            );
        } else {
            push_grammar_token(
                &mut output,
                if constituent_selection(clause, SyntaxConstituentRoleIR::Goal, selected).is_some()
                {
                    "could you"
                } else {
                    "Could you"
                },
                "EN.INQUIRY",
                &clause.event_node_id,
            );
            push_expression_token(
                &mut output,
                predicate,
                predicate.expression.lexical_root.clone(),
            );
            push_grammar_token(
                &mut output,
                "me your",
                "EN.REQUESTED_INPUT",
                &clause.event_node_id,
            );
            push_expression_token(&mut output, subject, format!("{root}?"));
        }
        return output;
    }
    if predicate.expression.concept_id == "C_WORLD_CLAUSE_DECISION_CONTEXT_BOUND" {
        if context.language == LanguageCodeIR::Korean {
            push_expression_token(
                &mut output,
                subject,
                world_korean_object(&subject.expression.lexical_root),
            );
            push_grammar_token(
                &mut output,
                "기준으로 이어서 볼게.",
                "KO.DECISION.CONTEXT.BOUND",
                &clause.event_node_id,
            );
        } else {
            push_grammar_token(
                &mut output,
                "I will use",
                "EN.DECISION.CONTEXT.BOUND",
                &clause.event_node_id,
            );
            push_expression_token(&mut output, subject, subject.expression.lexical_root.clone());
            push_grammar_token(
                &mut output,
                "as the basis for the decision.",
                "EN.DECISION.CONTEXT.BOUND",
                &clause.event_node_id,
            );
        }
        return output;
    }
    if predicate.expression.concept_id == "C_WORLD_CLAUSE_DECISION_CHOICE" {
        if context.language == LanguageCodeIR::Korean {
            push_grammar_token(
                &mut output,
                if context.register == LanguageRegisterIR::Formal {
                    "말씀해 주신 조건으로 보면,"
                } else {
                    "말해 준 조건으로 보면,"
                },
                "KO.DECISION.CHOICE.CONTEXT",
                &clause.event_node_id,
            );
            push_expression_token(
                &mut output,
                subject,
                subject.expression.lexical_root.clone(),
            );
            push_grammar_token(
                &mut output,
                if context.register == LanguageRegisterIR::Formal {
                    "쪽이 더 맞습니다."
                } else {
                    "쪽이 더 맞아."
                },
                "KO.DECISION.CHOICE.RESULT",
                &clause.event_node_id,
            );
        } else {
            push_grammar_token(
                &mut output,
                "Given the stated conditions,",
                "EN.DECISION.CHOICE.CONTEXT",
                &clause.event_node_id,
            );
            push_expression_token(
                &mut output,
                subject,
                subject.expression.lexical_root.clone(),
            );
            push_grammar_token(
                &mut output,
                "is the better fit.",
                "EN.DECISION.CHOICE.RESULT",
                &clause.event_node_id,
            );
        }
        return output;
    }
    if predicate.expression.concept_id == "C_WORLD_CLAUSE_REFERENCE_RESPONSE_CHOICE" {
        // A dialogue choice, not evidence about the unresolved world predicate.
        let topic = &subject.expression.lexical_root;
        let root = &predicate.expression.lexical_root;
        if context.language == LanguageCodeIR::Korean {
            push_expression_token(&mut output, subject, world_korean_topic(topic));
            push_grammar_token(
                &mut output,
                if context.register == LanguageRegisterIR::Formal {
                    "본인의"
                } else {
                    "네"
                },
                "KO.INTERLOCUTOR.POSSESSIVE",
                &clause.event_node_id,
            );
            push_expression_token(
                &mut output,
                predicate,
                format!(
                    "{root}{}",
                    if context.register == LanguageRegisterIR::Formal {
                        "이니까요."
                    } else {
                        "이니까."
                    }
                ),
            );
        } else {
            push_expression_token(&mut output, subject, topic.clone());
            push_grammar_token(
                &mut output,
                "is your",
                "EN.COPULA.INTERLOCUTOR.POSSESSIVE",
                &clause.event_node_id,
            );
            push_expression_token(&mut output, predicate, format!("{root}."));
        }
        return output;
    }
    if predicate.expression.concept_id == "C_WORLD_CLAUSE_REFERENCE_OPTIONAL_RESPONSE" {
        let root = &subject.expression.lexical_root;
        if context.language == LanguageCodeIR::Korean {
            push_expression_token(&mut output, subject, format!("{root}에"));
            let stem = predicate
                .expression
                .lexical_root
                .strip_suffix('다')
                .unwrap_or(&predicate.expression.lexical_root);
            push_expression_token(&mut output, predicate, format!("{stem}지"));
            push_grammar_token(
                &mut output,
                if context.register == LanguageRegisterIR::Formal {
                    "않으셔도 됩니다."
                } else {
                    "않아도 돼."
                },
                "KO.PERMISSION.NOT_REQUIRED",
                &clause.event_node_id,
            );
        } else {
            push_grammar_token(
                &mut output,
                "You don't have to",
                "EN.PERMISSION.NOT_REQUIRED",
                &clause.event_node_id,
            );
            push_expression_token(
                &mut output,
                predicate,
                predicate.expression.lexical_root.clone(),
            );
            push_expression_token(&mut output, subject, format!("{root}."));
        }
        return output;
    }
    if predicate.expression.concept_id == "C_WORLD_CLAUSE_REFERENCE_REQUIREMENT" {
        let root = &subject.expression.lexical_root;
        if context.language == LanguageCodeIR::Korean {
            push_grammar_token(
                &mut output,
                "답하려면",
                "KO.DIALOGUE.ANSWER_DEPENDENCY",
                &clause.event_node_id,
            );
            push_expression_token(&mut output, subject, world_korean_object(root));
            let stem = predicate
                .expression
                .lexical_root
                .strip_suffix('다')
                .unwrap_or(&predicate.expression.lexical_root);
            push_expression_token(
                &mut output,
                predicate,
                format!(
                    "{} 필요가 {}.",
                    korean_future_commitment(stem).trim_end_matches('게'),
                    if context.register == LanguageRegisterIR::Formal {
                        "있습니다"
                    } else {
                        "있어"
                    }
                ),
            );
        } else {
            push_grammar_token(
                &mut output,
                "To answer, I need to",
                "EN.DIALOGUE.ANSWER_DEPENDENCY",
                &clause.event_node_id,
            );
            push_expression_token(
                &mut output,
                predicate,
                predicate.expression.lexical_root.clone(),
            );
            push_expression_token(&mut output, subject, format!("{root}."));
        }
        return output;
    }
    if predicate.expression.concept_id == "C_WORLD_CLAUSE_REFERENCE" {
        let Some(other) = constituent_selection(clause, SyntaxConstituentRoleIR::Goal, selected)
        else {
            // Open referent: request an identity, not the truth of the open
            // predicate. The partial predicate remains in the source-bound gap.
            if context.language == LanguageCodeIR::Korean {
                push_grammar_token(
                    &mut output,
                    "어느",
                    "KO.WH.REFERENCE",
                    &clause.event_node_id,
                );
                let root = &subject.expression.lexical_root;
                push_expression_token(&mut output, subject, world_korean_object(root));
                push_expression_token(
                    &mut output,
                    predicate,
                    if context.register == LanguageRegisterIR::Formal {
                        "말씀하시는 건가요?"
                    } else {
                        "말하는 거야?"
                    }
                    .into(),
                );
            } else {
                push_grammar_token(
                    &mut output,
                    "Which",
                    "EN.WH.REFERENCE",
                    &clause.event_node_id,
                );
                push_expression_token(
                    &mut output,
                    subject,
                    subject.expression.lexical_root.clone(),
                );
                push_grammar_token(
                    &mut output,
                    "do you",
                    "EN.INTERLOCUTOR_QUESTION",
                    &clause.event_node_id,
                );
                push_expression_token(
                    &mut output,
                    predicate,
                    format!("{}?", predicate.expression.lexical_root),
                );
            }
            return output;
        };
        let a = &subject.expression.lexical_root;
        let b = &other.expression.lexical_root;
        if context.language == LanguageCodeIR::Korean {
            push_expression_token(&mut output, subject, world_korean_object(a));
            push_expression_token(
                &mut output,
                predicate,
                if context.register == LanguageRegisterIR::Formal {
                    "말씀하시는 건가요,"
                } else {
                    "말하는 거야,"
                }
                .into(),
            );
            push_grammar_token(
                &mut output,
                "아니면",
                "KO.ALTERNATIVE_QUESTION",
                &clause.event_node_id,
            );
            push_expression_token(&mut output, other, world_korean_object(b));
            push_expression_token(
                &mut output,
                predicate,
                if context.register == LanguageRegisterIR::Formal {
                    "말씀하시는 건가요?"
                } else {
                    "말하는 거야?"
                }
                .into(),
            );
        } else {
            push_grammar_token(
                &mut output,
                "Do you",
                "EN.INTERLOCUTOR_QUESTION",
                &clause.event_node_id,
            );
            push_expression_token(&mut output, predicate, "mean".into());
            push_expression_token(&mut output, subject, a.clone());
            push_grammar_token(
                &mut output,
                "or",
                "EN.ALTERNATIVE_QUESTION",
                &clause.event_node_id,
            );
            push_expression_token(&mut output, other, format!("{b}?"));
        }
        return output;
    }
    let Some(property) = constituent_selection(clause, SyntaxConstituentRoleIR::Property, selected)
    else {
        return output;
    };
    let negation = constituent_selection(clause, SyntaxConstituentRoleIR::Negation, selected);
    let mode = predicate
        .expression
        .concept_id
        .trim_start_matches("C_WORLD_CLAUSE_");
    let korean = context.language == LanguageCodeIR::Korean;
    let formal = context.register == LanguageRegisterIR::Formal;
    let acknowledgement = if korean && matches!(mode, "REMEMBER" | "IF" | "IF_AND") {
        format!("{},", korean_acknowledgement(context.register))
    } else {
        String::new()
    };
    let prefix = match (korean, mode) {
        (true, "CAUSE_UNKNOWN") => "말해 준 내용만으로는,",
        (false, "CAUSE_UNKNOWN") => "From what you've told me, I don't know why",
        (true, "PREMISE") => "말해 준 내용에서는,",
        (true, "DERIVED") => "그 조건에 따르면,",
        (true, "HYPOTHESIS") => "그렇다고 가정하면,",
        (true, "CONCLUSION") => "말해 준 내용대로라면,",
        (true, "CONFLICT") => "앞뒤 정보가 달라서,",
        (true, "BOUND") => "탐색 한도 안에서는,",
        (true, "UNKNOWN") => "아직은,",
        (true, "ASK") => "그러면,",
        (true, "REMEMBER" | "IF" | "IF_AND") => acknowledgement.as_str(),
        // The preceding Korean premise already carries the coordinate
        // connective `-고`; adding `그리고` here yields the redundant
        // `-고, 그리고` sequence.
        (true, "AND" | "AND_IF") => "",
        (true, "THEN") => "",
        (false, "PREMISE") => "You said",
        (false, "DERIVED") => "By that condition,",
        (false, "HYPOTHESIS") => "If we assume that,",
        (false, "CONCLUSION") => "From what you told me,",
        (false, "CONFLICT") => "The accounts disagree about whether",
        (false, "BOUND") => "Within the search bound, I cannot determine whether",
        (false, "UNKNOWN") => "I don't know yet whether",
        (false, "ASK") => "Then,",
        (false, "REMEMBER") => "Got it,",
        (false, "IF" | "IF_AND") => "Got it: if",
        (false, "AND" | "AND_IF") => "and",
        (false, "THEN") => "then",
        _ => return output,
    };
    if !prefix.is_empty() {
        push_grammar_token(
            &mut output,
            prefix,
            &format!("WORLD.EPISTEMIC.{mode}"),
            &clause.event_node_id,
        );
    }
    let object = constituent_selection(clause, SyntaxConstituentRoleIR::Goal, selected);
    if object.is_some() || property.expression.morphology == ExpressionMorphologyClassIR::KoreanHada
    {
        // Same ordered semantic relation, with language-specific role marking
        // and inflection selected exclusively from the lexical layer.
        let conditional = matches!(mode, "IF" | "AND_IF");
        let conjunction = matches!(mode, "IF_AND" | "AND");
        let epistemic = matches!(mode, "CONFLICT" | "UNKNOWN" | "BOUND");
        let s = &subject.expression.lexical_root;
        let root = &property.expression.lexical_root;
        if korean {
            push_expression_token(
                &mut output,
                subject,
                world_korean_subject(
                    s,
                    matches!(mode, "CAUSE_UNKNOWN" | "IF" | "AND_IF" | "IF_AND" | "AND"),
                ),
            );
            if let Some(object) = object {
                let o = &object.expression.lexical_root;
                let marker = match property.expression.morphology {
                    ExpressionMorphologyClassIR::KoreanHadaLocative => "에",
                    ExpressionMorphologyClassIR::KoreanHadaAccusative => {
                        if crate::korean_nominal::surface_coda(o).is_none() {
                            push_expression_token(&mut output, object, world_korean_object(o));
                            ""
                        } else {
                            korean_particle(o, "을", "를")
                        }
                    }
                    _ => return Vec::new(),
                };
                if !marker.is_empty() {
                    push_expression_token(&mut output, object, format!("{o}{marker}"));
                }
            }
            push_expression_token(&mut output, property, root.clone());
            use crate::korean_hada::KoreanHadaFormIR as F;
            let state_hada =
                property.expression.morphology == ExpressionMorphologyClassIR::KoreanHada;
            let form = if conditional {
                F::Conditional
            } else if conjunction {
                F::Coordinate
            } else if epistemic {
                if state_hada {
                    F::StateEmbeddedQuestion
                } else {
                    F::ActionEmbeddedQuestion
                }
            } else if mode == "CAUSE_UNKNOWN" {
                if state_hada {
                    F::StateAttributive
                } else {
                    F::ActionAttributive
                }
            } else if mode == "ASK" {
                if formal {
                    F::FormalQuestion
                } else {
                    F::PoliteQuestion
                }
            } else if formal {
                F::FormalStatement
            } else if mode == "REMEMBER" {
                F::RememberInformal
            } else {
                F::InformalStatement
            };
            let punctuation = if conditional || conjunction {
                ","
            } else if mode == "ASK" {
                "?"
            } else if epistemic || mode == "CAUSE_UNKNOWN" {
                ""
            } else {
                "."
            };
            if let Some(negation) = negation {
                // Preserve the existing neutral negative acknowledgement while
                // sharing every productive HADA ending with the input parser.
                let negative_form = if mode == "REMEMBER" {
                    F::InformalStatement
                } else {
                    form
                };
                let (connective, auxiliary) =
                    crate::korean_hada::negative_components(negative_form)
                        .expect("productive Korean HADA negative form");
                push_expression_token(&mut output, predicate, connective.into());
                if let Some(token) = output.last_mut() {
                    token.attach_left = true;
                }
                push_expression_token(&mut output, negation, format!("{auxiliary}{punctuation}"));
            } else {
                let ending = crate::korean_hada::suffix(form, true)
                    .expect("productive Korean HADA positive form");
                push_expression_token(&mut output, predicate, format!("{ending}{punctuation}"));
                if let Some(token) = output.last_mut() {
                    token.attach_left = true;
                }
            }
            if epistemic {
                push_grammar_token(
                    &mut output,
                    if formal {
                        "판단할 수 없습니다."
                    } else {
                        "판단할 수 없어."
                    },
                    "WORLD.WITHHOLD_JUDGMENT",
                    &clause.event_node_id,
                );
            }
            if mode == "CAUSE_UNKNOWN" {
                push_grammar_token(
                    &mut output,
                    if formal {
                        "이유는 아직 모르겠습니다."
                    } else {
                        "이유는 아직 모르겠어."
                    },
                    "WORLD.NO_CAUSAL_EXPLANATION",
                    &clause.event_node_id,
                );
            }
        } else {
            let Some(object) = object else {
                return Vec::new();
            };
            let o = &object.expression.lexical_root;
            if property.expression.morphology != ExpressionMorphologyClassIR::EnglishRegularRelation
            {
                return Vec::new();
            }
            if mode == "ASK" {
                push_expression_token(
                    &mut output,
                    predicate,
                    if s == "you" { "do" } else { "does" }.into(),
                );
            }
            push_expression_token(&mut output, subject, s.clone());
            if let Some(negation) = negation {
                if mode != "ASK" {
                    push_expression_token(
                        &mut output,
                        predicate,
                        if s == "you" { "do" } else { "does" }.into(),
                    );
                }
                push_expression_token(&mut output, negation, "not".into());
            }
            push_expression_token(
                &mut output,
                property,
                if mode == "ASK" || negation.is_some() || s == "you" {
                    root.clone()
                } else {
                    english_third_person(root)
                },
            );
            push_expression_token(&mut output, object, o.clone());
            push_grammar_token(
                &mut output,
                if conditional || conjunction {
                    ","
                } else if mode == "ASK" {
                    "?"
                } else {
                    "."
                },
                "EN.CLAUSE_PUNCTUATION",
                &clause.event_node_id,
            );
            if let Some(token) = output.last_mut() {
                token.attach_left = true;
            }
        }
        return output;
    }
    if !korean && mode == "ASK" {
        push_expression_token(
            &mut output,
            predicate,
            if subject.expression.lexical_root == "you" {
                "are"
            } else {
                "is"
            }
            .into(),
        );
    }
    let surface = &subject.expression.lexical_root;
    push_expression_token(
        &mut output,
        subject,
        if korean {
            world_korean_subject(
                surface,
                matches!(mode, "CAUSE_UNKNOWN" | "IF" | "AND_IF" | "IF_AND" | "AND"),
            )
        } else {
            surface.clone()
        },
    );
    if !korean && mode != "ASK" {
        push_expression_token(
            &mut output,
            predicate,
            if surface == "you" { "are" } else { "is" }.into(),
        );
    }
    if !korean {
        if let Some(negation) = negation {
            push_expression_token(&mut output, negation, "not".into());
        }
    }
    push_expression_token(
        &mut output,
        property,
        property.expression.lexical_root.clone(),
    );
    if korean {
        let epistemic = matches!(mode, "CONFLICT" | "UNKNOWN" | "BOUND");
        use crate::korean_copula::KoreanCopulaFormIR as F;
        let form = if matches!(mode, "IF" | "AND_IF") {
            F::Conditional
        } else if matches!(mode, "IF_AND" | "AND") {
            F::Coordinate
        } else if epistemic {
            F::EmbeddedQuestion
        } else if mode == "CAUSE_UNKNOWN" {
            F::Attributive
        } else if mode == "ASK" {
            if formal {
                F::FormalQuestion
            } else {
                F::PoliteQuestion
            }
        } else if formal {
            F::FormalStatement
        } else if mode == "REMEMBER" {
            F::RememberInformal
        } else {
            F::InformalStatement
        };
        let punctuation = if matches!(mode, "IF" | "AND_IF" | "IF_AND" | "AND") {
            ","
        } else if mode == "ASK" {
            "?"
        } else if epistemic || mode == "CAUSE_UNKNOWN" {
            ""
        } else {
            "."
        };
        let final_coda = has_korean_final_consonant(&property.expression.lexical_root);
        if let Some(negation) = negation {
            let negative_form = if mode == "REMEMBER" {
                F::InformalStatement
            } else {
                form
            };
            let (particle, auxiliary) =
                crate::korean_copula::negative_components(negative_form, final_coda);
            push_expression_token(&mut output, predicate, particle.into());
            if let Some(token) = output.last_mut() {
                token.attach_left = true;
            }
            push_expression_token(&mut output, negation, format!("{auxiliary}{punctuation}"));
        } else {
            let ending = crate::korean_copula::positive_suffix(form, final_coda);
            push_expression_token(&mut output, predicate, format!("{ending}{punctuation}"));
            if let Some(token) = output.last_mut() {
                token.attach_left = true;
            }
        }
        if epistemic {
            push_grammar_token(
                &mut output,
                if formal {
                    "판단할 수 없습니다."
                } else {
                    "판단할 수 없어."
                },
                "WORLD.WITHHOLD_JUDGMENT",
                &clause.event_node_id,
            );
        }
        if mode == "CAUSE_UNKNOWN" {
            push_grammar_token(
                &mut output,
                if formal {
                    "이유는 아직 모르겠습니다."
                } else {
                    "이유는 아직 모르겠어."
                },
                "WORLD.NO_CAUSAL_EXPLANATION",
                &clause.event_node_id,
            );
        }
    } else {
        push_grammar_token(
            &mut output,
            if matches!(mode, "IF" | "IF_AND" | "AND" | "AND_IF") {
                ","
            } else if mode == "ASK" {
                "?"
            } else {
                "."
            },
            "EN.CLAUSE_PUNCTUATION",
            &clause.event_node_id,
        );
        if let Some(token) = output.last_mut() {
            token.attach_left = true;
        }
    }
    output
}

fn world_entity_root(entity: &str, language: LanguageCodeIR) -> String {
    match (entity, language) {
        ("__user__", LanguageCodeIR::Korean) => "너".into(),
        ("__user__", LanguageCodeIR::English) => "you".into(),
        _ => entity.into(),
    }
}

fn world_korean_subject(root: &str, nominative: bool) -> String {
    if crate::korean_nominal::surface_coda(root).is_none() {
        return format!("‘{root}’ 개체{}", if nominative { "가" } else { "는" });
    }
    if !nominative {
        return format!("{root}{}", korean_particle(root, "은", "는"));
    }
    match root {
        "나" => "내가".into(),
        "저" => "제가".into(),
        "너" => "네가".into(),
        _ => format!("{root}{}", korean_particle(root, "이", "가")),
    }
}

fn world_korean_topic(root: &str) -> String {
    world_korean_subject(root, false)
}

fn world_korean_object(root: &str) -> String {
    if crate::korean_nominal::surface_coda(root).is_none() {
        format!("‘{root}’ 개체를")
    } else {
        format!("{root}{}", korean_particle(root, "을", "를"))
    }
}

#[cfg(test)]
mod register_tests {
    use super::*;

    #[test]
    fn opaque_latin_entities_use_korean_head_nouns_instead_of_guessed_particles() {
        assert_eq!(world_korean_topic("node"), "‘node’ 개체는");
        assert_eq!(world_korean_subject("node", true), "‘node’ 개체가");
        assert_eq!(world_korean_object("node"), "‘node’ 개체를");
        assert_eq!(world_korean_topic("장치"), "장치는");
        assert_eq!(world_korean_subject("장치", true), "장치가");
        assert_eq!(world_korean_object("장치"), "장치를");
    }

    #[test]
    fn receipt_register_composes_with_each_world_clause_family() {
        for modes in [
            vec!["REMEMBER"],
            vec!["IF", "THEN"],
            vec!["IF_AND", "AND_IF", "THEN"],
        ] {
            for value in [true, false] {
                let clauses = modes
                    .iter()
                    .enumerate()
                    .map(|(index, &mode)| {
                        (
                            WorldAtomIR {
                                entity: format!("Zev{index}"),
                                property: crate::world_dialogue::WorldPropertyIR::Active,
                                object: None,
                            },
                            value,
                            mode,
                            vec![format!("REGISTER_SOURCE:{index}")],
                        )
                    })
                    .collect::<Vec<_>>();
                let baseline = generate_world_clauses(
                    LanguageCodeIR::Korean,
                    clauses.clone(),
                    &WorldVocabularyIR::default(),
                )
                .unwrap();
                MORPHOLOGY_PASSES.with(|c| c.set(0));
                let formal = generate_world_clauses(
                    GenerationSettings::with_policy(
                        LanguageCodeIR::Korean,
                        &crate::affective_field::AffectiveRealizationPolicyIR {
                            formal: true,
                            ..Default::default()
                        },
                    ),
                    clauses,
                    &WorldVocabularyIR::default(),
                )
                .unwrap();
                assert_eq!(MORPHOLOGY_PASSES.with(|c| c.get()), 1);
                assert_eq!(baseline.meaning, formal.meaning);
                assert_eq!(baseline.verification, formal.verification);
                assert!(formal.validate());
                assert!(baseline.morphology.realized_text.starts_with("알겠어,"));
                assert!(formal.morphology.realized_text.starts_with("알겠습니다,"));
                assert!(!formal.morphology.realized_text.contains("알겠어,"));
            }
        }
    }
}
