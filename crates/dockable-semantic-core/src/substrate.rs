use serde::{Deserialize, Serialize};

use crate::dsl::{InstructionPattern, ScalarOperator, ValueType};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ConceptKind {
    Primitive,
    Candidate,
    Promoted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PromotionState {
    Primitive,
    Candidate,
    Rejected,
    Promoted,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Signature {
    pub inputs: Vec<ValueType>,
    pub output: ValueType,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParameterSpec {
    pub parameter_id: String,
    pub value_type: ValueType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PreconditionCode {
    InputIsFiniteSequence,
    ScalarOperatorIsChecked,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum InvariantCode {
    InputRemainsImmutable,
    OutputOrderMatchesInputOrder,
    OutputLengthMatchesInputLength,
    EveryOutputHasOneInputDependency,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RelationCode {
    DerivedFrom,
    DependsOn,
    ExpandsTo,
    VerifiedBy,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Relation {
    pub relation: RelationCode,
    pub target_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "form", content = "body", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ExecutableSemantics {
    Primitive(ScalarOperator),
    Pattern(Vec<InstructionPattern>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PredictionCode {
    DeterministicSequenceOutput,
    PreservesInputCardinality,
    RejectsCheckedArithmeticFailure,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CounterfactualCode {
    EmptyInput,
    SingletonInput,
    RepeatedValues,
    NegativeValues,
    ReorderedInput,
    ChangedOperator,
    ChangedParameter,
    NumericBoundary,
    ArithmeticOverflow,
    MissingEvidence,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EvidenceRecord {
    pub gate_id: String,
    pub passed: bool,
    pub observations: usize,
    pub metric: f64,
    pub artifact_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Provenance {
    pub discovery_run_id: String,
    pub source_task_ids: Vec<String>,
    pub source_derivation_ids: Vec<String>,
    pub primitive_ids: Vec<String>,
    pub parent_concept_ids: Vec<String>,
    pub supplied_by_teacher: bool,
    pub lexical_information_used: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConceptIR {
    pub concept_id: String,
    pub kind: ConceptKind,
    pub signature: Signature,
    pub parameters: Vec<ParameterSpec>,
    pub preconditions: Vec<PreconditionCode>,
    pub invariants: Vec<InvariantCode>,
    pub relations: Vec<Relation>,
    pub transition_semantics: ExecutableSemantics,
    pub predictions: Vec<PredictionCode>,
    pub counterfactual_interface: Vec<CounterfactualCode>,
    pub derivation_graph_ids: Vec<String>,
    pub evidence: Vec<EvidenceRecord>,
    pub promotion_state: PromotionState,
    pub version: u32,
    pub provenance: Provenance,
    pub historical_derivation_cost: usize,
    pub operational_cost: usize,
    pub content_hash_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Episode {
    pub episode_id: String,
    pub task_id: String,
    pub derivation_graph_id: String,
    pub solved: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CacheEntry {
    pub cache_id: String,
    pub exact_signature_sha256: String,
    pub output: Vec<i64>,
    pub source_task_id: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StructuralMacro {
    pub macro_id: String,
    pub pattern: Vec<InstructionPattern>,
    pub parameter_types: Vec<ValueType>,
    pub source_derivation_ids: Vec<String>,
    pub validated_semantically: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LexicalAlias {
    pub concept_id: String,
    pub language_code: String,
    pub alias: String,
    pub attached_after_canonical_evaluation: bool,
}

impl ConceptIR {
    pub fn freeze_hash(&mut self) -> Result<(), serde_json::Error> {
        use sha2::{Digest, Sha256};

        self.content_hash_sha256.clear();
        let bytes = serde_json::to_vec(self)?;
        let digest = Sha256::digest(bytes);
        self.content_hash_sha256 = format!("{digest:x}");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{CacheEntry, ConceptKind, LexicalAlias, PromotionState};

    #[test]
    fn record_kinds_remain_separate() {
        let cache = CacheEntry {
            cache_id: "K000001".to_string(),
            exact_signature_sha256: "abc".to_string(),
            output: vec![1],
            source_task_id: "T000001".to_string(),
        };
        let alias = LexicalAlias {
            concept_id: "C000001".to_string(),
            language_code: "und".to_string(),
            alias: "forensic-only".to_string(),
            attached_after_canonical_evaluation: true,
        };
        assert_ne!(cache.cache_id, alias.concept_id);
        assert_ne!(ConceptKind::Candidate, ConceptKind::Promoted);
        assert_ne!(PromotionState::Candidate, PromotionState::Promoted);
    }
}
