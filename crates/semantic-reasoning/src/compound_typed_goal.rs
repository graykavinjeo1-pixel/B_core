//! Evidence-bound functional composition of typed behavior goals.
//!
//! A compound goal is admitted only when a producer's public postimage can be
//! joined to a causally relevant consumer operand. The resulting observations
//! are a relational join of the two original public contracts, not examples
//! invented by the composer. This keeps the existing SEM-5 synthesizer and
//! verifier authoritative while allowing it to materialize a genuinely new
//! single ProgramIR for the composed behavior.

use std::collections::{BTreeMap, BTreeSet};

use crate::self_repair_contract::sha256;
use crate::sem5::{
    model::{Effect, ProgramType, Value},
    typed_mechanism::{
        validate_typed_mechanism_synthesis_goal, SourceOperandIR, TypedMechanismObservationIR,
        TypedMechanismSynthesisGoalIR, TYPED_MECHANISM_SYNTHESIS_GOAL_SCHEMA,
    },
};

pub const COMPOUND_TYPED_GOAL_SCHEMA_REVISION: u64 = 1;
pub const MAX_COMPOUND_TYPED_GOAL_COMPONENTS: usize = 3;
pub const MAX_COMPOUND_TYPED_GOAL_CANDIDATES: usize = 32;
const MAX_COMPOUND_PUBLIC_OBSERVATIONS: usize = 64;
const MAX_COMPOUND_OPERANDS: usize = 32;

#[derive(Debug, Clone)]
struct GoalState {
    goal: TypedMechanismSynthesisGoalIR,
    component_ids: BTreeSet<String>,
}

/// Derive bounded, executable compound contracts from independently grounded
/// typed goals. Smaller valid chains are returned first so a failed deeper
/// hypothesis cannot starve the minimal causally supported composition.
pub fn derive_compound_typed_behavior_goals(
    goals: &[TypedMechanismSynthesisGoalIR],
) -> Result<Vec<TypedMechanismSynthesisGoalIR>, String> {
    let mut bases = BTreeMap::new();
    let mut conflicting_ids = BTreeSet::new();
    for goal in goals {
        validate_typed_mechanism_synthesis_goal(goal)
            .map_err(|error| format!("COMPOUND_COMPONENT_INVALID:{error}"))?;
        if conflicting_ids.contains(&goal.goal_id) {
            continue;
        }
        if let Some(existing) = bases.get(&goal.goal_id) {
            if existing != goal {
                // A historical identity collision is not evidence that either
                // variant is the authoritative component. Quarantine only the
                // ambiguous identity; do not let one stale memory item stop
                // every unrelated valid composition in the supervisor.
                bases.remove(&goal.goal_id);
                conflicting_ids.insert(goal.goal_id.clone());
            }
            continue;
        }
        bases.insert(goal.goal_id.clone(), goal.clone());
    }
    if bases.len() < 2 {
        return Ok(Vec::new());
    }
    let base_states = bases
        .into_values()
        .map(|goal| GoalState {
            component_ids: BTreeSet::from([goal.goal_id.clone()]),
            goal,
        })
        .collect::<Vec<_>>();
    let mut frontier = base_states.clone();
    let mut admitted = BTreeMap::new();

    for _depth in 2..=MAX_COMPOUND_TYPED_GOAL_COMPONENTS {
        let mut next = BTreeMap::new();
        for state in &frontier {
            for base in &base_states {
                if !state.component_ids.is_disjoint(&base.component_ids) {
                    continue;
                }
                // A previously promoted compound may re-enter a later
                // plateau as one of the base goals. Reconstructing that same
                // semantic goal can give a frontier state the same goal id as
                // the retained base while their historical component-id sets
                // differ. Never self-wire identical semantic programs.
                if state.goal.goal_id == base.goal.goal_id {
                    continue;
                }
                for (producer, consumer) in [(&state.goal, &base.goal), (&base.goal, &state.goal)] {
                    for wire in &consumer.operands {
                        if wire.value_type != producer.output_type {
                            continue;
                        }
                        let Some(goal) = compose_over_operand(producer, consumer, wire)? else {
                            continue;
                        };
                        let mut component_ids = state.component_ids.clone();
                        component_ids.extend(base.component_ids.iter().cloned());
                        let candidate = GoalState {
                            goal: goal.clone(),
                            component_ids,
                        };
                        next.entry(goal.goal_id.clone()).or_insert(candidate);
                        admitted.entry(goal.goal_id.clone()).or_insert(goal);
                    }
                }
            }
        }
        if next.is_empty() || admitted.len() >= MAX_COMPOUND_TYPED_GOAL_CANDIDATES {
            break;
        }
        frontier = next.into_values().collect();
    }

    let mut results = admitted.into_values().collect::<Vec<_>>();
    results.sort_by(|left, right| {
        compound_depth(left)
            .cmp(&compound_depth(right))
            .then_with(|| {
                right
                    .public_observations
                    .len()
                    .cmp(&left.public_observations.len())
            })
            .then_with(|| left.goal_id.cmp(&right.goal_id))
    });
    results.truncate(MAX_COMPOUND_TYPED_GOAL_CANDIDATES);
    Ok(results)
}

