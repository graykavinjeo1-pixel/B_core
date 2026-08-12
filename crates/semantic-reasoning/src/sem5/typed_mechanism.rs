//! Name-independent lowering from a typed mechanism goal to executable SEM-5
//! syntax.  This is deliberately a compiler boundary: the goal supplies typed
//! operand roles and an expression graph, while this module resolves those
//! roles to concrete repository expressions, checks types/effects, parses the
//! resulting Rust syntax, and falsifies it against public observations.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::bounded_parallel::{
    map_ordered_batched as parallel_map_ordered_batched, worker_count_for,
};
use crate::self_repair_contract::sha256;

use super::ir::eval_scalar;
use super::model::{
    ApiDefinition, BinaryOperator, BindingSpec, DataSplit, Effect, ProgramTask, ProgramType,
    RelationSpec, ScalarExpression, UnaryOperator, Value,
};
use super::tasks::evaluate_contract;

pub const TYPED_MECHANISM_GOAL_SCHEMA: &str = "B_CORE_TYPED_MECHANISM_GOAL_1";
pub const TYPED_MECHANISM_SYNTHESIS_GOAL_SCHEMA: &str = "B_CORE_TYPED_MECHANISM_SYNTHESIS_GOAL_1";
pub const CONCRETE_SYNTAX_TEMPLATE_SCHEMA: &str = "B_CORE_CONCRETE_SYNTAX_TEMPLATE_1";
const MAX_MECHANISM_OPERANDS: usize = 32;
const MAX_MECHANISM_EXPRESSION_NODES: usize = 256;
const MAX_MECHANISM_OBSERVATIONS: usize = 64;
const MAX_SYNTHESIS_CANDIDATES: usize = 1_024;
const MAX_SYNTHESIS_DEPTH: usize = 3;
const TYPED_OPERATOR_REPLAY_ITEMS_PER_WORKER: usize = 16;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceOperandIR {
    /// Stable semantic role used by the mechanism graph.
    pub role: String,
    /// Concrete Rust expression selected from the repository AST.
    pub source: String,
    pub value_type: ProgramType,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "syntax_kind", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TypedSyntaxExpressionIR {
    Operand {
        role: String,
    },
    IntLiteral {
        value: i64,
    },
    BoolLiteral {
        value: bool,
    },
    Unary {
        operator: UnaryOperator,
        input: Box<TypedSyntaxExpressionIR>,
    },
    Binary {
        operator: BinaryOperator,
        left: Box<TypedSyntaxExpressionIR>,
        right: Box<TypedSyntaxExpressionIR>,
    },
    Length {
        input: Box<TypedSyntaxExpressionIR>,
    },
    Index {
        collection: Box<TypedSyntaxExpressionIR>,
        index: Box<TypedSyntaxExpressionIR>,
    },
    Call {
        api_token: String,
        arguments: Vec<TypedSyntaxExpressionIR>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TypedMechanismObservationIR {
    pub operands: BTreeMap<String, Value>,
    pub expected_postimage: Value,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TypedMechanismGoalIR {
    pub schema: String,
    pub goal_id: String,
    pub split: DataSplit,
    pub operands: Vec<SourceOperandIR>,
    pub output_type: ProgramType,
    pub condition: Option<TypedSyntaxExpressionIR>,
    pub postimage: TypedSyntaxExpressionIR,
    pub otherwise: Option<TypedSyntaxExpressionIR>,
    pub definitions: Vec<ApiDefinition>,
    pub allowed_effects: Vec<Effect>,
    pub preconditions: Vec<String>,
    pub postconditions: Vec<String>,
    pub invariants: Vec<String>,
    pub public_observations: Vec<TypedMechanismObservationIR>,
    pub provenance: Vec<String>,
}

/// A synthesis goal deliberately omits condition/postimage syntax.  Those
/// expressions must be discovered from the typed operands, API signatures,
/// and public observations rather than selected by a task-name switch.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TypedMechanismSynthesisGoalIR {
    pub schema: String,
    pub goal_id: String,
    pub split: DataSplit,
    pub operands: Vec<SourceOperandIR>,
    pub output_type: ProgramType,
    pub definitions: Vec<ApiDefinition>,
    pub allowed_effects: Vec<Effect>,
    pub preconditions: Vec<String>,
    pub postconditions: Vec<String>,
    pub invariants: Vec<String>,
    pub public_observations: Vec<TypedMechanismObservationIR>,
    pub require_conditional: bool,
    pub max_expression_depth: usize,
    pub max_candidates: usize,
    pub provenance: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TypedMechanismSynthesisReceiptIR {
    pub schema: String,
    pub goal_id: String,
    pub candidates_enumerated: usize,
    pub candidates_falsified: usize,
    pub counterexample_guided_selection: bool,
    pub conditional_synthesized: bool,
    pub winning_expression_nodes: usize,
    #[serde(default)]
    pub preferred_operator_attempts: usize,
    #[serde(default)]
    pub preferred_operator_selected: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selected_operator_id: Option<String>,
    #[serde(default)]
    pub attempted_operator_ids: Vec<String>,
    #[serde(default)]
    pub rejected_operator_ids: Vec<String>,
    #[serde(default)]
    pub parallel_operator_evaluation: bool,
    pub winning_goal: TypedMechanismGoalIR,
    pub template: ConcreteSyntaxTemplateIR,
    pub receipt_sha256: String,
}

/// A name-independent, content-addressed expression recipe retained from a
/// previously falsified and externally verified typed repair. Operand roles
/// are canonical ARG_0..ARG_N positions so the recipe can be transported to a
/// fresh repository without retaining source identifiers or patch text.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TypedMechanismImprovementOperatorIR {
    pub schema: String,
    pub operator_id: String,
    pub operand_types: Vec<ProgramType>,
    pub output_type: ProgramType,
    pub condition: Option<TypedSyntaxExpressionIR>,
    pub postimage: TypedSyntaxExpressionIR,
    pub otherwise: Option<TypedSyntaxExpressionIR>,
    pub validation_contract: Vec<String>,
    pub evidence_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConcreteSyntaxTemplateIR {
    pub schema: String,
    pub goal_id: String,
    pub source_operands: Vec<SourceOperandIR>,
    pub condition_source: Option<String>,
    pub postimage_source: String,
    pub otherwise_source: Option<String>,
    pub complete_expression_source: String,
    pub canonical_compilable_source: String,
    pub program_task: ProgramTask,
    pub expression_nodes: usize,
    pub recombinations: usize,
    pub public_observations_checked: usize,
    pub public_observations_passed: usize,
    pub syntax_parse_pass: bool,
    pub type_effect_check_pass: bool,
    pub syntax_sha256: String,
}

/// Compile an abstract typed mechanism into both transplantable repository
/// syntax and an executable SEM-5 task.  No goal/task name participates in
/// expression selection; only operand roles, types, operators, and API
/// signatures are consulted.
pub fn lower_typed_mechanism_goal(
    goal: &TypedMechanismGoalIR,
) -> Result<ConcreteSyntaxTemplateIR, String> {
    validate_goal_envelope(goal)?;

    let operand_types = goal
        .operands
        .iter()
        .map(|operand| (operand.role.clone(), operand.value_type.clone()))
        .collect::<BTreeMap<_, _>>();
    let operand_sources = goal
        .operands
        .iter()
        .map(|operand| (operand.role.clone(), operand.source.clone()))
        .collect::<BTreeMap<_, _>>();
    let operand_indices = goal
        .operands
        .iter()
        .enumerate()
        .map(|(index, operand)| (operand.role.clone(), index))
        .collect::<BTreeMap<_, _>>();
    let definitions = goal
        .definitions
        .iter()
        .map(|definition| (definition.api_token.clone(), definition))
        .collect::<BTreeMap<_, _>>();

    let mut effects = BTreeSet::new();
    let condition_type = goal
        .condition
        .as_ref()
        .map(|condition| {
            infer_expression_type(condition, &operand_types, &definitions, &mut effects)
        })
        .transpose()?;
    if condition_type
        .as_ref()
        .is_some_and(|kind| *kind != ProgramType::Bool)
    {
        return Err("TYPED_MECHANISM_CONDITION_NOT_BOOL".to_string());
    }
    let postimage_type =
        infer_expression_type(&goal.postimage, &operand_types, &definitions, &mut effects)?;
    if postimage_type != goal.output_type {
        return Err(format!(
            "TYPED_MECHANISM_POSTIMAGE_TYPE:{postimage_type:?}:{:?}",
            goal.output_type
        ));
    }
    let otherwise_type = goal
        .otherwise
        .as_ref()
        .map(|otherwise| {
            infer_expression_type(otherwise, &operand_types, &definitions, &mut effects)
        })
        .transpose()?;
    match (&goal.condition, &goal.otherwise, otherwise_type) {
        (Some(_), Some(_), Some(kind)) if kind == goal.output_type => {}
        (Some(_), Some(_), Some(kind)) => {
            return Err(format!(
                "TYPED_MECHANISM_OTHERWISE_TYPE:{kind:?}:{:?}",
                goal.output_type
            ));
        }
        (Some(_), None, _) => return Err("TYPED_MECHANISM_OTHERWISE_MISSING".to_string()),
        (None, Some(_), _) => {
            return Err("TYPED_MECHANISM_OTHERWISE_WITHOUT_CONDITION".to_string());
        }
        (None, None, _) => {}
        _ => return Err("TYPED_MECHANISM_OTHERWISE_INVALID".to_string()),
    }
    if !effects
        .iter()
        .all(|effect| goal.allowed_effects.contains(effect))
    {
        return Err("TYPED_MECHANISM_EFFECT_NOT_ALLOWED".to_string());
    }

    let condition_source = goal
        .condition
        .as_ref()
        .map(|condition| emit_expression(condition, &operand_sources, &operand_types, &definitions))
        .transpose()?;
    let postimage_source = emit_expression(
        &goal.postimage,
        &operand_sources,
        &operand_types,
        &definitions,
    )?;
    let otherwise_source = goal
        .otherwise
        .as_ref()
        .map(|otherwise| emit_expression(otherwise, &operand_sources, &operand_types, &definitions))
        .transpose()?;
    let complete_expression_source = complete_expression(
        condition_source.as_deref(),
        &postimage_source,
        otherwise_source.as_deref(),
    )?;
    syn::parse_str::<syn::Expr>(&complete_expression_source)
        .map_err(|error| format!("TYPED_MECHANISM_SOURCE_PARSE:{error}"))?;

    let canonical_sources = goal
        .operands
        .iter()
        .map(|operand| (operand.role.clone(), operand.role.clone()))
        .collect::<BTreeMap<_, _>>();
    let canonical_condition = goal
        .condition
        .as_ref()
        .map(|condition| {
            emit_expression(condition, &canonical_sources, &operand_types, &definitions)
        })
        .transpose()?;
    let canonical_postimage = emit_expression(
        &goal.postimage,
        &canonical_sources,
        &operand_types,
        &definitions,
    )?;
    let canonical_otherwise = goal
        .otherwise
        .as_ref()
        .map(|otherwise| {
            emit_expression(otherwise, &canonical_sources, &operand_types, &definitions)
        })
        .transpose()?;
    let canonical_expression = complete_expression(
        canonical_condition.as_deref(),
        &canonical_postimage,
        canonical_otherwise.as_deref(),
    )?;
    let parameters = goal
        .operands
        .iter()
        .map(|operand| format!("{}: {}", operand.role, rust_type(&operand.value_type)))
        .collect::<Vec<_>>()
        .join(", ");
    let canonical_compilable_source = format!(
        "fn __b_core_typed_mechanism({parameters}) -> {} {{ {canonical_expression} }}",
        rust_type(&goal.output_type)
    );
    syn::parse_file(&canonical_compilable_source)
        .map_err(|error| format!("TYPED_MECHANISM_TEMPLATE_PARSE:{error}"))?;

    let condition = goal
        .condition
        .as_ref()
        .map(|condition| lower_expression(condition, &operand_indices))
        .transpose()?;
    let postimage = lower_expression(&goal.postimage, &operand_indices)?;
    let otherwise = goal
        .otherwise
        .as_ref()
        .map(|otherwise| lower_expression(otherwise, &operand_indices))
        .transpose()?;
    let inputs = goal
        .operands
        .iter()
        .map(|operand| BindingSpec {
            name: operand.role.clone(),
            value_type: operand.value_type.clone(),
            mutable: false,
        })
        .collect::<Vec<_>>();
    let program_task = ProgramTask {
        task_id: goal.goal_id.clone(),
        split: goal.split,
        inputs,
        output_type: goal.output_type.clone(),
        relation: RelationSpec::Mechanism {
            condition,
            postimage,
            otherwise,
        },
        definitions: goal.definitions.clone(),
        allowed_effects: goal.allowed_effects.clone(),
        preconditions: goal.preconditions.clone(),
        postconditions: goal.postconditions.clone(),
        invariants: goal.invariants.clone(),
        demonstrations: goal
            .public_observations
            .iter()
            .map(|observation| {
                goal.operands
                    .iter()
                    .filter_map(|operand| observation.operands.get(&operand.role).cloned())
                    .collect()
            })
            .collect(),
        provenance: goal
            .provenance
            .iter()
            .cloned()
            .chain(["TYPED_MECHANISM_TO_CONCRETE_SYNTAX".to_string()])
            .collect(),
    };

    for (index, observation) in goal.public_observations.iter().enumerate() {
        validate_observation_bindings(goal, observation)?;
        let actual = evaluate_contract(&program_task, &observation.operands)
            .map_err(|error| format!("TYPED_MECHANISM_OBSERVATION_EXECUTE:{index}:{error}"))?;
        if actual != observation.expected_postimage {
            return Err(format!("TYPED_MECHANISM_COUNTEREXAMPLE:{index}"));
        }
    }

    let expression_nodes = goal
        .condition
        .iter()
        .map(expression_nodes)
        .sum::<usize>()
        .saturating_add(expression_nodes(&goal.postimage))
        .saturating_add(goal.otherwise.iter().map(expression_nodes).sum::<usize>());
    let recombinations = expression_nodes.saturating_sub(1);
    let syntax_sha256 = sha256(
        serde_json::to_vec(&(
            CONCRETE_SYNTAX_TEMPLATE_SCHEMA,
            &goal.goal_id,
            &goal.operands,
            &complete_expression_source,
            &program_task,
        ))
        .map_err(|error| format!("TYPED_MECHANISM_TEMPLATE_SERIALIZE:{error}"))?
        .as_slice(),
    );

    Ok(ConcreteSyntaxTemplateIR {
        schema: CONCRETE_SYNTAX_TEMPLATE_SCHEMA.to_string(),
        goal_id: goal.goal_id.clone(),
        source_operands: goal.operands.clone(),
        condition_source,
        postimage_source,
        otherwise_source,
        complete_expression_source,
        canonical_compilable_source,
        program_task,
        expression_nodes,
        recombinations,
        public_observations_checked: goal.public_observations.len(),
        public_observations_passed: goal.public_observations.len(),
        syntax_parse_pass: true,
        type_effect_check_pass: true,
        syntax_sha256,
    })
}

#[derive(Debug, Clone)]
struct EnumeratedExpression {
    expression: TypedSyntaxExpressionIR,
    value_type: ProgramType,
    outputs: Vec<Value>,
    nodes: usize,
    canonical_key: String,
}

#[derive(Debug, Clone)]
struct TransportedImprovementOperator {
    operator_id: String,
    condition: Option<TypedSyntaxExpressionIR>,
    postimage: TypedSyntaxExpressionIR,
    otherwise: Option<TypedSyntaxExpressionIR>,
}

fn remap_expression_roles(
    expression: &TypedSyntaxExpressionIR,
    role_map: &BTreeMap<String, String>,
) -> Result<TypedSyntaxExpressionIR, String> {
    Ok(match expression {
        TypedSyntaxExpressionIR::Operand { role } => TypedSyntaxExpressionIR::Operand {
            role: role_map
                .get(role)
                .cloned()
                .ok_or_else(|| format!("TYPED_MECHANISM_PRIOR_ROLE_MISSING:{role}"))?,
        },
        TypedSyntaxExpressionIR::IntLiteral { value } => {
            TypedSyntaxExpressionIR::IntLiteral { value: *value }
        }
        TypedSyntaxExpressionIR::BoolLiteral { value } => {
            TypedSyntaxExpressionIR::BoolLiteral { value: *value }
        }
        TypedSyntaxExpressionIR::Unary { operator, input } => TypedSyntaxExpressionIR::Unary {
            operator: *operator,
            input: Box::new(remap_expression_roles(input, role_map)?),
        },
        TypedSyntaxExpressionIR::Binary {
            operator,
            left,
            right,
        } => TypedSyntaxExpressionIR::Binary {
            operator: *operator,
            left: Box::new(remap_expression_roles(left, role_map)?),
            right: Box::new(remap_expression_roles(right, role_map)?),
        },
        TypedSyntaxExpressionIR::Length { input } => TypedSyntaxExpressionIR::Length {
            input: Box::new(remap_expression_roles(input, role_map)?),
        },
        TypedSyntaxExpressionIR::Index { collection, index } => TypedSyntaxExpressionIR::Index {
            collection: Box::new(remap_expression_roles(collection, role_map)?),
            index: Box::new(remap_expression_roles(index, role_map)?),
        },
        TypedSyntaxExpressionIR::Call {
            api_token,
            arguments,
        } => TypedSyntaxExpressionIR::Call {
            api_token: api_token.clone(),
            arguments: arguments
                .iter()
                .map(|argument| remap_expression_roles(argument, role_map))
                .collect::<Result<Vec<_>, _>>()?,
        },
    })
}

pub fn validate_typed_mechanism_improvement_operator(
    prior: &TypedMechanismImprovementOperatorIR,
) -> Result<(), String> {
    if prior.schema != "B_CORE_TYPED_MECHANISM_IMPROVEMENT_OPERATOR_1"
        || prior.operator_id.is_empty()
        || prior.operand_types.is_empty()
        || prior.evidence_sha256.len() != 64
        || prior.validation_contract
            != [
                "PUBLIC_OBSERVATION_REPLAY",
                "TYPE_EFFECT_CHECK",
                "SOURCE_BOUND_ATOMIC_MATERIALIZATION",
                "SANDBOX_PUBLIC_REGRESSION",
                "AUTHORITATIVE_SCOPE_STABLE",
            ]
    {
        return Err("TYPED_MECHANISM_IMPROVEMENT_OPERATOR_ENVELOPE".to_string());
    }
    let mut identity = prior.clone();
    identity.operator_id.clear();
    identity.evidence_sha256.clear();
    let encoded = serde_json::to_vec(&identity)
        .map_err(|error| format!("TYPED_MECHANISM_PRIOR_SERIALIZE:{error}"))?;
    if sha256(&encoded) != prior.operator_id {
        return Err("TYPED_MECHANISM_IMPROVEMENT_OPERATOR_ID_MISMATCH".to_string());
    }
    Ok(())
}

pub fn typed_mechanism_improvement_operator_from_receipt(
    receipt: &TypedMechanismSynthesisReceiptIR,
    evidence_sha256: String,
) -> Result<TypedMechanismImprovementOperatorIR, String> {
    if evidence_sha256.len() != 64 || receipt.winning_goal.operands.is_empty() {
        return Err("TYPED_MECHANISM_PRIOR_EVIDENCE".to_string());
    }
    let role_map = receipt
        .winning_goal
        .operands
        .iter()
        .enumerate()
        .map(|(index, operand)| (operand.role.clone(), format!("ARG_{index}")))
        .collect::<BTreeMap<_, _>>();
    if role_map.len() != receipt.winning_goal.operands.len() {
        return Err("TYPED_MECHANISM_PRIOR_DUPLICATE_ROLE".to_string());
    }
    let mut prior = TypedMechanismImprovementOperatorIR {
        schema: "B_CORE_TYPED_MECHANISM_IMPROVEMENT_OPERATOR_1".to_string(),
        operator_id: String::new(),
        operand_types: receipt
            .winning_goal
            .operands
            .iter()
            .map(|operand| operand.value_type.clone())
            .collect(),
        output_type: receipt.winning_goal.output_type.clone(),
        condition: receipt
            .winning_goal
            .condition
            .as_ref()
            .map(|condition| remap_expression_roles(condition, &role_map))
            .transpose()?,
        postimage: remap_expression_roles(&receipt.winning_goal.postimage, &role_map)?,
        otherwise: receipt
            .winning_goal
            .otherwise
            .as_ref()
            .map(|otherwise| remap_expression_roles(otherwise, &role_map))
            .transpose()?,
        validation_contract: vec![
            "PUBLIC_OBSERVATION_REPLAY".to_string(),
            "TYPE_EFFECT_CHECK".to_string(),
            "SOURCE_BOUND_ATOMIC_MATERIALIZATION".to_string(),
            "SANDBOX_PUBLIC_REGRESSION".to_string(),
            "AUTHORITATIVE_SCOPE_STABLE".to_string(),
        ],
        evidence_sha256,
    };
    let mut identity = prior.clone();
    identity.evidence_sha256.clear();
    let encoded = serde_json::to_vec(&identity)
        .map_err(|error| format!("TYPED_MECHANISM_PRIOR_SERIALIZE:{error}"))?;
    prior.operator_id = sha256(&encoded);
    validate_typed_mechanism_improvement_operator(&prior)?;
    Ok(prior)
}

fn transport_prior_to_request(
    prior: &TypedMechanismImprovementOperatorIR,
    request: &TypedMechanismSynthesisGoalIR,
) -> Result<
    (
        Option<TypedSyntaxExpressionIR>,
        TypedSyntaxExpressionIR,
        Option<TypedSyntaxExpressionIR>,
    ),
    String,
> {
    validate_typed_mechanism_improvement_operator(prior)?;
    let request_types = request
        .operands
        .iter()
        .map(|operand| operand.value_type.clone())
        .collect::<Vec<_>>();
    if prior.operand_types != request_types || prior.output_type != request.output_type {
        return Err("TYPED_MECHANISM_PRIOR_TYPE_SHAPE_MISMATCH".to_string());
    }
    let role_map = request
        .operands
        .iter()
        .enumerate()
        .map(|(index, operand)| (format!("ARG_{index}"), operand.role.clone()))
        .collect::<BTreeMap<_, _>>();
    Ok((
        prior
            .condition
            .as_ref()
            .map(|condition| remap_expression_roles(condition, &role_map))
            .transpose()?,
        remap_expression_roles(&prior.postimage, &role_map)?,
        prior
            .otherwise
            .as_ref()
            .map(|otherwise| remap_expression_roles(otherwise, &role_map))
            .transpose()?,
    ))
}

#[allow(clippy::too_many_arguments)]
fn evaluate_prior_expression(
    expression: &TypedSyntaxExpressionIR,
    operand_types: &BTreeMap<String, ProgramType>,
    operand_indices: &BTreeMap<String, usize>,
    definitions: &BTreeMap<String, &ApiDefinition>,
    api_map: &BTreeMap<String, &ApiDefinition>,
    observation_arguments: &[Vec<Value>],
    allowed_effects: &[Effect],
) -> Result<(ProgramType, Vec<Value>), String> {
    let mut effects = BTreeSet::new();
    let value_type = infer_expression_type(expression, operand_types, definitions, &mut effects)?;
    if !effects
        .iter()
        .all(|effect| allowed_effects.contains(effect))
    {
        return Err("TYPED_MECHANISM_PRIOR_EFFECT_FORBIDDEN".to_string());
    }
    let scalar = lower_expression(expression, operand_indices)?;
    let outputs = observation_arguments
        .iter()
        .map(|arguments| eval_scalar(&scalar, arguments, api_map))
        .collect::<Result<Vec<_>, _>>()?;
    Ok((value_type, outputs))
}

#[allow(clippy::too_many_arguments)]
fn prior_matches_public_observations(
    request: &TypedMechanismSynthesisGoalIR,
    condition: &Option<TypedSyntaxExpressionIR>,
    postimage: &TypedSyntaxExpressionIR,
    otherwise: &Option<TypedSyntaxExpressionIR>,
    operand_types: &BTreeMap<String, ProgramType>,
    operand_indices: &BTreeMap<String, usize>,
    definitions: &BTreeMap<String, &ApiDefinition>,
    api_map: &BTreeMap<String, &ApiDefinition>,
    observation_arguments: &[Vec<Value>],
) -> Result<bool, String> {
    let (postimage_type, postimage_outputs) = evaluate_prior_expression(
        postimage,
        operand_types,
        operand_indices,
        definitions,
        api_map,
        observation_arguments,
        &request.allowed_effects,
    )?;
    if postimage_type != request.output_type {
        return Ok(false);
    }
    let expected = request
        .public_observations
        .iter()
        .map(|observation| observation.expected_postimage.clone())
        .collect::<Vec<_>>();
    match (condition, otherwise) {
        (None, None) => Ok(!request.require_conditional && postimage_outputs == expected),
        (Some(condition), Some(otherwise)) => {
            let (condition_type, condition_outputs) = evaluate_prior_expression(
                condition,
                operand_types,
                operand_indices,
                definitions,
                api_map,
                observation_arguments,
                &request.allowed_effects,
            )?;
            let (otherwise_type, otherwise_outputs) = evaluate_prior_expression(
                otherwise,
                operand_types,
                operand_indices,
                definitions,
                api_map,
                observation_arguments,
                &request.allowed_effects,
            )?;
            if condition_type != ProgramType::Bool || otherwise_type != request.output_type {
                return Ok(false);
            }
            let outputs = condition_outputs
                .iter()
                .zip(postimage_outputs.iter().zip(&otherwise_outputs))
                .map(|(condition, (then_value, else_value))| match condition {
                    Value::Bool(true) => Ok(then_value.clone()),
                    Value::Bool(false) => Ok(else_value.clone()),
                    _ => Err("TYPED_MECHANISM_PRIOR_CONDITION_NOT_BOOL".to_string()),
                })
                .collect::<Result<Vec<_>, _>>()?;
            Ok(outputs == expected)
        }
        _ => Ok(false),
    }
}

#[allow(clippy::too_many_arguments)]
fn build_synthesis_receipt(
    request: &TypedMechanismSynthesisGoalIR,
    condition: Option<TypedSyntaxExpressionIR>,
    postimage: TypedSyntaxExpressionIR,
    otherwise: Option<TypedSyntaxExpressionIR>,
    enumerated: usize,
    candidates_falsified: usize,
    attempted_operator_ids: Vec<String>,
    rejected_operator_ids: Vec<String>,
    selected_operator_id: Option<String>,
    parallel_operator_evaluation: bool,
) -> Result<TypedMechanismSynthesisReceiptIR, String> {
    let preferred_operator_attempts = attempted_operator_ids.len();
    let conditional_synthesized = condition.is_some();
    let winning_goal = TypedMechanismGoalIR {
        schema: TYPED_MECHANISM_GOAL_SCHEMA.to_string(),
        goal_id: request.goal_id.clone(),
        split: request.split,
        operands: request.operands.clone(),
        output_type: request.output_type.clone(),
        condition,
        postimage,
        otherwise,
        definitions: request.definitions.clone(),
        allowed_effects: request.allowed_effects.clone(),
        preconditions: request.preconditions.clone(),
        postconditions: request.postconditions.clone(),
        invariants: request.invariants.clone(),
        public_observations: request.public_observations.clone(),
        provenance: request
            .provenance
            .iter()
            .cloned()
            .chain([if selected_operator_id.is_some() {
                "CONTENT_ADDRESSED_OPERATOR_REUSE".to_string()
            } else {
                "BOUNDED_TYPED_GRAMMAR_SYNTHESIS".to_string()
            }])
            .collect(),
    };
    let template = lower_typed_mechanism_goal(&winning_goal)?;
    let receipt_sha256 = sha256(
        serde_json::to_vec(&(
            TYPED_MECHANISM_SYNTHESIS_GOAL_SCHEMA,
            request,
            &winning_goal,
            &template.syntax_sha256,
            enumerated,
            candidates_falsified,
            preferred_operator_attempts,
            &selected_operator_id,
            &attempted_operator_ids,
            &rejected_operator_ids,
            parallel_operator_evaluation,
        ))
        .map_err(|error| format!("TYPED_MECHANISM_RECEIPT_SERIALIZE:{error}"))?
        .as_slice(),
    );
    Ok(TypedMechanismSynthesisReceiptIR {
        schema: "B_CORE_TYPED_MECHANISM_SYNTHESIS_RECEIPT_1".to_string(),
        goal_id: request.goal_id.clone(),
        candidates_enumerated: enumerated,
        candidates_falsified,
        counterexample_guided_selection: true,
        conditional_synthesized,
        winning_expression_nodes: template.expression_nodes,
        preferred_operator_attempts,
        preferred_operator_selected: selected_operator_id.is_some(),
        selected_operator_id,
        attempted_operator_ids,
        rejected_operator_ids,
        parallel_operator_evaluation,
        winning_goal,
        template,
        receipt_sha256,
    })
}

/// Enumerate a bounded, typed expression grammar and use public
/// counterexamples to select either one postimage or a guarded pair of
/// postimages.  Search order is deterministic and independent of goal names.
pub fn synthesize_typed_mechanism_goal(
    request: &TypedMechanismSynthesisGoalIR,
) -> Result<TypedMechanismSynthesisReceiptIR, String> {
    synthesize_typed_mechanism_goal_with_priors(request, &[])
}

pub fn synthesize_typed_mechanism_goal_with_priors(
    request: &TypedMechanismSynthesisGoalIR,
    priors: &[TypedMechanismImprovementOperatorIR],
) -> Result<TypedMechanismSynthesisReceiptIR, String> {
    validate_synthesis_envelope(request)?;
    let max_depth = request.max_expression_depth.clamp(1, MAX_SYNTHESIS_DEPTH);
    let max_candidates = request.max_candidates.clamp(16, MAX_SYNTHESIS_CANDIDATES);
    let operand_types = request
        .operands
        .iter()
        .map(|operand| (operand.role.clone(), operand.value_type.clone()))
        .collect::<BTreeMap<_, _>>();
    let operand_indices = request
        .operands
        .iter()
        .enumerate()
        .map(|(index, operand)| (operand.role.clone(), index))
        .collect::<BTreeMap<_, _>>();
    let definitions = request
        .definitions
        .iter()
        .map(|definition| (definition.api_token.clone(), definition))
        .collect::<BTreeMap<_, _>>();
    let api_map = request
        .definitions
        .iter()
        .map(|definition| (definition.api_token.clone(), definition))
        .collect::<BTreeMap<_, _>>();
    let observation_arguments = request
        .public_observations
        .iter()
        .map(|observation| {
            request
                .operands
                .iter()
                .map(|operand| {
                    observation
                        .operands
                        .get(&operand.role)
                        .cloned()
                        .ok_or_else(|| {
                            format!("TYPED_MECHANISM_OBSERVATION_MISSING:{}", operand.role)
                        })
                })
                .collect::<Result<Vec<_>, _>>()
        })
        .collect::<Result<Vec<_>, _>>()?;

    let mut applicable_operators = Vec::new();
    let mut seen_operator_ids = BTreeSet::new();
    for prior in priors
        .iter()
        .filter(|prior| seen_operator_ids.insert(prior.operator_id.clone()))
    {
        validate_typed_mechanism_improvement_operator(prior)?;
        let transported = match transport_prior_to_request(prior, request) {
            Ok(transported) => transported,
            Err(error) if error == "TYPED_MECHANISM_PRIOR_TYPE_SHAPE_MISMATCH" => continue,
            Err(error) => return Err(error),
        };
        applicable_operators.push(TransportedImprovementOperator {
            operator_id: prior.operator_id.clone(),
            condition: transported.0,
            postimage: transported.1,
            otherwise: transported.2,
        });
    }
    let operator_worker_count = worker_count_for(
        applicable_operators.len(),
        TYPED_OPERATOR_REPLAY_ITEMS_PER_WORKER,
    );
    let operator_matches = parallel_map_ordered_batched(
        &applicable_operators,
        "TYPED_OPERATOR_PUBLIC_REPLAY",
        TYPED_OPERATOR_REPLAY_ITEMS_PER_WORKER,
        |operator| {
            Ok(prior_matches_public_observations(
                request,
                &operator.condition,
                &operator.postimage,
                &operator.otherwise,
                &operand_types,
                &operand_indices,
                &definitions,
                &api_map,
                &observation_arguments,
            )
            .unwrap_or(false))
        },
    )?;
    let attempted_operator_ids = applicable_operators
        .iter()
        .map(|operator| operator.operator_id.clone())
        .collect::<Vec<_>>();
    if let Some(winner_index) = operator_matches.iter().position(|matches| *matches) {
        let winner = &applicable_operators[winner_index];
        let rejected_operator_ids = applicable_operators
            .iter()
            .zip(&operator_matches)
            .filter(|(_, matches)| !**matches)
            .map(|(operator, _)| operator.operator_id.clone())
            .collect::<Vec<_>>();
        let enumerated = applicable_operators
            .iter()
            .map(|operator| {
                1 + usize::from(operator.condition.is_some())
                    + usize::from(operator.otherwise.is_some())
            })
            .sum();
        return build_synthesis_receipt(
            request,
            winner.condition.clone(),
            winner.postimage.clone(),
            winner.otherwise.clone(),
            enumerated,
            rejected_operator_ids.len(),
            attempted_operator_ids,
            rejected_operator_ids,
            Some(winner.operator_id.clone()),
            operator_worker_count > 1,
        );
    }
    let rejected_operator_ids = attempted_operator_ids.clone();

    let mut expressions = Vec::<EnumeratedExpression>::new();
    let mut seen = BTreeSet::new();
    let mut enumerated = 0_usize;
    let mut evaluation_failures = 0_usize;
    for operand in &request.operands {
        add_enumerated_expression(
            TypedSyntaxExpressionIR::Operand {
                role: operand.role.clone(),
            },
            &operand_types,
            &operand_indices,
            &definitions,
            &api_map,
            &observation_arguments,
            &request.allowed_effects,
            max_candidates,
            &mut enumerated,
            &mut evaluation_failures,
            &mut seen,
            &mut expressions,
        )?;
    }
    // Keep the literal basis universal. Mining expected outputs as constants
    // can fit a finite observation table while ignoring the source operands,
    // which is precisely the template-selection failure this compiler is
    // intended to remove.
    let int_constants = BTreeSet::from([-1_i64, 0_i64, 1_i64]);
    let bool_constants = BTreeSet::from([false, true]);
    for value in int_constants {
        add_enumerated_expression(
            TypedSyntaxExpressionIR::IntLiteral { value },
            &operand_types,
            &operand_indices,
            &definitions,
            &api_map,
            &observation_arguments,
            &request.allowed_effects,
            max_candidates,
            &mut enumerated,
            &mut evaluation_failures,
            &mut seen,
            &mut expressions,
        )?;
    }
    for value in bool_constants {
        add_enumerated_expression(
            TypedSyntaxExpressionIR::BoolLiteral { value },
            &operand_types,
            &operand_indices,
            &definitions,
            &api_map,
            &observation_arguments,
            &request.allowed_effects,
            max_candidates,
            &mut enumerated,
            &mut evaluation_failures,
            &mut seen,
            &mut expressions,
        )?;
    }

    for _depth in 1..=max_depth {
        if expressions.len() >= max_candidates {
            break;
        }
        let prior = expressions.clone();
        for candidate in &prior {
            for operator in [UnaryOperator::Negate, UnaryOperator::Not] {
                if expressions.len() >= max_candidates {
                    break;
                }
                add_enumerated_expression(
                    TypedSyntaxExpressionIR::Unary {
                        operator,
                        input: Box::new(candidate.expression.clone()),
                    },
                    &operand_types,
                    &operand_indices,
                    &definitions,
                    &api_map,
                    &observation_arguments,
                    &request.allowed_effects,
                    max_candidates,
                    &mut enumerated,
                    &mut evaluation_failures,
                    &mut seen,
                    &mut expressions,
                )?;
            }
        }
        for collection in prior.iter().filter(|candidate| {
            matches!(
                candidate.value_type,
                ProgramType::SequenceInt | ProgramType::NestedSequenceInt | ProgramType::Bytes
            )
        }) {
            if expressions.len() >= max_candidates {
                break;
            }
            add_enumerated_expression(
                TypedSyntaxExpressionIR::Length {
                    input: Box::new(collection.expression.clone()),
                },
                &operand_types,
                &operand_indices,
                &definitions,
                &api_map,
                &observation_arguments,
                &request.allowed_effects,
                max_candidates,
                &mut enumerated,
                &mut evaluation_failures,
                &mut seen,
                &mut expressions,
            )?;
            for index in prior
                .iter()
                .filter(|candidate| candidate.value_type == ProgramType::Int)
            {
                if expressions.len() >= max_candidates {
                    break;
                }
                add_enumerated_expression(
                    TypedSyntaxExpressionIR::Index {
                        collection: Box::new(collection.expression.clone()),
                        index: Box::new(index.expression.clone()),
                    },
                    &operand_types,
                    &operand_indices,
                    &definitions,
                    &api_map,
                    &observation_arguments,
                    &request.allowed_effects,
                    max_candidates,
                    &mut enumerated,
                    &mut evaluation_failures,
                    &mut seen,
                    &mut expressions,
                )?;
            }
        }
        let operators = [
            BinaryOperator::Equal,
            BinaryOperator::LessThan,
            BinaryOperator::GreaterThan,
            BinaryOperator::Add,
            BinaryOperator::Subtract,
            BinaryOperator::Multiply,
            BinaryOperator::Divide,
            BinaryOperator::Modulo,
            BinaryOperator::And,
            BinaryOperator::Or,
        ];
        'binary: for operator in operators {
            for left in &prior {
                for right in &prior {
                    if expressions.len() >= max_candidates {
                        break 'binary;
                    }
                    add_enumerated_expression(
                        TypedSyntaxExpressionIR::Binary {
                            operator,
                            left: Box::new(left.expression.clone()),
                            right: Box::new(right.expression.clone()),
                        },
                        &operand_types,
                        &operand_indices,
                        &definitions,
                        &api_map,
                        &observation_arguments,
                        &request.allowed_effects,
                        max_candidates,
                        &mut enumerated,
                        &mut evaluation_failures,
                        &mut seen,
                        &mut expressions,
                    )?;
                }
            }
        }
        for definition in &request.definitions {
            if expressions.len() >= max_candidates {
                break;
            }
            let argument_sets = enumerate_api_arguments(definition, &prior, 64);
            for arguments in argument_sets {
                if expressions.len() >= max_candidates {
                    break;
                }
                add_enumerated_expression(
                    TypedSyntaxExpressionIR::Call {
                        api_token: definition.api_token.clone(),
                        arguments,
                    },
                    &operand_types,
                    &operand_indices,
                    &definitions,
                    &api_map,
                    &observation_arguments,
                    &request.allowed_effects,
                    max_candidates,
                    &mut enumerated,
                    &mut evaluation_failures,
                    &mut seen,
                    &mut expressions,
                )?;
            }
        }
    }

    let expected = request
        .public_observations
        .iter()
        .map(|observation| observation.expected_postimage.clone())
        .collect::<Vec<_>>();
    let mut output_candidates = expressions
        .iter()
        .filter(|candidate| candidate.value_type == request.output_type)
        .cloned()
        .collect::<Vec<_>>();
    output_candidates.sort_by(|left, right| {
        (left.nodes, &left.canonical_key).cmp(&(right.nodes, &right.canonical_key))
    });
    let exact = output_candidates
        .iter()
        .find(|candidate| candidate.outputs == expected)
        .cloned();
    let (condition, postimage, otherwise) = if !request.require_conditional {
        if let Some(exact) = exact {
            (None, exact.expression, None)
        } else {
            synthesize_conditional(&expressions, &output_candidates, &expected)?
        }
    } else {
        synthesize_conditional(&expressions, &output_candidates, &expected)?
    };
    let candidates_falsified = output_candidates
        .iter()
        .filter(|candidate| candidate.outputs != expected)
        .count()
        .saturating_add(evaluation_failures)
        .saturating_add(rejected_operator_ids.len());
    build_synthesis_receipt(
        request,
        condition,
        postimage,
        otherwise,
        enumerated,
        candidates_falsified,
        attempted_operator_ids,
        rejected_operator_ids,
        None,
        operator_worker_count > 1,
    )
}

#[allow(clippy::too_many_arguments)]
fn add_enumerated_expression(
    expression: TypedSyntaxExpressionIR,
    operand_types: &BTreeMap<String, ProgramType>,
    operand_indices: &BTreeMap<String, usize>,
    definitions: &BTreeMap<String, &ApiDefinition>,
    api_map: &BTreeMap<String, &ApiDefinition>,
    observation_arguments: &[Vec<Value>],
    allowed_effects: &[Effect],
    max_candidates: usize,
    enumerated: &mut usize,
    evaluation_failures: &mut usize,
    seen: &mut BTreeSet<String>,
    expressions: &mut Vec<EnumeratedExpression>,
) -> Result<(), String> {
    if expressions.len() >= max_candidates {
        return Ok(());
    }
    let canonical_key = serde_json::to_string(&expression)
        .map_err(|error| format!("TYPED_MECHANISM_EXPRESSION_SERIALIZE:{error}"))?;
    if !seen.insert(canonical_key.clone()) {
        return Ok(());
    }
    *enumerated = enumerated.saturating_add(1);
    let mut effects = BTreeSet::new();
    let Ok(value_type) =
        infer_expression_type(&expression, operand_types, definitions, &mut effects)
    else {
        *evaluation_failures = evaluation_failures.saturating_add(1);
        return Ok(());
    };
    if !effects
        .iter()
        .all(|effect| allowed_effects.contains(effect))
    {
        *evaluation_failures = evaluation_failures.saturating_add(1);
        return Ok(());
    }
    let scalar = lower_expression(&expression, operand_indices)?;
    let mut outputs = Vec::with_capacity(observation_arguments.len());
    for arguments in observation_arguments {
        match eval_scalar(&scalar, arguments, api_map) {
            Ok(output) => outputs.push(output),
            Err(_) => {
                *evaluation_failures = evaluation_failures.saturating_add(1);
                return Ok(());
            }
        }
    }
    expressions.push(EnumeratedExpression {
        nodes: expression_nodes(&expression),
        expression,
        value_type,
        outputs,
        canonical_key,
    });
    Ok(())
}

fn enumerate_api_arguments(
    definition: &ApiDefinition,
    expressions: &[EnumeratedExpression],
    limit: usize,
) -> Vec<Vec<TypedSyntaxExpressionIR>> {
    let pools = definition
        .inputs
        .iter()
        .map(|expected| {
            expressions
                .iter()
                .filter(|candidate| &candidate.value_type == expected)
                .take(16)
                .map(|candidate| candidate.expression.clone())
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    if pools.iter().any(Vec::is_empty) {
        return Vec::new();
    }
    let mut results = vec![Vec::new()];
    for pool in pools {
        let mut next = Vec::new();
        for prefix in &results {
            for expression in &pool {
                if next.len() >= limit {
                    break;
                }
                let mut arguments = prefix.clone();
                arguments.push(expression.clone());
                next.push(arguments);
            }
            if next.len() >= limit {
                break;
            }
        }
        results = next;
    }
    results
}

fn synthesize_conditional(
    expressions: &[EnumeratedExpression],
    output_candidates: &[EnumeratedExpression],
    expected: &[Value],
) -> Result<
    (
        Option<TypedSyntaxExpressionIR>,
        TypedSyntaxExpressionIR,
        Option<TypedSyntaxExpressionIR>,
    ),
    String,
> {
    let mut best: Option<(
        usize,
        String,
        TypedSyntaxExpressionIR,
        TypedSyntaxExpressionIR,
        TypedSyntaxExpressionIR,
    )> = None;
    for condition in expressions
        .iter()
        .filter(|candidate| candidate.value_type == ProgramType::Bool)
    {
        let mask = condition
            .outputs
            .iter()
            .map(|value| match value {
                Value::Bool(value) => Ok(*value),
                _ => Err("TYPED_MECHANISM_CONDITION_EVALUATION_TYPE".to_string()),
            })
            .collect::<Result<Vec<_>, _>>()?;
        if mask.iter().all(|value| *value) || mask.iter().all(|value| !*value) {
            continue;
        }
        let true_target = masked_signature(expected, &mask, true)?;
        let false_target = masked_signature(expected, &mask, false)?;
        let true_branch = output_candidates.iter().find(|candidate| {
            masked_signature(&candidate.outputs, &mask, true)
                .is_ok_and(|signature| signature == true_target)
        });
        let false_branch = output_candidates.iter().find(|candidate| {
            masked_signature(&candidate.outputs, &mask, false)
                .is_ok_and(|signature| signature == false_target)
        });
        let (Some(true_branch), Some(false_branch)) = (true_branch, false_branch) else {
            continue;
        };
        let nodes = condition
            .nodes
            .saturating_add(true_branch.nodes)
            .saturating_add(false_branch.nodes);
        let key = format!(
            "{}|{}|{}",
            condition.canonical_key, true_branch.canonical_key, false_branch.canonical_key
        );
        if best
            .as_ref()
            .is_none_or(|(best_nodes, best_key, ..)| (nodes, &key) < (*best_nodes, best_key))
        {
            best = Some((
                nodes,
                key,
                condition.expression.clone(),
                true_branch.expression.clone(),
                false_branch.expression.clone(),
            ));
        }
    }
    best.map(|(_, _, condition, postimage, otherwise)| {
        (Some(condition), postimage, Some(otherwise))
    })
    .ok_or_else(|| "TYPED_MECHANISM_SYNTHESIS_EXHAUSTED".to_string())
}

fn masked_signature(values: &[Value], mask: &[bool], selected: bool) -> Result<String, String> {
    let selected_values = values
        .iter()
        .zip(mask)
        .filter(|(_, value)| **value == selected)
        .map(|(value, _)| value)
        .collect::<Vec<_>>();
    serde_json::to_string(&selected_values)
        .map_err(|error| format!("TYPED_MECHANISM_SIGNATURE_SERIALIZE:{error}"))
}

fn validate_synthesis_envelope(request: &TypedMechanismSynthesisGoalIR) -> Result<(), String> {
    if request.schema != TYPED_MECHANISM_SYNTHESIS_GOAL_SCHEMA {
        return Err("TYPED_MECHANISM_SYNTHESIS_SCHEMA".to_string());
    }
    if request.goal_id.is_empty() {
        return Err("TYPED_MECHANISM_GOAL_ID_EMPTY".to_string());
    }
    if request.operands.is_empty() || request.operands.len() > MAX_MECHANISM_OPERANDS {
        return Err("TYPED_MECHANISM_OPERAND_BUDGET".to_string());
    }
    if request.public_observations.is_empty()
        || request.public_observations.len() > MAX_MECHANISM_OBSERVATIONS
    {
        return Err("TYPED_MECHANISM_OBSERVATION_BUDGET".to_string());
    }
    if request.max_expression_depth == 0 || request.max_candidates < 16 {
        return Err("TYPED_MECHANISM_SYNTHESIS_BUDGET".to_string());
    }
    let validation_goal = TypedMechanismGoalIR {
        schema: TYPED_MECHANISM_GOAL_SCHEMA.to_string(),
        goal_id: request.goal_id.clone(),
        split: request.split,
        operands: request.operands.clone(),
        output_type: request.output_type.clone(),
        condition: None,
        postimage: TypedSyntaxExpressionIR::Operand {
            role: request.operands[0].role.clone(),
        },
        otherwise: None,
        definitions: request.definitions.clone(),
        allowed_effects: request.allowed_effects.clone(),
        preconditions: request.preconditions.clone(),
        postconditions: request.postconditions.clone(),
        invariants: request.invariants.clone(),
        public_observations: Vec::new(),
        provenance: request.provenance.clone(),
    };
    validate_goal_envelope(&validation_goal)?;
    for observation in &request.public_observations {
        validate_observation_bindings(&validation_goal, observation)?;
    }
    Ok(())
}

fn validate_goal_envelope(goal: &TypedMechanismGoalIR) -> Result<(), String> {
    if goal.schema != TYPED_MECHANISM_GOAL_SCHEMA {
        return Err("TYPED_MECHANISM_SCHEMA".to_string());
    }
    if goal.goal_id.is_empty() {
        return Err("TYPED_MECHANISM_GOAL_ID_EMPTY".to_string());
    }
    if goal.operands.is_empty() || goal.operands.len() > MAX_MECHANISM_OPERANDS {
        return Err("TYPED_MECHANISM_OPERAND_BUDGET".to_string());
    }
    if goal.public_observations.len() > MAX_MECHANISM_OBSERVATIONS {
        return Err("TYPED_MECHANISM_OBSERVATION_BUDGET".to_string());
    }
    let node_count = goal
        .condition
        .iter()
        .map(expression_nodes)
        .sum::<usize>()
        .saturating_add(expression_nodes(&goal.postimage))
        .saturating_add(goal.otherwise.iter().map(expression_nodes).sum::<usize>());
    if node_count == 0 || node_count > MAX_MECHANISM_EXPRESSION_NODES {
        return Err("TYPED_MECHANISM_EXPRESSION_BUDGET".to_string());
    }
    let mut roles = BTreeSet::new();
    for operand in &goal.operands {
        syn::parse_str::<syn::Ident>(&operand.role)
            .map_err(|_| format!("TYPED_MECHANISM_ROLE_NOT_IDENTIFIER:{}", operand.role))?;
        syn::parse_str::<syn::Expr>(&operand.source)
            .map_err(|error| format!("TYPED_MECHANISM_OPERAND_SOURCE:{}:{error}", operand.role))?;
        if !roles.insert(operand.role.clone()) {
            return Err(format!("TYPED_MECHANISM_DUPLICATE_ROLE:{}", operand.role));
        }
    }
    let mut api_tokens = BTreeSet::new();
    for definition in &goal.definitions {
        syn::parse_str::<syn::Path>(&definition.api_token).map_err(|error| {
            format!("TYPED_MECHANISM_API_PATH:{}:{error}", definition.api_token)
        })?;
        if !api_tokens.insert(definition.api_token.clone()) {
            return Err(format!(
                "TYPED_MECHANISM_DUPLICATE_API:{}",
                definition.api_token
            ));
        }
    }
    Ok(())
}

fn infer_expression_type(
    expression: &TypedSyntaxExpressionIR,
    operands: &BTreeMap<String, ProgramType>,
    definitions: &BTreeMap<String, &ApiDefinition>,
    effects: &mut BTreeSet<Effect>,
) -> Result<ProgramType, String> {
    match expression {
        TypedSyntaxExpressionIR::Operand { role } => operands
            .get(role)
            .cloned()
            .ok_or_else(|| format!("TYPED_MECHANISM_UNKNOWN_ROLE:{role}")),
        TypedSyntaxExpressionIR::IntLiteral { .. } => Ok(ProgramType::Int),
        TypedSyntaxExpressionIR::BoolLiteral { .. } => Ok(ProgramType::Bool),
        TypedSyntaxExpressionIR::Unary { operator, input } => {
            let input_type = infer_expression_type(input, operands, definitions, effects)?;
            match (operator, input_type) {
                (UnaryOperator::Negate, ProgramType::Int) => Ok(ProgramType::Int),
                (UnaryOperator::Not, ProgramType::Bool) => Ok(ProgramType::Bool),
                _ => Err("TYPED_MECHANISM_UNARY_TYPE".to_string()),
            }
        }
        TypedSyntaxExpressionIR::Binary {
            operator,
            left,
            right,
        } => {
            let left_type = infer_expression_type(left, operands, definitions, effects)?;
            let right_type = infer_expression_type(right, operands, definitions, effects)?;
            use BinaryOperator as Op;
            match (operator, left_type, right_type) {
                (
                    Op::Add | Op::Subtract | Op::Multiply | Op::Divide | Op::Modulo,
                    ProgramType::Int,
                    ProgramType::Int,
                ) => Ok(ProgramType::Int),
                (Op::LessThan | Op::GreaterThan, ProgramType::Int, ProgramType::Int) => {
                    Ok(ProgramType::Bool)
                }
                (Op::Equal, left, right)
                    if left == right && matches!(left, ProgramType::Int | ProgramType::Bool) =>
                {
                    Ok(ProgramType::Bool)
                }
                (Op::And | Op::Or, ProgramType::Bool, ProgramType::Bool) => Ok(ProgramType::Bool),
                _ => Err("TYPED_MECHANISM_BINARY_TYPE".to_string()),
            }
        }
        TypedSyntaxExpressionIR::Length { input } => {
            let input_type = infer_expression_type(input, operands, definitions, effects)?;
            if matches!(
                input_type,
                ProgramType::SequenceInt | ProgramType::NestedSequenceInt | ProgramType::Bytes
            ) {
                Ok(ProgramType::Int)
            } else {
                Err("TYPED_MECHANISM_LENGTH_SOURCE_TYPE".to_string())
            }
        }
        TypedSyntaxExpressionIR::Index { collection, index } => {
            let collection_type =
                infer_expression_type(collection, operands, definitions, effects)?;
            let index_type = infer_expression_type(index, operands, definitions, effects)?;
            if index_type != ProgramType::Int {
                return Err("TYPED_MECHANISM_INDEX_NOT_INT".to_string());
            }
            match collection_type {
                ProgramType::SequenceInt | ProgramType::Bytes => Ok(ProgramType::Int),
                ProgramType::NestedSequenceInt => Ok(ProgramType::SequenceInt),
                _ => Err("TYPED_MECHANISM_INDEX_SOURCE_TYPE".to_string()),
            }
        }
        TypedSyntaxExpressionIR::Call {
            api_token,
            arguments,
        } => {
            let definition = definitions
                .get(api_token)
                .ok_or_else(|| format!("TYPED_MECHANISM_UNKNOWN_API:{api_token}"))?;
            if arguments.len() != definition.inputs.len() {
                return Err(format!("TYPED_MECHANISM_API_ARITY:{api_token}"));
            }
            for (argument, expected) in arguments.iter().zip(&definition.inputs) {
                let actual = infer_expression_type(argument, operands, definitions, effects)?;
                if &actual != expected {
                    return Err(format!("TYPED_MECHANISM_API_INPUT_TYPE:{api_token}"));
                }
            }
            effects.insert(definition.effect.clone());
            Ok(definition.output.clone())
        }
    }
}

fn emit_expression(
    expression: &TypedSyntaxExpressionIR,
    sources: &BTreeMap<String, String>,
    operands: &BTreeMap<String, ProgramType>,
    definitions: &BTreeMap<String, &ApiDefinition>,
) -> Result<String, String> {
    match expression {
        TypedSyntaxExpressionIR::Operand { role } => sources
            .get(role)
            .map(|source| format!("({source})"))
            .ok_or_else(|| format!("TYPED_MECHANISM_UNKNOWN_ROLE:{role}")),
        TypedSyntaxExpressionIR::IntLiteral { value } => Ok(format!("{value}i64")),
        TypedSyntaxExpressionIR::BoolLiteral { value } => Ok(value.to_string()),
        TypedSyntaxExpressionIR::Unary { operator, input } => Ok(format!(
            "({}{})",
            match operator {
                UnaryOperator::Negate => "-",
                UnaryOperator::Not => "!",
            },
            emit_expression(input, sources, operands, definitions)?
        )),
        TypedSyntaxExpressionIR::Binary {
            operator,
            left,
            right,
        } => Ok(format!(
            "({} {} {})",
            emit_expression(left, sources, operands, definitions)?,
            binary_token(*operator),
            emit_expression(right, sources, operands, definitions)?
        )),
        TypedSyntaxExpressionIR::Length { input } => Ok(format!(
            "({}).len() as i64",
            emit_expression(input, sources, operands, definitions)?
        )),
        TypedSyntaxExpressionIR::Index { collection, index } => {
            let mut effects = BTreeSet::new();
            let collection_type =
                infer_expression_type(collection, operands, definitions, &mut effects)?;
            let access = format!(
                "({})[({}) as usize]",
                emit_expression(collection, sources, operands, definitions)?,
                emit_expression(index, sources, operands, definitions)?
            );
            match collection_type {
                ProgramType::SequenceInt => Ok(access),
                ProgramType::NestedSequenceInt => Ok(format!("{access}.clone()")),
                ProgramType::Bytes => Ok(format!("i64::from({access})")),
                _ => Err("TYPED_MECHANISM_INDEX_SOURCE_TYPE".to_string()),
            }
        }
        TypedSyntaxExpressionIR::Call {
            api_token,
            arguments,
        } => Ok(format!(
            "{api_token}({})",
            arguments
                .iter()
                .map(|argument| emit_expression(argument, sources, operands, definitions))
                .collect::<Result<Vec<_>, _>>()?
                .join(", ")
        )),
    }
}

fn lower_expression(
    expression: &TypedSyntaxExpressionIR,
    indices: &BTreeMap<String, usize>,
) -> Result<ScalarExpression, String> {
    match expression {
        TypedSyntaxExpressionIR::Operand { role } => indices
            .get(role)
            .copied()
            .map(|index| ScalarExpression::Argument { index })
            .ok_or_else(|| format!("TYPED_MECHANISM_UNKNOWN_ROLE:{role}")),
        TypedSyntaxExpressionIR::IntLiteral { value } => {
            Ok(ScalarExpression::Constant { value: *value })
        }
        TypedSyntaxExpressionIR::BoolLiteral { value } => {
            Ok(ScalarExpression::BoolConstant { value: *value })
        }
        TypedSyntaxExpressionIR::Unary { operator, input } => Ok(ScalarExpression::Unary {
            operator: *operator,
            input: Box::new(lower_expression(input, indices)?),
        }),
        TypedSyntaxExpressionIR::Binary {
            operator,
            left,
            right,
        } => Ok(ScalarExpression::Binary {
            operator: *operator,
            left: Box::new(lower_expression(left, indices)?),
            right: Box::new(lower_expression(right, indices)?),
        }),
        TypedSyntaxExpressionIR::Length { input } => Ok(ScalarExpression::Length {
            input: Box::new(lower_expression(input, indices)?),
        }),
        TypedSyntaxExpressionIR::Index { collection, index } => Ok(ScalarExpression::Index {
            collection: Box::new(lower_expression(collection, indices)?),
            index: Box::new(lower_expression(index, indices)?),
        }),
        TypedSyntaxExpressionIR::Call {
            api_token,
            arguments,
        } => Ok(ScalarExpression::OpaqueCall {
            api_token: api_token.clone(),
            args: arguments
                .iter()
                .map(|argument| lower_expression(argument, indices))
                .collect::<Result<Vec<_>, _>>()?,
        }),
    }
}

fn validate_observation_bindings(
    goal: &TypedMechanismGoalIR,
    observation: &TypedMechanismObservationIR,
) -> Result<(), String> {
    if observation.operands.len() != goal.operands.len() {
        return Err("TYPED_MECHANISM_OBSERVATION_BINDING_COUNT".to_string());
    }
    for operand in &goal.operands {
        let value = observation
            .operands
            .get(&operand.role)
            .ok_or_else(|| format!("TYPED_MECHANISM_OBSERVATION_MISSING:{}", operand.role))?;
        if value.program_type() != operand.value_type {
            return Err(format!("TYPED_MECHANISM_OBSERVATION_TYPE:{}", operand.role));
        }
    }
    if observation.expected_postimage.program_type() != goal.output_type {
        return Err("TYPED_MECHANISM_OBSERVATION_OUTPUT_TYPE".to_string());
    }
    Ok(())
}

fn complete_expression(
    condition: Option<&str>,
    postimage: &str,
    otherwise: Option<&str>,
) -> Result<String, String> {
    match (condition, otherwise) {
        (Some(condition), Some(otherwise)) => Ok(format!(
            "if {condition} {{ {postimage} }} else {{ {otherwise} }}"
        )),
        (None, None) => Ok(postimage.to_string()),
        _ => Err("TYPED_MECHANISM_CONDITION_POSTIMAGE_SHAPE".to_string()),
    }
}

fn expression_nodes(expression: &TypedSyntaxExpressionIR) -> usize {
    match expression {
        TypedSyntaxExpressionIR::Operand { .. }
        | TypedSyntaxExpressionIR::IntLiteral { .. }
        | TypedSyntaxExpressionIR::BoolLiteral { .. } => 1,
        TypedSyntaxExpressionIR::Unary { input, .. } => 1 + expression_nodes(input),
        TypedSyntaxExpressionIR::Binary { left, right, .. } => {
            1 + expression_nodes(left) + expression_nodes(right)
        }
        TypedSyntaxExpressionIR::Length { input } => 1 + expression_nodes(input),
        TypedSyntaxExpressionIR::Index { collection, index } => {
            1 + expression_nodes(collection) + expression_nodes(index)
        }
        TypedSyntaxExpressionIR::Call { arguments, .. } => {
            1 + arguments.iter().map(expression_nodes).sum::<usize>()
        }
    }
}

fn binary_token(operator: BinaryOperator) -> &'static str {
    match operator {
        BinaryOperator::Add => "+",
        BinaryOperator::Subtract => "-",
        BinaryOperator::Multiply => "*",
        BinaryOperator::Divide => "/",
        BinaryOperator::Modulo => "%",
        BinaryOperator::Equal => "==",
        BinaryOperator::LessThan => "<",
        BinaryOperator::GreaterThan => ">",
        BinaryOperator::And => "&&",
        BinaryOperator::Or => "||",
    }
}

fn rust_type(value_type: &ProgramType) -> &'static str {
    match value_type {
        ProgramType::Int => "i64",
        ProgramType::Bool => "bool",
        ProgramType::SequenceInt => "Vec<i64>",
        ProgramType::NestedSequenceInt => "Vec<Vec<i64>>",
        ProgramType::Bytes => "Vec<u8>",
        ProgramType::Image => "Sem5Image",
        ProgramType::Unit => "()",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn operand(role: &str, source: &str, value_type: ProgramType) -> SourceOperandIR {
        SourceOperandIR {
            role: role.to_string(),
            source: source.to_string(),
            value_type,
        }
    }

    fn role(name: &str) -> TypedSyntaxExpressionIR {
        TypedSyntaxExpressionIR::Operand {
            role: name.to_string(),
        }
    }

    #[test]
    fn typed_goal_lowers_renamed_repository_operands_to_concrete_guarded_postimage() {
        let observations = [(7, 3, 5, 10), (2, 3, 5, -1), (5, 4, 5, 1)]
            .into_iter()
            .map(
                |(current, delta, threshold, expected)| TypedMechanismObservationIR {
                    operands: BTreeMap::from([
                        ("current".to_string(), Value::Int(current)),
                        ("delta".to_string(), Value::Int(delta)),
                        ("threshold".to_string(), Value::Int(threshold)),
                    ]),
                    expected_postimage: Value::Int(expected),
                },
            )
            .collect();
        let goal = TypedMechanismGoalIR {
            schema: TYPED_MECHANISM_GOAL_SCHEMA.to_string(),
            goal_id: "rename_independent_guarded_update".to_string(),
            split: DataSplit::FreshBlind,
            operands: vec![
                operand("current", "self.runtime.accumulator", ProgramType::Int),
                operand("delta", "event.payload.delta", ProgramType::Int),
                operand("threshold", "limits.activation_floor", ProgramType::Int),
            ],
            output_type: ProgramType::Int,
            condition: Some(TypedSyntaxExpressionIR::Binary {
                operator: BinaryOperator::GreaterThan,
                left: Box::new(role("current")),
                right: Box::new(role("threshold")),
            }),
            postimage: TypedSyntaxExpressionIR::Binary {
                operator: BinaryOperator::Add,
                left: Box::new(role("current")),
                right: Box::new(role("delta")),
            },
            otherwise: Some(TypedSyntaxExpressionIR::Binary {
                operator: BinaryOperator::Subtract,
                left: Box::new(role("current")),
                right: Box::new(role("delta")),
            }),
            definitions: Vec::new(),
            allowed_effects: vec![Effect::Pure],
            preconditions: vec!["operands are valid at the source site".to_string()],
            postconditions: vec!["guard selects exactly one postimage".to_string()],
            invariants: vec!["unselected branch has no effect".to_string()],
            public_observations: observations,
            provenance: vec!["RENAMED_REPOSITORY_CANARY".to_string()],
        };

        let template = lower_typed_mechanism_goal(&goal).expect("lower typed mechanism");
        assert!(template.syntax_parse_pass);
        assert!(template.type_effect_check_pass);
        assert_eq!(template.public_observations_checked, 3);
        assert_eq!(template.public_observations_passed, 3);
        assert!(template
            .complete_expression_source
            .contains("self.runtime.accumulator"));
        assert!(template
            .complete_expression_source
            .contains("event.payload.delta"));
        assert!(matches!(
            template.program_task.relation,
            RelationSpec::Mechanism { .. }
        ));
    }

    #[test]
    fn type_mismatch_is_rejected_before_source_installation() {
        let goal = TypedMechanismGoalIR {
            schema: TYPED_MECHANISM_GOAL_SCHEMA.to_string(),
            goal_id: "invalid_types".to_string(),
            split: DataSplit::FreshBlind,
            operands: vec![operand("enabled", "state.enabled", ProgramType::Bool)],
            output_type: ProgramType::Int,
            condition: None,
            postimage: TypedSyntaxExpressionIR::Unary {
                operator: UnaryOperator::Negate,
                input: Box::new(role("enabled")),
            },
            otherwise: None,
            definitions: Vec::new(),
            allowed_effects: vec![Effect::Pure],
            preconditions: Vec::new(),
            postconditions: Vec::new(),
            invariants: Vec::new(),
            public_observations: Vec::new(),
            provenance: Vec::new(),
        };
        assert_eq!(
            lower_typed_mechanism_goal(&goal),
            Err("TYPED_MECHANISM_UNARY_TYPE".to_string())
        );
    }

    #[test]
    fn collection_index_fails_closed_on_non_integer_and_out_of_bounds_indices() {
        let base_goal = TypedMechanismGoalIR {
            schema: TYPED_MECHANISM_GOAL_SCHEMA.to_string(),
            goal_id: "collection_index_contract".to_string(),
            split: DataSplit::FreshBlind,
            operands: vec![
                operand("items", "state.items", ProgramType::SequenceInt),
                operand("position", "request.position", ProgramType::Bool),
            ],
            output_type: ProgramType::Int,
            condition: None,
            postimage: TypedSyntaxExpressionIR::Index {
                collection: Box::new(role("items")),
                index: Box::new(role("position")),
            },
            otherwise: None,
            definitions: Vec::new(),
            allowed_effects: vec![Effect::Pure],
            preconditions: Vec::new(),
            postconditions: Vec::new(),
            invariants: Vec::new(),
            public_observations: Vec::new(),
            provenance: Vec::new(),
        };
        assert_eq!(
            lower_typed_mechanism_goal(&base_goal),
            Err("TYPED_MECHANISM_INDEX_NOT_INT".to_string())
        );

        let mut out_of_bounds_goal = base_goal;
        out_of_bounds_goal.operands[1].value_type = ProgramType::Int;
        out_of_bounds_goal.public_observations = vec![TypedMechanismObservationIR {
            operands: BTreeMap::from([
                ("items".to_string(), Value::Sequence(Vec::new())),
                ("position".to_string(), Value::Int(0)),
            ]),
            expected_postimage: Value::Int(0),
        }];
        assert_eq!(
            lower_typed_mechanism_goal(&out_of_bounds_goal),
            Err("TYPED_MECHANISM_OBSERVATION_EXECUTE:0:INDEX_OUT_OF_BOUNDS".to_string())
        );
    }

    #[test]
    fn counterexample_rejects_semantically_wrong_postimage() {
        let goal = TypedMechanismGoalIR {
            schema: TYPED_MECHANISM_GOAL_SCHEMA.to_string(),
            goal_id: "counterexample".to_string(),
            split: DataSplit::FreshBlind,
            operands: vec![operand("left", "node.left", ProgramType::Int)],
            output_type: ProgramType::Int,
            condition: None,
            postimage: TypedSyntaxExpressionIR::Binary {
                operator: BinaryOperator::Add,
                left: Box::new(role("left")),
                right: Box::new(TypedSyntaxExpressionIR::IntLiteral { value: 1 }),
            },
            otherwise: None,
            definitions: Vec::new(),
            allowed_effects: vec![Effect::Pure],
            preconditions: Vec::new(),
            postconditions: Vec::new(),
            invariants: Vec::new(),
            public_observations: vec![TypedMechanismObservationIR {
                operands: BTreeMap::from([("left".to_string(), Value::Int(4))]),
                expected_postimage: Value::Int(9),
            }],
            provenance: Vec::new(),
        };
        assert_eq!(
            lower_typed_mechanism_goal(&goal),
            Err("TYPED_MECHANISM_COUNTEREXAMPLE:0".to_string())
        );
    }

    #[test]
    fn observations_synthesize_postimage_without_a_solution_template() {
        let request = TypedMechanismSynthesisGoalIR {
            schema: TYPED_MECHANISM_SYNTHESIS_GOAL_SCHEMA.to_string(),
            goal_id: "unknown_repository_addition".to_string(),
            split: DataSplit::FreshBlind,
            operands: vec![
                operand("left_role", "graph.nodes[source].weight", ProgramType::Int),
                operand("right_role", "delta.payload", ProgramType::Int),
            ],
            output_type: ProgramType::Int,
            definitions: Vec::new(),
            allowed_effects: vec![Effect::Pure],
            preconditions: Vec::new(),
            postconditions: vec!["matches all public postimages".to_string()],
            invariants: Vec::new(),
            public_observations: [(2, 3, 5), (-4, 9, 5), (11, -8, 3)]
                .into_iter()
                .map(|(left, right, expected)| TypedMechanismObservationIR {
                    operands: BTreeMap::from([
                        ("left_role".to_string(), Value::Int(left)),
                        ("right_role".to_string(), Value::Int(right)),
                    ]),
                    expected_postimage: Value::Int(expected),
                })
                .collect(),
            require_conditional: false,
            max_expression_depth: 2,
            max_candidates: 1_024,
            provenance: vec!["PUBLIC_OBSERVATION_ONLY".to_string()],
        };

        let receipt =
            synthesize_typed_mechanism_goal(&request).expect("synthesize typed expression");
        assert!(receipt.counterexample_guided_selection);
        assert!(!receipt.conditional_synthesized);
        assert!(receipt.candidates_falsified > 0);
        assert!(receipt.template.postimage_source.contains('+'));
        assert!(receipt
            .template
            .postimage_source
            .contains("graph.nodes[source].weight"));
        assert_eq!(receipt.template.public_observations_passed, 3);
    }

    #[test]
    fn successful_expression_operator_transfers_by_typed_role_and_short_circuits_search() {
        let request = TypedMechanismSynthesisGoalIR {
            schema: TYPED_MECHANISM_SYNTHESIS_GOAL_SCHEMA.to_string(),
            goal_id: "first_repository".to_string(),
            split: DataSplit::FreshBlind,
            operands: vec![
                operand("left", "left", ProgramType::Int),
                operand("right", "right", ProgramType::Int),
            ],
            output_type: ProgramType::Int,
            definitions: Vec::new(),
            allowed_effects: vec![Effect::Pure],
            preconditions: Vec::new(),
            postconditions: Vec::new(),
            invariants: Vec::new(),
            public_observations: [(2, 3, 5), (4, 7, 11)]
                .into_iter()
                .map(|(left, right, expected)| TypedMechanismObservationIR {
                    operands: BTreeMap::from([
                        ("left".to_string(), Value::Int(left)),
                        ("right".to_string(), Value::Int(right)),
                    ]),
                    expected_postimage: Value::Int(expected),
                })
                .collect(),
            require_conditional: false,
            max_expression_depth: 2,
            max_candidates: 1_024,
            provenance: Vec::new(),
        };
        let learned = synthesize_typed_mechanism_goal(&request).unwrap();
        let operator =
            typed_mechanism_improvement_operator_from_receipt(&learned, "a".repeat(64)).unwrap();
        let renamed = TypedMechanismSynthesisGoalIR {
            goal_id: "renamed_repository".to_string(),
            operands: vec![
                operand("alpha", "payload.alpha", ProgramType::Int),
                operand("beta", "payload.beta", ProgramType::Int),
            ],
            public_observations: [(8, 5, 13), (-4, 9, 5)]
                .into_iter()
                .map(|(alpha, beta, expected)| TypedMechanismObservationIR {
                    operands: BTreeMap::from([
                        ("alpha".to_string(), Value::Int(alpha)),
                        ("beta".to_string(), Value::Int(beta)),
                    ]),
                    expected_postimage: Value::Int(expected),
                })
                .collect(),
            ..request
        };
        let transferred =
            synthesize_typed_mechanism_goal_with_priors(&renamed, std::slice::from_ref(&operator))
                .unwrap();
        assert!(transferred.preferred_operator_selected);
        assert_eq!(
            transferred.selected_operator_id.as_deref(),
            Some(operator.operator_id.as_str())
        );
        assert_eq!(transferred.candidates_enumerated, 1);
        assert!(transferred.candidates_enumerated < learned.candidates_enumerated);
        assert!(transferred
            .template
            .postimage_source
            .contains("payload.alpha"));
        assert!(transferred
            .template
            .postimage_source
            .contains("payload.beta"));

        let multiplication_seed = TypedMechanismSynthesisGoalIR {
            goal_id: "multiplication_seed".to_string(),
            public_observations: [(3, 4, 12), (-2, 5, -10)]
                .into_iter()
                .map(|(alpha, beta, expected)| TypedMechanismObservationIR {
                    operands: BTreeMap::from([
                        ("alpha".to_string(), Value::Int(alpha)),
                        ("beta".to_string(), Value::Int(beta)),
                    ]),
                    expected_postimage: Value::Int(expected),
                })
                .collect(),
            ..renamed.clone()
        };
        let multiplication_receipt = synthesize_typed_mechanism_goal(&multiplication_seed).unwrap();
        let multiplication_operator = typed_mechanism_improvement_operator_from_receipt(
            &multiplication_receipt,
            "b".repeat(64),
        )
        .unwrap();
        let collision = synthesize_typed_mechanism_goal_with_priors(
            &renamed,
            &[multiplication_operator.clone(), operator.clone()],
        )
        .unwrap();
        assert!(!collision.parallel_operator_evaluation);
        assert_eq!(collision.preferred_operator_attempts, 2);
        assert_eq!(
            collision.selected_operator_id.as_deref(),
            Some(operator.operator_id.as_str())
        );
        assert_eq!(
            collision.rejected_operator_ids,
            [multiplication_operator.operator_id]
        );

        let counterexample = TypedMechanismSynthesisGoalIR {
            goal_id: "operator_counterexample".to_string(),
            public_observations: [(3, 4, 12), (-2, 5, -10)]
                .into_iter()
                .map(|(alpha, beta, expected)| TypedMechanismObservationIR {
                    operands: BTreeMap::from([
                        ("alpha".to_string(), Value::Int(alpha)),
                        ("beta".to_string(), Value::Int(beta)),
                    ]),
                    expected_postimage: Value::Int(expected),
                })
                .collect(),
            ..renamed
        };
        let revised = synthesize_typed_mechanism_goal_with_priors(
            &counterexample,
            std::slice::from_ref(&operator),
        )
        .unwrap();
        assert!(!revised.preferred_operator_selected);
        assert_eq!(revised.preferred_operator_attempts, 1);
        assert!(revised.template.postimage_source.contains('*'));

        let mut tampered = operator;
        tampered.postimage = TypedSyntaxExpressionIR::IntLiteral { value: 0 };
        assert_eq!(
            validate_typed_mechanism_improvement_operator(&tampered),
            Err("TYPED_MECHANISM_IMPROVEMENT_OPERATOR_ID_MISMATCH".to_string())
        );
    }

    #[test]
    fn observations_synthesize_condition_and_two_postimages() {
        let request = TypedMechanismSynthesisGoalIR {
            schema: TYPED_MECHANISM_SYNTHESIS_GOAL_SCHEMA.to_string(),
            goal_id: "unknown_absolute_delta".to_string(),
            split: DataSplit::FreshBlind,
            operands: vec![
                operand("new_value", "sample.current", ProgramType::Int),
                operand("old_value", "sample.previous", ProgramType::Int),
            ],
            output_type: ProgramType::Int,
            definitions: Vec::new(),
            allowed_effects: vec![Effect::Pure],
            preconditions: Vec::new(),
            postconditions: vec!["result is an unsigned distance".to_string()],
            invariants: Vec::new(),
            public_observations: [(9, 4, 5), (3, 8, 5), (-2, -9, 7), (-8, -3, 5)]
                .into_iter()
                .map(
                    |(new_value, old_value, expected)| TypedMechanismObservationIR {
                        operands: BTreeMap::from([
                            ("new_value".to_string(), Value::Int(new_value)),
                            ("old_value".to_string(), Value::Int(old_value)),
                        ]),
                        expected_postimage: Value::Int(expected),
                    },
                )
                .collect(),
            require_conditional: true,
            max_expression_depth: 2,
            max_candidates: 1_024,
            provenance: vec!["PUBLIC_OBSERVATION_ONLY".to_string()],
        };

        let receipt =
            synthesize_typed_mechanism_goal(&request).expect("synthesize guarded typed expression");
        assert!(receipt.conditional_synthesized);
        assert!(receipt.template.condition_source.is_some());
        assert!(receipt.template.otherwise_source.is_some());
        assert!(receipt
            .template
            .complete_expression_source
            .starts_with("if "));
        assert_eq!(receipt.template.public_observations_passed, 4);
    }
}
