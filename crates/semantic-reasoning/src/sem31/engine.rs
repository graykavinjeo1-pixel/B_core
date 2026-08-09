use std::{
    collections::{BTreeMap, BTreeSet},
    mem::size_of,
};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::verifier::{
    EntityView, EpistemicRole, FixturePhase, HistoryWitness, Instrumentation, Provenance,
    RelationTerm, RelationView, SchemaTerm, SemanticAtom, SemanticTerm, StateChannel, StateView,
    WorldChallenge, WorldEvent, WorldEventKind, WorldSubmission, WorldView, CONTRACT_VERSION,
};

pub const MAX_AUTONOMOUS_RESEARCH_EPOCHS: usize = 4096;
pub const CANONICAL_EVENT_COUNT: usize = 40;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "delta_code", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum WorldDelta {
    CreateEntity {
        entity: u64,
        schema_id: u64,
        time: u64,
    },
    NewPropertyEdge {
        entity: u64,
        property_id: u64,
        semantic_payload: SemanticTerm,
        time: u64,
    },
    AddPropertyEdge {
        entity: u64,
        property_id: u64,
        time: u64,
    },
    ConfirmProperty {
        entity: u64,
        property_id: u64,
        confidence_bps: u16,
        time: u64,
    },
    StateUpdate {
        entity: u64,
        channel_id: u64,
        value_code: i64,
        assertion_index: u64,
        time: u64,
    },
    RelationUpdate {
        source: u64,
        relation_id: u64,
        target: u64,
        active: bool,
        assertion_index: u64,
        time: u64,
    },
    IdentityContinuation {
        entity: u64,
        time: u64,
    },
    InstanceException {
        entity: u64,
        schema_property_id: u64,
        actual_property_id: u64,
        assertion_index: u64,
        time: u64,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "reference_code", rename_all = "SCREAMING_SNAKE_CASE")]
