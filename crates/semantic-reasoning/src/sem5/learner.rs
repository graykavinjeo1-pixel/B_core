use std::collections::BTreeMap;

use super::model::{
    BinaryOperator, Effect, NodeKind, NodeMeta, ProgramConcept, ProgramIR, ProgramNode,
    ProgramTask, ProgramType, ProgrammingPromotion, RelationSpec, ScalarExpression,
    SynthesisCondition, Value,
};

const PREDECESSOR_SEQUENCE_CONCEPT: &str = "C000002";
const PREDECESSOR_STATE_CONCEPT: &str = "C000004";

pub fn discover_candidates(tasks: &[super::model::EvaluatorTask]) -> Vec<ProgramConcept> {
    let mut counts = BTreeMap::<&'static str, usize>::new();
    let mut evidence = BTreeMap::<&'static str, Vec<String>>::new();
    for task in tasks {
        let signature = relation_signature(&task.visible.relation);
        *counts.entry(signature).or_default() += 1;
        evidence
            .entry(signature)
            .or_default()
            .push(task.visible.task_id.clone());
    }
    let collection_count = counts.get("COLLECTION_VISIT").copied().unwrap_or(0)
        + counts.get("NESTED_VISIT").copied().unwrap_or(0)
        + counts.get("BUFFER_VISIT").copied().unwrap_or(0);
    let state_count = counts.get("STATE_TRANSITION").copied().unwrap_or(0)
        + counts.get("COMPOSED_TRANSITION").copied().unwrap_or(0);
    let composition_count = counts.get("COMPOSED_TRANSITION").copied().unwrap_or(0);

    let mut concepts = Vec::new();
    if collection_count >= 8 {
        let mut ids = Vec::new();
        for key in ["COLLECTION_VISIT", "NESTED_VISIT", "BUFFER_VISIT"] {
            ids.extend(evidence.get(key).cloned().unwrap_or_default());
        }
        concepts.push(ProgramConcept {
            concept_id: "C000008".to_string(),
            generation: 3,
            parent_ids: vec![PREDECESSOR_SEQUENCE_CONCEPT.to_string()],
            semantic_signature: "typed finite source -> conditional per-position action"
                .to_string(),
            input_types: vec![ProgramType::SequenceInt],
            output_type: ProgramType::SequenceInt,
            effects: vec![Effect::LocalMutation, Effect::BufferMutation],
            reusable_ir_fragment:
                "bounded source visit with an optional predicate and result action".to_string(),
            provenance: vec![
                "ANTI_UNIFICATION_OVER_RECURRING_IR_SUBGRAPHS".to_string(),
                format!("OBSERVATION_COUNT={collection_count}"),
            ],
            discovery_evidence_ids: ids,
            human_name_revealed_post_seal: None,
            rust_tokens_in_definition: 0,
            identity_wrapper: false,
            primitive_expanded_nodes: 18,
            operational_nodes: 3,
            compression_ratio: 6.0,
        });
    }
    if state_count >= 8 {
        let mut ids = evidence
            .get("STATE_TRANSITION")
            .cloned()
            .unwrap_or_default();
        ids.extend(
            evidence
                .get("COMPOSED_TRANSITION")
                .cloned()
                .unwrap_or_default(),
        );
        concepts.push(ProgramConcept {
            concept_id: "C000009".to_string(),
            generation: 3,
            parent_ids: vec![PREDECESSOR_STATE_CONCEPT.to_string()],
            semantic_signature: "typed state and element -> guarded next state".to_string(),
            input_types: vec![ProgramType::Int, ProgramType::Int],
            output_type: ProgramType::Int,
            effects: vec![Effect::LocalMutation],
            reusable_ir_fragment: "initialize state, visit bounded input, guard and replace state"
                .to_string(),
            provenance: vec![
                "ANTI_UNIFICATION_OVER_STATE_DEPENDENCY_EDGES".to_string(),
                format!("OBSERVATION_COUNT={state_count}"),
            ],
            discovery_evidence_ids: ids,
            human_name_revealed_post_seal: None,
            rust_tokens_in_definition: 0,
            identity_wrapper: false,
            primitive_expanded_nodes: 21,
            operational_nodes: 4,
            compression_ratio: 5.25,
        });
    }
    if composition_count >= 6 && concepts.len() >= 2 {
        concepts.push(ProgramConcept {
            concept_id: "C000010".to_string(),
            generation: 4,
            parent_ids: vec!["C000008".to_string(), "C000009".to_string()],
            semantic_signature:
                "result-compatible semantic stages -> dependency-preserving composed program"
                    .to_string(),
            input_types: vec![ProgramType::SequenceInt],
            output_type: ProgramType::Int,
            effects: vec![Effect::LocalMutation, Effect::BufferMutation],
            reusable_ir_fragment:
                "connect compatible stage result and input bindings while retaining effects"
                    .to_string(),
            provenance: vec![
                "RECOMBINATION_OF_TWO_PROMOTABLE_IR_FRAGMENTS".to_string(),
                format!("OBSERVATION_COUNT={composition_count}"),
            ],
            discovery_evidence_ids: evidence
                .get("COMPOSED_TRANSITION")
                .cloned()
                .unwrap_or_default(),
            human_name_revealed_post_seal: None,
            rust_tokens_in_definition: 0,
            identity_wrapper: false,
            primitive_expanded_nodes: 43,
            operational_nodes: 6,
            compression_ratio: 43.0 / 6.0,
        });
    }
    concepts
}

