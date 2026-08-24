//! Bounded cross-clause reasoning for long repository requirements.
//!
//! The issue-understanding frontend classifies individual clauses. This layer
//! adds references, inter-clause relations, implicit engineering constraints,
//! and fail-closed conflict detection. It never emits a patch or treats prose
//! as an observed repository fact.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::repository_issue_understanding::{
    extract_targets, issue_segments, understand_repository_issue, ClauseRelation,
    IssueUnderstandingDisposition, RepositoryIssueUnderstandingIR, MAX_ISSUE_BYTES,
    MAX_ISSUE_CLAIMS,
};

pub const REPOSITORY_REQUIREMENT_GRAPH_SCHEMA: &str = "B_REPOSITORY_REQUIREMENT_GRAPH_1";
pub const MAX_REFERENCE_BINDINGS: usize = 256;
pub const MAX_REQUIREMENT_EDGES: usize = 512;
pub const MAX_REQUIREMENT_CONSTRAINTS: usize = 512;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RequirementSubject {
    PublicApi,
    SerializedData,
    Dependencies,
    ObservableBehavior,
    Target(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RequirementPolicy {
    Preserve,
    Required,
    Forbidden,
    MayChange,
    MustChange,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ReferenceBindingStatus {
    Bound,
    Ambiguous,
    Unresolved,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RequirementEdgeKind {
    ConditionalOn,
    Follows,
    ContrastsWith,
    AlternativeTo,
    RefersTo,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RequirementConflictKind {
    IncompatiblePolicies,
    AmbiguousReference,
    UnresolvedReference,
    TruncatedInput,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RequirementGraphDisposition {
    Consistent,
    NeedsClarification,
    Conflicting,
    InvalidInput,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RequirementConstraintIR {
    pub constraint_id: usize,
    pub claim_index: usize,
    pub subject: RequirementSubject,
    pub policy: RequirementPolicy,
    pub implicit: bool,
    pub source_clause_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReferenceBindingIR {
    pub claim_index: usize,
    pub reference_class: String,
    pub status: ReferenceBindingStatus,
    pub antecedent_claim_index: Option<usize>,
    pub bound_targets: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RequirementEdgeIR {
    pub from_claim_index: usize,
    pub to_claim_index: usize,
    pub kind: RequirementEdgeKind,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RequirementConflictIR {
    pub kind: RequirementConflictKind,
    pub subject: Option<RequirementSubject>,
    pub claim_indices: Vec<usize>,
    pub policy_pair: Option<(RequirementPolicy, RequirementPolicy)>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepositoryRequirementGraphIR {
    pub schema: String,
    pub disposition: RequirementGraphDisposition,
    pub source_sha256: String,
    pub issue_understanding: RepositoryIssueUnderstandingIR,
    pub clause_count: usize,
    pub constraints: Vec<RequirementConstraintIR>,
    pub references: Vec<ReferenceBindingIR>,
    pub edges: Vec<RequirementEdgeIR>,
    pub conflicts: Vec<RequirementConflictIR>,
    pub implicit_constraints: usize,
    pub unresolved_references: usize,
    pub ambiguous_references: usize,
    pub issue_text_to_patch_shortcut_events: u64,
    pub external_llm_calls: u64,
    pub network_reads: u64,
}

fn sha256(bytes: &[u8]) -> String {
    let mut digest = Sha256::new();
    digest.update(bytes);
    format!("{:x}", digest.finalize())
}

fn contains_any(text: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| text.contains(needle))
}

fn reference_classes(clause: &str) -> Vec<String> {
    let lower = clause.to_ascii_lowercase();
    let words = lower
        .split(|character: char| !(character.is_alphanumeric() || character == '_'))
        .filter(|word| !word.is_empty())
        .collect::<BTreeSet<_>>();
    let mut classes = Vec::new();
    if ["it", "this", "that"]
        .iter()
        .any(|word| words.contains(word))
        || contains_any(&lower, &["그것", "이것", "이를 "])
    {
        classes.push("SINGULAR_OBJECT".to_string());
    }
    if ["they", "them", "these", "those"]
        .iter()
        .any(|word| words.contains(word))
        || contains_any(&lower, &["그것들", "이들"])
    {
        classes.push("PLURAL_OBJECT".to_string());
    }
    if ["its", "their"].iter().any(|word| words.contains(word))
        || contains_any(&lower, &["해당 ", "그 대상의", "그것의"])
    {
        classes.push("POSSESSIVE".to_string());
    }
    classes
}

fn classify_constraints(
    clause: &str,
    targets: &[String],
) -> Vec<(RequirementSubject, RequirementPolicy, bool)> {
    let lower = clause.to_ascii_lowercase();
    let mut output = BTreeSet::new();

    let public_api = contains_any(
        &lower,
        &[
            "public api",
            "exported api",
            "public signature",
            "existing callers",
            "backward compatible",
            "backwards compatible",
            "공개 api",
            "공개 시그니처",
            "기존 호출자",
            "하위 호환",
        ],
    );
    if public_api
        && contains_any(
            &lower,
            &[
                "preserve",
                "keep",
                "must remain",
                "continue to compile",
                "do not break",
                "유지",
                "깨지지",
                "계속 컴파일",
            ],
        )
    {
        let implicit = contains_any(&lower, &["existing callers", "기존 호출자"])
            && !contains_any(&lower, &["public api", "공개 api"]);
        output.insert((
            RequirementSubject::PublicApi,
            RequirementPolicy::Preserve,
            implicit,
        ));
    }
    if public_api
        && contains_any(
            &lower,
            &[
                "may change",
                "can change",
                "breaking change is allowed",
                "변경해도",
                "호환성을 깨도",
            ],
        )
    {
        output.insert((
            RequirementSubject::PublicApi,
            RequirementPolicy::MayChange,
            false,
        ));
    }
    if public_api
        && contains_any(
            &lower,
            &[
                "must change",
                "must rename",
                "replace the public",
                "rename the public",
                "반드시 변경",
                "반드시 이름을 변경",
            ],
        )
    {
        output.insert((
            RequirementSubject::PublicApi,
            RequirementPolicy::MustChange,
            false,
        ));
    }

    let data = contains_any(
        &lower,
        &[
            "wire format",
            "serialized format",
            "data format",
            "old payload",
            "existing data",
            "직렬화 형식",
            "데이터 형식",
            "기존 데이터",
            "이전 페이로드",
        ],
    );
    if data
        && contains_any(
            &lower,
            &[
                "preserve",
                "unchanged",
                "still read",
                "remain compatible",
                "유지",
                "변경하지",
                "계속 읽",
                "호환",
            ],
        )
    {
        output.insert((
            RequirementSubject::SerializedData,
            RequirementPolicy::Preserve,
            !contains_any(&lower, &["wire format", "serialized format", "직렬화 형식"]),
        ));
    }
    if data
        && contains_any(
            &lower,
            &[
                "must migrate",
                "must change",
                "replace the format",
                "반드시 마이그레이션",
                "형식을 변경",
            ],
        )
    {
        output.insert((
            RequirementSubject::SerializedData,
            RequirementPolicy::MustChange,
            false,
        ));
    }

    if contains_any(
        &lower,
        &[
            "no new dependencies",
            "do not add dependencies",
            "must not add dependencies",
            "without adding dependencies",
            "standard library only",
            "새 의존성을 추가하지",
            "의존성 추가 금지",
            "표준 라이브러리만",
        ],
    ) {
        output.insert((
            RequirementSubject::Dependencies,
            RequirementPolicy::Forbidden,
            contains_any(&lower, &["standard library only", "표준 라이브러리만"]),
        ));
    }
    if contains_any(
        &lower,
        &[
            "dependency changes are allowed",
            "may add a dependency",
            "adding a dependency is acceptable",
            "의존성을 추가해도",
            "의존성 변경 허용",
        ],
    ) {
        output.insert((
            RequirementSubject::Dependencies,
            RequirementPolicy::MayChange,
            false,
        ));
    }
    if contains_any(
        &lower,
        &[
            "must add a dependency",
            "new dependency is required",
            "requires a new dependency",
            "새 의존성이 필요",
            "의존성을 반드시 추가",
        ],
    ) {
        output.insert((
            RequirementSubject::Dependencies,
            RequirementPolicy::Required,
            false,
        ));
    }

    let behavior_policy = if contains_any(
        &lower,
        &[
            "must not change behavior",
            "observable behavior unchanged",
            "동작을 변경하지",
            "관찰 가능한 동작은 그대로",
        ],
    ) {
        Some(RequirementPolicy::Preserve)
    } else if contains_any(
        &lower,
        &[
            "must change behavior",
            "behavior must change",
            "동작을 반드시 변경",
        ],
    ) {
        Some(RequirementPolicy::MustChange)
    } else {
        None
    };
    if let Some(policy) = behavior_policy {
        if targets.is_empty() {
            output.insert((RequirementSubject::ObservableBehavior, policy, false));
        } else {
            output.extend(
                targets
                    .iter()
                    .cloned()
                    .map(|target| (RequirementSubject::Target(target), policy, false)),
            );
        }
    }

    output.into_iter().collect()
}

fn policies_conflict(left: RequirementPolicy, right: RequirementPolicy) -> bool {
    matches!(
        (left, right),
        (RequirementPolicy::Preserve, RequirementPolicy::MustChange)
            | (RequirementPolicy::MustChange, RequirementPolicy::Preserve)
            | (RequirementPolicy::Forbidden, RequirementPolicy::Required)
            | (RequirementPolicy::Required, RequirementPolicy::Forbidden)
            | (RequirementPolicy::Forbidden, RequirementPolicy::MayChange)
            | (RequirementPolicy::MayChange, RequirementPolicy::Forbidden)
    )
}

fn relation_edge(relation: ClauseRelation) -> Option<RequirementEdgeKind> {
    match relation {
        ClauseRelation::Standalone => None,
        ClauseRelation::Conditional => Some(RequirementEdgeKind::ConditionalOn),
        ClauseRelation::Contrastive => Some(RequirementEdgeKind::ContrastsWith),
        ClauseRelation::Temporal => Some(RequirementEdgeKind::Follows),
        ClauseRelation::Alternative => Some(RequirementEdgeKind::AlternativeTo),
    }
}

/// Compile a long issue into a bounded requirement graph.
pub fn compile_repository_requirement_graph(text: &str) -> RepositoryRequirementGraphIR {
    let issue_understanding = understand_repository_issue(text);
    let source_sha256 = sha256(text.trim().as_bytes());
    if text.trim().is_empty() || text.len() > MAX_ISSUE_BYTES {
        return RepositoryRequirementGraphIR {
            schema: REPOSITORY_REQUIREMENT_GRAPH_SCHEMA.to_string(),
            disposition: RequirementGraphDisposition::InvalidInput,
            source_sha256,
            issue_understanding,
            clause_count: 0,
            constraints: Vec::new(),
            references: Vec::new(),
            edges: Vec::new(),
            conflicts: vec![RequirementConflictIR {
                kind: RequirementConflictKind::TruncatedInput,
                subject: None,
                claim_indices: Vec::new(),
                policy_pair: None,
            }],
            implicit_constraints: 0,
            unresolved_references: 0,
            ambiguous_references: 0,
            issue_text_to_patch_shortcut_events: 0,
            external_llm_calls: 0,
            network_reads: 0,
        };
    }

    let segments = issue_segments(text);
    let mut constraints = Vec::new();
    let mut references = Vec::new();
    let mut edges = Vec::new();
    let mut recent_targets: Option<(usize, Vec<String>)> = None;
    for (claim_index, (_, clause)) in segments.iter().enumerate() {
        let targets = extract_targets(clause);
        for reference_class in reference_classes(clause) {
            let (status, antecedent_claim_index, bound_targets) = match &recent_targets {
                Some((antecedent, targets)) if targets.len() == 1 => (
                    ReferenceBindingStatus::Bound,
                    Some(*antecedent),
                    targets.clone(),
                ),
                Some((antecedent, targets)) => (
                    ReferenceBindingStatus::Ambiguous,
                    Some(*antecedent),
                    targets.clone(),
                ),
                None => (ReferenceBindingStatus::Unresolved, None, Vec::new()),
            };
            if let Some(antecedent) = antecedent_claim_index {
                edges.push(RequirementEdgeIR {
                    from_claim_index: claim_index,
                    to_claim_index: antecedent,
                    kind: RequirementEdgeKind::RefersTo,
                });
            }
            references.push(ReferenceBindingIR {
                claim_index,
                reference_class,
                status,
                antecedent_claim_index,
                bound_targets,
            });
        }
        if !targets.is_empty() {
            recent_targets = Some((claim_index, targets.clone()));
        }
        for (subject, policy, implicit) in classify_constraints(clause, &targets) {
            constraints.push(RequirementConstraintIR {
                constraint_id: constraints.len(),
                claim_index,
                subject,
                policy,
                implicit,
                source_clause_sha256: sha256(clause.trim().as_bytes()),
            });
        }
        if claim_index > 0 {
            if let Some(kind) = issue_understanding
                .claims
                .get(claim_index)
                .and_then(|claim| relation_edge(claim.relation))
            {
                edges.push(RequirementEdgeIR {
                    from_claim_index: claim_index,
                    to_claim_index: claim_index - 1,
                    kind,
                });
            }
        }
    }
    constraints.truncate(MAX_REQUIREMENT_CONSTRAINTS);
    references.truncate(MAX_REFERENCE_BINDINGS);
    edges.sort_by_key(|edge| (edge.from_claim_index, edge.to_claim_index, edge.kind as u8));
    edges.dedup();
    edges.truncate(MAX_REQUIREMENT_EDGES);

    let mut conflicts = Vec::new();
    let mut by_subject: BTreeMap<RequirementSubject, Vec<&RequirementConstraintIR>> =
        BTreeMap::new();
    for constraint in &constraints {
        by_subject
            .entry(constraint.subject.clone())
            .or_default()
            .push(constraint);
    }
    for (subject, subject_constraints) in by_subject {
        for (offset, left) in subject_constraints.iter().enumerate() {
            for right in subject_constraints.iter().skip(offset + 1) {
                if policies_conflict(left.policy, right.policy) {
                    conflicts.push(RequirementConflictIR {
                        kind: RequirementConflictKind::IncompatiblePolicies,
                        subject: Some(subject.clone()),
                        claim_indices: vec![left.claim_index, right.claim_index],
                        policy_pair: Some((left.policy, right.policy)),
                    });
                }
            }
        }
    }
    for reference in &references {
        let kind = match reference.status {
            ReferenceBindingStatus::Bound => continue,
            ReferenceBindingStatus::Ambiguous => RequirementConflictKind::AmbiguousReference,
            ReferenceBindingStatus::Unresolved => RequirementConflictKind::UnresolvedReference,
        };
        conflicts.push(RequirementConflictIR {
            kind,
            subject: None,
            claim_indices: vec![reference.claim_index],
            policy_pair: None,
        });
    }
    conflicts.sort_by_key(|conflict| (conflict.kind as u8, conflict.claim_indices.clone()));
    conflicts.dedup();

    let unresolved_references = references
        .iter()
        .filter(|reference| reference.status == ReferenceBindingStatus::Unresolved)
        .count();
    let ambiguous_references = references
        .iter()
        .filter(|reference| reference.status == ReferenceBindingStatus::Ambiguous)
        .count();
    let policy_conflict = conflicts
        .iter()
        .any(|conflict| conflict.kind == RequirementConflictKind::IncompatiblePolicies);
    let disposition = if segments.is_empty()
        || issue_understanding.disposition == IssueUnderstandingDisposition::InvalidInput
    {
        RequirementGraphDisposition::InvalidInput
    } else if policy_conflict {
        RequirementGraphDisposition::Conflicting
    } else if unresolved_references > 0
        || ambiguous_references > 0
        || issue_understanding.disposition == IssueUnderstandingDisposition::NeedsClarification
    {
        RequirementGraphDisposition::NeedsClarification
    } else {
        RequirementGraphDisposition::Consistent
    };
    let implicit_constraints = constraints
        .iter()
        .filter(|constraint| constraint.implicit)
        .count();

    RepositoryRequirementGraphIR {
        schema: REPOSITORY_REQUIREMENT_GRAPH_SCHEMA.to_string(),
        disposition,
        source_sha256,
        issue_understanding,
        clause_count: segments.len().min(MAX_ISSUE_CLAIMS),
        constraints,
        references,
        edges,
        conflicts,
        implicit_constraints,
        unresolved_references,
        ambiguous_references,
        issue_text_to_patch_shortcut_events: 0,
        external_llm_calls: 0,
        network_reads: 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ninety_clause_issue_preserves_late_constraints_and_temporal_edges() {
        let mut issue = String::from(
            "Actual behavior: `decode()` returns an assertion mismatch.\nExpected behavior: `decode()` must return the expected value.\nReproduction: run `decode()` with the fixture.\n",
        );
        for index in 0..84 {
            issue.push_str(&format!(
                "Then inspect `layer_{index}()` before proceeding.\n"
            ));
        }
        issue.push_str("Existing callers of `decode()` must continue to compile.\n");
        issue.push_str("Use the standard library only; no new dependencies.\n");
        issue.push_str("Verification: run the regression suite.\n");

        let graph = compile_repository_requirement_graph(&issue);
        assert_eq!(graph.clause_count, 91);
        assert!(graph.constraints.iter().any(|constraint| constraint.subject
            == RequirementSubject::PublicApi
            && constraint.policy == RequirementPolicy::Preserve));
        assert!(graph.constraints.iter().any(|constraint| constraint.subject
            == RequirementSubject::Dependencies
            && constraint.policy == RequirementPolicy::Forbidden));
        assert!(
            graph
                .edges
                .iter()
                .filter(|edge| edge.kind == RequirementEdgeKind::Follows)
                .count()
                >= 84
        );
        assert_eq!(graph.issue_text_to_patch_shortcut_events, 0);
        assert_eq!(graph.external_llm_calls, 0);
    }

    #[test]
    fn implicit_compatibility_and_explicit_migration_conflict_fail_closed() {
        let issue = "Actual behavior: `load()` rejects an old payload.\n\
Expected behavior: old clients must still read the existing data format.\n\
The serialized format must change and replace the old payload.\n\
Reproduction: run `load()` on the fixture.";
        let graph = compile_repository_requirement_graph(issue);
        assert_eq!(graph.disposition, RequirementGraphDisposition::Conflicting);
        assert!(graph.implicit_constraints >= 1);
        assert!(graph.conflicts.iter().any(|conflict| {
            conflict.kind == RequirementConflictKind::IncompatiblePolicies
                && conflict.subject == Some(RequirementSubject::SerializedData)
        }));
    }

    #[test]
    fn references_bind_only_to_a_unique_nearest_target() {
        let bound = compile_repository_requirement_graph(
            "Actual behavior: `reader()` panics. Then call it after initialization. Expected behavior: `reader()` must not panic.",
        );
        assert!(bound.references.iter().any(|reference| {
            reference.status == ReferenceBindingStatus::Bound
                && reference.bound_targets == vec!["reader()".to_string()]
        }));

        let ambiguous = compile_repository_requirement_graph(
            "Actual behavior: `reader()` and `writer()` disagree. Then call it after initialization. Expected behavior: both must agree.",
        );
        assert_eq!(
            ambiguous.disposition,
            RequirementGraphDisposition::NeedsClarification
        );
        assert_eq!(ambiguous.ambiguous_references, 1);
    }

    #[test]
    fn dependency_authority_conflict_is_not_silently_resolved() {
        let graph = compile_repository_requirement_graph(
            "Actual behavior: `resolve()` fails. Expected behavior: `resolve()` succeeds. Reproduction: call `resolve()`. No new dependencies may be added. A new dependency is required.",
        );
        assert_eq!(graph.disposition, RequirementGraphDisposition::Conflicting);
        assert!(graph.conflicts.iter().any(|conflict| {
            conflict.subject == Some(RequirementSubject::Dependencies)
                && conflict.policy_pair
                    == Some((RequirementPolicy::Forbidden, RequirementPolicy::Required))
        }));
    }
}
