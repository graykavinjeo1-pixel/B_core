//! Precommitted, evidence-addressed verification for repository repairs.
//!
//! A repair candidate is not allowed to choose its success criterion after it
//! has run.  This module seals mutually discriminating support and refutation
//! clauses before execution, then evaluates only typed, content-addressed
//! observations.  It grants no source-mutation authority.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::self_repair_contract::sha256;

pub const DECISIVE_REPAIR_VERIFICATION_SCHEMA: &str = "B_DECISIVE_REPAIR_VERIFICATION_1";
pub const EXECUTION_OBSERVATION_KEY: &str = "$EXECUTION_SUCCEEDED";
const MAX_PREDICATES: usize = 64;
const MAX_OBSERVATIONS: usize = 128;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RepairObservationValueIR {
    Bool(bool),
    Integer(i64),
    Text(String),
    Sha256(String),
}

impl RepairObservationValueIR {
    fn valid(&self) -> bool {
        match self {
            Self::Text(value) => !value.is_empty() && value.len() <= 1_024,
            Self::Sha256(value) => is_sha256(value),
            Self::Bool(_) | Self::Integer(_) => true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(tag = "predicate", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RepairVerificationPredicateIR {
    ExecutionSucceeded {
        expected: bool,
    },
    ObservationEquals {
        key: String,
        expected: RepairObservationValueIR,
    },
    ObservationPresent {
        key: String,
    },
}

impl RepairVerificationPredicateIR {
    fn subject_key(&self) -> &str {
        match self {
            Self::ExecutionSucceeded { .. } => EXECUTION_OBSERVATION_KEY,
            Self::ObservationEquals { key, .. } | Self::ObservationPresent { key } => key,
        }
    }

    fn expected_value(&self) -> Option<RepairObservationValueIR> {
        match self {
            Self::ExecutionSucceeded { expected } => {
                Some(RepairObservationValueIR::Bool(*expected))
            }
            Self::ObservationEquals { expected, .. } => Some(expected.clone()),
            Self::ObservationPresent { .. } => None,
        }
    }

    fn valid(&self) -> bool {
        valid_key(self.subject_key())
            && match self {
                Self::ObservationEquals { expected, .. } => expected.valid(),
                Self::ExecutionSucceeded { .. } | Self::ObservationPresent { .. } => true,
            }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "operator",
    content = "predicates",
    rename_all = "SCREAMING_SNAKE_CASE"
)]
pub enum RepairVerificationClauseIR {
    All(Vec<RepairVerificationPredicateIR>),
    Any(Vec<RepairVerificationPredicateIR>),
}

impl RepairVerificationClauseIR {
    fn predicates(&self) -> &[RepairVerificationPredicateIR] {
        match self {
            Self::All(predicates) | Self::Any(predicates) => predicates,
        }
    }

    fn valid(&self) -> bool {
        let predicates = self.predicates();
        !predicates.is_empty()
            && predicates.len() <= MAX_PREDICATES
            && predicates.iter().all(RepairVerificationPredicateIR::valid)
            && predicates.iter().collect::<BTreeSet<_>>().len() == predicates.len()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DecisiveRepairContractDraftIR {
    pub hypothesis_id: String,
    pub predecessor_tree_sha256: String,
    pub candidate_tree_sha256: String,
    pub support_clause: RepairVerificationClauseIR,
    pub refutation_clause: RepairVerificationClauseIR,
    pub discriminating_observation_keys: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DecisiveRepairContractIR {
    pub schema: String,
    pub hypothesis_id: String,
    pub predecessor_tree_sha256: String,
    pub candidate_tree_sha256: String,
    pub support_clause: RepairVerificationClauseIR,
    pub refutation_clause: RepairVerificationClauseIR,
    pub discriminating_observation_keys: Vec<String>,
    pub source_mutation_authorized: bool,
    pub external_llm_calls: u64,
    pub network_reads: u64,
    pub contract_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepairVerificationObservationIR {
    pub key: String,
    pub value: RepairObservationValueIR,
    pub evidence_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepairVerificationObservationSetIR {
    pub schema: String,
    pub execution_succeeded: bool,
    pub observations: Vec<RepairVerificationObservationIR>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum VerificationClauseTruthIR {
    True,
    False,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DecisiveRepairAssessmentIR {
    Supported,
    Refuted,
    Inconclusive,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DecisiveRepairVerificationReceiptIR {
    pub schema: String,
    pub contract_sha256: String,
    pub observation_set_sha256: String,
    pub support_truth: VerificationClauseTruthIR,
    pub refutation_truth: VerificationClauseTruthIR,
    pub assessment: DecisiveRepairAssessmentIR,
    pub source_mutation_authorized: bool,
    pub external_llm_calls: u64,
    pub network_reads: u64,
    pub receipt_sha256: String,
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn valid_key(value: &str) -> bool {
    !value.trim().is_empty()
        && value.len() <= 256
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.' | b'$'))
}

fn clauses_discriminate(
    support: &RepairVerificationClauseIR,
    refutation: &RepairVerificationClauseIR,
) -> bool {
    support.predicates().iter().any(|left| {
        refutation.predicates().iter().any(|right| {
            left.subject_key() == right.subject_key()
                && left
                    .expected_value()
                    .zip(right.expected_value())
                    .is_some_and(|(left_value, right_value)| left_value != right_value)
        })
    })
}

fn contract_projection(contract: &DecisiveRepairContractIR) -> Result<Vec<u8>, String> {
    serde_json::to_vec(&(
        &contract.schema,
        &contract.hypothesis_id,
        &contract.predecessor_tree_sha256,
        &contract.candidate_tree_sha256,
        &contract.support_clause,
        &contract.refutation_clause,
        &contract.discriminating_observation_keys,
        contract.source_mutation_authorized,
        contract.external_llm_calls,
        contract.network_reads,
    ))
    .map_err(|error| format!("DECISIVE_CONTRACT_SERIALIZE:{error}"))
}

pub fn validate_decisive_repair_contract(
    contract: &DecisiveRepairContractIR,
) -> Result<(), String> {
    let key_set = contract
        .discriminating_observation_keys
        .iter()
        .collect::<BTreeSet<_>>();
    let referenced_keys = contract
        .support_clause
        .predicates()
        .iter()
        .chain(contract.refutation_clause.predicates())
        .map(RepairVerificationPredicateIR::subject_key)
        .collect::<BTreeSet<_>>();
    if contract.schema != DECISIVE_REPAIR_VERIFICATION_SCHEMA
        || contract.hypothesis_id.trim().is_empty()
        || contract.hypothesis_id.len() > 256
        || !is_sha256(&contract.predecessor_tree_sha256)
        || !is_sha256(&contract.candidate_tree_sha256)
        || contract.predecessor_tree_sha256 == contract.candidate_tree_sha256
        || !contract.support_clause.valid()
        || !contract.refutation_clause.valid()
        || contract.support_clause == contract.refutation_clause
        || !clauses_discriminate(&contract.support_clause, &contract.refutation_clause)
        || key_set.is_empty()
        || key_set.len() != contract.discriminating_observation_keys.len()
        || key_set.iter().any(|key| !valid_key(key))
        || referenced_keys != key_set.into_iter().map(String::as_str).collect()
        || contract.source_mutation_authorized
        || contract.external_llm_calls != 0
        || contract.network_reads != 0
        || !is_sha256(&contract.contract_sha256)
        || contract.contract_sha256 != sha256(&contract_projection(contract)?)
    {
        return Err("DECISIVE_CONTRACT_INVALID".to_string());
    }
    Ok(())
}

pub fn seal_decisive_repair_contract(
    draft: DecisiveRepairContractDraftIR,
) -> Result<DecisiveRepairContractIR, String> {
    let mut keys = draft.discriminating_observation_keys;
    keys.sort();
    if keys.windows(2).any(|pair| pair[0] == pair[1]) {
        return Err("DECISIVE_CONTRACT_DUPLICATE_KEY".to_string());
    }
    let mut contract = DecisiveRepairContractIR {
        schema: DECISIVE_REPAIR_VERIFICATION_SCHEMA.to_string(),
        hypothesis_id: draft.hypothesis_id,
        predecessor_tree_sha256: draft.predecessor_tree_sha256,
        candidate_tree_sha256: draft.candidate_tree_sha256,
        support_clause: draft.support_clause,
        refutation_clause: draft.refutation_clause,
        discriminating_observation_keys: keys,
        source_mutation_authorized: false,
        external_llm_calls: 0,
        network_reads: 0,
        contract_sha256: String::new(),
    };
    contract.contract_sha256 = sha256(&contract_projection(&contract)?);
    validate_decisive_repair_contract(&contract)?;
    Ok(contract)
}

fn validate_observation_set(
    contract: &DecisiveRepairContractIR,
    set: &RepairVerificationObservationSetIR,
) -> Result<(), String> {
    if set.schema != DECISIVE_REPAIR_VERIFICATION_SCHEMA
        || set.observations.len() > MAX_OBSERVATIONS
    {
        return Err("DECISIVE_OBSERVATION_ENVELOPE".to_string());
    }
    let declared = contract
        .discriminating_observation_keys
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    let mut observed = BTreeSet::new();
    for observation in &set.observations {
        if observation.key == EXECUTION_OBSERVATION_KEY
            || !declared.contains(observation.key.as_str())
            || !observed.insert(observation.key.as_str())
            || !observation.value.valid()
            || !is_sha256(&observation.evidence_sha256)
        {
            return Err("DECISIVE_OBSERVATION_BINDING".to_string());
        }
    }
    Ok(())
}

fn predicate_truth(
    predicate: &RepairVerificationPredicateIR,
    execution_succeeded: bool,
    observations: &BTreeMap<&str, &RepairObservationValueIR>,
) -> VerificationClauseTruthIR {
    match predicate {
        RepairVerificationPredicateIR::ExecutionSucceeded { expected } => {
            if execution_succeeded == *expected {
                VerificationClauseTruthIR::True
            } else {
                VerificationClauseTruthIR::False
            }
        }
        RepairVerificationPredicateIR::ObservationEquals { key, expected } => {
            match observations.get(key.as_str()) {
                Some(actual) if *actual == expected => VerificationClauseTruthIR::True,
                Some(_) => VerificationClauseTruthIR::False,
                None => VerificationClauseTruthIR::Unknown,
            }
        }
        RepairVerificationPredicateIR::ObservationPresent { key } => {
            if observations.contains_key(key.as_str()) {
                VerificationClauseTruthIR::True
            } else {
                VerificationClauseTruthIR::Unknown
            }
        }
    }
}

fn clause_truth(
    clause: &RepairVerificationClauseIR,
    execution_succeeded: bool,
    observations: &BTreeMap<&str, &RepairObservationValueIR>,
) -> VerificationClauseTruthIR {
    let truths = clause
        .predicates()
        .iter()
        .map(|predicate| predicate_truth(predicate, execution_succeeded, observations));
    match clause {
        RepairVerificationClauseIR::All(_) => {
            let mut unknown = false;
            for truth in truths {
                match truth {
                    VerificationClauseTruthIR::False => return VerificationClauseTruthIR::False,
                    VerificationClauseTruthIR::Unknown => unknown = true,
                    VerificationClauseTruthIR::True => {}
                }
            }
            if unknown {
                VerificationClauseTruthIR::Unknown
            } else {
                VerificationClauseTruthIR::True
            }
        }
        RepairVerificationClauseIR::Any(_) => {
            let mut unknown = false;
            for truth in truths {
                match truth {
                    VerificationClauseTruthIR::True => return VerificationClauseTruthIR::True,
                    VerificationClauseTruthIR::Unknown => unknown = true,
                    VerificationClauseTruthIR::False => {}
                }
            }
            if unknown {
                VerificationClauseTruthIR::Unknown
            } else {
                VerificationClauseTruthIR::False
            }
        }
    }
}

fn observation_set_sha256(set: &RepairVerificationObservationSetIR) -> Result<String, String> {
    serde_json::to_vec(set)
        .map(|bytes| sha256(&bytes))
        .map_err(|error| format!("DECISIVE_OBSERVATION_SERIALIZE:{error}"))
}

fn receipt_projection(receipt: &DecisiveRepairVerificationReceiptIR) -> Result<Vec<u8>, String> {
    serde_json::to_vec(&(
        &receipt.schema,
        &receipt.contract_sha256,
        &receipt.observation_set_sha256,
        receipt.support_truth,
        receipt.refutation_truth,
        receipt.assessment,
        receipt.source_mutation_authorized,
        receipt.external_llm_calls,
        receipt.network_reads,
    ))
    .map_err(|error| format!("DECISIVE_RECEIPT_SERIALIZE:{error}"))
}

pub fn assess_decisive_repair_contract(
    contract: &DecisiveRepairContractIR,
    set: &RepairVerificationObservationSetIR,
) -> Result<DecisiveRepairVerificationReceiptIR, String> {
    validate_decisive_repair_contract(contract)?;
    validate_observation_set(contract, set)?;
    let observations = set
        .observations
        .iter()
        .map(|observation| (observation.key.as_str(), &observation.value))
        .collect::<BTreeMap<_, _>>();
    let support_truth = clause_truth(
        &contract.support_clause,
        set.execution_succeeded,
        &observations,
    );
    let refutation_truth = clause_truth(
        &contract.refutation_clause,
        set.execution_succeeded,
        &observations,
    );
    let assessment = match (support_truth, refutation_truth) {
        (VerificationClauseTruthIR::True, VerificationClauseTruthIR::False) => {
            DecisiveRepairAssessmentIR::Supported
        }
        (VerificationClauseTruthIR::False, VerificationClauseTruthIR::True) => {
            DecisiveRepairAssessmentIR::Refuted
        }
        _ => DecisiveRepairAssessmentIR::Inconclusive,
    };
    let mut receipt = DecisiveRepairVerificationReceiptIR {
        schema: DECISIVE_REPAIR_VERIFICATION_SCHEMA.to_string(),
        contract_sha256: contract.contract_sha256.clone(),
        observation_set_sha256: observation_set_sha256(set)?,
        support_truth,
        refutation_truth,
        assessment,
        source_mutation_authorized: false,
        external_llm_calls: 0,
        network_reads: 0,
        receipt_sha256: String::new(),
    };
    receipt.receipt_sha256 = sha256(&receipt_projection(&receipt)?);
    Ok(receipt)
}

pub fn validate_decisive_repair_receipt(
    contract: &DecisiveRepairContractIR,
    set: &RepairVerificationObservationSetIR,
    receipt: &DecisiveRepairVerificationReceiptIR,
) -> Result<(), String> {
    let expected = assess_decisive_repair_contract(contract, set)?;
    if receipt != &expected
        || receipt.schema != DECISIVE_REPAIR_VERIFICATION_SCHEMA
        || receipt.source_mutation_authorized
        || receipt.external_llm_calls != 0
        || receipt.network_reads != 0
        || !is_sha256(&receipt.receipt_sha256)
        || receipt.receipt_sha256 != sha256(&receipt_projection(receipt)?)
    {
        return Err("DECISIVE_RECEIPT_INVALID".to_string());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hash(value: &str) -> String {
        sha256(value.as_bytes())
    }

    fn contract() -> DecisiveRepairContractIR {
        seal_decisive_repair_contract(DecisiveRepairContractDraftIR {
            hypothesis_id: "boundary-repair".to_string(),
            predecessor_tree_sha256: hash("before"),
            candidate_tree_sha256: hash("after"),
            support_clause: RepairVerificationClauseIR::All(vec![
                RepairVerificationPredicateIR::ExecutionSucceeded { expected: true },
                RepairVerificationPredicateIR::ObservationEquals {
                    key: "focused_regression_passed".to_string(),
                    expected: RepairObservationValueIR::Bool(true),
                },
            ]),
            refutation_clause: RepairVerificationClauseIR::Any(vec![
                RepairVerificationPredicateIR::ExecutionSucceeded { expected: false },
                RepairVerificationPredicateIR::ObservationEquals {
                    key: "focused_regression_passed".to_string(),
                    expected: RepairObservationValueIR::Bool(false),
                },
            ]),
            discriminating_observation_keys: vec![
                "focused_regression_passed".to_string(),
                EXECUTION_OBSERVATION_KEY.to_string(),
            ],
        })
        .expect("contract")
    }

    fn observations(
        execution_succeeded: bool,
        value: Option<bool>,
    ) -> RepairVerificationObservationSetIR {
        RepairVerificationObservationSetIR {
            schema: DECISIVE_REPAIR_VERIFICATION_SCHEMA.to_string(),
            execution_succeeded,
            observations: value
                .map(|value| RepairVerificationObservationIR {
                    key: "focused_regression_passed".to_string(),
                    value: RepairObservationValueIR::Bool(value),
                    evidence_sha256: hash(if value { "pass" } else { "fail" }),
                })
                .into_iter()
                .collect(),
        }
    }

    #[test]
    fn precommitted_evidence_supports_or_refutes_candidate() {
        let contract = contract();
        let supported = assess_decisive_repair_contract(&contract, &observations(true, Some(true)))
            .expect("supported");
        let refuted = assess_decisive_repair_contract(&contract, &observations(false, Some(false)))
            .expect("refuted");
        assert_eq!(supported.assessment, DecisiveRepairAssessmentIR::Supported);
        assert_eq!(refuted.assessment, DecisiveRepairAssessmentIR::Refuted);
    }

    #[test]
    fn missing_discriminating_evidence_fails_closed() {
        let receipt = assess_decisive_repair_contract(&contract(), &observations(true, None))
            .expect("receipt");
        assert_eq!(receipt.assessment, DecisiveRepairAssessmentIR::Inconclusive);
        assert_eq!(receipt.support_truth, VerificationClauseTruthIR::Unknown);
    }

    #[test]
    fn contract_requires_opposed_precommitted_expectations() {
        let invalid = DecisiveRepairContractDraftIR {
            hypothesis_id: "not-discriminating".to_string(),
            predecessor_tree_sha256: hash("before"),
            candidate_tree_sha256: hash("after"),
            support_clause: RepairVerificationClauseIR::All(vec![
                RepairVerificationPredicateIR::ObservationPresent {
                    key: "x".to_string(),
                },
            ]),
            refutation_clause: RepairVerificationClauseIR::All(vec![
                RepairVerificationPredicateIR::ObservationPresent {
                    key: "y".to_string(),
                },
            ]),
            discriminating_observation_keys: vec!["x".to_string(), "y".to_string()],
        };
        assert!(seal_decisive_repair_contract(invalid).is_err());
    }

    #[test]
    fn tampering_is_rejected_by_replay() {
        let contract = contract();
        let set = observations(true, Some(true));
        let mut receipt = assess_decisive_repair_contract(&contract, &set).expect("receipt");
        validate_decisive_repair_receipt(&contract, &set, &receipt).expect("valid");
        receipt.assessment = DecisiveRepairAssessmentIR::Refuted;
        assert!(validate_decisive_repair_receipt(&contract, &set, &receipt).is_err());
    }
}