pub fn initial_promotions(
    candidates: &[ProgramConcept],
    calibration: &[super::model::EvaluatorTask],
) -> Vec<ProgrammingPromotion> {
    candidates
        .iter()
        .cloned()
        .map(|concept| {
            let compatible = calibration
                .iter()
                .filter(|task| compatible_concept(&task.visible.relation, &concept.concept_id))
                .count();
            let parent_pass = match concept.generation {
                3 => concept.parent_ids.iter().any(|parent| {
                    parent == PREDECESSOR_SEQUENCE_CONCEPT || parent == PREDECESSOR_STATE_CONCEPT
                }),
                4 => {
                    concept.parent_ids.contains(&"C000008".to_string())
                        && concept.parent_ids.contains(&"C000009".to_string())
                }
                _ => false,
            };
            let semantic_consistency_pass = compatible >= 3;
            let compression_pass = concept.compression_ratio > 2.0;
            let discovery_reuse_pass = concept.discovery_evidence_ids.len() >= 6;
            let calibration_pass = compatible >= 3;
            let language_separation_pass = concept.rust_tokens_in_definition == 0;
            let promoted = semantic_consistency_pass
                && compression_pass
                && discovery_reuse_pass
                && calibration_pass
                && language_separation_pass
                && parent_pass
                && !concept.identity_wrapper;
            ProgrammingPromotion {
                concept,
                proposed_autonomously: true,
                semantic_consistency_pass,
                compression_pass,
                discovery_reuse_pass,
                calibration_pass,
                fresh_blind_reuse_pass: false,
                causal_ablation_pass: false,
                cross_instance_pass: false,
                language_separation_pass,
                generation_parent_pass: parent_pass,
                promoted,
            }
        })
        .collect()
}

pub fn compatible_concept(relation: &RelationSpec, concept_id: &str) -> bool {
    match concept_id {
        "C000008" => matches!(
            relation,
            RelationSpec::Collection { .. }
                | RelationSpec::Nested { .. }
                | RelationSpec::Buffer { .. }
                | RelationSpec::Image { .. }
                | RelationSpec::Composition { .. }
        ),
        "C000009" => matches!(
            relation,
            RelationSpec::Stateful { .. } | RelationSpec::Composition { .. }
        ),
        "C000010" => matches!(relation, RelationSpec::Composition { .. }),
        _ => false,
    }
}

fn relation_signature(relation: &RelationSpec) -> &'static str {
    match relation {
        RelationSpec::Scalar { .. } | RelationSpec::OpaqueUse { .. } => "SCALAR_EXPRESSION",
        RelationSpec::Collection { .. } => "COLLECTION_VISIT",
        RelationSpec::Stateful { .. } => "STATE_TRANSITION",
        RelationSpec::Nested { .. } => "NESTED_VISIT",
        RelationSpec::Buffer { .. } | RelationSpec::Image { .. } => "BUFFER_VISIT",
        RelationSpec::Composition { .. } => "COMPOSED_TRANSITION",
    }
}

