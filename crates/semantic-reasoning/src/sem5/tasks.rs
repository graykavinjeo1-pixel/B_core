use std::collections::BTreeMap;

use serde::Serialize;
use sha2::{Digest, Sha256};

use super::{
    ir::eval_scalar,
    model::{
        ApiDefinition, BinaryOperator, BindingSpec, DataSplit, Effect, EvaluatorMetadata,
        EvaluatorTask, ImageValue, ProgramIrSpec, ProgramTask, ProgramTaskFamily, ProgramType,
        ProgrammingPrimitiveRecord, RelationSpec, RustMinAllowlist, ScalarExpression, TaskManifest,
        Value,
    },
};

pub const BLIND_GENERATOR_VERSION: &str = "SEM5-BLIND-GENERATOR-1.0.1";

#[derive(Debug, Clone)]
pub struct GeneratedTaskSets {
    pub discovery: Vec<EvaluatorTask>,
    pub calibration: Vec<EvaluatorTask>,
    pub blind: Vec<EvaluatorTask>,
    pub opaque_api: Vec<EvaluatorTask>,
    pub adversarial: Vec<EvaluatorTask>,
}

#[derive(Debug, Clone)]
struct DeterministicRng {
    state: u64,
}

impl DeterministicRng {
    fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    fn next_u64(&mut self) -> u64 {
        self.state = self
            .state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        self.state
    }

    fn range(&mut self, start: i64, end: i64) -> i64 {
        start + (self.next_u64() % u64::try_from(end - start).expect("positive range")) as i64
    }

    fn tag(&mut self) -> String {
        format!("{:08x}", self.next_u64() as u32)
    }
}

fn arg(index: usize) -> ScalarExpression {
    ScalarExpression::Argument { index }
}

fn constant(value: i64) -> ScalarExpression {
    ScalarExpression::Constant { value }
}

fn binary(
    operator: BinaryOperator,
    left: ScalarExpression,
    right: ScalarExpression,
) -> ScalarExpression {
    ScalarExpression::Binary {
        operator,
        left: Box::new(left),
        right: Box::new(right),
    }
}

fn scalar_rule(rng: &mut DeterministicRng, arity: usize) -> ScalarExpression {
    let selected = usize::try_from(rng.next_u64() % u64::try_from(arity).unwrap()).unwrap();
    let offset = rng.range(-7, 8);
    let factor = rng.range(1, 5);
    binary(
        BinaryOperator::Add,
        binary(BinaryOperator::Multiply, arg(selected), constant(factor)),
        constant(offset),
    )
}

fn predicate_rule(rng: &mut DeterministicRng, argument: usize) -> ScalarExpression {
    let modulus = rng.range(2, 6);
    let residue = rng.range(0, modulus);
    binary(
        BinaryOperator::Equal,
        binary(BinaryOperator::Modulo, arg(argument), constant(modulus)),
        constant(residue),
    )
}

fn inputs_for(types: &[ProgramType]) -> Vec<BindingSpec> {
    types
        .iter()
        .enumerate()
        .map(|(index, value_type)| BindingSpec {
            name: format!("v{index}"),
            value_type: value_type.clone(),
            mutable: false,
        })
        .collect()
}

