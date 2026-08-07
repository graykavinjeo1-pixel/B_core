use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::dsl::InstructionPattern;

const EMBEDDED_SEMANTIC_STATE: &str = include_str!("../state/semantic_state.json");
const EMBEDDED_SPARSE_INDEX: &str = include_str!("../state/sparse_index.json");

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConceptReceipt {
    pub concept_id: String,
    pub generation: usize,
    pub semantic_payload_sha256: String,
    pub runtime_role: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimePattern {
    pub concept_id: String,
    pub instructions: Vec<InstructionPattern>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SemanticState {
    pub semantic_state_version: String,
    pub concepts: Vec<ConceptReceipt>,
    pub runtime_patterns: Vec<RuntimePattern>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct SparseIndexFile {
    index_version: String,
    routes: BTreeMap<String, usize>,
}

#[derive(Debug, Clone)]
pub struct SparseIndex {
    routes: BTreeMap<String, usize>,
}

impl SemanticState {
    pub fn load_embedded() -> Result<Self, serde_json::Error> {
        serde_json::from_str(EMBEDDED_SEMANTIC_STATE)
    }

    pub fn pattern(&self, concept_id: &str) -> Option<&RuntimePattern> {
        self.runtime_patterns
            .iter()
            .find(|pattern| pattern.concept_id == concept_id)
    }

    pub fn validate_receipts(&self) -> bool {
        self.concepts.iter().all(|concept| {
            concept.concept_id.starts_with('C')
                && concept.semantic_payload_sha256.len() == 64
                && concept
                    .semantic_payload_sha256
                    .bytes()
                    .all(|byte| byte.is_ascii_hexdigit())
        })
    }
}

impl SparseIndex {
    pub fn load_embedded(state: &SemanticState) -> Result<Self, serde_json::Error> {
        let file: SparseIndexFile = serde_json::from_str(EMBEDDED_SPARSE_INDEX)?;
        let routes = file
            .routes
            .into_iter()
            .filter(|(concept_id, slot)| {
                state
                    .concepts
                    .get(*slot)
                    .is_some_and(|concept| concept.concept_id == *concept_id)
            })
            .collect();
        Ok(Self { routes })
    }

    pub fn route(&self, concept_id: &str) -> Option<usize> {
        self.routes.get(concept_id).copied()
    }

    pub fn len(&self) -> usize {
        self.routes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.routes.is_empty()
    }
}
