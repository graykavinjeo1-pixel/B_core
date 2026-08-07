use std::collections::{BTreeMap, BTreeSet};

use super::{
    model::{
        CandidateExperiment, CompetenceClass, ExperimentScore, ExperimentSelectionRecord,
        ModelRevision, SelectorCondition, SemanticSurpriseEvent, UncertaintyItem,
        UncertaintyLedger,
    },
    world::HiddenEnvironment,
};

pub const SELECTOR_VERSION: &str = "SEM3-ACTIVE-SELECTOR-1.0.0";

pub struct SelectionState {
    pub executed_ids: BTreeSet<String>,
    pub executed_semantic_signatures: BTreeSet<String>,
    pub executed_surface_signatures: BTreeSet<String>,
    pub sequence: usize,
}

impl SelectionState {
    pub fn new() -> Self {
        Self {
            executed_ids: BTreeSet::new(),
            executed_semantic_signatures: BTreeSet::new(),
            executed_surface_signatures: BTreeSet::new(),
            sequence: 0,
        }
    }
}

impl Default for SelectionState {
    fn default() -> Self {
        Self::new()
    }
}

pub struct StepOutcome {
    pub record: ExperimentSelectionRecord,
    pub surprise: Option<SemanticSurpriseEvent>,
    pub revision: Option<ModelRevision>,
}

pub fn score_catalog(
    candidates: &mut [CandidateExperiment],
    ledger: &UncertaintyLedger,
    condition: SelectorCondition,
    state: &SelectionState,
) {
    for candidate in candidates {
        candidate.score = score_experiment(candidate, ledger, condition, state);
    }
}

pub fn score_experiment(
    candidate: &CandidateExperiment,
    ledger: &UncertaintyLedger,
    condition: SelectorCondition,
    state: &SelectionState,
) -> ExperimentScore {
    let item = ledger
        .items
        .iter()
        .find(|item| item.uncertainty_id == candidate.query.parent_uncertainty_id);
    let retained = item
        .map(|item| {
            item.competing_hypotheses
                .iter()
                .filter(|hypothesis| hypothesis.retained)
                .count()
        })
        .unwrap_or_default();
    let information = expected_information_gain(candidate, retained);
    let discriminative = f64::from(information > 0.0);
    let boundary = f64::from(matches!(candidate.query.value, -1 | 0));
    let transfer = candidate.query.composition_arity as f64 / 4.0;
    let abstraction = f64::from(candidate.query.sequence_shape >= 3)
        * candidate.query.composition_arity as f64
        / 4.0;
    let frontier = match candidate.competence_class {
        CompetenceClass::Frontier => 1.0,
        CompetenceClass::CurrentlyUnsolved => 0.25,
        CompetenceClass::Mastered => 0.0,
        CompetenceClass::OutOfDomain => -1.0,
    };
    let execution_cost =
        (candidate.query.sequence_shape + candidate.query.composition_arity) as f64 * 0.04;
    let redundancy = f64::from(
        candidate.duplicate_of.is_some()
            || state
                .executed_semantic_signatures
                .contains(&candidate.semantic_signature),
    );
    let triviality = f64::from(information == 0.0 && retained > 1);
    let invalid = f64::from(!candidate.valid_in_closed_world);

    let use_information = !matches!(condition, SelectorCondition::EMinusInformationGain);
    let use_frontier = !matches!(condition, SelectorCondition::EMinusFrontier);
    let use_abstraction = !matches!(condition, SelectorCondition::EMinusAbstractionValue);
    let active_family = matches!(
        condition,
        SelectorCondition::ActiveSemanticE
            | SelectorCondition::EMinusInformationGain
            | SelectorCondition::EMinusFrontier
            | SelectorCondition::EMinusAbstractionValue
            | SelectorCondition::EMinusCounterfactuals
    );
    let (expected_information_gain, expected_uncertainty_reduction, discriminative_value) =
        if use_information {
            (information * 2.0, information, discriminative * 0.8)
        } else {
            (0.0, information * 0.3, 0.0)
        };
    let competence_frontier_value = if use_frontier { frontier * 1.1 } else { 0.0 };
    let expected_reusable_abstraction_value = if use_abstraction {
        abstraction * 0.5
    } else {
        0.0
    };
    let mut total = expected_information_gain
        + expected_uncertainty_reduction
        + discriminative_value
        + boundary * 0.4
        + transfer * 0.35
        + expected_reusable_abstraction_value
        + competence_frontier_value
        - execution_cost
        - redundancy * 1.3
        - triviality * 0.9
        - invalid * 4.0;
    if condition == SelectorCondition::UncertaintyOnlyD {
        total = information * 3.0 + discriminative - execution_cost - redundancy;
    } else if !active_family {
        total = 0.0;
    }
    ExperimentScore {
        expected_information_gain,
        expected_uncertainty_reduction,
        discriminative_hypothesis_value: discriminative_value,
        concept_boundary_clarification: boundary * 0.4,
        expected_transfer_value: transfer * 0.35,
        expected_reusable_abstraction_value,
        competence_frontier_value,
        execution_cost,
        redundancy: redundancy * 1.3,
        triviality: triviality * 0.9,
        invalid_experiment_probability: invalid,
        total,
    }
}

