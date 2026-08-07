use super::{
    algebra::{
        add, divide, equality_holds, equivalent, expand_definitions, multiply, substitute, subtract,
    },
    model::{
        Equality, Expr, FormalCondition, OperatorDefinition, ProofCertificate, ProofStep, RuleCode,
        TransformationRuleRecord,
    },
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KernelVerification {
    pub valid: bool,
    pub transformation_steps_checked: usize,
    pub induction_verified: bool,
}

#[derive(Debug, Clone)]
pub struct ProofKernel {
    rules: Vec<TransformationRuleRecord>,
}

impl ProofKernel {
    pub fn new() -> Self {
        Self {
            rules: transformation_rule_catalog(),
        }
    }

    pub fn rules(&self) -> &[TransformationRuleRecord] {
        &self.rules
    }

    pub fn verify(
        &self,
        certificate: &ProofCertificate,
        definitions: &[OperatorDefinition],
    ) -> KernelVerification {
        if certificate.steps.is_empty() {
            return KernelVerification {
                valid: false,
                transformation_steps_checked: 0,
                induction_verified: false,
            };
        }
        let mut prior = certificate.initial_statement.clone();
        let mut checked = 0;
        for step in &certificate.steps {
            if step.source_state != prior
                || !self
                    .rules
                    .iter()
                    .any(|entry| entry.rule == step.rule_applied)
                || !self.verify_step(step, definitions)
            {
                return KernelVerification {
                    valid: false,
                    transformation_steps_checked: checked,
                    induction_verified: false,
                };
            }
            prior = step.result_state.clone();
            checked += 1;
        }
        if prior != certificate.conclusion {
            return KernelVerification {
                valid: false,
                transformation_steps_checked: checked,
                induction_verified: false,
            };
        }
        let induction_verified = certificate
            .induction
            .as_ref()
            .is_some_and(|obligation| self.verify_induction(obligation));
        let requires_induction = certificate.induction.is_some();
        KernelVerification {
            valid: (!requires_induction || induction_verified)
                && certificate
                    .formal_proof_evidence_ids
                    .iter()
                    .all(|identifier| !identifier.is_empty()),
            transformation_steps_checked: checked,
            induction_verified,
        }
    }

    fn verify_step(&self, step: &ProofStep, definitions: &[OperatorDefinition]) -> bool {
        match step.rule_applied {
            RuleCode::EqualityReflexivity => equality_holds(&step.result_state).unwrap_or(false),
            RuleCode::EqualitySymmetry => {
                step.result_state.left == step.source_state.right
                    && step.result_state.right == step.source_state.left
            }
            RuleCode::SubtractBothSides => step.witness.as_ref().is_some_and(|witness| {
                equivalent(
                    &step.result_state.left,
                    &subtract(step.source_state.left.clone(), witness.clone()),
                )
                .unwrap_or(false)
                    && equivalent(
                        &step.result_state.right,
                        &subtract(step.source_state.right.clone(), witness.clone()),
                    )
                    .unwrap_or(false)
            }),
            RuleCode::AddBothSides => step.witness.as_ref().is_some_and(|witness| {
                equivalent(
                    &step.result_state.left,
                    &add(step.source_state.left.clone(), witness.clone()),
                )
                .unwrap_or(false)
                    && equivalent(
                        &step.result_state.right,
                        &add(step.source_state.right.clone(), witness.clone()),
                    )
                    .unwrap_or(false)
            }),
            RuleCode::MultiplyBothSides => step.witness.as_ref().is_some_and(|witness| {
                equivalent(
                    &step.result_state.left,
                    &multiply(step.source_state.left.clone(), witness.clone()),
                )
                .unwrap_or(false)
                    && equivalent(
                        &step.result_state.right,
                        &multiply(step.source_state.right.clone(), witness.clone()),
                    )
                    .unwrap_or(false)
            }),
            RuleCode::DivideBothSidesNonZero => step.witness.as_ref().is_some_and(|divisor| {
                condition_proves_nonzero(&step.preconditions_checked, divisor)
                    && divisor_is_provably_nonzero(divisor)
                    && equivalent(
                        &step.result_state.left,
                        &divide(step.source_state.left.clone(), divisor.clone()),
                    )
                    .unwrap_or(false)
                    && equivalent(
                        &step.result_state.right,
                        &divide(step.source_state.right.clone(), divisor.clone()),
                    )
                    .unwrap_or(false)
            }),
            RuleCode::DefinitionExpansion => {
                let left = expand_definitions(&step.source_state.left, definitions);
                let right = expand_definitions(&step.source_state.right, definitions);
                left.is_ok_and(|left| equivalent(&left, &step.result_state.left).unwrap_or(false))
                    && right.is_ok_and(|right| {
                        equivalent(&right, &step.result_state.right).unwrap_or(false)
                    })
            }
            RuleCode::InductionBase | RuleCode::InductionStep => {
                equality_holds(&step.result_state).unwrap_or(false)
            }
            RuleCode::CaseSplit => !step.preconditions_checked.is_empty(),
            RuleCode::EqualityTransitivity
            | RuleCode::SubstituteEquals
            | RuleCode::AdditionAssociativity
            | RuleCode::AdditionCommutativity
            | RuleCode::MultiplicationAssociativity
            | RuleCode::MultiplicationCommutativity
            | RuleCode::Distributivity
            | RuleCode::IdentityElements
            | RuleCode::AdditiveInverse
            | RuleCode::RationalPolynomialNormalization => {
                equivalent(&step.source_state.left, &step.result_state.left).unwrap_or(false)
                    && equivalent(&step.source_state.right, &step.result_state.right)
                        .unwrap_or(false)
            }
        }
    }

    fn verify_induction(&self, obligation: &super::model::InductionObligation) -> bool {
        if obligation.base_index < 0 {
            return false;
        }
        let base_substitution = std::collections::BTreeMap::from([(
            obligation.index_variable.clone(),
            Expr::Rational(super::model::Rational::integer(obligation.base_index)),
        )]);
        let expected_base = Equality {
            left: substitute(&obligation.candidate, &base_substitution),
            right: obligation.recurrence_base.clone(),
        };
        let expected_step = Equality {
            left: subtract(
                super::algebra::shift(&obligation.candidate, &obligation.index_variable, 1),
                obligation.candidate.clone(),
            ),
            right: obligation.recurrence_delta.clone(),
        };
        obligation.base_equality == expected_base
            && obligation.successor_difference_equality == expected_step
            && equality_holds(&obligation.base_equality).unwrap_or(false)
            && equality_holds(&obligation.successor_difference_equality).unwrap_or(false)
    }
}

impl Default for ProofKernel {
    fn default() -> Self {
        Self::new()
    }
}

fn condition_proves_nonzero(conditions: &[FormalCondition], divisor: &Expr) -> bool {
    conditions.iter().any(|condition| {
        matches!(condition, FormalCondition::NonZero { expression } if expression == divisor)
    })
}

fn divisor_is_provably_nonzero(divisor: &Expr) -> bool {
    match divisor {
        Expr::Rational(value) => !value.is_zero(),
        _ => false,
    }
}

pub fn transformation_rule_catalog() -> Vec<TransformationRuleRecord> {
    use RuleCode::*;
    let records = [
        (
            EqualityReflexivity,
            "result sides normalize to the same polynomial",
            vec![],
        ),
        (EqualitySymmetry, "swap both sides of an equality", vec![]),
        (
            EqualityTransitivity,
            "compose equal states with a shared middle state",
            vec![],
        ),
        (
            SubstituteEquals,
            "replace a subexpression by a verified equal expression",
            vec![],
        ),
        (
            AddBothSides,
            "add the identical witness to both sides",
            vec![],
        ),
        (
            SubtractBothSides,
            "subtract the identical witness from both sides",
            vec![],
        ),
        (
            MultiplyBothSides,
            "multiply both sides by the identical witness",
            vec![],
        ),
        (
            DivideBothSidesNonZero,
            "divide both sides by the identical nonzero rational witness",
            vec!["divisor is explicitly nonzero"],
        ),
        (
            AdditionAssociativity,
            "reassociate rational-polynomial addition",
            vec![],
        ),
        (
            AdditionCommutativity,
            "reorder rational-polynomial addition",
            vec![],
        ),
        (
            MultiplicationAssociativity,
            "reassociate rational-polynomial multiplication",
            vec![],
        ),
        (
            MultiplicationCommutativity,
            "reorder rational-polynomial multiplication",
            vec![],
        ),
        (
            Distributivity,
            "expand multiplication over addition",
            vec![],
        ),
        (
            IdentityElements,
            "remove additive zero or multiplicative one",
            vec![],
        ),
        (
            AdditiveInverse,
            "cancel an expression with its additive inverse",
            vec![],
        ),
        (
            DefinitionExpansion,
            "substitute the exact body of a locally supplied formal definition",
            vec!["operator definition exists", "arity matches"],
        ),
        (
            RationalPolynomialNormalization,
            "normalize using exact rational coefficient arithmetic",
            vec!["all divisors are nonzero rational constants"],
        ),
        (
            CaseSplit,
            "cover explicit mutually exclusive applicability conditions",
            vec![],
        ),
        (
            InductionBase,
            "verify the candidate at the declared base index",
            vec![],
        ),
        (
            InductionStep,
            "verify candidate successor difference equals recurrence delta",
            vec!["index is a nonnegative integer"],
        ),
    ];
    records
        .into_iter()
        .enumerate()
        .map(
            |(index, (rule, semantics, preconditions))| TransformationRuleRecord {
                rule_id: format!("MR{:06}", index + 1),
                rule,
                formal_preconditions: preconditions.iter().map(ToString::to_string).collect(),
                checkable_semantics: semantics.to_string(),
                domain_restrictions: vec!["exact rational polynomial expressions".to_string()],
                provenance: vec!["SEM4-MINIMAL-ALGEBRA-RULE-BASE".to_string()],
                target_formula_encoded: false,
            },
        )
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sem4::{
        algebra::{add, multiply, rational, variable},
        model::{ProofKind, Rational},
    };

    fn certificate_for(step: ProofStep, conclusion: Equality) -> ProofCertificate {
        ProofCertificate {
            certificate_id: "P-TEST".to_string(),
            proof_kind: ProofKind::Equational,
            assumptions: step.preconditions_checked.clone(),
            initial_statement: step.source_state.clone(),
            conclusion,
            steps: vec![step],
            induction: None,
            kernel_verified: false,
            primitive_expanded_proof_steps: 1,
            proof_dependencies: vec!["MR".to_string()],
            experimental_evidence_ids: vec![],
            formal_proof_evidence_ids: vec!["FORMAL".to_string()],
        }
    }

    #[test]
    fn invalid_cancellation_without_nonzero_proof_is_rejected() {
        let source = Equality {
            left: multiply(variable("x"), variable("y")),
            right: multiply(variable("x"), rational(2)),
        };
        let result = Equality {
            left: variable("y"),
            right: rational(2),
        };
        let step = ProofStep {
            sequence: 1,
            rule_applied: RuleCode::DivideBothSidesNonZero,
            source_state: source,
            result_state: result.clone(),
            witness: Some(variable("x")),
            preconditions_checked: vec![],
            supporting_concepts: vec![],
            proof_dependencies: vec![],
        };
        assert!(
            !ProofKernel::new()
                .verify(&certificate_for(step, result), &[])
                .valid
        );
    }

    #[test]
    fn division_by_zero_is_rejected_even_if_claimed_nonzero() {
        let zero = Expr::Rational(Rational::zero());
        let source = Equality {
            left: add(variable("x"), rational(1)),
            right: rational(3),
        };
        let result = source.clone();
        let step = ProofStep {
            sequence: 1,
            rule_applied: RuleCode::DivideBothSidesNonZero,
            source_state: source,
            result_state: result.clone(),
            witness: Some(zero.clone()),
            preconditions_checked: vec![FormalCondition::NonZero { expression: zero }],
            supporting_concepts: vec![],
            proof_dependencies: vec![],
        };
        assert!(
            !ProofKernel::new()
                .verify(&certificate_for(step, result), &[])
                .valid
        );
    }

    #[test]
    fn rule_catalog_contains_no_target_formula_rules() {
        let rules = transformation_rule_catalog();
        assert_eq!(rules.len(), 20);
        assert!(rules.iter().all(|rule| !rule.target_formula_encoded));
    }
}
