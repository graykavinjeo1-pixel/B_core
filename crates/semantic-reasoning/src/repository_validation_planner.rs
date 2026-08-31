//! Content-addressed validation impact planning for repository repairs.
//!
//! A prior passing proof is not reusable merely because its test name still
//! exists. Reuse is admitted only when the proof receipt, validation identity,
//! repository graph, and every dependency content hash remain bound. Local
//! source changes invalidate the reverse dependency closure. Structural graph,
//! manifest, configuration, or file-set changes fail closed to full-workspace
//! validation.

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::path::{Component, Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::repository_horizon::{
    RepositoryCausalEdgeIR, RepositoryCausalGraphIR, RepositoryEdgeKind, RepositoryFileKind,
    RepositoryFileNodeIR, REPOSITORY_HORIZON_SCHEMA,
};
use crate::self_repair_contract::sha256;

pub const REPOSITORY_VALIDATION_PLANNER_SCHEMA: &str = "B_REPOSITORY_VALIDATION_PLANNER_1";
pub const MAX_VALIDATION_CHANGES: usize = 256;
pub const MAX_VALIDATION_PROOFS: usize = 4_096;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RepositoryValidationScopeIR {
    ReuseOnly,
    AffectedDependencyClosure,
    FullWorkspace,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ValidationProofDispositionIR {
    Reused,
    Invalidated,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValidationIdentityIR {
    pub validation_contract_sha256: String,
    pub toolchain_sha256: String,
    pub evaluator_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct RepositoryFileChangeIR {
    pub relative_path: PathBuf,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub predecessor_sha256: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub candidate_sha256: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValidationProofReceiptIR {
    pub schema: String,
    pub proof_id: String,
    pub repository_graph_sha256: String,
    pub validation_identity: ValidationIdentityIR,
    pub subject_paths: Vec<PathBuf>,
    pub dependency_paths: Vec<PathBuf>,
    pub dependency_snapshot_sha256: String,
    pub validation_passed: bool,
    pub proof_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepositoryValidationRequestIR {
    pub schema: String,
    pub changes: Vec<RepositoryFileChangeIR>,
    pub prior_proofs: Vec<ValidationProofReceiptIR>,
    pub validation_identity: ValidationIdentityIR,
    pub max_affected_files: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValidationProofDecisionIR {
    pub proof_id: String,
    pub disposition: ValidationProofDispositionIR,
    pub reason_codes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepositoryValidationPlanIR {
    pub schema: String,
    pub predecessor_graph_sha256: String,
    pub candidate_graph_sha256: String,
    pub scope: RepositoryValidationScopeIR,
    pub changed_relative_paths: Vec<PathBuf>,
    pub affected_relative_paths: Vec<PathBuf>,
    pub selected_test_paths: Vec<PathBuf>,
    pub validation_target_paths: Vec<PathBuf>,
    pub reusable_proof_ids: Vec<String>,
    pub invalidated_proof_ids: Vec<String>,
    pub proof_decisions: Vec<ValidationProofDecisionIR>,
    pub escalation_reasons: Vec<String>,
    pub affected_budget_exhausted: bool,
    pub dependency_edges_examined: usize,
    pub full_catalog_rescans: u64,
    pub external_llm_calls: u64,
    pub network_reads: u64,
    pub source_mutation_authorized: bool,
    pub plan_sha256: String,
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn safe_relative_path(path: &Path) -> bool {
    !path.as_os_str().is_empty()
        && !path.is_absolute()
        && path
            .components()
            .all(|component| matches!(component, Component::Normal(_)))
}

fn normalized_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

fn graph_root_sha256(graph: &RepositoryCausalGraphIR) -> Result<String, String> {
    serde_json::to_vec(&(&graph.files, &graph.edges))
        .map(|bytes| sha256(&bytes))
        .map_err(|error| format!("VALIDATION_GRAPH_SERIALIZE:{error}"))
}

fn validate_graph(graph: &RepositoryCausalGraphIR) -> Result<(), String> {
    if graph.schema != REPOSITORY_HORIZON_SCHEMA
        || graph.files.len() != graph.indexed_files
        || !is_sha256(&graph.root_sha256)
        || graph.root_sha256 != graph_root_sha256(graph)?
        || graph.external_llm_calls != 0
        || graph.network_reads != 0
    {
        return Err("VALIDATION_GRAPH_ENVELOPE".to_string());
    }
    let mut paths = BTreeSet::new();
    for (expected_node, file) in graph.files.iter().enumerate() {
        if file.node_id != expected_node
            || !safe_relative_path(&file.relative_path)
            || !paths.insert(normalized_path(&file.relative_path))
            || !is_sha256(&file.content_sha256)
        {
            return Err("VALIDATION_GRAPH_FILE_BINDING".to_string());
        }
    }
    for edge in &graph.edges {
        if edge.from_node >= graph.files.len()
            || edge.to_node >= graph.files.len()
            || edge.from_node == edge.to_node
            || edge.evidence_symbol.trim().is_empty()
            || edge.confidence_millis > 1_000
        {
            return Err("VALIDATION_GRAPH_EDGE_BINDING".to_string());
        }
    }
    Ok(())
}

fn validate_identity(identity: &ValidationIdentityIR) -> bool {
    is_sha256(&identity.validation_contract_sha256)
        && is_sha256(&identity.toolchain_sha256)
        && is_sha256(&identity.evaluator_sha256)
}

fn graph_files_by_path(graph: &RepositoryCausalGraphIR) -> BTreeMap<String, &RepositoryFileNodeIR> {
    graph
        .files
        .iter()
        .map(|file| (normalized_path(&file.relative_path), file))
        .collect()
}

fn dependency_snapshot_sha256(
    graph: &RepositoryCausalGraphIR,
    dependency_paths: &[PathBuf],
) -> Result<String, String> {
    let files = graph_files_by_path(graph);
    let mut normalized = dependency_paths
        .iter()
        .map(|path| normalized_path(path))
        .collect::<Vec<_>>();
    normalized.sort();
    normalized.dedup();
    if normalized.len() != dependency_paths.len() {
        return Err("VALIDATION_PROOF_DUPLICATE_DEPENDENCY".to_string());
    }
    let rows = normalized
        .into_iter()
        .map(|path| {
            files
                .get(&path)
                .map(|file| (path, file.content_sha256.clone()))
                .ok_or_else(|| "VALIDATION_PROOF_DEPENDENCY_MISSING".to_string())
        })
        .collect::<Result<Vec<_>, _>>()?;
    serde_json::to_vec(&rows)
        .map(|bytes| sha256(&bytes))
        .map_err(|error| format!("VALIDATION_PROOF_DEPENDENCY_SERIALIZE:{error}"))
}

fn proof_sha256(receipt: &ValidationProofReceiptIR) -> Result<String, String> {
    serde_json::to_vec(&(
        &receipt.schema,
        &receipt.proof_id,
        &receipt.repository_graph_sha256,
        &receipt.validation_identity,
        &receipt.subject_paths,
        &receipt.dependency_paths,
        &receipt.dependency_snapshot_sha256,
        receipt.validation_passed,
    ))
    .map(|bytes| sha256(&bytes))
    .map_err(|error| format!("VALIDATION_PROOF_SERIALIZE:{error}"))
}

pub fn seal_validation_proof_receipt(
    graph: &RepositoryCausalGraphIR,
    proof_id: impl Into<String>,
    subject_paths: Vec<PathBuf>,
    dependency_paths: Vec<PathBuf>,
    validation_identity: ValidationIdentityIR,
) -> Result<ValidationProofReceiptIR, String> {
    validate_graph(graph)?;
    let proof_id = proof_id.into();
    if proof_id.trim().is_empty()
        || proof_id.len() > 256
        || subject_paths.is_empty()
        || dependency_paths.is_empty()
        || !validate_identity(&validation_identity)
        || subject_paths.iter().any(|path| !safe_relative_path(path))
        || dependency_paths
            .iter()
            .any(|path| !safe_relative_path(path))
    {
        return Err("VALIDATION_PROOF_ENVELOPE".to_string());
    }
    let dependency_set = dependency_paths
        .iter()
        .map(|path| normalized_path(path))
        .collect::<BTreeSet<_>>();
    if !subject_paths
        .iter()
        .all(|path| dependency_set.contains(&normalized_path(path)))
    {
        return Err("VALIDATION_PROOF_SUBJECT_NOT_DEPENDENCY".to_string());
    }
    let mut receipt = ValidationProofReceiptIR {
        schema: REPOSITORY_VALIDATION_PLANNER_SCHEMA.to_string(),
        proof_id,
        repository_graph_sha256: graph.root_sha256.clone(),
        validation_identity,
        subject_paths,
        dependency_snapshot_sha256: dependency_snapshot_sha256(graph, &dependency_paths)?,
        dependency_paths,
        validation_passed: true,
        proof_sha256: String::new(),
    };
    receipt.proof_sha256 = proof_sha256(&receipt)?;
    Ok(receipt)
}

fn actual_changes(
    predecessor: &RepositoryCausalGraphIR,
    candidate: &RepositoryCausalGraphIR,
) -> Vec<RepositoryFileChangeIR> {
    let predecessor_files = graph_files_by_path(predecessor);
    let candidate_files = graph_files_by_path(candidate);
    let paths = predecessor_files
        .keys()
        .chain(candidate_files.keys())
        .cloned()
        .collect::<BTreeSet<_>>();
    paths
        .into_iter()
        .filter_map(|path| {
            let before = predecessor_files
                .get(&path)
                .map(|file| file.content_sha256.clone());
            let after = candidate_files
                .get(&path)
                .map(|file| file.content_sha256.clone());
            (before != after).then(|| RepositoryFileChangeIR {
                relative_path: PathBuf::from(path),
                predecessor_sha256: before,
                candidate_sha256: after,
            })
        })
        .collect()
}

fn validate_request(
    predecessor: &RepositoryCausalGraphIR,
    candidate: &RepositoryCausalGraphIR,
    request: &RepositoryValidationRequestIR,
) -> Result<Vec<RepositoryFileChangeIR>, String> {
    if request.schema != REPOSITORY_VALIDATION_PLANNER_SCHEMA
        || request.changes.len() > MAX_VALIDATION_CHANGES
        || request.prior_proofs.len() > MAX_VALIDATION_PROOFS
        || request.max_affected_files == 0
        || request.max_affected_files > predecessor.files.len().max(candidate.files.len()).max(1)
        || !validate_identity(&request.validation_identity)
    {
        return Err("VALIDATION_REQUEST_ENVELOPE".to_string());
    }
    let mut proof_ids = BTreeSet::new();
    if request.prior_proofs.iter().any(|proof| {
        proof.proof_id.trim().is_empty()
            || proof.proof_id.len() > 256
            || !proof_ids.insert(proof.proof_id.clone())
    }) {
        return Err("VALIDATION_REQUEST_PROOF_ID".to_string());
    }
    let mut changes = request.changes.clone();
    changes.sort();
    if changes.windows(2).any(|pair| {
        normalized_path(&pair[0].relative_path) == normalized_path(&pair[1].relative_path)
    }) || changes.iter().any(|change| {
        !safe_relative_path(&change.relative_path)
            || change
                .predecessor_sha256
                .as_deref()
                .is_some_and(|value| !is_sha256(value))
            || change
                .candidate_sha256
                .as_deref()
                .is_some_and(|value| !is_sha256(value))
            || change.predecessor_sha256 == change.candidate_sha256
    }) {
        return Err("VALIDATION_REQUEST_CHANGE_BINDING".to_string());
    }
    let actual = actual_changes(predecessor, candidate);
    if changes != actual {
        return Err("VALIDATION_REQUEST_INCOMPLETE_CHANGESET".to_string());
    }
    Ok(actual)
}

type EdgeSignature = (String, String, RepositoryEdgeKind, String, u16);

fn edge_signatures(graph: &RepositoryCausalGraphIR) -> BTreeSet<EdgeSignature> {
    graph
        .edges
        .iter()
        .map(|edge| {
            (
                normalized_path(&graph.files[edge.from_node].relative_path),
                normalized_path(&graph.files[edge.to_node].relative_path),
                edge.kind,
                edge.evidence_symbol.clone(),
                edge.confidence_millis,
            )
        })
        .collect()
}

fn structural_escalations(
    predecessor: &RepositoryCausalGraphIR,
    candidate: &RepositoryCausalGraphIR,
    changes: &[RepositoryFileChangeIR],
) -> Vec<String> {
    let predecessor_files = graph_files_by_path(predecessor);
    let candidate_files = graph_files_by_path(candidate);
    let mut reasons = BTreeSet::new();
    if predecessor_files.keys().collect::<Vec<_>>() != candidate_files.keys().collect::<Vec<_>>() {
        reasons.insert("REPOSITORY_FILE_SET_CHANGED".to_string());
    }
    if edge_signatures(predecessor) != edge_signatures(candidate) {
        reasons.insert("REPOSITORY_CAUSAL_TOPOLOGY_CHANGED".to_string());
    }
    if predecessor.duplicate_symbol_definitions != candidate.duplicate_symbol_definitions {
        reasons.insert("DUPLICATE_SYMBOL_AUTHORITY_CHANGED".to_string());
    }
    if predecessor.skipped_files != candidate.skipped_files {
        reasons.insert("UNINDEXED_FILE_SURFACE_CHANGED".to_string());
    }
    for change in changes {
        let key = normalized_path(&change.relative_path);
        let kind = candidate_files
            .get(&key)
            .or_else(|| predecessor_files.get(&key))
            .map(|file| file.kind);
        if matches!(
            kind,
            Some(RepositoryFileKind::Manifest | RepositoryFileKind::Configuration)
        ) {
            reasons.insert("BUILD_OR_CONFIGURATION_CHANGED".to_string());
        }
    }
    for (path, predecessor_file) in &predecessor_files {
        if let Some(candidate_file) = candidate_files.get(path) {
            if predecessor_file.language != candidate_file.language
                || predecessor_file.kind != candidate_file.kind
            {
                reasons.insert("FILE_ROLE_CHANGED".to_string());
            }
        }
    }
    reasons.into_iter().collect()
}

fn affected_dependency_closure(
    predecessor: &RepositoryCausalGraphIR,
    candidate: &RepositoryCausalGraphIR,
    changed_paths: &BTreeSet<String>,
    max_affected_files: usize,
) -> (BTreeSet<String>, usize, bool) {
    let mut reverse = BTreeMap::<String, BTreeSet<String>>::new();
    let mut examined = 0usize;
    for graph in [predecessor, candidate] {
        for RepositoryCausalEdgeIR {
            from_node, to_node, ..
        } in &graph.edges
        {
            examined = examined.saturating_add(1);
            let dependent = normalized_path(&graph.files[*from_node].relative_path);
            let dependency = normalized_path(&graph.files[*to_node].relative_path);
            reverse.entry(dependency).or_default().insert(dependent);
        }
    }
    let mut affected = changed_paths.clone();
    let mut queue = changed_paths.iter().cloned().collect::<VecDeque<_>>();
    let mut exhausted = affected.len() > max_affected_files;
    while let Some(path) = queue.pop_front() {
        if exhausted {
            break;
        }
        for dependent in reverse.get(&path).into_iter().flatten() {
            if affected.insert(dependent.clone()) {
                if affected.len() > max_affected_files {
                    exhausted = true;
                    break;
                }
                queue.push_back(dependent.clone());
            }
        }
    }
    (affected, examined, exhausted)
}

fn invalid_proof_reasons(
    proof: &ValidationProofReceiptIR,
    predecessor: &RepositoryCausalGraphIR,
    request_identity: &ValidationIdentityIR,
) -> Vec<String> {
    let mut reasons = BTreeSet::new();
    if proof.schema != REPOSITORY_VALIDATION_PLANNER_SCHEMA
        || !proof.validation_passed
        || !is_sha256(&proof.repository_graph_sha256)
        || !is_sha256(&proof.dependency_snapshot_sha256)
        || !is_sha256(&proof.proof_sha256)
        || proof.subject_paths.is_empty()
        || proof.dependency_paths.is_empty()
        || proof
            .subject_paths
            .iter()
            .chain(proof.dependency_paths.iter())
            .any(|path| !safe_relative_path(path))
        || proof_sha256(proof).ok().as_deref() != Some(proof.proof_sha256.as_str())
    {
        reasons.insert("PROOF_RECEIPT_INVALID".to_string());
    }
    if proof.repository_graph_sha256 != predecessor.root_sha256 {
        reasons.insert("PROOF_PREDECESSOR_GRAPH_MISMATCH".to_string());
    }
    if proof.validation_identity.validation_contract_sha256
        != request_identity.validation_contract_sha256
    {
        reasons.insert("VALIDATION_CONTRACT_CHANGED".to_string());
    }
    if proof.validation_identity.toolchain_sha256 != request_identity.toolchain_sha256 {
        reasons.insert("TOOLCHAIN_CHANGED".to_string());
    }
    if proof.validation_identity.evaluator_sha256 != request_identity.evaluator_sha256 {
        reasons.insert("EVALUATOR_CHANGED".to_string());
    }
    match dependency_snapshot_sha256(predecessor, &proof.dependency_paths) {
        Ok(hash) if hash == proof.dependency_snapshot_sha256 => {}
        Ok(_) => {
            reasons.insert("PROOF_DEPENDENCY_SNAPSHOT_MISMATCH".to_string());
        }
        Err(_) => {
            reasons.insert("PROOF_DEPENDENCY_MISSING".to_string());
        }
    }
    let dependency_set = proof
        .dependency_paths
        .iter()
        .map(|path| normalized_path(path))
        .collect::<BTreeSet<_>>();
    if !proof
        .subject_paths
        .iter()
        .all(|path| dependency_set.contains(&normalized_path(path)))
    {
        reasons.insert("PROOF_SUBJECT_NOT_DEPENDENCY".to_string());
    }
    reasons.into_iter().collect()
}

fn plan_sha256(plan: &RepositoryValidationPlanIR) -> Result<String, String> {
    let mut hash_projection = plan.clone();
    hash_projection.plan_sha256.clear();
    serde_json::to_vec(&hash_projection)
        .map(|bytes| sha256(&bytes))
        .map_err(|error| format!("VALIDATION_PLAN_SERIALIZE:{error}"))
}

pub fn plan_repository_validation(
    predecessor: &RepositoryCausalGraphIR,
    candidate: &RepositoryCausalGraphIR,
    request: &RepositoryValidationRequestIR,
) -> Result<RepositoryValidationPlanIR, String> {
    validate_graph(predecessor)?;
    validate_graph(candidate)?;
    let changes = validate_request(predecessor, candidate, request)?;
    let changed_paths = changes
        .iter()
        .map(|change| normalized_path(&change.relative_path))
        .collect::<BTreeSet<_>>();
    let mut escalations = structural_escalations(predecessor, candidate, &changes);
    let (mut affected, dependency_edges_examined, budget_exhausted) = affected_dependency_closure(
        predecessor,
        candidate,
        &changed_paths,
        request.max_affected_files,
    );
    if budget_exhausted {
        escalations.push("AFFECTED_DEPENDENCY_BUDGET_EXHAUSTED".to_string());
        escalations.sort();
        escalations.dedup();
    }
    let scope = if !escalations.is_empty() {
        RepositoryValidationScopeIR::FullWorkspace
    } else if changes.is_empty() {
        RepositoryValidationScopeIR::ReuseOnly
    } else {
        RepositoryValidationScopeIR::AffectedDependencyClosure
    };
    let candidate_files = graph_files_by_path(candidate);
    if scope == RepositoryValidationScopeIR::FullWorkspace {
        affected = candidate_files.keys().cloned().collect();
    }
    let affected_relative_paths = affected.iter().map(PathBuf::from).collect::<Vec<_>>();
    let selected_test_paths = affected
        .iter()
        .filter_map(|path| {
            candidate_files
                .get(path)
                .filter(|file| file.kind == RepositoryFileKind::Test)
                .map(|file| file.relative_path.clone())
        })
        .collect::<Vec<_>>();
    let validation_target_paths = match scope {
        RepositoryValidationScopeIR::ReuseOnly => Vec::new(),
        RepositoryValidationScopeIR::AffectedDependencyClosure
        | RepositoryValidationScopeIR::FullWorkspace => affected_relative_paths.clone(),
    };

    let mut decisions = Vec::with_capacity(request.prior_proofs.len());
    for proof in &request.prior_proofs {
        let mut reasons = invalid_proof_reasons(proof, predecessor, &request.validation_identity);
        if scope == RepositoryValidationScopeIR::FullWorkspace {
            reasons.push("FULL_WORKSPACE_VALIDATION_REQUIRED".to_string());
        } else if proof
            .dependency_paths
            .iter()
            .map(|path| normalized_path(path))
            .any(|path| affected.contains(&path))
        {
            reasons.push("DEPENDENCY_CLOSURE_CHANGED".to_string());
        } else {
            match dependency_snapshot_sha256(candidate, &proof.dependency_paths) {
                Ok(hash) if hash == proof.dependency_snapshot_sha256 => {}
                Ok(_) => reasons.push("CANDIDATE_DEPENDENCY_CHANGED".to_string()),
                Err(_) => reasons.push("CANDIDATE_DEPENDENCY_MISSING".to_string()),
            }
        }
        reasons.sort();
        reasons.dedup();
        decisions.push(ValidationProofDecisionIR {
            proof_id: proof.proof_id.clone(),
            disposition: if reasons.is_empty() {
                ValidationProofDispositionIR::Reused
            } else {
                ValidationProofDispositionIR::Invalidated
            },
            reason_codes: reasons,
        });
    }
    let reusable_proof_ids = decisions
        .iter()
        .filter(|decision| decision.disposition == ValidationProofDispositionIR::Reused)
        .map(|decision| decision.proof_id.clone())
        .collect::<Vec<_>>();
    let invalidated_proof_ids = decisions
        .iter()
        .filter(|decision| decision.disposition == ValidationProofDispositionIR::Invalidated)
        .map(|decision| decision.proof_id.clone())
        .collect::<Vec<_>>();
    let mut plan = RepositoryValidationPlanIR {
        schema: REPOSITORY_VALIDATION_PLANNER_SCHEMA.to_string(),
        predecessor_graph_sha256: predecessor.root_sha256.clone(),
        candidate_graph_sha256: candidate.root_sha256.clone(),
        scope,
        changed_relative_paths: changed_paths.into_iter().map(PathBuf::from).collect(),
        affected_relative_paths,
        selected_test_paths,
        validation_target_paths,
        reusable_proof_ids,
        invalidated_proof_ids,
        proof_decisions: decisions,
        escalation_reasons: escalations,
        affected_budget_exhausted: budget_exhausted,
        dependency_edges_examined,
        full_catalog_rescans: 0,
        external_llm_calls: 0,
        network_reads: 0,
        source_mutation_authorized: false,
        plan_sha256: String::new(),
    };
    plan.plan_sha256 = plan_sha256(&plan)?;
    Ok(plan)
}

pub fn validate_repository_validation_plan(
    predecessor: &RepositoryCausalGraphIR,
    candidate: &RepositoryCausalGraphIR,
    request: &RepositoryValidationRequestIR,
    plan: &RepositoryValidationPlanIR,
) -> Result<(), String> {
    let replay = plan_repository_validation(predecessor, candidate, request)?;
    if &replay != plan || !is_sha256(&plan.plan_sha256) {
        return Err("VALIDATION_PLAN_REPLAY_MISMATCH".to_string());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repository_coding_knowledge::RepositoryLanguage;

    fn hash(label: &str) -> String {
        sha256(label.as_bytes())
    }

    fn file(
        node_id: usize,
        path: &str,
        kind: RepositoryFileKind,
        content: &str,
    ) -> RepositoryFileNodeIR {
        RepositoryFileNodeIR {
            node_id,
            relative_path: PathBuf::from(path),
            language: RepositoryLanguage::Rust,
            kind,
            content_sha256: hash(content),
            byte_count: content.len(),
            defined_symbols: vec![format!("symbol_{node_id}")],
            referenced_symbols: Vec::new(),
            import_targets: Vec::new(),
        }
    }

    fn graph(leaf_content: &str, extra_edge: bool) -> RepositoryCausalGraphIR {
        let files = vec![
            file(0, "src/leaf.rs", RepositoryFileKind::Source, leaf_content),
            file(1, "src/service.rs", RepositoryFileKind::Source, "service"),
            file(
                2,
                "tests/service.rs",
                RepositoryFileKind::Test,
                "service test",
            ),
            file(
                3,
                "src/unrelated.rs",
                RepositoryFileKind::Source,
                "unrelated",
            ),
            file(
                4,
                "tests/unrelated.rs",
                RepositoryFileKind::Test,
                "unrelated test",
            ),
        ];
        let mut edges = vec![
            RepositoryCausalEdgeIR {
                from_node: 1,
                to_node: 0,
                kind: RepositoryEdgeKind::Calls,
                evidence_symbol: "leaf".to_string(),
                confidence_millis: 1_000,
            },
            RepositoryCausalEdgeIR {
                from_node: 2,
                to_node: 1,
                kind: RepositoryEdgeKind::TestReferences,
                evidence_symbol: "service".to_string(),
                confidence_millis: 1_000,
            },
            RepositoryCausalEdgeIR {
                from_node: 4,
                to_node: 3,
                kind: RepositoryEdgeKind::TestReferences,
                evidence_symbol: "unrelated".to_string(),
                confidence_millis: 1_000,
            },
        ];
        if extra_edge {
            edges.push(RepositoryCausalEdgeIR {
                from_node: 1,
                to_node: 3,
                kind: RepositoryEdgeKind::Calls,
                evidence_symbol: "unrelated".to_string(),
                confidence_millis: 1_000,
            });
        }
        edges.sort();
        let mut graph = RepositoryCausalGraphIR {
            schema: REPOSITORY_HORIZON_SCHEMA.to_string(),
            root_sha256: String::new(),
            indexed_files: files.len(),
            indexed_bytes: files.iter().map(|node| node.byte_count as u64).sum(),
            files,
            edges,
            skipped_files: 0,
            duplicate_symbol_definitions: 0,
            initial_catalog_scans: 1,
            full_catalog_rescans: 0,
            external_llm_calls: 0,
            network_reads: 0,
        };
        graph.root_sha256 = graph_root_sha256(&graph).expect("graph hash");
        graph
    }

    fn identity() -> ValidationIdentityIR {
        ValidationIdentityIR {
            validation_contract_sha256: hash("contract"),
            toolchain_sha256: hash("toolchain"),
            evaluator_sha256: hash("evaluator"),
        }
    }

    fn request(
        before: &RepositoryCausalGraphIR,
        after: &RepositoryCausalGraphIR,
        proofs: Vec<ValidationProofReceiptIR>,
    ) -> RepositoryValidationRequestIR {
        RepositoryValidationRequestIR {
            schema: REPOSITORY_VALIDATION_PLANNER_SCHEMA.to_string(),
            changes: actual_changes(before, after),
            prior_proofs: proofs,
            validation_identity: identity(),
            max_affected_files: after.files.len(),
        }
    }

    #[test]
    fn local_change_invalidates_only_the_reverse_dependency_closure() {
        let before = graph("leaf-before", false);
        let after = graph("leaf-after", false);
        let service_proof = seal_validation_proof_receipt(
            &before,
            "service-proof",
            vec![PathBuf::from("tests/service.rs")],
            vec![
                PathBuf::from("src/leaf.rs"),
                PathBuf::from("src/service.rs"),
                PathBuf::from("tests/service.rs"),
            ],
            identity(),
        )
        .expect("service proof");
        let unrelated_proof = seal_validation_proof_receipt(
            &before,
            "unrelated-proof",
            vec![PathBuf::from("tests/unrelated.rs")],
            vec![
                PathBuf::from("src/unrelated.rs"),
                PathBuf::from("tests/unrelated.rs"),
            ],
            identity(),
        )
        .expect("unrelated proof");
        let request = request(&before, &after, vec![service_proof, unrelated_proof]);
        let plan = plan_repository_validation(&before, &after, &request).expect("plan");
        assert_eq!(
            plan.scope,
            RepositoryValidationScopeIR::AffectedDependencyClosure
        );
        assert_eq!(
            plan.affected_relative_paths,
            vec![
                PathBuf::from("src/leaf.rs"),
                PathBuf::from("src/service.rs"),
                PathBuf::from("tests/service.rs"),
            ]
        );
        assert_eq!(
            plan.selected_test_paths,
            vec![PathBuf::from("tests/service.rs")]
        );
        assert_eq!(plan.reusable_proof_ids, vec!["unrelated-proof"]);
        assert_eq!(plan.invalidated_proof_ids, vec!["service-proof"]);
        validate_repository_validation_plan(&before, &after, &request, &plan)
            .expect("replay validation");
    }

    #[test]
    fn topology_change_fails_closed_to_full_workspace_validation() {
        let before = graph("leaf-before", false);
        let after = graph("leaf-after", true);
        let proof = seal_validation_proof_receipt(
            &before,
            "unrelated-proof",
            vec![PathBuf::from("tests/unrelated.rs")],
            vec![
                PathBuf::from("src/unrelated.rs"),
                PathBuf::from("tests/unrelated.rs"),
            ],
            identity(),
        )
        .expect("proof");
        let request = request(&before, &after, vec![proof]);
        let plan = plan_repository_validation(&before, &after, &request).expect("plan");
        assert_eq!(plan.scope, RepositoryValidationScopeIR::FullWorkspace);
        assert!(plan
            .escalation_reasons
            .contains(&"REPOSITORY_CAUSAL_TOPOLOGY_CHANGED".to_string()));
        assert_eq!(plan.validation_target_paths.len(), after.files.len());
        assert!(plan.reusable_proof_ids.is_empty());
    }

    #[test]
    fn tampered_proof_and_plan_are_never_reused() {
        let before = graph("leaf-before", false);
        let after = graph("leaf-after", false);
        let mut proof = seal_validation_proof_receipt(
            &before,
            "unrelated-proof",
            vec![PathBuf::from("tests/unrelated.rs")],
            vec![
                PathBuf::from("src/unrelated.rs"),
                PathBuf::from("tests/unrelated.rs"),
            ],
            identity(),
        )
        .expect("proof");
        proof.dependency_snapshot_sha256 = hash("forged");
        let request = request(&before, &after, vec![proof]);
        let mut plan = plan_repository_validation(&before, &after, &request).expect("plan");
        assert!(plan.reusable_proof_ids.is_empty());
        assert!(plan.proof_decisions[0]
            .reason_codes
            .contains(&"PROOF_RECEIPT_INVALID".to_string()));
        plan.full_catalog_rescans = 1;
        assert_eq!(
            validate_repository_validation_plan(&before, &after, &request, &plan),
            Err("VALIDATION_PLAN_REPLAY_MISMATCH".to_string())
        );
    }

    #[test]
    fn incomplete_changeset_is_rejected_before_proof_reuse() {
        let before = graph("leaf-before", false);
        let after = graph("leaf-after", false);
        let mut request = request(&before, &after, Vec::new());
        request.changes.clear();
        assert_eq!(
            plan_repository_validation(&before, &after, &request),
            Err("VALIDATION_REQUEST_INCOMPLETE_CHANGESET".to_string())
        );
    }

    #[test]
    fn manifest_change_and_affected_budget_exhaustion_require_full_validation() {
        let mut manifest_before = graph("leaf-before", false);
        manifest_before.files[3].relative_path = PathBuf::from("Cargo.toml");
        manifest_before.files[3].kind = RepositoryFileKind::Manifest;
        manifest_before.root_sha256 = graph_root_sha256(&manifest_before).expect("graph hash");
        let mut manifest_after = manifest_before.clone();
        manifest_after.files[3].content_sha256 = hash("manifest-after");
        manifest_after.root_sha256 = graph_root_sha256(&manifest_after).expect("graph hash");
        let manifest_request = request(&manifest_before, &manifest_after, Vec::new());
        let manifest_plan =
            plan_repository_validation(&manifest_before, &manifest_after, &manifest_request)
                .expect("manifest plan");
        assert_eq!(
            manifest_plan.scope,
            RepositoryValidationScopeIR::FullWorkspace
        );
        assert!(manifest_plan
            .escalation_reasons
            .contains(&"BUILD_OR_CONFIGURATION_CHANGED".to_string()));

        let before = graph("leaf-before", false);
        let after = graph("leaf-after", false);
        let mut budget_request = request(&before, &after, Vec::new());
        budget_request.max_affected_files = 2;
        let budget_plan =
            plan_repository_validation(&before, &after, &budget_request).expect("budget plan");
        assert_eq!(
            budget_plan.scope,
            RepositoryValidationScopeIR::FullWorkspace
        );
        assert!(budget_plan.affected_budget_exhausted);
        assert!(budget_plan
            .escalation_reasons
            .contains(&"AFFECTED_DEPENDENCY_BUDGET_EXHAUSTED".to_string()));
    }
}
