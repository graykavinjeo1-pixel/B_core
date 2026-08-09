use serde::{Deserialize, Serialize};

pub const CONTRACT_VERSION: &str = "SEM30-LONG-HORIZON-SEMANTIC-CONTRACT-V1";
const WIDTH: usize = 6;

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
    TemporalCouple {
        current: usize,
        lagged: usize,
        target: usize,
        phase: u32,
        salt: u64,
    },
    CrossBind {
        local: usize,
        peer: usize,
        target: usize,
        rotate: u32,
        salt: u64,
    },
    ConstraintPropagate {
        source: usize,
        constraint: usize,
        target: usize,
        modulus: u64,
        salt: u64,
    },
    CausalIntervene {
        cause: usize,
        intervention: usize,
        target: usize,
        guard: u64,
        salt: u64,
    },
    HierarchicalCompose {
        parent: usize,
        left: usize,
        right: usize,
        target: usize,
        salt: u64,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Challenge {
    pub contract_version: String,
    pub substrate_id: String,
    pub substrate_family: String,
    pub difficulty_dimension: String,
    pub instance_id: u64,
    pub public_seed: u64,
    pub context_values: Vec<u64>,
    pub lagged_context_values: Vec<u64>,
    pub peer_context_values: Vec<u64>,
    pub constraint_values: Vec<u64>,
    pub intervention_values: Vec<u64>,
    pub hierarchy_values: Vec<u64>,
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
    if let Some(value) = evaluation {
        if request.solution.result_digest != value.digest {
            violations.push("RESULT_DIGEST_MISMATCH".to_string());
        }
        if request.solution.trace_commitment != value.trace {
            violations.push("TRACE_COMMITMENT_MISMATCH".to_string());
        }
        VerificationResult {
            accepted: violations.is_empty(),
            violations,
            contract_version: CONTRACT_VERSION.to_string(),
            semantic_work_units: value.work_units,
            dependency_depth: value.dependency_depth,
            structural_signature: value.structural_signature,
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
    let value = evaluate(challenge)?;
    Ok((
        value.work_units,
        value.dependency_depth,
        value.structural_signature,
    ))
}

fn validate_challenge(challenge: &Challenge) -> Vec<String> {
    let mut violations = Vec::new();
    if challenge.contract_version != CONTRACT_VERSION {
        violations.push("CONTRACT_VERSION_MISMATCH".to_string());
    }
    if challenge.substrate_id.is_empty()
        || challenge.substrate_family.is_empty()
        || challenge.difficulty_dimension.is_empty()
    {
        violations.push("SUBSTRATE_IDENTITY_MISSING".to_string());
    }
    for (name, values) in [
        ("CONTEXT", &challenge.context_values),
        ("LAGGED", &challenge.lagged_context_values),
        ("PEER", &challenge.peer_context_values),
        ("CONSTRAINT", &challenge.constraint_values),
        ("INTERVENTION", &challenge.intervention_values),
        ("HIERARCHY", &challenge.hierarchy_values),
    ] {
        if values.len() != WIDTH {
            violations.push(format!("{name}_CARDINALITY_MISMATCH"));
        }
    }
    if challenge.rules.len() < 5 || challenge.rules.len() > 24 {
        violations.push("RULE_COUNT_OUT_OF_RANGE".to_string());
    }
    let temporal = contains(&challenge.rules, 5);
    let cross = contains(&challenge.rules, 6);
    let constraint = contains(&challenge.rules, 7);
    let causal = contains(&challenge.rules, 8);
    let hierarchy = contains(&challenge.rules, 9);
    let special_count = [temporal, cross, constraint, causal, hierarchy]
        .into_iter()
        .filter(|present| *present)
        .count();
    let dimension_valid = match challenge.difficulty_dimension.as_str() {
        "STRUCTURAL_INTERACTION_RANK" => special_count == 0,
        "TEMPORAL_COUPLING_ORDER" => temporal && special_count == 1,
        "CROSS_INSTANCE_BINDING_ARITY" => cross && special_count == 1,
        "CONSTRAINT_PROPAGATION_WIDTH" => constraint && special_count == 1,
        "CAUSAL_INTERVENTION_DEPTH" => causal && special_count == 1,
        "HIERARCHICAL_COMPOSITION_DEPTH" => hierarchy && special_count == 1,
        _ => false,
    };
    if !dimension_valid {
        violations.push("DIFFICULTY_DIMENSION_STRUCTURE_MISMATCH".to_string());
    }
    violations
}

fn contains(rules: &[Rule], code: u8) -> bool {
    rules.iter().any(|rule| {
        matches!(
            (code, rule),
            (5, Rule::TemporalCouple { .. })
                | (6, Rule::CrossBind { .. })
                | (7, Rule::ConstraintPropagate { .. })
                | (8, Rule::CausalIntervene { .. })
                | (9, Rule::HierarchicalCompose { .. })
        )
    })
}

fn evaluate(challenge: &Challenge) -> Result<Evaluation, String> {
    let mut state = [0_u64; WIDTH];
    if challenge.context_values.len() != WIDTH {
        return Err("CONTEXT_CARDINALITY_MISMATCH".to_string());
    }
    state.copy_from_slice(&challenge.context_values);
    let mut depth = [0_u64; WIDTH];
    let mut trace = mix(challenge.public_seed, challenge.public_nonce);
    let mut work_units = 0_u64;
    let mut signature = mix(challenge.rules.len() as u64, challenge.public_seed);
    for (index, rule) in challenge.rules.iter().enumerate() {
        let (target, value, rule_depth, weight, code) =
            evaluate_rule(challenge, rule, &state, &depth)?;
        state[target] = value;
        depth[target] = rule_depth;
        work_units = work_units.saturating_add(weight * rule_depth.max(1));
        trace = mix(trace, value ^ (index as u64).rotate_left(17));
        signature = mix(signature, code ^ rule_depth.rotate_left(11));
    }
    let digest = state
        .iter()
        .enumerate()
        .fold(challenge.public_nonce, |acc, (index, value)| {
            mix(acc, value.rotate_left((index * 9) as u32))
        });
    Ok(Evaluation {
        digest,
        trace,
        work_units,
        dependency_depth: depth.into_iter().max().unwrap_or(0),
        structural_signature: signature,
    })
}

fn evaluate_rule(
    challenge: &Challenge,
    rule: &Rule,
    state: &[u64; WIDTH],
    depth: &[u64; WIDTH],
) -> Result<(usize, u64, u64, u64, u64), String> {
    let result = match *rule {
        Rule::Affine {
            source,
            target,
            multiplier,
            increment,
        } => {
            check(&[source, target])?;
            (
                target,
                state[source]
                    .wrapping_mul(multiplier | 1)
                    .wrapping_add(increment),
                depth[source] + 1,
                1,
                1,
            )
        }
        Rule::Relate {
            left,
            right,
            target,
            rotate,
            salt,
        } => {
            check(&[left, right, target])?;
            (
                target,
                mix(state[left], state[right] ^ salt).rotate_left(rotate % 64),
                depth[left].max(depth[right]) + 1,
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
            check(&[condition, when_even, when_odd, target])?;
            let chosen = if state[condition] & 1 == 0 {
                state[when_even]
            } else {
                state[when_odd]
            };
            (
                target,
                mix(chosen, challenge.public_nonce ^ salt),
                depth[condition].max(depth[when_even]).max(depth[when_odd]) + 1,
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
            check(&[left, right, target])?;
            (
                target,
                mix(
                    state[left].rotate_left(rotate % 64),
                    state[right].wrapping_add(salt),
                ),
                depth[left].max(depth[right]) + 1,
                3,
                4,
            )
        }
        Rule::TemporalCouple {
            current,
            lagged,
            target,
            phase,
            salt,
        } => {
            check(&[current, lagged, target])?;
            (
                target,
                mix(
                    state[current].rotate_left(phase % 64),
                    challenge.lagged_context_values[lagged].wrapping_add(salt),
                ),
                depth[current] + 2,
                7,
                5,
            )
        }
        Rule::CrossBind {
            local,
            peer,
            target,
            rotate,
            salt,
        } => {
            check(&[local, peer, target])?;
            (
                target,
                mix(state[local] ^ salt, challenge.peer_context_values[peer])
                    .rotate_left(rotate % 64),
                depth[local] + 2,
                8,
                6,
            )
        }
        Rule::ConstraintPropagate {
            source,
            constraint,
            target,
            modulus,
            salt,
        } => {
            check(&[source, constraint, target])?;
            if modulus < 2 {
                return Err("CONSTRAINT_MODULUS_INVALID".to_string());
            }
            (
                target,
                mix(
                    state[source] % modulus,
                    challenge.constraint_values[constraint] ^ salt,
                ),
                depth[source] + 2,
                9,
                7,
            )
        }
        Rule::CausalIntervene {
            cause,
            intervention,
            target,
            guard,
            salt,
        } => {
            check(&[cause, intervention, target])?;
            let intervention_value = challenge.intervention_values[intervention];
            let value = if intervention_value & guard == guard {
                mix(intervention_value ^ salt, state[cause])
            } else {
                mix(state[cause], salt)
            };
            (target, value, depth[cause] + 2, 10, 8)
        }
        Rule::HierarchicalCompose {
            parent,
            left,
            right,
            target,
            salt,
        } => {
            check(&[parent, left, right, target])?;
            (
                target,
                mix(
                    challenge.hierarchy_values[parent],
                    mix(state[left], state[right]) ^ salt,
                ),
                depth[left].max(depth[right]) + 3,
                11,
                9,
            )
        }
    };
    Ok(result)
}

fn check(indices: &[usize]) -> Result<(), String> {
    if indices.iter().all(|index| *index < WIDTH) {
        Ok(())
    } else {
        Err("CONTEXT_INDEX_OUT_OF_RANGE".to_string())
    }
}

pub fn mix(left: u64, right: u64) -> u64 {
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
    fn invalid_payload_discloses_no_internal_result() {
        let result = verify(&VerificationRequest {
            challenge: Challenge {
                contract_version: CONTRACT_VERSION.to_string(),
                substrate_id: "T".to_string(),
                substrate_family: "T".to_string(),
                difficulty_dimension: "UNKNOWN".to_string(),
                instance_id: 1,
                public_seed: 2,
                context_values: vec![1; WIDTH],
                lagged_context_values: vec![2; WIDTH],
                peer_context_values: vec![3; WIDTH],
                constraint_values: vec![4; WIDTH],
                intervention_values: vec![5; WIDTH],
                hierarchy_values: vec![6; WIDTH],
                rules: vec![],
                public_nonce: 7,
            },
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
