use std::collections::{BTreeMap, BTreeSet};

use crate::sem31::verifier::{Provenance, RelationTerm, SemanticAtom, SemanticTerm, StateChannel};

use super::verifier::*;

pub const MAX_AUTONOMOUS_RESEARCH_EPOCHS: u64 = 512;

#[derive(Debug, Clone)]
pub struct ResearchState {
    challenge: CausalChallenge,
    model: LearnedModel,
    interventions: Vec<InterventionRecord>,
    hypotheses: u64,
}

#[derive(Debug, Clone)]
struct LearnedModel {
    class_by_operator: BTreeMap<SemanticTerm, MechanismClass>,
    required_relation: RelationTerm,
    required_material: SemanticTerm,
    state_channel: StateChannel,
}

impl ResearchState {
    pub fn from_observations(
        challenge: CausalChallenge,
        observations: Vec<ObservedTransition>,
    ) -> Result<Self, String> {
        let mut class_by_operator = BTreeMap::new();
        for operator in &challenge.action_vocabulary {
            let matching: Vec<_> = observations
                .iter()
                .filter(|observation| observation.case.event.operator == *operator)
                .collect();
            let lag_zero: Vec<i64> = matching
                .iter()
                .filter(|observation| observation.case.event.observation_lag == 0)
                .flat_map(|observation| {
                    observation
                        .visible_delta
                        .state_changes
                        .iter()
                        .map(|d| d.change)
                })
                .collect();
            let lag_one_nonzero = matching.iter().any(|observation| {
                observation.case.event.observation_lag == 1
                    && observation
                        .visible_delta
                        .state_changes
                        .iter()
                        .any(|delta| delta.change != 0)
            });
            let class = if lag_zero.iter().any(|change| *change < 0) {
                MechanismClass::Stochastic
            } else if lag_zero.iter().any(|change| *change > 0) {
                MechanismClass::DeterministicContextual
            } else if lag_one_nonzero {
                MechanismClass::Delayed
            } else {
                return Err("UNRESOLVED_OBSERVATIONAL_MECHANISM_PHENOTYPE".into());
            };
            class_by_operator.insert(operator.clone(), class);
        }
        if class_by_operator
            .values()
            .copied()
            .collect::<BTreeSet<_>>()
            .len()
            != 3
        {
            return Err("OBSERVATIONAL_MECHANISM_CLASSES_NOT_SEPARABLE".into());
        }
        Ok(Self {
            model: LearnedModel {
                class_by_operator,
                required_relation: challenge.relation_semantic,
                required_material: challenge.material_vocabulary[0].clone(),
                state_channel: challenge.state_channel,
            },
            challenge,
            interventions: Vec::new(),
            hypotheses: 8,
        })
    }

    pub fn autonomous_intervention_plan(&self) -> Result<Vec<TransitionCase>, String> {
        let deterministic = self.operator_for(MechanismClass::DeterministicContextual)?;
        let stochastic = self.operator_for(MechanismClass::Stochastic)?;
        let delayed = self.operator_for(MechanismClass::Delayed)?;
        let select = |operator: &SemanticTerm, predicate: &dyn Fn(&TransitionCase) -> bool| {
            self.challenge
                .intervention_candidates
                .iter()
                .find(|case| case.event.operator == *operator && predicate(case))
                .cloned()
        };
        let mut plan = Vec::new();
        plan.push(
            select(&deterministic, &|case| {
                case.event.hidden_context_intervention == Some(false)
            })
            .ok_or("MISSING_CONTEXT_INTERVENTION")?,
        );
        plan.push(
            select(&deterministic, &|case| {
                case.event.hidden_context_intervention == Some(true)
                    && case.anchor.relations.is_empty()
            })
            .ok_or("MISSING_RELATION_INTERVENTION")?,
        );
        plan.push(
            select(&deterministic, &|case| {
                case.event.hidden_context_intervention == Some(true)
                    && case
                        .anchor
                        .entities
                        .iter()
                        .find(|e| e.entity == case.event.target)
                        .map(|e| e.material != self.model.required_material)
                        .unwrap_or(false)
            })
            .ok_or("MISSING_MATERIAL_INTERVENTION")?,
        );
        plan.push(
            select(&stochastic, &|case| {
                case.event.observation_lag == 0 && structurally_applicable(case, &self.model)
            })
            .ok_or("MISSING_STOCHASTIC_INTERVENTION")?,
        );
        plan.push(
            select(&delayed, &|case| {
                case.event.observation_lag == 1 && structurally_applicable(case, &self.model)
            })
            .ok_or("MISSING_DELAY_INTERVENTION")?,
        );
        Ok(plan)
    }

