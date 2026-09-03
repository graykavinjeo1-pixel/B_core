//! Bounded, adapter-local entity memory and typed coreference.
//!
//! This module never promotes a mention into the semantic catalog.  It keeps
//! dialogue-local, provenance-bearing bindings and fails closed when more than
//! one type-compatible antecedent remains.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::attribution::{AttributionGraphIR, DiscourseActorKindIR};
use crate::semantic_roles::{SemanticNodeKindIR, SemanticRoleGraphIR, SemanticRoleKindIR};

pub const MAX_TYPED_ENTITY_REFERENTS: usize = 32;
pub const MAX_TYPED_REFERENCE_TURN_DISTANCE: u64 = 16;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TypedEntityKindIR {
    Person,
    Organization,
    Place,
    Artifact,
    System,
    Process,
    Abstract,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TypedMentionRoleIR {
    AttributionSource,
    Agent,
    Topic,
    Theme,
    Patient,
    Experiencer,
    Recipient,
    Source,
    Destination,
    Instrument,
    Location,
    Result,
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TypedEntityReferentIR {
    pub entity_id: String,
    pub canonical_surface: String,
    pub normalized_label: String,
    pub kind: TypedEntityKindIR,
    pub mention_roles: Vec<TypedMentionRoleIR>,
    pub introduced_turn: u64,
    pub last_mentioned_turn: u64,
    pub mention_count: u32,
    pub semantic_authority: bool,
}

impl TypedEntityReferentIR {
    pub fn validate(&self, completed_turns: u64) -> bool {
        !self.entity_id.trim().is_empty()
            && !self.canonical_surface.trim().is_empty()
            && !self.normalized_label.trim().is_empty()
            && self.introduced_turn > 0
            && self.last_mentioned_turn >= self.introduced_turn
            && self.last_mentioned_turn <= completed_turns
            && self.mention_count > 0
            && !self.semantic_authority
            && self.mention_roles.windows(2).all(|pair| pair[0] < pair[1])
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TypedCoreferenceBindingKind {
    Entity,
    BeliefHolder,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypedCoreferenceResolution {
    pub resolved_text: String,
    pub source_surface: Option<String>,
    pub resolved_surface: Option<String>,
    pub entity_ids: Vec<String>,
    pub binding_kind: Option<TypedCoreferenceBindingKind>,
    pub confidence_millis: u16,
    pub ambiguous_surfaces: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ReferenceClass {
    Person,
    System,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Realization {
    Plain,
    EnglishPossessive,
    KoreanSubject,
    KoreanObject,
    KoreanPossessive,
}

#[derive(Debug, Clone, Copy)]
struct ReferencePattern {
    marker: &'static str,
    class: ReferenceClass,
    binding: TypedCoreferenceBindingKind,
    realization: Realization,
}

pub fn merge_typed_mentions(
    referents: &mut Vec<TypedEntityReferentIR>,
    turn: u64,
    role_graph: Option<&SemanticRoleGraphIR>,
    attribution_graph: Option<&AttributionGraphIR>,
) {
    let mut mentions = BTreeMap::<String, Mention>::new();
    if let Some(graph) = attribution_graph {
        for actor in &graph.actors {
            if actor.kind == DiscourseActorKindIR::DialogueSpeaker {
                continue;
            }
            let surface = clean_surface(&actor.surface);
            let normalized = normalize_label(&surface);
            if invalid_mention(&surface, &normalized) {
                continue;
            }
            let kind = infer_kind(&surface, true, None);
            mentions
                .entry(normalized)
                .and_modify(|mention| {
                    mention.roles.insert(TypedMentionRoleIR::AttributionSource);
                })
                .or_insert_with(|| {
                    Mention::new(surface, kind, TypedMentionRoleIR::AttributionSource)
                });
        }
    }
    if let Some(graph) = role_graph {
        for node in &graph.nodes {
            if node.kind != SemanticNodeKindIR::Entity {
                continue;
            }
            let surface = clean_surface(&node.surface);
            let normalized = normalize_label(&surface);
            if invalid_mention(&surface, &normalized) {
                continue;
            }
            let roles = graph
                .role_edges
                .iter()
                .filter(|edge| edge.argument_node_id == node.node_id)
                .map(|edge| mention_role(edge.role))
                .collect::<BTreeSet<_>>();
            let actor_like = roles.iter().any(|role| {
                matches!(
                    role,
                    TypedMentionRoleIR::Agent | TypedMentionRoleIR::Experiencer
                )
            });
            let kind = infer_kind(&surface, actor_like, node.concept_id_hint.as_deref());
            let entry = mentions
                .entry(normalized)
                .or_insert_with(|| Mention::new(surface, kind, TypedMentionRoleIR::Other));
            if entry.kind == TypedEntityKindIR::Unknown && kind != TypedEntityKindIR::Unknown {
                entry.kind = kind;
            }
            entry.roles.extend(roles);
            entry.roles.remove(&TypedMentionRoleIR::Other);
        }
    }

    for (normalized, mention) in mentions {
        if let Some(existing) = referents
            .iter_mut()
            .find(|referent| referent.normalized_label == normalized)
        {
            existing.last_mentioned_turn = turn;
            existing.mention_count = existing.mention_count.saturating_add(1);
            if existing.kind == TypedEntityKindIR::Unknown
                && mention.kind != TypedEntityKindIR::Unknown
            {
                existing.kind = mention.kind;
            }
            let mut roles = existing
                .mention_roles
                .iter()
                .copied()
                .collect::<BTreeSet<_>>();
            roles.extend(mention.roles);
            existing.mention_roles = roles.into_iter().collect();
            continue;
        }
        let suffix = referents
            .iter()
            .filter(|referent| referent.introduced_turn == turn)
            .count()
            + 1;
        referents.push(TypedEntityReferentIR {
            entity_id: format!("TREF-{turn:06}-{suffix:02}"),
            canonical_surface: mention.surface,
            normalized_label: normalized,
            kind: mention.kind,
            mention_roles: mention.roles.into_iter().collect(),
            introduced_turn: turn,
            last_mentioned_turn: turn,
            mention_count: 1,
            semantic_authority: false,
        });
    }
    referents.sort_by(|left, right| {
        right
            .last_mentioned_turn
            .cmp(&left.last_mentioned_turn)
            .then_with(|| right.mention_count.cmp(&left.mention_count))
            .then_with(|| left.entity_id.cmp(&right.entity_id))
    });
    referents.truncate(MAX_TYPED_ENTITY_REFERENTS);
}

pub fn resolve_typed_coreference(
    referents: &[TypedEntityReferentIR],
    completed_turns: u64,
    text: &str,
) -> TypedCoreferenceResolution {
    let Some(pattern) = reference_patterns()
        .iter()
        .find(|pattern| contains_marker(text, pattern.marker))
    else {
        return unchanged(text);
    };
    if is_explanation_recipient(text, pattern.marker)
        || is_local_process_object(text, pattern.marker)
    {
        return unchanged(text);
    }
    if marker_is_quoted(text, pattern.marker) {
        return unchanged(text);
    }
    let candidates = referents
        .iter()
        .filter(|referent| {
            completed_turns.saturating_sub(referent.last_mentioned_turn)
                <= MAX_TYPED_REFERENCE_TURN_DISTANCE
                && class_compatible(pattern.class, referent.kind)
        })
        .collect::<Vec<_>>();
    if candidates.len() != 1 {
        return TypedCoreferenceResolution {
            resolved_text: text.to_string(),
            source_surface: None,
            resolved_surface: None,
            entity_ids: Vec::new(),
            binding_kind: None,
            confidence_millis: 0,
            ambiguous_surfaces: vec![pattern.marker.to_string()],
        };
    }
    let referent = candidates[0];
    let replacement = realize(
        &referent.canonical_surface,
        pattern.realization,
        pattern.marker,
    );
    TypedCoreferenceResolution {
        resolved_text: replace_first_case_insensitive(text, pattern.marker, &replacement),
        source_surface: Some(pattern.marker.to_string()),
        resolved_surface: Some(replacement),
        entity_ids: vec![referent.entity_id.clone()],
        binding_kind: Some(pattern.binding),
        confidence_millis: if pattern.class == ReferenceClass::System {
            950
        } else {
            910
        },
        ambiguous_surfaces: Vec::new(),
    }
}

fn is_explanation_recipient(text: &str, marker: &str) -> bool {
    let lower = text.to_lowercase();
    ["walk", "talk"].iter().any(|verb| {
        let construction = format!("{verb} {} through", marker.to_lowercase());
        lower.contains(&construction)
    })
}

fn is_local_process_object(text: &str, marker: &str) -> bool {
    if !marker.eq_ignore_ascii_case("them") {
        return false;
    }
    let lower = text.to_lowercase();
    let has_local_plural = ["steps", "checks", "actions"]
        .iter()
        .any(|surface| lower.contains(surface));
    has_local_plural
        && [
            "apply them",
            "applying them",
            "execute them",
            "executing them",
            "perform them",
            "performing them",
        ]
        .iter()
        .any(|construction| lower.contains(construction))
}

struct Mention {
    surface: String,
    kind: TypedEntityKindIR,
    roles: BTreeSet<TypedMentionRoleIR>,
}

impl Mention {
    fn new(surface: String, kind: TypedEntityKindIR, role: TypedMentionRoleIR) -> Self {
        Self {
            surface,
            kind,
            roles: BTreeSet::from([role]),
        }
    }
}

fn unchanged(text: &str) -> TypedCoreferenceResolution {
    TypedCoreferenceResolution {
        resolved_text: text.to_string(),
        source_surface: None,
        resolved_surface: None,
        entity_ids: Vec::new(),
        binding_kind: None,
        confidence_millis: 0,
        ambiguous_surfaces: Vec::new(),
    }
}

fn reference_patterns() -> &'static [ReferencePattern] {
    &[
        ReferencePattern {
            marker: "her claim",
            class: ReferenceClass::Person,
            binding: TypedCoreferenceBindingKind::BeliefHolder,
            realization: Realization::EnglishPossessive,
        },
        ReferencePattern {
            marker: "his claim",
            class: ReferenceClass::Person,
            binding: TypedCoreferenceBindingKind::BeliefHolder,
            realization: Realization::EnglishPossessive,
        },
        ReferencePattern {
            marker: "her belief",
            class: ReferenceClass::Person,
            binding: TypedCoreferenceBindingKind::BeliefHolder,
            realization: Realization::EnglishPossessive,
        },
        ReferencePattern {
            marker: "his belief",
            class: ReferenceClass::Person,
            binding: TypedCoreferenceBindingKind::BeliefHolder,
            realization: Realization::EnglishPossessive,
        },
        ReferencePattern {
            marker: "her statement",
            class: ReferenceClass::Person,
            binding: TypedCoreferenceBindingKind::BeliefHolder,
            realization: Realization::EnglishPossessive,
        },
        ReferencePattern {
            marker: "his statement",
            class: ReferenceClass::Person,
            binding: TypedCoreferenceBindingKind::BeliefHolder,
            realization: Realization::EnglishPossessive,
        },
        ReferencePattern {
            marker: "그녀의 주장",
            class: ReferenceClass::Person,
            binding: TypedCoreferenceBindingKind::BeliefHolder,
            realization: Realization::KoreanPossessive,
        },
        ReferencePattern {
            marker: "그의 주장",
            class: ReferenceClass::Person,
            binding: TypedCoreferenceBindingKind::BeliefHolder,
            realization: Realization::KoreanPossessive,
        },
        ReferencePattern {
            marker: "그녀의 믿음",
            class: ReferenceClass::Person,
            binding: TypedCoreferenceBindingKind::BeliefHolder,
            realization: Realization::KoreanPossessive,
        },
        ReferencePattern {
            marker: "그의 믿음",
            class: ReferenceClass::Person,
            binding: TypedCoreferenceBindingKind::BeliefHolder,
            realization: Realization::KoreanPossessive,
        },
        ReferencePattern {
            marker: "그녀의 말",
            class: ReferenceClass::Person,
            binding: TypedCoreferenceBindingKind::BeliefHolder,
            realization: Realization::KoreanPossessive,
        },
        ReferencePattern {
            marker: "그의 말",
            class: ReferenceClass::Person,
            binding: TypedCoreferenceBindingKind::BeliefHolder,
            realization: Realization::KoreanPossessive,
        },
        ReferencePattern {
            marker: "that service",
            class: ReferenceClass::System,
            binding: TypedCoreferenceBindingKind::Entity,
            realization: Realization::Plain,
        },
        ReferencePattern {
            marker: "그 서비스를",
            class: ReferenceClass::System,
            binding: TypedCoreferenceBindingKind::Entity,
            realization: Realization::KoreanObject,
        },
        ReferencePattern {
            marker: "그녀가",
            class: ReferenceClass::Person,
            binding: TypedCoreferenceBindingKind::Entity,
            realization: Realization::KoreanSubject,
        },
        ReferencePattern {
            marker: "그가",
            class: ReferenceClass::Person,
            binding: TypedCoreferenceBindingKind::Entity,
            realization: Realization::KoreanSubject,
        },
        ReferencePattern {
            marker: "she",
            class: ReferenceClass::Person,
            binding: TypedCoreferenceBindingKind::Entity,
            realization: Realization::Plain,
        },
        ReferencePattern {
            marker: "he",
            class: ReferenceClass::Person,
            binding: TypedCoreferenceBindingKind::Entity,
            realization: Realization::Plain,
        },
        ReferencePattern {
            marker: "her",
            class: ReferenceClass::Person,
            binding: TypedCoreferenceBindingKind::Entity,
            realization: Realization::Plain,
        },
        ReferencePattern {
            marker: "him",
            class: ReferenceClass::Person,
            binding: TypedCoreferenceBindingKind::Entity,
            realization: Realization::Plain,
        },
    ]
}

fn class_compatible(class: ReferenceClass, kind: TypedEntityKindIR) -> bool {
    match class {
        ReferenceClass::Person => kind == TypedEntityKindIR::Person,
        ReferenceClass::System => kind == TypedEntityKindIR::System,
    }
}

fn contains_marker(text: &str, marker: &str) -> bool {
    marker_start(text, marker).is_some()
}

fn marker_start(text: &str, marker: &str) -> Option<usize> {
    let lower = text.to_lowercase();
    lower.match_indices(marker).find_map(|(start, _)| {
        let before = lower[..start].chars().next_back();
        let after = lower[start + marker.len()..].chars().next();
        (before.is_none_or(|character| !character.is_alphanumeric() && character != '_')
            && after.is_none_or(|character| marker_has_valid_right_boundary(marker, character)))
        .then_some(start)
    })
}

fn marker_has_valid_right_boundary(marker: &str, character: char) -> bool {
    if !character.is_alphanumeric() && character != '_' {
        return true;
    }
    let korean_marker = marker
        .chars()
        .any(|value| ('\u{ac00}'..='\u{d7a3}').contains(&value));
    korean_marker && "은는이가을를도의에와과로만부터까지처럼보다".contains(character)
}

fn marker_is_quoted(text: &str, marker: &str) -> bool {
    let Some(start) = marker_start(text, marker) else {
        return false;
    };
    let before = text[..start]
        .chars()
        .filter(|character| matches!(character, '"' | '\'' | '‘' | '“'))
        .count();
    let after = text[start + marker.len()..]
        .chars()
        .filter(|character| matches!(character, '"' | '\'' | '’' | '”'))
        .count();
    before % 2 == 1 && after > 0
}

fn replace_first_case_insensitive(text: &str, marker: &str, replacement: &str) -> String {
    let Some(start) = marker_start(text, marker) else {
        return text.to_string();
    };
    let end = start + marker.len();
    if !text.is_char_boundary(start) || !text.is_char_boundary(end) {
        return text.to_string();
    }
    format!("{}{}{}", &text[..start], replacement, &text[end..])
}

fn realize(surface: &str, form: Realization, marker: &str) -> String {
    match form {
        Realization::Plain => surface.to_string(),
        Realization::EnglishPossessive => {
            format!(
                "{surface}'s {}",
                marker.split_whitespace().next_back().unwrap_or("claim")
            )
        }
        Realization::KoreanSubject => format!("{surface}{}", subject_particle(surface)),
        Realization::KoreanObject => format!("{surface}{}", object_particle(surface)),
        Realization::KoreanPossessive => {
            format!(
                "{surface}의 {}",
                marker.split_whitespace().next_back().unwrap_or("주장")
            )
        }
    }
}

fn clean_surface(surface: &str) -> String {
    let trimmed = surface
        .trim()
        .trim_matches(|character: char| character.is_ascii_punctuation())
        .trim();
    let lower = trimmed.to_lowercase();
    for article in ["the ", "a ", "an "] {
        if lower.starts_with(article) {
            return trimmed[article.len()..].trim().to_string();
        }
    }
    trimmed.to_string()
}

fn normalize_label(surface: &str) -> String {
    surface
        .to_lowercase()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn invalid_mention(surface: &str, normalized: &str) -> bool {
    surface.is_empty()
        || normalized.is_empty()
        || surface.chars().count() > 64
        || is_pronominal(normalized)
        || matches!(normalized, "user" | "dialogue_speaker" | "사용자")
}

fn is_pronominal(label: &str) -> bool {
    matches!(
        label,
        "she"
            | "he"
            | "her"
            | "him"
            | "his"
            | "it"
            | "they"
            | "them"
            | "그"
            | "그녀"
            | "그것"
            | "그들"
    )
}

fn infer_kind(surface: &str, actor_like: bool, concept_hint: Option<&str>) -> TypedEntityKindIR {
    let lower = surface.to_lowercase();
    if contains_any(
        &lower,
        &[
            "service",
            "server",
            "worker",
            "parser",
            "compiler",
            "scheduler",
            "서비스",
            "서버",
            "워커",
            "파서",
            "컴파일러",
            "스케줄러",
        ],
    ) {
        return TypedEntityKindIR::System;
    }
    if contains_any(
        &lower,
        &["company", "team", "organization", "회사", "팀", "조직"],
    ) {
        return TypedEntityKindIR::Organization;
    }
    if contains_any(
        &lower,
        &["room", "office", "region", "방", "사무실", "지역"],
    ) {
        return TypedEntityKindIR::Place;
    }
    if contains_any(
        &lower,
        &[
            "migration",
            "deployment",
            "build",
            "마이그레이션",
            "배포",
            "빌드",
        ],
    ) {
        return TypedEntityKindIR::Process;
    }
    if concept_hint.is_some()
        || contains_any(
            &lower,
            &[
                "file",
                "folder",
                "report",
                "document",
                "code",
                "repository",
                "plan",
                "cache",
                "파일",
                "폴더",
                "보고서",
                "문서",
                "코드",
                "저장소",
                "계획",
                "캐시",
            ],
        )
    {
        return TypedEntityKindIR::Artifact;
    }
    if actor_like {
        return TypedEntityKindIR::Person;
    }
    TypedEntityKindIR::Unknown
}

fn mention_role(role: SemanticRoleKindIR) -> TypedMentionRoleIR {
    match role {
        SemanticRoleKindIR::Agent => TypedMentionRoleIR::Agent,
        SemanticRoleKindIR::Topic => TypedMentionRoleIR::Topic,
        SemanticRoleKindIR::Theme | SemanticRoleKindIR::CoTheme => TypedMentionRoleIR::Theme,
        SemanticRoleKindIR::Patient => TypedMentionRoleIR::Patient,
        SemanticRoleKindIR::Experiencer => TypedMentionRoleIR::Experiencer,
        SemanticRoleKindIR::Recipient => TypedMentionRoleIR::Recipient,
        SemanticRoleKindIR::Source => TypedMentionRoleIR::Source,
        SemanticRoleKindIR::Destination => TypedMentionRoleIR::Destination,
        SemanticRoleKindIR::Instrument => TypedMentionRoleIR::Instrument,
        SemanticRoleKindIR::Location => TypedMentionRoleIR::Location,
        SemanticRoleKindIR::Result | SemanticRoleKindIR::PriorResult => TypedMentionRoleIR::Result,
        SemanticRoleKindIR::ComparisonPeer => TypedMentionRoleIR::Other,
    }
}

fn contains_any(text: &str, markers: &[&str]) -> bool {
    markers.iter().any(|marker| text.contains(marker))
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
    use crate::compositional_semantics::CompositionalSemanticAnalyzer;

    #[test]
    fn attribution_actor_survives_incompatible_distractors() {
        let analysis = CompositionalSemanticAnalyzer.analyze("Avery says that the cache is stale");
        let mut entities = Vec::new();
        merge_typed_mentions(
            &mut entities,
            1,
            Some(&analysis.semantic_role_graph),
            Some(&analysis.attribution_graph),
        );
        let resolution = resolve_typed_coreference(&entities, 9, "She corrected the report");
        assert_eq!(resolution.resolved_text, "avery corrected the report");
        assert_eq!(
            resolution.binding_kind,
            Some(TypedCoreferenceBindingKind::Entity)
        );
    }

    #[test]
    fn two_people_fail_closed() {
        let mut entities = Vec::new();
        for (turn, text) in [
            "Quinn says that the build failed",
            "Rowan says that the cache failed",
        ]
        .into_iter()
        .enumerate()
        {
            let analysis = CompositionalSemanticAnalyzer.analyze(text);
            merge_typed_mentions(
                &mut entities,
                u64::try_from(turn + 1).expect("turn"),
                Some(&analysis.semantic_role_graph),
                Some(&analysis.attribution_graph),
            );
        }
        let resolution = resolve_typed_coreference(&entities, 2, "She corrected the report");
        assert_eq!(resolution.resolved_text, "She corrected the report");
        assert_eq!(resolution.ambiguous_surfaces, vec!["she"]);
    }

    #[test]
    fn quoted_pronoun_is_not_bound() {
        let mut entities = vec![TypedEntityReferentIR {
            entity_id: "TREF-000001-01".to_string(),
            canonical_surface: "Avery".to_string(),
            normalized_label: "avery".to_string(),
            kind: TypedEntityKindIR::Person,
            mention_roles: vec![TypedMentionRoleIR::AttributionSource],
            introduced_turn: 1,
            last_mentioned_turn: 1,
            mention_count: 1,
            semantic_authority: false,
        }];
        let resolution = resolve_typed_coreference(&entities, 1, "quote ‘she failed’");
        assert_eq!(resolution.resolved_text, "quote ‘she failed’");
        entities[0].semantic_authority = true;
        assert!(!entities[0].validate(1));
    }

    #[test]
    fn korean_particle_suffix_is_allowed_but_embedded_pronoun_is_not() {
        let entities = vec![TypedEntityReferentIR {
            entity_id: "TREF-000001-01".to_string(),
            canonical_surface: "가람".to_string(),
            normalized_label: "가람".to_string(),
            kind: TypedEntityKindIR::Person,
            mention_roles: vec![TypedMentionRoleIR::AttributionSource],
            introduced_turn: 1,
            last_mentioned_turn: 1,
            mention_count: 1,
            semantic_authority: false,
        }];
        let possessive = resolve_typed_coreference(&entities, 1, "그녀의 주장을 설명해");
        assert_eq!(possessive.resolved_text, "가람의 주장을 설명해");
        assert_eq!(
            possessive.binding_kind,
            Some(TypedCoreferenceBindingKind::BeliefHolder)
        );

        let embedded = resolve_typed_coreference(&entities, 1, "로그가 비었다");
        assert_eq!(embedded.resolved_text, "로그가 비었다");
        assert!(embedded.binding_kind.is_none());
    }
}
