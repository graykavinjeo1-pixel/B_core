//! Bounded, source-attributed event recap. This projects event roles; it does
//! not retrieve a stored summary, infer a cause, or promote a reported fact.
use super::*;
use crate::proposition_content::{
    ContentSlotIR, DescribedEventIR, EventSourceIR, PropositionContentIR,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EventSummaryIR {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub omitted_roles: Vec<ContentSlotIR>,
    pub belief_id: String,
    pub source_actor: String,
    pub source_proposition: String,
    pub context_sources: Vec<EventSourceIR>,
    pub event: DescribedEventIR,
}

impl EventSummaryIR {
    pub(crate) fn can_acknowledge(&self, language: LanguageCodeIR) -> bool {
        self.event.kind == crate::proposition_content::DescriptionKindIR::State
            && self.source_actor == "DIALOGUE_USER"
            && self.can_realize(language)
            && (language == LanguageCodeIR::English
                || source_predicate_stem(&self.event.predicate_surface).is_some())
    }
    pub(crate) fn can_realize(&self, language: LanguageCodeIR) -> bool {
        self.validate() && self.finite_form(language).is_some()
    }

    pub fn validate(&self) -> bool {
        !self.belief_id.is_empty()
            && crate::proposition_content::valid_elaboration_omissions(
                &self.event,
                &self.omitted_roles,
            )
            && !self.source_actor.is_empty()
            && !self.event.has_references()
            && if self.event.kind == crate::proposition_content::DescriptionKindIR::State {
                self.event.roles.len() == 1 && self.event.roles.contains_key(&ContentSlotIR::Theme)
            } else {
                self.event.roles.contains_key(&ContentSlotIR::Agent)
            }
            && self.event.roles.keys().all(|r| {
                matches!(
                    r,
                    ContentSlotIR::Agent
                        | ContentSlotIR::Theme
                        | ContentSlotIR::Recipient
                        | ContentSlotIR::Source
                        | ContentSlotIR::Location
                        | ContentSlotIR::Time
                        | ContentSlotIR::Duration
                )
            })
            && PropositionContentIR::compile_contextual(
                &self.source_proposition,
                &self.context_sources,
            )
            .is_some_and(|c| c.events == [self.event.clone()])
    }

    // Preserve the reported finite form rather than inventing tense from a
    // lemma. Cross-language predicate/argument translation is not licensed by
    // a dictionary entry with multiple senses, so this first projection is
    // monolingual on each of the Korean and English paths.
    fn finite_form(&self, language: LanguageCodeIR) -> Option<String> {
        let korean = self
            .source_proposition
            .chars()
            .any(|c| ('가'..='힣').contains(&c));
        if korean != (language == LanguageCodeIR::Korean) {
            return None;
        }
        let raw = crate::proposition_content::reported_event_surface(&self.source_proposition)
            .trim()
            .trim_end_matches(['.', '?', '!']);
        let words = raw.split_whitespace().collect::<Vec<_>>();
        if self.event.kind == crate::proposition_content::DescriptionKindIR::State {
            return words
                .join(" ")
                .ends_with(&self.event.predicate_surface)
                .then(|| self.event.predicate_surface.clone());
        }
        if korean {
            if self.event.negated {
                let last = *words.last()?;
                if matches!(last, "않았어" | "않았다") {
                    let prior = *words.get(words.len().checked_sub(2)?)?;
                    return prior.ends_with('지').then(|| format!("{prior} {last}"));
                }
                let pos = words.iter().position(|w| matches!(*w, "안" | "못"))?;
                return (words.get(pos + 1)? == &self.event.predicate_surface)
                    .then(|| format!("{} {}", words[pos], self.event.predicate_surface));
            }
            return words
                .contains(&self.event.predicate_surface.as_str())
                .then(|| self.event.predicate_surface.clone());
        }
        // Passive-to-active tense conversion requires a separate grammar;
        // fail closed rather than output "Nico written the note".
        if words.iter().any(|w| {
            matches!(
                w.to_lowercase().as_str(),
                "was" | "were" | "is" | "are" | "been"
            )
        }) {
            return None;
        }
        let pos = words
            .iter()
            .position(|w| w.eq_ignore_ascii_case(&self.event.predicate_surface))?;
        if self.event.negated {
            if pos < 2 || !words[pos - 1].eq_ignore_ascii_case("not") {
                return None;
            }
            let aux = words[pos - 2].to_lowercase();
            return matches!(aux.as_str(), "did" | "does" | "do")
                .then(|| format!("{aux} not {}", self.event.predicate_surface));
        }
        Some(self.event.predicate_surface.clone())
    }
}

pub(crate) fn generate_event_summary(
    settings: impl Into<GenerationSettings>,
    summary: &EventSummaryIR,
) -> Result<GenerativeLanguageIR, String> {
    generate_event_summaries(settings, std::slice::from_ref(summary))
}

/// Select a clause from the already-validated answer roles and given query
/// arguments. This is a pre-generation plan, not a draft or a new fact.
pub(crate) fn role_answer_clause(
    projection: &crate::proposition_content::ContentProjectionIR,
    question: &str,
    language: LanguageCodeIR,
) -> Option<EventSummaryIR> {
    if projection.additional_bindings.is_empty()
        || !projection.co_answers.is_empty()
        || projection.event_perspective.is_some()
        || !projection.validate()
        || !projection.matches_question(question)
    {
        return None;
    }
    let query = crate::proposition_content::described_event(question, true)?;
    if query.has_references() {
        return None;
    }
    let content = PropositionContentIR::compile_contextual(
        &projection.source_proposition,
        &projection.context_sources,
    )?;
    let event = content
        .events
        .into_iter()
        .find(|e| Some(&e.event_id) == projection.binding.event_id.as_ref())?;
    let omitted_roles = event
        .roles
        .iter()
        .filter_map(|(role, value)| {
            let selected = projection
                .all_bindings()
                .any(|b| b.slot == *role && b.value == *value);
            let given = query.roles.get(role) == Some(value);
            (!selected && !given).then_some(*role)
        })
        .collect();
    let summary = EventSummaryIR {
        omitted_roles,
        belief_id: projection.belief_id.clone(),
        source_actor: projection.source_actor.clone(),
        source_proposition: projection.source_proposition.clone(),
        context_sources: projection.context_sources.clone(),
        event,
    };
    // Core arguments cannot be dropped merely to force a fluent sentence.
    // Unsupported omissions, perspectives and languages retain role projection.
    summary.can_realize(language).then_some(summary)
}

/// Build one discourse plan for all selected source events. Lexical selections
/// are event-local, so two participants with the same role cannot overwrite
/// one another. No draft generation or output repair is performed here.
pub(crate) fn generate_event_summaries(
    settings: impl Into<GenerationSettings>,
    summaries: &[EventSummaryIR],
) -> Result<GenerativeLanguageIR, String> {
    generate_descriptions(settings.into(), summaries, GenerationSpeechIntentIR::Inform)
}

pub(crate) fn generate_state_acknowledgement(
    settings: GenerationSettings,
    summary: &EventSummaryIR,
) -> Result<GenerativeLanguageIR, String> {
    if !summary.can_acknowledge(settings.language) {
        return Err("UNSUPPORTED_STATE_ACKNOWLEDGEMENT".into());
    }
    generate_descriptions(
        settings,
        std::slice::from_ref(summary),
        GenerationSpeechIntentIR::Acknowledge,
    )
}

fn generate_descriptions(
    settings: GenerationSettings,
    summaries: &[EventSummaryIR],
    speech_intent: GenerationSpeechIntentIR,
) -> Result<GenerativeLanguageIR, String> {
    let language = settings.language;
    if summaries.is_empty() || summaries.len() > 8 {
        return Err("INVALID_EVENT_SUMMARY_COUNT".into());
    }
    let mut store = ExpressionNodeStore::bilingual_builtin();
    let mut nodes = Vec::new();
    let mut edges = Vec::new();
    for (index, summary) in summaries.iter().enumerate() {
        if !summary.validate() {
            return Err("INVALID_EVENT_SUMMARY_SOURCE".into());
        }
        let mut finite = summary
            .finite_form(language)
            .ok_or("UNSUPPORTED_EVENT_SUMMARY_MORPHOLOGY")?;
        let source_speaker_is_bearer = summary.event.kind
            == crate::proposition_content::DescriptionKindIR::State
            && summary.source_actor == "DIALOGUE_USER"
            && summary
                .event
                .roles
                .get(&ContentSlotIR::Theme)
                .is_some_and(|bearer| {
                    matches!(
                        bearer.to_lowercase().as_str(),
                        "i" | "나" | "저" | "내" | "제"
                    )
                });
        if source_speaker_is_bearer && language == LanguageCodeIR::English {
            if let Some(rest) = finite.strip_prefix("am ") {
                finite = format!("are {rest}");
            } else if let Some(rest) = finite.strip_prefix("was ") {
                finite = format!("were {rest}");
            }
        }
        let mut grounding = vec![
            format!("DIALOGUE_BELIEF_ID:{}", summary.belief_id),
            format!("SOURCE_EVENT:{}", summary.event.event_id),
            format!(
                "SOURCE_SHA256:{:x}",
                Sha256::digest(summary.source_proposition.as_bytes())
            ),
        ];
        if source_speaker_is_bearer {
            grounding.push("DIALOGUE_DEIXIS:SOURCE_SPEAKER_AS_CURRENT_ADDRESSEE".into());
        }
        let predicate_concept = format!(
            "C_EVENT_RECAP_{}{}",
            if summary.event.kind == crate::proposition_content::DescriptionKindIR::State {
                "STATE_"
            } else {
                ""
            },
            summary.event.lexical_entry_ids.join("_")
        );
        let mut add = |id: &str, concept: &str, surface: &str, event: bool| -> Result<(), String> {
            let id = format!("{id}@{index}");
            let concept = format!("{concept}@{index}");
            store.attach_alias(
                &format!("EXPR.RECAP.{id}"),
                language,
                &concept,
                surface,
                if event {
                    ExpressionPartOfSpeechIR::Verb
                } else {
                    ExpressionPartOfSpeechIR::Noun
                },
                "RUNTIME_REFERENT_SURFACE:SOURCE_EVENT_PROJECTION",
            )?;
            nodes.push(GenerationMeaningNodeIR {
                node_id: id,
                concept_id: concept,
                kind: if event {
                    GenerationMeaningNodeKindIR::Event
                } else {
                    GenerationMeaningNodeKindIR::Entity
                },
                grounding_refs: grounding.clone(),
            });
            Ok(())
        };
        let meaning_edge = |id: &str, from: &str, to: &str, relation| {
            super::meaning_edge(
                &format!("{id}@{index}"),
                &format!("{from}@{index}"),
                &format!("{to}@{index}"),
                relation,
            )
        };
        add("RECAP", &predicate_concept, &finite, true)?;
        let actor = if summary.source_actor == "DIALOGUE_USER" {
            if language == LanguageCodeIR::Korean {
                "네 말"
            } else {
                "your account"
            }
        } else {
            &summary.source_actor
        };
        add("ATTRIBUTION", "C_RECAP_ATTRIBUTION", actor, false)?;
        edges.push(meaning_edge(
            "ATTRIBUTION",
            "RECAP",
            "ATTRIBUTION",
            GenerationMeaningRelationIR::Goal,
        ));
        for (role, value) in &summary.event.roles {
            if summary.omitted_roles.contains(role) {
                continue;
            }
            let id = format!("ROLE_{role:?}");
            let mut surface = value.clone();
            if source_speaker_is_bearer && *role == ContentSlotIR::Theme {
                surface = if language == LanguageCodeIR::English {
                    "you"
                } else {
                    "너"
                }
                .into();
            }
            if language == LanguageCodeIR::English && *role != ContentSlotIR::Agent {
                let words =
                    crate::proposition_content::reported_event_surface(&summary.source_proposition)
                        .trim_end_matches(['.', '?', '!'])
                        .split_whitespace()
                        .collect::<Vec<_>>();
                let noun = value.split_whitespace().collect::<Vec<_>>();
                let matches = words
                    .windows(noun.len() + 1)
                    .filter(|span| {
                        matches!(span[0].to_lowercase().as_str(), "a" | "an" | "the")
                            && span[1..]
                                .iter()
                                .zip(&noun)
                                .all(|(a, b)| a.eq_ignore_ascii_case(b))
                    })
                    .collect::<Vec<_>>();
                if matches.len() == 1 {
                    surface = matches[0].join(" ");
                }
            }
            // Determiners lose sentence-initial casing inside a composed
            // clause; never lowercase arbitrary names or the whole source.
            if language == LanguageCodeIR::English {
                for (initial, medial) in [("The ", "the "), ("A ", "a "), ("An ", "an ")] {
                    if let Some(rest) = surface.strip_prefix(initial) {
                        surface = format!("{medial}{rest}");
                        break;
                    }
                }
            }
            add(&id, &format!("C_RECAP_ROLE_{role:?}"), &surface, false)?;
            edges.push(meaning_edge(
                &id,
                "RECAP",
                &id,
                GenerationMeaningRelationIR::Property,
            ));
        }
        if summary.event.negated {
            add("NEGATION", "C_RECAP_NEGATION", "NEGATED", false)?;
            edges.push(meaning_edge(
                "NEGATION",
                "RECAP",
                "NEGATION",
                GenerationMeaningRelationIR::Negates,
            ));
        }
        // A location preposition can encode a distinction. Preserve it as a
        // source-bound expression, not a default that turns every "in" into "at".
        if language == LanguageCodeIR::English
            && summary.event.roles.contains_key(&ContentSlotIR::Location)
            && !summary.omitted_roles.contains(&ContentSlotIR::Location)
        {
            let prep =
                if crate::proposition_content::reported_event_surface(&summary.source_proposition)
                    .to_lowercase()
                    .contains(" in ")
                {
                    "in"
                } else {
                    "at"
                };
            add("LOCATION_PREP", "C_RECAP_LOCATION_PREP", prep, false)?;
            edges.push(meaning_edge(
                "LOCATION_PREP",
                "RECAP",
                "LOCATION_PREP",
                GenerationMeaningRelationIR::Property,
            ));
        }
        if summary.source_actor == "DIALOGUE_USER" && language == LanguageCodeIR::Korean {
            let mut polite_source = expression(
                &format!("EXPR.RECAP.ATTRIBUTION.FORMAL@{index}"),
                language,
                &format!("C_RECAP_ATTRIBUTION@{index}"),
                "말씀",
                ExpressionPartOfSpeechIR::Noun,
                ExpressionMorphologyClassIR::KoreanInvariable,
                LanguageRegisterIR::Formal,
            );
            polite_source.provenance = "RUNTIME_REFERENT_SURFACE:SOURCE_EVENT_REGISTER".into();
            store.inject(polite_source)?;
        }
        if index > 0 {
            edges.push(super::meaning_edge(
                &format!("RECAP_SEQUENCE_{index}"),
                &format!("RECAP@{}", index - 1),
                &format!("RECAP@{index}"),
                GenerationMeaningRelationIR::Sequence,
            ));
        }
    }
    settings.generate(GenerativeLanguageRequestIR {
        meaning: GenerationMeaningGraphIR::new(nodes, edges),
        context: GenerationContextIR {
            language,
            register: LanguageRegisterIR::Informal,
            tense: GenerationTenseIR::SourcePreserved,
            emotion: GenerationEmotionIR::Neutral,
            urgency_millis: 0,
            default_speech_intent: speech_intent,
        },
        expressions: &store,
    })
}

/// Select a grammatical stem from indexed morphology, never a reply sentence.
/// Past morphology stays past; local negation remains in the prefix/head.
fn source_predicate_stem(surface: &str) -> Option<String> {
    let (prefix, head) = surface
        .rsplit_once(char::is_whitespace)
        .unwrap_or(("", surface));
    let lookup = crate::lexical_knowledge_pack::builtin_pack().lookup(head);
    if lookup.truncated {
        return None;
    }
    let stems = lookup
        .matches
        .iter()
        .filter(|m| {
            m.matched_form == head
                && matches!(
                    m.entry.pos.as_str(),
                    "형용사" | "동사" | "보조 형용사" | "보조 동사"
                )
        })
        .filter_map(|m| {
            match m.morphology.grammar_rule.as_str() {
                "KO_PRINCIPAL_FORM_PAST_ENDING" => Some(m.morphology.base.clone()),
                "KO_HADA_FINITE_CONTRACTION" if m.morphology.ending.starts_with("했") => m
                    .entry
                    .lemma
                    .strip_suffix("하다")
                    .map(|stem| format!("{stem}했")),
                "KO_CONNECTIVE_CONTRACTION"
                | "KO_CONNECTIVE_POLITE"
                | "KO_STEM_SENTENTIAL_ENDING"
                | "KO_HADA_FINITE_CONTRACTION"
                | "LEXICAL_LEMMA" => m.entry.lemma.strip_suffix('다').map(str::to_string),
                "SOURCE_KOREAN_PRINCIPAL_FORM"
                    if crate::lexical_knowledge_pack::is_connective_principal_form(head) =>
                {
                    m.entry.lemma.strip_suffix('다').map(str::to_string)
                }
                // Modal, adnominal and conditional endings cannot become a
                // plain present statement just to satisfy a register setting.
                _ => None,
            }
        })
        .collect::<BTreeSet<_>>();
    if stems.len() != 1 {
        // A source-authorized nominal status (for example, "오프라인이야")
        // is not a lexical verb. Preserve its copular stem solely for the
        // acknowledgement ending; do not promote the nominal value into a
        // dictionary predicate or infer a new state.
        let nominal_copula = ["이에요", "이야"]
            .iter()
            .find_map(|ending| head.strip_suffix(ending))
            .filter(|base| !base.trim().is_empty())
            .map(|base| format!("{base}이"));
        return nominal_copula.map(|stem| {
            if prefix.is_empty() {
                stem
            } else {
                format!("{prefix} {stem}")
            }
        });
    }
    let stem = stems.into_iter().next()?;
    Some(if prefix.is_empty() {
        stem
    } else {
        format!("{prefix} {stem}")
    })
}

pub(super) fn realize_event_summary(
    clause: &SyntaxClauseIR,
    context: &GenerationContextIR,
    selected: &BTreeMap<(&str, &str), &ExpressionSelectionIR>,
    predicate: &ExpressionSelectionIR,
) -> Vec<MorphologicalTokenIR> {
    let mut out = Vec::new();
    let korean = context.language == LanguageCodeIR::Korean;
    let state = predicate
        .expression
        .concept_id
        .starts_with("C_EVENT_RECAP_STATE_");
    let acknowledgement =
        state && context.default_speech_intent == GenerationSpeechIntentIR::Acknowledge;
    let selections = clause
        .constituents
        .iter()
        .filter_map(|c| {
            selected
                .get(&(c.expression_id.as_str(), c.meaning_node_id.as_str()))
                .copied()
        })
        .collect::<Vec<_>>();
    let get = |concept: &str| {
        selections
            .iter()
            .copied()
            .find(|s| s.expression.concept_id.split('@').next() == Some(concept))
    };
    let Some(source) = get("C_RECAP_ATTRIBUTION") else {
        return out;
    };
    if acknowledgement {
        push_grammar_token(
            &mut out,
            if korean { "" } else { "I see," },
            "STATE.ACKNOWLEDGE_CURRENT_USER_REPORT",
            &source.meaning_node_id,
        );
    } else if korean {
        push_expression_token(
            &mut out,
            source,
            format!("{}에 따르면,", source.expression.lexical_root),
        );
    } else {
        push_grammar_token(
            &mut out,
            "According to",
            "EN.ATTRIBUTED_EVENT",
            &clause.event_node_id,
        );
        push_expression_token(
            &mut out,
            source,
            format!(
                "{},",
                english_embedded_nominal(&source.expression.lexical_root)
            ),
        );
    }
    let roles = if korean {
        [
            "Time",
            "Agent",
            "Location",
            "Source",
            "Recipient",
            "Duration",
            "Theme",
        ]
    } else {
        [
            "Agent",
            "Theme",
            "Recipient",
            "Source",
            "Location",
            "Duration",
            "Time",
        ]
    };
    for role in roles {
        if let Some(s) = get(&format!("C_RECAP_ROLE_{role}")) {
            let root = &s.expression.lexical_root;
            if korean {
                let suffix = match role {
                    "Agent" => korean_particle(root, "이", "가"),
                    "Theme" if state => korean_particle(root, "이", "가"),
                    "Theme" => korean_particle(root, "을", "를"),
                    "Recipient" => "에게",
                    "Source" => "에게서",
                    "Location" => "에서",
                    "Duration" => " 동안",
                    _ => "",
                };
                let surface = if state && role == "Theme" && root == "너" {
                    "네가".to_string()
                } else {
                    format!("{root}{suffix}")
                };
                push_expression_token(&mut out, s, surface);
            } else {
                let prep = match role {
                    "Recipient" => "to",
                    "Source" => "from",
                    "Duration" => "for",
                    _ => "",
                };
                if !prep.is_empty() {
                    push_grammar_token(
                        &mut out,
                        prep,
                        "EN.EVENT_ROLE_PREPOSITION",
                        &s.meaning_node_id,
                    );
                }
                if role == "Location" {
                    if let Some(p) = get("C_RECAP_LOCATION_PREP") {
                        push_expression_token(&mut out, p, p.expression.lexical_root.clone());
                    }
                }
                push_expression_token(&mut out, s, root.clone());
            }
        }
        if !korean && role == if state { "Theme" } else { "Agent" } {
            push_expression_token(
                &mut out,
                predicate,
                predicate.expression.lexical_root.clone(),
            );
        }
    }
    if korean {
        push_expression_token(
            &mut out,
            predicate,
            if acknowledgement {
                source_predicate_stem(&predicate.expression.lexical_root)
                    .expect("preflight checked source-backed acknowledgement morphology")
            } else if context.register == LanguageRegisterIR::Formal {
                // Re-inflect the selected predicate before emitting the clause,
                // not a finished response. Indexed past stems and the source
                // negation prefix remain intact. Unknown morphology preserves
                // the source form rather than guessing a new tense or stem.
                source_predicate_stem(&predicate.expression.lexical_root)
                    .map(|stem| korean_formal_statement(&stem))
                    .unwrap_or_else(|| predicate.expression.lexical_root.clone())
            } else {
                predicate.expression.lexical_root.clone()
            },
        );
        if acknowledgement {
            push_grammar_token(
                &mut out,
                if context.register == LanguageRegisterIR::Formal {
                    "군요"
                } else {
                    "구나"
                },
                "KO.STATE.ACKNOWLEDGEMENT_ENDING",
                &predicate.meaning_node_id,
            );
            out.last_mut().unwrap().attach_left = true;
        }
    }
    if let Some(negative) = get("C_RECAP_NEGATION") {
        // The source-validated finite verb phrase already realizes negation.
        push_grammar_token(
            &mut out,
            "",
            "EVENT.NEGATION_IN_FINITE_FORM",
            &negative.meaning_node_id,
        );
    }
    push_grammar_token(&mut out, ".", "EVENT.CLAUSE_END", &clause.event_node_id);
    if let Some(t) = out.last_mut() {
        t.attach_left = true;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn role_clause_plan_preserves_source_and_rejects_unrequested_core_arguments() {
        use crate::proposition_content::ContentProjectionIR;
        let make = |source: &str| {
            let event = PropositionContentIR::compile(source).events[0].clone();
            let binding = |slot| {
                event
                    .bindings()
                    .into_iter()
                    .find(|b| b.slot == slot)
                    .unwrap()
            };
            ContentProjectionIR {
                elaboration_omitted_roles: vec![],
                elaboration_event: None,
                event_perspective: None,
                co_answers: vec![],
                belief_id: "ROLE-SOURCE".into(),
                source_actor: "DIALOGUE_USER".into(),
                source_proposition: source.into(),
                binding: binding(ContentSlotIR::Agent),
                additional_bindings: vec![binding(ContentSlotIR::Theme)],
                context_sources: vec![],
                reference_context: None,
                reference_bindings: vec![],
            }
        };
        let p = make("Mina read a book in the library yesterday.");
        let original = p.clone();
        let c = role_answer_clause(&p, "Who read what?", LanguageCodeIR::English).unwrap();
        assert_eq!(p, original);
        assert_eq!(
            c.omitted_roles,
            [ContentSlotIR::Location, ContentSlotIR::Time]
        );
        assert!(role_answer_clause(&p, "Who wrote what?", LanguageCodeIR::English).is_none());
        assert!(role_answer_clause(&p, "Who read what?", LanguageCodeIR::Korean).is_none());
        let mut forged = p;
        forged.additional_bindings[0].value = "fabricated object".into();
        assert!(role_answer_clause(&forged, "Who read what?", LanguageCodeIR::English).is_none());
        let core_argument = make("Mina read a book to Nora yesterday.");
        assert!(
            role_answer_clause(&core_argument, "Who read what?", LanguageCodeIR::English).is_none()
        );
        assert!(role_answer_clause(
            &original,
            "Who read what happened?",
            LanguageCodeIR::English
        )
        .is_none());
        assert!(
            role_answer_clause(&original, "Who read what what?", LanguageCodeIR::English).is_none()
        );
    }

    #[test]
    fn optional_role_selection_precedes_alias_and_sentence_construction() {
        let source = "Galen read a letter in the studio yesterday.";
        let event = PropositionContentIR::compile(source).events[0].clone();
        for (role, excluded) in [
            (ContentSlotIR::Location, "studio"),
            (ContentSlotIR::Time, "yesterday"),
        ] {
            let summary = EventSummaryIR {
                omitted_roles: vec![role],
                belief_id: "SELECTION-B1".into(),
                source_actor: "DIALOGUE_USER".into(),
                source_proposition: source.into(),
                context_sources: vec![],
                event: event.clone(),
            };
            assert!(summary.can_realize(LanguageCodeIR::English));
            let generated = generate_event_summary(LanguageCodeIR::English, &summary).unwrap();
            assert!(generated.validate());
            assert!(!generated.morphology.realized_text.contains(excluded));
            assert!(generated
                .morphology
                .realized_text
                .contains("Galen read a letter"));
            assert!(!generated
                .meaning
                .nodes
                .iter()
                .any(|n| n.concept_id.starts_with(&format!("C_RECAP_ROLE_{role:?}@"))));
            assert_eq!(summary.event, event);
            for invalid in [
                vec![ContentSlotIR::Agent],
                vec![ContentSlotIR::Source],
                vec![role, role],
            ] {
                let mut changed = summary.clone();
                changed.omitted_roles = invalid;
                assert!(!changed.validate());
            }
        }
    }

    #[test]
    fn recap_composes_source_roles_and_retains_negation() {
        for (language, source, expected) in [
            (
                LanguageCodeIR::English,
                "Nico read the note at the harbor.",
                "Nico read the note at the harbor",
            ),
            (
                LanguageCodeIR::English,
                "Mina did not read a book in the library.",
                "Mina did not read a book in the library",
            ),
            (
                LanguageCodeIR::Korean,
                "규리는 강당에서 일기를 읽었어.",
                "규리가 강당에서 일기를 읽었어",
            ),
            (
                LanguageCodeIR::Korean,
                "민수는 책을 읽지 않았어.",
                "민수가 책을 읽지 않았어",
            ),
            (
                LanguageCodeIR::Korean,
                "민수는 책을 안 읽었어.",
                "민수가 책을 안 읽었어",
            ),
        ] {
            let content = PropositionContentIR::compile(source);
            let summary = EventSummaryIR {
                omitted_roles: vec![],
                belief_id: "B1".into(),
                source_actor: "DIALOGUE_USER".into(),
                source_proposition: source.into(),
                context_sources: vec![],
                event: content.events[0].clone(),
            };
            assert!(summary.validate(), "{source}");
            let g = generate_event_summary(language, &summary).expect(source);
            assert!(g.validate());
            assert!(
                g.morphology.realized_text.contains(expected),
                "{}",
                g.morphology.realized_text
            );
            let mut changed = summary.clone();
            changed.event.negated = !changed.event.negated;
            assert!(!changed.validate());
            changed = summary.clone();
            changed
                .event
                .roles
                .insert(ContentSlotIR::Agent, "INVENTED".into());
            assert!(!changed.validate());
            let other = if language == LanguageCodeIR::Korean {
                LanguageCodeIR::English
            } else {
                LanguageCodeIR::Korean
            };
            assert!(generate_event_summary(other, &summary).is_err());
        }
    }

    #[test]
    fn source_predicate_register_preserves_tense_negation_and_roles() {
        for unknown_or_modal in ["읽을까", "피곤하겠어", "읽으면", "가상의어근"] {
            assert!(
                source_predicate_stem(unknown_or_modal).is_none(),
                "{unknown_or_modal}"
            );
        }
        for (source, expected) in [
            ("규리는 피곤해.", "피곤합니다"),
            ("규리는 행복했어.", "행복했습니다"),
            ("규리는 행복하지 않았어.", "행복하지 않았습니다"),
            ("하린은 일기를 읽었어.", "읽었습니다"),
            ("하린은 편지를 안 읽었어.", "안 읽었습니다"),
            ("하린은 책을 읽지 않았어.", "읽지 않았습니다"),
        ] {
            let content = PropositionContentIR::compile(source);
            let summary = EventSummaryIR {
                omitted_roles: vec![],
                belief_id: "B-REGISTER".into(),
                source_actor: "DIALOGUE_USER".into(),
                source_proposition: source.into(),
                context_sources: vec![],
                event: content.events[0].clone(),
            };
            let neutral = generate_event_summary(LanguageCodeIR::Korean, &summary).unwrap();
            MORPHOLOGY_PASSES.with(|c| c.set(0));
            let formal = generate_event_summary(
                GenerationSettings::with_policy(
                    LanguageCodeIR::Korean,
                    &crate::affective_field::AffectiveRealizationPolicyIR {
                        formal: true,
                        ..Default::default()
                    },
                ),
                &summary,
            )
            .unwrap();
            assert_eq!(MORPHOLOGY_PASSES.with(|c| c.get()), 1);
            assert_eq!(neutral.meaning, formal.meaning);
            assert_eq!(neutral.verification, formal.verification);
            assert!(formal.validate());
            assert!(
                formal.morphology.realized_text.contains(expected),
                "{}",
                formal.morphology.realized_text
            );
            assert_ne!(
                neutral.morphology.realized_text,
                formal.morphology.realized_text
            );
        }
    }
}