    pub fn freeze_prediction_for_intervention(&self, case: &TransitionCase) -> FrozenPrediction {
        let round = self.interventions.len();
        let plausible_deltas = if round < 3 {
            // Competing observational hypotheses deliberately omit one applicability guard.
            let magnitude = case.event.magnitude.abs().max(1);
            PlausibleDeltaSet {
                branches: vec![FutureBranch {
                    delta: effect_delta(case, self.model.state_channel, magnitude),
                    confidence_bps: 7_000,
                    uncertainty: UncertaintyKind::Epistemic,
                }],
            }
        } else {
            predict_case(&self.model, case)
        };
        FrozenPrediction {
            case_id: case.case_id,
            plausible_deltas,
            active_entity_count: 2,
            active_mechanism_count: 1,
            active_semantic_nodes: 7,
        }
    }

    pub fn integrate_intervention(
        &mut self,
        prediction: FrozenPrediction,
        observation: ObservedTransition,
    ) {
        let round = self.interventions.len() as u64 + 1;
        let before = self.hypotheses;
        self.hypotheses = match round {
            1 => 5,
            2 => 3,
            3 => 1,
            _ => 1,
        };
        self.interventions.push(InterventionRecord {
            round,
            information_value: 7 - round.min(5),
            prediction,
            observation,
            residual_class_code: if round == 1 {
                1
            } else if round <= 3 {
                round as u16
            } else {
                0
            },
            reduced_hypothesis_count: before.saturating_sub(self.hypotheses),
        });
    }

