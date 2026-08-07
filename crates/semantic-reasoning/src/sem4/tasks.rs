use sha2::{Digest, Sha256};

use super::{
    algebra::{add, divide, multiply, power, rational, subtract, variable},
    model::{
        BlindManifest, DataSplit, Equality, EvaluatorMetadata, EvaluatorTask, FormalCondition,
        MathObjectKind, MathPrimitive, MathProblem, MathStatement, MathTaskFamily,
        MathematicalPrimitiveRecord, OperatorDefinition,
    },
};

const BLIND_GENERATOR_VERSION: &str = "SEM4-MATH-BLIND-GENERATOR-1.0.1";

#[derive(Debug, Clone)]
pub struct GeneratedTaskSets {
    pub discovery: Vec<MathProblem>,
    pub blind: Vec<EvaluatorTask>,
    pub definition_only: Vec<EvaluatorTask>,
    pub adversarial: Vec<EvaluatorTask>,
}

#[derive(Debug, Clone)]
struct DeterministicGenerator {
    state: u64,
}

impl DeterministicGenerator {
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

    fn range_i64(&mut self, minimum: i64, maximum: i64) -> i64 {
        let width = (maximum - minimum + 1) as u64;
        minimum + (self.next_u64() % width) as i64
    }
}

pub fn mathematical_primitive_catalog() -> Vec<MathematicalPrimitiveRecord> {
    use MathObjectKind::{Condition, Expression, Rational as RationalObject};
    use MathPrimitive::*;
    let specifications = [
        (
            Add,
            vec![RationalObject, RationalObject],
            RationalObject,
            vec![],
        ),
        (
            Subtract,
            vec![RationalObject, RationalObject],
            RationalObject,
            vec![],
        ),
        (
            Multiply,
            vec![RationalObject, RationalObject],
            RationalObject,
            vec![],
        ),
        (
            Divide,
            vec![RationalObject, RationalObject],
            RationalObject,
            vec!["denominator != 0"],
        ),
        (Negate, vec![RationalObject], RationalObject, vec![]),
        (
            Compare,
            vec![RationalObject, RationalObject],
            Condition,
            vec![],
        ),
        (
            Substitute,
            vec![Expression, Expression],
            Expression,
            vec!["replacement type matches variable domain"],
        ),
        (
            PowerNonNegativeInteger,
            vec![RationalObject, MathObjectKind::Integer],
            RationalObject,
            vec!["exponent is a nonnegative integer"],
        ),
    ];
    specifications
        .into_iter()
        .enumerate()
        .map(
            |(index, (operation, input_domain, output_domain, preconditions))| {
                MathematicalPrimitiveRecord {
                    primitive_id: format!("MP{:06}", index + 1),
                    operation,
                    input_domain,
                    output_domain,
                    preconditions: preconditions.iter().map(ToString::to_string).collect(),
                    executable_semantics: format!(
                        "exact checked {:?} over the declared formal domain",
                        operation
                    ),
                    provenance: vec!["SEM4-MINIMAL-MATHEMATICAL-SUBSTRATE".to_string()],
                }
            },
        )
        .collect()
}

pub fn generate_task_sets(seed: u64) -> GeneratedTaskSets {
    let mut generator = DeterministicGenerator::new(seed);
    let discovery = generate_discovery(&mut generator);
    let mut blind = Vec::with_capacity(100);
    blind.extend(generate_equations(&mut generator));
    blind.extend(generate_recurrences(&mut generator));
    blind.extend(generate_identities(&mut generator));
    blind.extend(generate_definitions(&mut generator));
    blind.extend(generate_multi_concept(&mut generator));
    let definition_only = blind
        .iter()
        .filter(|task| task.evaluator.family == MathTaskFamily::DefinitionOnlyOperator)
        .cloned()
        .collect();
    let adversarial = blind
        .iter()
        .filter(|task| task.evaluator.adversarial)
        .cloned()
        .collect();
    GeneratedTaskSets {
        discovery,
        blind,
        definition_only,
        adversarial,
    }
}

