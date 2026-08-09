use serde::{Deserialize, Serialize};

pub const CONTRACT_VERSION: &str = "SEM28-RELATIONAL-PROGRAM-CONTRACT-V1";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "operation", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Rule {
    Affine {
        source: usize,
        target: usize,
        multiplier: u64,
        increment: u64,
    },
    Relate {
        left: usize,
        right: usize,
        target: usize,
        rotate: u32,
        salt: u64,
    },
    Gate {
        condition: usize,
        when_even: usize,
        when_odd: usize,
        target: usize,
        salt: u64,
    },
    Fold {
        left: usize,
        right: usize,
        target: usize,
        rotate: u32,
        salt: u64,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Challenge {
    pub contract_version: String,
    pub substrate_id: String,
    pub instance_id: u64,
    pub public_seed: u64,
    pub interaction_rank: u8,
    pub context_values: Vec<u64>,
    pub rules: Vec<Rule>,
    pub public_nonce: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CandidateSolution {
    pub result_digest: u64,
    pub trace_commitment: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VerificationRequest {
    pub challenge: Challenge,
    pub solution: CandidateSolution,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VerificationResult {
    pub accepted: bool,
    pub violations: Vec<String>,
    pub contract_version: String,
    pub semantic_work_units: u64,
    pub dependency_depth: u64,
    pub structural_signature: u64,
    pub expected_result_disclosed: bool,
    pub verifier_internal_witness_disclosed: bool,
    pub generator_is_success_authority: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Evaluation {
    digest: u64,
    trace: u64,
    work_units: u64,
    dependency_depth: u64,
    structural_signature: u64,
}

pub fn verify(request: &VerificationRequest) -> VerificationResult {
    let mut violations = validate_challenge(&request.challenge);
    let evaluation = if violations.is_empty() {
        match evaluate(&request.challenge) {
            Ok(value) => Some(value),
            Err(error) => {
                violations.push(error);
                None
            }
        }
    } else {
        None
    };
    if let Some(expected) = evaluation {
        if request.solution.result_digest != expected.digest {
            violations.push("RESULT_DIGEST_MISMATCH".to_string());
        }
        if request.solution.trace_commitment != expected.trace {
            violations.push("TRACE_COMMITMENT_MISMATCH".to_string());
        }
        VerificationResult {
            accepted: violations.is_empty(),
            violations,
            contract_version: CONTRACT_VERSION.to_string(),
            semantic_work_units: expected.work_units,
            dependency_depth: expected.dependency_depth,
            structural_signature: expected.structural_signature,
            expected_result_disclosed: false,
            verifier_internal_witness_disclosed: false,
            generator_is_success_authority: false,
        }
    } else {
        VerificationResult {
            accepted: false,
            violations,
            contract_version: CONTRACT_VERSION.to_string(),
            semantic_work_units: 0,
            dependency_depth: 0,
            structural_signature: 0,
            expected_result_disclosed: false,
            verifier_internal_witness_disclosed: false,
            generator_is_success_authority: false,
        }
    }
}

pub fn semantic_metrics(challenge: &Challenge) -> Result<(u64, u64, u64), String> {
    let violations = validate_challenge(challenge);
    if !violations.is_empty() {
        return Err(violations.join("|"));
    }
    let evaluation = evaluate(challenge)?;
    Ok((
        evaluation.work_units,
        evaluation.dependency_depth,
        evaluation.structural_signature,
    ))
}

fn validate_challenge(challenge: &Challenge) -> Vec<String> {
    let mut violations = Vec::new();
    if challenge.contract_version != CONTRACT_VERSION {
        violations.push("CONTRACT_VERSION_MISMATCH".to_string());
    }
    if challenge.substrate_id.is_empty() {
        violations.push("SUBSTRATE_ID_MISSING".to_string());
    }
    if !(1..=4).contains(&challenge.interaction_rank) {
        violations.push("INTERACTION_RANK_OUT_OF_RANGE".to_string());
    }
    if challenge.context_values.len() != 4 {
        violations.push("CONTEXT_CARDINALITY_MISMATCH".to_string());
    }
    if challenge.rules.is_empty() || challenge.rules.len() > 16 {
        violations.push("RULE_COUNT_OUT_OF_RANGE".to_string());
    }
    let has_relate = challenge
        .rules
        .iter()
        .any(|rule| matches!(rule, Rule::Relate { .. }));
    let has_gate = challenge
        .rules
        .iter()
        .any(|rule| matches!(rule, Rule::Gate { .. }));
    let has_fold = challenge
        .rules
        .iter()
        .any(|rule| matches!(rule, Rule::Fold { .. }));
    if challenge.interaction_rank == 1
        && challenge
            .rules
            .iter()
            .any(|rule| !matches!(rule, Rule::Affine { .. }))
    {
        violations.push("RANK_ONE_CONTAINS_RELATIONAL_RULE".to_string());
    }
    if challenge.interaction_rank >= 2 && !has_relate {
        violations.push("RELATIONAL_RULE_REQUIRED".to_string());
    }
    if challenge.interaction_rank >= 3 && !(has_gate && has_fold) {
        violations.push("COMPOSITE_INTERACTION_RULES_REQUIRED".to_string());
    }
    violations
}

fn evaluate(challenge: &Challenge) -> Result<Evaluation, String> {
    let mut state = [0_u64; 4];
    state.copy_from_slice(&challenge.context_values);
    let mut depth = [0_u64; 4];
    let mut trace = mix(challenge.public_seed, challenge.public_nonce);
    let mut work_units = 0_u64;
    let mut signature = mix(
        u64::from(challenge.interaction_rank),
        challenge.rules.len() as u64,
    );
    for (index, rule) in challenge.rules.iter().enumerate() {
        let (target, value, rule_depth, weight, code) = match *rule {
            Rule::Affine {
                source,
                target,
                multiplier,
                increment,
            } => {
                check_index(source)?;
                check_index(target)?;
                (
                    target,
                    state[source]
                        .wrapping_mul(multiplier | 1)
                        .wrapping_add(increment),
                    depth[source].saturating_add(1),
                    1_u64,
                    1_u64,
                )
            }
            Rule::Relate {
                left,
                right,
                target,
                rotate,
                salt,
            } => {
                check_index(left)?;
                check_index(right)?;
                check_index(target)?;
                (
                    target,
                    mix(state[left], state[right] ^ salt).rotate_left(rotate % 64),
                    depth[left].max(depth[right]).saturating_add(1),
                    4,
                    2,
                )
            }
            Rule::Gate {
                condition,
                when_even,
                when_odd,
                target,
                salt,
            } => {
                check_index(condition)?;
                check_index(when_even)?;
                check_index(when_odd)?;
                check_index(target)?;
                let selected = if state[condition] & 1 == 0 {
                    state[when_even]
                } else {
                    state[when_odd]
                };
                (
                    target,
                    mix(selected, challenge.public_nonce ^ salt),
                    depth[condition]
                        .max(depth[when_even])
                        .max(depth[when_odd])
                        .saturating_add(1),
                    5,
                    3,
                )
            }
            Rule::Fold {
                left,
                right,
                target,
                rotate,
                salt,
            } => {
                check_index(left)?;
                check_index(right)?;
                check_index(target)?;
                (
                    target,
                    mix(
                        state[left].rotate_left(rotate % 64),
                        state[right].wrapping_add(salt),
                    ),
                    depth[left].max(depth[right]).saturating_add(1),
                    3,
                    4,
                )
            }
        };
        state[target] = value;
        depth[target] = rule_depth;
        work_units = work_units.saturating_add(weight.saturating_mul(rule_depth.max(1)));
        trace = mix(trace, value ^ (index as u64).rotate_left(17));
        signature = mix(signature, code ^ rule_depth.rotate_left(11));
    }
    let digest = state
        .iter()
        .enumerate()
        .fold(challenge.public_nonce, |accumulator, (index, value)| {
            mix(accumulator, value.rotate_left((index * 13) as u32))
        });
    Ok(Evaluation {
        digest,
        trace,
        work_units,
        dependency_depth: depth.into_iter().max().unwrap_or(0),
        structural_signature: signature,
    })
}

fn check_index(index: usize) -> Result<(), String> {
    if index < 4 {
        Ok(())
    } else {
        Err("CONTEXT_INDEX_OUT_OF_RANGE".to_string())
    }
}

fn mix(left: u64, right: u64) -> u64 {
    let mut value = left ^ right.wrapping_add(0x9E37_79B9_7F4A_7C15);
    value ^= value >> 30;
    value = value.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    value ^= value >> 27;
    value = value.wrapping_mul(0x94D0_49BB_1331_11EB);
    value ^ (value >> 31)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn malformed_contract_is_rejected_without_disclosure() {
        let challenge = Challenge {
            contract_version: CONTRACT_VERSION.to_string(),
            substrate_id: "test".to_string(),
            instance_id: 1,
            public_seed: 3,
            interaction_rank: 3,
            context_values: vec![1, 2, 3, 4],
            rules: vec![Rule::Affine {
                source: 0,
                target: 1,
                multiplier: 3,
                increment: 1,
            }],
            public_nonce: 9,
        };
        let result = verify(&VerificationRequest {
            challenge,
            solution: CandidateSolution {
                result_digest: 0,
                trace_commitment: 0,
            },
        });
        assert!(!result.accepted);
        assert!(!result.expected_result_disclosed);
        assert!(!result.verifier_internal_witness_disclosed);
    }
}