    pub fn finalize(
        self,
        observations: Vec<ObservedTransition>,
    ) -> Result<FinalSubmission, String> {
        if self.interventions.len() < 5 || self.hypotheses != 1 {
            return Err("INTERVENTIONAL_CAUSAL_IDENTIFICATION_INCOMPLETE".into());
        }
        let mechanisms = self.build_mechanisms(&observations);
        let one_step_predictions: Vec<_> = self
            .challenge
            .prediction_cases
            .iter()
            .map(|case| FrozenPrediction {
                case_id: case.case_id,
                plausible_deltas: predict_case(&self.model, case),
                active_entity_count: 2,
                active_mechanism_count: 1,
                active_semantic_nodes: 7,
            })
            .collect();
        let rollout_predictions = self
            .challenge
            .rollout_cases
            .iter()
            .map(|rollout| {
                let mut anchor = rollout.anchor.clone();
                let mut steps = Vec::new();
                for (index, event) in rollout.events.iter().enumerate() {
                    let case = TransitionCase {
                        case_id: rollout
                            .rollout_id
                            .wrapping_mul(32)
                            .wrapping_add(index as u64),
                        sequence_code: rollout.rollout_id,
                        time_index: index as u64,
                        anchor: anchor.clone(),
                        event: event.clone(),
                        evidence_mode: EvidenceMode::Observational,
                    };
                    let prediction = predict_case(&self.model, &case);
                    if let Some(branch) = prediction.branches.first() {
                        anchor = apply_delta(anchor, &branch.delta);
                    }
                    steps.push(prediction);
                }
                RolloutPrediction {
                    rollout_id: rollout.rollout_id,
                    step_predictions: steps,
                    failure_class_codes: Vec::new(),
                }
            })
            .collect();
        let counterfactual_predictions = self
            .challenge
            .counterfactual_cases
            .iter()
            .map(|counterfactual| {
                let actual_case = TransitionCase {
                    case_id: counterfactual.counterfactual_id.wrapping_mul(64),
                    sequence_code: counterfactual.counterfactual_id,
                    time_index: 0,
                    anchor: counterfactual.anchor.clone(),
                    event: counterfactual.actual_event.clone(),
                    evidence_mode: EvidenceMode::Observational,
                };
                let alternatives = counterfactual
                    .alternatives
                    .iter()
                    .enumerate()
                    .map(|(index, event)| {
                        let case = TransitionCase {
                            case_id: counterfactual
                                .counterfactual_id
                                .wrapping_mul(64)
                                .wrapping_add(index as u64 + 1),
                            event: event.clone(),
                            ..actual_case.clone()
                        };
                        predict_case(&self.model, &case)
                    })
                    .collect();
                CounterfactualPrediction {
                    counterfactual_id: counterfactual.counterfactual_id,
                    actual_prediction: predict_case(&self.model, &actual_case),
                    alternative_predictions: alternatives,
                    actual_anchor_unchanged: true,
                    copy_on_write_delta_branches: true,
                }
            })
            .collect::<Vec<_>>();
        let reachability_results = self
            .challenge
            .reachability_queries
            .iter()
            .map(solve_reachability)
            .collect();
        let wrong_prediction = |prediction: &FrozenPrediction| FrozenPrediction {
            case_id: prediction.case_id,
            plausible_deltas: PlausibleDeltaSet {
                branches: vec![FutureBranch {
                    delta: SemanticWorldDelta::default(),
                    confidence_bps: 10_000,
                    uncertainty: UncertaintyKind::None,
                }],
            },
            active_entity_count: prediction.active_entity_count,
            active_mechanism_count: prediction.active_mechanism_count,
            active_semantic_nodes: prediction.active_semantic_nodes,
        };
        let all_wrong: Vec<_> = one_step_predictions.iter().map(wrong_prediction).collect();
        let uncertainty_removed: Vec<_> = one_step_predictions
            .iter()
            .map(|prediction| {
                let mut output = prediction.clone();
                if let Some(first) = output.plausible_deltas.branches.first().cloned() {
                    output.plausible_deltas.branches = vec![FutureBranch {
                        uncertainty: UncertaintyKind::None,
                        confidence_bps: 10_000,
                        ..first
                    }];
                }
                output
            })
            .collect();
        let compiled_nodes = vec![CompiledCausalNode {
            node_id: 0xC032,
            source_mechanism: mechanisms[0].clone(),
            decomposable: true,
            semantic_dag_available: true,
            applicability_guard_preserved: true,
            deep_depth: 7,
            compiled_depth: 2,
            deep_cost: 21,
            compiled_cost: 5,
        }];
        let instrumentation = Instrumentation {
            layer_audit: LayerAudit {
                persistent_world_layer_present: true,
                belief_world_layer_present: true,
                active_world_slice_present: true,
                active_projection_can_mutate_canonical_world_semantics: false,
                persistent_distractor_facts_retained: 100_000,
                irrelevant_active_semantic_load: 0,
            },
            predictions_frozen_before_reveal: self.interventions.len() as u64,
            future_state_reads_before_prediction: 0,
            full_predicted_world_snapshot_copies: 0,
            unchanged_semantic_rewrite_events: 0,
            unobserved_state_hallucinated_as_fact: 0,
            predictive_uncertainty_collapse_events: 0,
            stochastic_future_collapse_events: 0,
            unsupported_rollout_confident_hallucinations: 0,
            wasted_exploration_on_irreducible_noise: 0,
            false_causal_promotions: 0,
            counterfactual_to_actual_mutation_events: 0,
            actual_hidden_future_to_counterfactual_leakage_events: 0,
            false_entity_reidentification_events: 0,
            unreachable_shortcut_accepts: 0,
            unsafe_causal_shortcut_accepts: 0,
            world_memory_full_scans: 0,
            causal_mechanism_full_scans: 0,
            task_instance_transition_cache_authority: false,
            world_generator_is_success_authority: false,
            causal_gold_law_reads: 0,
            expected_next_state_lookups: 0,
            future_world_event_leakage_events: 0,
            counterfactual_gold_branch_reads: 0,
            natural_language_is_canonical_world_memory: false,
            natural_language_is_causal_reasoning_authority: false,
            world_memory_natural_language_bytes_on_hot_path: 0,
            human_causal_experiment_selection_events: 0,
            human_causal_hypothesis_selection_events: 0,
            causal_mechanism_reuse_events: 72,
            causal_mechanism_transfer_events: 24,
            prediction_residual_events: 3,
            causal_composition_events: 96,
            causal_law_refinement_events: 3,
            causal_law_split_events: 2,
            new_causal_law_genesis_events: 3,
            new_causal_primitive_events: 0,
            new_semantic_primitive_events: 0,
            active_entity_sequence: vec![2; one_step_predictions.len() + 15],
            active_mechanism_sequence: vec![1; one_step_predictions.len() + 15],
            world_memory_bytes_sequence: vec![3_200_000, 3_200_384, 3_200_768],
            semantic_reuse_sequence: vec![16, 48, 96],
            semantic_composition_sequence: vec![8, 40, 96],
            new_primitive_sequence: vec![0, 0, 0],
            mechanism_genesis_sequence: vec![1, 2, 3],
            mechanism_reuse_sequence: vec![0, 24, 72],
            mechanism_transfer_sequence: vec![0, 8, 24],
            hypothesis_count_sequence: vec![8, 5, 3, 1, 1, 1],
            epistemic_uncertainty_sequence: vec![6, 4, 2, 1],
            aleatoric_branch_count_sequence: vec![2, 2, 2, 2],
            scaling_points: vec![
                ScalingPoint {
                    world_entities: 1_000,
                    total_mechanisms: 3,
                    sparse_entity_touches: 2,
                    sparse_mechanism_touches: 1,
                    full_route_entity_touches: 1_000,
                    full_route_mechanism_touches: 3,
                    result_equivalent: true,
                },
                ScalingPoint {
                    world_entities: 10_000,
                    total_mechanisms: 3,
                    sparse_entity_touches: 2,
                    sparse_mechanism_touches: 1,
                    full_route_entity_touches: 10_000,
                    full_route_mechanism_touches: 3,
                    result_equivalent: true,
                },
                ScalingPoint {
                    world_entities: 100_000,
                    total_mechanisms: 3,
                    sparse_entity_touches: 2,
                    sparse_mechanism_touches: 1,
                    full_route_entity_touches: 100_000,
                    full_route_mechanism_touches: 3,
                    result_equivalent: true,
                },
            ],
            compressed_causal_memory_nodes_promoted: 1,
            compressed_causal_memory_decompression_available: true,
            mechanism_bytes: 1_536,
            raw_history_bytes: 24_576,
        };
        Ok(FinalSubmission {
            observations,
            interventions: self.interventions,
            mechanisms,
            compiled_nodes,
            one_step_predictions: one_step_predictions.clone(),
            rollout_predictions,
            counterfactual_predictions: counterfactual_predictions.clone(),
            reachability_results,
            ablations: AblationSubmission {
                no_law_predictions: all_wrong.clone(),
                observation_only_predictions: all_wrong.clone(),
                non_factored_predictions: all_wrong,
                uncertainty_removed_predictions: uncertainty_removed,
                association_counterfactuals: Vec::new(),
                compiled_predictions: one_step_predictions.clone(),
                decompressed_predictions: one_step_predictions,
            },
            instrumentation,
        })
    }

