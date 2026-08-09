use serde::{Deserialize, Serialize};

use super::{
    baseline::Sem35R1EpistemicBaseline,
    engine::{ResearchMode, ResearchOutcome},
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Sem36Evaluation {
    pub sem36_status: String,
    pub disposition: String,
    pub level_a_pass: bool,
    pub level_b_pass: bool,
    pub level_c_pass: bool,
    pub level_d_pass: bool,
    pub level_e_pass: bool,
    pub level_f_pass: bool,
    pub level_g_pass: bool,
    pub level_h_pass: bool,
    pub epistemic_frontier_selection_ablation_pass: bool,
    pub scientific_intervention_ablation_pass: bool,
    pub competing_hypothesis_ablation_pass: bool,
    pub discovered_mechanism_memory_ablation_pass: bool,
    pub negative_scientific_memory_ablation_pass: bool,
    pub invariants_pass: bool,
    pub violations: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SecondarySem36Evaluation {
    pub sem36_status: String,
    pub levels: [bool; 8],
    pub ablations: [bool; 5],
    pub invariants_pass: bool,
}

fn arm(arms: &[ResearchOutcome], mode: ResearchMode) -> Result<&ResearchOutcome, String> {
    arms.iter()
        .find(|arm| arm.mode == mode)
        .ok_or_else(|| format!("SEM36_REQUIRED_ARM_MISSING:{mode:?}"))
}

pub fn evaluate_primary(
    baseline: &Sem35R1EpistemicBaseline,
    arms: &[ResearchOutcome],
) -> Result<Sem36Evaluation, String> {
    let full = arm(arms, ResearchMode::Full)?;
    let no_selection = arm(arms, ResearchMode::FrontierSelectionOff)?;
    let observation_only = arm(arms, ResearchMode::ObservationOnly)?;
    let single = arm(arms, ResearchMode::PrematureSingleHypothesis)?;
    let no_mechanistic_memory = arm(arms, ResearchMode::MechanisticMemoryOff)?;
    let no_negative_memory = arm(arms, ResearchMode::NegativeMemoryOff)?;
    let metrics = &full.metrics;

    let selection_ablation = (metrics.novel_predictions_verified as u128
        * no_selection.metrics.experiments_executed as u128)
        > (no_selection.metrics.novel_predictions_verified as u128
            * metrics.experiments_executed as u128)
        && metrics.irreducible_noise_research_loops
            < no_selection.metrics.irreducible_noise_research_loops
        && metrics.experiments_executed < no_selection.metrics.experiments_executed;
    let intervention_ablation = metrics.novel_predictions_verified
        > observation_only.metrics.novel_predictions_verified
        && metrics.research_questions_terminated_discovered
            > observation_only
                .metrics
                .research_questions_terminated_discovered;
    let hypothesis_ablation = metrics.novel_predictions_verified
        > single.metrics.novel_predictions_verified
        && metrics.novel_prediction_errors <= single.metrics.novel_prediction_errors;
    let mechanism_memory_ablation = metrics.novel_predictions_verified
        > no_mechanistic_memory.metrics.novel_predictions_verified
        && metrics.discovered_mechanism_transfer_events
            > no_mechanistic_memory
                .metrics
                .discovered_mechanism_transfer_events;
    let negative_memory_ablation = metrics.research_questions_terminated_discovered
        == no_negative_memory
            .metrics
            .research_questions_terminated_discovered
        && metrics.experiments_executed < no_negative_memory.metrics.experiments_executed;

    let level_a = baseline.unexplained_residuals > 0
        && baseline.self_detected_epistemic_frontiers == 0
        && metrics.self_detected_epistemic_frontiers > 0
        && metrics.human_research_question_selection_events == 0;
    let level_b = metrics.autonomous_scientific_questions > 0
        && metrics.available_epistemic_frontiers > metrics.epistemic_frontiers_selected
        && !full.frontier_selection_sequence.is_empty()
        && !full.natural_language_is_research_question_authority;
    let level_c = metrics.hypotheses_generated >= metrics.autonomous_scientific_questions * 2
        && metrics.hypotheses_rejected > 0
        && metrics.hypotheses_retained > 0
        && metrics.human_hypothesis_selection_events == 0;
    let level_d = metrics.experiments_proposed > 0
        && metrics.experiments_executed > 0
        && metrics.interventions_executed > 0
        && metrics.experiment_outcome_reads_before_prediction == 0
        && full.experiment_prediction_order_valid
        && metrics.human_experiment_selection_events == 0;
    let level_e = !full.mechanisms.is_empty()
        && metrics.residuals_after_discovery < metrics.residuals_before_discovery
        && metrics.law_refinement_events
            + metrics.law_composition_events
            + metrics.new_causal_law_genesis_events
            > 0;
    let level_f = metrics.novel_predictions > 0
        && metrics.novel_predictions_verified == metrics.novel_predictions
        && metrics.novel_prediction_errors == 0;
    let level_g = metrics.discovered_mechanism_transfer_events > 0
        && metrics.scientific_overgeneralization_events == 0;
    let level_h = selection_ablation
        && intervention_ablation
        && hypothesis_ablation
        && mechanism_memory_ablation
        && negative_memory_ablation;
    let invariants = metrics.irreducible_noise_research_loops == 0
        && metrics.experiment_outcome_reads_before_prediction == 0
        && metrics.world_ground_truth_mechanism_reads == 0
        && metrics.gold_hypothesis_reads == 0
        && metrics.gold_experiment_reads == 0
        && metrics.expected_discovery_lookups == 0
        && metrics.scientific_question_from_difficulty_generator_events == 0
        && metrics.world_memory_full_scans == 0
        && metrics.causal_mechanism_full_scans == 0
        && metrics.temporal_memory_full_scans == 0
        && metrics.autonomous_research_epochs_executed <= 4096;
    let levels = [
        level_a, level_b, level_c, level_d, level_e, level_f, level_g, level_h,
    ];
    let mut violations = Vec::new();
    for (index, passed) in levels.iter().enumerate() {
        if !passed {
            violations.push(format!(
                "SEM36_LEVEL_{}_FAILED",
                (b'A' + index as u8) as char
            ));
        }
    }
    if !invariants {
        violations.push("SEM36_INVARIANTS_FAILED".to_string());
    }
    let pass = violations.is_empty();
    Ok(Sem36Evaluation {
        sem36_status: if pass { "PASS" } else { "FAIL" }.to_string(),
        disposition: if pass {
            "VERIFIED_AUTONOMOUS_EPISTEMIC_FRONTIER_SCIENTIFIC_DISCOVERY"
        } else {
            "EPISTEMIC_FRONTIER_DISCOVERY_UNRESOLVED"
        }
        .to_string(),
        level_a_pass: level_a,
        level_b_pass: level_b,
        level_c_pass: level_c,
        level_d_pass: level_d,
        level_e_pass: level_e,
        level_f_pass: level_f,
        level_g_pass: level_g,
        level_h_pass: level_h,
        epistemic_frontier_selection_ablation_pass: selection_ablation,
        scientific_intervention_ablation_pass: intervention_ablation,
        competing_hypothesis_ablation_pass: hypothesis_ablation,
        discovered_mechanism_memory_ablation_pass: mechanism_memory_ablation,
        negative_scientific_memory_ablation_pass: negative_memory_ablation,
        invariants_pass: invariants,
        violations,
    })
}

/// A separate raw-field derivation. It never calls the primary evaluator or
/// consumes primary-derived booleans.
pub fn evaluate_secondary(
    baseline: &Sem35R1EpistemicBaseline,
    arms: &[ResearchOutcome],
) -> Result<SecondarySem36Evaluation, String> {
    let full = arm(arms, ResearchMode::Full)?;
    let no_selection = arm(arms, ResearchMode::FrontierSelectionOff)?;
    let observation_only = arm(arms, ResearchMode::ObservationOnly)?;
    let single = arm(arms, ResearchMode::PrematureSingleHypothesis)?;
    let no_memory = arm(arms, ResearchMode::MechanisticMemoryOff)?;
    let no_negative = arm(arms, ResearchMode::NegativeMemoryOff)?;
    let m = &full.metrics;
    let ablations = [
        (m.novel_predictions_verified as u128 * no_selection.metrics.experiments_executed as u128)
            > (no_selection.metrics.novel_predictions_verified as u128
                * m.experiments_executed as u128)
            && m.irreducible_noise_research_loops
                < no_selection.metrics.irreducible_noise_research_loops
            && m.experiments_executed < no_selection.metrics.experiments_executed,
        m.novel_predictions_verified > observation_only.metrics.novel_predictions_verified,
        m.novel_predictions_verified > single.metrics.novel_predictions_verified,
        m.discovered_mechanism_transfer_events
            > no_memory.metrics.discovered_mechanism_transfer_events,
        m.experiments_executed < no_negative.metrics.experiments_executed
            && m.research_questions_terminated_discovered
                == no_negative.metrics.research_questions_terminated_discovered,
    ];
    let levels = [
        baseline.unexplained_residuals > 0
            && m.self_detected_epistemic_frontiers > 0
            && m.human_research_question_selection_events == 0,
        m.available_epistemic_frontiers > m.epistemic_frontiers_selected
            && m.autonomous_scientific_questions > 0
            && !full.natural_language_is_research_question_authority,
        m.hypotheses_generated > m.autonomous_scientific_questions
            && m.hypotheses_rejected > 0
            && m.hypotheses_retained > 0,
        m.interventions_executed > 0
            && m.experiment_outcome_reads_before_prediction == 0
            && full.experiment_prediction_order_valid,
        !full.mechanisms.is_empty() && m.residuals_after_discovery < m.residuals_before_discovery,
        m.novel_predictions > 0
            && m.novel_predictions == m.novel_predictions_verified
            && m.novel_prediction_errors == 0,
        m.discovered_mechanism_transfer_events > 0 && m.scientific_overgeneralization_events == 0,
        ablations.into_iter().all(|passed| passed),
    ];
    let invariants = m.irreducible_noise_research_loops == 0
        && m.world_ground_truth_mechanism_reads == 0
        && m.gold_hypothesis_reads == 0
        && m.gold_experiment_reads == 0
        && m.expected_discovery_lookups == 0
        && m.scientific_question_from_difficulty_generator_events == 0
        && m.world_memory_full_scans == 0
        && m.causal_mechanism_full_scans == 0
        && m.temporal_memory_full_scans == 0;
    Ok(SecondarySem36Evaluation {
        sem36_status: if levels.into_iter().all(|passed| passed) && invariants {
            "PASS"
        } else {
            "FAIL"
        }
        .to_string(),
        levels,
        ablations,
        invariants_pass: invariants,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sem36::{
        baseline::run_sealed_sem35_r1_baseline,
        engine::run_research_campaign,
        world::{WorldOracle, WorldSet},
    };

    fn fixture() -> (Sem35R1EpistemicBaseline, Vec<ResearchOutcome>) {
        let mut baseline_world = WorldOracle::sealed(WorldSet::Development, 11, 18);
        let baseline = run_sealed_sem35_r1_baseline(&mut baseline_world).unwrap();
        let arms = [
            ResearchMode::Full,
            ResearchMode::FrontierSelectionOff,
            ResearchMode::ObservationOnly,
            ResearchMode::PrematureSingleHypothesis,
            ResearchMode::MechanisticMemoryOff,
            ResearchMode::NegativeMemoryOff,
        ]
        .into_iter()
        .map(|mode| {
            let mut world = WorldOracle::sealed(WorldSet::Development, 11, 18);
            run_research_campaign(&mut world, mode, 91).unwrap()
        })
        .collect();
        (baseline, arms)
    }

    #[test]
    fn primary_and_secondary_derive_the_same_result_independently() {
        let (baseline, arms) = fixture();
        let primary = evaluate_primary(&baseline, &arms).unwrap();
        let secondary = evaluate_secondary(&baseline, &arms).unwrap();
        assert_eq!(primary.sem36_status, secondary.sem36_status);
        assert_eq!(primary.sem36_status, "PASS", "{primary:?}");
    }
}
