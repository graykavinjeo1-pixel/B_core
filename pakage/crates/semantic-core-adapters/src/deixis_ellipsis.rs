//! Typed, dialogue-local deixis and argument ellipsis resolution.
//!
//! This adapter only makes an already selected discourse focus explicit. It
//! never establishes facts, changes semantic concepts, or grants permission to
//! execute the resulting request.

use serde::{Deserialize, Serialize};

pub const TYPED_DEIXIS_ELLIPSIS_SCHEMA: &str = "B_CORE_TYPED_DEIXIS_ELLIPSIS_IR_1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TypedDeixisEllipsisKindIR {
    PossessiveFocusReference,
    DemonstrativeFocusReference,
    ZeroArgumentEllipsis,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TypedDeixisEllipsisResolutionIR {
    pub schema: String,
    pub kind: TypedDeixisEllipsisKindIR,
    pub original_text: String,
    pub resolved_text: String,
    pub source_surface: String,
    pub antecedent_surface: String,
    pub confidence_millis: u16,
    pub evidence: Vec<String>,
    pub semantic_authority: bool,
    pub external_execution_authorized: bool,
}

impl TypedDeixisEllipsisResolutionIR {
    pub fn validate(&self) -> bool {
        self.schema == TYPED_DEIXIS_ELLIPSIS_SCHEMA
            && !self.original_text.trim().is_empty()
            && self.resolved_text != self.original_text
            && !self.source_surface.trim().is_empty()
            && !self.antecedent_surface.trim().is_empty()
            && self.confidence_millis <= 1_000
            && !self.semantic_authority
            && !self.external_execution_authorized
    }
}

pub fn resolve_typed_deixis_or_ellipsis(
    text: &str,
    antecedent_surface: &str,
) -> Option<TypedDeixisEllipsisResolutionIR> {
    let antecedent = antecedent_surface.trim();
    if antecedent.is_empty() || antecedent.chars().count() > 96 {
        return None;
    }
    resolve_possessive(text, antecedent)
        .or_else(|| resolve_demonstrative_nominal(text, antecedent))
        .or_else(|| resolve_zero_argument(text, antecedent))
        .filter(TypedDeixisEllipsisResolutionIR::validate)
}

pub fn unresolved_typed_deixis_kind(text: &str) -> Option<TypedDeixisEllipsisKindIR> {
    possessive_marker(text)
        .map(|_| TypedDeixisEllipsisKindIR::PossessiveFocusReference)
        .or_else(|| {
            demonstrative_marker(text)
                .map(|_| TypedDeixisEllipsisKindIR::DemonstrativeFocusReference)
        })
        .or_else(|| {
            zero_argument_shape(text).then_some(TypedDeixisEllipsisKindIR::ZeroArgumentEllipsis)
        })
}

fn resolve_possessive(text: &str, antecedent: &str) -> Option<TypedDeixisEllipsisResolutionIR> {
    let marker = possessive_marker(text)?;
    let replacement = if marker.eq_ignore_ascii_case("its") {
        format!("{antecedent}'s")
    } else {
        format!("{antecedent}의")
    };
    resolution(
        TypedDeixisEllipsisKindIR::PossessiveFocusReference,
        text,
        marker,
        antecedent,
        replace_unique_case_insensitive(text, marker, &replacement)?,
        940,
        "POSSESSIVE_FORM_TO_CURRENT_DISCOURSE_FOCUS",
    )
}

fn resolve_demonstrative_nominal(
    text: &str,
    antecedent: &str,
) -> Option<TypedDeixisEllipsisResolutionIR> {
    let marker = demonstrative_marker(text)?;
    let replacement = if marker.ends_with('을') || marker.ends_with('를') {
        format!("{antecedent}{}", object_particle(antecedent))
    } else if marker.ends_with('이') || marker.ends_with('가') {
        format!("{antecedent}{}", subject_particle(antecedent))
    } else {
        antecedent.to_string()
    };
    resolution(
        TypedDeixisEllipsisKindIR::DemonstrativeFocusReference,
        text,
        marker,
        antecedent,
        replace_unique_case_insensitive(text, marker, &replacement)?,
        930,
        "GENERIC_DEMONSTRATIVE_TO_CURRENT_DISCOURSE_FOCUS",
    )
}