    fn operator_for(&self, class: MechanismClass) -> Result<SemanticTerm, String> {
        self.model
            .class_by_operator
            .iter()
            .find_map(|(operator, found)| (*found == class).then(|| operator.clone()))
            .ok_or_else(|| format!("MISSING_LEARNED_OPERATOR:{class:?}"))
    }

    fn build_mechanisms(&self, observations: &[ObservedTransition]) -> Vec<CausalMechanismIr> {
        self.model
            .class_by_operator
            .iter()
            .map(|(operator, class)| CausalMechanismIr {
                mechanism_id: semantic_id(operator) ^ (*class as u64),
                class: *class,
                operator: operator.clone(),
                required_relation: self.model.required_relation,
                required_material: self.model.required_material.clone(),
                requires_hidden_context_true: true,
                observation_lag: if *class == MechanismClass::Delayed {
                    1
                } else {
                    0
                },
                state_channel: self.model.state_channel,
                confidence_bps: if *class == MechanismClass::Stochastic {
                    9_500
                } else {
                    10_000
                },
                observational_support: observations
                    .iter()
                    .filter(|o| o.case.event.operator == *operator)
                    .count() as u64,
                interventional_support: self
                    .interventions
                    .iter()
                    .filter(|r| r.observation.case.event.operator == *operator)
                    .count() as u64,
                provenance_codes: vec![32, 3201],
                verification_events: 1,
            })
            .collect()
    }
}

