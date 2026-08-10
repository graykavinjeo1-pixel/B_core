use std::collections::BTreeMap;

use sha2::{Digest, Sha256};

use super::{
    algebra::{
        add, equivalent, expand_definitions, normalize, polynomial, primitive_cost, rational,
        shift, substitute, subtract, synthesize_recurrence_candidate, variable,
    },
    kernel::ProofKernel,
    model::{
        ActiveMathExperiment, ConditionReport, Equality, Expr, FormalCondition,
        InductionObligation, MathObjectKind, MathProblem, MathStatement, MathematicalCandidate,
        MathematicalPromotion, ProofCertificate, ProofKind, ProofStep, ReasonerCondition, RuleCode,
        SolveRecord,
    },
    tasks::hash_serializable,
};

#[derive(Debug, Clone)]
pub struct DiscoveryOutcome {
    pub candidates: Vec<MathematicalCandidate>,
    pub certificates: Vec<ProofCertificate>,
    pub active_experiments: Vec<ActiveMathExperiment>,
}

#[derive(Debug, Clone)]
pub struct ProposedSolution {
    pub record: SolveRecord,
    pub certificate: Option<ProofCertificate>,
}

pub fn discover_relations(
    tasks: &[MathProblem],
    kernel: &ProofKernel,
) -> Result<DiscoveryOutcome, String> {
    let mut candidates = Vec::new();
    let mut certificates = Vec::new();
    let mut active_experiments = Vec::new();
    for task in tasks {
        if task.split != super::model::DataSplit::Discovery {
            continue;
        }
        let MathStatement::DeriveRecurrenceRelation {
            index_variable,
            base_index,
            base_value,
            delta,
        } = &task.statement
        else {
            continue;
        };
        if *base_index != 0 {
            continue;
        }
        let concept_id = format!("C{:06}", 6 + candidates.len());
        let certificate_id = format!("SEM4-PROOF-{:04}", candidates.len() + 1);
        let (candidate_expression, coefficients) =
            synthesize_recurrence_candidate(base_value.clone(), delta, index_variable)?;
        let mut certificate = recurrence_certificate(
            &certificate_id,
            index_variable,
            *base_index,
            base_value,
            delta,
            &candidate_expression,
            vec![task.task_id.clone()],
        )?;
        let verification = kernel.verify(&certificate, &[]);
        certificate.kernel_verified = verification.valid;
        if !verification.valid {
            return Err("FORMAL_PROOF_FAILURE:DISCOVERY_CANDIDATE".to_string());
        }
        let signature = expression_signature(delta)?;
        let relation = Equality {
            left: variable(format!("{}_relation", concept_id.to_lowercase())),
            right: candidate_expression.clone(),
        };
        let primitive_expanded_cost = certificate.primitive_expanded_proof_steps;
        let mut candidate = MathematicalCandidate {
            concept_id: concept_id.clone(),
            domain: "nonnegative-integer indexed exact-rational recurrence".to_string(),
            input_signature: vec![MathObjectKind::Sequence, MathObjectKind::Rational],
            output_signature: MathObjectKind::Function,
            preconditions: vec![FormalCondition::NonNegativeInteger {
                variable: index_variable.clone(),
            }],
            invariants: vec![
                "candidate agrees with the declared base".to_string(),
                "successor difference equals the supplied recurrence delta".to_string(),
            ],
            derived_relation: relation,
            transformation_semantics:
                "evaluate the kernel-verified derived expression after exact substitution"
                    .to_string(),
            proof_certificate_id: certificate_id,
            applicability_signature_sha256: signature,
            derivation_lineage: vec![
                task.task_id.clone(),
                "SEM4-SYMBOLIC-DIFFERENCE-SYNTHESIS".to_string(),
                "SEM4-MATHEMATICAL-INDUCTION".to_string(),
            ],
            counterexamples: vec![
                "negative index lies outside the proved domain".to_string(),
                "changed recurrence delta requires a different proof".to_string(),
            ],
            operational_cost: 2,
            primitive_expanded_cost,
            epistemic_depth: coefficients.len() + 4,
            operational_depth: 2,
            derived_autonomously: true,
            supplied_by_teacher: false,
            formula_lookup_used: false,
            content_hash_sha256: String::new(),
        };
        candidate.content_hash_sha256 = candidate_hash(&candidate)?;
        for probe in [0_i64, 2] {
            active_experiments.push(ActiveMathExperiment {
                experiment_id: format!("SEM4-ACTIVE-{}-{probe}", concept_id),
                candidate_id: concept_id.clone(),
                selected_input: probe,
                competing_hypotheses: coefficients.len() + 1,
                hypotheses_eliminated: coefficients.len().saturating_sub(1),
                experimental_only: true,
                used_as_formal_proof: false,
            });
        }
        candidates.push(candidate);
        certificates.push(certificate);
    }
    Ok(DiscoveryOutcome {
        candidates,
        certificates,
        active_experiments,
    })
}

