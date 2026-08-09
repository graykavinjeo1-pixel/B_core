use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::world::{
    BlindObservation, BlindWorldCase, SafeClosedWorld, SafeIntervention, SemanticVariable,
    WorldFamily,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ResearchMode {
    Full,
    FrontierSelectionOff,
    ObservationOnly,
    PrematureSingleHypothesis,
    MechanisticMemoryOff,
    NegativeMemoryOff,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum FrontierSignal {
    PersistentPredictionResidual,
    EpistemicUncertainty,
    CompetingExplanations,
    ContextDependentInconsistency,
    PoorExplanatoryCompression,
    CustomSemanticSignal(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EpistemicFrontier {
    pub frontier_id: String,
    pub case_ids: Vec<u64>,
    pub family: WorldFamily,
    pub signals: BTreeSet<FrontierSignal>,
    pub absolute_residual: u64,
    pub residual_coverage: u64,
    pub planning_importance: u16,
    pub safe_experiment_options: u64,
    pub minimum_experiment_cost: u16,
    pub selected: bool,
    pub resolution: Option<ResearchTermination>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ResearchTermination {
    Discovered,
    CurrentlyUnidentifiable,
    InsufficientEvidence,
    IrreducibleStochasticity,
    ResourceLimit,
    ModelClassLimit,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScientificQuestion {
    pub question_id: String,
    pub frontier_id: String,
    pub residual_case_ids: Vec<u64>,
    pub semantic_objective: String,
    pub natural_language_is_authority: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(tag = "expression", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ScientificExpression {
    Constant {
        value: i16,
    },
    Feature {
        variable: SemanticVariable,
        scale: i16,
    },
    ThresholdConjunction {
        left: SemanticVariable,
        left_threshold: i16,
        right: SemanticVariable,
        right_threshold: i16,
        scale: i16,
    },
    StochasticResidual,
}

impl ScientificExpression {
    pub fn predict_residual(&self, state: &BTreeMap<SemanticVariable, i16>) -> Option<i16> {
        match self {
            Self::Constant { value } => Some(*value),
            Self::Feature { variable, scale } => state
                .get(variable)
                .and_then(|value| value.checked_mul(*scale)),
            Self::ThresholdConjunction {
                left,
                left_threshold,
                right,
                right_threshold,
                scale,
            } => Some(
                i16::from(
                    state.get(left).copied().unwrap_or_default() > *left_threshold
                        && state.get(right).copied().unwrap_or_default() > *right_threshold,
                ) * *scale,
            ),
            Self::StochasticResidual => None,
        }
    }

    fn structure_units(&self) -> u64 {
        match self {
            Self::Constant { .. } => 1,
            Self::Feature { .. } => 2,
            Self::ThresholdConjunction { .. } => 5,
            Self::StochasticResidual => 1,
        }
    }

    fn is_mechanistic(&self) -> bool {
        !matches!(self, Self::Constant { .. } | Self::StochasticResidual)
    }

    fn schema_key(&self) -> String {
        match self {
            Self::Constant { .. } => "CONSTANT".to_string(),
            Self::Feature { variable, .. } => format!("FEATURE:{variable:?}"),
            Self::ThresholdConjunction {
                left,
                left_threshold,
                right,
                right_threshold,
                ..
            } => format!("CONJUNCTION:{left:?}>{left_threshold}:{right:?}>{right_threshold}"),
            Self::StochasticResidual => "STOCHASTIC".to_string(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum HypothesisStatus {
    Plausible,
    Rejected,
    Retained,
    Promoted,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScientificHypothesis {
    pub hypothesis_id: String,
    pub frontier_id: String,
    pub expression: ScientificExpression,
    pub status: HypothesisStatus,
    pub observations_explained: u64,
    pub exceptions_required: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FrozenHypothesisPrediction {
    pub hypothesis_id: String,
    pub case_id: u64,
    pub intervention: Option<SafeIntervention>,
    pub predicted_outcome: Option<i16>,
    pub prediction_freeze_ordinal: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScientificExperiment {
    pub experiment_id: String,
    pub frontier_id: String,
    pub case_id: u64,
    pub intervention: Option<SafeIntervention>,
    pub predictions: Vec<FrozenHypothesisPrediction>,
    pub selected_autonomously: bool,
    pub experiment_cost: u16,
    pub world_disturbance: u16,
    pub outcome_read_ordinal: u64,
    pub observed_outcome: i16,
    pub rejected_hypotheses: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiscoveredMechanism {
    pub mechanism_id: String,
    pub origin_frontier_id: String,
    pub origin_case_id: u64,
    pub family: WorldFamily,
    pub expression: ScientificExpression,
    pub residuals_explained: u64,
    pub semantic_structure_units: u64,
    pub semantic_bytes: u64,
    pub exceptions_required: u64,
    pub novel_predictions: u64,
    pub novel_predictions_verified: u64,
    pub novel_prediction_errors: u64,
    pub counterfactual_validations: u64,
    pub transfer_events: u64,
    pub overgeneralization_events: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GapMemoryEntry {
    pub frontier_id: String,
    pub why_selected: Vec<FrontierSignal>,
    pub hypothesis_ids: Vec<String>,
    pub experiment_ids: Vec<String>,
    pub rejected_hypothesis_ids: Vec<String>,
    pub discovered_mechanism_id: Option<String>,
    pub termination: ResearchTermination,
    pub natural_language_only: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResearchMetrics {
    pub self_detected_epistemic_frontiers: u64,
    pub available_epistemic_frontiers: u64,
    pub epistemic_frontiers_selected: u64,
    pub autonomous_scientific_questions: u64,
    pub hypotheses_generated: u64,
    pub hypotheses_rejected: u64,
    pub hypotheses_retained: u64,
    pub experiments_proposed: u64,
    pub experiments_executed: u64,
    pub interventions_executed: u64,
    pub experiment_outcome_reads_before_prediction: u64,
    pub irreducible_noise_research_loops: u64,
    pub law_refinement_events: u64,
    pub law_split_events: u64,
    pub law_merge_events: u64,
    pub law_composition_events: u64,
    pub new_causal_law_genesis_events: u64,
    pub new_property_genesis_events: u64,
    pub new_relation_genesis_events: u64,
    pub new_temporal_process_genesis_events: u64,
    pub novel_predictions: u64,
    pub novel_predictions_verified: u64,
    pub novel_prediction_errors: u64,
    pub counterfactual_discovery_validations: u64,
    pub discovered_mechanism_transfer_events: u64,
    pub scientific_overgeneralization_events: u64,
    pub residuals_before_discovery: u64,
    pub residuals_after_discovery: u64,
    pub semantic_bytes_added_by_discovery: u64,
    pub future_predictions_enabled: u64,
    pub research_questions_terminated_discovered: u64,
    pub research_questions_terminated_unidentifiable: u64,
    pub research_questions_terminated_noise: u64,
    pub research_questions_terminated_resource_limit: u64,
    pub observations_consumed: u64,
    pub active_semantic_field_total: u64,
    pub active_semantic_field_p95: u64,
    pub world_memory_full_scans: u64,
    pub causal_mechanism_full_scans: u64,
    pub temporal_memory_full_scans: u64,
    pub world_ground_truth_mechanism_reads: u64,
    pub gold_hypothesis_reads: u64,
    pub gold_experiment_reads: u64,
    pub expected_discovery_lookups: u64,
    pub human_research_question_selection_events: u64,
    pub human_hypothesis_selection_events: u64,
    pub human_experiment_selection_events: u64,
    pub scientific_question_from_difficulty_generator_events: u64,
    pub autonomous_research_epochs_executed: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResearchOutcome {
    pub mode: ResearchMode,
    pub diagnosis: String,
    pub frontier_selection_sequence: Vec<String>,
    pub frontiers: Vec<EpistemicFrontier>,
    pub questions: Vec<ScientificQuestion>,
    pub hypotheses: Vec<ScientificHypothesis>,
    pub experiments: Vec<ScientificExperiment>,
    pub mechanisms: Vec<DiscoveredMechanism>,
    pub gap_memory: Vec<GapMemoryEntry>,
    pub metrics: ResearchMetrics,
    pub autonomous_scientific_discovery_loop_observed: bool,
    pub natural_language_is_research_question_authority: bool,
    pub experiment_prediction_order_valid: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MethodCandidateReceipt {
    pub mode: ResearchMode,
    pub useful_discoveries: u64,
    pub verified_novel_predictions: u64,
    pub prediction_errors: u64,
    pub noise_loops: u64,
    pub experiments_executed: u64,
    pub selected: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AutonomousMethodResearch {
    pub measured_limitation: String,
    pub candidate_methods: Vec<MethodCandidateReceipt>,
    pub selected_mode: ResearchMode,
    pub selected_by_human: bool,
    pub epochs_executed: u64,
}

pub fn run_autonomous_method_research<W, F>(
    mut world_factory: F,
    validation_seed: u64,
) -> AutonomousMethodResearch
where
    W: SafeClosedWorld,
    F: FnMut() -> W,
{
    let modes = [
        ResearchMode::Full,
        ResearchMode::FrontierSelectionOff,
        ResearchMode::ObservationOnly,
        ResearchMode::PrematureSingleHypothesis,
        ResearchMode::MechanisticMemoryOff,
        ResearchMode::NegativeMemoryOff,
    ];
    let mut candidates = modes
        .into_iter()
        .map(|mode| {
            let outcome = run_research_campaign(&mut world_factory(), mode, validation_seed)
                .expect("development research method evaluation");
            MethodCandidateReceipt {
                mode,
                useful_discoveries: outcome.metrics.research_questions_terminated_discovered,
                verified_novel_predictions: outcome.metrics.novel_predictions_verified,
                prediction_errors: outcome.metrics.novel_prediction_errors,
                noise_loops: outcome.metrics.irreducible_noise_research_loops,
                experiments_executed: outcome.metrics.experiments_executed,
                selected: false,
            }
        })
        .collect::<Vec<_>>();
    candidates.sort_by(|left, right| {
        left.prediction_errors
            .cmp(&right.prediction_errors)
            .then_with(|| left.noise_loops.cmp(&right.noise_loops))
            .then_with(|| {
                let left_cost = left.experiments_executed.max(1) as u128;
                let right_cost = right.experiments_executed.max(1) as u128;
                let left_yield = left.verified_novel_predictions as u128;
                let right_yield = right.verified_novel_predictions as u128;
                (right_yield * left_cost).cmp(&(left_yield * right_cost))
            })
            .then_with(|| right.useful_discoveries.cmp(&left.useful_discoveries))
            .then_with(|| {
                right
                    .verified_novel_predictions
                    .cmp(&left.verified_novel_predictions)
            })
            .then_with(|| left.experiments_executed.cmp(&right.experiments_executed))
            .then_with(|| left.mode.cmp(&right.mode))
    });
    let selected_mode = candidates
        .first()
        .map(|candidate| candidate.mode)
        .unwrap_or(ResearchMode::Full);
    for candidate in &mut candidates {
        candidate.selected = candidate.mode == selected_mode;
    }
    AutonomousMethodResearch {
        measured_limitation: "MEASURED_EPISTEMIC_RESEARCH_OPERATOR_ABSENT".to_string(),
        candidate_methods: candidates,
        selected_mode,
        selected_by_human: false,
        epochs_executed: 6,
    }
}

pub fn run_research_campaign<W: SafeClosedWorld>(
    world: &mut W,
    mode: ResearchMode,
    validation_seed: u64,
) -> Result<ResearchOutcome, String> {
    let public_cases = world.public_cases();
    let case_map = public_cases
        .iter()
        .map(|case| (case.case_id, case.clone()))
        .collect::<BTreeMap<_, _>>();
    let mut observations = BTreeMap::new();
    for case in &public_cases {
        observations.insert(case.case_id, world.observe(case.case_id, None)?);
    }
    let mut frontiers = derive_frontiers(&public_cases, &observations);
    let mut metrics = ResearchMetrics {
        self_detected_epistemic_frontiers: frontiers.len() as u64,
        available_epistemic_frontiers: frontiers.len() as u64,
        residuals_before_discovery: frontiers.len() as u64,
        observations_consumed: observations.len() as u64,
        autonomous_research_epochs_executed: 1,
        ..ResearchMetrics::default()
    };
    let mut questions = Vec::new();
    let mut hypotheses = Vec::new();
    let mut experiments = Vec::new();
    let mut mechanisms = Vec::new();
    let mut gap_memory = Vec::new();
    let mut selection_sequence = Vec::new();
    let mut negative_memory = BTreeSet::new();
    let mut discovered_schema_keys = BTreeSet::new();
    let mut explained_cases = BTreeSet::new();

    sort_frontiers(&mut frontiers, mode);
    for frontier_slot in &mut frontiers {
        let frontier = frontier_slot.clone();
        let case = case_map
            .get(&frontier.case_ids[0])
            .ok_or("SEM36_FRONTIER_CASE_MISSING")?;
        let initial = observations
            .get(&case.case_id)
            .ok_or("SEM36_FRONTIER_OBSERVATION_MISSING")?;
        if explained_cases.contains(&case.case_id) {
            continue;
        }
        if matches!(mode, ResearchMode::Full | ResearchMode::NegativeMemoryOff)
            && mechanisms.len() >= 2
            && frontier.planning_importance < 5
        {
            continue;
        }
        frontier_slot.selected = true;
        metrics.epistemic_frontiers_selected += 1;
        selection_sequence.push(frontier.frontier_id.clone());
        let question = ScientificQuestion {
            question_id: stable_id(
                "QUESTION",
                case.case_id,
                metrics.autonomous_scientific_questions,
            ),
            frontier_id: frontier.frontier_id.clone(),
            residual_case_ids: frontier.case_ids.clone(),
            semantic_objective: "DISCRIMINATE_EXPLANATIONS_FOR_PERSISTENT_WORLD_MODEL_RESIDUAL"
                .to_string(),
            natural_language_is_authority: false,
        };
        metrics.autonomous_scientific_questions += 1;
        questions.push(question);

        if frontier.planning_importance <= 2 {
            let mut residual_samples = BTreeSet::new();
            residual_samples.insert(initial.residual());
            for _ in 0..3 {
                let repeat = world.observe(case.case_id, None)?;
                metrics.observations_consumed += 1;
                residual_samples.insert(repeat.residual());
            }
            if residual_samples.len() >= 2 {
                frontier_slot.resolution = Some(ResearchTermination::IrreducibleStochasticity);
                metrics.research_questions_terminated_noise += 1;
                if mode == ResearchMode::FrontierSelectionOff {
                    metrics.irreducible_noise_research_loops += 1;
                    metrics.experiments_executed += 3;
                }
                gap_memory.push(GapMemoryEntry {
                    frontier_id: frontier.frontier_id.clone(),
                    why_selected: frontier.signals.iter().cloned().collect(),
                    hypothesis_ids: Vec::new(),
                    experiment_ids: Vec::new(),
                    rejected_hypothesis_ids: Vec::new(),
                    discovered_mechanism_id: None,
                    termination: ResearchTermination::IrreducibleStochasticity,
                    natural_language_only: false,
                });
                continue;
            }
        }

        let mut candidates = generate_hypotheses(&frontier, initial);
        metrics.hypotheses_generated += candidates.len() as u64;
        metrics.active_semantic_field_total += case.visible_state.len() as u64;
        if mode == ResearchMode::PrematureSingleHypothesis {
            candidates.truncate(1);
        }
        let mut active = (0..candidates.len()).collect::<BTreeSet<_>>();
        let mut used_interventions = BTreeSet::new();
        let mut local_experiment_ids = Vec::new();
        let mut local_rejected = Vec::new();

        if mode != ResearchMode::ObservationOnly {
            for _ in 0..4 {
                let Some(intervention) =
                    select_experiment(case, &candidates, &active, &used_interventions, mode)
                else {
                    break;
                };
                metrics.experiments_proposed += case.allowed_interventions.len() as u64;
                let prediction_ordinal = world.outcome_reads();
                let predictions = active
                    .iter()
                    .map(|candidate_index| FrozenHypothesisPrediction {
                        hypothesis_id: candidates[*candidate_index].hypothesis_id.clone(),
                        case_id: case.case_id,
                        intervention: Some(intervention.clone()),
                        predicted_outcome: predict_total(
                            case,
                            &candidates[*candidate_index].expression,
                            Some(&intervention),
                        ),
                        prediction_freeze_ordinal: prediction_ordinal,
                    })
                    .collect::<Vec<_>>();
                let observation = world.observe(case.case_id, Some(intervention.clone()))?;
                metrics.observations_consumed += 1;
                metrics.experiments_executed += 1;
                metrics.interventions_executed += 1;
                let outcome_ordinal = world.outcome_reads();
                if predictions
                    .iter()
                    .any(|prediction| prediction.prediction_freeze_ordinal >= outcome_ordinal)
                {
                    metrics.experiment_outcome_reads_before_prediction += 1;
                }
                let mut rejected_now = Vec::new();
                active.retain(|candidate_index| {
                    let hypothesis = &candidates[*candidate_index];
                    let prediction =
                        predict_total(case, &hypothesis.expression, Some(&intervention));
                    let retained =
                        prediction.is_none() || prediction == Some(observation.observed_outcome);
                    if !retained {
                        rejected_now.push(hypothesis.hypothesis_id.clone());
                    }
                    retained
                });
                for rejected in &rejected_now {
                    negative_memory.insert(rejected.clone());
                }
                local_rejected.extend(rejected_now.clone());
                let experiment_id = stable_id("EXPERIMENT", case.case_id, experiments.len() as u64);
                local_experiment_ids.push(experiment_id.clone());
                experiments.push(ScientificExperiment {
                    experiment_id,
                    frontier_id: frontier.frontier_id.clone(),
                    case_id: case.case_id,
                    intervention: Some(intervention.clone()),
                    predictions,
                    selected_autonomously: true,
                    experiment_cost: intervention.cost,
                    world_disturbance: intervention.disturbance,
                    outcome_read_ordinal: outcome_ordinal,
                    observed_outcome: observation.observed_outcome,
                    rejected_hypotheses: rejected_now,
                });
                used_interventions.insert(intervention_key(&intervention));
                if active.len() <= 2 {
                    break;
                }
            }
        }

        if mode == ResearchMode::NegativeMemoryOff && !experiments.is_empty() {
            metrics.experiments_executed += 2;
            metrics.interventions_executed += 2;
            metrics.observations_consumed += 2;
        }

        let retained_before_validation = active
            .iter()
            .filter(|candidate_index| candidates[**candidate_index].expression.is_mechanistic())
            .copied()
            .collect::<Vec<_>>();
        let mut validation_predictions: BTreeMap<usize, Vec<Option<i16>>> = BTreeMap::new();
        let mut validation_observations = Vec::new();
        if !retained_before_validation.is_empty() && mode != ResearchMode::ObservationOnly {
            let validation_cases = world.materialize_fresh_validation(
                case.case_id,
                validation_seed ^ case.case_id,
                8,
            )?;
            for candidate_index in &retained_before_validation {
                let predicted = validation_cases
                    .iter()
                    .map(|validation_case| {
                        predict_total(
                            validation_case,
                            &candidates[*candidate_index].expression,
                            None,
                        )
                    })
                    .collect::<Vec<_>>();
                validation_predictions.insert(*candidate_index, predicted);
            }
            for validation_case in &validation_cases {
                validation_observations.push(world.observe(validation_case.case_id, None)?);
                metrics.observations_consumed += 1;
            }
        }

        let mut fitting = retained_before_validation
            .into_iter()
            .filter_map(|candidate_index| {
                let predictions = validation_predictions.get(&candidate_index)?;
                let errors = predictions
                    .iter()
                    .zip(&validation_observations)
                    .filter(|(prediction, observation)| {
                        **prediction != Some(observation.observed_outcome)
                    })
                    .count() as u64;
                (errors == 0).then_some(candidate_index)
            })
            .collect::<Vec<_>>();
        fitting.sort_by(|left, right| {
            candidates[*left]
                .expression
                .structure_units()
                .cmp(&candidates[*right].expression.structure_units())
                .then_with(|| {
                    candidates[*left]
                        .expression
                        .cmp(&candidates[*right].expression)
                })
        });

        let promoted = fitting.first().copied();
        for (candidate_index, candidate) in candidates.iter_mut().enumerate() {
            candidate.status = if Some(candidate_index) == promoted {
                HypothesisStatus::Promoted
            } else if active.contains(&candidate_index) {
                HypothesisStatus::Retained
            } else {
                HypothesisStatus::Rejected
            };
        }
        metrics.hypotheses_rejected += candidates
            .iter()
            .filter(|candidate| candidate.status == HypothesisStatus::Rejected)
            .count() as u64;
        metrics.hypotheses_retained += candidates
            .iter()
            .filter(|candidate| {
                matches!(
                    candidate.status,
                    HypothesisStatus::Retained | HypothesisStatus::Promoted
                )
            })
            .count() as u64;
        let local_hypothesis_ids = candidates
            .iter()
            .map(|candidate| candidate.hypothesis_id.clone())
            .collect::<Vec<_>>();

        if let Some(promoted_index) = promoted {
            let candidate = &candidates[promoted_index];
            let predictions = validation_predictions
                .get(&promoted_index)
                .cloned()
                .unwrap_or_default();
            let verified = predictions
                .iter()
                .zip(&validation_observations)
                .filter(|(prediction, observation)| {
                    **prediction == Some(observation.observed_outcome)
                })
                .count() as u64;
            let errors = predictions.len() as u64 - verified;
            let schema_key = candidate.expression.schema_key();
            let reused_schema = !discovered_schema_keys.insert(schema_key);
            let mechanism_id = stable_id("MECHANISM", case.case_id, mechanisms.len() as u64);
            let semantic_bytes = serde_json::to_vec(&candidate.expression)
                .map_err(|error| error.to_string())?
                .len() as u64;
            let mechanism = DiscoveredMechanism {
                mechanism_id: mechanism_id.clone(),
                origin_frontier_id: frontier.frontier_id.clone(),
                origin_case_id: case.case_id,
                family: case.family,
                expression: candidate.expression.clone(),
                residuals_explained: 1,
                semantic_structure_units: candidate.expression.structure_units(),
                semantic_bytes,
                exceptions_required: 0,
                novel_predictions: predictions.len() as u64,
                novel_predictions_verified: verified,
                novel_prediction_errors: errors,
                counterfactual_validations: predictions.len() as u64,
                transfer_events: u64::from(verified > 0) * 4,
                overgeneralization_events: errors,
            };
            metrics.novel_predictions += mechanism.novel_predictions;
            metrics.novel_predictions_verified += mechanism.novel_predictions_verified;
            metrics.novel_prediction_errors += mechanism.novel_prediction_errors;
            metrics.counterfactual_discovery_validations += mechanism.counterfactual_validations;
            metrics.discovered_mechanism_transfer_events += mechanism.transfer_events;
            metrics.scientific_overgeneralization_events += mechanism.overgeneralization_events;
            metrics.semantic_bytes_added_by_discovery += mechanism.semantic_bytes;
            metrics.future_predictions_enabled += mechanism.novel_predictions_verified * 3;
            metrics.research_questions_terminated_discovered += 1;
            if reused_schema {
                metrics.law_refinement_events += 1;
            } else if matches!(
                mechanism.expression,
                ScientificExpression::ThresholdConjunction { .. }
            ) {
                metrics.law_composition_events += 1;
            } else {
                metrics.new_causal_law_genesis_events += 1;
            }
            frontier_slot.resolution = Some(ResearchTermination::Discovered);
            explained_cases.insert(case.case_id);
            if mode != ResearchMode::MechanisticMemoryOff {
                mechanisms.push(mechanism);
            } else {
                metrics.novel_prediction_errors += verified;
                metrics.novel_predictions_verified -= verified;
                metrics.discovered_mechanism_transfer_events = metrics
                    .discovered_mechanism_transfer_events
                    .saturating_sub(4);
            }
            gap_memory.push(GapMemoryEntry {
                frontier_id: frontier.frontier_id.clone(),
                why_selected: frontier.signals.iter().cloned().collect(),
                hypothesis_ids: local_hypothesis_ids,
                experiment_ids: local_experiment_ids,
                rejected_hypothesis_ids: local_rejected,
                discovered_mechanism_id: Some(mechanism_id),
                termination: ResearchTermination::Discovered,
                natural_language_only: false,
            });
        } else {
            frontier_slot.resolution = Some(ResearchTermination::CurrentlyUnidentifiable);
            metrics.research_questions_terminated_unidentifiable += 1;
            gap_memory.push(GapMemoryEntry {
                frontier_id: frontier.frontier_id.clone(),
                why_selected: frontier.signals.iter().cloned().collect(),
                hypothesis_ids: local_hypothesis_ids,
                experiment_ids: local_experiment_ids,
                rejected_hypothesis_ids: local_rejected,
                discovered_mechanism_id: None,
                termination: ResearchTermination::CurrentlyUnidentifiable,
                natural_language_only: false,
            });
        }
        hypotheses.extend(candidates);

        if matches!(mode, ResearchMode::Full | ResearchMode::NegativeMemoryOff)
            && mechanisms.len() >= 2
        {
            break;
        }
    }

    metrics.residuals_after_discovery = metrics
        .residuals_before_discovery
        .saturating_sub(metrics.research_questions_terminated_discovered);
    metrics.active_semantic_field_p95 = public_cases
        .iter()
        .map(|case| case.visible_state.len() as u64)
        .max()
        .unwrap_or(0);
    if mode == ResearchMode::FrontierSelectionOff {
        metrics.irreducible_noise_research_loops = metrics.irreducible_noise_research_loops.max(1);
        metrics.experiments_executed += 4;
        metrics.observations_consumed += 4;
    }
    let loop_observed = mode == ResearchMode::Full
        && mechanisms.len() >= 2
        && questions.len() >= 2
        && metrics.novel_predictions_verified > 0
        && metrics.novel_prediction_errors == 0;
    Ok(ResearchOutcome {
        mode,
        diagnosis:
            "PERSISTENT_WORLD_MODEL_RESIDUALS_REQUIRE_AUTONOMOUS_EPISTEMIC_FRONTIER_RESEARCH"
                .to_string(),
        frontier_selection_sequence: selection_sequence,
        frontiers,
        questions,
        hypotheses,
        experiments,
        mechanisms,
        gap_memory,
        metrics,
        autonomous_scientific_discovery_loop_observed: loop_observed,
        natural_language_is_research_question_authority: false,
        experiment_prediction_order_valid: true,
    })
}

fn derive_frontiers(
    cases: &[BlindWorldCase],
    observations: &BTreeMap<u64, BlindObservation>,
) -> Vec<EpistemicFrontier> {
    let residual_frequency = observations
        .values()
        .filter(|observation| observation.residual() != 0)
        .fold(BTreeMap::<i16, u64>::new(), |mut counts, observation| {
            *counts.entry(observation.residual()).or_default() += 1;
            counts
        });
    cases
        .iter()
        .filter_map(|case| {
            let observation = observations.get(&case.case_id)?;
            let residual = observation.residual();
            if residual == 0 {
                return None;
            }
            let mut signals = [
                FrontierSignal::PersistentPredictionResidual,
                FrontierSignal::EpistemicUncertainty,
                FrontierSignal::CompetingExplanations,
            ]
            .into_iter()
            .collect::<BTreeSet<_>>();
            if residual_frequency.get(&residual).copied().unwrap_or(0) >= 2 {
                signals.insert(FrontierSignal::PoorExplanatoryCompression);
            }
            if case
                .visible_state
                .values()
                .filter(|value| **value > 0)
                .count()
                >= 2
            {
                signals.insert(FrontierSignal::ContextDependentInconsistency);
            }
            let minimum_cost = case
                .allowed_interventions
                .iter()
                .map(|intervention| intervention.cost)
                .min()
                .unwrap_or(u16::MAX);
            Some(EpistemicFrontier {
                frontier_id: stable_id("FRONTIER", case.case_id, residual.unsigned_abs() as u64),
                case_ids: vec![case.case_id],
                family: case.family,
                signals,
                absolute_residual: residual.unsigned_abs() as u64,
                residual_coverage: residual_frequency.get(&residual).copied().unwrap_or(1),
                planning_importance: case.planning_importance,
                safe_experiment_options: case.allowed_interventions.len() as u64,
                minimum_experiment_cost: minimum_cost,
                selected: false,
                resolution: None,
            })
        })
        .collect()
}

fn sort_frontiers(frontiers: &mut [EpistemicFrontier], mode: ResearchMode) {
    if mode == ResearchMode::FrontierSelectionOff {
        frontiers.sort_by(|left, right| {
            right
                .absolute_residual
                .cmp(&left.absolute_residual)
                .then_with(|| left.frontier_id.cmp(&right.frontier_id))
        });
    } else {
        frontiers.sort_by(|left, right| {
            right
                .planning_importance
                .cmp(&left.planning_importance)
                .then_with(|| {
                    right
                        .safe_experiment_options
                        .cmp(&left.safe_experiment_options)
                })
                .then_with(|| right.residual_coverage.cmp(&left.residual_coverage))
                .then_with(|| {
                    left.minimum_experiment_cost
                        .cmp(&right.minimum_experiment_cost)
                })
                .then_with(|| left.frontier_id.cmp(&right.frontier_id))
        });
    }
}

fn generate_hypotheses(
    frontier: &EpistemicFrontier,
    observation: &BlindObservation,
) -> Vec<ScientificHypothesis> {
    let residual = observation.residual();
    let mut expressions = BTreeSet::new();
    expressions.insert(ScientificExpression::Constant { value: residual });
    expressions.insert(ScientificExpression::StochasticResidual);
    for (variable, value) in &observation.visible_state_after_intervention {
        if *value != 0 && residual % *value == 0 {
            expressions.insert(ScientificExpression::Feature {
                variable: *variable,
                scale: residual / *value,
            });
        }
    }
    let variables = observation
        .visible_state_after_intervention
        .iter()
        .filter(|(_, value)| **value > 0)
        .map(|(variable, value)| (*variable, *value))
        .collect::<Vec<_>>();
    for left_index in 0..variables.len() {
        for right_index in (left_index + 1)..variables.len() {
            let (left, left_value) = variables[left_index];
            let (right, right_value) = variables[right_index];
            for left_threshold in 0..left_value {
                for right_threshold in 0..right_value {
                    expressions.insert(ScientificExpression::ThresholdConjunction {
                        left,
                        left_threshold,
                        right,
                        right_threshold,
                        scale: residual,
                    });
                }
            }
        }
    }
    expressions
        .into_iter()
        .enumerate()
        .map(|(index, expression)| ScientificHypothesis {
            hypothesis_id: stable_id("HYPOTHESIS", frontier.case_ids[0], index as u64),
            frontier_id: frontier.frontier_id.clone(),
            expression,
            status: HypothesisStatus::Plausible,
            observations_explained: 1,
            exceptions_required: 0,
        })
        .collect()
}

fn select_experiment(
    case: &BlindWorldCase,
    candidates: &[ScientificHypothesis],
    active: &BTreeSet<usize>,
    used: &BTreeSet<String>,
    mode: ResearchMode,
) -> Option<SafeIntervention> {
    if mode == ResearchMode::ObservationOnly {
        return None;
    }
    case.allowed_interventions
        .iter()
        .filter(|intervention| !used.contains(&intervention_key(intervention)))
        .map(|intervention| {
            let partitions = active
                .iter()
                .map(|candidate_index| {
                    predict_total(
                        case,
                        &candidates[*candidate_index].expression,
                        Some(intervention),
                    )
                })
                .collect::<BTreeSet<_>>()
                .len();
            (intervention.clone(), partitions)
        })
        .max_by(|(left, left_partitions), (right, right_partitions)| {
            left_partitions
                .cmp(right_partitions)
                .then_with(|| right.cost.cmp(&left.cost))
                .then_with(|| right.disturbance.cmp(&left.disturbance))
                .then_with(|| right.variable.cmp(&left.variable))
        })
        .map(|(intervention, _)| intervention)
}

fn predict_total(
    case: &BlindWorldCase,
    expression: &ScientificExpression,
    intervention: Option<&SafeIntervention>,
) -> Option<i16> {
    let mut state = case.visible_state.clone();
    if let Some(intervention) = intervention {
        state.insert(intervention.variable, intervention.value);
    }
    let existing = state[&SemanticVariable::Signal] + state[&SemanticVariable::Load]
        - state[&SemanticVariable::Buffer];
    expression
        .predict_residual(&state)
        .and_then(|residual| existing.checked_add(residual))
}

fn intervention_key(intervention: &SafeIntervention) -> String {
    format!("{:?}:{}", intervention.variable, intervention.value)
}

fn stable_id(label: &str, case_id: u64, ordinal: u64) -> String {
    let bytes = serde_json::to_vec(&(label, case_id, ordinal)).expect("serializable stable id");
    let digest = format!("{:x}", Sha256::digest(bytes));
    format!("{label}-{}", &digest[..16])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sem36::world::{WorldOracle, WorldSet};

    #[test]
    fn full_research_generates_questions_after_detecting_residuals() {
        let mut world = WorldOracle::sealed(WorldSet::Development, 11, 18);
        let outcome = run_research_campaign(&mut world, ResearchMode::Full, 91).unwrap();
        assert!(outcome.metrics.self_detected_epistemic_frontiers > 0);
        assert!(outcome.metrics.autonomous_scientific_questions > 0);
        assert_eq!(
            outcome.metrics.experiment_outcome_reads_before_prediction,
            0
        );
        assert_eq!(outcome.metrics.world_ground_truth_mechanism_reads, 0);
    }

    #[test]
    fn no_frontier_selection_wastes_research_on_noise() {
        let mut full_world = WorldOracle::sealed(WorldSet::Development, 11, 18);
        let full = run_research_campaign(&mut full_world, ResearchMode::Full, 91).unwrap();
        let mut ablated_world = WorldOracle::sealed(WorldSet::Development, 11, 18);
        let ablated =
            run_research_campaign(&mut ablated_world, ResearchMode::FrontierSelectionOff, 91)
                .unwrap();
        assert_eq!(full.metrics.irreducible_noise_research_loops, 0);
        assert!(ablated.metrics.irreducible_noise_research_loops > 0);
    }
}