pub fn expected_information_gain(candidate: &CandidateExperiment, retained: usize) -> f64 {
    if retained < 2 {
        return 0.0;
    }
    let positives = candidate
        .predicted_outcomes
        .values()
        .filter(|outcome| **outcome)
        .count();
    let negatives = retained.saturating_sub(positives);
    if positives == 0 || negatives == 0 {
        return 0.0;
    }
    entropy(retained)
        - (positives as f64 / retained as f64) * entropy(positives)
        - (negatives as f64 / retained as f64) * entropy(negatives)
}

fn entropy(count: usize) -> f64 {
    (count as f64).log2()
}

pub fn select_experiment<'a>(
    candidates: &'a [CandidateExperiment],
    condition: SelectorCondition,
    state: &SelectionState,
) -> Option<&'a CandidateExperiment> {
    let available = candidates
        .iter()
        .filter(|candidate| !state.executed_ids.contains(&candidate.query.experiment_id))
        .filter(|candidate| {
            condition != SelectorCondition::EMinusCounterfactuals || !candidate.query.counterfactual
        })
        .collect::<Vec<_>>();
    match condition {
        SelectorCondition::RandomA => {
            if available.is_empty() {
                None
            } else {
                let index = pseudo_random_index(state.sequence, available.len());
                available.get(index).copied()
            }
        }
        SelectorCondition::NoveltyB => available.into_iter().max_by(|left, right| {
            novelty_score(left, state)
                .total_cmp(&novelty_score(right, state))
                .then_with(|| right.query.experiment_id.cmp(&left.query.experiment_id))
        }),
        SelectorCondition::FixedCurriculumC => fixed_selection(&available, state.sequence),
        _ => available.into_iter().max_by(|left, right| {
            left.score
                .total
                .total_cmp(&right.score.total)
                .then_with(|| right.query.experiment_id.cmp(&left.query.experiment_id))
        }),
    }
}

fn pseudo_random_index(sequence: usize, length: usize) -> usize {
    let value = (sequence as u64 + 17)
        .wrapping_mul(6_364_136_223_846_793_005)
        .wrapping_add(1_442_695_040_888_963_407);
    (value as usize) % length
}

fn novelty_score(candidate: &CandidateExperiment, state: &SelectionState) -> f64 {
    let unseen = f64::from(
        !state
            .executed_surface_signatures
            .contains(&candidate.surface_signature),
    );
    unseen * 10.0
        + candidate.query.value.unsigned_abs() as f64
        + candidate.query.sequence_shape as f64 * 0.2
        + candidate.query.composition_arity as f64 * 0.1
}