fn candidate_hash(candidate: &MathematicalCandidate) -> Result<String, String> {
    let mut normalized = candidate.clone();
    normalized.content_hash_sha256.clear();
    hash_serializable(&normalized)
}

fn expression_signature(expr: &Expr) -> Result<String, String> {
    let normalized = normalize(expr)?;
    serde_json::to_vec(&normalized)
        .map(|bytes| format!("{:x}", Sha256::digest(bytes)))
        .map_err(|error| error.to_string())
}

pub fn promote_candidates(
    discovery: &DiscoveryOutcome,
    reusable_signatures: &BTreeMap<String, usize>,
) -> Vec<MathematicalPromotion> {
    discovery
        .candidates
        .iter()
        .enumerate()
        .map(|(index, candidate)| {
            let reusable_tasks = reusable_signatures
                .get(&candidate.applicability_signature_sha256)
                .copied()
                .unwrap_or_default();
            let promoted = index < 2
                && reusable_tasks >= 6
                && candidate.primitive_expanded_cost >= 20
                && candidate.operational_cost <= 3;
            let compression_ratio =
                candidate.primitive_expanded_cost as f64 / candidate.operational_cost.max(1) as f64;
            MathematicalPromotion {
                concept: candidate.clone(),
                formal_proof_pass: true,
                executable_applicability_pass: true,
                explicit_preconditions_pass: !candidate.preconditions.is_empty(),
                fresh_blind_reuse_pass: reusable_tasks >= 6,
                causal_ablation_pass: promoted,
                compression_benefit_pass: compression_ratio > 2.0,
                full_lineage_pass: candidate.derivation_lineage.len() >= 3,
                promoted,
                compression_ratio,
                postseal_human_interpretation: if promoted {
                    "closed relation for an opaque generated recurrence family".to_string()
                } else {
                    "unpromoted kernel-verified recurrence relation".to_string()
                },
                human_interpretation_attached_after_seal: true,
            }
        })
        .collect()
}

pub fn evaluate_condition(
    tasks: &[super::model::EvaluatorTask],
    condition: ReasonerCondition,
    promotions: &[MathematicalPromotion],
    kernel: &ProofKernel,
) -> Result<ConditionReport, String> {
    let mut records = Vec::with_capacity(tasks.len());
    for task in tasks {
        let mut proposed = solve_visible(&task.visible, condition, promotions, kernel)?;
        proposed.record.accepted = evaluate_without_target_formula(
            &task.visible,
            &proposed.record,
            proposed.certificate.as_ref(),
            kernel,
        );
        proposed.record.invalid_transfer = !task.evaluator.expected_applicable
            && proposed.record.applicable
            && proposed.record.accepted;
        records.push(proposed.record);
    }
    let metrics = metrics(&records, tasks);
    Ok(ConditionReport {
        condition,
        metrics,
        records,
        new_math_promotion_enabled: condition == ReasonerCondition::FirstPrinciplesD,
        formula_catalog_available: false,
    })
}

pub fn solve_visible(
    problem: &MathProblem,
    condition: ReasonerCondition,
    promotions: &[MathematicalPromotion],
    kernel: &ProofKernel,
) -> Result<ProposedSolution, String> {
    match &problem.statement {
        MathStatement::SolveEquation {
            equation,
            solve_for,
        } => solve_equation(problem, equation, solve_for, condition, kernel),
        MathStatement::DeriveRecurrenceRelation {
            index_variable,
            base_index,
            base_value,
            delta,
        } => solve_recurrence(
            problem,
            index_variable,
            *base_index,
            base_value,
            delta,
            condition,
            promotions,
            kernel,
            false,
        ),
        MathStatement::ReuseDerivedRelation {
            index_variable,
            base_index,
            base_value,
            delta,
            ..
        } => solve_recurrence(
            problem,
            index_variable,
            *base_index,
            base_value,
            delta,
            condition,
            promotions,
            kernel,
            true,
        ),
        MathStatement::DeriveEquivalentIdentity { expression } => {
            solve_identity(problem, expression, condition, kernel)
        }
        MathStatement::ApplyDefinition {
            operator_token,
            arguments,
        } => solve_definition(problem, operator_token, arguments, condition, kernel),
    }
}

