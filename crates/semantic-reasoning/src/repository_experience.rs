//! Public repository-issue intake and causal pipeline provenance.
//!
//! The source synthesizer can only learn a transferable repair when the
//! observed issue, exact source owner, materialized edit, verifier output, and
//! promoted operator remain connected.  This module transports that identity
//! without granting natural language patch authority.  Repository source and
//! public tests still decide what can be synthesized; the independent
//! verifier still decides what can be accepted.

use std::collections::BTreeSet;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::self_repair_contract::sha256;

pub const REPOSITORY_ISSUE_INTAKE_REQUEST_SCHEMA: &str = "B_CORE_REPOSITORY_ISSUE_INTAKE_REQUEST_1";
pub const REPOSITORY_ISSUE_EVIDENCE_SCHEMA: &str = "B_CORE_REPOSITORY_ISSUE_EVIDENCE_1";
pub const REPOSITORY_ISSUE_INTAKE_RECEIPT_SCHEMA: &str = "B_CORE_REPOSITORY_ISSUE_INTAKE_RECEIPT_1";
pub const REPOSITORY_PIPELINE_PROVENANCE_SCHEMA: &str = "B_CORE_REPOSITORY_PIPELINE_PROVENANCE_1";
pub const REPOSITORY_REPAIR_CONTRACT_SCHEMA: &str = "B_CORE_REPOSITORY_REPAIR_CONTRACT_1";

