use std::collections::BTreeSet;

use dockable_semantic_core::PlanIntentIR;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::compositional_semantics::{
    CompositionalSemanticAnalyzer, PredicateLexemeIR, PREDICATE_LEXEME_SCHEMA,
};
use crate::language_knowledge::LanguageCodeIR;

pub const DEFINITION_GROUNDING_SCHEMA: &str = "B_CORE_DEFINITION_GROUNDING_IR_1";
pub const PREDICATE_ALIAS_BINDING_SCHEMA: &str = "B_CORE_PREDICATE_ALIAS_BINDING_IR_1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DefinitionGroundingDispositionIR {
    NoDefinition,
    Bound,
    ConflictRejected,
    NonAssertedRejected,
    AmbiguousRejected,
    UnresolvedRejected,
    InvalidAliasRejected,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PredicateAliasBindingIR {
    pub schema: String,
    pub alias_id: String,
    pub alias_surface: String,
    pub alias_language: LanguageCodeIR,
    pub canonical_predicate: String,
    pub intent_hint: PlanIntentIR,
    pub definition_surface: String,
    pub source_turn: u64,
    pub provenance_sha256: String,
    pub semantic_payload_sha256: String,
    pub semantic_authority: bool,
    pub external_action_execution_authorized: bool,
    pub binding_sha256: String,
}

impl PredicateAliasBindingIR {
    pub fn validate(&self) -> bool {
        self.schema == PREDICATE_ALIAS_BINDING_SCHEMA
            && valid_alias(&self.alias_surface)
            && !self.canonical_predicate.trim().is_empty()
            && !self.definition_surface.trim().is_empty()
            && self.provenance_sha256.len() == 64
            && self.semantic_payload_sha256
                == semantic_payload_hash(&self.canonical_predicate, self.intent_hint)
            && !self.semantic_authority
            && !self.external_action_execution_authorized
            && self.binding_sha256 == binding_hash(self)
    }