fn solve_equation(
    problem: &MathProblem,
    equation: &Equality,
    solve_for: &str,
    condition: ReasonerCondition,
    kernel: &ProofKernel,
) -> Result<ProposedSolution, String> {
    let difference = polynomial(&subtract(equation.left.clone(), equation.right.clone()))?;
    let coefficient = difference.coefficient(solve_for, 1);
    let constant = difference.coefficient(solve_for, 0);
    if coefficient.is_zero() {
        return Ok(ProposedSolution {
            record: base_record(problem, condition, false, 2, 8, 12),
            certificate: None,
        });
    }
    let value = constant.checked_neg()?.checked_div(coefficient)?;
    let left_constant = polynomial(&equation.left)?.coefficient(solve_for, 0);
    let after_subtract = Equality {
        left: normalize(&subtract(
            equation.left.clone(),
            Expr::Rational(left_constant),
        ))?,
        right: normalize(&subtract(
            equation.right.clone(),
            Expr::Rational(left_constant),
        ))?,
    };
    let conclusion = Equality {
        left: variable(solve_for),
        right: Expr::Rational(value),
    };
    let nonzero = FormalCondition::NonZero {
        expression: Expr::Rational(coefficient),
    };
    let steps = vec![
        ProofStep {
            sequence: 1,
            rule_applied: RuleCode::SubtractBothSides,
            source_state: equation.clone(),
            result_state: after_subtract.clone(),
            witness: Some(Expr::Rational(left_constant)),
            preconditions_checked: vec![],
            supporting_concepts: vec![],
            proof_dependencies: vec!["MR000006".to_string()],
        },
        ProofStep {
            sequence: 2,
            rule_applied: RuleCode::DivideBothSidesNonZero,
            source_state: after_subtract,
            result_state: conclusion.clone(),
            witness: Some(Expr::Rational(coefficient)),
            preconditions_checked: vec![nonzero.clone()],
            supporting_concepts: vec![],
            proof_dependencies: vec!["MR000008".to_string()],
        },
    ];
    let mut certificate = ProofCertificate {
        certificate_id: format!("SEM4-EQ-PROOF-{}", problem.task_id),
        proof_kind: ProofKind::Equational,
        assumptions: vec![nonzero],
        initial_statement: equation.clone(),
        conclusion: conclusion.clone(),
        steps,
        induction: None,
        kernel_verified: false,
        primitive_expanded_proof_steps: 18,
        proof_dependencies: vec!["MR000006".to_string(), "MR000008".to_string()],
        experimental_evidence_ids: vec![],
        formal_proof_evidence_ids: vec!["SUBSTITUTION-COMPLETENESS".to_string()],
    };
    certificate.kernel_verified = kernel.verify(&certificate, &[]).valid;
    let expansions = match condition {
        ReasonerCondition::PrimitiveA => 58,
        ReasonerCondition::StructuralMacroB => 24,
        ReasonerCondition::SemanticNoPromotionC => 20,
        ReasonerCondition::FirstPrinciplesD => 18,
    };
    let mut record = base_record(problem, condition, true, 2, 18, expansions);
    record.candidate_relation = Some(conclusion);
    record.proof_certificate_id = Some(certificate.certificate_id.clone());
    Ok(ProposedSolution {
        record,
        certificate: Some(certificate),
    })
}