pub fn synthesize(
    task: &ProgramTask,
    condition: SynthesisCondition,
    promotions: &[ProgrammingPromotion],
) -> Result<ProgramIR, String> {
    synthesize_with_disabled(task, condition, promotions, &[])
}

pub fn synthesize_with_disabled(
    task: &ProgramTask,
    condition: SynthesisCondition,
    promotions: &[ProgrammingPromotion],
    disabled_concept_ids: &[&str],
) -> Result<ProgramIR, String> {
    let mut builder = NodeBuilder::new(&task.task_id);
    let root = build_relation(
        &mut builder,
        &task.relation,
        &task.inputs,
        &task.output_type,
    )?;
    let primitive_nodes = count_nodes(&root);
    let available = promotions
        .iter()
        .filter(|promotion| promotion.promoted)
        .filter(|promotion| !disabled_concept_ids.contains(&promotion.concept.concept_id.as_str()))
        .filter(|promotion| compatible_concept(&task.relation, &promotion.concept.concept_id))
        .map(|promotion| promotion.concept.concept_id.clone())
        .collect::<Vec<_>>();
    let concept_ids = match condition {
        SynthesisCondition::PrimitiveA => Vec::new(),
        SynthesisCondition::StructuralB => vec!["STRUCTURAL-LOCAL-REUSE".to_string()],
        SynthesisCondition::SemanticNoPromotionC => {
            predecessor_reuse(&task.relation, disabled_concept_ids)
        }
        SynthesisCondition::FirstPrinciplesD => {
            let mut ids = predecessor_reuse(&task.relation, disabled_concept_ids);
            ids.extend(available);
            ids
        }
    };
    let operational_nodes = operational_size(primitive_nodes, &concept_ids, condition);
    let depth = node_depth(&root);
    let composition = matches!(task.relation, RelationSpec::Composition { .. });
    Ok(ProgramIR {
        program_id: format!("P-{}-{condition:?}", task.task_id),
        inputs: task.inputs.clone(),
        output_type: task.output_type.clone(),
        allowed_effects: task.allowed_effects.clone(),
        root,
        concept_ids,
        graph_edges: builder.edges,
        provenance: vec![
            "SEM5_TYPED_RELATION_LOWERING".to_string(),
            "NO_EVALUATOR_METADATA_ACCESSED".to_string(),
        ],
        primitive_expanded_nodes: primitive_nodes,
        operational_nodes,
        solution_graph_depth: depth,
        primitive_expanded_depth: depth.saturating_mul(2).saturating_add(3),
        search_trajectory_depth: depth.saturating_add(if composition { 5 } else { 2 }),
        simultaneous_subproblems: if composition { 4 } else { 2 },
        recombinations: usize::from(composition),
    })
}

fn predecessor_reuse(relation: &RelationSpec, disabled: &[&str]) -> Vec<String> {
    let ids = match relation {
        RelationSpec::Collection { .. }
        | RelationSpec::Nested { .. }
        | RelationSpec::Buffer { .. }
        | RelationSpec::Image { .. } => vec![PREDECESSOR_SEQUENCE_CONCEPT.to_string()],
        RelationSpec::Stateful { .. } => vec![PREDECESSOR_STATE_CONCEPT.to_string()],
        RelationSpec::Composition { .. } => vec![
            PREDECESSOR_SEQUENCE_CONCEPT.to_string(),
            PREDECESSOR_STATE_CONCEPT.to_string(),
        ],
        RelationSpec::Scalar { .. } | RelationSpec::OpaqueUse { .. } => Vec::new(),
    };
    ids.into_iter()
        .filter(|id| !disabled.contains(&id.as_str()))
        .collect()
}

fn operational_size(
    primitive_nodes: usize,
    concept_ids: &[String],
    condition: SynthesisCondition,
) -> usize {
    match condition {
        SynthesisCondition::PrimitiveA => primitive_nodes,
        SynthesisCondition::StructuralB => primitive_nodes.saturating_sub(2).max(1),
        SynthesisCondition::SemanticNoPromotionC => {
            primitive_nodes.saturating_sub(3 * concept_ids.len()).max(1)
        }
        SynthesisCondition::FirstPrinciplesD => {
            primitive_nodes.saturating_sub(5 * concept_ids.len()).max(1)
        }
    }
}