pub fn generate_task_sets(seed: u64) -> GeneratedTaskSets {
    let mut rng = DeterministicRng::new(seed);
    let mut blind = Vec::with_capacity(130);
    for index in 0..20 {
        blind.push(make_scalar_task(&mut rng, index, DataSplit::FreshBlind));
    }
    for index in 0..30 {
        blind.push(if index % 2 == 0 {
            make_collection_task(&mut rng, index, DataSplit::FreshBlind)
        } else {
            make_stateful_task(&mut rng, index, DataSplit::FreshBlind)
        });
    }
    for index in 0..20 {
        blind.push(make_nested_task(&mut rng, index, DataSplit::FreshBlind));
    }
    for index in 0..20 {
        blind.push(if index < 10 {
            make_buffer_task(&mut rng, index, DataSplit::FreshBlind)
        } else {
            make_image_task(&mut rng, index, DataSplit::FreshBlind)
        });
    }
    for index in 0..20 {
        blind.push(make_opaque_task(&mut rng, index, DataSplit::OpaqueApiBlind));
    }
    for index in 0..20 {
        blind.push(make_composition_task(
            &mut rng,
            index,
            DataSplit::AdversarialBlind,
        ));
    }
    let opaque_api = blind
        .iter()
        .filter(|task| task.visible.split == DataSplit::OpaqueApiBlind)
        .cloned()
        .collect();
    let adversarial = blind
        .iter()
        .filter(|task| task.visible.split == DataSplit::AdversarialBlind)
        .cloned()
        .collect();

    let mut discovery = Vec::with_capacity(40);
    let mut calibration = Vec::with_capacity(16);
    for index in 0..40 {
        let task = match index % 5 {
            0 => make_collection_task(&mut rng, index, DataSplit::Discovery),
            1 => make_stateful_task(&mut rng, index, DataSplit::Discovery),
            2 => make_nested_task(&mut rng, index, DataSplit::Discovery),
            3 => make_buffer_task(&mut rng, index, DataSplit::Discovery),
            _ => make_composition_task(&mut rng, index, DataSplit::Discovery),
        };
        discovery.push(task);
    }
    for index in 0..16 {
        let task = match index % 4 {
            0 => make_collection_task(&mut rng, index, DataSplit::Calibration),
            1 => make_stateful_task(&mut rng, index, DataSplit::Calibration),
            2 => make_nested_task(&mut rng, index, DataSplit::Calibration),
            _ => make_composition_task(&mut rng, index, DataSplit::Calibration),
        };
        calibration.push(task);
    }
    GeneratedTaskSets {
        discovery,
        calibration,
        blind,
        opaque_api,
        adversarial,
    }
}

#[allow(clippy::too_many_arguments)]
fn base_task(
    rng: &mut DeterministicRng,
    index: usize,
    split: DataSplit,
    input_types: &[ProgramType],
    output_type: ProgramType,
    relation: RelationSpec,
    definitions: Vec<ApiDefinition>,
    allowed_effects: Vec<Effect>,
    family: ProgramTaskFamily,
    adversarial: bool,
) -> EvaluatorTask {
    let task_id = format!("T-{index:03}-{}", rng.tag());
    let inputs = inputs_for(input_types);
    let visible = ProgramTask {
        task_id,
        split,
        inputs,
        output_type,
        relation,
        definitions,
        allowed_effects: allowed_effects.clone(),
        preconditions: vec!["inputs satisfy declared types and bounded sizes".to_string()],
        postconditions: vec!["result satisfies the declared relation".to_string()],
        invariants: vec!["all indexed accesses remain within the current buffer".to_string()],
        demonstrations: Vec::new(),
        provenance: vec![
            BLIND_GENERATOR_VERSION.to_string(),
            "DETERMINISTIC_SEED".to_string(),
        ],
    };
    let invalid_cases = vec![BTreeMap::new()];
    EvaluatorTask {
        visible,
        evaluator: EvaluatorMetadata {
            family,
            adversarial,
            hidden_cases: Vec::new(),
            invalid_cases,
            expected_effects: allowed_effects,
            solution_graph_depth: if adversarial { 8 } else { 4 },
            primitive_expanded_depth: if adversarial { 17 } else { 9 },
            concepts_composed: if adversarial { 4 } else { 2 },
        },
    }
}

fn make_scalar_task(rng: &mut DeterministicRng, index: usize, split: DataSplit) -> EvaluatorTask {
    let arity = 2 + index % 2;
    let types = vec![ProgramType::Int; arity];
    let relation = RelationSpec::Scalar {
        expression: scalar_rule(rng, arity),
    };
    base_task(
        rng,
        index,
        split,
        &types,
        ProgramType::Int,
        relation,
        Vec::new(),
        vec![Effect::Pure],
        ProgramTaskFamily::ScalarBasic,
        false,
    )
}

