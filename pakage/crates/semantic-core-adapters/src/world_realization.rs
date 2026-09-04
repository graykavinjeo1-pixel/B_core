//! Clause construction from a sealed core decision, never from the input string.
//! State predicates, object identifiers, polarity and epistemic force are separate
//! nodes. The same clause grammar is used for every object and property.

use super::*;
use crate::world_dialogue::{WorldAtomIR, WorldMemoryUpdateIR, WorldReasoningIR};
use crate::world_vocabulary::{english_third_person, WorldLexicalGrammarIR, WorldVocabularyIR};

pub(crate) fn generate_world_clarification(
    language: LanguageCodeIR,
    c: &crate::world_dialogue::WorldClarificationIR,
) -> Result<GenerativeLanguageIR, String> {
    if !c.validate() {
        return Err("INVALID_REFERENCE_GAP".into());
    }
    let candidates = c.gap.candidates();
    let mut store = ExpressionNodeStore::default();
    let mut nodes = Vec::new();
    for (id, concept, root, kind, pos) in [
        (
            "R".to_string(),
            "C_WORLD_CLAUSE_REFERENCE".to_string(),
            if language == LanguageCodeIR::Korean {
                "말하다"
            } else {
                "mean"
            }
            .to_string(),
            GenerationMeaningNodeKindIR::Event,
            ExpressionPartOfSpeechIR::Verb,
        ),
        (
            "A".into(),
            format!("C_ENTITY_{}", candidates[0]),
            world_entity_root(&candidates[0], language),
            GenerationMeaningNodeKindIR::Entity,
            ExpressionPartOfSpeechIR::Noun,
        ),
        (
            "B".into(),
            format!("C_ENTITY_{}", candidates[1]),
            world_entity_root(&candidates[1], language),
            GenerationMeaningNodeKindIR::Entity,
            ExpressionPartOfSpeechIR::Noun,
        ),
    ] {
        store.attach_alias(
            &format!("EXPR.REF.{id}"),
            language,
            &concept,
            &root,
            pos,
            "RUNTIME_REFERENT_SURFACE:WORLD_REFERENCE",
        )?;
        nodes.push(GenerationMeaningNodeIR {
            node_id: id,
            concept_id: concept,
            kind,
            grounding_refs: vec![format!("WORLD_REFERENCE_GAP:{}", c.gap.source_sha256)],
        });
    }
    GenerativeLanguageCortex.generate(GenerativeLanguageRequestIR {
        meaning: GenerationMeaningGraphIR::new(
            nodes,
            vec![
                meaning_edge("RA", "R", "A", GenerationMeaningRelationIR::Theme),
                meaning_edge("RB", "R", "B", GenerationMeaningRelationIR::Goal),
            ],
        ),
        context: GenerationContextIR {
            language,
            register: LanguageRegisterIR::Informal,
            tense: GenerationTenseIR::Present,
            emotion: GenerationEmotionIR::Neutral,
            urgency_millis: 0,
            default_speech_intent: GenerationSpeechIntentIR::Ask,
        },
        expressions: &store,
    })
}

pub(crate) fn generate_world_decision(
    language: LanguageCodeIR,
    world: &WorldReasoningIR,
) -> Result<GenerativeLanguageIR, String> {
    if !world.validate() {
        return Err("INVALID_WORLD_DECISION".into());
    }
    let clauses = world
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
        .collect::<Result<Vec<_>, String>>()?;
    generate_world_clauses(language, clauses, &world.memory.vocabulary)
}