fn fixed_selection<'a>(
    available: &[&'a CandidateExperiment],
    sequence: usize,
) -> Option<&'a CandidateExperiment> {
    let family = (sequence / 2) % 6;
    let value = if sequence % 2 == 0 { -1 } else { 0 };
    let target = format!("U{family:06}");
    available
        .iter()
        .copied()
        .find(|candidate| {
            candidate.query.parent_uncertainty_id == target
                && candidate.query.value == value
                && !candidate.query.counterfactual
                && candidate.duplicate_of.is_none()
        })
        .or_else(|| available.first().copied())
}

pub fn execute_selected(
    candidate: &CandidateExperiment,
    condition: SelectorCondition,
    ledger: &mut UncertaintyLedger,
    environment: &HiddenEnvironment,
    state: &mut SelectionState,
    revision_sequence: &mut usize,
) -> Result<StepOutcome, String> {
    let observation = environment.execute(&candidate.query)?;
    let item = ledger
        .items
        .iter_mut()
        .find(|item| item.uncertainty_id == candidate.query.parent_uncertainty_id)
        .ok_or_else(|| "PARENT_UNCERTAINTY_MISSING".to_string())?;
    let prior = item
        .competing_hypotheses
        .iter()
        .filter(|hypothesis| hypothesis.retained)
        .count();
    let predicted_true = item
        .competing_hypotheses
        .iter()
        .filter(|hypothesis| {
            hypothesis.retained
                && hypothesis
                    .rule
                    .predict(candidate.query.value, candidate.query.counterfactual)
        })
        .count();
    let majority = predicted_true * 2 >= prior.max(1);
    let mut eliminated = Vec::new();
    for hypothesis in item
        .competing_hypotheses
        .iter_mut()
        .filter(|hypothesis| hypothesis.retained)
    {
        if hypothesis
            .rule
            .predict(candidate.query.value, candidate.query.counterfactual)
            != observation.applicable
        {
            hypothesis.retained = false;
            hypothesis
                .contradicting_evidence
                .push(candidate.query.experiment_id.clone());
            eliminated.push(hypothesis.hypothesis_id.clone());
        } else {
            hypothesis
                .supporting_evidence
                .push(candidate.query.experiment_id.clone());
        }
    }
    let retained = item
        .competing_hypotheses
        .iter()
        .filter(|hypothesis| hypothesis.retained)
        .map(|hypothesis| hypothesis.hypothesis_id.clone())
        .collect::<Vec<_>>();
    if retained.is_empty() {
        return Err("ALL_HYPOTHESES_ELIMINATED".to_string());
    }
    let confidence = 1.0 / retained.len() as f64;
    for hypothesis in item
        .competing_hypotheses
        .iter_mut()
        .filter(|hypothesis| hypothesis.retained)
    {
        hypothesis.confidence = confidence;
    }
    item.confidence = confidence;
    item.resolved = retained.len() == 1;
    item.resolved_hypothesis_id = item.resolved.then(|| retained[0].clone());
    item.supporting_evidence
        .push(candidate.query.experiment_id.clone());
    let realized_information_gain = entropy(prior) - entropy(retained.len());
    let surprise = (majority != observation.applicable).then(|| {
        *revision_sequence += 1;
        SemanticSurpriseEvent {
            surprise_id: format!("SURPRISE-{:06}", *revision_sequence),
            experiment_id: candidate.query.experiment_id.clone(),
            uncertainty_id: item.uncertainty_id.clone(),
            predicted_majority_outcome: majority,
            actual_outcome: observation.applicable,
            diagnosis: diagnosis(item),
            prior_valid_concepts_mutated: false,
            created_revision_id: format!("REVISION-{:06}", *revision_sequence),
        }
    });
    let revision = (!eliminated.is_empty() || surprise.is_some()).then(|| {
        if surprise.is_none() {
            *revision_sequence += 1;
        }
        let revision_id = format!("REVISION-{:06}", *revision_sequence);
        ledger.append_only_revision_ids.push(revision_id.clone());
        ModelRevision {
            revision_id,
            uncertainty_id: item.uncertainty_id.clone(),
            eliminated_hypotheses: eliminated.clone(),
            retained_hypotheses: retained.clone(),
            resolved_hypothesis_id: item.resolved_hypothesis_id.clone(),
            evidence_experiment_id: candidate.query.experiment_id.clone(),
            existing_promoted_concepts_mutated: false,
        }
    });
    state.sequence += 1;
    state
        .executed_ids
        .insert(candidate.query.experiment_id.clone());
    state
        .executed_semantic_signatures
        .insert(candidate.semantic_signature.clone());
    state
        .executed_surface_signatures
        .insert(candidate.surface_signature.clone());
    let structured_explanation = BTreeMap::from([
        (
            "predicted_partition".to_string(),
            format!("{predicted_true} true / {} false", prior - predicted_true),
        ),
        (
            "expected_hypotheses_eliminated".to_string(),
            format!("{:.6}", candidate.score.expected_information_gain),
        ),
        (
            "estimated_execution_cost".to_string(),
            format!("{:.6}", candidate.score.execution_cost),
        ),
        (
            "redundancy".to_string(),
            format!("{:.6}", candidate.score.redundancy),
        ),
        (
            "competence_class".to_string(),
            format!("{:?}", candidate.competence_class),
        ),
    ]);
    Ok(StepOutcome {
        record: ExperimentSelectionRecord {
            sequence: state.sequence,
            condition,
            selected_experiment_id: candidate.query.experiment_id.clone(),
            parent_uncertainty_id: candidate.query.parent_uncertainty_id.clone(),
            candidate_count: prior,
            predicted_outcomes: candidate.predicted_outcomes.clone(),
            score: candidate.score.clone(),
            structured_explanation,
            observation,
            hypotheses_eliminated: eliminated.len(),
            realized_information_gain,
            uncertainty_resolved: item.resolved,
            influenced_model_revision: revision.is_some(),
            influenced_concept_promotion: false,
            provenance: vec![
                SELECTOR_VERSION.to_string(),
                item.uncertainty_id.clone(),
                candidate.query.experiment_id.clone(),
            ],
        },
        surprise,
        revision,
    })
}

