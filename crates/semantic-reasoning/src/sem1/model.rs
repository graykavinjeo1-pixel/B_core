use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::{
    dsl::{execute_program as execute_sem0_program, InstructionPattern, ScalarOperator},
    substrate::{ConceptIR, ExecutableSemantics},
};

use super::integrity::hash_serializable;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Sem1ValueType {
    #[serde(rename = "V001")]
    IntegerSequence,
    #[serde(rename = "V002")]
    Integer,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", content = "value")]
pub enum Value {
    #[serde(rename = "V001")]
    IntegerSequence(Vec<i64>),
    #[serde(rename = "V002")]
    Integer(i64),
}

impl Value {
    pub fn value_type(&self) -> Sem1ValueType {
        match self {
            Self::IntegerSequence(_) => Sem1ValueType::IntegerSequence,
            Self::Integer(_) => Sem1ValueType::Integer,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(tag = "code", content = "parameter")]
pub enum CheckedOperator {
    #[serde(rename = "S011")]
    Add(i64),
    #[serde(rename = "S012")]
    Sub(i64),
    #[serde(rename = "S013")]
    Mul(i64),
    #[serde(rename = "S091")]
    AddViaSubNeg(i64),
    #[serde(rename = "S092")]
    MulViaRepeatedAdd(i64),
}

impl CheckedOperator {
    pub fn apply(self, value: i64) -> Result<i64, ExecutionFault> {
        match self {
            Self::Add(parameter) => value
                .checked_add(parameter)
                .ok_or(ExecutionFault::ArithmeticOverflow),
            Self::Sub(parameter) => value
                .checked_sub(parameter)
                .ok_or(ExecutionFault::ArithmeticOverflow),
            Self::Mul(parameter) => value
                .checked_mul(parameter)
                .ok_or(ExecutionFault::ArithmeticOverflow),
            Self::AddViaSubNeg(parameter) => value
                .checked_sub(
                    parameter
                        .checked_neg()
                        .ok_or(ExecutionFault::ArithmeticOverflow)?,
                )
                .ok_or(ExecutionFault::ArithmeticOverflow),
            Self::MulViaRepeatedAdd(parameter) => {
                if parameter == 0 {
                    return Ok(0);
                }
                let count = parameter.unsigned_abs();
                if count > 64 {
                    return Err(ExecutionFault::OperationDomainViolation);
                }
                let mut output = 0i64;
                for _ in 0..count {
                    output = output
                        .checked_add(value)
                        .ok_or(ExecutionFault::ArithmeticOverflow)?;
                }
                if parameter < 0 {
                    output
                        .checked_neg()
                        .ok_or(ExecutionFault::ArithmeticOverflow)
                } else {
                    Ok(output)
                }
            }
        }
    }

    pub fn canonical_sem0(self) -> ScalarOperator {
        match self {
            Self::Add(parameter) | Self::AddViaSubNeg(parameter) => ScalarOperator::Add(parameter),
            Self::Sub(parameter) => ScalarOperator::Sub(parameter),
            Self::Mul(parameter) | Self::MulViaRepeatedAdd(parameter) => {
                ScalarOperator::Mul(parameter)
            }
        }
    }

    pub fn semantic_relation_key(self) -> String {
        match self {
            Self::Add(parameter) | Self::AddViaSubNeg(parameter) => {
                format!("CHECKED_ADD:{parameter}")
            }
            Self::Sub(parameter) => format!("CHECKED_SUB:{parameter}"),
            Self::Mul(parameter) | Self::MulViaRepeatedAdd(parameter) => {
                format!("CHECKED_MUL:{parameter}")
            }
        }
    }

    pub fn structural_code(self) -> &'static str {
        match self {
            Self::Add(_) => "S011",
            Self::Sub(_) => "S012",
            Self::Mul(_) => "S013",
            Self::AddViaSubNeg(_) => "S091",
            Self::MulViaRepeatedAdd(_) => "S092",
        }
    }

    pub fn is_equivalent_variant(self) -> bool {
        matches!(self, Self::AddViaSubNeg(_) | Self::MulViaRepeatedAdd(_))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Predicate {
    #[serde(rename = "S021")]
    Positive,
    #[serde(rename = "S022")]
    NonZero,
    #[serde(rename = "S023")]
    Even,
    #[serde(rename = "S024")]
    Negative,
}

impl Predicate {
    pub fn test(self, value: i64) -> bool {
        match self {
            Self::Positive => value > 0,
            Self::NonZero => value != 0,
            Self::Even => matches!((value).checked_rem(2), Some(0)),
            Self::Negative => value < 0,
        }
    }

    pub fn structural_code(self) -> &'static str {
        match self {
            Self::Positive => "S021",
            Self::NonZero => "S022",
            Self::Even => "S023",
            Self::Negative => "S024",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Reducer {
    #[serde(rename = "S031")]
    Sum,
    #[serde(rename = "S032")]
    Product,
}

impl Reducer {
    pub fn structural_code(self) -> &'static str {
        match self {
            Self::Sum => "S031",
            Self::Product => "S032",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum StageKind {
    #[serde(rename = "P100001")]
    Transform,
    #[serde(rename = "P100002")]
    Retain,
    #[serde(rename = "P100003")]
    Aggregate,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(tag = "stage", content = "binding")]
pub enum Stage {
    #[serde(rename = "P100001")]
    Transform(CheckedOperator),
    #[serde(rename = "P100002")]
    Retain(Predicate),
    #[serde(rename = "P100003")]
    Aggregate(Reducer),
}

impl Stage {
    pub fn kind(&self) -> StageKind {
        match self {
            Self::Transform(_) => StageKind::Transform,
            Self::Retain(_) => StageKind::Retain,
            Self::Aggregate(_) => StageKind::Aggregate,
        }
    }

    pub fn input_type(&self) -> Sem1ValueType {
        Sem1ValueType::IntegerSequence
    }

    pub fn output_type(&self) -> Sem1ValueType {
        match self {
            Self::Transform(_) | Self::Retain(_) => Sem1ValueType::IntegerSequence,
            Self::Aggregate(_) => Sem1ValueType::Integer,
        }
    }

    pub fn structural_code(&self) -> String {
        match self {
            Self::Transform(operator) => operator.structural_code().to_string(),
            Self::Retain(predicate) => predicate.structural_code().to_string(),
            Self::Aggregate(reducer) => reducer.structural_code().to_string(),
        }
    }

    pub fn semantic_relation_key(&self) -> String {
        match self {
            Self::Transform(operator) => format!("R100:{}", operator.semantic_relation_key()),
            Self::Retain(predicate) => format!("R200:{predicate:?}"),
            Self::Aggregate(reducer) => format!("R300:{reducer:?}"),
        }
    }

    pub fn primitive_id(&self) -> &'static str {
        match self {
            Self::Transform(_) => "P100001",
            Self::Retain(_) => "P100002",
            Self::Aggregate(_) => "P100003",
        }
    }

    pub fn expanded_graph_nodes(&self) -> usize {
        match self {
            Self::Transform(_) => 8,
            Self::Retain(_) => 5,
            Self::Aggregate(_) => 4,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StageCapability {
    pub capability_id: String,
    pub stage: Stage,
    pub structural_tokens: Vec<String>,
}

impl StageCapability {
    pub fn new(capability_id: impl Into<String>, stage: Stage) -> Self {
        let structural_tokens = match &stage {
            Stage::Transform(operator) if operator.is_equivalent_variant() => {
                vec![operator.structural_code().to_string(), "S099".to_string()]
            }
            _ => vec![stage.structural_code()],
        };
        Self {
            capability_id: capability_id.into(),
            stage,
            structural_tokens,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ParameterKind {
    #[serde(rename = "K001")]
    CheckedScalarOperator,
    #[serde(rename = "K002")]
    Predicate,
    #[serde(rename = "K003")]
    Reducer,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value")]
pub enum Binding {
    #[serde(rename = "K001")]
    CheckedScalarOperator(CheckedOperator),
    #[serde(rename = "K002")]
    Predicate(Predicate),
    #[serde(rename = "K003")]
    Reducer(Reducer),
}

impl Binding {
    pub fn kind(&self) -> ParameterKind {
        match self {
            Self::CheckedScalarOperator(_) => ParameterKind::CheckedScalarOperator,
            Self::Predicate(_) => ParameterKind::Predicate,
            Self::Reducer(_) => ParameterKind::Reducer,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "stage")]
pub enum StageTemplate {
    #[serde(rename = "P100001")]
    Transform { slot: usize },
    #[serde(rename = "P100002")]
    Retain { slot: usize },
    #[serde(rename = "P100003")]
    Aggregate { slot: usize },
}

impl StageTemplate {
    pub fn kind(&self) -> StageKind {
        match self {
            Self::Transform { .. } => StageKind::Transform,
            Self::Retain { .. } => StageKind::Retain,
            Self::Aggregate { .. } => StageKind::Aggregate,
        }
    }

    pub fn instantiate(&self, bindings: &[Binding]) -> Result<Stage, ExecutionFault> {
        match self {
            Self::Transform { slot } => match bindings.get(*slot) {
                Some(Binding::CheckedScalarOperator(value)) => Ok(Stage::Transform(*value)),
                _ => Err(ExecutionFault::BindingMismatch),
            },
            Self::Retain { slot } => match bindings.get(*slot) {
                Some(Binding::Predicate(value)) => Ok(Stage::Retain(*value)),
                _ => Err(ExecutionFault::BindingMismatch),
            },
            Self::Aggregate { slot } => match bindings.get(*slot) {
                Some(Binding::Reducer(value)) => Ok(Stage::Aggregate(*value)),
                _ => Err(ExecutionFault::BindingMismatch),
            },
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "node", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum OperationalTemplateNode {
    Concept {
        concept_id: String,
        binding_slots: Vec<usize>,
    },
    Primitive {
        stage: StageTemplate,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConceptRecord {
    pub concept_id: String,
    pub generation: usize,
    pub promotion_state: String,
    pub signature_input: Sem1ValueType,
    pub signature_output: Sem1ValueType,
    pub parameter_kinds: Vec<ParameterKind>,
    pub operational_template: Vec<OperationalTemplateNode>,
    pub primitive_expansion: Vec<StageTemplate>,
    pub direct_parent_concepts: Vec<String>,
    pub ancestor_concept_ids: Vec<String>,
    pub primitive_ancestor_ids: Vec<String>,
    pub complete_ancestor_set: Vec<String>,
    pub epistemic_historical_depth: usize,
    pub operational_depth: usize,
    pub operational_cost: usize,
    pub composition_arity: usize,
    pub promoted_ancestor_count: usize,
    pub source_task_ids: Vec<String>,
    pub source_derivation_ids: Vec<String>,
    pub preconditions: Vec<String>,
    pub invariants: Vec<String>,
    pub predictions: Vec<String>,
    pub counterfactual_interface: Vec<String>,
    pub content_hash_sha256: String,
    pub derived_autonomously: bool,
    pub lexical_information_used: bool,
}

impl ConceptRecord {
    pub fn freeze_hash(&mut self) -> Result<(), String> {
        self.content_hash_sha256.clear();
        self.content_hash_sha256 = hash_serializable(self)?;
        Ok(())
    }

    pub fn instantiate(&self, bindings: &[Binding]) -> Result<Vec<Stage>, ExecutionFault> {
        if bindings.len() != self.parameter_kinds.len()
            || bindings
                .iter()
                .zip(&self.parameter_kinds)
                .any(|(binding, kind)| binding.kind() != *kind)
        {
            return Err(ExecutionFault::BindingMismatch);
        }
        self.primitive_expansion
            .iter()
            .map(|stage| stage.instantiate(bindings))
            .collect()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MacroRecord {
    pub macro_id: String,
    pub parameter_kinds: Vec<ParameterKind>,
    pub structural_pattern: Vec<Vec<String>>,
    pub operational_template: Vec<MacroTemplateNode>,
    pub primitive_expansion: Vec<StageTemplate>,
    pub direct_parent_macros: Vec<String>,
    pub source_derivation_ids: Vec<String>,
    pub typed_parameters: bool,
    pub variable_operators: bool,
    pub composition_supported: bool,
    pub macro_on_macro_reuse: bool,
    pub semantic_validation: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "node", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum MacroTemplateNode {
    Macro {
        macro_id: String,
        binding_slots: Vec<usize>,
    },
    Primitive {
        stage: StageTemplate,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConceptInstance {
    pub concept_id: String,
    pub bindings: Vec<Binding>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "node", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PlanNode {
    Concept(ConceptInstance),
    Macro {
        macro_id: String,
        bindings: Vec<Binding>,
    },
    Primitive(Stage),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Plan {
    pub nodes: Vec<PlanNode>,
    pub expanded_stages: Vec<Stage>,
    pub direct_concept_ids: Vec<String>,
    pub all_executed_concept_ids: Vec<String>,
    pub primitive_ancestor_ids: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ExecutionFault {
    TypeMismatch,
    ArithmeticOverflow,
    OperationDomainViolation,
    BindingMismatch,
    MissingConcept,
    MissingMacro,
    InvalidPredecessorSemantics,
    StepBudgetExhausted,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExecutionRecord {
    pub value: Value,
    pub reasoning_depth: usize,
    pub primitive_expanded_depth: usize,
    pub graph_nodes: usize,
    pub graph_edges: usize,
    pub operational_nodes: usize,
    pub operational_depth: usize,
    pub executed_concepts: Vec<String>,
}

pub fn execute_stage(stage: &Stage, input: Value) -> Result<(Value, usize), ExecutionFault> {
    match (stage, input) {
        (Stage::Transform(operator), Value::IntegerSequence(values)) => {
            let output = values
                .iter()
                .map(|value| operator.apply(*value))
                .collect::<Result<Vec<_>, _>>()?;
            Ok((
                Value::IntegerSequence(output),
                values.len().saturating_mul(4).saturating_add(4),
            ))
        }
        (Stage::Retain(predicate), Value::IntegerSequence(values)) => {
            let cost = values.len().saturating_mul(2).saturating_add(2);
            Ok((
                Value::IntegerSequence(
                    values
                        .into_iter()
                        .filter(|value| predicate.test(*value))
                        .collect(),
                ),
                cost,
            ))
        }
        (Stage::Aggregate(reducer), Value::IntegerSequence(values)) => {
            let cost = values.len().saturating_add(2);
            let result = match reducer {
                Reducer::Sum => values.into_iter().try_fold(0i64, |acc, value| {
                    acc.checked_add(value)
                        .ok_or(ExecutionFault::ArithmeticOverflow)
                })?,
                Reducer::Product => values.into_iter().try_fold(1i64, |acc, value| {
                    acc.checked_mul(value)
                        .ok_or(ExecutionFault::ArithmeticOverflow)
                })?,
            };
            Ok((Value::Integer(result), cost))
        }
        _ => Err(ExecutionFault::TypeMismatch),
    }
}

pub fn execute_primitive_pipeline(
    stages: &[Stage],
    input: Value,
) -> Result<ExecutionRecord, ExecutionFault> {
    let mut value = input;
    let mut depth = 0usize;
    for stage in stages {
        let (next, cost) = execute_stage(stage, value)?;
        value = next;
        depth = depth.saturating_add(cost);
    }
    let graph_nodes = stages.iter().map(Stage::expanded_graph_nodes).sum();
    Ok(ExecutionRecord {
        value,
        reasoning_depth: depth,
        primitive_expanded_depth: graph_nodes,
        graph_nodes,
        graph_edges: graph_nodes.saturating_sub(1),
        operational_nodes: stages.len(),
        operational_depth: stages.len(),
        executed_concepts: Vec::new(),
    })
}

pub fn execute_concept_instance(
    instance: &ConceptInstance,
    input: Value,
    catalog: &BTreeMap<String, ConceptRecord>,
    predecessor: &ConceptIR,
) -> Result<ExecutionRecord, ExecutionFault> {
    let concept = catalog
        .get(&instance.concept_id)
        .ok_or(ExecutionFault::MissingConcept)?;
    if concept.concept_id == "C000001" {
        return execute_c000001(predecessor, &instance.bindings, input);
    }
    let mut value = input;
    let mut reasoning_depth = 0usize;
    let mut expanded_depth = 0usize;
    let mut graph_nodes = 0usize;
    let mut graph_edges = 0usize;
    let mut executed = vec![concept.concept_id.clone()];
    for node in &concept.operational_template {
        match node {
            OperationalTemplateNode::Concept {
                concept_id,
                binding_slots,
            } => {
                let bindings = binding_slots
                    .iter()
                    .map(|slot| {
                        instance
                            .bindings
                            .get(*slot)
                            .cloned()
                            .ok_or(ExecutionFault::BindingMismatch)
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                let record = execute_concept_instance(
                    &ConceptInstance {
                        concept_id: concept_id.clone(),
                        bindings,
                    },
                    value,
                    catalog,
                    predecessor,
                )?;
                value = record.value;
                reasoning_depth = reasoning_depth.saturating_add(record.reasoning_depth);
                expanded_depth = expanded_depth.saturating_add(record.primitive_expanded_depth);
                graph_nodes = graph_nodes.saturating_add(record.graph_nodes);
                graph_edges = graph_edges.saturating_add(record.graph_edges);
                executed.extend(record.executed_concepts);
            }
            OperationalTemplateNode::Primitive { stage } => {
                let instantiated = stage.instantiate(&instance.bindings)?;
                let (next, cost) = execute_stage(&instantiated, value)?;
                value = next;
                reasoning_depth = reasoning_depth.saturating_add(cost);
                expanded_depth = expanded_depth.saturating_add(instantiated.expanded_graph_nodes());
                graph_nodes = graph_nodes.saturating_add(instantiated.expanded_graph_nodes());
                graph_edges = graph_edges
                    .saturating_add(instantiated.expanded_graph_nodes().saturating_sub(1));
            }
        }
    }
    executed.sort();
    executed.dedup();
    Ok(ExecutionRecord {
        value,
        reasoning_depth,
        primitive_expanded_depth: expanded_depth,
        graph_nodes,
        graph_edges: graph_edges
            .saturating_add(concept.operational_template.len().saturating_sub(1)),
        operational_nodes: 1,
        operational_depth: 1,
        executed_concepts: executed,
    })
}

fn execute_c000001(
    predecessor: &ConceptIR,
    bindings: &[Binding],
    input: Value,
) -> Result<ExecutionRecord, ExecutionFault> {
    let operator = match bindings {
        [Binding::CheckedScalarOperator(operator)] => operator.canonical_sem0(),
        _ => return Err(ExecutionFault::BindingMismatch),
    };
    let values = match input {
        Value::IntegerSequence(values) => values,
        _ => return Err(ExecutionFault::TypeMismatch),
    };
    let pattern = match &predecessor.transition_semantics {
        ExecutableSemantics::Pattern(pattern) => pattern,
        _ => return Err(ExecutionFault::InvalidPredecessorSemantics),
    };
    let program = pattern
        .iter()
        .map(|instruction| instruction.bind(operator))
        .collect::<Vec<_>>();
    let step_budget = values.len().saturating_mul(8).saturating_add(16);
    let trace =
        execute_sem0_program(&program, &values, step_budget).map_err(|error| match error {
            crate::dsl::ExecutionError::ArithmeticOverflow => ExecutionFault::ArithmeticOverflow,
            crate::dsl::ExecutionError::StepBudgetExhausted => ExecutionFault::StepBudgetExhausted,
            _ => ExecutionFault::InvalidPredecessorSemantics,
        })?;
    Ok(ExecutionRecord {
        value: Value::IntegerSequence(trace.output),
        reasoning_depth: trace.instruction_indices.len(),
        primitive_expanded_depth: pattern.len(),
        graph_nodes: pattern.len(),
        graph_edges: pattern.len().saturating_sub(1),
        operational_nodes: 1,
        operational_depth: 1,
        executed_concepts: vec!["C000001".to_string()],
    })
}

pub fn semantic_preflight(stages: &[Stage], input: &Value) -> bool {
    execute_primitive_pipeline(stages, input.clone()).is_ok()
}

pub fn stage_templates_from_kinds(kinds: &[StageKind]) -> (Vec<ParameterKind>, Vec<StageTemplate>) {
    let mut parameters = Vec::new();
    let mut templates = Vec::new();
    for kind in kinds {
        let slot = parameters.len();
        match kind {
            StageKind::Transform => {
                parameters.push(ParameterKind::CheckedScalarOperator);
                templates.push(StageTemplate::Transform { slot });
            }
            StageKind::Retain => {
                parameters.push(ParameterKind::Predicate);
                templates.push(StageTemplate::Retain { slot });
            }
            StageKind::Aggregate => {
                parameters.push(ParameterKind::Reducer);
                templates.push(StageTemplate::Aggregate { slot });
            }
        }
    }
    (parameters, templates)
}

pub fn bindings_from_stages(stages: &[Stage]) -> Vec<Binding> {
    stages
        .iter()
        .map(|stage| match stage {
            Stage::Transform(value) => Binding::CheckedScalarOperator(*value),
            Stage::Retain(value) => Binding::Predicate(*value),
            Stage::Aggregate(value) => Binding::Reducer(*value),
        })
        .collect()
}

pub fn stage_kinds(stages: &[Stage]) -> Vec<StageKind> {
    stages.iter().map(Stage::kind).collect()
}

pub fn ancestor_closure(
    direct_parents: &[String],
    catalog: &BTreeMap<String, ConceptRecord>,
) -> BTreeSet<String> {
    let mut ancestors = BTreeSet::new();
    let mut pending = direct_parents.to_vec();
    while let Some(id) = pending.pop() {
        if ancestors.insert(id.clone()) {
            if let Some(parent) = catalog.get(&id) {
                pending.extend(parent.direct_parent_concepts.iter().cloned());
            }
        }
    }
    ancestors
}

pub fn c000001_record(predecessor: &ConceptIR) -> Result<ConceptRecord, String> {
    let pattern_len = match &predecessor.transition_semantics {
        ExecutableSemantics::Pattern(pattern)
            if pattern
                == &vec![
                    InstructionPattern::InitOutput,
                    InstructionPattern::BranchIfEmpty(7),
                    InstructionPattern::ReadCurrent,
                    InstructionPattern::ScalarSlot,
                    InstructionPattern::AppendCurrent,
                    InstructionPattern::Advance,
                    InstructionPattern::BranchIfRemaining(2),
                    InstructionPattern::Return,
                ] =>
        {
            pattern.len()
        }
        _ => return Err("PREDECESSOR_INTEGRITY_FAILURE:C000001_EXECUTABLE".to_string()),
    };
    Ok(ConceptRecord {
        concept_id: predecessor.concept_id.clone(),
        generation: 1,
        promotion_state: "PROMOTED_IMMUTABLE_PREDECESSOR".to_string(),
        signature_input: Sem1ValueType::IntegerSequence,
        signature_output: Sem1ValueType::IntegerSequence,
        parameter_kinds: vec![ParameterKind::CheckedScalarOperator],
        operational_template: Vec::new(),
        primitive_expansion: vec![StageTemplate::Transform { slot: 0 }],
        direct_parent_concepts: Vec::new(),
        ancestor_concept_ids: Vec::new(),
        primitive_ancestor_ids: predecessor.provenance.primitive_ids.clone(),
        complete_ancestor_set: predecessor.provenance.primitive_ids.clone(),
        epistemic_historical_depth: predecessor.historical_derivation_cost,
        operational_depth: predecessor.operational_cost,
        operational_cost: predecessor.operational_cost,
        composition_arity: 1,
        promoted_ancestor_count: 0,
        source_task_ids: predecessor.provenance.source_task_ids.clone(),
        source_derivation_ids: predecessor.provenance.source_derivation_ids.clone(),
        preconditions: predecessor
            .preconditions
            .iter()
            .map(|value| format!("{value:?}"))
            .collect(),
        invariants: predecessor
            .invariants
            .iter()
            .map(|value| format!("{value:?}"))
            .collect(),
        predictions: predecessor
            .predictions
            .iter()
            .map(|value| format!("{value:?}"))
            .collect(),
        counterfactual_interface: predecessor
            .counterfactual_interface
            .iter()
            .map(|value| format!("{value:?}"))
            .collect(),
        content_hash_sha256: predecessor.content_hash_sha256.clone(),
        derived_autonomously: true,
        lexical_information_used: false,
    })
    .and_then(|record| {
        if pattern_len == 8 {
            Ok(record)
        } else {
            Err("C000001_PATTERN_LENGTH".to_string())
        }
    })
}

#[cfg(test)]
mod tests {
    use super::{execute_primitive_pipeline, CheckedOperator, Predicate, Reducer, Stage, Value};

    #[test]
    fn multi_stage_pipeline_changes_type_and_preserves_checked_behavior() {
        let stages = vec![
            Stage::Transform(CheckedOperator::Add(2)),
            Stage::Retain(Predicate::Positive),
            Stage::Aggregate(Reducer::Sum),
        ];
        let record = execute_primitive_pipeline(&stages, Value::IntegerSequence(vec![-4, 1, 3]))
            .expect("valid pipeline");
        assert_eq!(record.value, Value::Integer(8));
        assert!(record.reasoning_depth > record.operational_depth);
    }

    #[test]
    fn equivalent_operator_variants_share_semantic_relation_not_structure() {
        let direct = CheckedOperator::Add(4);
        let expanded = CheckedOperator::AddViaSubNeg(4);
        assert_eq!(
            direct.semantic_relation_key(),
            expanded.semantic_relation_key()
        );
        assert_ne!(direct.structural_code(), expanded.structural_code());
        assert_eq!(direct.apply(3), expanded.apply(3));
    }
}