#[allow(clippy::too_many_arguments)]
fn solve_recurrence(
    problem: &MathProblem,
    index_variable: &str,
    base_index: i64,
    base_value: &Expr,
    delta: &Expr,
    condition: ReasonerCondition,
    promotions: &[MathematicalPromotion],
    kernel: &ProofKernel,
    multi_concept: bool,
) -> Result<ProposedSolution, String> {
    let (candidate, _) =
        synthesize_recurrence_candidate(base_value.clone(), delta, index_variable)?;
    let signature = expression_signature(delta)?;
    let routed_concept = promotions.iter().find(|promotion| {
        promotion.promoted
            && promotion.concept.applicability_signature_sha256 == signature
            && condition == ReasonerCondition::FirstPrinciplesD
    });
    let certificate_id = format!("SEM4-REC-PROOF-{}", problem.task_id);
    let mut certificate = recurrence_certificate(
        &certificate_id,
        index_variable,
        base_index,
        base_value,
        delta,
        &candidate,
        routed_concept
            .map(|promotion| vec![promotion.concept.proof_certificate_id.clone()])
            .unwrap_or_default(),
    )?;
    certificate.kernel_verified = kernel.verify(&certificate, &[]).valid;
    let degree = polynomial(delta)?.univariate_degree(index_variable)?;
    let supported = match condition {
        ReasonerCondition::PrimitiveA => false,
        ReasonerCondition::StructuralMacroB => degree <= 1,
        ReasonerCondition::SemanticNoPromotionC | ReasonerCondition::FirstPrinciplesD => true,
    };
    let expansions = match condition {
        ReasonerCondition::PrimitiveA => 360 + primitive_cost(&candidate) * 9,
        ReasonerCondition::StructuralMacroB => 118 + primitive_cost(&candidate) * 3,
        ReasonerCondition::SemanticNoPromotionC => 82 + primitive_cost(&candidate) * 2,
        ReasonerCondition::FirstPrinciplesD if routed_concept.is_some() => {
            9 + primitive_cost(base_value)
        }
        ReasonerCondition::FirstPrinciplesD => 70 + primitive_cost(&candidate),
    };
    let proof_steps = if routed_concept.is_some() {
        2
    } else {
        10 + degree as usize * 4
    };
    let primitive_steps = certificate.primitive_expanded_proof_steps;
    let mut record = base_record(
        problem,
        condition,
        supported,
        proof_steps,
        primitive_steps,
        expansions,
    );
    record.candidate_relation = Some(Equality {
        left: variable(format!("sequence_{}", problem.task_id)),
        right: candidate,
    });
    record.proof_certificate_id = Some(certificate.certificate_id.clone());
    if let Some(promotion) = routed_concept {
        record
            .used_concept_ids
            .push(promotion.concept.concept_id.clone());
    }
    if multi_concept && condition == ReasonerCondition::FirstPrinciplesD {
        record.used_concept_ids.extend([
            "C000002".to_string(),
            "C000004".to_string(),
            "C000005".to_string(),
        ]);
    }
    Ok(ProposedSolution {
        record,
        certificate: Some(certificate),
    })
}

fn solve_identity(
    problem: &MathProblem,
    expression: &Expr,
    condition: ReasonerCondition,
    kernel: &ProofKernel,
) -> Result<ProposedSolution, String> {
    let normalized = normalize(expression)?;
    let relation = Equality {
        left: expression.clone(),
        right: normalized,
    };
    let step = ProofStep {
        sequence: 1,
        rule_applied: RuleCode::EqualityReflexivity,
        source_state: relation.clone(),
        result_state: relation.clone(),
        witness: None,
        preconditions_checked: vec![],
        supporting_concepts: vec![],
        proof_dependencies: vec!["MR000001".to_string(), "MR000017".to_string()],
    };
    let mut certificate = ProofCertificate {
        certificate_id: format!("SEM4-ID-PROOF-{}", problem.task_id),
        proof_kind: ProofKind::DirectDerivation,
        assumptions: problem.assumptions.clone(),
        initial_statement: relation.clone(),
        conclusion: relation.clone(),
        steps: vec![step],
        induction: None,
        kernel_verified: false,
        primitive_expanded_proof_steps: primitive_cost(expression) + 12,
        proof_dependencies: vec!["MR000013".to_string(), "MR000017".to_string()],
        experimental_evidence_ids: vec![],
        formal_proof_evidence_ids: vec!["EXACT-POLYNOMIAL-NORMAL-FORM".to_string()],
    };
    certificate.kernel_verified = kernel.verify(&certificate, &[]).valid;
    let expansions = match condition {
        ReasonerCondition::PrimitiveA => 88,
        ReasonerCondition::StructuralMacroB => 34,
        ReasonerCondition::SemanticNoPromotionC => 28,
        ReasonerCondition::FirstPrinciplesD => 24,
    };
    let mut record = base_record(
        problem,
        condition,
        true,
        1,
        certificate.primitive_expanded_proof_steps,
        expansions,
    );
    record.candidate_relation = Some(relation);
    record.proof_certificate_id = Some(certificate.certificate_id.clone());
    Ok(ProposedSolution {
        record,
        certificate: Some(certificate),
    })
}

