//! Evidence-bound semantic compilation of natural-language causal claims.
//!
//! Language proposes which typed propositions participate in a mechanism.
//! Repeated state transitions, failed attempts, and no-action controls decide
//! whether the proposal has causal support. Text alone can never produce
//! executable mechanism knowledge.

use std::collections::{BTreeMap, BTreeSet};

use dockable_semantic_core::{
    ActionAuthorityIR, CausalMechanismIR, LiteralIR, MechanismKindIR, MechanismKnowledgeIR,
    MECHANISM_KNOWLEDGE_SCHEMA,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub const MECHANISM_INDUCTION_REQUEST_SCHEMA: &str = "B_CORE_MECHANISM_INDUCTION_REQUEST_IR_1";
pub const MECHANISM_INDUCTION_SCHEMA: &str = "B_CORE_MECHANISM_INDUCTION_IR_1";
const MAX_OBSERVATIONS: usize = 128;
const MAX_PROPOSITIONS: usize = 64;
const MAX_LEXICON: usize = 128;
const MAX_ALIASES: usize = 16;
const MAX_EVIDENCE_REFS: usize = 64;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TransitionArmIR {
    AppliedSuccess,
    AppliedFailure,
    NoActionControl,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StateTransitionObservationIR {
    pub observation_id: String,
    pub arm: TransitionArmIR,
    pub before: Vec<LiteralIR>,
    pub after: Vec<LiteralIR>,
    pub reliability_millis: u16,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PropositionLexemeIR {
    pub proposition_id: String,
    pub aliases: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MechanismInductionRequestIR {
    pub schema: String,
    pub request_id: String,
    pub knowledge_id: String,
    pub mechanism_id: String,
    pub natural_language_statement: String,
    pub kind: MechanismKindIR,
    pub authority: ActionAuthorityIR,
    pub authorized: bool,
    pub reversible: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recovery_reference: Option<String>,
    pub semantic_tags: Vec<String>,
    pub proposition_lexicon: Vec<PropositionLexemeIR>,
    pub observations: Vec<StateTransitionObservationIR>,
    pub minimum_positive_support: usize,
    pub minimum_confidence_millis: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum MechanismInductionDispositionIR {
    Compiled,
    PublicInformationInsufficient,
    ConflictingObservations,
    AmbiguousLanguageBinding,
    NonCausalCorrelation,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MechanismInductionIR {
    pub schema: String,
    pub request_id: String,
    pub disposition: MechanismInductionDispositionIR,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub knowledge: Option<MechanismKnowledgeIR>,
    pub language_bound_preconditions: Vec<String>,
    pub language_bound_effects: Vec<String>,
    pub supporting_observation_ids: Vec<String>,
    pub failed_attempt_observation_ids: Vec<String>,
    pub control_observation_ids: Vec<String>,
    pub rejected_effect_proposition_ids: Vec<String>,
    pub positive_support: usize,
    pub causal_control_support: usize,
    pub confidence_millis: u16,
    pub text_only_authority_events: usize,
    pub external_model_calls: usize,
    pub induction_sha256: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum MechanismInductionError {
    InvalidSchema,
    InvalidRequest,
    InvalidObservation,
    DuplicateIdentity,
}

#[derive(Debug, Default)]
pub struct MechanismInductionEngine;

impl MechanismInductionEngine {
    pub fn compile(
        &self,
        request: &MechanismInductionRequestIR,
    ) -> Result<MechanismInductionIR, MechanismInductionError> {
        validate_request(request)?;
        let bindings = language_bindings(request)?;
        let positives = request
            .observations
            .iter()
            .filter(|observation| observation.arm == TransitionArmIR::AppliedSuccess)
            .collect::<Vec<_>>();
        let failed = request
            .observations
            .iter()
            .filter(|observation| observation.arm == TransitionArmIR::AppliedFailure)
            .collect::<Vec<_>>();
        let controls = request
            .observations
            .iter()
            .filter(|observation| observation.arm == TransitionArmIR::NoActionControl)
            .collect::<Vec<_>>();
        if positives.len() < request.minimum_positive_support {
            return Ok(result_without_knowledge(
                request,
                MechanismInductionDispositionIR::PublicInformationInsufficient,
                &bindings,
                &positives,
                &failed,
                &controls,
                Vec::new(),
                0,
            ));
        }

        let positive_deltas = positives
            .iter()
            .map(|observation| state_delta(observation))
            .collect::<Vec<_>>();
        if has_cross_observation_conflict(&positive_deltas)
            || positive_outcomes_conflict(&positives, &positive_deltas)
        {
            return Ok(result_without_knowledge(
                request,
                MechanismInductionDispositionIR::ConflictingObservations,
                &bindings,
                &positives,
                &failed,
                &controls,
                Vec::new(),
                0,
            ));
        }
        let common_effects = intersection(&positive_deltas);
        let mut effects = common_effects
            .iter()
            .filter(|effect| bindings.effect_hints.contains(&effect.proposition_id))
            .cloned()
            .collect::<Vec<_>>();
        effects.sort();
        let mut rejected_effects = common_effects
            .iter()
            .filter(|effect| !bindings.effect_hints.contains(&effect.proposition_id))
            .map(|effect| effect.proposition_id.clone())
            .collect::<Vec<_>>();
        let control_deltas = controls
            .iter()
            .map(|observation| state_delta(observation))
            .collect::<Vec<_>>();
        let failed_deltas = failed
            .iter()
            .map(|observation| state_delta(observation))
            .collect::<Vec<_>>();
        let mut noncausal = Vec::new();
        effects.retain(|effect| {
            let appears_without_success = control_deltas
                .iter()
                .chain(&failed_deltas)
                .any(|delta| delta.contains(effect));
            if appears_without_success {
                noncausal.push(effect.proposition_id.clone());
            }
            !appears_without_success
        });
        let has_noncausal_effect = !noncausal.is_empty();
        rejected_effects.extend(noncausal);
        rejected_effects.sort();
        rejected_effects.dedup();

        let common_before = intersection(
            &positives
                .iter()
                .map(|observation| literal_set(&observation.before))
                .collect::<Vec<_>>(),
        );
        let mut prerequisites = common_before
            .iter()
            .filter(|literal| {
                bindings
                    .prerequisite_hints
                    .contains(&literal.proposition_id)
                    && !effects
                        .iter()
                        .any(|effect| effect.proposition_id == literal.proposition_id)
            })
            .cloned()
            .collect::<Vec<_>>();
        prerequisites.sort();

        let language_bound_effects = effects
            .iter()
            .map(|effect| effect.proposition_id.clone())
            .collect::<Vec<_>>();
        let language_bound_preconditions = prerequisites
            .iter()
            .map(|literal| literal.proposition_id.clone())
            .collect::<Vec<_>>();
        if language_bound_effects.is_empty()
            || (request.kind != MechanismKindIR::Diagnostic
                && language_bound_preconditions.is_empty())
        {
            return Ok(result_without_knowledge(
                request,
                if has_noncausal_effect {
                    MechanismInductionDispositionIR::NonCausalCorrelation
                } else {
                    MechanismInductionDispositionIR::AmbiguousLanguageBinding
                },
                &bindings,
                &positives,
                &failed,
                &controls,
                rejected_effects,
                0,
            ));
        }
        if controls.is_empty() && failed.is_empty() {
            return Ok(result_without_knowledge(
                request,
                MechanismInductionDispositionIR::PublicInformationInsufficient,
                &bindings,
                &positives,
                &failed,
                &controls,
                rejected_effects,
                0,
            ));
        }

        let causal_control_support = controls
            .iter()
            .chain(&failed)
            .filter(|observation| {
                let delta = state_delta(observation);
                effects.iter().all(|effect| !delta.contains(effect))
            })
            .count();
        let confidence_millis = induction_confidence(&positives, causal_control_support);
        if confidence_millis < request.minimum_confidence_millis {
            return Ok(result_without_knowledge(
                request,
                MechanismInductionDispositionIR::PublicInformationInsufficient,
                &bindings,
                &positives,
                &failed,
                &controls,
                rejected_effects,
                confidence_millis,
            ));
        }

        let evidence_refs = request
            .observations
            .iter()
            .flat_map(|observation| observation.evidence_refs.iter().cloned())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();
        let observes = if request.kind == MechanismKindIR::Diagnostic {
            effects
                .iter()
                .map(|effect| effect.proposition_id.clone())
                .collect()
        } else {
            Vec::new()
        };
        let mechanism_effects = if request.kind == MechanismKindIR::Diagnostic {
            Vec::new()
        } else {
            effects
        };
        let knowledge = MechanismKnowledgeIR {
            schema: MECHANISM_KNOWLEDGE_SCHEMA.to_string(),
            knowledge_id: request.knowledge_id.clone(),
            mechanism: CausalMechanismIR {
                mechanism_id: request.mechanism_id.clone(),
                kind: request.kind,
                prerequisites,
                effects: mechanism_effects,
                observes,
                authority: request.authority,
                authorized: request.authorized,
                reversible: request.reversible,
                recovery_reference: request.recovery_reference.clone(),
                cost_millis: 10,
                risk_millis: authority_risk(request.authority),
                provenance_refs: evidence_refs.clone(),
            },
            semantic_tags: request.semantic_tags.clone(),
            validation_evidence_refs: evidence_refs,
            confidence_millis,
        };
        let mut result = MechanismInductionIR {
            schema: MECHANISM_INDUCTION_SCHEMA.to_string(),
            request_id: request.request_id.clone(),
            disposition: MechanismInductionDispositionIR::Compiled,
            knowledge: Some(knowledge),
            language_bound_preconditions,
            language_bound_effects,
            supporting_observation_ids: observation_ids(&positives),
            failed_attempt_observation_ids: observation_ids(&failed),
            control_observation_ids: observation_ids(&controls),
            rejected_effect_proposition_ids: rejected_effects,
            positive_support: positives.len(),
            causal_control_support,
            confidence_millis,
            text_only_authority_events: 0,
            external_model_calls: 0,
            induction_sha256: String::new(),
        };
        result.induction_sha256 = induction_hash(&result);
        Ok(result)
    }
}

#[derive(Debug)]
struct LanguageBindings {
    prerequisite_hints: BTreeSet<String>,
    effect_hints: BTreeSet<String>,
}

fn language_bindings(
    request: &MechanismInductionRequestIR,
) -> Result<LanguageBindings, MechanismInductionError> {
    let normalized = request.natural_language_statement.to_lowercase();
    let mut alias_owner = BTreeMap::<String, String>::new();
    let mut mentions = Vec::<(usize, String)>::new();
    for lexeme in &request.proposition_lexicon {
        for alias in &lexeme.aliases {
            let alias = alias.trim().to_lowercase();
            if normalized.contains(&alias) {
                if alias_owner
                    .insert(alias.clone(), lexeme.proposition_id.clone())
                    .is_some_and(|owner| owner != lexeme.proposition_id)
                {
                    return Err(MechanismInductionError::InvalidRequest);
                }
                if let Some(position) = normalized.find(&alias) {
                    mentions.push((position, lexeme.proposition_id.clone()));
                }
            }
        }
    }
    let mut prerequisite_hints = BTreeSet::new();
    let mut effect_hints = BTreeSet::new();
    if let Some((cause_range, effect_range)) = causal_clause_ranges(&normalized) {
        for (position, proposition_id) in mentions {
            if cause_range.contains(&position) {
                prerequisite_hints.insert(proposition_id);
            } else if effect_range.contains(&position) {
                effect_hints.insert(proposition_id);
            }
        }
    }
    Ok(LanguageBindings {
        prerequisite_hints,
        effect_hints,
    })
}

fn causal_clause_ranges(text: &str) -> Option<(std::ops::Range<usize>, std::ops::Range<usize>)> {
    for connector in [" because of ", " because ", " due to ", "때문에", "덕분에"] {
        if let Some(position) = text.find(connector) {
            return Some((position + connector.len()..text.len(), 0..position));
        }
    }
    let trimmed = text.trim_start();
    if trimmed.starts_with("if ") || trimmed.starts_with("when ") {
        if let Some(comma) = text.find(',') {
            return Some((0..comma, comma + 1..text.len()));
        }
    }
    for connector in [
        " causes ",
        " cause ",
        " leads to ",
        " enables ",
        " produces ",
        " results in ",
        " makes ",
        "하면",
        "이면",
        "할 때",
        "일 때",
    ] {
        if let Some(position) = text.find(connector) {
            return Some((0..position, position + connector.len()..text.len()));
        }
    }
    None
}

fn validate_request(request: &MechanismInductionRequestIR) -> Result<(), MechanismInductionError> {
    if request.schema != MECHANISM_INDUCTION_REQUEST_SCHEMA {
        return Err(MechanismInductionError::InvalidSchema);
    }
    if !valid_id(&request.request_id)
        || !valid_id(&request.knowledge_id)
        || !valid_id(&request.mechanism_id)
        || request.natural_language_statement.trim().is_empty()
        || request.natural_language_statement.len() > 64 * 1024
        || request.semantic_tags.is_empty()
        || request.semantic_tags.len() > MAX_PROPOSITIONS
        || request
            .semantic_tags
            .iter()
            .any(|tag| tag.trim().is_empty() || tag.len() > 128)
        || request.proposition_lexicon.is_empty()
        || request.proposition_lexicon.len() > MAX_LEXICON
        || request.observations.is_empty()
        || request.observations.len() > MAX_OBSERVATIONS
        || !(2..=32).contains(&request.minimum_positive_support)
        || request.minimum_confidence_millis > 1_000
        || request.minimum_confidence_millis == 0
        || request.authority == ActionAuthorityIR::IrreversibleMutation
        || !matches!(
            (request.kind, request.authority),
            (
                MechanismKindIR::Inference,
                ActionAuthorityIR::InternalInference
            ) | (
                MechanismKindIR::Diagnostic,
                ActionAuthorityIR::ReadOnlyObservation
            ) | (
                MechanismKindIR::Intervention,
                ActionAuthorityIR::ReversibleMutation
            )
        )
        || (request.authority == ActionAuthorityIR::ReversibleMutation
            && (!request.reversible || request.recovery_reference.is_none()))
    {
        return Err(MechanismInductionError::InvalidRequest);
    }
    let mut proposition_ids = BTreeSet::new();
    for lexeme in &request.proposition_lexicon {
        if !valid_id(&lexeme.proposition_id)
            || !proposition_ids.insert(lexeme.proposition_id.clone())
            || lexeme.aliases.is_empty()
            || lexeme.aliases.len() > MAX_ALIASES
            || lexeme
                .aliases
                .iter()
                .any(|alias| alias.trim().is_empty() || alias.len() > 256)
        {
            return Err(MechanismInductionError::InvalidRequest);
        }
    }
    let mut observation_ids = BTreeSet::new();
    let mut evidence_refs = BTreeSet::new();
    for observation in &request.observations {
        let before = literal_map(&observation.before);
        let after = literal_map(&observation.after);
        if !valid_id(&observation.observation_id)
            || !observation_ids.insert(observation.observation_id.clone())
            || observation.before.len() > MAX_PROPOSITIONS
            || observation.after.len() > MAX_PROPOSITIONS
            || observation.reliability_millis == 0
            || observation.reliability_millis > 1_000
            || observation.evidence_refs.is_empty()
            || observation.evidence_refs.len() > MAX_EVIDENCE_REFS
            || observation
                .evidence_refs
                .iter()
                .any(|reference| reference.trim().is_empty() || reference.len() > 1_024)
            || before.is_none()
            || after.is_none()
            || before
                .as_ref()
                .zip(after.as_ref())
                .is_none_or(|(before, after)| before.keys().ne(after.keys()))
            || observation
                .before
                .iter()
                .chain(&observation.after)
                .any(|literal| !proposition_ids.contains(&literal.proposition_id))
        {
            return Err(MechanismInductionError::InvalidObservation);
        }
        evidence_refs.extend(observation.evidence_refs.iter().cloned());
        if evidence_refs.len() > MAX_EVIDENCE_REFS {
            return Err(MechanismInductionError::InvalidRequest);
        }
    }
    Ok(())
}

fn state_delta(observation: &StateTransitionObservationIR) -> BTreeSet<LiteralIR> {
    let before = literal_map(&observation.before).unwrap_or_default();
    observation
        .after
        .iter()
        .filter(|literal| before.get(&literal.proposition_id) != Some(&literal.value))
        .cloned()
        .collect()
}

fn literal_set(literals: &[LiteralIR]) -> BTreeSet<LiteralIR> {
    literals.iter().cloned().collect()
}

fn literal_map(literals: &[LiteralIR]) -> Option<BTreeMap<String, bool>> {
    let mut map = BTreeMap::new();
    for literal in literals {
        if !valid_id(&literal.proposition_id)
            || map
                .insert(literal.proposition_id.clone(), literal.value)
                .is_some()
        {
            return None;
        }
    }
    Some(map)
}

fn intersection(sets: &[BTreeSet<LiteralIR>]) -> BTreeSet<LiteralIR> {
    let Some(first) = sets.first() else {
        return BTreeSet::new();
    };
    sets.iter().skip(1).fold(first.clone(), |common, set| {
        common.intersection(set).cloned().collect()
    })
}

fn has_cross_observation_conflict(deltas: &[BTreeSet<LiteralIR>]) -> bool {
    let mut values = BTreeMap::<String, bool>::new();
    for delta in deltas {
        for literal in delta {
            if values
                .insert(literal.proposition_id.clone(), literal.value)
                .is_some_and(|value| value != literal.value)
            {
                return true;
            }
        }
    }
    false
}

fn positive_outcomes_conflict(
    positives: &[&StateTransitionObservationIR],
    deltas: &[BTreeSet<LiteralIR>],
) -> bool {
    let changed_ids = deltas
        .iter()
        .flat_map(|delta| delta.iter().map(|literal| literal.proposition_id.clone()))
        .collect::<BTreeSet<_>>();
    changed_ids.into_iter().any(|proposition_id| {
        positives
            .iter()
            .filter_map(|observation| {
                observation
                    .after
                    .iter()
                    .find(|literal| literal.proposition_id == proposition_id)
                    .map(|literal| literal.value)
            })
            .collect::<BTreeSet<_>>()
            .len()
            > 1
    })
}

fn induction_confidence(
    positives: &[&StateTransitionObservationIR],
    causal_control_support: usize,
) -> u16 {
    let reliability = positives
        .iter()
        .map(|observation| u32::from(observation.reliability_millis))
        .sum::<u32>()
        / u32::try_from(positives.len()).unwrap_or(1).max(1);
    let support_bonus = u32::try_from(positives.len().saturating_sub(1).min(5) * 25).unwrap_or(125);
    let control_bonus = u32::try_from(causal_control_support.min(5) * 30).unwrap_or(150);
    u16::try_from(
        reliability
            .saturating_add(support_bonus)
            .saturating_add(control_bonus)
            .min(1_000),
    )
    .unwrap_or(1_000)
}

fn observation_ids(observations: &[&StateTransitionObservationIR]) -> Vec<String> {
    observations
        .iter()
        .map(|observation| observation.observation_id.clone())
        .collect()
}

fn authority_risk(authority: ActionAuthorityIR) -> u16 {
    match authority {
        ActionAuthorityIR::InternalInference => 0,
        ActionAuthorityIR::ReadOnlyObservation => 10,
        ActionAuthorityIR::ReversibleMutation => 100,
        ActionAuthorityIR::IrreversibleMutation => 1_000,
    }
}

#[allow(clippy::too_many_arguments)]
fn result_without_knowledge(
    request: &MechanismInductionRequestIR,
    disposition: MechanismInductionDispositionIR,
    bindings: &LanguageBindings,
    positives: &[&StateTransitionObservationIR],
    failed: &[&StateTransitionObservationIR],
    controls: &[&StateTransitionObservationIR],
    rejected_effects: Vec<String>,
    confidence_millis: u16,
) -> MechanismInductionIR {
    let mut result = MechanismInductionIR {
        schema: MECHANISM_INDUCTION_SCHEMA.to_string(),
        request_id: request.request_id.clone(),
        disposition,
        knowledge: None,
        language_bound_preconditions: bindings.prerequisite_hints.iter().cloned().collect(),
        language_bound_effects: bindings.effect_hints.iter().cloned().collect(),
        supporting_observation_ids: observation_ids(positives),
        failed_attempt_observation_ids: observation_ids(failed),
        control_observation_ids: observation_ids(controls),
        rejected_effect_proposition_ids: rejected_effects,
        positive_support: positives.len(),
        causal_control_support: 0,
        confidence_millis,
        text_only_authority_events: 0,
        external_model_calls: 0,
        induction_sha256: String::new(),
    };
    result.induction_sha256 = induction_hash(&result);
    result
}

fn induction_hash(result: &MechanismInductionIR) -> String {
    let mut unsigned = result.clone();
    unsigned.induction_sha256.clear();
    format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(&unsigned).unwrap_or_default())
    )
}

fn valid_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':'))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn literal(id: &str, value: bool) -> LiteralIR {
        LiteralIR {
            proposition_id: id.to_string(),
            value,
        }
    }

    fn observation(
        id: &str,
        arm: TransitionArmIR,
        before: Vec<LiteralIR>,
        after: Vec<LiteralIR>,
    ) -> StateTransitionObservationIR {
        StateTransitionObservationIR {
            observation_id: id.to_string(),
            arm,
            before,
            after,
            reliability_millis: 900,
            evidence_refs: vec![format!("test:{id}")],
        }
    }

    fn request(statement: &str) -> MechanismInductionRequestIR {
        MechanismInductionRequestIR {
            schema: MECHANISM_INDUCTION_REQUEST_SCHEMA.to_string(),
            request_id: "INDUCE-1".to_string(),
            knowledge_id: "KNOWLEDGE-FLUX".to_string(),
            mechanism_id: "NORMALIZE-FLUX".to_string(),
            natural_language_statement: statement.to_string(),
            kind: MechanismKindIR::Intervention,
            authority: ActionAuthorityIR::ReversibleMutation,
            authorized: true,
            reversible: true,
            recovery_reference: Some("sealed-predecessor:test".to_string()),
            semantic_tags: vec!["fresh-domain".to_string(), "repair".to_string()],
            proposition_lexicon: vec![
                PropositionLexemeIR {
                    proposition_id: "FLUX_READY".to_string(),
                    aliases: vec!["flux-ready".to_string(), "플럭스 준비".to_string()],
                },
                PropositionLexemeIR {
                    proposition_id: "OUTPUT_RESTORED".to_string(),
                    aliases: vec!["output restored".to_string(), "출력 복원".to_string()],
                },
            ],
            observations: vec![
                observation(
                    "POS-1",
                    TransitionArmIR::AppliedSuccess,
                    vec![
                        literal("FLUX_READY", true),
                        literal("OUTPUT_RESTORED", false),
                    ],
                    vec![
                        literal("FLUX_READY", true),
                        literal("OUTPUT_RESTORED", true),
                    ],
                ),
                observation(
                    "POS-2",
                    TransitionArmIR::AppliedSuccess,
                    vec![
                        literal("FLUX_READY", true),
                        literal("OUTPUT_RESTORED", false),
                    ],
                    vec![
                        literal("FLUX_READY", true),
                        literal("OUTPUT_RESTORED", true),
                    ],
                ),
                observation(
                    "CONTROL-1",
                    TransitionArmIR::NoActionControl,
                    vec![
                        literal("FLUX_READY", true),
                        literal("OUTPUT_RESTORED", false),
                    ],
                    vec![
                        literal("FLUX_READY", true),
                        literal("OUTPUT_RESTORED", false),
                    ],
                ),
            ],
            minimum_positive_support: 2,
            minimum_confidence_millis: 700,
        }
    }

    #[test]
    fn fresh_english_and_korean_names_compile_from_the_same_observations() {
        for statement in [
            "When flux-ready holds, normalization makes the output restored.",
            "플럭스 준비 상태에서 정규화를 하면 출력 복원이 된다.",
        ] {
            let result = MechanismInductionEngine
                .compile(&request(statement))
                .unwrap();
            assert_eq!(
                result.disposition,
                MechanismInductionDispositionIR::Compiled
            );
            let knowledge = result.knowledge.unwrap();
            assert_eq!(
                knowledge.mechanism.prerequisites,
                [literal("FLUX_READY", true)]
            );
            assert_eq!(
                knowledge.mechanism.effects,
                [literal("OUTPUT_RESTORED", true)]
            );
            assert_eq!(result.text_only_authority_events, 0);
        }
    }

    #[test]
    fn text_without_repeated_observations_never_becomes_knowledge() {
        let mut request = request("flux-ready causes output restored");
        request.observations.truncate(1);
        let result = MechanismInductionEngine.compile(&request).unwrap();
        assert_eq!(
            result.disposition,
            MechanismInductionDispositionIR::PublicInformationInsufficient
        );
        assert!(result.knowledge.is_none());
    }

    #[test]
    fn effect_seen_in_no_action_control_is_rejected_as_noncausal() {
        let mut request = request("flux-ready causes output restored");
        request.observations[2].after = vec![
            literal("FLUX_READY", true),
            literal("OUTPUT_RESTORED", true),
        ];
        let result = MechanismInductionEngine.compile(&request).unwrap();
        assert_eq!(
            result.disposition,
            MechanismInductionDispositionIR::NonCausalCorrelation
        );
        assert!(result.knowledge.is_none());
        assert_eq!(result.rejected_effect_proposition_ids, ["OUTPUT_RESTORED"]);
    }

    #[test]
    fn contradictory_success_transitions_remain_conflicting() {
        let mut request = request("flux-ready causes output restored");
        request.observations[1].after = vec![
            literal("FLUX_READY", true),
            literal("OUTPUT_RESTORED", false),
        ];
        let result = MechanismInductionEngine.compile(&request).unwrap();
        assert_eq!(
            result.disposition,
            MechanismInductionDispositionIR::ConflictingObservations
        );
        assert!(result.knowledge.is_none());
    }

    #[test]
    fn reverse_causal_language_binds_roles_without_name_specific_rules() {
        let result = MechanismInductionEngine
            .compile(&request(
                "The output restored because of the flux-ready condition.",
            ))
            .unwrap();
        assert_eq!(
            result.disposition,
            MechanismInductionDispositionIR::Compiled
        );
        let knowledge = result.knowledge.unwrap();
        assert_eq!(
            knowledge.mechanism.prerequisites,
            [literal("FLUX_READY", true)]
        );
        assert_eq!(
            knowledge.mechanism.effects,
            [literal("OUTPUT_RESTORED", true)]
        );
    }

    #[test]
    fn language_direction_that_disagrees_with_observations_is_not_rewritten() {
        let result = MechanismInductionEngine
            .compile(&request("output restored causes flux-ready"))
            .unwrap();
        assert_eq!(
            result.disposition,
            MechanismInductionDispositionIR::AmbiguousLanguageBinding
        );
        assert!(result.knowledge.is_none());
    }

    #[test]
    fn ambiguous_alias_ownership_fails_closed() {
        let mut request = request("flux-ready causes output restored");
        request.proposition_lexicon.push(PropositionLexemeIR {
            proposition_id: "ANOTHER_STATE".to_string(),
            aliases: vec!["flux-ready".to_string()],
        });
        assert_eq!(
            MechanismInductionEngine.compile(&request),
            Err(MechanismInductionError::InvalidRequest)
        );
    }

    #[test]
    fn irreversible_or_kind_mismatched_authority_is_never_induced() {
        let mut irreversible_request = request("flux-ready causes output restored");
        irreversible_request.authority = ActionAuthorityIR::IrreversibleMutation;
        irreversible_request.reversible = false;
        irreversible_request.recovery_reference = None;
        assert_eq!(
            MechanismInductionEngine.compile(&irreversible_request),
            Err(MechanismInductionError::InvalidRequest)
        );

        let mut mismatched_request = request("flux-ready causes output restored");
        mismatched_request.kind = MechanismKindIR::Inference;
        assert_eq!(
            MechanismInductionEngine.compile(&mismatched_request),
            Err(MechanismInductionError::InvalidRequest)
        );
    }
}