fn make_collection_task(
    rng: &mut DeterministicRng,
    index: usize,
    split: DataSplit,
) -> EvaluatorTask {
    let relation = RelationSpec::Collection {
        expression: scalar_rule(rng, 2),
        include_when: (index % 3 != 0).then(|| predicate_rule(rng, 0)),
    };
    base_task(
        rng,
        index,
        split,
        &[ProgramType::SequenceInt],
        ProgramType::SequenceInt,
        relation,
        Vec::new(),
        vec![Effect::Pure, Effect::LocalMutation, Effect::BufferMutation],
        ProgramTaskFamily::Sequence,
        false,
    )
}

fn make_stateful_task(rng: &mut DeterministicRng, index: usize, split: DataSplit) -> EvaluatorTask {
    let initial = rng.range(-4, 5);
    let update = binary(
        BinaryOperator::Add,
        arg(0),
        binary(BinaryOperator::Multiply, arg(1), constant(rng.range(1, 4))),
    );
    let relation = RelationSpec::Stateful {
        initial,
        update,
        reset_when: (index % 4 == 0).then(|| predicate_rule(rng, 1)),
        emit_each: false,
    };
    base_task(
        rng,
        index,
        split,
        &[ProgramType::SequenceInt],
        ProgramType::Int,
        relation,
        Vec::new(),
        vec![Effect::Pure, Effect::LocalMutation],
        ProgramTaskFamily::Stateful,
        false,
    )
}

fn make_nested_task(rng: &mut DeterministicRng, index: usize, split: DataSplit) -> EvaluatorTask {
    let relation = RelationSpec::Nested {
        expression: scalar_rule(rng, 3),
        include_when: (index % 2 == 0).then(|| predicate_rule(rng, 0)),
    };
    base_task(
        rng,
        index,
        split,
        &[ProgramType::NestedSequenceInt],
        ProgramType::SequenceInt,
        relation,
        Vec::new(),
        vec![Effect::Pure, Effect::LocalMutation, Effect::BufferMutation],
        ProgramTaskFamily::NestedSequence,
        false,
    )
}

fn make_buffer_task(rng: &mut DeterministicRng, index: usize, split: DataSplit) -> EvaluatorTask {
    let relation = RelationSpec::Buffer {
        expression: binary(BinaryOperator::Add, arg(0), constant(rng.range(1, 8))),
        write_output: index % 2 == 0,
    };
    let mut effects = vec![Effect::Pure, Effect::LocalMutation, Effect::BufferMutation];
    effects.push(Effect::SandboxFileRead);
    if index % 2 == 0 {
        effects.push(Effect::SandboxFileWrite);
    }
    base_task(
        rng,
        index,
        split,
        &[ProgramType::Bytes],
        ProgramType::Bytes,
        relation,
        Vec::new(),
        effects,
        ProgramTaskFamily::FileTransform,
        false,
    )
}

fn make_image_task(rng: &mut DeterministicRng, index: usize, split: DataSplit) -> EvaluatorTask {
    let relation = RelationSpec::Image {
        expression: binary(BinaryOperator::Add, arg(0), constant(rng.range(-4, 5))),
        apply_when: (index % 2 == 0).then(|| {
            binary(
                BinaryOperator::GreaterThan,
                arg(0),
                constant(rng.range(20, 100)),
            )
        }),
    };
    base_task(
        rng,
        index,
        split,
        &[ProgramType::Image],
        ProgramType::Image,
        relation,
        Vec::new(),
        vec![Effect::Pure, Effect::LocalMutation, Effect::BufferMutation],
        ProgramTaskFamily::ImageTransform,
        false,
    )
}

