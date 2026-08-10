//! Constitutional objects and trust-boundary checks for bounded self-repair.
//!
//! The core may construct proposals through Observation -> DefectContract ->
//! RepairSpec -> PatchCandidate.  It cannot mint verifier approval or install
//! a candidate without a receipt issued by an external authority.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub const CAMPAIGN_ID: &str = "B_CORE-RSI-CONTRACT-01";
pub const AUTHORITATIVE_PREDECESSOR: &str =
    "09fe306e96711b6194eefa5b379ce775a1fe4079";
pub const MAX_ACTIVE_CODING_OBJECTS: usize = 3;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObservationIR {
    pub observed_event: String,
    pub trigger: String,
    pub expected_observable: String,
    pub actual_observable: String,
    pub evidence: Vec<String>,
    pub provenance: Vec<String>,
    pub contains_hidden_diagnosis: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DefectContractIR {
    pub affected_behavior: String,
    pub violated_invariant: String,
    pub scope: String,
    pub trigger_conditions: Vec<String>,
    pub expected_vs_observed: String,
    pub causal_evidence: Vec<String>,
    pub uncertainty: String,
    pub suspected_mechanism_classes: Vec<String>,
    pub affected_interfaces: Vec<String>,
    pub preserved_behavior: Vec<String>,
    pub provenance: Vec<String>,
    pub prescribes_concrete_edit: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepairSpecIR {
    pub required_postcondition: String,
    pub restored_invariants: Vec<String>,
    pub allowed_semantic_changes: Vec<String>,
    pub forbidden_semantic_changes: Vec<String>,
    pub compatibility_requirements: Vec<String>,
    pub resource_constraints: Vec<String>,
    pub expected_consequences: Vec<String>,
    pub rollback_conditions: Vec<String>,
    pub verification_requirements: Vec<String>,
    pub applicability: Vec<String>,
    pub uncertainty: String,
    pub encodes_exact_patch: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PatchCandidateIR {
    pub predecessor_tree_hash: String,
    pub changed_files: Vec<String>,
    pub changed_symbols: Vec<String>,
    pub unified_diff_sha256: String,
    pub repair_spec_sha256: String,
    pub consequence_predictions: Vec<String>,
    pub proposer_confidence_millis: u16,
    pub core_self_approved: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum VerificationDecision {
    Accept,
    Reject,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VerificationReceipt {
    pub patch_sha256: String,
    pub repair_spec_sha256: String,
    pub defect_contract_sha256: String,
    pub semantic_checks_sha256: String,
    pub regression_checks_sha256: String,
    pub resource_checks_sha256: String,
    pub invariant_checks_sha256: String,
    pub decision: VerificationDecision,
    pub verifier_identity: String,
    pub verifier_is_proposer: bool,
    pub gold_patch_text_equality_is_authority: bool,
    pub receipt_sha256: String,
    pub authority_seal: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InstallationReceipt {
    pub source_predecessor_hash: String,
    pub patch_sha256: String,
    pub repair_spec_sha256: String,
    pub defect_contract_sha256: String,
    pub verification_receipt_sha256: String,
    pub resulting_source_tree_hash: String,
    pub installer_identity: String,
    pub campaign_id: String,
    pub rollback_reference: String,
    pub installed_into_authoritative_predecessor: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RollbackReceipt {
    pub installation_receipt_sha256: String,
    pub predecessor_tree_hash: String,
    pub candidate_tree_hash: String,
    pub rollback_reference: String,
    pub mechanically_reversible: bool,
    pub historical_receipts_mutated: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Frozen<T> {
    pub value: T,
    pub sha256: String,
    pub frozen_before_downstream_result: bool,
}

impl<T: Serialize> Frozen<T> {
    pub fn new(value: T) -> Result<Self, serde_json::Error> {
        let encoded = serde_json::to_vec(&value)?;
        Ok(Self {
            value,
            sha256: sha256(&encoded),
            frozen_before_downstream_result: true,
        })
    }

    pub fn integrity_valid(&self) -> bool {
        serde_json::to_vec(&self.value)
            .map(|encoded| sha256(&encoded) == self.sha256)
            .unwrap_or(false)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstallationGateError {
    CoreSelfApproval,
    Rejected,
    ProposerVerifierCollision,
    GoldPatchTextAuthority,
    PatchReceiptMismatch,
    RepairSpecReceiptMismatch,
    MissingVerifierIdentity,
    MissingAuthoritySeal,
}

/// Constitutional gate used by an installer.  This does not install anything;
/// it only checks that an external receipt is eligible to authorize an
/// isolated installation.
pub fn validate_installation_authority(
    patch: &PatchCandidateIR,
    receipt: &VerificationReceipt,
) -> Result<(), InstallationGateError> {
    if patch.core_self_approved {
        return Err(InstallationGateError::CoreSelfApproval);
    }
    if receipt.decision != VerificationDecision::Accept {
        return Err(InstallationGateError::Rejected);
    }
    if receipt.verifier_is_proposer {
        return Err(InstallationGateError::ProposerVerifierCollision);
    }
    if receipt.gold_patch_text_equality_is_authority {
        return Err(InstallationGateError::GoldPatchTextAuthority);
    }
    if receipt.patch_sha256 != patch.unified_diff_sha256 {
        return Err(InstallationGateError::PatchReceiptMismatch);
    }
    if receipt.repair_spec_sha256 != patch.repair_spec_sha256 {
        return Err(InstallationGateError::RepairSpecReceiptMismatch);
    }
    if receipt.verifier_identity.is_empty() {
        return Err(InstallationGateError::MissingVerifierIdentity);
    }
    if receipt.authority_seal.is_empty() {
        return Err(InstallationGateError::MissingAuthoritySeal);
    }
    Ok(())
}

pub fn sha256(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

// Canonical self-source behavioral targets.  Evaluator infrastructure copies
// this source into isolated sandboxes and applies hidden semantic mutations.
// The authoritative file is never modified in place by a repair task.

pub fn timeout_is_valid(timeout_ms: i64) -> bool {
    timeout_ms > 0
}

pub fn capacity_allows(used: u32, limit: u32) -> bool {
    used <= limit
}

pub fn verifier_quorum(primary: bool, secondary: bool) -> bool {
    primary && secondary
}

pub fn receipt_binds_patch(receipt_patch_hash: u64, candidate_patch_hash: u64) -> bool {
    receipt_patch_hash == candidate_patch_hash
}

pub fn range_is_ordered(lower: i32, upper: i32) -> bool {
    lower <= upper
}

pub fn evidence_is_sufficient(observation_supported: bool, control_supported: bool) -> bool {
    observation_supported && control_supported
}

pub fn external_control_gate(type_valid: bool, state_valid: bool) -> bool {
    type_valid && state_valid
}

pub fn retry_allowed(attempt: u32, budget: u32) -> bool {
    attempt < budget
}

pub fn generation_installable(verified: bool, post_install_regression: bool) -> bool {
    verified && post_install_regression
}

pub fn resource_within_budget(used: u64, budget: u64) -> bool {
    used <= budget
}

pub fn rollback_checkpoint_valid(checkpoint_present: bool, source_hash_matches: bool) -> bool {
    checkpoint_present && source_hash_matches
}

#[cfg(test)]
mod tests {
    use super::*;

    fn patch() -> PatchCandidateIR {
        PatchCandidateIR {
            predecessor_tree_hash: "predecessor".into(),
            changed_files: vec!["src/self_repair_contract.rs".into()],
            changed_symbols: vec!["timeout_is_valid".into()],
            unified_diff_sha256: "patch".into(),
            repair_spec_sha256: "spec".into(),
            consequence_predictions: vec!["zero is rejected".into()],
            proposer_confidence_millis: 900,
            core_self_approved: false,
        }
    }

    fn receipt() -> VerificationReceipt {
        VerificationReceipt {
            patch_sha256: "patch".into(),
            repair_spec_sha256: "spec".into(),
            defect_contract_sha256: "contract".into(),
            semantic_checks_sha256: "semantic".into(),
            regression_checks_sha256: "regression".into(),
            resource_checks_sha256: "resource".into(),
            invariant_checks_sha256: "invariant".into(),
            decision: VerificationDecision::Accept,
            verifier_identity: "external-verifier".into(),
            verifier_is_proposer: false,
            gold_patch_text_equality_is_authority: false,
            receipt_sha256: "receipt".into(),
            authority_seal: "seal".into(),
        }
    }

    #[test]
    fn frozen_ir_detects_mutation() {
        let frozen = Frozen::new(RepairSpecIR {
            required_postcondition: "timeout must be positive".into(),
            restored_invariants: vec!["zero is invalid".into()],
            allowed_semantic_changes: vec!["timeout boundary".into()],
            forbidden_semantic_changes: vec!["test suppression".into()],
            compatibility_requirements: vec!["positive timeouts remain valid".into()],
            resource_constraints: vec![],
            expected_consequences: vec!["zero is rejected".into()],
            rollback_conditions: vec!["regression".into()],
            verification_requirements: vec!["boundary property".into()],
            applicability: vec!["timeout validation".into()],
            uncertainty: "bounded".into(),
            encodes_exact_patch: false,
        })
        .expect("serializable");
        assert!(frozen.integrity_valid());
    }

    #[test]
    fn core_cannot_self_approve_installation() {
        let mut candidate = patch();
        candidate.core_self_approved = true;
        assert_eq!(
            validate_installation_authority(&candidate, &receipt()),
            Err(InstallationGateError::CoreSelfApproval)
        );
    }

    #[test]
    fn rejected_or_self_verified_receipt_cannot_install() {
        let candidate = patch();
        let mut rejected = receipt();
        rejected.decision = VerificationDecision::Reject;
        assert_eq!(
            validate_installation_authority(&candidate, &rejected),
            Err(InstallationGateError::Rejected)
        );
        let mut self_verified = receipt();
        self_verified.verifier_is_proposer = true;
        assert_eq!(
            validate_installation_authority(&candidate, &self_verified),
            Err(InstallationGateError::ProposerVerifierCollision)
        );
    }

    #[test]
    fn external_receipt_binds_patch_and_spec() {
        assert_eq!(validate_installation_authority(&patch(), &receipt()), Ok(()));
    }

    #[test]
    fn canonical_self_source_invariants_hold() {
        assert!(timeout_is_valid(1));
        assert!(!timeout_is_valid(0));
        assert!(capacity_allows(4, 4));
        assert!(verifier_quorum(true, true));
        assert!(!verifier_quorum(true, false));
        assert!(receipt_binds_patch(7, 7));
        assert!(range_is_ordered(2, 3));
        assert!(evidence_is_sufficient(true, true));
        assert!(external_control_gate(true, true));
        assert!(retry_allowed(2, 3));
        assert!(!retry_allowed(3, 3));
        assert!(generation_installable(true, true));
        assert!(!generation_installable(true, false));
        assert!(resource_within_budget(8, 8));
        assert!(rollback_checkpoint_valid(true, true));
    }
}