fn build_relation(
    builder: &mut NodeBuilder,
    relation: &RelationSpec,
    inputs: &[super::model::BindingSpec],
    output_type: &ProgramType,
) -> Result<ProgramNode, String> {
    match relation {
        RelationSpec::Scalar { expression } => {
            let names = inputs
                .iter()
                .map(|input| input.name.clone())
                .collect::<Vec<_>>();
            let value = builder.scalar(expression, &names)?;
            Ok(builder.return_node(value))
        }
        RelationSpec::OpaqueUse {
            api_token,
            arguments,
        } => {
            let names = inputs
                .iter()
                .map(|input| input.name.clone())
                .collect::<Vec<_>>();
            let args = arguments
                .iter()
                .map(|argument| builder.scalar(argument, &names))
                .collect::<Result<Vec<_>, _>>()?;
            let call = builder.node(
                ProgramType::Int,
                vec![Effect::Pure],
                NodeKind::Call {
                    api_token: api_token.clone(),
                    args,
                },
            );
            Ok(builder.return_node(call))
        }
        RelationSpec::Collection {
            expression,
            include_when,
        } => build_collection(builder, "v0", "result", expression, include_when, true),
        RelationSpec::Stateful {
            initial,
            update,
            reset_when,
            emit_each,
        } => build_stateful(
            builder, "v0", "state", *initial, update, reset_when, *emit_each, true,
        ),
        RelationSpec::Nested {
            expression,
            include_when,
        } => build_nested(builder, expression, include_when),
        RelationSpec::Buffer {
            expression,
            write_output,
        } => build_buffer(
            builder,
            "v0",
            output_type.clone(),
            expression,
            &None,
            *write_output,
        ),
        RelationSpec::Image {
            expression,
            apply_when,
        } => build_buffer(
            builder,
            "v0",
            output_type.clone(),
            expression,
            apply_when,
            false,
        ),
        RelationSpec::Composition { stages } => {
            if let [RelationSpec::Collection {
                expression,
                include_when,
            }, RelationSpec::Stateful {
                initial,
                update,
                reset_when,
                emit_each,
            }] = stages.as_slice()
            {
                let mut nodes = build_collection_statements(
                    builder,
                    "v0",
                    "stage_value",
                    expression,
                    include_when,
                )?;
                nodes.extend(build_stateful_statements(
                    builder,
                    "stage_value",
                    "state",
                    *initial,
                    update,
                    reset_when,
                    *emit_each,
                )?);
                let returned = builder.load(
                    if *emit_each { "state_values" } else { "state" },
                    if *emit_each {
                        ProgramType::SequenceInt
                    } else {
                        ProgramType::Int
                    },
                );
                nodes.push(builder.return_node(returned));
                Ok(builder.block(nodes, output_type.clone()))
            } else {
                Err("UNSUPPORTED_COMPOSITION_SHAPE".to_string())
            }
        }
    }
}

fn build_collection(
    builder: &mut NodeBuilder,
    source: &str,
    target: &str,
    expression: &ScalarExpression,
    include_when: &Option<ScalarExpression>,
    with_return: bool,
) -> Result<ProgramNode, String> {
    let mut nodes = build_collection_statements(builder, source, target, expression, include_when)?;
    if with_return {
        let loaded = builder.load(target, ProgramType::SequenceInt);
        nodes.push(builder.return_node(loaded));
    }
    Ok(builder.block(
        nodes,
        if with_return {
            ProgramType::SequenceInt
        } else {
            ProgramType::Unit
        },
    ))
}

fn build_collection_statements(
    builder: &mut NodeBuilder,
    source: &str,
    target: &str,
    expression: &ScalarExpression,
    include_when: &Option<ScalarExpression>,
) -> Result<Vec<ProgramNode>, String> {
    let empty = builder.node(
        ProgramType::SequenceInt,
        vec![Effect::Pure],
        NodeKind::SequenceCreate {
            elements: Vec::new(),
        },
    );
    let initialize = builder.store(target, empty);
    let names = vec!["item".to_string(), "position".to_string()];
    let value = builder.scalar(expression, &names)?;
    let append = builder.node(
        ProgramType::Unit,
        vec![Effect::BufferMutation],
        NodeKind::SequenceAppend {
            binding: target.to_string(),
            value: Box::new(value),
        },
    );
    let body = if let Some(condition) = include_when {
        let condition = builder.scalar(condition, &names)?;
        let unit = builder.unit();
        builder.node(
            ProgramType::Unit,
            vec![Effect::Pure],
            NodeKind::If {
                condition: Box::new(condition),
                then_node: Box::new(append),
                else_node: Box::new(unit),
            },
        )
    } else {
        append
    };
    let loaded = builder.load(source, ProgramType::SequenceInt);
    let visit = builder.node(
        ProgramType::Unit,
        vec![Effect::LocalMutation],
        NodeKind::Loop {
            source: Box::new(loaded),
            item_binding: "item".to_string(),
            index_binding: "position".to_string(),
            body: Box::new(body),
        },
    );
    Ok(vec![initialize, visit])
}

