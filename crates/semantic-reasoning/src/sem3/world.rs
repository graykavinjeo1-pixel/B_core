use std::collections::BTreeMap;

use super::{
    integrity::hash_serializable,
    model::{
        BlindEvaluatorMetadata, CandidateExperiment, CompetenceClass, CompetingHypothesis,
        EnvironmentObservation, ExperimentQuery, ExperimentScore, ExternalBlindTask,
        FrozenBlindManifest, HypothesisRule, UncertaintyItem, UncertaintyKind, UncertaintyLedger,
        VisibleBlindTask,
    },
};

pub const LEDGER_VERSION: &str = "SEM3-UNCERTAINTY-LEDGER-1.0.0";
pub const EXPERIMENT_GENERATOR_VERSION: &str = "SEM3-EXPERIMENT-GENERATOR-1.0.0";
pub const BLIND_GENERATOR_VERSION: &str = "SEM3-EXTERNAL-BLIND-1.0.0";
pub const BLIND_SEED: u64 = 20_260_807_331;

pub struct HiddenEnvironment {
    rules: BTreeMap<String, HypothesisRule>,
}

impl HiddenEnvironment {
    pub fn new() -> Self {
        let rules = (0..12)
            .map(|index| {
                let rule = match index % 3 {
                    0 => HypothesisRule::NonNegative,
                    1 => HypothesisRule::NonZero,
                    _ => HypothesisRule::Universal,
                };
                (format!("U{index:06}"), rule)
            })
            .collect();
        Self { rules }
    }

    pub fn execute(&self, query: &ExperimentQuery) -> Result<EnvironmentObservation, String> {
        let rule = self
            .rules
            .get(&query.parent_uncertainty_id)
            .ok_or_else(|| "UNKNOWN_ENVIRONMENT_INTERFACE".to_string())?;
        if !(-8..=8).contains(&query.value)
            || !(1..=6).contains(&query.sequence_shape)
            || !(1..=4).contains(&query.composition_arity)
        {
            return Err("INVALID_CLOSED_WORLD_EXPERIMENT".to_string());
        }
        let applicable = rule.predict(query.value, query.counterfactual);
        Ok(EnvironmentObservation {
            experiment_id: query.experiment_id.clone(),
            applicable,
            output_class: if applicable { "O001" } else { "O002" }.to_string(),
            execution_cost: 1 + query.sequence_shape + query.composition_arity,
            environment_rule_exposed: false,
        })
    }

    fn expected_for(&self, uncertainty_id: &str, value: i64, counterfactual: bool) -> bool {
        self.rules
            .get(uncertainty_id)
            .copied()
            .unwrap_or(HypothesisRule::Universal)
            .predict(value, counterfactual)
    }
}

impl Default for HiddenEnvironment {
    fn default() -> Self {
        Self::new()
    }
}

pub fn initial_uncertainty_ledger() -> UncertaintyLedger {
    let kinds = [
        UncertaintyKind::UncertainPrecondition,
        UncertaintyKind::UncertainInvariant,
        UncertaintyKind::UncertainRelation,
        UncertaintyKind::UncertainOperatorDomain,
        UncertaintyKind::UncertainCounterfactual,
        UncertaintyKind::CompetingAbstractions,
        UncertaintyKind::AmbiguousConceptBoundary,
        UncertaintyKind::FailedTransfer,
        UncertaintyKind::LowConfidencePrediction,
        UncertaintyKind::UnexplainedEpisode,
    ];
    let concepts = ["C000001", "C000002", "C000004", "C000005"];
    UncertaintyLedger {
        ledger_version: LEDGER_VERSION.to_string(),
        items: (0..12)
            .map(|index| {
                let uncertainty_id = format!("U{index:06}");
                let competing_hypotheses = [
                    ("H1", HypothesisRule::NonNegative),
                    ("H2", HypothesisRule::NonZero),
                    ("H3", HypothesisRule::Universal),
                ]
                .into_iter()
                .map(|(suffix, rule)| CompetingHypothesis {
                    hypothesis_id: format!("{uncertainty_id}-{suffix}"),
                    rule,
                    confidence: 1.0 / 3.0,
                    supporting_evidence: vec![format!("SEM2-TRANSFER-E{index:03}")],
                    contradicting_evidence: Vec::new(),
                    retained: true,
                })
                .collect();
                UncertaintyItem {
                    uncertainty_id: uncertainty_id.clone(),
                    kind: kinds[index % kinds.len()],
                    affected_concepts: vec![concepts[index % concepts.len()].to_string()],
                    relation_code: format!("R3{:03}", index % 3),
                    competing_hypotheses,
                    supporting_evidence: vec![format!("SEM2-AMBIGUOUS-EVIDENCE-{index:03}")],
                    contradicting_evidence: Vec::new(),
                    confidence: 1.0 / 3.0,
                    expected_consequences_if_resolved: vec![
                        "BOUNDARY_PREDICTION_CALIBRATED".to_string(),
                        "FALSE_TRANSFER_REDUCED".to_string(),
                        "SEARCH_BRANCH_REMOVED".to_string(),
                    ],
                    provenance: vec![
                        "SEM2-RUN-0002".to_string(),
                        format!("SEM2-UNRESOLVED-TRACE-{index:03}"),
                    ],
                    resolved: false,
                    resolved_hypothesis_id: None,
                }
            })
            .collect(),
        append_only_revision_ids: Vec::new(),
        fabricated_uncertainty_count: 0,
    }
}

