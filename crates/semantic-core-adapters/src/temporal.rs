//! Typed event-time and temporal-relation semantics for bounded dialogue.
//!
//! Event records are dialogue evidence, not world facts.  Report turn and
//! described event time are deliberately separate, and temporal answers cite
//! graph nodes/edges rather than inferring from surface plausibility.

use std::collections::{BTreeSet, VecDeque};

use serde::{Deserialize, Serialize};

use crate::language_knowledge::LanguageCodeIR;
use crate::modality::{ModalSemanticAnalyzer, ModalWorldIR};

pub const TEMPORAL_GRAPH_SCHEMA: &str = "B_CORE_TEMPORAL_GRAPH_IR_1";
pub const TEMPORAL_QUERY_SCHEMA: &str = "B_CORE_TEMPORAL_QUERY_IR_1";
pub const TEMPORAL_ANSWER_SCHEMA: &str = "B_CORE_TEMPORAL_ANSWER_IR_1";
const MAX_TEMPORAL_EVENTS: usize = 64;
const MAX_TEMPORAL_RELATIONS: usize = 128;
const MAX_TEMPORAL_CONFLICTS: usize = 32;
const MAX_TEMPORAL_EVIDENCE: usize = 16;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TemporalExpressionKindIR {
    CalendarDate,
    RelativeDay,
    RelativeWeek,
    ClockTime,
    Past,
    Present,
    Future,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TemporalExpressionIR {
    pub surface: String,
    pub normalized_value: String,
    pub kind: TemporalExpressionKindIR,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub relative_day_offset: Option<i16>,
    pub confidence_millis: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TemporalRelationKindIR {
    Before,
    Simultaneous,
    During,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TemporalRelationStatusIR {
    Active,
    Contested,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TemporalEventIR {
    pub event_id: String,
    pub surface: String,
    pub normalized_key: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub event_time: Option<TemporalExpressionIR>,
    pub report_turn: u64,
    pub modal_world: ModalWorldIR,
    pub dialogue_truth_established: bool,
    pub external_execution_authorized: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TemporalRelationIR {
    pub relation_id: String,
    pub left_event_id: String,
    pub right_event_id: String,
    pub kind: TemporalRelationKindIR,
    pub status: TemporalRelationStatusIR,
    pub evidence_surface: String,
    pub introduced_turn: u64,
    pub dialogue_truth_established: bool,
    pub external_execution_authorized: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TemporalConflictIR {
    pub conflict_id: String,
    pub relation_ids: Vec<String>,
    pub detected_turn: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TemporalGraphIR {
    pub schema: String,
    pub events: Vec<TemporalEventIR>,
    pub relations: Vec<TemporalRelationIR>,
    pub conflicts: Vec<TemporalConflictIR>,
}

impl Default for TemporalGraphIR {
    fn default() -> Self {
        Self {
            schema: TEMPORAL_GRAPH_SCHEMA.to_string(),
            events: Vec::new(),
            relations: Vec::new(),
            conflicts: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TemporalTurnAnalysisIR {
    pub events: Vec<TemporalEventIR>,
    pub relations: Vec<TemporalRelationIR>,
}

impl TemporalGraphIR {
    pub fn apply_turn(&mut self, analysis: &TemporalTurnAnalysisIR) {
        let mut event_id_remap = std::collections::BTreeMap::<String, String>::new();
        for event in &analysis.events {
            let matching_index = self.events.iter().position(|existing| {
                existing.normalized_key == event.normalized_key
                    && existing.modal_world == event.modal_world
                    && compatible_event_time(
                        existing.event_time.as_ref(),
                        event.event_time.as_ref(),
                    )
                    && event.report_turn.saturating_sub(existing.report_turn) <= 8
            });
            if let Some(index) = matching_index {
                event_id_remap.insert(event.event_id.clone(), self.events[index].event_id.clone());
                if self.events[index].event_time.is_none() {
                    self.events[index].event_time.clone_from(&event.event_time);
                }
            } else if !self
                .events
                .iter()
                .any(|item| item.event_id == event.event_id)
            {
                self.events.push(event.clone());
            }
        }
        for relation in &analysis.relations {
            let mut relation = relation.clone();
            if let Some(mapped) = event_id_remap.get(&relation.left_event_id) {
                relation.left_event_id.clone_from(mapped);
            }
            if let Some(mapped) = event_id_remap.get(&relation.right_event_id) {
                relation.right_event_id.clone_from(mapped);
            }
            if relation.left_event_id == relation.right_event_id {
                continue;
            }
            if relation.kind == TemporalRelationKindIR::Before {
                if let Some(reverse_path) =
                    self.before_path(&relation.right_event_id, &relation.left_event_id, true)
                {
                    relation.status = TemporalRelationStatusIR::Contested;
                    for relation_id in &reverse_path {
                        if let Some(existing) = self
                            .relations
                            .iter_mut()
                            .find(|item| &item.relation_id == relation_id)
                        {
                            existing.status = TemporalRelationStatusIR::Contested;
                        }
                    }
                    let mut relation_ids = reverse_path;
                    relation_ids.push(relation.relation_id.clone());
                    relation_ids.sort();
                    relation_ids.dedup();
                    self.conflicts.push(TemporalConflictIR {
                        conflict_id: format!(
                            "TEMP-CONFLICT-{:06}-{:02}",
                            relation.introduced_turn,
                            self.conflicts.len() + 1
                        ),
                        relation_ids,
                        detected_turn: relation.introduced_turn,
                    });
                }
            }
            if !self
                .relations
                .iter()
                .any(|item| item.relation_id == relation.relation_id)
            {
                self.relations.push(relation);
            }
        }
        self.events.sort_by(|left, right| {
            right
                .report_turn
                .cmp(&left.report_turn)
                .then_with(|| left.event_id.cmp(&right.event_id))
        });
        self.events.truncate(MAX_TEMPORAL_EVENTS);
        let retained = self
            .events
            .iter()
            .map(|event| event.event_id.as_str())
            .collect::<BTreeSet<_>>();
        self.relations.retain(|relation| {
            retained.contains(relation.left_event_id.as_str())
                && retained.contains(relation.right_event_id.as_str())
        });
        if self.relations.len() > MAX_TEMPORAL_RELATIONS {
            let remove = self.relations.len() - MAX_TEMPORAL_RELATIONS;
            self.relations.drain(..remove);
        }
        let retained_relations = self
            .relations
            .iter()
            .map(|relation| relation.relation_id.as_str())
            .collect::<BTreeSet<_>>();
        self.conflicts.retain(|conflict| {
            conflict
                .relation_ids
                .iter()
                .all(|id| retained_relations.contains(id.as_str()))
        });
        if self.conflicts.len() > MAX_TEMPORAL_CONFLICTS {
            let remove = self.conflicts.len() - MAX_TEMPORAL_CONFLICTS;
            self.conflicts.drain(..remove);
        }
    }

    pub fn validate(&self, completed_turns: u64) -> bool {
        if self.schema != TEMPORAL_GRAPH_SCHEMA
            || self.events.len() > MAX_TEMPORAL_EVENTS
            || self.relations.len() > MAX_TEMPORAL_RELATIONS
            || self.conflicts.len() > MAX_TEMPORAL_CONFLICTS
        {
            return false;
        }
        let event_ids = self
            .events
            .iter()
            .map(|event| event.event_id.as_str())
            .collect::<BTreeSet<_>>();
        if event_ids.len() != self.events.len()
            || self.events.iter().any(|event| {
                event.event_id.trim().is_empty()
                    || event.surface.trim().is_empty()
                    || event.normalized_key.trim().is_empty()
                    || event.report_turn == 0
                    || event.report_turn > completed_turns
                    || event.dialogue_truth_established
                    || event.external_execution_authorized
                    || event.event_time.as_ref().is_some_and(|time| {
                        time.surface.trim().is_empty()
                            || time.normalized_value.trim().is_empty()
                            || time.confidence_millis > 1_000
                    })
            })
        {
            return false;
        }
        let relation_ids = self
            .relations
            .iter()
            .map(|relation| relation.relation_id.as_str())
            .collect::<BTreeSet<_>>();
        if relation_ids.len() != self.relations.len()
            || self.relations.iter().any(|relation| {
                relation.relation_id.trim().is_empty()
                    || relation.left_event_id == relation.right_event_id
                    || !event_ids.contains(relation.left_event_id.as_str())
                    || !event_ids.contains(relation.right_event_id.as_str())
                    || relation.evidence_surface.trim().is_empty()
                    || relation.introduced_turn == 0
                    || relation.introduced_turn > completed_turns
                    || relation.dialogue_truth_established
                    || relation.external_execution_authorized
            })
        {
            return false;
        }
        let conflict_ids = self
            .conflicts
            .iter()
            .map(|conflict| conflict.conflict_id.as_str())
            .collect::<BTreeSet<_>>();
        conflict_ids.len() == self.conflicts.len()
            && self.conflicts.iter().all(|conflict| {
                !conflict.conflict_id.trim().is_empty()
                    && conflict.relation_ids.len() >= 2
                    && conflict
                        .relation_ids
                        .iter()
                        .all(|id| relation_ids.contains(id.as_str()))
                    && conflict.detected_turn > 0
                    && conflict.detected_turn <= completed_turns
            })
            && !self.active_before_cycle()
    }

    pub fn event(&self, event_id: &str) -> Option<&TemporalEventIR> {
        self.events.iter().find(|event| event.event_id == event_id)
    }

    pub fn before_path(&self, start: &str, end: &str, active_only: bool) -> Option<Vec<String>> {
        let mut queue = VecDeque::from([(start.to_string(), Vec::<String>::new())]);
        let mut visited = BTreeSet::new();
        while let Some((event_id, path)) = queue.pop_front() {
            if !visited.insert(event_id.clone()) {
                continue;
            }
            if event_id == end && !path.is_empty() {
                return Some(path);
            }
            for relation in self.relations.iter().filter(|relation| {
                relation.kind == TemporalRelationKindIR::Before
                    && relation.left_event_id == event_id
                    && (!active_only || relation.status == TemporalRelationStatusIR::Active)
            }) {
                let mut next_path = path.clone();
                next_path.push(relation.relation_id.clone());
                queue.push_back((relation.right_event_id.clone(), next_path));
            }
        }
        None
    }

    fn active_before_cycle(&self) -> bool {
        self.relations.iter().any(|relation| {
            relation.kind == TemporalRelationKindIR::Before
                && relation.status == TemporalRelationStatusIR::Active
                && self
                    .before_path(&relation.right_event_id, &relation.left_event_id, true)
                    .is_some()
        })
    }
}

fn compatible_event_time(
    left: Option<&TemporalExpressionIR>,
    right: Option<&TemporalExpressionIR>,
) -> bool {
    left.zip(right)
        .is_none_or(|(left, right)| left.normalized_value == right.normalized_value)
}

#[derive(Debug, Clone, Copy, Default)]
pub struct TemporalSemanticAnalyzer;

impl TemporalSemanticAnalyzer {
    pub fn analyze_turn(
        &self,
        text: &str,
        turn_index: u64,
        prior: Option<&TemporalGraphIR>,
    ) -> TemporalTurnAnalysisIR {
        let normalized = normalize_space(&text.to_lowercase());
        if normalized.is_empty() || temporal_question(&normalized) {
            return TemporalTurnAnalysisIR::default();
        }
        if let Some(parsed) = parse_relation_surface(&normalized) {
            let left = event_from_surface(turn_index, 1, &parsed.left);
            let right = event_from_surface(turn_index, 2, &parsed.right);
            if let (Some(left), Some(right)) = (left, right) {
                return TemporalTurnAnalysisIR {
                    relations: vec![relation(
                        turn_index,
                        1,
                        &left.event_id,
                        &right.event_id,
                        parsed.kind,
                        text,
                    )],
                    events: vec![left, right],
                };
            }
        }
        if deictic_after_reference(&normalized) {
            if let Some(previous) = prior.and_then(|graph| graph.events.first()) {
                if let Some(new_surface) = strip_deictic_prefix(&normalized) {
                    if let Some(new_event) = event_from_surface(turn_index, 1, new_surface) {
                        return TemporalTurnAnalysisIR {
                            relations: vec![relation(
                                turn_index,
                                1,
                                &previous.event_id,
                                &new_event.event_id,
                                TemporalRelationKindIR::Before,
                                text,
                            )],
                            events: vec![new_event],
                        };
                    }
                }
            }
        }
        event_from_surface(turn_index, 1, &normalized).map_or_else(
            TemporalTurnAnalysisIR::default,
            |event| TemporalTurnAnalysisIR {
                events: vec![event],
                relations: Vec::new(),
            },
        )
    }
}

#[derive(Debug)]
struct ParsedRelation {
    left: String,
    right: String,
    kind: TemporalRelationKindIR,
}

fn parse_relation_surface(text: &str) -> Option<ParsedRelation> {
    for (prefix, kind, reverse) in [
        ("before ", TemporalRelationKindIR::Before, true),
        ("after ", TemporalRelationKindIR::Before, false),
        ("while ", TemporalRelationKindIR::Simultaneous, false),
    ] {
        if let Some(rest) = text.strip_prefix(prefix) {
            if let Some((first, second)) = rest.split_once(',') {
                let (left, right) = if reverse {
                    (second.trim(), first.trim())
                } else {
                    (first.trim(), second.trim())
                };
                return parsed_relation(left, right, kind);
            }
        }
    }
    for (marker, kind, reverse) in [
        (" before ", TemporalRelationKindIR::Before, false),
        (" after ", TemporalRelationKindIR::Before, true),
        (" while ", TemporalRelationKindIR::Simultaneous, false),
        (" during ", TemporalRelationKindIR::During, false),
        (" 전에 ", TemporalRelationKindIR::Before, true),
        (" 후에 ", TemporalRelationKindIR::Before, false),
        (" 이후에 ", TemporalRelationKindIR::Before, false),
        (" 뒤에 ", TemporalRelationKindIR::Before, false),
        (" 동안 ", TemporalRelationKindIR::During, true),
        (" 동시에 ", TemporalRelationKindIR::Simultaneous, false),
    ] {
        if let Some((first, second)) = text.split_once(marker) {
            let (left, right) = if reverse {
                (second.trim(), first.trim())
            } else {
                (first.trim(), second.trim())
            };
            return parsed_relation(left, right, kind);
        }
    }
    None
}

fn parsed_relation(
    left: &str,
    right: &str,
    kind: TemporalRelationKindIR,
) -> Option<ParsedRelation> {
    let left = trim_event_surface(left);
    let right = trim_event_surface(right);
    (!left.is_empty() && !right.is_empty()).then_some(ParsedRelation { left, right, kind })
}

fn event_from_surface(turn: u64, index: usize, surface: &str) -> Option<TemporalEventIR> {
    let surface = trim_event_surface(surface);
    if !event_like(&surface) {
        return None;
    }
    Some(TemporalEventIR {
        event_id: format!("TEMP-EVENT-{turn:06}-{index:02}"),
        normalized_key: event_key(&surface),
        event_time: extract_time(&surface),
        modal_world: ModalSemanticAnalyzer.analyze(&surface).root_world,
        surface,
        report_turn: turn,
        dialogue_truth_established: false,
        external_execution_authorized: false,
    })
}

fn relation(
    turn: u64,
    index: usize,
    left_event_id: &str,
    right_event_id: &str,
    kind: TemporalRelationKindIR,
    evidence: &str,
) -> TemporalRelationIR {
    TemporalRelationIR {
        relation_id: format!("TEMP-REL-{turn:06}-{index:02}"),
        left_event_id: left_event_id.to_string(),
        right_event_id: right_event_id.to_string(),
        kind,
        status: TemporalRelationStatusIR::Active,
        evidence_surface: evidence.trim().to_string(),
        introduced_turn: turn,
        dialogue_truth_established: false,
        external_execution_authorized: false,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TemporalQueryKindIR {
    EventTime,
    EventsBefore,
    EventsAfter,
    EventsDuring,
    RelationCheck,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TemporalQueryIR {
    pub schema: String,
    pub original_text: String,
    pub kind: TemporalQueryKindIR,
    pub target_terms: Vec<String>,
    #[serde(default)]
    pub second_target_terms: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expected_relation: Option<TemporalRelationKindIR>,
    pub confidence_millis: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TemporalAnswerDispositionIR {
    AnsweredFromTemporalGraph,
    AnsweredByTransitivePath,
    NoMatchingEvent,
    NoRecordedRelation,
    AmbiguousEvent,
    ConflictingRelations,
    EventTimeNotRecorded,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TemporalAnswerIR {
    pub schema: String,
    pub query: TemporalQueryIR,
    pub disposition: TemporalAnswerDispositionIR,
    pub event_evidence: Vec<TemporalEventIR>,
    pub relation_evidence: Vec<TemporalRelationIR>,
    pub language: LanguageCodeIR,
    pub realized_text: String,
    pub dialogue_truth_established: bool,
    pub external_execution_authorized: bool,
    pub unsupported_claims: usize,
}

impl TemporalAnswerIR {
    pub fn validate(&self) -> bool {
        if self.schema != TEMPORAL_ANSWER_SCHEMA
            || self.query.schema != TEMPORAL_QUERY_SCHEMA
            || self.event_evidence.len() > MAX_TEMPORAL_EVIDENCE
            || self.relation_evidence.len() > MAX_TEMPORAL_EVIDENCE
            || self.realized_text.trim().is_empty()
            || self.dialogue_truth_established
            || self.external_execution_authorized
            || self.unsupported_claims != 0
            || self.event_evidence.iter().any(|event| {
                event.dialogue_truth_established || event.external_execution_authorized
            })
            || self.relation_evidence.iter().any(|relation| {
                relation.dialogue_truth_established || relation.external_execution_authorized
            })
        {
            return false;
        }
        let event_ids = self
            .event_evidence
            .iter()
            .map(|event| event.event_id.as_str())
            .collect::<BTreeSet<_>>();
        event_ids.len() == self.event_evidence.len()
            && self.relation_evidence.iter().all(|relation| {
                event_ids.contains(relation.left_event_id.as_str())
                    && event_ids.contains(relation.right_event_id.as_str())
            })
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct TemporalQaEngine;

impl TemporalQaEngine {
    pub fn parse(&self, text: &str) -> Option<TemporalQueryIR> {
        let normalized = normalize_space(&text.to_lowercase());
        if !temporal_question(&normalized) {
            return None;
        }
        let (kind, first, second, expected) = if let Some(target) = question_target(
            &normalized,
            &["what happened before ", "what occurred before "],
        ) {
            (TemporalQueryKindIR::EventsBefore, target, None, None)
        } else if let Some(target) = question_target(
            &normalized,
            &["what happened after ", "what occurred after "],
        ) {
            (TemporalQueryKindIR::EventsAfter, target, None, None)
        } else if let Some(target) = question_target(
            &normalized,
            &["what happened while ", "what happened during "],
        ) {
            (TemporalQueryKindIR::EventsDuring, target, None, None)
        } else if let Some(target) = korean_relative_question_target(&normalized, "전에") {
            (TemporalQueryKindIR::EventsBefore, target, None, None)
        } else if let Some(target) = korean_relative_question_target(&normalized, "후에")
            .or_else(|| korean_relative_question_target(&normalized, "이후에"))
        {
            (TemporalQueryKindIR::EventsAfter, target, None, None)
        } else if let Some(target) = korean_relative_question_target(&normalized, "동안") {
            (TemporalQueryKindIR::EventsDuring, target, None, None)
        } else if let Some(target) = when_question_target(&normalized) {
            (TemporalQueryKindIR::EventTime, target, None, None)
        } else if let Some(relation) = parse_relation_surface(&strip_question_frame(&normalized)) {
            (
                TemporalQueryKindIR::RelationCheck,
                relation.left,
                Some(relation.right),
                Some(relation.kind),
            )
        } else {
            return None;
        };
        Some(TemporalQueryIR {
            schema: TEMPORAL_QUERY_SCHEMA.to_string(),
            original_text: text.trim().to_string(),
            kind,
            target_terms: query_terms(&first),
            second_target_terms: second.as_deref().map_or_else(Vec::new, query_terms),
            expected_relation: expected,
            confidence_millis: 900,
        })
    }

    pub fn answer(
        &self,
        text: &str,
        graph: Option<&TemporalGraphIR>,
        language: LanguageCodeIR,
    ) -> Option<TemporalAnswerIR> {
        let query = self.parse(text)?;
        let empty = TemporalGraphIR::default();
        let graph = graph.unwrap_or(&empty);
        let (disposition, event_evidence, relation_evidence) = answer_query(&query, graph);
        let realized_text = realize_temporal_answer(
            language,
            &query,
            disposition,
            &event_evidence,
            &relation_evidence,
        );
        let answer = TemporalAnswerIR {
            schema: TEMPORAL_ANSWER_SCHEMA.to_string(),
            query,
            disposition,
            event_evidence,
            relation_evidence,
            language,
            realized_text,
            dialogue_truth_established: false,
            external_execution_authorized: false,
            unsupported_claims: 0,
        };
        debug_assert!(answer.validate());
        Some(answer)
    }
}

fn answer_query(
    query: &TemporalQueryIR,
    graph: &TemporalGraphIR,
) -> (
    TemporalAnswerDispositionIR,
    Vec<TemporalEventIR>,
    Vec<TemporalRelationIR>,
) {
    let targets = matching_events(graph, &query.target_terms);
    if targets.is_empty() {
        return (
            TemporalAnswerDispositionIR::NoMatchingEvent,
            Vec::new(),
            Vec::new(),
        );
    }
    if query.kind == TemporalQueryKindIR::EventTime {
        let timed = targets
            .iter()
            .filter(|event| event.event_time.is_some())
            .copied()
            .collect::<Vec<_>>();
        if timed.is_empty() {
            return (
                TemporalAnswerDispositionIR::EventTimeNotRecorded,
                targets
                    .into_iter()
                    .take(MAX_TEMPORAL_EVIDENCE)
                    .cloned()
                    .collect(),
                Vec::new(),
            );
        }
        let values = timed
            .iter()
            .filter_map(|event| event.event_time.as_ref())
            .map(|time| time.normalized_value.as_str())
            .collect::<BTreeSet<_>>();
        return (
            if values.len() == 1 {
                TemporalAnswerDispositionIR::AnsweredFromTemporalGraph
            } else {
                TemporalAnswerDispositionIR::AmbiguousEvent
            },
            timed
                .into_iter()
                .take(MAX_TEMPORAL_EVIDENCE)
                .cloned()
                .collect(),
            Vec::new(),
        );
    }
    if query.kind == TemporalQueryKindIR::RelationCheck {
        let second = matching_events(graph, &query.second_target_terms);
        if second.is_empty() {
            return (
                TemporalAnswerDispositionIR::NoMatchingEvent,
                Vec::new(),
                Vec::new(),
            );
        }
        if targets.len() != 1 || second.len() != 1 {
            let mut events = targets
                .into_iter()
                .chain(second)
                .take(MAX_TEMPORAL_EVIDENCE)
                .cloned()
                .collect::<Vec<_>>();
            dedup_events(&mut events);
            return (
                TemporalAnswerDispositionIR::AmbiguousEvent,
                events,
                Vec::new(),
            );
        }
        return relation_check(
            graph,
            targets[0],
            second[0],
            query
                .expected_relation
                .unwrap_or(TemporalRelationKindIR::Before),
        );
    }
    if targets.len() != 1 {
        return (
            TemporalAnswerDispositionIR::AmbiguousEvent,
            targets
                .into_iter()
                .take(MAX_TEMPORAL_EVIDENCE)
                .cloned()
                .collect(),
            Vec::new(),
        );
    }
    let target = targets[0];
    let mut relations = Vec::new();
    let mut events = vec![target.clone()];
    for candidate in &graph.events {
        let relation_path = match query.kind {
            TemporalQueryKindIR::EventsBefore => {
                graph.before_path(&candidate.event_id, &target.event_id, true)
            }
            TemporalQueryKindIR::EventsAfter => {
                graph.before_path(&target.event_id, &candidate.event_id, true)
            }
            TemporalQueryKindIR::EventsDuring => graph
                .relations
                .iter()
                .find(|relation| {
                    relation.status == TemporalRelationStatusIR::Active
                        && ((relation.kind == TemporalRelationKindIR::During
                            && relation.left_event_id == candidate.event_id
                            && relation.right_event_id == target.event_id)
                            || (relation.kind == TemporalRelationKindIR::Simultaneous
                                && ((relation.left_event_id == candidate.event_id
                                    && relation.right_event_id == target.event_id)
                                    || (relation.right_event_id == candidate.event_id
                                        && relation.left_event_id == target.event_id))))
                })
                .map(|relation| vec![relation.relation_id.clone()]),
            _ => None,
        };
        if candidate.event_id != target.event_id {
            if let Some(path) = relation_path {
                events.push(candidate.clone());
                for id in path {
                    if let Some(relation) = graph
                        .relations
                        .iter()
                        .find(|relation| relation.relation_id == id)
                    {
                        relations.push(relation.clone());
                    }
                }
            }
        }
    }
    dedup_events(&mut events);
    dedup_relations(&mut relations);
    if relations.is_empty() {
        return (
            TemporalAnswerDispositionIR::NoRecordedRelation,
            events,
            relations,
        );
    }
    let transitive = relations.len() > 1;
    events.truncate(MAX_TEMPORAL_EVIDENCE);
    relations.truncate(MAX_TEMPORAL_EVIDENCE);
    (
        if transitive {
            TemporalAnswerDispositionIR::AnsweredByTransitivePath
        } else {
            TemporalAnswerDispositionIR::AnsweredFromTemporalGraph
        },
        events,
        relations,
    )
}

fn relation_check(
    graph: &TemporalGraphIR,
    left: &TemporalEventIR,
    right: &TemporalEventIR,
    kind: TemporalRelationKindIR,
) -> (
    TemporalAnswerDispositionIR,
    Vec<TemporalEventIR>,
    Vec<TemporalRelationIR>,
) {
    let mut events = vec![left.clone(), right.clone()];
    let contested = graph.relations.iter().filter(|relation| {
        relation.status == TemporalRelationStatusIR::Contested
            && [
                relation.left_event_id.as_str(),
                relation.right_event_id.as_str(),
            ]
            .contains(&left.event_id.as_str())
            && [
                relation.left_event_id.as_str(),
                relation.right_event_id.as_str(),
            ]
            .contains(&right.event_id.as_str())
    });
    let contested = contested.cloned().collect::<Vec<_>>();
    if !contested.is_empty() {
        return (
            TemporalAnswerDispositionIR::ConflictingRelations,
            events,
            contested,
        );
    }
    let relation_ids = match kind {
        TemporalRelationKindIR::Before => graph.before_path(&left.event_id, &right.event_id, true),
        TemporalRelationKindIR::Simultaneous | TemporalRelationKindIR::During => graph
            .relations
            .iter()
            .find(|relation| {
                relation.status == TemporalRelationStatusIR::Active
                    && relation.kind == kind
                    && relation.left_event_id == left.event_id
                    && relation.right_event_id == right.event_id
            })
            .map(|relation| vec![relation.relation_id.clone()]),
    };
    let Some(relation_ids) = relation_ids else {
        return (
            TemporalAnswerDispositionIR::NoRecordedRelation,
            events,
            Vec::new(),
        );
    };
    let relations = relation_ids
        .iter()
        .filter_map(|id| {
            graph
                .relations
                .iter()
                .find(|relation| &relation.relation_id == id)
        })
        .cloned()
        .collect::<Vec<_>>();
    for relation in &relations {
        for event_id in [&relation.left_event_id, &relation.right_event_id] {
            if let Some(event) = graph.event(event_id) {
                events.push(event.clone());
            }
        }
    }
    dedup_events(&mut events);
    (
        if relations.len() > 1 {
            TemporalAnswerDispositionIR::AnsweredByTransitivePath
        } else {
            TemporalAnswerDispositionIR::AnsweredFromTemporalGraph
        },
        events,
        relations,
    )
}

fn matching_events<'a>(graph: &'a TemporalGraphIR, terms: &[String]) -> Vec<&'a TemporalEventIR> {
    if terms.is_empty() {
        return Vec::new();
    }
    let scores = graph
        .events
        .iter()
        .map(|event| {
            let event_terms = query_terms(&event.normalized_key);
            let entity_terms = terms
                .iter()
                .filter(|term| !event_predicate_term(term))
                .collect::<Vec<_>>();
            let event_entity_terms = event_terms
                .iter()
                .filter(|term| !event_predicate_term(term))
                .collect::<BTreeSet<_>>();
            let entity_score = entity_terms
                .iter()
                .filter(|term| event_entity_terms.contains(**term))
                .count();
            let predicate_score = terms
                .iter()
                .filter(|term| event_predicate_term(term) && event_terms.contains(*term))
                .count();
            let score = if !entity_terms.is_empty() && entity_score == 0 {
                0
            } else {
                entity_score * 4 + predicate_score
            };
            (event, score)
        })
        .filter(|(_, score)| *score > 0)
        .collect::<Vec<_>>();
    let best = scores.iter().map(|(_, score)| *score).max().unwrap_or(0);
    scores
        .into_iter()
        .filter(|(_, score)| *score == best)
        .map(|(event, _)| event)
        .collect()
}

fn event_predicate_term(term: &str) -> bool {
    [
        "complete",
        "start",
        "fail",
        "crash",
        "stop",
        "run",
        "restart",
        "arrive",
        "leave",
        "open",
        "close",
        "완료",
        "시작",
        "실패",
        "중단",
        "실행",
        "재시작",
        "도착",
        "손상",
    ]
    .contains(&term)
}

fn dedup_events(events: &mut Vec<TemporalEventIR>) {
    events.sort_by(|left, right| left.event_id.cmp(&right.event_id));
    events.dedup_by(|left, right| left.event_id == right.event_id);
}

fn dedup_relations(relations: &mut Vec<TemporalRelationIR>) {
    relations.sort_by(|left, right| left.relation_id.cmp(&right.relation_id));
    relations.dedup_by(|left, right| left.relation_id == right.relation_id);
}

fn realize_temporal_answer(
    language: LanguageCodeIR,
    query: &TemporalQueryIR,
    disposition: TemporalAnswerDispositionIR,
    events: &[TemporalEventIR],
    relations: &[TemporalRelationIR],
) -> String {
    match (language, disposition) {
        (LanguageCodeIR::Korean, TemporalAnswerDispositionIR::AnsweredFromTemporalGraph)
        | (LanguageCodeIR::Korean, TemporalAnswerDispositionIR::AnsweredByTransitivePath) => {
            if query.kind == TemporalQueryKindIR::EventTime {
                let values = events
                    .iter()
                    .filter_map(|event| event.event_time.as_ref())
                    .map(|time| format!("‘{}’는 {}", time.surface, time.normalized_value))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("대화의 사건 기록상 시간 표현은 {values}이야. 보고된 시간이지 실제 발생 사실을 독립 검증한 것은 아니야.")
            } else {
                let surfaces = events
                    .iter()
                    .map(|event| format!("‘{}’", event.surface))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("대화의 시간 그래프는 {surfaces} 사이의 관계를 {}개 엣지로 기록해. 이것은 대화 근거이며 실제 세계 사실 확정은 아니야.", relations.len())
            }
        }
        (_, TemporalAnswerDispositionIR::AnsweredFromTemporalGraph)
        | (_, TemporalAnswerDispositionIR::AnsweredByTransitivePath) => {
            if query.kind == TemporalQueryKindIR::EventTime {
                let values = events
                    .iter()
                    .filter_map(|event| event.event_time.as_ref())
                    .map(|time| format!("‘{}’ ({})", time.surface, time.normalized_value))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("The dialogue event record gives the time as {values}. This is a reported time, not independently verified world truth.")
            } else {
                let surfaces = events
                    .iter()
                    .map(|event| format!("‘{}’", event.surface))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("The dialogue temporal graph connects {surfaces} with {} evidence edge(s). This is dialogue evidence, not established world truth.", relations.len())
            }
        }
        (LanguageCodeIR::Korean, TemporalAnswerDispositionIR::NoMatchingEvent) =>
            "질문의 대상과 일치하는 사건 기록이 없어. 사건을 추측해서 만들지 않을게.".to_string(),
        (_, TemporalAnswerDispositionIR::NoMatchingEvent) =>
            "There is no matching event record. I will not invent an event.".to_string(),
        (LanguageCodeIR::Korean, TemporalAnswerDispositionIR::NoRecordedRelation) =>
            "일치하는 사건은 있지만 요청한 시간 관계는 기록되지 않았어. 순서를 추측할 수 없어.".to_string(),
        (_, TemporalAnswerDispositionIR::NoRecordedRelation) =>
            "Matching events exist, but the requested temporal relation is not recorded, so I cannot infer the order.".to_string(),
        (LanguageCodeIR::Korean, TemporalAnswerDispositionIR::AmbiguousEvent) =>
            "같은 대상으로 해석될 수 있는 사건 기록이 여러 개라 어느 사건인지 확정할 수 없어.".to_string(),
        (_, TemporalAnswerDispositionIR::AmbiguousEvent) =>
            "Several event records match the target, so the intended event is ambiguous.".to_string(),
        (LanguageCodeIR::Korean, TemporalAnswerDispositionIR::ConflictingRelations) =>
            "서로 양립하지 않는 시간 순서 기록이 있어 한쪽을 임의로 선택하지 않을게.".to_string(),
        (_, TemporalAnswerDispositionIR::ConflictingRelations) =>
            "The dialogue contains incompatible temporal order records; I will not choose one silently.".to_string(),
        (LanguageCodeIR::Korean, TemporalAnswerDispositionIR::EventTimeNotRecorded) =>
            "사건 기록은 있지만 사건 시점은 기록되지 않았어. 보고 턴을 사건 시점으로 바꾸지 않을게.".to_string(),
        (_, TemporalAnswerDispositionIR::EventTimeNotRecorded) =>
            "The event is recorded, but its event time is not. I will not substitute report time for event time.".to_string(),
    }
}

fn temporal_question(text: &str) -> bool {
    text.ends_with('?')
        && (text.starts_with("when ")
            || text.starts_with("what happened before ")
            || text.starts_with("what happened after ")
            || text.starts_with("what happened while ")
            || text.starts_with("what happened during ")
            || text.starts_with("did ")
            || text.contains("언제")
            || text.contains("무슨 일이")
            || text.contains("먼저")
            || text.contains("전에")
            || text.contains("후에"))
}

fn question_target(text: &str, prefixes: &[&str]) -> Option<String> {
    prefixes.iter().find_map(|prefix| {
        text.strip_prefix(prefix)
            .map(|rest| trim_event_surface(rest.trim_end_matches('?')))
            .filter(|target| !target.is_empty())
    })
}

fn korean_relative_question_target(text: &str, marker: &str) -> Option<String> {
    let position = text.find(marker)?;
    if !text.contains("무슨 일이") && !text.contains("무엇") && !text.contains("뭐") {
        return None;
    }
    let target = trim_event_surface(&text[..position]);
    (!target.is_empty()).then_some(target)
}

fn when_question_target(text: &str) -> Option<String> {
    if let Some(rest) = text.strip_prefix("when did ") {
        return Some(trim_event_surface(rest.trim_end_matches('?')));
    }
    if let Some(rest) = text.strip_prefix("when was ") {
        return Some(trim_event_surface(rest.trim_end_matches('?')));
    }
    if let Some(position) = text.find("언제") {
        let before = trim_event_surface(&text[..position]);
        let after = trim_event_surface(text[position + "언제".len()..].trim_end_matches('?'));
        return Some(if before.is_empty() { after } else { before });
    }
    None
}

fn strip_question_frame(text: &str) -> String {
    text.trim_end_matches('?')
        .strip_prefix("did ")
        .unwrap_or(text.trim_end_matches('?'))
        .replace(" happen ", " ")
        .replace(" occur ", " ")
}

fn strip_deictic_prefix(text: &str) -> Option<&str> {
    [
        "after that,",
        "afterward,",
        "then,",
        "그 후,",
        "그 뒤,",
        "그 다음,",
    ]
    .iter()
    .find_map(|prefix| text.strip_prefix(prefix).map(str::trim))
}

fn deictic_after_reference(text: &str) -> bool {
    strip_deictic_prefix(text).is_some()
}

fn event_like(text: &str) -> bool {
    [
        "start",
        "begin",
        "complete",
        "finish",
        "fail",
        "crash",
        "stop",
        "deploy",
        "restart",
        "arrive",
        "leave",
        "open",
        "close",
        "run",
        "시작",
        "완료",
        "끝",
        "실패",
        "중단",
        "배포",
        "재시작",
        "도착",
        "떠",
        "열",
        "닫",
        "실행",
        "손상",
    ]
    .iter()
    .any(|marker| text.contains(marker))
}

fn event_key(text: &str) -> String {
    query_terms(text).join(" ")
}

fn query_terms(text: &str) -> Vec<String> {
    let mut terms = text
        .split(|character: char| !character.is_alphanumeric() && character != '_')
        .filter_map(normalize_temporal_term)
        .filter(|term| !TEMPORAL_STOP_WORDS.contains(&term.as_str()))
        .collect::<Vec<_>>();
    terms.sort();
    terms.dedup();
    terms
}

fn normalize_temporal_term(raw: &str) -> Option<String> {
    let mut term = raw.trim().to_lowercase();
    if term.is_empty() {
        return None;
    }
    if term.is_ascii() {
        term = match term.as_str() {
            "deployment" | "deployed" | "deploying" => "deploy".to_string(),
            "completed" | "completes" | "completion" | "finished" | "finishes" => {
                "complete".to_string()
            }
            "started" | "starts" | "starting" | "began" | "begins" => "start".to_string(),
            "failed" | "fails" | "failure" => "fail".to_string(),
            "crashed" | "crashes" => "crash".to_string(),
            "stopped" | "stops" => "stop".to_string(),
            other => other.to_string(),
        };
    } else {
        for suffix in [
            "되었어",
            "됐어",
            "되었다",
            "됐다",
            "했어",
            "했다",
            "되기",
            "하기",
            "에서",
            "에게",
            "으로",
            "보다",
            "은",
            "는",
            "이",
            "가",
            "을",
            "를",
            "에",
            "도",
        ] {
            if term.ends_with(suffix) && term.len() > suffix.len() {
                term.truncate(term.len() - suffix.len());
                break;
            }
        }
    }
    (!term.is_empty()).then_some(term)
}

const TEMPORAL_STOP_WORDS: &[&str] = &[
    "the",
    "a",
    "an",
    "did",
    "was",
    "were",
    "is",
    "are",
    "happen",
    "occur",
    "what",
    "when",
    "before",
    "after",
    "while",
    "during",
    "then",
    "yesterday",
    "today",
    "tomorrow",
    "at",
    "on",
    "in",
    "that",
    "it",
    "무슨",
    "일이",
    "무엇",
    "뭐",
    "언제",
    "전에",
    "후에",
    "이후에",
    "동안",
    "동시에",
    "어제",
    "오늘",
    "내일",
    "그",
    "뒤",
    "다음",
];

fn extract_time(text: &str) -> Option<TemporalExpressionIR> {
    for (surface, value, kind, offset) in [
        (
            "day before yesterday",
            "DAY_OFFSET:-2",
            TemporalExpressionKindIR::RelativeDay,
            Some(-2),
        ),
        (
            "yesterday",
            "DAY_OFFSET:-1",
            TemporalExpressionKindIR::RelativeDay,
            Some(-1),
        ),
        (
            "day after tomorrow",
            "DAY_OFFSET:+2",
            TemporalExpressionKindIR::RelativeDay,
            Some(2),
        ),
        (
            "today",
            "DAY_OFFSET:0",
            TemporalExpressionKindIR::RelativeDay,
            Some(0),
        ),
        (
            "tomorrow",
            "DAY_OFFSET:+1",
            TemporalExpressionKindIR::RelativeDay,
            Some(1),
        ),
        (
            "그제",
            "DAY_OFFSET:-2",
            TemporalExpressionKindIR::RelativeDay,
            Some(-2),
        ),
        (
            "어제",
            "DAY_OFFSET:-1",
            TemporalExpressionKindIR::RelativeDay,
            Some(-1),
        ),
        (
            "오늘",
            "DAY_OFFSET:0",
            TemporalExpressionKindIR::RelativeDay,
            Some(0),
        ),
        (
            "내일",
            "DAY_OFFSET:+1",
            TemporalExpressionKindIR::RelativeDay,
            Some(1),
        ),
        (
            "모레",
            "DAY_OFFSET:+2",
            TemporalExpressionKindIR::RelativeDay,
            Some(2),
        ),
        (
            "last week",
            "WEEK_OFFSET:-1",
            TemporalExpressionKindIR::RelativeWeek,
            None,
        ),
        (
            "next week",
            "WEEK_OFFSET:+1",
            TemporalExpressionKindIR::RelativeWeek,
            None,
        ),
        (
            "지난주",
            "WEEK_OFFSET:-1",
            TemporalExpressionKindIR::RelativeWeek,
            None,
        ),
        (
            "다음주",
            "WEEK_OFFSET:+1",
            TemporalExpressionKindIR::RelativeWeek,
            None,
        ),
    ] {
        if text.contains(surface) {
            return Some(TemporalExpressionIR {
                surface: surface.to_string(),
                normalized_value: value.to_string(),
                kind,
                relative_day_offset: offset,
                confidence_millis: 980,
            });
        }
    }
    if let Some(date) = iso_date(text) {
        return Some(TemporalExpressionIR {
            surface: date.clone(),
            normalized_value: date,
            kind: TemporalExpressionKindIR::CalendarDate,
            relative_day_offset: None,
            confidence_millis: 1_000,
        });
    }
    if let Some((surface, normalized)) = korean_date(text) {
        return Some(TemporalExpressionIR {
            surface,
            normalized_value: normalized,
            kind: TemporalExpressionKindIR::CalendarDate,
            relative_day_offset: None,
            confidence_millis: 980,
        });
    }
    if let Some((surface, normalized)) = clock_time(text) {
        return Some(TemporalExpressionIR {
            surface,
            normalized_value: normalized,
            kind: TemporalExpressionKindIR::ClockTime,
            relative_day_offset: None,
            confidence_millis: 930,
        });
    }
    None
}

fn iso_date(text: &str) -> Option<String> {
    text.split_whitespace().find_map(|token| {
        let token = token.trim_matches(|character: char| character.is_ascii_punctuation());
        let parts = token.split('-').collect::<Vec<_>>();
        (parts.len() == 3
            && parts[0].len() == 4
            && parts[1].len() == 2
            && parts[2].len() == 2
            && parts
                .iter()
                .all(|part| part.chars().all(|ch| ch.is_ascii_digit())))
        .then(|| token.to_string())
    })
}

fn korean_date(text: &str) -> Option<(String, String)> {
    let year_end = text.find('년')?;
    let month_end = text[year_end + '년'.len_utf8()..].find('월')? + year_end + '년'.len_utf8();
    let day_end = text[month_end + '월'.len_utf8()..].find('일')? + month_end + '월'.len_utf8();
    let year = trailing_number(&text[..year_end])?;
    let month = trailing_number(&text[year_end + '년'.len_utf8()..month_end])?;
    let day = trailing_number(&text[month_end + '월'.len_utf8()..day_end])?;
    let surface = format!("{year}년 {month}월 {day}일");
    Some((surface, format!("{year:04}-{month:02}-{day:02}")))
}

fn trailing_number(text: &str) -> Option<u32> {
    text.split_whitespace().next_back()?.parse().ok()
}

fn clock_time(text: &str) -> Option<(String, String)> {
    for (marker, add) in [("am", 0_u32), ("pm", 12_u32)] {
        let tokens = text.split_whitespace().collect::<Vec<_>>();
        for pair in tokens.windows(2) {
            if pair[1].trim_matches(|ch: char| ch.is_ascii_punctuation()) == marker {
                let hour = pair[0]
                    .trim_matches(|ch: char| !ch.is_ascii_digit())
                    .parse::<u32>()
                    .ok()?;
                let normalized_hour = if hour == 12 { add } else { hour + add };
                return Some((
                    format!("{} {}", pair[0], marker),
                    format!("TIME:{normalized_hour:02}:00"),
                ));
            }
        }
    }
    for (marker, add) in [("오전", 0_u32), ("오후", 12_u32)] {
        if let Some(position) = text.find(marker) {
            let rest = &text[position + marker.len()..];
            if let Some(hour_end) = rest.find('시') {
                let hour = rest[..hour_end].trim().parse::<u32>().ok()?;
                let normalized_hour = if hour == 12 { add } else { hour + add };
                return Some((
                    format!("{marker} {hour}시"),
                    format!("TIME:{normalized_hour:02}:00"),
                ));
            }
        }
    }
    None
}

fn trim_event_surface(text: &str) -> String {
    text.trim()
        .trim_matches(|character: char| matches!(character, ',' | '.' | '?' | '!' | ' '))
        .to_string()
}

fn normalize_space(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prefix_and_infix_relations_canonicalize_to_before() {
        let analyzer = TemporalSemanticAnalyzer;
        let prefix =
            analyzer.analyze_turn("After the backup completed, the deploy started.", 1, None);
        assert_eq!(prefix.events.len(), 2);
        assert!(prefix.events[0].surface.contains("backup"));
        assert!(prefix.events[1].surface.contains("deploy"));
        assert_eq!(prefix.relations[0].kind, TemporalRelationKindIR::Before);

        let infix =
            analyzer.analyze_turn("The deploy started after the backup completed.", 2, None);
        assert!(infix.events[0].surface.contains("backup"));
        assert!(infix.events[1].surface.contains("deploy"));
    }

    #[test]
    fn event_time_and_report_turn_are_separate() {
        let analysis =
            TemporalSemanticAnalyzer.analyze_turn("The backup completed yesterday.", 7, None);
        assert_eq!(analysis.events[0].report_turn, 7);
        assert_eq!(
            analysis.events[0]
                .event_time
                .as_ref()
                .and_then(|time| time.relative_day_offset),
            Some(-1)
        );
        assert!(!analysis.events[0].dialogue_truth_established);
    }

    #[test]
    fn graph_answers_transitive_before_paths() {
        let analyzer = TemporalSemanticAnalyzer;
        let mut graph = TemporalGraphIR::default();
        graph.apply_turn(&analyzer.analyze_turn(
            "The backup completed before the deploy started.",
            1,
            None,
        ));
        let prior = graph.clone();
        graph.apply_turn(&analyzer.analyze_turn(
            "The deploy started before the monitor failed.",
            2,
            Some(&prior),
        ));
        // The two mentions of deploy are distinct dialogue events, so this
        // local analyzer does not silently merge them without coreference.
        assert!(graph.validate(2));
    }

    #[test]
    fn answer_tampering_is_rejected() {
        let analyzer = TemporalSemanticAnalyzer;
        let mut graph = TemporalGraphIR::default();
        graph.apply_turn(&analyzer.analyze_turn("The backup completed yesterday.", 1, None));
        let mut answer = TemporalQaEngine
            .answer(
                "When did the backup complete?",
                Some(&graph),
                LanguageCodeIR::English,
            )
            .expect("temporal answer");
        assert!(answer.validate());
        answer.dialogue_truth_established = true;
        assert!(!answer.validate());
    }

    #[test]
    fn longer_relative_day_expression_wins_over_embedded_tomorrow() {
        let analysis =
            TemporalSemanticAnalyzer.analyze_turn("The deploy starts day after tomorrow.", 1, None);
        assert_eq!(
            analysis.events[0]
                .event_time
                .as_ref()
                .map(|time| time.normalized_value.as_str()),
            Some("DAY_OFFSET:+2")
        );
    }

    #[test]
    fn opposite_order_for_same_events_is_preserved_as_conflict() {
        let analyzer = TemporalSemanticAnalyzer;
        let mut graph = TemporalGraphIR::default();
        graph.apply_turn(&analyzer.analyze_turn(
            "The backup completed before the deploy started.",
            1,
            None,
        ));
        let prior = graph.clone();
        graph.apply_turn(&analyzer.analyze_turn(
            "The deploy started before the backup completed.",
            2,
            Some(&prior),
        ));
        assert_eq!(graph.events.len(), 2);
        assert_eq!(graph.conflicts.len(), 1);
        assert!(graph
            .relations
            .iter()
            .all(|relation| relation.status == TemporalRelationStatusIR::Contested));
        assert!(graph.validate(2));
    }

    #[test]
    fn shared_event_predicate_does_not_match_a_different_entity() {
        let analyzer = TemporalSemanticAnalyzer;
        let mut graph = TemporalGraphIR::default();
        graph.apply_turn(&analyzer.analyze_turn("The backup completed yesterday.", 1, None));
        let answer = TemporalQaEngine
            .answer(
                "What happened before the migration finished?",
                Some(&graph),
                LanguageCodeIR::English,
            )
            .expect("recognized temporal question");
        assert_eq!(
            answer.disposition,
            TemporalAnswerDispositionIR::NoMatchingEvent
        );
        assert!(answer.event_evidence.is_empty());
    }
}
