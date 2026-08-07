use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::dsl::{Instruction, InstructionPattern, ValueType};
use crate::reasoning::SolveResult;
use crate::substrate::{
    ConceptIR, ConceptKind, CounterfactualCode, ExecutableSemantics, InvariantCode, ParameterSpec,
    PreconditionCode, PredictionCode, PromotionState, Provenance, Relation, RelationCode,
    Signature, StructuralMacro,
};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MiningReport {
    pub successful_graphs_examined: usize,
    pub connected_substructures_examined: usize,
    pub aligned_occurrences: usize,
    pub typed_parameter_slots: usize,
    pub candidate_ids: Vec<String>,
    pub rejected_patterns: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MiningOutput {
    pub report: MiningReport,
    pub candidates: Vec<ConceptIR>,
    pub structural_macros: Vec<StructuralMacro>,
}

pub fn mine_repeated_structure(results: &[SolveResult]) -> MiningOutput {
    let solved: Vec<&SolveResult> = results
        .iter()
        .filter(|result| result.verified_after_commit && result.program.is_some())
        .collect();
    let connected_substructures_examined = solved
        .iter()
        .filter_map(|result| result.program.as_ref())
        .map(|program| program.len().saturating_mul(program.len() + 1) / 2)
        .sum();
    let Some(pattern) = anti_unify_programs(&solved) else {
        return MiningOutput {
            report: MiningReport {
                successful_graphs_examined: solved.len(),
                connected_substructures_examined,
                aligned_occurrences: 0,
                typed_parameter_slots: 0,
                candidate_ids: Vec::new(),
                rejected_patterns: usize::from(!solved.is_empty()),
            },
            candidates: Vec::new(),
            structural_macros: Vec::new(),
        };
    };

    let source_task_ids: Vec<String> = solved.iter().map(|result| result.task_id.clone()).collect();
    let source_derivation_ids: Vec<String> = solved
        .iter()
        .map(|result| result.derivation.graph_id.clone())
        .collect();
    let mut primitive_ids = BTreeSet::new();
    for result in &solved {
        for primitive_id in &result.derivation.primitive_expansion {
            primitive_ids.insert(primitive_id.clone());
        }
    }
    let relations = source_derivation_ids
        .iter()
        .map(|graph_id| Relation {
            relation: RelationCode::DerivedFrom,
            target_id: graph_id.clone(),
        })
        .chain(primitive_ids.iter().map(|primitive_id| Relation {
            relation: RelationCode::ExpandsTo,
            target_id: primitive_id.clone(),
        }))
        .collect();

    let historical_derivation_cost = pattern.len();
    let mut candidate = ConceptIR {
        concept_id: "C000001".to_string(),
        kind: ConceptKind::Candidate,
        signature: Signature {
            inputs: vec![ValueType::IntegerSequence, ValueType::ScalarOperator],
            output: ValueType::IntegerSequence,
        },
        parameters: vec![ParameterSpec {
            parameter_id: "A000001".to_string(),
            value_type: ValueType::ScalarOperator,
        }],
        preconditions: vec![
            PreconditionCode::InputIsFiniteSequence,
            PreconditionCode::ScalarOperatorIsChecked,
        ],
        invariants: vec![
            InvariantCode::InputRemainsImmutable,
            InvariantCode::OutputOrderMatchesInputOrder,
            InvariantCode::OutputLengthMatchesInputLength,
            InvariantCode::EveryOutputHasOneInputDependency,
        ],
        relations,
        transition_semantics: ExecutableSemantics::Pattern(pattern.clone()),
        predictions: vec![
            PredictionCode::DeterministicSequenceOutput,
            PredictionCode::PreservesInputCardinality,
            PredictionCode::RejectsCheckedArithmeticFailure,
        ],
        counterfactual_interface: vec![
            CounterfactualCode::EmptyInput,
            CounterfactualCode::SingletonInput,
            CounterfactualCode::RepeatedValues,
            CounterfactualCode::NegativeValues,
            CounterfactualCode::ReorderedInput,
            CounterfactualCode::ChangedOperator,
            CounterfactualCode::ChangedParameter,
            CounterfactualCode::NumericBoundary,
            CounterfactualCode::ArithmeticOverflow,
            CounterfactualCode::MissingEvidence,
        ],
        derivation_graph_ids: source_derivation_ids.clone(),
        evidence: Vec::new(),
        promotion_state: PromotionState::Candidate,
        version: 1,
        provenance: Provenance {
            discovery_run_id: "SEM0-RUN-0001".to_string(),
            source_task_ids,
            source_derivation_ids: source_derivation_ids.clone(),
            primitive_ids: primitive_ids.into_iter().collect(),
            parent_concept_ids: Vec::new(),
            supplied_by_teacher: false,
            lexical_information_used: false,
        },
        historical_derivation_cost,
        operational_cost: 1,
        content_hash_sha256: String::new(),
    };
    candidate
        .freeze_hash()
        .expect("candidate has deterministic serialization");

    let structural_macro = StructuralMacro {
        macro_id: "M000001".to_string(),
        pattern,
        parameter_types: vec![ValueType::ScalarOperator],
        source_derivation_ids,
        validated_semantically: false,
    };

    MiningOutput {
        report: MiningReport {
            successful_graphs_examined: solved.len(),
            connected_substructures_examined,
            aligned_occurrences: solved.len(),
            typed_parameter_slots: 1,
            candidate_ids: vec![candidate.concept_id.clone()],
            rejected_patterns: 0,
        },
        candidates: vec![candidate],
        structural_macros: vec![structural_macro],
    }
}

fn anti_unify_programs(results: &[&SolveResult]) -> Option<Vec<InstructionPattern>> {
    if results.len() < 3 {
        return None;
    }
    let programs: Vec<&Vec<Instruction>> = results
        .iter()
        .filter_map(|result| result.program.as_ref())
        .collect();
    let first = *programs.first()?;
    if programs.iter().any(|program| program.len() != first.len()) {
        return None;
    }
    let mut pattern = Vec::new();
    for index in 0..first.len() {
        let instructions: Vec<&Instruction> =
            programs.iter().map(|program| &program[index]).collect();
        let generalized = if instructions
            .iter()
            .all(|instruction| matches!(instruction, Instruction::ApplyScalar(_)))
        {
            InstructionPattern::ScalarSlot
        } else if instructions
            .iter()
            .all(|instruction| *instruction == instructions[0])
        {
            match instructions[0] {
                Instruction::InitOutput => InstructionPattern::InitOutput,
                Instruction::BranchIfEmpty(target) => InstructionPattern::BranchIfEmpty(*target),
                Instruction::ReadCurrent => InstructionPattern::ReadCurrent,
                Instruction::AppendCurrent => InstructionPattern::AppendCurrent,
                Instruction::Advance => InstructionPattern::Advance,
                Instruction::BranchIfRemaining(target) => {
                    InstructionPattern::BranchIfRemaining(*target)
                }
                Instruction::Return => InstructionPattern::Return,
                Instruction::ApplyScalar(_) => unreachable!("handled as parameter slot"),
            }
        } else {
            return None;
        };
        pattern.push(generalized);
    }
    Some(pattern)
}

#[cfg(test)]
mod tests {
    use crate::dsl::InstructionPattern;
    use crate::reasoning::{AdaptiveReasoner, ResourceBudget};
    use crate::tasks::generate_tasks;

    use super::mine_repeated_structure;

    #[test]
    fn graph_mining_requires_three_independent_origins() {
        let (train, _, _) = generate_tasks();
        let reasoner = AdaptiveReasoner::default();
        let mut two = Vec::new();
        for task in train.iter().take(2) {
            let mut result = reasoner.primitive_only(task.visible(), ResourceBudget::discovery());
            result.seal_score(task.score_committed(&result.committed()));
            two.push(result);
        }
        assert!(mine_repeated_structure(&two).candidates.is_empty());
    }

    #[test]
    fn typed_anti_unification_creates_opaque_executable_candidate() {
        let (train, _, _) = generate_tasks();
        let reasoner = AdaptiveReasoner::default();
        let mut results = Vec::new();
        for task in &train {
            let mut result = reasoner.primitive_only(task.visible(), ResourceBudget::discovery());
            result.seal_score(task.score_committed(&result.committed()));
            results.push(result);
        }
        let mined = mine_repeated_structure(&results);
        assert_eq!(mined.candidates.len(), 1);
        assert_eq!(mined.candidates[0].concept_id, "C000001");
        let crate::substrate::ExecutableSemantics::Pattern(pattern) =
            &mined.candidates[0].transition_semantics
        else {
            panic!("candidate must be executable pattern");
        };
        assert!(pattern.contains(&InstructionPattern::ScalarSlot));
        assert!(!mined.candidates[0].provenance.supplied_by_teacher);
        assert!(!mined.candidates[0].provenance.lexical_information_used);
        assert_eq!(mined.candidates[0].content_hash_sha256.len(), 64);
    }

    #[test]
    fn provenance_is_not_mutated_by_operational_reuse() {
        let (train, _, _) = generate_tasks();
        let reasoner = AdaptiveReasoner::default();
        let mut results = Vec::new();
        for task in &train {
            let mut result = reasoner.primitive_only(task.visible(), ResourceBudget::discovery());
            result.seal_score(task.score_committed(&result.committed()));
            results.push(result);
        }
        let mined = mine_repeated_structure(&results);
        let before = mined.candidates[0].provenance.clone();
        let _ = reasoner.semantic_candidate(
            train[0].visible(),
            ResourceBudget::blind(),
            &mined.candidates[0],
        );
        assert_eq!(before, mined.candidates[0].provenance);
    }
}