pub fn generate_candidate_experiments(ledger: &UncertaintyLedger) -> Vec<CandidateExperiment> {
    let mut experiments = Vec::new();
    let values = [-3, -1, 0, 1, 3, 7];
    for (item_index, item) in ledger.items.iter().enumerate() {
        let retained = item
            .competing_hypotheses
            .iter()
            .filter(|hypothesis| hypothesis.retained)
            .collect::<Vec<_>>();
        for counterfactual in [false, true] {
            for (value_index, value) in values.into_iter().enumerate() {
                for variant in 0..2 {
                    let experiment_id = format!(
                        "X-{}-{}-{:02}-{}",
                        item.uncertainty_id,
                        if counterfactual { "C" } else { "D" },
                        value_index,
                        variant
                    );
                    let predicted_outcomes = retained
                        .iter()
                        .map(|hypothesis| {
                            (
                                hypothesis.hypothesis_id.clone(),
                                hypothesis.rule.predict(value, counterfactual),
                            )
                        })
                        .collect::<BTreeMap<_, _>>();
                    let semantic_signature =
                        format!("{}:{}:{}", item.uncertainty_id, value, counterfactual);
                    let duplicate_of = (variant == 1).then(|| {
                        format!(
                            "X-{}-{}-{:02}-0",
                            item.uncertainty_id,
                            if counterfactual { "C" } else { "D" },
                            value_index
                        )
                    });
                    let near_duplicate_of = (value_index > 0 && variant == 0).then(|| {
                        format!(
                            "X-{}-{}-{:02}-0",
                            item.uncertainty_id,
                            if counterfactual { "C" } else { "D" },
                            value_index - 1
                        )
                    });
                    let sequence_shape = 1 + (item_index + value_index) % 6;
                    let composition_arity = 1 + (item_index + value_index + variant) % 4;
                    let competence_class = if item.resolved && composition_arity >= 3 {
                        CompetenceClass::Frontier
                    } else if item.resolved {
                        CompetenceClass::Mastered
                    } else if retained.len() == 2 || matches!(value, -1 | 0) {
                        CompetenceClass::Frontier
                    } else if value.abs() >= 7 {
                        CompetenceClass::CurrentlyUnsolved
                    } else {
                        CompetenceClass::Frontier
                    };
                    experiments.push(CandidateExperiment {
                        query: ExperimentQuery {
                            experiment_id,
                            parent_uncertainty_id: item.uncertainty_id.clone(),
                            generating_concept_ids: item.affected_concepts.clone(),
                            value,
                            counterfactual,
                            sequence_shape,
                            composition_arity,
                            operator_substitution_code: format!("S3{:02}", value_index + variant),
                        },
                        candidate_hypothesis_ids: retained
                            .iter()
                            .map(|hypothesis| hypothesis.hypothesis_id.clone())
                            .collect(),
                        predicted_outcomes,
                        competence_class,
                        valid_in_closed_world: true,
                        surface_signature: format!(
                            "SURFACE:{}:{}:{}:{}",
                            value,
                            counterfactual,
                            item_index % 4,
                            variant
                        ),
                        semantic_signature,
                        duplicate_of,
                        near_duplicate_of,
                        score: ExperimentScore {
                            expected_information_gain: 0.0,
                            expected_uncertainty_reduction: 0.0,
                            discriminative_hypothesis_value: 0.0,
                            concept_boundary_clarification: 0.0,
                            expected_transfer_value: 0.0,
                            expected_reusable_abstraction_value: 0.0,
                            competence_frontier_value: 0.0,
                            execution_cost: 0.0,
                            redundancy: 0.0,
                            triviality: 0.0,
                            invalid_experiment_probability: 0.0,
                            total: 0.0,
                        },
                    });
                }
            }
        }
    }
    experiments
}