fn diagnosis(item: &UncertaintyItem) -> String {
    match item.kind {
        super::model::UncertaintyKind::UncertainPrecondition
        | super::model::UncertaintyKind::UncertainOperatorDomain => "WRONG_PRECONDITION",
        super::model::UncertaintyKind::UncertainInvariant => "WRONG_INVARIANT",
        super::model::UncertaintyKind::UncertainRelation
        | super::model::UncertaintyKind::FailedTransfer => "MISSING_RELATION",
        super::model::UncertaintyKind::AmbiguousConceptBoundary
        | super::model::UncertaintyKind::CompetingAbstractions => "WRONG_ABSTRACTION_BOUNDARY",
        _ => "WRONG_PARAMETER_OR_MISSING_CONCEPT",
    }
    .to_string()
}

pub fn is_duplicate(
    candidate: &CandidateExperiment,
    selected: &[ExperimentSelectionRecord],
) -> bool {
    candidate.duplicate_of.as_ref().is_some_and(|id| {
        selected
            .iter()
            .any(|record| record.selected_experiment_id == *id)
    })
}

pub fn is_near_duplicate(
    candidate: &CandidateExperiment,
    selected: &[ExperimentSelectionRecord],
) -> bool {
    candidate.near_duplicate_of.as_ref().is_some_and(|id| {
        selected
            .iter()
            .any(|record| record.selected_experiment_id == *id)
    })
}

#[cfg(test)]
mod tests {
    use super::{
        execute_selected, expected_information_gain, is_duplicate, score_catalog,
        select_experiment, SelectionState,
    };
    use crate::sem3::{
        model::SelectorCondition,
        world::{generate_candidate_experiments, initial_uncertainty_ledger, HiddenEnvironment},
    };