const MAX_PROBLEM_STATEMENT_BYTES: usize = 64 * 1024;
const MAX_ISSUE_CLAIMS: usize = 16;
const MAX_ISSUE_CLAIM_BYTES: usize = 1_024;
const MAX_ISSUE_REFERENCES: usize = 64;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepositoryIssueIntakeRequestIR {
    pub schema: String,
    #[serde(default)]
    pub issue_id: String,
    pub problem_statement: String,
    pub paths: Vec<PathBuf>,
    #[serde(default)]
    pub evidence_artifacts: Vec<PathBuf>,
    #[serde(default)]
    pub occurred_at_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepositoryIssueEvidenceIR {
    pub schema: String,
    pub issue_id: String,
    pub problem_statement_sha256: String,
    pub observed_behavior: Vec<String>,
    pub expected_behavior: Vec<String>,
    pub constraints: Vec<String>,
    pub referenced_paths: Vec<String>,
    pub referenced_symbols: Vec<String>,
    /// Natural language is evidence and localization context, never an edit
    /// or verifier authority.
    pub natural_language_is_patch_authority: bool,
    pub evidence_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepositoryIssuePathBindingIR {
    pub root_index: usize,
    pub relative_path: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepositoryIssueIntakeReceiptIR {
    pub schema: String,
    pub issue: RepositoryIssueEvidenceIR,
    pub path_bindings: Vec<RepositoryIssuePathBindingIR>,
    pub evidence_artifact_sha256s: Vec<String>,
    pub work_event_id: String,
    pub occurred_at_ms: u64,
    pub receipt_sha256: String,
}

/// A problem-independent synthesis contract generated from bound public issue
/// evidence and an actual validation. It controls the compiler call; it is
/// not a prose report and cannot waive a verifier boundary.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepositoryRepairContractIR {
    #[serde(default)]
    pub schema: String,
    #[serde(default)]
    pub contract_id: String,
    #[serde(default)]
    pub issue_evidence_sha256s: Vec<String>,
    #[serde(default)]
    pub validation_id: String,
    #[serde(default)]
    pub target_symbols: Vec<String>,
    #[serde(default)]
    pub allowed_edit_atoms: Vec<String>,
    #[serde(default)]
    pub max_expression_depth: usize,
    #[serde(default)]
    pub max_candidates: usize,
    #[serde(default)]
    pub exact_source_owner_required: bool,
    #[serde(default)]
    pub public_behavioral_verification_required: bool,
    #[serde(default)]
    pub atomic_install_and_rollback_required: bool,
    #[serde(default)]
    pub natural_language_is_patch_authority: bool,
    #[serde(default)]
    pub contract_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepositoryPipelineProvenanceNodeIR {
    pub node_id: String,
    pub kind: String,
    pub semantic_identity: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepositoryPipelineProvenanceEdgeIR {
    pub edge_id: String,
    pub source_node_id: String,
    pub target_node_id: String,
    pub transition: String,
    pub decision: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepositoryPipelineProvenanceGraphIR {
    #[serde(default)]
    pub schema: String,
    #[serde(default)]
    pub task_identity_sha256: String,
    #[serde(default)]
    pub nodes: Vec<RepositoryPipelineProvenanceNodeIR>,
    #[serde(default)]
    pub edges: Vec<RepositoryPipelineProvenanceEdgeIR>,
    #[serde(default)]
    pub graph_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepositoryRepairProvenanceInput<'a> {
    pub issue_evidence_sha256s: &'a [String],
    pub validation_id: &'a str,
    pub target_symbols: &'a [String],
    pub repair_contract_sha256: Option<&'a str>,
    pub source_bound_receipt_sha256: Option<&'a str>,
    pub candidate_sha256: Option<&'a str>,
    pub sandbox_output_sha256: Option<&'a str>,
    pub authoritative_output_sha256: Option<&'a str>,
    pub promoted_operator_ids: &'a [String],
}

fn canonical_sha<T: Serialize>(value: &T) -> Result<String, String> {
    serde_json::to_vec(value)
        .map(|bytes| sha256(&bytes))
        .map_err(|error| format!("REPOSITORY_EXPERIENCE_HASH:{error}"))
}

fn valid_issue_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 80
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
}

fn normalized_claim(value: &str) -> Option<String> {
    let value = value
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .trim_matches(|character: char| matches!(character, '#' | '-' | '*' | ':' | ' '))
        .to_string();
    (!value.is_empty() && value.len() <= MAX_ISSUE_CLAIM_BYTES).then_some(value)
}

fn push_bounded_unique(values: &mut Vec<String>, value: String, limit: usize) {
    if values.len() < limit && !values.contains(&value) {
        values.push(value);
    }
}

fn label_kind(value: &str) -> Option<&'static str> {
    let normalized = value
        .trim()
        .trim_start_matches('#')
        .trim()
        .trim_end_matches(':')
        .trim()
        .to_ascii_lowercase();
    match normalized.as_str() {
        "actual" | "actual behavior" | "actual result" | "observed" | "observed behavior"
        | "current behavior" | "problem" | "error" | "현재" | "실제" | "문제" | "오류" => {
            Some("OBSERVED")
        }
        "expected" | "expected behavior" | "expected result" | "desired behavior" | "proposal"
        | "solution" | "기대" | "예상" | "요구" | "원하는 동작" => Some("EXPECTED"),
        "constraint" | "constraints" | "compatibility" | "제약" | "호환성" => {
            Some("CONSTRAINT")
        }
        _ => None,
    }
}

fn classify_claim(value: &str) -> Option<&'static str> {
    let lowered = value.to_ascii_lowercase();
    if [
        "currently",
        "actual",
        "observed",
        "fails",
        "failure",
        "error",
        "bug",
        "crash",
        "현재",
        "실제",
        "오류",
        "실패",
        "문제",
    ]
    .iter()
    .any(|cue| lowered.contains(cue))
    {
        return Some("OBSERVED");
    }
    // Negative/compatibility constraints must be classified before the
    // generic `must` cue.  Otherwise "must not" is silently transported as a
    // desired postcondition and can point localization at the wrong owner.
    if ["must not", "without changing", "compatible", "제약", "호환"]
        .iter()
        .any(|cue| lowered.contains(cue))
    {
        return Some("CONSTRAINT");
    }
    if [
        "expected",
        "should",
        "must",
        "needs to",
        "desired",
        "instead",
        "기대",
        "예상",
        "해야",
        "되어야",
        "필요",
    ]
    .iter()
    .any(|cue| lowered.contains(cue))
    {
        return Some("EXPECTED");
    }
    None
}

fn identifier(value: &str) -> bool {
    let mut parts = value.split('.');
    let mut count = 0_usize;
    for part in &mut parts {
        let mut bytes = part.bytes();
        let Some(first) = bytes.next() else {
            return false;
        };
        if !(first == b'_' || first.is_ascii_alphabetic())
            || !bytes.all(|byte| byte == b'_' || byte.is_ascii_alphanumeric())
        {
            return false;
        }
        count = count.saturating_add(1);
    }
    count > 0
}

fn extract_references(statement: &str) -> (Vec<String>, Vec<String>) {
    let mut paths = Vec::new();
    let mut symbols = Vec::new();
    for raw in statement.split_whitespace() {
        let token = raw.trim_matches(|character: char| {
            matches!(
                character,
                '`' | '\'' | '"' | '(' | ')' | '[' | ']' | '{' | '}' | ',' | ':' | ';' | '.'
            )
        });
        let normalized_path = token.replace('\\', "/");
        if normalized_path.len() <= 512
            && (normalized_path.ends_with(".py")
                || normalized_path.ends_with(".rs")
                || normalized_path.ends_with(".toml"))
        {
            push_bounded_unique(&mut paths, normalized_path, MAX_ISSUE_REFERENCES);
        }
        let symbol = token.trim_end_matches("()").trim_end_matches('(');
        if symbol.len() <= 256
            && identifier(symbol)
            && (symbol.contains('.') || token.ends_with('(') || raw.starts_with('`'))
        {
            push_bounded_unique(&mut symbols, symbol.to_string(), MAX_ISSUE_REFERENCES);
        }
    }
    (paths, symbols)
}

pub fn ground_repository_issue(
    request: &RepositoryIssueIntakeRequestIR,
) -> Result<RepositoryIssueEvidenceIR, String> {
    if request.schema != REPOSITORY_ISSUE_INTAKE_REQUEST_SCHEMA
        || request.problem_statement.trim().is_empty()
        || request.problem_statement.len() > MAX_PROBLEM_STATEMENT_BYTES
        || request.paths.is_empty()
        || request.paths.len() > 32
        || request.evidence_artifacts.len() > 32
    {
        return Err("REPOSITORY_ISSUE_INTAKE_INVALID".to_string());
    }
    let statement = request.problem_statement.replace('\0', " ");
    let statement_sha256 = sha256(statement.as_bytes());
    let issue_id = if request.issue_id.is_empty() {
        format!("issue-{}", &statement_sha256[..32])
    } else {
        request.issue_id.clone()
    };
    if !valid_issue_id(&issue_id) {
        return Err("REPOSITORY_ISSUE_ID_INVALID".to_string());
    }

    let mut observed = Vec::new();
    let mut expected = Vec::new();
    let mut constraints = Vec::new();
    let mut active_section = None;
    for raw_line in statement.lines() {
        let line = raw_line.trim();
        if line.is_empty() {
            continue;
        }
        if let Some(kind) = label_kind(line) {
            active_section = Some(kind);
            continue;
        }
        let (label, body) = line
            .split_once(':')
            .map(|(label, body)| (label_kind(label), body))
            .unwrap_or((None, line));
        let kind = label.or(active_section).or_else(|| classify_claim(body));
        let Some(claim) = normalized_claim(body) else {
            continue;
        };
        match kind {
            Some("OBSERVED") => push_bounded_unique(&mut observed, claim, MAX_ISSUE_CLAIMS),
            Some("EXPECTED") => push_bounded_unique(&mut expected, claim, MAX_ISSUE_CLAIMS),
            Some("CONSTRAINT") => push_bounded_unique(&mut constraints, claim, MAX_ISSUE_CLAIMS),
            _ => {}
        }
    }
    let (referenced_paths, referenced_symbols) = extract_references(&statement);
    if observed.is_empty() || expected.is_empty() {
        return Err("REPOSITORY_ISSUE_OBSERVED_EXPECTED_CONTRACT_MISSING".to_string());
    }
    let hash_body = (
        REPOSITORY_ISSUE_EVIDENCE_SCHEMA,
        issue_id.as_str(),
        statement_sha256.as_str(),
        &observed,
        &expected,
        &constraints,
        &referenced_paths,
        &referenced_symbols,
        false,
    );
    let evidence_sha256 = canonical_sha(&hash_body)?;
    Ok(RepositoryIssueEvidenceIR {
        schema: REPOSITORY_ISSUE_EVIDENCE_SCHEMA.to_string(),
        issue_id,
        problem_statement_sha256: statement_sha256,
        observed_behavior: observed,
        expected_behavior: expected,
        constraints,
        referenced_paths,
        referenced_symbols,
        natural_language_is_patch_authority: false,
        evidence_sha256,
    })
}

pub fn validate_repository_issue_evidence(
    evidence: &RepositoryIssueEvidenceIR,
) -> Result<(), String> {
    let hash_body = (
        REPOSITORY_ISSUE_EVIDENCE_SCHEMA,
        evidence.issue_id.as_str(),
        evidence.problem_statement_sha256.as_str(),
        &evidence.observed_behavior,
        &evidence.expected_behavior,
        &evidence.constraints,
        &evidence.referenced_paths,
        &evidence.referenced_symbols,
        evidence.natural_language_is_patch_authority,
    );
    if evidence.schema != REPOSITORY_ISSUE_EVIDENCE_SCHEMA
        || !valid_issue_id(&evidence.issue_id)
        || evidence.problem_statement_sha256.len() != 64
        || evidence.evidence_sha256 != canonical_sha(&hash_body)?
        || evidence.natural_language_is_patch_authority
        || evidence.observed_behavior.len() > MAX_ISSUE_CLAIMS
        || evidence.expected_behavior.len() > MAX_ISSUE_CLAIMS
        || evidence.observed_behavior.is_empty()
        || evidence.expected_behavior.is_empty()
        || evidence.constraints.len() > MAX_ISSUE_CLAIMS
        || evidence.referenced_paths.len() > MAX_ISSUE_REFERENCES
        || evidence.referenced_symbols.len() > MAX_ISSUE_REFERENCES
        || evidence
            .referenced_symbols
            .iter()
            .any(|symbol| !identifier(symbol))
    {
        return Err("REPOSITORY_ISSUE_EVIDENCE_INVALID".to_string());
    }
    Ok(())
}

pub fn derive_repository_repair_contract(
    issue_evidence_sha256s: &[String],
    validation_id: &str,
    target_symbols: &[String],
) -> Result<RepositoryRepairContractIR, String> {
    let mut issues = issue_evidence_sha256s.to_vec();
    issues.sort();
    issues.dedup();
    let mut targets = target_symbols
        .iter()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .collect::<Vec<_>>();
    targets.sort();
    targets.dedup();
    let allowed_edit_atoms = vec![
        "REPLACE".to_string(),
        "INSERT".to_string(),
        "DELETE".to_string(),
        "MOVE".to_string(),
        "ATOMIC_MULTI_EDIT".to_string(),
    ];
    let contract_id = canonical_sha(&(
        REPOSITORY_REPAIR_CONTRACT_SCHEMA,
        &issues,
        validation_id,
        &targets,
    ))?;
    let mut contract = RepositoryRepairContractIR {
        schema: REPOSITORY_REPAIR_CONTRACT_SCHEMA.to_string(),
        contract_id,
        issue_evidence_sha256s: issues,
        validation_id: validation_id.to_string(),
        target_symbols: targets,
        allowed_edit_atoms,
        max_expression_depth: 3,
        max_candidates: 2_048,
        exact_source_owner_required: true,
        public_behavioral_verification_required: true,
        atomic_install_and_rollback_required: true,
        natural_language_is_patch_authority: false,
        contract_sha256: String::new(),
    };
    contract.contract_sha256 = repository_repair_contract_hash(&contract)?;
    validate_repository_repair_contract(&contract)?;
    Ok(contract)
}

fn repository_repair_contract_hash(
    contract: &RepositoryRepairContractIR,
) -> Result<String, String> {
    canonical_sha(&(
        REPOSITORY_REPAIR_CONTRACT_SCHEMA,
        contract.contract_id.as_str(),
        &contract.issue_evidence_sha256s,
        contract.validation_id.as_str(),
        &contract.target_symbols,
        &contract.allowed_edit_atoms,
        contract.max_expression_depth,
        contract.max_candidates,
        contract.exact_source_owner_required,
        contract.public_behavioral_verification_required,
        contract.atomic_install_and_rollback_required,
        contract.natural_language_is_patch_authority,
    ))
}

pub fn validate_repository_repair_contract(
    contract: &RepositoryRepairContractIR,
) -> Result<(), String> {
    let expected_atoms = ["REPLACE", "INSERT", "DELETE", "MOVE", "ATOMIC_MULTI_EDIT"]
        .map(str::to_string)
        .to_vec();
    if contract.schema != REPOSITORY_REPAIR_CONTRACT_SCHEMA
        || contract.contract_id.len() != 64
        || contract.validation_id.is_empty()
        || contract
            .issue_evidence_sha256s
            .iter()
            .any(|hash| hash.len() != 64 || !hash.bytes().all(|byte| byte.is_ascii_hexdigit()))
        || contract
            .target_symbols
            .iter()
            .any(|symbol| !identifier(symbol))
        || contract.allowed_edit_atoms != expected_atoms
        || contract.max_expression_depth == 0
        || contract.max_expression_depth > 8
        || contract.max_candidates == 0
        || contract.max_candidates > 8_192
        || !contract.exact_source_owner_required
        || !contract.public_behavioral_verification_required
        || !contract.atomic_install_and_rollback_required
        || contract.natural_language_is_patch_authority
        || contract.contract_sha256 != repository_repair_contract_hash(contract)?
    {
        return Err("REPOSITORY_REPAIR_CONTRACT_INVALID".to_string());
    }
    Ok(())
}

fn provenance_node(kind: &str, identity: &str) -> RepositoryPipelineProvenanceNodeIR {
    RepositoryPipelineProvenanceNodeIR {
        node_id: sha256(format!("{kind}:{identity}").as_bytes()),
        kind: kind.to_string(),
        semantic_identity: identity.to_string(),
    }
}

fn provenance_edge(
    source: &RepositoryPipelineProvenanceNodeIR,
    target: &RepositoryPipelineProvenanceNodeIR,
    transition: &str,
    decision: &str,
) -> RepositoryPipelineProvenanceEdgeIR {
    RepositoryPipelineProvenanceEdgeIR {
        edge_id: sha256(
            format!(
                "{}:{}:{transition}:{decision}",
                source.node_id, target.node_id
            )
            .as_bytes(),
        ),
        source_node_id: source.node_id.clone(),
        target_node_id: target.node_id.clone(),
        transition: transition.to_string(),
        decision: decision.to_string(),
    }
}

pub fn build_repository_repair_provenance(
    input: &RepositoryRepairProvenanceInput<'_>,
) -> Result<RepositoryPipelineProvenanceGraphIR, String> {
    let task_identity_sha256 = canonical_sha(&(
        input.issue_evidence_sha256s,
        input.validation_id,
        input.target_symbols,
    ))?;
    let validation = provenance_node("VALIDATION", input.validation_id);
    let target_identity = canonical_sha(&input.target_symbols)?;
    let target = provenance_node("TARGET_FRONTIER", &target_identity);
    let mut nodes = vec![validation.clone(), target.clone()];
    let mut edges = vec![provenance_edge(
        &validation,
        &target,
        "VALIDATION_TO_TARGET_FRONTIER",
        if input.target_symbols.is_empty() {
            "TEST_DISCOVERED"
        } else {
            "ISSUE_OR_CONTRACT_BOUND"
        },
    )];
    for issue_sha256 in input.issue_evidence_sha256s {
        let issue = provenance_node("ISSUE_EVIDENCE", issue_sha256);
        edges.push(provenance_edge(
            &issue,
            &validation,
            "ISSUE_TO_VALIDATION",
            "TRANSPORTED_NOT_AUTHORIZED",
        ));
        nodes.push(issue);
    }
    let mut predecessor = target;
    if let Some(contract_sha256) = input.repair_contract_sha256 {
        let contract = provenance_node("REPAIR_CONTRACT", contract_sha256);
        edges.push(provenance_edge(
            &predecessor,
            &contract,
            "TARGET_FRONTIER_TO_REPAIR_CONTRACT",
            "AUTONOMOUSLY_DERIVED",
        ));
        nodes.push(contract.clone());
        predecessor = contract;
    }
    if let Some(receipt_sha256) = input.source_bound_receipt_sha256 {
        let synthesis = provenance_node("SOURCE_BOUND_SYNTHESIS", receipt_sha256);
        edges.push(provenance_edge(
            &predecessor,
            &synthesis,
            "TARGET_FRONTIER_TO_SYNTHESIS",
            "GENERATED",
        ));
        nodes.push(synthesis.clone());
        predecessor = synthesis;
    }
    if let Some(candidate_sha256) = input.candidate_sha256 {
        let materialization = provenance_node("MATERIALIZATION", candidate_sha256);
        edges.push(provenance_edge(
            &predecessor,
            &materialization,
            "SYNTHESIS_TO_MATERIALIZATION",
            "ONE_TO_ONE_CANDIDATE",
        ));
        nodes.push(materialization.clone());
        predecessor = materialization;
    }
    if let Some(output_sha256) = input.sandbox_output_sha256 {
        let verification = provenance_node("SANDBOX_VERIFICATION", output_sha256);
        edges.push(provenance_edge(
            &predecessor,
            &verification,
            "MATERIALIZATION_TO_SANDBOX_VERIFICATION",
            "PASS",
        ));
        nodes.push(verification.clone());
        predecessor = verification;
    }
    if let Some(output_sha256) = input.authoritative_output_sha256 {
        let installation = provenance_node("AUTHORITATIVE_INSTALLATION", output_sha256);
        edges.push(provenance_edge(
            &predecessor,
            &installation,
            "SANDBOX_TO_AUTHORITATIVE_INSTALLATION",
            "PASS",
        ));
        nodes.push(installation.clone());
        predecessor = installation;
    }
    for operator_id in input.promoted_operator_ids {
        let operator = provenance_node("IMPROVEMENT_OPERATOR", operator_id);
        edges.push(provenance_edge(
            &predecessor,
            &operator,
            "VERIFIED_EXECUTION_TO_OPERATOR_PROMOTION",
            "PROMOTED",
        ));
        nodes.push(operator);
    }
    nodes.sort_by(|left, right| left.node_id.cmp(&right.node_id));
    nodes.dedup_by(|left, right| left.node_id == right.node_id);
    edges.sort_by(|left, right| left.edge_id.cmp(&right.edge_id));
    edges.dedup_by(|left, right| left.edge_id == right.edge_id);
    let hash_body = (
        REPOSITORY_PIPELINE_PROVENANCE_SCHEMA,
        task_identity_sha256.as_str(),
        &nodes,
        &edges,
    );
    let graph_sha256 = canonical_sha(&hash_body)?;
    let graph = RepositoryPipelineProvenanceGraphIR {
        schema: REPOSITORY_PIPELINE_PROVENANCE_SCHEMA.to_string(),
        task_identity_sha256,
        nodes,
        edges,
        graph_sha256,
    };
    validate_repository_pipeline_provenance(&graph)?;
    Ok(graph)
}

pub fn validate_repository_pipeline_provenance(
    graph: &RepositoryPipelineProvenanceGraphIR,
) -> Result<(), String> {
    let hash_body = (
        REPOSITORY_PIPELINE_PROVENANCE_SCHEMA,
        graph.task_identity_sha256.as_str(),
        &graph.nodes,
        &graph.edges,
    );
    let node_ids = graph
        .nodes
        .iter()
        .map(|node| node.node_id.as_str())
        .collect::<BTreeSet<_>>();
    let edge_ids = graph
        .edges
        .iter()
        .map(|edge| edge.edge_id.as_str())
        .collect::<BTreeSet<_>>();
    if graph.schema != REPOSITORY_PIPELINE_PROVENANCE_SCHEMA
        || graph.task_identity_sha256.len() != 64
        || graph.nodes.is_empty()
        || graph.edges.is_empty()
        || node_ids.len() != graph.nodes.len()
        || edge_ids.len() != graph.edges.len()
        || graph.edges.iter().any(|edge| {
            !node_ids.contains(edge.source_node_id.as_str())
                || !node_ids.contains(edge.target_node_id.as_str())
        })
        || graph.graph_sha256 != canonical_sha(&hash_body)?
    {
        return Err("REPOSITORY_PIPELINE_PROVENANCE_INVALID".to_string());
    }
    Ok(())
}

pub fn repository_issue_intake_receipt_hash(
    receipt: &RepositoryIssueIntakeReceiptIR,
) -> Result<String, String> {
    canonical_sha(&(
        REPOSITORY_ISSUE_INTAKE_RECEIPT_SCHEMA,
        &receipt.issue,
        &receipt.path_bindings,
        &receipt.evidence_artifact_sha256s,
        receipt.work_event_id.as_str(),
        receipt.occurred_at_ms,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn issue_grounding_transports_public_claims_without_patch_authority() {
        let issue = ground_repository_issue(&RepositoryIssueIntakeRequestIR {
            schema: REPOSITORY_ISSUE_INTAKE_REQUEST_SCHEMA.to_string(),
            issue_id: "owner-identity".to_string(),
            problem_statement: "Actual behavior: Policy.resolve() returns the sibling owner.\nExpected behavior: Policy.resolve() should preserve Policy.owner.\nConstraint: must not change tests/test_policy.py"
                .to_string(),
            paths: vec![PathBuf::from("policy.py")],
            evidence_artifacts: Vec::new(),
            occurred_at_ms: 1,
        })
        .unwrap();
        validate_repository_issue_evidence(&issue).unwrap();
        assert!(!issue.observed_behavior.is_empty());
        assert!(!issue.expected_behavior.is_empty());
        assert!(issue
            .referenced_symbols
            .contains(&"Policy.resolve".to_string()));
        assert!(issue
            .referenced_paths
            .contains(&"tests/test_policy.py".to_string()));
        assert!(!issue.natural_language_is_patch_authority);
    }

    #[test]
    fn provenance_binds_issue_through_verified_operator_promotion() {
        let issue_sha256 = "a".repeat(64);
        let validation_id = "b".repeat(64);
        let source_bound_sha256 = "c".repeat(64);
        let candidate_sha256 = "d".repeat(64);
        let sandbox_sha256 = "e".repeat(64);
        let authoritative_sha256 = "f".repeat(64);
        let contract = derive_repository_repair_contract(
            std::slice::from_ref(&issue_sha256),
            &validation_id,
            &["Policy.resolve".to_string()],
        )
        .unwrap();
        let graph = build_repository_repair_provenance(&RepositoryRepairProvenanceInput {
            issue_evidence_sha256s: &[issue_sha256],
            validation_id: &validation_id,
            target_symbols: &["Policy.resolve".to_string()],
            repair_contract_sha256: Some(&contract.contract_sha256),
            source_bound_receipt_sha256: Some(&source_bound_sha256),
            candidate_sha256: Some(&candidate_sha256),
            sandbox_output_sha256: Some(&sandbox_sha256),
            authoritative_output_sha256: Some(&authoritative_sha256),
            promoted_operator_ids: &["operator-1".to_string()],
        })
        .unwrap();
        validate_repository_pipeline_provenance(&graph).unwrap();
        assert!(graph
            .edges
            .iter()
            .any(|edge| edge.transition == "ISSUE_TO_VALIDATION"));
        assert!(graph
            .edges
            .iter()
            .any(|edge| edge.transition == "VERIFIED_EXECUTION_TO_OPERATOR_PROMOTION"));
        assert!(graph
            .edges
            .iter()
            .any(|edge| edge.transition == "TARGET_FRONTIER_TO_REPAIR_CONTRACT"));
    }
}
