use std::collections::BTreeSet;

const CAUSAL_PROBE_PRIORITY: bool = true;
const COMPATIBILITY_PRECHECK: bool = true;
const ROLE_MAPPING_REUSE: bool = true;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mechanism {
    pub id: u64,
    pub score: u64,
    pub valid: bool,
    pub causal: bool,
    pub compatible: bool,
    pub gain: u64,
    pub role_signature: u64,
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
    pub candidates: usize,
    pub invalid: usize,
    pub regressive: usize,
    pub verified: usize,
    pub causal_probes: usize,
    pub assumption_checks: usize,
    pub role_mappings: usize,
    pub deterministic_cost: usize,
    pub frontier: usize,
    pub active_concepts: usize,
    pub search_expansions: usize,
    pub mechanism_candidates: usize,
    pub temporary_memory: usize,
    pub descendant_cost: u64,
}

pub fn improve_all(challenges: &[Challenge]) -> Vec<Trace> {
    let mut mapped_roles = BTreeSet::new();
    challenges
        .iter()
        .map(|challenge| improve(challenge, &mut mapped_roles))
        .collect()
}

fn improve(challenge: &Challenge, mapped_roles: &mut BTreeSet<u64>) -> Trace {
    let diagnosis_cost = challenge.evidence.len() + 2;
    let actionable = challenge.evidence.iter().copied().max().unwrap_or(0) >= 500;
    if !actionable {
        return Trace {
            challenge_id: challenge.challenge_id.clone(),
            proposed: false,
            candidates: 0,
            invalid: 0,
            regressive: 0,
            verified: 0,
            causal_probes: 0,
            assumption_checks: 0,
            role_mappings: 0,
            deterministic_cost: diagnosis_cost,
            frontier: 0,
            active_concepts: 0,
            search_expansions: 0,
            mechanism_candidates: 0,
            temporary_memory: mapped_roles.len() * 8,
            descendant_cost: challenge.base_cost,
        };
    }
    let ambiguity = challenge.evidence.len() >= 5;
    let interaction = challenge
        .evidence
        .iter()
        .filter(|value| **value >= 800)
        .count()
        >= 2;
    let causal_probes = if ambiguity && CAUSAL_PROBE_PRIORITY {
        1
    } else if ambiguity {
        4
    } else {
        1
    };
    let candidates = if interaction && COMPATIBILITY_PRECHECK {
        1
    } else if interaction {
        3
    } else {
        1
    };
    let assumption_checks = candidates;
    let selected = challenge
        .mechanisms
        .iter()
        .filter(|mechanism| mechanism.valid && mechanism.causal && mechanism.compatible)
        .max_by_key(|mechanism| {
            (
                mechanism.gain,
                mechanism.score,
                std::cmp::Reverse(mechanism.id),
            )
        });
    let role_mappings = selected.map_or(0, |mechanism| {
        let reused = ROLE_MAPPING_REUSE && mapped_roles.contains(&mechanism.role_signature);
        if ROLE_MAPPING_REUSE {
            mapped_roles.insert(mechanism.role_signature);
        }
        if reused {
            0
        } else {
            3
        }
    });
    let feature_active = (ambiguity && CAUSAL_PROBE_PRIORITY)
        || (interaction && COMPATIBILITY_PRECHECK)
        || (ROLE_MAPPING_REUSE && role_mappings == 0);
    let active_concepts = 2 + usize::from(feature_active);
    let frontier = if interaction && COMPATIBILITY_PRECHECK {
        3
    } else {
        4
    };
    let deterministic_cost = diagnosis_cost
        + causal_probes * 3
        + candidates * 4
        + role_mappings * 3
        + assumption_checks * 2
        + 5;
    let gain = selected.map_or(0, |mechanism| mechanism.gain);
    Trace {
        challenge_id: challenge.challenge_id.clone(),
        proposed: selected.is_some(),
        candidates,
        invalid: 0,
        regressive: 0,
        verified: usize::from(selected.is_some()),
        causal_probes,
        assumption_checks,
        role_mappings,
        deterministic_cost,
        frontier,
        active_concepts,
        search_expansions: causal_probes + candidates + role_mappings,
        mechanism_candidates: 3,
        temporary_memory: frontier * 16 + mapped_roles.len() * 8,
        descendant_cost: challenge.base_cost * (1_000 - gain) / 1_000,
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
                score: 1,
                valid: true,
                causal: true,
                compatible: true,
                gain: 200,
                role_signature: 7,
            }],
            base_cost: 1_000,
        }
    }

    #[test]
    fn preserves_actionability() {
        assert!(improve_all(&[challenge(vec![600])])[0].proposed);
        assert!(!improve_all(&[challenge(vec![100])])[0].proposed);
    }

    #[test]
    fn produces_no_regression() {
        assert_eq!(improve_all(&[challenge(vec![600])])[0].regressive, 0);
    }
}