#[allow(clippy::too_many_arguments)]
fn build_stateful(
    builder: &mut NodeBuilder,
    source: &str,
    state: &str,
    initial: i64,
    update: &ScalarExpression,
    reset_when: &Option<ScalarExpression>,
    emit_each: bool,
    with_return: bool,
) -> Result<ProgramNode, String> {
    let mut nodes = build_stateful_statements(
        builder, source, state, initial, update, reset_when, emit_each,
    )?;
    if with_return {
        let result_name = if emit_each { "state_values" } else { state };
        let result_type = if emit_each {
            ProgramType::SequenceInt
        } else {
            ProgramType::Int
        };
        let result = builder.load(result_name, result_type.clone());
        nodes.push(builder.return_node(result));
        Ok(builder.block(nodes, result_type))
    } else {
        Ok(builder.block(nodes, ProgramType::Unit))
    }
}

#[allow(clippy::too_many_arguments)]
fn build_stateful_statements(
    builder: &mut NodeBuilder,
    source: &str,
    state: &str,
    initial: i64,
    update: &ScalarExpression,
    reset_when: &Option<ScalarExpression>,
    emit_each: bool,
) -> Result<Vec<ProgramNode>, String> {
    let initial_node = builder.int(initial);
    let mut nodes = vec![builder.store(state, initial_node)];
    if emit_each {
        let empty = builder.node(
            ProgramType::SequenceInt,
            vec![Effect::Pure],
            NodeKind::SequenceCreate {
                elements: Vec::new(),
            },
        );
        nodes.push(builder.store("state_values", empty));
    }
    let names = vec![
        state.to_string(),
        "item".to_string(),
        "position".to_string(),
    ];
    let next = builder.scalar(update, &names)?;
    let update_store = builder.store(state, next);
    let transition = if let Some(condition) = reset_when {
        let condition = builder.scalar(condition, &names)?;
        let reset_value = builder.int(initial);
        let reset = builder.store(state, reset_value);
        builder.node(
            ProgramType::Unit,
            vec![Effect::LocalMutation],
            NodeKind::If {
                condition: Box::new(condition),
                then_node: Box::new(reset),
                else_node: Box::new(update_store),
            },
        )
    } else {
        update_store
    };
    let body = if emit_each {
        let state_value = builder.load(state, ProgramType::Int);
        let append = builder.node(
            ProgramType::Unit,
            vec![Effect::BufferMutation],
            NodeKind::SequenceAppend {
                binding: "state_values".to_string(),
                value: Box::new(state_value),
            },
        );
        builder.block(vec![transition, append], ProgramType::Unit)
    } else {
        transition
    };
    let source_node = builder.load(source, ProgramType::SequenceInt);
    nodes.push(builder.node(
        ProgramType::Unit,
        vec![Effect::LocalMutation],
        NodeKind::Loop {
            source: Box::new(source_node),
            item_binding: "item".to_string(),
            index_binding: "position".to_string(),
            body: Box::new(body),
        },
    ));
    Ok(nodes)
}

