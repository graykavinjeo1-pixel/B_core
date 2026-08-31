//! Compact atomic knowledge for source-authorized repository repair.
//!
//! Rules in this module select only a mutation *family* and its proof
//! obligations. They carry no patch text, task identity, repository identity,
//! benchmark answer, or authority to mutate source.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::repository_coding_knowledge::RepositoryLanguage;
use crate::self_repair_contract::sha256;

pub const REPOSITORY_REPAIR_OPERATION_KNOWLEDGE_SCHEMA: &str =
    "B_REPOSITORY_REPAIR_OPERATION_KNOWLEDGE_1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RepairMutationFamilyIR {
    InsertGuard,
    ReplaceCondition,
    ReorderGuard,
    ChangeComparisonBoundary,
    ChangeValueSource,
    InsertStateUpdate,
    InsertCleanup,
    WrapWithBoundedRetry,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RepairSemanticBindingIR {
    DirectPublicParameterCondition,
    ExactDivisorBoundaryExclusion,
    OptionalValueFailurePolarity,
    AbsenceGuardBeforeDereference,
    EqualityBoundaryBeforeLengthIndex,
    OptionalMatchResultPropagation,
    StandardComparatorDirection,
    OrderedFallbackComposition,
    PreserveDirectiveSourceOrder,
    CooperativeCancellationInIteration,
    BoundedWaitReleasesStaleHandle,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RepairAuthorityEvidenceIR {
    BaselineFailureReproduced,
    PublicContractPresent,
    SourceAuthorityPresent,
    ExactCandidateReference,
    OptionalValueContract,
    ComparatorContract,
    DivisorBoundaryContract,
    FallbackOrderContract,
    DirectiveOrderContract,
    CancellationContract,
    BoundedWaitContract,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RepairProofObligationIR {
    FocusedVerifierPasses,
    BroaderRegressionPasses,
    CandidateParses,
    OutsideSelectedSpanExact,
    RollbackExact,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepairOperationKnowledgeRuleIR {
    pub binding: RepairSemanticBindingIR,
    pub mutation_family: RepairMutationFamilyIR,
    pub supported_languages: Vec<RepositoryLanguage>,
    pub required_evidence: Vec<RepairAuthorityEvidenceIR>,
    pub proof_obligations: Vec<RepairProofObligationIR>,
    pub patch_template: Option<String>,
    pub source_mutation_authorized: bool,
    pub task_identity_authority: bool,
    pub repository_identity_authority: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepairOperationKnowledgeCatalogIR {
    pub schema: String,
    pub rules: Vec<RepairOperationKnowledgeRuleIR>,
    pub external_llm_calls: u64,
    pub network_reads: u64,
    pub catalog_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepairOperationQueryIR {
    pub schema: String,
    pub language: RepositoryLanguage,
    pub requested_binding: RepairSemanticBindingIR,
    pub observed_evidence: Vec<RepairAuthorityEvidenceIR>,
    pub exact_candidate_references: usize,
    pub target_protected: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RepairOperationDispositionIR {
    Applicable,
    Abstain,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepairOperationDecisionIR {
    pub schema: String,
    pub disposition: RepairOperationDispositionIR,
    pub binding: RepairSemanticBindingIR,
    pub mutation_family: Option<RepairMutationFamilyIR>,
    pub missing_evidence: Vec<RepairAuthorityEvidenceIR>,
    pub proof_obligations: Vec<RepairProofObligationIR>,
    pub reason_codes: Vec<String>,
    pub patch_content: Option<String>,
    pub source_mutation_authorized: bool,
    pub task_identity_routing_events: u64,
    pub repository_identity_routing_events: u64,
}

fn all_languages() -> Vec<RepositoryLanguage> {
    vec![
        RepositoryLanguage::Rust,
        RepositoryLanguage::Python,
        RepositoryLanguage::TypeScript,
        RepositoryLanguage::JavaScript,
        RepositoryLanguage::Go,
    ]
}

fn scripting_languages() -> Vec<RepositoryLanguage> {
    vec![
        RepositoryLanguage::Python,
        RepositoryLanguage::TypeScript,
        RepositoryLanguage::JavaScript,
    ]
}

fn base_evidence() -> Vec<RepairAuthorityEvidenceIR> {
    vec![
        RepairAuthorityEvidenceIR::BaselineFailureReproduced,
        RepairAuthorityEvidenceIR::PublicContractPresent,
        RepairAuthorityEvidenceIR::SourceAuthorityPresent,
        RepairAuthorityEvidenceIR::ExactCandidateReference,
    ]
}

fn rule(
    binding: RepairSemanticBindingIR,
    mutation_family: RepairMutationFamilyIR,
    supported_languages: Vec<RepositoryLanguage>,
    specialized_evidence: Option<RepairAuthorityEvidenceIR>,
) -> RepairOperationKnowledgeRuleIR {
    let mut required_evidence = base_evidence();
    required_evidence.extend(specialized_evidence);
    RepairOperationKnowledgeRuleIR {
        binding,
        mutation_family,
        supported_languages,
        required_evidence,
        proof_obligations: vec![
            RepairProofObligationIR::FocusedVerifierPasses,
            RepairProofObligationIR::BroaderRegressionPasses,
            RepairProofObligationIR::CandidateParses,
            RepairProofObligationIR::OutsideSelectedSpanExact,
            RepairProofObligationIR::RollbackExact,
        ],
        patch_template: None,
        source_mutation_authorized: false,
        task_identity_authority: false,
        repository_identity_authority: false,
    }
}

fn catalog_rules() -> Vec<RepairOperationKnowledgeRuleIR> {
    use RepairAuthorityEvidenceIR as Evidence;
    use RepairMutationFamilyIR as Mutation;
    use RepairSemanticBindingIR as Binding;

    vec![
        rule(
            Binding::DirectPublicParameterCondition,
            Mutation::ReplaceCondition,
            all_languages(),
            None,
        ),
        rule(
            Binding::ExactDivisorBoundaryExclusion,
            Mutation::ChangeComparisonBoundary,
            vec![RepositoryLanguage::Rust],
            Some(Evidence::DivisorBoundaryContract),
        ),
        rule(
            Binding::OptionalValueFailurePolarity,
            Mutation::ReplaceCondition,
            scripting_languages(),
            Some(Evidence::OptionalValueContract),
        ),
        rule(
            Binding::AbsenceGuardBeforeDereference,
            Mutation::InsertGuard,
            all_languages(),
            Some(Evidence::OptionalValueContract),
        ),
        rule(
            Binding::EqualityBoundaryBeforeLengthIndex,
            Mutation::ChangeComparisonBoundary,
            all_languages(),
            Some(Evidence::ComparatorContract),
        ),
        rule(
            Binding::OptionalMatchResultPropagation,
            Mutation::ChangeValueSource,
            scripting_languages(),
            Some(Evidence::OptionalValueContract),
        ),
        rule(
            Binding::StandardComparatorDirection,
            Mutation::ChangeComparisonBoundary,
            all_languages(),
            Some(Evidence::ComparatorContract),
        ),
        rule(
            Binding::OrderedFallbackComposition,
            Mutation::ReorderGuard,
            all_languages(),
            Some(Evidence::FallbackOrderContract),
        ),
        rule(
            Binding::PreserveDirectiveSourceOrder,
            Mutation::ReorderGuard,
            all_languages(),
            Some(Evidence::DirectiveOrderContract),
        ),
        rule(
            Binding::CooperativeCancellationInIteration,
            Mutation::InsertStateUpdate,
            all_languages(),
            Some(Evidence::CancellationContract),
        ),
        rule(
            Binding::BoundedWaitReleasesStaleHandle,
            Mutation::WrapWithBoundedRetry,
            all_languages(),
            Some(Evidence::BoundedWaitContract),
        ),
    ]
}

fn catalog_projection(catalog: &RepairOperationKnowledgeCatalogIR) -> Result<Vec<u8>, String> {
    serde_json::to_vec(&(
        &catalog.schema,
        &catalog.rules,
        catalog.external_llm_calls,
        catalog.network_reads,
    ))
    .map_err(|error| format!("REPAIR_OPERATION_CATALOG_SERIALIZE:{error}"))
}

pub fn build_repair_operation_knowledge_catalog(
) -> Result<RepairOperationKnowledgeCatalogIR, String> {
    let mut catalog = RepairOperationKnowledgeCatalogIR {
        schema: REPOSITORY_REPAIR_OPERATION_KNOWLEDGE_SCHEMA.to_string(),
        rules: catalog_rules(),
        external_llm_calls: 0,
        network_reads: 0,
        catalog_sha256: String::new(),
    };
    catalog.catalog_sha256 = sha256(&catalog_projection(&catalog)?);
    validate_repair_operation_knowledge_catalog(&catalog)?;
    Ok(catalog)
}

pub fn validate_repair_operation_knowledge_catalog(
    catalog: &RepairOperationKnowledgeCatalogIR,
) -> Result<(), String> {
    let base = base_evidence().into_iter().collect::<BTreeSet<_>>();
    let mut bindings = BTreeSet::new();
    let valid = catalog.schema == REPOSITORY_REPAIR_OPERATION_KNOWLEDGE_SCHEMA
        && !catalog.rules.is_empty()
        && catalog.external_llm_calls == 0
        && catalog.network_reads == 0
        && catalog.catalog_sha256.len() == 64
        && catalog
            .catalog_sha256
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit())
        && catalog.catalog_sha256 == sha256(&catalog_projection(catalog)?)
        && catalog.rules.iter().all(|rule| {
            let languages = rule
                .supported_languages
                .iter()
                .copied()
                .collect::<BTreeSet<_>>();
            let evidence = rule
                .required_evidence
                .iter()
                .copied()
                .collect::<BTreeSet<_>>();
            let obligations = rule
                .proof_obligations
                .iter()
                .copied()
                .collect::<BTreeSet<_>>();
            bindings.insert(rule.binding)
                && !languages.is_empty()
                && languages.len() == rule.supported_languages.len()
                && languages.iter().all(|language| language.supported())
                && evidence.len() == rule.required_evidence.len()
                && base.is_subset(&evidence)
                && obligations.len() == rule.proof_obligations.len()
                && obligations.contains(&RepairProofObligationIR::FocusedVerifierPasses)
                && obligations.contains(&RepairProofObligationIR::BroaderRegressionPasses)
                && obligations.contains(&RepairProofObligationIR::OutsideSelectedSpanExact)
                && obligations.contains(&RepairProofObligationIR::RollbackExact)
                && rule.patch_template.is_none()
                && !rule.source_mutation_authorized
                && !rule.task_identity_authority
                && !rule.repository_identity_authority
        });
    if !valid {
        return Err("REPAIR_OPERATION_CATALOG_INVALID".to_string());
    }
    Ok(())
}

pub fn query_repair_operation_knowledge(
    catalog: &RepairOperationKnowledgeCatalogIR,
    query: &RepairOperationQueryIR,
) -> Result<RepairOperationDecisionIR, String> {
    validate_repair_operation_knowledge_catalog(catalog)?;
    if query.schema != REPOSITORY_REPAIR_OPERATION_KNOWLEDGE_SCHEMA {
        return Err("REPAIR_OPERATION_QUERY_SCHEMA".to_string());
    }
    let observed = query
        .observed_evidence
        .iter()
        .copied()
        .collect::<BTreeSet<_>>();
    if observed.len() != query.observed_evidence.len() {
        return Err("REPAIR_OPERATION_QUERY_DUPLICATE_EVIDENCE".to_string());
    }
    let rule = catalog
        .rules
        .iter()
        .find(|rule| rule.binding == query.requested_binding)
        .ok_or_else(|| "REPAIR_OPERATION_BINDING_UNKNOWN".to_string())?;
    let missing_evidence = rule
        .required_evidence
        .iter()
        .copied()
        .filter(|evidence| !observed.contains(evidence))
        .collect::<Vec<_>>();
    let mut reason_codes = Vec::new();
    if !rule.supported_languages.contains(&query.language) {
        reason_codes.push("LANGUAGE_UNSUPPORTED".to_string());
    }
    if query.exact_candidate_references != 1 {
        reason_codes.push("EXACT_CANDIDATE_REFERENCE_NOT_UNIQUE".to_string());
    }
    if query.target_protected {
        reason_codes.push("TARGET_PROTECTED".to_string());
    }
    if !missing_evidence.is_empty() {
        reason_codes.push("SOURCE_AUTHORITY_EVIDENCE_INCOMPLETE".to_string());
    }
    let applicable = reason_codes.is_empty();
    Ok(RepairOperationDecisionIR {
        schema: REPOSITORY_REPAIR_OPERATION_KNOWLEDGE_SCHEMA.to_string(),
        disposition: if applicable {
            RepairOperationDispositionIR::Applicable
        } else {
            RepairOperationDispositionIR::Abstain
        },
        binding: query.requested_binding,
        mutation_family: applicable.then_some(rule.mutation_family),
        missing_evidence,
        proof_obligations: if applicable {
            rule.proof_obligations.clone()
        } else {
            Vec::new()
        },
        reason_codes,
        patch_content: None,
        source_mutation_authorized: false,
        task_identity_routing_events: 0,
        repository_identity_routing_events: 0,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn query_for(
        catalog: &RepairOperationKnowledgeCatalogIR,
        binding: RepairSemanticBindingIR,
        language: RepositoryLanguage,
    ) -> RepairOperationQueryIR {
        let rule = catalog
            .rules
            .iter()
            .find(|rule| rule.binding == binding)
            .expect("rule");
        RepairOperationQueryIR {
            schema: REPOSITORY_REPAIR_OPERATION_KNOWLEDGE_SCHEMA.to_string(),
            language,
            requested_binding: binding,
            observed_evidence: rule.required_evidence.clone(),
            exact_candidate_references: 1,
            target_protected: false,
        }
    }

    #[test]
    fn catalog_is_compact_and_contains_no_patch_templates() {
        let catalog = build_repair_operation_knowledge_catalog().expect("catalog");
        assert_eq!(catalog.rules.len(), 11);
        assert!(catalog
            .rules
            .iter()
            .all(|rule| rule.patch_template.is_none()));
        assert!(catalog
            .rules
            .iter()
            .all(|rule| !rule.source_mutation_authorized));
    }

    #[test]
    fn complete_source_authority_selects_only_an_operation_family() {
        let catalog = build_repair_operation_knowledge_catalog().expect("catalog");
        let decision = query_repair_operation_knowledge(
            &catalog,
            &query_for(
                &catalog,
                RepairSemanticBindingIR::CooperativeCancellationInIteration,
                RepositoryLanguage::Rust,
            ),
        )
        .expect("decision");
        assert_eq!(
            decision.disposition,
            RepairOperationDispositionIR::Applicable
        );
        assert_eq!(
            decision.mutation_family,
            Some(RepairMutationFamilyIR::InsertStateUpdate)
        );
        assert!(decision.patch_content.is_none());
        assert!(!decision.source_mutation_authorized);
    }

    #[test]
    fn ambiguous_or_protected_targets_fail_closed() {
        let catalog = build_repair_operation_knowledge_catalog().expect("catalog");
        let mut ambiguous = query_for(
            &catalog,
            RepairSemanticBindingIR::StandardComparatorDirection,
            RepositoryLanguage::TypeScript,
        );
        ambiguous.exact_candidate_references = 2;
        let decision = query_repair_operation_knowledge(&catalog, &ambiguous).expect("decision");
        assert_eq!(decision.disposition, RepairOperationDispositionIR::Abstain);
        assert!(decision.mutation_family.is_none());

        ambiguous.exact_candidate_references = 1;
        ambiguous.target_protected = true;
        let decision = query_repair_operation_knowledge(&catalog, &ambiguous).expect("decision");
        assert_eq!(decision.disposition, RepairOperationDispositionIR::Abstain);
    }

    #[test]
    fn missing_specialized_evidence_abstains() {
        let catalog = build_repair_operation_knowledge_catalog().expect("catalog");
        let mut query = query_for(
            &catalog,
            RepairSemanticBindingIR::BoundedWaitReleasesStaleHandle,
            RepositoryLanguage::Go,
        );
        query
            .observed_evidence
            .retain(|item| *item != RepairAuthorityEvidenceIR::BoundedWaitContract);
        let decision = query_repair_operation_knowledge(&catalog, &query).expect("decision");
        assert_eq!(decision.disposition, RepairOperationDispositionIR::Abstain);
        assert_eq!(
            decision.missing_evidence,
            vec![RepairAuthorityEvidenceIR::BoundedWaitContract]
        );
    }

    #[test]
    fn one_semantic_binding_is_shared_across_languages() {
        let catalog = build_repair_operation_knowledge_catalog().expect("catalog");
        for language in all_languages() {
            let decision = query_repair_operation_knowledge(
                &catalog,
                &query_for(
                    &catalog,
                    RepairSemanticBindingIR::DirectPublicParameterCondition,
                    language,
                ),
            )
            .expect("decision");
            assert_eq!(
                decision.disposition,
                RepairOperationDispositionIR::Applicable
            );
        }
    }
}