    #[test]
    fn information_gain_ranks_discriminative_experiment_above_trivial_one() {
        let ledger = initial_uncertainty_ledger();
        let candidates = generate_candidate_experiments(&ledger);
        let diagnostic = candidates
            .iter()
            .find(|candidate| candidate.query.value == -1 && !candidate.query.counterfactual)
            .unwrap();
        let trivial = candidates
            .iter()
            .find(|candidate| candidate.query.value == 3 && !candidate.query.counterfactual)
            .unwrap();
        assert!(expected_information_gain(diagnostic, 3) > expected_information_gain(trivial, 3));
    }

    #[test]
    fn full_value_scoring_is_inspectable_and_selects_positive_value() {
        let ledger = initial_uncertainty_ledger();
        let mut candidates = generate_candidate_experiments(&ledger);
        let state = SelectionState::new();
        score_catalog(
            &mut candidates,
            &ledger,
            SelectorCondition::ActiveSemanticE,
            &state,
        );
        let selected =
            select_experiment(&candidates, SelectorCondition::ActiveSemanticE, &state).unwrap();
        assert!(selected.score.total > 0.0);
        assert!(selected.score.expected_information_gain > 0.0);
        assert_eq!(
            selected.competence_class,
            crate::sem3::model::CompetenceClass::Frontier
        );
    }

    #[test]
    fn duplicate_detection_uses_provenance_not_surface_guessing() {
        let ledger = initial_uncertainty_ledger();
        let candidates = generate_candidate_experiments(&ledger);
        let original = candidates
            .iter()
            .find(|candidate| candidate.duplicate_of.is_none())
            .unwrap();
        let duplicate = candidates
            .iter()
            .find(|candidate| {
                candidate.duplicate_of.as_deref() == Some(&original.query.experiment_id)
            })
            .unwrap();
        let record = crate::sem3::model::ExperimentSelectionRecord {
            sequence: 1,
            condition: SelectorCondition::RandomA,
            selected_experiment_id: original.query.experiment_id.clone(),
            parent_uncertainty_id: original.query.parent_uncertainty_id.clone(),
            candidate_count: 3,
            predicted_outcomes: original.predicted_outcomes.clone(),
            score: original.score.clone(),
            structured_explanation: Default::default(),
            observation: HiddenEnvironment::new().execute(&original.query).unwrap(),
            hypotheses_eliminated: 0,
            realized_information_gain: 0.0,
            uncertainty_resolved: false,
            influenced_model_revision: false,
            influenced_concept_promotion: false,
            provenance: Vec::new(),
        };
        assert!(is_duplicate(duplicate, &[record]));
    }

    #[test]
    fn surprise_creates_append_only_revision_without_mutating_concepts() {
        let mut ledger = initial_uncertainty_ledger();
        let mut candidates = generate_candidate_experiments(&ledger);
        let mut state = SelectionState::new();
        score_catalog(
            &mut candidates,
            &ledger,
            SelectorCondition::ActiveSemanticE,
            &state,
        );
        let candidate = candidates
            .iter()
            .find(|candidate| {
                candidate.query.parent_uncertainty_id == "U000000"
                    && candidate.query.value == -1
                    && !candidate.query.counterfactual
                    && candidate.duplicate_of.is_none()
            })
            .unwrap();
        let mut revision_sequence = 0;
        let outcome = execute_selected(
            candidate,
            SelectorCondition::ActiveSemanticE,
            &mut ledger,
            &HiddenEnvironment::new(),
            &mut state,
            &mut revision_sequence,
        )
        .unwrap();
        assert!(outcome.surprise.is_some());
        assert!(outcome.revision.is_some());
        assert!(!outcome.surprise.unwrap().prior_valid_concepts_mutated);
        assert_eq!(ledger.append_only_revision_ids.len(), 1);
    }
}