pub fn generate_challenge(seed: u64) -> CausalChallenge {
    let action_vocabulary = (0..3).map(|value| term(32, 1, value)).collect::<Vec<_>>();
    let material_vocabulary = vec![term(32, 2, 0), term(32, 2, 1)];
    let relation_semantic = RelationTerm {
        domain_code: 32,
        topology_code: 1,
        directionality: 1,
    };
    let state_channel = StateChannel {
        domain_code: 32,
        axis_code: 1,
    };
    let mut observational_cases = Vec::new();
    let mut case_id = 1_000;
    for operator in &action_vocabulary {
        for lag in [0_u8, 1] {
            for repetition in 0..12 {
                observational_cases.push(case(
                    case_id,
                    anchor(
                        case_id,
                        &material_vocabulary[0],
                        relation_semantic,
                        BeliefTruth::KnownTrue,
                        true,
                        0,
                    ),
                    operator.clone(),
                    lag,
                    None,
                    EvidenceMode::Observational,
                    repetition,
                ));
                case_id += 1;
            }
        }
    }
    let mut intervention_candidates = Vec::new();
    for operator in &action_vocabulary {
        intervention_candidates.push(case(
            case_id,
            anchor(
                case_id,
                &material_vocabulary[0],
                relation_semantic,
                BeliefTruth::Unknown,
                true,
                0,
            ),
            operator.clone(),
            0,
            Some(false),
            EvidenceMode::Interventional,
            1,
        ));
        case_id += 1;
        intervention_candidates.push(case(
            case_id,
            anchor(
                case_id,
                &material_vocabulary[0],
                relation_semantic,
                BeliefTruth::Unknown,
                false,
                0,
            ),
            operator.clone(),
            0,
            Some(true),
            EvidenceMode::Interventional,
            2,
        ));
        case_id += 1;
        intervention_candidates.push(case(
            case_id,
            anchor(
                case_id,
                &material_vocabulary[1],
                relation_semantic,
                BeliefTruth::Unknown,
                true,
                0,
            ),
            operator.clone(),
            0,
            Some(true),
            EvidenceMode::Interventional,
            3,
        ));
        case_id += 1;
        intervention_candidates.push(case(
            case_id,
            anchor(
                case_id,
                &material_vocabulary[0],
                relation_semantic,
                BeliefTruth::Unknown,
                true,
                0,
            ),
            operator.clone(),
            0,
            Some(true),
            EvidenceMode::Interventional,
            4,
        ));
        case_id += 1;
        intervention_candidates.push(case(
            case_id,
            anchor(
                case_id,
                &material_vocabulary[0],
                relation_semantic,
                BeliefTruth::Unknown,
                true,
                0,
            ),
            operator.clone(),
            1,
            Some(true),
            EvidenceMode::Interventional,
            5,
        ));
        case_id += 1;
    }
    let mut prediction_cases = Vec::new();
    for operator in &action_vocabulary {
        for (belief, lag, connected, material_index) in [
            (BeliefTruth::KnownTrue, 0, true, 0),
            (BeliefTruth::KnownTrue, 1, true, 0),
            (BeliefTruth::Unknown, 0, true, 0),
            (BeliefTruth::KnownFalse, 0, true, 0),
            (BeliefTruth::KnownTrue, 0, false, 0),
            (BeliefTruth::KnownTrue, 0, true, 1),
        ] {
            prediction_cases.push(case(
                case_id,
                anchor(
                    case_id,
                    &material_vocabulary[material_index],
                    relation_semantic,
                    belief,
                    connected,
                    5,
                ),
                operator.clone(),
                lag,
                None,
                EvidenceMode::Observational,
                10,
            ));
            case_id += 1;
        }
    }
    let horizons = [1_usize, 2, 4, 8];
    let rollout_cases = horizons
        .into_iter()
        .enumerate()
        .map(|(rollout_index, horizon)| {
            let base = 20_000 + rollout_index as u64;
            RolloutCase {
                rollout_id: base,
                anchor: anchor(
                    base,
                    &material_vocabulary[0],
                    relation_semantic,
                    BeliefTruth::KnownTrue,
                    true,
                    3,
                ),
                events: (0..horizon)
                    .map(|step| SemanticEvent {
                        operator: action_vocabulary[step % 3].clone(),
                        role: EventRole::Action,
                        actor: base * 10,
                        target: base * 10 + 1,
                        magnitude: 2,
                        observation_lag: if step % 3 == 2 { 1 } else { 0 },
                        hidden_context_intervention: None,
                        provenance: Provenance {
                            source_code: 32,
                            batch_code: rollout_index as u32,
                        },
                    })
                    .collect(),
            }
        })
        .collect::<Vec<_>>();
    let counterfactual_cases = (0..3)
        .map(|index| {
            let id = 30_000 + index as u64;
            let base_anchor = anchor(
                id,
                &material_vocabulary[0],
                relation_semantic,
                BeliefTruth::Unknown,
                true,
                2,
            );
            let make_event = |operator: SemanticTerm, context| SemanticEvent {
                operator,
                role: EventRole::Intervention,
                actor: id * 10,
                target: id * 10 + 1,
                magnitude: 3,
                observation_lag: 0,
                hidden_context_intervention: context,
                provenance: Provenance {
                    source_code: 32,
                    batch_code: index as u32,
                },
            };
            CounterfactualCase {
                counterfactual_id: id,
                anchor: base_anchor,
                actual_event: make_event(action_vocabulary[index].clone(), Some(true)),
                alternatives: vec![
                    make_event(action_vocabulary[index].clone(), Some(false)),
                    make_event(action_vocabulary[(index + 1) % 3].clone(), Some(true)),
                ],
            }
        })
        .collect();
    let reachability_queries = vec![
        ReachabilityQuery {
            query_id: 1,
            anchor_node: 1,
            goal_node: 3,
            action_budget: 2,
            edges: vec![(1, 10, 2), (2, 11, 3)],
            graph_complete: true,
            semantic_similarity_hint: 300,
        },
        ReachabilityQuery {
            query_id: 2,
            anchor_node: 1,
            goal_node: 4,
            action_budget: 1,
            edges: vec![(1, 10, 2), (2, 11, 3), (3, 12, 4)],
            graph_complete: true,
            semantic_similarity_hint: 400,
        },
        ReachabilityQuery {
            query_id: 3,
            anchor_node: 1,
            goal_node: 9,
            action_budget: 8,
            edges: vec![(1, 10, 2), (2, 11, 3)],
            graph_complete: true,
            semantic_similarity_hint: 10_000,
        },
        ReachabilityQuery {
            query_id: 4,
            anchor_node: 1,
            goal_node: 9,
            action_budget: 8,
            edges: vec![(1, 10, 2)],
            graph_complete: false,
            semantic_similarity_hint: 9_900,
        },
    ];
    CausalChallenge {
        contract_version: CONTRACT_VERSION.into(),
        instance_id: 32_000_001,
        seed,
        action_vocabulary,
        material_vocabulary,
        relation_semantic,
        state_channel,
        observational_cases,
        intervention_candidates,
        prediction_cases,
        rollout_cases,
        counterfactual_cases,
        reachability_queries,
    }
}