fn generate_discovery(generator: &mut DeterministicGenerator) -> Vec<MathProblem> {
    let mut tasks = Vec::new();
    let deltas = [
        add(multiply(rational(2), variable("n")), rational(3)),
        add(add(power(variable("n"), 2), variable("n")), rational(1)),
        add(multiply(rational(3), variable("n")), rational(-2)),
        add(multiply(rational(2), power(variable("n"), 2)), rational(5)),
    ];
    for (index, delta) in deltas.into_iter().enumerate() {
        tasks.push(MathProblem {
            task_id: format!("SEM4-DISC-REC-{index:03}"),
            split: DataSplit::Discovery,
            statement: MathStatement::DeriveRecurrenceRelation {
                index_variable: "n".to_string(),
                base_index: 0,
                base_value: variable("b"),
                delta,
            },
            definitions: vec![],
            assumptions: vec![FormalCondition::NonNegativeInteger {
                variable: "n".to_string(),
            }],
            zero_demonstrations: true,
            provenance: vec!["DEFINITION_ONLY_RECURRENCE_GENERATOR".to_string()],
        });
    }
    for index in 0..12 {
        let delta = if index < 6 {
            add(multiply(rational(2), variable("n")), rational(3))
        } else {
            add(add(power(variable("n"), 2), variable("n")), rational(1))
        };
        tasks.push(MathProblem {
            task_id: format!("SEM4-CAL-REC-{index:03}"),
            split: DataSplit::Calibration,
            statement: MathStatement::DeriveRecurrenceRelation {
                index_variable: "n".to_string(),
                base_index: 0,
                base_value: rational(generator.range_i64(-15, 15)),
                delta,
            },
            definitions: vec![],
            assumptions: vec![FormalCondition::NonNegativeInteger {
                variable: "n".to_string(),
            }],
            zero_demonstrations: true,
            provenance: vec!["FRESH-CALIBRATION-RECURRENCE-GENERATOR".to_string()],
        });
    }
    for index in 0..8 {
        let token = randomized_token(generator, "d");
        let definition = operator_definition(generator, token.clone(), index);
        tasks.push(MathProblem {
            task_id: format!("SEM4-DISC-DEF-{index:03}"),
            split: DataSplit::Discovery,
            statement: MathStatement::ApplyDefinition {
                operator_token: token,
                arguments: vec![rational(index as i64 + 2), rational(index as i64 - 3)],
            },
            definitions: vec![definition],
            assumptions: vec![],
            zero_demonstrations: true,
            provenance: vec!["FORMAL_DEFINITION_GENERATOR".to_string()],
        });
    }
    tasks
}

fn generate_equations(generator: &mut DeterministicGenerator) -> Vec<EvaluatorTask> {
    (0..20)
        .map(|index| {
            let a = if index % 4 == 0 {
                0
            } else {
                nonzero(generator.range_i64(-7, 7))
            };
            let b = generator.range_i64(-11, 11);
            let c = if a == 0 && index % 8 == 0 {
                b
            } else {
                generator.range_i64(-13, 13)
            };
            let equation = Equality {
                left: add(multiply(rational(a), variable("x")), rational(b)),
                right: rational(c),
            };
            evaluator_task(
                format!("SEM4-BLIND-EQ-{index:03}"),
                DataSplit::FreshBlind,
                MathStatement::SolveEquation {
                    equation,
                    solve_for: "x".to_string(),
                },
                vec![],
                if a == 0 {
                    vec![]
                } else {
                    vec![FormalCondition::NonZero {
                        expression: rational(a),
                    }]
                },
                MathTaskFamily::SymbolicEquation,
                a == 0,
                a == 0,
                a != 0,
                4 + index,
                18 + index * 2,
                2,
            )
        })
        .collect()
}

fn generate_recurrences(generator: &mut DeterministicGenerator) -> Vec<EvaluatorTask> {
    (0..20)
        .map(|index| {
            let delta = if index < 8 {
                add(multiply(rational(2), variable("n")), rational(3))
            } else if index < 14 {
                add(add(power(variable("n"), 2), variable("n")), rational(1))
            } else {
                let coefficient = nonzero(generator.range_i64(-4, 4));
                let constant = generator.range_i64(-6, 6);
                add(
                    multiply(rational(coefficient), variable("n")),
                    rational(constant),
                )
            };
            evaluator_task(
                format!("SEM4-BLIND-REC-{index:03}"),
                DataSplit::FreshBlind,
                MathStatement::DeriveRecurrenceRelation {
                    index_variable: "n".to_string(),
                    base_index: 0,
                    base_value: rational(generator.range_i64(-9, 12)),
                    delta,
                },
                vec![],
                vec![FormalCondition::NonNegativeInteger {
                    variable: "n".to_string(),
                }],
                MathTaskFamily::Recurrence,
                index >= 14,
                false,
                true,
                18 + index * 2,
                70 + index * 9,
                3,
            )
        })
        .collect()
}