fn make_opaque_task(rng: &mut DeterministicRng, index: usize, split: DataSplit) -> EvaluatorTask {
    let token = format!("q_{}", rng.tag());
    let formal_body = binary(
        BinaryOperator::Add,
        binary(BinaryOperator::Multiply, arg(0), constant(rng.range(2, 6))),
        binary(BinaryOperator::Subtract, arg(1), constant(rng.range(-5, 6))),
    );
    let definition = ApiDefinition {
        api_token: token.clone(),
        inputs: vec![ProgramType::Int, ProgramType::Int],
        output: ProgramType::Int,
        effect: Effect::Pure,
        preconditions: vec!["arguments are signed 64-bit integers".to_string()],
        postconditions: vec!["result equals formal_body".to_string()],
        formal_body,
        examples: Vec::new(),
        randomized_symbol: true,
        provenance: vec!["DEFINITION_ONLY_RANDOMIZED_API".to_string()],
    };
    let relation = RelationSpec::OpaqueUse {
        api_token: token,
        arguments: vec![arg(0), arg(1)],
    };
    base_task(
        rng,
        index,
        split,
        &[ProgramType::Int, ProgramType::Int],
        ProgramType::Int,
        relation,
        vec![definition],
        vec![Effect::Pure],
        ProgramTaskFamily::OpaqueApi,
        false,
    )
}

fn make_composition_task(
    rng: &mut DeterministicRng,
    index: usize,
    split: DataSplit,
) -> EvaluatorTask {
    let first = RelationSpec::Collection {
        expression: scalar_rule(rng, 2),
        include_when: Some(predicate_rule(rng, 0)),
    };
    let second = RelationSpec::Stateful {
        initial: rng.range(-3, 4),
        update: binary(BinaryOperator::Add, arg(0), arg(1)),
        reset_when: (index % 3 == 0).then(|| predicate_rule(rng, 1)),
        emit_each: false,
    };
    base_task(
        rng,
        index,
        split,
        &[ProgramType::SequenceInt],
        ProgramType::Int,
        RelationSpec::Composition {
            stages: vec![first, second],
        },
        Vec::new(),
        vec![Effect::Pure, Effect::LocalMutation, Effect::BufferMutation],
        ProgramTaskFamily::MultiStage,
        true,
    )
}

fn hidden_case(
    rng: &mut DeterministicRng,
    inputs: &[BindingSpec],
    case_index: usize,
) -> BTreeMap<String, Value> {
    inputs
        .iter()
        .map(|binding| {
            let value = match binding.value_type {
                ProgramType::Int => Value::Int(rng.range(-20, 21)),
                ProgramType::Bool => Value::Bool(rng.next_u64() % 2 == 0),
                ProgramType::SequenceInt => Value::Sequence(
                    (0..(3 + case_index % 5))
                        .map(|_| rng.range(-12, 13))
                        .collect(),
                ),
                ProgramType::NestedSequenceInt => Value::NestedSequence(
                    (0..(2 + case_index % 3))
                        .map(|outer| (0..(2 + outer % 3)).map(|_| rng.range(-10, 11)).collect())
                        .collect(),
                ),
                ProgramType::Bytes => Value::Bytes(
                    (0..(5 + case_index % 4))
                        .map(|_| u8::try_from(rng.range(0, 201)).unwrap())
                        .collect(),
                ),
                ProgramType::Image => {
                    let width = 3 + case_index % 3;
                    let height = 2 + case_index % 2;
                    Value::Image(ImageValue {
                        width,
                        height,
                        channels: 1,
                        pixels: (0..width * height).map(|_| rng.range(8, 220)).collect(),
                    })
                }
                ProgramType::Unit => Value::Unit,
            };
            (binding.name.clone(), value)
        })
        .collect()
}

pub fn generate_property_cases(task: &ProgramTask, seed: u64) -> Vec<BTreeMap<String, Value>> {
    let mut rng = DeterministicRng::new(seed);
    (0..8)
        .map(|case_index| hidden_case(&mut rng, &task.inputs, case_index))
        .collect()
}

