use std::collections::{HashMap, HashSet, VecDeque};
use std::time::Instant;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::dsl::{
    execute_program, ExecutionError, ExecutionTrace, Instruction, InstructionPattern,
    ScalarOperator,
};
use crate::substrate::{CacheEntry, ConceptIR, StructuralMacro};
use crate::task::VisibleTask;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DerivationNodeKind {
    Goal,
    Hypothesis,
    PrimitiveExecution,
    MacroExecution,
    ConceptExecution,
    BranchDecision,
    Rollback,
    FinalOutput,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DerivationNode {
    pub node_id: String,
    pub kind: DerivationNodeKind,
    pub executable_id: String,
    pub depth: usize,
    pub branch_id: usize,
    pub state_before: String,
    pub state_after: String,
    pub cost: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DerivationEdge {
    pub source: String,
    pub target: String,
    pub dependency: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DerivationGraph {
    pub graph_id: String,
    pub task_id: String,
    pub nodes: Vec<DerivationNode>,
    pub edges: Vec<DerivationEdge>,
    pub primitive_expansion: Vec<String>,
    pub final_output: Option<Vec<i64>>,
    pub verified: bool,
    pub historical_cost: usize,
    pub operational_cost: usize,
}

impl DerivationGraph {
    pub fn validate_integrity(&self) -> bool {
        let mut ids = HashSet::new();
        let mut positions = HashMap::new();
        for (position, node) in self.nodes.iter().enumerate() {
            if !ids.insert(node.node_id.clone()) {
                return false;
            }
            positions.insert(node.node_id.as_str(), position);
        }
        self.edges.iter().all(|edge| {
            let Some(source) = positions.get(edge.source.as_str()) else {
                return false;
            };
            let Some(target) = positions.get(edge.target.as_str()) else {
                return false;
            };
            source < target
        })
    }

    pub fn seal_verification(&mut self, verified: bool) {
        self.verified = verified;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResourceBudget {
    pub expansion_budget: usize,
    pub execution_step_budget: usize,
    pub active_set_budget: usize,
    pub stagnation_budget: usize,
    pub wall_time_budget_ms: u64,
    pub memory_budget_bytes: usize,
}

impl ResourceBudget {
    pub fn discovery() -> Self {
        Self {
            expansion_budget: 256,
            execution_step_budget: 256,
            active_set_budget: 8,
            stagnation_budget: 64,
            wall_time_budget_ms: 1_000,
            memory_budget_bytes: 4 * 1024 * 1024,
        }
    }

    pub fn blind() -> Self {
        Self {
            expansion_budget: 8,
            execution_step_budget: 256,
            active_set_budget: 8,
            stagnation_budget: 8,
            wall_time_budget_ms: 1_000,
            memory_budget_bytes: 4 * 1024 * 1024,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SparseRoutingStep {
    pub step: usize,
    pub total_available: usize,
    pub candidates_considered: usize,
    pub active_count: usize,
    pub full_catalog_scan: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct ReasoningMetrics {
    pub reasoning_depth: usize,
    pub reasoning_width: usize,
    pub live_branches: usize,
    pub concepts_composed: usize,
    pub peak_active_concepts: usize,
    pub search_expansions: usize,
    pub rollback_count: usize,
    pub decomposition_count: usize,
    pub recombination_count: usize,
    pub wall_time_ns: u128,
    pub memory_bytes: usize,
    pub total_available_primitives_concepts: usize,
    pub full_catalog_scans: usize,
    pub cache_hits: usize,
    pub macro_uses: usize,
    pub concept_uses: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SolveResult {
    pub task_id: String,
    pub committed_output: Option<Vec<i64>>,
    pub execution_error: Option<ExecutionError>,
    pub verified_after_commit: bool,
    pub termination: String,
    pub inferred_operator: Option<ScalarOperator>,
    pub program: Option<Vec<Instruction>>,
    pub derivation: DerivationGraph,
    pub metrics: ReasoningMetrics,
    pub routing_trace: Vec<SparseRoutingStep>,
}

impl SolveResult {
    pub fn committed(&self) -> Result<Vec<i64>, ExecutionError> {
        match (&self.committed_output, self.execution_error) {
            (Some(output), None) => Ok(output.clone()),
            (_, Some(error)) => Err(error),
            _ => Err(ExecutionError::MissingReturn),
        }
    }

    pub fn seal_score(&mut self, verified: bool) {
        self.verified_after_commit = verified;
        self.derivation.seal_verification(verified);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ConstructionState {
    Start,
    Initialized,
    Guarded,
    Read,
    Applied,
    Appended,
    Advanced,
    Looped,
    Done,
}

#[derive(Debug, Clone)]
struct PartialProgram {
    instructions: Vec<Instruction>,
    state: ConstructionState,
}

pub struct AdaptiveReasoner {
    primitive_count: usize,
}

impl Default for AdaptiveReasoner {
    fn default() -> Self {
        Self {
            primitive_count: 12,
        }
    }
}

impl AdaptiveReasoner {
    pub fn primitive_only(&self, task: &VisibleTask, budget: ResourceBudget) -> SolveResult {
        let started = Instant::now();
        let mut metrics = ReasoningMetrics {
            total_available_primitives_concepts: self.primitive_count,
            ..ReasoningMetrics::default()
        };
        let mut routing = Vec::new();
        let operator = infer_operator(
            task,
            &mut metrics,
            &mut routing,
            self.primitive_count,
            budget.expansion_budget,
        );
        let Some(operator) = operator else {
            return failed_result(
                task,
                metrics,
                routing,
                started,
                "SCALAR_INFERENCE_EXHAUSTED",
            );
        };
        let program = search_program(
            task,
            operator,
            budget,
            &mut metrics,
            &mut routing,
            self.primitive_count,
        );
        let Some(program) = program else {
            return failed_result(
                task,
                metrics,
                routing,
                started,
                "EXPANSION_BUDGET_EXHAUSTED",
            );
        };
        execute_committed(
            task, operator, program, budget, metrics, routing, started, None,
        )
    }

    pub fn exact_cache(
        &self,
        task: &VisibleTask,
        budget: ResourceBudget,
        cache: &[CacheEntry],
    ) -> SolveResult {
        let signature = exact_task_signature(task);
        if let Some(entry) = cache
            .iter()
            .find(|entry| entry.exact_signature_sha256 == signature)
        {
            let started = Instant::now();
            let mut result = result_from_reuse(
                task,
                entry.output.clone(),
                "CACHE_ENTRY",
                self.primitive_count,
                0,
                0,
                0,
                started,
                Vec::new(),
            );
            result.metrics.cache_hits = 1;
            result.termination = "CACHE_HIT_COMMITTED".to_string();
            return result;
        }
        self.primitive_only(task, budget)
    }

    pub fn structural_macro(
        &self,
        task: &VisibleTask,
        budget: ResourceBudget,
        structural_macro: &StructuralMacro,
    ) -> SolveResult {
        self.pattern_reuse(
            task,
            budget,
            &structural_macro.pattern,
            &structural_macro.macro_id,
            false,
        )
    }

    pub fn semantic_candidate(
        &self,
        task: &VisibleTask,
        budget: ResourceBudget,
        concept: &ConceptIR,
    ) -> SolveResult {
        let crate::substrate::ExecutableSemantics::Pattern(pattern) = &concept.transition_semantics
        else {
            return failed_result(
                task,
                ReasoningMetrics::default(),
                Vec::new(),
                Instant::now(),
                "CONCEPT_NOT_EXECUTABLE_PATTERN",
            );
        };
        self.pattern_reuse(task, budget, pattern, &concept.concept_id, true)
    }

    /// Executes an already promoted semantic pattern from the authoritative
    /// runtime state package. This is the extracted interface to the same
    /// `pattern_reuse` path used by `semantic_candidate`.
    pub fn semantic_pattern(
        &self,
        task: &VisibleTask,
        budget: ResourceBudget,
        pattern: &[InstructionPattern],
        concept_id: &str,
    ) -> SolveResult {
        self.pattern_reuse(task, budget, pattern, concept_id, true)
    }

    fn pattern_reuse(
        &self,
        task: &VisibleTask,
        budget: ResourceBudget,
        pattern: &[InstructionPattern],
        reusable_id: &str,
        semantic: bool,
    ) -> SolveResult {
        let started = Instant::now();
        let total_available = self.primitive_count + 1;
        let mut metrics = ReasoningMetrics {
            total_available_primitives_concepts: total_available,
            peak_active_concepts: 1,
            concepts_composed: usize::from(semantic),
            ..ReasoningMetrics::default()
        };
        let mut routing = Vec::new();
        let operator = infer_operator(
            task,
            &mut metrics,
            &mut routing,
            total_available,
            budget.expansion_budget,
        );
        let Some(operator) = operator else {
            return failed_result(
                task,
                metrics,
                routing,
                started,
                "SCALAR_INFERENCE_EXHAUSTED",
            );
        };
        if metrics.search_expansions >= budget.expansion_budget {
            return failed_result(
                task,
                metrics,
                routing,
                started,
                "EXPANSION_BUDGET_EXHAUSTED",
            );
        }
        metrics.search_expansions += 1;
        routing.push(SparseRoutingStep {
            step: routing.len(),
            total_available,
            candidates_considered: 1,
            active_count: 1,
            full_catalog_scan: false,
        });
        let program: Vec<Instruction> = pattern
            .iter()
            .map(|instruction| instruction.bind(operator))
            .collect();
        let mut result = execute_committed(
            task,
            operator,
            program,
            budget,
            metrics,
            routing,
            started,
            Some((reusable_id, semantic)),
        );
        if semantic {
            result.metrics.concept_uses = 1;
        } else {
            result.metrics.macro_uses = 1;
        }
        result
    }
}

fn infer_operator(
    task: &VisibleTask,
    metrics: &mut ReasoningMetrics,
    routing: &mut Vec<SparseRoutingStep>,
    total_available: usize,
    expansion_budget: usize,
) -> Option<ScalarOperator> {
    let candidates = [
        ScalarOperator::Add(task.scalar_parameter),
        ScalarOperator::Sub(task.scalar_parameter),
        ScalarOperator::Mul(task.scalar_parameter),
    ];
    let mut matched = None;
    for candidate in candidates {
        if metrics.search_expansions >= expansion_budget {
            break;
        }
        metrics.search_expansions += 1;
        metrics.reasoning_width = metrics.reasoning_width.max(candidates.len());
        metrics.live_branches = metrics.live_branches.max(candidates.len());
        metrics.peak_active_concepts = metrics.peak_active_concepts.max(3);
        routing.push(SparseRoutingStep {
            step: routing.len(),
            total_available,
            candidates_considered: 3,
            active_count: 3,
            full_catalog_scan: false,
        });
        let valid = task.demonstrations.iter().all(|demonstration| {
            let mut observed = Vec::new();
            for value in &demonstration.input {
                let Ok(transformed) = candidate.apply(*value) else {
                    return false;
                };
                observed.push(transformed);
            }
            observed == demonstration.observed_output
        });
        if valid {
            if matched.is_some() {
                return None;
            }
            matched = Some(candidate);
        } else {
            metrics.rollback_count += 1;
        }
    }
    matched
}

fn search_program(
    task: &VisibleTask,
    operator: ScalarOperator,
    budget: ResourceBudget,
    metrics: &mut ReasoningMetrics,
    routing: &mut Vec<SparseRoutingStep>,
    total_available: usize,
) -> Option<Vec<Instruction>> {
    let mut frontier = VecDeque::new();
    frontier.push_back(PartialProgram {
        instructions: Vec::new(),
        state: ConstructionState::Start,
    });
    let mut stagnation = 0usize;

    while let Some(partial) = frontier.pop_front() {
        if metrics.search_expansions >= budget.expansion_budget
            || stagnation >= budget.stagnation_budget
        {
            return None;
        }
        let candidates = next_instructions(partial.state, operator);
        metrics.reasoning_width = metrics
            .reasoning_width
            .max(frontier.len() + candidates.len());
        metrics.live_branches = metrics.live_branches.max(frontier.len() + candidates.len());
        metrics.peak_active_concepts = metrics
            .peak_active_concepts
            .max(candidates.len().min(budget.active_set_budget));
        routing.push(SparseRoutingStep {
            step: routing.len(),
            total_available,
            candidates_considered: candidates.len(),
            active_count: candidates.len().min(budget.active_set_budget),
            full_catalog_scan: false,
        });
        for (instruction, next_state) in candidates {
            if metrics.search_expansions >= budget.expansion_budget {
                return None;
            }
            metrics.search_expansions += 1;
            let mut program = partial.instructions.clone();
            program.push(instruction);
            if next_state == ConstructionState::Done {
                if demonstrations_match(task, &program, budget.execution_step_budget) {
                    return Some(program);
                }
                metrics.rollback_count += 1;
                stagnation += 1;
            } else {
                frontier.push_back(PartialProgram {
                    instructions: program,
                    state: next_state,
                });
            }
        }
    }
    None
}

fn next_instructions(
    state: ConstructionState,
    operator: ScalarOperator,
) -> Vec<(Instruction, ConstructionState)> {
    match state {
        ConstructionState::Start => vec![
            (Instruction::InitOutput, ConstructionState::Initialized),
            (Instruction::Return, ConstructionState::Done),
        ],
        ConstructionState::Initialized => vec![
            (Instruction::BranchIfEmpty(7), ConstructionState::Guarded),
            (Instruction::ReadCurrent, ConstructionState::Read),
            (Instruction::Return, ConstructionState::Done),
        ],
        ConstructionState::Guarded => vec![
            (Instruction::ReadCurrent, ConstructionState::Read),
            (Instruction::Return, ConstructionState::Done),
        ],
        ConstructionState::Read => vec![
            (
                Instruction::ApplyScalar(operator),
                ConstructionState::Applied,
            ),
            (Instruction::AppendCurrent, ConstructionState::Appended),
            (Instruction::Advance, ConstructionState::Advanced),
            (Instruction::Return, ConstructionState::Done),
        ],
        ConstructionState::Applied => vec![
            (Instruction::AppendCurrent, ConstructionState::Appended),
            (Instruction::Advance, ConstructionState::Advanced),
            (Instruction::Return, ConstructionState::Done),
        ],
        ConstructionState::Appended => vec![
            (Instruction::Advance, ConstructionState::Advanced),
            (Instruction::Return, ConstructionState::Done),
        ],
        ConstructionState::Advanced => vec![
            (Instruction::BranchIfRemaining(2), ConstructionState::Looped),
            (Instruction::ReadCurrent, ConstructionState::Read),
            (Instruction::Return, ConstructionState::Done),
        ],
        ConstructionState::Looped => vec![(Instruction::Return, ConstructionState::Done)],
        ConstructionState::Done => Vec::new(),
    }
}

fn demonstrations_match(task: &VisibleTask, program: &[Instruction], step_budget: usize) -> bool {
    task.demonstrations.iter().all(|demonstration| {
        execute_program(program, &demonstration.input, step_budget)
            .is_ok_and(|trace| trace.output == demonstration.observed_output)
    })
}

#[allow(clippy::too_many_arguments)]
fn execute_committed(
    task: &VisibleTask,
    operator: ScalarOperator,
    program: Vec<Instruction>,
    budget: ResourceBudget,
    mut metrics: ReasoningMetrics,
    routing: Vec<SparseRoutingStep>,
    started: Instant,
    reusable: Option<(&str, bool)>,
) -> SolveResult {
    let execution = execute_program(&program, &task.query_input, budget.execution_step_budget);
    let (output, error, trace) = match execution {
        Ok(trace) => (Some(trace.output.clone()), None, Some(trace)),
        Err(error) => (None, Some(error), None),
    };
    let derivation = build_derivation(task, &program, trace.as_ref(), reusable);
    metrics.reasoning_depth = derivation.operational_cost;
    metrics.wall_time_ns = started.elapsed().as_nanos();
    metrics.memory_bytes = serde_json::to_vec(&derivation).map_or(0, |bytes| bytes.len());
    SolveResult {
        task_id: task.task_id.clone(),
        committed_output: output,
        execution_error: error,
        verified_after_commit: false,
        termination: if trace.is_some() {
            "COMMITTED_UNSCORED".to_string()
        } else {
            "EXECUTION_REJECTED".to_string()
        },
        inferred_operator: Some(operator),
        program: Some(program),
        derivation,
        metrics,
        routing_trace: routing,
    }
}

fn build_derivation(
    task: &VisibleTask,
    program: &[Instruction],
    trace: Option<&ExecutionTrace>,
    reusable: Option<(&str, bool)>,
) -> DerivationGraph {
    let mut nodes = vec![DerivationNode {
        node_id: "N000000".to_string(),
        kind: DerivationNodeKind::Goal,
        executable_id: "G000001".to_string(),
        depth: 0,
        branch_id: 0,
        state_before: serde_json::to_string(&task.query_input).unwrap_or_default(),
        state_after: String::new(),
        cost: 0,
    }];
    let mut edges = Vec::new();
    let mut primitive_expansion = Vec::new();
    let mut previous = "N000000".to_string();

    if let Some((id, semantic)) = reusable {
        let node_id = "N000001".to_string();
        nodes.push(DerivationNode {
            node_id: node_id.clone(),
            kind: if semantic {
                DerivationNodeKind::ConceptExecution
            } else {
                DerivationNodeKind::MacroExecution
            },
            executable_id: id.to_string(),
            depth: 1,
            branch_id: 0,
            state_before: serde_json::to_string(&task.query_input).unwrap_or_default(),
            state_after: trace
                .map(|trace| serde_json::to_string(&trace.output).unwrap_or_default())
                .unwrap_or_default(),
            cost: 1,
        });
        edges.push(DerivationEdge {
            source: previous,
            target: node_id.clone(),
            dependency: "REQUIRES".to_string(),
        });
        previous = node_id;
        for instruction in program {
            primitive_expansion.push(primitive_id(instruction).to_string());
        }
    } else if let Some(trace) = trace {
        for (index, instruction_index) in trace.instruction_indices.iter().enumerate() {
            let Some(instruction) = program.get(*instruction_index) else {
                continue;
            };
            let node_id = format!("N{:06}", index + 1);
            let kind = if matches!(
                instruction,
                Instruction::BranchIfEmpty(_) | Instruction::BranchIfRemaining(_)
            ) {
                DerivationNodeKind::BranchDecision
            } else {
                DerivationNodeKind::PrimitiveExecution
            };
            let before = if index == 0 {
                serde_json::to_string(&task.query_input).unwrap_or_default()
            } else {
                serde_json::to_string(&trace.snapshots[index - 1]).unwrap_or_default()
            };
            let after = trace
                .snapshots
                .get(index)
                .map(|snapshot| serde_json::to_string(snapshot).unwrap_or_default())
                .unwrap_or_default();
            let executable_id = primitive_id(instruction).to_string();
            primitive_expansion.push(executable_id.clone());
            nodes.push(DerivationNode {
                node_id: node_id.clone(),
                kind,
                executable_id,
                depth: index + 1,
                branch_id: trace.branch_count,
                state_before: before,
                state_after: after,
                cost: 1,
            });
            edges.push(DerivationEdge {
                source: previous,
                target: node_id.clone(),
                dependency: "PRECEDES".to_string(),
            });
            previous = node_id;
        }
    }

    let final_node_id = format!("N{:06}", nodes.len());
    let final_output = trace.map(|trace| trace.output.clone());
    nodes.push(DerivationNode {
        node_id: final_node_id.clone(),
        kind: DerivationNodeKind::FinalOutput,
        executable_id: "V000001".to_string(),
        depth: nodes.len(),
        branch_id: 0,
        state_before: final_output
            .as_ref()
            .map(|output| serde_json::to_string(output).unwrap_or_default())
            .unwrap_or_default(),
        state_after: "COMMITTED".to_string(),
        cost: 0,
    });
    edges.push(DerivationEdge {
        source: previous,
        target: final_node_id,
        dependency: "VERIFIED_AFTER_COMMIT".to_string(),
    });
    let historical_cost = primitive_expansion.len();
    let operational_cost = if reusable.is_some() {
        1
    } else {
        historical_cost
    };
    DerivationGraph {
        graph_id: format!("D-{}", task.task_id),
        task_id: task.task_id.clone(),
        nodes,
        edges,
        primitive_expansion,
        final_output,
        verified: false,
        historical_cost,
        operational_cost,
    }
}

fn failed_result(
    task: &VisibleTask,
    mut metrics: ReasoningMetrics,
    routing: Vec<SparseRoutingStep>,
    started: Instant,
    termination: &str,
) -> SolveResult {
    metrics.wall_time_ns = started.elapsed().as_nanos();
    let derivation = DerivationGraph {
        graph_id: format!("D-{}", task.task_id),
        task_id: task.task_id.clone(),
        nodes: vec![DerivationNode {
            node_id: "N000000".to_string(),
            kind: DerivationNodeKind::Goal,
            executable_id: "G000001".to_string(),
            depth: 0,
            branch_id: 0,
            state_before: serde_json::to_string(&task.query_input).unwrap_or_default(),
            state_after: termination.to_string(),
            cost: 0,
        }],
        edges: Vec::new(),
        primitive_expansion: Vec::new(),
        final_output: None,
        verified: false,
        historical_cost: 0,
        operational_cost: 0,
    };
    metrics.memory_bytes = serde_json::to_vec(&derivation).map_or(0, |bytes| bytes.len());
    SolveResult {
        task_id: task.task_id.clone(),
        committed_output: None,
        execution_error: None,
        verified_after_commit: false,
        termination: termination.to_string(),
        inferred_operator: None,
        program: None,
        derivation,
        metrics,
        routing_trace: routing,
    }
}

#[allow(clippy::too_many_arguments)]
fn result_from_reuse(
    task: &VisibleTask,
    output: Vec<i64>,
    executable_id: &str,
    total_available: usize,
    concepts_composed: usize,
    macro_uses: usize,
    concept_uses: usize,
    started: Instant,
    primitive_expansion: Vec<String>,
) -> SolveResult {
    let graph = DerivationGraph {
        graph_id: format!("D-{}", task.task_id),
        task_id: task.task_id.clone(),
        nodes: vec![
            DerivationNode {
                node_id: "N000000".to_string(),
                kind: DerivationNodeKind::Goal,
                executable_id: "G000001".to_string(),
                depth: 0,
                branch_id: 0,
                state_before: serde_json::to_string(&task.query_input).unwrap_or_default(),
                state_after: String::new(),
                cost: 0,
            },
            DerivationNode {
                node_id: "N000001".to_string(),
                kind: DerivationNodeKind::ConceptExecution,
                executable_id: executable_id.to_string(),
                depth: 1,
                branch_id: 0,
                state_before: String::new(),
                state_after: serde_json::to_string(&output).unwrap_or_default(),
                cost: 1,
            },
            DerivationNode {
                node_id: "N000002".to_string(),
                kind: DerivationNodeKind::FinalOutput,
                executable_id: "V000001".to_string(),
                depth: 2,
                branch_id: 0,
                state_before: serde_json::to_string(&output).unwrap_or_default(),
                state_after: "COMMITTED".to_string(),
                cost: 0,
            },
        ],
        edges: vec![
            DerivationEdge {
                source: "N000000".to_string(),
                target: "N000001".to_string(),
                dependency: "REQUIRES".to_string(),
            },
            DerivationEdge {
                source: "N000001".to_string(),
                target: "N000002".to_string(),
                dependency: "VERIFIED_AFTER_COMMIT".to_string(),
            },
        ],
        primitive_expansion,
        final_output: Some(output.clone()),
        verified: false,
        historical_cost: 1,
        operational_cost: 1,
    };
    SolveResult {
        task_id: task.task_id.clone(),
        committed_output: Some(output),
        execution_error: None,
        verified_after_commit: false,
        termination: "COMMITTED_UNSCORED".to_string(),
        inferred_operator: None,
        program: None,
        derivation: graph.clone(),
        metrics: ReasoningMetrics {
            reasoning_depth: 1,
            reasoning_width: 1,
            live_branches: 1,
            concepts_composed,
            peak_active_concepts: usize::from(concept_uses > 0),
            search_expansions: 0,
            rollback_count: 0,
            decomposition_count: 0,
            recombination_count: 0,
            wall_time_ns: started.elapsed().as_nanos(),
            memory_bytes: serde_json::to_vec(&graph).map_or(0, |bytes| bytes.len()),
            total_available_primitives_concepts: total_available,
            full_catalog_scans: 0,
            cache_hits: 1,
            macro_uses,
            concept_uses,
        },
        routing_trace: Vec::new(),
    }
}

pub fn exact_task_signature(task: &VisibleTask) -> String {
    let bytes = serde_json::to_vec(task).expect("visible task serializes");
    format!("{:x}", Sha256::digest(bytes))
}

pub fn primitive_id(instruction: &Instruction) -> &'static str {
    match instruction {
        Instruction::InitOutput => "P000003",
        Instruction::BranchIfEmpty(_) => "P000008",
        Instruction::ReadCurrent => "P000004",
        Instruction::ApplyScalar(ScalarOperator::Add(_)) => "P000010",
        Instruction::ApplyScalar(ScalarOperator::Sub(_)) => "P000011",
        Instruction::ApplyScalar(ScalarOperator::Mul(_)) => "P000012",
        Instruction::AppendCurrent => "P000006",
        Instruction::Advance => "P000007",
        Instruction::BranchIfRemaining(_) => "P000009",
        Instruction::Return => "P000005",
    }
}

#[cfg(test)]
mod tests {
    use super::{AdaptiveReasoner, ResourceBudget};
    use crate::task::{Demonstration, Split, VisibleTask};

    fn task() -> VisibleTask {
        VisibleTask {
            task_id: "CORE-UNIT-001".to_string(),
            split: Split::Calibration,
            scalar_parameter: 5,
            demonstrations: vec![
                Demonstration {
                    input: vec![1, -2, 4],
                    observed_output: vec![6, 3, 9],
                },
                Demonstration {
                    input: vec![0, 3],
                    observed_output: vec![5, 8],
                },
            ],
            query_input: vec![7, -3, 2, 0, 11],
        }
    }

    #[test]
    fn adaptive_search_solves_discovery_without_fixed_depth_ceiling() {
        let reasoner = AdaptiveReasoner::default();
        let mut result = reasoner.primitive_only(&task(), ResourceBudget::discovery());
        let verified = result.committed() == Ok(vec![12, 2, 7, 5, 16]);
        result.seal_score(verified);
        assert!(result.verified_after_commit);
        assert!(result.metrics.reasoning_depth > 5);
        assert!(result.derivation.validate_integrity());
    }

    #[test]
    fn tight_expansion_budget_fails_closed() {
        let reasoner = AdaptiveReasoner::default();
        let result = reasoner.primitive_only(&task(), ResourceBudget::blind());
        assert!(result.committed_output.is_none());
        assert_eq!(result.termination, "EXPANSION_BUDGET_EXHAUSTED");
    }
}
