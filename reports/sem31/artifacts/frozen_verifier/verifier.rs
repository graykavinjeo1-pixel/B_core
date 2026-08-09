use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

pub const CONTRACT_VERSION: &str = "SEM31_INDEPENDENT_WORLD_VERIFIER_1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum EpistemicRole {
    Observed,
    Inferred,
    Predicted,
    Hypothesized,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct SemanticAtom {
    pub namespace_code: u16,
    pub axis_code: u16,
    pub value_code: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(tag = "form", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SemanticTerm {
    Primitive { atom: SemanticAtom },
    Composition { components: Vec<SemanticAtom> },
}

impl SemanticTerm {
    pub fn primitive(atom: SemanticAtom) -> Self {
        Self::Primitive { atom }
    }

    pub fn normalized(self) -> Self {
        match self {
            Self::Composition { mut components } => {
                components.sort_unstable();
                components.dedup();
                Self::Composition { components }
            }
            primitive => primitive,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct SchemaTerm {
    pub domain_code: u16,
    pub structural_axes: Vec<u16>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct RelationTerm {
    pub domain_code: u16,
    pub topology_code: u16,
    pub directionality: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct StateChannel {
    pub domain_code: u16,
    pub axis_code: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Provenance {
    pub source_code: u16,
    pub batch_code: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum FixturePhase {
    Foundation,
    Redundant,
    Novel,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "event_type", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum WorldEventKind {
    CreateEntity {
        entity: u64,
        schema: SchemaTerm,
    },
    ObserveProperty {
        entity: u64,
        property: SemanticTerm,
        role: EpistemicRole,
        confidence_bps: u16,
        provenance: Provenance,
    },
    ObserveState {
        entity: u64,
        channel: StateChannel,
        value_code: i64,
        role: EpistemicRole,
        confidence_bps: u16,
        provenance: Provenance,
    },
    AssertRelation {
        source: u64,
        relation: RelationTerm,
        target: u64,
        active: bool,
        role: EpistemicRole,
        confidence_bps: u16,
        provenance: Provenance,
    },
    ObserveIdentity {
        entity: u64,
        confidence_bps: u16,
        provenance: Provenance,
    },
    RecordException {
        entity: u64,
        schema_property: SemanticTerm,
        actual_property: SemanticTerm,
        confidence_bps: u16,
        provenance: Provenance,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorldEvent {
    pub event_index: u64,
    pub time: u64,
    pub phase: FixturePhase,
    pub kind: WorldEventKind,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorldChallenge {
    pub contract_version: String,
    pub world_id: u64,
    pub seed: u64,
    pub events: Vec<WorldEvent>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct EntityView {
    pub entity: u64,
    pub schema: SchemaTerm,
    pub properties: Vec<SemanticTerm>,
    pub exceptions: Vec<(SemanticTerm, SemanticTerm)>,
    pub first_seen: u64,
    pub last_seen: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct RelationView {
    pub source: u64,
    pub relation: RelationTerm,
    pub target: u64,
    pub role: EpistemicRole,
    pub confidence_bps: u16,
    pub provenance: Provenance,
    pub asserted_at: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct StateView {
    pub entity: u64,
    pub channel: StateChannel,
    pub value_code: i64,
    pub role: EpistemicRole,
    pub confidence_bps: u16,
    pub provenance: Provenance,
    pub asserted_at: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorldView {
    pub entities: Vec<EntityView>,
    pub property_nodes: Vec<SemanticTerm>,
    pub relation_types: Vec<RelationTerm>,
    pub state_channels: Vec<StateChannel>,
    pub active_relations: Vec<RelationView>,
    pub current_states: Vec<StateView>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HistoryWitness {
    pub after_event_count: u64,
    pub world: WorldView,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Instrumentation {
    pub world_delta_events: u64,
    pub full_world_snapshot_copies: u64,
    pub full_entity_rewrite_events: u64,
    pub duplicated_shared_semantic_payload_events: u64,
    pub persistent_property_transient_state_confusion_events: u64,
    pub uncertain_assertions_collapsed_to_certain: u64,
    pub unnecessary_schema_fork_events: u64,
    pub unresolved_silent_world_contradictions: u64,
    pub world_memory_full_scans: u64,
    pub world_gold_graph_reads: u64,
    pub expected_world_state_lookups: u64,
    pub future_world_event_leakage_events: u64,
    pub node_id_is_semantic_payload: bool,
    pub natural_language_is_canonical_world_memory: bool,
    pub natural_language_is_world_reasoning_authority: bool,
    pub world_memory_natural_language_bytes_on_hot_path: u64,
    pub canonical_property_payload_instances: u64,
    pub conflict_evidence_records: u64,
    pub active_entities_sequence: Vec<u64>,
    pub active_semantic_nodes_sequence: Vec<u64>,
    pub new_semantic_bytes_per_experience_sequence: Vec<u64>,
    pub total_semantic_memory_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldSubmission {
    pub final_world: WorldView,
    pub history_witnesses: Vec<HistoryWitness>,
    pub instrumentation: Instrumentation,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationRequest {
    pub challenge: WorldChallenge,
    pub submission: WorldSubmission,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct VerifiedMetrics {
    pub world_entities_total: u64,
    pub world_property_nodes_total: u64,
    pub world_relation_types_total: u64,
    pub world_relations_total: u64,
    pub world_state_events_total: u64,
    pub identity_continuity_events: u64,
    pub existing_property_reuse_events: u64,
    pub property_composition_events: u64,
    pub new_property_primitive_genesis_events: u64,
    pub incremental_entity_update_events: u64,
    pub observed_assertions: u64,
    pub inferred_assertions: u64,
    pub predicted_assertions: u64,
    pub hypothesized_assertions: u64,
    pub uncertain_assertions_total: u64,
    pub explained_observation_events: u64,
    pub irreducible_residual_events: u64,
    pub total_experience_events: u64,
    pub instance_exception_events: u64,
    pub relation_delta_events: u64,
    pub contradiction_evidence_events: u64,
    pub redundant_group_events: u64,
    pub novel_group_events: u64,
    pub total_world_semantic_nodes: u64,
    pub active_entities_p50: u64,
    pub active_entities_p95: u64,
    pub active_semantic_nodes_p50: u64,
    pub active_semantic_nodes_p95: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationResult {
    pub contract_version: String,
    pub accepted: bool,
    pub violations: Vec<String>,
    pub entity_identity_pass: bool,
    pub property_correctness_pass: bool,
    pub relation_correctness_pass: bool,
    pub state_correctness_pass: bool,
    pub history_reconstruction_pass: bool,
    pub epistemic_integrity_pass: bool,
    pub semantic_duplication_pass: bool,
    pub residual_accounting_pass: bool,
    pub metrics: VerifiedMetrics,
}

#[derive(Debug, Clone)]
struct OracleEntity {
    schema: SchemaTerm,
    properties: BTreeSet<SemanticTerm>,
    exceptions: BTreeSet<(SemanticTerm, SemanticTerm)>,
    first_seen: u64,
    last_seen: u64,
}

#[derive(Debug, Clone, Default)]
struct Oracle {
    entities: BTreeMap<u64, OracleEntity>,
    property_nodes: BTreeSet<SemanticTerm>,
    relation_types: BTreeSet<RelationTerm>,
    state_channels: BTreeSet<StateChannel>,
    relations: BTreeMap<(u64, RelationTerm, u64), RelationView>,
    states: BTreeMap<(u64, StateChannel), StateView>,
    metrics: VerifiedMetrics,
    conflict_records: u64,
    active_entities: Vec<u64>,
    active_nodes: Vec<u64>,
}

impl Oracle {
    fn apply(&mut self, event: &WorldEvent, violations: &mut Vec<String>) {
        if event.event_index != self.metrics.total_experience_events + 1 {
            violations.push(format!("NON_CONTIGUOUS_EVENT_INDEX:{}", event.event_index));
        }
        self.metrics.total_experience_events += 1;
        match event.phase {
            FixturePhase::Redundant => self.metrics.redundant_group_events += 1,
            FixturePhase::Novel => self.metrics.novel_group_events += 1,
            FixturePhase::Foundation => {}
        }
        let mut active_entities = BTreeSet::new();
        let mut active_nodes = 0_u64;
        match &event.kind {
            WorldEventKind::CreateEntity { entity, schema } => {
                active_entities.insert(*entity);
                active_nodes = 1;
                if self.entities.contains_key(entity) {
                    violations.push(format!("DUPLICATE_ENTITY_CREATION:{entity}"));
                } else {
                    self.entities.insert(
                        *entity,
                        OracleEntity {
                            schema: schema.clone(),
                            properties: BTreeSet::new(),
                            exceptions: BTreeSet::new(),
                            first_seen: event.time,
                            last_seen: event.time,
                        },
                    );
                }
            }
            WorldEventKind::ObserveProperty {
                entity,
                property,
                role,
                confidence_bps,
                ..
            } => {
                active_entities.insert(*entity);
                active_nodes = match property {
                    SemanticTerm::Primitive { .. } => 1,
                    SemanticTerm::Composition { components } => components.len() as u64 + 1,
                };
                count_assertion(&mut self.metrics, *role, *confidence_bps);
                let property = property.clone().normalized();
                let existed = self.property_nodes.contains(&property);
                if !existed {
                    match &property {
                        SemanticTerm::Primitive { .. } => {
                            self.metrics.new_property_primitive_genesis_events += 1;
                        }
                        SemanticTerm::Composition { components } => {
                            if components.len() < 2
                                || !components.iter().all(|component| {
                                    self.property_nodes
                                        .contains(&SemanticTerm::primitive(*component))
                                })
                            {
                                violations.push(format!(
                                    "UNJUSTIFIED_PROPERTY_COMPOSITION:{}",
                                    event.event_index
                                ));
                            }
                            self.metrics.property_composition_events += 1;
                        }
                    }
                    self.property_nodes.insert(property.clone());
                    self.metrics.irreducible_residual_events += 1;
                } else {
                    self.metrics.explained_observation_events += 1;
                }
                match self.entities.get_mut(entity) {
                    Some(record) => {
                        if record.properties.insert(property) {
                            self.metrics.incremental_entity_update_events += 1;
                            if existed {
                                self.metrics.existing_property_reuse_events += 1;
                            }
                        }
                        record.last_seen = record.last_seen.max(event.time);
                    }
                    None => violations.push(format!("UNKNOWN_PROPERTY_ENTITY:{entity}")),
                }
            }
            WorldEventKind::ObserveState {
                entity,
                channel,
                value_code,
                role,
                confidence_bps,
                provenance,
            } => {
                active_entities.insert(*entity);
                active_nodes = 1;
                self.metrics.world_state_events_total += 1;
                count_assertion(&mut self.metrics, *role, *confidence_bps);
                if !self.state_channels.insert(*channel) {
                    self.metrics.explained_observation_events += 1;
                } else {
                    self.metrics.irreducible_residual_events += 1;
                }
                if let Some(record) = self.entities.get_mut(entity) {
                    record.last_seen = record.last_seen.max(event.time);
                } else {
                    violations.push(format!("UNKNOWN_STATE_ENTITY:{entity}"));
                }
                let key = (*entity, *channel);
                let incoming = StateView {
                    entity: *entity,
                    channel: *channel,
                    value_code: *value_code,
                    role: *role,
                    confidence_bps: *confidence_bps,
                    provenance: *provenance,
                    asserted_at: event.time,
                };
                if let Some(previous) = self.states.get(&key) {
                    if previous.asserted_at == event.time && previous.value_code != *value_code {
                        self.conflict_records += 1;
                        self.metrics.contradiction_evidence_events += 1;
                    }
                }
                let replace = self.states.get(&key).is_none_or(|previous| {
                    event.time > previous.asserted_at
                        || (event.time == previous.asserted_at
                            && *confidence_bps > previous.confidence_bps)
                });
                if replace {
                    self.states.insert(key, incoming);
                }
            }
            WorldEventKind::AssertRelation {
                source,
                relation,
                target,
                active,
                role,
                confidence_bps,
                provenance,
            } => {
                active_entities.insert(*source);
                active_entities.insert(*target);
                active_nodes = 1;
                count_assertion(&mut self.metrics, *role, *confidence_bps);
                if !self.relation_types.insert(*relation) {
                    self.metrics.explained_observation_events += 1;
                } else {
                    self.metrics.irreducible_residual_events += 1;
                }
                if !self.entities.contains_key(source) || !self.entities.contains_key(target) {
                    violations.push(format!("UNKNOWN_RELATION_ENDPOINT:{}", event.event_index));
                }
                let key = (*source, *relation, *target);
                if *active {
                    self.relations.insert(
                        key,
                        RelationView {
                            source: *source,
                            relation: *relation,
                            target: *target,
                            role: *role,
                            confidence_bps: *confidence_bps,
                            provenance: *provenance,
                            asserted_at: event.time,
                        },
                    );
                } else {
                    self.relations.remove(&key);
                }
                self.metrics.relation_delta_events += 1;
            }
            WorldEventKind::ObserveIdentity {
                entity,
                confidence_bps,
                ..
            } => {
                active_entities.insert(*entity);
                self.metrics.identity_continuity_events += 1;
                count_assertion(&mut self.metrics, EpistemicRole::Observed, *confidence_bps);
                self.metrics.explained_observation_events += 1;
                match self.entities.get_mut(entity) {
                    Some(record) => record.last_seen = record.last_seen.max(event.time),
                    None => violations.push(format!("UNKNOWN_IDENTITY_ENTITY:{entity}")),
                }
            }
            WorldEventKind::RecordException {
                entity,
                schema_property,
                actual_property,
                confidence_bps,
                ..
            } => {
                active_entities.insert(*entity);
                active_nodes = 2;
                count_assertion(&mut self.metrics, EpistemicRole::Observed, *confidence_bps);
                self.metrics.instance_exception_events += 1;
                self.metrics.irreducible_residual_events += 1;
                match self.entities.get_mut(entity) {
                    Some(record) => {
                        if !self.property_nodes.contains(schema_property)
                            || !self.property_nodes.contains(actual_property)
                        {
                            violations
                                .push(format!("EXCEPTION_PROPERTY_MISSING:{}", event.event_index));
                        }
                        record
                            .exceptions
                            .insert((schema_property.clone(), actual_property.clone()));
                        record.last_seen = record.last_seen.max(event.time);
                        self.metrics.incremental_entity_update_events += 1;
                    }
                    None => violations.push(format!("UNKNOWN_EXCEPTION_ENTITY:{entity}")),
                }
            }
        }
        self.active_entities.push(active_entities.len() as u64);
        self.active_nodes.push(active_nodes);
    }

    fn view(&self) -> WorldView {
        WorldView {
            entities: self
                .entities
                .iter()
                .map(|(entity, record)| EntityView {
                    entity: *entity,
                    schema: record.schema.clone(),
                    properties: record.properties.iter().cloned().collect(),
                    exceptions: record.exceptions.iter().cloned().collect(),
                    first_seen: record.first_seen,
                    last_seen: record.last_seen,
                })
                .collect(),
            property_nodes: self.property_nodes.iter().cloned().collect(),
            relation_types: self.relation_types.iter().copied().collect(),
            state_channels: self.state_channels.iter().copied().collect(),
            active_relations: self.relations.values().cloned().collect(),
            current_states: self.states.values().cloned().collect(),
        }
    }

    fn finish_metrics(&mut self) {
        self.metrics.world_entities_total = self.entities.len() as u64;
        self.metrics.world_property_nodes_total = self.property_nodes.len() as u64;
        self.metrics.world_relation_types_total = self.relation_types.len() as u64;
        self.metrics.world_relations_total = self.relations.len() as u64;
        self.metrics.total_world_semantic_nodes = (self.property_nodes.len()
            + self.relation_types.len()
            + self.state_channels.len()
            + self
                .entities
                .values()
                .map(|entity| entity.schema.clone())
                .collect::<BTreeSet<_>>()
                .len()) as u64;
        let mut active_entities = self.active_entities.clone();
        let mut active_nodes = self.active_nodes.clone();
        active_entities.sort_unstable();
        active_nodes.sort_unstable();
        self.metrics.active_entities_p50 = percentile(&active_entities, 50);
        self.metrics.active_entities_p95 = percentile(&active_entities, 95);
        self.metrics.active_semantic_nodes_p50 = percentile(&active_nodes, 50);
        self.metrics.active_semantic_nodes_p95 = percentile(&active_nodes, 95);
    }
}

pub fn verify(request: &VerificationRequest) -> VerificationResult {
    let mut violations = Vec::new();
    if request.challenge.contract_version != CONTRACT_VERSION {
        violations.push("CONTRACT_VERSION_MISMATCH".to_string());
    }
    let mut oracle = Oracle::default();
    let witness_map: BTreeMap<u64, &WorldView> = request
        .submission
        .history_witnesses
        .iter()
        .map(|witness| (witness.after_event_count, &witness.world))
        .collect();
    let mut history_pass = !witness_map.is_empty();
    for event in &request.challenge.events {
        oracle.apply(event, &mut violations);
        if let Some(submitted) = witness_map.get(&event.event_index) {
            if **submitted != oracle.view() {
                history_pass = false;
                violations.push(format!("HISTORY_WITNESS_MISMATCH:{}", event.event_index));
            }
        }
    }
    if witness_map
        .keys()
        .any(|index| *index == 0 || *index > request.challenge.events.len() as u64)
    {
        history_pass = false;
        violations.push("INVALID_HISTORY_WITNESS_INDEX".to_string());
    }
    oracle.finish_metrics();
    let expected = oracle.view();
    let final_equal = request.submission.final_world == expected;
    if !final_equal {
        violations.push("FINAL_WORLD_MISMATCH".to_string());
    }
    let instrument = &request.submission.instrumentation;
    let expected_event_count = request.challenge.events.len() as u64;
    for (label, value) in [
        (
            "FULL_WORLD_SNAPSHOT_COPIES",
            instrument.full_world_snapshot_copies,
        ),
        (
            "FULL_ENTITY_REWRITE_EVENTS",
            instrument.full_entity_rewrite_events,
        ),
        (
            "DUPLICATED_SHARED_SEMANTIC_PAYLOAD_EVENTS",
            instrument.duplicated_shared_semantic_payload_events,
        ),
        (
            "PERSISTENT_PROPERTY_TRANSIENT_STATE_CONFUSION_EVENTS",
            instrument.persistent_property_transient_state_confusion_events,
        ),
        (
            "UNCERTAIN_ASSERTIONS_COLLAPSED_TO_CERTAIN",
            instrument.uncertain_assertions_collapsed_to_certain,
        ),
        (
            "UNNECESSARY_SCHEMA_FORK_EVENTS",
            instrument.unnecessary_schema_fork_events,
        ),
        (
            "UNRESOLVED_SILENT_WORLD_CONTRADICTIONS",
            instrument.unresolved_silent_world_contradictions,
        ),
        (
            "WORLD_MEMORY_FULL_SCANS",
            instrument.world_memory_full_scans,
        ),
        ("WORLD_GOLD_GRAPH_READS", instrument.world_gold_graph_reads),
        (
            "EXPECTED_WORLD_STATE_LOOKUPS",
            instrument.expected_world_state_lookups,
        ),
        (
            "FUTURE_WORLD_EVENT_LEAKAGE_EVENTS",
            instrument.future_world_event_leakage_events,
        ),
        (
            "WORLD_MEMORY_NATURAL_LANGUAGE_BYTES_ON_HOT_PATH",
            instrument.world_memory_natural_language_bytes_on_hot_path,
        ),
    ] {
        if value != 0 {
            violations.push(format!("{label}_NONZERO:{value}"));
        }
    }
    if instrument.world_delta_events != expected_event_count {
        violations.push("WORLD_DELTA_COUNT_MISMATCH".to_string());
    }
    if instrument.node_id_is_semantic_payload
        || instrument.natural_language_is_canonical_world_memory
        || instrument.natural_language_is_world_reasoning_authority
    {
        violations.push("LANGUAGE_OR_ID_AUTHORITY_VIOLATION".to_string());
    }
    if instrument.canonical_property_payload_instances != oracle.metrics.world_property_nodes_total
    {
        violations.push("PROPERTY_PAYLOAD_INSTANCE_COUNT_MISMATCH".to_string());
    }
    if instrument.conflict_evidence_records != oracle.conflict_records {
        violations.push("CONFLICT_EVIDENCE_RECORD_COUNT_MISMATCH".to_string());
    }
    if instrument.active_entities_sequence != oracle.active_entities
        || instrument.active_semantic_nodes_sequence != oracle.active_nodes
    {
        violations.push("ACTIVE_FIELD_INSTRUMENTATION_MISMATCH".to_string());
    }
    if instrument.new_semantic_bytes_per_experience_sequence.len() != request.challenge.events.len()
        || instrument.total_semantic_memory_bytes == 0
    {
        violations.push("STORAGE_INSTRUMENTATION_INCOMPLETE".to_string());
    }
    let redundant_bytes = phase_average(
        &request.challenge.events,
        &instrument.new_semantic_bytes_per_experience_sequence,
        FixturePhase::Redundant,
    );
    let novel_bytes = phase_average(
        &request.challenge.events,
        &instrument.new_semantic_bytes_per_experience_sequence,
        FixturePhase::Novel,
    );
    let residual_pass = oracle.metrics.redundant_group_events == oracle.metrics.novel_group_events
        && oracle.metrics.redundant_group_events > 0
        && redundant_bytes < novel_bytes;
    if !residual_pass {
        violations.push("RESIDUAL_STORAGE_ADVANTAGE_NOT_OBSERVED".to_string());
    }
    let epistemic_pass = oracle.metrics.observed_assertions > 0
        && oracle.metrics.inferred_assertions > 0
        && oracle.metrics.predicted_assertions > 0
        && oracle.metrics.hypothesized_assertions > 0
        && oracle.metrics.uncertain_assertions_total > 0
        && instrument.uncertain_assertions_collapsed_to_certain == 0;
    if !epistemic_pass {
        violations.push("EPISTEMIC_ROLE_COVERAGE_INCOMPLETE".to_string());
    }
    let identity_pass = oracle.metrics.identity_continuity_events > 0
        && !violations
            .iter()
            .any(|value| value.contains("ENTITY") || value.contains("IDENTITY"));
    let property_pass = oracle.metrics.existing_property_reuse_events > 0
        && oracle.metrics.property_composition_events > 0
        && oracle.metrics.new_property_primitive_genesis_events > 0
        && final_equal;
    let relation_pass = oracle.metrics.relation_delta_events >= 2 && final_equal;
    let state_pass = oracle.metrics.world_state_events_total > 0
        && oracle.metrics.contradiction_evidence_events > 0
        && instrument.unresolved_silent_world_contradictions == 0
        && final_equal;
    let duplication_pass = instrument.duplicated_shared_semantic_payload_events == 0
        && instrument.canonical_property_payload_instances
            == oracle.metrics.world_property_nodes_total;
    let accepted = violations.is_empty()
        && identity_pass
        && property_pass
        && relation_pass
        && state_pass
        && history_pass
        && epistemic_pass
        && duplication_pass
        && residual_pass;
    VerificationResult {
        contract_version: CONTRACT_VERSION.to_string(),
        accepted,
        violations,
        entity_identity_pass: identity_pass,
        property_correctness_pass: property_pass,
        relation_correctness_pass: relation_pass,
        state_correctness_pass: state_pass,
        history_reconstruction_pass: history_pass,
        epistemic_integrity_pass: epistemic_pass,
        semantic_duplication_pass: duplication_pass,
        residual_accounting_pass: residual_pass,
        metrics: oracle.metrics,
    }
}

fn count_assertion(metrics: &mut VerifiedMetrics, role: EpistemicRole, confidence_bps: u16) {
    match role {
        EpistemicRole::Observed => metrics.observed_assertions += 1,
        EpistemicRole::Inferred => metrics.inferred_assertions += 1,
        EpistemicRole::Predicted => metrics.predicted_assertions += 1,
        EpistemicRole::Hypothesized => metrics.hypothesized_assertions += 1,
    }
    if confidence_bps < 10_000 {
        metrics.uncertain_assertions_total += 1;
    }
}

fn phase_average(events: &[WorldEvent], bytes: &[u64], phase: FixturePhase) -> u64 {
    let selected: Vec<u64> = events
        .iter()
        .zip(bytes)
        .filter_map(|(event, value)| (event.phase == phase).then_some(*value))
        .collect();
    if selected.is_empty() {
        0
    } else {
        selected.iter().sum::<u64>() / selected.len() as u64
    }
}

fn percentile(values: &[u64], percent: usize) -> u64 {
    if values.is_empty() {
        return 0;
    }
    let index = ((values.len() - 1) * percent).div_ceil(100);
    values[index.min(values.len() - 1)]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn semantic_composition_normalizes_without_language() {
        let left = SemanticAtom {
            namespace_code: 1,
            axis_code: 2,
            value_code: 3,
        };
        let right = SemanticAtom {
            namespace_code: 1,
            axis_code: 4,
            value_code: 5,
        };
        assert_eq!(
            SemanticTerm::Composition {
                components: vec![right, left, left]
            }
            .normalized(),
            SemanticTerm::Composition {
                components: vec![left, right]
            }
        );
    }
}