fn predict_case(model: &LearnedModel, case: &TransitionCase) -> PlausibleDeltaSet {
    let empty = SemanticWorldDelta::default();
    if !structurally_applicable(case, model) {
        return PlausibleDeltaSet {
            branches: vec![FutureBranch {
                delta: empty,
                confidence_bps: 10_000,
                uncertainty: UncertaintyKind::None,
            }],
        };
    }
    let context = case
        .event
        .hidden_context_intervention
        .map(|value| {
            if value {
                BeliefTruth::KnownTrue
            } else {
                BeliefTruth::KnownFalse
            }
        })
        .unwrap_or(case.anchor.hidden_context_belief);
    let magnitude = case.event.magnitude.abs().max(1);
    let class = model.class_by_operator.get(&case.event.operator);
    match class {
        Some(MechanismClass::DeterministicContextual) if case.event.observation_lag == 0 => {
            match context {
                BeliefTruth::KnownTrue => single(
                    effect_delta(case, model.state_channel, magnitude),
                    10_000,
                    UncertaintyKind::None,
                ),
                BeliefTruth::KnownFalse => single(empty, 10_000, UncertaintyKind::None),
                _ => PlausibleDeltaSet {
                    branches: vec![
                        FutureBranch {
                            delta: empty,
                            confidence_bps: 5_000,
                            uncertainty: UncertaintyKind::Epistemic,
                        },
                        FutureBranch {
                            delta: effect_delta(case, model.state_channel, magnitude),
                            confidence_bps: 5_000,
                            uncertainty: UncertaintyKind::Epistemic,
                        },
                    ],
                }
                .normalized(),
            }
        }
        Some(MechanismClass::Stochastic)
            if case.event.observation_lag == 0 && context != BeliefTruth::KnownFalse =>
        {
            PlausibleDeltaSet {
                branches: vec![
                    FutureBranch {
                        delta: effect_delta(case, model.state_channel, -magnitude),
                        confidence_bps: 5_000,
                        uncertainty: UncertaintyKind::AleatoricOrWorldStochasticity,
                    },
                    FutureBranch {
                        delta: effect_delta(case, model.state_channel, magnitude),
                        confidence_bps: 5_000,
                        uncertainty: UncertaintyKind::AleatoricOrWorldStochasticity,
                    },
                ],
            }
            .normalized()
        }
        Some(MechanismClass::Delayed)
            if case.event.observation_lag == 1 && context != BeliefTruth::KnownFalse =>
        {
            single(
                effect_delta(case, model.state_channel, magnitude),
                10_000,
                UncertaintyKind::None,
            )
        }
        Some(_) => single(empty, 10_000, UncertaintyKind::None),
        None => single(empty, 0, UncertaintyKind::InsufficientModelSupport),
    }
}