pub fn evaluate_contract(
    task: &ProgramTask,
    inputs: &BTreeMap<String, Value>,
) -> Result<Value, String> {
    let values = task
        .inputs
        .iter()
        .map(|binding| {
            inputs
                .get(&binding.name)
                .cloned()
                .ok_or_else(|| format!("MISSING_INPUT:{}", binding.name))
        })
        .collect::<Result<Vec<_>, _>>()?;
    let api_map = task
        .definitions
        .iter()
        .map(|api| (api.api_token.clone(), api))
        .collect::<BTreeMap<_, _>>();
    evaluate_relation(&task.relation, values, &api_map)
}

fn evaluate_relation(
    relation: &RelationSpec,
    inputs: Vec<Value>,
    apis: &BTreeMap<String, &ApiDefinition>,
) -> Result<Value, String> {
    match relation {
        RelationSpec::Scalar { expression } => eval_scalar(expression, &inputs, apis),
        RelationSpec::Collection {
            expression,
            include_when,
        } => {
            let source = expect_sequence(inputs.first())?;
            let mut result = Vec::new();
            for (index, item) in source.iter().enumerate() {
                let args = [Value::Int(*item), Value::Int(index as i64)];
                let include = include_when
                    .as_ref()
                    .map(|condition| {
                        eval_scalar(condition, &args, apis).and_then(expect_bool_owned)
                    })
                    .transpose()?
                    .unwrap_or(true);
                if include {
                    result.push(expect_int_owned(eval_scalar(expression, &args, apis)?)?);
                }
            }
            Ok(Value::Sequence(result))
        }
        RelationSpec::Stateful {
            initial,
            update,
            reset_when,
            emit_each,
        } => {
            let source = expect_sequence(inputs.first())?;
            let mut state = *initial;
            let mut emitted = Vec::new();
            for (index, item) in source.iter().enumerate() {
                let args = [
                    Value::Int(state),
                    Value::Int(*item),
                    Value::Int(index as i64),
                ];
                if reset_when
                    .as_ref()
                    .map(|condition| {
                        eval_scalar(condition, &args, apis).and_then(expect_bool_owned)
                    })
                    .transpose()?
                    .unwrap_or(false)
                {
                    state = *initial;
                } else {
                    state = expect_int_owned(eval_scalar(update, &args, apis)?)?;
                }
                if *emit_each {
                    emitted.push(state);
                }
            }
            if *emit_each {
                Ok(Value::Sequence(emitted))
            } else {
                Ok(Value::Int(state))
            }
        }
        RelationSpec::Nested {
            expression,
            include_when,
        } => {
            let source = match inputs.first() {
                Some(Value::NestedSequence(values)) => values,
                _ => return Err("EXPECTED_NESTED_SEQUENCE".to_string()),
            };
            let mut result = Vec::new();
            for (outer, row) in source.iter().enumerate() {
                for (inner, item) in row.iter().enumerate() {
                    let args = [
                        Value::Int(*item),
                        Value::Int(inner as i64),
                        Value::Int(outer as i64),
                    ];
                    let include = include_when
                        .as_ref()
                        .map(|condition| {
                            eval_scalar(condition, &args, apis).and_then(expect_bool_owned)
                        })
                        .transpose()?
                        .unwrap_or(true);
                    if include {
                        result.push(expect_int_owned(eval_scalar(expression, &args, apis)?)?);
                    }
                }
            }
            Ok(Value::Sequence(result))
        }
        RelationSpec::Buffer { expression, .. } => {
            let bytes = match inputs.first() {
                Some(Value::Bytes(values)) => values,
                _ => return Err("EXPECTED_BYTES".to_string()),
            };
            let mut output = Vec::with_capacity(bytes.len());
            for (index, item) in bytes.iter().enumerate() {
                let value = expect_int_owned(eval_scalar(
                    expression,
                    &[Value::Int(i64::from(*item)), Value::Int(index as i64)],
                    apis,
                )?)?;
                output.push(u8::try_from(value).map_err(|_| "BYTE_RANGE".to_string())?);
            }
            Ok(Value::Bytes(output))
        }
        RelationSpec::Image {
            expression,
            apply_when,
        } => {
            let image = match inputs.first() {
                Some(Value::Image(image)) => image,
                _ => return Err("EXPECTED_IMAGE".to_string()),
            };
            let mut output = image.clone();
            for (index, item) in image.pixels.iter().enumerate() {
                let args = [Value::Int(*item), Value::Int(index as i64)];
                let apply = apply_when
                    .as_ref()
                    .map(|condition| {
                        eval_scalar(condition, &args, apis).and_then(expect_bool_owned)
                    })
                    .transpose()?
                    .unwrap_or(true);
                if apply {
                    output.pixels[index] = expect_int_owned(eval_scalar(expression, &args, apis)?)?;
                }
            }
            Ok(Value::Image(output))
        }
        RelationSpec::Composition { stages } => {
            let mut current = inputs;
            let mut output = Value::Unit;
            for stage in stages {
                output = evaluate_relation(stage, current, apis)?;
                current = vec![output.clone()];
            }
            Ok(output)
        }
        RelationSpec::OpaqueUse {
            api_token,
            arguments,
        } => {
            let api = apis
                .get(api_token)
                .ok_or_else(|| format!("UNKNOWN_API:{api_token}"))?;
            let values = arguments
                .iter()
                .map(|argument| eval_scalar(argument, &inputs, apis))
                .collect::<Result<Vec<_>, _>>()?;
            eval_scalar(&api.formal_body, &values, apis)
        }
    }
}