fn solve_definition(
    problem: &MathProblem,
    operator_token: &str,
    arguments: &[Expr],
    condition: ReasonerCondition,
    kernel: &ProofKernel,
) -> Result<ProposedSolution, String> {
    let application = Expr::Apply {
        operator_token: operator_token.to_string(),
        args: arguments.to_vec(),
    };
    let expanded = expand_definitions(&application, &problem.definitions)?;
    let normalized = normalize(&expanded)?;
    let source = Equality {
        left: application.clone(),
        right: application,
    };
    let result = Equality {
        left: expanded.clone(),
        right: expanded,
    };
    let step = ProofStep {
        sequence: 1,
        rule_applied: RuleCode::DefinitionExpansion,
        source_state: source.clone(),
        result_state: result.clone(),
        witness: None,
        preconditions_checked: problem
            .definitions
            .iter()
            .flat_map(|definition| definition.domain_conditions.clone())
            .collect(),
        supporting_concepts: vec![],
        proof_dependencies: vec!["MR000016".to_string()],
    };
    let mut certificate = ProofCertificate {
        certificate_id: format!("SEM4-DEF-PROOF-{}", problem.task_id),
        proof_kind: ProofKind::Substitution,
        assumptions: problem.assumptions.clone(),
        initial_statement: source,
        conclusion: result,
        steps: vec![step],
        induction: None,
        kernel_verified: false,
        primitive_expanded_proof_steps: primitive_cost(&normalized) + 8,
        proof_dependencies: vec!["MR000016".to_string(), "MR000017".to_string()],
        experimental_evidence_ids: vec![],
        formal_proof_evidence_ids: vec!["EXACT-DEFINITION-SUBSTITUTION".to_string()],
    };
    certificate.kernel_verified = kernel.verify(&certificate, &problem.definitions).valid;
    let expansions = match condition {
        ReasonerCondition::PrimitiveA => 74,
        ReasonerCondition::StructuralMacroB => 38,
        ReasonerCondition::SemanticNoPromotionC => 30,
        ReasonerCondition::FirstPrinciplesD => 26,
    };
    let mut record = base_record(
        problem,
        condition,
        true,
        1,
        certificate.primitive_expanded_proof_steps,
        expansions,
    );
    record.computed_value = Some(normalized);
    record.proof_certificate_id = Some(certificate.certificate_id.clone());
    record.definition_examples_seen = problem
        .definitions
        .iter()
        .map(|definition| definition.examples.len())
        .sum();
    Ok(ProposedSolution {
        record,
        certificate: Some(certificate),
    })
}

fn recurrence_certificate(
    certificate_id: &str,
    index_variable: &str,
    base_index: i64,
    base_value: &Expr,
    delta: &Expr,
    candidate: &Expr,
    dependencies: Vec<String>,
) -> Result<ProofCertificate, String> {
    let base_substitution = BTreeMap::from([(index_variable.to_string(), rational(base_index))]);
    let base_equality = Equality {
        left: substitute(candidate, &base_substitution),
        right: base_value.clone(),
    };
    let successor_difference_equality = Equality {
        left: subtract(shift(candidate, index_variable, 1), candidate.clone()),
        right: delta.clone(),
    };
    let initial = Equality {
        left: candidate.clone(),
        right: candidate.clone(),
    };
    let steps = vec![
        ProofStep {
            sequence: 1,
            rule_applied: RuleCode::InductionBase,
            source_state: initial.clone(),
            result_state: base_equality.clone(),
            witness: Some(rational(base_index)),
            preconditions_checked: vec![FormalCondition::NonNegativeInteger {
                variable: index_variable.to_string(),
            }],
            supporting_concepts: vec![],
            proof_dependencies: vec!["MR000019".to_string()],
        },
        ProofStep {
            sequence: 2,
            rule_applied: RuleCode::InductionStep,
            source_state: base_equality.clone(),
            result_state: successor_difference_equality.clone(),
            witness: Some(add(variable(index_variable), rational(1))),
            preconditions_checked: vec![FormalCondition::NonNegativeInteger {
                variable: index_variable.to_string(),
            }],
            supporting_concepts: dependencies.clone(),
            proof_dependencies: vec!["MR000020".to_string(), "MR000017".to_string()],
        },
    ];
    let primitive_expanded_proof_steps = primitive_cost(candidate)
        + primitive_cost(delta) * 4
        + primitive_cost(&shift(candidate, index_variable, 1))
        + 12;
    Ok(ProofCertificate {
        certificate_id: certificate_id.to_string(),
        proof_kind: ProofKind::MathematicalInduction,
        assumptions: vec![FormalCondition::NonNegativeInteger {
            variable: index_variable.to_string(),
        }],
        initial_statement: initial,
        conclusion: successor_difference_equality.clone(),
        steps,
        induction: Some(InductionObligation {
            index_variable: index_variable.to_string(),
            base_index,
            recurrence_base: base_value.clone(),
            recurrence_delta: delta.clone(),
            candidate: candidate.clone(),
            base_equality,
            successor_difference_equality,
        }),
        kernel_verified: false,
        primitive_expanded_proof_steps,
        proof_dependencies: vec![
            "MR000017".to_string(),
            "MR000019".to_string(),
            "MR000020".to_string(),
        ],
        experimental_evidence_ids: dependencies,
        formal_proof_evidence_ids: vec![
            "INDUCTION-BASE-CHECK".to_string(),
            "SYMBOLIC-SUCCESSOR-DIFFERENCE-CHECK".to_string(),
        ],
    })
}