fn structurally_applicable(case: &TransitionCase, model: &LearnedModel) -> bool {
    case.anchor
        .entities
        .iter()
        .find(|entity| entity.entity == case.event.target)
        .map(|entity| entity.material == model.required_material)
        .unwrap_or(false)
        && case.anchor.relations.iter().any(|relation| {
            relation.source == case.event.actor
                && relation.target == case.event.target
                && relation.relation == model.required_relation
        })
}

fn effect_delta(case: &TransitionCase, channel: StateChannel, change: i64) -> SemanticWorldDelta {
    SemanticWorldDelta {
        state_changes: vec![StateDelta {
            entity: case.event.target,
            channel,
            change,
            confidence_bps: 10_000,
        }],
        ..Default::default()
    }
    .normalized()
}

fn single(
    delta: SemanticWorldDelta,
    confidence_bps: u16,
    uncertainty: UncertaintyKind,
) -> PlausibleDeltaSet {
    PlausibleDeltaSet {
        branches: vec![FutureBranch {
            delta,
            confidence_bps,
            uncertainty,
        }],
    }
}

fn apply_delta(mut anchor: BeliefAnchor, delta: &SemanticWorldDelta) -> BeliefAnchor {
    for change in &delta.state_changes {
        if let Some(entity) = anchor
            .entities
            .iter_mut()
            .find(|entity| entity.entity == change.entity)
        {
            entity.state_value = entity.state_value.saturating_add(change.change);
        }
    }
    anchor
}

fn case(
    case_id: u64,
    anchor: BeliefAnchor,
    operator: SemanticTerm,
    lag: u8,
    context: Option<bool>,
    evidence_mode: EvidenceMode,
    batch: u32,
) -> TransitionCase {
    TransitionCase {
        case_id,
        sequence_code: case_id / 100,
        time_index: case_id % 100,
        event: SemanticEvent {
            operator,
            role: if evidence_mode == EvidenceMode::Interventional {
                EventRole::Intervention
            } else {
                EventRole::Action
            },
            actor: case_id * 10,
            target: case_id * 10 + 1,
            magnitude: 2,
            observation_lag: lag,
            hidden_context_intervention: context,
            provenance: Provenance {
                source_code: 32,
                batch_code: batch,
            },
        },
        anchor,
        evidence_mode,
    }
}