fn resolve_zero_argument(text: &str, antecedent: &str) -> Option<TypedDeixisEllipsisResolutionIR> {
    if !zero_argument_shape(text) {
        return None;
    }
    let (resolved, source) = if text_is_english(text) {
        resolve_english_zero_argument(text, antecedent)?
    } else {
        resolve_korean_zero_argument(text, antecedent)?
    };
    resolution(
        TypedDeixisEllipsisKindIR::ZeroArgumentEllipsis,
        text,
        &source,
        antecedent,
        resolved,
        910,
        "TRANSITIVE_PREDICATE_ARGUMENT_FROM_CURRENT_DISCOURSE_FOCUS",
    )
}

fn resolution(
    kind: TypedDeixisEllipsisKindIR,
    original: &str,
    source: &str,
    antecedent: &str,
    resolved: String,
    confidence_millis: u16,
    path: &str,
) -> Option<TypedDeixisEllipsisResolutionIR> {
    (resolved != original).then(|| TypedDeixisEllipsisResolutionIR {
        schema: TYPED_DEIXIS_ELLIPSIS_SCHEMA.to_string(),
        kind,
        original_text: original.to_string(),
        resolved_text: resolved,
        source_surface: source.to_string(),
        antecedent_surface: antecedent.to_string(),
        confidence_millis,
        evidence: vec![
            format!("TYPED_DEIXIS_ELLIPSIS_PATH:{path}"),
            "ANTECEDENT_SOURCE:CURRENT_DISCOURSE_FOCUS".to_string(),
            "SEMANTIC_AUTHORITY:false".to_string(),
            "EXTERNAL_EXECUTION_AUTHORIZED:false".to_string(),
        ],
        semantic_authority: false,
        external_execution_authorized: false,
    })
}

fn possessive_marker(text: &str) -> Option<&'static str> {
    ["its", "그것의", "그거의"]
        .into_iter()
        .find(|marker| unique_bounded_occurrence(text, marker).is_some())
}

fn demonstrative_marker(text: &str) -> Option<&'static str> {
    [
        "that object",
        "that item",
        "that one",
        "그 대상을",
        "그 항목을",
        "그 객체를",
        "그 대상이",
        "그 항목이",
        "그 객체가",
        "그 대상",
        "그 항목",
        "그 객체",
    ]
    .into_iter()
    .find(|marker| unique_bounded_occurrence(text, marker).is_some())
}

fn replace_unique_case_insensitive(text: &str, marker: &str, replacement: &str) -> Option<String> {
    let start = unique_bounded_occurrence(text, marker)?;
    let end = start + marker.len();
    Some(format!("{}{}{}", &text[..start], replacement, &text[end..]))
}

fn unique_bounded_occurrence(text: &str, marker: &str) -> Option<usize> {
    let lower = text.to_lowercase();
    let marker_lower = marker.to_lowercase();
    let mut matches = lower.match_indices(&marker_lower).filter_map(|(start, _)| {
        let end = start + marker_lower.len();
        (text.is_char_boundary(start)
            && text.is_char_boundary(end)
            && boundary_before(&lower, start)
            && boundary_after(&lower, end)
            && !inside_quote(text, start))
        .then_some(start)
    });
    let first = matches.next()?;
    matches.next().is_none().then_some(first)
}

fn boundary_before(text: &str, index: usize) -> bool {
    index == 0
        || text[..index]
            .chars()
            .next_back()
            .is_none_or(|character| !character.is_ascii_alphanumeric())
}

fn boundary_after(text: &str, index: usize) -> bool {
    index == text.len()
        || text[index..]
            .chars()
            .next()
            .is_none_or(|character| !character.is_ascii_alphanumeric())
}

fn inside_quote(text: &str, index: usize) -> bool {
    let prefix = &text[..index];
    ['"', '\'', '“', '”', '‘', '’'].into_iter().any(|quote| {
        prefix
            .chars()
            .filter(|character| *character == quote)
            .count()
            % 2
            == 1
    })
}

