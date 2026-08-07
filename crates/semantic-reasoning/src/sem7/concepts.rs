use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::model::{MeaningRequestIR, Quantifier, SemanticConcept, SemanticOperation};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "value_kind",
    content = "value",
    rename_all = "SCREAMING_SNAKE_CASE"
)]
pub enum SemanticValue {
    Sequence(Vec<i64>),
    Int(i64),
    Bool(bool),
    ConceptId(String),
}

#[derive(Debug, Clone)]
pub struct ConceptRegistry {
    concepts: BTreeMap<String, SemanticConcept>,
}

impl ConceptRegistry {
    pub fn canonical() -> Self {
        let concepts = [
            concept(
                "C000006",
                3,
                &["C000002"],
                "kernel-verified exact recurrence relation",
                "(state, delta) -> successor state",
                "1a9a17c76076d42ea6d982faa91131de77d8ff3a4fa54e4cda30ad03fd098039",
            ),
            concept(
                "C000007",
                3,
                &["C000002"],
                "kernel-verified exact recurrence relation",
                "(state, delta) -> successor state",
                "02208e61413a568c91cddba13bcf84d704e46f24e01ffdee967ec101a564e40e",
            ),
            concept(
                "C000008",
                3,
                &["C000002"],
                "typed finite source with conditional per-position action",
                "sequence, predicate?, scalar action? -> sequence",
                "SEM5-C000008-SEALED-PAYLOAD",
            ),
            concept(
                "C000009",
                3,
                &["C000004"],
                "typed state and element to guarded next state",
                "state, element, guard -> state",
                "SEM5-C000009-SEALED-PAYLOAD",
            ),
            concept(
                "C000010",
                4,
                &["C000008", "C000009"],
                "dependency-preserving composition of compatible stages",
                "stage output, compatible next input -> composed result",
                "SEM5-C000010-SEALED-PAYLOAD",
            ),
            concept(
                "C000011",
                5,
                &["C000010"],
                "scoped versioned typed pure relation",
                "alias, scope, version -> validated relation",
                "SEM6-C000011-SEALED-PAYLOAD",
            ),
            concept(
                "C000012",
                5,
                &["C000007", "C000010"],
                "bounded integer quotient classification",
                "bounded integer, positive divisor -> discrete class",
                "SEM6-C000012-SEALED-PAYLOAD",
            ),
        ]
        .into_iter()
        .map(|concept| (concept.concept_id.clone(), concept))
        .collect();
        Self { concepts }
    }

    pub fn get(&self, concept_id: &str) -> Option<&SemanticConcept> {
        self.concepts.get(concept_id)
    }

    pub fn len(&self) -> usize {
        self.concepts.len()
    }

    pub fn is_empty(&self) -> bool {
        self.concepts.is_empty()
    }

    pub fn concepts(&self) -> impl Iterator<Item = &SemanticConcept> {
        self.concepts.values()
    }

    pub fn without_concept(&self, concept_id: &str) -> Self {
        let mut reduced = self.clone();
        reduced.concepts.remove(concept_id);
        reduced
    }

    pub fn semantic_hash(&self, concept_id: &str) -> Result<String, String> {
        let concept = self
            .get(concept_id)
            .ok_or_else(|| format!("UNKNOWN_CONCEPT:{concept_id}"))?;
        Ok(hash_serializable(concept))
    }

    pub fn semantically_equivalent_signature(&self, signature: &str) -> Option<&SemanticConcept> {
        let normalized = signature.to_ascii_lowercase();
        let concept_id = if normalized.contains("multiply")
            || normalized.contains("곱")
            || normalized.contains("add every")
            || normalized.contains("add to every")
            || normalized.contains("모든 정수에") && normalized.contains("더")
            || normalized.contains("filter")
            || normalized.contains("greater values")
            || normalized.contains("큰 값을")
        {
            "C000008"
        } else if normalized.contains("accumulat")
            || normalized.contains("sum")
            || normalized.contains("누적")
        {
            "C000009"
        } else if normalized.contains("recurrence") || normalized.contains("점화") {
            "C000006"
        } else if normalized.contains("status class")
            || normalized.contains("응답 상태")
            || normalized.contains("http")
        {
            "C000012"
        } else if normalized.contains("versioned")
            || normalized.contains("scoped contract")
            || normalized.contains("버전 범위")
        {
            "C000011"
        } else {
            return None;
        };
        self.get(concept_id)
    }