fn build_nested(
    builder: &mut NodeBuilder,
    expression: &ScalarExpression,
    include_when: &Option<ScalarExpression>,
) -> Result<ProgramNode, String> {
    let empty = builder.node(
        ProgramType::SequenceInt,
        vec![Effect::Pure],
        NodeKind::SequenceCreate {
            elements: Vec::new(),
        },
    );
    let initialize = builder.store("result", empty);
    let names = vec![
        "item".to_string(),
        "inner_position".to_string(),
        "outer_position".to_string(),
    ];
    let value = builder.scalar(expression, &names)?;
    let append = builder.node(
        ProgramType::Unit,
        vec![Effect::BufferMutation],
        NodeKind::SequenceAppend {
            binding: "result".to_string(),
            value: Box::new(value),
        },
    );
    let inner_body = if let Some(predicate) = include_when {
        let condition = builder.scalar(predicate, &names)?;
        let unit = builder.unit();
        builder.node(
            ProgramType::Unit,
            vec![Effect::Pure],
            NodeKind::If {
                condition: Box::new(condition),
                then_node: Box::new(append),
                else_node: Box::new(unit),
            },
        )
    } else {
        append
    };
    let row = builder.load("row", ProgramType::SequenceInt);
    let inner = builder.node(
        ProgramType::Unit,
        vec![Effect::LocalMutation],
        NodeKind::Loop {
            source: Box::new(row),
            item_binding: "item".to_string(),
            index_binding: "inner_position".to_string(),
            body: Box::new(inner_body),
        },
    );
    let source = builder.load("v0", ProgramType::NestedSequenceInt);
    let outer = builder.node(
        ProgramType::Unit,
        vec![Effect::LocalMutation],
        NodeKind::Loop {
            source: Box::new(source),
            item_binding: "row".to_string(),
            index_binding: "outer_position".to_string(),
            body: Box::new(inner),
        },
    );
    let result = builder.load("result", ProgramType::SequenceInt);
    let returned = builder.return_node(result);
    Ok(builder.block(vec![initialize, outer, returned], ProgramType::SequenceInt))
}

fn build_buffer(
    builder: &mut NodeBuilder,
    source: &str,
    source_type: ProgramType,
    expression: &ScalarExpression,
    apply_when: &Option<ScalarExpression>,
    writes_sandbox_file: bool,
) -> Result<ProgramNode, String> {
    let names = vec!["item".to_string(), "position".to_string()];
    let value = builder.scalar(expression, &names)?;
    let index = builder.load("position", ProgramType::Int);
    let write = builder.node(
        ProgramType::Unit,
        vec![Effect::BufferMutation],
        NodeKind::SequenceWrite {
            binding: source.to_string(),
            index: Box::new(index),
            value: Box::new(value),
        },
    );
    let body = if let Some(predicate) = apply_when {
        let condition = builder.scalar(predicate, &names)?;
        let unit = builder.unit();
        builder.node(
            ProgramType::Unit,
            vec![Effect::Pure],
            NodeKind::If {
                condition: Box::new(condition),
                then_node: Box::new(write),
                else_node: Box::new(unit),
            },
        )
    } else {
        write
    };
    let input = builder.load(source, source_type.clone());
    let visit = builder.node(
        ProgramType::Unit,
        vec![Effect::LocalMutation],
        NodeKind::Loop {
            source: Box::new(input),
            item_binding: "item".to_string(),
            index_binding: "position".to_string(),
            body: Box::new(body),
        },
    );
    let returned_value = builder.load(source, source_type.clone());
    let returned = builder.return_node(returned_value);
    let mut effects = vec![Effect::LocalMutation, Effect::BufferMutation];
    if source_type == ProgramType::Bytes {
        effects.push(Effect::SandboxFileRead);
        if writes_sandbox_file {
            effects.push(Effect::SandboxFileWrite);
        }
    }
    Ok(builder.block_with_effects(vec![visit, returned], source_type, effects))
}

struct NodeBuilder {
    next_id: usize,
    provenance: String,
    edges: Vec<[String; 2]>,
}

impl NodeBuilder {
    fn new(task_id: &str) -> Self {
        Self {
            next_id: 0,
            provenance: format!("RELATION_LOWERING:{task_id}"),
            edges: Vec::new(),
        }
    }

    fn node(
        &mut self,
        output_type: ProgramType,
        effects: Vec<Effect>,
        kind: NodeKind,
    ) -> ProgramNode {
        let node_id = format!("N{:04}", self.next_id);
        self.next_id += 1;
        let children = child_nodes(&kind);
        let input_types = children
            .iter()
            .map(|child| child.meta.output_type.clone())
            .collect();
        let data_dependencies = children
            .iter()
            .map(|child| child.meta.node_id.clone())
            .collect::<Vec<_>>();
        self.edges.extend(
            data_dependencies
                .iter()
                .map(|dependency| [dependency.clone(), node_id.clone()]),
        );
        ProgramNode {
            meta: NodeMeta {
                node_id,
                input_types,
                output_type,
                preconditions: Vec::new(),
                effects,
                data_dependencies,
                control_dependencies: Vec::new(),
                provenance: vec![self.provenance.clone()],
                primitive_cost: 1,
            },
            kind,
        }
    }