fn zero_argument_shape(text: &str) -> bool {
    if text.trim_end().ends_with('?') {
        return false;
    }
    if text_is_english(text) {
        english_zero_argument_action(text).is_some()
    } else {
        korean_zero_argument_action(text).is_some()
    }
}

fn english_zero_argument_action(text: &str) -> Option<(usize, String)> {
    let tokens = text.split_whitespace().collect::<Vec<_>>();
    let verbs = [
        "open",
        "read",
        "transform",
        "convert",
        "save",
        "delete",
        "deploy",
        "run",
        "check",
        "inspect",
        "analyze",
        "fix",
        "repair",
        "restore",
        "verify",
    ];
    let trailing_non_arguments = [
        "again",
        "next",
        "now",
        "then",
        "later",
        "please",
        "immediately",
        "first",
        "afterward",
        "afterwards",
    ];
    let matches = tokens
        .iter()
        .enumerate()
        .filter_map(|(index, token)| {
            let core = clean_ascii_token(token);
            (verbs.contains(&core.as_str()) && english_directive_predicate_position(&tokens, index))
                .then_some((index, core))
        })
        .collect::<Vec<_>>();
    if matches.len() != 1 {
        return None;
    }
    let (index, verb) = matches[0].clone();
    let has_object = tokens[index + 1..].iter().any(|token| {
        let core = clean_ascii_token(token);
        !core.is_empty()
            && !trailing_non_arguments.contains(&core.as_str())
            && !["and", "or", "but"].contains(&core.as_str())
    });
    (!has_object).then_some((index, verb))
}

fn english_directive_predicate_position(tokens: &[&str], index: usize) -> bool {
    if index == 0 {
        return true;
    }
    if tokens[index - 1].trim_end().ends_with(',') {
        return true;
    }
    tokens[..index].iter().all(|token| {
        matches!(
            clean_ascii_token(token).as_str(),
            "then" | "now" | "next" | "please" | "again" | "later"
        )
    })
}

fn resolve_english_zero_argument(text: &str, antecedent: &str) -> Option<(String, String)> {
    let (verb_index, verb) = english_zero_argument_action(text)?;
    let mut tokens = text
        .split_whitespace()
        .map(ToString::to_string)
        .collect::<Vec<_>>();
    tokens.insert(verb_index + 1, antecedent.to_string());
    let mut resolved = tokens.join(" ");
    let lower = text.trim_start().to_lowercase();
    if lower.starts_with("if ") {
        let comma = resolved.find(',')?;
        let condition = resolved[3..comma].trim();
        if condition.split_whitespace().count() == 1 {
            resolved = format!("if {antecedent} is {condition},{}", &resolved[comma + 1..]);
        }
    }
    Some((resolved, verb))
}

fn korean_zero_argument_action(text: &str) -> Option<(usize, &'static str)> {
    let action_stems = [
        "열", "읽", "변환", "저장", "삭제", "지우", "배포", "실행", "확인", "검사", "조사", "분석",
        "고치", "수정", "수리", "복구", "검증",
    ];
    let final_token = text.split_whitespace().last()?;
    let final_token_start = text.rfind(final_token)?;
    let matches = action_stems
        .into_iter()
        .filter_map(|stem| {
            text.rfind(stem)
                .filter(|position| {
                    *position >= final_token_start
                        && korean_action_tail_is_directive(text, *position)
                })
                .map(|position| (position, stem))
        })
        .collect::<Vec<_>>();
    let (position, stem) = matches.into_iter().max_by_key(|(position, _)| *position)?;
    let prefix = &text[..position];
    let explicit_object = prefix.split_whitespace().any(|token| {
        token.ends_with('을')
            || token.ends_with('를')
            || matches!(
                token.trim_matches(|character: char| character.is_ascii_punctuation()),
                "그걸" | "그것을" | "그거" | "그거를" | "이걸" | "저걸" | "전자를" | "후자를"
            )
    });
    (!explicit_object).then_some((position, stem))
}

