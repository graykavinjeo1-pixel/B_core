use serde::{Deserialize, Serialize};

use crate::task::Demonstration;

pub const CORE_ABI_VERSION: u32 = 1;
pub const SEMANTIC_STATE_VERSION: &str = "SEMANTIC-STATE-SEM8-1";
pub const CAPABILITY_CONTRACT_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SemanticType {
    Integer,
    IntegerSequence,
    Boolean,
    Unit,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", content = "value", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SemanticValue {
    Integer(i64),
    IntegerSequence(Vec<i64>),
    Boolean(bool),
    Unit,
}

impl SemanticValue {
    pub fn semantic_type(&self) -> SemanticType {
        match self {
            Self::Integer(_) => SemanticType::Integer,
            Self::IntegerSequence(_) => SemanticType::IntegerSequence,
            Self::Boolean(_) => SemanticType::Boolean,
            Self::Unit => SemanticType::Unit,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CapabilityEffect {
    Pure,
    LocalStateMutation,
    FilesystemRead,
    FilesystemWrite,
    NetworkRead,
    NetworkWrite,
    DeviceInput,
    DeviceOutput,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CapabilityPermission {
    None,
    LocalState,
    Filesystem,
    Network,
    Device,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilityResources {
    pub max_wall_time_ms: u64,
    pub max_memory_bytes: usize,
    pub deterministic: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilityContract {
    pub capability_id: String,
    pub core_abi_version: u32,
    pub contract_version: u32,
    pub input_type: SemanticType,
    pub output_type: SemanticType,
    pub preconditions: Vec<String>,
    pub postconditions: Vec<String>,
    pub effects: Vec<CapabilityEffect>,
    pub mutates_state: bool,
    pub resources: CapabilityResources,
    pub failure_modes: Vec<String>,
    pub permissions: Vec<CapabilityPermission>,
    pub semantic_relations: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilityRequest {
    pub capability_id: String,
    pub input: SemanticValue,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilityResult {
    pub capability_id: String,
    pub output: Option<SemanticValue>,
    pub failure: Option<String>,
    pub contract_validated: bool,
}

pub trait Capability {
    fn contract(&self) -> CapabilityContract;
    fn execute(&mut self, request: CapabilityRequest) -> CapabilityResult;
}

/// Language-independent request IR accepted by the extracted runtime.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GoalIR {
    pub request_id: String,
    pub core_abi_version: u32,
    pub semantic_state_version: String,
    pub target_concept_id: String,
    pub scalar_parameter: i64,
    pub demonstrations: Vec<Demonstration>,
    pub query_input: Vec<i64>,
    pub constraints: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResultIR {
    pub request_id: String,
    pub target_concept_id: String,
    pub output: Option<Vec<i64>>,
    pub failure: Option<String>,
    pub derivation_sha256: String,
    pub verified: bool,
    pub search_expansions: usize,
    pub peak_active_concepts: usize,
    pub full_catalog_scans: usize,
    pub routing_false_negatives: usize,
}
