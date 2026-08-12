use std::collections::{HashMap, HashSet, VecDeque};

use serde::{Deserialize, Serialize};

pub type NeuronId = usize;
const EMOTION_COUNT: usize = 5;
const DESIRE_COUNT: usize = 5;
const GOAL_COUNT: usize = 4;
const CONCEPT_FULL_SCAN_LIMIT: usize = 32;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EmotionKind {
    Curiosity,
    Anxiety,
    Trust,
    Loneliness,
    Joy,
}

impl EmotionKind {
    fn index(self) -> usize {
        match self {
            EmotionKind::Curiosity => 0,
            EmotionKind::Anxiety => 1,
            EmotionKind::Trust => 2,
            EmotionKind::Loneliness => 3,
            EmotionKind::Joy => 4,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DesireKind {
    Learning,
    Relationship,
    Growth,
    Recognition,
    Safety,
}

impl DesireKind {
    fn index(self) -> usize {
        match self {
            DesireKind::Learning => 0,
            DesireKind::Relationship => 1,
            DesireKind::Growth => 2,
            DesireKind::Recognition => 3,
            DesireKind::Safety => 4,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum GoalKind {
    HelpUser,
    LearnMore,
    MaintainRelationship,
    PreserveCoherence,
}

impl GoalKind {
    fn index(self) -> usize {
        match self {
            GoalKind::HelpUser => 0,
            GoalKind::LearnMore => 1,
            GoalKind::MaintainRelationship => 2,
            GoalKind::PreserveCoherence => 3,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CognitiveState {
    emotions: [f32; EMOTION_COUNT],
    desires: [f32; DESIRE_COUNT],
    goals: [f32; GOAL_COUNT],
}

impl CognitiveState {
    pub fn new() -> Self {
        let mut state = Self {
            emotions: [0.0; EMOTION_COUNT],
            desires: [0.0; DESIRE_COUNT],
            goals: [0.0; GOAL_COUNT],
        };
        state.set_goal(GoalKind::HelpUser, 0.8);
        state.set_goal(GoalKind::PreserveCoherence, 0.7);
        state
    }

    pub fn set_emotion(&mut self, kind: EmotionKind, intensity: f32) {
        self.emotions[kind.index()] = clamp(intensity);
    }

    pub fn set_desire(&mut self, kind: DesireKind, intensity: f32) {
        self.desires[kind.index()] = clamp(intensity);
    }

    pub fn set_goal(&mut self, kind: GoalKind, intensity: f32) {
        self.goals[kind.index()] = clamp(intensity);
    }

    pub fn emotion(&self, kind: EmotionKind) -> f32 {
        self.emotions[kind.index()]
    }

    pub fn desire(&self, kind: DesireKind) -> f32 {
        self.desires[kind.index()]
    }

    pub fn goal(&self, kind: GoalKind) -> f32 {
        self.goals[kind.index()]
    }
}

impl Default for CognitiveState {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeModulation {
    emotion_relevance: [f32; EMOTION_COUNT],
    desire_relevance: [f32; DESIRE_COUNT],
    goal_relevance: [f32; GOAL_COUNT],
}

impl NodeModulation {
    pub fn new() -> Self {
        Self {
            emotion_relevance: [0.0; EMOTION_COUNT],
            desire_relevance: [0.0; DESIRE_COUNT],
            goal_relevance: [0.0; GOAL_COUNT],
        }
    }

    pub fn with_emotion(mut self, kind: EmotionKind, relevance: f32) -> Self {
        self.emotion_relevance[kind.index()] = clamp(relevance);
        self
    }

    pub fn with_desire(mut self, kind: DesireKind, relevance: f32) -> Self {
        self.desire_relevance[kind.index()] = clamp(relevance);
        self
    }

    pub fn with_goal(mut self, kind: GoalKind, relevance: f32) -> Self {
        self.goal_relevance[kind.index()] = clamp(relevance);
        self
    }

    fn modulation_factor(&self, state: &CognitiveState) -> f32 {
        let emotion = dot(&self.emotion_relevance, &state.emotions);
        let desire = dot(&self.desire_relevance, &state.desires);
        let goal = dot(&self.goal_relevance, &state.goals);
        0.30f32
            .mul_add(goal, 0.20f32.mul_add(desire, 0.25f32.mul_add(emotion, 1.0)))
            .clamp(0.25, 2.0)
    }

    fn goal_fit(&self, state: &CognitiveState) -> f32 {
        dot(&self.goal_relevance, &state.goals)
    }
}

impl Default for NodeModulation {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RelationType {
    Association,
    Support,
    Cause,
    Contradiction,
    Sequence,
    Hierarchy,
    Goal,
    Emotion,
}

impl RelationType {
    pub fn propagation_coefficient(self) -> f32 {
        match self {
            RelationType::Association => 1.00,
            RelationType::Support => 1.10,
            RelationType::Cause => 1.20,
            RelationType::Contradiction => 0.0,
            RelationType::Sequence => 1.05,
            RelationType::Hierarchy => 0.95,
            RelationType::Goal => 1.25,
            RelationType::Emotion => 1.15,
        }
    }

    pub fn reinforcement_coefficient(self) -> f32 {
        match self {
            RelationType::Association => 1.00,
            RelationType::Support => 1.10,
            RelationType::Cause => 1.20,
            RelationType::Contradiction => 0.80,
            RelationType::Sequence => 1.05,
            RelationType::Hierarchy => 0.95,
            RelationType::Goal => 1.25,
            RelationType::Emotion => 1.15,
        }
    }

    fn index_key(self) -> &'static str {
        match self {
            RelationType::Association => "association",
            RelationType::Support => "support",
            RelationType::Cause => "cause",
            RelationType::Contradiction => "contradiction",
            RelationType::Sequence => "sequence",
            RelationType::Hierarchy => "hierarchy",
            RelationType::Goal => "goal",
            RelationType::Emotion => "emotion",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NeuronMeta {
    pub key: String,
    pub kind: String,
    pub label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SynapseCore {
    meta: Vec<NeuronMeta>,
    key_to_id: HashMap<String, NeuronId>,
    activation: Vec<f32>,
    threshold: Vec<f32>,
    importance: Vec<f32>,
    plasticity: Vec<f32>,
    decay_rate: Vec<f32>,
    modulation: Vec<NodeModulation>,
    access_count: Vec<u64>,
    average_activation: Vec<f32>,
    node_idle_cycles: Vec<u32>,
    source_offsets: Vec<usize>,
    targets: Vec<NeuronId>,
    strengths: Vec<f32>,
    relations: Vec<RelationType>,
    edge_activation_count: Vec<u64>,
    edge_idle_cycles: Vec<u32>,
    last_connected_source: Option<NeuronId>,
    activation_floor: f32,
    resonance_threshold: f32,
    stable_cycles_required: usize,
    max_cycles: usize,
    cluster_depth: usize,
    learning_rate: f32,
    forgetting_rate: f32,
    consolidation_rate: f32,
    synapse_forgetting_rate: f32,
    crystal_counts: HashMap<String, u32>,
    thought_crystals: Vec<ThoughtCrystal>,
    corrections: Vec<CorrectionRecord>,
    contradiction_edges: Vec<ContradictionEdge>,
    reflexes: Vec<ReflexCircuit>,
    reflex_threshold: u32,
    cognitive_state: CognitiveState,
    concept_schemas: Vec<ConceptSchema>,
    concept_index: HashMap<String, Vec<usize>>,
    token_index: HashMap<String, Vec<NeuronId>>,
    phrase_index: HashMap<String, Vec<NeuronId>>,
    entity_index: HashMap<String, Vec<NeuronId>>,
    relation_index: HashMap<String, Vec<NeuronId>>,
    domain_index: HashMap<String, Vec<NeuronId>>,
    emotion_relevance_index: HashMap<String, Vec<NeuronId>>,
    desire_relevance_index: HashMap<String, Vec<NeuronId>>,
    goal_relevance_index: HashMap<String, Vec<NeuronId>>,
    reflex_trigger_index: HashMap<String, Vec<NeuronId>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterHypothesis {
    pub root_id: NeuronId,
    pub node_ids: Vec<NeuronId>,
    pub score: f32,
    pub relation_strength: f32,
    pub importance_bonus: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThoughtCrystal {
    pub id: String,
    pub label: String,
    pub source_node_ids: Vec<NeuronId>,
    pub activation: f32,
    pub confidence: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorrectionRecord {
    pub stimulus_signature: String,
    pub wrong_crystal_id: String,
    pub wrong_schema_id: Option<String>,
    pub alternative_schema_id: String,
    pub contradiction_strength: f32,
    pub confidence_delta: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContradictionEdge {
    pub source_crystal_id: String,
    pub source_schema_id: Option<String>,
    pub target_schema_id: String,
    pub stimulus_signature: String,
    pub strength: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReflexCircuit {
    pub crystal_id: String,
    pub label: String,
    pub source_node_ids: Vec<NeuronId>,
    pub trigger_count: u32,
    pub confidence: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResonanceResult {
    pub achieved: bool,
    pub reflex_hit: bool,
    pub cycles: usize,
    pub mean_delta_activation: f32,
    pub active_node_ids: Vec<NeuronId>,
    pub winning_cluster: Option<ClusterHypothesis>,
    pub thought_crystal: Option<ThoughtCrystal>,
    pub concept_recalls: Vec<ConceptRecallResult>,
    pub generalization: Option<GeneralizationResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryConsolidationReport {
    pub strengthened_nodes: usize,
    pub weakened_nodes: usize,
    pub weakened_synapses: usize,
    pub promoted_concepts: usize,
    pub reflex_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DefinitionCue {
    pub cue: String,
    pub role: String,
    pub weight: f32,
}

impl DefinitionCue {
    pub fn new(cue: impl Into<String>, role: impl Into<String>, weight: f32) -> Self {
        Self {
            cue: cue.into(),
            role: role.into(),
            weight: clamp(weight),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConceptSchema {
    pub id: String,
    pub label: String,
    pub definition: String,
    pub domain: String,
    pub abstraction_level: f32,
    pub importance: f32,
    pub reflex_bonus: f32,
    pub cues: Vec<DefinitionCue>,
}

impl ConceptSchema {
    pub fn new(
        id: impl Into<String>,
        label: impl Into<String>,
        definition: impl Into<String>,
        domain: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            definition: definition.into(),
            domain: domain.into(),
            abstraction_level: 0.6,
            importance: 0.5,
            reflex_bonus: 0.0,
            cues: Vec::new(),
        }
    }

    pub fn with_abstraction_level(mut self, value: f32) -> Self {
        self.abstraction_level = clamp(value);
        self
    }

    pub fn with_importance(mut self, value: f32) -> Self {
        self.importance = clamp(value);
        self
    }

    pub fn with_reflex_bonus(mut self, value: f32) -> Self {
        self.reflex_bonus = clamp(value);
        self
    }

    pub fn with_cue(
        mut self,
        cue: impl Into<String>,
        role: impl Into<String>,
        weight: f32,
    ) -> Self {
        self.cues.push(DefinitionCue::new(cue, role, weight));
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextBinding {
    pub cue: String,
    pub role: String,
    pub evidence: String,
    pub score: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConceptRecallResult {
    pub schema_id: String,
    pub label: String,
    pub definition: String,
    pub domain: String,
    pub score: f32,
    pub context_fit: f32,
    pub bindings: Vec<ContextBinding>,
    pub interpretation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneralizationResult {
    pub source_schema_ids: Vec<String>,
    pub synthesized_label: String,
    pub interpretation: String,
    pub confidence: f32,
    pub thought_crystal: ThoughtCrystal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorrectionReport {
    pub corrected: bool,
    pub wrong_confidence_after: f32,
    pub contradiction_edges: usize,
    pub alternative_schema_id: String,
    pub alternative_importance_after: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivationIndexReport {
    pub total_nodes: usize,
    pub candidate_nodes: usize,
    pub active_field_size: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NeuronParams {
    pub threshold: f32,
    pub importance: f32,
    pub plasticity: f32,
    pub decay_rate: f32,
    pub modulation: NodeModulation,
}

impl Default for NeuronParams {
    fn default() -> Self {
        Self {
            threshold: 0.5,
            importance: 0.5,
            plasticity: 0.5,
            decay_rate: 0.15,
            modulation: NodeModulation::default(),
        }
    }
}

impl SynapseCore {
    pub fn new() -> Self {
        Self {
            meta: Vec::new(),
            key_to_id: HashMap::new(),
            activation: Vec::new(),
            threshold: Vec::new(),
            importance: Vec::new(),
            plasticity: Vec::new(),
            decay_rate: Vec::new(),
            modulation: Vec::new(),
            access_count: Vec::new(),
            average_activation: Vec::new(),
            node_idle_cycles: Vec::new(),
            source_offsets: vec![0],
            targets: Vec::new(),
            strengths: Vec::new(),
            relations: Vec::new(),
            edge_activation_count: Vec::new(),
            edge_idle_cycles: Vec::new(),
            last_connected_source: None,
            activation_floor: 0.03,
            resonance_threshold: 0.01,
            stable_cycles_required: 2,
            max_cycles: 12,
            cluster_depth: 2,
            learning_rate: 0.04,
            forgetting_rate: 0.015,
            consolidation_rate: 0.025,
            synapse_forgetting_rate: 0.01,
            crystal_counts: HashMap::new(),
            thought_crystals: Vec::new(),
            corrections: Vec::new(),
            contradiction_edges: Vec::new(),
            reflexes: Vec::new(),
            reflex_threshold: 3,
            cognitive_state: CognitiveState::default(),
            concept_schemas: Vec::new(),
            concept_index: HashMap::new(),
            token_index: HashMap::new(),
            phrase_index: HashMap::new(),
            entity_index: HashMap::new(),
            relation_index: HashMap::new(),
            domain_index: HashMap::new(),
            emotion_relevance_index: HashMap::new(),
            desire_relevance_index: HashMap::new(),
            goal_relevance_index: HashMap::new(),
            reflex_trigger_index: HashMap::new(),
        }
    }

    pub fn add_node(
        &mut self,
        key: impl Into<String>,
        kind: impl Into<String>,
        label: impl Into<String>,
        params: NeuronParams,
    ) -> NeuronId {
        let key = key.into();
        let id = self.meta.len();
        self.key_to_id.insert(key.clone(), id);
        self.meta.push(NeuronMeta {
            key,
            kind: kind.into(),
            label: label.into(),
        });
        self.activation.push(0.0);
        self.threshold.push(clamp(params.threshold));
        self.importance.push(clamp(params.importance));
        self.plasticity.push(clamp(params.plasticity));
        self.decay_rate.push(clamp(params.decay_rate));
        self.modulation.push(params.modulation);
        self.access_count.push(0);
        self.average_activation.push(0.0);
        self.node_idle_cycles.push(0);
        self.source_offsets.push(self.targets.len());
        self.index_node(id);
        id
    }

    pub fn connect(
        &mut self,
        source: NeuronId,
        target: NeuronId,
        strength: f32,
        relation: RelationType,
    ) {
        assert!(source < self.meta.len(), "source node does not exist");
        assert!(target < self.meta.len(), "target node does not exist");
        if let Some(previous_source) = self.last_connected_source {
            assert!(
                source >= previous_source,
                "synapses must be added in nondecreasing source-id order for compact CSR construction"
            );
        }
        self.targets.push(target);
        self.strengths.push(clamp(strength));
        self.relations.push(relation);
        self.edge_activation_count.push(0);
        self.edge_idle_cycles.push(0);
        self.last_connected_source = Some(source);
        for offset in self.source_offsets.iter_mut().skip(source + 1) {
            *offset += 1;
        }
        self.index_relation(source, target, relation);
    }

    pub fn connect_bidirectional(
        &mut self,
        a: NeuronId,
        b: NeuronId,
        strength: f32,
        relation: RelationType,
    ) {
        self.connect(a, b, strength, relation);
        self.connect(b, a, strength, relation);
    }

    pub fn node(&self, id: NeuronId) -> &NeuronMeta {
        &self.meta[id]
    }

    pub fn activation(&self, id: NeuronId) -> f32 {
        self.activation[id]
    }

    pub fn importance(&self, id: NeuronId) -> f32 {
        self.importance[id]
    }

    pub fn reflexes(&self) -> &[ReflexCircuit] {
        &self.reflexes
    }

    pub fn concept_schemas(&self) -> &[ConceptSchema] {
        &self.concept_schemas
    }

    pub fn add_concept_schema(&mut self, schema: ConceptSchema) -> usize {
        let id = self.concept_schemas.len();
        for token in concept_tokens(&schema.label)
            .into_iter()
            .chain(concept_tokens(&schema.definition))
            .chain(concept_tokens(&schema.domain))
            .chain(schema.cues.iter().flat_map(|cue| concept_tokens(&cue.cue)))
        {
            self.concept_index.entry(token).or_default().push(id);
        }
        self.concept_schemas.push(schema);
        id
    }

    pub fn cognitive_state(&self) -> &CognitiveState {
        &self.cognitive_state
    }

    pub fn set_emotion(&mut self, kind: EmotionKind, intensity: f32) {
        self.cognitive_state.set_emotion(kind, intensity);
    }

    pub fn set_desire(&mut self, kind: DesireKind, intensity: f32) {
        self.cognitive_state.set_desire(kind, intensity);
    }

    pub fn set_goal(&mut self, kind: GoalKind, intensity: f32) {
        self.cognitive_state.set_goal(kind, intensity);
    }

    pub fn modulation_factor(&self, id: NeuronId) -> f32 {
        self.modulation[id].modulation_factor(&self.cognitive_state)
    }

    pub fn stimulate(&mut self, id: NeuronId, amount: f32) {
        self.activation[id] = self.activation[id].max(clamp(amount));
        self.access_count[id] += 1;
    }

    pub fn activation_index_report(&self, stimulus: &str, limit: usize) -> ActivationIndexReport {
        let candidates = self.activation_candidates(stimulus);
        let active_field_size = self
            .score_activation_candidates(stimulus, &candidates, limit)
            .len();
        ActivationIndexReport {
            total_nodes: self.meta.len(),
            candidate_nodes: candidates.len(),
            active_field_size,
        }
    }

    pub fn activate(&mut self, stimulus: &str, limit: usize) -> Vec<NeuronId> {
        let candidates = self.activation_candidates(stimulus);
        let scored = self.score_activation_candidates(stimulus, &candidates, limit);

        for (score, id) in &scored {
            self.stimulate(*id, *score);
        }

        scored.into_iter().map(|(_, id)| id).collect()
    }

    pub fn activation_candidates(&self, stimulus: &str) -> Vec<NeuronId> {
        let mut candidates = Vec::new();
        let tokens = concept_tokens(stimulus);

        for token in &tokens {
            self.extend_index_hits(&mut candidates, &self.token_index, token);
            for prefix in token_prefixes(token) {
                self.extend_index_hits(&mut candidates, &self.token_index, &prefix);
                self.extend_index_hits(&mut candidates, &self.entity_index, &prefix);
            }
            self.extend_index_hits(&mut candidates, &self.entity_index, token);
            self.extend_index_hits(&mut candidates, &self.domain_index, token);
            self.extend_index_hits(&mut candidates, &self.relation_index, token);
            self.extend_index_hits(&mut candidates, &self.emotion_relevance_index, token);
            self.extend_index_hits(&mut candidates, &self.desire_relevance_index, token);
            self.extend_index_hits(&mut candidates, &self.goal_relevance_index, token);
            self.extend_index_hits(&mut candidates, &self.reflex_trigger_index, token);
        }

        for phrase in token_phrases(&tokens, 4) {
            self.extend_index_hits(&mut candidates, &self.phrase_index, &phrase);
            self.extend_index_hits(&mut candidates, &self.entity_index, &phrase);
        }

        for domain in domain_hints(stimulus) {
            self.extend_index_hits(&mut candidates, &self.domain_index, &domain);
        }

        candidates.sort_unstable();
        candidates.dedup();
        candidates
    }

    fn score_activation_candidates(
        &self,
        stimulus: &str,
        candidates: &[NeuronId],
        limit: usize,
    ) -> Vec<(f32, NeuronId)> {
        let mut scored = candidates
            .iter()
            .filter_map(|id| {
                let meta = &self.meta[*id];
                let similarity = crate::similarity::lexical_similarity(
                    stimulus,
                    [&meta.key, &meta.kind, &meta.label],
                );
                let score = similarity.max(index_hint_similarity(stimulus, meta))
                    * self.importance[*id]
                    * self.modulation_factor(*id);
                (score > self.activation_floor).then_some((score, *id))
            })
            .collect::<Vec<_>>();

        scored.sort_by(|a, b| b.0.total_cmp(&a.0));
        scored.truncate(limit);
        scored
    }

    fn index_node(&mut self, id: NeuronId) {
        let meta = &self.meta[id];
        let key = meta.key.clone();
        let kind = meta.kind.clone();
        let label = meta.label.clone();

        for token in concept_tokens(&key)
            .into_iter()
            .chain(concept_tokens(&kind))
            .chain(concept_tokens(&label))
        {
            push_index(&mut self.token_index, token, id);
        }

        for phrase in [key.as_str(), kind.as_str(), label.as_str()] {
            let normalized = normalize_phrase(phrase);
            if !normalized.is_empty() {
                push_index(&mut self.phrase_index, normalized.clone(), id);
                if looks_like_entity(phrase) {
                    push_index(&mut self.entity_index, normalized, id);
                }
            }
        }

        for domain in domain_hints(&format!("{key} {kind} {label}")) {
            push_index(&mut self.domain_index, domain, id);
        }
        push_index(&mut self.domain_index, normalize_phrase(&kind), id);

        for (name, relevance) in emotion_relevance_pairs(&self.modulation[id]) {
            if relevance > 0.0 {
                push_index(&mut self.emotion_relevance_index, name.to_string(), id);
            }
        }
        for (name, relevance) in desire_relevance_pairs(&self.modulation[id]) {
            if relevance > 0.0 {
                push_index(&mut self.desire_relevance_index, name.to_string(), id);
            }
        }
        for (name, relevance) in goal_relevance_pairs(&self.modulation[id]) {
            if relevance > 0.0 {
                push_index(&mut self.goal_relevance_index, name.to_string(), id);
            }
        }

        if kind.contains("reflex") {
            push_index(&mut self.reflex_trigger_index, normalize_phrase(&label), id);
        }
    }

    fn index_relation(&mut self, source: NeuronId, target: NeuronId, relation: RelationType) {
        let relation_key = relation.index_key().to_string();
        push_index(&mut self.relation_index, relation_key.clone(), source);
        push_index(&mut self.relation_index, relation_key, target);
    }

    fn extend_index_hits(
        &self,
        candidates: &mut Vec<NeuronId>,
        index: &HashMap<String, Vec<NeuronId>>,
        key: &str,
    ) {
        if let Some(ids) = index.get(key) {
            candidates.extend(ids.iter().copied());
        }
    }

    pub fn reset_activation(&mut self) {
        self.activation.fill(0.0);
    }

    pub fn propagate_once(&mut self) -> f32 {
        let previous = self.activation.clone();
        let mut next = previous.clone();

        for source in self.active_ids_from(&previous) {
            let source_activation = previous[source];
            let source_decay = 1.0 - self.decay_rate[source];
            for edge in self.edge_range(source) {
                let relation = self.relations[edge];
                if relation == RelationType::Contradiction {
                    continue;
                }
                let target = self.targets[edge];
                let propagated = source_activation
                    * self.strengths[edge]
                    * source_decay
                    * relation.propagation_coefficient();
                next[target] = next[target].max(clamp(propagated));
                if propagated > self.activation_floor {
                    self.edge_activation_count[edge] += 1;
                    self.edge_idle_cycles[edge] = 0;
                }
            }
        }

        let mut delta = 0.0;
        for (id, value) in next.iter_mut().enumerate() {
            if *value <= self.activation_floor {
                *value = 0.0;
            }
            delta += (*value - self.activation[id]).abs();
            self.activation[id] = clamp(*value);
        }

        delta / self.activation.len().max(1) as f32
    }

    pub fn inhibit(&mut self) -> f32 {
        let previous = self.activation.clone();
        let mut delta = 0.0;

        for source in self.active_ids_from(&previous) {
            for edge in self.edge_range(source) {
                if self.relations[edge] != RelationType::Contradiction {
                    continue;
                }
                let target = self.targets[edge];
                let before = self.activation[target];
                let pressure = previous[source] * self.strengths[edge];
                self.activation[target] = clamp(self.activation[target] - pressure);
                if self.activation[target] < self.activation_floor {
                    self.activation[target] = 0.0;
                }
                delta += (before - self.activation[target]).abs();
            }
        }

        delta / self.activation.len().max(1) as f32
    }

    pub fn reinforce_active_synapses(&mut self) {
        let active = self.activation.clone();
        for source in self.active_ids_from(&active) {
            for edge in self.edge_range(source) {
                let target = self.targets[edge];
                if active[target] <= self.activation_floor {
                    continue;
                }
                let relation_weight = self.relations[edge].reinforcement_coefficient();
                let delta = self.learning_rate
                    * self.plasticity[source]
                    * self.plasticity[target]
                    * active[source]
                    * active[target]
                    * relation_weight;
                self.strengths[edge] = clamp(self.strengths[edge] + delta);
                self.edge_activation_count[edge] += 1;
                self.edge_idle_cycles[edge] = 0;
            }
        }
    }

    pub fn compete(&self) -> Vec<ClusterHypothesis> {
        let mut clusters = self
            .active_node_ids()
            .into_iter()
            .map(|root| self.build_cluster(root))
            .collect::<Vec<_>>();
        clusters.sort_by(|a, b| b.score.total_cmp(&a.score));
        clusters
    }

    pub fn resonate(&mut self, stimulus: &str) -> ResonanceResult {
        let concept_recalls = self.recall_concepts(stimulus, 3);
        if let Some(crystal) = self.try_reflex(stimulus) {
            self.reset_activation();
            for id in &crystal.source_node_ids {
                self.stimulate(*id, crystal.activation);
            }
            return ResonanceResult {
                achieved: true,
                reflex_hit: true,
                cycles: 0,
                mean_delta_activation: 0.0,
                active_node_ids: self.active_node_ids(),
                winning_cluster: None,
                thought_crystal: Some(crystal),
                concept_recalls,
                generalization: None,
            };
        }
        if let Some((crystal, promoted_to_reflex)) = self.try_thought_crystal(&concept_recalls) {
            self.reset_activation();
            for id in &crystal.source_node_ids {
                self.stimulate(*id, crystal.activation);
            }
            return ResonanceResult {
                achieved: true,
                reflex_hit: promoted_to_reflex,
                cycles: if promoted_to_reflex { 0 } else { 1 },
                mean_delta_activation: 0.0,
                active_node_ids: self.active_node_ids(),
                winning_cluster: None,
                thought_crystal: Some(crystal),
                concept_recalls,
                generalization: None,
            };
        }

        self.reset_activation();
        self.activate(stimulus, 8);

        let mut stable_cycles = 0;
        let mut mean_delta = 1.0;
        let mut winning_cluster = None;

        for cycle in 1..=self.max_cycles {
            mean_delta = self.propagate_once() + self.inhibit();
            self.reinforce_active_synapses();
            self.update_memory_traces();
            winning_cluster = self.compete().into_iter().next();

            if mean_delta < self.resonance_threshold {
                stable_cycles += 1;
            } else {
                stable_cycles = 0;
            }

            if stable_cycles >= self.stable_cycles_required {
                let crystal = if let Some(cluster) = winning_cluster.as_ref() {
                    self.crystallize(cluster)
                } else {
                    None
                };
                let active_node_ids = self.active_node_ids();
                let generalization =
                    self.generalize_from_concepts(stimulus, &concept_recalls, &active_node_ids);
                return ResonanceResult {
                    achieved: true,
                    reflex_hit: false,
                    cycles: cycle,
                    mean_delta_activation: mean_delta,
                    active_node_ids,
                    winning_cluster,
                    thought_crystal: crystal,
                    concept_recalls,
                    generalization,
                };
            }
        }

        let crystal = if let Some(cluster) = winning_cluster.as_ref() {
            self.crystallize(cluster)
        } else {
            None
        };
        let active_node_ids = self.active_node_ids();
        let generalization =
            self.generalize_from_concepts(stimulus, &concept_recalls, &active_node_ids);
        ResonanceResult {
            achieved: false,
            reflex_hit: false,
            cycles: self.max_cycles,
            mean_delta_activation: mean_delta,
            active_node_ids,
            winning_cluster,
            thought_crystal: crystal,
            concept_recalls,
            generalization,
        }
    }

    pub fn active_node_ids(&self) -> Vec<NeuronId> {
        let mut ids = self
            .activation
            .iter()
            .enumerate()
            .filter_map(|(id, activation)| (*activation > self.activation_floor).then_some(id))
            .collect::<Vec<_>>();
        ids.sort_by(|a, b| self.activation[*b].total_cmp(&self.activation[*a]));
        ids
    }

    pub fn sleep_cycle(&mut self) -> MemoryConsolidationReport {
        let total_access = self.access_count.iter().copied().sum::<u64>().max(1) as f32;
        let mut strengthened_nodes = 0;
        let mut weakened_nodes = 0;

        for id in 0..self.meta.len() {
            let access = self.access_count[id] as f32 / total_access;
            let retention = clamp(0.05f32.mul_add(
                self.plasticity[id],
                0.20f32.mul_add(
                    access,
                    0.30f32.mul_add(self.average_activation[id], 0.45 * self.importance[id]),
                ),
            ));
            let idle_pressure = (self.node_idle_cycles[id] as f32 / 64.0).min(1.0);
            let consolidation =
                self.consolidation_rate * self.average_activation[id] * self.plasticity[id];
            let forgetting = self.forgetting_rate * (1.0 - retention) * idle_pressure;
            let before = self.importance[id];
            self.importance[id] = clamp(self.importance[id] + consolidation - forgetting);

            if self.importance[id] > before {
                strengthened_nodes += 1;
            } else if self.importance[id] < before {
                weakened_nodes += 1;
            }
        }

        let mut weakened_synapses = 0;
        for edge in 0..self.strengths.len() {
            self.edge_idle_cycles[edge] = self.edge_idle_cycles[edge].saturating_add(1);
            let idle_pressure = (self.edge_idle_cycles[edge] as f32 / 64.0).min(1.0);
            let before = self.strengths[edge];
            self.strengths[edge] = clamp(
                self.strengths[edge] * self.synapse_forgetting_rate.mul_add(-idle_pressure, 1.0),
            );
            if self.strengths[edge] < before {
                weakened_synapses += 1;
            }
        }
        let promoted_concepts = self.promote_reflexes_to_concepts();

        MemoryConsolidationReport {
            strengthened_nodes,
            weakened_nodes,
            weakened_synapses,
            promoted_concepts,
            reflex_count: self.reflexes.len(),
        }
    }

    pub fn recall_concepts(&self, stimulus: &str, limit: usize) -> Vec<ConceptRecallResult> {
        let mut candidate_ids = Vec::new();
        for token in concept_tokens(stimulus) {
            if let Some(ids) = self.concept_index.get(&token) {
                candidate_ids.extend(ids.iter().copied());
            }
        }
        candidate_ids.sort_unstable();
        candidate_ids.dedup();
        if candidate_ids.is_empty() && self.concept_schemas.len() <= CONCEPT_FULL_SCAN_LIMIT {
            candidate_ids.extend(0..self.concept_schemas.len());
        }

        let mut results = candidate_ids
            .into_iter()
            .filter_map(|id| self.recall_schema(stimulus, &self.concept_schemas[id]))
            .collect::<Vec<_>>();
        results.sort_by(|a, b| b.score.total_cmp(&a.score));
        results.truncate(limit);
        results
    }

    pub fn apply_feedback(
        &mut self,
        stimulus: &str,
        wrong_crystal_id: &str,
        alternative_schema_id: &str,
    ) -> CorrectionReport {
        let stimulus_signature = correction_signature(stimulus);
        let mut wrong_confidence_after = 0.0;

        for crystal in &mut self.thought_crystals {
            if crystal.id == wrong_crystal_id {
                crystal.confidence = clamp(crystal.confidence * 0.35);
                crystal.activation = clamp(crystal.activation * 0.35);
                wrong_confidence_after = crystal.confidence;
            }
        }
        if let Some(count) = self.crystal_counts.get_mut(wrong_crystal_id) {
            *count = count.saturating_sub(1);
        }
        self.reflexes
            .retain(|reflex| reflex.crystal_id != wrong_crystal_id);

        let wrong_schema_id = self
            .concept_schemas
            .iter()
            .find(|schema| {
                schema.id != alternative_schema_id
                    && crate::similarity::lexical_similarity(stimulus, [&schema.label]).max(
                        crate::similarity::lexical_similarity(stimulus, [&schema.definition]),
                    ) > 0.05
            })
            .map(|schema| schema.id.clone());

        if let Some(schema_id) = &wrong_schema_id {
            if let Some(schema) = self
                .concept_schemas
                .iter_mut()
                .find(|schema| schema.id == *schema_id)
            {
                schema.importance = clamp(schema.importance * 0.55);
                schema.reflex_bonus = clamp(schema.reflex_bonus * 0.4);
            }
        }

        let mut alternative_importance_after = 0.0;
        if let Some(schema) = self
            .concept_schemas
            .iter_mut()
            .find(|schema| schema.id == alternative_schema_id)
        {
            schema.importance = clamp(schema.importance + 0.25);
            schema.reflex_bonus = clamp(schema.reflex_bonus + 0.35);
            alternative_importance_after = schema.importance;
        }

        self.contradiction_edges.push(ContradictionEdge {
            source_crystal_id: wrong_crystal_id.to_string(),
            source_schema_id: wrong_schema_id.clone(),
            target_schema_id: alternative_schema_id.to_string(),
            stimulus_signature: stimulus_signature.clone(),
            strength: 1.0,
        });

        self.corrections.push(CorrectionRecord {
            stimulus_signature,
            wrong_crystal_id: wrong_crystal_id.to_string(),
            wrong_schema_id,
            alternative_schema_id: alternative_schema_id.to_string(),
            contradiction_strength: 1.0,
            confidence_delta: -0.65,
        });

        CorrectionReport {
            corrected: true,
            wrong_confidence_after,
            contradiction_edges: self.contradiction_edges.len(),
            alternative_schema_id: alternative_schema_id.to_string(),
            alternative_importance_after,
        }
    }

    fn recall_schema(&self, stimulus: &str, schema: &ConceptSchema) -> Option<ConceptRecallResult> {
        let total_weight = schema
            .cues
            .iter()
            .map(|cue| cue.weight)
            .sum::<f32>()
            .max(0.001);
        let mut matched_weight = 0.0;
        let mut bindings = Vec::new();

        for cue in &schema.cues {
            let candidates = vec![cue.cue.clone()];
            let similarity = crate::similarity::lexical_similarity(stimulus, &candidates);
            if similarity > 0.05 {
                matched_weight = cue.weight.mul_add(similarity.max(0.35), matched_weight);
                bindings.push(ContextBinding {
                    cue: cue.cue.clone(),
                    role: cue.role.clone(),
                    evidence: cue.cue.clone(),
                    score: similarity,
                });
            }
        }

        let cue_overlap = (matched_weight / total_weight).clamp(0.0, 1.0);
        let definition_candidates = vec![
            schema.label.clone(),
            schema.definition.clone(),
            schema.domain.clone(),
        ];
        let definition_match =
            crate::similarity::lexical_similarity(stimulus, &definition_candidates);
        let context_fit = if schema.cues.is_empty() {
            definition_match
        } else {
            let total_roles = schema
                .cues
                .iter()
                .map(|cue| cue.role.as_str())
                .collect::<HashSet<_>>()
                .len()
                .max(1) as f32;
            let matched_roles = bindings
                .iter()
                .map(|binding| binding.role.as_str())
                .collect::<HashSet<_>>()
                .len() as f32;
            matched_roles / total_roles
        };
        let mut score = 0.10f32.mul_add(
            schema.reflex_bonus,
            0.10f32.mul_add(
                schema.abstraction_level,
                0.15f32.mul_add(
                    schema.importance,
                    0.20f32.mul_add(definition_match, 0.45 * cue_overlap),
                ),
            ),
        );
        score = clamp(score + self.correction_bias(stimulus, schema));

        if score <= 0.08 {
            return None;
        }

        Some(ConceptRecallResult {
            schema_id: schema.id.clone(),
            label: schema.label.clone(),
            definition: schema.definition.clone(),
            domain: schema.domain.clone(),
            score,
            context_fit,
            interpretation: interpretation_for(schema, &bindings),
            bindings,
        })
    }

    fn correction_bias(&self, stimulus: &str, schema: &ConceptSchema) -> f32 {
        let signature = correction_signature(stimulus);
        self.corrections
            .iter()
            .filter(|record| record.stimulus_signature == signature)
            .map(|record| {
                if record.alternative_schema_id == schema.id {
                    0.35 * record.contradiction_strength
                } else if record.wrong_schema_id.as_deref() == Some(schema.id.as_str()) {
                    -0.35 * record.contradiction_strength
                } else {
                    0.0
                }
            })
            .sum::<f32>()
            .clamp(-0.75, 0.75)
    }

    fn build_cluster(&self, root: NeuronId) -> ClusterHypothesis {
        let mut visited = vec![false; self.meta.len()];
        let mut nodes = Vec::new();
        let mut queue = VecDeque::new();
        let mut relation_strength = 0.0;

        visited[root] = true;
        nodes.push(root);
        queue.push_back((root, 0));

        while let Some((current, depth)) = queue.pop_front() {
            if depth >= self.cluster_depth {
                continue;
            }

            for edge in self.edge_range(current) {
                let target = self.targets[edge];
                if visited[target] || self.activation[target] <= self.activation_floor {
                    continue;
                }
                visited[target] = true;
                nodes.push(target);
                queue.push_back((target, depth + 1));
                relation_strength += self.strengths[edge];
            }
        }

        let activation_sum = nodes.iter().map(|id| self.activation[*id]).sum::<f32>();
        let importance_bonus =
            nodes.iter().map(|id| self.importance[*id]).sum::<f32>() / nodes.len().max(1) as f32;
        let goal_fit = nodes
            .iter()
            .map(|id| self.modulation[*id].goal_fit(&self.cognitive_state))
            .sum::<f32>()
            / nodes.len().max(1) as f32;
        let contradiction = self.contradiction_pressure(&nodes);
        let score = 0.35f32.mul_add(
            goal_fit,
            activation_sum + relation_strength + importance_bonus,
        ) - contradiction;

        ClusterHypothesis {
            root_id: root,
            node_ids: nodes,
            score,
            relation_strength,
            importance_bonus,
        }
    }

    fn crystallize(&mut self, cluster: &ClusterHypothesis) -> Option<ThoughtCrystal> {
        if cluster.node_ids.is_empty() || cluster.score <= self.activation_floor {
            return None;
        }

        let mut source_node_ids = cluster.node_ids.clone();
        source_node_ids.sort_by(|a, b| self.activation[*b].total_cmp(&self.activation[*a]));
        source_node_ids.truncate(5);

        let label = source_node_ids
            .iter()
            .take(3)
            .map(|id| self.meta[*id].label.as_str())
            .collect::<Vec<_>>()
            .join(" + ");

        let activation = source_node_ids
            .iter()
            .map(|id| self.activation[*id])
            .fold(0.0, f32::max);
        let confidence = clamp(cluster.score / (cluster.node_ids.len().max(1) as f32 * 2.0));

        let crystal = ThoughtCrystal {
            id: format!("crystal:{}", stable_hash(&source_node_ids)),
            label,
            source_node_ids,
            activation,
            confidence,
        };
        self.absorb_crystal(&crystal);
        Some(crystal)
    }

    fn generalize_from_concepts(
        &mut self,
        stimulus: &str,
        recalls: &[ConceptRecallResult],
        active_node_ids: &[NeuronId],
    ) -> Option<GeneralizationResult> {
        let usable = recalls
            .iter()
            .filter(|recall| recall.context_fit >= 0.45 || recall.score >= 0.20)
            .take(3)
            .collect::<Vec<_>>();
        if usable.len() < 2 {
            return None;
        }

        let source_schema_ids = usable
            .iter()
            .map(|recall| recall.schema_id.clone())
            .collect::<Vec<_>>();
        let labels = usable
            .iter()
            .map(|recall| recall.label.clone())
            .collect::<Vec<_>>();
        let synthesized_label = format!("Generalized: {}", labels.join(" + "));
        let confidence = clamp(
            usable
                .iter()
                .map(|recall| recall.score * recall.context_fit.max(0.35))
                .sum::<f32>()
                / usable.len() as f32
                + 0.25,
        );
        let interpretation = format!(
            "Untrained context generalized by combining {}. Current situation: {}.",
            labels.join(" with "),
            stimulus
        );
        let mut source_node_ids = active_node_ids.iter().copied().take(5).collect::<Vec<_>>();
        source_node_ids.sort_unstable();
        let crystal_label = format!("{synthesized_label} | trigger: {stimulus}");
        let crystal = ThoughtCrystal {
            id: format!(
                "generalized:{}",
                stable_text_hash(&format!("{}::{stimulus}", source_schema_ids.join("|")))
            ),
            label: crystal_label,
            source_node_ids,
            activation: confidence,
            confidence,
        };
        self.absorb_crystal(&crystal);

        Some(GeneralizationResult {
            source_schema_ids,
            synthesized_label,
            interpretation,
            confidence,
            thought_crystal: crystal,
        })
    }

    fn absorb_crystal(&mut self, crystal: &ThoughtCrystal) {
        if !self
            .thought_crystals
            .iter()
            .any(|stored| stored.id == crystal.id)
        {
            self.thought_crystals.push(crystal.clone());
        }
        let count = self.crystal_counts.entry(crystal.id.clone()).or_insert(0);
        *count += 1;

        if *count < self.reflex_threshold {
            return;
        }
        if self
            .reflexes
            .iter()
            .any(|reflex| reflex.crystal_id == crystal.id)
        {
            return;
        }

        self.reflexes.push(ReflexCircuit {
            crystal_id: crystal.id.clone(),
            label: crystal.label.clone(),
            source_node_ids: crystal.source_node_ids.clone(),
            trigger_count: *count,
            confidence: crystal.confidence,
        });
    }

    fn try_reflex(&self, stimulus: &str) -> Option<ThoughtCrystal> {
        self.reflexes
            .iter()
            .filter_map(|reflex| {
                let source_labels = reflex
                    .source_node_ids
                    .iter()
                    .map(|id| self.meta[*id].label.clone())
                    .collect::<Vec<_>>();
                let mut candidates = vec![reflex.label.clone()];
                candidates.extend(source_labels);
                let similarity = crate::similarity::lexical_similarity(stimulus, &candidates);
                (similarity >= 0.25).then_some((similarity, reflex))
            })
            .max_by(|a, b| a.0.total_cmp(&b.0))
            .map(|(_, reflex)| ThoughtCrystal {
                id: reflex.crystal_id.clone(),
                label: reflex.label.clone(),
                source_node_ids: reflex.source_node_ids.clone(),
                activation: reflex.confidence,
                confidence: reflex.confidence,
            })
    }

    fn try_thought_crystal(
        &mut self,
        recalls: &[ConceptRecallResult],
    ) -> Option<(ThoughtCrystal, bool)> {
        if recalls.len() < 2 {
            return None;
        }
        let recall_context = recalls
            .iter()
            .take(3)
            .map(|recall| recall.label.clone())
            .collect::<Vec<_>>()
            .join(" ");
        let matched = self
            .thought_crystals
            .iter()
            .filter(|crystal| crystal.id.starts_with("generalized:"))
            .filter_map(|crystal| {
                let candidates = vec![crystal.label.clone()];
                let similarity =
                    crate::similarity::lexical_similarity(&recall_context, &candidates);
                (similarity >= 0.30).then_some((similarity, crystal.clone()))
            })
            .max_by(|a, b| a.0.total_cmp(&b.0))
            .map(|(_, crystal)| crystal)?;

        self.absorb_crystal(&matched);
        let promoted_to_reflex = self
            .reflexes
            .iter()
            .any(|reflex| reflex.crystal_id == matched.id);
        Some((matched, promoted_to_reflex))
    }

    fn promote_reflexes_to_concepts(&mut self) -> usize {
        let schemas = self
            .reflexes
            .iter()
            .filter_map(|reflex| {
                let schema_id = format!("learned.{}", reflex.crystal_id.replace(':', "."));
                if self
                    .concept_schemas
                    .iter()
                    .any(|schema| schema.id == schema_id)
                {
                    return None;
                }

                let source_labels = reflex
                    .source_node_ids
                    .iter()
                    .map(|id| self.meta[*id].label.clone())
                    .collect::<Vec<_>>();
                let definition = format!(
                    "A learned compressed thought pattern formed by repeated co-activation of {}",
                    source_labels.join(", ")
                );
                let mut schema = ConceptSchema::new(
                    schema_id,
                    reflex.label.clone(),
                    definition,
                    "learned_memory",
                )
                .with_abstraction_level(0.72)
                .with_importance(reflex.confidence.max(0.55))
                .with_reflex_bonus((reflex.trigger_count as f32 / 8.0).min(1.0));

                for (index, label) in source_labels.into_iter().enumerate() {
                    let role = if index == 0 {
                        "core_signal".to_string()
                    } else {
                        format!("support_signal_{index}")
                    };
                    schema = schema.with_cue(label, role, 0.65);
                }
                Some(schema)
            })
            .collect::<Vec<_>>();
        let promoted = schemas.len();
        for schema in schemas {
            self.add_concept_schema(schema);
        }
        promoted
    }

    fn update_memory_traces(&mut self) {
        for id in 0..self.activation.len() {
            self.average_activation[id] =
                0.05f32.mul_add(self.activation[id], 0.95 * self.average_activation[id]);
            if self.activation[id] > self.activation_floor {
                self.node_idle_cycles[id] = 0;
            } else {
                self.node_idle_cycles[id] = self.node_idle_cycles[id].saturating_add(1);
            }
        }
    }

    fn contradiction_pressure(&self, node_ids: &[NeuronId]) -> f32 {
        let mut in_cluster = vec![false; self.meta.len()];
        for id in node_ids {
            in_cluster[*id] = true;
        }

        let mut pressure = 0.0;
        for source in node_ids {
            for edge in self.edge_range(*source) {
                let target = self.targets[edge];
                if in_cluster[target] && self.relations[edge] == RelationType::Contradiction {
                    pressure = (self.activation[*source] * self.activation[target])
                        .mul_add(self.strengths[edge], pressure);
                }
            }
        }
        pressure
    }

    fn active_ids_from(&self, activation: &[f32]) -> Vec<NeuronId> {
        activation
            .iter()
            .enumerate()
            .filter_map(|(id, value)| (*value > self.activation_floor).then_some(id))
            .collect()
    }

    fn edge_range(&self, source: NeuronId) -> std::ops::Range<usize> {
        self.source_offsets[source]..self.source_offsets[source + 1]
    }
}

impl Default for SynapseCore {
    fn default() -> Self {
        Self::new()
    }
}

fn clamp(value: f32) -> f32 {
    value.clamp(0.0, 1.0)
}

fn stable_hash(ids: &[NeuronId]) -> u64 {
    ids.iter().fold(14_695_981_039_346_656_037_u64, |hash, id| {
        (hash ^ *id as u64).wrapping_mul(1_099_511_628_211)
    })
}

fn stable_text_hash(value: &str) -> u64 {
    value
        .as_bytes()
        .iter()
        .fold(14_695_981_039_346_656_037_u64, |hash, byte| {
            (hash ^ *byte as u64).wrapping_mul(1_099_511_628_211)
        })
}

fn correction_signature(value: &str) -> String {
    let mut tokens = concept_tokens(value);
    tokens.sort();
    tokens.dedup();
    tokens.join("|")
}

fn dot<const N: usize>(left: &[f32; N], right: &[f32; N]) -> f32 {
    left.iter().zip(right.iter()).map(|(a, b)| a * b).sum()
}

fn concept_tokens(text: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    for ch in text.chars() {
        if ch.is_alphanumeric() || ch == '_' {
            current.push(ch.to_lowercase().next().unwrap_or(ch));
        } else if !current.is_empty() {
            tokens.push(std::mem::take(&mut current));
        }
    }
    if !current.is_empty() {
        tokens.push(current);
    }
    tokens
}

fn normalize_phrase(text: &str) -> String {
    concept_tokens(text).join(" ")
}

fn token_prefixes(token: &str) -> Vec<String> {
    let chars = token.chars().collect::<Vec<_>>();
    if chars.len() <= 2 {
        return vec![token.to_string()];
    }
    (2..=chars.len())
        .map(|len| chars.iter().take(len).collect::<String>())
        .collect()
}

fn token_phrases(tokens: &[String], max_len: usize) -> Vec<String> {
    let mut phrases = Vec::new();
    for start in 0..tokens.len() {
        for len in 1..=max_len.min(tokens.len() - start) {
            phrases.push(tokens[start..start + len].join(" "));
        }
    }
    phrases
}

fn push_index(index: &mut HashMap<String, Vec<NeuronId>>, key: String, id: NeuronId) {
    if key.is_empty() {
        return;
    }
    let ids = index.entry(key).or_default();
    if !ids.contains(&id) {
        ids.push(id);
    }
}

fn looks_like_entity(value: &str) -> bool {
    value
        .chars()
        .any(|ch| ch.is_ascii_digit() || ch.is_uppercase())
        || ["삼성전자", "QQQ", "TQQQ", "491620"]
            .iter()
            .any(|entity| value.contains(entity))
}

fn domain_hints(text: &str) -> Vec<String> {
    let lower = text.to_lowercase();
    let mut domains = Vec::new();
    if [
        "삼성전자",
        "주가",
        "하락",
        "외국인",
        "반대매매",
        "수급",
        "실적",
        "finance",
        "market",
        "earnings",
        "margin",
    ]
    .iter()
    .any(|marker| lower.contains(marker))
    {
        domains.push("finance".to_string());
        domains.push("market".to_string());
    }
    if ["뜨거움", "화상", "danger", "emotion", "anxiety"]
        .iter()
        .any(|marker| lower.contains(marker))
    {
        domains.push("emotion".to_string());
    }
    domains.sort();
    domains.dedup();
    domains
}

fn index_hint_similarity(stimulus: &str, meta: &NeuronMeta) -> f32 {
    let stimulus = normalize_phrase(stimulus);
    let key = normalize_phrase(&meta.key);
    let label = normalize_phrase(&meta.label);
    if (!key.is_empty() && stimulus.contains(&key))
        || (!label.is_empty() && stimulus.contains(&label))
    {
        0.75
    } else {
        0.0
    }
}

fn emotion_relevance_pairs(modulation: &NodeModulation) -> [(&'static str, f32); EMOTION_COUNT] {
    [
        ("curiosity", modulation.emotion_relevance[0]),
        ("anxiety", modulation.emotion_relevance[1]),
        ("trust", modulation.emotion_relevance[2]),
        ("loneliness", modulation.emotion_relevance[3]),
        ("joy", modulation.emotion_relevance[4]),
    ]
}

fn desire_relevance_pairs(modulation: &NodeModulation) -> [(&'static str, f32); DESIRE_COUNT] {
    [
        ("learning", modulation.desire_relevance[0]),
        ("relationship", modulation.desire_relevance[1]),
        ("growth", modulation.desire_relevance[2]),
        ("recognition", modulation.desire_relevance[3]),
        ("safety", modulation.desire_relevance[4]),
    ]
}

fn goal_relevance_pairs(modulation: &NodeModulation) -> [(&'static str, f32); GOAL_COUNT] {
    [
        ("help_user", modulation.goal_relevance[0]),
        ("learn_more", modulation.goal_relevance[1]),
        ("maintain_relationship", modulation.goal_relevance[2]),
        ("preserve_coherence", modulation.goal_relevance[3]),
    ]
}

fn interpretation_for(schema: &ConceptSchema, bindings: &[ContextBinding]) -> String {
    if bindings.is_empty() {
        return format!(
            "{}: apply definition to current context - {}",
            schema.label, schema.definition
        );
    }
    let roles = bindings
        .iter()
        .map(|binding| format!("{}={}", binding.role, binding.evidence))
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "{}: {}. Current context matches {}.",
        schema.label, schema.definition, roles
    )
}