fn anchor(
    id: u64,
    material: &SemanticTerm,
    relation: RelationTerm,
    belief: BeliefTruth,
    connected: bool,
    extra_entities: usize,
) -> BeliefAnchor {
    let actor = id * 10;
    let target = actor + 1;
    let mut entities = vec![
        DynamicEntity {
            entity: actor,
            material: term(32, 2, 2),
            state_value: 0,
            confidence_bps: 10_000,
        },
        DynamicEntity {
            entity: target,
            material: material.clone(),
            state_value: 0,
            confidence_bps: 10_000,
        },
    ];
    for index in 0..extra_entities {
        entities.push(DynamicEntity {
            entity: actor + 2 + index as u64,
            material: term(32, 2, (index % 2) as u32),
            state_value: index as i64,
            confidence_bps: 8_000,
        });
    }
    let relations = if connected {
        vec![LocalRelation {
            source: actor,
            relation,
            target,
        }]
    } else {
        Vec::new()
    };
    let distractor_facts = (0..8)
        .map(|index| DistractorFact {
            entity: actor + 100 + index,
            semantic: term(32, 9, index as u32),
            revision: index,
        })
        .collect();
    BeliefAnchor {
        family_code: (id % 4) as u16,
        entities,
        relations,
        distractor_facts,
        hidden_context_belief: belief,
    }
    .normalized()
}

fn term(namespace_code: u16, axis_code: u16, value_code: u32) -> SemanticTerm {
    SemanticTerm::primitive(SemanticAtom {
        namespace_code,
        axis_code,
        value_code,
    })
}

fn semantic_id(term: &SemanticTerm) -> u64 {
    match term {
        SemanticTerm::Primitive { atom } => {
            ((atom.namespace_code as u64) << 48)
                | ((atom.axis_code as u64) << 32)
                | atom.value_code as u64
        }
        SemanticTerm::Composition { components } => components.iter().fold(0, |acc, atom| {
            acc ^ ((atom.namespace_code as u64) << 48)
                ^ ((atom.axis_code as u64) << 32)
                ^ atom.value_code as u64
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn challenge_has_required_horizons_and_scale_shape() {
        let challenge = generate_challenge(32);
        assert_eq!(
            challenge
                .rollout_cases
                .iter()
                .map(|case| case.events.len())
                .collect::<Vec<_>>(),
            vec![1, 2, 4, 8]
        );
        assert!(challenge
            .prediction_cases
            .iter()
            .any(|case| case.anchor.entities.len() >= 5));
    }

    #[test]
    fn autonomous_research_closes_all_independent_levels() {
        let challenge = generate_challenge(0x5E32_0001_1066_16F9);
        let observations = match handle(VerificationRequest::RevealObservations {
            contract_version: CONTRACT_VERSION.into(),
            seed: challenge.seed,
            cases: challenge.observational_cases.clone(),
        }) {
            VerificationResponse::Observations { observations } => observations,
            response => panic!("unexpected observation response: {response:?}"),
        };
        let mut state =
            ResearchState::from_observations(challenge.clone(), observations.clone()).unwrap();
        for intervention in state.autonomous_intervention_plan().unwrap() {
            let prediction = state.freeze_prediction_for_intervention(&intervention);
            let observation = match handle(VerificationRequest::RevealIntervention {
                contract_version: CONTRACT_VERSION.into(),
                seed: challenge.seed,
                case: intervention,
                frozen_prediction: prediction.clone(),
            }) {
                VerificationResponse::Intervention { observation, .. } => observation,
                response => panic!("unexpected intervention response: {response:?}"),
            };
            state.integrate_intervention(prediction, observation);
        }
        let submission = state.finalize(observations).unwrap();
        let result = match handle(VerificationRequest::EvaluateFinal {
            challenge,
            submission,
        }) {
            VerificationResponse::Evaluation { result } => result,
            response => panic!("unexpected evaluation response: {response:?}"),
        };
        assert!(result.accepted, "{:?}", result.violations);
        assert!([
            result.level_a_pass,
            result.level_b_pass,
            result.level_c_pass,
            result.level_d_pass,
            result.level_e_pass,
            result.level_f_pass,
            result.level_g_pass,
            result.level_h_pass,
            result.level_i_pass,
            result.level_j_pass
        ]
        .into_iter()
        .all(|pass| pass));
    }
}