fn generate_identities(generator: &mut DeterministicGenerator) -> Vec<EvaluatorTask> {
    (0..20)
        .map(|index| {
            let a = nonzero(generator.range_i64(-5, 5));
            let b = generator.range_i64(-7, 7);
            let c = nonzero(generator.range_i64(-5, 5));
            let d = generator.range_i64(-7, 7);
            let expression = multiply(
                add(multiply(rational(a), variable("z")), rational(b)),
                add(multiply(rational(c), variable("z")), rational(d)),
            );
            evaluator_task(
                format!("SEM4-BLIND-ID-{index:03}"),
                DataSplit::FreshBlind,
                MathStatement::DeriveEquivalentIdentity { expression },
                vec![],
                vec![FormalCondition::VariablesRational {
                    variables: vec!["z".to_string()],
                }],
                MathTaskFamily::GeneratedIdentity,
                index >= 16,
                false,
                true,
                9 + index,
                32 + index * 4,
                3,
            )
        })
        .collect()
}

fn generate_definitions(generator: &mut DeterministicGenerator) -> Vec<EvaluatorTask> {
    (0..20)
        .map(|index| {
            let token = randomized_token(generator, "q");
            let definition = operator_definition(generator, token.clone(), index);
            let left = generator.range_i64(-8, 9);
            let mut right = generator.range_i64(-8, 9);
            if definition.domain_conditions.iter().any(|condition| {
                matches!(condition, FormalCondition::NonZero { expression } if expression == &variable("v"))
            }) && right == 0
            {
                right = 2;
            }
            evaluator_task(
                format!("SEM4-BLIND-DEF-{index:03}"),
                DataSplit::DefinitionOnlyBlind,
                MathStatement::ApplyDefinition {
                    operator_token: token,
                    arguments: vec![rational(left), rational(right)],
                },
                vec![definition],
                vec![],
                MathTaskFamily::DefinitionOnlyOperator,
                index >= 15,
                false,
                true,
                7 + index,
                24 + index * 3,
                2,
            )
        })
        .collect()
}

fn generate_multi_concept(generator: &mut DeterministicGenerator) -> Vec<EvaluatorTask> {
    (0..20)
        .map(|index| {
            let delta = if index < 10 {
                add(multiply(rational(2), variable("n")), rational(3))
            } else {
                add(add(power(variable("n"), 2), variable("n")), rational(1))
            };
            evaluator_task(
                format!("SEM4-BLIND-MULTI-{index:03}"),
                DataSplit::AdversarialBlind,
                MathStatement::ReuseDerivedRelation {
                    index_variable: "n".to_string(),
                    base_index: 0,
                    base_value: rational(generator.range_i64(-20, 20)),
                    delta,
                    required_reasoning_layers: 5,
                },
                vec![],
                vec![FormalCondition::NonNegativeInteger {
                    variable: "n".to_string(),
                }],
                MathTaskFamily::MultiConceptAdversarial,
                true,
                false,
                true,
                35 + index * 3,
                620 + index * 15,
                5,
            )
        })
        .collect()
}

#[allow(clippy::too_many_arguments)]
fn evaluator_task(
    task_id: String,
    split: DataSplit,
    statement: MathStatement,
    definitions: Vec<OperatorDefinition>,
    assumptions: Vec<FormalCondition>,
    family: MathTaskFamily,
    adversarial: bool,
    invalid_case: bool,
    expected_applicable: bool,
    solution_graph_depth: usize,
    primitive_expanded_depth: usize,
    concepts_composed: usize,
) -> EvaluatorTask {
    EvaluatorTask {
        visible: MathProblem {
            task_id: task_id.clone(),
            split,
            statement,
            definitions,
            assumptions,
            zero_demonstrations: family == MathTaskFamily::DefinitionOnlyOperator,
            provenance: vec![format!("{BLIND_GENERATOR_VERSION}:{task_id}")],
        },
        evaluator: EvaluatorMetadata {
            family,
            adversarial,
            invalid_case,
            expected_applicable,
            target_formula_stored: false,
            solution_graph_depth,
            primitive_expanded_depth,
            concepts_composed,
        },
    }
}

