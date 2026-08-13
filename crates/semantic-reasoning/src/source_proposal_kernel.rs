//! Language-neutral source proposal authority.
//!
//! Frontends may observe syntax and generators may submit independently
//! materialized edits, but they do not merge, rank, or select programs.  This
//! Rust kernel owns bounded ranking, postimage deduplication, required-group
//! completeness, and atomic edit composition for every language backend.

use std::cmp::Reverse;
use std::collections::{BTreeMap, BTreeSet};

use crate::self_repair_contract::sha256;
use crate::structural_source_repair::{apply_edit_atom, SourceEditAtom};

pub(crate) const MAX_SELECTED_SOURCE_PROPOSALS: usize = 3;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct SourceProposalRankingEvidenceIR {
    pub predicted_value: u16,
    pub priority_adjustment: i32,
    pub related_family_members: usize,
    pub required_family_members: usize,
    pub counterexample_feedback: i32,
    pub public_observation_support: i32,
    pub bounded_strategy_prior: i32,
    pub public_owner_members: usize,
    pub source_local_closure_members: usize,
    pub source_local_closure_depth: usize,
}

impl SourceProposalRankingEvidenceIR {
    fn score(self) -> i32 {
        i32::from(self.predicted_value)
            .saturating_add(self.priority_adjustment)
            .saturating_add(self.counterexample_feedback)
            .saturating_add(self.public_observation_support)
            .saturating_add(self.bounded_strategy_prior)
            .saturating_add(
                i32::try_from(self.related_family_members.min(8))
                    .unwrap_or(8)
                    .saturating_mul(2),
            )
            .saturating_add(
                i32::try_from(self.required_family_members.min(16))
                    .unwrap_or(16)
                    .saturating_mul(25),
            )
            .saturating_add(
                i32::try_from(self.public_owner_members.min(32))
                    .unwrap_or(32)
                    .saturating_mul(10),
            )
            .saturating_add(
                i32::try_from(self.source_local_closure_members.min(32))
                    .unwrap_or(32)
                    .saturating_mul(30),
            )
            .saturating_add(i32::try_from(self.source_local_closure_depth.min(512)).unwrap_or(512))
    }
}