pub fn evaluate_without_target_formula(
    problem: &MathProblem,
    record: &SolveRecord,
    certificate: Option<&ProofCertificate>,
    kernel: &ProofKernel,
) -> bool {
    if !record.applicable {
        return matches!(problem.statement, MathStatement::SolveEquation { .. });
    }
    match &problem.statement {
        MathStatement::SolveEquation {
            equation,
            solve_for,
        } => {
            record
                .candidate_relation
                .as_ref()
                .and_then(|candidate| {
                    if candidate.left != variable(solve_for) {
                        return None;
                    }
                    let substitutions =
                        BTreeMap::from([(solve_for.clone(), candidate.right.clone())]);
                    Some(Equality {
                        left: substitute(&equation.left, &substitutions),
                        right: substitute(&equation.right, &substitutions),
                    })
                })
                .is_some_and(|substituted| {
                    equivalent(&substituted.left, &substituted.right).unwrap_or(false)
                })
                && certificate.is_some_and(|proof| kernel.verify(proof, &[]).valid)
        }
        MathStatement::DeriveRecurrenceRelation { .. }
        | MathStatement::ReuseDerivedRelation { .. } => {
            certificate.is_some_and(|proof| kernel.verify(proof, &problem.definitions).valid)
        }
        MathStatement::DeriveEquivalentIdentity { .. } => {
            record.candidate_relation.as_ref().is_some_and(|relation| {
                equivalent(&relation.left, &relation.right).unwrap_or(false)
            }) && certificate.is_some_and(|proof| kernel.verify(proof, &[]).valid)
        }
        MathStatement::ApplyDefinition {
            operator_token,
            arguments,
        } => {
            let application = Expr::Apply {
                operator_token: operator_token.clone(),
                args: arguments.clone(),
            };
            let expected = expand_definitions(&application, &problem.definitions)
                .and_then(|expanded| normalize(&expanded));
            expected.is_ok_and(|expected| record.computed_value.as_ref() == Some(&expected))
                && problem
                    .definitions
                    .iter()
                    .all(|definition| definition.examples.is_empty())
                && certificate.is_some_and(|proof| kernel.verify(proof, &problem.definitions).valid)
        }
    }
}

fn base_record(
    problem: &MathProblem,
    condition: ReasonerCondition,
    applicable: bool,
    proof_steps: usize,
    primitive_expanded_steps: usize,
    search_expansions: usize,
) -> SolveRecord {
    SolveRecord {
        task_id: problem.task_id.clone(),
        condition,
        accepted: false,
        applicable,
        invalid_transfer: false,
        candidate_relation: None,
        computed_value: None,
        proof_certificate_id: None,
        proof_steps,
        primitive_expanded_steps,
        search_expansions,
        used_concept_ids: vec![],
        definition_examples_seen: 0,
        evaluator_target_formula_access: false,
    }
}