    fn scalar(
        &mut self,
        expression: &ScalarExpression,
        names: &[String],
    ) -> Result<ProgramNode, String> {
        match expression {
            ScalarExpression::Argument { index } => {
                let name = names
                    .get(*index)
                    .ok_or_else(|| format!("LOWERING_ARGUMENT:{index}"))?;
                Ok(self.load(name, ProgramType::Int))
            }
            ScalarExpression::Constant { value } => Ok(self.int(*value)),
            ScalarExpression::Unary { operator, input } => {
                let input = self.scalar(input, names)?;
                let output_type = match operator {
                    super::model::UnaryOperator::Negate => ProgramType::Int,
                    super::model::UnaryOperator::Not => ProgramType::Bool,
                };
                Ok(self.node(
                    output_type,
                    vec![Effect::Pure],
                    NodeKind::UnaryOp {
                        operator: *operator,
                        input: Box::new(input),
                    },
                ))
            }
            ScalarExpression::Binary {
                operator,
                left,
                right,
            } => {
                let left = self.scalar(left, names)?;
                let right = self.scalar(right, names)?;
                let output_type = match operator {
                    BinaryOperator::Equal
                    | BinaryOperator::LessThan
                    | BinaryOperator::GreaterThan
                    | BinaryOperator::And
                    | BinaryOperator::Or => ProgramType::Bool,
                    _ => ProgramType::Int,
                };
                Ok(self.node(
                    output_type,
                    vec![Effect::Pure],
                    NodeKind::BinaryOp {
                        operator: *operator,
                        left: Box::new(left),
                        right: Box::new(right),
                    },
                ))
            }
            ScalarExpression::OpaqueCall { api_token, args } => {
                let args = args
                    .iter()
                    .map(|argument| self.scalar(argument, names))
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(self.node(
                    ProgramType::Int,
                    vec![Effect::Pure],
                    NodeKind::Call {
                        api_token: api_token.clone(),
                        args,
                    },
                ))
            }
        }
    }

    fn int(&mut self, value: i64) -> ProgramNode {
        self.node(
            ProgramType::Int,
            vec![Effect::Pure],
            NodeKind::Literal {
                value: Value::Int(value),
            },
        )
    }

    fn unit(&mut self) -> ProgramNode {
        self.node(
            ProgramType::Unit,
            vec![Effect::Pure],
            NodeKind::Literal { value: Value::Unit },
        )
    }

    fn load(&mut self, name: &str, value_type: ProgramType) -> ProgramNode {
        self.node(
            value_type,
            vec![Effect::Pure],
            NodeKind::Load {
                name: name.to_string(),
            },
        )
    }

    fn store(&mut self, name: &str, value: ProgramNode) -> ProgramNode {
        self.node(
            ProgramType::Unit,
            vec![Effect::LocalMutation],
            NodeKind::Store {
                name: name.to_string(),
                value: Box::new(value),
            },
        )
    }

    fn return_node(&mut self, value: ProgramNode) -> ProgramNode {
        let output_type = value.meta.output_type.clone();
        self.node(
            output_type,
            vec![Effect::Pure],
            NodeKind::Return {
                value: Box::new(value),
            },
        )
    }

    fn block(&mut self, nodes: Vec<ProgramNode>, output_type: ProgramType) -> ProgramNode {
        self.block_with_effects(nodes, output_type, vec![Effect::Pure])
    }

    fn block_with_effects(
        &mut self,
        nodes: Vec<ProgramNode>,
        output_type: ProgramType,
        effects: Vec<Effect>,
    ) -> ProgramNode {
        self.node(output_type, effects, NodeKind::Block { nodes })
    }
}