fn compose_over_operand(
    producer: &TypedMechanismSynthesisGoalIR,
    consumer: &TypedMechanismSynthesisGoalIR,
    wire: &SourceOperandIR,
) -> Result<Option<TypedMechanismSynthesisGoalIR>, String> {
    if producer.goal_id == consumer.goal_id
        || producer.split != consumer.split
        || producer.output_type != wire.value_type
        || !pure_contract(producer)
        || !pure_contract(consumer)
        || !consumer_operand_is_causal(consumer, &wire.role)
    {
        return Ok(None);
    }

    let mut operands_by_identity = BTreeMap::<(String, ProgramType), SourceOperandIR>::new();
    let mut role_map = BTreeMap::<(String, String), String>::new();
    add_component_operands(producer, None, 0, &mut operands_by_identity, &mut role_map)?;
    add_component_operands(
        consumer,
        Some(&wire.role),
        1,
        &mut operands_by_identity,
        &mut role_map,
    )?;
    // A valid pair of bounded components can still exceed the shared SEM-5
    // operand budget after its public join. That is a rejected composition
    // candidate, not a daemon-fatal compiler invariant. Historical large
    // programs remain valid on their own and are simply not expanded further.
    if operands_by_identity.len() > MAX_COMPOUND_OPERANDS {
        return Ok(None);
    }

    let mut observations = BTreeMap::new();
    let mut producer_outputs = BTreeSet::new();
    let mut compound_outputs = BTreeSet::new();
    for producer_observation in &producer.public_observations {
        for consumer_observation in &consumer.public_observations {
            if consumer_observation.operands.get(&wire.role)
                != Some(&producer_observation.expected_postimage)
            {
                continue;
            }
            let Some(merged) = merge_observation_operands(
                producer,
                producer_observation,
                consumer,
                consumer_observation,
                &wire.role,
                &role_map,
            ) else {
                continue;
            };
            let observation = TypedMechanismObservationIR {
                operands: merged,
                expected_postimage: consumer_observation.expected_postimage.clone(),
            };
            let key = serde_json::to_vec(&observation)
                .map(|bytes| sha256(&bytes))
                .map_err(|error| format!("COMPOUND_OBSERVATION_SERIALIZE:{error}"))?;
            producer_outputs.insert(value_identity(&producer_observation.expected_postimage)?);
            compound_outputs.insert(value_identity(&observation.expected_postimage)?);
            observations.entry(key).or_insert(observation);
        }
    }
    if observations.len() < 3 || producer_outputs.len() < 2 || compound_outputs.len() < 2 {
        return Ok(None);
    }
    let public_observations = observations
        .into_values()
        .take(MAX_COMPOUND_PUBLIC_OBSERVATIONS)
        .collect::<Vec<_>>();

    let definitions = merge_definitions(producer, consumer)?;
    let components = compound_component_ids(producer, consumer);
    let identity_operands = operands_by_identity.values().cloned().collect::<Vec<_>>();
    let identity = serde_json::to_vec(&(
        COMPOUND_TYPED_GOAL_SCHEMA_REVISION,
        &components,
        &wire.role,
        &identity_operands,
        &public_observations,
    ))
    .map_err(|error| format!("COMPOUND_GOAL_SERIALIZE:{error}"))?;
    let goal_hash = sha256(&identity);
    let mut provenance = merge_strings(&producer.provenance, &consumer.provenance);
    provenance.extend(
        components
            .iter()
            .map(|component| format!("COMPOUND_COMPONENT_GOAL:{component}")),
    );
    provenance.push(format!(
        "COMPOUND_WIRING:{}->{}:{}",
        producer.goal_id, consumer.goal_id, wire.role
    ));
    provenance.push("COMPOUND_DERIVED_FROM_PUBLIC_OBSERVATION_JOIN".to_string());
    provenance.sort();
    provenance.dedup();

    let mut postconditions = merge_strings(&producer.postconditions, &consumer.postconditions);
    postconditions.push(format!(
        "feed the verified {} output into {}.{} and preserve the consumer postimage",
        producer.goal_id, consumer.goal_id, wire.role
    ));
    postconditions.sort();
    postconditions.dedup();
    let mut invariants = merge_strings(&producer.invariants, &consumer.invariants);
    invariants.extend([
        "compound examples are a lossless join of component public observations".to_string(),
        "the wired intermediate is not exposed as an operator-selected answer".to_string(),
    ]);
    invariants.sort();
    invariants.dedup();

    let request = TypedMechanismSynthesisGoalIR {
        schema: TYPED_MECHANISM_SYNTHESIS_GOAL_SCHEMA.to_string(),
        goal_id: format!("compound_{}", &goal_hash[..32]),
        split: producer.split,
        operands: operands_by_identity.into_values().collect(),
        output_type: consumer.output_type.clone(),
        definitions,
        allowed_effects: if producer.allowed_effects.contains(&Effect::Pure)
            || consumer.allowed_effects.contains(&Effect::Pure)
        {
            vec![Effect::Pure]
        } else {
            Vec::new()
        },
        preconditions: merge_strings(&producer.preconditions, &consumer.preconditions),
        postconditions,
        invariants,
        public_observations,
        require_conditional: producer.require_conditional
            || consumer.require_conditional
            || wire.value_type == ProgramType::Bool,
        max_expression_depth: producer
            .max_expression_depth
            .max(consumer.max_expression_depth)
            .saturating_add(1)
            .min(3),
        max_candidates: producer
            .max_candidates
            .saturating_add(consumer.max_candidates)
            .clamp(16, 1_024),
        provenance,
    };
    validate_typed_mechanism_synthesis_goal(&request)
        .map_err(|error| format!("COMPOUND_RESULT_INVALID:{error}"))?;
    Ok(Some(request))
}

