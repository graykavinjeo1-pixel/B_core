use std::collections::{BTreeMap, BTreeSet, VecDeque};

use serde::{Deserialize, Serialize};

use crate::substrate::ConceptIR;

use super::{
    integrity::hash_serializable,
    model::{
        ancestor_closure, bindings_from_stages, execute_concept_instance,
        execute_primitive_pipeline, semantic_preflight, stage_kinds, stage_templates_from_kinds,
        ConceptInstance, ConceptRecord, ExecutionFault, ExecutionRecord, MacroRecord,
        MacroTemplateNode, OperationalTemplateNode, ParameterKind, Plan, PlanNode, Sem1ValueType,
        Stage, StageCapability, StageKind, StageTemplate, Value,
    },
    tasks::{Demonstration, EvaluationTask, VisibleTask},
};

pub const REASONER_VERSION: &str = "SEM1-REASONER-1.1.0";
pub const STRUCTURAL_BASELINE_VERSION: &str = "SEM1-STRONG-MACRO-C-1.1.0";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Condition {
    PrimitiveOnlyA,
    ExactCacheB,
    StrongStructuralMacroC,
    SemanticRecursiveD,
    SemanticNoCounterfactualE,
    SemanticNoInvariantF,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SolveStatus {
    Solved,
    SemanticAbstention,
    ExecutionFailure,
    NoPlan,
    ExactCacheMiss,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ReasoningMetrics {
    pub reasoning_depth: usize,
    pub primitive_expanded_depth: usize,
    pub reasoning_width: usize,
    pub live_branches: usize,
    pub search_expansions: usize,
    pub rollback_count: usize,
    pub wall_time_units: usize,
    pub memory_units: usize,
    pub graph_node_count: usize,
    pub graph_edge_count: usize,
    pub dependency_depth: usize,
    pub alternative_branch_count: usize,
    pub recombination_count: usize,
    pub promoted_concept_reuse_count: usize,
    pub concepts_composed: usize,
    pub macro_uses: usize,
    pub concept_uses: usize,
    pub total_concepts_available: usize,
    pub routed_candidates: usize,
    pub active_working_set: usize,
    pub full_catalog_scans: usize,
    pub precondition_checks: usize,
    pub semantic_equivalence_matches: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SolveResult {
    pub task_id: String,
    pub condition: Condition,
    pub status: SolveStatus,
    pub output: Option<Value>,
    pub fault: Option<ExecutionFault>,
    pub plan: Option<Plan>,
    pub derivation_id: Option<String>,
    pub metrics: ReasoningMetrics,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExactCacheEntry {
    pub signature_sha256: String,
    pub output: Value,
    pub source_task_id: String,
}

#[derive(Debug, Clone)]
pub struct ResourceBudget {
    pub max_expansions: usize,
    pub max_pipeline_stages: usize,
}

impl ResourceBudget {
    pub fn discovery() -> Self {
        Self {
            max_expansions: 20_000,
            max_pipeline_stages: 4,
        }
    }

    pub fn blind() -> Self {
        Self {
            max_expansions: 30_000,
            max_pipeline_stages: 5,
        }
    }
}

#[derive(Default)]
pub struct SparseConceptIndex {
    by_signature: BTreeMap<(Sem1ValueType, Sem1ValueType), Vec<String>>,
}

impl SparseConceptIndex {
    pub fn build(catalog: &BTreeMap<String, ConceptRecord>) -> Self {
        let mut index = Self::default();
        for concept in catalog.values() {
            index
                .by_signature
                .entry((concept.signature_input, concept.signature_output))
                .or_default()
                .push(concept.concept_id.clone());
        }
        for ids in index.by_signature.values_mut() {
            ids.sort_by(|left, right| {
                let left_generation = catalog
                    .get(left)
                    .map(|concept| concept.generation)
                    .unwrap_or_default();
                let right_generation = catalog
                    .get(right)
                    .map(|concept| concept.generation)
                    .unwrap_or_default();
                right_generation
                    .cmp(&left_generation)
                    .then_with(|| left.cmp(right))
            });
        }
        index
    }

    pub fn route(&self, input: Sem1ValueType, target: Sem1ValueType) -> Vec<String> {
        let mut routed = self
            .by_signature
            .get(&(input, target))
            .cloned()
            .unwrap_or_default();
        if target == Sem1ValueType::Integer {
            routed.extend(
                self.by_signature
                    .get(&(input, Sem1ValueType::IntegerSequence))
                    .cloned()
                    .unwrap_or_default(),
            );
        }
        routed.sort();
        routed.dedup();
        routed
    }
}

pub struct Sem1Reasoner<'a> {
    pub concepts: &'a BTreeMap<String, ConceptRecord>,
    pub macros: &'a BTreeMap<String, MacroRecord>,
    pub predecessor: &'a ConceptIR,
    pub cache: &'a [ExactCacheEntry],
    index: SparseConceptIndex,
}

impl<'a> Sem1Reasoner<'a> {
    pub fn new(
        concepts: &'a BTreeMap<String, ConceptRecord>,
        macros: &'a BTreeMap<String, MacroRecord>,
        predecessor: &'a ConceptIR,
        cache: &'a [ExactCacheEntry],
    ) -> Self {
        Self {
            concepts,
            macros,
            predecessor,
            cache,
            index: SparseConceptIndex::build(concepts),
        }
    }

    pub fn solve(
        &self,
        task: &VisibleTask,
        condition: Condition,
        budget: ResourceBudget,
    ) -> SolveResult {
        match condition {
            Condition::ExactCacheB => self.solve_cache(task),
            Condition::PrimitiveOnlyA => self.solve_with_plans(
                task,
                condition,
                primitive_plans(task, budget.max_pipeline_stages),
                budget,
                false,
            ),
            Condition::StrongStructuralMacroC => {
                let mut plans = macro_plans(task, self.macros);
                plans.extend(primitive_plans(task, budget.max_pipeline_stages));
                self.solve_with_plans(task, condition, deduplicate_plans(plans), budget, false)
            }
            Condition::SemanticRecursiveD
            | Condition::SemanticNoCounterfactualE
            | Condition::SemanticNoInvariantF => {
                let routed = self.index.route(task.input_type, task.output_type);
                let mut plans = semantic_plans(
                    task,
                    self.concepts,
                    &routed,
                    budget.max_pipeline_stages,
                    budget.max_expansions,
                );
                plans.extend(primitive_plans(task, budget.max_pipeline_stages));
                let semantic_preconditions = condition != Condition::SemanticNoInvariantF;
                self.solve_with_plans(
                    task,
                    condition,
                    deduplicate_plans(plans),
                    budget,
                    semantic_preconditions,
                )
            }
        }
    }

    fn solve_cache(&self, task: &VisibleTask) -> SolveResult {
        let signature = exact_signature(task).unwrap_or_default();
        if let Some(entry) = self
            .cache
            .iter()
            .find(|entry| entry.signature_sha256 == signature)
        {
            return SolveResult {
                task_id: task.task_id.clone(),
                condition: Condition::ExactCacheB,
                status: SolveStatus::Solved,
                output: Some(entry.output.clone()),
                fault: None,
                plan: None,
                derivation_id: Some(format!("CACHE-{}", entry.source_task_id)),
                metrics: ReasoningMetrics {
                    search_expansions: 1,
                    reasoning_width: 1,
                    ..ReasoningMetrics::default()
                },
            };
        }
        SolveResult {
            task_id: task.task_id.clone(),
            condition: Condition::ExactCacheB,
            status: SolveStatus::ExactCacheMiss,
            output: None,
            fault: None,
            plan: None,
            derivation_id: None,
            metrics: ReasoningMetrics {
                search_expansions: 1,
                reasoning_width: 1,
                ..ReasoningMetrics::default()
            },
        }
    }

    fn solve_with_plans(
        &self,
        task: &VisibleTask,
        condition: Condition,
        plans: Vec<Plan>,
        budget: ResourceBudget,
        semantic_preconditions: bool,
    ) -> SolveResult {
        let routed_count = if matches!(
            condition,
            Condition::SemanticRecursiveD
                | Condition::SemanticNoCounterfactualE
                | Condition::SemanticNoInvariantF
        ) {
            self.index.route(task.input_type, task.output_type).len()
        } else {
            0
        };
        let width = plans.len();
        let mut metrics = ReasoningMetrics {
            reasoning_width: width,
            live_branches: width,
            alternative_branch_count: width.saturating_sub(1),
            total_concepts_available: self.concepts.len(),
            routed_candidates: routed_count,
            active_working_set: routed_count.max(1),
            full_catalog_scans: 0,
            memory_units: width.saturating_mul(3),
            ..ReasoningMetrics::default()
        };

        for (index, plan) in plans.into_iter().enumerate() {
            if index >= budget.max_expansions {
                break;
            }
            metrics.search_expansions += 1;
            if !demonstrations_match(
                &plan,
                &task.demonstrations,
                self.concepts,
                self.macros,
                self.predecessor,
            ) {
                metrics.rollback_count += 1;
                continue;
            }

            if semantic_preconditions {
                metrics.precondition_checks += 1;
                if !semantic_preflight(&plan.expanded_stages, &task.query_input) {
                    metrics.reasoning_depth = plan.expanded_stages.len();
                    metrics.primitive_expanded_depth = expanded_graph_nodes(&plan);
                    metrics.graph_node_count = expanded_graph_nodes(&plan);
                    metrics.graph_edge_count = metrics.graph_node_count.saturating_sub(1);
                    metrics.dependency_depth = plan.nodes.len();
                    metrics.concepts_composed = plan.all_executed_concept_ids.len();
                    metrics.concept_uses = plan.all_executed_concept_ids.len();
                    metrics.promoted_concept_reuse_count = plan.all_executed_concept_ids.len();
                    metrics.wall_time_units = metrics.search_expansions + metrics.reasoning_depth;
                    metrics.semantic_equivalence_matches =
                        equivalent_matches(&plan.expanded_stages);
                    return SolveResult {
                        task_id: task.task_id.clone(),
                        condition,
                        status: SolveStatus::SemanticAbstention,
                        output: None,
                        fault: Some(ExecutionFault::ArithmeticOverflow),
                        derivation_id: Some(format!("D-{}-{:04}", task.task_id, index + 1)),
                        plan: Some(plan),
                        metrics,
                    };
                }
            }

            match execute_plan(
                &plan,
                task.query_input.clone(),
                self.concepts,
                self.macros,
                self.predecessor,
            ) {
                Ok(record) => {
                    merge_execution_metrics(&mut metrics, &plan, &record);
                    metrics.wall_time_units = metrics.search_expansions + metrics.reasoning_depth;
                    metrics.semantic_equivalence_matches =
                        equivalent_matches(&plan.expanded_stages);
                    return SolveResult {
                        task_id: task.task_id.clone(),
                        condition,
                        status: SolveStatus::Solved,
                        output: Some(record.value),
                        fault: None,
                        derivation_id: Some(format!("D-{}-{:04}", task.task_id, index + 1)),
                        plan: Some(plan),
                        metrics,
                    };
                }
                Err(fault) => {
                    metrics.wall_time_units =
                        metrics.search_expansions + plan.expanded_stages.len();
                    metrics.primitive_expanded_depth = expanded_graph_nodes(&plan);
                    metrics.graph_node_count = expanded_graph_nodes(&plan);
                    metrics.graph_edge_count = metrics.graph_node_count.saturating_sub(1);
                    metrics.dependency_depth = plan.nodes.len();
                    metrics.concepts_composed = plan.all_executed_concept_ids.len();
                    return SolveResult {
                        task_id: task.task_id.clone(),
                        condition,
                        status: SolveStatus::ExecutionFailure,
                        output: None,
                        fault: Some(fault),
                        derivation_id: Some(format!("D-{}-{:04}", task.task_id, index + 1)),
                        plan: Some(plan),
                        metrics,
                    };
                }
            }
        }

        metrics.wall_time_units = metrics.search_expansions;
        SolveResult {
            task_id: task.task_id.clone(),
            condition,
            status: SolveStatus::NoPlan,
            output: None,
            fault: None,
            plan: None,
            derivation_id: None,
            metrics,
        }
    }
}

fn merge_execution_metrics(metrics: &mut ReasoningMetrics, plan: &Plan, record: &ExecutionRecord) {
    metrics.reasoning_depth = record.reasoning_depth;
    metrics.primitive_expanded_depth = record.primitive_expanded_depth;
    metrics.graph_node_count = record.graph_nodes;
    metrics.graph_edge_count = record.graph_edges;
    metrics.dependency_depth = plan.nodes.len();
    metrics.recombination_count = plan.nodes.len().saturating_sub(1);
    metrics.concepts_composed = plan.all_executed_concept_ids.len();
    metrics.concept_uses = plan.all_executed_concept_ids.len();
    metrics.macro_uses = plan
        .nodes
        .iter()
        .filter(|node| matches!(node, PlanNode::Macro { .. }))
        .count();
    metrics.promoted_concept_reuse_count = plan.all_executed_concept_ids.len();
}

fn demonstrations_match(
    plan: &Plan,
    demonstrations: &[Demonstration],
    concepts: &BTreeMap<String, ConceptRecord>,
    macros: &BTreeMap<String, MacroRecord>,
    predecessor: &ConceptIR,
) -> bool {
    demonstrations.iter().all(|demo| {
        execute_plan(plan, demo.input.clone(), concepts, macros, predecessor)
            .map(|record| record.value == demo.output)
            .unwrap_or(false)
    })
}

pub fn execute_plan(
    plan: &Plan,
    input: Value,
    concepts: &BTreeMap<String, ConceptRecord>,
    _macros: &BTreeMap<String, MacroRecord>,
    predecessor: &ConceptIR,
) -> Result<ExecutionRecord, ExecutionFault> {
    let mut value = input;
    let mut depth = 0usize;
    let mut primitive_depth = 0usize;
    let mut graph_nodes = 0usize;
    let mut graph_edges = 0usize;
    let mut executed = BTreeSet::new();
    for node in &plan.nodes {
        match node {
            PlanNode::Concept(instance) => {
                let record = execute_concept_instance(instance, value, concepts, predecessor)?;
                value = record.value;
                depth += record.reasoning_depth;
                primitive_depth += record.primitive_expanded_depth;
                graph_nodes += record.graph_nodes;
                graph_edges += record.graph_edges;
                executed.extend(record.executed_concepts);
            }
            PlanNode::Macro { .. } => {
                let record = execute_primitive_pipeline(&plan.expanded_stages, value)?;
                value = record.value;
                depth += record.reasoning_depth;
                primitive_depth += record.primitive_expanded_depth;
                graph_nodes += record.graph_nodes;
                graph_edges += record.graph_edges;
                break;
            }
            PlanNode::Primitive(stage) => {
                let record = execute_primitive_pipeline(std::slice::from_ref(stage), value)?;
                value = record.value;
                depth += record.reasoning_depth;
                primitive_depth += record.primitive_expanded_depth;
                graph_nodes += record.graph_nodes;
                graph_edges += record.graph_edges;
            }
        }
    }
    Ok(ExecutionRecord {
        value,
        reasoning_depth: depth,
        primitive_expanded_depth: primitive_depth,
        graph_nodes,
        graph_edges: graph_edges.saturating_add(plan.nodes.len().saturating_sub(1)),
        operational_nodes: plan.nodes.len(),
        operational_depth: plan.nodes.len(),
        executed_concepts: executed.into_iter().collect(),
    })
}

fn primitive_plans(task: &VisibleTask, max_stages: usize) -> Vec<Plan> {
    let mut output = Vec::new();
    let mut current = Vec::new();
    enumerate_primitive_sequences(
        &task.capabilities,
        task.input_type,
        task.output_type,
        max_stages,
        &mut current,
        &mut output,
    );
    output.sort_by_key(|plan| (plan.nodes.len(), structural_plan_key(plan)));
    output
}

fn enumerate_primitive_sequences(
    capabilities: &[StageCapability],
    current_type: Sem1ValueType,
    target_type: Sem1ValueType,
    max_stages: usize,
    current: &mut Vec<Stage>,
    output: &mut Vec<Plan>,
) {
    if !current.is_empty() && current_type == target_type {
        output.push(plan_from_nodes(
            current.iter().cloned().map(PlanNode::Primitive).collect(),
            current.clone(),
            Vec::new(),
        ));
    }
    if current.len() >= max_stages || current_type == Sem1ValueType::Integer {
        return;
    }
    for capability in capabilities {
        if capability.stage.input_type() != current_type || current.contains(&capability.stage) {
            continue;
        }
        current.push(capability.stage.clone());
        enumerate_primitive_sequences(
            capabilities,
            capability.stage.output_type(),
            target_type,
            max_stages,
            current,
            output,
        );
        current.pop();
    }
}

fn semantic_plans(
    task: &VisibleTask,
    catalog: &BTreeMap<String, ConceptRecord>,
    routed: &[String],
    max_stages: usize,
    max_states: usize,
) -> Vec<Plan> {
    let atoms = semantic_atoms(task, catalog, routed);
    let mut plans = Vec::new();
    let mut queue = VecDeque::from([SemanticState {
        current_type: task.input_type,
        nodes: Vec::new(),
        expanded_stages: Vec::new(),
        executed_concept_ids: BTreeSet::new(),
    }]);
    let mut states_examined = 0usize;
    while let Some(state) = queue.pop_front() {
        if states_examined >= max_states {
            break;
        }
        states_examined += 1;
        if !state.nodes.is_empty() && state.current_type == task.output_type {
            plans.push(plan_from_nodes(
                state.nodes.clone(),
                state.expanded_stages.clone(),
                state.executed_concept_ids.iter().cloned().collect(),
            ));
        }
        if state.expanded_stages.len() >= max_stages || state.current_type == Sem1ValueType::Integer
        {
            continue;
        }
        for atom in &atoms {
            if atom.input_type != state.current_type
                || state.expanded_stages.len() + atom.expanded_stages.len() > max_stages
            {
                continue;
            }
            let mut next = state.clone();
            next.current_type = atom.output_type;
            next.nodes.push(atom.node.clone());
            next.expanded_stages
                .extend(atom.expanded_stages.iter().cloned());
            next.executed_concept_ids
                .extend(atom.executed_concept_ids.iter().cloned());
            queue.push_back(next);
        }
    }
    plans.sort_by_key(|plan| {
        let max_generation = plan
            .direct_concept_ids
            .iter()
            .filter_map(|id| catalog.get(id).map(|concept| concept.generation))
            .max()
            .unwrap_or_default();
        (
            usize::MAX - max_generation,
            plan.nodes.len(),
            usize::MAX - plan.direct_concept_ids.len(),
            structural_plan_key(plan),
        )
    });
    plans
}

#[derive(Clone)]
struct SemanticAtom {
    node: PlanNode,
    input_type: Sem1ValueType,
    output_type: Sem1ValueType,
    expanded_stages: Vec<Stage>,
    executed_concept_ids: Vec<String>,
}

#[derive(Clone)]
struct SemanticState {
    current_type: Sem1ValueType,
    nodes: Vec<PlanNode>,
    expanded_stages: Vec<Stage>,
    executed_concept_ids: BTreeSet<String>,
}

fn semantic_atoms(
    task: &VisibleTask,
    catalog: &BTreeMap<String, ConceptRecord>,
    routed: &[String],
) -> Vec<SemanticAtom> {
    let mut atoms = Vec::new();
    let mut ordered_ids = routed.to_vec();
    ordered_ids.sort_by_key(|id| {
        let concept = catalog.get(id);
        (
            concept.is_none_or(|concept| concept.signature_output != task.output_type),
            usize::MAX
                - concept
                    .map(|concept| concept.generation)
                    .unwrap_or_default(),
            id.clone(),
        )
    });
    for id in &ordered_ids {
        let Some(concept) = catalog.get(id) else {
            continue;
        };
        let expanded_kinds = concept
            .primitive_expansion
            .iter()
            .map(StageTemplate::kind)
            .collect::<Vec<_>>();
        for stages in stage_choices_by_semantics(&expanded_kinds, &task.capabilities) {
            let bindings = bindings_from_stages(&stages);
            if concept.instantiate(&bindings).is_err() {
                continue;
            }
            atoms.push(SemanticAtom {
                node: PlanNode::Concept(ConceptInstance {
                    concept_id: id.clone(),
                    bindings,
                }),
                input_type: concept.signature_input,
                output_type: concept.signature_output,
                expanded_stages: stages,
                executed_concept_ids: ancestor_ids_for(id, catalog),
            });
        }
    }
    atoms.extend(task.capabilities.iter().map(|capability| SemanticAtom {
        node: PlanNode::Primitive(capability.stage.clone()),
        input_type: capability.stage.input_type(),
        output_type: capability.stage.output_type(),
        expanded_stages: vec![capability.stage.clone()],
        executed_concept_ids: Vec::new(),
    }));
    atoms
}

fn stage_choices_by_semantics(
    kinds: &[StageKind],
    capabilities: &[StageCapability],
) -> Vec<Vec<Stage>> {
    let mut choices = vec![Vec::new()];
    for kind in kinds {
        let matching = capabilities
            .iter()
            .filter(|capability| capability.stage.kind() == *kind);
        let mut next = Vec::new();
        for prefix in &choices {
            for capability in matching.clone() {
                let mut candidate = prefix.clone();
                candidate.push(capability.stage.clone());
                next.push(candidate);
            }
        }
        choices = next;
    }
    choices
}

fn macro_plans(task: &VisibleTask, macros: &BTreeMap<String, MacroRecord>) -> Vec<Plan> {
    let mut plans = Vec::new();
    let mut ordered = macros.values().collect::<Vec<_>>();
    ordered.sort_by_key(|item| {
        (
            usize::MAX - item.primitive_expansion.len(),
            item.macro_id.clone(),
        )
    });
    for macro_record in ordered {
        let kinds = macro_record
            .primitive_expansion
            .iter()
            .map(StageTemplate::kind)
            .collect::<Vec<_>>();
        for stages in stage_choices_by_semantics(&kinds, &task.capabilities) {
            if stages.last().map(Stage::output_type) != Some(task.output_type) {
                continue;
            }
            let normalized = stages
                .iter()
                .map(normalized_structural_tokens)
                .collect::<Vec<_>>();
            if normalized != macro_record.structural_pattern {
                continue;
            }
            let bindings = bindings_from_stages(&stages);
            plans.push(plan_from_nodes(
                vec![PlanNode::Macro {
                    macro_id: macro_record.macro_id.clone(),
                    bindings,
                }],
                stages,
                Vec::new(),
            ));
        }
    }
    plans
}

fn normalized_structural_tokens(stage: &Stage) -> Vec<String> {
    match stage {
        Stage::Transform(operator) if !operator.is_equivalent_variant() => vec!["S01X".to_string()],
        Stage::Transform(operator) => {
            vec![operator.structural_code().to_string(), "S099".to_string()]
        }
        Stage::Retain(_) => vec!["S02X".to_string()],
        Stage::Aggregate(_) => vec!["S03X".to_string()],
    }
}

fn plan_from_nodes(
    nodes: Vec<PlanNode>,
    expanded_stages: Vec<Stage>,
    all_concepts: Vec<String>,
) -> Plan {
    let direct_concept_ids = nodes
        .iter()
        .filter_map(|node| match node {
            PlanNode::Concept(instance) => Some(instance.concept_id.clone()),
            _ => None,
        })
        .collect::<Vec<_>>();
    let primitive_ancestor_ids = expanded_stages
        .iter()
        .map(|stage| stage.primitive_id().to_string())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    Plan {
        nodes,
        expanded_stages,
        direct_concept_ids,
        all_executed_concept_ids: all_concepts,
        primitive_ancestor_ids,
    }
}

fn ancestor_ids_for(id: &str, catalog: &BTreeMap<String, ConceptRecord>) -> Vec<String> {
    let mut ids = BTreeSet::from([id.to_string()]);
    if let Some(concept) = catalog.get(id) {
        ids.extend(concept.ancestor_concept_ids.iter().cloned());
    }
    ids.into_iter().collect()
}

fn deduplicate_plans(plans: Vec<Plan>) -> Vec<Plan> {
    let mut seen = BTreeSet::new();
    plans
        .into_iter()
        .filter(|plan| seen.insert(serde_json::to_string(&plan.nodes).unwrap_or_default()))
        .collect()
}

fn structural_plan_key(plan: &Plan) -> String {
    plan.expanded_stages
        .iter()
        .map(Stage::structural_code)
        .collect::<Vec<_>>()
        .join("/")
}

fn expanded_graph_nodes(plan: &Plan) -> usize {
    plan.expanded_stages
        .iter()
        .map(Stage::expanded_graph_nodes)
        .sum()
}

fn equivalent_matches(stages: &[Stage]) -> usize {
    stages
        .iter()
        .filter(
            |stage| matches!(stage, Stage::Transform(operator) if operator.is_equivalent_variant()),
        )
        .count()
}

pub fn exact_signature(task: &VisibleTask) -> Result<String, String> {
    hash_serializable(task)
}

pub fn build_exact_cache(tasks: &[EvaluationTask]) -> Result<Vec<ExactCacheEntry>, String> {
    tasks
        .iter()
        .filter_map(|task| match &task.expected_query {
            super::tasks::ExpectedOutcome::Value(output) => Some(
                exact_signature(&task.visible).map(|signature_sha256| ExactCacheEntry {
                    signature_sha256,
                    output: output.clone(),
                    source_task_id: task.visible.task_id.clone(),
                }),
            ),
            super::tasks::ExpectedOutcome::SemanticInvalid => None,
        })
        .collect()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MiningReport {
    pub successful_derivations_examined: usize,
    pub composition_groups_examined: usize,
    pub candidates_produced: usize,
    pub candidates_rejected_as_trivial: usize,
    pub minimum_independent_origins: usize,
    pub concept_on_concept_composition_open: bool,
    pub fixed_generation_target: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MiningOutput {
    pub candidates: Vec<ConceptRecord>,
    pub macros: Vec<MacroRecord>,
    pub report: MiningReport,
}

pub fn mine_compositions(
    results: &[SolveResult],
    catalog: &BTreeMap<String, ConceptRecord>,
    minimum_origins: usize,
) -> Result<MiningOutput, String> {
    let mut groups: BTreeMap<String, Vec<&SolveResult>> = BTreeMap::new();
    for result in results
        .iter()
        .filter(|result| result.status == SolveStatus::Solved)
    {
        let Some(plan) = &result.plan else { continue };
        if plan.direct_concept_ids.is_empty() || plan.nodes.len() < 2 {
            continue;
        }
        groups
            .entry(operational_signature(plan))
            .or_default()
            .push(result);
    }
    let mut candidates = Vec::new();
    let mut macros = Vec::new();
    let mut rejected = 0usize;
    for group in groups.values() {
        let origins = group
            .iter()
            .map(|result| result.task_id.clone())
            .collect::<BTreeSet<_>>();
        if origins.len() < minimum_origins {
            continue;
        }
        let first_plan = group[0].plan.as_ref().expect("group plan");
        let kinds = stage_kinds(&first_plan.expanded_stages);
        let existing_shape = catalog.values().any(|concept| {
            concept
                .primitive_expansion
                .iter()
                .map(StageTemplate::kind)
                .collect::<Vec<_>>()
                == kinds
        });
        let binding_variation = group
            .iter()
            .filter_map(|result| result.plan.as_ref())
            .map(|plan| bindings_from_stages(&plan.expanded_stages))
            .collect::<BTreeSet<_>>()
            .len();
        if existing_shape || binding_variation < 2 {
            rejected += 1;
            continue;
        }
        let direct_parents = first_plan.direct_concept_ids.clone();
        let parent_generation = direct_parents
            .iter()
            .filter_map(|id| catalog.get(id).map(|concept| concept.generation))
            .max()
            .unwrap_or_default();
        if parent_generation == 0 {
            rejected += 1;
            continue;
        }
        let generation = parent_generation + 1;
        let (parameter_kinds, primitive_expansion) = stage_templates_from_kinds(&kinds);
        let operational_template = operational_template_from_plan(first_plan, catalog)?;
        let ancestor_ids = ancestor_closure(&direct_parents, catalog);
        let mut primitive_ids = first_plan
            .primitive_ancestor_ids
            .iter()
            .cloned()
            .collect::<BTreeSet<_>>();
        for parent in &direct_parents {
            if let Some(concept) = catalog.get(parent) {
                primitive_ids.extend(concept.primitive_ancestor_ids.iter().cloned());
            }
        }
        let next_id = format!("C{:06}", 2 + candidates.len());
        let source_task_ids = origins.into_iter().collect::<Vec<_>>();
        let source_derivation_ids = group
            .iter()
            .filter_map(|result| result.derivation_id.clone())
            .collect::<Vec<_>>();
        let primitive_expanded_nodes = first_plan
            .expanded_stages
            .iter()
            .map(Stage::expanded_graph_nodes)
            .sum::<usize>();
        let mut complete_ancestors = ancestor_ids.clone();
        complete_ancestors.extend(primitive_ids.iter().cloned());
        let mut candidate = ConceptRecord {
            concept_id: next_id.clone(),
            generation,
            promotion_state: "CANDIDATE".to_string(),
            signature_input: Sem1ValueType::IntegerSequence,
            signature_output: first_plan
                .expanded_stages
                .last()
                .map(Stage::output_type)
                .unwrap_or(Sem1ValueType::IntegerSequence),
            parameter_kinds: parameter_kinds.clone(),
            operational_template,
            primitive_expansion: primitive_expansion.clone(),
            direct_parent_concepts: direct_parents.clone(),
            ancestor_concept_ids: ancestor_ids.iter().cloned().collect(),
            primitive_ancestor_ids: primitive_ids.iter().cloned().collect(),
            complete_ancestor_set: complete_ancestors.into_iter().collect(),
            epistemic_historical_depth: primitive_expanded_nodes,
            operational_depth: first_plan.nodes.len(),
            operational_cost: 1,
            composition_arity: first_plan.nodes.len(),
            promoted_ancestor_count: ancestor_ids.len(),
            source_task_ids: source_task_ids.clone(),
            source_derivation_ids: source_derivation_ids.clone(),
            preconditions: vec![
                "INPUT_TYPE_MATCHES".to_string(),
                "ALL_CHECKED_OPERATIONS_DEFINED".to_string(),
                "INTERMEDIATE_TYPES_COMPOSE".to_string(),
            ],
            invariants: vec![
                "INPUT_IMMUTABLE".to_string(),
                "DETERMINISTIC_STAGE_ORDER".to_string(),
                "OUTPUT_TYPE_PREDICTED".to_string(),
            ],
            predictions: vec![
                "PRIMITIVE_EXPANSION_EQUIVALENT".to_string(),
                "INVALID_DOMAIN_ABSTAINS".to_string(),
                "EQUIVALENT_OPERATOR_TRANSFER".to_string(),
            ],
            counterfactual_interface: vec![
                "PRECONDITION_REMOVAL".to_string(),
                "INVARIANT_VIOLATION".to_string(),
                "TYPE_MUTATION".to_string(),
                "OPERATION_REPLACEMENT".to_string(),
                "STAGE_REORDERING".to_string(),
                "PARTIAL_SUBGRAPH_DELETION".to_string(),
                "EQUIVALENT_OPERATOR_SUBSTITUTION".to_string(),
                "ADVERSARIAL_STRUCTURAL_MIMICRY".to_string(),
            ],
            content_hash_sha256: String::new(),
            derived_autonomously: true,
            lexical_information_used: false,
        };
        candidate.freeze_hash()?;

        let macro_id = format!("M{:06}", 2 + macros.len());
        let macro_template = macro_template_from_plan(first_plan, catalog);
        macros.push(MacroRecord {
            macro_id,
            parameter_kinds,
            structural_pattern: first_plan
                .expanded_stages
                .iter()
                .map(normalized_structural_tokens)
                .collect(),
            operational_template: macro_template.clone(),
            primitive_expansion,
            direct_parent_macros: macro_template
                .iter()
                .filter_map(|node| match node {
                    MacroTemplateNode::Macro { macro_id, .. } => Some(macro_id.clone()),
                    _ => None,
                })
                .collect(),
            source_derivation_ids,
            typed_parameters: true,
            variable_operators: true,
            composition_supported: true,
            macro_on_macro_reuse: macro_template
                .iter()
                .any(|node| matches!(node, MacroTemplateNode::Macro { .. })),
            semantic_validation: false,
        });
        candidates.push(candidate);
    }
    Ok(MiningOutput {
        report: MiningReport {
            successful_derivations_examined: results
                .iter()
                .filter(|result| result.status == SolveStatus::Solved)
                .count(),
            composition_groups_examined: groups.len(),
            candidates_produced: candidates.len(),
            candidates_rejected_as_trivial: rejected,
            minimum_independent_origins: minimum_origins,
            concept_on_concept_composition_open: true,
            fixed_generation_target: false,
        },
        candidates,
        macros,
    })
}

fn operational_signature(plan: &Plan) -> String {
    plan.nodes
        .iter()
        .map(|node| match node {
            PlanNode::Concept(instance) => format!("C:{}", instance.concept_id),
            PlanNode::Macro { macro_id, .. } => format!("M:{macro_id}"),
            PlanNode::Primitive(stage) => format!("P:{:?}", stage.kind()),
        })
        .collect::<Vec<_>>()
        .join("|")
}

fn operational_template_from_plan(
    plan: &Plan,
    catalog: &BTreeMap<String, ConceptRecord>,
) -> Result<Vec<OperationalTemplateNode>, String> {
    let mut nodes = Vec::new();
    let mut stage_offset = 0usize;
    for node in &plan.nodes {
        match node {
            PlanNode::Concept(instance) => {
                let parent = catalog
                    .get(&instance.concept_id)
                    .ok_or_else(|| "MISSING_PARENT".to_string())?;
                let slots =
                    (stage_offset..stage_offset + parent.parameter_kinds.len()).collect::<Vec<_>>();
                stage_offset += parent.parameter_kinds.len();
                nodes.push(OperationalTemplateNode::Concept {
                    concept_id: instance.concept_id.clone(),
                    binding_slots: slots,
                });
            }
            PlanNode::Primitive(stage) => {
                let template = match stage.kind() {
                    StageKind::Transform => StageTemplate::Transform { slot: stage_offset },
                    StageKind::Retain => StageTemplate::Retain { slot: stage_offset },
                    StageKind::Aggregate => StageTemplate::Aggregate { slot: stage_offset },
                };
                stage_offset += 1;
                nodes.push(OperationalTemplateNode::Primitive { stage: template });
            }
            PlanNode::Macro { .. } => {
                return Err("SEMANTIC_CANDIDATE_FROM_MACRO_FORBIDDEN".to_string())
            }
        }
    }
    Ok(nodes)
}

fn macro_template_from_plan(
    plan: &Plan,
    catalog: &BTreeMap<String, ConceptRecord>,
) -> Vec<MacroTemplateNode> {
    let mut nodes = Vec::new();
    let mut offset = 0usize;
    for node in &plan.nodes {
        match node {
            PlanNode::Concept(instance) => {
                let slots = catalog
                    .get(&instance.concept_id)
                    .map(|concept| {
                        let range =
                            (offset..offset + concept.parameter_kinds.len()).collect::<Vec<_>>();
                        offset += concept.parameter_kinds.len();
                        range
                    })
                    .unwrap_or_default();
                let macro_id = if instance.concept_id == "C000001" {
                    "M000001".to_string()
                } else {
                    instance.concept_id.replacen('C', "M", 1)
                };
                nodes.push(MacroTemplateNode::Macro {
                    macro_id,
                    binding_slots: slots,
                });
            }
            PlanNode::Primitive(stage) => {
                let template = match stage.kind() {
                    StageKind::Transform => StageTemplate::Transform { slot: offset },
                    StageKind::Retain => StageTemplate::Retain { slot: offset },
                    StageKind::Aggregate => StageTemplate::Aggregate { slot: offset },
                };
                offset += 1;
                nodes.push(MacroTemplateNode::Primitive { stage: template });
            }
            PlanNode::Macro { macro_id, bindings } => {
                let slots = (offset..offset + bindings.len()).collect::<Vec<_>>();
                offset += bindings.len();
                nodes.push(MacroTemplateNode::Macro {
                    macro_id: macro_id.clone(),
                    binding_slots: slots,
                });
            }
        }
    }
    nodes
}

pub fn m000001_record() -> MacroRecord {
    MacroRecord {
        macro_id: "M000001".to_string(),
        parameter_kinds: vec![ParameterKind::CheckedScalarOperator],
        structural_pattern: vec![vec!["S01X".to_string()]],
        operational_template: Vec::new(),
        primitive_expansion: vec![StageTemplate::Transform { slot: 0 }],
        direct_parent_macros: Vec::new(),
        source_derivation_ids: (1..=6).map(|index| format!("D-T{index:06}")).collect(),
        typed_parameters: true,
        variable_operators: true,
        composition_supported: true,
        macro_on_macro_reuse: false,
        semantic_validation: false,
    }
}

#[cfg(test)]
mod tests {
    use std::{collections::BTreeMap, path::PathBuf};

    use super::{build_exact_cache, mine_compositions, Condition, ResourceBudget, Sem1Reasoner};
    use crate::sem1::{
        integrity::verify_and_load, model::c000001_record, tasks::generate_curriculum,
    };

    #[test]
    fn discovery_composes_immutable_predecessor_with_primitives() {
        let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let root = manifest_dir
            .parent()
            .and_then(|path| path.parent())
            .expect("workspace root");
        let (_, predecessor) = verify_and_load(root).expect("predecessor");
        let (discovery, _, _) = generate_curriculum().expect("curriculum");
        let mut concepts = BTreeMap::new();
        concepts.insert(
            "C000001".to_string(),
            c000001_record(&predecessor).expect("record"),
        );
        let cache = build_exact_cache(&discovery).expect("cache");
        let macros = BTreeMap::new();
        let reasoner = Sem1Reasoner::new(&concepts, &macros, &predecessor, &cache);
        let results = discovery
            .iter()
            .map(|task| {
                reasoner.solve(
                    &task.visible,
                    Condition::SemanticRecursiveD,
                    ResourceBudget::discovery(),
                )
            })
            .collect::<Vec<_>>();
        assert!(results
            .iter()
            .all(|result| result.plan.as_ref().is_some_and(|plan| plan
                .all_executed_concept_ids
                .contains(&"C000001".to_string()))));
        assert!(results.iter().any(|result| {
            result.plan.as_ref().is_some_and(|plan| {
                plan.direct_concept_ids
                    .iter()
                    .filter(|concept_id| concept_id.as_str() == "C000001")
                    .count()
                    >= 2
            })
        }));
        let mined = mine_compositions(&results, &concepts, 3).expect("mined");
        assert!(mined.candidates.len() >= 4);
        assert!(mined
            .candidates
            .iter()
            .any(|candidate| candidate.generation == 2));
    }
}