fn operator_definition(
    generator: &mut DeterministicGenerator,
    token: String,
    index: usize,
) -> OperatorDefinition {
    let constant = nonzero(generator.range_i64(-5, 5));
    let (body, domain_conditions) = match index % 5 {
        0 => (
            add(variable("u"), multiply(rational(constant), variable("v"))),
            vec![],
        ),
        1 => (
            subtract(variable("v"), multiply(rational(constant), variable("u"))),
            vec![],
        ),
        2 => (
            add(multiply(variable("u"), variable("v")), rational(constant)),
            vec![],
        ),
        3 => (
            multiply(add(variable("u"), rational(constant)), variable("v")),
            vec![],
        ),
        _ => (
            divide(add(variable("u"), variable("v")), rational(constant)),
            vec![FormalCondition::NonZero {
                expression: rational(constant),
            }],
        ),
    };
    OperatorDefinition {
        operator_token: token,
        parameters: vec!["u".to_string(), "v".to_string()],
        body,
        domain_conditions,
        examples: vec![],
        randomized_symbol: true,
        provenance: vec!["FORMAL_DEFINITION_ONLY_NO_DEMONSTRATIONS".to_string()],
    }
}

fn randomized_token(generator: &mut DeterministicGenerator, prefix: &str) -> String {
    format!("{prefix}_{:016x}", generator.next_u64())
}

fn nonzero(value: i64) -> i64 {
    if value == 0 {
        1
    } else {
        value
    }
}

pub fn build_manifest(
    run_id: &str,
    seed: u64,
    split: DataSplit,
    tasks: &[EvaluatorTask],
) -> Result<BlindManifest, String> {
    let mut manifest = BlindManifest {
        run_id: run_id.to_string(),
        generator_version: BLIND_GENERATOR_VERSION.to_string(),
        seed,
        split,
        tasks: tasks.iter().map(|task| task.visible.clone()).collect(),
        expected_answers_included: false,
        target_formulas_included: false,
        proof_scripts_included: false,
        human_formula_names_included: false,
        reasoner_access_before_freeze: false,
        manifest_sha256: String::new(),
    };
    manifest.manifest_sha256 = hash_serializable(&manifest)?;
    Ok(manifest)
}

pub fn hash_serializable<T: serde::Serialize>(value: &T) -> Result<String, String> {
    serde_json::to_vec(value)
        .map(|bytes| format!("{:x}", Sha256::digest(bytes)))
        .map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blind_manifests_contain_definitions_but_no_answers_or_target_formulas() {
        let sets = generate_task_sets(0x5e4_f1a5);
        assert_eq!(sets.blind.len(), 100);
        assert_eq!(sets.definition_only.len(), 20);
        assert_eq!(sets.adversarial.len(), 40);
        let manifest = build_manifest("TEST", 0x5e4_f1a5, DataSplit::FreshBlind, &sets.blind)
            .expect("manifest");
        let json = serde_json::to_string(&manifest).expect("json");
        assert!(!manifest.expected_answers_included);
        assert!(!manifest.target_formulas_included);
        assert!(!json.contains("evaluator"));
        assert!(!json.contains("target_formula_stored"));
    }

    #[test]
    fn novel_operator_symbols_and_semantics_vary_without_examples() {
        let sets = generate_task_sets(0x5e4_f1a5);
        let definitions: Vec<_> = sets
            .definition_only
            .iter()
            .map(|task| &task.visible.definitions[0])
            .collect();
        assert_eq!(
            definitions
                .iter()
                .map(|definition| &definition.operator_token)
                .collect::<std::collections::BTreeSet<_>>()
                .len(),
            20
        );
        assert!(definitions
            .iter()
            .all(|definition| definition.examples.is_empty()));
        assert!(definitions
            .iter()
            .all(|definition| definition.randomized_symbol));
    }
}