fn korean_action_tail_is_directive(text: &str, action_position: usize) -> bool {
    let tail = text[action_position..]
        .trim()
        .trim_end_matches(|character: char| character.is_ascii_punctuation());
    ["해", "해줘", "하세요", "해라", "해봐", "어", "아", "줘"]
        .iter()
        .any(|ending| tail.ends_with(ending))
}

fn resolve_korean_zero_argument(text: &str, antecedent: &str) -> Option<(String, String)> {
    let (action_position, stem) = korean_zero_argument_action(text)?;
    let mut resolved = format!(
        "{}{}{} {}",
        &text[..action_position],
        antecedent,
        object_particle(antecedent),
        &text[action_position..]
    );
    let first_token = text.split_whitespace().next().unwrap_or_default();
    if first_token.ends_with('면') && action_position >= first_token.len() {
        resolved = format!(
            "{antecedent}{} {first_token} {}",
            subject_particle(antecedent),
            resolved[first_token.len()..].trim_start()
        );
    }
    Some((resolved, stem.to_string()))
}

fn text_is_english(text: &str) -> bool {
    let ascii_letters = text
        .chars()
        .filter(|character| character.is_ascii_alphabetic())
        .count();
    let hangul = text
        .chars()
        .filter(|character| ('\u{ac00}'..='\u{d7a3}').contains(character))
        .count();
    ascii_letters > hangul
}

fn clean_ascii_token(token: &str) -> String {
    token
        .trim_matches(|character: char| !character.is_ascii_alphanumeric())
        .to_lowercase()
}

fn has_final_consonant(value: &str) -> bool {
    value.chars().next_back().is_some_and(|character| {
        let code = u32::from(character);
        (0xac00..=0xd7a3).contains(&code) && (code - 0xac00) % 28 != 0
    })
}

fn object_particle(value: &str) -> &'static str {
    if has_final_consonant(value) {
        "을"
    } else {
        "를"
    }
}

fn subject_particle(value: &str) -> &'static str {
    if has_final_consonant(value) {
        "이"
    } else {
        "가"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_structural_deixis_without_authority() {
        let possessive =
            resolve_typed_deixis_or_ellipsis("inspect its status", "queue").expect("possessive");
        assert_eq!(possessive.resolved_text, "inspect queue's status");
        assert!(possessive.validate());
        let demonstrative =
            resolve_typed_deixis_or_ellipsis("그 대상을 분석해", "스냅샷").expect("demonstrative");
        assert_eq!(demonstrative.resolved_text, "스냅샷을 분석해");
    }

    #[test]
    fn resolves_direct_and_conditional_zero_arguments() {
        let direct = resolve_typed_deixis_or_ellipsis("then inspect again", "queue")
            .expect("direct ellipsis");
        assert_eq!(direct.resolved_text, "then inspect queue again");
        let conditional = resolve_typed_deixis_or_ellipsis("if stale, repair", "worker")
            .expect("conditional ellipsis");
        assert_eq!(
            conditional.resolved_text,
            "if worker is stale, repair worker"
        );
        let korean = resolve_typed_deixis_or_ellipsis("깨졌으면 다시 검사해", "스냅샷")
            .expect("Korean conditional ellipsis");
        assert_eq!(
            korean.resolved_text,
            "스냅샷이 깨졌으면 다시 스냅샷을 검사해"
        );
    }

    #[test]
    fn identifies_unresolved_zero_argument_but_not_explicit_object() {
        assert_eq!(
            unresolved_typed_deixis_kind("repair next"),
            Some(TypedDeixisEllipsisKindIR::ZeroArgumentEllipsis)
        );
        assert_eq!(unresolved_typed_deixis_kind("repair queue"), None);
        assert_eq!(
            unresolved_typed_deixis_kind("배포가 시작되기 전에 백업이 완료됐다."),
            None
        );
        assert_eq!(
            unresolved_typed_deixis_kind("if the cache is valid, continue the run"),
            None
        );
        assert_eq!(unresolved_typed_deixis_kind("그 저장 계층을 검토해"), None);
    }
}
