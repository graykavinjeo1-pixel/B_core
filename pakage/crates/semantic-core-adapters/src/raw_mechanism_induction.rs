//! Automatic semantic normalization for causal observations.
//!
//! This adapter removes the requirement for callers to invent proposition
//! identifiers, lexicons, or `LiteralIR` state vectors. It accepts bounded raw
//! scalar state maps and a causal sentence, derives a deterministic typed
//! vocabulary, and delegates causal authority to the evidence-bound mechanism
//! induction engine. The language statement proposes bindings; repeated
//! transitions and controls still decide whether executable knowledge exists.

use std::collections::{BTreeMap, BTreeSet};

use dockable_semantic_core::{ActionAuthorityIR, LiteralIR, MechanismKindIR};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::mechanism_induction::{
    causal_clause_ranges, MechanismInductionEngine, MechanismInductionIR,
    MechanismInductionRequestIR, PropositionLexemeIR, StateTransitionObservationIR,
    TransitionArmIR, MECHANISM_INDUCTION_REQUEST_SCHEMA,
};

pub const RAW_MECHANISM_INDUCTION_REQUEST_SCHEMA: &str =
    "B_CORE_RAW_MECHANISM_INDUCTION_REQUEST_IR_1";
pub const RAW_MECHANISM_INDUCTION_SCHEMA: &str = "B_CORE_RAW_MECHANISM_INDUCTION_IR_1";
const MAX_RAW_FIELDS: usize = 32;
const MAX_RAW_OBSERVATIONS: usize = 128;
const MAX_CATEGORY_VALUES: usize = 16;
const MAX_DERIVED_PROPOSITIONS: usize = 64;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ObservedValueIR {
    Boolean(bool),
    Integer(i64),
    Text(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RawStateTransitionObservationIR {
    pub observation_id: String,
    pub arm: TransitionArmIR,
    pub before: BTreeMap<String, ObservedValueIR>,
    pub after: BTreeMap<String, ObservedValueIR>,
    pub reliability_millis: u16,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RawMechanismInductionRequestIR {
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
    pub observations: Vec<RawStateTransitionObservationIR>,
    pub minimum_positive_support: usize,
    pub minimum_confidence_millis: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CausalClauseRoleIR {
    Prerequisite,
    Effect,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AutoPropositionKindIR {
    BooleanState,
    CategoricalState,
    NumericIncreased,
    NumericDecreased,
    NumericGreaterThan,
    NumericGreaterOrEqual,
    NumericLessThan,
    NumericLessOrEqual,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AutoPropositionBindingIR {
    pub proposition_id: String,
    pub source_field: String,
    pub kind: AutoPropositionKindIR,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub category_value: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub comparison_value: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selected_role: Option<CausalClauseRoleIR>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RawMechanismInductionIR {
    pub schema: String,
    pub request_id: String,
    pub original_statement: String,
    pub automatically_derived_propositions: Vec<AutoPropositionBindingIR>,
    pub normalized_request: MechanismInductionRequestIR,
    pub induction: MechanismInductionIR,
    pub explicit_proposition_lexicon_entries: usize,
    pub external_model_calls: usize,
    pub normalization_sha256: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RawMechanismInductionError {
    InvalidSchema,
    InvalidRequest,
    InvalidObservation,
    InconsistentStateShape,
    UnsupportedValueType,
    AmbiguousLanguageBinding,
    DerivedVocabularyOverflow,
    EvidenceCompilerRejected,
}

#[derive(Debug, Default)]
pub struct RawMechanismInductionEngine;

impl RawMechanismInductionEngine {
    pub fn compile(
        &self,
        request: &RawMechanismInductionRequestIR,
    ) -> Result<RawMechanismInductionIR, RawMechanismInductionError> {
        validate_raw_request(request)?;
        let normalized_statement = request.natural_language_statement.to_lowercase();
        let (cause_range, effect_range) = causal_clause_ranges(&normalized_statement)
            .ok_or(RawMechanismInductionError::AmbiguousLanguageBinding)?;
        let cause_clause = &normalized_statement[cause_range];
        let effect_clause = &normalized_statement[effect_range];
        let models = derive_field_models(request)?;
        let positives = request
            .observations
            .iter()
            .filter(|observation| observation.arm == TransitionArmIR::AppliedSuccess)
            .collect::<Vec<_>>();

        let mut bindings = derive_base_bindings(&models);
        let mut prerequisites = BTreeSet::new();
        let mut effects = BTreeSet::new();
        bind_boolean_and_category_roles(
            &models,
            &positives,
            cause_clause,
            effect_clause,
            &mut prerequisites,
            &mut effects,
        )?;
        bind_numeric_roles(
            &models,
            &positives,
            cause_clause,
            effect_clause,
            &mut bindings,
            &mut prerequisites,
            &mut effects,
        )?;
        let distinct_binding_ids = bindings
            .iter()
            .map(|binding| &binding.proposition_id)
            .collect::<BTreeSet<_>>();
        if bindings.len() > MAX_DERIVED_PROPOSITIONS {
            return Err(RawMechanismInductionError::DerivedVocabularyOverflow);
        }
        if distinct_binding_ids.len() != bindings.len() {
            return Err(RawMechanismInductionError::AmbiguousLanguageBinding);
        }
        if prerequisites.is_empty() || effects.is_empty() {
            return Err(RawMechanismInductionError::AmbiguousLanguageBinding);
        }
        for binding in &mut bindings {
            binding.selected_role = if prerequisites.contains(&binding.proposition_id) {
                Some(CausalClauseRoleIR::Prerequisite)
            } else if effects.contains(&binding.proposition_id) {
                Some(CausalClauseRoleIR::Effect)
            } else {
                None
            };
        }

        let synthetic_statement = format!(
            "{} causes {}",
            prerequisites
                .iter()
                .cloned()
                .collect::<Vec<_>>()
                .join(" and "),
            effects.iter().cloned().collect::<Vec<_>>().join(" and ")
        );
        let proposition_lexicon = bindings
            .iter()
            .map(|binding| PropositionLexemeIR {
                proposition_id: binding.proposition_id.clone(),
                aliases: vec![binding.proposition_id.to_lowercase()],
            })
            .collect::<Vec<_>>();
        let observations = request
            .observations
            .iter()
            .map(|observation| normalize_observation(observation, &bindings))
            .collect::<Result<Vec<_>, _>>()?;
        let normalized_request = MechanismInductionRequestIR {
            schema: MECHANISM_INDUCTION_REQUEST_SCHEMA.to_string(),
            request_id: request.request_id.clone(),
            knowledge_id: request.knowledge_id.clone(),
            mechanism_id: request.mechanism_id.clone(),
            natural_language_statement: synthetic_statement,
            kind: request.kind,
            authority: request.authority,
            authorized: request.authorized,
            reversible: request.reversible,
            recovery_reference: request.recovery_reference.clone(),
            semantic_tags: request.semantic_tags.clone(),
            proposition_lexicon,
            observations,
            minimum_positive_support: request.minimum_positive_support,
            minimum_confidence_millis: request.minimum_confidence_millis,
        };
        let induction = MechanismInductionEngine
            .compile(&normalized_request)
            .map_err(|_| RawMechanismInductionError::EvidenceCompilerRejected)?;
        let mut result = RawMechanismInductionIR {
            schema: RAW_MECHANISM_INDUCTION_SCHEMA.to_string(),
            request_id: request.request_id.clone(),
            original_statement: request.natural_language_statement.clone(),
            automatically_derived_propositions: bindings,
            normalized_request,
            induction,
            explicit_proposition_lexicon_entries: 0,
            external_model_calls: 0,
            normalization_sha256: String::new(),
        };
        result.normalization_sha256 = normalization_hash(&result);
        Ok(result)
    }
}

#[derive(Debug, Clone)]
enum FieldKind {
    Boolean,
    Integer,
    Text(BTreeSet<String>),
}

#[derive(Debug, Clone)]
struct FieldModel {
    label: String,
    id: String,
    kind: FieldKind,
}

fn validate_raw_request(
    request: &RawMechanismInductionRequestIR,
) -> Result<(), RawMechanismInductionError> {
    if request.schema != RAW_MECHANISM_INDUCTION_REQUEST_SCHEMA {
        return Err(RawMechanismInductionError::InvalidSchema);
    }
    if request.natural_language_statement.trim().is_empty()
        || request.natural_language_statement.len() > 64 * 1024
        || request.observations.is_empty()
        || request.observations.len() > MAX_RAW_OBSERVATIONS
    {
        return Err(RawMechanismInductionError::InvalidRequest);
    }
    let Some(first) = request.observations.first() else {
        return Err(RawMechanismInductionError::InvalidRequest);
    };
    if first.before.is_empty() || first.before.len() > MAX_RAW_FIELDS {
        return Err(RawMechanismInductionError::InvalidObservation);
    }
    if first
        .before
        .keys()
        .any(|field| field.trim().is_empty() || field.len() > 128)
    {
        return Err(RawMechanismInductionError::InvalidObservation);
    }
    let expected_keys = first.before.keys().collect::<Vec<_>>();
    for observation in &request.observations {
        if observation.before.keys().ne(expected_keys.iter().copied())
            || observation.after.keys().ne(expected_keys.iter().copied())
        {
            return Err(RawMechanismInductionError::InconsistentStateShape);
        }
    }
    Ok(())
}

fn derive_field_models(
    request: &RawMechanismInductionRequestIR,
) -> Result<Vec<FieldModel>, RawMechanismInductionError> {
    let first = request
        .observations
        .first()
        .ok_or(RawMechanismInductionError::InvalidObservation)?;
    let models = first
        .before
        .keys()
        .map(|field| {
            let values = request
                .observations
                .iter()
                .flat_map(|observation| {
                    [observation.before.get(field), observation.after.get(field)]
                })
                .flatten()
                .collect::<Vec<_>>();
            let kind = match values.first() {
                Some(ObservedValueIR::Boolean(_))
                    if values
                        .iter()
                        .all(|value| matches!(value, ObservedValueIR::Boolean(_))) =>
                {
                    FieldKind::Boolean
                }
                Some(ObservedValueIR::Integer(_))
                    if values
                        .iter()
                        .all(|value| matches!(value, ObservedValueIR::Integer(_))) =>
                {
                    FieldKind::Integer
                }
                Some(ObservedValueIR::Text(_))
                    if values
                        .iter()
                        .all(|value| matches!(value, ObservedValueIR::Text(_))) =>
                {
                    let domain = values
                        .iter()
                        .filter_map(|value| match value {
                            ObservedValueIR::Text(value) => Some(value.trim().to_lowercase()),
                            _ => None,
                        })
                        .collect::<BTreeSet<_>>();
                    if domain.is_empty()
                        || domain.len() > MAX_CATEGORY_VALUES
                        || domain
                            .iter()
                            .any(|value| value.is_empty() || value.len() > 128)
                    {
                        return Err(RawMechanismInductionError::UnsupportedValueType);
                    }
                    FieldKind::Text(domain)
                }
                _ => return Err(RawMechanismInductionError::UnsupportedValueType),
            };
            Ok(FieldModel {
                label: field.clone(),
                id: identifier_component(field),
                kind,
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    if models
        .iter()
        .map(|model| &model.id)
        .collect::<BTreeSet<_>>()
        .len()
        != models.len()
    {
        return Err(RawMechanismInductionError::AmbiguousLanguageBinding);
    }
    Ok(models)
}

fn derive_base_bindings(models: &[FieldModel]) -> Vec<AutoPropositionBindingIR> {
    let mut bindings = Vec::new();
    for model in models {
        match &model.kind {
            FieldKind::Boolean => bindings.push(binding(
                format!("STATE::{}", model.id),
                model,
                AutoPropositionKindIR::BooleanState,
                None,
                None,
            )),
            FieldKind::Text(domain) => {
                for value in domain {
                    bindings.push(binding(
                        format!("STATE::{}::{}", model.id, identifier_component(value)),
                        model,
                        AutoPropositionKindIR::CategoricalState,
                        Some(value.clone()),
                        None,
                    ));
                }
            }
            FieldKind::Integer => {
                bindings.push(binding(
                    format!("DELTA::{}::INCREASED", model.id),
                    model,
                    AutoPropositionKindIR::NumericIncreased,
                    None,
                    None,
                ));
                bindings.push(binding(
                    format!("DELTA::{}::DECREASED", model.id),
                    model,
                    AutoPropositionKindIR::NumericDecreased,
                    None,
                    None,
                ));
            }
        }
    }
    bindings
}

fn binding(
    proposition_id: String,
    model: &FieldModel,
    kind: AutoPropositionKindIR,
    category_value: Option<String>,
    comparison_value: Option<i64>,
) -> AutoPropositionBindingIR {
    AutoPropositionBindingIR {
        proposition_id,
        source_field: model.label.clone(),
        kind,
        category_value,
        comparison_value,
        selected_role: None,
    }
}

#[allow(clippy::too_many_arguments)]
fn bind_boolean_and_category_roles(
    models: &[FieldModel],
    positives: &[&RawStateTransitionObservationIR],
    cause_clause: &str,
    effect_clause: &str,
    prerequisites: &mut BTreeSet<String>,
    effects: &mut BTreeSet<String>,
) -> Result<(), RawMechanismInductionError> {
    for model in models {
        match &model.kind {
            FieldKind::Boolean => {
                let proposition_id = format!("STATE::{}", model.id);
                if field_is_mentioned(cause_clause, &model.label) {
                    ensure_boolean_clause_matches_observations(
                        cause_clause,
                        common_boolean_value(positives, &model.label, true),
                    )?;
                    prerequisites.insert(proposition_id.clone());
                }
                if field_is_mentioned(effect_clause, &model.label) {
                    ensure_boolean_clause_matches_observations(
                        effect_clause,
                        common_boolean_value(positives, &model.label, false),
                    )?;
                    effects.insert(proposition_id);
                }
            }
            FieldKind::Text(_) => {
                if let Some(value) = common_text_value(positives, &model.label, true) {
                    if field_value_is_mentioned(cause_clause, model, &value, models) {
                        prerequisites.insert(format!(
                            "STATE::{}::{}",
                            model.id,
                            identifier_component(&value)
                        ));
                    }
                }
                let before = common_text_value(positives, &model.label, true);
                let after = common_text_value(positives, &model.label, false);
                if let Some(value) = after.filter(|value| Some(value) != before.as_ref()) {
                    if field_value_is_mentioned(effect_clause, model, &value, models) {
                        effects.insert(format!(
                            "STATE::{}::{}",
                            model.id,
                            identifier_component(&value)
                        ));
                    }
                }
            }
            FieldKind::Integer => {}
        }
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn bind_numeric_roles(
    models: &[FieldModel],
    positives: &[&RawStateTransitionObservationIR],
    cause_clause: &str,
    effect_clause: &str,
    bindings: &mut Vec<AutoPropositionBindingIR>,
    prerequisites: &mut BTreeSet<String>,
    effects: &mut BTreeSet<String>,
) -> Result<(), RawMechanismInductionError> {
    for model in models {
        if !matches!(model.kind, FieldKind::Integer) {
            continue;
        }
        if field_is_mentioned(cause_clause, &model.label) {
            if let Some((kind, threshold)) = numeric_comparison(cause_clause) {
                let proposition_id = format!(
                    "STATE::{}::{}::{}",
                    model.id,
                    comparison_token(kind),
                    signed_component(threshold)
                );
                bindings.push(binding(
                    proposition_id.clone(),
                    model,
                    kind,
                    None,
                    Some(threshold),
                ));
                prerequisites.insert(proposition_id);
            }
        }
        if field_is_mentioned(effect_clause, &model.label) {
            let direction = common_numeric_direction(positives, &model.label);
            match (direction, direction_from_text(effect_clause)) {
                (Some(std::cmp::Ordering::Greater), Some(std::cmp::Ordering::Greater)) => {
                    effects.insert(format!("DELTA::{}::INCREASED", model.id));
                }
                (Some(std::cmp::Ordering::Less), Some(std::cmp::Ordering::Less)) => {
                    effects.insert(format!("DELTA::{}::DECREASED", model.id));
                }
                (Some(_), Some(_)) => {
                    return Err(RawMechanismInductionError::AmbiguousLanguageBinding);
                }
                _ => {}
            }
        }
    }
    Ok(())
}

fn normalize_observation(
    observation: &RawStateTransitionObservationIR,
    bindings: &[AutoPropositionBindingIR],
) -> Result<StateTransitionObservationIR, RawMechanismInductionError> {
    let mut before = Vec::with_capacity(bindings.len());
    let mut after = Vec::with_capacity(bindings.len());
    for binding in bindings {
        let before_value = observation
            .before
            .get(&binding.source_field)
            .ok_or(RawMechanismInductionError::InconsistentStateShape)?;
        let after_value = observation
            .after
            .get(&binding.source_field)
            .ok_or(RawMechanismInductionError::InconsistentStateShape)?;
        let (before_truth, after_truth) = binding_truth(binding, before_value, after_value)?;
        before.push(LiteralIR {
            proposition_id: binding.proposition_id.clone(),
            value: before_truth,
        });
        after.push(LiteralIR {
            proposition_id: binding.proposition_id.clone(),
            value: after_truth,
        });
    }
    before.sort();
    after.sort();
    Ok(StateTransitionObservationIR {
        observation_id: observation.observation_id.clone(),
        arm: observation.arm,
        before,
        after,
        reliability_millis: observation.reliability_millis,
        evidence_refs: observation.evidence_refs.clone(),
    })
}

fn binding_truth(
    binding: &AutoPropositionBindingIR,
    before: &ObservedValueIR,
    after: &ObservedValueIR,
) -> Result<(bool, bool), RawMechanismInductionError> {
    match (binding.kind, before, after) {
        (
            AutoPropositionKindIR::BooleanState,
            ObservedValueIR::Boolean(before),
            ObservedValueIR::Boolean(after),
        ) => Ok((*before, *after)),
        (
            AutoPropositionKindIR::CategoricalState,
            ObservedValueIR::Text(before),
            ObservedValueIR::Text(after),
        ) => {
            let category = binding.category_value.as_deref().unwrap_or_default();
            Ok((
                before.trim().eq_ignore_ascii_case(category),
                after.trim().eq_ignore_ascii_case(category),
            ))
        }
        (
            AutoPropositionKindIR::NumericIncreased,
            ObservedValueIR::Integer(before),
            ObservedValueIR::Integer(after),
        ) => Ok((false, after > before)),
        (
            AutoPropositionKindIR::NumericDecreased,
            ObservedValueIR::Integer(before),
            ObservedValueIR::Integer(after),
        ) => Ok((false, after < before)),
        (kind, ObservedValueIR::Integer(before), ObservedValueIR::Integer(after))
            if is_comparison(kind) =>
        {
            let threshold = binding
                .comparison_value
                .ok_or(RawMechanismInductionError::InvalidRequest)?;
            Ok((
                compare_numeric(*before, kind, threshold),
                compare_numeric(*after, kind, threshold),
            ))
        }
        _ => Err(RawMechanismInductionError::UnsupportedValueType),
    }
}

fn common_text_value(
    positives: &[&RawStateTransitionObservationIR],
    field: &str,
    before: bool,
) -> Option<String> {
    let values = positives
        .iter()
        .filter_map(|observation| {
            let state = if before {
                &observation.before
            } else {
                &observation.after
            };
            match state.get(field) {
                Some(ObservedValueIR::Text(value)) => Some(value.trim().to_lowercase()),
                _ => None,
            }
        })
        .collect::<BTreeSet<_>>();
    (values.len() == 1)
        .then(|| values.into_iter().next())
        .flatten()
}

fn common_boolean_value(
    positives: &[&RawStateTransitionObservationIR],
    field: &str,
    before: bool,
) -> Option<bool> {
    let values = positives
        .iter()
        .filter_map(|observation| {
            let state = if before {
                &observation.before
            } else {
                &observation.after
            };
            match state.get(field) {
                Some(ObservedValueIR::Boolean(value)) => Some(*value),
                _ => None,
            }
        })
        .collect::<BTreeSet<_>>();
    (values.len() == 1)
        .then(|| values.into_iter().next())
        .flatten()
}

fn ensure_boolean_clause_matches_observations(
    clause: &str,
    observed: Option<bool>,
) -> Result<(), RawMechanismInductionError> {
    if let Some(expected) = boolean_value_from_text(clause) {
        if observed != Some(expected) {
            return Err(RawMechanismInductionError::AmbiguousLanguageBinding);
        }
    }
    Ok(())
}

fn boolean_value_from_text(clause: &str) -> Option<bool> {
    let compacted = compact(clause);
    let negative = ["false", "inactive", "disabled", "off", "not ready"]
        .iter()
        .any(|cue| contains_ascii_phrase(clause, cue))
        || ["비활성", "꺼짐", "거짓", "준비되지않"]
            .iter()
            .any(|cue| compacted.contains(cue));
    let positive = ["true", "active", "enabled", "on", "ready"]
        .iter()
        .any(|cue| contains_ascii_phrase(clause, cue))
        || ["활성", "켜짐", "참", "준비"]
            .iter()
            .any(|cue| compacted.contains(cue));
    if negative {
        Some(false)
    } else if positive {
        Some(true)
    } else {
        None
    }
}

fn common_numeric_direction(
    positives: &[&RawStateTransitionObservationIR],
    field: &str,
) -> Option<std::cmp::Ordering> {
    let directions = positives
        .iter()
        .filter_map(|observation| {
            match (observation.before.get(field), observation.after.get(field)) {
                (Some(ObservedValueIR::Integer(before)), Some(ObservedValueIR::Integer(after))) => {
                    Some(after.cmp(before))
                }
                _ => None,
            }
        })
        .filter(|direction| *direction != std::cmp::Ordering::Equal)
        .collect::<BTreeSet<_>>();
    (directions.len() == 1)
        .then(|| directions.into_iter().next())
        .flatten()
}

fn field_value_is_mentioned(
    clause: &str,
    model: &FieldModel,
    value: &str,
    models: &[FieldModel],
) -> bool {
    (field_is_mentioned(clause, &model.label) && phrase_is_mentioned(clause, value))
        || (unique_category_value(value, models) && phrase_is_mentioned(clause, value))
}

fn unique_category_value(value: &str, models: &[FieldModel]) -> bool {
    models
        .iter()
        .filter(|model| match &model.kind {
            FieldKind::Text(domain) => domain.contains(value),
            _ => false,
        })
        .count()
        == 1
}

fn field_is_mentioned(clause: &str, field: &str) -> bool {
    field_aliases(field)
        .into_iter()
        .any(|alias| phrase_is_mentioned(clause, &alias))
}

fn field_aliases(field: &str) -> Vec<String> {
    let humanized = field
        .replace(['_', '-', '.'], " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    let mut aliases = vec![field.to_lowercase(), humanized.clone()];
    let mut tokens = humanized.split_whitespace().collect::<Vec<_>>();
    if tokens.last().is_some_and(|token| {
        matches!(
            *token,
            "ms" | "us" | "ns" | "sec" | "seconds" | "count" | "pct" | "percent" | "ratio"
        )
    }) {
        tokens.pop();
        aliases.push(tokens.join(" "));
    }
    aliases
}

fn numeric_comparison(clause: &str) -> Option<(AutoPropositionKindIR, i64)> {
    let threshold = first_integer(clause)?;
    let normalized = clause.to_lowercase();
    let compacted = compact(clause);
    let kind = if normalized.contains(">=")
        || contains_ascii_phrase(&normalized, "at least")
        || compacted.contains("이상")
    {
        AutoPropositionKindIR::NumericGreaterOrEqual
    } else if normalized.contains('>')
        || contains_ascii_phrase(&normalized, "above")
        || contains_ascii_phrase(&normalized, "over")
        || contains_ascii_phrase(&normalized, "greater than")
        || compacted.contains("초과")
        || compacted.contains("보다크")
    {
        AutoPropositionKindIR::NumericGreaterThan
    } else if normalized.contains("<=")
        || contains_ascii_phrase(&normalized, "at most")
        || compacted.contains("이하")
    {
        AutoPropositionKindIR::NumericLessOrEqual
    } else if normalized.contains('<')
        || contains_ascii_phrase(&normalized, "below")
        || contains_ascii_phrase(&normalized, "under")
        || contains_ascii_phrase(&normalized, "less than")
        || compacted.contains("미만")
        || compacted.contains("보다작")
    {
        AutoPropositionKindIR::NumericLessThan
    } else {
        return None;
    };
    Some((kind, threshold))
}

fn direction_from_text(clause: &str) -> Option<std::cmp::Ordering> {
    let text = compact(clause);
    let increased = [
        "increase",
        "increases",
        "increased",
        "rise",
        "rises",
        "rose",
        "higher",
        "goes up",
    ]
    .iter()
    .any(|cue| contains_ascii_phrase(clause, cue))
        || ["증가", "상승", "높아", "늘어"]
            .iter()
            .any(|cue| text.contains(cue));
    let decreased = [
        "decrease",
        "decreases",
        "decreased",
        "fall",
        "falls",
        "fell",
        "lower",
        "goes down",
    ]
    .iter()
    .any(|cue| contains_ascii_phrase(clause, cue))
        || ["감소", "하락", "낮아", "줄어"]
            .iter()
            .any(|cue| text.contains(cue));
    match (increased, decreased) {
        (true, false) => Some(std::cmp::Ordering::Greater),
        (false, true) => Some(std::cmp::Ordering::Less),
        _ => None,
    }
}

fn first_integer(text: &str) -> Option<i64> {
    let mut token = String::new();
    let mut started = false;
    for character in text.chars() {
        if character.is_ascii_digit() || (!started && character == '-') {
            token.push(character);
            started = true;
        } else if started {
            if token != "-" {
                break;
            }
            token.clear();
            started = false;
        }
    }
    token.parse().ok()
}

fn compare_numeric(value: i64, kind: AutoPropositionKindIR, threshold: i64) -> bool {
    match kind {
        AutoPropositionKindIR::NumericGreaterThan => value > threshold,
        AutoPropositionKindIR::NumericGreaterOrEqual => value >= threshold,
        AutoPropositionKindIR::NumericLessThan => value < threshold,
        AutoPropositionKindIR::NumericLessOrEqual => value <= threshold,
        _ => false,
    }
}

fn is_comparison(kind: AutoPropositionKindIR) -> bool {
    matches!(
        kind,
        AutoPropositionKindIR::NumericGreaterThan
            | AutoPropositionKindIR::NumericGreaterOrEqual
            | AutoPropositionKindIR::NumericLessThan
            | AutoPropositionKindIR::NumericLessOrEqual
    )
}

fn comparison_token(kind: AutoPropositionKindIR) -> &'static str {
    match kind {
        AutoPropositionKindIR::NumericGreaterThan => "GT",
        AutoPropositionKindIR::NumericGreaterOrEqual => "GE",
        AutoPropositionKindIR::NumericLessThan => "LT",
        AutoPropositionKindIR::NumericLessOrEqual => "LE",
        _ => "INVALID",
    }
}

fn compact(value: &str) -> String {
    value
        .to_lowercase()
        .chars()
        .filter(|character| character.is_alphanumeric())
        .collect()
}

fn phrase_is_mentioned(clause: &str, phrase: &str) -> bool {
    if phrase.is_ascii() {
        contains_ascii_phrase(clause, phrase)
    } else {
        let phrase = compact(phrase);
        !phrase.is_empty() && compact(clause).contains(&phrase)
    }
}

fn contains_ascii_phrase(text: &str, phrase: &str) -> bool {
    let text_tokens = ascii_tokens(text);
    let phrase_tokens = ascii_tokens(phrase);
    !phrase_tokens.is_empty()
        && text_tokens
            .windows(phrase_tokens.len())
            .any(|window| window == phrase_tokens)
}

fn ascii_tokens(value: &str) -> Vec<String> {
    value
        .to_lowercase()
        .split(|character: char| !character.is_ascii_alphanumeric())
        .filter(|token| !token.is_empty())
        .map(str::to_string)
        .collect()
}

fn identifier_component(value: &str) -> String {
    let ascii = value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character.to_ascii_uppercase()
            } else {
                '_'
            }
        })
        .collect::<String>()
        .split('_')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("_");
    if !ascii.is_empty() {
        return ascii.chars().take(48).collect();
    }
    let digest = format!("{:x}", Sha256::digest(value.as_bytes()));
    format!("UNICODE_{}", &digest[..16])
}

fn signed_component(value: i64) -> String {
    if value < 0 {
        format!("NEG_{}", value.unsigned_abs())
    } else {
        value.to_string()
    }
}

fn normalization_hash(result: &RawMechanismInductionIR) -> String {
    let mut unsigned = result.clone();
    unsigned.normalization_sha256.clear();
    format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(&unsigned).unwrap_or_default())
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mechanism_induction::MechanismInductionDispositionIR;

    fn state(values: &[(&str, ObservedValueIR)]) -> BTreeMap<String, ObservedValueIR> {
        values
            .iter()
            .map(|(field, value)| ((*field).to_string(), value.clone()))
            .collect()
    }

    fn observation(
        id: &str,
        arm: TransitionArmIR,
        before: BTreeMap<String, ObservedValueIR>,
        after: BTreeMap<String, ObservedValueIR>,
    ) -> RawStateTransitionObservationIR {
        RawStateTransitionObservationIR {
            observation_id: id.to_string(),
            arm,
            before,
            after,
            reliability_millis: 950,
            evidence_refs: vec![format!("raw-test:{id}")],
        }
    }

    fn request(
        id: &str,
        statement: &str,
        observations: Vec<RawStateTransitionObservationIR>,
    ) -> RawMechanismInductionRequestIR {
        RawMechanismInductionRequestIR {
            schema: RAW_MECHANISM_INDUCTION_REQUEST_SCHEMA.to_string(),
            request_id: id.to_string(),
            knowledge_id: format!("K-{id}"),
            mechanism_id: format!("M-{id}"),
            natural_language_statement: statement.to_string(),
            kind: MechanismKindIR::Inference,
            authority: ActionAuthorityIR::InternalInference,
            authorized: true,
            reversible: true,
            recovery_reference: None,
            semantic_tags: vec!["raw-induction".to_string()],
            observations,
            minimum_positive_support: 2,
            minimum_confidence_millis: 700,
        }
    }

    #[test]
    fn boolean_states_compile_without_explicit_ids_lexicon_or_literals() {
        for (id, statement, cause, effect) in [
            (
                "RAW-BOOL-EN",
                "When cache_warm is true, service_ready becomes true.",
                "cache_warm",
                "service_ready",
            ),
            (
                "RAW-BOOL-KO",
                "캐시준비 상태이면 서비스준비가 활성화된다.",
                "캐시준비",
                "서비스준비",
            ),
            (
                "RAW-BOOL-EN-REVERSE",
                "Service_ready becomes true when cache_warm is true.",
                "cache_warm",
                "service_ready",
            ),
            (
                "RAW-BOOL-KO-BECAUSE",
                "캐시준비 때문에 서비스준비가 활성화된다.",
                "캐시준비",
                "서비스준비",
            ),
        ] {
            let before = state(&[
                (cause, ObservedValueIR::Boolean(true)),
                (effect, ObservedValueIR::Boolean(false)),
            ]);
            let success = state(&[
                (cause, ObservedValueIR::Boolean(true)),
                (effect, ObservedValueIR::Boolean(true)),
            ]);
            let result = RawMechanismInductionEngine
                .compile(&request(
                    id,
                    statement,
                    vec![
                        observation(
                            &format!("{id}-P1"),
                            TransitionArmIR::AppliedSuccess,
                            before.clone(),
                            success.clone(),
                        ),
                        observation(
                            &format!("{id}-P2"),
                            TransitionArmIR::AppliedSuccess,
                            before.clone(),
                            success,
                        ),
                        observation(
                            &format!("{id}-C"),
                            TransitionArmIR::NoActionControl,
                            before.clone(),
                            before,
                        ),
                    ],
                ))
                .unwrap();
            assert_eq!(
                result.induction.disposition,
                MechanismInductionDispositionIR::Compiled
            );
            assert_eq!(result.explicit_proposition_lexicon_entries, 0);
            assert_eq!(result.external_model_calls, 0);
            assert_eq!(
                result
                    .automatically_derived_propositions
                    .iter()
                    .filter(|binding| binding.selected_role.is_some())
                    .count(),
                2
            );
        }
    }

    #[test]
    fn categorical_precondition_and_postimage_are_induced_from_raw_values() {
        let before = state(&[
            ("mode", ObservedValueIR::Text("safe".to_string())),
            (
                "output_status",
                ObservedValueIR::Text("pending".to_string()),
            ),
        ]);
        let after = state(&[
            ("mode", ObservedValueIR::Text("safe".to_string())),
            (
                "output_status",
                ObservedValueIR::Text("restored".to_string()),
            ),
        ]);
        let result = RawMechanismInductionEngine
            .compile(&request(
                "RAW-CATEGORY",
                "safe mode causes output status restored",
                vec![
                    observation(
                        "CAT-P1",
                        TransitionArmIR::AppliedSuccess,
                        before.clone(),
                        after.clone(),
                    ),
                    observation(
                        "CAT-P2",
                        TransitionArmIR::AppliedSuccess,
                        before.clone(),
                        after,
                    ),
                    observation(
                        "CAT-C",
                        TransitionArmIR::NoActionControl,
                        before.clone(),
                        before,
                    ),
                ],
            ))
            .unwrap();
        assert_eq!(
            result.induction.disposition,
            MechanismInductionDispositionIR::Compiled
        );
        let knowledge = result.induction.knowledge.unwrap();
        assert!(knowledge.mechanism.prerequisites[0]
            .proposition_id
            .contains("MODE::SAFE"));
        assert!(knowledge.mechanism.effects[0]
            .proposition_id
            .contains("OUTPUT_STATUS::RESTORED"));
    }

    #[test]
    fn bilingual_numeric_threshold_and_direction_compile_from_values() {
        for (id, statement, temperature, latency) in [
            (
                "RAW-NUMERIC-EN",
                "When temperature is at least 80, latency falls.",
                "temperature",
                "latency_ms",
            ),
            (
                "RAW-NUMERIC-KO",
                "온도가 80 이상이면 지연시간이 감소한다.",
                "온도",
                "지연시간",
            ),
        ] {
            let before = state(&[
                (temperature, ObservedValueIR::Integer(90)),
                (latency, ObservedValueIR::Integer(120)),
            ]);
            let after = state(&[
                (temperature, ObservedValueIR::Integer(90)),
                (latency, ObservedValueIR::Integer(70)),
            ]);
            let result = RawMechanismInductionEngine
                .compile(&request(
                    id,
                    statement,
                    vec![
                        observation(
                            &format!("{id}-P1"),
                            TransitionArmIR::AppliedSuccess,
                            before.clone(),
                            after.clone(),
                        ),
                        observation(
                            &format!("{id}-P2"),
                            TransitionArmIR::AppliedSuccess,
                            before.clone(),
                            after,
                        ),
                        observation(
                            &format!("{id}-C"),
                            TransitionArmIR::NoActionControl,
                            before.clone(),
                            before,
                        ),
                    ],
                ))
                .unwrap();
            assert_eq!(
                result.induction.disposition,
                MechanismInductionDispositionIR::Compiled
            );
            assert!(result
                .automatically_derived_propositions
                .iter()
                .any(
                    |binding| binding.kind == AutoPropositionKindIR::NumericGreaterOrEqual
                        && binding.comparison_value == Some(80)
                        && binding.selected_role == Some(CausalClauseRoleIR::Prerequisite)
                ));
            assert!(result
                .automatically_derived_propositions
                .iter()
                .any(
                    |binding| binding.kind == AutoPropositionKindIR::NumericDecreased
                        && binding.selected_role == Some(CausalClauseRoleIR::Effect)
                ));
        }
    }

    #[test]
    fn missing_causal_structure_and_reversed_numeric_claim_fail_closed() {
        let before = state(&[
            ("temperature", ObservedValueIR::Integer(90)),
            ("latency", ObservedValueIR::Integer(120)),
        ]);
        let after = state(&[
            ("temperature", ObservedValueIR::Integer(90)),
            ("latency", ObservedValueIR::Integer(70)),
        ]);
        let observations = vec![
            observation(
                "BAD-P1",
                TransitionArmIR::AppliedSuccess,
                before.clone(),
                after.clone(),
            ),
            observation(
                "BAD-P2",
                TransitionArmIR::AppliedSuccess,
                before.clone(),
                after,
            ),
            observation(
                "BAD-C",
                TransitionArmIR::NoActionControl,
                before.clone(),
                before,
            ),
        ];
        assert_eq!(
            RawMechanismInductionEngine.compile(&request(
                "RAW-NO-CAUSE",
                "temperature 90 and latency 70",
                observations.clone(),
            )),
            Err(RawMechanismInductionError::AmbiguousLanguageBinding)
        );
        assert_eq!(
            RawMechanismInductionEngine.compile(&request(
                "RAW-WRONG-DIRECTION",
                "When temperature is above 80, latency increases.",
                observations,
            )),
            Err(RawMechanismInductionError::AmbiguousLanguageBinding)
        );
    }

    #[test]
    fn explicit_boolean_meaning_cannot_be_rewritten_by_opposite_observations() {
        let before = state(&[
            ("gate_open", ObservedValueIR::Boolean(false)),
            ("alarm_active", ObservedValueIR::Boolean(false)),
        ]);
        let after = state(&[
            ("gate_open", ObservedValueIR::Boolean(false)),
            ("alarm_active", ObservedValueIR::Boolean(true)),
        ]);
        let observations = vec![
            observation(
                "BOOL-MISMATCH-P1",
                TransitionArmIR::AppliedSuccess,
                before.clone(),
                after.clone(),
            ),
            observation(
                "BOOL-MISMATCH-P2",
                TransitionArmIR::AppliedSuccess,
                before.clone(),
                after,
            ),
            observation(
                "BOOL-MISMATCH-C",
                TransitionArmIR::NoActionControl,
                before.clone(),
                before,
            ),
        ];
        assert_eq!(
            RawMechanismInductionEngine.compile(&request(
                "RAW-BOOL-MISMATCH",
                "When gate_open is true, alarm_active becomes true.",
                observations,
            )),
            Err(RawMechanismInductionError::AmbiguousLanguageBinding)
        );
    }
}