    pub fn predicate_lexeme(&self) -> PredicateLexemeIR {
        PredicateLexemeIR {
            schema: PREDICATE_LEXEME_SCHEMA.to_string(),
            predicate_id: self.alias_id.clone(),
            language: self.alias_language,
            surface_forms: vec![self.alias_surface.clone()],
            canonical_predicate: self.canonical_predicate.clone(),
            intent_hint: self.intent_hint,
            definition: self.definition_surface.clone(),
            confidence_millis: 900,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DefinitionGroundingIR {
    pub schema: String,
    pub disposition: DefinitionGroundingDispositionIR,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub binding: Option<PredicateAliasBindingIR>,
    #[serde(default)]
    pub rejection_reasons: Vec<String>,
    pub lexical_store_changed: bool,
    pub semantic_payload_mutated: bool,
    pub semantic_authority: bool,
    pub external_action_execution_authorized: bool,
    pub grounding_sha256: String,
}

impl DefinitionGroundingIR {
    pub fn no_definition() -> Self {
        Self::sealed(
            DefinitionGroundingDispositionIR::NoDefinition,
            None,
            Vec::new(),
            false,
        )
    }

    pub fn consumes_turn(&self) -> bool {
        self.disposition != DefinitionGroundingDispositionIR::NoDefinition
    }

    pub fn validate(&self) -> bool {
        let shape = match self.disposition {
            DefinitionGroundingDispositionIR::NoDefinition => {
                self.binding.is_none()
                    && self.rejection_reasons.is_empty()
                    && !self.lexical_store_changed
            }
            DefinitionGroundingDispositionIR::Bound => {
                self.binding
                    .as_ref()
                    .is_some_and(PredicateAliasBindingIR::validate)
                    && self.rejection_reasons.is_empty()
            }
            DefinitionGroundingDispositionIR::ConflictRejected
            | DefinitionGroundingDispositionIR::NonAssertedRejected
            | DefinitionGroundingDispositionIR::AmbiguousRejected
            | DefinitionGroundingDispositionIR::UnresolvedRejected
            | DefinitionGroundingDispositionIR::InvalidAliasRejected => {
                self.binding.is_none()
                    && !self.rejection_reasons.is_empty()
                    && !self.lexical_store_changed
            }
        };
        self.schema == DEFINITION_GROUNDING_SCHEMA
            && shape
            && !self.semantic_payload_mutated
            && !self.semantic_authority
            && !self.external_action_execution_authorized
            && self.grounding_sha256 == grounding_hash(self)
    }

    fn sealed(
        disposition: DefinitionGroundingDispositionIR,
        binding: Option<PredicateAliasBindingIR>,
        mut rejection_reasons: Vec<String>,
        lexical_store_changed: bool,
    ) -> Self {
        rejection_reasons.sort();
        rejection_reasons.dedup();
        let mut result = Self {
            schema: DEFINITION_GROUNDING_SCHEMA.to_string(),
            disposition,
            binding,
            rejection_reasons,
            lexical_store_changed,
            semantic_payload_mutated: false,
            semantic_authority: false,
            external_action_execution_authorized: false,
            grounding_sha256: String::new(),
        };
        result.grounding_sha256 = grounding_hash(&result);
        result
    }
}

impl Default for DefinitionGroundingIR {
    fn default() -> Self {
        Self::no_definition()
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct DefinitionGrounder;

impl DefinitionGrounder {
    pub fn ground(
        &self,
        text: &str,
        turn_index: u64,
        learned_predicates: &[PredicateLexemeIR],
    ) -> DefinitionGroundingIR {
        let Some(candidate) = definition_candidate(text) else {
            return DefinitionGroundingIR::no_definition();
        };
        if candidate.non_asserted {
            return DefinitionGroundingIR::sealed(
                DefinitionGroundingDispositionIR::NonAssertedRejected,
                None,
                vec!["DEFINITION_NOT_ASSERTED_BY_CURRENT_USER".to_string()],
                false,
            );
        }
        if !valid_alias(&candidate.alias) {
            return DefinitionGroundingIR::sealed(
                DefinitionGroundingDispositionIR::InvalidAliasRejected,
                None,
                vec!["INVALID_ALIAS_SURFACE".to_string()],
                false,
            );
        }

        let analysis = CompositionalSemanticAnalyzer
            .analyze_with_predicates(&candidate.definition, learned_predicates);
        let meanings = analysis
            .frames
            .iter()
            .map(|frame| (frame.canonical_predicate.clone(), frame.intent_hint))
            .collect::<BTreeSet<_>>();
        if meanings.is_empty() {
            return DefinitionGroundingIR::sealed(
                DefinitionGroundingDispositionIR::UnresolvedRejected,
                None,
                vec!["DEFINITION_HAS_NO_GROUNDED_PREDICATE".to_string()],
                false,
            );
        }
        if meanings.len() != 1 {
            return DefinitionGroundingIR::sealed(
                DefinitionGroundingDispositionIR::AmbiguousRejected,
                None,
                vec!["DEFINITION_HAS_MULTIPLE_PREDICATES".to_string()],
                false,
            );
        }
        let (canonical_predicate, intent_hint) = meanings.into_iter().next().expect("one meaning");
        let normalized_alias = normalize_alias(&candidate.alias);

        if let Some((known_canonical, known_intent)) = builtin_alias_meaning(&normalized_alias) {
            if known_canonical != canonical_predicate || known_intent != intent_hint {
                return DefinitionGroundingIR::sealed(
                    DefinitionGroundingDispositionIR::ConflictRejected,
                    None,
                    vec!["ALIAS_ALREADY_OWNED_BY_BUILTIN_PREDICATE".to_string()],
                    false,
                );
            }
        }

        let existing = learned_predicates.iter().find(|predicate| {
            predicate
                .surface_forms
                .iter()
                .any(|surface| normalize_alias(surface) == normalized_alias)
        });
        if let Some(existing) = existing {
            if existing.canonical_predicate != canonical_predicate
                || existing.intent_hint != intent_hint
            {
                return DefinitionGroundingIR::sealed(
                    DefinitionGroundingDispositionIR::ConflictRejected,
                    None,
                    vec!["ALIAS_ALREADY_BOUND_TO_DIFFERENT_SEMANTICS".to_string()],
                    false,
                );
            }
        }

        let binding = build_binding(
            &normalized_alias,
            &candidate.definition,
            &canonical_predicate,
            intent_hint,
            turn_index,
            text,
        );
        DefinitionGroundingIR::sealed(
            DefinitionGroundingDispositionIR::Bound,
            Some(binding),
            Vec::new(),
            existing.is_none() && builtin_alias_meaning(&normalized_alias).is_none(),
        )
    }
}

#[derive(Debug)]
struct DefinitionCandidate {
    alias: String,
    definition: String,
    non_asserted: bool,
}

fn definition_candidate(text: &str) -> Option<DefinitionCandidate> {
    let trimmed = text.trim();
    let lower = trimmed.to_lowercase();
    if lower.starts_with("i meant ")
        || lower.contains(" i meant ")
        || lower.contains(" was meant ")
        || lower.contains(" meant for ")
    {
        return None;
    }
    let english_marker = lower
        .find(" to mean ")
        .map(|index| (index, " to mean ".len()))
        .or_else(|| lower.find(" means ").map(|index| (index, " means ".len())))
        .or_else(|| lower.find(" meant ").map(|index| (index, " meant ".len())));
    let korean_marker = korean_definition_marker(trimmed);
    if english_marker.is_none() && korean_marker.is_none() {
        return None;
    }

    let non_asserted = is_non_asserted_definition(trimmed, &lower);
    let alias = extract_first_quoted(trimmed).or_else(|| {
        english_marker
            .map(|(index, _)| extract_unquoted_english_alias(&lower[..index]))
            .or_else(|| korean_marker.map(|index| extract_unquoted_korean_alias(&trimmed[..index])))
    })?;
    let definition = if let Some((index, marker_len)) = english_marker {
        trimmed[index + marker_len..]
            .trim_matches(|character: char| {
                character.is_whitespace() || ".!?;:".contains(character)
            })
            .to_string()
    } else {
        let alias_end = find_alias_end(trimmed, &alias).unwrap_or(0);
        trimmed[alias_end..]
            .trim_matches(|character: char| {
                character.is_whitespace() || "'\"‘’“”.,!?;:".contains(character)
            })
            .to_string()
    };
    Some(DefinitionCandidate {
        alias,
        definition,
        non_asserted,
    })
}

/// Locate the lexical noun `뜻` only when it participates in a definition
/// construction.  A raw substring search confuses it with a syllable sequence
/// inside unrelated words such as `따뜻한`.  Korean particles and copular
/// morphology provide the structural right boundary without depending on a
/// completed sentence template.
fn korean_definition_marker(text: &str) -> Option<usize> {
    text.match_indices('뜻').find_map(|(index, marker)| {
        let after = &text[index + marker.len()..];
        let grammatical_continuation = after.is_empty()
            || after.chars().next().is_some_and(|character| {
                character.is_whitespace() || ".,!?;:'\"”’".contains(character)
            })
            || [
                "이",
                "은",
                "을",
                "으로",
                "이라고",
                "이라는",
                "인",
                "만",
                "도",
            ]
            .iter()
            .any(|suffix| after.starts_with(suffix));
        grammatical_continuation.then_some(index)
    })
}

fn is_non_asserted_definition(text: &str, lower: &str) -> bool {
    let question = text.trim_end().ends_with('?');
    let hypothetical = lower.starts_with("if ")
        || lower.contains(" if ")
        || text.contains("만약")
        || text.contains("라면")
        || text.contains("이라면");
    let negated = lower.contains("does not mean")
        || lower.contains("doesn't mean")
        || text.contains("뜻이 아니")
        || text.contains("의미하지 않");
    let reported = lower.contains(" said ")
        || lower.starts_with("said ")
        || lower.contains(" reported ")
        || lower.contains(" claimed ")
        || text.contains("말했")
        || text.contains("말하") && text.contains("라고")
        || text.contains("주장") && text.contains("라고");
    question || hypothetical || negated || reported
}

fn extract_first_quoted(text: &str) -> Option<String> {
    const PAIRS: &[(char, char)] = &[('"', '"'), ('\'', '\''), ('‘', '’'), ('“', '”')];
    for (open, close) in PAIRS {
        if let Some(start) = text.find(*open) {
            let content_start = start + open.len_utf8();
            if let Some(relative_end) = text[content_start..].find(*close) {
                let candidate = text[content_start..content_start + relative_end].trim();
                if !candidate.is_empty() {
                    return Some(candidate.to_string());
                }
            }
        }
    }
    None
}

fn extract_unquoted_english_alias(prefix: &str) -> String {
    prefix
        .split(|character: char| character.is_whitespace() || ",;:".contains(character))
        .rfind(|token| !token.is_empty())
        .unwrap_or_default()
        .trim_matches(|character: char| "'\"‘’“”".contains(character))
        .to_string()
}

fn extract_unquoted_korean_alias(prefix: &str) -> String {
    let tail = prefix
        .split(|character: char| character.is_whitespace() || ",;:".contains(character))
        .rfind(|token| !token.is_empty())
        .unwrap_or_default()
        .trim_matches(|character: char| "'\"‘’“”".contains(character));
    for particle in ["이라는", "라는", "은", "는", "을", "를"] {
        if let Some(alias) = tail.strip_suffix(particle) {
            return alias.to_string();
        }
    }
    tail.to_string()
}

fn find_alias_end(text: &str, alias: &str) -> Option<usize> {
    text.find(alias).map(|start| start + alias.len())
}

fn valid_alias(alias: &str) -> bool {
    let trimmed = alias.trim();
    !trimmed.is_empty()
        && trimmed.chars().count() <= 32
        && trimmed.split_whitespace().count() <= 3
        && trimmed
            .chars()
            .all(|character| character.is_alphanumeric() || character == '_' || character == '-')
}

fn normalize_alias(alias: &str) -> String {
    alias.trim().to_lowercase()
}

fn builtin_alias_meaning(alias: &str) -> Option<(String, PlanIntentIR)> {
    let analysis = CompositionalSemanticAnalyzer.analyze(alias);
    let meanings = analysis
        .frames
        .into_iter()
        .map(|frame| (frame.canonical_predicate, frame.intent_hint))
        .collect::<BTreeSet<_>>();
    (meanings.len() == 1)
        .then(|| meanings.into_iter().next())
        .flatten()
}

fn build_binding(
    alias: &str,
    definition: &str,
    canonical_predicate: &str,
    intent_hint: PlanIntentIR,
    source_turn: u64,
    source_text: &str,
) -> PredicateAliasBindingIR {
    let alias_language = if alias.chars().any(|character| {
        ('\u{ac00}'..='\u{d7a3}').contains(&character)
            || ('\u{3131}'..='\u{318e}').contains(&character)
    }) {
        LanguageCodeIR::Korean
    } else {
        LanguageCodeIR::English
    };
    let alias_digest = format!(
        "{:x}",
        Sha256::digest(format!("{alias_language:?}:{alias}"))
    );
    let mut binding = PredicateAliasBindingIR {
        schema: PREDICATE_ALIAS_BINDING_SCHEMA.to_string(),
        alias_id: format!("P-USER-ALIAS-{}", &alias_digest[..24]),
        alias_surface: alias.to_string(),
        alias_language,
        canonical_predicate: canonical_predicate.to_string(),
        intent_hint,
        definition_surface: definition.trim().to_string(),
        source_turn,
        provenance_sha256: format!("{:x}", Sha256::digest(source_text.as_bytes())),
        semantic_payload_sha256: semantic_payload_hash(canonical_predicate, intent_hint),
        semantic_authority: false,
        external_action_execution_authorized: false,
        binding_sha256: String::new(),
    };
    binding.binding_sha256 = binding_hash(&binding);
    binding
}

fn semantic_payload_hash(canonical_predicate: &str, intent_hint: PlanIntentIR) -> String {
    let bytes =
        serde_json::to_vec(&(canonical_predicate, intent_hint)).expect("serializable payload");
    format!("{:x}", Sha256::digest(bytes))
}

fn binding_hash(binding: &PredicateAliasBindingIR) -> String {
    let bytes = serde_json::to_vec(&(
        binding.schema.as_str(),
        binding.alias_id.as_str(),
        binding.alias_surface.as_str(),
        binding.alias_language,
        binding.canonical_predicate.as_str(),
        binding.intent_hint,
        binding.definition_surface.as_str(),
        binding.source_turn,
        binding.provenance_sha256.as_str(),
        binding.semantic_payload_sha256.as_str(),
        binding.semantic_authority,
        binding.external_action_execution_authorized,
    ))
    .expect("serializable binding");
    format!("{:x}", Sha256::digest(bytes))
}

fn grounding_hash(grounding: &DefinitionGroundingIR) -> String {
    let bytes = serde_json::to_vec(&(
        grounding.schema.as_str(),
        grounding.disposition,
        &grounding.binding,
        &grounding.rejection_reasons,
        grounding.lexical_store_changed,
        grounding.semantic_payload_mutated,
        grounding.semantic_authority,
        grounding.external_action_execution_authorized,
    ))
    .expect("serializable grounding");
    format!("{:x}", Sha256::digest(bytes))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ground(text: &str) -> DefinitionGroundingIR {
        DefinitionGrounder.ground(text, 1, &[])
    }

    #[test]
    fn english_definition_binds_only_to_existing_semantics() {
        let result = ground("In this conversation, \"nexel\" means inspect.");
        assert_eq!(result.disposition, DefinitionGroundingDispositionIR::Bound);
        let binding = result.binding.expect("binding");
        assert_eq!(binding.alias_surface, "nexel");
        assert_eq!(binding.canonical_predicate, "INVESTIGATE");
        assert!(binding.validate());
    }

    #[test]
    fn korean_definition_binds_only_to_existing_semantics() {
        let result = ground("이 대화에서 \"무루\"는 검사하라는 뜻이야.");
        assert_eq!(result.disposition, DefinitionGroundingDispositionIR::Bound);
        assert_eq!(
            result.binding.expect("binding").canonical_predicate,
            "INVESTIGATE"
        );
    }

    #[test]
    fn lexical_substring_inside_korean_adjective_is_not_a_definition_marker() {
        let result = ground("파스타보다는 따뜻한 국물 있는 게 좋겠다.");
        assert_eq!(
            result.disposition,
            DefinitionGroundingDispositionIR::NoDefinition
        );
        assert!(!result.consumes_turn());
    }

    #[test]
    fn aliases_for_one_concept_share_semantic_payload_hash() {
        let ko = ground("\"무루\"는 검사하라는 뜻이야.").binding.expect("ko");
        let en = ground("\"nexel\" means inspect.").binding.expect("en");
        assert_eq!(ko.semantic_payload_sha256, en.semantic_payload_sha256);
        assert_ne!(ko.binding_sha256, en.binding_sha256);
    }

    #[test]
    fn learned_alias_can_define_a_second_alias_without_new_semantics() {
        let first = ground("\"nexel\" means inspect.").binding.expect("first");
        let lexeme = first.predicate_lexeme();
        let second = DefinitionGrounder.ground("\"sora\" means nexel.", 2, &[lexeme]);
        assert_eq!(second.disposition, DefinitionGroundingDispositionIR::Bound);
        assert_eq!(
            second.binding.expect("second").canonical_predicate,
            "INVESTIGATE"
        );
    }

    #[test]
    fn ambiguous_definition_is_rejected() {
        let result = ground("\"zorv\" means inspect or delete.");
        assert_eq!(
            result.disposition,
            DefinitionGroundingDispositionIR::AmbiguousRejected
        );
        assert!(result.binding.is_none());
    }

    #[test]
    fn reported_hypothetical_and_question_definitions_are_rejected() {
        for text in [
            "Alice said 'zorv means delete'.",
            "If \"zorv\" meant delete, would it help?",
            "\"zorv\" means delete?",
        ] {
            let result = ground(text);
            assert_ne!(
                result.disposition,
                DefinitionGroundingDispositionIR::Bound,
                "{text}"
            );
            assert!(result.binding.is_none());
        }
    }

    #[test]
    fn conflicting_alias_binding_fails_closed() {
        let first = ground("\"nexel\" means inspect.")
            .binding
            .expect("first")
            .predicate_lexeme();
        let conflict = DefinitionGrounder.ground("\"nexel\" means delete.", 2, &[first]);
        assert_eq!(
            conflict.disposition,
            DefinitionGroundingDispositionIR::ConflictRejected
        );
        assert!(conflict.binding.is_none());
    }

    #[test]
    fn grounding_and_binding_hashes_detect_tampering() {
        let mut result = ground("\"nexel\" means inspect.");
        assert!(result.validate());
        result.binding.as_mut().expect("binding").alias_surface = "changed".to_string();
        assert!(!result.validate());
    }
}
