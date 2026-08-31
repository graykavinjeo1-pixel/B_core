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
use crate::language_knowledge::LanguageCodeIR;
use crate::modality::{ModalScopeGraphIR, ModalSemanticAnalyzer};
use crate::semantic_roles::{SemanticRoleAnalyzer, SemanticRoleGraphIR};

pub const COMPOSITIONAL_ANALYSIS_SCHEMA: &str = "B_CORE_COMPOSITIONAL_ANALYSIS_IR_4";
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
    pub attribution_graph: AttributionGraphIR,
    #[serde(default)]
    pub semantic_role_graph: SemanticRoleGraphIR,
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
            "조사",
            "분석",
            "검토",
            "진단",
            "비교",
            "찾아",
            "inspect",
            "investigate",
            "analyze",
            "check",
            "review",
            "reviewed",
            "examine",
            "examined",
            "diagnose",
            "diagnosed",
            "compare",
            "compared",
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
        canonical: "EXECUTE",
        intent: PlanIntentIR::Execute,
        forms: &[
            "실행",
            "열어",
            "읽",
            "변환",
            "저장",
            "삭제",
            "배포",
            "지우",
            "지웠",
            "옮겨",
            "run",
            "open",
            "read",
            "transform",
            "convert",
            "save",
            "delete",
            "deleted",
            "deleting",
            "deploy",
            "deployed",
            "clear",
            "cleared",
            "move",
            "moved",
        ],
    },
    ActionFamily {
        canonical: "CONTINUE",
        intent: PlanIntentIR::Execute,
        forms: &["계속", "진행", "이어가", "continue", "proceed", "resume"],
    },
    ActionFamily {
        canonical: "COMMUNICATE",
        intent: PlanIntentIR::Communicate,
        forms: &[
            "기록", "보고", "전달", "보내", "말해", "record", "recorded", "report", "send", "sent",
            "tell", "notify",
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
                let counterfactual = is_counterfactual(&clause.text);
                let conditional = is_conditional(&clause.text, &occurrence);
                let interrogative = global_question || is_question(&clause.text);
                let mood = if reported {
                    FrameMoodIR::Reported
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
                        FrameMoodIR::Counterfactual => FrameModalityIR::Counterfactual,
                        FrameMoodIR::Conditional => FrameModalityIR::Hypothetical,
                        FrameMoodIR::Interrogative => FrameModalityIR::Possible,
                        FrameMoodIR::Imperative => FrameModalityIR::Requested,
                        FrameMoodIR::Declarative => detect_asserted_modality(&clause.text),
                    }
                };
                let frame_id = format!("FRAME-{:02}", frames.len() + 1);
                let theme = extract_theme(&clause.text, &occurrence);
                let authorized = mood == FrameMoodIR::Imperative && !negated && !reported;
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
        let attribution_graph = AttributionAnalyzer.analyze(&normalized, &frames);
        let semantic_role_graph = SemanticRoleAnalyzer.analyze(&normalized, &frames);
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
        let goal_graph = build_goal_graph(&normalized, &frames, &candidates);
        let mut unresolved_competitions = Vec::new();
        let modal_goal_ambiguity = !modal_scope_graph.unresolved_ambiguities.is_empty()
            && !viable.is_empty()
            && !modal_scope_graph.is_polite_request();
        if modal_goal_ambiguity {
            unresolved_competitions.extend(
                modal_scope_graph
                    .unresolved_ambiguities
                    .iter()
                    .map(|item| format!("MODAL:{item}")),
            );
        }
        let candidate_competition = goal_graph.is_none()
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
            attribution_graph,
            semantic_role_graph,
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

fn build_goal_graph(
    text: &str,
    frames: &[PredicateFrameIR],
    candidates: &[InterpretationCandidateIR],
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
        let relation = coordination_relation(evidence)?;
        edges.push(CompositionalGoalEdgeIR {
            source_node_id: format!("GOAL-NODE-{:02}", index + 1),
            target_node_id: format!("GOAL-NODE-{:02}", index + 2),
            relation,
            evidence_surface: evidence.trim().to_string(),
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
    let mut conditions = extract_graph_conditions(text);
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
    if [" and ", "그리고", "및 ", "와 ", "과 "]
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
    } else if let Some(argument) = semantic_role_graph.primary_argument_for_frame(&frame.frame_id) {
        argument.normalized_label.clone()
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
            for (position, _) in text.match_indices(form) {
                if valid_form_boundary(text, position, form)
                    && !ascii_nominal_context(text, position, form)
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
    occurrences.sort_by(|left, right| {
        left.local_start
            .cmp(&right.local_start)
            .then_with(|| right.form.len().cmp(&left.form.len()))
    });
    let mut seen = BTreeSet::new();
    occurrences.retain(|occurrence| {
        seen.insert((
            occurrence.local_start,
            occurrence.canonical_predicate.clone(),
        ))
    });
    occurrences
}

fn ascii_nominal_context(text: &str, start: usize, form: &str) -> bool {
    if !form
        .chars()
        .all(|character| character.is_ascii_alphabetic())
    {
        return false;
    }
    let prior = text[..start]
        .split(|character: char| !character.is_ascii_alphanumeric() && character != '_')
        .rfind(|token| !token.is_empty())
        .unwrap_or_default();
    let after = text[start + form.len()..].trim_start();
    let bounded_nominal = matches!(form, "report" | "record" | "document")
        && !text[..start]
            .rsplit(['.', '?', '!', ';', '\n', '\r'])
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
                | "they"
                | "he"
                | "she"
                | "user"
                | "system"
        );
    bounded_nominal
        || matches!(
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
        )
        || matches!(
            after.split_whitespace().next().unwrap_or_default(),
            "is" | "are" | "was" | "were" | "has" | "had"
        )
}

fn valid_form_boundary(text: &str, start: usize, form: &str) -> bool {
    if !form
        .chars()
        .all(|character| character.is_ascii_alphabetic())
    {
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
        return [
            "하", "해", "했", "한", "할", "되", "시", "고", "지", "면", "어", "아", "여", "줘",
            "세요", "자", "며", "면서", "던", "더", "기", "으",
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
    if occurrence
        .form
        .chars()
        .all(|character| character.is_ascii_alphabetic())
    {
        let prefix = &text[..occurrence.local_start];
        let contrast_directive = [", but now ", ", but ", ", however ", "; now "]
            .iter()
            .any(|marker| prefix.ends_with(marker));
        let prefixed_directive = trimmed
            .strip_prefix("please ")
            .or_else(|| trimmed.strip_prefix("just "))
            .is_some_and(|tail| tail.starts_with(occurrence.form.as_str()));
        return trimmed.starts_with(occurrence.form.as_str())
            || prefixed_directive
            || contrast_directive
            || trimmed.contains(&format!(" and {}", occurrence.form))
            || trimmed.contains(&format!(", {}", occurrence.form))
            || trimmed.contains(&format!(" then {}", occurrence.form));
    }
    let tail = &text[occurrence.local_start + occurrence.form.len()..];
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
    ]
    .iter()
    .any(|ending| trimmed.ends_with(ending))
        || trimmed.contains(" 해줘")
        || trimmed.contains(" 말고 ");
    if !clause_is_directive {
        return false;
    }
    tail.is_empty()
        || [
            "하", "해", "했", "한", "할", "고", "지", "면", "어", "아", "여", "줘", "세요", "자",
            "며", "면서",
        ]
        .iter()
        .any(|suffix| tail.starts_with(suffix))
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
    if occurrence
        .form
        .chars()
        .all(|character| character.is_ascii_alphabetic())
    {
        return extract_english_theme(text, occurrence);
    }
    let before = text[..occurrence.local_start].trim();
    let corrected = if let Some(position) = before.rfind("말고") {
        &before[position + "말고".len()..]
    } else {
        before
    };
    let token = corrected
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
        theme.to_string()
    }
}

fn extract_english_theme(text: &str, occurrence: &ActionOccurrence) -> String {
    let tail = text[occurrence.local_start + occurrence.form.len()..].trim();
    let tail = tail.split([',', ';']).next().unwrap_or(tail).trim();
    let tail = tail.split(" not ").next().unwrap_or(tail).trim();
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
}