fn expect_sequence(value: Option<&Value>) -> Result<&Vec<i64>, String> {
    match value {
        Some(Value::Sequence(values)) => Ok(values),
        _ => Err("EXPECTED_SEQUENCE".to_string()),
    }
}

fn expect_int_owned(value: Value) -> Result<i64, String> {
    match value {
        Value::Int(value) => Ok(value),
        _ => Err("EXPECTED_INT".to_string()),
    }
}

fn expect_bool_owned(value: Value) -> Result<bool, String> {
    match value {
        Value::Bool(value) => Ok(value),
        _ => Err("EXPECTED_BOOL".to_string()),
    }
}

pub fn build_manifest(
    run_id: &str,
    seed: u64,
    split: DataSplit,
    tasks: &[EvaluatorTask],
) -> Result<TaskManifest, String> {
    let visible = tasks
        .iter()
        .map(|task| task.visible.clone())
        .collect::<Vec<_>>();
    #[derive(Serialize)]
    struct Commitment<'a> {
        run_id: &'a str,
        generator_version: &'a str,
        seed_commitment_sha256: String,
        split: DataSplit,
        tasks: &'a [ProgramTask],
        expected_outputs_included: bool,
        family_metadata_included: bool,
        reference_source_included: bool,
    }
    let seed_commitment_sha256 = hex_sha256(&seed.to_le_bytes());
    let commitment = Commitment {
        run_id,
        generator_version: BLIND_GENERATOR_VERSION,
        seed_commitment_sha256: seed_commitment_sha256.clone(),
        split,
        tasks: &visible,
        expected_outputs_included: false,
        family_metadata_included: false,
        reference_source_included: false,
    };
    let manifest_sha256 = hex_sha256(
        &serde_json::to_vec(&commitment).map_err(|error| format!("MANIFEST_JSON:{error}"))?,
    );
    Ok(TaskManifest {
        run_id: run_id.to_string(),
        generator_version: BLIND_GENERATOR_VERSION.to_string(),
        seed_commitment_sha256,
        split,
        tasks: visible,
        expected_outputs_included: false,
        family_metadata_included: false,
        reference_source_included: false,
        manifest_sha256,
    })
}

