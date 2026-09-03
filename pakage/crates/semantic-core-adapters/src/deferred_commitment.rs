//! Hash-bound lifecycle for conditionally authorized conversational actions.
//!
//! Natural-language dialogue may create a pending commitment, but it cannot
//! satisfy its own condition. Activation requires a typed receipt delivered
//! through the trusted host evidence channel.

use dockable_semantic_core::PlanIntentIR;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub const CONDITION_EVIDENCE_REQUEST_SCHEMA: &str = "B_CORE_CONDITION_EVIDENCE_REQUEST_1";
pub const CONDITION_EVIDENCE_RECEIPT_SCHEMA: &str = "B_CORE_CONDITION_EVIDENCE_RECEIPT_1";
pub const DEFERRED_ACTION_COMMITMENT_SCHEMA: &str = "B_CORE_DEFERRED_ACTION_COMMITMENT_IR_1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DeferredCommitmentStatusIR {
    ConditionPending,
    Activated,
    Contradicted,
    Withdrawn,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ConditionEvidenceDispositionIR {
    VerifiedSatisfied,
    VerifiedContradicted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ConditionEvidenceSourceIR {
    TrustedVerifier,
    ExecutionReceipt,
    AuthorizationReceipt,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeferredActionIR {
    pub intent: PlanIntentIR,
    pub canonical_predicate: String,
    pub predicate_surface: String,
    pub subject: String,
    pub source_semantic_text: String,
    pub external_execution_authorized_after_verification: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeferredActionCommitmentIR {
    pub schema: String,
    pub commitment_id: String,
    pub condition_surface: String,
    pub normalized_condition: String,
    pub condition_sha256: String,
    pub action: DeferredActionIR,
    pub status: DeferredCommitmentStatusIR,
    pub introduced_turn: u64,
    pub last_transition_turn: u64,
    #[serde(default)]
    pub evidence_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub activated_goal_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConditionEvidenceRequestIR {
    pub schema: String,
    pub evidence_id: String,
    pub conversation_id: String,
    pub commitment_id: String,
    pub condition_sha256: String,
    pub disposition: ConditionEvidenceDispositionIR,
    pub source: ConditionEvidenceSourceIR,
    pub verifier_receipt_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConditionEvidenceReceiptIR {
    pub schema: String,
    pub evidence_id: String,
    pub conversation_id: String,
    pub commitment_id: String,
    pub accepted: bool,
    pub prior_status: DeferredCommitmentStatusIR,
    pub resulting_status: DeferredCommitmentStatusIR,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub activated_goal_id: Option<String>,
    pub state_sha256: String,
    pub external_action_executed: bool,
    pub unsupported_claims: usize,
}

impl ConditionEvidenceRequestIR {
    pub fn validate(&self) -> bool {
        self.schema == CONDITION_EVIDENCE_REQUEST_SCHEMA
            && valid_id(&self.evidence_id)
            && valid_id(&self.conversation_id)
            && valid_id(&self.commitment_id)
            && valid_sha256(&self.condition_sha256)
            && valid_sha256(&self.verifier_receipt_sha256)
            && self.verifier_receipt_sha256 == condition_evidence_receipt_sha256(self)
    }
}

impl ConditionEvidenceReceiptIR {
    pub fn validate(&self) -> bool {
        self.schema == CONDITION_EVIDENCE_RECEIPT_SCHEMA
            && valid_id(&self.evidence_id)
            && valid_id(&self.conversation_id)
            && valid_id(&self.commitment_id)
            && self.accepted
            && self.prior_status == DeferredCommitmentStatusIR::ConditionPending
            && matches!(
                self.resulting_status,
                DeferredCommitmentStatusIR::Activated | DeferredCommitmentStatusIR::Contradicted
            )
            && (self.resulting_status == DeferredCommitmentStatusIR::Activated)
                == self.activated_goal_id.is_some()
            && valid_sha256(&self.state_sha256)
            && !self.external_action_executed
            && self.unsupported_claims == 0
    }
}

impl DeferredActionCommitmentIR {
    pub fn validate(&self, completed_turns: u64) -> bool {
        self.schema == DEFERRED_ACTION_COMMITMENT_SCHEMA
            && valid_id(&self.commitment_id)
            && !self.condition_surface.trim().is_empty()
            && !self.normalized_condition.trim().is_empty()
            && self.condition_sha256 == condition_sha256(&self.condition_surface)
            && !self.action.canonical_predicate.trim().is_empty()
            && !self.action.predicate_surface.trim().is_empty()
            && !self.action.subject.trim().is_empty()
            && !self.action.source_semantic_text.trim().is_empty()
            && self.action.external_execution_authorized_after_verification
            && self.introduced_turn > 0
            && self.introduced_turn <= self.last_transition_turn
            && self.last_transition_turn <= completed_turns
            && self.evidence_ids.len() <= 8
            && self.evidence_ids.iter().all(|id| valid_id(id))
            && self
                .evidence_ids
                .iter()
                .collect::<std::collections::BTreeSet<_>>()
                .len()
                == self.evidence_ids.len()
            && match self.status {
                DeferredCommitmentStatusIR::ConditionPending
                | DeferredCommitmentStatusIR::Withdrawn => self.activated_goal_id.is_none(),
                DeferredCommitmentStatusIR::Activated => {
                    self.activated_goal_id.as_deref().is_some_and(valid_id)
                }
                DeferredCommitmentStatusIR::Contradicted => self.activated_goal_id.is_none(),
            }
    }

    pub fn is_pending(&self) -> bool {
        self.status == DeferredCommitmentStatusIR::ConditionPending
    }
}

pub fn normalize_condition(text: &str) -> String {
    text.to_lowercase()
        .split(|character: char| !character.is_alphanumeric() && character != '_')
        .filter(|token| !token.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

pub fn condition_sha256(text: &str) -> String {
    format!("{:x}", Sha256::digest(normalize_condition(text).as_bytes()))
}

pub fn condition_evidence_receipt_sha256(request: &ConditionEvidenceRequestIR) -> String {
    let canonical = [
        request.schema.as_str(),
        request.evidence_id.as_str(),
        request.conversation_id.as_str(),
        request.commitment_id.as_str(),
        request.condition_sha256.as_str(),
        match request.disposition {
            ConditionEvidenceDispositionIR::VerifiedSatisfied => "VERIFIED_SATISFIED",
            ConditionEvidenceDispositionIR::VerifiedContradicted => "VERIFIED_CONTRADICTED",
        },
        match request.source {
            ConditionEvidenceSourceIR::TrustedVerifier => "TRUSTED_VERIFIER",
            ConditionEvidenceSourceIR::ExecutionReceipt => "EXECUTION_RECEIPT",
            ConditionEvidenceSourceIR::AuthorizationReceipt => "AUTHORIZATION_RECEIPT",
        },
    ]
    .join("\0");
    format!("{:x}", Sha256::digest(canonical.as_bytes()))
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn valid_id(value: &str) -> bool {
    !value.trim().is_empty()
        && value.len() <= 160
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn receipt_hash_binds_every_authority_field() {
        let mut request = ConditionEvidenceRequestIR {
            schema: CONDITION_EVIDENCE_REQUEST_SCHEMA.to_string(),
            evidence_id: "E-1".to_string(),
            conversation_id: "CHAT-1".to_string(),
            commitment_id: "DEFERRED-000001-01".to_string(),
            condition_sha256: "a".repeat(64),
            disposition: ConditionEvidenceDispositionIR::VerifiedSatisfied,
            source: ConditionEvidenceSourceIR::TrustedVerifier,
            verifier_receipt_sha256: String::new(),
        };
        request.verifier_receipt_sha256 = condition_evidence_receipt_sha256(&request);
        assert!(request.validate());
        request.disposition = ConditionEvidenceDispositionIR::VerifiedContradicted;
        assert!(!request.validate());
    }

    #[test]
    fn condition_hash_is_language_surface_normalized_not_raw_byte_bound() {
        assert_eq!(
            condition_sha256("Checksum   is VERIFIED."),
            condition_sha256("checksum is verified")
        );
    }
}