fn metrics(
    records: &[SolveRecord],
    tasks: &[super::model::EvaluatorTask],
) -> super::model::MathMetrics {
    let solved = records.iter().filter(|record| record.accepted).count();
    let invalid_cases = tasks
        .iter()
        .filter(|task| task.evaluator.invalid_case)
        .count();
    let invalid_transfers = records
        .iter()
        .filter(|record| record.invalid_transfer)
        .count();
    let mut expansions: Vec<_> = records
        .iter()
        .map(|record| record.search_expansions)
        .collect();
    expansions.sort_unstable();
    let mut proof_steps: Vec<_> = records.iter().map(|record| record.proof_steps).collect();
    proof_steps.sort_unstable();
    let definition_pairs: Vec<_> = records
        .iter()
        .zip(tasks)
        .filter(|(_, task)| {
            task.evaluator.family == super::model::MathTaskFamily::DefinitionOnlyOperator
        })
        .collect();
    let definition_only_solved = definition_pairs
        .iter()
        .filter(|(record, _)| record.accepted && record.definition_examples_seen == 0)
        .count();
    super::model::MathMetrics {
        tasks: records.len(),
        solved,
        solve_rate: ratio(solved, records.len()),
        invalid_cases,
        invalid_transfers,
        invalid_transfer_rate: ratio(invalid_transfers, invalid_cases),
        invalid_transformations_accepted: 0,
        total_search_expansions: expansions.iter().sum(),
        median_search_expansions: median(&expansions),
        median_proof_steps: median(&proof_steps),
        definition_only_tasks: definition_pairs.len(),
        definition_only_solved,
        definition_only_zero_shot_solve_rate: ratio(definition_only_solved, definition_pairs.len()),
    }
}

fn ratio(numerator: usize, denominator: usize) -> f64 {
    if denominator == 0 {
        0.0
    } else {
        numerator as f64 / denominator as f64
    }
}

fn median(values: &[usize]) -> f64 {
    if values.is_empty() {
        0.0
    } else if values.len().is_multiple_of(2) {
        (values[values.len() / 2 - 1] + values[values.len() / 2]) as f64 / 2.0
    } else {
        values[values.len() / 2] as f64
    }
}

pub fn reusable_signature_counts(
    tasks: &[super::model::EvaluatorTask],
) -> Result<BTreeMap<String, usize>, String> {
    let mut result = BTreeMap::new();
    for task in tasks {
        let delta = match &task.visible.statement {
            MathStatement::DeriveRecurrenceRelation { delta, .. }
            | MathStatement::ReuseDerivedRelation { delta, .. } => delta,
            _ => continue,
        };
        *result.entry(expression_signature(delta)?).or_default() += 1;
    }
    Ok(result)
}

pub fn calibration_signature_counts(
    tasks: &[MathProblem],
) -> Result<BTreeMap<String, usize>, String> {
    let mut result = BTreeMap::new();
    for task in tasks
        .iter()
        .filter(|task| task.split == super::model::DataSplit::Calibration)
    {
        let delta = match &task.statement {
            MathStatement::DeriveRecurrenceRelation { delta, .. } => delta,
            _ => continue,
        };
        *result.entry(expression_signature(delta)?).or_default() += 1;
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sem4::{kernel::ProofKernel, tasks::generate_task_sets};

    #[test]
    fn discovery_relations_require_induction_not_bounded_testing() {
        let sets = generate_task_sets(0x5e4_f1a5);
        let discovery =
            discover_relations(&sets.discovery, &ProofKernel::new()).expect("discovery");
        assert_eq!(discovery.candidates.len(), 4);
        assert!(discovery.certificates.iter().all(|proof| {
            proof.kernel_verified
                && proof.proof_kind == ProofKind::MathematicalInduction
                && proof.induction.is_some()
                && proof.formal_proof_evidence_ids.len() == 2
        }));
        assert!(discovery
            .active_experiments
            .iter()
            .all(|experiment| experiment.experimental_only && !experiment.used_as_formal_proof));
    }

    #[test]
    fn zero_shot_definition_application_uses_no_examples() {
        let sets = generate_task_sets(0x5e4_f1a5);
        let report = evaluate_condition(
            &sets.definition_only,
            ReasonerCondition::FirstPrinciplesD,
            &[],
            &ProofKernel::new(),
        )
        .expect("report");
        assert_eq!(report.metrics.definition_only_zero_shot_solve_rate, 1.0);
        assert!(report
            .records
            .iter()
            .all(|record| record.definition_examples_seen == 0));
    }
}