fn add_component_operands(
    goal: &TypedMechanismSynthesisGoalIR,
    excluded_role: Option<&str>,
    component_index: usize,
    operands: &mut BTreeMap<(String, ProgramType), SourceOperandIR>,
    role_map: &mut BTreeMap<(String, String), String>,
) -> Result<(), String> {
    for operand in &goal.operands {
        if excluded_role == Some(operand.role.as_str()) {
            continue;
        }
        let identity = (operand.source.clone(), operand.value_type.clone());
        let role = if let Some(existing) = operands.get(&identity) {
            existing.role.clone()
        } else {
            let role = format!(
                "compound_{component_index}_{}_{:02}",
                identifier_fragment(&operand.role),
                operands.len()
            );
            let mut rebound = operand.clone();
            rebound.role = role.clone();
            operands.insert(identity, rebound);
            role
        };
        if role_map
            .insert((goal.goal_id.clone(), operand.role.clone()), role)
            .is_some()
        {
            return Err("COMPOUND_ROLE_MAPPING_DUPLICATE".to_string());
        }
    }
    Ok(())
}

fn merge_observation_operands(
    producer: &TypedMechanismSynthesisGoalIR,
    producer_observation: &TypedMechanismObservationIR,
    consumer: &TypedMechanismSynthesisGoalIR,
    consumer_observation: &TypedMechanismObservationIR,
    wire_role: &str,
    role_map: &BTreeMap<(String, String), String>,
) -> Option<BTreeMap<String, Value>> {
    let mut merged = BTreeMap::new();
    for (goal, observation, excluded) in [
        (producer, producer_observation, None),
        (consumer, consumer_observation, Some(wire_role)),
    ] {
        for (role, value) in &observation.operands {
            if excluded == Some(role.as_str()) {
                continue;
            }
            let rebound = role_map.get(&(goal.goal_id.clone(), role.clone()))?;
            if merged
                .get(rebound)
                .is_some_and(|existing| existing != value)
            {
                return None;
            }
            merged.insert(rebound.clone(), value.clone());
        }
    }
    Some(merged)
}