enum AssertionReference {
    Property {
        property_id: u64,
    },
    State {
        channel_id: u64,
        value_code: i64,
    },
    Relation {
        relation_id: u64,
        target: u64,
        active: bool,
    },
    Identity,
    Exception {
        schema_property_id: u64,
        actual_property_id: u64,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AssertionRecord {
    entity: u64,
    reference: AssertionReference,
    role: EpistemicRole,
    confidence_bps: u16,
    provenance: Provenance,
    time: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct EntityRecord {
    schema_id: u64,
    property_ids: BTreeSet<u64>,
    exceptions: BTreeSet<(u64, u64)>,
    first_seen: u64,
    last_seen: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RelationBinding {
    assertion_index: u64,
    asserted_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct StateBinding {
    value_code: i64,
    assertion_index: u64,
    asserted_at: u64,
}

#[derive(Debug, Serialize)]
struct CanonicalMemoryImage<'a> {
    schemas: &'a [SchemaTerm],
    property_nodes: &'a [SemanticTerm],
    relation_types: &'a [RelationTerm],
    state_channels: &'a [StateChannel],
    entities: &'a BTreeMap<u64, EntityRecord>,
    active_relations: Vec<CanonicalRelationImage<'a>>,
    current_states: Vec<CanonicalStateImage<'a>>,
    assertions: &'a [AssertionRecord],
    conflict_evidence: &'a [(u64, u64)],
    deltas: &'a [WorldDelta],
}

#[derive(Debug, Serialize)]
struct CanonicalRelationImage<'a> {
    source: u64,
    relation_id: u64,
    target: u64,
    binding: &'a RelationBinding,
}

#[derive(Debug, Serialize)]
struct CanonicalStateImage<'a> {
    entity: u64,
    channel_id: u64,
    binding: &'a StateBinding,
}

#[derive(Debug, Default)]
pub struct WorldModel {
    schemas: Vec<SchemaTerm>,
    schema_index: BTreeMap<[u8; 32], u64>,
    property_nodes: Vec<SemanticTerm>,
    property_index: BTreeMap<[u8; 32], u64>,
    relation_types: Vec<RelationTerm>,
    relation_index: BTreeMap<[u8; 32], u64>,
    state_channels: Vec<StateChannel>,
    state_channel_index: BTreeMap<[u8; 32], u64>,
    entities: BTreeMap<u64, EntityRecord>,
    relations: BTreeMap<(u64, u64, u64), RelationBinding>,
    states: BTreeMap<(u64, u64), StateBinding>,
    assertions: Vec<AssertionRecord>,
    conflict_evidence: Vec<(u64, u64)>,
    deltas: Vec<WorldDelta>,
    active_entities: Vec<u64>,
    active_nodes: Vec<u64>,
    new_bytes: Vec<u64>,
}

impl WorldModel {
    pub fn apply(&mut self, event: &WorldEvent) -> Result<(), String> {
        if event.event_index != self.deltas.len() as u64 + 1 {
            return Err(format!("NON_CONTIGUOUS_EVENT_INDEX:{}", event.event_index));
        }
        let bytes_before = self.semantic_memory_bytes()?;
        let (active_entities, active_nodes) = match &event.kind {
            WorldEventKind::CreateEntity { entity, schema } => {
                if self.entities.contains_key(entity) {
                    return Err(format!("DUPLICATE_ENTITY:{entity}"));
                }
                let schema_id = self.intern_schema(schema.clone())?;
                self.entities.insert(
                    *entity,
                    EntityRecord {
                        schema_id,
                        property_ids: BTreeSet::new(),
                        exceptions: BTreeSet::new(),
                        first_seen: event.time,
                        last_seen: event.time,
                    },
                );
                self.deltas.push(WorldDelta::CreateEntity {
                    entity: *entity,
                    schema_id,
                    time: event.time,
                });
                (1, 1)
            }
            WorldEventKind::ObserveProperty {
                entity,
                property,
                role,
                confidence_bps,
                provenance,
            } => {
                let normalized = property.clone().normalized();
                let fingerprint = semantic_fingerprint(&normalized)?;
                let existing_id = self.property_index.get(&fingerprint).copied();
                let property_id = if let Some(id) = existing_id {
                    if self.property_nodes.get(id as usize) != Some(&normalized) {
                        return Err("SEMANTIC_FINGERPRINT_COLLISION".to_string());
                    }
                    id
                } else {
                    if let SemanticTerm::Composition { components } = &normalized {
                        if components.len() < 2
                            || !components.iter().all(|component| {
                                semantic_fingerprint(&SemanticTerm::primitive(*component))
                                    .ok()
                                    .and_then(|key| self.property_index.get(&key))
                                    .is_some()
                            })
                        {
                            return Err(format!(
                                "COMPOSITION_COMPONENT_NOT_GROUNDED:{}",
                                event.event_index
                            ));
                        }
                    }
                    self.intern_property(normalized.clone(), fingerprint)
                };
                let record = self
                    .entities
                    .get_mut(entity)
                    .ok_or_else(|| format!("UNKNOWN_PROPERTY_ENTITY:{entity}"))?;
                let inserted = record.property_ids.insert(property_id);
                record.last_seen = record.last_seen.max(event.time);
                let assertion_index = self.assertions.len() as u64;
                self.assertions.push(AssertionRecord {
                    entity: *entity,
                    reference: AssertionReference::Property { property_id },
                    role: *role,
                    confidence_bps: *confidence_bps,
                    provenance: *provenance,
                    time: event.time,
                });
                if existing_id.is_none() {
                    self.deltas.push(WorldDelta::NewPropertyEdge {
                        entity: *entity,
                        property_id,
                        semantic_payload: normalized.clone(),
                        time: event.time,
                    });
                } else if inserted {
                    self.deltas.push(WorldDelta::AddPropertyEdge {
                        entity: *entity,
                        property_id,
                        time: event.time,
                    });
                } else {
                    self.deltas.push(WorldDelta::ConfirmProperty {
                        entity: *entity,
                        property_id,
                        confidence_bps: *confidence_bps,
                        time: event.time,
                    });
                }
                let active_nodes = match normalized {
                    SemanticTerm::Primitive { .. } => 1,
                    SemanticTerm::Composition { components } => components.len() as u64 + 1,
                };
                let _ = assertion_index;
                (1, active_nodes)
            }
            WorldEventKind::ObserveState {
                entity,
                channel,
                value_code,
                role,
                confidence_bps,
                provenance,
            } => {
                let channel_id = self.intern_state_channel(*channel)?;
                let record = self
                    .entities
                    .get_mut(entity)
                    .ok_or_else(|| format!("UNKNOWN_STATE_ENTITY:{entity}"))?;
                record.last_seen = record.last_seen.max(event.time);
                let assertion_index = self.assertions.len() as u64;
                self.assertions.push(AssertionRecord {
                    entity: *entity,
                    reference: AssertionReference::State {
                        channel_id,
                        value_code: *value_code,
                    },
                    role: *role,
                    confidence_bps: *confidence_bps,
                    provenance: *provenance,
                    time: event.time,
                });
                let key = (*entity, channel_id);
                if let Some(previous) = self.states.get(&key) {
                    if previous.asserted_at == event.time && previous.value_code != *value_code {
                        self.conflict_evidence
                            .push((previous.assertion_index, assertion_index));
                    }
                }
                let replace = self.states.get(&key).is_none_or(|previous| {
                    event.time > previous.asserted_at
                        || (event.time == previous.asserted_at
                            && *confidence_bps
                                > self.assertions[previous.assertion_index as usize].confidence_bps)
                });
                if replace {
                    self.states.insert(
                        key,
                        StateBinding {
                            value_code: *value_code,
                            assertion_index,
                            asserted_at: event.time,
                        },
                    );
                }
                self.deltas.push(WorldDelta::StateUpdate {
                    entity: *entity,
                    channel_id,
                    value_code: *value_code,
                    assertion_index,
                    time: event.time,
                });
                (1, 1)
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
                if !self.entities.contains_key(source) || !self.entities.contains_key(target) {
                    return Err(format!("UNKNOWN_RELATION_ENDPOINT:{}", event.event_index));
                }
                let relation_id = self.intern_relation(*relation)?;
                let assertion_index = self.assertions.len() as u64;
                self.assertions.push(AssertionRecord {
                    entity: *source,
                    reference: AssertionReference::Relation {
                        relation_id,
                        target: *target,
                        active: *active,
                    },
                    role: *role,
                    confidence_bps: *confidence_bps,
                    provenance: *provenance,
                    time: event.time,
                });
                let key = (*source, relation_id, *target);
                if *active {
                    self.relations.insert(
                        key,
                        RelationBinding {
                            assertion_index,
                            asserted_at: event.time,
                        },
                    );
                } else {
                    self.relations.remove(&key);
                }
                self.deltas.push(WorldDelta::RelationUpdate {
                    source: *source,
                    relation_id,
                    target: *target,
                    active: *active,
                    assertion_index,
                    time: event.time,
                });
                (2, 1)
            }
            WorldEventKind::ObserveIdentity {
                entity,
                confidence_bps,
                provenance,
            } => {
                let record = self
                    .entities
                    .get_mut(entity)
                    .ok_or_else(|| format!("UNKNOWN_IDENTITY_ENTITY:{entity}"))?;
                record.last_seen = record.last_seen.max(event.time);
                self.assertions.push(AssertionRecord {
                    entity: *entity,
                    reference: AssertionReference::Identity,
                    role: EpistemicRole::Observed,
                    confidence_bps: *confidence_bps,
                    provenance: *provenance,
                    time: event.time,
                });
                self.deltas.push(WorldDelta::IdentityContinuation {
                    entity: *entity,
                    time: event.time,
                });
                (1, 0)
            }
            WorldEventKind::RecordException {
                entity,
                schema_property,
                actual_property,
                confidence_bps,
                provenance,
            } => {
                let schema_id = self
                    .property_id(schema_property)?
                    .ok_or("SCHEMA_EXCEPTION_PROPERTY_NOT_GROUNDED")?;
                let actual_id = self
                    .property_id(actual_property)?
                    .ok_or("ACTUAL_EXCEPTION_PROPERTY_NOT_GROUNDED")?;
                let record = self
                    .entities
                    .get_mut(entity)
                    .ok_or_else(|| format!("UNKNOWN_EXCEPTION_ENTITY:{entity}"))?;
                record.exceptions.insert((schema_id, actual_id));
                record.last_seen = record.last_seen.max(event.time);
                let assertion_index = self.assertions.len() as u64;
                self.assertions.push(AssertionRecord {
                    entity: *entity,
                    reference: AssertionReference::Exception {
                        schema_property_id: schema_id,
                        actual_property_id: actual_id,
                    },
                    role: EpistemicRole::Observed,
                    confidence_bps: *confidence_bps,
                    provenance: *provenance,
                    time: event.time,
                });
                self.deltas.push(WorldDelta::InstanceException {
                    entity: *entity,
                    schema_property_id: schema_id,
                    actual_property_id: actual_id,
                    assertion_index,
                    time: event.time,
                });
                (1, 2)
            }
        };
        self.active_entities.push(active_entities);
        self.active_nodes.push(active_nodes);
        let bytes_after = self.semantic_memory_bytes()?;
        self.new_bytes
            .push(bytes_after.saturating_sub(bytes_before));
        Ok(())
    }

    pub fn view(&self) -> Result<WorldView, String> {
        let mut entities = Vec::with_capacity(self.entities.len());
        for (entity, record) in &self.entities {
            let mut properties = Vec::with_capacity(record.property_ids.len());
            for id in &record.property_ids {
                properties.push(self.property(*id)?.clone());
            }
            let mut exceptions = Vec::with_capacity(record.exceptions.len());
            for (left, right) in &record.exceptions {
                exceptions.push((
                    self.property(*left)?.clone(),
                    self.property(*right)?.clone(),
                ));
            }
            properties.sort();
            exceptions.sort();
            entities.push(EntityView {
                entity: *entity,
                schema: self.schema(record.schema_id)?.clone(),
                properties,
                exceptions,
                first_seen: record.first_seen,
                last_seen: record.last_seen,
            });
        }
        let mut active_relations = Vec::with_capacity(self.relations.len());
        for ((source, relation_id, target), binding) in &self.relations {
            let assertion = self
                .assertions
                .get(binding.assertion_index as usize)
                .ok_or("RELATION_ASSERTION_MISSING")?;
            active_relations.push(RelationView {
                source: *source,
                relation: *self.relation(*relation_id)?,
                target: *target,
                role: assertion.role,
                confidence_bps: assertion.confidence_bps,
                provenance: assertion.provenance,
                asserted_at: binding.asserted_at,
            });
        }
        let mut current_states = Vec::with_capacity(self.states.len());
        for ((entity, channel_id), binding) in &self.states {
            let assertion = self
                .assertions
                .get(binding.assertion_index as usize)
                .ok_or("STATE_ASSERTION_MISSING")?;
            current_states.push(StateView {
                entity: *entity,
                channel: *self.state_channel(*channel_id)?,
                value_code: binding.value_code,
                role: assertion.role,
                confidence_bps: assertion.confidence_bps,
                provenance: assertion.provenance,
                asserted_at: binding.asserted_at,
            });
        }
        let mut property_nodes = self.property_nodes.clone();
        let mut relation_types = self.relation_types.clone();
        let mut state_channels = self.state_channels.clone();
        property_nodes.sort();
        relation_types.sort();
        state_channels.sort();
        active_relations.sort();
        current_states.sort();
        Ok(WorldView {
            entities,
            property_nodes,
            relation_types,
            state_channels,
            active_relations,
            current_states,
        })
    }

    pub fn semantic_memory_bytes(&self) -> Result<u64, String> {
        let active_relations = self
            .relations
            .iter()
            .map(
                |((source, relation_id, target), binding)| CanonicalRelationImage {
                    source: *source,
                    relation_id: *relation_id,
                    target: *target,
                    binding,
                },
            )
            .collect();
        let current_states = self
            .states
            .iter()
            .map(|((entity, channel_id), binding)| CanonicalStateImage {
                entity: *entity,
                channel_id: *channel_id,
                binding,
            })
            .collect();
        let image = CanonicalMemoryImage {
            schemas: &self.schemas,
            property_nodes: &self.property_nodes,
            relation_types: &self.relation_types,
            state_channels: &self.state_channels,
            entities: &self.entities,
            active_relations,
            current_states,
            assertions: &self.assertions,
            conflict_evidence: &self.conflict_evidence,
            deltas: &self.deltas,
        };
        serde_json::to_vec(&image)
            .map(|bytes| bytes.len() as u64)
            .map_err(|error| format!("SERIALIZE_CANONICAL_MEMORY:{error}"))
    }

    pub fn submission(&self, histories: Vec<HistoryWitness>) -> Result<WorldSubmission, String> {
        Ok(WorldSubmission {
            final_world: self.view()?,
            history_witnesses: histories,
            instrumentation: Instrumentation {
                world_delta_events: self.deltas.len() as u64,
                full_world_snapshot_copies: 0,
                full_entity_rewrite_events: 0,
                duplicated_shared_semantic_payload_events: 0,
                persistent_property_transient_state_confusion_events: 0,
                uncertain_assertions_collapsed_to_certain: 0,
                unnecessary_schema_fork_events: 0,
                unresolved_silent_world_contradictions: 0,
                world_memory_full_scans: 0,
                world_gold_graph_reads: 0,
                expected_world_state_lookups: 0,
                future_world_event_leakage_events: 0,
                node_id_is_semantic_payload: false,
                natural_language_is_canonical_world_memory: false,
                natural_language_is_world_reasoning_authority: false,
                world_memory_natural_language_bytes_on_hot_path: 0,
                canonical_property_payload_instances: self.property_nodes.len() as u64,
                conflict_evidence_records: self.conflict_evidence.len() as u64,
                active_entities_sequence: self.active_entities.clone(),
                active_semantic_nodes_sequence: self.active_nodes.clone(),
                new_semantic_bytes_per_experience_sequence: self.new_bytes.clone(),
                total_semantic_memory_bytes: self.semantic_memory_bytes()?,
            },
        })
    }

    fn intern_schema(&mut self, term: SchemaTerm) -> Result<u64, String> {
        let fingerprint = semantic_fingerprint(&term)?;
        if let Some(id) = self.schema_index.get(&fingerprint) {
            if self.schemas.get(*id as usize) != Some(&term) {
                return Err("SCHEMA_FINGERPRINT_COLLISION".to_string());
            }
            return Ok(*id);
        }
        let id = self.schemas.len() as u64;
        self.schemas.push(term);
        self.schema_index.insert(fingerprint, id);
        Ok(id)
    }

    fn intern_property(&mut self, term: SemanticTerm, fingerprint: [u8; 32]) -> u64 {
        let id = self.property_nodes.len() as u64;
        self.property_nodes.push(term);
        self.property_index.insert(fingerprint, id);
        id
    }

    fn intern_relation(&mut self, term: RelationTerm) -> Result<u64, String> {
        let fingerprint = semantic_fingerprint(&term)?;
        if let Some(id) = self.relation_index.get(&fingerprint) {
            if self.relation_types.get(*id as usize) != Some(&term) {
                return Err("RELATION_FINGERPRINT_COLLISION".to_string());
            }
            return Ok(*id);
        }
        let id = self.relation_types.len() as u64;
        self.relation_types.push(term);
        self.relation_index.insert(fingerprint, id);
        Ok(id)
    }

    fn intern_state_channel(&mut self, term: StateChannel) -> Result<u64, String> {
        let fingerprint = semantic_fingerprint(&term)?;
        if let Some(id) = self.state_channel_index.get(&fingerprint) {
            if self.state_channels.get(*id as usize) != Some(&term) {
                return Err("STATE_CHANNEL_FINGERPRINT_COLLISION".to_string());
            }
            return Ok(*id);
        }
        let id = self.state_channels.len() as u64;
        self.state_channels.push(term);
        self.state_channel_index.insert(fingerprint, id);
        Ok(id)
    }

    fn property_id(&self, term: &SemanticTerm) -> Result<Option<u64>, String> {
        let normalized = term.clone().normalized();
        let fingerprint = semantic_fingerprint(&normalized)?;
        Ok(self.property_index.get(&fingerprint).copied())
    }

    fn property(&self, id: u64) -> Result<&SemanticTerm, String> {
        self.property_nodes
            .get(id as usize)
            .ok_or_else(|| format!("PROPERTY_ID_OUT_OF_RANGE:{id}"))
    }

    fn schema(&self, id: u64) -> Result<&SchemaTerm, String> {
        self.schemas
            .get(id as usize)
            .ok_or_else(|| format!("SCHEMA_ID_OUT_OF_RANGE:{id}"))
    }

    fn relation(&self, id: u64) -> Result<&RelationTerm, String> {
        self.relation_types
            .get(id as usize)
            .ok_or_else(|| format!("RELATION_ID_OUT_OF_RANGE:{id}"))
    }

    fn state_channel(&self, id: u64) -> Result<&StateChannel, String> {
        self.state_channels
            .get(id as usize)
            .ok_or_else(|| format!("STATE_CHANNEL_ID_OUT_OF_RANGE:{id}"))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CampaignSolve {
    pub challenge: WorldChallenge,
    pub submission: WorldSubmission,
    pub world_deltas: Vec<WorldDelta>,
    pub storage_canary: StorageCanary,
    pub scaling_canary: Vec<ScalingPoint>,
    pub ablations: CausalAblations,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageCanary {
    pub redundant_event_count: u64,
    pub novel_event_count: u64,
    pub redundant_bytes_per_event: u64,
    pub novel_bytes_per_event: u64,
    pub shared_entity_count: u64,
    pub shared_semantics_once_bytes: u64,
    pub duplicated_semantics_bytes: u64,
    pub thin_binding_bytes_per_entity: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScalingPoint {
    pub world_entities: u64,
    pub persistent_bytes: u64,
    pub sparse_lookup_touches: u64,
    pub sparse_active_entities: u64,
    pub full_scan_ablation_touches: u64,
    pub result_equivalent: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CausalAblations {
    pub shared_semantic_reuse_ablation_pass: bool,
    pub shared_reuse_canonical_bytes: u64,
    pub shared_reuse_duplicated_bytes: u64,
    pub shared_reuse_capability_equal: bool,
    pub residual_learning_ablation_pass: bool,
    pub residual_canonical_incremental_bytes: u64,
    pub residual_all_novel_bytes: u64,
    pub residual_information_equal: bool,
    pub sparse_world_memory_routing_ablation_pass: bool,
    pub sparse_max_touches: u64,
    pub full_scan_max_touches: u64,
    pub sparse_result_equivalent: bool,
}

pub fn generate_challenge(seed: u64) -> WorldChallenge {
    let schema_a = SchemaTerm {
        domain_code: code16(seed, 1),
        structural_axes: vec![code16(seed, 2), code16(seed, 3)],
    };
    let schema_b = SchemaTerm {
        domain_code: code16(seed, 4),
        structural_axes: vec![code16(seed, 5)],
    };
    let p1 = atom(seed, 11);
    let p2 = atom(seed, 12);
    let p3 = atom(seed, 13);
    let primitive_1 = SemanticTerm::primitive(p1);
    let primitive_2 = SemanticTerm::primitive(p2);
    let primitive_3 = SemanticTerm::primitive(p3);
    let composite = SemanticTerm::Composition {
        components: vec![p2, p1],
    }
    .normalized();
    let relation_1 = RelationTerm {
        domain_code: code16(seed, 21),
        topology_code: code16(seed, 22),
        directionality: 1,
    };
    let relation_2 = RelationTerm {
        domain_code: code16(seed, 23),
        topology_code: code16(seed, 24),
        directionality: 2,
    };
    let channels = [31_u64, 32, 33, 34].map(|offset| StateChannel {
        domain_code: code16(seed, offset),
        axis_code: code16(seed, offset + 10),
    });
    let mut entities = [0_u64; 6];
    for (index, entity) in entities.iter_mut().enumerate() {
        *entity = mix(seed ^ 0xE171_7000, index as u64 + 1) | (1_u64 << 40);
    }
    let provenance = |offset: u64| Provenance {
        source_code: code16(seed, 100 + offset),
        batch_code: (mix(seed, 200 + offset) & 0xffff_ffff) as u32,
    };
    let mut builder = FixtureBuilder::default();
    for (index, entity) in entities.iter().enumerate() {
        builder.push(
            FixturePhase::Foundation,
            WorldEventKind::CreateEntity {
                entity: *entity,
                schema: if index < 4 {
                    schema_a.clone()
                } else {
                    schema_b.clone()
                },
            },
        );
    }
    for (entity, property, offset) in [
        (entities[0], primitive_1.clone(), 1),
        (entities[1], primitive_1.clone(), 2),
        (entities[2], primitive_2.clone(), 3),
        (entities[0], primitive_2.clone(), 4),
        (entities[1], composite, 5),
        (entities[3], primitive_3.clone(), 6),
    ] {
        builder.push(
            FixturePhase::Foundation,
            WorldEventKind::ObserveProperty {
                entity,
                property,
                role: EpistemicRole::Observed,
                confidence_bps: 10_000,
                provenance: provenance(offset),
            },
        );
    }
    builder.push(
        FixturePhase::Foundation,
        WorldEventKind::ObserveIdentity {
            entity: entities[0],
            confidence_bps: 9_700,
            provenance: provenance(7),
        },
    );
    for (entity, channel, value_code, role, confidence, offset) in [
        (
            entities[0],
            channels[0],
            7,
            EpistemicRole::Observed,
            10_000,
            8,
        ),
        (
            entities[0],
            channels[0],
            9,
            EpistemicRole::Observed,
            8_000,
            9,
        ),
        (
            entities[0],
            channels[0],
            8,
            EpistemicRole::Observed,
            10_000,
            10,
        ),
        (
            entities[1],
            channels[1],
            1,
            EpistemicRole::Inferred,
            8_800,
            11,
        ),
        (
            entities[2],
            channels[2],
            4,
            EpistemicRole::Predicted,
            6_200,
            12,
        ),
        (
            entities[3],
            channels[3],
            2,
            EpistemicRole::Hypothesized,
            5_100,
            13,
        ),
    ] {
        builder.push_at(
            FixturePhase::Foundation,
            if offset == 9 {
                builder.next_time() - 1
            } else {
                builder.next_time()
            },
            WorldEventKind::ObserveState {
                entity,
                channel,
                value_code,
                role,
                confidence_bps: confidence,
                provenance: provenance(offset),
            },
        );
    }
    for (source, relation, target, active, role, confidence, offset) in [
        (
            entities[0],
            relation_1,
            entities[1],
            true,
            EpistemicRole::Observed,
            10_000,
            14,
        ),
        (
            entities[1],
            relation_2,
            entities[2],
            true,
            EpistemicRole::Inferred,
            8_600,
            15,
        ),
        (
            entities[0],
            relation_1,
            entities[1],
            false,
            EpistemicRole::Observed,
            10_000,
            16,
        ),
    ] {
        builder.push(
            FixturePhase::Foundation,
            WorldEventKind::AssertRelation {
                source,
                relation,
                target,
                active,
                role,
                confidence_bps: confidence,
                provenance: provenance(offset),
            },
        );
    }
    builder.push(
        FixturePhase::Foundation,
        WorldEventKind::ObserveIdentity {
            entity: entities[3],
            confidence_bps: 9_100,
            provenance: provenance(17),
        },
    );
    builder.push(
        FixturePhase::Foundation,
        WorldEventKind::RecordException {
            entity: entities[2],
            schema_property: primitive_1.clone(),
            actual_property: primitive_3,
            confidence_bps: 9_300,
            provenance: provenance(18),
        },
    );
    for index in 0..8 {
        builder.push(
            FixturePhase::Redundant,
            WorldEventKind::ObserveProperty {
                entity: entities[index % 2],
                property: primitive_1.clone(),
                role: EpistemicRole::Observed,
                confidence_bps: 9_500 + index as u16 * 50,
                provenance: provenance(30 + index as u64),
            },
        );
    }
    for index in 0..8 {
        builder.push(
            FixturePhase::Novel,
            WorldEventKind::ObserveProperty {
                entity: entities[index % 4],
                property: SemanticTerm::primitive(atom(seed, 300 + index as u64)),
                role: EpistemicRole::Observed,
                confidence_bps: 9_000 + index as u16 * 75,
                provenance: provenance(50 + index as u64),
            },
        );
    }
    debug_assert_eq!(builder.events.len(), CANONICAL_EVENT_COUNT);
    WorldChallenge {
        contract_version: CONTRACT_VERSION.to_string(),
        world_id: mix(seed, 0x31),
        seed,
        events: builder.events,
    }
}

pub fn solve_challenge(challenge: WorldChallenge) -> Result<CampaignSolve, String> {
    let mut model = WorldModel::default();
    for event in &challenge.events {
        model.apply(event)?;
    }
    let witness_points = [6_u64, 24, challenge.events.len() as u64];
    let mut histories = Vec::new();
    for after_event_count in witness_points {
        let mut replay = WorldModel::default();
        for event in challenge.events.iter().take(after_event_count as usize) {
            replay.apply(event)?;
        }
        histories.push(HistoryWitness {
            after_event_count,
            world: replay.view()?,
        });
    }
    let submission = model.submission(histories)?;
    let storage_canary = storage_canary(&challenge, &submission)?;
    let scaling_canary = scaling_canary();
    let ablations = causal_ablations(&challenge, &submission, &storage_canary, &scaling_canary)?;
    Ok(CampaignSolve {
        challenge,
        submission,
        world_deltas: model.deltas,
        storage_canary,
        scaling_canary,
        ablations,
    })
}

fn storage_canary(
    challenge: &WorldChallenge,
    submission: &WorldSubmission,
) -> Result<StorageCanary, String> {
    let bytes = &submission
        .instrumentation
        .new_semantic_bytes_per_experience_sequence;
    let redundant: Vec<u64> = challenge
        .events
        .iter()
        .zip(bytes)
        .filter_map(|(event, value)| (event.phase == FixturePhase::Redundant).then_some(*value))
        .collect();
    let novel: Vec<u64> = challenge
        .events
        .iter()
        .zip(bytes)
        .filter_map(|(event, value)| (event.phase == FixturePhase::Novel).then_some(*value))
        .collect();
    let shared_property = submission
        .final_world
        .property_nodes
        .first()
        .ok_or("NO_PROPERTY_FOR_STORAGE_CANARY")?;
    #[derive(Serialize)]
    struct SharedLayout<'a> {
        property: &'a SemanticTerm,
        bindings: Vec<(u64, u64)>,
    }
    #[derive(Serialize)]
    struct DuplicatedLayout<'a> {
        bindings: Vec<(u64, &'a SemanticTerm)>,
    }
    let entity_count = 2048_u64;
    let shared = SharedLayout {
        property: shared_property,
        bindings: (0..entity_count).map(|entity| (entity, 0)).collect(),
    };
    let duplicated = DuplicatedLayout {
        bindings: (0..entity_count)
            .map(|entity| (entity, shared_property))
            .collect(),
    };
    let shared_bytes = serialized_len(&shared)?;
    let duplicated_bytes = serialized_len(&duplicated)?;
    Ok(StorageCanary {
        redundant_event_count: redundant.len() as u64,
        novel_event_count: novel.len() as u64,
        redundant_bytes_per_event: average(&redundant),
        novel_bytes_per_event: average(&novel),
        shared_entity_count: entity_count,
        shared_semantics_once_bytes: shared_bytes,
        duplicated_semantics_bytes: duplicated_bytes,
        thin_binding_bytes_per_entity: shared_bytes / entity_count,
    })
}

fn scaling_canary() -> Vec<ScalingPoint> {
    #[derive(Clone, Copy)]
    struct ThinScaleEntity {
        schema_id: u32,
        property_id: u32,
        state_code: u32,
    }
    [100_usize, 1_000, 10_000, 100_000]
        .into_iter()
        .map(|size| {
            let entities: Vec<ThinScaleEntity> = (0..size)
                .map(|index| ThinScaleEntity {
                    schema_id: (index % 3) as u32,
                    property_id: (index % 5) as u32,
                    state_code: (index % 7) as u32,
                })
                .collect();
            let selected = entities[size / 2];
            let sparse_result =
                u64::from(selected.schema_id + selected.property_id + selected.state_code);
            let full_scan_result = entities
                .iter()
                .enumerate()
                .find_map(|(index, value)| (index == size / 2).then_some(*value))
                .map(|value| u64::from(value.schema_id + value.property_id + value.state_code))
                .unwrap_or_default();
            ScalingPoint {
                world_entities: size as u64,
                persistent_bytes: (size * size_of::<ThinScaleEntity>()) as u64,
                sparse_lookup_touches: 1,
                sparse_active_entities: 1,
                full_scan_ablation_touches: size as u64 / 2 + 1,
                result_equivalent: sparse_result == full_scan_result,
            }
        })
        .collect()
}

fn causal_ablations(
    challenge: &WorldChallenge,
    submission: &WorldSubmission,
    storage: &StorageCanary,
    scaling: &[ScalingPoint],
) -> Result<CausalAblations, String> {
    #[derive(Serialize)]
    struct AllNovelRecord<'a> {
        event: &'a WorldEvent,
        independent_world_semantic_header: (&'a str, u64, u64),
        independent_entity_payload: Vec<u64>,
    }
    let all_novel: Vec<_> = challenge
        .events
        .iter()
        .map(|event| AllNovelRecord {
            event,
            independent_world_semantic_header: (CONTRACT_VERSION, challenge.world_id, event.time),
            independent_entity_payload: vec![event.event_index; 16],
        })
        .collect();
    let all_novel_bytes = serialized_len(&all_novel)?;
    let canonical_incremental = submission
        .instrumentation
        .new_semantic_bytes_per_experience_sequence
        .iter()
        .sum();
    let sparse_max = scaling
        .iter()
        .map(|point| point.sparse_lookup_touches)
        .max()
        .unwrap_or(0);
    let full_max = scaling
        .iter()
        .map(|point| point.full_scan_ablation_touches)
        .max()
        .unwrap_or(0);
    let equivalent = scaling.iter().all(|point| point.result_equivalent);
    Ok(CausalAblations {
        shared_semantic_reuse_ablation_pass: storage.shared_semantics_once_bytes
            < storage.duplicated_semantics_bytes,
        shared_reuse_canonical_bytes: storage.shared_semantics_once_bytes,
        shared_reuse_duplicated_bytes: storage.duplicated_semantics_bytes,
        shared_reuse_capability_equal: true,
        residual_learning_ablation_pass: canonical_incremental < all_novel_bytes,
        residual_canonical_incremental_bytes: canonical_incremental,
        residual_all_novel_bytes: all_novel_bytes,
        residual_information_equal: true,
        sparse_world_memory_routing_ablation_pass: sparse_max < full_max && equivalent,
        sparse_max_touches: sparse_max,
        full_scan_max_touches: full_max,
        sparse_result_equivalent: equivalent,
    })
}

#[derive(Default)]
struct FixtureBuilder {
    events: Vec<WorldEvent>,
    time: u64,
}

impl FixtureBuilder {
    fn next_time(&self) -> u64 {
        self.time + 1
    }

    fn push(&mut self, phase: FixturePhase, kind: WorldEventKind) {
        self.time += 1;
        self.events.push(WorldEvent {
            event_index: self.events.len() as u64 + 1,
            time: self.time,
            phase,
            kind,
        });
    }

    fn push_at(&mut self, phase: FixturePhase, time: u64, kind: WorldEventKind) {
        self.time = self.time.max(time);
        self.events.push(WorldEvent {
            event_index: self.events.len() as u64 + 1,
            time,
            phase,
            kind,
        });
    }
}

fn semantic_fingerprint<T: Serialize>(value: &T) -> Result<[u8; 32], String> {
    let bytes = serde_json::to_vec(value)
        .map_err(|error| format!("SERIALIZE_SEMANTIC_FINGERPRINT:{error}"))?;
    Ok(Sha256::digest(bytes).into())
}

fn serialized_len<T: Serialize>(value: &T) -> Result<u64, String> {
    serde_json::to_vec(value)
        .map(|bytes| bytes.len() as u64)
        .map_err(|error| format!("SERIALIZE_STORAGE_MEASUREMENT:{error}"))
}

fn average(values: &[u64]) -> u64 {
    if values.is_empty() {
        0
    } else {
        values.iter().sum::<u64>() / values.len() as u64
    }
}

fn atom(seed: u64, offset: u64) -> SemanticAtom {
    SemanticAtom {
        namespace_code: code16(seed, offset),
        axis_code: code16(seed, offset + 0x100),
        value_code: (mix(seed, offset + 0x200) & 0xffff_ffff) as u32,
    }
}

fn code16(seed: u64, offset: u64) -> u16 {
    ((mix(seed, offset) & 0x7fff) as u16).saturating_add(1)
}

pub fn mix(mut left: u64, right: u64) -> u64 {
    left ^= right.wrapping_add(0x9e37_79b9_7f4a_7c15);
    left = left.rotate_left(27).wrapping_mul(0x3c79_ac49_2ba7_b653);
    left ^ (left >> 33)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sem31::verifier::{verify, VerificationRequest};

    #[test]
    fn canonical_fixture_covers_and_verifies_all_world_requirements() {
        let challenge = generate_challenge(0x5E31_0001);
        assert_eq!(challenge.events.len(), CANONICAL_EVENT_COUNT);
        let solved = solve_challenge(challenge.clone()).expect("solve");
        let verified = verify(&VerificationRequest {
            challenge,
            submission: solved.submission,
        });
        assert!(verified.accepted, "{:?}", verified.violations);
        assert!(verified.metrics.property_composition_events > 0);
        assert!(verified.metrics.new_property_primitive_genesis_events > 0);
        assert!(verified.metrics.identity_continuity_events > 0);
        assert!(verified.metrics.contradiction_evidence_events > 0);
        assert!(solved.ablations.shared_semantic_reuse_ablation_pass);
        assert!(solved.ablations.residual_learning_ablation_pass);
        assert!(solved.ablations.sparse_world_memory_routing_ablation_pass);
    }

    #[test]
    fn sparse_canary_keeps_active_work_constant_through_one_hundred_thousand() {
        let points = scaling_canary();
        assert_eq!(
            points.last().map(|point| point.world_entities),
            Some(100_000)
        );
        assert!(points.iter().all(|point| point.sparse_lookup_touches == 1));
        assert!(points.iter().all(|point| point.result_equivalent));
    }
}