pub fn programming_primitive_catalog() -> Vec<ProgrammingPrimitiveRecord> {
    [
        (
            "P-LITERAL",
            "LITERAL",
            vec![],
            ProgramType::Int,
            vec![Effect::Pure],
            "construct a typed scalar value",
        ),
        (
            "P-VARIABLE",
            "VARIABLE",
            vec![],
            ProgramType::Int,
            vec![Effect::Pure],
            "refer to a typed binding",
        ),
        (
            "P-LOAD",
            "LOAD",
            vec![],
            ProgramType::Int,
            vec![Effect::Pure],
            "read a typed binding",
        ),
        (
            "P-STORE",
            "STORE",
            vec![ProgramType::Int],
            ProgramType::Unit,
            vec![Effect::LocalMutation],
            "replace a local binding value",
        ),
        (
            "P-UNARY",
            "UNARY_OP",
            vec![ProgramType::Int],
            ProgramType::Int,
            vec![Effect::Pure],
            "apply one declared primitive operator",
        ),
        (
            "P-BINARY",
            "BINARY_OP",
            vec![ProgramType::Int, ProgramType::Int],
            ProgramType::Int,
            vec![Effect::Pure],
            "apply one declared binary operator",
        ),
        (
            "P-SEQ-CREATE",
            "SEQUENCE_CREATE",
            vec![],
            ProgramType::SequenceInt,
            vec![Effect::Pure],
            "create an empty or explicit sequence",
        ),
        (
            "P-SEQ-READ",
            "SEQUENCE_READ",
            vec![ProgramType::SequenceInt, ProgramType::Int],
            ProgramType::Int,
            vec![Effect::Pure],
            "read one bounded position",
        ),
        (
            "P-SEQ-WRITE",
            "SEQUENCE_WRITE",
            vec![ProgramType::SequenceInt, ProgramType::Int, ProgramType::Int],
            ProgramType::Unit,
            vec![Effect::BufferMutation],
            "write one bounded position",
        ),
        (
            "P-SEQ-APPEND",
            "SEQUENCE_APPEND",
            vec![ProgramType::SequenceInt, ProgramType::Int],
            ProgramType::Unit,
            vec![Effect::BufferMutation],
            "append one value",
        ),
        (
            "P-IF",
            "IF",
            vec![ProgramType::Bool],
            ProgramType::Unit,
            vec![Effect::Pure],
            "choose a branch from a boolean",
        ),
        (
            "P-LOOP",
            "LOOP",
            vec![ProgramType::SequenceInt],
            ProgramType::Unit,
            vec![Effect::LocalMutation],
            "visit bounded source positions",
        ),
        (
            "P-CALL",
            "CALL",
            vec![],
            ProgramType::Int,
            vec![Effect::Pure],
            "invoke a supplied formal definition",
        ),
        (
            "P-RETURN",
            "RETURN",
            vec![],
            ProgramType::Unit,
            vec![Effect::Pure],
            "produce the program result",
        ),
        (
            "P-BLOCK",
            "BLOCK",
            vec![],
            ProgramType::Unit,
            vec![Effect::Pure],
            "execute child nodes in order",
        ),
        (
            "P-BREAK",
            "BREAK",
            vec![],
            ProgramType::Unit,
            vec![Effect::Pure],
            "leave the current bounded loop",
        ),
        (
            "P-CONTINUE",
            "CONTINUE",
            vec![],
            ProgramType::Unit,
            vec![Effect::Pure],
            "advance the current bounded loop",
        ),
    ]
    .into_iter()
    .map(
        |(primitive_id, node_kind, input_types, output_type, effects, semantics)| {
            ProgrammingPrimitiveRecord {
                primitive_id: primitive_id.to_string(),
                node_kind: node_kind.to_string(),
                input_types,
                output_type,
                effects,
                executable_semantics: semantics.to_string(),
                provenance: vec!["SEM5_CANONICAL_LOW_LEVEL_BASIS".to_string()],
                high_level_algorithm_seeded: false,
            }
        },
    )
    .collect()
}