fn consumer_operand_is_causal(goal: &TypedMechanismSynthesisGoalIR, role: &str) -> bool {
    goal.public_observations
        .iter()
        .enumerate()
        .any(|(index, left)| {
            goal.public_observations
                .iter()
                .skip(index + 1)
                .any(|right| {
                    left.operands.get(role) != right.operands.get(role)
                        && left.expected_postimage != right.expected_postimage
                        && left
                            .operands
                            .iter()
                            .filter(|(candidate, _)| candidate.as_str() != role)
                            .all(|(candidate, value)| right.operands.get(candidate) == Some(value))
                })
        })
}

fn pure_contract(goal: &TypedMechanismSynthesisGoalIR) -> bool {
    goal.allowed_effects
        .iter()
        .all(|effect| *effect == Effect::Pure)
        && goal
            .definitions
            .iter()
            .all(|definition| definition.effect == Effect::Pure)
}

fn merge_definitions(
    left: &TypedMechanismSynthesisGoalIR,
    right: &TypedMechanismSynthesisGoalIR,
) -> Result<Vec<crate::sem5::model::ApiDefinition>, String> {
    let mut definitions = BTreeMap::new();
    for definition in left.definitions.iter().chain(&right.definitions) {
        if definitions
            .get(&definition.api_token)
            .is_some_and(|existing| *existing != *definition)
        {
            return Err(format!("COMPOUND_API_CONFLICT:{}", definition.api_token));
        }
        definitions.insert(definition.api_token.clone(), definition.clone());
    }
    Ok(definitions.into_values().collect())
}

fn compound_component_ids(
    producer: &TypedMechanismSynthesisGoalIR,
    consumer: &TypedMechanismSynthesisGoalIR,
) -> Vec<String> {
    let mut components = producer
        .provenance
        .iter()
        .chain(&consumer.provenance)
        .filter_map(|item| item.strip_prefix("COMPOUND_COMPONENT_GOAL:"))
        .map(str::to_string)
        .collect::<BTreeSet<_>>();
    if !producer.goal_id.starts_with("compound_") {
        components.insert(producer.goal_id.clone());
    }
    if !consumer.goal_id.starts_with("compound_") {
        components.insert(consumer.goal_id.clone());
    }
    components.into_iter().collect()
}

fn compound_depth(goal: &TypedMechanismSynthesisGoalIR) -> usize {
    goal.provenance
        .iter()
        .filter(|item| item.starts_with("COMPOUND_COMPONENT_GOAL:"))
        .count()
}

fn merge_strings(left: &[String], right: &[String]) -> Vec<String> {
    left.iter()
        .chain(right)
        .cloned()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn identifier_fragment(value: &str) -> String {
    let fragment = value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || character == '_' {
                character
            } else {
                '_'
            }
        })
        .collect::<String>();
    if fragment
        .chars()
        .next()
        .is_some_and(|character| character.is_ascii_alphabetic() || character == '_')
    {
        fragment
    } else {
        format!("role_{fragment}")
    }
}

