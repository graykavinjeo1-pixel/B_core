use serde::{Deserialize, Serialize};

use super::world::{BlindObservation, SafeClosedWorld};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Sem35R1EpistemicBaseline {
    pub observations_consumed: u64,
    pub passive_predictions: u64,
    pub correct_passive_predictions: u64,
    pub unexplained_residuals: u64,
    pub self_detected_epistemic_frontiers: u64,
    pub autonomous_scientific_questions: u64,
    pub hypotheses_generated: u64,
    pub experiments_proposed: u64,
    pub experiments_executed: u64,
    pub interventions_executed: u64,
    pub baseline_gap_measured: bool,
    pub limitation: String,
    pub raw_observations: Vec<BlindObservation>,
}

/// The sealed predecessor can predict with its existing laws, but it has no
/// SEM-36 frontier/question/experiment operator. This function adds none.
pub fn run_sealed_sem35_r1_baseline<W: SafeClosedWorld>(
    world: &mut W,
) -> Result<Sem35R1EpistemicBaseline, String> {
    let cases = world.public_cases();
    let mut raw_observations = Vec::with_capacity(cases.len());
    let mut correct = 0_u64;
    let mut residuals = 0_u64;
    for case in cases {
        let observation = world.observe(case.case_id, None)?;
        if observation.observed_outcome == observation.existing_law_prediction {
            correct += 1;
        } else {
            residuals += 1;
        }
        raw_observations.push(observation);
    }
    Ok(Sem35R1EpistemicBaseline {
        observations_consumed: raw_observations.len() as u64,
        passive_predictions: raw_observations.len() as u64,
        correct_passive_predictions: correct,
        unexplained_residuals: residuals,
        self_detected_epistemic_frontiers: 0,
        autonomous_scientific_questions: 0,
        hypotheses_generated: 0,
        experiments_proposed: 0,
        experiments_executed: 0,
        interventions_executed: 0,
        baseline_gap_measured: residuals > 0,
        limitation: "MEASURED_EPISTEMIC_RESEARCH_OPERATOR_ABSENT".to_string(),
        raw_observations,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sem36::world::{WorldOracle, WorldSet};

    #[test]
    fn sealed_predecessor_does_not_invent_a_sem36_research_operator() {
        let mut world = WorldOracle::sealed(WorldSet::Development, 11, 18);
        let baseline = run_sealed_sem35_r1_baseline(&mut world).unwrap();
        assert!(baseline.unexplained_residuals > 0);
        assert_eq!(baseline.self_detected_epistemic_frontiers, 0);
        assert_eq!(baseline.autonomous_scientific_questions, 0);
        assert_eq!(baseline.experiments_executed, 0);
    }
}