pub(crate) fn generate_world_memory_update(
    language: LanguageCodeIR,
    update: &WorldMemoryUpdateIR,
) -> Result<GenerativeLanguageIR, String> {
    if !update.validate() {
        return Err("INVALID_WORLD_MEMORY_UPDATE".into());
    }
    let mut clauses = Vec::new();
    let refs = vec![format!("WORLD_MEMORY_TURN:{}", update.turn)];
    if let Some(p) = update
        .memory
        .premises
        .iter()
        .find(|p| p.introduced_turn == update.turn)
    {
        clauses.push((p.atom.clone(), p.value, "REMEMBER", refs));
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
    generate_world_clauses(language, clauses, &update.memory.vocabulary)
}

fn generate_world_clauses(
    language: LanguageCodeIR,
    clauses: Vec<(WorldAtomIR, bool, &str, Vec<String>)>,
    vocabulary: &WorldVocabularyIR,
) -> Result<GenerativeLanguageIR, String> {
    // The underlying proof depth is bounded. Do not silently truncate its explanation.
    if clauses.len() > 30 {
        return Err("WORLD_EXPLANATION_BOUND".into());
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
                        WorldLexicalGrammarIR::KoreanHadaState => {
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
    GenerativeLanguageCortex.generate(GenerativeLanguageRequestIR {
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
    if predicate.expression.concept_id == "C_WORLD_CLAUSE_REFERENCE" {
        let Some(other) = constituent_selection(clause, SyntaxConstituentRoleIR::Goal, selected)
        else {
            return output;
        };
        let a = &subject.expression.lexical_root;
        let b = &other.expression.lexical_root;
        if context.language == LanguageCodeIR::Korean {
            push_expression_token(
                &mut output,
                subject,
                format!("{a}{}", korean_particle(a, "을", "를")),
            );
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
            push_expression_token(
                &mut output,
                other,
                format!("{b}{}", korean_particle(b, "을", "를")),
            );
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
        (true, "REMEMBER") => "알겠어,",
        (true, "IF" | "IF_AND") => "알겠어,",
        (true, "AND" | "AND_IF") => "그리고",
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
                world_korean_subject(s, mode == "CAUSE_UNKNOWN"),
            );
            if let Some(object) = object {
                let o = &object.expression.lexical_root;
                let marker = match property.expression.morphology {
                    ExpressionMorphologyClassIR::KoreanHadaLocative => "에",
                    ExpressionMorphologyClassIR::KoreanHadaAccusative => {
                        korean_particle(o, "을", "를")
                    }
                    _ => return Vec::new(),
                };
                push_expression_token(&mut output, object, format!("{o}{marker}"));
            }
            push_expression_token(&mut output, property, root.clone());
            let ending = if negation.is_some() {
                "하지"
            } else if conditional {
                "하면,"
            } else if conjunction {
                "하고,"
            } else if epistemic {
                if property.expression.morphology == ExpressionMorphologyClassIR::KoreanHada {
                    "한지"
                } else {
                    "하는지"
                }
            } else if mode == "CAUSE_UNKNOWN" {
                if property.expression.morphology == ExpressionMorphologyClassIR::KoreanHada {
                    "한"
                } else {
                    "하는"
                }
            } else if mode == "ASK" {
                "하나요?"
            } else if formal {
                "합니다."
            } else if mode == "REMEMBER" {
                "하구나."
            } else {
                "해."
            };
            push_expression_token(&mut output, predicate, ending.into());
            if let Some(token) = output.last_mut() {
                token.attach_left = true;
            }
            if let Some(negation) = negation {
                let ending = if conditional {
                    "않으면,"
                } else if conjunction {
                    "않고,"
                } else if epistemic {
                    if property.expression.morphology == ExpressionMorphologyClassIR::KoreanHada {
                        "않은지"
                    } else {
                        "않는지"
                    }
                } else if mode == "CAUSE_UNKNOWN" {
                    if property.expression.morphology == ExpressionMorphologyClassIR::KoreanHada {
                        "않은"
                    } else {
                        "않는"
                    }
                } else if mode == "ASK" {
                    "않나요?"
                } else if formal {
                    "않습니다."
                } else {
                    "않아."
                };
                push_expression_token(&mut output, negation, ending.into());
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
            world_korean_subject(surface, mode == "CAUSE_UNKNOWN")
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
        if let Some(negation) = negation {
            push_expression_token(
                &mut output,
                predicate,
                korean_particle(&property.expression.lexical_root, "이", "가").into(),
            );
            if let Some(token) = output.last_mut() {
                token.attach_left = true;
            }
            let ending = if matches!(mode, "IF" | "AND_IF") {
                "아니면,"
            } else if matches!(mode, "IF_AND" | "AND") {
                "아니고,"
            } else if epistemic {
                "아닌지"
            } else if mode == "CAUSE_UNKNOWN" {
                "아닌"
            } else if mode == "ASK" {
                "아닌가요?"
            } else if formal {
                "아닙니다."
            } else {
                "아니야."
            };
            push_expression_token(&mut output, negation, ending.into());
        } else {
            let ending = if matches!(mode, "IF" | "AND_IF") {
                "이면,"
            } else if matches!(mode, "IF_AND" | "AND") {
                "이고,"
            } else if epistemic {
                "인지"
            } else if mode == "CAUSE_UNKNOWN" {
                "인"
            } else if mode == "ASK" {
                "인가요?"
            } else if formal {
                "입니다."
            } else if mode == "REMEMBER" {
                korean_particle(&property.expression.lexical_root, "이구나.", "구나.")
            } else {
                korean_particle(&property.expression.lexical_root, "이야.", "야.")
            };
            push_expression_token(&mut output, predicate, ending.into());
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