fn value_identity(value: &Value) -> Result<String, String> {
    serde_json::to_vec(value)
        .map(|bytes| sha256(&bytes))
        .map_err(|error| format!("COMPOUND_VALUE_SERIALIZE:{error}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::integrated_development::execute_typed_behavior_goal_canary;
    use crate::sem5::model::DataSplit;

    fn operand(role: &str, value_type: ProgramType) -> SourceOperandIR {
        SourceOperandIR {
            role: role.to_string(),
            source: format!("state.{role}"),
            value_type,
        }
    }

    fn observation(values: &[(&str, Value)], expected: Value) -> TypedMechanismObservationIR {
        TypedMechanismObservationIR {
            operands: values
                .iter()
                .map(|(role, value)| ((*role).to_string(), value.clone()))
                .collect(),
            expected_postimage: expected,
        }
    }

    fn producer() -> TypedMechanismSynthesisGoalIR {
        TypedMechanismSynthesisGoalIR {
            schema: TYPED_MECHANISM_SYNTHESIS_GOAL_SCHEMA.to_string(),
            goal_id: "verified_queue_gate".to_string(),
            split: DataSplit::FreshBlind,
            operands: vec![
                operand("verified", ProgramType::Bool),
                operand("executable", ProgramType::Bool),
            ],
            output_type: ProgramType::Bool,
            definitions: Vec::new(),
            allowed_effects: vec![Effect::Pure],
            preconditions: Vec::new(),
            postconditions: vec!["retain unverified or executable cohorts".to_string()],
            invariants: vec!["verification cannot create knowledge".to_string()],
            public_observations: vec![
                observation(
                    &[
                        ("verified", Value::Bool(false)),
                        ("executable", Value::Bool(false)),
                    ],
                    Value::Bool(true),
                ),
                observation(
                    &[
                        ("verified", Value::Bool(true)),
                        ("executable", Value::Bool(false)),
                    ],
                    Value::Bool(false),
                ),
                observation(
                    &[
                        ("verified", Value::Bool(true)),
                        ("executable", Value::Bool(true)),
                    ],
                    Value::Bool(true),
                ),
            ],
            require_conditional: false,
            max_expression_depth: 3,
            max_candidates: 128,
            provenance: vec!["PUBLIC_PRODUCER".to_string()],
        }
    }

    fn consumer() -> TypedMechanismSynthesisGoalIR {
        TypedMechanismSynthesisGoalIR {
            schema: TYPED_MECHANISM_SYNTHESIS_GOAL_SCHEMA.to_string(),
            goal_id: "conditional_string_transport".to_string(),
            split: DataSplit::FreshBlind,
            operands: vec![
                operand("condition", ProgramType::Bool),
                operand("typed_value", ProgramType::String),
            ],
            output_type: ProgramType::String,
            definitions: Vec::new(),
            allowed_effects: vec![Effect::Pure],
            preconditions: Vec::new(),
            postconditions: vec!["transport the value only when allowed".to_string()],
            invariants: vec!["false uses the universal empty value".to_string()],
            public_observations: vec![
                observation(
                    &[
                        ("condition", Value::Bool(true)),
                        ("typed_value", Value::String("alpha".to_string())),
                    ],
                    Value::String("alpha".to_string()),
                ),
                observation(
                    &[
                        ("condition", Value::Bool(true)),
                        ("typed_value", Value::String("beta".to_string())),
                    ],
                    Value::String("beta".to_string()),
                ),
                observation(
                    &[
                        ("condition", Value::Bool(false)),
                        ("typed_value", Value::String("alpha".to_string())),
                    ],
                    Value::String(String::new()),
                ),
            ],
            require_conditional: true,
            max_expression_depth: 3,
            max_candidates: 128,
            provenance: vec!["PUBLIC_CONSUMER".to_string()],
        }
    }

    #[test]
    fn public_join_materializes_one_new_executable_compound_program() {
        let components = vec![producer(), consumer()];
        let compounds = derive_compound_typed_behavior_goals(&components).unwrap();
        assert_eq!(compounds.len(), 1);
        let compound = &compounds[0];
        assert!(compound.goal_id.starts_with("compound_"));
        assert_eq!(compound.output_type, ProgramType::String);
        assert_eq!(compound.operands.len(), 3);
        assert!(compound.public_observations.len() >= 5);
        let compound_receipt =
            execute_typed_behavior_goal_canary(&"a".repeat(64), compound).unwrap();
        let producer_receipt =
            execute_typed_behavior_goal_canary(&"b".repeat(64), &components[0]).unwrap();
        let consumer_receipt =
            execute_typed_behavior_goal_canary(&"c".repeat(64), &components[1]).unwrap();
        assert_eq!(
            compound_receipt.cases_executed,
            compound.public_observations.len()
        );
        assert_eq!(
            compound_receipt.cases_passed,
            compound_receipt.cases_executed
        );
        assert_ne!(
            compound_receipt.program_ir_sha256,
            producer_receipt.program_ir_sha256
        );
        assert_ne!(
            compound_receipt.program_ir_sha256,
            consumer_receipt.program_ir_sha256
        );
    }

    #[test]
    fn type_only_match_without_causal_consumer_influence_is_rejected() {
        let mut insensitive = consumer();
        for item in &mut insensitive.public_observations {
            item.expected_postimage = Value::String("constant".to_string());
        }
        let compounds = derive_compound_typed_behavior_goals(&[producer(), insensitive]).unwrap();
        assert!(compounds.is_empty());
    }

    #[test]
    fn effectful_components_fail_closed() {
        let mut effectful = producer();
        effectful.allowed_effects = vec![Effect::SandboxFileWrite];
        let compounds = derive_compound_typed_behavior_goals(&[effectful, consumer()]).unwrap();
        assert!(compounds.is_empty());
    }

    #[test]
    fn incompatible_public_values_do_not_invent_join_examples() {
        let mut incompatible = consumer();
        incompatible.operands[0].value_type = ProgramType::String;
        for item in &mut incompatible.public_observations {
            item.operands.insert(
                "condition".to_string(),
                Value::String("unmatched".to_string()),
            );
        }
        let compounds = derive_compound_typed_behavior_goals(&[producer(), incompatible]).unwrap();
        assert!(compounds.is_empty());
    }

    #[test]
    fn over_budget_join_is_rejected_locally_instead_of_stopping_the_product_loop() {
        let mut large_producer = producer();
        let mut large_consumer = consumer();
        for index in 0..16 {
            let producer_role = format!("producer_context_{index}");
            large_producer
                .operands
                .push(operand(&producer_role, ProgramType::Int));
            for observation in &mut large_producer.public_observations {
                observation
                    .operands
                    .insert(producer_role.clone(), Value::Int(index));
            }
            let consumer_role = format!("consumer_context_{index}");
            large_consumer
                .operands
                .push(operand(&consumer_role, ProgramType::Int));
            for observation in &mut large_consumer.public_observations {
                observation
                    .operands
                    .insert(consumer_role.clone(), Value::Int(index));
            }
        }
        let compounds =
            derive_compound_typed_behavior_goals(&[large_producer, large_consumer]).unwrap();
        assert!(compounds.is_empty());
    }

    #[test]
    fn repeated_identical_goal_is_deduplicated_and_conflicting_identity_is_quarantined() {
        let original = producer();
        assert!(
            derive_compound_typed_behavior_goals(&[original.clone(), original.clone()])
                .unwrap()
                .is_empty()
        );

        let mut conflicting = original.clone();
        conflicting.postconditions = vec!["a different contract".to_string()];
        assert!(
            derive_compound_typed_behavior_goals(&[original, conflicting])
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn conflicting_identity_does_not_stop_unrelated_valid_composition() {
        let ambiguous = producer();
        let mut conflicting = ambiguous.clone();
        conflicting.postconditions = vec!["a different contract".to_string()];
        let valid_producer = TypedMechanismSynthesisGoalIR {
            goal_id: "independent_producer".to_string(),
            ..producer()
        };
        let compounds = derive_compound_typed_behavior_goals(&[
            ambiguous,
            conflicting,
            valid_producer,
            consumer(),
        ])
        .unwrap();
        assert!(!compounds.is_empty());
        assert!(compounds.iter().all(|goal| !goal
            .provenance
            .iter()
            .any(|item| { item == "COMPOUND_COMPONENT_GOAL:verified_queue_gate" })));
    }

    #[test]
    fn promoted_compound_can_reenter_the_base_set_without_self_wiring() {
        let components = vec![producer(), consumer()];
        let promoted = derive_compound_typed_behavior_goals(&components)
            .unwrap()
            .into_iter()
            .next()
            .expect("one promoted compound");
        let mut later_plateau = components;
        later_plateau.push(promoted);

        let result = derive_compound_typed_behavior_goals(&later_plateau)
            .expect("retained compound is candidate-local, not a supervisor failure");

        assert!(result.len() <= MAX_COMPOUND_TYPED_GOAL_CANDIDATES);
    }
}
