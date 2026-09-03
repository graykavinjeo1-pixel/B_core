//! Bounded compositional semantics for utterance scope and intent competition.
//!
//! This module does not attempt to manufacture meaning from a sentence-sized
//! lookup key. It finds predicate mentions, records whether they occur under
//! quotation, negation, a question, or a hypothetical construction, and keeps
//! losing readings in an inspectable lattice. Only a structurally viable outer
//! speech act may become a planning goal.

use std::collections::BTreeSet;

use dockable_semantic_core::PlanIntentIR;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::attribution::{AttributionAnalyzer, AttributionGraphIR};
use crate::clause_graph::{
    ClauseFunctionIR, ClauseGraphIR, ClauseRelationKindIR, ClauseStructureAnalyzer,
};
use crate::grammatical_scope::{GrammaticalScopeAnalyzer, GrammaticalScopeGraphIR};
use crate::language_knowledge::LanguageCodeIR;
use crate::modality::{ModalScopeGraphIR, ModalSemanticAnalyzer};
use crate::semantic_roles::{QuantifierKindIR, SemanticRoleAnalyzer, SemanticRoleGraphIR};

pub const COMPOSITIONAL_ANALYSIS_SCHEMA: &str = "B_CORE_COMPOSITIONAL_ANALYSIS_IR_6";
pub const PREDICATE_LEXEME_SCHEMA: &str = "B_CORE_PREDICATE_LEXEME_IR_1";
pub const PREDICATE_LEXICON_SNAPSHOT_SCHEMA: &str = "B_CORE_PREDICATE_LEXICON_SNAPSHOT_IR_1";
const MAX_PERSISTED_PREDICATES: usize = 4096;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum FrameMoodIR {
    Declarative,
    Imperative,
    Interrogative,
    Conditional,
    Counterfactual,
    Reported,
    RelativeClause,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum FrameModalityIR {
    Asserted,
    Requested,
    Prohibited,
    Possible,
    Necessary,
    Hypothetical,
    Counterfactual,
    Reported,
    Descriptive,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum FramePolarityIR {
    Positive,
    Negative,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ScopeKindIR {
    Negation,
    Quotation,
    ReportedSpeech,
    Hypothetical,
    Counterfactual,
    AlternativeExclusion,
    FocusOnly,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScopeConstraintIR {
    pub scope_id: String,
    pub kind: ScopeKindIR,
    pub governor_frame_id: Option<String>,
    pub surface_text: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PredicateFrameIR {
    pub frame_id: String,
    pub clause_id: String,
    pub predicate_surface: String,
    pub canonical_predicate: String,
    pub intent_hint: PlanIntentIR,
    pub theme: String,
    pub mood: FrameMoodIR,
    pub modality: FrameModalityIR,
    pub polarity: FramePolarityIR,
    pub embedded_under_quote: bool,
    pub external_execution_authorized: bool,
    pub source_start_byte: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CandidateDispositionIR {
    Viable,
    BlockedByNegation,
    NonAuthoritativeMention,
    HypotheticalOnly,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InterpretationCandidateIR {
    pub candidate_id: String,
    pub source_frame_id: String,
    pub intent: PlanIntentIR,
    pub subject: String,
    pub desired_outcome: String,
    pub disposition: CandidateDispositionIR,
    pub score_millis: u16,
    pub external_execution_authorized: bool,
    pub evidence: Vec<String>,
    pub blockers: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum GoalGraphRelationKindIR {
    Sequence,
    Coordination,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompositionalGoalNodeIR {
    pub node_id: String,
    pub candidate_id: String,
    pub intent: PlanIntentIR,
    pub subject: String,
    pub desired_outcome: String,
    pub external_execution_authorized: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompositionalGoalEdgeIR {
    pub source_node_id: String,
    pub target_node_id: String,
    pub relation: GoalGraphRelationKindIR,
    pub evidence_surface: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompositionalGoalGraphIR {
    pub nodes: Vec<CompositionalGoalNodeIR>,
    pub edges: Vec<CompositionalGoalEdgeIR>,
    pub conditions: Vec<String>,
    pub prohibitions: Vec<String>,
    pub confidence_millis: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompositionalAnalysisIR {
    pub schema: String,
    pub frames: Vec<PredicateFrameIR>,
    #[serde(default)]
    pub clause_graph: ClauseGraphIR,
    #[serde(default)]
    pub attribution_graph: AttributionGraphIR,
    #[serde(default)]
    pub semantic_role_graph: SemanticRoleGraphIR,
    #[serde(default)]
    pub grammatical_scope_graph: GrammaticalScopeGraphIR,
    #[serde(default)]
    pub modal_scope_graph: ModalScopeGraphIR,
    pub scopes: Vec<ScopeConstraintIR>,
    pub candidates: Vec<InterpretationCandidateIR>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selected_candidate_id: Option<String>,
    #[serde(default)]
    pub selected_candidate_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub goal_graph: Option<CompositionalGoalGraphIR>,
    pub clarification_required: bool,
    pub unresolved_competitions: Vec<String>,
    pub structural_coverage_millis: u16,
}

impl CompositionalAnalysisIR {
    pub fn selected_candidate(&self) -> Option<&InterpretationCandidateIR> {
        let selected = self.selected_candidate_id.as_deref()?;
        self.candidates
            .iter()
            .find(|candidate| candidate.candidate_id == selected)
    }

    pub fn selected_candidates(&self) -> Vec<&InterpretationCandidateIR> {
        self.selected_candidate_ids
            .iter()
            .filter_map(|selected| {
                self.candidates
                    .iter()
                    .find(|candidate| &candidate.candidate_id == selected)
            })
            .collect()
    }

    pub fn blocked_execution_count(&self) -> usize {
        self.candidates
            .iter()
            .filter(|candidate| candidate.disposition != CandidateDispositionIR::Viable)
            .count()
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct CompositionalSemanticAnalyzer;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PredicateLexemeIR {
    pub schema: String,
    pub predicate_id: String,
    pub language: LanguageCodeIR,
    pub surface_forms: Vec<String>,
    pub canonical_predicate: String,
    pub intent_hint: PlanIntentIR,
    pub definition: String,
    pub confidence_millis: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PredicateLexemeError {
    InvalidSchema,
    InvalidIdentity,
    InvalidSurfaceForms,
    InvalidSemantics,
    IdentityConflict,
}

impl PredicateLexemeIR {
    pub fn validate(&self) -> Result<(), PredicateLexemeError> {
        if self.schema != PREDICATE_LEXEME_SCHEMA {
            return Err(PredicateLexemeError::InvalidSchema);
        }
        if self.predicate_id.trim().is_empty()
            || self.predicate_id.len() > 128
            || !self
                .predicate_id
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
        {
            return Err(PredicateLexemeError::InvalidIdentity);
        }
        if self.surface_forms.is_empty()
            || self.surface_forms.len() > 32
            || self
                .surface_forms
                .iter()
                .any(|form| form.trim().is_empty() || form.len() > 128)
        {
            return Err(PredicateLexemeError::InvalidSurfaceForms);
        }
        if self.canonical_predicate.trim().is_empty()
            || self.canonical_predicate.len() > 128
            || self.definition.trim().is_empty()
            || self.definition.len() > 512
            || !(1..=1000).contains(&self.confidence_millis)
        {
            return Err(PredicateLexemeError::InvalidSemantics);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PredicateLexiconSnapshotIR {
    pub schema: String,
    pub entries: Vec<PredicateLexemeIR>,
    pub snapshot_sha256: String,
}

impl PredicateLexiconSnapshotIR {
    pub fn build(mut entries: Vec<PredicateLexemeIR>) -> Result<Self, PredicateLexemeError> {
        if entries.len() > MAX_PERSISTED_PREDICATES {
            return Err(PredicateLexemeError::InvalidSemantics);
        }
        for entry in &entries {
            entry.validate()?;
        }
        entries.sort_by(|left, right| left.predicate_id.cmp(&right.predicate_id));
        if entries
            .windows(2)
            .any(|pair| pair[0].predicate_id == pair[1].predicate_id)
        {
            return Err(PredicateLexemeError::IdentityConflict);
        }
        let mut snapshot = Self {
            schema: PREDICATE_LEXICON_SNAPSHOT_SCHEMA.to_string(),
            entries,
            snapshot_sha256: String::new(),
        };
        snapshot.snapshot_sha256 = predicate_snapshot_hash(&snapshot)?;
        Ok(snapshot)
    }

    pub fn validate(&self) -> Result<(), PredicateLexemeError> {
        if self.schema != PREDICATE_LEXICON_SNAPSHOT_SCHEMA
            || self.entries.len() > MAX_PERSISTED_PREDICATES
            || self.snapshot_sha256.len() != 64
        {
            return Err(PredicateLexemeError::InvalidSchema);
        }
        for entry in &self.entries {
            entry.validate()?;
        }
        if self
            .entries
            .windows(2)
            .any(|pair| pair[0].predicate_id >= pair[1].predicate_id)
        {
            return Err(PredicateLexemeError::IdentityConflict);
        }
        if predicate_snapshot_hash(self)? != self.snapshot_sha256 {
            return Err(PredicateLexemeError::InvalidSemantics);
        }
        Ok(())
    }
}

fn predicate_snapshot_hash(
    snapshot: &PredicateLexiconSnapshotIR,
) -> Result<String, PredicateLexemeError> {
    let bytes = serde_json::to_vec(&(snapshot.schema.as_str(), &snapshot.entries))
        .map_err(|_| PredicateLexemeError::InvalidSemantics)?;
    Ok(format!("{:x}", Sha256::digest(bytes)))
}

#[derive(Debug, Clone, Copy)]
struct ActionFamily {
    canonical: &'static str,
    intent: PlanIntentIR,
    forms: &'static [&'static str],
}

/// Event-denoting nouns are not interchangeable with verbs.  They become an
/// action frame only when a surrounding construction contributes request or
/// deontic force (for example, "draft recovery steps" or "the explanation
/// should cover ...").  Keeping this inventory separate prevents a bare noun
/// such as "the assessment" from acquiring execution authority.
const ACTION_NOMINAL_FAMILIES: &[ActionFamily] = &[
    ActionFamily {
        canonical: "EXPLAIN",
        intent: PlanIntentIR::Explain,
        forms: &["explanation", "walkthrough", "briefing"],
    },
    ActionFamily {
        canonical: "INVESTIGATE",
        intent: PlanIntentIR::Investigate,
        forms: &[
            "assessment",
            "investigation",
            "inspection",
            "analysis",
            "examination",
            "evaluation",
            "diagnosis",
            "diagnostic pass",
            "observation",
            "monitoring",
            "triage",
        ],
    },
    ActionFamily {
        canonical: "REPAIR",
        intent: PlanIntentIR::Repair,
        forms: &[
            "recovery",
            "repair",
            "restoration",
            "remediation",
            "correction",
        ],
    },
    ActionFamily {
        canonical: "PLAN",
        intent: PlanIntentIR::Plan,
        forms: &[
            "plan",
            "outline",
            "procedure",
            "steps",
            "sequence",
            "priority",
        ],
    },
];

const ACTION_FAMILIES: &[ActionFamily] = &[
    ActionFamily {
        canonical: "EXPLAIN",
        intent: PlanIntentIR::Explain,
        forms: &[
            "설명",
            "알려",
            "해설",
            "explain",
            "explained",
            "describe",
            "clarify",
        ],
    },
    ActionFamily {
        canonical: "INVESTIGATE",
        intent: PlanIntentIR::Investigate,
        forms: &[
            "확인",
            "검사",
            "조사",
            "분석",
            "요약",
            "검토",
            "진단",
            "비교",
            "검증",
            "점검",
            "찾아",
            "살펴봐",
            "알아봐",
            "봐",
            "inspect",
            "investigate",
            "take a look",
            "have a look",
            "look at",
            "find out",
            "analyze",
            "check",
            "summarize",
            "summarized",
            "review",
            "reviewed",
            "recheck",
            "rechecked",
            "examine",
            "examined",
            "diagnose",
            "diagnosed",
            "compare",
            "compared",
            "verify",
            "verified",
            "validate",
            "validated",
            "assess",
            "assessed",
            "evaluate",
            "evaluated",
            "평가",
            "평가하",
            "observe",
            "monitor",
            "trace",
            "triage",
            "narrow down",
            "focus on",
            "파악",
            "관찰",
            "추적",
            "좁혀",
            "좁히",
            "집중",
        ],
    },
    ActionFamily {
        canonical: "PLAN",
        intent: PlanIntentIR::Plan,
        forms: &[
            "plan",
            "outline",
            "map out",
            "design",
            "prepare",
            "arrange",
            "organize",
            "prioritize",
            "계획",
            "설계",
            "준비",
            "정리",
            "배치",
            "우선순위",
            "순서를 잡",
            "절차를 짜",
        ],
    },
    ActionFamily {
        canonical: "REPAIR",
        intent: PlanIntentIR::Repair,
        forms: &[
            "고치", "고쳐", "수정", "수리", "복구", "fix", "repair", "repaired", "restore",
            "correct",
        ],
    },
    ActionFamily {
        canonical: "CREATE",
        intent: PlanIntentIR::Create,
        forms: &[
            "만들",
            "작성",
            "문서화",
            "생성",
            "create",
            "created",
            "write",
            "document",
            "documented",
            "generate",
        ],
    },
    ActionFamily {
        canonical: "DELETE",
        intent: PlanIntentIR::Execute,
        forms: &[
            "삭제", "지우", "지워", "지웠", "제거", "delete", "deleted", "deleting", "clear",
            "cleared", "remove", "removed", "removing",
        ],
    },
    ActionFamily {
        canonical: "DEPLOY",
        intent: PlanIntentIR::Execute,
        forms: &[
            "배포",
            "게시",
            "deploy",
            "deployed",
            "publish",
            "published",
            "publishing",
        ],
    },
    ActionFamily {
        canonical: "UPDATE",
        intent: PlanIntentIR::Execute,
        forms: &[
            "갱신",
            "업데이트",
            "update",
            "updated",
            "updating",
            "refresh",
            "refreshed",
        ],
    },
    ActionFamily {
        canonical: "EXECUTE",
        intent: PlanIntentIR::Execute,
        forms: &[
            "실행",
            "열어",
            "읽",
            "변환",
            "저장",
            "옮겨",
            "run",
            "execute",
            "executed",
            "perform",
            "open",
            "read",
            "transform",
            "convert",
            "save",
            "move",
            "moved",
            "apply",
            "set aside",
            "shelve",
            "begin",
            "start",
            "적용",
            "보류",
            "접어",
            "시작",
        ],
    },
    ActionFamily {
        canonical: "CONTINUE",
        intent: PlanIntentIR::Execute,
        forms: &[
            "계속",
            "진행",
            "이어가",
            "continue",
            "keep doing",
            "proceed",
            "resume",
        ],
    },
    ActionFamily {
        canonical: "COMMUNICATE",
        intent: PlanIntentIR::Communicate,
        forms: &[
            "기록", "보고", "전달", "보내", "말해", "record", "recorded", "report", "reported",
            "send", "sent", "tell", "notify",
        ],
    },
    ActionFamily {
        canonical: "EXPLAIN",
        intent: PlanIntentIR::Explain,
        forms: &[
            "walk through",
            "talk through",
            "풀어 말",
            "짚어",
            "이야기해",
        ],
    },
    ActionFamily {
        canonical: "LEARN",
        intent: PlanIntentIR::Learn,
        forms: &["학습", "배워", "익혀", "learn", "study", "absorb"],
    },
];

#[derive(Debug, Clone)]
struct ClauseSlice {
    clause_id: String,
    text: String,
    start_byte: usize,
}

#[derive(Debug, Clone)]
struct ActionOccurrence {
    canonical_predicate: String,
    intent: PlanIntentIR,
    form: String,
    local_start: usize,
}

#[derive(Debug, Clone)]
pub(crate) struct PragmaticActionMentionIR {
    pub canonical_predicate: String,
    pub intent: PlanIntentIR,
    pub surface: String,
    pub start_byte: usize,
}

pub(crate) fn pragmatic_action_mentions(text: &str) -> Vec<PragmaticActionMentionIR> {
    let normalized = text.to_lowercase();
    let mut mentions = Vec::new();
    for family in ACTION_FAMILIES {
        for form in family.forms {
            for variant in action_form_variants(form) {
                for (start_byte, _) in normalized.match_indices(&variant) {
                    if pragmatic_form_boundary(&normalized, start_byte, &variant)
                        && !ascii_nominal_context(&normalized, start_byte, &variant)
                    {
                        mentions.push(PragmaticActionMentionIR {
                            canonical_predicate: family.canonical.to_string(),
                            intent: family.intent,
                            surface: variant.clone(),
                            start_byte,
                        });
                    }
                }
            }
        }
    }
    mentions.extend(
        structural_action_occurrences(&normalized)
            .into_iter()
            .map(|occurrence| PragmaticActionMentionIR {
                canonical_predicate: occurrence.canonical_predicate,
                intent: occurrence.intent,
                surface: occurrence.form,
                start_byte: occurrence.local_start,
            }),
    );
    mentions.sort_by(|left, right| {
        left.start_byte
            .cmp(&right.start_byte)
            .then_with(|| right.surface.len().cmp(&left.surface.len()))
            .then_with(|| left.canonical_predicate.cmp(&right.canonical_predicate))
    });
    let mut seen = BTreeSet::new();
    mentions
        .retain(|mention| seen.insert((mention.start_byte, mention.canonical_predicate.clone())));
    mentions
}

impl CompositionalSemanticAnalyzer {
    pub fn analyze(&self, text: &str) -> CompositionalAnalysisIR {
        self.analyze_with_predicates(text, &[])
    }

    pub fn analyze_with_predicates(
        &self,
        text: &str,
        learned_predicates: &[PredicateLexemeIR],
    ) -> CompositionalAnalysisIR {
        let normalized = text.to_lowercase();
        let quote_ranges = quote_ranges(&normalized);
        let clauses = clause_slices(&normalized, &quote_ranges);
        let global_question = is_question(&normalized);
        let mut frames = Vec::new();
        let mut scopes = Vec::new();

        for clause in &clauses {
            let occurrences = action_occurrences(&clause.text, learned_predicates);
            for occurrence in occurrences {
                let global_start = clause.start_byte + occurrence.local_start;
                let quoted = byte_inside_ranges(global_start, &quote_ranges);
                let negated = action_is_negated(&clause.text, &occurrence);
                let reported = quoted || action_is_reported(&clause.text, &occurrence);
                let relative_modifier = action_is_relative_modifier(&clause.text, &occurrence);
                let counterfactual = is_counterfactual(&clause.text);
                let conditional = is_conditional(&clause.text, &occurrence);
                let interrogative = global_question || is_question(&clause.text);
                let mood = if reported {
                    FrameMoodIR::Reported
                } else if relative_modifier {
                    FrameMoodIR::RelativeClause
                } else if counterfactual {
                    FrameMoodIR::Counterfactual
                } else if interrogative {
                    FrameMoodIR::Interrogative
                } else if conditional {
                    FrameMoodIR::Conditional
                } else if is_directive(&clause.text, &occurrence) {
                    FrameMoodIR::Imperative
                } else {
                    FrameMoodIR::Declarative
                };
                let modality = if negated {
                    FrameModalityIR::Prohibited
                } else {
                    match mood {
                        FrameMoodIR::Reported => FrameModalityIR::Reported,
                        FrameMoodIR::RelativeClause => FrameModalityIR::Descriptive,
                        FrameMoodIR::Counterfactual => FrameModalityIR::Counterfactual,
                        FrameMoodIR::Conditional => FrameModalityIR::Hypothetical,
                        FrameMoodIR::Interrogative => FrameModalityIR::Possible,
                        FrameMoodIR::Imperative => FrameModalityIR::Requested,
                        FrameMoodIR::Declarative => detect_asserted_modality(&clause.text),
                    }
                };
                let frame_id = format!("FRAME-{:02}", frames.len() + 1);
                let theme = extract_theme(&clause.text, &occurrence);
                let authorized =
                    mood == FrameMoodIR::Imperative && !negated && !reported && !relative_modifier;
                if negated {
                    scopes.push(scope(
                        scopes.len(),
                        ScopeKindIR::Negation,
                        Some(&frame_id),
                        &clause.text,
                    ));
                }
                if quoted {
                    scopes.push(scope(
                        scopes.len(),
                        ScopeKindIR::Quotation,
                        Some(&frame_id),
                        &occurrence.form,
                    ));
                } else if reported {
                    scopes.push(scope(
                        scopes.len(),
                        ScopeKindIR::ReportedSpeech,
                        Some(&frame_id),
                        &clause.text,
                    ));
                }
                if counterfactual {
                    scopes.push(scope(
                        scopes.len(),
                        ScopeKindIR::Counterfactual,
                        Some(&frame_id),
                        &clause.text,
                    ));
                } else if conditional {
                    scopes.push(scope(
                        scopes.len(),
                        ScopeKindIR::Hypothetical,
                        Some(&frame_id),
                        &clause.text,
                    ));
                }
                frames.push(PredicateFrameIR {
                    frame_id,
                    clause_id: clause.clause_id.clone(),
                    predicate_surface: occurrence.form.clone(),
                    canonical_predicate: occurrence.canonical_predicate,
                    intent_hint: occurrence.intent,
                    theme,
                    mood,
                    modality,
                    polarity: if negated {
                        FramePolarityIR::Negative
                    } else {
                        FramePolarityIR::Positive
                    },
                    embedded_under_quote: quoted,
                    external_execution_authorized: authorized,
                    source_start_byte: global_start,
                });
            }
            scopes.extend(alternative_scopes(&clause.text, scopes.len()));
        }

        scopes.sort_by(|left, right| left.scope_id.cmp(&right.scope_id));
        scopes.dedup_by(|left, right| {
            left.kind == right.kind
                && left.governor_frame_id == right.governor_frame_id
                && left.surface_text == right.surface_text
        });
        let clause_graph = ClauseStructureAnalyzer.analyze(&normalized, &frames);
        revise_frames_from_clause_graph(&normalized, &mut frames, &clause_graph);
        let attribution_graph = AttributionAnalyzer.analyze(&normalized, &frames);
        let mut semantic_role_graph = SemanticRoleAnalyzer.analyze(&normalized, &frames);
        semantic_role_graph.apply_clause_graph(&clause_graph, &frames);
        apply_negative_quantifier_scope(&mut frames, &semantic_role_graph, &mut scopes);
        scopes.sort_by(|left, right| left.scope_id.cmp(&right.scope_id));
        scopes.dedup_by(|left, right| {
            left.kind == right.kind
                && left.governor_frame_id == right.governor_frame_id
                && left.surface_text == right.surface_text
        });
        let grammatical_scope_graph =
            GrammaticalScopeAnalyzer.analyze(&normalized, &frames, &semantic_role_graph);
        let modal_scope_graph = ModalSemanticAnalyzer.analyze(&normalized);
        let mut candidates = frames
            .iter()
            .enumerate()
            .map(|(index, frame)| {
                candidate_from_frame(
                    index,
                    frame,
                    &normalized,
                    &semantic_role_graph,
                    &attribution_graph,
                    &modal_scope_graph,
                )
            })
            .collect::<Vec<_>>();
        candidates.sort_by(|left, right| {
            right
                .score_millis
                .cmp(&left.score_millis)
                .then_with(|| left.candidate_id.cmp(&right.candidate_id))
        });
        let viable = candidates
            .iter()
            .filter(|candidate| candidate.disposition == CandidateDispositionIR::Viable)
            .collect::<Vec<_>>();
        let goal_graph = build_goal_graph(&normalized, &frames, &candidates, &clause_graph);
        let contrastive_pivot = contrastive_pivot_candidate(&candidates, &clause_graph);
        let mut unresolved_competitions = Vec::new();
        let structural_deontic_request = frames.iter().any(|frame| {
            frame.mood == FrameMoodIR::Imperative
                && frame.modality == FrameModalityIR::Requested
                && is_action_nominal_form(&frame.predicate_surface)
        });
        let modal_goal_ambiguity = !modal_scope_graph.unresolved_ambiguities.is_empty()
            && !viable.is_empty()
            && !modal_scope_graph.is_polite_request()
            && !structural_deontic_request;
        if modal_goal_ambiguity {
            unresolved_competitions.extend(
                modal_scope_graph
                    .unresolved_ambiguities
                    .iter()
                    .map(|item| format!("MODAL:{item}")),
            );
        }
        let candidate_competition = goal_graph.is_none()
            && contrastive_pivot.is_none()
            && viable.get(1).is_some_and(|second| {
                let first = viable[0];
                let close = first.score_millis.saturating_sub(second.score_millis) < 40;
                let conflict = first.intent != second.intent && first.subject != second.subject;
                if close && conflict {
                    unresolved_competitions.push(format!(
                        "{}:{} <-> {}:{}",
                        first.intent_tag(),
                        first.subject,
                        second.intent_tag(),
                        second.subject
                    ));
                }
                close && conflict
            });
        let clarification_required = modal_goal_ambiguity || candidate_competition;
        let selected_candidate_id = if clarification_required {
            None
        } else if let Some(graph) = &goal_graph {
            graph.nodes.first().map(|node| node.candidate_id.clone())
        } else if contrastive_pivot.is_some() {
            contrastive_pivot.clone()
        } else {
            viable
                .first()
                .map(|candidate| candidate.candidate_id.clone())
        };
        let selected_candidate_ids = if clarification_required {
            Vec::new()
        } else if let Some(graph) = &goal_graph {
            graph
                .nodes
                .iter()
                .map(|node| node.candidate_id.clone())
                .collect()
        } else {
            selected_candidate_id.iter().cloned().collect()
        };
        let covered_clauses = clauses
            .iter()
            .filter(|clause| {
                frames
                    .iter()
                    .any(|frame| frame.clause_id == clause.clause_id)
            })
            .count();
        let structural_coverage_millis = if clauses.is_empty() {
            0
        } else {
            u16::try_from(covered_clauses.saturating_mul(1000) / clauses.len()).unwrap_or(1000)
        };
        CompositionalAnalysisIR {
            schema: COMPOSITIONAL_ANALYSIS_SCHEMA.to_string(),
            frames,
            clause_graph,
            attribution_graph,
            semantic_role_graph,
            grammatical_scope_graph,
            modal_scope_graph,
            scopes,
            candidates,
            selected_candidate_id,
            selected_candidate_ids,
            goal_graph,
            clarification_required,
            unresolved_competitions,
            structural_coverage_millis,
        }
    }
}

fn apply_negative_quantifier_scope(
    frames: &mut [PredicateFrameIR],
    role_graph: &SemanticRoleGraphIR,
    scopes: &mut Vec<ScopeConstraintIR>,
) {
    let negative_targets = role_graph
        .quantifier_scopes
        .iter()
        .filter(|scope| scope.negated || scope.quantifier == QuantifierKindIR::None)
        .map(|scope| scope.target_node_id.as_str())
        .collect::<BTreeSet<_>>();
    if negative_targets.is_empty() {
        return;
    }
    let negative_frame_ids = role_graph
        .role_edges
        .iter()
        .filter(|role| negative_targets.contains(role.argument_node_id.as_str()))
        .filter_map(|role| {
            role_graph
                .nodes
                .iter()
                .find(|node| node.node_id == role.event_node_id)
                .and_then(|node| node.source_frame_id.as_deref())
        })
        .collect::<BTreeSet<_>>();
    for frame in frames
        .iter_mut()
        .filter(|frame| negative_frame_ids.contains(frame.frame_id.as_str()))
    {
        frame.polarity = FramePolarityIR::Negative;
        frame.modality = FrameModalityIR::Prohibited;
        frame.external_execution_authorized = false;
        if !scopes.iter().any(|scope| {
            scope.kind == ScopeKindIR::Negation
                && scope.governor_frame_id.as_deref() == Some(frame.frame_id.as_str())
        }) {
            scopes.push(scope(
                scopes.len(),
                ScopeKindIR::Negation,
                Some(&frame.frame_id),
                "negative quantifier",
            ));
        }
    }
}

fn contrastive_pivot_candidate(
    candidates: &[InterpretationCandidateIR],
    clause_graph: &ClauseGraphIR,
) -> Option<String> {
    let target_frame_id = clause_graph
        .edges
        .iter()
        .filter(|edge| edge.relation == ClauseRelationKindIR::Contrast)
        .filter_map(|edge| {
            clause_graph
                .nodes
                .iter()
                .find(|node| node.clause_id == edge.target_clause_id)
                .map(|node| node.anchor_frame_id.as_str())
        })
        .next_back()?;
    candidates
        .iter()
        .find(|candidate| {
            candidate.source_frame_id == target_frame_id
                && candidate.disposition == CandidateDispositionIR::Viable
        })
        .map(|candidate| candidate.candidate_id.clone())
}

fn revise_frames_from_clause_graph(
    text: &str,
    frames: &mut [PredicateFrameIR],
    clause_graph: &ClauseGraphIR,
) {
    for frame in frames.iter_mut() {
        let Some(node) = clause_graph.node_for_frame(&frame.frame_id) else {
            continue;
        };
        let Some(local_start) = frame.source_start_byte.checked_sub(node.source_start_byte) else {
            continue;
        };
        if local_start.saturating_add(frame.predicate_surface.len()) > node.source_text.len()
            || !node.source_text.is_char_boundary(local_start)
        {
            continue;
        }
        let occurrence = ActionOccurrence {
            canonical_predicate: frame.canonical_predicate.clone(),
            intent: frame.intent_hint,
            form: frame.predicate_surface.clone(),
            local_start,
        };
        let revised_theme = extract_theme(&node.source_text, &occurrence);
        frame.theme = if is_structural_argument_gap(&revised_theme) {
            String::new()
        } else {
            revised_theme
        };
        if frame.embedded_under_quote
            || matches!(
                frame.mood,
                FrameMoodIR::RelativeClause
                    | FrameMoodIR::Counterfactual
                    | FrameMoodIR::Interrogative
            )
            || (frame.mood == FrameMoodIR::Reported
                && !node.function.permits_independent_directive())
        {
            frame.external_execution_authorized = false;
            continue;
        }
        if !node.function.permits_independent_directive() {
            frame.mood = if node.function == ClauseFunctionIR::Condition {
                FrameMoodIR::Conditional
            } else {
                FrameMoodIR::Declarative
            };
            frame.modality = if node.function == ClauseFunctionIR::Condition {
                FrameModalityIR::Hypothetical
            } else {
                FrameModalityIR::Descriptive
            };
            frame.external_execution_authorized = false;
        } else if is_directive(&node.source_text, &occurrence) {
            frame.mood = FrameMoodIR::Imperative;
            frame.modality = FrameModalityIR::Requested;
            frame.external_execution_authorized = frame.polarity == FramePolarityIR::Positive;
        } else {
            frame.mood = FrameMoodIR::Declarative;
            frame.modality = detect_asserted_modality(&node.source_text);
            frame.external_execution_authorized = false;
        }
    }

    let mut inherited_directives = frames
        .iter()
        .filter(|frame| frame.mood == FrameMoodIR::Imperative)
        .map(|frame| frame.frame_id.clone())
        .collect::<BTreeSet<_>>();
    loop {
        let mut changed = false;
        for edge in &clause_graph.edges {
            if !matches!(
                edge.relation,
                ClauseRelationKindIR::Coordination
                    | ClauseRelationKindIR::Sequence
                    | ClauseRelationKindIR::TemporalBefore
            ) {
                continue;
            }
            let Some(source_node) = clause_graph
                .nodes
                .iter()
                .find(|node| node.clause_id == edge.source_clause_id)
            else {
                continue;
            };
            let Some(target_node) = clause_graph
                .nodes
                .iter()
                .find(|node| node.clause_id == edge.target_clause_id)
            else {
                continue;
            };
            if !source_node.function.permits_independent_directive()
                || !target_node.function.permits_independent_directive()
            {
                continue;
            }
            let source_directive = inherited_directives.contains(&source_node.anchor_frame_id);
            let target_directive = inherited_directives.contains(&target_node.anchor_frame_id);
            if source_directive
                && !has_explicit_clause_agent(
                    &target_node.source_text,
                    frames
                        .iter()
                        .find(|frame| frame.frame_id == target_node.anchor_frame_id),
                    target_node.source_start_byte,
                )
            {
                changed |= inherited_directives.insert(target_node.anchor_frame_id.clone());
            }
            if target_directive
                && !has_explicit_clause_agent(
                    &source_node.source_text,
                    frames
                        .iter()
                        .find(|frame| frame.frame_id == source_node.anchor_frame_id),
                    source_node.source_start_byte,
                )
            {
                changed |= inherited_directives.insert(source_node.anchor_frame_id.clone());
            }
        }
        if !changed {
            break;
        }
    }
    for frame in frames.iter_mut() {
        if inherited_directives.contains(&frame.frame_id)
            && frame.polarity == FramePolarityIR::Positive
            && !frame.embedded_under_quote
            && !matches!(
                frame.mood,
                FrameMoodIR::RelativeClause | FrameMoodIR::Counterfactual
            )
        {
            frame.mood = FrameMoodIR::Imperative;
            frame.modality = FrameModalityIR::Requested;
            frame.external_execution_authorized = true;
        }
    }

    for edge in &clause_graph.edges {
        if !matches!(
            edge.relation,
            ClauseRelationKindIR::Sequence | ClauseRelationKindIR::TemporalBefore
        ) {
            continue;
        }
        let Some(source_node) = clause_graph
            .nodes
            .iter()
            .find(|node| node.clause_id == edge.source_clause_id)
        else {
            continue;
        };
        let Some(target_node) = clause_graph
            .nodes
            .iter()
            .find(|node| node.clause_id == edge.target_clause_id)
        else {
            continue;
        };
        if !source_node.function.permits_independent_directive()
            || !target_node.function.permits_independent_directive()
        {
            continue;
        }
        if let Some(target_frame) = frames
            .iter_mut()
            .find(|frame| frame.frame_id == target_node.anchor_frame_id)
        {
            if edge.relation == ClauseRelationKindIR::TemporalBefore
                || target_frame.theme.is_empty()
            {
                target_frame.theme = "PRIOR_RESULT".to_string();
            }
        }
    }

    debug_assert!(frames.iter().all(|frame| {
        frame.source_start_byte <= text.len() && text.is_char_boundary(frame.source_start_byte)
    }));
}

fn has_explicit_clause_agent(
    source_text: &str,
    frame: Option<&PredicateFrameIR>,
    source_start_byte: usize,
) -> bool {
    let Some(frame) = frame else {
        return true;
    };
    let Some(local_start) = frame.source_start_byte.checked_sub(source_start_byte) else {
        return true;
    };
    let prefix = source_text.get(..local_start).unwrap_or_default().trim();
    if prefix.is_empty() {
        return false;
    }
    if prefix
        .chars()
        .any(|character| ('\u{ac00}'..='\u{d7a3}').contains(&character))
    {
        let final_token = prefix
            .split_whitespace()
            .next_back()
            .unwrap_or_default()
            .trim_matches(|character: char| character.is_ascii_punctuation());
        if ["는지", "은지", "인지", "한지", "할지", "했는지", "되는지"]
            .iter()
            .any(|suffix| final_token.ends_with(suffix))
        {
            return false;
        }
        return prefix.split_whitespace().any(|word| {
            let word = word.trim_matches(|character: char| character.is_ascii_punctuation());
            word.chars().count() > 1 && (word.ends_with('이') || word.ends_with('가'))
        });
    }
    prefix.split_whitespace().any(|word| {
        !matches!(
            word.to_lowercase().as_str(),
            "please" | "just" | "then" | "and" | "first" | "next"
        )
    })
}

fn build_goal_graph(
    text: &str,
    frames: &[PredicateFrameIR],
    candidates: &[InterpretationCandidateIR],
    clause_graph: &ClauseGraphIR,
) -> Option<CompositionalGoalGraphIR> {
    let mut ordered = candidates
        .iter()
        .filter(|candidate| candidate.disposition == CandidateDispositionIR::Viable)
        .filter_map(|candidate| {
            frames
                .iter()
                .find(|frame| frame.frame_id == candidate.source_frame_id)
                .map(|frame| (frame, candidate))
        })
        .collect::<Vec<_>>();
    ordered.sort_by_key(|(frame, _)| frame.source_start_byte);
    if ordered.len() < 2 {
        return None;
    }

    let mut edges = Vec::new();
    for (index, pair) in ordered.windows(2).enumerate() {
        let (source_frame, _) = pair[0];
        let (target_frame, _) = pair[1];
        let source_end = source_frame
            .source_start_byte
            .saturating_add(source_frame.predicate_surface.len());
        if source_end > target_frame.source_start_byte
            || target_frame.source_start_byte > text.len()
        {
            return None;
        }
        let evidence = &text[source_end..target_frame.source_start_byte];
        let clause_edge = clause_graph.edges.iter().find(|edge| {
            let source = clause_graph.node_for_frame(&source_frame.frame_id);
            let target = clause_graph.node_for_frame(&target_frame.frame_id);
            source.is_some_and(|source| {
                target.is_some_and(|target| {
                    (edge.source_clause_id == source.clause_id
                        && edge.target_clause_id == target.clause_id)
                        || (edge.source_clause_id == target.clause_id
                            && edge.target_clause_id == source.clause_id)
                })
            })
        });
        let relation = clause_edge
            .and_then(|edge| match edge.relation {
                ClauseRelationKindIR::Coordination => Some(GoalGraphRelationKindIR::Coordination),
                ClauseRelationKindIR::Sequence | ClauseRelationKindIR::TemporalBefore => {
                    Some(GoalGraphRelationKindIR::Sequence)
                }
                ClauseRelationKindIR::Condition
                | ClauseRelationKindIR::Cause
                | ClauseRelationKindIR::Purpose
                | ClauseRelationKindIR::Contrast => None,
            })
            .or_else(|| coordination_relation(evidence))?;
        edges.push(CompositionalGoalEdgeIR {
            source_node_id: format!("GOAL-NODE-{:02}", index + 1),
            target_node_id: format!("GOAL-NODE-{:02}", index + 2),
            relation,
            evidence_surface: clause_edge.map_or_else(
                || evidence.trim().to_string(),
                |edge| edge.marker_surface.clone(),
            ),
        });
    }

    let nodes = ordered
        .iter()
        .enumerate()
        .map(|(index, (_, candidate))| CompositionalGoalNodeIR {
            node_id: format!("GOAL-NODE-{:02}", index + 1),
            candidate_id: candidate.candidate_id.clone(),
            intent: candidate.intent,
            subject: candidate.subject.clone(),
            desired_outcome: candidate.desired_outcome.clone(),
            external_execution_authorized: candidate.external_execution_authorized,
        })
        .collect::<Vec<_>>();
    let mut conditions = clause_graph
        .nodes
        .iter()
        .filter(|node| node.function == ClauseFunctionIR::Condition)
        .map(|node| node.source_text.clone())
        .chain(extract_graph_conditions(text))
        .collect::<Vec<_>>();
    conditions.sort();
    conditions.dedup();
    let mut prohibitions = candidates
        .iter()
        .filter(|candidate| candidate.disposition == CandidateDispositionIR::BlockedByNegation)
        .map(|candidate| format!("{}:{}", candidate.intent_tag(), candidate.subject))
        .collect::<Vec<_>>();
    prohibitions.sort();
    prohibitions.dedup();
    let confidence_millis = ordered
        .iter()
        .map(|(_, candidate)| candidate.score_millis)
        .min()
        .unwrap_or(0)
        .saturating_sub(u16::try_from(edges.len().saturating_sub(1) * 20).unwrap_or(200));
    Some(CompositionalGoalGraphIR {
        nodes,
        edges,
        conditions,
        prohibitions,
        confidence_millis,
    })
}

fn coordination_relation(evidence: &str) -> Option<GoalGraphRelationKindIR> {
    let normalized = evidence.to_lowercase();
    if normalized.contains(';') {
        return None;
    }
    if [
        " then ",
        "then ",
        " 뒤",
        " 후",
        "다음",
        "고 나서",
        "한 뒤",
        "한 후",
    ]
    .iter()
    .any(|marker| normalized.contains(marker))
        || normalized.contains(',')
        || normalized.contains("고 ")
    {
        return Some(GoalGraphRelationKindIR::Sequence);
    }
    if [" and ", "그리고", "및 ", "와 ", "과 ", "하되"]
        .iter()
        .any(|marker| normalized.contains(marker))
    {
        return Some(GoalGraphRelationKindIR::Coordination);
    }
    None
}

fn extract_graph_conditions(text: &str) -> Vec<String> {
    text.split(['.', '?', '!', ';', ',', '\n', '\r'])
        .map(str::trim)
        .filter(|segment| {
            ["면 ", "다면", "경우", "if ", "unless ", "provided that "]
                .iter()
                .any(|marker| segment.contains(marker))
        })
        .map(ToString::to_string)
        .collect()
}

impl InterpretationCandidateIR {
    fn intent_tag(&self) -> &'static str {
        match self.intent {
            PlanIntentIR::Plan => "PLAN",
            PlanIntentIR::Investigate => "INVESTIGATE",
            PlanIntentIR::Repair => "REPAIR",
            PlanIntentIR::Create => "CREATE",
            PlanIntentIR::Learn => "LEARN",
            PlanIntentIR::Explain => "EXPLAIN",
            PlanIntentIR::Communicate => "COMMUNICATE",
            PlanIntentIR::Execute => "EXECUTE",
        }
    }
}

fn candidate_from_frame(
    index: usize,
    frame: &PredicateFrameIR,
    full_text: &str,
    semantic_role_graph: &SemanticRoleGraphIR,
    attribution_graph: &AttributionGraphIR,
    modal_scope_graph: &ModalScopeGraphIR,
) -> InterpretationCandidateIR {
    let attributed = attribution_graph.attributes_frame(&frame.frame_id);
    let polite_request = modal_scope_graph.is_polite_request();
    let disposition = match (frame.polarity, frame.mood, attributed) {
        (FramePolarityIR::Negative, _, _) => CandidateDispositionIR::BlockedByNegation,
        (_, _, true) => CandidateDispositionIR::NonAuthoritativeMention,
        (_, FrameMoodIR::Reported, false) => CandidateDispositionIR::NonAuthoritativeMention,
        (_, FrameMoodIR::RelativeClause, false) => CandidateDispositionIR::NonAuthoritativeMention,
        (_, FrameMoodIR::Interrogative, false) if polite_request => CandidateDispositionIR::Viable,
        (_, FrameMoodIR::Declarative, false) if modal_scope_graph.blocks_goal_projection() => {
            CandidateDispositionIR::NonAuthoritativeMention
        }
        (_, FrameMoodIR::Counterfactual | FrameMoodIR::Conditional, false) => {
            CandidateDispositionIR::HypotheticalOnly
        }
        (_, FrameMoodIR::Declarative, false) if frame.modality != FrameModalityIR::Necessary => {
            CandidateDispositionIR::NonAuthoritativeMention
        }
        _ => CandidateDispositionIR::Viable,
    };
    let intent = if frame.mood == FrameMoodIR::Interrogative && !polite_request {
        PlanIntentIR::Investigate
    } else {
        frame.intent_hint
    };
    let subject = if frame.mood == FrameMoodIR::Interrogative && !polite_request {
        full_text.trim().trim_end_matches('?').to_string()
    } else if (is_action_nominal_form(&frame.predicate_surface)
        || frame.predicate_surface.ends_with(" through"))
        && !frame.theme.is_empty()
    {
        frame
            .theme
            .strip_prefix("of ")
            .unwrap_or(&frame.theme)
            .to_string()
    } else if frame.intent_hint == PlanIntentIR::Investigate
        && frame.theme.to_lowercase().starts_with("whether ")
    {
        frame.theme.clone()
    } else if let Some(argument) = semantic_role_graph.primary_argument_for_frame(&frame.frame_id) {
        if !frame
            .predicate_surface
            .chars()
            .all(|character| character.is_ascii_alphabetic())
            && frame.theme.contains(' ')
            && frame.theme.ends_with(&argument.normalized_label)
        {
            frame.theme.clone()
        } else {
            argument.normalized_label.clone()
        }
    } else if frame.theme.is_empty() {
        frame.canonical_predicate.to_lowercase()
    } else {
        frame.theme.clone()
    };
    let mut evidence = vec![format!("predicate={}", frame.canonical_predicate)];
    evidence.push(format!("mood={:?}", frame.mood));
    let mut blockers = Vec::new();
    match disposition {
        CandidateDispositionIR::Viable => {}
        CandidateDispositionIR::BlockedByNegation => {
            blockers.push("predicate is inside explicit negation/prohibition scope".to_string());
        }
        CandidateDispositionIR::NonAuthoritativeMention => {
            blockers.push(if attributed {
                "predicate is inside an attributed proposition and carries no dialogue execution authority"
                    .to_string()
            } else {
                "predicate is quoted, reported, or merely asserted rather than requested"
                    .to_string()
            });
        }
        CandidateDispositionIR::HypotheticalOnly => {
            blockers.push(
                "predicate is hypothetical/counterfactual and has no execution authority"
                    .to_string(),
            );
        }
    }
    let focus_boost = u16::from(has_focus_marker(full_text)) * 35;
    let base = match frame.mood {
        FrameMoodIR::Imperative => 900,
        FrameMoodIR::Interrogative => 820,
        FrameMoodIR::Declarative => 610,
        FrameMoodIR::Conditional => 360,
        FrameMoodIR::Counterfactual => 220,
        FrameMoodIR::Reported => 180,
        FrameMoodIR::RelativeClause => 160,
    };
    let score_millis = if disposition == CandidateDispositionIR::Viable {
        (base + focus_boost).min(980)
    } else {
        base
    };
    InterpretationCandidateIR {
        candidate_id: format!("CANDIDATE-{:02}", index + 1),
        source_frame_id: frame.frame_id.clone(),
        intent,
        subject: subject.clone(),
        desired_outcome: if intent == PlanIntentIR::Investigate {
            format!("determine whether the proposition holds: {subject}")
        } else {
            format!("apply {} to {subject}", frame.canonical_predicate)
        },
        disposition,
        score_millis,
        external_execution_authorized: (frame.external_execution_authorized || polite_request)
            && disposition == CandidateDispositionIR::Viable
            && !attributed,
        evidence,
        blockers,
    }
}

fn scope(
    index: usize,
    kind: ScopeKindIR,
    governor: Option<&str>,
    surface: &str,
) -> ScopeConstraintIR {
    ScopeConstraintIR {
        scope_id: format!("SCOPE-{:02}", index + 1),
        kind,
        governor_frame_id: governor.map(ToString::to_string),
        surface_text: surface.trim().to_string(),
    }
}

fn clause_slices(text: &str, quotes: &[(usize, usize)]) -> Vec<ClauseSlice> {
    let mut clauses = Vec::new();
    let mut start = 0;
    for (position, character) in text.char_indices() {
        if matches!(character, '.' | '?' | '!' | ';' | '\n' | '\r')
            && !byte_inside_ranges(position, quotes)
        {
            push_clause(&mut clauses, text, start, position);
            start = position + character.len_utf8();
        }
    }
    push_clause(&mut clauses, text, start, text.len());
    clauses
}

fn push_clause(clauses: &mut Vec<ClauseSlice>, text: &str, start: usize, end: usize) {
    let slice = &text[start..end];
    let leading = slice.len().saturating_sub(slice.trim_start().len());
    let trimmed = slice.trim();
    if !trimmed.is_empty() {
        clauses.push(ClauseSlice {
            clause_id: format!("STRUCT-CLAUSE-{:02}", clauses.len() + 1),
            text: trimmed.to_string(),
            start_byte: start + leading,
        });
    }
}

fn quote_ranges(text: &str) -> Vec<(usize, usize)> {
    let mut ranges = Vec::new();
    let mut stack: Vec<(char, usize)> = Vec::new();
    for (position, character) in text.char_indices() {
        match character {
            '“' | '‘' => stack.push((character, position)),
            '”' => close_quote(&mut stack, &mut ranges, position, '“'),
            '’' => close_quote(&mut stack, &mut ranges, position, '‘'),
            '"' | '\'' => {
                if stack.last().is_some_and(|(open, _)| *open == character) {
                    let (_, start) = stack.pop().expect("checked quote stack");
                    ranges.push((start, position + character.len_utf8()));
                } else {
                    stack.push((character, position));
                }
            }
            _ => {}
        }
    }
    ranges.sort_unstable();
    ranges
}

fn close_quote(
    stack: &mut Vec<(char, usize)>,
    ranges: &mut Vec<(usize, usize)>,
    end: usize,
    expected: char,
) {
    if let Some(position) = stack.iter().rposition(|(open, _)| *open == expected) {
        let (_, start) = stack.remove(position);
        ranges.push((start, end + 3));
    }
}

fn byte_inside_ranges(position: usize, ranges: &[(usize, usize)]) -> bool {
    ranges
        .iter()
        .any(|(start, end)| position >= *start && position < *end)
}

fn action_occurrences(
    text: &str,
    learned_predicates: &[PredicateLexemeIR],
) -> Vec<ActionOccurrence> {
    let mut occurrences = Vec::new();
    for family in ACTION_FAMILIES {
        for form in family.forms {
            for variant in action_form_variants(form) {
                for (position, _) in text.match_indices(&variant) {
                    if valid_form_boundary(text, position, &variant)
                        && !ascii_nominal_context(text, position, &variant)
                        && !korean_nominal_modifier_context(text, position, &variant)
                    {
                        occurrences.push(ActionOccurrence {
                            canonical_predicate: family.canonical.to_string(),
                            intent: family.intent,
                            form: variant.clone(),
                            local_start: position,
                        });
                    }
                }
            }
        }
    }
    occurrences.extend(structural_action_occurrences(text));
    for predicate in learned_predicates {
        if predicate.validate().is_err() {
            continue;
        }
        for form in &predicate.surface_forms {
            let normalized_form = form.to_lowercase();
            for (position, _) in text.match_indices(&normalized_form) {
                if valid_form_boundary(text, position, &normalized_form) {
                    occurrences.push(ActionOccurrence {
                        canonical_predicate: predicate.canonical_predicate.clone(),
                        intent: predicate.intent_hint,
                        form: normalized_form.clone(),
                        local_start: position,
                    });
                }
            }
        }
    }
    normalize_action_construction_occurrences(text, &mut occurrences);
    occurrences.sort_by(|left, right| {
        left.local_start
            .cmp(&right.local_start)
            .then_with(|| right.form.len().cmp(&left.form.len()))
    });
    let mut non_overlapping = Vec::<ActionOccurrence>::new();
    for occurrence in occurrences {
        let end = occurrence.local_start.saturating_add(occurrence.form.len());
        let overlaps_same_predicate = non_overlapping.iter().any(|retained| {
            let retained_end = retained.local_start.saturating_add(retained.form.len());
            retained.canonical_predicate == occurrence.canonical_predicate
                && occurrence.local_start < retained_end
                && retained.local_start < end
        });
        if !overlaps_same_predicate {
            non_overlapping.push(occurrence);
        }
    }
    let mut occurrences = non_overlapping;
    let mut seen = BTreeSet::new();
    occurrences.retain(|occurrence| {
        seen.insert((
            occurrence.local_start,
            occurrence.canonical_predicate.clone(),
        ))
    });
    occurrences
}

fn structural_action_occurrences(text: &str) -> Vec<ActionOccurrence> {
    let mut occurrences = Vec::new();
    for family in ACTION_NOMINAL_FAMILIES {
        for form in family.forms {
            for (position, _) in text.match_indices(form) {
                if valid_form_boundary(text, position, form)
                    && nominal_action_is_projectable(text, position, form)
                {
                    occurrences.push(ActionOccurrence {
                        canonical_predicate: family.canonical.to_string(),
                        intent: family.intent,
                        form: (*form).to_string(),
                        local_start: position,
                    });
                }
            }
        }
    }
    occurrences.extend(discontinuous_explanation_occurrences(text));
    occurrences
}

fn discontinuous_explanation_occurrences(text: &str) -> Vec<ActionOccurrence> {
    let mut occurrences = Vec::new();
    for verb in ["walk", "talk"] {
        for pronoun in ["me", "us", "you", "them", "him", "her"] {
            let form = format!("{verb} {pronoun} through");
            for (position, _) in text.match_indices(&form) {
                if pragmatic_form_boundary(text, position, &form) {
                    occurrences.push(ActionOccurrence {
                        canonical_predicate: "EXPLAIN".to_string(),
                        intent: PlanIntentIR::Explain,
                        form: form.clone(),
                        local_start: position,
                    });
                }
            }
        }
    }
    occurrences
}

fn nominal_action_is_projectable(text: &str, start: usize, form: &str) -> bool {
    if !form
        .chars()
        .all(|character| character.is_ascii_alphabetic())
    {
        return false;
    }
    let clause_start = text[..start]
        .rfind(['.', '?', '!', ';', '\n', '\r'])
        .map_or(0, |position| position + 1);
    let clause_end = text[start..]
        .find(['.', '?', '!', ';', '\n', '\r'])
        .map_or(text.len(), |position| start + position);
    let raw_clause = &text[clause_start..clause_end];
    let leading = raw_clause
        .len()
        .saturating_sub(raw_clause.trim_start().len());
    let clause = raw_clause.trim();
    let local_start = start.saturating_sub(clause_start + leading);
    let after = clause
        .get(local_start.saturating_add(form.len())..)
        .unwrap_or_default()
        .trim_start();
    let governed_by_copula = matches!(
        after.split_whitespace().next().unwrap_or_default(),
        "is" | "are" | "was" | "were" | "has" | "had"
    );
    let deontic_nominal = [
        " should ",
        " must ",
        " needs to ",
        " need to ",
        " is to ",
        " are to ",
    ]
    .iter()
    .any(|cue| clause.contains(cue));
    if governed_by_copula && !deontic_nominal {
        return false;
    }

    let directive_surface = strip_conversational_directive_lead_in(clause);
    let directive_head = directive_surface
        .strip_prefix("please ")
        .unwrap_or(directive_surface);
    let imperative_constructor = [
        "start ", "begin ", "draft ", "give ", "provide ", "prepare ", "make ", "create ", "set ",
        "put ", "develop ", "help ",
    ]
    .iter()
    .any(|head| directive_head.starts_with(head));
    let explicit_request_constructor = [
        "ask for ",
        "asking for ",
        "asking only for ",
        "want an ",
        "want a ",
        "need an ",
        "need a ",
        "request an ",
        "request a ",
    ]
    .iter()
    .any(|cue| clause.contains(cue));
    let nominal_is_imperative_head = directive_head.starts_with(form);

    imperative_constructor
        || explicit_request_constructor
        || deontic_nominal
        || nominal_is_imperative_head
}

fn normalize_action_construction_occurrences(text: &str, occurrences: &mut Vec<ActionOccurrence>) {
    let specific_nominals = occurrences
        .iter()
        .filter(|occurrence| {
            occurrence.intent != PlanIntentIR::Plan
                && is_action_nominal_form(&occurrence.form)
                && nominal_action_is_projectable(text, occurrence.local_start, &occurrence.form)
        })
        .map(|occurrence| occurrence.local_start)
        .collect::<Vec<_>>();
    let lifted_korean_actions = occurrences
        .iter()
        .filter(|occurrence| {
            occurrence.intent != PlanIntentIR::Plan
                && korean_embedded_action_request(text, occurrence)
        })
        .map(|occurrence| occurrence.local_start)
        .collect::<Vec<_>>();

    occurrences.retain(|occurrence| {
        if is_discourse_revision_operator(text, occurrence) {
            return false;
        }
        if (matches!(occurrence.form.as_str(), "start" | "begin")
            || occurrence.intent == PlanIntentIR::Plan)
            && specific_nominals
                .iter()
                .any(|position| *position > occurrence.local_start)
        {
            return false;
        }
        if occurrence.intent == PlanIntentIR::Plan
            && is_action_nominal_form(&occurrence.form)
            && specific_nominals.iter().any(|position| {
                *position < occurrence.local_start
                    && occurrence.local_start.saturating_sub(*position) <= 32
            })
        {
            return false;
        }
        if matches!(occurrence.form.as_str(), "진행" | "계속" | "proceed")
            && lifted_korean_actions
                .iter()
                .any(|position| *position < occurrence.local_start)
        {
            return false;
        }
        if occurrence.intent == PlanIntentIR::Plan
            && matches!(
                occurrence.form.as_str(),
                "계획" | "설계" | "준비" | "우선순위" | "순서를 잡" | "절차를 짜"
            )
            && occurrences_have_lifted_korean_action(text, occurrence, &lifted_korean_actions)
        {
            return false;
        }
        true
    });
}

/// A speaker can use an action-shaped verb to revise the current utterance
/// rather than request the denoted world action: "let me correct that: ...".
/// Treat the colon-delimited, first-person anaphoric preface as a discourse
/// operator and leave the replacement proposition to ordinary composition.
/// A concrete repair request ("correct the Birch cache") is intentionally not
/// covered by this boundary.
fn is_discourse_revision_operator(text: &str, occurrence: &ActionOccurrence) -> bool {
    if occurrence.canonical_predicate != "REPAIR"
        || !matches!(occurrence.form.as_str(), "correct" | "수정")
    {
        return false;
    }
    let Some(boundary) = text.find([':', '：']) else {
        return false;
    };
    if occurrence.local_start >= boundary {
        return false;
    }
    let preface = text[..boundary].trim();
    let before = preface[..occurrence.local_start.min(preface.len())].trim();
    let after_start = occurrence
        .local_start
        .saturating_add(occurrence.form.len())
        .min(preface.len());
    let after = preface[after_start..].trim_matches(|character: char| {
        character.is_whitespace() || character.is_ascii_punctuation()
    });
    let revision_lead = strip_conversational_directive_lead_in(before);
    let first_person_revision = matches!(
        revision_lead,
        "let me" | "i will" | "i'll" | "i want to" | "i need to"
    );
    let anaphoric_target = matches!(after, "that" | "this" | "myself");
    let korean_revision = occurrence.form == "수정"
        && ["내가", "제가", "말을", "표현을", "방금 말을"]
            .iter()
            .any(|marker| before.contains(marker))
        && (after.is_empty()
            || ["할게", "하겠습니다", "할게요", "해서"]
                .iter()
                .any(|ending| after.starts_with(ending)));
    (first_person_revision && anaphoric_target) || korean_revision
}

fn occurrences_have_lifted_korean_action(
    text: &str,
    carrier: &ActionOccurrence,
    candidate_positions: &[usize],
) -> bool {
    candidate_positions.iter().any(|position| {
        if *position >= carrier.local_start || !text.is_char_boundary(*position) {
            return false;
        }
        let Some(candidate) = text.get(*position..carrier.local_start) else {
            return false;
        };
        [
            "하는 ",
            "할 ",
            "한 ",
            "해 보는 ",
            "어 보는 ",
            "아 보는 ",
            " 보는 ",
        ]
        .iter()
        .any(|link| candidate.ends_with(link))
    })
}

fn is_action_nominal_form(form: &str) -> bool {
    ACTION_NOMINAL_FAMILIES
        .iter()
        .flat_map(|family| family.forms.iter())
        .any(|nominal| *nominal == form)
}

fn action_form_variants(form: &str) -> Vec<String> {
    let mut variants = vec![form.to_string()];
    if form == "assessment"
        || form.len() < 4
        || !form
            .chars()
            .all(|character| character.is_ascii_alphabetic())
    {
        return variants;
    }
    let present = if let Some(stem) = form.strip_suffix('y') {
        format!("{stem}ies")
    } else if form.ends_with('s')
        || form.ends_with('x')
        || form.ends_with("ch")
        || form.ends_with("sh")
    {
        format!("{form}es")
    } else {
        format!("{form}s")
    };
    let past = if form.ends_with('e') {
        format!("{form}d")
    } else {
        format!("{form}ed")
    };
    let progressive = if let Some(stem) = form.strip_suffix('e') {
        if form.ends_with("ee") {
            format!("{form}ing")
        } else {
            format!("{stem}ing")
        }
    } else {
        format!("{form}ing")
    };
    variants.extend([present, past, progressive]);
    variants.sort();
    variants.dedup();
    variants
}

fn pragmatic_form_boundary(text: &str, start: usize, form: &str) -> bool {
    let before = text[..start].chars().next_back();
    let after = text[start + form.len()..].chars().next();
    if form
        .chars()
        .all(|character| character.is_ascii_alphabetic())
    {
        return !before
            .is_some_and(|character| character.is_ascii_alphanumeric() || character == '_')
            && !after
                .is_some_and(|character| character.is_ascii_alphanumeric() || character == '_');
    }
    let auxiliary_try = form.ends_with(['어', '아', '여']) && after == Some('보');
    !before.is_some_and(is_korean_word_character)
        && !after.is_some_and(|character| {
            is_korean_word_character(character)
                && !auxiliary_try
                && ![
                    '가', '이', '은', '는', '을', '를', '의', '도', '만', '하', '해', '했', '한',
                    '할', '되', '시', '고', '지', '면', '어', '아', '여', '줘', '주', '줄', '줬',
                ]
                .contains(&character)
        })
}

fn is_korean_word_character(character: char) -> bool {
    ('\u{ac00}'..='\u{d7a3}').contains(&character) || ('\u{3131}'..='\u{318e}').contains(&character)
}

fn ascii_nominal_context(text: &str, start: usize, form: &str) -> bool {
    if !form
        .chars()
        .all(|character| character.is_ascii_alphabetic())
    {
        return false;
    }
    let prefix = &text[..start];
    let clause_prefix = prefix
        .rsplit(['.', '?', '!', ';', '\n', '\r'])
        .next()
        .unwrap_or_default();
    if form == "assessment"
        && [
            "ask for",
            "asking for",
            "asking only for",
            "want an",
            "need an",
            "request an",
        ]
        .iter()
        .any(|cue| clause_prefix.contains(cue))
    {
        return false;
    }
    if form == "assessment" {
        return true;
    }
    let prior = prefix
        .split(|character: char| !character.is_ascii_alphanumeric() && character != '_')
        .rfind(|token| !token.is_empty())
        .unwrap_or_default();
    let prior_is_adjacent = prefix
        .trim_end()
        .chars()
        .next_back()
        .is_some_and(|character| character.is_ascii_alphanumeric() || character == '_');
    let after = text[start + form.len()..].trim_start();
    let bounded_nominal = matches!(form, "report" | "record" | "document" | "plan")
        && !prefix
            .rsplit(['.', '?', '!', ';', ',', '\n', '\r'])
            .next()
            .unwrap_or_default()
            .trim()
            .is_empty()
        && !matches!(
            prior,
            "please"
                | "and"
                | "then"
                | "but"
                | "i"
                | "we"
                | "you"
                | "me"
                | "us"
                | "they"
                | "he"
                | "she"
                | "to"
                | "user"
                | "system"
        );
    bounded_nominal
        || (prior_is_adjacent
            && matches!(
                prior,
                "the"
                    | "a"
                    | "an"
                    | "this"
                    | "that"
                    | "each"
                    | "every"
                    | "some"
                    | "any"
                    | "no"
                    | "my"
                    | "your"
                    | "our"
                    | "their"
                    | "its"
            ))
        || matches!(
            after.split_whitespace().next().unwrap_or_default(),
            "is" | "are" | "was" | "were" | "has" | "had"
        )
}

fn korean_nominal_modifier_context(text: &str, start: usize, form: &str) -> bool {
    if form
        .chars()
        .all(|character| character.is_ascii_alphabetic())
    {
        return false;
    }
    let tail = &text[start + form.len()..];
    if !tail.starts_with(char::is_whitespace) {
        return false;
    }
    let head = tail
        .split_whitespace()
        .next()
        .map(strip_korean_focus_and_case)
        .unwrap_or_default();
    matches!(
        head,
        "비용"
            | "결과"
            | "상태"
            | "기록"
            | "경로"
            | "절차"
            | "정책"
            | "여부"
            | "가능성"
            | "시간"
            | "방식"
            | "기능"
            | "추적"
            | "이력"
    )
}

fn valid_form_boundary(text: &str, start: usize, form: &str) -> bool {
    if !form
        .chars()
        .all(|character| character.is_ascii_alphabetic())
    {
        if form == "보고"
            && text[..start]
                .chars()
                .next_back()
                .is_some_and(|character| matches!(character, '어' | '아' | '여'))
        {
            return false;
        }
        let tail = &text[start + form.len()..];
        if tail.is_empty()
            || tail.chars().next().is_some_and(|character| {
                character.is_whitespace()
                    || character.is_ascii_punctuation()
                    || matches!(character, '‘' | '’' | '“' | '”' | '「' | '」' | '『' | '』')
            })
        {
            return true;
        }
        if form.ends_with(['어', '아', '여']) && tail.starts_with('보') {
            return true;
        }
        return [
            "하", "해", "했", "한", "할", "되", "시", "고", "지", "면", "어", "아", "여", "줘",
            "주", "줄", "줬", "세요", "자", "며", "면서", "던", "더", "기", "으", "만", "는",
        ]
        .iter()
        .any(|suffix| tail.starts_with(suffix));
    }
    let before = text[..start].chars().next_back();
    let after = text[start + form.len()..].chars().next();
    !before.is_some_and(|character| character.is_ascii_alphanumeric() || character == '_')
        && !after.is_some_and(|character| character.is_ascii_alphanumeric() || character == '_')
}

fn action_is_negated(text: &str, occurrence: &ActionOccurrence) -> bool {
    let form = occurrence.form.as_str();
    let korean_patterns = [
        format!("{form}지 마"),
        format!("{form}지 말"),
        format!("{form}면 안"),
        format!("{form}하지 마"),
        format!("{form}하지 말"),
        format!("안 {form}"),
        format!("못 {form}"),
    ];
    if korean_patterns.iter().any(|pattern| text.contains(pattern)) {
        return true;
    }
    let english_patterns = [
        format!("don't {form}"),
        format!("do not {form}"),
        format!("not {form}"),
        format!("never {form}"),
    ];
    if english_patterns
        .iter()
        .any(|pattern| text.contains(pattern))
    {
        return true;
    }
    let prefix = &text[..occurrence.local_start];
    prefix
        .split_whitespace()
        .next_back()
        .is_some_and(|token| matches!(token, "not" | "never" | "말고"))
}

fn action_is_reported(text: &str, occurrence: &ActionOccurrence) -> bool {
    let before = &text[..occurrence.local_start];
    let after = &text[occurrence.local_start + occurrence.form.len()..];
    let korean_complement = ["라고", "다고", "라는", "다는", "하라고", "해달라고"]
        .iter()
        .any(|marker| after.contains(marker));
    let korean_report = ["말했", "들었", "전했", "요청했", "사실", "문장"]
        .iter()
        .any(|marker| text.contains(marker));
    let latest_report = ["said ", "told ", "asked ", "quoted "]
        .iter()
        .filter_map(|marker| before.rfind(marker).map(|position| position + marker.len()))
        .max();
    let english_report = latest_report.is_some_and(|report_end| {
        let intervening = &before[report_end..];
        ![", but ", ", but now ", ", however ", ", and now ", "; but "]
            .iter()
            .any(|boundary| intervening.contains(boundary))
    });
    (korean_complement && korean_report) || english_report
}

fn action_is_relative_modifier(text: &str, occurrence: &ActionOccurrence) -> bool {
    let form_is_english = occurrence
        .form
        .chars()
        .all(|character| character.is_ascii_alphabetic());
    if form_is_english {
        let before = &text[..occurrence.local_start];
        let relative_start = [" that ", " which "]
            .into_iter()
            .filter_map(|marker| before.rfind(marker).map(|position| position + marker.len()))
            .max();
        let Some(relative_start) = relative_start else {
            return false;
        };
        let relative_prefix = before[relative_start..].trim();
        if relative_prefix.is_empty()
            || relative_prefix.split_whitespace().count().saturating_sub(1) > 4
        {
            return false;
        }
        let prior = relative_prefix
            .split_whitespace()
            .next_back()
            .unwrap_or_default()
            .trim_matches(|character: char| !character.is_ascii_alphanumeric());
        return !matches!(prior, "and" | "then" | "but" | "however");
    }

    if korean_embedded_action_request(text, occurrence) {
        return false;
    }
    let tail = &text[occurrence.local_start + occurrence.form.len()..];
    let relative_inflection = ["한 ", "했던 ", "하는 ", "된 ", "되는 ", "할 "]
        .iter()
        .find(|suffix| tail.starts_with(**suffix));
    let Some(relative_inflection) = relative_inflection else {
        return false;
    };
    let next_boundary = tail.find([',', ';', '.', '?', '!']).unwrap_or(tail.len());
    let relative_tail = &tail[..next_boundary];
    let modified_head = relative_tail
        .strip_prefix(relative_inflection)
        .unwrap_or(relative_tail)
        .split_whitespace()
        .next()
        .unwrap_or_default();
    if matches!(modified_head, "뒤" | "후" | "다음" | "후에" | "다음에") {
        return false;
    }
    relative_tail.split_whitespace().count() >= 2
        && relative_tail
            .split_whitespace()
            .any(|token| token.ends_with(['을', '를', '은', '는', '이', '가']))
}

fn is_counterfactual(text: &str) -> bool {
    [
        "더라면",
        "었더라면",
        "았더라면",
        "했을 텐데",
        "했을텐데",
        "if only",
        "would have",
        "could have",
        "had ",
    ]
    .iter()
    .any(|marker| text.contains(marker))
}

fn is_conditional(text: &str, occurrence: &ActionOccurrence) -> bool {
    let tail = &text[occurrence.local_start..];
    let korean_condition = tail
        .find("면 ")
        .map(|marker| {
            let before_marker = &tail[..marker];
            !before_marker.contains("고 ")
                && !before_marker.contains(',')
                && !before_marker.contains("그리고")
        })
        .unwrap_or_else(|| tail.ends_with('면'));
    korean_condition
        || text.trim_start().starts_with("if ")
        || text.contains(" unless ")
        || text.contains(" provided that ")
}

fn is_question(text: &str) -> bool {
    let trimmed = text.trim();
    trimmed.ends_with('?')
        || [
            "할까", "될까", "일까", "인가", "나요", "니", "why ", "what ", "how ",
        ]
        .iter()
        .any(|marker| trimmed.starts_with(marker) || trimmed.ends_with(marker))
}

fn is_directive(text: &str, occurrence: &ActionOccurrence) -> bool {
    let trimmed = text.trim();
    let directive_surface = strip_conversational_directive_lead_in(trimmed);
    let ascii_form = occurrence
        .form
        .chars()
        .all(|character| character.is_ascii_alphabetic() || character.is_ascii_whitespace());
    let tail = &text[occurrence.local_start + occurrence.form.len()..];
    let korean_inflected_ascii = ascii_form && has_korean_predicate_inflection(tail);
    if ascii_form && !korean_inflected_ascii {
        if is_action_nominal_form(&occurrence.form)
            && nominal_action_is_projectable(text, occurrence.local_start, &occurrence.form)
        {
            return true;
        }
        let prefix = &text[..occurrence.local_start];
        let contrast_directive = [", but now ", ", but ", ", however ", "; now "]
            .iter()
            .any(|marker| prefix.ends_with(marker));
        let prefixed_directive = directive_surface
            .strip_prefix("please ")
            .or_else(|| directive_surface.strip_prefix("just "))
            .or_else(|| directive_surface.strip_prefix("now "))
            .or_else(|| directive_surface.strip_prefix("then "))
            .or_else(|| directive_surface.strip_prefix("next "))
            .or_else(|| directive_surface.strip_prefix("finally "))
            .is_some_and(|tail| tail.starts_with(occurrence.form.as_str()));
        let benefactive = directive_surface
            .strip_prefix("please ")
            .unwrap_or(directive_surface);
        let help_directive = ["help me ", "help us "].iter().any(|prefix| {
            benefactive.strip_prefix(prefix).is_some_and(|tail| {
                tail.split(['.', '?', '!', ';'])
                    .next()
                    .is_some_and(|clause| clause.contains(occurrence.form.as_str()))
            })
        });
        let help_by_directive = ["help by ", "help me by ", "help us by "]
            .iter()
            .any(|prefix| {
                benefactive
                    .strip_prefix(prefix)
                    .is_some_and(|tail| tail.contains(occurrence.form.as_str()))
            });
        return directive_surface.starts_with(occurrence.form.as_str())
            || prefixed_directive
            || help_directive
            || help_by_directive
            || contrast_directive
            || trimmed.contains(&format!(" and {}", occurrence.form))
            || trimmed.contains(&format!(", {}", occurrence.form))
            || trimmed.contains(&format!(" then {}", occurrence.form));
    }
    if tail.starts_with("하되")
        && ["지 마", "지 말", "하지 마", "하지 말"]
            .iter()
            .any(|marker| tail.contains(marker))
    {
        return true;
    }
    if korean_embedded_action_request(text, occurrence)
        || korean_nominal_light_directive(text, occurrence)
    {
        return true;
    }
    let korean_help_complement = ["는 걸", "는 것을", "도록"]
        .iter()
        .any(|prefix| tail.trim_start().starts_with(prefix))
        && ["도와줘", "도와 줘", "도와주세요", "도와 주세요"]
            .iter()
            .any(|ending| trimmed.ends_with(ending));
    if korean_help_complement {
        return true;
    }
    let inflection = tail.trim_matches(|character: char| {
        character.is_whitespace()
            || character.is_ascii_punctuation()
            || matches!(character, '‘' | '’' | '“' | '”')
    });
    if [
        "어", "아", "여", "어줘", "아줘", "여줘", "어라", "아라", "세요",
    ]
    .contains(&inflection)
        || (inflection.is_empty()
            && [
                "열어",
                "알려",
                "찾아",
                "고쳐",
                "고쳐줘",
                "만들어",
                "배워",
                "옮겨",
                "보내",
                "지워",
                "이어가",
                "말해",
            ]
            .contains(&occurrence.form.as_str()))
    {
        return true;
    }
    let clause_is_directive = [
        "해",
        "해줘",
        "해주세요",
        "하자",
        "해라",
        "줘",
        "세요",
        "말해",
        "기록해",
        "지 마",
        "지 말",
    ]
    .iter()
    .any(|ending| trimmed.ends_with(ending))
        || trimmed.contains(" 해줘")
        || trimmed.contains(" 말고 ");
    if !clause_is_directive {
        return false;
    }
    let directive_tail = tail.trim_start();
    directive_tail.is_empty()
        || [
            "하", "해", "했", "한", "할", "고", "지", "면", "어", "아", "여", "줘", "세요", "자",
            "며", "면서", "만",
        ]
        .iter()
        .any(|suffix| directive_tail.starts_with(suffix))
}

fn korean_embedded_action_request(text: &str, occurrence: &ActionOccurrence) -> bool {
    if occurrence
        .form
        .chars()
        .all(|character| character.is_ascii_alphabetic() || character.is_ascii_whitespace())
    {
        return false;
    }
    let tail = text[occurrence.local_start + occurrence.form.len()..].trim_start();
    let nonfinite_link = [
        "하는 ",
        "할 ",
        "한 ",
        "해 보는 ",
        "어 보는 ",
        "아 보는 ",
        "보는 ",
        "는 쪽으로 ",
    ]
    .iter()
    .any(|link| tail.starts_with(link));
    let action_carrier = ["계획", "절차", "방안", "순서", "쪽으로", "안"]
        .iter()
        .any(|carrier| tail.contains(carrier));
    nonfinite_link && action_carrier && korean_clause_has_light_directive(text)
}

fn korean_nominal_light_directive(text: &str, occurrence: &ActionOccurrence) -> bool {
    if !matches!(
        occurrence.form.as_str(),
        "계획" | "설계" | "준비" | "우선순위"
    ) {
        return false;
    }
    let tail = text[occurrence.local_start + occurrence.form.len()..].trim_start();
    let tail = tail
        .strip_prefix('을')
        .or_else(|| tail.strip_prefix('를'))
        .unwrap_or(tail)
        .trim_start();
    ["잡", "짜", "세우", "정하", "두", "마련하", "만들"]
        .iter()
        .any(|verb| tail.starts_with(verb))
        && korean_clause_has_light_directive(text)
}

fn korean_clause_has_light_directive(text: &str) -> bool {
    let trimmed = text.trim().trim_end_matches(['.', '!', '?']);
    [
        "해",
        "해 줘",
        "해줘",
        "해주세요",
        "해 주세요",
        "잡아",
        "잡아 줘",
        "잡아줘",
        "짜",
        "짜 줘",
        "짜줘",
        "세워",
        "세워 줘",
        "세워줘",
        "둬",
        "둬 줘",
        "둬줘",
    ]
    .iter()
    .any(|ending| trimmed.ends_with(ending))
}

fn strip_conversational_directive_lead_in(mut text: &str) -> &str {
    for _ in 0..3 {
        text = text.trim_start_matches(|character: char| {
            character.is_whitespace()
                || character.is_ascii_punctuation()
                || matches!(character, '—' | '–' | '…')
        });
        let Some(marker) = [
            "actually",
            "okay",
            "right",
            "wait",
            "well",
            "yeah",
            "yes",
            "ok",
            "아니",
            "잠깐",
            "그러면",
            "음",
            "어",
        ]
        .iter()
        .find(|marker| {
            text.strip_prefix(**marker).is_some_and(|tail| {
                tail.is_empty()
                    || tail.chars().next().is_some_and(|character| {
                        character.is_whitespace()
                            || character.is_ascii_punctuation()
                            || matches!(character, '—' | '–' | '…')
                    })
            })
        }) else {
            break;
        };
        text = &text[marker.len()..];
    }
    text.trim_start_matches(|character: char| {
        character.is_whitespace()
            || character.is_ascii_punctuation()
            || matches!(character, '—' | '–' | '…')
    })
}

fn detect_asserted_modality(text: &str) -> FrameModalityIR {
    if ["해야", "필수", "must ", "should ", "need to", "required"]
        .iter()
        .any(|marker| text.contains(marker))
    {
        FrameModalityIR::Necessary
    } else if ["수 있", "가능", "might ", "may ", "can ", "could "]
        .iter()
        .any(|marker| text.contains(marker))
    {
        FrameModalityIR::Possible
    } else {
        FrameModalityIR::Asserted
    }
}

fn extract_theme(text: &str, occurrence: &ActionOccurrence) -> String {
    let ascii_form = occurrence
        .form
        .chars()
        .all(|character| character.is_ascii_alphabetic() || character.is_ascii_whitespace());
    let tail = &text[occurrence.local_start + occurrence.form.len()..];
    if ascii_form && !has_korean_predicate_inflection(tail) {
        if is_action_nominal_form(&occurrence.form)
            && nominal_action_is_projectable(text, occurrence.local_start, &occurrence.form)
        {
            return extract_english_nominal_theme(text, occurrence);
        }
        return extract_english_theme(text, occurrence);
    }
    // Discourse particles introduce the clause, but they are not arguments of
    // its first predicate. Directive detection already normalizes this
    // boundary; argument extraction must consume the same normalized prefix
    // or an opener such as `아니,` can leak into Theme and become a plan target.
    let before = strip_conversational_directive_lead_in(
        text[..occurrence.local_start]
            .rsplit(['.', '?', '!', ';', '\n', '\r'])
            .next()
            .unwrap_or_default()
            .trim(),
    );
    let corrected = if let Some(position) = before.rfind("말고") {
        &before[position + "말고".len()..]
    } else {
        before
    };
    let mut theme_tokens = corrected.split_whitespace().collect::<Vec<_>>();
    while theme_tokens.last().is_some_and(|token| {
        matches!(
            token.trim_matches(|character: char| character.is_ascii_punctuation()),
            "지금" | "먼저" | "우선" | "바로" | "다시" | "계속" | "이제" | "실제로" | "좀"
        )
    }) {
        theme_tokens.pop();
    }
    let theme_prefix = theme_tokens.join(" ");
    let token = theme_prefix
        .split_whitespace()
        .next_back()
        .unwrap_or(corrected)
        .trim_matches(|character: char| {
            matches!(character, ',' | ':' | ';' | '"' | '\'' | '“' | '”')
        });
    let theme = strip_korean_focus_and_case(token);
    if matches!(theme, "뒤" | "후" | "다음" | "그리고" | "후에") {
        "PRIOR_RESULT".to_string()
    } else {
        korean_compound_theme(&theme_prefix, theme).unwrap_or_else(|| theme.to_string())
    }
}

fn extract_english_nominal_theme(text: &str, occurrence: &ActionOccurrence) -> String {
    let after = text[occurrence.local_start + occurrence.form.len()..]
        .split([',', ';', '.', '?', '!'])
        .next()
        .unwrap_or_default()
        .trim();
    for prefix in ["for ", "of ", "about ", "on "] {
        if let Some(theme) = after.strip_prefix(prefix) {
            let theme = trim_trailing_english_action_modifiers(theme.trim());
            if !theme.is_empty() {
                return theme.to_string();
            }
        }
    }
    for marker in [" for ", " of ", " about ", " on "] {
        if let Some((_, theme)) = after.split_once(marker) {
            let theme = trim_trailing_english_action_modifiers(theme.trim());
            if !theme.is_empty() {
                return theme.to_string();
            }
        }
    }
    for modal in [
        "should cover ",
        "must cover ",
        "needs to cover ",
        "need to cover ",
        "is to cover ",
    ] {
        if let Some(theme) = after.strip_prefix(modal) {
            let theme = trim_trailing_english_action_modifiers(theme.trim());
            if !theme.is_empty() {
                return theme.to_string();
            }
        }
    }

    let before = text[..occurrence.local_start]
        .rsplit(['.', '?', '!', ';', '\n', '\r'])
        .next()
        .unwrap_or_default()
        .trim();
    let before = strip_conversational_directive_lead_in(before);
    let before = before.strip_prefix("please ").unwrap_or(before);
    let before = [
        "start with ",
        "begin with ",
        "draft ",
        "give ",
        "provide ",
        "prepare ",
        "make ",
        "create ",
        "develop ",
    ]
    .iter()
    .find_map(|prefix| before.strip_prefix(prefix))
    .unwrap_or(before)
    .trim();
    let before = before
        .strip_prefix("an ")
        .or_else(|| before.strip_prefix("a "))
        .unwrap_or(before)
        .trim();
    if !before.is_empty() {
        return before.to_string();
    }

    after
        .split_whitespace()
        .filter(|token| {
            !matches!(
                *token,
                "plan" | "steps" | "procedure" | "sequence" | "priority" | "first"
            )
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn has_korean_predicate_inflection(tail: &str) -> bool {
    [
        "하",
        "해",
        "했",
        "한",
        "할",
        "하고",
        "하지",
        "하면",
        "해줘",
        "해주세요",
    ]
    .iter()
    .any(|suffix| tail.starts_with(suffix))
}

fn extract_english_theme(text: &str, occurrence: &ActionOccurrence) -> String {
    let tail = text[occurrence.local_start + occurrence.form.len()..].trim();
    if occurrence.form.ends_with(" through") {
        if let Some(embedded) = action_occurrences(tail, &[]).into_iter().next() {
            let embedded_theme = extract_theme(tail, &embedded);
            if !is_structural_argument_gap(&embedded_theme) {
                return embedded_theme;
            }
        }
    }
    let tail = tail
        .split([',', ';', '.', '?', '!'])
        .next()
        .unwrap_or(tail)
        .trim();
    let tail = truncate_at_coordinated_action(tail);
    let tail = tail.split(" not ").next().unwrap_or(tail).trim();
    let tail = trim_trailing_english_action_modifiers(tail);
    let tail = ["at ", "in ", "into "]
        .into_iter()
        .find_map(|prefix| tail.strip_prefix(prefix))
        .unwrap_or(tail)
        .trim();
    if !tail.is_empty() {
        tail.trim_matches(|character: char| matches!(character, '.' | '?' | '!' | '"' | '\''))
            .to_string()
    } else {
        let before = text[..occurrence.local_start].trim();
        if let Some(subject) = passive_subject(before) {
            subject
        } else {
            before
                .split_whitespace()
                .next_back()
                .unwrap_or_default()
                .trim_matches(|character: char| {
                    !character.is_ascii_alphanumeric() && character != '_'
                })
                .to_string()
        }
    }
}

fn trim_trailing_english_action_modifiers(text: &str) -> &str {
    let mut trimmed = text.trim_end();
    loop {
        let lower = trimmed.to_lowercase();
        let suffix = [" right now", " immediately", " now", " first"]
            .iter()
            .find(|suffix| lower.ends_with(**suffix));
        let Some(suffix) = suffix else {
            return trimmed;
        };
        trimmed = trimmed[..trimmed.len() - suffix.len()].trim_end();
    }
}

fn truncate_at_coordinated_action(text: &str) -> &str {
    [" and ", " then "]
        .into_iter()
        .filter_map(|connector| {
            text.match_indices(connector).find_map(|(position, _)| {
                let remainder = text[position + connector.len()..].trim_start();
                starts_with_action_clause(remainder).then_some(position)
            })
        })
        .min()
        .map_or(text, |position| text[..position].trim_end())
}

fn starts_with_action_clause(text: &str) -> bool {
    let normalized = text.to_lowercase();
    let action_surface = ["do not ", "don't ", "never ", "not "]
        .into_iter()
        .find_map(|prefix| normalized.strip_prefix(prefix))
        .unwrap_or(&normalized);
    pragmatic_action_mentions(action_surface)
        .iter()
        .any(|mention| mention.start_byte == 0)
}

fn is_structural_argument_gap(theme: &str) -> bool {
    if theme.trim().is_empty() {
        return true;
    }
    if !theme.is_ascii() {
        return false;
    }
    let tokens = theme
        .split_whitespace()
        .map(|token| {
            token
                .trim_matches(|character: char| !character.is_ascii_alphanumeric())
                .to_lowercase()
        })
        .filter(|token| !token.is_empty())
        .collect::<Vec<_>>();
    !tokens.is_empty()
        && tokens.iter().all(|token| {
            matches!(
                token.as_str(),
                "and" | "or" | "then" | "do" | "does" | "did" | "not" | "never" | "please"
            )
        })
}

fn korean_compound_theme(prefix: &str, head: &str) -> Option<String> {
    let semantic_compound_head = matches!(
        head,
        "비용"
            | "계획"
            | "결과"
            | "상태"
            | "기록"
            | "경로"
            | "절차"
            | "정책"
            | "여부"
            | "가능성"
            | "시간"
            | "방식"
            | "기능"
            | "추적"
            | "이력"
    );
    let tokens = prefix.split_whitespace().collect::<Vec<_>>();
    let modifier = tokens
        .len()
        .checked_sub(2)
        .and_then(|index| tokens.get(index))
        .map(|token| strip_korean_focus_and_case(token))?;
    if modifier.is_empty() || matches!(modifier, "그리고" | "하지만" | "그러나" | "이제" | "문서")
    {
        return None;
    }
    let labeled_object = modifier
        .chars()
        .any(|character| character.is_ascii_alphabetic());
    if !semantic_compound_head && !labeled_object {
        return None;
    }
    Some(format!("{modifier} {head}"))
}

fn passive_subject(before: &str) -> Option<String> {
    for auxiliary in [
        " should be",
        " must be",
        " can be",
        " was",
        " were",
        " is",
        " are",
    ] {
        if let Some(head) = before.strip_suffix(auxiliary) {
            let unquoted_head = head
                .split(['“', '”', '‘', '’', '"', '\''])
                .next()
                .unwrap_or(head)
                .trim();
            if !unquoted_head.is_empty() {
                return Some(unquoted_head.to_string());
            }
        }
    }
    None
}

fn strip_korean_focus_and_case(token: &str) -> &str {
    for suffix in [
        "에서만",
        "에게만",
        "으로만",
        "를",
        "을",
        "은",
        "는",
        "이",
        "가",
        "에",
        "만",
    ] {
        if let Some(stem) = token.strip_suffix(suffix) {
            if !stem.is_empty() {
                return stem;
            }
        }
    }
    token
}

fn alternative_scopes(text: &str, start_index: usize) -> Vec<ScopeConstraintIR> {
    let mut scopes = Vec::new();
    if let Some(position) = text.find("말고") {
        let excluded = text[..position]
            .split_whitespace()
            .next_back()
            .map(strip_korean_focus_and_case)
            .unwrap_or_default();
        if !excluded.is_empty() {
            scopes.push(scope(
                start_index + scopes.len(),
                ScopeKindIR::AlternativeExclusion,
                None,
                excluded,
            ));
        }
    }
    if let Some(position) = text.find(" not ") {
        let excluded = text[position + 5..]
            .trim()
            .trim_matches(|character: char| matches!(character, '.' | '?' | '!'));
        if !excluded.is_empty() {
            scopes.push(scope(
                start_index + scopes.len(),
                ScopeKindIR::AlternativeExclusion,
                None,
                excluded,
            ));
        }
    }
    if has_focus_marker(text) {
        scopes.push(scope(
            start_index + scopes.len(),
            ScopeKindIR::FocusOnly,
            None,
            text,
        ));
    }
    scopes
}

fn has_focus_marker(text: &str) -> bool {
    text.contains("만 ")
        || text.contains("만해")
        || text.contains("just ")
        || text.contains("only ")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn analyze(text: &str) -> CompositionalAnalysisIR {
        CompositionalSemanticAnalyzer.analyze(text)
    }

    #[test]
    fn negation_scope_blocks_repair_but_keeps_outer_explanation() {
        let analysis = analyze("오류를 고치지 말고 원인만 설명해");
        let selected = analysis.selected_candidate().expect("selected explanation");
        assert_eq!(selected.intent, PlanIntentIR::Explain);
        assert_eq!(selected.subject, "원인");
        assert!(analysis.candidates.iter().any(|candidate| {
            candidate.intent == PlanIntentIR::Repair
                && candidate.disposition == CandidateDispositionIR::BlockedByNegation
        }));
    }

    #[test]
    fn discourse_opener_never_becomes_the_first_predicates_theme() {
        for opener in ["아니,", "음,", "잠깐,"] {
            let surface = format!("{opener} 고치지는 말고 왜 실패하는지만 설명해.");
            let analysis = analyze(&surface);
            let repair = analysis
                .frames
                .iter()
                .find(|frame| frame.intent_hint == PlanIntentIR::Repair)
                .expect("negated repair frame");
            assert_ne!(repair.theme, opener.trim_end_matches(','));
            assert!(
                repair.theme.is_empty(),
                "an omitted repair argument must remain a bindable gap: {repair:#?}"
            );
            assert!(analysis.candidates.iter().all(|candidate| {
                candidate.subject != opener.trim_end_matches(',')
                    && candidate.subject.to_lowercase() != "no"
            }));
        }
    }

    #[test]
    fn recheck_is_a_productive_investigation_form_not_a_generic_plan_fallback() {
        let analysis = analyze("recheck the original pair's reports");
        let selected = analysis.selected_candidate().expect("selected recheck");
        assert_eq!(selected.intent, PlanIntentIR::Investigate);
        assert!(analysis.frames.iter().any(|frame| {
            frame.canonical_predicate == "INVESTIGATE" && frame.predicate_surface == "recheck"
        }));
        assert!(selected.external_execution_authorized);
    }

    #[test]
    fn polite_indirect_inspection_forms_ground_to_the_same_investigation_goal() {
        let english = analyze("Could you take a look at the Aster cache?");
        let english_goal = english
            .selected_candidate()
            .expect("English polite request");
        assert_eq!(english_goal.intent, PlanIntentIR::Investigate);
        assert_eq!(english_goal.subject, "the aster cache");
        assert!(english_goal.external_execution_authorized);

        let korean = analyze("Aster 캐시 좀 봐줄래?");
        let korean_goal = korean.selected_candidate().expect("Korean polite request");
        assert_eq!(korean_goal.intent, PlanIntentIR::Investigate);
        assert_eq!(korean_goal.subject, "aster 캐시");
        assert!(korean_goal.external_execution_authorized);
    }

    #[test]
    fn korean_concessive_directive_keeps_authorized_action_and_blocks_prohibition() {
        let analysis = analyze("로그를 분석하되 캐시는 삭제하지 마");
        let selected = analysis.selected_candidate().expect("authorized analysis");
        assert_eq!(selected.intent, PlanIntentIR::Investigate);
        assert_eq!(selected.subject, "로그");
        assert!(selected.external_execution_authorized);
        assert_eq!(analysis.blocked_execution_count(), 1);
        assert!(analysis.candidates.iter().any(|candidate| {
            candidate.intent == PlanIntentIR::Execute
                && candidate.subject == "캐시"
                && candidate.disposition == CandidateDispositionIR::BlockedByNegation
        }));
    }

    #[test]
    fn english_prohibition_is_not_overridden_by_a_later_explanation() {
        let analysis = analyze("Do not deploy it; just explain why the tests fail.");
        let selected = analysis.selected_candidate().expect("selected explanation");
        assert_eq!(selected.intent, PlanIntentIR::Explain);
        assert!(analysis.candidates.iter().any(|candidate| {
            candidate.intent == PlanIntentIR::Execute
                && candidate.disposition == CandidateDispositionIR::BlockedByNegation
        }));
    }

    #[test]
    fn quoted_command_has_no_authority() {
        let analysis = analyze("'데이터를 삭제해'라는 문장을 설명해");
        let selected = analysis.selected_candidate().expect("outer explanation");
        assert_eq!(selected.intent, PlanIntentIR::Explain);
        assert!(analysis.candidates.iter().any(|candidate| {
            candidate.intent == PlanIntentIR::Execute
                && candidate.disposition == CandidateDispositionIR::NonAuthoritativeMention
        }));
    }

    #[test]
    fn reported_deployment_is_not_a_user_deployment_request() {
        let analysis = analyze("팀장이 배포하라고 했다는 사실만 기록해");
        let selected = analysis.selected_candidate().expect("record request");
        assert_eq!(selected.intent, PlanIntentIR::Communicate);
        assert!(analysis.candidates.iter().any(|candidate| {
            candidate.intent == PlanIntentIR::Execute
                && candidate.disposition == CandidateDispositionIR::NonAuthoritativeMention
        }));
    }

    #[test]
    fn counterfactual_action_is_not_executable() {
        let analysis = analyze("캐시를 지웠더라면 빌드가 됐을 텐데");
        assert!(analysis.selected_candidate().is_none());
        assert!(!analysis.candidates.is_empty());
        assert!(analysis.candidates.iter().all(|candidate| {
            candidate.disposition == CandidateDispositionIR::HypotheticalOnly
                && !candidate.external_execution_authorized
        }));
    }

    #[test]
    fn unicode_quoted_korean_command_is_a_non_authoritative_mention() {
        let analysis = analyze("‘코드를 고쳐’라는 표현을 해설해");
        let selected = analysis.selected_candidate().expect("outer explanation");
        assert_eq!(selected.intent, PlanIntentIR::Explain);
        assert!(analysis.candidates.iter().any(|candidate| {
            candidate.intent == PlanIntentIR::Repair
                && candidate.disposition == CandidateDispositionIR::NonAuthoritativeMention
        }));
    }

    #[test]
    fn conditional_question_becomes_investigation() {
        let analysis = analyze("캐시를 지우면 빌드가 빨라질까?");
        let selected = analysis.selected_candidate().expect("investigation");
        assert_eq!(selected.intent, PlanIntentIR::Investigate);
        assert!(!selected.external_execution_authorized);
    }

    #[test]
    fn correction_selects_new_target_and_preserves_excluded_target() {
        let analysis = analyze("API 말고 CLI를 문서화해");
        let selected = analysis.selected_candidate().expect("create documentation");
        assert_eq!(selected.intent, PlanIntentIR::Create);
        assert_eq!(selected.subject, "cli");
        assert!(analysis.scopes.iter().any(|scope| {
            scope.kind == ScopeKindIR::AlternativeExclusion && scope.surface_text == "api"
        }));
    }

    #[test]
    fn definition_grounded_new_predicate_reuses_scope_and_intent_rules() {
        let predicate = PredicateLexemeIR {
            schema: PREDICATE_LEXEME_SCHEMA.to_string(),
            predicate_id: "P-REFINE-DOCUMENT-KO".to_string(),
            language: LanguageCodeIR::Korean,
            surface_forms: vec!["다듬".to_string()],
            canonical_predicate: "C_REFINE_DOCUMENT".to_string(),
            intent_hint: PlanIntentIR::Create,
            definition: "revise a document into a clearer finished form".to_string(),
            confidence_millis: 920,
        };
        let positive = CompositionalSemanticAnalyzer
            .analyze_with_predicates("문서를 다듬어줘", std::slice::from_ref(&predicate));
        let selected = positive.selected_candidate().expect("learned predicate");
        assert_eq!(selected.intent, PlanIntentIR::Create);
        assert_eq!(selected.subject, "문서");
        assert!(selected.external_execution_authorized);

        let negated = CompositionalSemanticAnalyzer.analyze_with_predicates(
            "문서를 다듬지 말고 원인을 설명해줘",
            std::slice::from_ref(&predicate),
        );
        assert_eq!(
            negated
                .selected_candidate()
                .expect("outer explanation")
                .intent,
            PlanIntentIR::Explain
        );
        assert!(negated.candidates.iter().any(|candidate| {
            candidate.source_frame_id
                == negated
                    .frames
                    .iter()
                    .find(|frame| frame.canonical_predicate == "C_REFINE_DOCUMENT")
                    .expect("learned frame")
                    .frame_id
                && candidate.disposition == CandidateDispositionIR::BlockedByNegation
        }));
    }

    #[test]
    fn predicate_snapshot_is_canonical_and_tamper_evident() {
        let predicate = PredicateLexemeIR {
            schema: PREDICATE_LEXEME_SCHEMA.to_string(),
            predicate_id: "P-PERSIST-REFINE".to_string(),
            language: LanguageCodeIR::Korean,
            surface_forms: vec!["다듬".to_string()],
            canonical_predicate: "C_REFINE_DOCUMENT".to_string(),
            intent_hint: PlanIntentIR::Create,
            definition: "revise a document into a clearer finished form".to_string(),
            confidence_millis: 920,
        };
        let snapshot = PredicateLexiconSnapshotIR::build(vec![predicate]).expect("snapshot");
        snapshot.validate().expect("valid snapshot");
        assert_eq!(snapshot.snapshot_sha256.len(), 64);

        let mut tampered = snapshot;
        tampered.entries[0].canonical_predicate = "C_DELETE_DOCUMENT".to_string();
        assert_eq!(
            tampered.validate(),
            Err(PredicateLexemeError::InvalidSemantics)
        );
    }

    #[test]
    fn korean_compound_request_becomes_ordered_goal_graph_with_prohibition() {
        let analysis = analyze("파일을 읽고 각 줄을 변환한 뒤 저장해. 원본은 지우지 마");
        let graph = analysis.goal_graph.as_ref().expect("ordered goal graph");
        assert_eq!(graph.nodes.len(), 3);
        assert_eq!(graph.edges.len(), 2);
        assert!(graph
            .edges
            .iter()
            .all(|edge| edge.relation == GoalGraphRelationKindIR::Sequence));
        assert_eq!(graph.prohibitions.len(), 1);
        assert_eq!(analysis.selected_candidates().len(), 3);
        assert!(!analysis.clarification_required);
        assert_eq!(graph.nodes[0].subject, "파일");
        assert_eq!(graph.nodes[1].subject, "줄");
        assert_eq!(graph.nodes[2].subject, "PRIOR_RESULT");
    }

    #[test]
    fn english_compound_request_preserves_sequence_and_delete_prohibition() {
        let analysis = analyze(
            "Read the file, transform each line, then save it. Do not delete the original.",
        );
        let graph = analysis.goal_graph.as_ref().expect("ordered goal graph");
        assert_eq!(graph.nodes.len(), 3);
        assert_eq!(graph.edges.len(), 2);
        assert_eq!(graph.prohibitions.len(), 1);
        assert!(graph
            .nodes
            .iter()
            .all(|node| node.external_execution_authorized));
        assert!(!analysis.clarification_required);
    }

    #[test]
    fn condition_is_attached_to_coordinated_goal_graph() {
        let analysis = analyze("파일을 읽고 오류가 없으면 저장해");
        let graph = analysis.goal_graph.as_ref().expect("conditional graph");
        assert_eq!(graph.nodes.len(), 2);
        assert_eq!(graph.edges.len(), 1);
        assert_eq!(graph.conditions.len(), 1);
        assert!(graph.conditions[0].contains("오류가 없으면"));
    }

    #[test]
    fn negated_english_conjunct_keeps_the_shared_object_gap() {
        let analysis = analyze("Inspect and do not delete the cache.");
        assert_eq!(analysis.frames.len(), 2);
        assert!(analysis.frames[0].theme.is_empty());
        assert_eq!(analysis.frames[1].theme, "the cache");
        assert_eq!(
            analysis.semantic_role_graph.shared_argument_bindings.len(),
            1,
            "clause={:#?}; role={:#?}",
            analysis.clause_graph,
            analysis.semantic_role_graph
        );
        let argument_ids = analysis
            .frames
            .iter()
            .map(|frame| {
                analysis
                    .semantic_role_graph
                    .primary_argument_for_frame(&frame.frame_id)
                    .expect("shared cache")
                    .node_id
                    .as_str()
            })
            .collect::<BTreeSet<_>>();
        assert_eq!(argument_ids.len(), 1);
        assert_eq!(analysis.selected_candidates().len(), 1);
        assert_eq!(
            analysis
                .candidates
                .iter()
                .filter(|candidate| {
                    candidate.disposition == CandidateDispositionIR::BlockedByNegation
                })
                .count(),
            1
        );
    }

    #[test]
    fn literal_execute_selects_the_execute_intent() {
        let analysis = analyze("Execute the Cinder migration");
        let selected = analysis.selected_candidate().expect("execute request");
        assert_eq!(selected.intent, PlanIntentIR::Execute);
        assert!(selected.external_execution_authorized);
    }

    #[test]
    fn korean_relative_action_is_descriptive_not_an_executable_goal() {
        let analysis = analyze("파서가 수리한 모든 파일을 분석해");
        let selected = analysis.selected_candidates();
        assert_eq!(selected.len(), 1);
        assert_eq!(selected[0].intent, PlanIntentIR::Investigate);
        assert_eq!(selected[0].subject, "파일");
        assert!(analysis.candidates.iter().any(|candidate| {
            candidate.intent == PlanIntentIR::Repair
                && candidate.disposition == CandidateDispositionIR::NonAuthoritativeMention
                && !candidate.external_execution_authorized
        }));
    }

    #[test]
    fn korean_temporal_adnominal_keeps_all_three_requested_events() {
        let analysis = analyze("파일을 분석하고 폴더를 수리한 뒤 보고서를 저장해");
        let graph = analysis.goal_graph.as_ref().expect("ordered goal graph");
        assert_eq!(graph.nodes.len(), 3);
        assert_eq!(
            graph
                .nodes
                .iter()
                .map(|node| node.intent)
                .collect::<Vec<_>>(),
            vec![
                PlanIntentIR::Investigate,
                PlanIntentIR::Repair,
                PlanIntentIR::Execute
            ]
        );
        assert!(graph
            .nodes
            .iter()
            .all(|node| node.external_execution_authorized));
    }

    #[test]
    fn nominal_determiner_does_not_cross_a_clause_boundary() {
        let mentions = pragmatic_action_mentions("No, inspect it instead of deleting it.");
        assert!(mentions
            .iter()
            .any(|mention| mention.canonical_predicate == "INVESTIGATE"));
        assert!(mentions
            .iter()
            .any(|mention| mention.canonical_predicate == "DELETE"));
    }

    #[test]
    fn noun_report_is_not_promoted_to_a_communicate_action() {
        let mentions = pragmatic_action_mentions("Not that, the report instead.");
        assert!(!mentions
            .iter()
            .any(|mention| mention.canonical_predicate == "COMMUNICATE"));
    }

    #[test]
    fn assessment_nominal_and_korean_hada_form_ground_to_investigation() {
        let english = analyze("I am asking only for an assessment of recovery cost.");
        assert!(english
            .frames
            .iter()
            .any(|frame| frame.canonical_predicate == "INVESTIGATE"));
        assert!(english
            .candidates
            .iter()
            .any(|candidate| candidate.subject.contains("recovery")));

        let korean = analyze("게시가 감사 추적을 보존하는지 평가하고 결과만 보고해.");
        assert!(korean
            .frames
            .iter()
            .any(|frame| frame.canonical_predicate == "INVESTIGATE"));

        let descriptive = analyze("The assessment was incomplete.");
        assert!(!descriptive
            .frames
            .iter()
            .any(|frame| frame.canonical_predicate == "INVESTIGATE"));

        let object_nominal = analyze("Do not publish assessment.");
        assert!(!object_nominal
            .frames
            .iter()
            .any(|frame| frame.canonical_predicate == "INVESTIGATE"));
    }

    #[test]
    fn event_nominals_compose_with_light_verbs_without_promoting_bare_nouns() {
        let cases = [
            (
                "Begin with an evaluation outline for the Birch queue.",
                PlanIntentIR::Investigate,
                "birch queue",
            ),
            (
                "Prepare remediation steps for the Maple worker.",
                PlanIntentIR::Repair,
                "maple worker",
            ),
            (
                "Give the Cedar pipeline inspection first priority.",
                PlanIntentIR::Investigate,
                "cedar pipeline",
            ),
            (
                "The explanation should cover the Juniper scheduler.",
                PlanIntentIR::Explain,
                "juniper scheduler",
            ),
        ];
        for (surface, expected_intent, expected_subject) in cases {
            let analysis = analyze(surface);
            let selected = analysis.selected_candidate().unwrap_or_else(|| {
                panic!("structural nominal request was not selected: {surface}: {analysis:#?}")
            });
            assert_eq!(selected.intent, expected_intent, "surface={surface}");
            assert!(
                selected.subject.contains(expected_subject),
                "surface={surface}; subject={}",
                selected.subject
            );
            assert!(selected.external_execution_authorized, "surface={surface}");
        }

        for surface in [
            "The inspection was incomplete.",
            "The recovery plan was archived.",
            "Do not publish the assessment.",
        ] {
            let analysis = analyze(surface);
            assert!(
                analysis.selected_candidate().is_none(),
                "bare/descriptive nominal acquired a goal: {surface}: {analysis:#?}"
            );
        }
    }

    #[test]
    fn productive_benefactive_and_discontinuous_explanation_constructions_are_directives() {
        let help = analyze("Help by investigating what causes the Quartz timeout.");
        let help_goal = help
            .selected_candidate()
            .expect("benefactive investigation");
        assert_eq!(help_goal.intent, PlanIntentIR::Investigate);
        assert!(help_goal.external_execution_authorized);

        let walkthrough = analyze("Walk us through what you would inspect in the Pine worker.");
        let walkthrough_goal = walkthrough
            .selected_candidate()
            .expect("discontinuous explanation request");
        assert_eq!(walkthrough_goal.intent, PlanIntentIR::Explain);
        assert!(walkthrough_goal.external_execution_authorized);

        let discourse_lead_in = analyze("Okay, go ahead and narrow down the latency cause first.");
        let lead_in_goal = discourse_lead_in
            .selected_candidate()
            .expect("lead-in investigation request");
        assert_eq!(lead_in_goal.intent, PlanIntentIR::Investigate);
        assert!(lead_in_goal.external_execution_authorized);
    }

    #[test]
    fn revision_prefaces_do_not_compete_with_the_replacement_request() {
        for surface in [
            "Let me correct that: the explanation should cover the Alder relay.",
            "Actually, let me correct this: the briefing should cover the Birch worker.",
        ] {
            let analysis = analyze(surface);
            let viable = analysis
                .candidates
                .iter()
                .filter(|candidate| candidate.disposition == CandidateDispositionIR::Viable)
                .collect::<Vec<_>>();
            assert_eq!(viable.len(), 1, "surface={surface}; analysis={analysis:#?}");
            assert_eq!(viable[0].intent, PlanIntentIR::Explain, "surface={surface}");
            assert!(viable[0].external_execution_authorized, "surface={surface}");
        }

        let concrete_repair = analyze("Correct the Cedar cache.");
        let selected = concrete_repair
            .selected_candidate()
            .expect("concrete correction remains a repair request");
        assert_eq!(selected.intent, PlanIntentIR::Repair);
        assert!(selected.subject.contains("cedar cache"));
    }

    #[test]
    fn korean_requested_plan_carrier_lifts_the_embedded_semantic_action() {
        let analysis = analyze("Birch 큐부터 상태를 파악하는 계획을 잡아 줘.");
        let selected = analysis
            .selected_candidate()
            .expect("embedded investigation request");
        assert_eq!(selected.intent, PlanIntentIR::Investigate);
        assert!(selected.subject.contains("상태"));
        assert!(selected.external_execution_authorized);
        assert!(!analysis.selected_candidates().iter().any(|candidate| {
            candidate.intent == PlanIntentIR::Plan && candidate.external_execution_authorized
        }));

        let progressive = analyze("Quartz 지연 원인을 좁혀 보는 쪽으로 진행해 줘.");
        let progressive_goal = progressive
            .selected_candidate()
            .expect("embedded progressive request");
        assert_eq!(progressive_goal.intent, PlanIntentIR::Investigate);
        assert!(progressive_goal.external_execution_authorized);
    }

    #[test]
    fn korean_try_auxiliary_preserves_the_lexical_action_boundary() {
        let analysis = analyze("아카이브를 열어보고 복구해");
        assert_eq!(
            analysis
                .frames
                .iter()
                .map(|frame| frame.canonical_predicate.as_str())
                .collect::<Vec<_>>(),
            vec!["EXECUTE", "REPAIR"]
        );
        assert!(!analysis
            .frames
            .iter()
            .any(|frame| frame.canonical_predicate == "COMMUNICATE"));
        assert_eq!(
            analysis.semantic_role_graph.shared_argument_bindings.len(),
            1
        );
    }

    #[test]
    fn korean_attribution_contrast_keeps_the_outer_requested_target() {
        let analysis = analyze("민수는 파일을 삭제하라고 말했지만 이제 로그를 확인해");
        assert!(analysis.candidates.iter().any(|candidate| {
            candidate.intent == PlanIntentIR::Investigate
                && candidate.subject.contains("로그")
                && candidate.external_execution_authorized
        }));
    }

    #[test]
    fn quoted_report_does_not_truncate_later_compound_assessment_subject() {
        let english = analyze(
            "The runbook says 'publish the bundle and report that result.' Assess recovery cost only; do not publish it.",
        );
        assert!(
            english.frames.iter().any(|frame| {
                frame.canonical_predicate == "INVESTIGATE"
                    && frame.theme.contains("recovery cost")
                    && !frame.embedded_under_quote
            }),
            "English frames: {:#?}",
            english.frames
        );
        let korean = analyze(
            "문서에는 '번들을 게시하고 그 결과를 보고해'라고 쓰여 있어. 복구 비용만 평가해. 게시하지 마.",
        );
        assert!(
            korean.frames.iter().any(|frame| {
                frame.canonical_predicate == "INVESTIGATE"
                    && frame.theme.contains("복구 비용")
                    && !frame.embedded_under_quote
            }),
            "Korean frames: {:#?}",
            korean.frames
        );
    }

    #[test]
    fn atomic_planning_and_observation_predicates_transfer_across_new_objects() {
        for (source, expected) in [
            (
                "Arrange the Birch queue before the Cedar cache.",
                PlanIntentIR::Plan,
            ),
            (
                "Design a recovery procedure for the Harbor worker.",
                PlanIntentIR::Plan,
            ),
            (
                "Walk through the Amber service layout.",
                PlanIntentIR::Explain,
            ),
            ("Delta 로그를 관찰만 해.", PlanIntentIR::Investigate),
            ("Harbor 워커 복구 절차를 설계해.", PlanIntentIR::Plan),
        ] {
            let analysis = analyze(source);
            assert!(
                analysis
                    .selected_candidates()
                    .iter()
                    .any(|candidate| candidate.intent == expected
                        && candidate.external_execution_authorized),
                "source={source} analysis={analysis:#?}"
            );
        }
    }

    #[test]
    fn new_atomic_predicates_do_not_turn_descriptions_into_requests() {
        for source in [
            "Mina described how Rowan could organize the Birch queue.",
            "The Harbor worker design was incomplete.",
            "Delta 로그 관찰은 어제 끝났다.",
        ] {
            let analysis = analyze(source);
            assert!(
                analysis.selected_candidates().is_empty(),
                "source={source} analysis={analysis:#?}"
            );
        }
    }

    #[test]
    fn benefactive_requests_and_discourse_lead_ins_preserve_directive_force() {
        for source in [
            "Please help us narrow down the Harbor worker failures.",
            "Okay—focus on separating the Birch cache causes.",
            "오류 원인을 좁히는 걸 도와줘.",
            "음, Cedar 큐 원인에 집중해.",
        ] {
            let analysis = analyze(source);
            assert!(
                analysis.selected_candidates().iter().any(|candidate| {
                    candidate.intent == PlanIntentIR::Investigate
                        && candidate.external_execution_authorized
                }),
                "source={source} analysis={analysis:#?}"
            );
        }
    }
}
