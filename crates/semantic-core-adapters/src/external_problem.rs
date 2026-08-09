use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub const EXTERNAL_GOAL_IR_VERSION: &str = "EXTERNAL-GOAL-IR-1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ExternalProblemClass {
    RepositoryRepair,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepositoryArtifact {
    pub relative_path: String,
    pub content_sha256: String,
    pub byte_length: u64,
    pub executable: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExecutableObservation {
    pub observation_id: String,
    pub command_sha256: String,
    pub exit_code: i32,
    pub stdout_sha256: String,
    pub stderr_sha256: String,
    pub passed_check_ids: Vec<String>,
    pub failed_check_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExternalProblemInput {
    pub request_id: String,
    pub problem_class: ExternalProblemClass,
    pub issue_text: String,
    pub repository_revision: String,
    pub repository_artifacts: Vec<RepositoryArtifact>,
    pub executable_observations: Vec<ExecutableObservation>,
    pub constraints: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AdapterCompatibility {
    Supported,
    Unsupported,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SemanticEvidenceRef {
    pub evidence_id: String,
    pub evidence_kind: String,
    pub content_sha256: String,
    pub byte_length: u64,
    pub executable_result: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CoreGoalBridge {
    pub compatibility: AdapterCompatibility,
    pub target_ir: String,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExternalGoalIr {
    pub schema_version: String,
    pub request_id: String,
    pub problem_class: ExternalProblemClass,
    pub repository_revision: String,
    pub opaque_language_goal_sha256: String,
    pub semantic_evidence: Vec<SemanticEvidenceRef>,
    pub constraints: Vec<String>,
    pub core_goal_bridge: CoreGoalBridge,
    pub source_language_is_reasoning_authority: bool,
    pub hot_reasoning_path_natural_language_authority: bool,
    pub task_specific_adapter_branches: u32,
    pub content_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ExternalAdapterError {
    EmptyRequestId,
    EmptyIssue,
    EmptyRepositoryRevision,
    InvalidDigest,
    InvalidRelativePath,
    DuplicateArtifactPath,
    DuplicateObservationId,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct ExternalProblemAdapter;

impl ExternalProblemAdapter {
    pub fn compile(
        &self,
        mut input: ExternalProblemInput,
    ) -> Result<ExternalGoalIr, ExternalAdapterError> {
        if input.request_id.trim().is_empty() {
            return Err(ExternalAdapterError::EmptyRequestId);
        }
        if input.issue_text.trim().is_empty() {
            return Err(ExternalAdapterError::EmptyIssue);
        }
        if input.repository_revision.trim().is_empty() {
            return Err(ExternalAdapterError::EmptyRepositoryRevision);
        }

        input.repository_artifacts.sort_by(|left, right| {
            left.relative_path
                .cmp(&right.relative_path)
                .then(left.content_sha256.cmp(&right.content_sha256))
        });
        input
            .executable_observations
            .sort_by(|left, right| left.observation_id.cmp(&right.observation_id));
        input.constraints.sort();
        input.constraints.dedup();

        let mut paths = BTreeSet::new();
        for artifact in &input.repository_artifacts {
            if !valid_relative_path(&artifact.relative_path) {
                return Err(ExternalAdapterError::InvalidRelativePath);
            }
            if !valid_digest(&artifact.content_sha256) {
                return Err(ExternalAdapterError::InvalidDigest);
            }
            if !paths.insert(artifact.relative_path.as_str()) {
                return Err(ExternalAdapterError::DuplicateArtifactPath);
            }
        }

        let mut observation_ids = BTreeSet::new();
        for observation in &mut input.executable_observations {
            if !observation_ids.insert(observation.observation_id.as_str()) {
                return Err(ExternalAdapterError::DuplicateObservationId);
            }
            if !valid_digest(&observation.command_sha256)
                || !valid_digest(&observation.stdout_sha256)
                || !valid_digest(&observation.stderr_sha256)
            {
                return Err(ExternalAdapterError::InvalidDigest);
            }
            observation.passed_check_ids.sort();
            observation.passed_check_ids.dedup();
            observation.failed_check_ids.sort();
            observation.failed_check_ids.dedup();
        }

        let mut semantic_evidence = Vec::new();
        semantic_evidence.push(SemanticEvidenceRef {
            evidence_id: "EVIDENCE.LANGUAGE_GOAL".to_string(),
            evidence_kind: "OPAQUE_LANGUAGE_GOAL".to_string(),
            content_sha256: digest(input.issue_text.as_bytes()),
            byte_length: input.issue_text.len() as u64,
            executable_result: None,
        });
        semantic_evidence.extend(input.repository_artifacts.iter().enumerate().map(
            |(index, artifact)| SemanticEvidenceRef {
                evidence_id: format!("EVIDENCE.REPOSITORY.{index:06}"),
                evidence_kind: "REPOSITORY_ARTIFACT".to_string(),
                content_sha256: artifact.content_sha256.clone(),
                byte_length: artifact.byte_length,
                executable_result: None,
            },
        ));
        semantic_evidence.extend(input.executable_observations.iter().enumerate().map(
            |(index, observation)| SemanticEvidenceRef {
                evidence_id: format!("EVIDENCE.EXECUTION.{index:06}"),
                evidence_kind: "EXECUTABLE_OBSERVATION".to_string(),
                content_sha256: digest(
                    &serde_json::to_vec(observation).expect("serialize executable observation"),
                ),
                byte_length: 0,
                executable_result: Some(observation.exit_code == 0),
            },
        ));

        let mut output = ExternalGoalIr {
            schema_version: EXTERNAL_GOAL_IR_VERSION.to_string(),
            request_id: input.request_id,
            problem_class: input.problem_class,
            repository_revision: input.repository_revision,
            opaque_language_goal_sha256: digest(input.issue_text.as_bytes()),
            semantic_evidence,
            constraints: input.constraints,
            core_goal_bridge: CoreGoalBridge {
                compatibility: AdapterCompatibility::Unsupported,
                target_ir: "DOCKABLE-SEMANTIC-CORE-GOAL-IR-1".to_string(),
                reason: Some(
                    "CURRENT_GOAL_IR_SUPPORTS_CHECKED_INTEGER_SEQUENCE_TRANSFORMS_ONLY".to_string(),
                ),
            },
            source_language_is_reasoning_authority: false,
            hot_reasoning_path_natural_language_authority: false,
            task_specific_adapter_branches: 0,
            content_sha256: String::new(),
        };
        output.content_sha256 = digest(
            &serde_json::to_vec(&output).expect("serialize external goal IR for freeze hash"),
        );
        Ok(output)
    }
}

fn valid_digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn valid_relative_path(value: &str) -> bool {
    !value.is_empty()
        && !value.starts_with('/')
        && !value.starts_with('\\')
        && !value.contains(':')
        && !value
            .split(['/', '\\'])
            .any(|component| component.is_empty() || component == "..")
}

fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

#[cfg(test)]
mod tests {
    use super::{
        AdapterCompatibility, ExecutableObservation, ExternalAdapterError, ExternalProblemAdapter,
        ExternalProblemClass, ExternalProblemInput, RepositoryArtifact,
    };

    fn hash(label: &str) -> String {
        use sha2::{Digest, Sha256};
        format!("{:x}", Sha256::digest(label.as_bytes()))
    }

    fn fixture() -> ExternalProblemInput {
        ExternalProblemInput {
            request_id: "CANARY-STRUCTURAL-1".to_string(),
            problem_class: ExternalProblemClass::RepositoryRepair,
            issue_text: "A public behavior conflicts with an executable check.".to_string(),
            repository_revision: hash("revision"),
            repository_artifacts: vec![
                RepositoryArtifact {
                    relative_path: "src/module.ext".to_string(),
                    content_sha256: hash("source"),
                    byte_length: 120,
                    executable: false,
                },
                RepositoryArtifact {
                    relative_path: "tests/check.ext".to_string(),
                    content_sha256: hash("test"),
                    byte_length: 80,
                    executable: false,
                },
            ],
            executable_observations: vec![ExecutableObservation {
                observation_id: "OBS-1".to_string(),
                command_sha256: hash("command"),
                exit_code: 1,
                stdout_sha256: hash("stdout"),
                stderr_sha256: hash("stderr"),
                passed_check_ids: vec!["CHECK-B".to_string()],
                failed_check_ids: vec!["CHECK-A".to_string()],
            }],
            constraints: vec![
                "NETWORK_DISABLED".to_string(),
                "PATCH_MUST_APPLY".to_string(),
            ],
        }
    }

    #[test]
    fn structural_order_does_not_change_frozen_ir() {
        let adapter = ExternalProblemAdapter;
        let left = adapter.compile(fixture()).expect("compile left");
        let mut reordered = fixture();
        reordered.repository_artifacts.reverse();
        reordered.constraints.reverse();
        let right = adapter.compile(reordered).expect("compile right");
        assert_eq!(left, right);
    }

    #[test]
    fn language_is_opaque_evidence_not_reasoning_authority() {
        let output = ExternalProblemAdapter
            .compile(fixture())
            .expect("compile canary");
        assert!(!output.source_language_is_reasoning_authority);
        assert!(!output.hot_reasoning_path_natural_language_authority);
        assert_eq!(output.task_specific_adapter_branches, 0);
    }

    #[test]
    fn unsupported_core_domain_is_reported_without_fabricating_a_goal() {
        let output = ExternalProblemAdapter
            .compile(fixture())
            .expect("compile canary");
        assert_eq!(
            output.core_goal_bridge.compatibility,
            AdapterCompatibility::Unsupported
        );
        assert_eq!(
            output.core_goal_bridge.reason.as_deref(),
            Some("CURRENT_GOAL_IR_SUPPORTS_CHECKED_INTEGER_SEQUENCE_TRANSFORMS_ONLY")
        );
    }

    #[test]
    fn parent_traversal_is_rejected() {
        let mut input = fixture();
        input.repository_artifacts[0].relative_path = "../gold.patch".to_string();
        assert_eq!(
            ExternalProblemAdapter.compile(input),
            Err(ExternalAdapterError::InvalidRelativePath)
        );
    }
}