pub fn program_ir_spec() -> ProgramIrSpec {
    ProgramIrSpec {
        version: "PROGRAM-IR-1.0.0".to_string(),
        authoritative_representation: "typed semantic graph".to_string(),
        node_types: [
            "LITERAL",
            "VARIABLE",
            "LOAD",
            "STORE",
            "UNARY_OP",
            "BINARY_OP",
            "SEQUENCE_CREATE",
            "SEQUENCE_READ",
            "SEQUENCE_WRITE",
            "SEQUENCE_APPEND",
            "IF",
            "LOOP",
            "CALL",
            "RETURN",
            "BLOCK",
            "BREAK",
            "CONTINUE",
        ]
        .into_iter()
        .map(str::to_string)
        .collect(),
        explicit_types: vec![
            ProgramType::Int,
            ProgramType::Bool,
            ProgramType::SequenceInt,
            ProgramType::NestedSequenceInt,
            ProgramType::Bytes,
            ProgramType::Image,
            ProgramType::Unit,
        ],
        explicit_effects: vec![
            Effect::Pure,
            Effect::LocalMutation,
            Effect::BufferMutation,
            Effect::SandboxFileRead,
            Effect::SandboxFileWrite,
        ],
        carries_dependencies: true,
        carries_provenance: true,
        rust_is_adapter_only: true,
    }
}

pub fn rust_min_allowlist() -> RustMinAllowlist {
    RustMinAllowlist {
        version: "RUST-MIN-1.0.0".to_string(),
        allowed: vec![
            "signed integer and boolean values".to_string(),
            "Vec<i64> and Vec<u8>".to_string(),
            "bounded for loops".to_string(),
            "if/else".to_string(),
            "local bindings and assignment".to_string(),
            "std::fs limited to sandbox input.bin and output.bin".to_string(),
            "println for bounded result serialization".to_string(),
        ],
        forbidden: vec![
            "unsafe".to_string(),
            "network".to_string(),
            "threads".to_string(),
            "process spawning".to_string(),
            "foreign interfaces".to_string(),
            "external crates".to_string(),
            "arbitrary filesystem paths".to_string(),
        ],
        offline_compilation: true,
        external_crates: 0,
    }
}

fn hex_sha256(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blind_shape_and_isolation_are_canonical() {
        let sets = generate_task_sets(17);
        assert_eq!(sets.blind.len(), 130);
        assert_eq!(sets.opaque_api.len(), 20);
        assert_eq!(sets.adversarial.len(), 20);
        let manifest =
            build_manifest("test", 17, DataSplit::FreshBlind, &sets.blind).expect("manifest");
        assert!(!manifest.expected_outputs_included);
        assert!(!manifest.family_metadata_included);
        assert!(!manifest.reference_source_included);
        let encoded = serde_json::to_string(&manifest).expect("json");
        assert!(!encoded.contains("hidden_cases"));
        assert!(!encoded.contains("\"family\":"));
    }

    #[test]
    fn opaque_definitions_are_randomized_and_example_free() {
        let sets = generate_task_sets(19);
        let tokens = sets
            .opaque_api
            .iter()
            .map(|task| {
                let definition = &task.visible.definitions[0];
                assert!(definition.randomized_symbol);
                assert!(definition.examples.is_empty());
                definition.api_token.clone()
            })
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(tokens.len(), 20);
    }

    #[test]
    fn hidden_contract_cases_are_evaluable() {
        let sets = generate_task_sets(23);
        for (index, task) in sets.blind.iter().enumerate() {
            let cases = generate_property_cases(&task.visible, 23 ^ index as u64);
            for case in &cases {
                evaluate_contract(&task.visible, case).expect("contract evaluates");
            }
        }
    }
}