    pub fn execute(
        &self,
        request: &MeaningRequestIR,
        input: &[i64],
    ) -> Result<SemanticValue, String> {
        if self.get(&request.target_concept_id).is_none() {
            return Err(format!(
                "SEMANTIC_CONCEPT_ABSENT:{}",
                request.target_concept_id
            ));
        }
        let parameter = request.parameter.unwrap_or(0);
        let result = match request.operation {
            SemanticOperation::Identify => {
                SemanticValue::ConceptId(request.target_concept_id.clone())
            }
            SemanticOperation::AddEach => {
                SemanticValue::Sequence(input.iter().map(|value| value + parameter).collect())
            }
            SemanticOperation::MultiplyEach => {
                SemanticValue::Sequence(input.iter().map(|value| value * parameter).collect())
            }
            SemanticOperation::FilterGreater => SemanticValue::Sequence(
                input
                    .iter()
                    .copied()
                    .filter(|value| *value > parameter)
                    .collect(),
            ),
            SemanticOperation::FilterAtLeast => SemanticValue::Sequence(
                input
                    .iter()
                    .copied()
                    .filter(|value| *value >= parameter)
                    .collect(),
            ),
            SemanticOperation::FilterNotGreater => SemanticValue::Sequence(
                input
                    .iter()
                    .copied()
                    .filter(|value| *value <= parameter)
                    .collect(),
            ),
            SemanticOperation::Sum => SemanticValue::Int(input.iter().sum()),
            SemanticOperation::CountGreater => {
                let matches = input.iter().filter(|value| **value > parameter).count();
                match request.quantifier {
                    None => SemanticValue::Int(matches as i64),
                    Some(Quantifier::All) => SemanticValue::Bool(matches == input.len()),
                    Some(Quantifier::Any) => SemanticValue::Bool(matches > 0),
                    Some(Quantifier::None) => SemanticValue::Bool(matches == 0),
                    Some(Quantifier::ExactlyOne) => SemanticValue::Bool(matches == 1),
                    Some(Quantifier::AtLeast) => {
                        SemanticValue::Bool(matches >= request.quantifier_threshold.unwrap_or(1))
                    }
                }
            }
            SemanticOperation::RecurrenceStep => {
                let state = input.first().copied().ok_or("MISSING_RECURRENCE_STATE")?;
                SemanticValue::Int(state + parameter)
            }
            SemanticOperation::StatusClass => {
                let status = input.first().copied().ok_or("MISSING_STATUS_CODE")?;
                if !(100..=599).contains(&status) {
                    return Err("STATUS_OUT_OF_SCOPE".to_string());
                }
                SemanticValue::Int(status / 100)
            }
            SemanticOperation::ScopedLookup => {
                SemanticValue::Int(input.first().copied().ok_or("MISSING_SCOPED_LOOKUP_KEY")?)
            }
        };
        Ok(result)
    }
}

fn concept(
    concept_id: &str,
    generation: usize,
    parents: &[&str],
    semantic_kind: &str,
    signature: &str,
    upstream_hash: &str,
) -> SemanticConcept {
    SemanticConcept {
        concept_id: concept_id.to_string(),
        generation,
        parent_ids: parents.iter().map(|parent| (*parent).to_string()).collect(),
        semantic_kind: semantic_kind.to_string(),
        executable_signature: signature.to_string(),
        invariants: vec![
            "opaque concept identity is independent of lexical aliases".to_string(),
            "semantic payload is immutable during language operations".to_string(),
        ],
        upstream_payload_sha256: upstream_hash.to_string(),
        concept_about_language: false,
        required_lexical_tokens: Vec::new(),
    }
}

pub fn hash_serializable(value: &impl Serialize) -> String {
    format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(value).expect("serialize"))
    )
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;

    #[test]
    fn unnamed_semantic_execution_uses_no_language() {
        let registry = ConceptRegistry::canonical();
        let request = MeaningRequestIR {
            target_concept_id: "C000008".to_string(),
            target_state: "mapped sequence".to_string(),
            inputs: vec!["values".to_string()],
            output: "sequence<int>".to_string(),
            constraints: Vec::new(),
            requested_relations: vec!["C000008".to_string()],
            operation: SemanticOperation::AddEach,
            parameter: Some(3),
            modifiers: Vec::new(),
            quantifier: None,
            quantifier_threshold: None,
            ordering: vec!["map".to_string()],
            scope: "sequence-transform".to_string(),
            reference_bindings: BTreeMap::new(),
            ambiguity_set: Vec::new(),
            lexical_mapping_confidence: 0.0,
            semantic_concept_confidence: 1.0,
            parse_confidence: 0.0,
            reference_resolution_confidence: 1.0,
            raw_text_in_reasoning_hot_path: false,
        };
        assert_eq!(
            registry.execute(&request, &[1, 2]).expect("execute"),
            SemanticValue::Sequence(vec![4, 5])
        );
        assert!(registry
            .concepts()
            .all(|concept| concept.required_lexical_tokens.is_empty()));
    }
}