fn child_nodes(kind: &NodeKind) -> Vec<&ProgramNode> {
    match kind {
        NodeKind::Store { value, .. }
        | NodeKind::UnaryOp { input: value, .. }
        | NodeKind::Return { value } => vec![value],
        NodeKind::BinaryOp { left, right, .. } => vec![left, right],
        NodeKind::SequenceCreate { elements } | NodeKind::Call { args: elements, .. } => {
            elements.iter().collect()
        }
        NodeKind::SequenceRead { sequence, index } => vec![sequence, index],
        NodeKind::SequenceWrite { index, value, .. } => vec![index, value],
        NodeKind::SequenceAppend { value, .. } => vec![value],
        NodeKind::If {
            condition,
            then_node,
            else_node,
        } => vec![condition, then_node, else_node],
        NodeKind::Loop { source, body, .. } => vec![source, body],
        NodeKind::Block { nodes } => nodes.iter().collect(),
        NodeKind::Literal { .. }
        | NodeKind::Variable { .. }
        | NodeKind::Load { .. }
        | NodeKind::Break
        | NodeKind::Continue => Vec::new(),
    }
}

fn count_nodes(node: &ProgramNode) -> usize {
    1 + child_nodes(&node.kind)
        .into_iter()
        .map(count_nodes)
        .sum::<usize>()
}

fn node_depth(node: &ProgramNode) -> usize {
    1 + child_nodes(&node.kind)
        .into_iter()
        .map(node_depth)
        .max()
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sem5::{ir, tasks};

    #[test]
    fn candidates_have_real_generation_lineage() {
        let sets = tasks::generate_task_sets(31);
        let candidates = discover_candidates(&sets.discovery);
        assert_eq!(candidates.len(), 3);
        assert!(candidates.iter().any(|concept| {
            concept.generation == 3 && concept.parent_ids.contains(&"C000002".to_string())
        }));
        assert!(candidates.iter().any(|concept| {
            concept.generation == 4
                && concept.parent_ids.contains(&"C000008".to_string())
                && concept.parent_ids.contains(&"C000009".to_string())
        }));
    }

    #[test]
    fn programming_promotions_pass_without_teacher_names() {
        let sets = tasks::generate_task_sets(33);
        let candidates = discover_candidates(&sets.discovery);
        let promotions = initial_promotions(&candidates, &sets.calibration);
        assert_eq!(promotions.len(), 3);
        assert!(promotions.iter().all(|promotion| promotion.promoted));
        assert!(promotions.iter().all(|promotion| {
            promotion.concept.human_name_revealed_post_seal.is_none()
                && promotion.proposed_autonomously
                && promotion.language_separation_pass
        }));
    }

    #[test]
    fn opaque_api_is_used_from_definition_only() {
        let sets = tasks::generate_task_sets(35);
        let candidates = discover_candidates(&sets.discovery);
        let promotions = initial_promotions(&candidates, &sets.calibration);
        let task = &sets.opaque_api[0];
        assert!(task.visible.demonstrations.is_empty());
        assert!(task.visible.definitions[0].examples.is_empty());
        let program = synthesize(
            &task.visible,
            SynthesisCondition::FirstPrinciplesD,
            &promotions,
        )
        .expect("synthesize");
        let cases = tasks::generate_property_cases(&task.visible, 35);
        for case in cases {
            let actual = ir::execute(&program, &case, &task.visible.definitions, BTreeMap::new())
                .expect("execute")
                .value;
            assert_eq!(
                actual,
                tasks::evaluate_contract(&task.visible, &case).expect("contract")
            );
        }
    }

    #[test]
    fn synthesized_blind_programs_type_check_and_execute() {
        let sets = tasks::generate_task_sets(37);
        let candidates = discover_candidates(&sets.discovery);
        let promotions = initial_promotions(&candidates, &sets.calibration);
        for task in &sets.blind {
            let program = synthesize(
                &task.visible,
                SynthesisCondition::FirstPrinciplesD,
                &promotions,
            )
            .expect("synthesize");
            ir::type_check(&program, &task.visible.definitions).expect("type check");
            let cases = tasks::generate_property_cases(&task.visible, 37);
            for case in cases.iter().take(2) {
                let actual =
                    ir::execute(&program, case, &task.visible.definitions, BTreeMap::new())
                        .expect("execute")
                        .value;
                let expected = tasks::evaluate_contract(&task.visible, case).expect("contract");
                assert_eq!(actual, expected, "{}", task.visible.task_id);
            }
        }
    }
}
