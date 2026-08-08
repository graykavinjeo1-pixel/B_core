use std::cmp::Reverse;

const EVIDENCE_REUSE: bool = false;
const CAUSAL_GUARD: bool = false;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mechanism {
    pub id: u64,
    pub raw_score: u64,
    pub assumption_valid: bool,
    pub causal_relevant: bool,
    pub expected_gain_milli: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Challenge {
    pub challenge_id: String,
    pub evidence: Vec<u64>,
    pub mechanisms: Vec<Mechanism>,
    pub base_descendant_primary_cost: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Trace {
    pub challenge_id: String,
    pub patch_proposed: bool,
    pub selected_mechanism_id: u64,
    pub mechanisms_considered: usize,
    pub role_mappings_attempted: usize,
    pub assumption_probes: usize,
    pub candidates_generated: usize,
    pub invalid_candidates: usize,
    pub regressive_candidates: usize,
    pub verified_improvements: usize,
    pub diagnosis_cost: usize,
    pub proposal_cost: usize,
    pub total_meta_deterministic_cost: usize,
    pub derived_descendant_primary_cost: u64,
}

pub fn improve(challenge: &Challenge) -> Trace {
    let diagnosis_cost = if EVIDENCE_REUSE {
        challenge.evidence.len() + 2
    } else {
        challenge.evidence.len() * 2 + 4
    };
    let actionable = challenge.evidence.iter().copied().max().unwrap_or(0) >= 500;
    if !actionable {
        return Trace {
            challenge_id: challenge.challenge_id.clone(),
            patch_proposed: false,
            selected_mechanism_id: 0,
            mechanisms_considered: 0,
            role_mappings_attempted: 0,
            assumption_probes: 0,
            candidates_generated: 0,
            invalid_candidates: 0,
            regressive_candidates: 0,
            verified_improvements: 0,
            diagnosis_cost,
            proposal_cost: 0,
            total_meta_deterministic_cost: diagnosis_cost,
            derived_descendant_primary_cost: challenge.base_descendant_primary_cost,
        };
    }

    let mut mechanisms_considered = 0;
    let mut role_mappings_attempted = 0;
    let mut assumption_probes = 0;
    let mut candidates_generated = 0;
    let mut invalid_candidates = 0;
    let selected = if CAUSAL_GUARD {
        mechanisms_considered = challenge.mechanisms.len();
        assumption_probes = 1;
        role_mappings_attempted = 1;
        candidates_generated = 1;
        challenge
            .mechanisms
            .iter()
            .filter(|mechanism| mechanism.assumption_valid && mechanism.causal_relevant)
            .max_by_key(|mechanism| mechanism.expected_gain_milli)
    } else {
        let mut ranked = challenge.mechanisms.iter().collect::<Vec<_>>();
        ranked.sort_by_key(|mechanism| Reverse(mechanism.raw_score));
        let mut selected = None;
        for mechanism in ranked {
            mechanisms_considered += 1;
            role_mappings_attempted += 1;
            assumption_probes += 2;
            candidates_generated += 1;
            if !mechanism.assumption_valid || !mechanism.causal_relevant {
                invalid_candidates += 1;
                continue;
            }
            selected = Some(mechanism);
            break;
        }
        selected
    };
    let proposal_cost = mechanisms_considered * 3
        + role_mappings_attempted * 4
        + assumption_probes * 2
        + candidates_generated * 5;
    let (patch_proposed, selected_mechanism_id, verified_improvements, derived_cost) = selected
        .map_or(
            (false, 0, 0, challenge.base_descendant_primary_cost),
            |mechanism| {
                (
                    true,
                    mechanism.id,
                    1,
                    challenge.base_descendant_primary_cost
                        * (1_000 - mechanism.expected_gain_milli)
                        / 1_000,
                )
            },
        );
    Trace {
        challenge_id: challenge.challenge_id.clone(),
        patch_proposed,
        selected_mechanism_id,
        mechanisms_considered,
        role_mappings_attempted,
        assumption_probes,
        candidates_generated,
        invalid_candidates,
        regressive_candidates: 0,
        verified_improvements,
        diagnosis_cost,
        proposal_cost,
        total_meta_deterministic_cost: diagnosis_cost + proposal_cost,
        derived_descendant_primary_cost: derived_cost,
    }
}

#[cfg(test)]
mod tests {
    use super::{improve, Challenge, Mechanism};

    fn challenge(evidence: Vec<u64>) -> Challenge {
        Challenge {
            challenge_id: "T".to_string(),
            evidence,
            mechanisms: vec![Mechanism {
                id: 1,
                raw_score: 1,
                assumption_valid: true,
                causal_relevant: true,
                expected_gain_milli: 200,
            }],
            base_descendant_primary_cost: 1_000,
        }
    }

    #[test]
    fn proposes_only_for_actionable_evidence() {
        assert!(improve(&challenge(vec![600])).patch_proposed);
        assert!(!improve(&challenge(vec![40])).patch_proposed);
    }

    #[test]
    fn preserves_external_quality_contract() {
        let trace = improve(&challenge(vec![600]));
        assert_eq!(trace.verified_improvements, 1);
        assert_eq!(trace.regressive_candidates, 0);
    }
}
