use std::{cmp::Reverse, collections::BTreeSet};

const FAILURE_EVIDENCE_REUSE: bool = false;
const MULTI_MECHANISM_PLANNING: bool = false;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mechanism {
    pub id: u64,
    pub signature: u64,
    pub score: u64,
    pub valid: bool,
    pub causal: bool,
    pub gain: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Challenge {
    pub challenge_id: String,
    pub evidence: Vec<u64>,
    pub mechanisms: Vec<Mechanism>,
    pub base_cost: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Trace {
    pub challenge_id: String,
    pub proposed: bool,
    pub selected_ids: Vec<u64>,
    pub considered: usize,
    pub candidates: usize,
    pub invalid: usize,
    pub regressive: usize,
    pub verified: usize,
    pub probes: usize,
    pub role_mappings: usize,
    pub deterministic_cost: usize,
    pub peak_frontier: usize,
    pub active_concepts: usize,
    pub temporary_memory: usize,
    pub descendant_cost: u64,
}

pub fn improve_all(challenges: &[Challenge]) -> Vec<Trace> {
    let mut rejected_signatures = BTreeSet::new();
    challenges
        .iter()
        .map(|challenge| improve(challenge, &mut rejected_signatures))
        .collect()
}

fn improve(challenge: &Challenge, rejected_signatures: &mut BTreeSet<u64>) -> Trace {
    let diagnosis_cost = challenge.evidence.len() + 2;
    let actionable = challenge.evidence.iter().copied().max().unwrap_or(0) >= 500;
    if !actionable {
        return Trace {
            challenge_id: challenge.challenge_id.clone(),
            proposed: false,
            selected_ids: Vec::new(),
            considered: 0,
            candidates: 0,
            invalid: 0,
            regressive: 0,
            verified: 0,
            probes: 0,
            role_mappings: 0,
            deterministic_cost: diagnosis_cost,
            peak_frontier: 0,
            active_concepts: 0,
            temporary_memory: rejected_signatures.len() * 8,
            descendant_cost: challenge.base_cost,
        };
    }
    let visible = challenge
        .mechanisms
        .iter()
        .filter(|mechanism| {
            !FAILURE_EVIDENCE_REUSE || !rejected_signatures.contains(&mechanism.signature)
        })
        .collect::<Vec<_>>();
    let considered = visible.len();
    let probes = visible.len();
    if FAILURE_EVIDENCE_REUSE {
        for mechanism in &visible {
            if !mechanism.valid {
                rejected_signatures.insert(mechanism.signature);
            }
        }
    }
    let mut admissible = visible
        .into_iter()
        .filter(|mechanism| mechanism.valid && mechanism.causal)
        .collect::<Vec<_>>();
    admissible.sort_by_key(|mechanism| {
        (
            Reverse(mechanism.gain),
            Reverse(mechanism.score),
            mechanism.id,
        )
    });
    let interaction_evidence = challenge
        .evidence
        .iter()
        .filter(|value| **value >= 700)
        .count()
        >= 2;
    let selected_count = if MULTI_MECHANISM_PLANNING && interaction_evidence {
        2
    } else {
        1
    };
    let selected = admissible
        .iter()
        .take(selected_count)
        .copied()
        .collect::<Vec<_>>();
    let selected_ids = selected
        .iter()
        .map(|mechanism| mechanism.id)
        .collect::<Vec<_>>();
    let total_gain = selected
        .iter()
        .map(|mechanism| mechanism.gain)
        .sum::<u64>()
        .min(900);
    let role_mappings = selected.len();
    let candidates = usize::from(!selected.is_empty());
    let cache_cost = usize::from(FAILURE_EVIDENCE_REUSE) * 2;
    let deterministic_cost = diagnosis_cost
        + considered * 2
        + probes * 3
        + role_mappings * 4
        + candidates * 5
        + cache_cost;
    Trace {
        challenge_id: challenge.challenge_id.clone(),
        proposed: !selected.is_empty(),
        selected_ids,
        considered,
        candidates,
        invalid: 0,
        regressive: 0,
        verified: usize::from(!selected.is_empty()),
        probes,
        role_mappings,
        deterministic_cost,
        peak_frontier: admissible.len(),
        active_concepts: selected.len(),
        temporary_memory: considered * 16 + rejected_signatures.len() * 8,
        descendant_cost: challenge.base_cost * (1_000 - total_gain) / 1_000,
    }
}

#[cfg(test)]
mod tests {
    use super::{improve_all, Challenge, Mechanism};

    fn challenge(evidence: Vec<u64>) -> Challenge {
        Challenge {
            challenge_id: "T".to_string(),
            evidence,
            mechanisms: vec![Mechanism {
                id: 1,
                signature: 10,
                score: 100,
                valid: true,
                causal: true,
                gain: 200,
            }],
            base_cost: 1_000,
        }
    }

    #[test]
    fn preserves_actionability_boundary() {
        assert!(improve_all(&[challenge(vec![600])])[0].proposed);
        assert!(!improve_all(&[challenge(vec![100])])[0].proposed);
    }

    #[test]
    fn never_reports_regression() {
        assert_eq!(improve_all(&[challenge(vec![600])])[0].regressive, 0);
    }
}