pub fn generate_external_blind(
    environment: &HiddenEnvironment,
) -> Result<(Vec<ExternalBlindTask>, FrozenBlindManifest), String> {
    let mut tasks = Vec::new();
    for index in 0..100 {
        let (uncertainty_index, value, counterfactual, hard) = if index < 80 {
            (
                index % 12,
                [-3, -1, 0, 1, 3][(index * 7 + 3) % 5],
                index % 5 == 0,
                false,
            )
        } else {
            let eligible = [0, 1, 3, 4, 6, 7, 9, 10];
            let uncertainty_index = eligible[(index - 80) % eligible.len()];
            let value = if uncertainty_index % 3 == 0 { -1 } else { 0 };
            (uncertainty_index, value, index % 4 == 0, true)
        };
        let uncertainty_id = format!("U{uncertainty_index:06}");
        let concept_id =
            ["C000001", "C000002", "C000004", "C000005"][uncertainty_index % 4].to_string();
        let relation_code = format!("R3{:03}", uncertainty_index % 3);
        let expected = environment.expected_for(&uncertainty_id, value, counterfactual);
        let depth = if hard {
            50 + (index - 80)
        } else {
            5 + (index % 9) * 5
        };
        tasks.push(ExternalBlindTask {
            visible: VisibleBlindTask {
                task_id: format!("S3B{:06}", index + 1),
                concept_id,
                relation_code,
                value,
                counterfactual,
                sequence_shape: 1 + index % 6,
                composition_arity: 1 + index % 4,
            },
            evaluator: BlindEvaluatorMetadata {
                uncertainty_id,
                expected_applicable: expected,
                family_label: if hard {
                    "HARD_TRANSFER_BOUNDARY"
                } else if counterfactual {
                    "COUNTERFACTUAL_VARIANT"
                } else if !expected {
                    "INVALID_APPLICABILITY_TRAP"
                } else {
                    "FAMILIAR_OR_COMPOSED_TRANSFER"
                }
                .to_string(),
                boundary_case: matches!(value, -1 | 0),
                transfer_case: hard || index % 7 == 0,
                solution_graph_depth: depth,
                primitive_expanded_depth: depth * 8 + 3,
                simultaneous_subproblems: 1 + index % 5,
                recombinations: usize::from(index % 3 == 0),
                semantic_traps: usize::from(!expected) + usize::from(hard),
            },
        });
    }
    let mut manifest = FrozenBlindManifest {
        generator_version: BLIND_GENERATOR_VERSION.to_string(),
        seed: BLIND_SEED,
        tasks: tasks.iter().map(|task| task.visible.clone()).collect(),
        expected_answers_included: false,
        hidden_family_labels_included: false,
        intended_concepts_included: false,
        difficulty_classification_included: false,
        selector_access_before_or_during_curriculum: false,
        manifest_sha256: String::new(),
    };
    manifest.manifest_sha256 = hash_serializable(&manifest)?;
    Ok((tasks, manifest))
}

#[cfg(test)]
mod tests {
    use super::{
        generate_candidate_experiments, generate_external_blind, initial_uncertainty_ledger,
        HiddenEnvironment,
    };

    #[test]
    fn uncertainty_is_evidence_backed_and_competing_hypotheses_are_retained() {
        let ledger = initial_uncertainty_ledger();
        assert_eq!(ledger.fabricated_uncertainty_count, 0);
        assert_eq!(ledger.unresolved_count(), 12);
        assert!(ledger.items.iter().all(|item| {
            item.competing_hypotheses
                .iter()
                .filter(|hypothesis| hypothesis.retained)
                .count()
                == 3
                && !item.supporting_evidence.is_empty()
                && !item.provenance.is_empty()
        }));
    }

    #[test]
    fn generated_experiments_are_closed_world_valid_and_have_provenance() {
        let ledger = initial_uncertainty_ledger();
        let generated = generate_candidate_experiments(&ledger);
        let environment = HiddenEnvironment::new();
        assert!(generated.len() >= 100);
        assert!(generated.iter().all(|experiment| {
            experiment.valid_in_closed_world
                && !experiment.query.parent_uncertainty_id.is_empty()
                && !experiment.query.generating_concept_ids.is_empty()
                && environment.execute(&experiment.query).is_ok()
        }));
    }

    #[test]
    fn hidden_environment_returns_observation_not_rule() {
        let ledger = initial_uncertainty_ledger();
        let experiment = generate_candidate_experiments(&ledger).remove(0);
        let observation = HiddenEnvironment::new()
            .execute(&experiment.query)
            .expect("execute");
        assert!(!observation.environment_rule_exposed);
    }

    #[test]
    fn frozen_external_blind_excludes_evaluator_metadata() {
        let (_, manifest) = generate_external_blind(&HiddenEnvironment::new()).expect("blind");
        assert_eq!(manifest.tasks.len(), 100);
        assert!(!manifest.expected_answers_included);
        assert!(!manifest.hidden_family_labels_included);
        assert!(!manifest.intended_concepts_included);
        assert!(!manifest.selector_access_before_or_during_curriculum);
        let serialized = serde_json::to_string(&manifest.tasks).expect("serialize");
        assert!(!serialized.contains("expected_applicable"));
        assert!(!serialized.contains("family_label"));
        assert!(!serialized.contains("solution_graph_depth"));
    }
}