#[derive(Debug)]
pub(crate) struct SourceProposalKernelInput<T> {
    pub proposal_id: String,
    pub candidate_sha256: String,
    pub tie_breaker: String,
    pub evidence: SourceProposalRankingEvidenceIR,
    pub payload: T,
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

pub(crate) fn rank_source_proposals<T>(
    mut proposals: Vec<SourceProposalKernelInput<T>>,
) -> Result<Vec<T>, String> {
    if proposals.iter().any(|proposal| {
        proposal.proposal_id.is_empty()
            || proposal.tie_breaker.is_empty()
            || !is_sha256(&proposal.candidate_sha256)
    }) {
        return Err("SOURCE_PROPOSAL_KERNEL_ENVELOPE_INVALID".to_string());
    }
    proposals.sort_by(|left, right| {
        (
            Reverse(left.evidence.score()),
            &left.proposal_id,
            &left.tie_breaker,
            &left.candidate_sha256,
        )
            .cmp(&(
                Reverse(right.evidence.score()),
                &right.proposal_id,
                &right.tie_breaker,
                &right.candidate_sha256,
            ))
    });
    let mut candidate_sha256s = BTreeSet::new();
    let mut selected = Vec::new();
    for proposal in proposals {
        if candidate_sha256s.insert(proposal.candidate_sha256) {
            selected.push(proposal.payload);
            if selected.len() == MAX_SELECTED_SOURCE_PROPOSALS {
                break;
            }
        }
    }
    Ok(selected)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum SourceProposalCompositionRequirementIR {
    Independent,
    RequiredGroup {
        group_id: String,
        expected_members: usize,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SourceEditProposalIR {
    pub proposal_id: String,
    pub edit: SourceEditAtom,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AtomicSourceCompositionIR {
    pub edit: SourceEditAtom,
    pub candidate_source: String,
    pub candidate_sha256: String,
}

pub(crate) fn compose_source_edit_proposals(
    predecessor: &str,
    proposals: &[SourceEditProposalIR],
    requirement: &SourceProposalCompositionRequirementIR,
) -> Result<AtomicSourceCompositionIR, String> {
    if proposals.is_empty() {
        return Err("SOURCE_PROPOSAL_COMPOSITION_EMPTY".to_string());
    }
    let mut proposal_ids = BTreeSet::new();
    if proposals.iter().any(|proposal| {
        proposal.proposal_id.is_empty() || !proposal_ids.insert(&proposal.proposal_id)
    }) {
        return Err("SOURCE_PROPOSAL_COMPOSITION_IDENTITY_INVALID".to_string());
    }
    if let SourceProposalCompositionRequirementIR::RequiredGroup {
        group_id,
        expected_members,
    } = requirement
    {
        if group_id.is_empty() || *expected_members < 2 || proposals.len() != *expected_members {
            return Err("SOURCE_PROPOSAL_REQUIRED_GROUP_INCOMPLETE".to_string());
        }
    }

    let mut edits = Vec::new();
    let mut insertion_by_offset = BTreeMap::<usize, usize>::new();
    let mut push_edit = |edit: SourceEditAtom| {
        if let SourceEditAtom::Insert { offset, content } = edit {
            if let Some(index) = insertion_by_offset.get(&offset).copied() {
                let SourceEditAtom::Insert {
                    content: existing, ..
                } = &mut edits[index]
                else {
                    unreachable!("insertion index only records inserts")
                };
                existing.push_str(&content);
            } else {
                insertion_by_offset.insert(offset, edits.len());
                edits.push(SourceEditAtom::Insert { offset, content });
            }
        } else {
            edits.push(edit);
        }
    };
    for proposal in proposals {
        match &proposal.edit {
            SourceEditAtom::AtomicMultiEdit { edits: nested } => {
                for edit in nested.iter().cloned() {
                    push_edit(edit);
                }
            }
            edit => push_edit(edit.clone()),
        }
    }
    let edit = SourceEditAtom::AtomicMultiEdit { edits };
    let candidate_source = apply_edit_atom(predecessor, &edit)
        .map_err(|error| format!("SOURCE_PROPOSAL_ATOMIC_COMPOSITION:{error}"))?;
    if candidate_source == predecessor {
        return Err("SOURCE_PROPOSAL_ATOMIC_COMPOSITION_NO_OP".to_string());
    }
    let replay = apply_edit_atom(predecessor, &edit)
        .map_err(|error| format!("SOURCE_PROPOSAL_ATOMIC_REPLAY:{error}"))?;
    if replay != candidate_source {
        return Err("SOURCE_PROPOSAL_ATOMIC_REPLAY_DIVERGED".to_string());
    }
    Ok(AtomicSourceCompositionIR {
        candidate_sha256: sha256(candidate_source.as_bytes()),
        candidate_source,
        edit,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::structural_source_repair::ByteRange;

    #[test]
    fn ranking_is_bounded_deduplicated_and_origin_neutral() {
        let candidates = (0..5)
            .map(|index| SourceProposalKernelInput {
                proposal_id: format!("proposal-{index}"),
                candidate_sha256: if index == 4 {
                    format!("{:064x}", 0)
                } else {
                    format!("{:064x}", index)
                },
                tie_breaker: format!("tie-{index}"),
                evidence: SourceProposalRankingEvidenceIR {
                    predicted_value: u16::try_from(index * 10).unwrap(),
                    ..Default::default()
                },
                payload: index,
            })
            .collect();
        assert_eq!(rank_source_proposals(candidates).unwrap(), [4, 3, 2]);
    }

    #[test]
    fn typed_evidence_not_proposal_identity_drives_ranking() {
        let candidates = vec![
            SourceProposalKernelInput {
                proposal_id: "A-LEXICOGRAPHICALLY-FIRST".to_string(),
                candidate_sha256: format!("{:064x}", 1),
                tie_breaker: "a".to_string(),
                evidence: SourceProposalRankingEvidenceIR {
                    predicted_value: 90,
                    counterexample_feedback: -100,
                    ..Default::default()
                },
                payload: "unsupported",
            },
            SourceProposalKernelInput {
                proposal_id: "Z-EVIDENCE-SUPPORTED".to_string(),
                candidate_sha256: format!("{:064x}", 2),
                tie_breaker: "z".to_string(),
                evidence: SourceProposalRankingEvidenceIR {
                    predicted_value: 60,
                    public_observation_support: 100,
                    required_family_members: 2,
                    ..Default::default()
                },
                payload: "supported",
            },
        ];
        assert_eq!(rank_source_proposals(candidates).unwrap()[0], "supported");
    }

    #[test]
    fn required_group_is_atomic_and_rejects_missing_members() {
        let proposals = vec![
            SourceEditProposalIR {
                proposal_id: "left".to_string(),
                edit: SourceEditAtom::Replace {
                    range: ByteRange { start: 0, end: 1 },
                    expected_sha256: sha256(b"a"),
                    replacement: "x".to_string(),
                },
            },
            SourceEditProposalIR {
                proposal_id: "right".to_string(),
                edit: SourceEditAtom::Replace {
                    range: ByteRange { start: 2, end: 3 },
                    expected_sha256: sha256(b"b"),
                    replacement: "z".to_string(),
                },
            },
        ];
        let requirement = SourceProposalCompositionRequirementIR::RequiredGroup {
            group_id: "paired-state-change".to_string(),
            expected_members: 2,
        };
        let composed = compose_source_edit_proposals("a b", &proposals, &requirement).unwrap();
        assert_eq!(composed.candidate_source, "x z");
        let incomplete = SourceProposalCompositionRequirementIR::RequiredGroup {
            group_id: "paired-state-change".to_string(),
            expected_members: 3,
        };
        assert_eq!(
            compose_source_edit_proposals("a b", &proposals, &incomplete),
            Err("SOURCE_PROPOSAL_REQUIRED_GROUP_INCOMPLETE".to_string())
        );
    }
}
